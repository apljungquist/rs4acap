use std::sync::{Arc, Mutex};

use anyhow::{bail, Context};
use digest_auth::{AuthContext, HttpMethod, WwwAuthenticateHeader};
use log::{debug, warn};
use reqwest::{
    header::{AUTHORIZATION, WWW_AUTHENTICATE},
    Method, StatusCode,
};
use url::{Host, Position, Url};

use crate::{
    apis,
    http::{HttpClient, Request, Response},
};

#[derive(Clone)]
struct Secret(String);

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "***")
    }
}

#[derive(Clone, Debug)]
struct DigestAuth {
    username: String,
    password: Secret,
    challenge: Arc<Mutex<Option<WwwAuthenticateHeader>>>,
}

impl DigestAuth {
    fn with_challenge(
        username: String,
        password: Secret,
        challenge: WwwAuthenticateHeader,
    ) -> Self {
        Self {
            username,
            password,
            challenge: Arc::new(Mutex::new(Some(challenge))),
        }
    }

    fn compute_header(
        &self,
        method: &str,
        path: &str,
        header: &mut WwwAuthenticateHeader,
    ) -> Option<String> {
        let ctx = AuthContext::new_with_method(
            &self.username,
            &self.password.0,
            path,
            None::<&[u8]>,
            HttpMethod::from(method),
        );
        header.respond(&ctx).ok().map(|a| a.to_header_string())
    }

    fn respond_to_challenge(
        &self,
        response: &reqwest::Response,
        path: &str,
        method: &str,
    ) -> Option<(WwwAuthenticateHeader, String)> {
        let www_auth = response.headers().get(WWW_AUTHENTICATE)?;
        let www_auth = match www_auth.to_str() {
            Ok(s) => s,
            Err(e) => {
                debug!("WWW-Authenticate header is not valid UTF-8: {e}");
                return None;
            }
        };
        let mut header = match digest_auth::parse(www_auth) {
            Ok(h) => h,
            Err(e) => {
                debug!("Failed to parse WWW-Authenticate header: {e}");
                return None;
            }
        };
        let value = self.compute_header(method, path, &mut header)?;
        Some((header, value))
    }

    async fn send(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let Some(cloned) = builder.try_clone() else {
            debug!("Digest auth skipped: request body is not cloneable");
            return builder.send().await;
        };
        let Ok(probe) = cloned.build() else {
            debug!("Digest auth skipped: failed to build request for inspection");
            return builder.send().await;
        };
        let method = probe.method().to_string();
        let path = probe.url()[Position::AfterPort..].to_string();

        let stored_auth_header = self
            .challenge
            .lock()
            .unwrap_or_else(|e| {
                let mut guard = e.into_inner();
                *guard = None;
                guard
            })
            .as_mut()
            .and_then(|h| self.compute_header(&method, &path, h));

        let Some(pilot) = builder.try_clone() else {
            debug!("Request builder is not cloneable, ignoring any challenges in the response");
            let builder = match stored_auth_header {
                None => builder,
                Some(auth_header) => builder.header(AUTHORIZATION, auth_header),
            };
            return builder.send().await;
        };

        let response = match stored_auth_header {
            None => pilot.send().await?,
            Some(auth_header) => pilot.header(AUTHORIZATION, auth_header).send().await?,
        };

        if response.status() != StatusCode::UNAUTHORIZED {
            return Ok(response);
        }

        match self.respond_to_challenge(&response, &path, &method) {
            None => {
                *self.challenge.lock().unwrap_or_else(|e| e.into_inner()) = None;
                Ok(response)
            }
            Some((header, value)) => {
                *self.challenge.lock().unwrap_or_else(|e| e.into_inner()) = Some(header);
                builder.header(AUTHORIZATION, value).send().await
            }
        }
    }
}

#[derive(Clone, Debug)]
enum Authentication {
    Basic { username: String, password: Secret },
    Digest(DigestAuth),
    Anonymous,
}

