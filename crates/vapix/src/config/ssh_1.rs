//! The SSH v1 API.
//!
//! Note that there is also a [v2].
//! However, v1 may still be the only API that can be used to manage SSH users on 11.x.
//!
//! [v2]: https://developer.axis.com/vapix/device-configuration/ssh-management/
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    http::{Error, HttpClient, Request},
    rest, rest_http,
};

#[derive(Serialize)]
pub struct AddUserRequest {
    comment: Option<String>,
    password: String,
    username: String,
}

impl AddUserRequest {
    /// Sets the full name or the comment of the SSH user.
    ///
    /// Must be no longer than 256 and must match `^[^:\n]*$`.
    pub fn comment(mut self, comment: impl ToString) -> Self {
        self.comment = Some(comment.to_string());
        self
    }

    pub fn into_request(self) -> Request {
        let body = serde_json::to_string_pretty(&json!({"data": self})).unwrap();
        Request::new(Method::POST, "config/rest/ssh/v1/users".to_string()).json(body)
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<AddUserResponse, Error<rest::Error>> {
        rest_http::send_request(client, self.into_request()).await
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AddUserResponse {
    /// Returned on AXIS OS 11.11.192.
    /// The data field is absent from the response.
    Null(()),
    /// Returned on AXIS OS 11.11.73.
    /// The string is usually empty.
    None(String),
    /// Returned on AXIS OS 12.7.61.
    Echo { comment: String, username: String },
}

#[derive(Serialize)]
pub struct SetUserProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
}

pub struct SetUserRequest {
    properties: SetUserProperties,
    username: String,
}

impl SetUserRequest {
    // TODO: Figure out how the config API measures length

    /// Sets the password of the SSH user.
    ///
    /// Must be no shorter than 1 and no longer than 256.
    pub fn password(mut self, password: impl ToString) -> Self {
        self.properties.password = Some(password.to_string());
        self
    }

    /// Sets the full name or the comment of the SSH user.
    ///
    /// Must be no longer than 256 and must match `^[^:\n]*$`.
    pub fn comment(mut self, comment: impl ToString) -> Self {
        self.properties.comment = Some(comment.to_string());
        self
    }

    pub fn into_request(self) -> Request {
        let Self {
            properties,
            username,
        } = self;
        let path = format!("config/rest/ssh/v1/users/{username}");
        let body = serde_json::to_string_pretty(&json!({"data": properties})).unwrap();
        Request::new(Method::PATCH, path).json(body)
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<SetUserResponse, Error<rest::Error>> {
        rest_http::send_request(client, self.into_request()).await
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SetUserResponse(());

pub struct DeleteUserRequest {
    username: String,
}

impl DeleteUserRequest {
    pub fn into_request(self) -> Request {
        Request::new(
            Method::DELETE,
            format!("config/rest/ssh/v1/users/{}", self.username),
        )
    }

    pub async fn send(self, client: &(impl HttpClient + Sync)) -> Result<(), Error<rest::Error>> {
        rest_http::send_request(client, self.into_request()).await
    }
}

// TODO: Consider creating new types for comment, username, and password.

/// Creates a new user.
///
/// # Arguments
///
/// - `username` no shorter than 1, no longer than 32 and matching `^[a-z_][a-z0-9-_]*[$]?$`.
/// - `password` shorter than 1 and no longer than 256.
pub fn add_user(username: impl ToString, password: impl ToString) -> AddUserRequest {
    AddUserRequest {
        comment: None,
        password: password.to_string(),
        username: username.to_string(),
    }
}

/// Updates an existing user.
///
/// # Arguments
///
/// - `username` name of the user to update.
pub fn set_user(username: impl ToString) -> SetUserRequest {
    SetUserRequest {
        properties: SetUserProperties {
            password: None,
            comment: None,
        },
        username: username.to_string(),
    }
}

/// Deletes an existing user.
///
/// # Arguments
///
/// - `username` name of the user to delete.
pub fn delete_user(username: impl ToString) -> DeleteUserRequest {
    DeleteUserRequest {
        username: username.to_string(),
    }
}
