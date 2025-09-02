mod axis_cgi;
mod client;
pub mod json_rpc;
pub use axis_cgi::{basic_device_info_1, system_ready_1};
pub use client::{Client, ClientBuilder, Scheme};
