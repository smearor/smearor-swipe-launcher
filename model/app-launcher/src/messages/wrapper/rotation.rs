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

/// ABI-stable version of `RotationConfigFile` for cross-plugin messaging.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct RotationConfigFileStabby {
    pub disable_rotation: stabby::option::Option<bool>,
    pub rotation: stabby::option::Option<f32>,
    pub animation_speed: stabby::option::Option<u64>,
    pub animation_overshoot: stabby::option::Option<f64>,
    pub disable_animations: stabby::option::Option<bool>,
}

impl From<RotationConfigFile> for RotationConfigFileStabby {
    fn from(value: RotationConfigFile) -> Self {
        Self {
            disable_rotation: value.disable_rotation.map(Into::into).into(),
            rotation: value.rotation.map(Into::into).into(),
            animation_speed: value.animation_speed.map(Into::into).into(),
            animation_overshoot: value.animation_overshoot.map(Into::into).into(),
            disable_animations: value.disable_animations.map(Into::into).into(),
        }
    }
}

impl From<RotationConfigFileStabby> for RotationConfigFile {
    fn from(value: RotationConfigFileStabby) -> Self {
        Self {
            disable_rotation: {
                let opt: Option<bool> = value.disable_rotation.into();
                opt
            },
            rotation: {
                let opt: Option<f32> = value.rotation.into();
                opt
            },
            animation_speed: {
                let opt: Option<u64> = value.animation_speed.into();
                opt
            },
            animation_overshoot: {
                let opt: Option<f64> = value.animation_overshoot.into();
                opt
            },
            disable_animations: {
                let opt: Option<bool> = value.disable_animations.into();
                opt
            },
        }
    }
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
