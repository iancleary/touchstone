//! EXPERIMENTAL proper real-rational models with one or two shared poles.
//!
//! This is pole-relocating vector fitting, not interpolation, passivity enforcement,
//! or an accuracy certificate. Inspect [`FitReport::status`](crate::vector_fitting::FitReport::status) and independently
//! [`RationalNetworkModel::assess`](crate::vector_fitting::RationalNetworkModel::assess) withheld measurements. Evaluation is in-band only.
//! See `docs/vector-fitting.md` for the algorithm, bounds, and measured limitations.
mod numerics;

use crate::{
    Complex, Extrapolation, Interpolation, Network, ReferenceImpedance, SMatrix, TouchstoneWarning,
};
use std::fmt;

/// Initial poles in normalized coordinates `z = s/(2π f_max) = j f/f_max`.
/// Topology can change during relocation; no order reduction is performed.
#[derive(Debug, Clone, Copy)]
pub enum InitialPoles {
    /// One strictly negative real pole.
    OneReal(f64),
    /// Two distinct strictly negative real poles.
    TwoReal(f64, f64),
    /// A conjugate pair specified by negative real part and positive imaginary part.
    ConjugatePair {
        /// Negative real part.
        real: f64,
        /// Positive imaginary part.
        imaginary: f64,
    },
}

/// Named fit configuration, checked at every fitting API boundary.
#[derive(Debug, Clone, Copy)]
pub struct FitOptions {
    /// Explicit initial pole configuration; no automatic order/seed selection.
    pub initial_poles: InitialPoles,
    /// Iteration budget in 1..=12. Convergence is distinct from accuracy.
    pub max_iterations: usize,
}

/// Failure to establish a supported or numerically identifiable fitting problem.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FitError {
    /// A configuration, sample grid, or held-out grid violates a documented bound.
    InvalidInput(&'static str),
    /// Current source semantics are unsupported or inconsistent.
    SourceSemantics(&'static str),
    /// Poles violate stability, topology, magnitude, or separation bounds.
    InvalidPoles,
    /// Scaled CPQR detected numerical rank loss; no pseudoinverse or lower-order fallback.
    RankDeficient {
        /// Number of independent columns found (zero for a zero column).
        rank: usize,
        /// Requested unknown count.
        columns: usize,
        /// Failing scaled pivot norm.
        pivot: f64,
    },
    /// A finite-input calculation did not yield a representable finite result.
    Numerical,
}
impl fmt::Display for FitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "experimental vector fitting: {self:?}")
    }
}
impl std::error::Error for FitError {}

/// Bounded numerical termination status, never an accuracy or physical certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitStatus {
    /// Pole movement <=1e-9, max |sigma-1| <=1e-7, no last-step reflection.
    Converged,
    /// Budget exhausted. The returned model is only a diagnostic candidate.
    IterationLimit,
    /// Explicit fixed-pole baseline: no relocation, NOT vector fitting/convergence.
    FixedPolesNotRelocated,
}

/// Absolute complex error over all values (or one matrix entry).
#[derive(Debug, Clone, Copy)]
pub struct ErrorMetrics {
    /// Root mean square absolute complex S error, rounded to physical f64 units.
    pub rms: f64,
    /// Maximum absolute complex S error.
    pub max: f64,
    /// Relative L2 error from scaled norms before physical RMS rounding.
    /// Absent only when the reference is identically zero.
    pub relative_l2: Option<f64>,
}

/// Aggregate and per-entry errors, row-major destination then source port order.
#[derive(Debug, Clone)]
pub struct MatrixErrors {
    /// All matrix entries equally weighted; absolute, not relative-error fitting.
    pub aggregate: ErrorMetrics,
    /// Per-entry errors in row-major order, including weak traces.
    pub per_entry: Vec<ErrorMetrics>,
}

