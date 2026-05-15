use std::convert::Infallible;

use anyhow::Context;
use serde::{de::IgnoredAny, Deserialize, Serialize};

use crate::{
    http::{HttpClient, Request},
    protocol_helpers::{http::Error, soap, soap_http, soap_http::SoapResponse},
};

const PATH: &str = "vapix/services";

const NAMESPACE: &str = "http://www.axis.com/vapix/ws/action1";

// Namespace URIs used by the action1 wire format. The dialect URIs identify the WSN filter
// dialects (Concrete topic expressions, ItemFilter message content); the topic prefixes
// (`tns1`, `tnsaxis`) are declared on the request so values like `tns1:Device/...` inside
// filter text are well-formed; `WSN_NS` redeclares the default namespace on each filter
// element, matching the WSDL.
const TOPIC_EXPRESSION_DIALECT: &str =
    "http://docs.oasis-open.org/wsn/t-1/TopicExpression/Concrete";
const MESSAGE_CONTENT_DIALECT: &str =
    "http://www.onvif.org/ver10/tev/messageContentFilter/ItemFilter";
const ONVIF_TOPICS_NS: &str = "http://www.onvif.org/ver10/topics";
const AXIS_TOPICS_NS: &str = "http://www.axis.com/2009/event/topics";
const WSN_NS: &str = "http://docs.oasis-open.org/wsn/b-2";

#[derive(Debug, Deserialize)]
pub struct TopicExpression {
    #[serde(rename = "$text")]
    pub value: String,
}

