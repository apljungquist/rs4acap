pub mod action_1 {
    pub use crate::action1::{
        add_action_configuration, add_action_rule, get_action_configurations, get_action_rules,
    };
}
pub mod basic_device_info_1 {
    pub use crate::basic_device_info_1::{get_all_properties, get_all_unrestricted_properties};
}

pub mod event_1 {
    pub use crate::event1::get_event_instances;
}

pub mod jpg_3 {
    pub use crate::axis_cgi::jpg_3::get_image;
}

pub mod remote_object_storage_1 {
    pub use crate::config::remote_object_storage_1::create_destinations;
}

pub mod recording_group_2 {
    pub use crate::config::recording_group_2::{create, delete, get, list};
}

pub mod ssh_1 {
    pub use crate::config::ssh_1::{add_user, set_user};
}

pub mod system_ready_1 {
    pub use crate::system_ready_1::system_ready;
}
