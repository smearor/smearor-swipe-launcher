use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_STATUS: &str = "service.app_launcher.status";

#[repr(u8)]
#[stabby::stabby]
#[derive(Debug, Clone, Default)]
pub enum DesktopFileStatus {
    #[default]
    Running,
    Stopped,
}

#[derive(Debug, Clone, Default)]
pub struct DesktopFileStatusMessage {
    /// The canonical path of the desktop file
    pub desktop_file: String,

    /// The new status of the process
    pub status: DesktopFileStatus,
}

/// ABI-stable version of `DesktopFileStatusMessage` for cross-plugin messaging.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default)]
pub struct DesktopFileStatusMessageStabby {
    pub desktop_file: stabby::string::String,
    pub status: DesktopFileStatus,
}

impl From<DesktopFileStatusMessage> for DesktopFileStatusMessageStabby {
    fn from(value: DesktopFileStatusMessage) -> Self {
        Self {
            desktop_file: value.desktop_file.into(),
            status: value.status,
        }
    }
}

impl From<DesktopFileStatusMessageStabby> for DesktopFileStatusMessage {
    fn from(value: DesktopFileStatusMessageStabby) -> Self {
        Self {
            desktop_file: value.desktop_file.to_string(),
            status: value.status,
        }
    }
}

impl DesktopFileStatusMessage {
    pub fn new(desktop_file: &str, status: DesktopFileStatus) -> Self {
        Self {
            desktop_file: desktop_file.to_string(),
            status,
        }
    }

    pub fn running(desktop_file: &str) -> Self {
        Self::new(desktop_file, DesktopFileStatus::Running)
    }

    pub fn stopped(desktop_file: &str) -> Self {
        Self::new(desktop_file, DesktopFileStatus::Stopped)
    }
}

impl TypedMessage for DesktopFileStatusMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_app_launcher_model::DesktopFileStatusMessageStabby");
}

impl TypedMessage for DesktopFileStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_app_launcher_model::DesktopFileStatusMessage");
}

impl MessageTopic for DesktopFileStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl MessageTopic for DesktopFileStatusMessageStabby {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl SharedMessage for DesktopFileStatusMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_STATUS
    }
}
