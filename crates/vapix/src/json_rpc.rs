//! Utilities for working with JSON RPC style APIs.

use anyhow::{bail, Context};
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

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

pub fn parse_data<T>(text: &str) -> anyhow::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let result = serde_json::from_str::<Response>(text)
        .with_context(|| format!("Could not parse response; status text: {text}"))?
        .try_into_data()?;
    let data = match result {
        Ok(d) => d,
        // TODO: Proper error
        Err(e) => bail!("Error: {:?}", e),
    };
    serde_json::from_str::<T>(data.get())
        .with_context(|| format!("Could not parse data: {}", data.get()))
}
