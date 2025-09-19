//! The SSH v1 API.
//!
//! Note that there is also a [v2].
//! However, v1 may still be the only API that can be used to manage SSH users on 11.x.
//!
//! [v2]: https://developer.axis.com/vapix/device-configuration/ssh-management/
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::rest::RestHttp;

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
}

impl RestHttp for AddUserRequest {
    type RequestData = AddUserRequest;
    type ResponseData = AddUserResponse;
    const METHOD: Method = Method::POST;

    fn to_path_and_data(self) -> anyhow::Result<(String, Self::RequestData)> {
        Ok(("config/rest/ssh/v1/users".to_string(), self))
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct AddUserResponse(String);

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
}

impl RestHttp for SetUserRequest {
    type RequestData = SetUserProperties;
    type ResponseData = SetUserResponse;
    // TODO: Figure out how to handle the change of method on >11.10
    const METHOD: Method = Method::PUT;

    fn to_path_and_data(self) -> anyhow::Result<(String, Self::RequestData)> {
        let Self {
            username,
            properties: data,
        } = self;
        // FIXME: Try non-URL safe characters
        Ok((format!("config/rest/ssh/v1/users/{username}"), data))
    }
}

#[derive(Debug, Deserialize)]
pub struct SetUserResponse(());

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
