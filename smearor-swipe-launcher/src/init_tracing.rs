use miette::IntoDiagnostic;
use miette::Result;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::FmtSubscriber;

/// Initialize the tracing subscriber and log tracer.
pub fn init() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .finish();

    tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;
    tracing_log::LogTracer::init().into_diagnostic()?;
    Ok(())
}
