//! The [Recording group API].
//!
//! [Recording group API]: https://developer.axis.com/vapix/device-configuration/recording-group/
use serde::Deserialize;

use crate::rest::RequestBuilder;

#[derive(Debug, Deserialize)]
pub struct CreateRecordingGroupResponse {
    pub id: String,
}

pub fn create_recording_groups() -> RequestBuilder<CreateRecordingGroupResponse> {
    RequestBuilder::new("config/rest/recording-group/v2beta/recordingGroups")
}
