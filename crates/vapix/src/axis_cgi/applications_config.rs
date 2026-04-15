//! The [Configure Applications] API.
//!
//! [Configure Applications]: https://developer.axis.com/vapix/applications/application-api/#configure-applications

use reqwest::{Method, StatusCode};

use crate::http::{Error, HttpClient, Request};

const PATH: &str = "axis-cgi/applications/config.cgi";

fn bool2string(b: bool) -> &'static str {
    match b {
        true => "true",
        false => "false",
    }
}

#[derive(Clone, Debug)]
pub struct ApplicationConfigRequest {
    name: &'static str,
    value: Option<&'static str>,
}

impl ApplicationConfigRequest {
    /// Available in AXIS OS 11.5 - 11.11
    ///
    /// Default:
    /// - `true` until AXIS OS 11.7
    /// - `false` since AXIS OS 11.8
    pub fn allow_root(allow: bool) -> Self {
        Self {
            name: "AllowRoot",
            value: Some(bool2string(allow)),
        }
    }

    /// Available since AXIS OS 11.2
    ///
    /// Default:
    /// - `true` until AXIS OS 11.11
    /// - `false` since AXIS OS 12.0
    pub fn allow_unsigned(allow: bool) -> Self {
        Self {
            name: "AllowUnsigned",
            value: Some(bool2string(allow)),
        }
    }

    fn into_request(self) -> Request {
        let Self { name, value } = self;
        let path = match value {
            None => format!("{PATH}?action=set&name={name}"),
            Some(value) => format!("{PATH}?action=set&name={name}&value={value}"),
        };
        Request::no_content(Method::GET, path)
    }

    // TODO: Implement lossless self-checks
    // Get requests return XML that needs parsing
    // TODO: Add support for get requests
    pub async fn send(
        self,
        client: &impl HttpClient,
    ) -> Result<(), Error<std::convert::Infallible>> {
        let response = client
            .execute(self.into_request())
            .await
            .map_err(Error::Transport)?;
        let body = response.body.map_err(|e| Error::Transport(e.into()))?;
        if response.status == StatusCode::OK && body.trim().starts_with(r#"<reply result="ok">"#) {
            Ok(())
        } else {
            Err(Error::Decode(anyhow::anyhow!(
                "Unexpected response: {} {}",
                response.status,
                body.trim()
            )))
        }
    }
}
