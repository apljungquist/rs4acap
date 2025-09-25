use reqwest::Method;
use url::{Host, Url};

use crate::Scheme;

pub struct BlockingClient {
    pub scheme: Scheme,
    pub host: Host,
    pub port: Option<u16>,
    pub client: reqwest::blocking::Client,
}

// If the request builder is wrapped in a new type, then the inner type can be anything and
// feature flags can be used to determine the underlying client. It could even be an enum
// and have the feature flags each add new constructors so that the features become additive as
// they ideally should.

// It is also possible to go the other way and return something other than a request builder
// depending on what works with curl and adapt the default implementation of the trait accordingly.

// Yet another approach is to define a trait and accept client implementations from anywhere,
// but this makes the code more difficult to use and to maintain and should only be done if there
// are demonstrable benefits.

impl BlockingClient {
    pub fn post(&self, path: &str) -> anyhow::Result<reqwest::blocking::RequestBuilder> {
        self.request(Method::POST, path)
    }

    pub fn request(
        &self,
        method: Method,
        path: &str,
    ) -> anyhow::Result<reqwest::blocking::RequestBuilder> {
        Ok(self.client.request(method, self.url().join(path)?))
    }

    fn url(&self) -> Url {
        let Self {
            scheme, host, port, ..
        } = self;
        let scheme = scheme.http();
        if let Some(port) = port {
            Url::parse(&format!("{scheme}://{host}:{port}"))
        } else {
            Url::parse(&format!("{scheme}://{host}"))
        }
        .expect("Restricted types are known to combine into a valid URL")
    }
}
