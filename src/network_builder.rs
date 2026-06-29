use crate::data_line;
use crate::data_pairs::{
    DecibelAngle, DecibelAngleMatrix, MagnitudeAngle, MagnitudeAngleMatrix, RealImaginary,
    RealImaginaryMatrix,
};
use crate::{Network, NetworkPoint, SMatrix, TouchstoneError};

/// Builder for generated S-parameter networks.
///
/// Frequencies passed to [`point`](Self::point) and [`push_point`](Self::push_point) are in Hz.
/// The `frequency_unit` setting controls how those frequencies are written when the network is
/// serialized.
///
/// # Examples
///
/// ```
/// use touchstone::{Complex, NetworkBuilder, SMatrix};
///
/// let network = NetworkBuilder::new("generated.s1p", 1)
///     .point(
///         1.0e9,
///         SMatrix {
///             rank: 1,
///             data: vec![vec![Complex { re: 0.5, im: -0.1 }]],
///         },
///     )
///     .build()?;
///
/// assert_eq!(network.rank, 1);
/// assert_eq!(network.try_s_ri_at(0, 1, 1)?.re, 0.5);
/// # Ok::<(), touchstone::TouchstoneError>(())
/// ```
#[derive(Debug, Clone)]
pub struct NetworkBuilder {
    name: String,
    rank: usize,
    frequency_unit: String,
    z0: f64,
    comments: Vec<String>,
    comments_after_option_line: Vec<String>,
    points: Vec<NetworkPoint>,
}

impl NetworkBuilder {
    /// Create a builder for a generated S-parameter network.
    ///
    /// The default frequency unit is `Hz`, the default reference impedance is `50`, and the
    /// serialized data format is `RI`.
    #[must_use]
    pub fn new<S: Into<String>>(name: S, rank: usize) -> Self {
        Self {
            name: name.into(),
            rank,
            frequency_unit: "Hz".to_string(),
            z0: 50.0,
            comments: Vec::new(),
            comments_after_option_line: Vec::new(),
            points: Vec::new(),
        }
    }

    /// Set the frequency unit used when serializing the generated network.
    ///
    /// Frequencies added to the builder are always provided in Hz. Supported units are `Hz`,
    /// `kHz`, `MHz`, `GHz`, and `THz`.
    #[must_use]
    pub fn frequency_unit<S: Into<String>>(mut self, frequency_unit: S) -> Self {
        self.frequency_unit = frequency_unit.into();
        self
    }

    /// Set the scalar reference impedance in ohms.
    ///
    /// The value must be finite and greater than zero.
    #[must_use]
    pub fn z0(mut self, z0: f64) -> Self {
        self.z0 = z0;
        self
    }

    /// Add a comment written before the option line.
    ///
    /// A leading `!` is added when the supplied text does not already start with one.
    #[must_use]
    pub fn comment<S: Into<String>>(mut self, comment: S) -> Self {
        self.comments.push(normalize_comment(comment.into()));
        self
    }

    /// Add a comment written inside the `[Network Data]` section before the first data point.
    ///
    /// A leading `!` is added when the supplied text does not already start with one.
    #[must_use]
    pub fn network_data_comment<S: Into<String>>(mut self, comment: S) -> Self {
        self.comments_after_option_line
            .push(normalize_comment(comment.into()));
        self
    }

    /// Add one generated frequency point and return the builder.
    ///
    /// The frequency is in Hz. Matrix rows are destination/output ports and columns are
    /// source/input ports.
    #[must_use]
    pub fn point(mut self, frequency: f64, s: SMatrix) -> Self {
        self.points.push(NetworkPoint { frequency, s });
        self
    }

    /// Add one generated frequency point to an existing builder.
    ///
    /// The frequency is in Hz. Matrix rows are destination/output ports and columns are
    /// source/input ports.
    pub fn push_point(&mut self, frequency: f64, s: SMatrix) -> &mut Self {
        self.points.push(NetworkPoint { frequency, s });
        self
    }

