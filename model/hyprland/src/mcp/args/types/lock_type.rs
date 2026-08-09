use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandLockType;

/// Lock action type for the lock command.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpLockType {
    /// Lock the session.
    #[default]
    Lock,
    /// Unlock the session.
    Unlock,
    /// Toggle the lock state of the session.
    ToggleLock,
}

impl From<McpLockType> for HyprlandLockType {
    fn from(value: McpLockType) -> Self {
        match value {
            McpLockType::Lock => HyprlandLockType::Lock,
            McpLockType::Unlock => HyprlandLockType::Unlock,
            McpLockType::ToggleLock => HyprlandLockType::ToggleLock,
        }
    }
}
