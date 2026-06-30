use std::fmt;

/// Source location attached to a Touchstone parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TouchstoneErrorContext {
    /// Source name or path that was being parsed.
    pub source_name: String,
    /// 1-based line number where the parser detected the error, when known.
    pub line_number: Option<usize>,
    /// Source line or logical data-line segment where the parser detected the error.
    pub line: Option<String>,
}

impl fmt::Display for TouchstoneErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line_number, self.line.as_deref()) {
            (Some(line_number), Some(line)) => {
                write!(f, "{}:{line_number}: {}", self.source_name, line)
            }
            (Some(line_number), None) => write!(f, "{}:{line_number}", self.source_name),
            (None, Some(line)) => write!(f, "{}: {}", self.source_name, line),
            (None, None) => write!(f, "{}", self.source_name),
        }
    }
}

/// Non-fatal condition reported while parsing Touchstone data.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TouchstoneWarning {
    /// No option line was found, so Touchstone default options were used.
    MissingOptionLine {
        /// Source name or path that was parsed.
        source_name: String,
    },
    /// A second or later option line was ignored after the first option line was parsed.
    AdditionalOptionLineIgnored {
        /// Source name or path that was parsed.
        source_name: String,
        /// 1-based line number of the ignored option line.
        line_number: usize,
        /// Ignored option line text.
        line: String,
    },
    /// An unsupported keyword was ignored.
    UnknownKeywordIgnored {
        /// Source name or path that was parsed.
        source_name: String,
        /// 1-based line number of the ignored keyword line.
        line_number: usize,
        /// Normalized keyword name.
        keyword: String,
    },
}

impl fmt::Display for TouchstoneWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOptionLine { source_name } => {
                write!(
                    f,
                    "{source_name}: no option line found; using Touchstone defaults"
                )
            }
            Self::AdditionalOptionLineIgnored {
                source_name,
                line_number,
                line,
            } => write!(
                f,
                "{source_name}:{line_number}: additional option line ignored: {line}"
            ),
            Self::UnknownKeywordIgnored {
                source_name,
                line_number,
                keyword,
            } => write!(
                f,
                "{source_name}:{line_number}: unsupported keyword ignored: [{keyword}]"
            ),
        }
    }
}

