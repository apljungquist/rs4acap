//! The [User Management] CGI.
//!
//! [User Management]: https://developer.axis.com/vapix/network-video/user-management/

use std::fmt::{Display, Formatter};

use reqwest::{Method, StatusCode};

use crate::{
    http::{HttpClient, Request},
    protocol_helpers::http::Error as HttpError,
};

const PATH: &str = "axis-cgi/pwdgrp.cgi";

fn extract_body(html: &str) -> Option<&str> {
    let body_start = html.find("<body")?;
    let content_start = html[body_start..].find('>')? + body_start + 1;
    let content_end = html[content_start..].find("</body>")? + content_start;
    Some(&html[content_start..content_end])
}

/// An error returned by the user management CGI.
#[derive(Clone, Debug)]
pub struct Error {
    message: String,
}

impl Error {
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

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

#[derive(Clone, Debug)]
pub struct AddUserRequest {
    username: String,
    password: String,
    group: Group,
    role: Role,
}

impl AddUserRequest {
    pub fn new(username: &str, password: &str, group: Group, role: Role) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
            group,
            role,
        }
    }

    fn into_request(self) -> Request {
        let group = self.group.to_string();
        let role = self.role.to_string();
        Request::new(
            Method::GET,
            format!(
                "{PATH}?action=add&user={}&pwd={}&grp={}&sgrp={}",
                self.username, self.password, group, role
            ),
        )
    }

    pub async fn send(self, client: &impl HttpClient) -> Result<(), HttpError<Error>> {
        let expected = format!("Created account {}.", self.username);
        let response = client
            .execute(self.into_request())
            .await
            .map_err(HttpError::Transport)?;
        let body = response.body.map_err(|e| HttpError::Transport(e.into()))?;
        let html_body = extract_body(&body).unwrap_or("");
        let trimmed = html_body.trim();
        if let Some(message) = trimmed.strip_prefix("Error: ") {
            let message = message.strip_suffix('.').unwrap_or(message);
            return Err(HttpError::Service(Error {
                message: message.to_string(),
            }));
        }
        if response.status == StatusCode::OK && trimmed == expected {
            return Ok(());
        }
        Err(HttpError::Decode(anyhow::anyhow!(
            "Unexpected response: {} {}",
            response.status,
            body.trim()
        )))
    }
}

#[derive(Clone, Debug)]
pub struct RemoveUserRequest {
    username: String,
}

impl RemoveUserRequest {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_string(),
        }
    }

    fn into_request(self) -> Request {
        Request::new(
            Method::GET,
            format!("{PATH}?action=remove&user={}", self.username),
        )
    }

    pub async fn send(self, client: &impl HttpClient) -> Result<(), HttpError<Error>> {
        let expected = format!("Removed account {}.", self.username);
        let response = client
            .execute(self.into_request())
            .await
            .map_err(HttpError::Transport)?;
        let body = response.body.map_err(|e| HttpError::Transport(e.into()))?;
        let html_body = extract_body(&body).unwrap_or("");
        let trimmed = html_body.trim();
        if let Some(message) = trimmed.strip_prefix("Error: ") {
            let message = message.strip_suffix('.').unwrap_or(message);
            return Err(HttpError::Service(Error {
                message: message.to_string(),
            }));
        }
        if response.status == StatusCode::OK && trimmed == expected {
            return Ok(());
        }
        Err(HttpError::Decode(anyhow::anyhow!(
            "Unexpected response: {} {}",
            response.status,
            body.trim()
        )))
    }
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
