//! The [action service API].
//!
//! [action service API]: https://developer.axis.com/vapix/network-video/event-and-action-services
mod action_configurations;
mod action_rules;

use std::str::FromStr;

pub use action_configurations::{
    AddActionConfigurationRequest, AddActionConfigurationResponse, GetActionConfigurationsRequest,
    GetActionConfigurationsResponse, RemoveActionConfigurationRequest,
};
pub use action_rules::{
    AddActionRuleRequest, AddActionRuleResponse, Condition, GetActionRulesRequest,
    GetActionRulesResponse, RemoveActionRuleRequest,
};

/// Known fault details returned from these APIs.
///
/// Use with [`crate::protocol_helpers::soap::Error::parse_detail_as`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaultDetail {
    ActionConfigurationIsInUse,
    ActionConfigurationNotFound,
    ActionRuleNotFound,
    ActionTemplateNotFound,
    InvalidConditionFilter,
    InvalidMessageContentExpression,
    InvalidTopicExpression,
    ParametersMismatch,
}

impl FromStr for FaultDetail {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "ActionConfigurationIsInUseFault" => Self::ActionConfigurationIsInUse,
            "ActionConfigurationNotFoundFault" => Self::ActionConfigurationNotFound,
            "ActionRuleNotFoundFault" => Self::ActionRuleNotFound,
            "ActionTemplateNotFoundFault" => Self::ActionTemplateNotFound,
            "InvalidConditionFilterFault" => Self::InvalidConditionFilter,
            "InvalidMessageContentExpressionFault" => Self::InvalidMessageContentExpression,
            "InvalidTopicExpressionFault" => Self::InvalidTopicExpression,
            "ParametersMissmatchFault" => Self::ParametersMismatch,
            other => anyhow::bail!("not a known action1 fault: {other}"),
        })
    }
}
