use serde::Deserialize;

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
    pub conditions: Conditions,
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

#[cfg(test)]
mod tests {
    use crate::{
        services::action1::{action_rules::AddActionRuleResponse, GetActionRulesResponse},
        soap::parse_soap,
    };

    #[test]
    fn can_deserialize_add_action_rule_200_response() {
        let text = include_str!("examples/add_action_rule_200_response.xml");
        let data: AddActionRuleResponse = parse_soap(text).unwrap();
        assert_eq!(1, data.id);
    }

    #[test]
    fn can_deserialize_get_action_rules_response() {
        let text = include_str!("examples/get_action_rules_response.xml");
        let data: GetActionRulesResponse = parse_soap(text).unwrap();
        assert!(data.action_rules.action_rule.is_empty());
    }
}
