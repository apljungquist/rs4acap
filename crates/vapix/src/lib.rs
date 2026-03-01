pub mod apis;
mod axis_cgi;
pub mod cassette;
mod client;
mod config;
pub mod http;
pub mod json_rpc;
pub mod json_rpc_http;
pub mod rest;
pub mod rest_http;
pub mod rest_http2;
mod services;
pub mod soap;
pub mod soap_http;

pub use axis_cgi::{basic_device_info_1, system_ready_1};
pub use client::{Client, ClientBuilder, Scheme};
pub use config::{
    recording_group_1, recording_group_2, remote_object_storage_1, remote_object_storage_1_beta,
    ssh_1,
};
pub use services::{action1, event1};
