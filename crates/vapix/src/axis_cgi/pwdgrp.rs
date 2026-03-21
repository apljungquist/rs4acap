//! The user management CGI at `axis-cgi/pwdgrp.cgi`.

use std::fmt::{Display, Formatter};

use anyhow::{bail, Context};
use reqwest::StatusCode;

use crate::Client;

fn extract_body(html: &str) -> Option<&str> {
    let body_start = html.find("<body")?;
    let content_start = html[body_start..].find('>')? + body_start + 1;
    let content_end = html[content_start..].find("</body>")? + content_start;
    Some(&html[content_start..content_end])
}

#[derive(Clone, Copy, Debug)]
pub enum Role {
    Viewer,
    OperatorViewer,
    AdminOperatorViewerPtz,
}

impl Display for Role {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Viewer => write!(f, "viewer"),
            Role::OperatorViewer => write!(f, "operator:viewer"),
            Role::AdminOperatorViewerPtz => write!(f, "admin:operator:viewer:ptz"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Group {
    Root,
    Users,
}

impl Display for Group {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Group::Root => write!(f, "root"),
            Group::Users => write!(f, "users"),
        }
    }
}

const PATH: &str = "axis-cgi/pwdgrp.cgi";

pub async fn add_user(
    client: &Client,
    username: &str,
    password: &str,
    group: Group,
    role: Role,
) -> anyhow::Result<()> {
    let role = role.to_string();
    let group = group.to_string();
    let query = [
        ("action", "add"),
        ("user", username),
        ("pwd", password),
        ("grp", group.as_str()),
        ("sgrp", role.as_str()),
    ];
    let resp = client
        .get(PATH)?
        .query(&query)
        .send()
        .await?
        .error_for_status()?;
    let status = resp.status();
    let text = resp.text().await?;
    let body = extract_body(&text).context(text.clone())?;
    let expected = format!("Created account {username}.");
    if status != StatusCode::OK || body.trim() != expected {
        bail!("Unexpected status and/or body: {status} {body:?}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::extract_body;

    #[test]
    fn extract_body_simple() {
        assert_eq!(
            extract_body("<html><body>Created account root.</body></html>")
                .unwrap()
                .trim(),
            "Created account root."
        );
    }

    #[test]
    fn extract_body_with_attributes() {
        assert_eq!(
            extract_body("<html><body class=\"foo\">Created account root.</body></html>")
                .unwrap()
                .trim(),
            "Created account root."
        );
    }
}
