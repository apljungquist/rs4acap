//! Utilities for working with SOAP style APIs.

use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use anyhow::{bail, Context};
use quick_xml::{events::Event, Reader};
use serde::Deserialize;

/// Error returned by SOAP-based APIs.
#[derive(Debug)]
pub struct Error {
    pub code: String,
    pub detail: String,
    pub reason: String,
}

impl Error {
    /// Parse the `detail` field as `T`.
    pub fn parse_detail_as<T: FromStr>(&self) -> Result<T, T::Err> {
        self.detail.parse::<T>()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Self {
            code,
            detail,
            reason,
        } = self;
        write!(f, "({code}/{detail}) {reason}")
    }
}

impl std::error::Error for Error {}

struct BodyChild<'a> {
    local_name: &'a [u8],
    /// Raw XML of the element, including its own start/end tags.
    xml: &'a str,
}

impl BodyChild<'_> {
    fn is_fault(&self) -> bool {
        self.local_name == b"Fault"
    }
}

/// Locate the first element child of `<Body>` and return its name and raw XML.
fn locate_body_child(xml: &str) -> anyhow::Result<BodyChild<'_>> {
    let mut reader = Reader::from_str(xml);
    let mut depth: u32 = 0;
    // depth at which `<Body>` was opened; `None` until then
    let mut body_depth: Option<u32> = None;
    // start position and local name of the body child, set on its start tag
    let mut child: Option<(usize, &[u8])> = None;

    loop {
        let pos_before = reader.buffer_position() as usize;
        let event = reader
            .read_event()
            .with_context(|| format!("could not read XML; text: {xml}"))?;
        let pos_after = reader.buffer_position() as usize;
        match event {
            Event::Start(_) => {
                depth += 1;
                let local_name = local_name_in_tag(xml[pos_before..pos_after].as_bytes());
                if child.is_some() {
                    // inside the body child; just keep walking until its End
                } else if body_depth.is_some_and(|bd| depth == bd + 1) {
                    child = Some((pos_before, local_name));
                } else if depth == 2 && local_name == b"Body" {
                    body_depth = Some(depth);
                }
            }
            Event::Empty(_) => {
                if child.is_none() && body_depth.is_some_and(|bd| depth + 1 == bd + 1) {
                    return Ok(BodyChild {
                        local_name: local_name_in_tag(xml[pos_before..pos_after].as_bytes()),
                        xml: &xml[pos_before..pos_after],
                    });
                }
            }
            Event::End(_) => {
                if let Some(bd) = body_depth {
                    if child.is_some() && depth == bd + 1 {
                        let (start, local_name) = child.take().unwrap();
                        return Ok(BodyChild {
                            local_name,
                            xml: &xml[start..pos_after],
                        });
                    }
                    if depth == bd {
                        body_depth = None;
                    }
                }
                depth = depth.saturating_sub(1);
            }
            Event::Eof => bail!("no element child found in <Body>; text: {xml}"),
            _ => {}
        }
    }
}