/// Diagnostics for a single relocation followed by a fresh residue/D solve.
#[derive(Debug, Clone)]
pub struct FitIteration {
    /// 1-based iteration number.
    pub iteration: usize,
    /// Normalized poles after relocation.
    pub poles: Vec<Complex>,
    /// Unordered matched pole movement divided by max(1, old pole magnitude).
    pub pole_movement: f64,
    /// Largest training |sigma-1| before relocation.
    pub max_sigma_minus_one: f64,
    /// Number of right-half-plane poles reflected on this step.
    pub reflected_poles: usize,
    /// Smallest/largest scaled QR diagonal; NOT a condition number bound.
    pub relocation_pivot_ratio: f64,
    /// Same indicator for the residue/D identification solve.
    pub residue_pivot_ratio: f64,
    /// Actual final-response training error, not the linearized equation residual.
    pub training_error: MatrixErrors,
}

/// Fit provenance and bounded termination evidence. Model-owned report is read-only.
#[derive(Debug, Clone)]
pub struct FitReport {
    /// Numerical termination, separate from suitability.
    pub status: FitStatus,
    /// Training point count (8..=512).
    pub training_points: usize,
    /// Shared pole count (1 or 2).
    pub order: usize,
    /// Closed training band in Hz.
    pub band_hz: [f64; 2],
    /// Relocation row count, at most 4096.
    pub real_rows: usize,
    /// Relocation unknown count, at most 14.
    pub relocation_columns: usize,
    /// At most 12 iteration records; empty for the fixed-pole baseline.
    pub iterations: Vec<FitIteration>,
    /// Final actual response error on training samples.
    pub training_error: MatrixErrors,
}

/// Independent held-out comparison with the crate's actual RI-linear interpolator.
#[derive(Debug, Clone)]
pub struct HeldOutReport {
    /// Number of distinct withheld in-band points.
    pub points: usize,
    /// Rational model errors, per entry and aggregate.
    pub model: MatrixErrors,
    /// Errors of `Network::sample_at(Linear, Error)` on the training network.
    pub ri_linear: MatrixErrors,
    /// Snapshot of current parser warnings from the held-out source.
    pub source_warnings: Vec<TouchstoneWarning>,
}

/// EXPERIMENTAL immutable proper rational S model, full 1/2-port matrices only.
///
/// Stored frequency and amplitude normalization avoids physical rad/s overflow.
/// A converged stable model is not necessarily accurate, passive, or a certificate
/// about the measurement. No delay term, extrapolation, or automatic order search.
#[derive(Debug, Clone)]
pub struct RationalNetworkModel {
    poles: Vec<Complex>,
    coefficients: Vec<Vec<f64>>,
    amplitude: f64,
    training: Network,
    report: FitReport,
}

impl RationalNetworkModel {
    /// Fit common sigma coefficients, relocate its zeros, then refit real D/residues.
    ///
    /// A successful return can have [`FitStatus::IterationLimit`]; always inspect
    /// [`Self::report`]. Rank loss is an error, including direct-only/overordered data.
    /// Validates current metadata/warnings and snapshots the source. Prior metadata
    /// retagging or warning deletion cannot be detected; metadata edits alone are
    /// not physical parameter conversion or impedance renormalization.
    pub fn fit(source: &Network, options: FitOptions) -> Result<Self, FitError> {
        Self::fit_impl(source, options, true)
    }

    /// EXPERIMENTAL fixed-pole least-squares comparison, explicitly NOT vector fitting.
    /// Uses the same normalization, real basis, and checked source contract.
    pub fn fixed_pole_baseline(source: &Network, options: FitOptions) -> Result<Self, FitError> {
        Self::fit_impl(source, options, false)
    }

