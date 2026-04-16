//! The [action service API].
//!
//! [action service API]: https://developer.axis.com/vapix/network-video/event-and-action-services
mod action_configurations;
mod action_rules;

pub use action_configurations::{
    AddActionConfigurationResponse, GetActionConfigurationsRequest, GetActionConfigurationsResponse,
};
pub use action_rules::{
    AddActionRuleResponse, Condition, GetActionRulesRequest, GetActionRulesResponse,
};

use crate::action1::{
    action_configurations::AddActionConfigurationRequest, action_rules::AddActionRuleRequest,
};

pub fn add_action_configuration(template_token: &str) -> AddActionConfigurationRequest {
    AddActionConfigurationRequest::new(template_token)
}

pub fn add_action_rule(name: String, primary_action: u16) -> AddActionRuleRequest {
    AddActionRuleRequest::new(name, primary_action)
}

pub fn get_action_configurations() -> GetActionConfigurationsRequest {
    GetActionConfigurationsRequest
}

pub fn get_action_rules() -> GetActionRulesRequest {
    GetActionRulesRequest
}