impl Authentication {
    fn digest(username: String, password: Secret) -> Self {
        Self::Digest(DigestAuth {
            username,
            password,
            challenge: Arc::new(Mutex::new(None)),
        })
    }
}

#[derive(Clone, Debug)]
struct Credentials {
    username: String,
    password: Secret,
}

pub struct ClientBuilder {
    host: Host,
    plain_port: Option<u16>,
    secure_port: Option<u16>,
    credentials: Option<Credentials>,
    inner: reqwest::ClientBuilder,
}

impl ClientBuilder {
    pub fn new(host: Host) -> Self {
        Self {
            host,
            plain_port: None,
            secure_port: None,
            credentials: None,
            inner: reqwest::Client::builder(),
        }
    }

    pub fn from_dut() -> anyhow::Result<Option<Self>> {
        let Some(device) = rs4a_dut::Device::from_env()? else {
            return Ok(None);
        };
        let rs4a_dut::Device {
            host,
            username,
            password,
            http_port,
            https_port,
            ssh_port: _,
            https_self_signed,
        } = device;

        debug!("Building client using username {username} from env");
        Ok(Some(
            ClientBuilder::new(host)
                .plain_port(http_port)
                .secure_port(https_port)
                .username_password(&username, &password)
                .with_inner(|b| b.danger_accept_invalid_certs(https_self_signed)),
        ))
    }