/// Extract the local name (after any `prefix:`) from a `<...>` or `<.../>` tag,
/// allowing leading whitespace before the `<`.
fn local_name_in_tag(tag: &[u8]) -> &[u8] {
    let after_lt = match tag.iter().position(|&b| b == b'<') {
        Some(p) => tag.split_at(p + 1).1,
        None => tag,
    };
    let name_end = after_lt
        .iter()
        .position(|&b| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'/' | b'>'))
        .unwrap_or(after_lt.len());
    let qualified = after_lt.split_at(name_end).0;
    match qualified.iter().rposition(|&b| b == b':') {
        Some(p) => qualified.split_at(p + 1).1,
        None => qualified,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct FaultShape {
    #[serde(default)]
    code: FaultCode,
    #[serde(default)]
    reason: FaultReason,
}

#[derive(Deserialize, Default)]
struct FaultCode {
    #[serde(rename = "Value", default)]
    value: String,
}

#[derive(Deserialize, Default)]
struct FaultReason {
    #[serde(rename = "Text", default)]
    text: FaultReasonText,
}

#[derive(Deserialize, Default)]
struct FaultReasonText {
    #[serde(rename = "$value", default)]
    value: String,
}

fn local_part(qualified: &str) -> &str {
    qualified.rsplit(':').next().unwrap_or(qualified)
}

/// Extract the local name of the first element child of `<Detail>` in a fault.
fn fault_detail_name(xml: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    let mut in_detail = false;
    loop {
        match reader.read_event().ok()? {
            Event::Start(e) | Event::Empty(e) => {
                if in_detail {
                    let local = e.local_name();
                    return std::str::from_utf8(local.as_ref()).ok().map(str::to_string);
                }
                if e.local_name().as_ref() == b"Detail" {
                    in_detail = true;
                }
            }
            Event::Eof => return None,
            _ => {}
        }
    }
}

/// Parse a `<SOAP-ENV:Fault>` element from its raw XML representation.
fn parse_fault(xml: &str) -> anyhow::Result<Error> {
    let shape: FaultShape = quick_xml::de::from_str(xml)
        .with_context(|| format!("could not parse SOAP fault; xml: {xml}"))?;
    let detail = fault_detail_name(xml).unwrap_or_default();
    Ok(Error {
        code: local_part(&shape.code.value).to_string(),
        detail,
        reason: shape.reason.text.value,
    })
}

/// Parse a SOAP envelope, returning either the typed body child or a fault.
///
/// The outer error represents errors parsing the response.
pub fn parse_soap<T>(s: &str) -> anyhow::Result<Result<T, Error>>
where
    T: for<'de> Deserialize<'de>,
{
    let child = locate_body_child(s)?;
    if child.is_fault() {
        return Ok(Err(parse_fault(child.xml)?));
    }
    let inner: T = quick_xml::de::from_str(child.xml)
        .with_context(|| format!("could not parse SOAP body; xml: {}", child.xml))?;
    Ok(Ok(inner))
}

pub fn envelope(namespace: &str, method: &str, params: Option<&str>) -> String {
    let mut s = String::new();
    s.push_str(r#"<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">"#);
    s.push_str(r#"<soap:Body>"#);
    s.push('<');
    s.push_str(method);
    s.push_str(r#" xmlns=""#);
    s.push_str(namespace);
    if let Some(params) = params {
        s.push_str(r#"">"#);
        s.push_str(params);
        s.push_str(r#"</"#);
        s.push_str(method);
        s.push('>');
    } else {
        s.push_str(r#""/>"#);
    }
    s.push_str(r#"</soap:Body>"#);
    s.push_str(r#"</soap:Envelope>"#);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAULT_XML: &str =
        include_str!("../apis/services/action1/examples/add_action_rule_400_response.xml");

    const SUCCESS_XML: &str =
        include_str!("../apis/services/action1/examples/add_action_configuration_response.xml");

    #[test]
    fn locator_finds_success_body_child() {
        let child = locate_body_child(SUCCESS_XML).unwrap();
        assert_eq!(child.local_name, b"AddActionConfigurationResponse");
        assert!(child
            .xml
            .contains("<aa:ConfigurationID>1</aa:ConfigurationID>"));
        assert!(!child.is_fault());
    }

    #[test]
    fn locator_finds_fault_body_child() {
        let child = locate_body_child(FAULT_XML).unwrap();
        assert_eq!(child.local_name, b"Fault");
        assert!(child.is_fault());
    }

    #[test]
    fn parse_fault_extracts_code_detail_and_reason() {
        let child = locate_body_child(FAULT_XML).unwrap();
        let fault = parse_fault(child.xml).unwrap();
        assert_eq!(fault.code, "Sender");
        assert_eq!(fault.detail, "InvalidConditionFilterFault");
        assert_eq!(fault.reason, "could not match any property events");
    }

    #[test]
    fn parse_soap_returns_err_variant_for_fault() {
        // The fault short-circuits before deserialization, so the type parameter doesn't matter.
        let fault = parse_soap::<()>(FAULT_XML).unwrap().unwrap_err();
        assert_eq!(fault.detail, "InvalidConditionFilterFault");
    }
}
