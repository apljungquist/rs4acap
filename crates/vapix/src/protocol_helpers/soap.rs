//! Utilities for working with SOAP style APIs.

use anyhow::Context;
use log::{error, warn};
use quick_xml::events::Event;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "PascalCase")]
struct Response<T> {
    body: ResponseBody<T>,
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
struct ResponseBody<T> {
    #[serde(rename = "$value")]
    inner: T,
}

pub fn parse_soap<T>(s: &str) -> anyhow::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let Response {
        body: ResponseBody { inner },
    } = quick_xml::de::from_str(s).with_context(|| format!("Could not parse text; text: {s}"))?;
    Ok(inner)
}

pub fn parse_soap_lossless<T>(s: &str) -> anyhow::Result<T>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let value = parse_soap::<T>(s)?;
    if let Err(e) = soft_assert_lossless(s, &value) {
        error!("Failed to verify SOAP losslessness: {e:?}");
        debug_assert!(false, "SOAP losslessness check itself failed: {e:?}");
    }
    Ok(value)
}

fn soft_assert_lossless<T: Serialize>(original: &str, value: &T) -> anyhow::Result<()> {
    let roundtripped = quick_xml::se::to_string(value).context("re-serializing parsed value")?;
    let original_events = inner_events(original)?;
    let roundtripped_events = normalized_events(&roundtripped)?;
    if original_events != roundtripped_events {
        warn!(
            "SOAP deserialization is not lossless.\noriginal events: {original_events:#?}\nroundtripped events: {roundtripped_events:#?}",
        );
        debug_assert_eq!(original_events, roundtripped_events);
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum NormalizedEvent {
    Start {
        name: String,
        attrs: Vec<(String, String)>,
    },
    End {
        name: String,
    },
    Text(String),
}

pub(crate) fn local_name(name: quick_xml::name::QName) -> String {
    String::from_utf8_lossy(name.local_name().into_inner()).into_owned()
}

fn collect_attrs(attrs: quick_xml::events::attributes::Attributes) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for attr in attrs.flatten() {
        let key = attr.key;
        if key.as_ref() == b"xmlns" || key.as_ref().starts_with(b"xmlns:") {
            continue;
        }
        out.push((
            local_name(key),
            String::from_utf8_lossy(&attr.value).into_owned(),
        ));
    }
    out.sort();
    out
}

fn normalized_events(s: &str) -> anyhow::Result<Vec<NormalizedEvent>> {
    let mut reader = quick_xml::Reader::from_str(s);
    let config = reader.config_mut();
    config.trim_text(true);
    let mut events = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => events.push(NormalizedEvent::Start {
                name: local_name(e.name()),
                attrs: collect_attrs(e.attributes()),
            }),
            Event::Empty(e) => {
                let name = local_name(e.name());
                let attrs = collect_attrs(e.attributes());
                events.push(NormalizedEvent::Start {
                    name: name.clone(),
                    attrs,
                });
                events.push(NormalizedEvent::End { name });
            }
            Event::End(e) => events.push(NormalizedEvent::End {
                name: local_name(e.name()),
            }),
            Event::Text(e) => {
                let decoded = e.decode()?;
                let text = quick_xml::escape::unescape(&decoded)?.trim().to_string();
                if !text.is_empty() {
                    events.push(NormalizedEvent::Text(text));
                }
            }
            Event::CData(e) => {
                let text = String::from_utf8_lossy(&e).trim().to_string();
                if !text.is_empty() {
                    events.push(NormalizedEvent::Text(text));
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(events)
}

/// Like [`normalized_events`], but drops the SOAP `Envelope` and `Body` wrapper events so the
/// stream lines up with what a `quick_xml::se` round-trip of the inner type produces.
fn inner_events(s: &str) -> anyhow::Result<Vec<NormalizedEvent>> {
    Ok(normalized_events(s)?
        .into_iter()
        .filter(|e| {
            let name = match e {
                NormalizedEvent::Start { name, .. } | NormalizedEvent::End { name } => name,
                NormalizedEvent::Text(_) => return true,
            };
            name != "Envelope" && name != "Body"
        })
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    pub code: String,
    pub reason: String,
    pub detail_element: Option<String>,
}

impl Fault {
    pub fn parse_code(&self) -> Result<FaultCode, anyhow::Error> {
        self.code.parse()
    }

    pub fn parse_detail_as<T: std::str::FromStr>(&self) -> Result<Option<T>, T::Err> {
        self.detail_element.as_deref().map(str::parse).transpose()
    }
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SOAP fault ({})", self.code)?;
        if !self.reason.is_empty() {
            write!(f, ": {}", self.reason)?;
        }
        if let Some(name) = &self.detail_element {
            write!(f, " [{}]", name)?;
        }
        Ok(())
    }
}

impl std::error::Error for Fault {}

/// The SOAP 1.2 standard fault codes that this crate currently recognizes. Adding a variant
/// (e.g. for `MustUnderstand`, `VersionMismatch`, `DataEncodingUnknown`) is a semver-breaking
/// change per the crate-wide policy on exhaustive matching in API bindings (see commit
/// `d22ebf7`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaultCode {
    Sender,
    Receiver,
}

impl std::str::FromStr for FaultCode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let local = s.rsplit_once(':').map_or(s, |(_, suffix)| suffix);
        match local {
            "Sender" => Ok(Self::Sender),
            "Receiver" => Ok(Self::Receiver),
            _ => Err(anyhow::anyhow!("unrecognized SOAP fault code '{s}'")),
        }
    }
}

