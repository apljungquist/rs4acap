pub mod action_1 {
    pub use crate::action1::{
        add_action_configuration, add_action_rule, get_action_configurations, get_action_rules,
    };
}
pub mod basic_device_info_1 {
    pub use crate::basic_device_info_1::get_all_unrestricted_properties;
}

pub mod event_1 {
    pub use crate::event1::get_event_instances;
}

pub mod system_ready_1 {
    pub use crate::system_ready_1::system_ready;
}
