use std::fmt;

#[derive(Debug)]
pub enum BackendError {
    IoError {
        message: String,
        error: String,
    },
    ConnectionError(String),
    UnsupportedOperation(String),
    NotFound(String),
    InvalidPath(String),
    ChecksumMismatch {
        expected: String,
        actual: String,
    },
    Other(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::IoError { message, error } => {
                write!(f, "IO error: {} ({})", message, error)
            }
            BackendError::ConnectionError(msg) => {
                write!(f, "Connection error: {}", msg)
            }
            BackendError::UnsupportedOperation(msg) => {
                write!(f, "Unsupported operation: {}", msg)
            }
            BackendError::NotFound(path) => {
                write!(f, "Path not found: {}", path)
            }
            BackendError::InvalidPath(path) => {
                write!(f, "Invalid path: {}", path)
            }
            BackendError::ChecksumMismatch { expected, actual } => {
                write!(f, "Checksum mismatch: expected {}, got {}", expected, actual)
            }
            BackendError::Other(msg) => {
                write!(f, "Error: {}", msg)
            }
        }
    }
}

impl std::error::Error for BackendError {}

