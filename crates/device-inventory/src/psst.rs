//! Utilities for avoiding accidental disclosure of secrets.

use std::{convert::Infallible, fmt};

// TODO: Consider removing implementation of `Serialize`
#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Password(String);

impl Password {
    pub fn parse(s: &str) -> Result<Self, Infallible> {
        Ok(Self(s.to_string()))
    }

    pub fn dangerous_reveal(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***")
    }
}
