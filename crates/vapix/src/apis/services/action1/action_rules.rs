use serde::{Deserialize, Serialize};

use crate::{
    http::{HttpClient, Request},
    protocol_helpers::{
        http::Error,
        soap,
        soap::{parse_empty_response_or_fault, Fault},
        soap_http,
        soap_http::SoapResponse,
    },
};

const PATH: &str = "vapix/services";

const NAMESPACE: &str = "http://www.axis.com/vapix/ws/action1";

const TOPIC_EXPRESSION_DIALECT: &str =
    "http://docs.oasis-open.org/wsn/t-1/TopicExpression/Concrete";
const MESSAGE_CONTENT_DIALECT: &str =
    "http://www.onvif.org/ver10/tev/messageContentFilter/ItemFilter";

#[derive(Debug, Deserialize, Serialize)]
pub struct TopicExpression {
    // Captured to keep deserialization lossless; in practice the server only emits the
    // Concrete dialect URI declared by `TopicExpression::new`.
    #[serde(rename = "@Dialect")]
    dialect: String,
    #[serde(rename = "$text")]
    pub value: String,
}

impl TopicExpression {
    pub fn new(value: impl Into<String>) -> Self {
        Self::with_dialect(TOPIC_EXPRESSION_DIALECT, value)
    }

    /// Construct with an explicit dialect URI. Reserved for callers that need a non-canonical
    /// dialect; the common case is [`TopicExpression::new`].
    pub fn with_dialect(dialect: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            dialect: dialect.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageContent {
    // Captured to keep deserialization lossless; in practice the server only emits the
    // ItemFilter dialect URI declared by `MessageContent::new`.
    #[serde(rename = "@Dialect")]
    dialect: String,
    #[serde(rename = "$text")]
    pub value: String,
}

impl MessageContent {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            dialect: MESSAGE_CONTENT_DIALECT.to_string(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Condition {
    pub topic_expression: TopicExpression,
    pub message_content: MessageContent,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ActionRule {
    #[serde(rename = "RuleID")]
    pub rule_id: u16,
    pub name: String,
    pub enabled: bool,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        with = "wrapped_conditions"
    )]
    pub conditions: Vec<Condition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_event: Option<Condition>,
    pub primary_action: u16,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetActionRulesResponse {
    #[serde(default, with = "wrapped_action_rules")]
    pub action_rules: Vec<ActionRule>,
}

#[derive(Debug, Deserialize, Serialize)]
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
        // `xmlns:tns1` / `xmlns:tnsaxis` are declared so values like
        // "tns1:Device/tnsaxis:Status/SystemReady" inside the filter text are valid
        // qualified names in the document the server receives. Each filter element redeclares
        // the default namespace as WSN, matching the WSDL.
        const WSN_NS: &str = "http://docs.oasis-open.org/wsn/b-2";

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
            #[serde(rename = "Conditions")]
            conditions: ConditionsList,
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
            dialect: String,
            #[serde(rename = "@xmlns")]
            xmlns: &'static str,
            #[serde(rename = "$text")]
            value: String,
        }
        impl From<TopicExpression> for FilterWire {
            fn from(t: TopicExpression) -> Self {
                Self {
                    dialect: t.dialect,
                    xmlns: WSN_NS,
                    value: t.value,
                }
            }
        }
        impl From<MessageContent> for FilterWire {
            fn from(m: MessageContent) -> Self {
                Self {
                    dialect: m.dialect,
                    xmlns: WSN_NS,
                    value: m.value,
                }
            }
        }
        let wire = Wire {
            xmlns_tns1: "http://www.onvif.org/ver10/topics",
            xmlns_tnsaxis: "http://www.axis.com/2009/event/topics",
            name: self.name,
            enabled: self.enabled,
            conditions: ConditionsList {
                condition: self
                    .conditions
                    .into_iter()
                    .map(|c| ConditionWire {
                        topic_expression: c.topic_expression.into(),
                        message_content: c.message_content.into(),
                    })
                    .collect(),
            },
            primary_action: self.primary_action,
        };
        let params = quick_xml::se::to_string_with_root("NewActionRule", &wire).unwrap();
        soap::envelope(NAMESPACE, "AddActionRule", Some(&params))
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<AddActionRuleResponse, Error<Fault>> {
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
    ) -> Result<GetActionRulesResponse, Error<Fault>> {
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
        let params = quick_xml::se::to_string_with_root("RuleID", &self.rule_id).unwrap();
        soap::envelope(NAMESPACE, "RemoveActionRule", Some(&params))
    }

    pub async fn send(self, client: &(impl HttpClient + Sync)) -> Result<(), Error<Fault>> {
        let request =
            Request::new(reqwest::Method::POST, PATH.to_string()).soap(self.into_envelope());
        let RemoveActionRuleResponse = soap_http::send_request(client, request).await?;
        Ok(())
    }
}

struct RemoveActionRuleResponse;

impl SoapResponse for RemoveActionRuleResponse {
    fn from_envelope(s: &str) -> anyhow::Result<Result<Self, Fault>> {
        Ok(parse_empty_response_or_fault(s, "RemoveActionRuleResponse")?.map(|()| Self))
    }
}

/// Generates a serde `with`-module that wraps a `Vec<$item>` as `<$tag>...</$tag>` repeated
/// inside the parent element. Needed because quick-xml's default `Vec<T>` serialization
/// repeats the parent's field name, but the SOAP wire format nests the items inside an
/// additional wrapper element (e.g. `<ActionRules><ActionRule/>...</ActionRules>`).
macro_rules! wrapped_vec_module {
    ($mod_name:ident, $item:ident, $tag:literal) => {
        mod $mod_name {
            use serde::{Deserialize, Deserializer, Serialize, Serializer};

            use super::$item;

            pub fn serialize<S: Serializer>(value: &Vec<$item>, s: S) -> Result<S::Ok, S::Error> {
                #[derive(Serialize)]
                struct Wire<'a> {
                    #[serde(rename = $tag)]
                    inner: &'a [$item],
                }
                Wire { inner: value }.serialize(s)
            }

            pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<$item>, D::Error> {
                #[derive(Deserialize)]
                struct Wire {
                    #[serde(default, rename = $tag)]
                    inner: Vec<$item>,
                }
                Ok(Wire::deserialize(d)?.inner)
            }
        }
    };
}

wrapped_vec_module!(wrapped_conditions, Condition, "Condition");
wrapped_vec_module!(wrapped_action_rules, ActionRule, "ActionRule");

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::{AddActionRuleResponse, GetActionRulesResponse};
    use crate::protocol_helpers::soap::parse_soap_lossless;

