use crate::SwipeLauncherConfig;
use crate::args::layer::LayerArguments;
use crate::args::rotation::RotationArguments;
use crate::config::services::ServicesConfig;
use clap::Parser;
use miette::IntoDiagnostic;
use miette::Result;
use miette::miette;
use std::path::PathBuf;
use tracing::trace;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct SwipeLauncherArguments {
    /// Configuration file for shared background services.
    /// Loaded once and shared across all launcher instances.
    #[arg(short = 's', long, default_value = Some("services.toml"))]
    pub(crate) services_config: Option<PathBuf>,

    /// Configuration files for each launcher instance window.
    /// Specify multiple times for multiple windows.
    #[arg(short, long, default_value = Some("config.toml"))]
    pub(crate) config: Vec<PathBuf>,

    /// Optional instance IDs corresponding to each --config.
    /// If omitted, the config file stem is used as the instance ID.
    #[arg(short = 'i', long)]
    pub(crate) instance_id: Vec<String>,

    #[command(flatten)]
    pub(crate) rotation: RotationArguments,

    #[command(flatten)]
    pub(crate) layer: LayerArguments,
}

impl SwipeLauncherArguments {
    pub fn load_config_from_file(&self, config_path: &PathBuf) -> Result<SwipeLauncherConfig> {
        let config_content = std::fs::read_to_string(config_path).into_diagnostic()?;
        let mut config: SwipeLauncherConfig = toml::from_str(&config_content).into_diagnostic()?;

        // Resolve global defaults for plugin configs (layered: hard-coded → global → instance)
        config.resolve_defaults();

        // Resolve area includes (external TOML files referenced by area configs)
        config.resolve_includes(config_path).map_err(|e| miette::miette!("{e}"))?;

        let total_plugins: usize = config
            .areas
            .iter()
            .filter_map(|area_id| config.get_area_config(area_id))
            .map(|area_config| area_config.plugins.len())
            .sum();
        trace!("Loaded configuration with {} plugins across {} areas", total_plugins, config.areas.len());
        Ok(config)
    }

    pub fn load_services_config(&self) -> Result<ServicesConfig> {
        match &self.services_config {
            Some(config_path) => {
                let config_content = std::fs::read_to_string(config_path).into_diagnostic()?;
                let config: ServicesConfig = toml::from_str(&config_content).into_diagnostic()?;
                trace!("Loaded services configuration with {} services", config.services.len());
                Ok(config)
            }
            None => Err(miette!("No services config file specified")),
        }
    }
}
