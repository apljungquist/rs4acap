//! WSDL-declared fault detail elements for the Axis action service.
//!
//! Variants are cleaned-up Rust names (PascalCase, no `Fault` suffix, spelling-normalized);
//! the WSDL wire spellings — including the `ParametersMissmatch` typo — live in the
//! [`FromStr`] match arms below.

use std::str::FromStr;

/// Typed representation of the `<aa:*Fault/>` element that the server places inside
/// `<SOAP-ENV:Detail>`. The variants correspond one-to-one with the `<wsdl:fault name="...">`
/// declarations in the action service WSDL.
///
/// Parse a raw [`crate::protocol_helpers::soap::Fault::detail_element`] string via
/// `name.parse::<FaultDetail>()`.
///
/// Adding a variant when the WSDL grows is a semver-breaking change per the crate-wide policy
/// on exhaustive matching in API bindings (see commit `d22ebf7`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaultDetail {
    ActionConfigurationIsInUse,
    ActionConfigurationNotFound,
    ActionRuleNotFound,
    ActionTemplateNotFound,
    InsufficientActivationRule,
    InvalidActionConfiguration,
    InvalidActivationTimeout,
    InvalidConditionFilter,
    InvalidFilter,
    InvalidMessageContentExpression,
    InvalidTopicExpression,
    ParametersMismatch,
    RecipientConfigurationNotFound,
    RecipientTemplateNotFound,
    TopicExpressionDialectUnknown,
}

impl FromStr for FaultDetail {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = match s {
            "ActionConfigurationIsInUseFault" => Self::ActionConfigurationIsInUse,
            "ActionConfigurationNotFoundFault" => Self::ActionConfigurationNotFound,
            "ActionRuleNotFoundFault" => Self::ActionRuleNotFound,
            "ActionTemplateNotFoundFault" => Self::ActionTemplateNotFound,
            "InsufficientActivationRuleFault" => Self::InsufficientActivationRule,
            "InvalidActionConfigurationFault" => Self::InvalidActionConfiguration,
            "InvalidActivationTimeoutFault" => Self::InvalidActivationTimeout,
            "InvalidConditionFilterFault" => Self::InvalidConditionFilter,
            "InvalidFilterFault" => Self::InvalidFilter,
            "InvalidMessageContentExpressionFault" => Self::InvalidMessageContentExpression,
            "InvalidTopicExpressionFault" => Self::InvalidTopicExpression,
            "ParametersMissmatchFault" => Self::ParametersMismatch,
            "RecipientConfigurationNotFoundFault" => Self::RecipientConfigurationNotFound,
            "RecipientTemplateNotFoundFault" => Self::RecipientTemplateNotFound,
            "TopicExpressionDialectUnknownFault" => Self::TopicExpressionDialectUnknown,
            _ => anyhow::bail!("unrecognized action1 fault detail '{s}'"),
        };
        Ok(value)
    }
}
