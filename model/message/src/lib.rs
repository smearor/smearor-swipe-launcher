use serde::Deserialize;
use serde::Serialize;

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
