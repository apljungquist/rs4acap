use anyhow::bail;
use base64::Engine;
use log::debug;
use reqwest::{
    header::{HeaderMap, AUTHORIZATION},
    Method,
};
use url::{Host, Url};

use crate::{apis, json_rpc_http::JsonRpcHttp};

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

    pub fn from_dut() -> anyhow::Result<Option<Self>> {
        let Some(device) = rs4a_dut::Device::from_anywhere()? else {
            return Ok(None);
        };
        let rs4a_dut::Device {
            host,
            username,
            password,
            http_port,
            https_port,
            ssh_port: _,
        } = device;

        debug!("Building client using username {username} from env");
        Ok(Some(
            ClientBuilder::new(host)
                .plain_port(http_port)
                .secure_port(https_port)
                .basic_authentication(&username, &password),
        ))
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
            if apis::system_ready_1::system_ready()
                .timeout(1)
                .send(&candidate)
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

    pub fn get(&self, path: &str) -> anyhow::Result<reqwest::RequestBuilder> {
        self.request(Method::GET, path)
    }

    pub fn post(&self, path: &str) -> anyhow::Result<reqwest::RequestBuilder> {
        self.request(Method::POST, path)
    }

    pub fn request(&self, method: Method, path: &str) -> anyhow::Result<reqwest::RequestBuilder> {
        Ok(self.client.request(method, dbg!(self.url().join(path)?)))
    }

    fn url(&self) -> Url {
        let Self {
            scheme, host, port, ..
        } = self;
        let scheme = scheme.http();
        let host = "127.0.0.1";
        if let Some(port) = port {
            Url::parse(&format!("{scheme}://{host}:{port}"))
        } else {
            Url::parse(&format!("{scheme}://{host}"))
        }
        .expect("Restricted types are known to combine into a valid URL")
    }
}
