//! Client independent utilities for VAPIX HTTP integration

use std::future::Future;

use reqwest::{Method, StatusCode};

#[derive(Debug, thiserror::Error)]
pub enum Error<E> {
    /// Incorrect API usage while building a request
    #[error(transparent)]
    Request(anyhow::Error),
    /// Transport failure
    #[error(transparent)]
    Transport(anyhow::Error),
    /// Failed to decode response
    #[error(transparent)]
    Decode(anyhow::Error),
    /// Error returned by the remote service
    #[error(transparent)]
    Service(E),
}

impl<E> Error<E> {
    pub(crate) fn flat_result<T>(r: anyhow::Result<Result<T, E>>) -> Result<T, Self> {
        match r {
            Ok(Ok(data)) => Ok(data),
            Ok(Err(e)) => Err(Self::Service(e)),
            Err(e) => Err(Self::Decode(e)),
        }
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub body: Option<Vec<u8>>,
    pub content_type: Option<String>,
}

impl Request {
    pub fn new(method: Method, path: String) -> Self {
        Self {
            method,
            path,
            body: None,
            content_type: None,
        }
    }

    pub fn json(mut self, body: String) -> Self {
        self.content_type = Some("application/json".to_string());
        self.body = Some(body.into_bytes());
        self
    }

    pub fn soap(mut self, body: String) -> Self {
        self.content_type = Some("application/soap+xml; charset=utf-8".to_string());
        self.body = Some(body.into_bytes());
        self
    }

    pub fn multipart(mut self, body: Vec<u8>, boundary: &str) -> Self {
        self.content_type = Some(format!("multipart/form-data; boundary={boundary}"));
        self.body = Some(body);
        self
    }
}

#[derive(Debug)]
pub struct Response {
    pub status: StatusCode,
    pub body: Result<String, reqwest::Error>,
}

pub trait HttpClient {
    fn execute(
        &self,
        request: Request,
    ) -> impl Future<Output = Result<Response, anyhow::Error>> + Send;
}
