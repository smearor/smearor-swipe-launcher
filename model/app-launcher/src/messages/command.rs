use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;

pub const TOPIC_COMMAND: &str = "service/app_launcher/command";

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

    /// Whether the desktop file should be executed with the same rotation as the swipe launcher
    pub follows_rotation: bool,

    /// The action to execute on the desktop file
    pub action: DesktopFileCommandAction,
}

impl DesktopFileCommandMessage {
    pub fn new(desktop_file: &str, follows_rotation: bool, action: DesktopFileCommandAction) -> Self {
        Self {
            desktop_file: desktop_file.to_string(),
            follows_rotation,
            action,
        }
    }

    pub fn exec(desktop_file: &str, follows_rotation: bool) -> Self {
        Self::new(desktop_file, follows_rotation, DesktopFileCommandAction::Exec)
    }

    pub fn exec_start(desktop_file: &str, follows_rotation: bool) -> Self {
        Self::new(desktop_file, follows_rotation, DesktopFileCommandAction::ExecStart)
    }

    pub fn terminate(desktop_file: &str, follows_rotation: bool) -> Self {
        Self::new(desktop_file, follows_rotation, DesktopFileCommandAction::Terminate)
    }
}

impl MessageTopic for DesktopFileCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}
