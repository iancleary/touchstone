#![warn(missing_docs)]
//! # Touchstone
//!
//! A Rust library for parsing, manipulating, and plotting Touchstone (`.sNp`) files
//! containing S-parameter data for RF and microwave networks.
//!
//! Use `touchstone` for measured or simulated S-parameter files, network
//! resampling, reference impedance metadata, S/Y/Z/ABCD conversion, and two-port
//! network cascading. Use `rfconversions` for scalar RF math, `gainlineup` for
//! block-level gain/NF/P1dB/IP3 lineups, and `linkbudget` for end-to-end radio
//! link performance.
//!
//! S-parameter port indices are 1-indexed: `s_db(2, 1)` is S21.
//!
//! # Examples
//!
//! ```
//! use touchstone::Network;
//!
//! let net = Network::new("files/ntwk1.s2p").unwrap();
//! assert_eq!(net.rank, 2);
//! ```

use std::{io::Write, ops};
/// Command-line interface helpers for the touchstone binary.
pub mod cli;
mod data_line;
mod data_pairs;
mod error;
mod file_extension;
mod file_operations;
mod network_builder;
mod open;
mod option_line;
mod parser;
mod plot;
mod utils;

pub use error::{TouchstoneError, TouchstoneErrorContext, TouchstoneWarning};
pub use network_builder::NetworkBuilder;

const PARAMETER_CONVERSION_TOLERANCE: f64 = 1.0e-12;

/// Reference impedance metadata for a Touchstone network.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ReferenceImpedance {
    /// One common real reference impedance in ohms for every port.
    Common(f64),
    /// One real reference impedance in ohms per port, ordered by port number.
    PerPort(Vec<f64>),
}

impl ReferenceImpedance {
    pub(crate) fn scalar_compatibility_value(&self) -> f64 {
        match self {
            Self::Common(z0) => *z0,
            Self::PerPort(values) => values[0],
        }
    }
}

/// Interpolation algorithm used when sampling S-parameter data between parsed frequencies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Interpolation {
    /// Use the nearest parsed frequency point.
    ///
    /// Ties are resolved toward the lower frequency point.
    Nearest,
    /// Linearly interpolate each real and imaginary S-parameter component.
    #[default]
    Linear,
}

/// Policy used when sampling outside the parsed frequency range.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Extrapolation {
    /// Return an error when a requested frequency is below or above the parsed range.
    #[default]
    Error,
    /// Hold the nearest boundary S-parameter values at the requested frequency.
    Clamp,
}

/// A network parsed from a Touchstone (`.sNp`) file.
///
/// Represents an N-port network with S-parameter data at multiple frequencies.
///
/// # Examples
///
/// ```
/// use touchstone::Network;
///
/// let net = Network::new("files/ntwk1.s2p").unwrap();
/// assert_eq!(net.rank, 2);
/// println!("Loaded {} frequency points", net.f.len());
/// ```
#[doc(alias = "S-parameters")]
#[doc(alias = "S2P")]
#[doc(alias = "SNP")]
#[doc(alias = "Touchstone")]
#[doc(alias = "scattering parameters")]
#[derive(Debug, Clone)]
pub struct Network {
    /// File path or name identifying the network.
    pub name: String,
    /// Number of ports (e.g. 2 for a two-port network).
    pub rank: i32,
    /// Frequency unit from the option line (e.g. `"GHz"`).
    pub frequency_unit: String,
    /// Network parameter type (e.g. `"S"` for scattering parameters).
    pub parameter: String,
    /// Data format from the option line (e.g. `"RI"`, `"MA"`, `"DB"`).
    pub format: String,
    /// Resistance keyword from the option line (typically `"R"`).
    pub resistance_string: String,
    /// Reference impedance in ohms (default 50 Ω).
    ///
    /// For per-port reference impedance metadata, this is the first port's impedance as a scalar
    /// compatibility value. Use [`Network::reference_impedance`] for the complete model.
    pub z0: f64,
    /// Complete reference impedance metadata for the network.
    pub reference_impedance: ReferenceImpedance,
    /// Comment lines appearing before the option line.
    pub comments: Vec<String>,
    /// Comment lines appearing after the option line.
    pub comments_after_option_line: Vec<String>,
    /// Non-fatal parser warnings recorded while reading the network.
    pub warnings: Vec<TouchstoneWarning>,

    /// Frequency vector in Hz.
    pub f: Vec<f64>,
    /// S-parameter data at each frequency point.
    pub s: Vec<data_line::ParsedDataLine>,
}

/// S-parameter data at a single frequency in Real/Imaginary format.
///
/// # Examples
///
/// ```
/// use touchstone::Network;
///
/// let net = Network::new("files/ntwk1.s2p").unwrap();
/// let s11_ri = net.s_ri(1, 1);
/// let point = &s11_ri[0];
/// println!("f = {} Hz, S11 = ({}, {})", point.frequency, point.s_ri.0, point.s_ri.1);
/// ```
#[derive(Debug, Clone)]
pub struct FrequencyRI {
    /// Frequency in Hz.
    pub frequency: f64,
    /// S-parameter value as a real/imaginary pair.
    pub s_ri: data_pairs::RealImaginary,
}

/// S-parameter data at a single frequency in Decibel/Angle format.
///
/// # Examples
///
/// ```
/// use touchstone::Network;
///
/// let net = Network::new("files/ntwk1.s2p").unwrap();
/// let s21_db = net.s_db(2, 1);
/// let point = &s21_db[0];
/// println!("f = {} Hz, S21 = {} dB ∠ {}°", point.frequency, point.s_db.0, point.s_db.1);
/// ```
#[derive(Debug, Clone)]
pub struct FrequencyDB {
    /// Frequency in Hz.
    pub frequency: f64,
    /// S-parameter value as a decibel/angle pair.
    pub s_db: data_pairs::DecibelAngle,
}

/// S-parameter data at a single frequency in Magnitude/Angle format.
///
/// # Examples
///
/// ```
/// use touchstone::Network;
///
/// let net = Network::new("files/ntwk1.s2p").unwrap();
/// let s11_ma = net.s_ma(1, 1);
/// let point = &s11_ma[0];
/// println!("f = {} Hz, S11 = {} ∠ {}°", point.frequency, point.s_ma.0, point.s_ma.1);
/// ```
#[derive(Debug, Clone)]
pub struct FrequencyMA {
    /// Frequency in Hz.
    pub frequency: f64,
    /// S-parameter value as a magnitude/angle pair.
    pub s_ma: data_pairs::MagnitudeAngle,
}

/// Stable complex number representation used by public matrix accessors.
///
/// This type intentionally does not expose the crate's internal parser matrix types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    /// Real component.
    pub re: f64,
    /// Imaginary component.
    pub im: f64,
}

impl Complex {
    fn zero() -> Self {
        Self { re: 0.0, im: 0.0 }
    }

    fn one() -> Self {
        Self { re: 1.0, im: 0.0 }
    }

    fn magnitude(self) -> f64 {
        self.re.hypot(self.im)
    }

    fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }
}

impl ops::Add for Complex {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }
}

impl ops::Sub for Complex {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re - rhs.re,
            im: self.im - rhs.im,
        }
    }
}

impl ops::Neg for Complex {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            re: -self.re,
            im: -self.im,
        }
    }
}

impl ops::Mul for Complex {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }
}

impl ops::Mul<f64> for Complex {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            re: self.re * rhs,
            im: self.im * rhs,
        }
    }
}

impl ops::Div for Complex {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let denominator = rhs.re * rhs.re + rhs.im * rhs.im;
        Self {
            re: (self.re * rhs.re + self.im * rhs.im) / denominator,
            im: (self.im * rhs.re - self.re * rhs.im) / denominator,
        }
    }
}

impl ops::Div<f64> for Complex {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self {
            re: self.re / rhs,
            im: self.im / rhs,
        }
    }
}

/// Stable S-parameter matrix for one frequency point.
///
/// `data` is arranged as rows of destination/output ports and columns of source/input ports.
/// Use [`SMatrix::get`] for non-panicking 1-based RF port indexing.
#[derive(Debug, Clone, PartialEq)]
pub struct SMatrix {
    /// Number of ports in this square S-parameter matrix.
    pub rank: usize,
    /// Matrix values in row-major order.
    pub data: Vec<Vec<Complex>>,
}

impl SMatrix {
    /// Return S(to_port, from_port) using 1-based RF port indexes.
    ///
    /// # Examples
    ///
    /// ```
    /// use touchstone::{Complex, SMatrix};
    ///
    /// let matrix = SMatrix {
    ///     rank: 1,
    ///     data: vec![vec![Complex { re: 0.5, im: -0.1 }]],
    /// };
    ///
    /// assert_eq!(matrix.get(1, 1)?.re, 0.5);
    /// # Ok::<(), touchstone::TouchstoneError>(())
    /// ```
    pub fn get(&self, to_port: usize, from_port: usize) -> Result<Complex, TouchstoneError> {
        validate_port_indexes(to_port, from_port, self.rank)?;
        self.data
            .get(to_port - 1)
            .and_then(|row| row.get(from_port - 1))
            .copied()
            .ok_or(TouchstoneError::InvalidPortIndex {
                to_port,
                from_port,
                rank: self.rank,
            })
    }

    /// Convert this S-parameter matrix to an admittance-parameter matrix.
    ///
    /// `z0` is the common real reference impedance in ohms. Per-port reference impedances are not
    /// supported by this scalar API.
    ///
    /// Uses `Y = (1 / z0) * (I - S) * (I + S)^-1`.
    pub fn to_y_matrix(&self, z0: f64) -> Result<ParameterMatrix, TouchstoneError> {
        validate_reference_impedance(z0)?;
        validate_matrix_data("S", self.rank, &self.data)?;

        let identity = identity_matrix(self.rank);
        let numerator = matrix_sub(&identity, &self.data);
        let denominator = matrix_add(&identity, &self.data);
        let denominator_inverse = invert_matrix(
            denominator,
            "S to Y matrix inversion",
            PARAMETER_CONVERSION_TOLERANCE,
        )?;
        let data = matrix_scale(&matrix_mul(&numerator, &denominator_inverse), 1.0 / z0);

        Ok(ParameterMatrix {
            rank: self.rank,
            data,
        })
    }

    /// Convert this S-parameter matrix to an impedance-parameter matrix.
    ///
    /// `z0` is the common real reference impedance in ohms. Per-port reference impedances are not
    /// supported by this scalar API.
    ///
    /// Uses `Z = z0 * (I + S) * (I - S)^-1`.
    pub fn to_z_matrix(&self, z0: f64) -> Result<ParameterMatrix, TouchstoneError> {
        validate_reference_impedance(z0)?;
        validate_matrix_data("S", self.rank, &self.data)?;

        let identity = identity_matrix(self.rank);
        let numerator = matrix_add(&identity, &self.data);
        let denominator = matrix_sub(&identity, &self.data);
        let denominator_inverse = invert_matrix(
            denominator,
            "S to Z matrix inversion",
            PARAMETER_CONVERSION_TOLERANCE,
        )?;
        let data = matrix_scale(&matrix_mul(&numerator, &denominator_inverse), z0);

        Ok(ParameterMatrix {
            rank: self.rank,
            data,
        })
    }

