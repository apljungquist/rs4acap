//! Facilities for building an HTTP client tuned for connecting to the VLT.
use anyhow::Context;
use reqwest::header::COOKIE;
use rs4a_authentication::SessionCookie;

/// An HTTP client tuned for connecting to the VLT.
pub struct Client(pub(crate) reqwest::Client);

impl Client {
    pub fn try_new(sid: SessionCookie) -> anyhow::Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(COOKIE, sid.into_header_value());
        reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map(Self)
            .context("Failed to create reqwest client.")
    }
}
