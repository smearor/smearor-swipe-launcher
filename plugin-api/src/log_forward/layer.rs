use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use tracing::field::Field;
use tracing::field::Visit;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

use crate::log_forward::init::GLOBAL_HANDLE;

/// A `tracing_subscriber::Layer` that forwards events to the host via FFI.
///
/// Reads the global `LogForwardHandle` set by `init_plugin_tracing`.
pub struct LogForwardLayer;

impl<S> Layer<S> for LogForwardLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let Some(handle) = GLOBAL_HANDLE.get() else { return };

        let metadata = event.metadata();
        let level = level_to_u8(*metadata.level());
        let target = metadata.target();
        let file = metadata.file();
        let line = metadata.line().unwrap_or(0);

        let mut visitor = ForwardVisitor::default();
        event.record(&mut visitor);

        let timestamp_ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);

        let target_bytes = target.as_bytes();
        let message_bytes = visitor.message.as_bytes();
        let (file_ptr, file_len) = if let Some(ref f) = file {
            (f.as_ptr(), f.len())
        } else {
            (core::ptr::null(), 0)
        };

        // SAFETY: all pointers are valid for the duration of the call.
        // The host callback copies the data before returning.
        unsafe {
            (handle.forward)(
                handle.context,
                level,
                target_bytes.as_ptr(),
                target_bytes.len(),
                message_bytes.as_ptr(),
                message_bytes.len(),
                file_ptr,
                file_len,
                line,
                timestamp_ms,
            );
        }
    }
}

fn level_to_u8(level: tracing::Level) -> u8 {
    match level {
        tracing::Level::ERROR => 1,
        tracing::Level::WARN => 2,
        tracing::Level::INFO => 3,
        tracing::Level::DEBUG => 4,
        tracing::Level::TRACE => 5,
    }
}

/// Visitor that extracts the formatted message from tracing event fields.
#[derive(Default)]
struct ForwardVisitor {
    message: String,
}

impl Visit for ForwardVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }
}
