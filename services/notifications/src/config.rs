use serde::Deserialize;
use typed_builder::TypedBuilder;

/// Configuration for the notifications service plugin.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct NotificationServiceConfig {
    /// Maximum number of notifications to keep in history
    #[builder(default = 50)]
    pub(crate) max_history: usize,
    /// Default timeout for notifications in milliseconds
    #[builder(default = 5000)]
    pub(crate) default_timeout_ms: i32,
}

impl Default for NotificationServiceConfig {
    fn default() -> Self {
        Self {
            max_history: 50,
            default_timeout_ms: 5000,
        }
    }
}
