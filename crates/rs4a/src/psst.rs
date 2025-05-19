//! Utilities for avoiding accidental disclosure of secrets.
use std::fmt;

#[derive(Clone, serde::Deserialize)]
pub struct Password(String);

impl Password {
    pub fn dangerous_reveal(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***")
    }
}
