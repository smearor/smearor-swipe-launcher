use crate::messages::wrapper::color_mask::ColorMaskConfigFile;
use crate::messages::wrapper::color_mask::ColorMaskConfigFileStabby;
use crate::messages::wrapper::layer::LayerConfigFile;
use crate::messages::wrapper::layer::LayerConfigFileStabby;
use crate::messages::wrapper::rotation::RotationConfigFile;
use crate::messages::wrapper::rotation::RotationConfigFileStabby;
use crate::messages::wrapper::window::WindowConfigFile;
use crate::messages::wrapper::window::WindowConfigFileStabby;
use serde::Deserialize;
use serde::Serialize;

// TODO: use config file struct(s) from smearor-wrot
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SmearorWindowRotationWrapper {
    /// Whether the desktop file should be executed with the same rotation as the swipe launcher
    pub follows_rotation: bool,

    #[serde(flatten)]
    pub color_mask: ColorMaskConfigFile,

    #[serde(flatten)]
    pub layer: LayerConfigFile,

    #[serde(flatten)]
    pub rotation: RotationConfigFile,

    #[serde(flatten)]
    pub window: WindowConfigFile,
}

/// ABI-stable version of `SmearorWindowRotationWrapper` for cross-plugin messaging.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct SmearorWindowRotationWrapperStabby {
    pub follows_rotation: bool,
    pub color_mask: ColorMaskConfigFileStabby,
    pub layer: LayerConfigFileStabby,
    pub rotation: RotationConfigFileStabby,
    pub window: WindowConfigFileStabby,
}

impl From<SmearorWindowRotationWrapper> for SmearorWindowRotationWrapperStabby {
    fn from(value: SmearorWindowRotationWrapper) -> Self {
        Self {
            follows_rotation: value.follows_rotation,
            color_mask: value.color_mask.into(),
            layer: value.layer.into(),
            rotation: value.rotation.into(),
            window: value.window.into(),
        }
    }
}

impl From<SmearorWindowRotationWrapperStabby> for SmearorWindowRotationWrapper {
    fn from(value: SmearorWindowRotationWrapperStabby) -> Self {
        Self {
            follows_rotation: value.follows_rotation,
            color_mask: value.color_mask.into(),
            layer: value.layer.into(),
            rotation: value.rotation.into(),
            window: value.window.into(),
        }
    }
}

impl SmearorWindowRotationWrapper {
    pub fn args(&self, rotation: Option<f32>) -> Vec<String> {
        let mut args = vec![];
        args.append(&mut self.color_mask.args());
        args.append(&mut self.layer.args());
        args.append(&mut self.rotation.args(rotation));
        args.append(&mut self.window.args());
        args
    }
}
