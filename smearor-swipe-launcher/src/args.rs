use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct SmearorWipeLauncherArgs {
    #[arg(short, long, default_value = Some("config.toml"))]
    pub(crate) config: Option<PathBuf>,

    #[arg(short, long)]
    pub(crate) rotation: Option<f32>,
}
