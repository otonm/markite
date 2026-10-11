use std::{fmt, io};

/// Errors Swift can switch on. Mirrors the `io::ErrorKind`s the core produces.
#[derive(Debug, uniffi::Error)]
pub enum MarkiteError {
    NotFound,
    PermissionDenied,
    TooLarge,
    InvalidInput { message: String },
    Other { message: String },
}

impl fmt::Display for MarkiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => f.write_str("not found"),
            Self::PermissionDenied => f.write_str("permission denied"),
            Self::TooLarge => f.write_str("file too large"),
            Self::InvalidInput { message } | Self::Other { message } => f.write_str(message),
        }
    }
}

impl std::error::Error for MarkiteError {}

impl From<io::Error> for MarkiteError {
    fn from(e: io::Error) -> Self {
        markite_core::trace!("bridge error: {:?}", e.kind());
        match e.kind() {
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            io::ErrorKind::FileTooLarge => Self::TooLarge,
            io::ErrorKind::InvalidInput | io::ErrorKind::InvalidData => Self::InvalidInput {
                message: e.to_string(),
            },
            _ => Self::Other {
                message: e.to_string(),
            },
        }
    }
}

pub fn invalid(message: &str) -> MarkiteError {
    markite_core::trace!("bridge rejected input: {message}");
    MarkiteError::InvalidInput {
        message: message.into(),
    }
}

/// Paths come from Swift strings; a NUL byte would truncate them in C APIs, so refuse it here.
pub fn check_path(path: &str) -> Result<&str, MarkiteError> {
    if path.contains('\0') {
        Err(invalid("path contains a NUL byte"))
    } else {
        Ok(path)
    }
}
