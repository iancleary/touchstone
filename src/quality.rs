use crate::{Complex, Network, ReferenceImpedance, TouchstoneError, TouchstoneWarning};

/// Explicit numerical policy for sampled quality reporting. No tolerance is implied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampledQualityOptions {
    /// Absolute dimensionless tolerance around the singular-value boundary of one.
    /// Must be finite and nonnegative; this is not a measurement uncertainty model.
    pub tolerance: f64,
}

/// Location of a sampled value relative to a boundary and its tolerance band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryClassification {
    /// Strictly below the lower edge of the tolerance band.
    BelowBoundary,
    /// Within the inclusive tolerance band; not a passivity certificate.
    NearBoundary,
    /// Strictly above the upper edge of the tolerance band.
    AboveBoundary,
}

/// A location in the original, unmodified sample grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampledQualityLocation {
    /// 0-based frequency-point index (not an RF port number).
    pub point_index: usize,
    /// Frequency in Hz; DC is allowed.
    pub frequency_hz: f64,
}

/// Quality diagnostics for one stored S matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampledQualityPoint {
    /// Location in the source network.
    pub location: SampledQualityLocation,
    /// Largest singular value, i.e. maximum output/input wave-amplitude ratio.
    pub sigma_max: f64,
    /// `max(sigma_max - 1, 0)`, without tolerance subtraction or data clipping.
    pub excess_above_one: f64,
    /// Classification of `sigma_max` relative to one using the configured tolerance.
    pub passivity: BoundaryClassification,
    /// Maximum `|Sij - Sji|`; zero for a one-port network.
    /// This is a symmetry diagnostic, not universal electromagnetic reciprocity.
    pub symmetry_residual: f64,
}

/// Opt-in diagnostics on a finite grid, not broadband passivity or causality certification.
#[derive(Debug, Clone, PartialEq)]
pub struct SampledQualityReport {
    /// Explicit policy used for this report.
    pub options: SampledQualityOptions,
    /// One result per stored point, in source order.
    pub points: Vec<SampledQualityPoint>,
    /// Location of the largest singular value; earliest point wins exact ties.
    pub worst_passivity: SampledQualityLocation,
    /// Independently selected largest symmetry residual; earliest point wins ties.
    pub worst_symmetry: SampledQualityLocation,
}

