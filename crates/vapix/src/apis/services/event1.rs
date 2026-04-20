//! The [event service API].
//!
//! [event service API]: https://developer.axis.com/vapix/network-video/event-and-action-services

use std::convert::Infallible;

use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::{
    http::{HttpClient, Request},
    protocol_helpers::{http::Error, soap, soap_http, soap_http::SoapResponse},
};

const PATH: &str = "vapix/services";

#[derive(Debug, Default)]
pub struct SimpleItemDeclaration {
    pub name: String,
    pub value_type: String,
    pub values: Vec<String>,
    pub is_property_state: bool,
}

#[derive(Debug, Default)]
pub struct MessageInstance {
    pub topic: Vec<String>,
    pub is_property: bool,
    pub source: Vec<SimpleItemDeclaration>,
    pub key: Vec<SimpleItemDeclaration>,
    pub data: Vec<SimpleItemDeclaration>,
}

#[derive(Debug)]
pub struct EventInstances {
    pub message_instances: Vec<MessageInstance>,
}

#[derive(Copy, Clone)]
enum Section {
    Source,
    Key,
    Data,
}

fn attr_value(e: &BytesStart, key: &str) -> Option<String> {
    e.attributes().filter_map(|a| a.ok()).find_map(|a| {
        (a.key.as_ref() == key.as_bytes())
            .then(|| String::from_utf8_lossy(a.value.as_ref()).into_owned())
    })
}

impl SoapResponse for EventInstances {
    fn from_envelope(s: &str) -> anyhow::Result<Self> {
        let mut message_instances: Vec<MessageInstance> = Vec::new();
        let mut reader = Reader::from_str(s);
        let mut stack: Vec<String> = Vec::new();
        let mut buf = Vec::new();
        let mut section: Option<Section> = None;
        let mut current_decl: Option<SimpleItemDeclaration> = None;
        let mut in_value = false;

        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    stack.push(name.clone());
                    handle_open(
                        &name,
                        &e,
                        &stack,
                        &mut message_instances,
                        &mut section,
                        &mut current_decl,
                        &mut in_value,
                    );
                }
                Event::Empty(e) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    stack.push(name.clone());
                    handle_open(
                        &name,
                        &e,
                        &stack,
                        &mut message_instances,
                        &mut section,
                        &mut current_decl,
                        &mut in_value,
                    );
                    handle_close(
                        &name,
                        &mut message_instances,
                        &mut section,
                        &mut current_decl,
                        &mut in_value,
                    );
                    stack.pop();
                }
                Event::End(_) => {
                    let name = stack.pop().unwrap_or_default();
                    handle_close(
                        &name,
                        &mut message_instances,
                        &mut section,
                        &mut current_decl,
                        &mut in_value,
                    );
                }
                Event::Text(t) if in_value => {
                    if let Some(decl) = current_decl.as_mut() {
                        let text = t.decode().map(|c| c.into_owned()).unwrap_or_default();
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            decl.values.push(trimmed.to_string());
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        Ok(Self { message_instances })
    }
}

fn handle_open(
    name: &str,
    e: &BytesStart,
    stack: &[String],
    message_instances: &mut Vec<MessageInstance>,
    section: &mut Option<Section>,
    current_decl: &mut Option<SimpleItemDeclaration>,
    in_value: &mut bool,
) {
    match name {
        "aev:MessageInstance" => {
            let topic: Vec<String> = stack
                .iter()
                .skip_while(|n| n.as_str() != "wstop:TopicSet")
                .skip(1)
                .take_while(|n| n.as_str() != "aev:MessageInstance")
                .map(|n| n.split(':').next_back().unwrap().to_string())
                .collect();
            let is_property = attr_value(e, "aev:isProperty").as_deref() == Some("true");
            message_instances.push(MessageInstance {
                topic,
                is_property,
                ..Default::default()
            });
        }
        "aev:SourceInstance" => *section = Some(Section::Source),
        "aev:KeyInstance" => *section = Some(Section::Key),
        "aev:DataInstance" => *section = Some(Section::Data),
        "aev:SimpleItemInstance" => {
            let mut decl = SimpleItemDeclaration::default();
            for a in e.attributes().flatten() {
                match a.key.as_ref() {
                    b"Name" => decl.name = String::from_utf8_lossy(&a.value).into_owned(),
                    b"Type" => decl.value_type = String::from_utf8_lossy(&a.value).into_owned(),
                    b"isPropertyState" => decl.is_property_state = a.value.as_ref() == b"true",
                    _ => {}
                }
            }
            *current_decl = Some(decl);
        }
        "aev:Value" => *in_value = true,
        _ => {}
    }
}

