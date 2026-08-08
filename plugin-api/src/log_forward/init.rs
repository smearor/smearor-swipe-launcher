use std::sync::OnceLock;

use tracing_subscriber::prelude::*;

use crate::log_forward::handle::LogForwardHandle;
use crate::log_forward::layer::LogForwardLayer;

/// Global handle set by `init_plugin_tracing`.
///
/// Kept in a `OnceLock` so the `LogForwardLayer` can access it without
/// borrowing issues — the layer must be `'static` for `set_global_default`.
pub static GLOBAL_HANDLE: OnceLock<LogForwardHandle> = OnceLock::new();

/// Initialise plugin tracing with the host's log-forward callback.
///
/// Installs a `tracing_subscriber` whose `LogForwardLayer` forwards every
/// event to the host via the FFI callback.  Falls back to a `FmtSubscriber`
/// (stdout) when no handle is available.
///
/// Called automatically by `service_plugin!` and `widget_plugin!` macros.
pub fn init_plugin_tracing(handle: Option<LogForwardHandle>) {
    let filter = tracing_subscriber::EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into());
    if let Some(handle) = handle {
        let _ = GLOBAL_HANDLE.set(handle);
        let fmt_layer = tracing_subscriber::fmt::layer().with_filter(filter);
        let forward_layer = LogForwardLayer;
        let subscriber = tracing_subscriber::registry().with(fmt_layer).with(forward_layer);
        let _ = tracing::subscriber::set_global_default(subscriber);
    } else {
        let subscriber = tracing_subscriber::FmtSubscriber::builder().with_env_filter(filter).finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    }
}
