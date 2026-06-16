use crate::SwipeLauncherConfig;
use crate::args::layer::LayerArguments;
use crate::args::rotation::RotationArguments;
use clap::Parser;
use miette::IntoDiagnostic;
use miette::Result;
use miette::miette;
use std::path::PathBuf;
use tracing::trace;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct SwipeLauncherArguments {
    #[arg(short, long, default_value = Some("config.toml"))]
    pub(crate) config: Option<PathBuf>,

    #[command(flatten)]
    pub(crate) rotation: RotationArguments,

    #[command(flatten)]
    pub(crate) layer: LayerArguments,
}

impl SwipeLauncherArguments {
    pub fn load_config_from_file(self) -> Result<SwipeLauncherConfig> {
        match &self.config {
            Some(config_path) => {
                let config_content = std::fs::read_to_string(config_path).into_diagnostic()?;
                let config: SwipeLauncherConfig = toml::from_str(&config_content).into_diagnostic()?;
                let total_plugins: usize = config
                    .areas
                    .iter()
                    .filter_map(|area_id| config.get_area_config(area_id))
                    .map(|area_config| area_config.plugins.len())
                    .sum();
                trace!("Loaded configuration with {} plugins across {} areas", total_plugins, config.areas.len());
                Ok(config)
            }
            None => Err(miette!("No config file specified")),
        }
    }
}