/// Error returned when Touchstone data cannot be read or parsed.
#[derive(Debug)]
#[non_exhaustive]
pub enum TouchstoneError {
    /// A parse error with source-location context.
    Parse {
        /// Source location where the parser detected the error.
        context: TouchstoneErrorContext,
        /// Structured root error.
        source: Box<TouchstoneError>,
    },
    /// The source file could not be read.
    Io(std::io::Error),
    /// In-memory bytes were not valid UTF-8 Touchstone text.
    InvalidUtf8(std::str::Utf8Error),
    /// The source name did not include a file extension.
    MissingFileType {
        /// Source name used for extension inference.
        source_name: String,
    },
    /// The source extension is not a supported Touchstone file type.
    UnsupportedFileType {
        /// Unsupported extension without the leading dot.
        file_type: String,
    },
    /// The Touchstone extension did not contain a valid port count.
    InvalidPortCount {
        /// Extension that could not be converted to a port count.
        file_type: String,
    },
    /// The file contained more than one option line.
    MultipleOptionLines,
    /// A data line did not contain the expected number of values.
    InvalidDataLineParts {
        /// Number of values required for the current network rank.
        expected: usize,
        /// Number of values found in the line.
        actual: usize,
    },
    /// A numeric token could not be parsed as `f64`.
    InvalidNumber {
        /// Token that failed numeric parsing.
        token: String,
    },
    /// The frequency unit is not supported.
    UnsupportedFrequencyUnit {
        /// Unit token from the option line.
        unit: String,
    },
    /// The network data format is not supported.
    UnsupportedFormat {
        /// Format token from the option line.
        format: String,
    },
    /// A keyword line was malformed.
    InvalidKeywordLine {
        /// Full keyword line.
        line: String,
    },
    /// The Touchstone version is not supported.
    UnsupportedVersion {
        /// Version string from the `[Version]` keyword.
        version: String,
    },
    /// The `[Number of Ports]` value was not a valid integer.
    InvalidNumberOfPorts {
        /// Raw value from the keyword.
        value: String,
    },
    /// The `[Number of Ports]` value did not match the source extension.
    NumberOfPortsMismatch {
        /// Port count declared by the keyword.
        keyword_ports: i32,
        /// Port count inferred from the source extension.
        extension_ports: i32,
    },
    /// `[Two-Port Data Order]` was present for a network other than `.s2p`.
    TwoPortDataOrderForNonTwoPort,
    /// The `[Two-Port Data Order]` value is not supported.
    UnsupportedTwoPortDataOrder {
        /// Order token from the keyword.
        order: String,
    },
    /// The `[Number of Frequencies]` value was not a valid integer.
    InvalidNumberOfFrequencies {
        /// Raw value from the keyword.
        value: String,
    },
    /// The parsed data line count did not match `[Number of Frequencies]`.
    NumberOfFrequenciesMismatch {
        /// Expected number of frequency rows.
        expected: usize,
        /// Parsed number of frequency rows.
        actual: usize,
    },
    /// Matrix format values other than `Full` are not supported.
    UnsupportedMatrixFormat {
        /// Matrix format token from the keyword.
        format: String,
    },
    /// `[Reference]` did not contain either one common value or one value per port.
    InvalidReferenceImpedanceCount {
        /// Number of ports in the network.
        ports: usize,
        /// Number of reference impedance values found.
        actual: usize,
    },
    /// A requested 0-based frequency point index was outside the parsed data range.
    InvalidPointIndex {
        /// Requested 0-based frequency point index.
        point_index: usize,
        /// Number of parsed frequency points.
        point_count: usize,
    },
    /// A requested 1-based S-parameter port index was outside the network port range.
    InvalidPortIndex {
        /// Requested destination/output port, using 1-based RF indexing.
        to_port: usize,
        /// Requested source/input port, using 1-based RF indexing.
        from_port: usize,
        /// Number of ports in the network or matrix.
        rank: usize,
    },
    /// A generated network rank was outside the supported range.
    InvalidNetworkRank {
        /// Requested network rank.
        rank: usize,
    },
    /// The generated network rank did not match an inferable `.sNp` extension.
    NetworkRankExtensionMismatch {
        /// Requested network rank.
        rank: usize,
        /// Port count inferred from the source name extension.
        extension_rank: usize,
    },
    /// A generated network did not contain any frequency points.
    EmptyNetworkData,
    /// The frequency vector length did not match the number of S-parameter data rows.
    FrequencyDataLengthMismatch {
        /// Number of values in the public frequency vector.
        frequency_count: usize,
        /// Number of S-parameter data rows.
        data_count: usize,
    },
    /// A generated S-parameter matrix declared a different rank than the network.
    InvalidMatrixRank {
        /// 0-based frequency point index.
        point_index: usize,
        /// Rank declared by the matrix.
        matrix_rank: usize,
        /// Rank required by the network.
        expected_rank: usize,
    },
    /// A generated S-parameter matrix was not shaped as a square rank-by-rank matrix.
    InvalidMatrixShape {
        /// 0-based frequency point index.
        point_index: usize,
        /// Number of rows in the matrix data.
        rows: usize,
        /// 0-based row index with the wrong column count, when known.
        row_index: Option<usize>,
        /// Number of columns in the row, or 0 when the row was missing.
        columns: usize,
        /// Expected row and column count.
        expected_rank: usize,
    },
    /// A generated, parsed, or requested frequency was not finite.
    InvalidFrequency {
        /// 0-based frequency point index.
        point_index: usize,
        /// Invalid frequency value in Hz.
        frequency: f64,
    },
    /// The frequency vector contained the same value more than once.
    DuplicateFrequency {
        /// 0-based index of the first occurrence.
        first_index: usize,
        /// 0-based index of the duplicate occurrence.
        duplicate_index: usize,
        /// Duplicate frequency value in Hz.
        frequency: f64,
    },
    /// The frequency vector was not strictly increasing.
    UnsortedFrequencies {
        /// 0-based index of the preceding frequency.
        previous_index: usize,
        /// Preceding frequency value in Hz.
        previous_frequency: f64,
        /// 0-based index of the next frequency.
        next_index: usize,
        /// Next frequency value in Hz.
        next_frequency: f64,
    },
    /// A requested sample frequency was outside the network frequency range.
    FrequencyOutOfRange {
        /// Requested frequency in Hz.
        frequency: f64,
        /// Minimum network frequency in Hz.
        min: f64,
        /// Maximum network frequency in Hz.
        max: f64,
    },
    /// A generated S-parameter value had a non-finite real or imaginary component.
    InvalidSParameterValue {
        /// 0-based frequency point index.
        point_index: usize,
        /// Destination/output port, using 1-based RF indexing.
        to_port: usize,
        /// Source/input port, using 1-based RF indexing.
        from_port: usize,
        /// Real component.
        re: f64,
        /// Imaginary component.
        im: f64,
    },
    /// A reference impedance was not finite and positive.
    InvalidReferenceImpedance {
        /// Invalid reference impedance in ohms.
        z0: f64,
    },
}

