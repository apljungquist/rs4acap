//! The [Recording group API].
//!
//! [Recording group API]: https://developer.axis.com/vapix/device-configuration/recording-group/
use serde::{Deserialize, Serialize};

use crate::rest_http::RequestBuilder;

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateRecordingGroupResponse {
    pub id: String,
}

pub fn create_recording_groups() -> RequestBuilder<CreateRecordingGroupResponse> {
    RequestBuilder::new("config/rest/recording-group/v2beta/recordingGroups")
}
