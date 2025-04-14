//! The [Systemready API].
//!
//! [Systemready API]: https://developer.axis.com/vapix/network-video/systemready-api/

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{client::Client, json_rpc};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EnglishBoolean {
    Yes,
    No,
}

impl Display for EnglishBoolean {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EnglishBoolean::Yes => write!(f, "yes"),
            EnglishBoolean::No => write!(f, "no"),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemreadyData {
    pub needsetup: EnglishBoolean,
    pub systemready: EnglishBoolean,
}

pub struct SystemReady1 {
    client: Client,
}

impl SystemReady1 {
    pub fn system_ready(self) -> json_rpc::RequestBuilder<SystemreadyData> {
        json_rpc::RequestBuilder {
            client: self.client,
            path: "axis-cgi/systemready.cgi",
            json: json!({
                "method": "systemready",
                "apiVersion": "1",
            }),
            _phantom: Default::default(),
        }
    }
}
impl Client {
    pub fn system_ready_1(&self) -> SystemReady1 {
        SystemReady1 {
            client: self.clone(),
        }
    }
}