    /// Build a [`Network`] without serializing or reparsing Touchstone text.
    pub fn build(self) -> Result<Network, TouchstoneError> {
        let rank = validate_rank(self.rank)?;
        validate_extension_rank(&self.name, self.rank)?;
        let frequency_unit = canonical_frequency_unit(&self.frequency_unit)
            .ok_or_else(|| TouchstoneError::UnsupportedFrequencyUnit {
                unit: self.frequency_unit.clone(),
            })?
            .to_string();

        if !self.z0.is_finite() || self.z0 <= 0.0 {
            return Err(TouchstoneError::InvalidReferenceImpedance { z0: self.z0 });
        }

        if self.points.is_empty() {
            return Err(TouchstoneError::EmptyNetworkData);
        }

        for (point_index, point) in self.points.iter().enumerate() {
            validate_frequency(point_index, point.frequency)?;
            validate_matrix(point_index, self.rank, &point.s)?;
        }

        let f = self
            .points
            .iter()
            .map(|point| point.frequency)
            .collect::<Vec<_>>();
        let s = self
            .points
            .iter()
            .map(|point| parsed_data_line_from_matrix(point.frequency, &point.s))
            .collect::<Vec<_>>();

        Ok(Network {
            name: self.name,
            rank,
            frequency_unit,
            parameter: "S".to_string(),
            format: "RI".to_string(),
            resistance_string: "R".to_string(),
            z0: self.z0,
            comments: self.comments,
            comments_after_option_line: self.comments_after_option_line,
            warnings: Vec::new(),
            f,
            s,
        })
    }
}

fn validate_rank(rank: usize) -> Result<i32, TouchstoneError> {
    if rank == 0 || rank > i32::MAX as usize {
        return Err(TouchstoneError::InvalidNetworkRank { rank });
    }

    Ok(rank as i32)
}

fn validate_extension_rank(name: &str, rank: usize) -> Result<(), TouchstoneError> {
    if let Some(extension_rank) = infer_touchstone_extension_rank(name) {
        if extension_rank != rank {
            return Err(TouchstoneError::NetworkRankExtensionMismatch {
                rank,
                extension_rank,
            });
        }
    }

    Ok(())
}

fn validate_frequency(point_index: usize, frequency: f64) -> Result<(), TouchstoneError> {
    if frequency.is_finite() {
        Ok(())
    } else {
        Err(TouchstoneError::InvalidFrequency {
            point_index,
            frequency,
        })
    }
}

fn validate_matrix(
    point_index: usize,
    expected_rank: usize,
    matrix: &SMatrix,
) -> Result<(), TouchstoneError> {
    if matrix.rank != expected_rank {
        return Err(TouchstoneError::InvalidMatrixRank {
            point_index,
            matrix_rank: matrix.rank,
            expected_rank,
        });
    }

    if matrix.data.len() != expected_rank {
        return Err(TouchstoneError::InvalidMatrixShape {
            point_index,
            rows: matrix.data.len(),
            row_index: None,
            columns: 0,
            expected_rank,
        });
    }

    for (row_index, row) in matrix.data.iter().enumerate() {
        if row.len() != expected_rank {
            return Err(TouchstoneError::InvalidMatrixShape {
                point_index,
                rows: matrix.data.len(),
                row_index: Some(row_index),
                columns: row.len(),
                expected_rank,
            });
        }

        for (column_index, value) in row.iter().enumerate() {
            if !value.re.is_finite() || !value.im.is_finite() {
                return Err(TouchstoneError::InvalidSParameterValue {
                    point_index,
                    to_port: row_index + 1,
                    from_port: column_index + 1,
                    re: value.re,
                    im: value.im,
                });
            }
        }
    }

    Ok(())
}

fn parsed_data_line_from_matrix(frequency: f64, matrix: &SMatrix) -> data_line::ParsedDataLine {
    let s_ri_data = matrix
        .data
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| RealImaginary(value.re, value.im))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let s_ri = RealImaginaryMatrix::from_vec(s_ri_data.clone());

    let s_db = DecibelAngleMatrix::from_vec(
        s_ri_data
            .iter()
            .map(|row| {
                row.iter()
                    .copied()
                    .map(DecibelAngle::from_real_imaginary)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    );

    let s_ma = MagnitudeAngleMatrix::from_vec(
        s_ri_data
            .iter()
            .map(|row| {
                row.iter()
                    .copied()
                    .map(MagnitudeAngle::from_real_imaginary)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    );

    data_line::ParsedDataLine {
        frequency,
        s_ri,
        s_db,
        s_ma,
    }
}

fn infer_touchstone_extension_rank(name: &str) -> Option<usize> {
    let extension = name.rsplit_once('.')?.1;
    if !extension.starts_with('s') || !extension.ends_with('p') {
        return None;
    }

    let digits = &extension[1..extension.len() - 1];
    if digits.is_empty() || digits.starts_with('0') || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    digits.parse::<usize>().ok()
}

fn canonical_frequency_unit(unit: &str) -> Option<&'static str> {
    match unit.trim().to_ascii_lowercase().as_str() {
        "hz" => Some("Hz"),
        "khz" => Some("kHz"),
        "mhz" => Some("MHz"),
        "ghz" => Some("GHz"),
        "thz" => Some("THz"),
        _ => None,
    }
}

fn normalize_comment(comment: String) -> String {
    if comment.trim_start().starts_with('!') {
        comment
    } else {
        format!("! {comment}")
    }
}
