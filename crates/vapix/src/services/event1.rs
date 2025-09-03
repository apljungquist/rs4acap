//! The [event service API].
//!
//! [event service API]: https://developer.axis.com/vapix/network-video/event-and-action-services
use std::str::FromStr;

use quick_xml::{events::Event, Reader};

use crate::{
    soap::{Body, RequestBuilder2},
    Client,
};

#[derive(Debug)]
pub struct MessageInstance {
    pub topic: Vec<String>,
}
#[derive(Debug)]
pub struct EventInstances {
    pub message_instances: Vec<MessageInstance>,
}

impl FromStr for EventInstances {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
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

pub struct Event1 {
    client: Client,
}

impl Event1 {
    pub fn get_event_instances(self) -> RequestBuilder2<EventInstances> {
        RequestBuilder2 {
            client: self.client,
            path: "vapix/services",
            body: Body::new("http://www.axis.com/vapix/ws/event1", "GetEventInstances"),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl Client {
    pub fn event1(&self) -> Event1 {
        Event1 {
            client: self.clone(),
        }
    }
}