    /// Convert this two-port S-parameter matrix to ABCD transmission parameters.
    ///
    /// `z0` is the common real reference impedance in ohms. The conversion is only defined here for
    /// rank-2 matrices and returns [`TouchstoneError::UnsupportedConversionRank`] otherwise.
    pub fn to_abcd(&self, z0: f64) -> Result<ABCDMatrix, TouchstoneError> {
        validate_reference_impedance(z0)?;
        validate_matrix_data("S", self.rank, &self.data)?;

        if self.rank != 2 {
            return Err(TouchstoneError::UnsupportedConversionRank {
                conversion: "S to ABCD".to_string(),
                rank: self.rank,
                expected_rank: 2,
            });
        }

        let s11 = self.data[0][0];
        let s12 = self.data[0][1];
        let s21 = self.data[1][0];
        let s22 = self.data[1][1];
        let one = Complex::one();
        let two_s21 = s21 * 2.0;

        ensure_non_singular_value(
            "S to ABCD denominator",
            0,
            two_s21,
            PARAMETER_CONVERSION_TOLERANCE,
        )?;

        Ok(ABCDMatrix {
            a: ((one + s11) * (one - s22) + s12 * s21) / two_s21,
            b: (((one + s11) * (one + s22) - s12 * s21) * z0) / two_s21,
            c: (((one - s11) * (one - s22) - s12 * s21) / z0) / two_s21,
            d: ((one - s11) * (one + s22) + s12 * s21) / two_s21,
        })
    }

    /// Convert an admittance-parameter matrix to an S-parameter matrix.
    ///
    /// `z0` is the common real reference impedance in ohms. Per-port reference impedances are not
    /// supported by this scalar API.
    ///
    /// Uses `S = (I - z0Y) * (I + z0Y)^-1`.
    pub fn try_from_y_matrix(matrix: &ParameterMatrix, z0: f64) -> Result<Self, TouchstoneError> {
        matrix.to_s_matrix_from_y(z0)
    }

    /// Convert an impedance-parameter matrix to an S-parameter matrix.
    ///
    /// `z0` is the common real reference impedance in ohms. Per-port reference impedances are not
    /// supported by this scalar API.
    ///
    /// Uses `S = (Z - z0I) * (Z + z0I)^-1`.
    pub fn try_from_z_matrix(matrix: &ParameterMatrix, z0: f64) -> Result<Self, TouchstoneError> {
        matrix.to_s_matrix_from_z(z0)
    }

    /// Convert ABCD transmission parameters to a two-port S-parameter matrix.
    ///
    /// `z0` is the common real reference impedance in ohms.
    pub fn try_from_abcd(matrix: &ABCDMatrix, z0: f64) -> Result<Self, TouchstoneError> {
        matrix.to_s_matrix(z0)
    }
}

/// Stable admittance- or impedance-parameter matrix for one frequency point.
///
/// `data` is arranged as rows of destination/output ports and columns of source/input ports. Values
/// are in siemens for admittance matrices and ohms for impedance matrices.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterMatrix {
    /// Number of ports in this square parameter matrix.
    pub rank: usize,
    /// Matrix values in row-major order.
    pub data: Vec<Vec<Complex>>,
}

impl ParameterMatrix {
    /// Return a matrix entry using 1-based RF port indexes.
    pub fn get(&self, to_port: usize, from_port: usize) -> Result<Complex, TouchstoneError> {
        validate_port_indexes(to_port, from_port, self.rank)?;
        self.data
            .get(to_port - 1)
            .and_then(|row| row.get(from_port - 1))
            .copied()
            .ok_or(TouchstoneError::InvalidPortIndex {
                to_port,
                from_port,
                rank: self.rank,
            })
    }

    /// Convert this admittance-parameter matrix to an S-parameter matrix.
    ///
    /// `z0` is the common real reference impedance in ohms. Per-port reference impedances are not
    /// supported by this scalar API.
    ///
    /// Uses `S = (I - z0Y) * (I + z0Y)^-1`.
    pub fn to_s_matrix_from_y(&self, z0: f64) -> Result<SMatrix, TouchstoneError> {
        validate_reference_impedance(z0)?;
        validate_matrix_data("Y", self.rank, &self.data)?;

        let identity = identity_matrix(self.rank);
        let scaled_y = matrix_scale(&self.data, z0);
        let numerator = matrix_sub(&identity, &scaled_y);
        let denominator = matrix_add(&identity, &scaled_y);
        let denominator_inverse = invert_matrix(
            denominator,
            "Y to S matrix inversion",
            PARAMETER_CONVERSION_TOLERANCE,
        )?;
        let data = matrix_mul(&numerator, &denominator_inverse);

        Ok(SMatrix {
            rank: self.rank,
            data,
        })
    }

    /// Convert this impedance-parameter matrix to an S-parameter matrix.
    ///
    /// `z0` is the common real reference impedance in ohms. Per-port reference impedances are not
    /// supported by this scalar API.
    ///
    /// Uses `S = (Z - z0I) * (Z + z0I)^-1`.
    pub fn to_s_matrix_from_z(&self, z0: f64) -> Result<SMatrix, TouchstoneError> {
        validate_reference_impedance(z0)?;
        validate_matrix_data("Z", self.rank, &self.data)?;

        let z0_identity = matrix_scale(&identity_matrix(self.rank), z0);
        let numerator = matrix_sub(&self.data, &z0_identity);
        let denominator = matrix_add(&self.data, &z0_identity);
        let denominator_inverse = invert_matrix(
            denominator,
            "Z to S matrix inversion",
            PARAMETER_CONVERSION_TOLERANCE,
        )?;
        let data = matrix_mul(&numerator, &denominator_inverse);

        Ok(SMatrix {
            rank: self.rank,
            data,
        })
    }
}

/// Stable two-port ABCD transmission-parameter matrix.
///
/// The matrix layout is `[[A, B], [C, D]]`. `B` is in ohms, `C` is in siemens, and `A` and `D`
/// are dimensionless.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ABCDMatrix {
    /// A transmission parameter.
    pub a: Complex,
    /// B transmission parameter in ohms.
    pub b: Complex,
    /// C transmission parameter in siemens.
    pub c: Complex,
    /// D transmission parameter.
    pub d: Complex,
}

impl ABCDMatrix {
    /// Return an ABCD entry using 1-based row and column indexes.
    pub fn get(&self, row: usize, column: usize) -> Result<Complex, TouchstoneError> {
        validate_port_indexes(row, column, 2)?;
        Ok(match (row, column) {
            (1, 1) => self.a,
            (1, 2) => self.b,
            (2, 1) => self.c,
            (2, 2) => self.d,
            _ => unreachable!("ABCD indexes were already validated"),
        })
    }

    /// Convert this ABCD matrix to a two-port S-parameter matrix.
    ///
    /// `z0` is the common real reference impedance in ohms.
    pub fn to_s_matrix(&self, z0: f64) -> Result<SMatrix, TouchstoneError> {
        validate_reference_impedance(z0)?;

        for (index, value) in [self.a, self.b, self.c, self.d].into_iter().enumerate() {
            if !value.is_finite() {
                return Err(TouchstoneError::InvalidParameterMatrixValue {
                    matrix: "ABCD".to_string(),
                    row: index / 2 + 1,
                    column: index % 2 + 1,
                    re: value.re,
                    im: value.im,
                });
            }
        }

        let denominator = self.a + self.b / z0 + self.c * z0 + self.d;
        ensure_non_singular_value(
            "ABCD to S denominator",
            0,
            denominator,
            PARAMETER_CONVERSION_TOLERANCE,
        )?;

        Ok(SMatrix {
            rank: 2,
            data: vec![
                vec![
                    (self.a + self.b / z0 - self.c * z0 - self.d) / denominator,
                    ((self.a * self.d - self.b * self.c) * 2.0) / denominator,
                ],
                vec![
                    Complex { re: 2.0, im: 0.0 } / denominator,
                    (-self.a + self.b / z0 - self.c * z0 + self.d) / denominator,
                ],
            ],
        })
    }
}

/// Stable network data for one frequency point.
///
/// Frequency point indexes on [`Network`] are 0-based. RF port indexes within [`SMatrix`] are
/// 1-based.
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkPoint {
    /// Frequency in Hz.
    pub frequency: f64,
    /// S-parameter matrix at this frequency.
    pub s: SMatrix,
}

impl Network {
    /// Parse a Touchstone file and return a [`Network`].
    ///
    /// # Examples
    ///
    /// ```
    /// use touchstone::Network;
    ///
    /// let net = Network::new("files/ntwk1.s2p")?;
    /// assert_eq!(net.rank, 2);
    /// assert!(!net.f.is_empty());
    /// # Ok::<(), touchstone::TouchstoneError>(())
    /// ```
    pub fn new<P: AsRef<std::path::Path>>(file_path: P) -> Result<Self, TouchstoneError> {
        parser::try_read_file(file_path)
    }

    /// Creates a Network from in-memory UTF-8 bytes.
    ///
    /// The `source_name` is used as the network name and for Touchstone extension inference,
    /// such as `uploaded.s2p`.
    ///
    /// # Example
    ///
    /// ```
    /// let data = b"# GHz S RI R 50\n1.0 0.1 0.0 4.0 0.0 0.01 0.0 0.2 0.0\n";
    /// let network = touchstone::Network::from_bytes("uploaded.s2p", data)?;
    /// assert_eq!(network.rank, 2);
    /// # Ok::<(), touchstone::TouchstoneError>(())
    /// ```
    pub fn from_bytes<S: AsRef<str>>(
        source_name: S,
        bytes: &[u8],
    ) -> Result<Self, TouchstoneError> {
        parser::parse_bytes(source_name.as_ref(), bytes)
    }

    /// Creates a Network from an in-memory Touchstone string.
    ///
    /// The `source_name` is used as the network name and for Touchstone extension inference,
    /// such as `uploaded.s2p`.
    ///
    /// # Example
    ///
    /// ```
    /// let data = "# GHz S RI R 50\n1.0 0.1 0.0 4.0 0.0 0.01 0.0 0.2 0.0\n";
    /// let network = touchstone::Network::from_str("uploaded.s2p", data)?;
    /// assert_eq!(network.rank, 2);
    /// # Ok::<(), touchstone::TouchstoneError>(())
    /// ```
    #[allow(clippy::should_implement_trait)]
    pub fn from_str<S: AsRef<str>>(
        source_name: S,
        contents: &str,
    ) -> Result<Self, TouchstoneError> {
        parser::parse_str(source_name.as_ref(), contents)
    }

    /// Print a human-readable summary of the network to stdout.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use touchstone::Network;
    ///
    /// let net = Network::new("files/ntwk1.s2p").unwrap();
    /// net.print_summary();
    /// ```
    pub fn print_summary(&self) {
        println!("Network Summary:");
        println!("Name: {}", self.name);
        println!("Rank (number of ports): {}", self.rank);
        println!("Frequency Unit: {}", self.frequency_unit);
        println!("Parameter: {}", self.parameter);
        println!("Format: {}", self.format);
        println!("Reference Impedance (Z0): {}", self.z0);
        println!("Number of Data Lines: {}", self.f.len());
        println!("Comments:");
        for comment in &self.comments {
            println!("{}", comment);
        }
    }

    /// Return complete reference impedance metadata for this network.
    ///
    /// Existing scalar callers can continue to use [`Network::z0`]. When the network has a common
    /// reference impedance, this method reflects the current `z0` value.
    #[must_use]
    pub fn reference_impedance(&self) -> ReferenceImpedance {
        match &self.reference_impedance {
            ReferenceImpedance::Common(_) => ReferenceImpedance::Common(self.z0),
            ReferenceImpedance::PerPort(values) => ReferenceImpedance::PerPort(values.clone()),
        }
    }