    fn fit_impl(
        source: &Network,
        options: FitOptions,
        relocate_poles: bool,
    ) -> Result<Self, FitError> {
        if !(1..=12).contains(&options.max_iterations) {
            return Err(FitError::InvalidInput("max_iterations must be 1..=12"));
        }
        let mut poles = options.initial_poles.checked()?;
        let count = source.f.len();
        if !(8..=512).contains(&count) {
            return Err(FitError::InvalidInput("training count must be 8..=512"));
        }
        validate_source(source)?;
        let scale = source.f[count - 1];
        let xs: Vec<_> = source.f.iter().map(|f| f / scale).collect();
        if !scale.is_finite()
            || scale <= 0.0
            || xs[count - 1] - xs[0] < 1e-6
            || xs.windows(2).any(|w| w[1] - w[0] < 1e-12)
        {
            return Err(FitError::InvalidInput(
                "training grid too narrow or clustered",
            ));
        }
        let values = source_values(source)?;
        // Component max, rather than complex magnitude, stays finite near f64::MAX.
        let amplitude = values
            .iter()
            .flatten()
            .fold(0.0_f64, |a, v| a.max(v.re.abs()).max(v.im.abs()));
        if amplitude == 0.0 {
            return Err(FitError::RankDeficient {
                rank: 0,
                columns: poles.len(),
                pivot: 0.0,
            });
        }
        let normalized: Vec<Vec<_>> = values
            .iter()
            .map(|row| row.iter().map(|v| *v / amplitude).collect())
            .collect();
        let channels = (source.rank * source.rank) as usize;
        let order = poles.len();
        let empty = measure(&values, &values, channels)?;
        let mut report = FitReport {
            status: if relocate_poles {
                FitStatus::IterationLimit
            } else {
                FitStatus::FixedPolesNotRelocated
            },
            training_points: count,
            order,
            band_hz: [source.f[0], scale],
            real_rows: count * channels * 2,
            relocation_columns: channels * (order + 1) + order,
            iterations: Vec::new(),
            training_error: empty,
        };
        let mut coefficients = Vec::new();
        if relocate_poles {
            for iteration in 1..=options.max_iterations {
                let solution = solve(&xs, &normalized, &poles, true)?;
                let sigma = &solution.coefficients[channels * (order + 1)..];
                let sigma_delta = xs
                    .iter()
                    .map(|x| linear_combination(&basis(*x, &poles), sigma).magnitude())
                    .fold(0.0_f64, f64::max);
                let (next, reflected) = relocate(&poles, sigma)?;
                let movement = pole_movement(&poles, &next);
                poles = next;
                let residues = solve(&xs, &normalized, &poles, false)?;
                coefficients = residues
                    .coefficients
                    .chunks(order + 1)
                    .map(|c| c.to_vec())
                    .collect();
                let prediction = predict_grid(&xs, &poles, &coefficients, amplitude)?;
                let error = measure(&prediction, &values, channels)?;
                report.iterations.push(FitIteration {
                    iteration,
                    poles: poles.clone(),
                    pole_movement: movement,
                    max_sigma_minus_one: sigma_delta,
                    reflected_poles: reflected,
                    relocation_pivot_ratio: solution.pivot_ratio,
                    residue_pivot_ratio: residues.pivot_ratio,
                    training_error: error.clone(),
                });
                report.training_error = error;
                if movement <= 1e-9 && sigma_delta <= 1e-7 && reflected == 0 {
                    report.status = FitStatus::Converged;
                    break;
                }
            }
        } else {
            let solution = solve(&xs, &normalized, &poles, false)?;
            coefficients = solution
                .coefficients
                .chunks(order + 1)
                .map(|c| c.to_vec())
                .collect();
            report.training_error = measure(
                &predict_grid(&xs, &poles, &coefficients, amplitude)?,
                &values,
                channels,
            )?;
        }
        Ok(Self {
            poles,
            coefficients,
            amplitude,
            training: source.clone(),
            report,
        })
    }

    /// Immutable termination, training-error, and dimension report.
    pub fn report(&self) -> &FitReport {
        &self.report
    }
    /// Shared poles in `s/(2π f_max)` coordinates (not physical rad/s).
    pub fn normalized_poles(&self) -> &[Complex] {
        &self.poles
    }
    /// Reference impedances snapshotted from the validated source at fitting time.
    pub fn reference_impedance(&self) -> &ReferenceImpedance {
        &self.training.reference_impedance
    }
    /// Current source warnings snapshotted at fitting time, unaffected by later edits.
    pub fn source_warnings(&self) -> &[TouchstoneWarning] {
        &self.training.warnings
    }

