//! The [action service API].
//!
//! [action service API]: https://developer.axis.com/vapix/network-video/event-and-action-services
pub mod action_configurations;
pub mod action_rules;

pub use action_configurations::{
    AddActionConfigurationResponse, GetActionConfigurationsResponse,
    RemoveActionConfigurationResponse,
};
pub use action_rules::{
    AddActionRuleResponse, Condition, GetActionRulesResponse, RemoveActionRuleResponse,
};

use crate::{
    action1::{
        action_configurations::{AddActionConfigurationRequest, RemoveActionConfigurationRequest},
        action_rules::{AddActionRuleRequest, GetActionRulesRequest, RemoveActionRuleRequest},
    },
    soap::SimpleRequest,
};

pub fn add_action_configuration(template_token: &str) -> AddActionConfigurationRequest {
    AddActionConfigurationRequest::new(template_token)
}

pub fn add_action_rule(name: String, primary_action: u32) -> AddActionRuleRequest {
    AddActionRuleRequest::new(name, primary_action)
}

pub fn remove_action_configuration(configuration_id: u32) -> RemoveActionConfigurationRequest {
    RemoveActionConfigurationRequest::new(configuration_id)
}

pub fn remove_action_rule(rule_id: u32) -> RemoveActionRuleRequest {
    RemoveActionRuleRequest::new(rule_id)
}

pub fn get_action_configurations() -> SimpleRequest<GetActionConfigurationsResponse> {
    SimpleRequest::new(
        "http://www.axis.com/vapix/ws/action1",
        "GetActionConfigurations",
    )
}

pub fn get_action_rules() -> GetActionRulesRequest {
    GetActionRulesRequest::new()
}
