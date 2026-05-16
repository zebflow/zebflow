use std::fmt::{Display, Formatter};

/// Error returned by ZebFS operations.
#[derive(Debug)]
pub struct ZebFsError {
    pub code: &'static str,
    pub message: String,
}

impl ZebFsError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl Display for ZebFsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ZebFsError {}

impl From<std::io::Error> for ZebFsError {
    fn from(value: std::io::Error) -> Self {
        Self::new("ZEBFS_IO", value.to_string())
    }
}