    /// Evaluate only within the closed training band. No automatic extrapolation.
    pub fn evaluate(&self, frequency_hz: f64) -> Result<SMatrix, FitError> {
        let [low, high] = self.report.band_hz;
        if !frequency_hz.is_finite() || frequency_hz < low || frequency_hz > high {
            return Err(FitError::InvalidInput(
                "evaluation must be finite and in training band",
            ));
        }
        let values = predict(
            frequency_hz / high,
            &self.poles,
            &self.coefficients,
            self.amplitude,
        )?;
        let rank = self.training.rank as usize;
        SMatrix::try_new(values.chunks(rank).map(|r| r.to_vec()).collect())
            .map_err(|_| FitError::Numerical)
    }

    /// Assess a separately supplied full S network at disjoint, in-band frequencies.
    /// Semantics, reference metadata, rank, and finite increasing grid must match.
    /// The RI baseline is evaluated on the immutable training snapshot via `sample_at`.
    /// Holdouts are never used for relocation, stopping, or seed selection.
    pub fn assess(&self, held_out: &Network) -> Result<HeldOutReport, FitError> {
        validate_source(held_out)?;
        if held_out.rank != self.training.rank
            || held_out.reference_impedance != self.training.reference_impedance
        {
            return Err(FitError::SourceSemantics(
                "held-out rank/reference mismatch",
            ));
        }
        let channels = (held_out.rank * held_out.rank) as usize;
        let mut model = Accumulators::new(channels, held_out.f.len());
        let mut linear = Accumulators::new(channels, held_out.f.len());
        for (index, &frequency) in held_out.f.iter().enumerate() {
            if self
                .training
                .f
                .binary_search_by(|f| {
                    // Signed zeros are the same physical frequency, not independent holdouts.
                    if *f == frequency {
                        std::cmp::Ordering::Equal
                    } else {
                        f.total_cmp(&frequency)
                    }
                })
                .is_ok()
            {
                return Err(FitError::InvalidInput(
                    "held-out frequency overlaps training",
                ));
            }
            let prediction = self.evaluate(frequency)?;
            let baseline = self
                .training
                .sample_at(frequency, Interpolation::Linear, Extrapolation::Error)
                .map_err(|_| FitError::Numerical)?;
            let truth = held_out
                .s_matrix_at(index)
                .map_err(|_| FitError::Numerical)?;
            for to in 1..=held_out.rank as usize {
                for from in 1..=held_out.rank as usize {
                    let channel = (to - 1) * held_out.rank as usize + from - 1;
                    let value = truth.get(to, from).map_err(|_| FitError::Numerical)?;
                    model.add(
                        channel,
                        prediction.get(to, from).map_err(|_| FitError::Numerical)?,
                        value,
                    )?;
                    linear.add(
                        channel,
                        baseline.s.get(to, from).map_err(|_| FitError::Numerical)?,
                        value,
                    )?;
                }
            }
        }
        Ok(HeldOutReport {
            points: held_out.f.len(),
            model: model.finish()?,
            ri_linear: linear.finish()?,
            source_warnings: held_out.warnings.clone(),
        })
    }
}

impl InitialPoles {
    fn checked(self) -> Result<Vec<Complex>, FitError> {
        let real = |re| Complex { re, im: 0.0 };
        let poles = match self {
            Self::OneReal(p) => vec![real(p)],
            Self::TwoReal(p, q) => vec![real(p), real(q)],
            Self::ConjugatePair { real, imaginary } => {
                if imaginary <= 0.0 {
                    return Err(FitError::InvalidPoles);
                }
                vec![
                    Complex {
                        re: real,
                        im: imaginary,
                    },
                    Complex {
                        re: real,
                        im: -imaginary,
                    },
                ]
            }
        };
        canonical(poles)
    }
}

