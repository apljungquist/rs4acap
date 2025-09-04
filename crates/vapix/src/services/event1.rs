//! The [event service API].
//!
//! [event service API]: https://developer.axis.com/vapix/network-video/event-and-action-services

use quick_xml::{events::Event, Reader};

use crate::{soap::SimpleRequest, soap_http::SoapResponse};

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

pub fn get_event_instances() -> SimpleRequest<EventInstances> {
    SimpleRequest::new("http://www.axis.com/vapix/ws/event1", "GetEventInstances")
}
