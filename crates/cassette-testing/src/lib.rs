//! Cassette recording and playback infrastructure for rs4a-vapix.
//!
//! Record HTTP request/response sequences against real devices, then replay them
//! in tests without network access.

mod cassette;
mod library;
mod serde;

pub use cassette::{Cassette, CassetteClient};
pub use library::{DeviceInfo, Library};