fn canonical(mut poles: Vec<Complex>) -> Result<Vec<Complex>, FitError> {
    if poles
        .iter()
        .any(|p| !p.is_finite() || p.re >= -1e-10 || p.magnitude() > 1e4)
    {
        return Err(FitError::InvalidPoles);
    }
    if poles.len() == 2 {
        if (poles[0] - poles[1]).magnitude()
            < 1e-7 * 1.0_f64.max(poles[0].magnitude()).max(poles[1].magnitude())
        {
            return Err(FitError::InvalidPoles);
        }
        if poles[0].im == 0.0 && poles[1].im == 0.0 {
            poles.sort_by(|a, b| a.re.total_cmp(&b.re));
        } else if poles[0].re != poles[1].re || poles[0].im != -poles[1].im {
            return Err(FitError::InvalidPoles);
        } else if poles[0].im < 0.0 {
            poles.swap(0, 1);
        }
    }
    Ok(poles)
}

fn validate_source(n: &Network) -> Result<(), FitError> {
    if n.parameter != "S" || !(1..=2).contains(&n.rank) {
        return Err(FitError::SourceSemantics(
            "requires full single-ended S 1/2-port matrix",
        ));
    }
    if n.warnings.iter().any(|w| matches!(w, TouchstoneWarning::UnknownKeywordIgnored { keyword, .. } if keyword.eq_ignore_ascii_case("mixed mode order"))) {
        return Err(FitError::SourceSemantics("ignored mixed-mode declaration"));
    }
    let valid_ref = match &n.reference_impedance {
        ReferenceImpedance::Common(z) => z.is_finite() && *z > 0.0 && *z == n.z0,
        ReferenceImpedance::PerPort(v) => {
            v.len() == n.rank as usize
                && v.iter().all(|z| z.is_finite() && *z > 0.0)
                && v[0] == n.z0
        }
    };
    if !valid_ref {
        return Err(FitError::SourceSemantics(
            "invalid/inconsistent reference impedance",
        ));
    }
    if n.f.is_empty() || n.f.len() != n.s.len() {
        return Err(FitError::InvalidInput("empty/inconsistent source grid"));
    }
    for (i, point) in n.iter_points().enumerate() {
        let f = n.f[i];
        if !f.is_finite()
            || f < 0.0
            || (i > 0 && f <= n.f[i - 1])
            || point.frequency() != f
            || point.s().rank() != n.rank as usize
        {
            return Err(FitError::InvalidInput(
                "requires finite increasing frequencies and complete matrices",
            ));
        }
        for to in 1..=n.rank as usize {
            for from in 1..=n.rank as usize {
                if !point
                    .s()
                    .get(to, from)
                    .map_err(|_| FitError::Numerical)?
                    .is_finite()
                {
                    return Err(FitError::InvalidInput("nonfinite S data"));
                }
            }
        }
    }
    Ok(())
}

fn source_values(n: &Network) -> Result<Vec<Vec<Complex>>, FitError> {
    n.iter_points()
        .map(|point| {
            let mut row = Vec::with_capacity((n.rank * n.rank) as usize);
            for to in 1..=n.rank as usize {
                for from in 1..=n.rank as usize {
                    row.push(point.s().get(to, from).map_err(|_| FitError::Numerical)?);
                }
            }
            Ok(row)
        })
        .collect()
}

fn basis(x: f64, poles: &[Complex]) -> Vec<Complex> {
    let z = Complex { re: 0.0, im: x };
    let first = Complex::one() / (z - poles[0]);
    if poles.len() == 1 {
        return vec![first];
    }
    let second = Complex::one() / (z - poles[1]);
    if poles[0].im == 0.0 {
        vec![first, second]
    } else {
        vec![
            first + second,
            Complex { re: 0.0, im: 1.0 } * (first - second),
        ]
    }
}
fn linear_combination(phi: &[Complex], coefficients: &[f64]) -> Complex {
    phi.iter()
        .zip(coefficients)
        .fold(Complex::zero(), |sum, (p, c)| sum + *p * *c)
}