    #[test]
    fn can_deserialize_add_action_rule_200_response() {
        let text = include_str!("examples/add_action_rule_200_response.xml");
        let data = parse_soap_lossless::<AddActionRuleResponse>(text).unwrap();
        assert_eq!(1, data.id);
    }

    #[test]
    fn can_deserialize_get_action_rules_200_empty_response() {
        let text = include_str!("examples/get_action_rules_200_empty.xml");
        let data = parse_soap_lossless::<GetActionRulesResponse>(text).unwrap();
        assert!(data.action_rules.is_empty());
    }

    #[test]
    fn can_deserialize_get_action_rules_200_response_with_conditions() {
        let text = include_str!("examples/get_action_rules_200_conditions.xml");
        let data = parse_soap_lossless::<GetActionRulesResponse>(text).unwrap();
        expect![[r#"
            [
                ActionRule {
                    rule_id: 30,
                    name: "remote_recording",
                    enabled: true,
                    conditions: [
                        Condition {
                            topic_expression: TopicExpression {
                                dialect: "http://docs.oasis-open.org/wsn/t-1/TopicExpression/Concrete",
                                value: "tnsaxis:CameraApplicationPlatform/ObjectAnalytics/Device1ScenarioANY",
                            },
                            message_content: MessageContent {
                                dialect: "http://www.onvif.org/ver10/tev/messageContentFilter/ItemFilter",
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
        let data = parse_soap_lossless::<GetActionRulesResponse>(text).unwrap();
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
                                dialect: "http://docs.oasis-open.org/wsn/t-1/TopicExpression/Concrete",
                                value: "tnsaxis:CameraApplicationPlatform/ObjectAnalytics/Device1Scenario1",
                            },
                            message_content: MessageContent {
                                dialect: "http://www.onvif.org/ver10/tev/messageContentFilter/ItemFilter",
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
