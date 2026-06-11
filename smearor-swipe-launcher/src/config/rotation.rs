use crate::args::rotation::RotationArguments;
use crate::config::merge::MergeWithArguments;
use serde::Deserialize;
use smearor_wrot_rotation::SmearorRotation;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RotationConfigFile {
    /// Disable the rotation widget even if a rotation value is provided.
    #[serde(default)]
    pub(crate) disable_rotation: Option<bool>,

    /// Rotation angle in degrees.
    #[serde(default)]
    pub(crate) rotation: Option<SmearorRotation>,

    /// Animation speed in milliseconds for rotation overshoot animation (default: 500).
    #[serde(default)]
    pub(crate) animation_speed: Option<u64>,

    /// Animation overshoot amount for rotation gesture (default: 1.7).
    #[serde(default)]
    pub(crate) animation_overshoot: Option<f64>,

    /// Disable all animations.
    #[serde(default)]
    pub(crate) disable_animations: Option<bool>,
}

impl RotationConfigFile {
    pub fn rotation_enabled(&self) -> bool {
        !self.disable_rotation.unwrap_or_default()
    }

    pub fn rotation(&self) -> SmearorRotation {
        self.rotation.unwrap_or_default()
    }

    pub fn animation_speed(&self) -> u64 {
        self.animation_speed.unwrap_or(500)
    }

    pub fn animation_overshoot(&self) -> f64 {
        self.animation_overshoot.unwrap_or(1.7)
    }

    pub fn animations_enabled(&self) -> bool {
        !self.disable_animations.unwrap_or_default()
    }
}

impl MergeWithArguments<RotationArguments> for RotationConfigFile {
    fn merge_with_arguments(self, args: &RotationArguments) -> Self {
        let mut config = self;
        if let Some(disable_rotation) = args.disable_rotation {
            config.disable_rotation = Some(disable_rotation);
        }
        if let Some(rotation) = args.rotation {
            config.rotation = Some(rotation);
        }
        if let Some(animation_speed) = args.animation_speed {
            config.animation_speed = Some(animation_speed);
        }
        if let Some(animation_overshoot) = args.animation_overshoot {
            config.animation_overshoot = Some(animation_overshoot);
        }
        if let Some(disable_animations) = args.disable_animations {
            config.disable_animations = Some(disable_animations);
        }
        config
    }
}