fn solve(
    xs: &[f64],
    ys: &[Vec<Complex>],
    poles: &[Complex],
    denominator: bool,
) -> Result<numerics::Solution, FitError> {
    let channels = ys[0].len();
    let order = poles.len();
    let columns = channels * (order + 1) + if denominator { order } else { 0 };
    let rows = xs.len() * channels * 2;
    let mut a = vec![0.0; rows * columns];
    let mut rhs = vec![0.0; rows];
    for (i, (&x, values)) in xs.iter().zip(ys).enumerate() {
        let phi = basis(x, poles);
        for (channel, &value) in values.iter().enumerate() {
            let row = 2 * (i * channels + channel);
            a[row * columns + channel * (order + 1)] = 1.0;
            for (k, p) in phi.iter().enumerate() {
                let column = channel * (order + 1) + k + 1;
                a[row * columns + column] = p.re;
                a[(row + 1) * columns + column] = p.im;
                if denominator {
                    let term = -(value * *p);
                    a[row * columns + columns - order + k] = term.re;
                    a[(row + 1) * columns + columns - order + k] = term.im;
                }
            }
            rhs[row] = value.re;
            rhs[row + 1] = value.im;
        }
    }
    numerics::least_squares(a, rhs, columns)
}

fn relocate(p: &[Complex], c: &[f64]) -> Result<(Vec<Complex>, usize), FitError> {
    let mut roots = if p.len() == 1 {
        vec![Complex {
            re: p[0].re - c[0],
            im: 0.0,
        }]
    } else if p[0].im == 0.0 {
        numerics::quadratic(
            -p[0].re - p[1].re + c[0] + c[1],
            p[0].re * p[1].re - c[0] * p[1].re - c[1] * p[0].re,
        )?
    } else {
        let (a, b) = (p[0].re, p[0].im);
        numerics::quadratic(
            -2.0 * a + 2.0 * c[0],
            a * a + b * b - 2.0 * a * c[0] - 2.0 * b * c[1],
        )?
    };
    let reflected = roots.iter().filter(|p| p.re > 0.0).count();
    for p in &mut roots {
        p.re = -p.re.abs();
    }
    Ok((canonical(roots)?, reflected))
}
fn pole_movement(old: &[Complex], new: &[Complex]) -> f64 {
    let distance = old
        .iter()
        .zip(new)
        .map(|(a, b)| (*a - *b).magnitude())
        .fold(0.0_f64, f64::max);
    let distance = if old.len() == 2 {
        distance.min(
            (old[0] - new[1])
                .magnitude()
                .max((old[1] - new[0]).magnitude()),
        )
    } else {
        distance
    };
    distance / old.iter().map(|p| p.magnitude()).fold(1.0_f64, f64::max)
}
fn predict(
    x: f64,
    poles: &[Complex],
    coeffs: &[Vec<f64>],
    amplitude: f64,
) -> Result<Vec<Complex>, FitError> {
    let phi = basis(x, poles);
    coeffs
        .iter()
        .map(|c| {
            let value =
                (Complex { re: c[0], im: 0.0 } + linear_combination(&phi, &c[1..])) * amplitude;
            if value.is_finite() {
                Ok(value)
            } else {
                Err(FitError::Numerical)
            }
        })
        .collect()
}
fn predict_grid(
    xs: &[f64],
    poles: &[Complex],
    coeffs: &[Vec<f64>],
    amplitude: f64,
) -> Result<Vec<Vec<Complex>>, FitError> {
    xs.iter()
        .map(|x| predict(*x, poles, coeffs, amplitude))
        .collect()
}

