use serde::Deserialize;
use serde::Serialize;
use smearor_wrot_rotation::layer::SmearorLayer;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LayerConfigFile {
    /// Specify the layer for the layer shell protocol (e.g., Background, Top).
    #[serde(default)]
    pub(crate) layer: Option<SmearorLayer>,

    /// Namespace for the layer shell, used by compositors for rules.
    #[serde(default)]
    pub(crate) namespace: Option<String>,
}

impl LayerConfigFile {
    pub fn args(&self) -> Vec<String> {
        let mut args = vec![];
        if let Some(layer) = &self.layer {
            args.push("--layer".to_string());
            args.push(format!("{:?}", layer));
        }
        if let Some(namespace) = &self.namespace {
            args.push("--namespace".to_string());
            args.push(namespace.to_string());
        }
        args
    }
}