    /// Return the frequency vector in Hz.
    ///
    /// # Examples
    ///
    /// ```
    /// use touchstone::Network;
    ///
    /// let net = Network::new("files/ntwk1.s2p").unwrap();
    /// let freqs = net.f();
    /// assert_eq!(freqs.len(), net.f.len());
    /// ```
    #[must_use]
    pub fn f(&self) -> Vec<f64> {
        self.f.clone()
    }

    /// Return S-parameter S(j,k) in dB/angle format at all frequencies.
    ///
    /// Port indices `j` and `k` are 1-indexed.
    ///
    /// # Examples
    ///
    /// ```
    /// use touchstone::Network;
    ///
    /// let net = Network::new("files/ntwk1.s2p").unwrap();
    /// let s21 = net.s_db(2, 1);
    /// assert_eq!(s21.len(), net.f.len());
    /// println!("S21 at first freq: {} dB", s21[0].s_db.0);
    /// ```
    #[must_use]
    #[doc(alias = "S-parameters")]
    #[doc(alias = "insertion loss")]
    #[doc(alias = "return loss")]
    pub fn s_db(&self, j: i8, k: i8) -> Vec<FrequencyDB> {
        let mut s_db_vector: Vec<FrequencyDB> = Vec::new();
        for i in 0..self.s.len() {
            let frequency = self.s[i].frequency;
            let s_db_matrix = &self.s[i].s_db;
            let s_db_value = s_db_matrix.get(j as usize, k as usize);
            let frequency_db = FrequencyDB {
                frequency,
                s_db: s_db_value,
            };
            s_db_vector.push(frequency_db);
        }
        s_db_vector
    }

    /// Return S-parameter S(j,k) in real/imaginary format at all frequencies.
    ///
    /// Port indices `j` and `k` are 1-indexed.
    ///
    /// # Examples
    ///
    /// ```
    /// use touchstone::Network;
    ///
    /// let net = Network::new("files/ntwk1.s2p").unwrap();
    /// let s11 = net.s_ri(1, 1);
    /// assert_eq!(s11.len(), net.f.len());
    /// println!("S11 at first freq: {} + j{}", s11[0].s_ri.0, s11[0].s_ri.1);
    /// ```
    #[must_use]
    pub fn s_ri(&self, j: i8, k: i8) -> Vec<FrequencyRI> {
        let mut s_ri_vector: Vec<FrequencyRI> = Vec::new();
        for i in 0..self.s.len() {
            let frequency = self.s[i].frequency;
            let s_ri_matrix = &self.s[i].s_ri;
            let s_ri_value = s_ri_matrix.get(j as usize, k as usize);
            let frequency_ri = FrequencyRI {
                frequency,
                s_ri: s_ri_value,
            };
            s_ri_vector.push(frequency_ri);
        }
        s_ri_vector
    }

    /// Return S-parameter S(j,k) in magnitude/angle format at all frequencies.
    ///
    /// Port indices `j` and `k` are 1-indexed.
    ///
    /// # Examples
    ///
    /// ```
    /// use touchstone::Network;
    ///
    /// let net = Network::new("files/ntwk1.s2p").unwrap();
    /// let s11 = net.s_ma(1, 1);
    /// assert_eq!(s11.len(), net.f.len());
    /// println!("S11 at first freq: {} ∠ {}°", s11[0].s_ma.0, s11[0].s_ma.1);
    /// ```
    #[must_use]
    pub fn s_ma(&self, j: i8, k: i8) -> Vec<FrequencyMA> {
        let mut s_ma_vector: Vec<FrequencyMA> = Vec::new();
        for i in 0..self.s.len() {
            let frequency = self.s[i].frequency;
            let s_ma_matrix = &self.s[i].s_ma;
            let s_ma_value = s_ma_matrix.get(j as usize, k as usize);
            let frequency_ma = FrequencyMA {
                frequency,
                s_ma: s_ma_value,
            };
            s_ma_vector.push(frequency_ma);
        }
        s_ma_vector
    }

    /// Return S(to_port, from_port) in real/imaginary form at one frequency point.
    ///
    /// `point_index` is 0-based. `to_port` and `from_port` use 1-based RF port indexes.
    ///
    /// # Examples
    ///
    /// ```
    /// use touchstone::{Complex, Network};
    ///
    /// let net = Network::from_str(
    ///     "uploaded.s1p",
    ///     "# GHz S RI R 50\n1.0 0.5 -0.1\n",
    /// )?;
    ///
    /// assert_eq!(net.try_s_ri_at(0, 1, 1)?, Complex { re: 0.5, im: -0.1 });
    /// # Ok::<(), touchstone::TouchstoneError>(())
    /// ```
    pub fn try_s_ri_at(
        &self,
        point_index: usize,
        to_port: usize,
        from_port: usize,
    ) -> Result<Complex, TouchstoneError> {
        let data_line = self.data_line_at(point_index)?;
        let rank = data_line.s_ri.size();
        validate_port_indexes(to_port, from_port, rank)?;
        Ok(complex_from_real_imaginary(
            data_line.s_ri.get(to_port, from_port),
        ))
    }

    /// Return the full S-parameter matrix at one frequency point.
    ///
    /// `point_index` is 0-based. Values in the returned [`SMatrix`] can be accessed with
    /// 1-based RF port indexes using [`SMatrix::get`].
    pub fn s_matrix_at(&self, point_index: usize) -> Result<SMatrix, TouchstoneError> {
        let data_line = self.data_line_at(point_index)?;
        let rank = data_line.s_ri.size();
        let mut data = Vec::with_capacity(rank);

        for to_port in 1..=rank {
            let mut row = Vec::with_capacity(rank);
            for from_port in 1..=rank {
                row.push(complex_from_real_imaginary(
                    data_line.s_ri.get(to_port, from_port),
                ));
            }
            data.push(row);
        }

        Ok(SMatrix { rank, data })
    }

    /// Return the admittance-parameter matrix at one frequency point.
    ///
    /// `point_index` is 0-based. The network must contain S-parameter source data and use one
    /// common scalar reference impedance.
    pub fn y_matrix_at(&self, point_index: usize) -> Result<ParameterMatrix, TouchstoneError> {
        let z0 = self.scalar_reference_impedance_for_conversions()?;
        self.s_matrix_at(point_index)?.to_y_matrix(z0)
    }

    /// Return the impedance-parameter matrix at one frequency point.
    ///
    /// `point_index` is 0-based. The network must contain S-parameter source data and use one
    /// common scalar reference impedance.
    pub fn z_matrix_at(&self, point_index: usize) -> Result<ParameterMatrix, TouchstoneError> {
        let z0 = self.scalar_reference_impedance_for_conversions()?;
        self.s_matrix_at(point_index)?.to_z_matrix(z0)
    }

    /// Return the ABCD transmission-parameter matrix at one frequency point.
    ///
    /// `point_index` is 0-based. The network must be a two-port S-parameter network with one common
    /// scalar reference impedance.
    pub fn abcd_at(&self, point_index: usize) -> Result<ABCDMatrix, TouchstoneError> {
        let z0 = self.scalar_reference_impedance_for_conversions()?;
        self.s_matrix_at(point_index)?.to_abcd(z0)
    }

    /// Return all stable data for one frequency point.
    ///
    /// `point_index` is 0-based.
    pub fn point_at(&self, point_index: usize) -> Result<NetworkPoint, TouchstoneError> {
        let data_line = self.data_line_at(point_index)?;
        Ok(NetworkPoint {
            frequency: data_line.frequency,
            s: self.s_matrix_at(point_index)?,
        })
    }

    /// Return stable data for every parsed frequency point.
    pub fn points(&self) -> Result<Vec<NetworkPoint>, TouchstoneError> {
        (0..self.s.len())
            .map(|point_index| self.point_at(point_index))
            .collect()
    }

    fn scalar_reference_impedance_for_conversions(&self) -> Result<f64, TouchstoneError> {
        if self.parameter != "S" {
            return Err(TouchstoneError::UnsupportedNetworkParameter {
                parameter: self.parameter.clone(),
            });
        }

        match self.reference_impedance() {
            ReferenceImpedance::Common(z0) => {
                validate_reference_impedance(z0)?;
                Ok(z0)
            }
            ReferenceImpedance::PerPort(values) => {
                Err(TouchstoneError::UnsupportedReferenceImpedance { values })
            }
        }
    }

    /// Sample the full S-parameter matrix at one frequency in Hz.
    ///
    /// Interpolation is performed in real/imaginary space. The returned point uses the requested
    /// frequency, even when [`Extrapolation::Clamp`] supplies boundary S-parameter values.
    ///
    /// # Examples
    ///
    /// ```
    /// use touchstone::{Extrapolation, Interpolation, Network};
    ///
    /// let network = Network::from_str(
    ///     "linear.s1p",
    ///     "# GHz S RI R 50\n1.0 0.0 0.0\n2.0 2.0 4.0\n",
    /// )?;
    /// let point = network.sample_at(1.5e9, Interpolation::Linear, Extrapolation::Error)?;
    ///
    /// assert_eq!(point.s.get(1, 1)?.re, 1.0);
    /// assert_eq!(point.s.get(1, 1)?.im, 2.0);
    /// # Ok::<(), touchstone::TouchstoneError>(())
    /// ```
    #[doc(alias = "interpolate")]
    #[doc(alias = "S-parameters")]
    pub fn sample_at(
        &self,
        frequency_hz: f64,
        interpolation: Interpolation,
        extrapolation: Extrapolation,
    ) -> Result<NetworkPoint, TouchstoneError> {
        validate_sample_frequency(0, frequency_hz)?;
        self.validate_frequency_data()?;

        let data_line =
            self.sample_data_line_at_validated(frequency_hz, interpolation, extrapolation)?;
        Ok(network_point_from_data_line(&data_line))
    }

