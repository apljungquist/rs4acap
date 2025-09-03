use std::env;

use anyhow::bail;
use base64::Engine;
use log::debug;
use reqwest::header::{HeaderMap, AUTHORIZATION};
use url::{Host, Url};

fn authorization_headers(username: &str, password: &str) -> HeaderMap {
    let credentials = format!("{username}:{password}");
    let auth_header = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(credentials)
    );
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, auth_header.try_into().unwrap());
    headers
}

pub struct ClientBuilder {
    host: Host,
    plain_port: Option<u16>,
    secure_port: Option<u16>,
    inner: reqwest::ClientBuilder,
}

impl ClientBuilder {
    pub fn new(host: Host) -> Self {
        Self {
            host,
            plain_port: None,
            secure_port: None,
            inner: reqwest::Client::builder(),
        }
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let username = env::var("AXIS_DEVICE_USER")?;
        let password = env::var("AXIS_DEVICE_PASS")?;
        let host = env::var("AXIS_DEVICE_IP")?;
        let plain_port = env::var("AXIS_DEVICE_HTTP_PORT")
            .ok()
            .map(|p| p.parse())
            .transpose()?;
        let secure_port = env::var("AXIS_DEVICE_HTTPS_PORT")
            .ok()
            .map(|p| p.parse())
            .transpose()?;
        let host = Host::parse(&host)?;

        debug!("Building client using username {username} from env");
        Ok(ClientBuilder::new(host)
            .plain_port(plain_port)
            .secure_port(secure_port)
            .basic_authentication(&username, &password))
    }

    pub fn basic_authentication(mut self, username: &str, password: &str) -> Self {
        let headers = authorization_headers(username, password);
        self.inner = self.inner.default_headers(headers);
        self
    }

    pub fn plain_port(mut self, port: Option<u16>) -> Self {
        self.plain_port = port;
        self
    }

    pub fn secure_port(mut self, port: Option<u16>) -> Self {
        self.secure_port = port;
        self
    }

    pub fn with_inner(
        mut self,
        f: impl FnOnce(reqwest::ClientBuilder) -> reqwest::ClientBuilder,
    ) -> Self {
        self.inner = f(self.inner);
        self
    }

    pub fn build_with_scheme(self, scheme: Scheme) -> anyhow::Result<Client> {
        let Self {
            host,
            plain_port,
            secure_port,
            inner,
        } = self;
        let client = inner.build()?;
        let client1 = Client {
            scheme,
            host: host.clone(),
            port: match scheme {
                Scheme::Secure => secure_port,
                Scheme::Plain => plain_port,
            },
            client,
        };
        Ok(client1)
    }

    /// Automatically select an appropriate scheme and create a new client.
    pub async fn build_with_automatic_scheme(self) -> anyhow::Result<Client> {
        let Self {
            host,
            plain_port,
            secure_port,
            inner,
        } = self;
        let client = inner.build()?;
        let mut candidate = Client {
            scheme: Scheme::Secure,
            host,
            port: None,
            client,
        };
        for (scheme, port) in [(Scheme::Secure, secure_port), (Scheme::Plain, plain_port)] {
            candidate.scheme = scheme;
            candidate.port = port;
            if candidate
                .system_ready_1()
                .system_ready()
                .send()
                .await
                .inspect_err(|e| debug!("Could not connect using {} because {e:?}", scheme.http()))
                .is_ok()
            {
                return Ok(candidate);
            }
        }
        bail!("Could not connect to either scheme")
    }
}

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
    port: Option<u16>,
    client: reqwest::Client,
}

impl Client {
    pub fn builder(host: Host) -> ClientBuilder {
        ClientBuilder::new(host)
    }

    pub(crate) fn get(&self, path: &str) -> anyhow::Result<reqwest::RequestBuilder> {
        Ok(self.client.get(self.url().join(path)?))
    }

    pub(crate) fn post(&self, path: &str) -> anyhow::Result<reqwest::RequestBuilder> {
        Ok(self.client.post(self.url().join(path)?))
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
