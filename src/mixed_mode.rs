//! Checked mixed-mode analysis of known single-ended S networks.
//!
//! This module does not interpret or emit mixed-mode Touchstone files.

use std::fmt;

use crate::{Complex, Network, ReferenceImpedance, SMatrix, TouchstoneError, TouchstoneWarning};

/// An ordered pair of 1-based single-ended RF ports defining differential polarity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortPair {
    /// Positive voltage terminal.
    pub positive: usize,
    /// Negative voltage terminal.
    pub negative: usize,
}

/// A row/column identity in an explicitly ordered mixed-mode basis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModePort {
    /// Difference waves for this ordered pair.
    Differential(PortPair),
    /// Sum waves for this ordered pair (same orientation as its differential mode).
    Common(PortPair),
    /// An unpaired single-ended port, using 1-based RF indexing.
    SingleEnded(usize),
}

/// Failure to establish or evaluate the checked mixed-mode contract.
#[derive(Debug)]
#[non_exhaustive]
pub enum MixedModeError {
    /// Invalid output ordering, coverage, pairing, or port bounds.
    InvalidLayout(&'static str),
    /// Public source rank disagrees with the layout.
    RankMismatch {
        /// Rank required by the layout.
        expected: usize,
        /// Rank declared by the source.
        actual: i32,
    },
    /// Source is not known single-ended S data (including an ignored mixed-mode declaration).
    UnsupportedSource(&'static str),
    /// Invalid, unequal, or unrepresentable reference impedance metadata.
    InvalidReference(&'static str),
    /// Source grid/matrix or computed output failed the existing numerical checks.
    Numerical(TouchstoneError),
}

impl fmt::Display for MixedModeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLayout(reason) => write!(f, "invalid mixed-mode layout: {reason}"),
            Self::RankMismatch { expected, actual } => write!(
                f,
                "mixed-mode rank mismatch: layout requires {expected}, source declares {actual}"
            ),
            Self::UnsupportedSource(reason) => write!(f, "unsupported mixed-mode source: {reason}"),
            Self::InvalidReference(reason) => write!(f, "invalid mixed-mode reference: {reason}"),
            Self::Numerical(error) => write!(f, "mixed-mode numerical validation: {error}"),
        }
    }
}

impl std::error::Error for MixedModeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Numerical(error) => Some(error),
            _ => None,
        }
    }
}

impl From<TouchstoneError> for MixedModeError {
    fn from(error: TouchstoneError) -> Self {
        Self::Numerical(error)
    }
}

/// Complete, checked ordering of output modes for a single-ended source rank.
///
/// Every pair must occur once as differential and once as common with identical
/// orientation. Pairs are disjoint; every remaining port occurs once as single-ended.
/// No implicit ordering or omitted ports are allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixedModeLayout {
    ports: Vec<ModePort>,
}

impl MixedModeLayout {
    /// Check an explicit output ordering against the single-ended source rank.
    pub fn new(rank: usize, ports: Vec<ModePort>) -> Result<Self, MixedModeError> {
        use MixedModeError::InvalidLayout;
        if rank == 0 || ports.len() != rank {
            return Err(InvalidLayout(
                "a nonempty ordering must contain exactly rank modes",
            ));
        }
        let mut owners = vec![None; rank];
        for (index, mode) in ports.iter().enumerate() {
            let in_range = |port: usize| port > 0 && port <= rank;
            match *mode {
                ModePort::SingleEnded(port) => {
                    if !in_range(port) {
                        return Err(InvalidLayout("single-ended port outside 1..=rank"));
                    }
                    if owners[port - 1].replace(index).is_some() {
                        return Err(InvalidLayout("overlapping or duplicate single-ended port"));
                    }
                }
                ModePort::Differential(pair) | ModePort::Common(pair) => {
                    if !in_range(pair.positive)
                        || !in_range(pair.negative)
                        || pair.positive == pair.negative
                    {
                        return Err(InvalidLayout(
                            "pair requires two distinct ports in 1..=rank",
                        ));
                    }
                    if ports[..index].contains(mode) {
                        return Err(InvalidLayout("duplicate mode"));
                    }
                    let partner = match mode {
                        ModePort::Differential(_) => ModePort::Common(pair),
                        _ => ModePort::Differential(pair),
                    };
                    if !ports.contains(&partner) {
                        return Err(InvalidLayout("pair requires differential and common modes with identical orientation"));
                    }
                    for port in [pair.positive, pair.negative] {
                        if let Some(owner) = owners[port - 1] {
                            if ports[owner] != partner {
                                return Err(InvalidLayout("pairs must be disjoint from other pairs and single-ended ports"));
                            }
                        } else {
                            owners[port - 1] = Some(index);
                        }
                    }
                }
            }
        }
        if owners.contains(&None) {
            return Err(InvalidLayout("every source port must be covered"));
        }
        Ok(Self { ports })
    }