impl TouchstoneError {
    pub(crate) fn with_context(self, context: TouchstoneErrorContext) -> Self {
        match self {
            Self::Parse { .. } => self,
            _ => Self::Parse {
                context,
                source: Box::new(self),
            },
        }
    }

    /// Return parser source-location context when this error has it.
    ///
    /// # Examples
    ///
    /// ```
    /// let error =
    ///     touchstone::Network::from_str("uploaded.s1p", "# GHz S RI R 50\n1.0 0.1\n")
    ///         .unwrap_err();
    ///
    /// let context = error.context().unwrap();
    /// assert_eq!(context.source_name, "uploaded.s1p");
    /// assert_eq!(context.line_number, Some(2));
    /// assert_eq!(context.line.as_deref(), Some("1.0 0.1"));
    /// ```
    #[must_use]
    pub fn context(&self) -> Option<&TouchstoneErrorContext> {
        match self {
            Self::Parse { context, .. } => Some(context),
            _ => None,
        }
    }

    /// Return the deepest structured error wrapped by parser context.
    ///
    /// # Examples
    ///
    /// ```
    /// use touchstone::{Network, TouchstoneError};
    ///
    /// let error = Network::from_str("uploaded.s1p", "# GHz S RI R 50\n1.0 0.1\n").unwrap_err();
    ///
    /// assert!(matches!(
    ///     error.root_cause(),
    ///     TouchstoneError::InvalidDataLineParts {
    ///         expected: 3,
    ///         actual: 2
    ///     }
    /// ));
    /// ```
    #[must_use]
    pub fn root_cause(&self) -> &TouchstoneError {
        let mut current = self;
        while let Self::Parse { source, .. } = current {
            current = source;
        }
        current
    }
}

