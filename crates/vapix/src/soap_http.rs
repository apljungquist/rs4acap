//! Utilities for working with SOAP style APIs over HTTP.

use std::{convert::Infallible, future::Future};

use anyhow::Context;
use log::{trace, warn};
use serde::Deserialize;

use crate::{
    http::Error,
    soap::{parse_soap, SimpleRequest},
    Client,
};

const PATH: &str = "vapix/services";

pub trait SoapHttpRequest: SoapRequest + Send + Sized {
    type Data: SoapResponse;

    fn send(
        self,
        client: &Client,
    ) -> impl Future<Output = Result<Self::Data, Error<Infallible>>> + Send {
        async move {
            let body = self.to_envelope().map_err(Error::Request)?;
            if cfg!(debug_assertions) {
                println!("Sending to {PATH}: {body}");
            }
            let response = client
                .post(PATH)
                .map_err(Error::Request)?
                .header("Content-Type", "application/soap+xml; charset=utf-8")
                .body(body)
                .send()
                .await
                .context("Failed to send request")
                .map_err(Error::Transport)?;
            let status = response.status();
            let text = response
                .text()
                .await
                .context(status)
                .map_err(Error::Transport)?;
            if cfg!(debug_assertions) {
                trace!("Received {status}: {text}");
            }
            let result = Self::Data::from_envelope(&text)
                .context(status)
                .map_err(Error::Decode);
            if status.is_success() != result.is_ok() {
                warn!("HTTP status {status} does not match SOAP response");
            }
            result
        }
    }
}

pub trait SoapRequest {
    fn to_envelope(self) -> anyhow::Result<String>;
}

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

impl<T> SoapHttpRequest for SimpleRequest<T>
where
    T: SoapResponse + Send + Sized,
{
    type Data = T;
}
