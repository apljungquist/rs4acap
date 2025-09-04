//! Utilities for working with SOAP style APIs.
use std::marker::PhantomData;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::soap_http::SoapRequest;

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

pub struct SimpleRequest<T> {
    namespace: &'static str,
    method: &'static str,
    params: Option<String>,
    _phantom: PhantomData<T>,
}

impl<T> SimpleRequest<T> {
    pub fn new(namespace: &'static str, method: &'static str) -> Self {
        Self {
            namespace,
            method,
            params: None,
            _phantom: PhantomData,
        }
    }

    pub fn params(mut self, params: String) -> Self {
        self.params = Some(params);
        self
    }
}

impl<T> SoapRequest for SimpleRequest<T> {
    fn to_envelope(self) -> anyhow::Result<String> {
        let Self {
            namespace,
            method,
            params,
            _phantom,
        } = self;
        let mut s = String::new();
        s.push_str(r#"<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">"#);
        s.push_str(r#"<soap:Body xmlns:tns1="http://www.onvif.org/ver10/topics" xmlns:tnsaxis="http://www.axis.com/2009/event/topics">"#);

        s.push('<');
        s.push_str(method);
        s.push_str(r#" xmlns=""#);
        s.push_str(namespace);
        if let Some(params) = params.as_deref() {
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
        Ok(s)
    }
}
