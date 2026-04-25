pub mod api_discovery_1 {
    pub use crate::api_discovery_1::{GetApiListRequest, GetSupportedVersionsRequest};
}

pub mod discover {
    pub use crate::discover::DiscoverRequest;
}

pub mod applications_config {
    pub use crate::applications_config::ApplicationConfigRequest;
}

pub mod action_1 {
    pub use crate::action1::{
        AddActionConfigurationRequest, AddActionRuleRequest, GetActionConfigurationsRequest,
        GetActionRulesRequest,
    };
}
pub mod basic_device_info_1 {
    pub use crate::basic_device_info_1::{
        GetAllPropertiesRequest, GetAllUnrestrictedPropertiesRequest,
    };
}

pub mod event_1 {
    pub use crate::event1::GetEventInstancesRequest;
}

pub mod jpg_3 {
    pub use crate::axis_cgi::jpg_3::GetImageRequest;
}

pub mod remote_object_storage_1_beta {
    pub use crate::config::remote_object_storage_1_beta::{
        CreateDestinationRequest, DeleteDestinationRequest, ListDestinationsRequest,
        UpdateDestinationRequest,
    };
}

pub mod recording_group_1 {
    pub use crate::config::recording_group_1::CreateRecordingGroupsRequest;
}

pub mod siren_and_light_2_alpha {
    pub use crate::config::siren_and_light_2_alpha::{
        GetMaintenanceModeRequest, StartMaintenanceModeRequest, StopMaintenanceModeRequest,
    };
}

pub mod ssh_1 {
    pub use crate::config::ssh_1::{AddUserRequest, DeleteUserRequest, SetUserRequest};
}

pub mod system_ready_1 {
    pub use crate::system_ready_1::SystemReadyRequest;
}

pub mod firmware_management_1 {
    pub use crate::firmware_management_1::{FactoryDefaultRequest, UpgradeRequest};
}

pub mod network_settings_1 {
    pub use crate::network_settings_1::{
        GetNetworkInfoRequest, SetGlobalProxyConfigurationRequest,
    };
}

pub mod parameter_management {
    pub use crate::parameter_management::{
        ImageResolution, ListRequest, ParamList, Parameter, Resolution, UpdateRequest,
    };
}

pub mod pwdgrp {
    pub use crate::pwdgrp::AddUserRequest;
}
