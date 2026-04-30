//! Utilities for building requests and parsing responses that follow some common patterns.
//!
//! These typically have two layers (or three):
//! - Naive:
//!   - Must not be `async`.
//!   - Must operate on the request and response bodies only so that they in theory could use a
//!     transport other than HTTP.
//! - Transport aware (`*_http`):
//!   - Must not be `async`.
//!   - May leverage non-body parts of requests and responses, such as the status.
//!   - May depend on [`crate::http::Request`] and [`crate::http::Response`].
//! - Client aware:
//!   - May be async and depend on [`crate::http::HttpClient`].

pub mod http;
pub mod json_rpc;
pub mod json_rpc_http;
pub mod rest;
pub mod rest_http;
pub mod soap;
pub mod soap_http;
