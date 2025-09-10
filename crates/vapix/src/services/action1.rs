//! The [action service API].
//!
//! [action service API]: https://developer.axis.com/vapix/network-video/event-and-action-services
mod action_configurations;
mod action_rules;

pub use action_configurations::{AddActionConfigurationResponse, GetActionConfigurationsResponse};
pub use action_rules::{AddActionRuleResponse, Condition, GetActionRulesResponse};

use crate::{
    action1::{
        action_configurations::AddActionConfigurationRequest, action_rules::AddActionRuleRequest,
    },
    soap::SimpleRequest,
};

pub fn add_action_configuration(template_token: &str) -> AddActionConfigurationRequest {
    AddActionConfigurationRequest::new(template_token)
}

pub fn add_action_rule(name: String, primary_action: u16) -> AddActionRuleRequest {
    AddActionRuleRequest::new(name, primary_action)
}

pub fn get_action_configurations() -> SimpleRequest<GetActionConfigurationsResponse> {
    SimpleRequest::new(
        "http://www.axis.com/vapix/ws/action1",
        "GetActionConfigurations",
    )
}

pub fn get_action_rules() -> SimpleRequest<GetActionRulesResponse> {
    SimpleRequest::new("http://www.axis.com/vapix/ws/action1", "GetActionRules")
}