// LAPACK-style component norm: scale² * squares, never formed in physical units.
#[derive(Clone, Default)]
struct ScaledSquares {
    scale: f64,
    squares: f64,
}
impl ScaledSquares {
    fn add(&mut self, value: f64) {
        let value = value.abs();
        if value > self.scale {
            self.squares = 1.0 + self.squares * (self.scale / value).powi(2);
            self.scale = value;
        } else if value != 0.0 {
            self.squares += (value / self.scale).powi(2);
        }
    }
    fn rms(&self, count: f64) -> f64 {
        self.scale * (self.squares / count).sqrt()
    }
    fn relative_to(&self, reference: &Self) -> Option<f64> {
        if reference.scale == 0.0 {
            return None;
        }
        if self.scale == 0.0 {
            return Some(0.0);
        }
        // Split binary exponents so neither a norm nor an intermediate scale ratio
        // over/underflows before a representable final ratio can be formed.
        let (a, ae) = binary_parts(self.scale);
        let (b, be) = binary_parts(reference.scale);
        let mut fraction = (a / b) * (self.squares / reference.squares).sqrt();
        let mut exponent = ae - be;
        while fraction >= 2.0 {
            fraction *= 0.5;
            exponent += 1;
        }
        while fraction < 1.0 {
            fraction *= 2.0;
            exponent -= 1;
        }
        Some(if exponent > 1023 {
            f64::INFINITY
        } else if exponent < -1075 {
            0.0
        } else if exponent < -1022 {
            (fraction * 2.0_f64.powi(exponent + 1022)) * f64::MIN_POSITIVE
        } else {
            fraction * 2.0_f64.powi(exponent)
        })
    }
}
fn binary_parts(value: f64) -> (f64, i32) {
    if value < f64::MIN_POSITIVE {
        let (fraction, exponent) = binary_parts(value * 2.0_f64.powi(52));
        return (fraction, exponent - 52);
    }
    let bits = value.to_bits();
    let fraction = f64::from_bits((bits & ((1_u64 << 52) - 1)) | (1023_u64 << 52));
    (fraction, ((bits >> 52) as i32) - 1023)
}

#[derive(Clone, Default)]
struct ErrorSum {
    error: ScaledSquares,
    reference: ScaledSquares,
    max: f64,
}
impl ErrorSum {
    fn add(&mut self, predicted: Complex, truth: Complex) -> Result<(), FitError> {
        let delta = predicted - truth;
        let error = delta.magnitude();
        if !error.is_finite() || !truth.is_finite() {
            return Err(FitError::Numerical);
        }
        self.error.add(delta.re);
        self.error.add(delta.im);
        self.reference.add(truth.re);
        self.reference.add(truth.im);
        self.max = self.max.max(error);
        Ok(())
    }
    fn finish(self, count: f64) -> Result<ErrorMetrics, FitError> {
        let rms = self.error.rms(count);
        let relative_l2 = self.error.relative_to(&self.reference);
        if !rms.is_finite() || relative_l2.is_some_and(|r| !r.is_finite()) {
            return Err(FitError::Numerical);
        }
        Ok(ErrorMetrics {
            rms,
            max: self.max,
            relative_l2,
        })
    }
}
struct Accumulators {
    aggregate: ErrorSum,
    entries: Vec<ErrorSum>,
    points: f64,
    values: f64,
}
impl Accumulators {
    fn new(channels: usize, points: usize) -> Self {
        Self {
            aggregate: ErrorSum::default(),
            entries: vec![ErrorSum::default(); channels],
            points: points as f64,
            values: (points as f64) * (channels as f64),
        }
    }
    fn add(&mut self, channel: usize, predicted: Complex, truth: Complex) -> Result<(), FitError> {
        self.aggregate.add(predicted, truth)?;
        self.entries[channel].add(predicted, truth)
    }
    fn finish(self) -> Result<MatrixErrors, FitError> {
        Ok(MatrixErrors {
            aggregate: self.aggregate.finish(self.values)?,
            per_entry: self
                .entries
                .into_iter()
                .map(|sum| sum.finish(self.points))
                .collect::<Result<_, _>>()?,
        })
    }
}
fn measure(
    predicted: &[Vec<Complex>],
    truth: &[Vec<Complex>],
    channels: usize,
) -> Result<MatrixErrors, FitError> {
    let mut sums = Accumulators::new(channels, truth.len());
    for (p, t) in predicted.iter().zip(truth) {
        for (channel, (&p, &t)) in p.iter().zip(t).enumerate() {
            sums.add(channel, p, t)?;
        }
    }
    sums.finish()
}