    pub fn username_password(mut self, username: &str, password: &str) -> Self {
        self.credentials = Some(Credentials {
            username: username.to_string(),
            password: Secret(password.to_string()),
        });
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

    pub fn build_with_scheme(self, scheme: Scheme, digest: bool) -> anyhow::Result<Client> {
        let Self {
            host,
            plain_port,
            secure_port,
            credentials,
            inner,
        } = self;
        let client = inner.build()?;
        Ok(Client {
            auth: match credentials {
                None => Authentication::Anonymous,
                Some(Credentials { username, password }) => match digest {
                    true => Authentication::digest(username, password),
                    false => Authentication::Basic { username, password },
                },
            },
            scheme,
            host,
            port: match scheme {
                Scheme::Secure => secure_port,
                Scheme::Plain => plain_port,
            },
            client,
        })
    }

    /// Create a new client and automatically set authentication and scheme
    pub async fn build(self) -> anyhow::Result<Client> {
        let Self {
            host,
            plain_port,
            secure_port,
            credentials,
            inner,
        } = self;

        let mut client = Client {
            auth: Authentication::Anonymous,
            scheme: Scheme::Secure,
            host,
            port: None,
            client: inner.build()?,
        };
        // If the certificate is self-signed and self-signed certificates are not allowed,
        // then this will fall back on plain HTTP, which is probably worse.
        // TODO: Consider skipping plain by default or falling through only on specific errors
        let () = Self::set_scheme(
            &mut client,
            &[(Scheme::Secure, secure_port), (Scheme::Plain, plain_port)],
        )
        .await?;
        if let Some(credentials) = credentials {
            let () = Self::set_authentication(&mut client, credentials).await?;
        }
        Ok(client)
    }

    async fn set_scheme(
        candidate: &mut Client,
        schemes: &[(Scheme, Option<u16>)],
    ) -> anyhow::Result<()> {
        for (scheme, port) in schemes {
            candidate.scheme = *scheme;
            candidate.port = *port;
            if apis::system_ready_1::SystemReadyRequest::new()
                .timeout(1)
                .send(candidate)
                .await
                .inspect_err(|e| debug!("Could not connect using {} because {e:?}", scheme.http()))
                .is_ok()
            {
                return Ok(());
            }
        }
        Err(anyhow::anyhow!("Could not connect to either scheme"))
    }

    // TODO: Consider handling multiple www_authenticate headers.
    async fn set_authentication(
        client: &mut Client,
        credentials: Credentials,
    ) -> anyhow::Result<()> {
        let response = client
            .post("axis-cgi/basicdeviceinfo.cgi")?
            .json(&serde_json::json!({
                "apiVersion": "1.0",
                "method": "getAllProperties"
            }))
            .send()
            .await
            .context("Failed to send request to server to check authentication")?;

        if response.status() != StatusCode::UNAUTHORIZED {
            warn!("Credentials were provided but the server did not require authentication");
            return Ok(());
        }

        let Credentials { username, password } = credentials;

        let www_auth = response
            .headers()
            .get(WWW_AUTHENTICATE)
            .and_then(|x| x.to_str().ok())
            .unwrap_or_default();

        match www_auth.to_lowercase().as_str() {
            s if s.starts_with("basic ") => {
                client.auth = Authentication::Basic { username, password };
            }
            s if s.starts_with("digest ") => {
                let header = digest_auth::parse(www_auth)?;
                client.auth =
                    Authentication::Digest(DigestAuth::with_challenge(username, password, header));
            }
            "" => {
                bail!("Server requires authentication but no methods were offered")
            }
            s => bail!("Authentication method not supported: {s}"),
        }

        Ok(())
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
    auth: Authentication,
    scheme: Scheme,
    host: Host,
    port: Option<u16>,
    client: reqwest::Client,
}

impl Client {
    pub fn builder(host: Host) -> ClientBuilder {
        ClientBuilder::new(host)
    }

    pub fn get(&self, path: &str) -> anyhow::Result<RequestBuilder> {
        self.request(Method::GET, path)
    }

    pub fn post(&self, path: &str) -> anyhow::Result<RequestBuilder> {
        self.request(Method::POST, path)
    }

    pub fn request(&self, method: Method, path: &str) -> anyhow::Result<RequestBuilder> {
        let builder = self.client.request(method, self.url().join(path)?);
        Ok(RequestBuilder::new(self.auth.clone(), builder))
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

pub struct RequestBuilder {
    auth: Authentication,
    builder: reqwest::RequestBuilder,
}

impl RequestBuilder {
    fn new(auth: Authentication, builder: reqwest::RequestBuilder) -> Self {
        Self { auth, builder }
    }

    pub fn query<T: serde::Serialize + ?Sized>(self, query: &T) -> Self {
        Self {
            auth: self.auth,
            builder: self.builder.query(query),
        }
    }

    pub fn header(self, key: reqwest::header::HeaderName, value: &str) -> Self {
        Self {
            auth: self.auth,
            builder: self.builder.header(key, value),
        }
    }

    pub fn body<T: Into<reqwest::Body>>(self, body: T) -> Self {
        Self {
            auth: self.auth,
            builder: self.builder.body(body),
        }
    }

    pub fn json<T: serde::Serialize + ?Sized>(self, json: &T) -> Self {
        Self {
            auth: self.auth,
            builder: self.builder.json(json),
        }
    }

    pub async fn send(self) -> Result<reqwest::Response, reqwest::Error> {
        match self.auth {
            Authentication::Basic { username, password } => {
                self.builder
                    .basic_auth(&username, Some(&password.0))
                    .send()
                    .await
            }
            Authentication::Digest(digest) => digest.send(self.builder).await,
            Authentication::Anonymous => self.builder.send().await,
        }
    }
}

impl HttpClient for Client {
    async fn execute(&self, request: Request) -> Result<Response, anyhow::Error> {
        let mut request_builder = self.request(request.method, &request.path)?;
        if let Some(body) = request.body {
            debug_assert!(request.content_type.is_some());
            request_builder = request_builder.body(body);
        }
        if let Some(content_type) = request.content_type {
            request_builder = request_builder.header(reqwest::header::CONTENT_TYPE, &content_type);
        }
        let response = request_builder.send().await.context("failed to send")?;
        Ok(Response {
            status: response.status(),
            body: response.text().await,
        })
    }
}
