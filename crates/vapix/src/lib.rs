mod axis_cgi;
mod client;
pub mod json_rpc;
mod services;
pub mod soap;
pub use axis_cgi::{basic_device_info_1, system_ready_1};
pub use client::{Client, ClientBuilder, Scheme};
pub use services::{action1, event1};
