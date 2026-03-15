//! Record and replay sequences of HTTP requests and responses.
use std::{
    fs,
    fs::create_dir_all,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Context;
use log::debug;
use reqwest::{Method, StatusCode};

use crate::{http::Error, Client};

#[derive(Debug)]
pub struct Request {
    method: Method,
    path: String,
    body: Option<String>,
    content_type: Option<String>,
}

impl Request {
    pub fn json(method: Method, path: String) -> Self {
        Self {
            method,
            path,
            body: None,
            content_type: Some("application/json".to_string()),
        }
    }

    pub fn no_content(method: Method, path: String) -> Self {
        Self {
            method,
            path,
            body: None,
            content_type: None,
        }
    }

    pub fn body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }

    fn write(&self, file: &Path) -> anyhow::Result<()> {
        let Self {
            method,
            path,
            body,
            content_type,
        } = self;
        let () = create_dir_all(file.parent().context("the file has no parent")?)?;
        let mut content = format!("{method} {path}\n");
        if let Some(content_type) = content_type {
            content.push_str(&format!("Content-Type: {content_type}\n"));
        }
        if let Some(body) = body {
            content.push_str(&format!("\n{body}"));
        }
        let () =
            fs::write(file, content).with_context(|| format!("Could not write file {file:?}"))?;

        Ok(())
    }

    fn checksum(&self) -> u64 {
        let Self {
            method,
            path,
            body,
            content_type,
        } = self;

        // The `DefaultHasher` is not guaranteed to be stable across rust versions.
        // If it changes, all cassettes tracked in VCS will be invalidated.
        // TODO: Use a stable hashing algorithm.

        let mut hasher = DefaultHasher::new();
        method.hash(&mut hasher);
        path.hash(&mut hasher);
        if let Some(body) = body {
            body.hash(&mut hasher);
        }
        content_type.hash(&mut hasher);
        hasher.finish()
    }

    // TODO: Make passing the client optional when reading from a cassette
    /// Sends the request and returns a future response.
    ///
    /// # Panics
    ///
    /// This method panics if reading from or writing to a provided cassette fails.
    pub async fn send<T>(
        self,
        client: &Client,
        cassette: Option<&mut Cassette>,
    ) -> Result<Response, Error<T>> {
        let response_file = match cassette {
            Some(cassette) => {
                // PANICS:
                // Unwrapping is acceptable when a cassette is provided as stated in the docstring.
                cassette.advance();
                let checksum = self.checksum();
                match cassette.mode {
                    Mode::Read => {
                        let file = cassette.response_file(checksum);
                        debug!("Reading response from the cassette file {file:?}");
                        return Ok(Response::read(&file).unwrap());
                    }
                    Mode::Write => {
                        let file = cassette.request_file(checksum);
                        debug!("Writing request to the cassette file {file:?}");
                        self.write(&file).unwrap();
                        Some(cassette.response_file(checksum))
                    }
                }
            }
            None => None,
        };

        let mut request_builder = client
            .request(self.method, &self.path)
            .map_err(Error::Request)?;
        if let Some(content_type) = self.content_type.as_deref() {
            request_builder = request_builder.header(reqwest::header::CONTENT_TYPE, content_type);
        }
        if let Some(body) = self.body {
            debug_assert!(self.content_type.is_some());
            request_builder = request_builder.body(body);
        }
        let response = request_builder
            .send()
            .await
            .context("failed to send")
            .map_err(Error::Transport)?;
        let response = Response {
            status: response.status(),
            body: response.text().await,
        };

        if let Some(file) = response_file {
            // PANICS:
            // Unwrapping is acceptable when a cassette is provided as stated in the docstring and
            // `response_file` is set only when a cassette is provided.
            debug!("Writing response to the cassette file {file:?}");
            response.write(&file).unwrap();
        }

        Ok(response)
    }
}

#[derive(Debug)]
pub struct Response {
    pub status: StatusCode,
    pub body: Result<String, reqwest::Error>,
}

impl Response {
    fn read(file: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(file)
            .with_context(|| format!("Could not read the file {file:?}"))?;
        let (status, body) = content
            .split_once("\n\n")
            .context("Could not split response")?;
        let code = status
            .split_whitespace()
            .next()
            .context("Could not get status code")?;

        Ok(Self {
            status: StatusCode::from_str(code).context("Could not parse status code")?,
            body: Ok(body.to_string()),
        })
    }

    fn write(&self, file: &Path) -> anyhow::Result<()> {
        let Self { status, body } = self;
        let Ok(body) = body else {
            unimplemented!();
        };
        let content = format!("{status}\n\n{body}");

        let () = create_dir_all(file.parent().context("the file has no parent")?)?;
        let () = fs::write(file, content)
            .with_context(|| format!("Could not write the file {file:?}"))?;

        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Mode {
    Read,
    Write,
}

#[derive(Clone, Debug)]
pub struct Cassette {
    dir: PathBuf,
    request_number: i8,
    mode: Mode,
}

impl Cassette {
    pub fn new(dir: PathBuf, mode: Mode) -> Self {
        Self {
            dir,
            request_number: -1,
            mode,
        }
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        match fs::remove_dir_all(self.dir.as_path()) {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

impl Cassette {
    fn advance(&mut self) {
        self.request_number += 1;
    }

    fn request_file(&self, checksum: u64) -> PathBuf {
        self.dir.join(format!(
            "{:>03}-{checksum:016x}-request",
            self.request_number
        ))
    }

    fn response_file(&self, checksum: u64) -> PathBuf {
        self.dir.join(format!(
            "{:>03}-{checksum:016x}-response",
            self.request_number
        ))
    }
}
