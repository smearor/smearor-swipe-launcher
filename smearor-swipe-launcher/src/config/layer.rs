use crate::args::layer::LayerArguments;
use crate::config::merge::MergeWithArguments;
use serde::Deserialize;
use smearor_wrot_rotation::layer::SmearorLayer;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LayerConfigFile {
    /// Specify the layer for the layer shell protocol (e.g., Background, Top).
    #[serde(default)]
    pub(crate) layer: Option<SmearorLayer>,

    /// Namespace for the layer shell, used by compositors for rules.
    #[serde(default)]
    pub(crate) namespace: Option<String>,
}

impl MergeWithArguments<LayerArguments> for LayerConfigFile {
    fn merge_with_arguments(self, args: &LayerArguments) -> Self {
        let mut config = self;
        if let Some(layer) = args.layer {
            config.layer = Some(layer);
        }
        if let Some(namespace) = &args.namespace {
            config.namespace = Some(namespace.clone());
        }
        config
    }
}
