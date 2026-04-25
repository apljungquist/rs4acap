use std::convert::Infallible;

use serde::Deserialize;

use crate::{
    http::{Error, HttpClient, Request},
    soap, soap_http,
};

const PATH: &str = "vapix/services";

const NAMESPACE: &str = "http://www.axis.com/vapix/ws/action1";

pub struct AddActionRuleRequest {
    pub name: String,
    pub enabled: bool,
    pub conditions: Conditions,
    pub primary_action: u16,
}

impl AddActionRuleRequest {
    pub fn new(name: String, primary_action: u16) -> Self {
        Self {
            name,
            enabled: true,
            conditions: Conditions {
                condition: Vec::new(),
            },
            primary_action,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn condition(mut self, condition: Condition) -> Self {
        self.conditions.condition.push(condition);
        self
    }

    pub fn into_envelope(self) -> String {
        let Self {
            name,
            enabled,
            conditions,
            primary_action,
        } = self;
        let mut params = String::new();
        params.push_str(r#"<NewActionRule xmlns:tns1="http://www.onvif.org/ver10/topics" xmlns:tnsaxis="http://www.axis.com/2009/event/topics">"#);
        params.push_str(r#"<Name>"#);
        params.push_str(&name);
        params.push_str(r#"</Name>"#);
        params.push_str(r#"<Enabled>"#);
        params.push_str(&enabled.to_string());
        params.push_str(r#"</Enabled>"#);
        params.push_str(r#"<Conditions>"#);
        for condition in conditions.condition {
            let Condition {
                topic_expression,
                message_content,
            } = condition;
            params.push_str(r#"<Condition>"#);
            params.push_str(r#"<TopicExpression Dialect="http://docs.oasis-open.org/wsn/t-1/TopicExpression/Concrete" xmlns="http://docs.oasis-open.org/wsn/b-2">"#);
            params.push_str(&topic_expression);
            params.push_str(r#"</TopicExpression>"#);
            params.push_str(r#"<MessageContent Dialect="http://www.onvif.org/ver10/tev/messageContentFilter/ItemFilter" xmlns="http://docs.oasis-open.org/wsn/b-2">"#);
            params.push_str(&message_content);
            params.push_str(r#"</MessageContent>"#);
            params.push_str(r#"</Condition>"#);
        }
        params.push_str(r#"</Conditions>"#);
        params.push_str(r#"<PrimaryAction>"#);
        params.push_str(&primary_action.to_string());
        params.push_str(r#"</PrimaryAction>"#);
        params.push_str(r#"</NewActionRule>"#);
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AddActionRuleResponse {
    #[serde(rename = "RuleID")]
    pub id: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Condition {
    pub topic_expression: String,
    pub message_content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Conditions {
    pub condition: Vec<Condition>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ActionRule {
    #[serde(rename = "RuleID")]
    pub rule_id: u16,
    pub name: String,
    pub enabled: String,
    // TODO: Consider encoding the observation that conditions and start_event are not both set
    pub conditions: Option<Conditions>,
    pub start_event: Option<Condition>,
    pub primary_action: u16,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ActionRules {
    #[serde(default)]
    pub action_rule: Vec<ActionRule>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetActionRulesResponse {
    pub action_rules: ActionRules,
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

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use crate::{
        services::action1::{action_rules::AddActionRuleResponse, GetActionRulesResponse},
        soap::parse_soap,
    };

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
        assert!(data.action_rules.action_rule.is_empty());
    }

    #[test]
    fn can_deserialize_get_action_rules_200_response_with_conditions() {
        let text = include_str!("examples/get_action_rules_200_conditions.xml");
        let data = parse_soap::<GetActionRulesResponse>(text).unwrap();
        expect![[r#"
            ActionRules {
                action_rule: [
                    ActionRule {
                        rule_id: 30,
                        name: "remote_recording",
                        enabled: "true",
                        conditions: Some(
                            Conditions {
                                condition: [
                                    Condition {
                                        topic_expression: "tnsaxis:CameraApplicationPlatform/ObjectAnalytics/Device1ScenarioANY",
                                        message_content: "boolean(//SimpleItem[@Name=\"active\" and @Value=\"1\"])",
                                    },
                                ],
                            },
                        ),
                        start_event: None,
                        primary_action: 31,
                    },
                ],
            }
        "#]].assert_debug_eq(&data.action_rules);
    }

    #[test]
    fn can_deserialize_get_action_rules_200_response_with_start_event() {
        let text = include_str!("examples/get_action_rules_200_start_event.xml");
        let data = parse_soap::<GetActionRulesResponse>(text).unwrap();
        expect![[r#"
            ActionRules {
                action_rule: [
                    ActionRule {
                        rule_id: 16,
                        name: "Motion (email)",
                        enabled: "false",
                        conditions: None,
                        start_event: Some(
                            Condition {
                                topic_expression: "tnsaxis:CameraApplicationPlatform/ObjectAnalytics/Device1Scenario1",
                                message_content: "boolean(//SimpleItem[@Name=\"active\" and @Value=\"1\"])",
                            },
                        ),
                        primary_action: 17,
                    },
                ],
            }
        "#]].assert_debug_eq(&data.action_rules);
    }
}
