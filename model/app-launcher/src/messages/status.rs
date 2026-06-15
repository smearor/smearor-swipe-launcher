use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;

pub const TOPIC_STATUS: &str = "service.app_launcher.status";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DesktopFileStatus {
    Running,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopFileStatusMessage {
    /// The canonical path of the desktop file
    pub desktop_file: String,

    /// The new status of the process
    pub status: DesktopFileStatus,
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

impl MessageTopic for DesktopFileStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}
