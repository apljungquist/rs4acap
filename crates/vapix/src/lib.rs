pub mod apis;
mod axis_cgi;
mod client;
pub mod client_blocking;
mod config;
pub mod json_rpc;
pub mod json_rpc_http;
pub mod json_rpc_http_blocking;
pub mod rest;
pub mod rest_http;
mod services;
pub mod soap;
pub mod soap_http;

pub use axis_cgi::{basic_device_info_1, system_ready_1};
pub use client::{authorization_headers, Client, ClientBuilder, Scheme};
pub use config::{recording_group_1, remote_object_storage_1, ssh_1};
pub use services::{action1, event1};
