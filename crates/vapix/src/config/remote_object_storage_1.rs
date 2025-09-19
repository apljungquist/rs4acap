//! The remote object storage API (beta)
use serde::Deserialize;

use crate::rest_http::RequestBuilder;

#[derive(Debug, Deserialize)]
pub struct CreateDestinationResponse {
    pub id: String,
}

pub fn create_destinations() -> RequestBuilder<CreateDestinationResponse> {
    RequestBuilder::new("config/rest/remote-object-storage/v1beta/destinations")
}