    /// Output identities, in matrix row and column order.
    #[must_use]
    pub fn ports(&self) -> &[ModePort] {
        &self.ports
    }

    /// Number of single-ended source ports and output modes.
    #[must_use]
    pub fn rank(&self) -> usize {
        self.ports.len()
    }
}

/// Lazy mixed-mode view tied to an immutable, known single-ended S source.
///
/// Construction scans the source grid and matrices, but stores no transformed
/// matrices. Each read allocates one matrix and computes `T S T^T` in O(rank²).
/// Physical references are 2R (differential), R/2 (common), and R (unpaired).
/// There is deliberately no conversion to `Network` or Touchstone serialization.
/// See the repository's `docs/mixed-mode.md` for derivation and numerical limits.
#[derive(Debug)]
pub struct MixedModeView<'a> {
    source: &'a Network,
    layout: MixedModeLayout,
    references: Vec<f64>,
}

impl<'a> MixedModeView<'a> {
    /// Validate a single-ended source and bind its complete output ordering.
    ///
    /// Rejects non-S metadata and ignored `[Mixed-Mode Order]` declarations to
    /// prevent double transformation. Callers remain responsible for knowing the
    /// source is single-ended: missing/deleted declarations cannot be detected.
    pub fn new(source: &'a Network, layout: MixedModeLayout) -> Result<Self, MixedModeError> {
        if source.parameter != "S" {
            return Err(MixedModeError::UnsupportedSource(
                "requires S-parameter data",
            ));
        }
        if source.warnings.iter().any(|warning| matches!(warning,
            TouchstoneWarning::UnknownKeywordIgnored { keyword, .. }
                if keyword.replace('-', " ").split_whitespace().collect::<Vec<_>>().join(" ").eq_ignore_ascii_case("mixed mode order")
        )) {
            return Err(MixedModeError::UnsupportedSource("ignored Mixed-Mode Order declaration; input may already be mixed-mode"));
        }
        let rank = layout.rank();
        if usize::try_from(source.rank).ok() != Some(rank) {
            return Err(MixedModeError::RankMismatch {
                expected: rank,
                actual: source.rank,
            });
        }
        source.validate_frequency_data()?;
        for (index, point) in source.s.iter().enumerate() {
            if point.frequency != source.f[index] {
                return Err(TouchstoneError::FrequencyRowMismatch {
                    point_index: index,
                    vector_frequency: source.f[index],
                    row_frequency: point.frequency,
                }
                .into());
            }
            if point.s_ri.size() != rank {
                return Err(TouchstoneError::InvalidMatrixRank {
                    point_index: index,
                    matrix_rank: point.s_ri.size(),
                    expected_rank: rank,
                }
                .into());
            }
            crate::validate_sample_values(point)?;
        }
        let refs = match source.reference_impedance() {
            ReferenceImpedance::Common(value) => vec![value; rank],
            ReferenceImpedance::PerPort(values) => values,
        };
        if refs.len() != rank || refs.iter().any(|r| !r.is_finite() || *r <= 0.0) {
            return Err(MixedModeError::InvalidReference(
                "requires one positive finite real reference per source port",
            ));
        }
        let mut references = Vec::with_capacity(rank);
        for mode in layout.ports() {
            let value = match *mode {
                ModePort::SingleEnded(port) => refs[port - 1],
                ModePort::Differential(pair) | ModePort::Common(pair) => {
                    let r = refs[pair.positive - 1];
                    if r != refs[pair.negative - 1] {
                        return Err(MixedModeError::InvalidReference(
                            "references within each pair must be exactly equal",
                        ));
                    }
                    match mode {
                        ModePort::Differential(_) => 2.0 * r,
                        _ => r / 2.0,
                    }
                }
            };
            if !value.is_finite() || value <= 0.0 {
                return Err(MixedModeError::InvalidReference(
                    "modal reference overflow or underflow",
                ));
            }
            references.push(value);
        }
        Ok(Self {
            source,
            layout,
            references,
        })
    }

