//! The [event service API].
//!
//! [event service API]: https://developer.axis.com/vapix/network-video/event-and-action-services

use std::convert::Infallible;

use quick_xml::{events::Event, Reader};

use crate::{
    http::{Error, HttpClient, Request},
    soap, soap_http,
    soap_http::SoapResponse,
};

const PATH: &str = "vapix/services";

#[derive(Debug)]
pub struct MessageInstance {
    pub topic: Vec<String>,
}
#[derive(Debug)]
pub struct EventInstances {
    pub message_instances: Vec<MessageInstance>,
}

impl SoapResponse for EventInstances {
    fn from_envelope(s: &str) -> anyhow::Result<Self> {
        let mut message_instances = Vec::new();
        let mut reader = Reader::from_str(s);
        let mut stack: Vec<String> = Vec::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    stack.push(name.clone());

                    if name == "aev:MessageInstance" {
                        let topic: Vec<String> = stack
                            .iter()
                            .skip_while(|n| n.as_str() != "wstop:TopicSet") // skip until TopicSet
                            .skip(1)
                            .take_while(|n| n.as_str() != "aev:MessageInstance")
                            .map(|n| n.split(':').next_back().unwrap().to_string()) // strip namespace prefix
                            .collect();

                        message_instances.push(MessageInstance { topic });
                    }
                }
                Event::End(_) => {
                    stack.pop();
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        Ok(Self { message_instances })
    }
}

#[derive(Debug)]
pub struct GetEventInstancesRequest;

impl GetEventInstancesRequest {
    pub fn into_envelope(self) -> String {
        soap::envelope(
            "http://www.axis.com/vapix/ws/event1",
            "GetEventInstances",
            None,
        )
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<EventInstances, Error<Infallible>> {
        let request =
            Request::new(reqwest::Method::POST, PATH.to_string()).soap(self.into_envelope());
        soap_http::send_request(client, request).await
    }
}

pub fn get_event_instances() -> GetEventInstancesRequest {
    GetEventInstancesRequest
}
