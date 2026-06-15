use crate::messages::wrapper::color_mask::ColorMaskConfigFile;
use crate::messages::wrapper::layer::LayerConfigFile;
use crate::messages::wrapper::rotation::RotationConfigFile;
use crate::messages::wrapper::window::WindowConfigFile;
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
