//! A gallery of bindings for various APIs

mod axis_cgi;
mod config;
mod services;
pub use axis_cgi::{
    api_discovery_1, applications_config, basic_device_info_1, firmware_management_1, jpg_3,
    network_settings_1, parameter_management, pwdgrp, system_ready_1,
};
pub use config::{
    discover, recording_group_1, remote_object_storage_1_beta, siren_and_light_2_alpha, ssh_1,
};
pub use services::{action1, event1};