impl TopicExpression {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct MessageContent {
    #[serde(rename = "$text")]
    pub value: String,
}

impl MessageContent {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Condition {
    pub topic_expression: TopicExpression,
    pub message_content: MessageContent,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ActionRule {
    #[serde(rename = "RuleID")]
    pub rule_id: u16,
    pub name: String,
    pub enabled: bool,
    // TODO: Consider encoding the observation that conditions and start_event are not both set
    #[serde(default, deserialize_with = "deserialize_condition_list")]
    pub conditions: Vec<Condition>,
    pub start_event: Option<Condition>,
    pub primary_action: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetActionRulesResponse {
    #[serde(default, deserialize_with = "deserialize_action_rule_list")]
    pub action_rules: Vec<ActionRule>,
}

#[derive(Debug, Deserialize)]
pub struct AddActionRuleResponse {
    #[serde(rename = "RuleID")]
    pub id: u16,
}

pub struct AddActionRuleRequest {
    pub name: String,
    pub enabled: bool,
    pub conditions: Vec<Condition>,
    pub primary_action: u16,
}

impl AddActionRuleRequest {
    pub fn new(name: String, primary_action: u16) -> Self {
        Self {
            name,
            enabled: true,
            conditions: Vec::new(),
            primary_action,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn condition(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }

    pub fn into_envelope(self) -> String {
        #[derive(Serialize)]
        struct Wire {
            #[serde(rename = "@xmlns:tns1")]
            xmlns_tns1: &'static str,
            #[serde(rename = "@xmlns:tnsaxis")]
            xmlns_tnsaxis: &'static str,
            #[serde(rename = "Name")]
            name: String,
            #[serde(rename = "Enabled")]
            enabled: bool,
            #[serde(rename = "Conditions", skip_serializing_if = "Option::is_none")]
            conditions: Option<ConditionsList>,
            #[serde(rename = "PrimaryAction")]
            primary_action: u16,
        }
        #[derive(Serialize)]
        struct ConditionsList {
            #[serde(rename = "Condition")]
            condition: Vec<ConditionWire>,
        }
        #[derive(Serialize)]
        struct ConditionWire {
            #[serde(rename = "TopicExpression")]
            topic_expression: FilterWire,
            #[serde(rename = "MessageContent")]
            message_content: FilterWire,
        }
        #[derive(Serialize)]
        struct FilterWire {
            #[serde(rename = "@Dialect")]
            dialect: &'static str,
            #[serde(rename = "@xmlns")]
            xmlns: &'static str,
            #[serde(rename = "$text")]
            value: String,
        }
        impl From<TopicExpression> for FilterWire {
            fn from(t: TopicExpression) -> Self {
                Self {
                    dialect: TOPIC_EXPRESSION_DIALECT,
                    xmlns: WSN_NS,
                    value: t.value,
                }
            }
        }
        impl From<MessageContent> for FilterWire {
            fn from(m: MessageContent) -> Self {
                Self {
                    dialect: MESSAGE_CONTENT_DIALECT,
                    xmlns: WSN_NS,
                    value: m.value,
                }
            }
        }
        let wire = Wire {
            xmlns_tns1: ONVIF_TOPICS_NS,
            xmlns_tnsaxis: AXIS_TOPICS_NS,
            name: self.name,
            enabled: self.enabled,
            conditions: (!self.conditions.is_empty()).then(|| ConditionsList {
                condition: self
                    .conditions
                    .into_iter()
                    .map(|c| ConditionWire {
                        topic_expression: c.topic_expression.into(),
                        message_content: c.message_content.into(),
                    })
                    .collect(),
            }),
            primary_action: self.primary_action,
        };
        let params = quick_xml::se::to_string_with_root("NewActionRule", &wire).unwrap();
        soap::envelope(NAMESPACE, "AddActionRule", Some(&params))
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<AddActionRuleResponse, Error<Infallible>> {
        let request =
            Request::new(reqwest::Method::POST, PATH.to_string()).soap(self.into_envelope());
        soap_http::send_request(client, request).await
    }
}

#[derive(Debug, Default)]
pub struct GetActionRulesRequest;

impl GetActionRulesRequest {
    pub fn new() -> Self {
        Self
    }

    pub fn into_envelope(self) -> String {
        soap::envelope(NAMESPACE, "GetActionRules", None)
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<GetActionRulesResponse, Error<Infallible>> {
        let request =
            Request::new(reqwest::Method::POST, PATH.to_string()).soap(self.into_envelope());
        soap_http::send_request(client, request).await
    }
}

pub struct RemoveActionRuleRequest {
    rule_id: u16,
}

impl RemoveActionRuleRequest {
    pub fn new(rule_id: u16) -> Self {
        Self { rule_id }
    }

    pub fn into_envelope(self) -> String {
        let mut params = String::new();
        params.push_str(r#"<RuleID>"#);
        params.push_str(&self.rule_id.to_string());
        params.push_str(r#"</RuleID>"#);
        soap::envelope(NAMESPACE, "RemoveActionRule", Some(&params))
    }

    pub async fn send(self, client: &(impl HttpClient + Sync)) -> Result<(), Error<Infallible>> {
        let request =
            Request::new(reqwest::Method::POST, PATH.to_string()).soap(self.into_envelope());
        let RemoveActionRuleResponse = soap_http::send_request(client, request).await?;
        Ok(())
    }
}

struct RemoveActionRuleResponse;

// TODO: Consider making the `aa:RemoveActionRuleResponse` tag available to deserialization
impl SoapResponse for RemoveActionRuleResponse {
    fn from_envelope(s: &str) -> anyhow::Result<Self> {
        // The body is empty, but the wrapper element must still be present and must be
        // `RemoveActionRuleResponse` — anything else (e.g. a `SOAP-ENV:Fault`) should fail.
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Envelope {
            #[allow(dead_code, reason = "Required for shape validation")]
            body: Body,
        }
        #[derive(Deserialize)]
        struct Body {
            #[serde(rename = "RemoveActionRuleResponse")]
            _inner: IgnoredAny,
        }
        let Envelope { .. } = quick_xml::de::from_str(s)
            .with_context(|| format!("Could not parse text; text: {s}"))?;
        Ok(Self)
    }
}

// The SOAP wire format nests repeated items inside a wrapper element
// (e.g. `<ActionRules><ActionRule/>...</ActionRules>`). quick-xml's default `Vec<T>`
// deserialization expects the parent's field name to be repeated, not nested inside another
// element, so we provide an explicit `deserialize_with` for each list field.

fn deserialize_condition_list<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<Condition>, D::Error> {
    #[derive(Deserialize)]
    struct Wire {
        #[serde(default, rename = "Condition")]
        inner: Vec<Condition>,
    }
    Ok(Wire::deserialize(d)?.inner)
}

fn deserialize_action_rule_list<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<ActionRule>, D::Error> {
    #[derive(Deserialize)]
    struct Wire {
        #[serde(default, rename = "ActionRule")]
        inner: Vec<ActionRule>,
    }
    Ok(Wire::deserialize(d)?.inner)
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::{AddActionRuleResponse, GetActionRulesResponse};
    use crate::protocol_helpers::soap::parse_soap;

    #[test]
    fn can_deserialize_add_action_rule_200_response() {
        let text = include_str!("examples/add_action_rule_200_response.xml");
        let data = parse_soap::<AddActionRuleResponse>(text).unwrap();
        assert_eq!(1, data.id);
    }

    #[test]
    fn can_deserialize_get_action_rules_200_empty_response() {
        let text = include_str!("examples/get_action_rules_200_empty.xml");
        let data = parse_soap::<GetActionRulesResponse>(text).unwrap();
        assert!(data.action_rules.is_empty());
    }

    #[test]
    fn can_deserialize_get_action_rules_200_response_with_conditions() {
        let text = include_str!("examples/get_action_rules_200_conditions.xml");
        let data = parse_soap::<GetActionRulesResponse>(text).unwrap();
        expect![[r#"
            [
                ActionRule {
                    rule_id: 30,
                    name: "remote_recording",
                    enabled: true,
                    conditions: [
                        Condition {
                            topic_expression: TopicExpression {
                                value: "tnsaxis:CameraApplicationPlatform/ObjectAnalytics/Device1ScenarioANY",
                            },
                            message_content: MessageContent {
                                value: "boolean(//SimpleItem[@Name=\"active\" and @Value=\"1\"])",
                            },
                        },
                    ],
                    start_event: None,
                    primary_action: 31,
                },
            ]
        "#]]
        .assert_debug_eq(&data.action_rules);
    }

    #[test]
    fn can_deserialize_get_action_rules_200_response_with_start_event() {
        let text = include_str!("examples/get_action_rules_200_start_event.xml");
        let data = parse_soap::<GetActionRulesResponse>(text).unwrap();
        expect![[r#"
            [
                ActionRule {
                    rule_id: 16,
                    name: "Motion (email)",
                    enabled: false,
                    conditions: [],
                    start_event: Some(
                        Condition {
                            topic_expression: TopicExpression {
                                value: "tnsaxis:CameraApplicationPlatform/ObjectAnalytics/Device1Scenario1",
                            },
                            message_content: MessageContent {
                                value: "boolean(//SimpleItem[@Name=\"active\" and @Value=\"1\"])",
                            },
                        },
                    ),
                    primary_action: 17,
                },
            ]
        "#]]
        .assert_debug_eq(&data.action_rules);
    }
}
