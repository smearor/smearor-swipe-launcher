use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use std::fmt::Display;
use std::fmt::Formatter;

pub const TOPIC_COMMAND: &str = "service/app_launcher/command";

pub const TOPIC_STATUS: &str = "service/app_launcher/status";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DesktopFileCommandAction {
    Exec,
    ExecStart,
    ExecReload,
    Terminate,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopFileCommandMessage {
    /// The canonical path of the desktop file
    pub desktop_file: String,

    /// The action to execute on the desktop file
    pub action: DesktopFileCommandAction,
}

impl DesktopFileCommandMessage {
    pub fn new(desktop_file: &str, action: DesktopFileCommandAction) -> Self {
        Self {
            desktop_file: desktop_file.to_string(),
            action,
        }
    }
}

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
}

// impl TryFrom<FfiEnvelope> for DesktopFileStatusMessage {
//     type Error = serde_json::Error;
//
//     fn try_from(message: FfiEnvelope) -> Result<Self, Self::Error> {
//         serde_json::from_str(&message.payload.to_string())
//     }
// }
