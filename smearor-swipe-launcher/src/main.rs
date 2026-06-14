mod application;
mod area;
mod args;
mod config;
mod context;
mod css_provider;
mod display;
mod error;
mod messages;
mod plugin;
mod plugin_manager;
mod service;
mod service_manager;
mod window;

pub use application::LauncherApplication;
pub use args::launcher::SwipeLauncherArguments;
pub use config::area::config::AreaConfig;
pub use config::launcher::SwipeLauncherConfig;
pub use config::plugin::PluginEntry;
pub use plugin::LoadedPlugin;
pub use plugin_manager::PluginManager;
pub use service::LoadedService;
pub use service_manager::ServiceManager;

use clap::Parser;
use gtk4::Application;
use miette::IntoDiagnostic;
use miette::Result;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::FmtSubscriber;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = SwipeLauncherArguments::parse();

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .finish();

    tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;

    info!("Starting smearor-swipe-launcher");
    info!("Config file: {:?}", args.config);

    let config = args.load_config_from_file()?;
    let total_plugins: usize = config
        .areas
        .iter()
        .filter_map(|area_id| config.get_area_config(area_id))
        .map(|area_config| area_config.plugins.len())
        .sum();
    info!("Loaded configuration with {} plugins across {} areas", total_plugins, config.areas.len());

    let initial_rotation = &config.launcher.rotation.rotation.unwrap_or_default();
    info!("Initial rotation: {} degrees", initial_rotation.to_degrees());

    let gtk_app = Application::builder().application_id("com.smearor.swipe-launcher").build();

    let app = Arc::new(LauncherApplication::new(config.clone(), gtk_app.clone()));
    app.load_services();
    app.load_plugins();

    info!("Application initialized successfully");

    app.clone().build_ui(&config)?;
    app.run();

    Ok(())
}