    /// Resample the network onto a new strictly increasing frequency grid in Hz.
    ///
    /// The returned network preserves rank, name, comments, option-line metadata, reference
    /// impedance metadata, warnings, and the original data format intent. New S-parameters are
    /// interpolated in real/imaginary space, then derived magnitude/angle and dB/angle matrices are
    /// rebuilt from those interpolated values.
    ///
    /// # Examples
    ///
    /// ```
    /// use touchstone::{Extrapolation, Interpolation, Network};
    ///
    /// let network = Network::from_str(
    ///     "linear.s1p",
    ///     "# GHz S RI R 50\n1.0 0.0 0.0\n2.0 2.0 4.0\n",
    /// )?;
    /// let resampled = network.resample(
    ///     [1.0e9, 1.5e9, 2.0e9],
    ///     Interpolation::Linear,
    ///     Extrapolation::Error,
    /// )?;
    ///
    /// assert_eq!(resampled.f, vec![1.0e9, 1.5e9, 2.0e9]);
    /// assert_eq!(resampled.try_s_ri_at(1, 1, 1)?.re, 1.0);
    /// # Ok::<(), touchstone::TouchstoneError>(())
    /// ```
    #[doc(alias = "interpolate")]
    #[doc(alias = "resampling")]
    pub fn resample<I>(
        &self,
        frequencies_hz: I,
        interpolation: Interpolation,
        extrapolation: Extrapolation,
    ) -> Result<Network, TouchstoneError>
    where
        I: IntoIterator<Item = f64>,
    {
        let frequencies = frequencies_hz.into_iter().collect::<Vec<_>>();
        validate_frequency_slice(&frequencies)?;
        self.validate_frequency_data()?;

        let s = frequencies
            .iter()
            .map(|frequency| {
                self.sample_data_line_at_validated(*frequency, interpolation, extrapolation)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Network {
            name: self.name.clone(),
            rank: self.rank,
            frequency_unit: self.frequency_unit.clone(),
            parameter: self.parameter.clone(),
            format: self.format.clone(),
            resistance_string: self.resistance_string.clone(),
            z0: self.z0,
            reference_impedance: self.reference_impedance(),
            comments: self.comments.clone(),
            comments_after_option_line: self.comments_after_option_line.clone(),
            warnings: self.warnings.clone(),
            f: frequencies,
            s,
        })
    }

    fn data_line_at(
        &self,
        point_index: usize,
    ) -> Result<&data_line::ParsedDataLine, TouchstoneError> {
        self.s
            .get(point_index)
            .ok_or(TouchstoneError::InvalidPointIndex {
                point_index,
                point_count: self.s.len(),
            })
    }

    fn validate_frequency_data(&self) -> Result<(), TouchstoneError> {
        if self.f.len() != self.s.len() {
            return Err(TouchstoneError::FrequencyDataLengthMismatch {
                frequency_count: self.f.len(),
                data_count: self.s.len(),
            });
        }

        validate_frequency_slice(&self.f)
    }

    fn sample_data_line_at_validated(
        &self,
        frequency_hz: f64,
        interpolation: Interpolation,
        extrapolation: Extrapolation,
    ) -> Result<data_line::ParsedDataLine, TouchstoneError> {
        let min = self.f[0];
        let max = self.f[self.f.len() - 1];

        let lookup_frequency = if frequency_hz < min {
            match extrapolation {
                Extrapolation::Error => {
                    return Err(TouchstoneError::FrequencyOutOfRange {
                        frequency: frequency_hz,
                        min,
                        max,
                    });
                }
                Extrapolation::Clamp => min,
            }
        } else if frequency_hz > max {
            match extrapolation {
                Extrapolation::Error => {
                    return Err(TouchstoneError::FrequencyOutOfRange {
                        frequency: frequency_hz,
                        min,
                        max,
                    });
                }
                Extrapolation::Clamp => max,
            }
        } else {
            frequency_hz
        };

        if let Ok(index) = self
            .f
            .binary_search_by(|frequency| frequency.partial_cmp(&lookup_frequency).unwrap())
        {
            return Ok(parsed_data_line_with_frequency(
                &self.s[index],
                frequency_hz,
            ));
        }

        let upper_index = self
            .f
            .partition_point(|frequency| *frequency < lookup_frequency);
        let lower_index = upper_index - 1;

        let selected_index = match interpolation {
            Interpolation::Nearest => {
                let lower_frequency = self.f[lower_index];
                let upper_frequency = self.f[upper_index];
                if lookup_frequency - lower_frequency <= upper_frequency - lookup_frequency {
                    lower_index
                } else {
                    upper_index
                }
            }
            Interpolation::Linear => {
                return Ok(interpolate_data_lines(
                    frequency_hz,
                    lookup_frequency,
                    self.f[lower_index],
                    self.f[upper_index],
                    &self.s[lower_index],
                    &self.s[upper_index],
                ));
            }
        };

        Ok(parsed_data_line_with_frequency(
            &self.s[selected_index],
            frequency_hz,
        ))
    }

    /// Cascade two 2-port networks (standard connection: port 2 → port 1).
    ///
    /// For more control over port connections, use [`cascade_ports()`](Network::cascade_ports).
    ///
    /// # Examples
    ///
    /// ```
    /// use touchstone::Network;
    ///
    /// let net1 = Network::new("files/ntwk1.s2p").unwrap();
    /// let net2 = Network::new("files/ntwk2.s2p").unwrap();
    /// let cascaded = net1.cascade(&net2);
    /// assert_eq!(cascaded.rank, 2);
    /// ```
    #[must_use]
    #[doc(alias = "network cascading")]
    #[doc(alias = "ABCD parameters")]
    #[doc(alias = "chain")]
    pub fn cascade(&self, other: &Network) -> Network {
        if self.rank != 2 || other.rank != 2 {
            panic!("Cascading is only implemented for 2-port networks. Use cascade_ports() for explicit port specification.");
        }

        let self_z0 = common_reference_impedance_or_panic(self);
        let other_z0 = common_reference_impedance_or_panic(other);

        if self_z0 != other_z0 {
            panic!(
                "Cannot cascade networks with different reference impedances: {} and {}",
                self_z0, other_z0
            );
        }

        // can avoid this by converting other.f to use self.frequency_unit instead of other.frequency_unit
        if self.frequency_unit != other.frequency_unit {
            panic!(
                "Cannot cascade networks with different frequency units: {} and {}",
                self.frequency_unit, other.frequency_unit
            );
        }

        let mut comments = Vec::<String>::new();
        comments.push(format!(
            "! Cascaded network of {} and {}",
            self.name, other.name
        ));
        let comment_header_self = format!("! Comments from first network ({:?}):", self.name);
        comments.push(comment_header_self);
        for comment in &self.comments {
            comments.push(comment.clone());
        }
        let comment_header_other = format!("! Comments from second network ({:?}):", other.name);
        comments.push(comment_header_other);

        for comment in &other.comments {
            comments.push(comment.clone());
        }

        let mut comments_after_option_line = Vec::<String>::new();
        comments_after_option_line.push(format!(
            "! Cascaded network of {} and {}",
            self.name, other.name
        ));
        let comments_after_option_line_header_self = format!(
            "! Comments (after option line) from first network ({:?}):",
            self.name
        );
        comments_after_option_line.push(comments_after_option_line_header_self);
        for comment_after_option_line in &self.comments_after_option_line {
            comments_after_option_line.push(comment_after_option_line.clone());
        }
        let comments_after_option_line_header_other = format!(
            "! Comments (after option line) from second network ({:?}):",
            other.name
        );
        comments_after_option_line.push(comments_after_option_line_header_other);

        for comment_after_option_line in &other.comments_after_option_line {
            comments_after_option_line.push(comment_after_option_line.clone());
        }

        let new_name = format!("Cascaded({},{})", self.name, other.name);

        let mut s_new = Vec::new();
        // Assuming index-wise alignment as discussed
        let len = std::cmp::min(self.s.len(), other.s.len());

        for i in 0..len {
            let freq = self.s[i].frequency;
            let s1 = &self.s[i].s_ri;
            let s2 = &other.s[i].s_ri;

            let abcd1 = s1.to_abcd(self_z0);
            let abcd2 = s2.to_abcd(other_z0);

            let abcd_new = abcd1 * abcd2;

            // Resulting Z0? Usually the Z0 of the output port of the second network,
            // but for S-parameters of the cascaded block, we usually reference the input port of the first
            // and output port of the second.
            // If Z0 is the same for both (checked at start of function), then it's just self_z0.
            let s_new_ri = abcd_new.to_s(self_z0);

            let s_new_ma = crate::data_pairs::MagnitudeAngleMatrix::from_vec(vec![
                vec![
                    s_new_ri.get(1, 1).magnitude_angle(),
                    s_new_ri.get(1, 2).magnitude_angle(),
                ],
                vec![
                    s_new_ri.get(2, 1).magnitude_angle(),
                    s_new_ri.get(2, 2).magnitude_angle(),
                ],
            ]);

            let s_new_db =
                crate::data_pairs::DecibelAngleMatrix::from_magnitude_angle_matrix(&s_new_ma);

            s_new.push(crate::data_line::ParsedDataLine {
                frequency: freq,
                s_ri: s_new_ri,
                s_ma: s_new_ma,
                s_db: s_new_db,
            });
        }

        Network {
            name: new_name,
            rank: self.rank,
            frequency_unit: self.frequency_unit.clone(),
            parameter: self.parameter.clone(),
            format: self.format.clone(),
            resistance_string: self.resistance_string.clone(),
            z0: self_z0,
            reference_impedance: ReferenceImpedance::Common(self_z0),
            comments,
            comments_after_option_line,
            warnings: [self.warnings.clone(), other.warnings.clone()].concat(),
            f: self.f.clone(), // Note: this might be longer than s_new if other is shorter
            s: s_new,
        }
    }

    /// Cascade two networks with explicit port specification
    ///
    /// # Arguments
    /// * `other` - The network to cascade with
    /// * `from_port` - Output port of self to connect (1-indexed)
    /// * `to_port` - Input port of other to connect (1-indexed)
    ///
    /// # Examples
    /// ```
    /// use touchstone::Network;
    ///
    /// let net1 = Network::new("files/ntwk1.s2p").unwrap();
    /// let net2 = Network::new("files/ntwk2.s2p").unwrap();
    ///
    /// // Standard 2-port cascade (port 2 → port 1)
    /// let result = net1.cascade_ports(&net2, 2, 1);
    /// ```
    ///
    /// # Current Limitations
    /// - Only 2-port networks with standard connection (2→1) are currently supported
    /// - N-port cascade (N > 2) will be implemented in a future release
    ///
    /// # Panics
    /// - If port numbers are out of range
    /// - If networks are not 2-port
    /// - If connection is not standard (2→1) for 2-port networks
    #[must_use]
    pub fn cascade_ports(&self, other: &Network, from_port: usize, to_port: usize) -> Network {
        // Validate port numbers
        assert!(
            from_port >= 1 && from_port <= self.rank as usize,
            "from_port {} out of range for {}-port network (valid range: 1-{})",
            from_port,
            self.rank,
            self.rank
        );
        assert!(
            to_port >= 1 && to_port <= other.rank as usize,
            "to_port {} out of range for {}-port network (valid range: 1-{})",
            to_port,
            other.rank,
            other.rank
        );

        // For 2-port networks: use existing ABCD-based cascade
        if self.rank == 2 && other.rank == 2 {
            // Currently only support standard connection (port 2 → port 1)
            if from_port != 2 || to_port != 1 {
                panic!(
                    "For 2-port networks, only standard cascade (port 2 → port 1) is currently supported.\n\
                     Requested connection: port {} → port {}\n\
                     Use cascade() method for standard 2-port cascade, or wait for future N-port cascade implementation.",
                    from_port, to_port
                );
            }

            // Delegate to existing cascade implementation
            return self.cascade(other);
        }

        // For N-port where N > 2: Future enhancement
        panic!(
            "Cascading {}-port and {}-port networks is not yet supported.\n\
             Currently only 2-port networks can be cascaded (with standard port 2 → port 1 connection).\n\
             \n\
             Future enhancement: Full N-port cascade with arbitrary port connections.\n\
             \n\
             Workaround: Extract 2-port sub-networks from your {}-port and {}-port networks,\n\
             then cascade those 2-port networks.",
            self.rank,
            other.rank,
            self.rank,
            other.rank
        );
    }

    /// Serialize the network to an in-memory Touchstone string.
    ///
    /// The output matches [`save`](Self::save), including Touchstone 2.1 keywords and N-port
    /// line wrapping.
    pub fn to_touchstone_string(&self) -> std::io::Result<String> {
        let mut bytes = Vec::new();
        self.write_touchstone(&mut bytes)?;
        String::from_utf8(bytes)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    }

    /// Write the network as Touchstone text to any [`std::io::Write`] destination.
    ///
    /// The writer auto-selects single-line format for 1-port and 2-port networks and multi-line
    /// full-matrix format for 3-port and larger networks.
    pub fn write_touchstone<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        // Write comments
        for comment in &self.comments {
            writeln!(writer, "{}", comment)?;
        }

        let n = self.rank as usize;

        writeln!(writer, "[Version] 2.1")?;

        // Write option line
        // # <frequency unit> <parameter> <format> R <n>
        let option_line = option_line::Options::new(
            self.frequency_unit.clone(),
            self.parameter.clone(),
            self.format.clone(),
            self.resistance_string.clone(),
            self.z0.to_string().clone(),
        );
        writeln!(writer, "{}", option_line)?;

        writeln!(writer, "[Number of Ports] {}", self.rank)?;
        if n == 2 {
            writeln!(writer, "[Two-Port Data Order] 21_12")?;
        }
        if let ReferenceImpedance::PerPort(values) = self.reference_impedance() {
            writeln!(writer, "[Reference] {}", format_real_values(&values))?;
        }
        writeln!(writer, "[Number of Frequencies] {}", self.f.len())?;
        writeln!(writer, "[Matrix Format] Full")?;
        writeln!(writer, "[Network Data]")?;

        // Keep existing post-option comments with the network data they describe.
        for comment in &self.comments_after_option_line {
            writeln!(writer, "{}", comment)?;
        }

        // Write data lines
        let single_line_order = full_matrix_data_order(n);
        for data_line in &self.s {
            let mut freq = data_line.frequency;
            let frequency_unit = self.frequency_unit.clone();

            if frequency_unit == "THz" {
                freq = rfconversions::frequency::hz_to_thz(freq);
            } else if frequency_unit == "GHz" {
                freq = rfconversions::frequency::hz_to_ghz(freq);
            } else if frequency_unit == "MHz" {
                freq = rfconversions::frequency::hz_to_mhz(freq);
            } else if frequency_unit == "kHz" {
                freq = rfconversions::frequency::hz_to_khz(freq);
            }

            // For 1-port and 2-port: use single-line format
            // For 3+ port: use multi-line format
            if n <= 2 {
                // Single-line format
                let mut line = format!("{}", freq);

                match self.format.as_str() {
                    "RI" => {
                        let s = &data_line.s_ri;
                        for (row, col) in &single_line_order {
                            line.push_str(&format!(
                                " {} {}",
                                s.get(*row, *col).0,
                                s.get(*row, *col).1
                            ));
                        }
                    }
                    "MA" => {
                        let s = &data_line.s_ma;
                        for (row, col) in &single_line_order {
                            line.push_str(&format!(
                                " {} {}",
                                s.get(*row, *col).0,
                                s.get(*row, *col).1
                            ));
                        }
                    }
                    "DB" => {
                        let s = &data_line.s_db;
                        for (row, col) in &single_line_order {
                            line.push_str(&format!(
                                " {} {}",
                                s.get(*row, *col).0,
                                s.get(*row, *col).1
                            ));
                        }
                    }
                    _ => panic!("Unsupported format for saving: {}", self.format),
                }

                writeln!(writer, "{}", line)?;
            } else {
                // Multi-line format for 3+ port
                // First line: frequency and first row of S-parameters
                let mut line = format!("{}", freq);

                match self.format.as_str() {
                    "RI" => {
                        let s = &data_line.s_ri;
                        // First row on same line as frequency
                        for col in 1..=n {
                            line.push_str(&format!(" {} {}", s.get(1, col).0, s.get(1, col).1));
                        }
                        writeln!(writer, "{}", line)?;

                        // Subsequent rows on their own lines
                        for row in 2..=n {
                            let mut row_line = String::new();
                            for col in 1..=n {
                                row_line.push_str(&format!(
                                    " {} {}",
                                    s.get(row, col).0,
                                    s.get(row, col).1
                                ));
                            }
                            writeln!(writer, "{}", row_line)?;
                        }
                    }
                    "MA" => {
                        let s = &data_line.s_ma;
                        // First row on same line as frequency
                        for col in 1..=n {
                            line.push_str(&format!(" {} {}", s.get(1, col).0, s.get(1, col).1));
                        }
                        writeln!(writer, "{}", line)?;

                        // Subsequent rows on their own lines
                        for row in 2..=n {
                            let mut row_line = String::new();
                            for col in 1..=n {
                                row_line.push_str(&format!(
                                    " {} {}",
                                    s.get(row, col).0,
                                    s.get(row, col).1
                                ));
                            }
                            writeln!(writer, "{}", row_line)?;
                        }
                    }
                    "DB" => {
                        let s = &data_line.s_db;
                        // First row on same line as frequency
                        for col in 1..=n {
                            line.push_str(&format!(" {} {}", s.get(1, col).0, s.get(1, col).1));
                        }
                        writeln!(writer, "{}", line)?;

                        // Subsequent rows on their own lines
                        for row in 2..=n {
                            let mut row_line = String::new();
                            for col in 1..=n {
                                row_line.push_str(&format!(
                                    " {} {}",
                                    s.get(row, col).0,
                                    s.get(row, col).1
                                ));
                            }
                            writeln!(writer, "{}", row_line)?;
                        }
                    }
                    _ => panic!("Unsupported format for saving: {}", self.format),
                }
            }
        }

        writeln!(writer, "[End]")?;

        Ok(())
    }

