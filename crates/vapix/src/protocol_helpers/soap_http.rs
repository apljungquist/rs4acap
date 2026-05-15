//! Utilities for working with SOAP style APIs over HTTP.

use anyhow::Context;
use log::warn;
use serde::{Deserialize, Serialize};

use super::{
    http::Error,
    soap::{parse_soap_or_fault, Fault},
};
use crate::http::{HttpClient, Request};

pub trait SoapResponse: Sized {
    fn from_envelope(s: &str) -> anyhow::Result<Result<Self, Fault>>;
}

impl<T> SoapResponse for T
where
    T: for<'a> Deserialize<'a> + Serialize,
{
    fn from_envelope(s: &str) -> anyhow::Result<Result<Self, Fault>> {
        parse_soap_or_fault(s)
    }
}

// TODO: Factor out
pub async fn send_request<T: SoapResponse>(
    client: &(impl HttpClient + Sync),
    request: Request,
) -> Result<T, Error<Fault>> {
    let response = client.execute(request).await.map_err(Error::Transport)?;
    let status = response.status;
    let text = response.body.context(status).map_err(Error::Transport)?;
    let result = Error::flat_result(T::from_envelope(&text).context(status));
    if status.is_success() != result.is_ok() {
        warn!("HTTP status {status} does not match SOAP response");
    }
    result
}