impl Network {
    /// Report sampled passivity and matrix symmetry without modifying data or warnings.
    ///
    /// Supports only single-ended one- and two-port S networks with positive finite
    /// real common or per-port references. Rejects unsupported mixed-mode declarations
    /// retained as parser warnings, invalid metadata, non-finite complex values, and
    /// nonempty-grid contract violations (frequencies must be finite, nonnegative,
    /// strictly increasing). Public rank metadata must match the stored matrices.
    /// Common references follow the existing scalar `z0` compatibility convention;
    /// per-port references must also agree with `z0` at the first port.
    ///
    /// The analytic scaled two-port calculation needs no numerical backend. N-port
    /// SVD, automatic repair, broadband certification, and causality are not provided.
    /// Unknown parser warnings other than mixed-mode declarations remain the caller's
    /// responsibility. Clearing warnings or relabeling metadata cannot convert a wave basis.
    ///
    /// ```
    /// use touchstone::{BoundaryClassification, Network, SampledQualityOptions};
    /// let network = Network::from_str("thru.s2p", "# Hz S RI R 50\n1 0 0 1 0 1 0 0 0\n")?;
    /// let report = network.sampled_quality(SampledQualityOptions { tolerance: 1e-12 })?;
    /// assert_eq!(report.points[0].passivity, BoundaryClassification::NearBoundary);
    /// # Ok::<(), touchstone::TouchstoneError>(())
    /// ```
    pub fn sampled_quality(
        &self,
        options: SampledQualityOptions,
    ) -> Result<SampledQualityReport, TouchstoneError> {
        let fail = |point_index, message| TouchstoneError::SampledQuality {
            source_name: self.name.clone(),
            point_index,
            message,
        };
        if !options.tolerance.is_finite() || options.tolerance < 0.0 {
            return Err(fail(
                None,
                "tolerance must be finite and nonnegative".into(),
            ));
        }
        if self.parameter != "S" {
            return Err(fail(
                None,
                format!("requires S parameters, found {}", self.parameter),
            ));
        }
        if !matches!(self.rank, 1 | 2) {
            return Err(fail(None, format!("only 1/2-port networks are supported, found rank {} (general N-port SVD is unavailable)", self.rank)));
        }
        if self.warnings.iter().any(|warning| {
            matches!(warning,
            TouchstoneWarning::UnknownKeywordIgnored { keyword, .. }
                if keyword.replace('-', " ").eq_ignore_ascii_case("mixed mode order"))
        }) {
            return Err(fail(None, "unsupported [Mixed-Mode Order] declaration; single-ended semantics are not established".into()));
        }
        crate::validate_reference_impedance(self.z0)
            .map_err(|error| fail(None, error.to_string()))?;
        match &self.reference_impedance {
            ReferenceImpedance::Common(value) => {
                crate::validate_reference_impedance(*value)
                    .map_err(|error| fail(None, error.to_string()))?;
            }
            ReferenceImpedance::PerPort(values) => {
                if values.len() != self.rank as usize {
                    return Err(fail(
                        None,
                        "per-port reference count must match rank".into(),
                    ));
                }
                for value in values {
                    crate::validate_reference_impedance(*value)
                        .map_err(|error| fail(None, error.to_string()))?;
                }
                if values[0] != self.z0 {
                    return Err(fail(
                        None,
                        "scalar z0 must agree with the first per-port reference".into(),
                    ));
                }
            }
        }
        self.validate_frequency_data()
            .map_err(|error| fail(None, error.to_string()))?;
        let mut points: Vec<SampledQualityPoint> = Vec::with_capacity(self.frequencies().len());
        let mut worst_passivity = 0;
        let mut worst_symmetry = 0;
        for (point_index, point) in self.iter_points().enumerate() {
            let frequency_hz = point.frequency();
            let point_fail =
                |message: &str| fail(Some(point_index), format!("{frequency_hz} Hz: {message}"));
            let matrix = point.s();
            if matrix.rank() != self.rank as usize {
                return Err(point_fail("stored matrix rank does not match network rank"));
            }
            if frequency_hz != self.frequencies()[point_index] {
                return Err(point_fail("stored point frequency does not match grid"));
            }
            let mut entries = [Complex::zero(); 4];
            for row in 1..=matrix.rank() {
                for col in 1..=matrix.rank() {
                    let value = matrix.get(row, col)?;
                    if !value.is_finite() {
                        return Err(point_fail(&format!("non-finite S({row},{col})")));
                    }
                    entries[(row - 1) * 2 + col - 1] = value;
                }
            }
            let sigma_max = if matrix.rank() == 1 {
                entries[0].magnitude()
            } else {
                sigma_max_2x2(entries)
            };
            let symmetry_residual = (entries[1] - entries[2]).magnitude();
            if !sigma_max.is_finite() || !symmetry_residual.is_finite() {
                return Err(point_fail(
                    "singular value or symmetry residual exceeds the finite f64 range",
                ));
            }
            let deviation = sigma_max - 1.0;
            let passivity = if deviation > options.tolerance {
                BoundaryClassification::AboveBoundary
            } else if deviation < -options.tolerance {
                BoundaryClassification::BelowBoundary
            } else {
                BoundaryClassification::NearBoundary
            };
            let location = SampledQualityLocation {
                point_index,
                frequency_hz,
            };
            let result = SampledQualityPoint {
                location,
                sigma_max,
                excess_above_one: deviation.max(0.0),
                passivity,
                symmetry_residual,
            };
            if point_index > 0 {
                if sigma_max > points[worst_passivity].sigma_max {
                    worst_passivity = point_index;
                }
                if symmetry_residual > points[worst_symmetry].symmetry_residual {
                    worst_symmetry = point_index;
                }
            }
            points.push(result);
        }
        Ok(SampledQualityReport {
            options,
            worst_passivity: points[worst_passivity].location,
            worst_symmetry: points[worst_symmetry].location,
            points,
        })
    }
}

fn sigma_max_2x2(entries: [Complex; 4]) -> f64 {
    let scale = entries.iter().fold(0.0_f64, |scale, value| {
        scale.max(value.re.abs()).max(value.im.abs())
    });
    if scale == 0.0 {
        return 0.0;
    }
    let [a, b, c, d] = entries.map(|v| Complex {
        re: v.re / scale,
        im: v.im / scale,
    });
    let norm_squared = |v: Complex| v.re * v.re + v.im * v.im;
    let x = norm_squared(a) + norm_squared(c);
    let y = norm_squared(b) + norm_squared(d);
    let z = Complex {
        re: a.re * b.re + a.im * b.im + c.re * d.re + c.im * d.im,
        im: a.re * b.im - a.im * b.re + c.re * d.im - c.im * d.re,
    };
    let lambda_max = 0.5 * (x + y + (x - y).hypot(2.0 * z.magnitude()));
    lambda_max.sqrt() * scale
}
