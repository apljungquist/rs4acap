//! The [User Management] CGI.
//!
//! [User Management]: https://developer.axis.com/vapix/network-video/user-management/

use std::fmt::{Display, Formatter};

use reqwest::{Method, StatusCode};

use crate::{cassette::Request, http::Error, Client};

const PATH: &str = "axis-cgi/pwdgrp.cgi";

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
        Request::no_content(
            Method::GET,
            format!(
                "{PATH}?action=add&user={}&pwd={}&grp={}&sgrp={}",
                self.username, self.password, group, role
            ),
        )
    }

    pub async fn send(self, client: &Client) -> Result<(), Error<std::convert::Infallible>> {
        let expected = format!("Created account {}.", self.username);
        let response = self.into_request().send(client, None).await?;
        let body = response.body.map_err(|e| Error::Transport(e.into()))?;
        if response.status == StatusCode::OK {
            let html_body = extract_body(&body).unwrap_or("");
            if html_body.trim() == expected {
                return Ok(());
            }
        }
        Err(Error::Decode(anyhow::anyhow!(
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
