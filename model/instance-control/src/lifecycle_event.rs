use serde::Deserialize;
use serde::Serialize;

/// Lifecycle events for launcher instances.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstanceLifecycleEvent {
    /// A new instance was loaded and its window was created.
    #[default]
    Loaded,
    /// An instance was stopped and its window was closed.
    Stopped,
    /// An instance was hot-reloaded (stopped and reloaded with the same ID).
    Reloaded,
}

impl InstanceLifecycleEvent {
    /// Returns the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            InstanceLifecycleEvent::Loaded => "Loaded",
            InstanceLifecycleEvent::Stopped => "Stopped",
            InstanceLifecycleEvent::Reloaded => "Reloaded",
        }
    }

    /// Parse from a string representation.
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "Loaded" => Ok(InstanceLifecycleEvent::Loaded),
            "Stopped" => Ok(InstanceLifecycleEvent::Stopped),
            "Reloaded" => Ok(InstanceLifecycleEvent::Reloaded),
            other => Err(format!("unknown lifecycle event: {}", other)),
        }
    }
}
