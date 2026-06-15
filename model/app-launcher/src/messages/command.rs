use crate::SmearorWindowRotationWrapper;
use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;

pub const TOPIC_COMMAND: &str = "service.app_launcher.command";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum DesktopFileCommandAction {
    #[default]
    Exec,
    ExecStart,
    ExecReload,
    Terminate,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DesktopFileCommandMessage {
    /// The canonical path of the desktop file
    pub desktop_file: String,

    /// The smearor window rotation wrapper configuration
    pub wrapper: Option<SmearorWindowRotationWrapper>,

    /// The action to execute on the desktop file
    pub action: DesktopFileCommandAction,
}

impl DesktopFileCommandMessage {
    pub fn new(desktop_file: &str, wrapper: Option<SmearorWindowRotationWrapper>, action: DesktopFileCommandAction) -> Self {
        Self {
            desktop_file: desktop_file.to_string(),
            wrapper,
            action,
        }
    }

    pub fn exec(desktop_file: &str, wrapper: Option<SmearorWindowRotationWrapper>) -> Self {
        Self::new(desktop_file, wrapper, DesktopFileCommandAction::Exec)
    }

    pub fn exec_start(desktop_file: &str, wrapper: Option<SmearorWindowRotationWrapper>) -> Self {
        Self::new(desktop_file, wrapper, DesktopFileCommandAction::ExecStart)
    }

    pub fn terminate(desktop_file: &str, wrapper: Option<SmearorWindowRotationWrapper>) -> Self {
        Self::new(desktop_file, wrapper, DesktopFileCommandAction::Terminate)
    }
}

impl MessageTopic for DesktopFileCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}
