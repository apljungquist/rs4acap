//! Facilities for:
//! 1. Getting a [`SessionCookie`] used to authenticate with Axis APIs.
//! 2. Persisting the cookie and making it sharing it among crates.
//!
//! The login flow starts with [`AuthenticationFlow::start`].
//! Beware that this flow is fragile and often broken.
use std::fmt::Formatter;

use anyhow::bail;
use reqwest::header::HeaderValue;

mod login;
mod store;

pub use self::{
    login::{AuthenticationFlow, OneTimePasswordForm, UsernamePasswordForm},
    store::CookieStore,
};

const SID_COOKIE_PREFIX: &str = "axis_connect_session_sid=";

/// The cookie that grants access to most many Axis APIs,
/// including most VLT APIs.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SessionCookie(HeaderValue);

impl SessionCookie {
    pub fn into_header_value(self) -> HeaderValue {
        self.0
    }
}

impl std::fmt::Display for SessionCookie {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // PANICS:
        // The unwrap will never panic because the constructors ensure that the header value was
        // constructed from a string,
        self.0.to_str().unwrap().fmt(f)
    }
}

impl std::str::FromStr for SessionCookie {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().trim_end_matches(';');

        if !s.starts_with(SID_COOKIE_PREFIX) {
            bail!("Expected cookie to start with {SID_COOKIE_PREFIX}, but got {s}");
        }
        Ok(Self(HeaderValue::from_str(s)?))
    }
}
