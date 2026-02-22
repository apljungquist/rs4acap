//! Utilities for working with REST-style configuration APIs.

use std::{
    any,
    fmt::{Display, Formatter},
};

use anyhow::Context;
use serde::Deserialize;
use serde_json::value::RawValue;

#[derive(Debug, Deserialize)]
struct Response<'a> {
    #[serde(borrow)]
    data: Option<&'a RawValue>,
    #[serde(borrow)]
    error: Option<&'a RawValue>,
    #[serde(borrow)]
    status: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
pub struct Error {
    pub code: u16,
    message: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_null_data() {
        assert_eq!(
            parse_data::<serde_json::Value>(r#"{"status":"success"}"#)
                .unwrap()
                .unwrap(),
            serde_json::Value::Null
        );
    }
}
