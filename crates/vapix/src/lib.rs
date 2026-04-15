pub mod apis;
mod axis_cgi;
pub mod cassette;
mod client;
mod config;
pub mod http;
pub mod json_rpc;
pub(crate) mod json_rpc_http;
pub mod rest;
pub mod rest_http;
mod services;
pub mod soap;
pub mod soap_http;

pub use axis_cgi::{
    applications_config, basic_device_info_1, firmware_management_1, parameter_management, pwdgrp,
    system_ready_1,
};
pub use client::{Client, ClientBuilder, Scheme};
pub use config::{
    recording_group_1, remote_object_storage_1, remote_object_storage_1_beta,
    siren_and_light_2_alpha, ssh_1,
};
pub use services::{action1, event1};
