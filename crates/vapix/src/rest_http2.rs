//! Utilities for working with REST-style configuration APIs over HTTP.

use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::{
    cassette::{Cassette, Request, Response},
    http::Error,
    rest,
    rest_http::from_response_lossless,
    Client,
};

// As a user of the request builders, I find having to import the correct trait annoying.
// TODO: Consider consolidating traits or implementing send directly on the request types.

pub trait RestHttp2: Send + Sized {
    type ResponseData: for<'a> Deserialize<'a> + Serialize;

    fn to_request(self) -> Request;

    fn send(
        self,
        client: &Client,
        cassette: Option<&mut Cassette>,
    ) -> impl Future<Output = Result<Self::ResponseData, Error<rest::Error>>> + Send {
        async move {
            let Response { status, body } = self.to_request().send(client, cassette).await?;
            from_response_lossless(status, body)
        }
    }
}
