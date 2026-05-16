//! Utilities for working with SOAP style APIs over HTTP.

use anyhow::Context;
use log::warn;
use serde::Deserialize;

use super::{
    http::Error,
    soap::{self, parse_soap},
};
use crate::http::{HttpClient, Request};

pub trait SoapResponse: Sized {
    /// Parse the SOAP envelope, return either the typed body or a fault.
    ///
    /// The outer error represents errors parsing the response.
    fn from_envelope(s: &str) -> anyhow::Result<Result<Self, soap::Error>>;
}

impl<T> SoapResponse for T
where
    T: for<'a> Deserialize<'a>,
{
    fn from_envelope(s: &str) -> anyhow::Result<Result<Self, soap::Error>> {
        parse_soap(s)
    }
}

// TODO: Factor out
pub async fn send_request<T: SoapResponse>(
    client: &(impl HttpClient + Sync),
    request: Request,
) -> Result<T, Error<soap::Error>> {
    let response = client.execute(request).await.map_err(Error::Transport)?;
    let status = response.status;
    let text = response.body.context(status).map_err(Error::Transport)?;
    let result = Error::flat_result(T::from_envelope(&text).context(status));
    if status.is_success() != result.is_ok() {
        warn!("HTTP status {status} does not match SOAP response");
    }
    result
}
