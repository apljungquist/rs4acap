#![allow(clippy::indexing_slicing, reason = "TODO: Improve this")]

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::Mutex,
};

use anyhow::Context;
use regex::Regex;
use rs4a_vapix::{
    http::{HttpClient, Request, Response},
    Client,
};

use crate::serde::{parse_response, serialize_request, serialize_response};

// TODO: Use a stable hashing algorithm; DefaultHasher is not guaranteed
// to be stable across Rust versions, invalidating VCS-tracked cassettes.
fn request_checksum(request: &Request) -> u64 {
    let mut hasher = DefaultHasher::new();
    request.method.hash(&mut hasher);
    request.path.hash(&mut hasher);
    if let Some(body) = &request.body {
        hasher.write(body);
        hasher.write_u8(0xff);
    }
    request.content_type.hash(&mut hasher);
    hasher.finish()
}

#[derive(Clone, Debug)]
struct Track {
    checksum: u64,
    request: String,
    response: String,
}

/// An in-memory recording of HTTP request/response pairs.
#[derive(Clone, Debug)]
pub struct Cassette {
    tracks: Vec<Track>,
    substitutions: &'static [(&'static str, &'static str)],
    cursor: usize,
}

impl Cassette {
    /// Create an empty cassette for recording.
    pub fn new(substitutions: &'static [(&'static str, &'static str)]) -> Self {
        Self {
            tracks: Vec::new(),
            substitutions,
            cursor: 0,
        }
    }

    pub(crate) fn loaded(tracks: Vec<(u64, String, String)>) -> Self {
        Self {
            tracks: tracks
                .into_iter()
                .map(|(checksum, request, response)| Track {
                    checksum,
                    request,
                    response,
                })
                .collect(),
            substitutions: &[],
            cursor: 0,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// Iterate over (checksum, request, response) with substitutions applied to responses.
    pub(crate) fn normalized_tracks(&self) -> anyhow::Result<Vec<(u64, &str, String)>> {
        let responses = normalize_strings(
            &self
                .tracks
                .iter()
                .map(|t| t.response.clone())
                .collect::<Vec<_>>(),
            self.substitutions,
        )?;
        Ok(self
            .tracks
            .iter()
            .zip(responses)
            .map(|(t, resp)| (t.checksum, t.request.as_str(), resp))
            .collect())
    }

    fn record(&mut self, checksum: u64, request: String, response: String) {
        self.tracks.push(Track {
            checksum,
            request,
            response,
        });
    }

    fn play(&mut self, actual_request: &str) -> anyhow::Result<&str> {
        let idx = self.cursor;
        anyhow::ensure!(
            idx < self.tracks.len(),
            "Cassette exhausted: tried to read response {idx} but only {} recorded",
            self.tracks.len()
        );
        anyhow::ensure!(
            self.tracks[idx].request == actual_request,
            "Request mismatch at position {idx}:\nexpected:\n{}\nactual:\n{actual_request}",
            self.tracks[idx].request,
        );
        self.cursor += 1;
        Ok(&self.tracks[idx].response)
    }
}

pub struct CassetteClient {
    inner: Option<Client>,
    cassette: Mutex<Cassette>,
}

impl CassetteClient {
    /// Create a playback client that replays responses from a loaded cassette.
    pub fn for_playback(cassette: Cassette) -> Self {
        Self {
            inner: None,
            cassette: Mutex::new(cassette),
        }
    }

    /// Create a recording client that captures requests and responses in memory.
    pub fn for_recording(client: Client, cassette: Cassette) -> Self {
        Self {
            inner: Some(client),
            cassette: Mutex::new(cassette),
        }
    }

    /// Snapshot the recorded cassette data.
    pub fn take_cassette(&self) -> Cassette {
        self.cassette.lock().unwrap().clone()
    }
}

impl std::fmt::Debug for CassetteClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            None => f
                .debug_struct("CassetteClient")
                .field("mode", &"playback")
                .finish(),
            Some(_) => f
                .debug_struct("CassetteClient")
                .field("mode", &"recording")
                .finish(),
        }
    }
}

impl HttpClient for CassetteClient {
    async fn execute(&self, request: Request) -> Result<Response, anyhow::Error> {
        let checksum = request_checksum(&request);
        let serialized_request = serialize_request(&request);
        match &self.inner {
            None => {
                let response_str = self
                    .cassette
                    .lock()
                    .unwrap()
                    .play(&serialized_request)?
                    .to_string();
                parse_response(&response_str)
            }
            Some(client) => {
                let response = client.execute(request).await?;
                let serialized_response = serialize_response(&response)?;
                self.cassette.lock().unwrap().record(
                    checksum,
                    serialized_request,
                    serialized_response,
                );
                Ok(response)
            }
        }
    }
}

fn normalize_strings(
    strings: &[String],
    substitutions: &[(&str, &str)],
) -> anyhow::Result<Vec<String>> {
    if substitutions.is_empty() {
        return Ok(strings.to_vec());
    }

    let patterns: Vec<(Regex, &str)> = substitutions
        .iter()
        .map(|(pattern, replacement)| {
            let re =
                Regex::new(pattern).with_context(|| format!("Invalid regex pattern: {pattern}"))?;
            Ok((re, *replacement))
        })
        .collect::<anyhow::Result<_>>()?;

    Ok(strings
        .iter()
        .map(|s| {
            let mut result = s.clone();
            for (re, replacement) in &patterns {
                result = re.replace_all(&result, *replacement).into_owned();
            }
            result
        })
        .collect())
}
