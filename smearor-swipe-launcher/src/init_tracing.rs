use std::sync::Arc;

use miette::IntoDiagnostic;
use miette::Result;
use smearor_mcp_server::LogBuffer;
use smearor_mcp_server::LogBufferLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;

const DEFAULT_LOG_BUFFER_CAPACITY: usize = 10000;

/// Initialize the tracing subscriber and log tracer.
///
/// Returns `Some(Arc<LogBuffer>)` when log capture is enabled, or `None` when
/// `enabled` is `false` or `capacity` is `0`. When `None`, no `LogBufferLayer`
/// is installed — zero overhead on the tracing hot path.
pub fn init(enabled: bool, capacity: usize) -> Result<Option<Arc<LogBuffer>>> {
    let log_buffer = if enabled && capacity > 0 {
        Some(Arc::new(LogBuffer::new(capacity)))
    } else {
        None
    };

    let fmt_layer = fmt::layer();
    let filter = EnvFilter::from_default_env()
        .add_directive(tracing::Level::DEBUG.into())
        .add_directive("hyper_util=warn".parse().unwrap_or_else(|_| tracing::Level::DEBUG.into()));

    match log_buffer {
        Some(ref buffer) => {
            let subscriber = tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer)
                .with(LogBufferLayer::new(buffer.clone()));
            tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;
        }
        None => {
            let subscriber = tracing_subscriber::registry().with(filter).with(fmt_layer);
            tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;
        }
    }

    tracing_log::LogTracer::init().into_diagnostic()?;
    Ok(log_buffer)
}
