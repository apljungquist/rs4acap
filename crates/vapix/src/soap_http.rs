//! Utilities for working with SOAP style APIs over HTTP.
use std::future::Future;

use anyhow::Context;
use log::warn;

use crate::{
    soap::{SimpleRequest, SoapRequest, SoapResponse},
    Client,
};

const PATH: &str = "vapix/services";

pub trait SoapHttpRequest: SoapRequest + Send + Sized {
    type Data: SoapResponse;

    fn send(self, client: &Client) -> impl Future<Output = anyhow::Result<Self::Data>> + Send {
        async move {
            let body = self.to_envelope()?;
            if cfg!(debug_assertions) {
                println!("Sending to {PATH}: {body}");
            }
            let response = client
                .post(PATH)?
                .header("Content-Type", "application/soap+xml; charset=utf-8")
                .body(body)
                .send()
                .await?;
            let status = response.status();
            let text = response.text().await.context(status)?;
            if cfg!(debug_assertions) {
                println!("Received {status}: {text}");
            }
            let result = Self::Data::from_envelope(&text).context(status);
            if status.is_success() != result.is_ok() {
                warn!("HTTP status {status} does not match SOAP response");
            }
            result
        }
    }
}

impl<T> SoapHttpRequest for SimpleRequest<T>
where
    T: SoapResponse + Send + Sized,
{
    type Data = T;
}
