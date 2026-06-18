use serde::Deserialize;
use serde::Serialize;
use smearor_wrot_rotation::layer::SmearorLayer;

/// ABI-stable layer enum for cross-plugin messaging.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub enum StabbyLayer {
    Background,
    Bottom,
    #[default]
    Top,
    Overlay,
}

impl From<SmearorLayer> for StabbyLayer {
    fn from(value: SmearorLayer) -> Self {
        match value {
            SmearorLayer::Background => StabbyLayer::Background,
            SmearorLayer::Bottom => StabbyLayer::Bottom,
            SmearorLayer::Top => StabbyLayer::Top,
            SmearorLayer::Overlay => StabbyLayer::Overlay,
        }
    }
}

impl From<StabbyLayer> for SmearorLayer {
    fn from(value: StabbyLayer) -> Self {
        match value {
            StabbyLayer::Background => SmearorLayer::Background,
            StabbyLayer::Bottom => SmearorLayer::Bottom,
            StabbyLayer::Top => SmearorLayer::Top,
            StabbyLayer::Overlay => SmearorLayer::Overlay,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LayerConfigFile {
    /// Specify the layer for the layer shell protocol (e.g., Background, Top).
    #[serde(default)]
    pub(crate) layer: Option<SmearorLayer>,

    /// Namespace for the layer shell, used by compositors for rules.
    #[serde(default)]
    pub(crate) namespace: Option<String>,
}

/// ABI-stable version of `LayerConfigFile` for cross-plugin messaging.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct LayerConfigFileStabby {
    pub layer: stabby::option::Option<StabbyLayer>,
    pub namespace: stabby::option::Option<stabby::string::String>,
}

impl From<LayerConfigFile> for LayerConfigFileStabby {
    fn from(value: LayerConfigFile) -> Self {
        Self {
            layer: value.layer.map(Into::into).into(),
            namespace: value.namespace.map(Into::into).into(),
        }
    }
}

impl From<LayerConfigFileStabby> for LayerConfigFile {
    fn from(value: LayerConfigFileStabby) -> Self {
        Self {
            layer: {
                let opt: Option<StabbyLayer> = value.layer.into();
                opt.map(Into::into)
            },
            namespace: {
                let opt: Option<stabby::string::String> = value.namespace.into();
                opt.map(|s| s.to_string())
            },
        }
    }
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
