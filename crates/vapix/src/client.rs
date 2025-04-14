use log::debug;
use url::{Host, Url};

#[derive(Clone, Copy, Debug)]
pub enum Scheme {
    /// HTTPS and WSS
    Secure,
    /// HTTP and WS
    Plain,
}

impl Scheme {
    const fn http(self) -> &'static str {
        match self {
            Scheme::Secure => "https",
            Scheme::Plain => "http",
        }
    }
}

// TODO: Figure out how to expose APIs that use other, or multiple, transports.
/// The main client through which all HTTP APIs can be used.
#[derive(Clone)]
pub struct Client {
    scheme: Scheme,
    host: Host,
    client: reqwest::Client,
}

impl Client {
    pub fn new(scheme: Scheme, host: Host, client: reqwest::Client) -> Self {
        Self {
            scheme,
            host,
            client,
        }
    }

    /// Automatically select an appropriate scheme and create a new client.
    pub async fn detect_scheme(host: &Host, client: reqwest::Client) -> Option<Self> {
        for scheme in [Scheme::Secure, Scheme::Plain] {
            let candidate = Self::new(scheme, host.clone(), client.clone());
            if candidate
                .system_ready_1()
                .system_ready()
                .send()
                .await
                .inspect_err(|e| debug!("Could not connect using {} because {e:?}", scheme.http()))
                .is_ok()
            {
                return Some(candidate);
            }
        }
        None
    }

    pub(crate) fn post(&self, path: &str) -> anyhow::Result<reqwest::RequestBuilder> {
        Ok(self.client.post(self.url().join(path)?))
    }

    fn url(&self) -> Url {
        let Self { scheme, host, .. } = self;
        let scheme = scheme.http();
        Url::parse(&format!("{scheme}://{host}"))
            .expect("Restricted types are known to combine into a valid URL")
    }
}
