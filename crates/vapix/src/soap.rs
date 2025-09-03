//! Utilities for working with SOAP style APIs.
use std::{marker::PhantomData, str::FromStr};

use anyhow::Context;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::Client;

pub struct Body {
    pub(crate) namespace: String,
    pub(crate) method: String,
    pub(crate) params: Option<String>,
}

impl Body {
    pub fn new(namespace: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            method: method.into(),
            params: None,
        }
    }

    pub fn build(&self) -> String {
        let mut s = String::new();
        s.push_str(r#"<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">"#);
        s.push_str(r#"<soap:Body xmlns:tns1="http://www.onvif.org/ver10/topics" xmlns:tnsaxis="http://www.axis.com/2009/event/topics">"#);

        s.push('<');
        s.push_str(&self.method);
        s.push_str(r#" xmlns=""#);
        s.push_str(&self.namespace);
        if let Some(params) = self.params.as_deref() {
            s.push_str(r#"">"#);
            s.push_str(params);
            s.push_str(r#"</"#);
            s.push_str(&self.method);
            s.push('>');
        } else {
            s.push_str(r#""/>"#);
        }
        s.push_str(r#"</soap:Body>"#);
        s.push_str(r#"</soap:Envelope>"#);
        s
    }
}

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

pub fn from_response<T>(status: StatusCode, text: reqwest::Result<String>) -> anyhow::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let text: String = text.with_context(|| format!("Could not fetch text ({status})"))?;
    // TODO: Consider splitting XML parsing from conversion to T for better errors
    let Response {
        body: ResponseBody { inner },
    } = quick_xml::de::from_str(&text)
        .with_context(|| format!("Could not parse text; status: {status}; text: {text}"))?;
    Ok(inner)
}

pub struct RequestBuilder<T> {
    pub(crate) client: Client,
    pub(crate) path: &'static str,
    pub(crate) body: Body,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T> RequestBuilder<T>
where
    T: for<'a> Deserialize<'a>,
{
    pub async fn send(self) -> anyhow::Result<T> {
        let Self {
            client,
            path,
            body,
            _phantom,
        } = self;
        let response = client
            .post(path)?
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .body(body.build())
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await;
        from_response(status, text)
    }
}

pub fn from_response2<T>(status: StatusCode, text: reqwest::Result<String>) -> anyhow::Result<T>
where
    T: FromStr<Err = anyhow::Error>,
{
    let text: String = text.with_context(|| format!("Could not fetch text ({status})"))?;
    // TODO: Consider splitting XML parsing from conversion to T for better errors
    let inner = T::from_str(&text)
        .with_context(|| format!("Could not parse text; status: {status}; text: {text}"))?;
    Ok(inner)
}

pub struct RequestBuilder2<T> {
    pub(crate) client: Client,
    pub(crate) path: &'static str,
    pub(crate) body: Body,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T> RequestBuilder2<T>
where
    T: FromStr<Err = anyhow::Error>,
{
    pub async fn send(self) -> anyhow::Result<T> {
        let Self {
            client,
            path,
            body,
            _phantom,
        } = self;
        let response = client
            .post(path)?
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .body(body.build())
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await;
        from_response2(status, text)
    }
}
