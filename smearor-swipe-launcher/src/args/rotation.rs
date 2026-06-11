use clap::Parser;
use smearor_wrot_rotation::SmearorRotation;

#[derive(Parser, Debug, Clone)]
pub struct RotationArguments {
    /// Disable the rotation widget even if a rotation value is provided.
    #[arg(short = 'R', long, action)]
    pub(crate) disable_rotation: Option<bool>,

    /// Rotation angle in degrees.
    #[arg(short, long)]
    pub(crate) rotation: Option<SmearorRotation>,

    /// Animation speed in milliseconds for rotation overshoot animation (default: 500).
    #[arg(long)]
    pub(crate) animation_speed: Option<u64>,

    /// Animation overshoot amount for rotation gesture (default: 1.7).
    #[arg(long)]
    pub(crate) animation_overshoot: Option<f64>,

    /// Disable all animations.
    #[arg(long, action)]
    pub(crate) disable_animations: Option<bool>,
}