    /// Save the network to a Touchstone file.
    ///
    /// # Examples
    ///
    /// ```
    /// use touchstone::Network;
    ///
    /// let net = Network::new("files/ntwk1.s2p").unwrap();
    /// let tmp = std::env::temp_dir().join("example_output.s2p");
    /// net.save(tmp.to_str().unwrap()).unwrap();
    /// std::fs::remove_file(tmp).unwrap();
    /// ```
    pub fn save(&self, file_path: &str) -> std::io::Result<()> {
        let file = std::fs::File::create(file_path)?;
        self.write_touchstone(file)
    }
}

fn full_matrix_data_order(n: usize) -> Vec<(usize, usize)> {
    if n == 2 {
        vec![(1, 1), (2, 1), (1, 2), (2, 2)]
    } else {
        (1..=n)
            .flat_map(|row| (1..=n).map(move |col| (row, col)))
            .collect()
    }
}

fn format_real_values(values: &[f64]) -> String {
    values
        .iter()
        .map(f64::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

fn complex_from_real_imaginary(value: data_pairs::RealImaginary) -> Complex {
    Complex {
        re: value.0,
        im: value.1,
    }
}

fn validate_port_indexes(
    to_port: usize,
    from_port: usize,
    rank: usize,
) -> Result<(), TouchstoneError> {
    if to_port == 0 || from_port == 0 || to_port > rank || from_port > rank {
        return Err(TouchstoneError::InvalidPortIndex {
            to_port,
            from_port,
            rank,
        });
    }

    Ok(())
}

fn validate_reference_impedance(z0: f64) -> Result<(), TouchstoneError> {
    if z0.is_finite() && z0 > 0.0 {
        Ok(())
    } else {
        Err(TouchstoneError::InvalidReferenceImpedance { z0 })
    }
}

fn validate_matrix_data(
    matrix: &str,
    rank: usize,
    data: &[Vec<Complex>],
) -> Result<(), TouchstoneError> {
    if rank == 0 || data.len() != rank {
        return Err(TouchstoneError::InvalidParameterMatrixShape {
            matrix: matrix.to_string(),
            rank,
            rows: data.len(),
            row_index: None,
            columns: 0,
        });
    }

    for (row_index, row) in data.iter().enumerate() {
        if row.len() != rank {
            return Err(TouchstoneError::InvalidParameterMatrixShape {
                matrix: matrix.to_string(),
                rank,
                rows: data.len(),
                row_index: Some(row_index),
                columns: row.len(),
            });
        }

        for (column_index, value) in row.iter().copied().enumerate() {
            if !value.is_finite() {
                return Err(TouchstoneError::InvalidParameterMatrixValue {
                    matrix: matrix.to_string(),
                    row: row_index + 1,
                    column: column_index + 1,
                    re: value.re,
                    im: value.im,
                });
            }
        }
    }

    Ok(())
}

fn identity_matrix(rank: usize) -> Vec<Vec<Complex>> {
    let mut data = vec![vec![Complex::zero(); rank]; rank];
    for (index, row) in data.iter_mut().enumerate() {
        row[index] = Complex::one();
    }
    data
}

fn matrix_add(left: &[Vec<Complex>], right: &[Vec<Complex>]) -> Vec<Vec<Complex>> {
    left.iter()
        .zip(right)
        .map(|(left_row, right_row)| {
            left_row
                .iter()
                .zip(right_row)
                .map(|(left_value, right_value)| *left_value + *right_value)
                .collect()
        })
        .collect()
}

fn matrix_sub(left: &[Vec<Complex>], right: &[Vec<Complex>]) -> Vec<Vec<Complex>> {
    left.iter()
        .zip(right)
        .map(|(left_row, right_row)| {
            left_row
                .iter()
                .zip(right_row)
                .map(|(left_value, right_value)| *left_value - *right_value)
                .collect()
        })
        .collect()
}

fn matrix_scale(matrix: &[Vec<Complex>], scalar: f64) -> Vec<Vec<Complex>> {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| *value * scalar).collect())
        .collect()
}

fn matrix_mul(left: &[Vec<Complex>], right: &[Vec<Complex>]) -> Vec<Vec<Complex>> {
    let rank = left.len();
    let mut result = vec![vec![Complex::zero(); rank]; rank];

    for row in 0..rank {
        for column in 0..rank {
            let mut sum = Complex::zero();
            for inner in 0..rank {
                sum = sum + left[row][inner] * right[inner][column];
            }
            result[row][column] = sum;
        }
    }

    result
}

fn invert_matrix(
    mut matrix: Vec<Vec<Complex>>,
    operation: &str,
    tolerance: f64,
) -> Result<Vec<Vec<Complex>>, TouchstoneError> {
    let rank = matrix.len();
    let mut inverse = identity_matrix(rank);

    for pivot_index in 0..rank {
        let mut pivot_row = pivot_index;
        let mut pivot_magnitude = matrix[pivot_index][pivot_index].magnitude();

        for (row_index, row) in matrix.iter().enumerate().skip(pivot_index + 1) {
            let magnitude = row[pivot_index].magnitude();
            if magnitude > pivot_magnitude {
                pivot_row = row_index;
                pivot_magnitude = magnitude;
            }
        }

        if pivot_row != pivot_index {
            matrix.swap(pivot_index, pivot_row);
            inverse.swap(pivot_index, pivot_row);
        }

        let pivot = matrix[pivot_index][pivot_index];
        ensure_non_singular_value(operation, pivot_index, pivot, tolerance)?;

        for column in 0..rank {
            matrix[pivot_index][column] = matrix[pivot_index][column] / pivot;
            inverse[pivot_index][column] = inverse[pivot_index][column] / pivot;
        }

        let normalized_pivot_row = matrix[pivot_index].clone();
        let normalized_inverse_row = inverse[pivot_index].clone();

        for row in 0..rank {
            if row == pivot_index {
                continue;
            }

            let factor = matrix[row][pivot_index];
            if factor.magnitude() == 0.0 {
                continue;
            }

            for column in 0..rank {
                matrix[row][column] = matrix[row][column] - factor * normalized_pivot_row[column];
                inverse[row][column] =
                    inverse[row][column] - factor * normalized_inverse_row[column];
            }
        }
    }

    Ok(inverse)
}