/// Probe the SOAP envelope for a `<Fault>` and return it if present, falling back to parsing
/// `T` (with the same lossless check as [`parse_soap_lossless`]) otherwise. Returns
/// `Ok(Err(Fault))` when the server replied with a fault, `Ok(Ok(T))` on success, and
/// `Err(_)` only when neither shape parses.
pub fn parse_soap_or_fault<T>(s: &str) -> anyhow::Result<Result<T, Fault>>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    if let Some(fault) = try_parse_fault(s) {
        return Ok(Err(fault));
    }
    let value = parse_soap_lossless::<T>(s)?;
    Ok(Ok(value))
}

pub(crate) fn try_parse_fault(s: &str) -> Option<Fault> {
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Envelope {
        body: BodyFault,
    }
    #[derive(Deserialize)]
    struct BodyFault {
        #[serde(rename = "Fault")]
        fault: FaultWire,
    }
    #[derive(Deserialize)]
    struct FaultWire {
        #[serde(rename = "Code")]
        code: CodeWire,
        #[serde(default, rename = "Reason")]
        reason: ReasonWire,
    }
    #[derive(Deserialize)]
    struct CodeWire {
        #[serde(rename = "Value")]
        value: String,
    }
    #[derive(Default, Deserialize)]
    struct ReasonWire {
        #[serde(default, rename = "Text")]
        text: String,
    }

    let env = quick_xml::de::from_str::<Envelope>(s).ok()?;
    Some(Fault {
        code: env.body.fault.code.value,
        reason: env.body.fault.reason.text,
        detail_element: extract_detail_element(s),
    })
}

fn extract_detail_element(s: &str) -> Option<String> {
    let mut reader = quick_xml::Reader::from_str(s);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut in_detail = false;
    loop {
        match reader.read_event_into(&mut buf).ok()? {
            Event::Start(e) | Event::Empty(e) => {
                let name = local_name(e.name());
                if in_detail {
                    return Some(name);
                }
                if name == "Detail" {
                    in_detail = true;
                }
            }
            Event::End(e) if local_name(e.name()) == "Detail" => return None,
            Event::Eof => return None,
            _ => {}
        }
        buf.clear();
    }
}

/// Parse a SOAP envelope whose body is expected to be an empty element with `expected` as the
/// local name (used for `RemoveXxxResponse`-style replies). Returns `Ok(Err(Fault))` when the
/// server replied with a fault, `Ok(Ok(()))` when the body shape matches, and `Err(_)` when
/// neither shape parses.
pub(crate) fn parse_empty_response_or_fault(
    s: &str,
    expected: &str,
) -> anyhow::Result<Result<(), Fault>> {
    if let Some(fault) = try_parse_fault(s) {
        return Ok(Err(fault));
    }
    let actual = body_first_child(s)?;
    anyhow::ensure!(
        actual.as_deref() == Some(expected),
        "expected SOAP body to contain <{expected}> but found <{}>",
        actual.as_deref().unwrap_or("(nothing)"),
    );
    Ok(Ok(()))
}

fn body_first_child(s: &str) -> anyhow::Result<Option<String>> {
    let mut reader = quick_xml::Reader::from_str(s);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut in_body = false;
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) | Event::Empty(e) => {
                let name = local_name(e.name());
                if in_body {
                    return Ok(Some(name));
                }
                if name == "Body" {
                    in_body = true;
                }
            }
            Event::Eof => return Ok(None),
            _ => {}
        }
        buf.clear();
    }
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
