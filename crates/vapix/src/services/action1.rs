//! The [action service API].
//!
//! [action service API]: https://developer.axis.com/vapix/network-video/event-and-action-services
mod action_configurations;
mod action_rules;

pub use action_configurations::GetActionConfigurationsResponse;
pub use action_rules::GetActionRulesResponse;

use crate::{
    services::action1::{
        action_configurations::AddActionConfigurationResponse, action_rules::AddActionRuleResponse,
    },
    soap::SimpleRequest,
};

pub fn add_action_configuration() -> SimpleRequest<AddActionConfigurationResponse> {
    SimpleRequest::new(
        "http://www.axis.com/vapix/ws/action1",
        "AddActionConfiguration",
    )
}

pub fn add_action_rule() -> SimpleRequest<AddActionRuleResponse> {
    SimpleRequest::new("http://www.axis.com/vapix/ws/action1", "AddActionRule")
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
