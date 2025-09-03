//! The remote object storage API (beta)
use serde::Deserialize;

use crate::{rest::RequestBuilder, Client};

#[derive(Debug, Deserialize)]
pub struct CreateDestinationResponse {
    pub id: String,
}

pub struct RemoteObjectStorage1 {
    client: Client,
}

impl RemoteObjectStorage1 {
    pub fn create(self) -> RequestBuilder<CreateDestinationResponse> {
        RequestBuilder::new(
            self.client,
            "config/rest/remote-object-storage/v1beta/destinations",
        )
    }
}

impl Client {
    pub fn remote_object_storage_1(&self) -> RemoteObjectStorage1 {
        RemoteObjectStorage1 {
            client: self.clone(),
        }
    }
}