    /// Borrow the original source, including its unchanged warnings and metadata.
    #[must_use]
    pub fn source(&self) -> &'a Network {
        self.source
    }

    /// Complete ordered mode identities.
    #[must_use]
    pub fn layout(&self) -> &MixedModeLayout {
        &self.layout
    }

    /// Physical modal reference resistances in output order, in ohms.
    /// These are NOT a Touchstone `[Reference]` record.
    #[must_use]
    pub fn modal_references(&self) -> &[f64] {
        &self.references
    }

    /// Borrow the checked frequency grid in Hz.
    #[must_use]
    pub fn frequencies(&self) -> &'a [f64] {
        self.source.frequencies()
    }

    /// Compute one owned mixed-mode matrix at a 0-based frequency point index.
    ///
    /// Matrix port indexes are 1-based output positions from [`Self::layout`],
    /// not single-ended source port numbers. Non-finite results are errors.
    /// Keep the layout and modal references with this numeric snapshot: scalar
    /// S/Y/Z conversion or insertion into a `NetworkBuilder` would not preserve
    /// its generally different modal references and port identities.
    pub fn s_matrix_at(&self, point_index: usize) -> Result<SMatrix, MixedModeError> {
        let source = self.source.point_ref(point_index)?.s();
        let rank = self.layout.rank();
        let mut data = vec![vec![Complex::zero(); rank]; rank];
        for (row, to) in self.layout.ports().iter().enumerate() {
            for (col, from) in self.layout.ports().iter().enumerate() {
                let mut terms = [(Complex::zero(), 0.0); 4];
                let mut count = 0;
                for (i, a) in coefficients(*to) {
                    for (j, b) in coefficients(*from) {
                        if a != 0.0 && b != 0.0 {
                            // Two modal factors multiply to exactly ±1/2; do not
                            // round sqrt(1/2)² upward at the f64 overflow boundary.
                            let weight = if a.abs() != 1.0 && b.abs() != 1.0 {
                                0.5_f64.copysign(a * b)
                            } else {
                                a * b
                            };
                            terms[count] = (source.get(i, j)?, weight);
                            count += 1;
                        }
                    }
                }
                data[row][col] = Complex {
                    re: scaled_sum(&terms[..count], |z| z.re),
                    im: scaled_sum(&terms[..count], |z| z.im),
                };
            }
        }
        Ok(SMatrix::try_new(data)?)
    }

    /// Lazily traverse matrices; every traversal recomputes the transform.
    pub fn iter_matrices(
        &self,
    ) -> impl ExactSizeIterator<Item = Result<SMatrix, MixedModeError>> + DoubleEndedIterator + '_
    {
        (0..self.frequencies().len()).map(|index| self.s_matrix_at(index))
    }
}

fn coefficients(mode: ModePort) -> [(usize, f64); 2] {
    let k = std::f64::consts::FRAC_1_SQRT_2;
    match mode {
        ModePort::SingleEnded(port) => [(port, 1.0), (port, 0.0)],
        ModePort::Differential(pair) => [(pair.positive, k), (pair.negative, -k)],
        ModePort::Common(pair) => [(pair.positive, k), (pair.negative, k)],
    }
}

// Normalize before summing up to four sparse terms, so cancellable intermediate
// sums cannot overflow. Neumaier compensation retains small normalized terms
// across cancellation, but cannot recover terms lost to scaling underflow.
// The final rescaling can still overflow and is checked.
fn scaled_sum(terms: &[(Complex, f64)], component: impl Fn(Complex) -> f64) -> f64 {
    let scale = terms
        .iter()
        .map(|(z, _)| component(*z).abs())
        .fold(0.0, f64::max);
    if scale == 0.0 {
        return 0.0;
    }
    let mut sum: f64 = 0.0;
    let mut correction = 0.0;
    for (z, weight) in terms {
        let term = (component(*z) / scale) * weight;
        let next = sum + term;
        correction += if sum.abs() >= term.abs() {
            (sum - next) + term
        } else {
            (term - next) + sum
        };
        sum = next;
    }
    (sum + correction) * scale
}