impl fmt::Display for TouchstoneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { context, source } => write!(f, "{source} at {context}"),
            Self::Io(err) => write!(f, "failed to read Touchstone file: {err}"),
            Self::InvalidUtf8(err) => write!(f, "Touchstone data is not valid UTF-8: {err}"),
            Self::MissingFileType { source_name } => {
                write!(f, "Touchstone source name has no file extension: {source_name}")
            }
            Self::UnsupportedFileType { file_type } => {
                write!(f, "unsupported Touchstone file type: {file_type}")
            }
            Self::InvalidPortCount { file_type } => {
                write!(f, "invalid port count in Touchstone file type: {file_type}")
            }
            Self::MultipleOptionLines => write!(f, "multiple option lines found"),
            Self::InvalidDataLineParts { expected, actual } => write!(
                f,
                "invalid data line: expected {expected} values, found {actual}"
            ),
            Self::InvalidNumber { token } => write!(f, "invalid numeric token: {token}"),
            Self::UnsupportedFrequencyUnit { unit } => {
                write!(f, "unsupported frequency unit: {unit}")
            }
            Self::UnsupportedFormat { format } => write!(f, "unsupported data format: {format}"),
            Self::InvalidKeywordLine { line } => write!(f, "invalid keyword line: {line}"),
            Self::UnsupportedVersion { version } => {
                write!(f, "unsupported Touchstone version: {version}")
            }
            Self::InvalidNumberOfPorts { value } => {
                write!(f, "invalid [Number of Ports] value: {value}")
            }
            Self::NumberOfPortsMismatch {
                keyword_ports,
                extension_ports,
            } => write!(
                f,
                "[Number of Ports] value {keyword_ports} does not match extension port count {extension_ports}"
            ),
            Self::TwoPortDataOrderForNonTwoPort => {
                write!(f, "[Two-Port Data Order] is only valid for two-port networks")
            }
            Self::UnsupportedTwoPortDataOrder { order } => {
                write!(f, "unsupported [Two-Port Data Order] value: {order}")
            }
            Self::InvalidNumberOfFrequencies { value } => {
                write!(f, "invalid [Number of Frequencies] value: {value}")
            }
            Self::NumberOfFrequenciesMismatch { expected, actual } => write!(
                f,
                "[Number of Frequencies] value {expected} does not match parsed data rows {actual}"
            ),
            Self::UnsupportedMatrixFormat { format } => {
                write!(f, "unsupported [Matrix Format] value: {format}")
            }
            Self::InvalidReferenceImpedanceCount { ports, actual } => write!(
                f,
                "invalid [Reference] value count: expected 1 common value or {ports} per-port values, found {actual}"
            ),
            Self::InvalidPointIndex {
                point_index,
                point_count,
            } => write!(
                f,
                "frequency point index {point_index} out of range for {point_count} points"
            ),
            Self::InvalidPortIndex {
                to_port,
                from_port,
                rank,
            } => write!(
                f,
                "S-parameter port index S{to_port}{from_port} out of range for {rank}-port network"
            ),
            Self::InvalidNetworkRank { rank } => {
                write!(f, "invalid generated network rank: {rank}")
            }
            Self::NetworkRankExtensionMismatch {
                rank,
                extension_rank,
            } => write!(
                f,
                "generated network rank {rank} does not match extension port count {extension_rank}"
            ),
            Self::EmptyNetworkData => write!(f, "network has no frequency points"),
            Self::FrequencyDataLengthMismatch {
                frequency_count,
                data_count,
            } => write!(
                f,
                "frequency vector has {frequency_count} values but network data has {data_count} rows"
            ),
            Self::InvalidMatrixRank {
                point_index,
                matrix_rank,
                expected_rank,
            } => write!(
                f,
                "generated matrix at point {point_index} has rank {matrix_rank}, expected {expected_rank}"
            ),
            Self::InvalidMatrixShape {
                point_index,
                rows,
                row_index,
                columns,
                expected_rank,
            } => {
                if let Some(row_index) = row_index {
                    write!(
                        f,
                        "generated matrix at point {point_index} row {row_index} has {columns} columns, expected {expected_rank}"
                    )
                } else {
                    write!(
                        f,
                        "generated matrix at point {point_index} has {rows} rows, expected {expected_rank}"
                    )
                }
            }
            Self::InvalidFrequency {
                point_index,
                frequency,
            } => write!(f, "frequency at point {point_index} is not finite: {frequency}"),
            Self::DuplicateFrequency {
                first_index,
                duplicate_index,
                frequency,
            } => write!(
                f,
                "duplicate frequency {frequency} Hz at points {first_index} and {duplicate_index}"
            ),
            Self::UnsortedFrequencies {
                previous_index,
                previous_frequency,
                next_index,
                next_frequency,
            } => write!(
                f,
                "frequencies must be strictly increasing: point {previous_index} is {previous_frequency} Hz, point {next_index} is {next_frequency} Hz"
            ),
            Self::FrequencyOutOfRange {
                frequency,
                min,
                max,
            } => write!(
                f,
                "frequency {frequency} Hz is outside network range {min} Hz to {max} Hz"
            ),
            Self::InvalidSParameterValue {
                point_index,
                to_port,
                from_port,
                re,
                im,
            } => write!(
                f,
                "generated S{to_port}{from_port} at point {point_index} is not finite: ({re}, {im})"
            ),
            Self::InvalidReferenceImpedance { z0 } => {
                write!(f, "reference impedance must be finite and positive: {z0}")
            }
        }
    }
}

impl std::error::Error for TouchstoneError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse { source, .. } => Some(source),
            Self::Io(err) => Some(err),
            Self::InvalidUtf8(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for TouchstoneError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<std::str::Utf8Error> for TouchstoneError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::InvalidUtf8(err)
    }
}
