//! The [Recording group API].
//!
//! [Recording group API]: https://developer.axis.com/vapix/device-configuration/recording-group/
use serde::Deserialize;

use crate::{rest::RequestBuilder, Client};

#[derive(Debug, Deserialize)]
pub struct CreateRecordingGroupResponse {
    pub id: String,
}

pub struct RecordingGroup1 {
    client: Client,
}

impl RecordingGroup1 {
    pub fn create(self) -> RequestBuilder<CreateRecordingGroupResponse> {
        RequestBuilder::new(
            self.client,
            "config/rest/recording-group/v2beta/recordingGroups",
        )
    }
}

impl Client {
    pub fn recording_group_1(&self) -> RecordingGroup1 {
        RecordingGroup1 {
            client: self.clone(),
        }
    }
}
