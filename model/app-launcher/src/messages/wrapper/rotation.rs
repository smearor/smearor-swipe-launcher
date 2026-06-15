use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RotationConfigFile {
    /// Disable the rotation widget even if a rotation value is provided.
    #[serde(default)]
    pub(crate) disable_rotation: Option<bool>,

    /// Rotation angle in degrees.
    #[serde(default)]
    pub(crate) rotation: Option<f32>,

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
    pub fn args(&self, rotation: Option<f32>) -> Vec<String> {
        let mut args = vec![];
        if self.disable_rotation.unwrap_or_default() {
            args.push("--disable-rotation".to_string());
        }
        if let Some(rotation) = &self.rotation {
            args.push("--rotation".to_string());
            args.push(rotation.to_string());
        } else if let Some(rotation) = rotation {
            args.push("--rotation".to_string());
            args.push(rotation.to_string());
        }
        if let Some(animation_speed) = &self.animation_speed {
            args.push("--animation-speed".to_string());
            args.push(animation_speed.to_string());
        }
        if let Some(animation_overshoot) = &self.animation_overshoot {
            args.push("--animation-overshoot".to_string());
            args.push(animation_overshoot.to_string());
        }
        if self.disable_animations.unwrap_or_default() {
            args.push("--disable-animations".to_string());
        }
        args
    }
}
