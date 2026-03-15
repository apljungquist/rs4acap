//! Utilities for working with REST-style configuration APIs.

use std::{
    any,
    fmt::{Display, Formatter},
};

use anyhow::Context;
use log::{error, warn};
use serde::{Deserialize, Serialize};
use serde_json::{value::RawValue, Value};

#[derive(Debug, Deserialize)]
struct Response<'a> {
    #[serde(borrow)]
    data: Option<&'a RawValue>,
    #[serde(borrow)]
    error: Option<&'a RawValue>,
    #[serde(borrow)]
    status: Option<&'a str>,
}

/// A list specifying the kind of errors that can occur.
///
/// The **current hypothesis** is that these are the same across the device configuration framework.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Internal error
    InternalError = 0,
    /// Item does not exist
    NotFound = 2,
    /// Validation error
    ValidationError = 5,
    /// Item already exists
    AlreadyExists = 6,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Error {
    pub code: u16,
    message: String,
}

impl Error {
    pub fn kind(&self) -> Option<ErrorKind> {
        match self.code {
            0 => {
                debug_assert!(self.message.starts_with("Internal error"));
                Some(ErrorKind::InternalError)
            }
            2 => {
                debug_assert!(self.message.starts_with("Item does not exist:"));
                Some(ErrorKind::NotFound)
            }
            5 => {
                debug_assert!(self.message.starts_with("Validation error:"));
                Some(ErrorKind::ValidationError)
            }
            6 => {
                debug_assert!(self.message.starts_with("Item already exists:"));
                Some(ErrorKind::AlreadyExists)
            }
            _ => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Self { code, message } = self;
        write!(f, "({code}) {message}")
    }
}

impl std::error::Error for Error {}

pub fn parse_data<T>(text: &str) -> anyhow::Result<Result<T, Error>>
where
    T: for<'a> Deserialize<'a>,
{
    let Response {
        data,
        status,
        error,
    } = serde_json::from_str(text)
        .with_context(|| format!("Could not parse response; text: {text}"))?;
    if let Some(error) = error {
        let error: Error = serde_json::from_str(error.get()).with_context(|| {
            format!(
                "Could not parse error; config-status: {status:?}; error-text: {}",
                error.get()
            )
        })?;
        return Ok(Err(error));
    }
    let data = match data {
        None => serde_json::from_str("null")
            .with_context(|| format!("Could not parse {} from null", any::type_name::<T>())),
        Some(data) => serde_json::from_str(data.get()).with_context(|| {
            format!(
                "Could not parse data as {}; config-status: {status:?}; data-text: {}",
                any::type_name::<T>(),
                data.get()
            )
        }),
    };
    Ok(Ok(data?))
}

// The serialization-deserialization round-trip may carry an overhead in terms of CPU, memory and storage.
// TODO: Consider making this a feature.

fn soft_assert_lossless<T, E>(text: &str, result: &Result<T, E>) -> anyhow::Result<()>
where
    T: for<'a> Deserialize<'a> + Serialize,
    E: for<'a> Deserialize<'a> + Serialize,
{
    let Response {
        data,
        status: _,
        error,
    } = serde_json::from_str(text)
        .with_context(|| format!("Could not parse response; text: {text}"))?;

    match &result {
        Ok(d) => {
            if let Some(data) = data {
                let actual: Value = serde_json::from_str(&serde_json::to_string(d)?)?;
                let expected: Value = serde_json::from_str(data.get())?;
                if actual != expected {
                    warn!("Data deserialization is not lossless");
                }
                debug_assert_eq!(actual, expected);
            } else {
                // Deserialization uses "null" when no data is returned,
                // so it is reasonable to expect that it would serialize back to "null".
                // TODO: Consider distinguishing between "undefined" and "null".
                let actual: Value = serde_json::from_str(&serde_json::to_string(d)?)?;
                let expected = Value::Null;
                if actual != expected {
                    warn!("Data deserialization is not lossless (null data)");
                }
                debug_assert_eq!(actual, expected);
            }
        }
        Err(e) => {
            // PANICS:
            // The `unwrap` will never panic because `parse_data` will return an error only when
            // the raw response has an error and we are deserializing the same text.
            let actual: Value = serde_json::from_str(&serde_json::to_string(e)?)?;
            let expected: Value = serde_json::from_str(error.unwrap().get())?;
            if actual != expected {
                warn!("Error deserialization is not lossless");
            }
            debug_assert_eq!(actual, expected);
        }
    };

    Ok(())
}

pub fn parse_data_lossless<T>(text: &str) -> anyhow::Result<Result<T, Error>>
where
    T: for<'a> Deserialize<'a> + Serialize,
{
    let result = parse_data::<T>(text)?;

    if let Err(e) = soft_assert_lossless(text, &result) {
        error!("Failed to verify losslessness: {e:?}");
        debug_assert!(false);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_null_data() {
        let () = parse_data::<()>(r#"{"status":"success"}"#)
            .unwrap()
            .unwrap();
    }
}
