use std::fmt;
use std::num::{ParseFloatError, ParseIntError, TryFromIntError};
use std::sync::PoisonError;

use rhai::EvalAltResult;

#[derive(Debug)]
pub enum AppError {
    IoError(std::io::Error),
    RegexError(regex::Error),
    ParseIntError(ParseIntError),
    ParseFloatError(ParseFloatError),
    TryFromIntError(TryFromIntError),
    RhaiError(Box<EvalAltResult>),
    MutexPoisoned(String),
    InvalidWorldFile,
    // InvalidExtensionVersion,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::IoError(err) => write!(f, "IO Error: {}", err),
            AppError::RegexError(err) => write!(f, "Extension Regex Error: {}", err),
            AppError::ParseIntError(err) => write!(f, "Parse int error: {}", err),
            AppError::ParseFloatError(err) => write!(f, "Parse float error: {}", err),
            AppError::TryFromIntError(err) => write!(f, "Integer conversion error: {}", err),
            AppError::RhaiError(err) => write!(f, "Rhai compilation error: {}", err),
            AppError::MutexPoisoned(err) => write!(f, "Poisoned mutex: {}", err),
            AppError::InvalidWorldFile => write!(f, "Invalid world file"),
            // AppError::InvalidExtensionVersion => write!(f, "Invalid extension version"),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::IoError(err)
    }
}

impl From<regex::Error> for AppError {
    fn from(err: regex::Error) -> Self {
        AppError::RegexError(err)
    }
}

impl From<ParseIntError> for AppError {
    fn from(err: ParseIntError) -> Self {
        AppError::ParseIntError(err)
    }
}

impl From<ParseFloatError> for AppError {
    fn from(err: ParseFloatError) -> Self {
        AppError::ParseFloatError(err)
    }
}

impl From<TryFromIntError> for AppError {
    fn from(err: TryFromIntError) -> Self {
        AppError::TryFromIntError(err)
    }
}

impl From<Box<EvalAltResult>> for AppError {
    fn from(err: Box<EvalAltResult>) -> Self {
        AppError::RhaiError(err)
    }
}

impl<T> From<PoisonError<T>> for AppError {
    fn from(err: PoisonError<T>) -> Self {
        AppError::MutexPoisoned(err.to_string())
    }
}
