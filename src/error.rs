use std::fmt;

#[derive(Debug)]
pub enum AppError {
    IoError(std::io::Error),
    InvalidWorldFile,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::IoError(err) => write!(f, "IO Error: {}", err),
            AppError::InvalidWorldFile => write!(f, "Invalid world file"),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::IoError(err)
    }
}
