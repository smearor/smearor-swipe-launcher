mod application;
mod args;
mod config;
mod context;
mod controller;
mod display;
mod error;
mod messages;
mod plugin;
mod plugin_manager;
mod window;

use crate::args::SmearorWipeLauncherArgs;
use application::LauncherApplication;
use clap::Parser;
use config::LauncherConfig;
use gtk4::Application;
use miette::IntoDiagnostic;
use miette::Result;
use miette::miette;
use smearor_wrot_rotation::SmearorRotation;
use std::sync::Arc;
use tracing::error;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::FmtSubscriber;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = SmearorWipeLauncherArgs::parse();

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .finish();

    tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;

    info!("Starting smearor-swipe-launcher");
    info!("Config file: {:?}", args.config);

    let mut config = if let Some(config_path) = args.config {
        let config_content = std::fs::read_to_string(&config_path).into_diagnostic()?;
        let config: LauncherConfig = toml::from_str(&config_content).into_diagnostic()?;
        info!("Loaded configuration with {} plugins in scroll band", config.scroll_band.plugins.len());
        config
    } else {
        error!("No config file specified");
        return Err(miette!("No config file specified"));
    };

    if let Some(rotation) = args.rotation {
        config.launcher.rotation = SmearorRotation::Deg(rotation);
    }

    info!("Initial rotation: {} degrees", config.launcher.rotation.to_degrees());

    let gtk_app = Application::builder().application_id("com.smearor.swipe-launcher").build();

    let app = Arc::new(LauncherApplication::new(config.clone(), gtk_app.clone()));
    app.load_plugins();

    info!("Application initialized successfully");

    app.clone().build_ui(&config)?;
    app.run();

    Ok(())
}
