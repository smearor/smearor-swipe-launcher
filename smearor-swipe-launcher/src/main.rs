mod application;
mod config;
mod error;
mod messages;
mod plugin;

use application::LauncherApplication;
use clap::Parser;
use config::LauncherConfig;
use error::Result;
use serde_json::json;
use smearor_plugin_api::PluginConfig;
use smearor_wrot_rotation::SmearorRotation;
use std::path::PathBuf;
use tracing::error;
use tracing::info;
use tracing::warn;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    #[arg(short, long, default_value = "0")]
    rotation: u32,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting smearor-swipe-launcher");
    info!("Config file: {:?}", args.config);
    info!("Initial rotation: {} degrees", args.rotation);

    let config_content = std::fs::read_to_string(&args.config)?;
    let config: LauncherConfig = toml::from_str(&config_content)?;
    info!("Loaded configuration with {} plugins in scroll band", config.scroll_band.plugins.len());

    let rotation = SmearorRotation::from(args.rotation.to_string().as_str());

    let gtk_app = gtk4::Application::builder().application_id("com.smearor.swipe-launcher").build();

    let mut app = LauncherApplication::new(rotation, gtk_app.clone());

    for plugin_entry in &config.left_area.plugins {
        let plugin_config_json = config.plugins.get(&plugin_entry.id).cloned().unwrap_or_else(|| {
            warn!("No config found for plugin {}, using empty config", plugin_entry.id);
            json!({})
        });

        let plugin_config = PluginConfig { config: plugin_config_json };

        let plugin_path = PathBuf::from(&plugin_entry.path);
        if let Err(e) = app.load_plugin(plugin_entry.id.clone(), plugin_path, plugin_config) {
            error!("Failed to load plugin {}: {}", plugin_entry.id, e);
        }
    }

    for plugin_entry in &config.scroll_band.plugins {
        let plugin_config_json = config.plugins.get(&plugin_entry.id).cloned().unwrap_or_else(|| {
            warn!("No config found for plugin {}, using empty config", plugin_entry.id);
            json!({})
        });

        let plugin_config = PluginConfig { config: plugin_config_json };

        let plugin_path = PathBuf::from(&plugin_entry.path);
        if let Err(e) = app.load_plugin(plugin_entry.id.clone(), plugin_path, plugin_config) {
            error!("Failed to load plugin {}: {}", plugin_entry.id, e);
        }
    }

    for plugin_entry in &config.right_area.plugins {
        let plugin_config_json = config.plugins.get(&plugin_entry.id).cloned().unwrap_or_else(|| {
            warn!("No config found for plugin {}, using empty config", plugin_entry.id);
            json!({})
        });

        let plugin_config = PluginConfig { config: plugin_config_json };

        let plugin_path = PathBuf::from(&plugin_entry.path);
        if let Err(e) = app.load_plugin(plugin_entry.id.clone(), plugin_path, plugin_config) {
            error!("Failed to load plugin {}: {}", plugin_entry.id, e);
        }
    }

    info!("Application initialized successfully");

    app.build_ui(&config);
    app.run();

    Ok(())
}
