mod application;
mod area;
mod args;
mod config;
mod context;
mod css_provider;
mod display;
mod error;
mod instance;
mod json_converter;
mod messages;
mod plugin;
mod plugin_manager;
mod service;
mod service_manager;
mod window;

pub use application::LauncherHost;
pub use args::launcher::SwipeLauncherArguments;
pub use config::launcher::SwipeLauncherConfig;
pub use plugin::LoadedPlugin;
pub use plugin_manager::PluginManager;
pub use service::LoadedService;
pub use service_manager::ServiceManager;

use clap::Parser;
use gtk4::Application;
use miette::IntoDiagnostic;
use miette::Result;
use tracing::debug;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::FmtSubscriber;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = SwipeLauncherArguments::parse();

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .finish();

    tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;

    debug!("Starting smearor-swipe-launcher with config files: {:?}", args.config);

    let gtk_app = Application::builder().application_id("com.smearor.swipe-launcher").build();

    let host = LauncherHost::new(gtk_app.clone());

    // Load shared services from dedicated config
    let services_config = args.load_services_config()?;
    host.load_services(&services_config);

    // Create one instance per config file
    for (index, config_path) in args.config.iter().enumerate() {
        let config = args.load_config_from_file(config_path)?;
        let instance_id = args
            .instance_id
            .get(index)
            .cloned()
            .unwrap_or_else(|| config_path.file_stem().unwrap_or_default().to_string_lossy().to_string());
        host.create_instance(instance_id, config);
    }

    debug!("Application initialized successfully");

    host.build_ui()?;
    host.run();

    Ok(())
}