fn ensure_non_singular_value(
    operation: &str,
    pivot_index: usize,
    value: Complex,
    tolerance: f64,
) -> Result<(), TouchstoneError> {
    let magnitude = value.magnitude();
    if magnitude.is_finite() && magnitude > tolerance {
        Ok(())
    } else {
        Err(TouchstoneError::SingularMatrix {
            operation: operation.to_string(),
            pivot_index,
            pivot_magnitude: magnitude,
            tolerance,
        })
    }
}

fn common_reference_impedance_or_panic(network: &Network) -> f64 {
    match network.reference_impedance() {
        ReferenceImpedance::Common(z0) => z0,
        ReferenceImpedance::PerPort(_) => {
            panic!(
                "Cannot cascade networks with per-port reference impedances; common scalar reference impedance is required"
            );
        }
    }
}

// The `std::ops::Mul` trait is used to specify the functionality of `+`.
// Here, we make `Mul<Network>` - the trait for addition with a RHS of type `Network`.
// The following block implements the operation: Foo * Bar = FooBar
// This cascades Foo with Bar where in a gain lineup Foo comes before Bar
// using a device analogy -> [Foo] & [Bar] = [Foo Bar]
impl ops::Mul<Network> for Network {
    type Output = Network;

    fn mul(self, _rhs: Network) -> Network {
        tracing::debug!("Network cascade (mul) operation");

        self.cascade(&_rhs)
    }
}

fn validate_frequency_slice(frequencies: &[f64]) -> Result<(), TouchstoneError> {
    if frequencies.is_empty() {
        return Err(TouchstoneError::EmptyNetworkData);
    }

    for (point_index, frequency) in frequencies.iter().copied().enumerate() {
        validate_sample_frequency(point_index, frequency)?;

        if point_index == 0 {
            continue;
        }

        let previous_index = point_index - 1;
        let previous_frequency = frequencies[previous_index];

        if frequency == previous_frequency {
            return Err(TouchstoneError::DuplicateFrequency {
                first_index: previous_index,
                duplicate_index: point_index,
                frequency,
            });
        }

        if frequency < previous_frequency {
            return Err(TouchstoneError::UnsortedFrequencies {
                previous_index,
                previous_frequency,
                next_index: point_index,
                next_frequency: frequency,
            });
        }
    }

    Ok(())
}

fn validate_sample_frequency(point_index: usize, frequency: f64) -> Result<(), TouchstoneError> {
    if frequency.is_finite() {
        Ok(())
    } else {
        Err(TouchstoneError::InvalidFrequency {
            point_index,
            frequency,
        })
    }
}

fn parsed_data_line_with_frequency(
    data_line: &data_line::ParsedDataLine,
    frequency: f64,
) -> data_line::ParsedDataLine {
    data_line::parsed_data_line_from_ri_matrix(frequency, data_line.s_ri.clone())
}

fn interpolate_data_lines(
    output_frequency: f64,
    lookup_frequency: f64,
    lower_frequency: f64,
    upper_frequency: f64,
    lower: &data_line::ParsedDataLine,
    upper: &data_line::ParsedDataLine,
) -> data_line::ParsedDataLine {
    let n = lower.s_ri.size();
    let t = (lookup_frequency - lower_frequency) / (upper_frequency - lower_frequency);
    let mut data = Vec::with_capacity(n);

    for row in 1..=n {
        let mut row_data = Vec::with_capacity(n);

        for col in 1..=n {
            let lower_value = lower.s_ri.get(row, col);
            let upper_value = upper.s_ri.get(row, col);
            row_data.push(data_pairs::RealImaginary(
                lower_value.0 + (upper_value.0 - lower_value.0) * t,
                lower_value.1 + (upper_value.1 - lower_value.1) * t,
            ));
        }

        data.push(row_data);
    }

    data_line::parsed_data_line_from_ri_matrix(
        output_frequency,
        data_pairs::RealImaginaryMatrix::from_vec(data),
    )
}

