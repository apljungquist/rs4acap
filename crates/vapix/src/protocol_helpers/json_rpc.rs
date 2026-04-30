//! Utilities for working with JSON RPC style APIs.

use std::fmt::{Display, Formatter};

use anyhow::{bail, Context};
use log::{debug, error, warn};
use serde::{Deserialize, Serialize};
use serde_json::{value::RawValue, Value};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Response<'a> {
    #[serde(borrow)]
    data: Option<&'a RawValue>,
    #[serde(borrow)]
    error: Option<&'a RawValue>,
}

impl<'a> Response<'a> {
    pub fn try_into_data(self) -> anyhow::Result<Result<&'a RawValue, &'a RawValue>> {
        let Self { data, error } = self;
        match (data, error) {
            (Some(d), Some(e)) => {
                debug!("data: {d:?}, error: {e:?}");
                bail!("Response included data and an error ({e:?})")
            }
            (Some(d), None) => Ok(Ok(d)),
            (None, Some(e)) => Ok(Err(e)),
            (None, None) => bail!("Response included neither data nor error"),
        }
    }
}

/// Error returned by all JSON-RPC-style APIs
#[derive(Debug, Deserialize, Serialize)]
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
    T: for<'de> Deserialize<'de>,
{
    let result = serde_json::from_str::<Response>(text)
        .with_context(|| format!("Could not parse response; status text: {text}"))?
        .try_into_data()?;
    match result {
        Ok(d) => {
            let data = serde_json::from_str::<T>(d.get())
                .with_context(|| format!("Could not parse data: {}", d.get()))?;
            Ok(Ok(data))
        }
        Err(e) => {
            let error = serde_json::from_str::<Error>(e.get())
                .with_context(|| format!("Could not parse error: {}", e.get()))?;
            Ok(Err(error))
        }
    }
}

fn soft_assert_lossless<T>(text: &str, result: &Result<T, Error>) -> anyhow::Result<()>
where
    T: for<'a> Deserialize<'a> + Serialize,
{
    let Response { data, error } = serde_json::from_str(text)
        .with_context(|| format!("Could not parse response; text: {text}"))?;

    match result {
        Ok(d) => {
            let data = data.expect("If it deserializes to Ok, then it has a data field");
            let actual: Value = serde_json::from_str(&serde_json::to_string(d)?)?;
            let expected: Value = serde_json::from_str(data.get())?;
            if actual != expected {
                warn!("Data deserialization is not lossless");
            }
            debug_assert_eq!(actual, expected);
        }
        Err(e) => {
            let error = error.expect("If it deserializes to Err, then it has an error field");
            let actual: Value = serde_json::from_str(&serde_json::to_string(e)?)?;
            let expected: Value = serde_json::from_str(error.get())?;
            if actual != expected {
                warn!("Error deserialization is not lossless");
            }
            debug_assert_eq!(actual, expected);
        }
    }

    Ok(())
}

pub fn parse_data_lossless<T>(text: &str) -> anyhow::Result<Result<T, Error>>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let result = parse_data(text)?;

    if let Err(e) = soft_assert_lossless(text, &result) {
        error!("Failed to verify losslessness: {e:?}");
        debug_assert!(false);
    }

    Ok(result)
}
