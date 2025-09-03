mod axis_cgi;
mod client;
mod config;
pub mod json_rpc;
mod rest;
mod services;
pub mod soap;

pub use axis_cgi::{basic_device_info_1, system_ready_1};
pub use client::{Client, ClientBuilder, Scheme};
pub use config::{recording_group_1, remote_object_storage_1};
pub use services::{action1, event1};
