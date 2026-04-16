use std::str::FromStr;

use anyhow::Context;
use reqwest::StatusCode;
use rs4a_vapix::http::{Request, Response};

pub(crate) fn serialize_request(request: &Request) -> String {
    let mut content = format!("{} {}\n", request.method, request.path);
    if let Some(content_type) = &request.content_type {
        content.push_str(&format!("Content-Type: {content_type}\n"));
    }
    if let Some(body) = &request.body {
        content.push_str(&format!("\n{}", String::from_utf8_lossy(body)));
    }
    content
}

pub(crate) fn serialize_response(response: &Response) -> anyhow::Result<String> {
    let body = response
        .body
        .as_ref()
        .map_err(|e| anyhow::anyhow!("cannot serialize error response: {e}"))?;
    Ok(format!("{}\n\n{body}", response.status))
}

pub(crate) fn parse_response(content: &str) -> anyhow::Result<Response> {
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
