//! Record and replay sequences of HTTP requests and responses.
use std::{
    fs,
    fs::create_dir_all,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Mutex,
};

use anyhow::Context;
use log::debug;
use reqwest::StatusCode;

use crate::{
    http::{HttpClient, Request, Response},
    Client,
};

// TODO: Move the cassette infrastructure out of the vapix library crate.

fn write_request(request: &Request, file: &Path) -> anyhow::Result<()> {
    let () = create_dir_all(file.parent().context("the file has no parent")?)?;
    let mut content = format!("{} {}\n", request.method, request.path);
    if let Some(content_type) = &request.content_type {
        content.push_str(&format!("Content-Type: {content_type}\n"));
    }
    if let Some(body) = &request.body {
        content.push_str(&format!("\n{}", String::from_utf8_lossy(body)));
    }
    let () = fs::write(file, content).with_context(|| format!("Could not write file {file:?}"))?;
    Ok(())
}

fn request_checksum(request: &Request) -> u64 {
    // The `DefaultHasher` is not guaranteed to be stable across rust versions.
    // If it changes, all cassettes tracked in VCS will be invalidated.
    // TODO: Use a stable hashing algorithm.

    let mut hasher = DefaultHasher::new();
    request.method.hash(&mut hasher);
    request.path.hash(&mut hasher);
    if let Some(body) = &request.body {
        // Hash as str to stay compatible with cassettes recorded when body was String.
        hasher.write(body);
        hasher.write_u8(0xff);
    }
    request.content_type.hash(&mut hasher);
    hasher.finish()
}

fn read_response(file: &Path) -> anyhow::Result<Response> {
    let content =
        fs::read_to_string(file).with_context(|| format!("Could not read the file {file:?}"))?;
    let (status, body) = content
        .split_once("\n\n")
        .context("Could not split response")?;
    let code = status
        .split_whitespace()
        .next()
        .context("Could not get status code")?;

    Ok(Response {
        status: StatusCode::from_str(code).context("Could not parse status code")?,
        body: Ok(body.to_string()),
    })
}

fn write_response(response: &Response, file: &Path) -> anyhow::Result<()> {
    let Ok(body) = &response.body else {
        unimplemented!();
    };
    let content = format!("{}\n\n{body}", response.status);

    let () = create_dir_all(file.parent().context("the file has no parent")?)?;
    let () =
        fs::write(file, content).with_context(|| format!("Could not write the file {file:?}"))?;

    Ok(())
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

pub struct CassetteClient {
    inner: Option<Client>,
    cassette: Mutex<Cassette>,
}

impl CassetteClient {
    pub fn for_playback(cassette: Cassette) -> Self {
        assert!(matches!(cassette.mode, Mode::Read));
        Self {
            inner: None,
            cassette: Mutex::new(cassette),
        }
    }

    pub fn for_recording(client: Client, cassette: Cassette) -> Self {
        assert!(matches!(cassette.mode, Mode::Write));
        Self {
            inner: Some(client),
            cassette: Mutex::new(cassette),
        }
    }
}

impl HttpClient for CassetteClient {
    async fn execute(&self, request: Request) -> Result<Response, anyhow::Error> {
        let (mode, req_file, resp_file) = {
            let mut cassette = self.cassette.lock().unwrap();
            cassette.advance();
            let checksum = request_checksum(&request);
            let req_file = cassette.request_file(checksum);
            let resp_file = cassette.response_file(checksum);
            (cassette.mode, req_file, resp_file)
        };

        match mode {
            Mode::Read => {
                debug!("Reading response from the cassette file {resp_file:?}");
                Ok(read_response(&resp_file).unwrap())
            }
            Mode::Write => {
                debug!("Writing request to the cassette file {req_file:?}");
                write_request(&request, &req_file).unwrap();
                let response = self.inner.as_ref().unwrap().execute(request).await?;
                debug!("Writing response to the cassette file {resp_file:?}");
                write_response(&response, &resp_file).unwrap();
                Ok(response)
            }
        }
    }
}
