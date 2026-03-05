use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub struct EngineError {
    pub code: &'static str,
    pub message: String,
}

impl EngineError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl Display for EngineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for EngineError {}
