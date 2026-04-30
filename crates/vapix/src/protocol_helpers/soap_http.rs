//! Utilities for working with SOAP style APIs over HTTP.

use std::convert::Infallible;

use anyhow::Context;
use log::warn;
use serde::Deserialize;

use super::{http::Error, soap::parse_soap};
use crate::http::{HttpClient, Request};

pub trait SoapResponse: Sized {
    fn from_envelope(s: &str) -> anyhow::Result<Self>;
}

impl<T> SoapResponse for T
where
    T: for<'a> Deserialize<'a>,
{
    fn from_envelope(s: &str) -> anyhow::Result<Self> {
        parse_soap(s)
    }
}

// TODO: Factor out
pub async fn send_request<T: SoapResponse>(
    client: &(impl HttpClient + Sync),
    request: Request,
) -> Result<T, Error<Infallible>> {
    let response = client.execute(request).await.map_err(Error::Transport)?;
    let status = response.status;
    let text = response.body.context(status).map_err(Error::Transport)?;
    let result = T::from_envelope(&text)
        .context(status)
        .map_err(Error::Decode);
    if status.is_success() != result.is_ok() {
        warn!("HTTP status {status} does not match SOAP response");
    }
    result
}
