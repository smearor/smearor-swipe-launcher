use serde::Deserialize;
use serde::Serialize;

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

    pub fn exec(desktop_file: &str) -> Self {
        Self::new(desktop_file, DesktopFileCommandAction::Exec)
    }

    pub fn exec_start(desktop_file: &str) -> Self {
        Self::new(desktop_file, DesktopFileCommandAction::ExecStart)
    }

    pub fn terminate(desktop_file: &str) -> Self {
        Self::new(desktop_file, DesktopFileCommandAction::Terminate)
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

    pub fn running(desktop_file: &str) -> Self {
        Self::new(desktop_file, DesktopFileStatus::Running)
    }

    pub fn stopped(desktop_file: &str) -> Self {
        Self::new(desktop_file, DesktopFileStatus::Stopped)
    }
}
