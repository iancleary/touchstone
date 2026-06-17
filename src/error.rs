use std::fmt;

/// Error returned when Touchstone data cannot be read or parsed.
#[derive(Debug)]
pub enum TouchstoneError {
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
}

impl fmt::Display for TouchstoneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
        }
    }
}

impl std::error::Error for TouchstoneError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
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
