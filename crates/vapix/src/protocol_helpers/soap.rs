//! Utilities for working with SOAP style APIs.

use anyhow::Context;
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