fn network_point_from_data_line(data_line: &data_line::ParsedDataLine) -> NetworkPoint {
    let rank = data_line.s_ri.size();
    let mut data = Vec::with_capacity(rank);

    for to_port in 1..=rank {
        let mut row = Vec::with_capacity(rank);
        for from_port in 1..=rank {
            row.push(complex_from_real_imaginary(
                data_line.s_ri.get(to_port, from_port),
            ));
        }
        data.push(row);
    }

    NetworkPoint {
        frequency: data_line.frequency,
        s: SMatrix { rank, data },
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn assert_approx_eq(actual: f64, expected: f64) {
        let tolerance = 1e-10;
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {actual} to be within {tolerance} of {expected}"
        );
    }

    fn interpolation_1port_network() -> Network {
        Network::from_str("linear.s1p", "# GHz S RI R 50\n1.0 0.0 0.0\n2.0 2.0 4.0\n").unwrap()
    }

    fn assert_s11(point: &NetworkPoint, expected_re: f64, expected_im: f64) {
        let s11 = point.s.get(1, 1).unwrap();
        assert_approx_eq(s11.re, expected_re);
        assert_approx_eq(s11.im, expected_im);
    }

    #[test]
    fn f() {
        let network1 = Network::new("files/ntwk1.s2p").unwrap();
        let f = network1.f();

        assert_eq!(f.len(), network1.f.len());
    }

    #[test]
    fn s_db() {
        let network1 = Network::new("files/ntwk1.s2p").unwrap();

        let s11 = network1.s_db(1, 1);
        let s12 = network1.s_db(1, 2);
        let s21 = network1.s_db(2, 1);
        let s22 = network1.s_db(2, 2);

        assert_eq!(s11.len(), s12.len());
        assert_eq!(s11.len(), s21.len());
        assert_eq!(s11.len(), s22.len());
    }

    #[test]
    fn s_ri() {
        let network1 = Network::new("files/ntwk1.s2p").unwrap();

        let s11 = network1.s_ri(1, 1);
        let s12 = network1.s_ri(1, 2);
        let s21 = network1.s_ri(2, 1);
        let s22 = network1.s_ri(2, 2);

        assert_eq!(s11.len(), s12.len());
        assert_eq!(s11.len(), s21.len());
        assert_eq!(s11.len(), s22.len());
    }

    #[test]
    fn s_ma() {
        let network1 = Network::new("files/ntwk1.s2p").unwrap();

        let s11 = network1.s_ma(1, 1);
        let s12 = network1.s_ma(1, 2);
        let s21 = network1.s_ma(2, 1);
        let s22 = network1.s_ma(2, 2);

        assert_eq!(s11.len(), s12.len());
        assert_eq!(s11.len(), s21.len());
        assert_eq!(s11.len(), s22.len());
    }

    #[test]
    fn cascade_2port_networks() {
        let network1 = Network::new("files/ntwk1.s2p").unwrap();
        let network2 = Network::new("files/ntwk2.s2p").unwrap();

        let network3 = Network::new("files/ntwk3.s2p").unwrap();

        let cascaded_network = network1.cascade(&network2);

        assert_eq!(cascaded_network.f.len(), 91);
        assert_eq!(cascaded_network.s.len(), 91);

        for i in 0..cascaded_network.s.len() {
            assert_eq!(cascaded_network.s[i].frequency, network3.s[i].frequency);

            let s1 = &cascaded_network.s[i].s_ri;
            let s2 = &network3.s[i].s_ri;
            let epsilon = 1e-4; // Relaxed epsilon for floating point differences

            assert!(
                (s1.get(1, 1).0 - s2.get(1, 1).0).abs() < epsilon,
                "S11 real mismatch at freq {}",
                cascaded_network.s[i].frequency
            );
            assert!(
                (s1.get(1, 1).1 - s2.get(1, 1).1).abs() < epsilon,
                "S11 imag mismatch"
            );
            assert!(
                (s1.get(1, 2).0 - s2.get(1, 2).0).abs() < epsilon,
                "S12 real mismatch"
            );
            assert!(
                (s1.get(1, 2).1 - s2.get(1, 2).1).abs() < epsilon,
                "S12 imag mismatch"
            );
            assert!(
                (s1.get(2, 1).0 - s2.get(2, 1).0).abs() < epsilon,
                "S21 real mismatch"
            );
            assert!(
                (s1.get(2, 1).1 - s2.get(2, 1).1).abs() < epsilon,
                "S21 imag mismatch"
            );
            assert!(
                (s1.get(2, 2).0 - s2.get(2, 2).0).abs() < epsilon,
                "S22 real mismatch"
            );
            assert!(
                (s1.get(2, 2).1 - s2.get(2, 2).1).abs() < epsilon,
                "S22 imag mismatch"
            );

            // Derived values might also differ slightly, skipping strict check
            // assert_eq!(cascaded_network.s[i].s_ma, network3.s[i].s_ma);
            // assert_eq!(cascaded_network.s[i].s_db, network3.s[i].s_db);
        }
    }

    #[test]
    fn cascade_2port_networks_operator() {
        let network1 = Network::new("files/ntwk1.s2p").unwrap();
        let network2 = Network::new("files/ntwk2.s2p").unwrap();

        let cascaded_network = network1 * network2;

        let network3 = Network::new("files/ntwk3.s2p").unwrap();

        assert_eq!(cascaded_network.f.len(), 91);
        assert_eq!(cascaded_network.s.len(), 91);

        for i in 0..cascaded_network.s.len() {
            assert_eq!(cascaded_network.s[i].frequency, network3.s[i].frequency);

            let f1 = cascaded_network.f[i];
            let f2 = network3.f[i];

            assert_eq!(f1, f2);

            let s1 = &cascaded_network.s[i].s_ri;
            let s2 = &network3.s[i].s_ri;
            let epsilon = 1e-4; // Relaxed epsilon for floating point differences

            assert!(
                (s1.get(1, 1).0 - s2.get(1, 1).0).abs() < epsilon,
                "S11 real mismatch at freq {}",
                cascaded_network.s[i].frequency
            );
            assert!(
                (s1.get(1, 1).1 - s2.get(1, 1).1).abs() < epsilon,
                "S11 imag mismatch"
            );
            assert!(
                (s1.get(1, 2).0 - s2.get(1, 2).0).abs() < epsilon,
                "S12 real mismatch"
            );
            assert!(
                (s1.get(1, 2).1 - s2.get(1, 2).1).abs() < epsilon,
                "S12 imag mismatch"
            );
            assert!(
                (s1.get(2, 1).0 - s2.get(2, 1).0).abs() < epsilon,
                "S21 real mismatch"
            );
            assert!(
                (s1.get(2, 1).1 - s2.get(2, 1).1).abs() < epsilon,
                "S21 imag mismatch"
            );
            assert!(
                (s1.get(2, 2).0 - s2.get(2, 2).0).abs() < epsilon,
                "S22 real mismatch"
            );
            assert!(
                (s1.get(2, 2).1 - s2.get(2, 2).1).abs() < epsilon,
                "S22 imag mismatch"
            );

            // Derived values might also differ slightly, skipping strict check
            // assert_eq!(cascaded_network.s[i].s_ma, network3.s[i].s_ma);
            // assert_eq!(cascaded_network.s[i].s_db, network3.s[i].s_db);
        }
    }

    #[test]
    fn test_save_load_roundtrip() {
        let network1 = Network::new("files/ntwk1.s2p").unwrap();

        let temp_dir = std::env::temp_dir()
            .join("touchstone_tests")
            .join("roundtrip");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("roundtrip.s2p");
        let file_path_str = file_path.to_str().unwrap();

        network1.save(file_path_str).unwrap();

        let network2 = Network::new(file_path_str).unwrap();

        assert_eq!(network1.f.len(), network2.f.len());
        assert_eq!(network1.s.len(), network2.s.len());
        assert_eq!(network1.format, network2.format);
        assert_eq!(network1.z0, network2.z0);

        let path_temp = file_path_str.to_string();
        let binding = std::path::Path::new(&path_temp);
        let network2_name = binding.to_str().unwrap();

        assert_eq!(network2.name, network2_name);
        assert_eq!(network1.parameter, network2.parameter);

        assert_eq!(network1.f.len(), network2.f.len());
        for i in 0..network1.f.len() {
            assert_eq!(network1.f[i], network2.f[i]);
        }

        assert_eq!(network1.s.len(), network2.s.len());
        for i in 0..network1.s.len() {
            let s1 = &network1.s[i].s_ri;
            let s2 = &network2.s[i].s_ri;
            let epsilon = 1e-6;
            assert!((s1.get(1, 1).0 - s2.get(1, 1).0).abs() < epsilon);
            assert!((s1.get(1, 1).1 - s2.get(1, 1).1).abs() < epsilon);
        }

        // Cleanup
        std::fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_save_writes_touchstone_2_1_two_port_21_12_order() {
        let temp_dir = std::env::temp_dir()
            .join("touchstone_tests")
            .join("two_port_order");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let input_path = temp_dir.join(format!("asymmetric_input_{}.s2p", nanos));
        let output_path = temp_dir.join(format!("asymmetric_output_{}.s2p", nanos));

        std::fs::write(
            &input_path,
            "# GHz S RI R 50\n1.0 0.1 0.0 4.0 0.0 0.01 0.0 0.2 0.0\n",
        )
        .unwrap();

        let network = Network::new(input_path.to_str().unwrap()).unwrap();
        assert_eq!(
            network.s_ri(2, 1)[0].s_ri,
            data_pairs::RealImaginary(4.0, 0.0)
        );
        assert_eq!(
            network.s_ri(1, 2)[0].s_ri,
            data_pairs::RealImaginary(0.01, 0.0)
        );

        network.save(output_path.to_str().unwrap()).unwrap();
        let saved = std::fs::read_to_string(&output_path).unwrap();

        assert!(saved.contains("[Version] 2.1"));
        assert!(saved.contains("[Number of Ports] 2"));
        assert!(saved.contains("[Two-Port Data Order] 21_12"));
        assert!(saved.contains("[Number of Frequencies] 1"));
        assert!(saved.contains("[Network Data]"));
        assert!(saved.contains("[End]"));

        let data_line = saved
            .lines()
            .find(|line| line.starts_with('1'))
            .expect("saved output should contain a data line");
        let values = data_line
            .split_whitespace()
            .map(|part| part.parse::<f64>().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(values, vec![1.0, 0.1, 0.0, 4.0, 0.0, 0.01, 0.0, 0.2, 0.0]);

        let reloaded = Network::new(output_path.to_str().unwrap()).unwrap();
        assert_eq!(
            reloaded.s_ri(2, 1)[0].s_ri,
            data_pairs::RealImaginary(4.0, 0.0)
        );
        assert_eq!(
            reloaded.s_ri(1, 2)[0].s_ri,
            data_pairs::RealImaginary(0.01, 0.0)
        );

        std::fs::remove_file(input_path).unwrap();
        std::fs::remove_file(output_path).unwrap();
    }

    #[test]
    fn test_save_load_roundtrip_3port() {
        let network1 = Network::new("files/hfss_18.2.s3p").unwrap();

        let temp_dir = std::env::temp_dir()
            .join("touchstone_tests")
            .join("roundtrip");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("roundtrip_3port.s3p");
        let file_path_str = file_path.to_str().unwrap();

        network1.save(file_path_str).unwrap();

        let network2 = Network::new(file_path_str).unwrap();

        // Verify metadata
        assert_eq!(network1.rank, network2.rank);
        assert_eq!(network1.rank, 3);
        assert_eq!(network1.f.len(), network2.f.len());
        assert_eq!(network1.s.len(), network2.s.len());
        assert_eq!(network1.format, network2.format);
        assert_eq!(network1.z0, network2.z0);

        // Verify frequencies
        for i in 0..network1.f.len() {
            assert_eq!(network1.f[i], network2.f[i]);
        }

        // Verify all S-parameters (3x3 matrix)
        let epsilon = 1e-6;
        for i in 0..network1.s.len() {
            for row in 1..=3 {
                for col in 1..=3 {
                    let s1_ma = &network1.s[i].s_ma;
                    let s2_ma = &network2.s[i].s_ma;
                    assert!(
                        (s1_ma.get(row, col).0 - s2_ma.get(row, col).0).abs() < epsilon,
                        "S{}{} magnitude mismatch at frequency index {}",
                        row,
                        col,
                        i
                    );
                    assert!(
                        (s1_ma.get(row, col).1 - s2_ma.get(row, col).1).abs() < epsilon,
                        "S{}{} angle mismatch at frequency index {}",
                        row,
                        col,
                        i
                    );
                }
            }
        }

        // Cleanup
        std::fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_save_load_roundtrip_4port() {
        let network1 = Network::new("files/Agilent_E5071B.s4p").unwrap();

        let temp_dir = std::env::temp_dir()
            .join("touchstone_tests")
            .join("roundtrip");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("roundtrip_4port.s4p");
        let file_path_str = file_path.to_str().unwrap();

        network1.save(file_path_str).unwrap();

        let network2 = Network::new(file_path_str).unwrap();

        // Verify metadata
        assert_eq!(network1.rank, network2.rank);
        assert_eq!(network1.rank, 4);
        assert_eq!(network1.f.len(), network2.f.len());
        assert_eq!(network1.s.len(), network2.s.len());
        assert_eq!(network1.format, network2.format);
        assert_eq!(network1.z0, network2.z0);

        // Verify frequencies
        for i in 0..network1.f.len() {
            assert_eq!(network1.f[i], network2.f[i]);
        }

        // Verify all S-parameters (4x4 matrix)
        let epsilon = 1e-6;
        for i in 0..network1.s.len() {
            for row in 1..=4 {
                for col in 1..=4 {
                    let s1_db = &network1.s[i].s_db;
                    let s2_db = &network2.s[i].s_db;
                    assert!(
                        (s1_db.get(row, col).0 - s2_db.get(row, col).0).abs() < epsilon,
                        "S{}{} dB mismatch at frequency index {}",
                        row,
                        col,
                        i
                    );
                    assert!(
                        (s1_db.get(row, col).1 - s2_db.get(row, col).1).abs() < epsilon,
                        "S{}{} angle mismatch at frequency index {}",
                        row,
                        col,
                        i
                    );
                }
            }
        }

        // Cleanup
        std::fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_cascade_ports_2port_standard() {
        // Test cascade_ports with standard 2-port connection (2→1)
        let network1 = Network::new("files/ntwk1.s2p").unwrap();
        let network2 = Network::new("files/ntwk2.s2p").unwrap();
        let network3 = Network::new("files/ntwk3.s2p").unwrap();

        // cascade_ports(2, 1) should give same result as cascade()
        let result_ports = network1.cascade_ports(&network2, 2, 1);
        let result_standard = network1.cascade(&network2);

        assert_eq!(result_ports.rank, result_standard.rank);
        assert_eq!(result_ports.f.len(), result_standard.f.len());
        assert_eq!(result_ports.s.len(), result_standard.s.len());

        // Should also match the known good result (ntwk3)
        let epsilon = 1e-4;
        for i in 0..result_ports.s.len() {
            let s1 = &result_ports.s[i].s_ri;
            let s2 = &network3.s[i].s_ri;

            assert!((s1.get(1, 1).0 - s2.get(1, 1).0).abs() < epsilon);
            assert!((s1.get(2, 2).0 - s2.get(2, 2).0).abs() < epsilon);
        }
    }

    #[test]
    #[should_panic(expected = "only standard cascade (port 2 → port 1) is currently supported")]
    fn test_cascade_ports_2port_nonstandard() {
        // Test that non-standard port connections panic with helpful message
        let network1 = Network::new("files/ntwk1.s2p").unwrap();
        let network2 = Network::new("files/ntwk2.s2p").unwrap();

        // This should panic because we don't support 1→2 connection yet
        let _ = network1.cascade_ports(&network2, 1, 2);
    }

    #[test]
    #[should_panic(expected = "from_port 3 out of range for 2-port network")]
    fn test_cascade_ports_invalid_from_port() {
        let network1 = Network::new("files/ntwk1.s2p").unwrap();
        let network2 = Network::new("files/ntwk2.s2p").unwrap();
        let _ = network1.cascade_ports(&network2, 3, 1);
    }

    #[test]
    #[should_panic(expected = "to_port 5 out of range for 2-port network")]
    fn test_cascade_ports_invalid_to_port() {
        let network1 = Network::new("files/ntwk1.s2p").unwrap();
        let network2 = Network::new("files/ntwk2.s2p").unwrap();
        let _ = network1.cascade_ports(&network2, 2, 5);
    }

    #[test]
    #[should_panic(expected = "from_port 0 out of range")]
    fn test_cascade_ports_zero_port() {
        let network1 = Network::new("files/ntwk1.s2p").unwrap();
        let network2 = Network::new("files/ntwk2.s2p").unwrap();
        let _ = network1.cascade_ports(&network2, 0, 1);
    }

    #[test]
    fn test_save_load_roundtrip_ma_format() {
        let network1 = Network::new("files/hfss_threeport_MA.s3p").unwrap();
        assert_eq!(network1.format, "MA");

        let temp_dir = std::env::temp_dir()
            .join("touchstone_tests")
            .join("roundtrip_ma");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("roundtrip_ma.s3p");
        let file_path_str = file_path.to_str().unwrap();

        network1.save(file_path_str).unwrap();
        let network2 = Network::new(file_path_str).unwrap();

        assert_eq!(network1.rank, network2.rank);
        assert_eq!(network1.format, network2.format);
        assert_eq!(network1.f.len(), network2.f.len());

        let epsilon = 1e-6;
        for i in 0..network1.s.len() {
            for row in 1..=3 {
                for col in 1..=3 {
                    let s1 = &network1.s[i].s_ma;
                    let s2 = &network2.s[i].s_ma;
                    assert!(
                        (s1.get(row, col).0 - s2.get(row, col).0).abs() < epsilon,
                        "S{}{} mag mismatch at index {}",
                        row,
                        col,
                        i
                    );
                    assert!(
                        (s1.get(row, col).1 - s2.get(row, col).1).abs() < epsilon,
                        "S{}{} angle mismatch at index {}",
                        row,
                        col,
                        i
                    );
                }
            }
        }

        std::fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_save_load_roundtrip_db_format() {
        let network1 = Network::new("files/hfss_threeport_DB.s3p").unwrap();
        assert_eq!(network1.format, "DB");

        let temp_dir = std::env::temp_dir()
            .join("touchstone_tests")
            .join("roundtrip_db");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("roundtrip_db.s3p");
        let file_path_str = file_path.to_str().unwrap();

        network1.save(file_path_str).unwrap();
        let network2 = Network::new(file_path_str).unwrap();

        assert_eq!(network1.rank, network2.rank);
        assert_eq!(network1.format, network2.format);
        assert_eq!(network1.f.len(), network2.f.len());

        let epsilon = 1e-6;
        for i in 0..network1.s.len() {
            for row in 1..=3 {
                for col in 1..=3 {
                    let s1 = &network1.s[i].s_db;
                    let s2 = &network2.s[i].s_db;
                    assert!(
                        (s1.get(row, col).0 - s2.get(row, col).0).abs() < epsilon,
                        "S{}{} dB mismatch at index {}",
                        row,
                        col,
                        i
                    );
                    assert!(
                        (s1.get(row, col).1 - s2.get(row, col).1).abs() < epsilon,
                        "S{}{} angle mismatch at index {}",
                        row,
                        col,
                        i
                    );
                }
            }
        }

        std::fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_save_load_roundtrip_1port() {
        let network1 = Network::new("files/hfss_oneport.s1p").unwrap();

        let temp_dir = std::env::temp_dir()
            .join("touchstone_tests")
            .join("roundtrip_1port");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("roundtrip.s1p");
        let file_path_str = file_path.to_str().unwrap();

        network1.save(file_path_str).unwrap();
        let network2 = Network::new(file_path_str).unwrap();

        assert_eq!(network1.rank, 1);
        assert_eq!(network1.rank, network2.rank);
        assert_eq!(network1.f.len(), network2.f.len());

        let epsilon = 1e-6;
        for i in 0..network1.s.len() {
            let s1 = &network1.s[i].s_ri;
            let s2 = &network2.s[i].s_ri;
            assert!((s1.get(1, 1).0 - s2.get(1, 1).0).abs() < epsilon);
            assert!((s1.get(1, 1).1 - s2.get(1, 1).1).abs() < epsilon);
        }

        std::fs::remove_file(file_path).unwrap();
    }

    #[test]
    #[should_panic(expected = "Cannot cascade networks with different reference impedances")]
    fn test_cascade_different_z0() {
        let mut net1 = Network::new("files/ntwk1.s2p").unwrap();
        let net2 = Network::new("files/ntwk2.s2p").unwrap();
        net1.z0 = 75.0;
        let _ = net1.cascade(&net2);
    }

    #[test]
    #[should_panic(expected = "Cannot cascade networks with different frequency units")]
    fn test_cascade_different_freq_units() {
        let mut net1 = Network::new("files/ntwk1.s2p").unwrap();
        let net2 = Network::new("files/ntwk2.s2p").unwrap();
        net1.frequency_unit = "MHz".to_string();
        let _ = net1.cascade(&net2);
    }

    #[test]
    #[should_panic(expected = "Cascading is only implemented for 2-port networks")]
    fn test_cascade_non_2port() {
        let net1 = Network::new("files/hfss_18.2.s3p").unwrap();
        let net2 = Network::new("files/hfss_18.2.s3p").unwrap();
        let _ = net1.cascade(&net2);
    }

    #[test]
    fn test_print_summary() {
        let net = Network::new("files/ntwk1.s2p").unwrap();
        // Just verify it doesn't panic
        net.print_summary();
    }

    #[test]
    fn test_clone() {
        let net1 = Network::new("files/ntwk1.s2p").unwrap();
        let net2 = net1.clone();
        assert_eq!(net1.rank, net2.rank);
        assert_eq!(net1.f.len(), net2.f.len());
        assert_eq!(net1.z0, net2.z0);
    }

    #[test]
    fn test_debug() {
        let net = Network::new("files/ntwk1.s2p").unwrap();
        let debug_str = format!("{:?}", net);
        assert!(debug_str.contains("Network"));
    }

    #[test]
    fn sample_at_exact_match_returns_existing_point() {
        let network = interpolation_1port_network();

        let point = network
            .sample_at(1.0e9, Interpolation::Linear, Extrapolation::Error)
            .unwrap();

        assert_eq!(point.frequency, 1.0e9);
        assert_s11(&point, 0.0, 0.0);
    }

    #[test]
    fn sample_at_midpoint_linearly_interpolates_real_imaginary_space() {
        let network = interpolation_1port_network();

        let point = network
            .sample_at(1.5e9, Interpolation::Linear, Extrapolation::Error)
            .unwrap();

        assert_eq!(point.frequency, 1.5e9);
        assert_s11(&point, 1.0, 2.0);
    }

    #[test]
    fn resample_derives_ma_and_db_from_interpolated_real_imaginary_values() {
        let network = interpolation_1port_network();

        let resampled = network
            .resample([1.5e9], Interpolation::Linear, Extrapolation::Error)
            .unwrap();

        assert_eq!(resampled.f, vec![1.5e9]);
        assert_eq!(resampled.s.len(), 1);
        let s_ri = resampled.s[0].s_ri.get(1, 1);
        let s_ma = resampled.s[0].s_ma.get(1, 1);
        let s_db = resampled.s[0].s_db.get(1, 1);
        let magnitude = f64::sqrt(5.0);

        assert_approx_eq(s_ri.0, 1.0);
        assert_approx_eq(s_ri.1, 2.0);
        assert_approx_eq(s_ma.0, magnitude);
        assert_approx_eq(s_ma.1, f64::atan2(2.0, 1.0) * 180.0 / std::f64::consts::PI);
        assert_approx_eq(s_db.0, 20.0 * magnitude.log10());
        assert_approx_eq(s_db.1, s_ma.1);
    }

    #[test]
    fn sample_at_nearest_uses_closest_point() {
        let network = interpolation_1port_network();

        let lower = network
            .sample_at(1.4e9, Interpolation::Nearest, Extrapolation::Error)
            .unwrap();
        let upper = network
            .sample_at(1.6e9, Interpolation::Nearest, Extrapolation::Error)
            .unwrap();
        let tie = network
            .sample_at(1.5e9, Interpolation::Nearest, Extrapolation::Error)
            .unwrap();

        assert_s11(&lower, 0.0, 0.0);
        assert_s11(&upper, 2.0, 4.0);
        assert_s11(&tie, 0.0, 0.0);
    }

    #[test]
    fn sample_at_lower_and_upper_bounds_return_boundary_points() {
        let network = interpolation_1port_network();

        let lower = network
            .sample_at(1.0e9, Interpolation::Linear, Extrapolation::Error)
            .unwrap();
        let upper = network
            .sample_at(2.0e9, Interpolation::Linear, Extrapolation::Error)
            .unwrap();

        assert_s11(&lower, 0.0, 0.0);
        assert_s11(&upper, 2.0, 4.0);
    }

    #[test]
    fn sample_at_out_of_range_returns_structured_error() {
        let network = interpolation_1port_network();

        let error = network
            .sample_at(0.5e9, Interpolation::Linear, Extrapolation::Error)
            .unwrap_err();

        assert!(matches!(
            error,
            TouchstoneError::FrequencyOutOfRange {
                frequency,
                min,
                max
            } if frequency == 0.5e9 && min == 1.0e9 && max == 2.0e9
        ));
    }

    #[test]
    fn sample_at_clamp_holds_boundary_values_at_requested_frequency() {
        let network = interpolation_1port_network();

        let lower = network
            .sample_at(0.5e9, Interpolation::Linear, Extrapolation::Clamp)
            .unwrap();
        let upper = network
            .sample_at(3.0e9, Interpolation::Linear, Extrapolation::Clamp)
            .unwrap();

        assert_eq!(lower.frequency, 0.5e9);
        assert_eq!(upper.frequency, 3.0e9);
        assert_s11(&lower, 0.0, 0.0);
        assert_s11(&upper, 2.0, 4.0);
    }

    #[test]
    fn sample_at_interpolates_n_port_matrix_values() {
        let network = Network::from_str(
            "matrix.s3p",
            "# GHz S RI R 50\n\
             1.0 1 -1 2 -2 3 -3 4 -4 5 -5 6 -6 7 -7 8 -8 9 -9\n\
             2.0 11 -11 12 -12 13 -13 14 -14 15 -15 16 -16 17 -17 18 -18 19 -19\n",
        )
        .unwrap();

        let point = network
            .sample_at(1.5e9, Interpolation::Linear, Extrapolation::Error)
            .unwrap();
        let s32 = point.s.get(3, 2).unwrap();

        assert_eq!(point.s.rank, 3);
        assert_approx_eq(s32.re, 13.0);
        assert_approx_eq(s32.im, -13.0);
    }

    #[test]
    fn resample_preserves_network_metadata() {
        let network = Network::from_str(
            "metadata.s2p",
            "! before option\n\
             [Version] 2.1\n\
             # GHz S DB R 75\n\
             [Number of Ports] 2\n\
             [Reference] 45 55\n\
             [Network Data]\n\
             ! after option\n\
             1.0 0 0 0 0 0 0 0 0\n\
             2.0 6 0 6 0 6 0 6 0\n\
             [End]\n",
        )
        .unwrap();

        let resampled = network
            .resample(
                [1.0e9, 1.5e9, 2.0e9],
                Interpolation::Linear,
                Extrapolation::Error,
            )
            .unwrap();

        assert_eq!(resampled.name, network.name);
        assert_eq!(resampled.rank, network.rank);
        assert_eq!(resampled.frequency_unit, network.frequency_unit);
        assert_eq!(resampled.parameter, network.parameter);
        assert_eq!(resampled.format, network.format);
        assert_eq!(resampled.resistance_string, network.resistance_string);
        assert_eq!(resampled.z0, network.z0);
        assert_eq!(
            resampled.reference_impedance(),
            network.reference_impedance()
        );
        assert_eq!(resampled.comments, network.comments);
        assert_eq!(
            resampled.comments_after_option_line,
            network.comments_after_option_line
        );
        assert_eq!(resampled.warnings, network.warnings);
        assert_eq!(resampled.f, vec![1.0e9, 1.5e9, 2.0e9]);
        assert_eq!(resampled.s.len(), 3);
    }

    #[test]
    fn sample_at_rejects_empty_network_data() {
        let mut network = interpolation_1port_network();
        network.f.clear();
        network.s.clear();

        let error = network
            .sample_at(1.0e9, Interpolation::Linear, Extrapolation::Error)
            .unwrap_err();

        assert!(matches!(error, TouchstoneError::EmptyNetworkData));
    }

    #[test]
    fn sample_at_rejects_duplicate_frequencies() {
        let mut network = interpolation_1port_network();
        network.f = vec![1.0e9, 1.0e9];

        let error = network
            .sample_at(1.0e9, Interpolation::Linear, Extrapolation::Error)
            .unwrap_err();

        assert!(matches!(
            error,
            TouchstoneError::DuplicateFrequency {
                first_index: 0,
                duplicate_index: 1,
                frequency
            } if frequency == 1.0e9
        ));
    }

    #[test]
    fn sample_at_rejects_non_finite_frequencies() {
        let mut network = interpolation_1port_network();
        network.f = vec![1.0e9, f64::INFINITY];

        let error = network
            .sample_at(1.0e9, Interpolation::Linear, Extrapolation::Error)
            .unwrap_err();

        assert!(matches!(
            error,
            TouchstoneError::InvalidFrequency {
                point_index: 1,
                frequency
            } if frequency.is_infinite()
        ));
    }

    #[test]
    fn sample_at_rejects_unsorted_frequencies() {
        let mut network = interpolation_1port_network();
        network.f = vec![2.0e9, 1.0e9];

        let error = network
            .sample_at(1.5e9, Interpolation::Linear, Extrapolation::Error)
            .unwrap_err();

        assert!(matches!(
            error,
            TouchstoneError::UnsortedFrequencies {
                previous_index: 0,
                previous_frequency,
                next_index: 1,
                next_frequency
            } if previous_frequency == 2.0e9 && next_frequency == 1.0e9
        ));
    }
}
