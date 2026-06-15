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
