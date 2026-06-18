use serde::Deserialize;
use serde::Serialize;

/// Window configuration section
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct WindowConfigFile {
    /// Aspect ratio
    #[serde(default)]
    pub aspect_ratio: Option<f32>,

    /// Start in fullscreen mode
    #[serde(default)]
    pub fullscreen: Option<bool>,

    /// Initial height
    #[serde(default)]
    pub height: Option<i32>,

    /// Minimum height
    #[serde(default)]
    pub min_height: Option<i32>,

    /// Minimum width
    #[serde(default)]
    pub min_width: Option<i32>,

    /// Maximum height
    #[serde(default)]
    pub max_height: Option<i32>,

    /// Maximum width
    #[serde(default)]
    pub max_width: Option<i32>,

    /// Start in maximized mode
    #[serde(default)]
    pub maximized: Option<bool>,

    /// Whether the window should be resizable
    #[serde(default)]
    pub resizable: Option<bool>,

    /// Whether the window should have decorations
    #[serde(default)]
    pub show_decorations: Option<bool>,

    /// Title of the application window.
    #[serde(default)]
    pub title: Option<String>,

    /// Initial width
    #[serde(default)]
    pub width: Option<i32>,

    /// Window opacity for the compositor window (0.0 = fully transparent, 1.0 = fully opaque).
    #[serde(default)]
    pub window_opacity: Option<f32>,

    /// Initial x position
    #[serde(default)]
    pub x: Option<i32>,

    /// Initial y position
    #[serde(default)]
    pub y: Option<i32>,
}

/// ABI-stable version of `WindowConfigFile` for cross-plugin messaging.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct WindowConfigFileStabby {
    pub aspect_ratio: stabby::option::Option<f32>,
    pub fullscreen: stabby::option::Option<bool>,
    pub height: stabby::option::Option<i32>,
    pub min_height: stabby::option::Option<i32>,
    pub min_width: stabby::option::Option<i32>,
    pub max_height: stabby::option::Option<i32>,
    pub max_width: stabby::option::Option<i32>,
    pub maximized: stabby::option::Option<bool>,
    pub resizable: stabby::option::Option<bool>,
    pub show_decorations: stabby::option::Option<bool>,
    pub title: stabby::option::Option<stabby::string::String>,
    pub width: stabby::option::Option<i32>,
    pub window_opacity: stabby::option::Option<f32>,
    pub x: stabby::option::Option<i32>,
    pub y: stabby::option::Option<i32>,
}

impl From<WindowConfigFile> for WindowConfigFileStabby {
    fn from(value: WindowConfigFile) -> Self {
        Self {
            aspect_ratio: value.aspect_ratio.map(Into::into).into(),
            fullscreen: value.fullscreen.map(Into::into).into(),
            height: value.height.map(Into::into).into(),
            min_height: value.min_height.map(Into::into).into(),
            min_width: value.min_width.map(Into::into).into(),
            max_height: value.max_height.map(Into::into).into(),
            max_width: value.max_width.map(Into::into).into(),
            maximized: value.maximized.map(Into::into).into(),
            resizable: value.resizable.map(Into::into).into(),
            show_decorations: value.show_decorations.map(Into::into).into(),
            title: value.title.map(Into::into).into(),
            width: value.width.map(Into::into).into(),
            window_opacity: value.window_opacity.map(Into::into).into(),
            x: value.x.map(Into::into).into(),
            y: value.y.map(Into::into).into(),
        }
    }
}

impl From<WindowConfigFileStabby> for WindowConfigFile {
    fn from(value: WindowConfigFileStabby) -> Self {
        Self {
            aspect_ratio: {
                let opt: Option<f32> = value.aspect_ratio.into();
                opt
            },
            fullscreen: {
                let opt: Option<bool> = value.fullscreen.into();
                opt
            },
            height: {
                let opt: Option<i32> = value.height.into();
                opt
            },
            min_height: {
                let opt: Option<i32> = value.min_height.into();
                opt
            },
            min_width: {
                let opt: Option<i32> = value.min_width.into();
                opt
            },
            max_height: {
                let opt: Option<i32> = value.max_height.into();
                opt
            },
            max_width: {
                let opt: Option<i32> = value.max_width.into();
                opt
            },
            maximized: {
                let opt: Option<bool> = value.maximized.into();
                opt
            },
            resizable: {
                let opt: Option<bool> = value.resizable.into();
                opt
            },
            show_decorations: {
                let opt: Option<bool> = value.show_decorations.into();
                opt
            },
            title: {
                let opt: Option<stabby::string::String> = value.title.into();
                opt.map(|s| s.to_string())
            },
            width: {
                let opt: Option<i32> = value.width.into();
                opt
            },
            window_opacity: {
                let opt: Option<f32> = value.window_opacity.into();
                opt
            },
            x: {
                let opt: Option<i32> = value.x.into();
                opt
            },
            y: {
                let opt: Option<i32> = value.y.into();
                opt
            },
        }
    }
}

impl WindowConfigFile {
    pub fn args(&self) -> Vec<String> {
        let mut args = vec![];
        if !self.show_decorations.unwrap_or_default() {
            args.push("--decorated".to_string());
        }
        // TODO: Changed CLI args
        // if self.show_decorations.unwrap_or_default() {
        //     args.push("--show-decorations".to_string());
        // }
        args
    }
}
