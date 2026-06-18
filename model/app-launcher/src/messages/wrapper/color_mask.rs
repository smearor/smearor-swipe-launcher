use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ColorMaskConfigFile {
    #[serde(default)]
    pub background_color: Option<String>,

    #[serde(default)]
    pub color_mask: Option<String>,

    #[serde(default)]
    pub auto_color_mask: Option<bool>,

    #[serde(default)]
    pub subsurface_background_color: Option<String>,

    #[serde(default)]
    pub subsurface_color_mask: Option<String>,

    #[serde(default)]
    pub auto_subsurface_color_mask: Option<bool>,

    #[serde(default)]
    pub color_mask_tolerance: Option<f32>,

    #[serde(default)]
    pub color_mask_shader: Option<bool>,
}

/// ABI-stable version of `ColorMaskConfigFile` for cross-plugin messaging.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ColorMaskConfigFileStabby {
    pub background_color: stabby::option::Option<stabby::string::String>,
    pub color_mask: stabby::option::Option<stabby::string::String>,
    pub auto_color_mask: stabby::option::Option<bool>,
    pub subsurface_background_color: stabby::option::Option<stabby::string::String>,
    pub subsurface_color_mask: stabby::option::Option<stabby::string::String>,
    pub auto_subsurface_color_mask: stabby::option::Option<bool>,
    pub color_mask_tolerance: stabby::option::Option<f32>,
    pub color_mask_shader: stabby::option::Option<bool>,
}

impl From<ColorMaskConfigFile> for ColorMaskConfigFileStabby {
    fn from(value: ColorMaskConfigFile) -> Self {
        Self {
            background_color: value.background_color.map(Into::into).into(),
            color_mask: value.color_mask.map(Into::into).into(),
            auto_color_mask: value.auto_color_mask.map(Into::into).into(),
            subsurface_background_color: value.subsurface_background_color.map(Into::into).into(),
            subsurface_color_mask: value.subsurface_color_mask.map(Into::into).into(),
            auto_subsurface_color_mask: value.auto_subsurface_color_mask.map(Into::into).into(),
            color_mask_tolerance: value.color_mask_tolerance.map(Into::into).into(),
            color_mask_shader: value.color_mask_shader.map(Into::into).into(),
        }
    }
}

impl From<ColorMaskConfigFileStabby> for ColorMaskConfigFile {
    fn from(value: ColorMaskConfigFileStabby) -> Self {
        Self {
            background_color: {
                let opt: Option<stabby::string::String> = value.background_color.into();
                opt.map(|s| s.to_string())
            },
            color_mask: {
                let opt: Option<stabby::string::String> = value.color_mask.into();
                opt.map(|s| s.to_string())
            },
            auto_color_mask: {
                let opt: Option<bool> = value.auto_color_mask.into();
                opt
            },
            subsurface_background_color: {
                let opt: Option<stabby::string::String> = value.subsurface_background_color.into();
                opt.map(|s| s.to_string())
            },
            subsurface_color_mask: {
                let opt: Option<stabby::string::String> = value.subsurface_color_mask.into();
                opt.map(|s| s.to_string())
            },
            auto_subsurface_color_mask: {
                let opt: Option<bool> = value.auto_subsurface_color_mask.into();
                opt
            },
            color_mask_tolerance: {
                let opt: Option<f32> = value.color_mask_tolerance.into();
                opt
            },
            color_mask_shader: {
                let opt: Option<bool> = value.color_mask_shader.into();
                opt
            },
        }
    }
}

impl ColorMaskConfigFile {
    pub fn args(&self) -> Vec<String> {
        let mut args = vec![];
        if let Some(background_color) = &self.background_color {
            args.push("--background-color".to_string());
            args.push(format!("\"{background_color}\""));
        }
        if let Some(color_mask) = &self.color_mask {
            args.push("--color-mask".to_string());
            args.push(format!("\"{color_mask}\""));
        }
        if self.auto_color_mask.unwrap_or_default() {
            args.push("--auto-color-mask".to_string());
        }
        if let Some(subsurface_background_color) = &self.subsurface_background_color {
            args.push("--subsurface-background-color".to_string());
            args.push(format!("\"{subsurface_background_color}\""));
        }
        if let Some(subsurface_color_mask) = &self.subsurface_color_mask {
            args.push("--subsurface-color-mask".to_string());
            args.push(format!("\"{subsurface_color_mask}\""));
        }
        if self.auto_subsurface_color_mask.unwrap_or_default() {
            args.push("--auto-subsurface_color-mask".to_string());
        }
        if self.color_mask_tolerance.unwrap_or_default() > 0.0 {
            args.push("--color-mask-tolerance".to_string());
            args.push(format!("{}", self.color_mask_tolerance.unwrap_or_default()));
        }
        if self.color_mask_shader.unwrap_or_default() {
            args.push("--color-mask-shader".to_string());
        }
        args
    }
}