fn handle_close(
    name: &str,
    message_instances: &mut [MessageInstance],
    section: &mut Option<Section>,
    current_decl: &mut Option<SimpleItemDeclaration>,
    in_value: &mut bool,
) {
    match name {
        "aev:SourceInstance" | "aev:KeyInstance" | "aev:DataInstance" => *section = None,
        "aev:SimpleItemInstance" => {
            if let (Some(decl), Some(sec), Some(msg)) =
                (current_decl.take(), *section, message_instances.last_mut())
            {
                match sec {
                    Section::Source => msg.source.push(decl),
                    Section::Key => msg.key.push(decl),
                    Section::Data => msg.data.push(decl),
                }
            }
        }
        "aev:Value" => *in_value = false,
        _ => {}
    }
}

#[derive(Debug, Default)]
pub struct GetEventInstancesRequest;

impl GetEventInstancesRequest {
    pub fn new() -> Self {
        Self
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_topic_schema_and_attributes() {
        let envelope = r#"<?xml version="1.0" encoding="UTF-8"?>
<SOAP-ENV:Envelope xmlns:SOAP-ENV="http://www.w3.org/2003/05/soap-envelope"
                   xmlns:wstop="http://docs.oasis-open.org/wsn/t-1"
                   xmlns:tns1="http://www.onvif.org/ver10/topics"
                   xmlns:tnsaxis="http://www.axis.com/2009/event/topics"
                   xmlns:aev="http://www.axis.com/vapix/ws/event1"
                   xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <SOAP-ENV:Body>
    <aev:GetEventInstancesResponse>
      <wstop:TopicSet>
        <tns1:Device>
          <tnsaxis:IO>
            <Port aev:topic="true">
              <aev:MessageInstance aev:isProperty="true">
                <aev:SourceInstance>
                  <aev:SimpleItemInstance Name="port" Type="xsd:int">
                    <aev:Value>1</aev:Value>
                    <aev:Value>2</aev:Value>
                  </aev:SimpleItemInstance>
                </aev:SourceInstance>
                <aev:DataInstance>
                  <aev:SimpleItemInstance Name="active" Type="xsd:boolean"
                                         isPropertyState="true" />
                </aev:DataInstance>
              </aev:MessageInstance>
            </Port>
          </tnsaxis:IO>
        </tns1:Device>
      </wstop:TopicSet>
    </aev:GetEventInstancesResponse>
  </SOAP-ENV:Body>
</SOAP-ENV:Envelope>"#;

        let parsed = EventInstances::from_envelope(envelope).unwrap();

        assert_eq!(parsed.message_instances.len(), 1);
        let msg = &parsed.message_instances[0];
        assert_eq!(msg.topic, vec!["Device", "IO", "Port"]);
        assert!(msg.is_property);

        assert_eq!(msg.source.len(), 1);
        assert_eq!(msg.source[0].name, "port");
        assert_eq!(msg.source[0].value_type, "xsd:int");
        assert_eq!(msg.source[0].values, vec!["1", "2"]);
        assert!(!msg.source[0].is_property_state);

        assert!(msg.key.is_empty());

        assert_eq!(msg.data.len(), 1);
        assert_eq!(msg.data[0].name, "active");
        assert_eq!(msg.data[0].value_type, "xsd:boolean");
        assert!(msg.data[0].values.is_empty());
        assert!(msg.data[0].is_property_state);
    }
}
