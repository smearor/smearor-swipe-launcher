use smearor_model_mcp::UnknownPromptError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP prompts registered by the wallpaper service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WallpaperMcpPrompts {
    /// Guide for wallpaper theme management: select, start, stop, add, remove.
    WallpaperGuide,
}

impl AsRef<str> for WallpaperMcpPrompts {
    fn as_ref(&self) -> &str {
        match self {
            Self::WallpaperGuide => "wallpaper_guide",
        }
    }
}

impl FromStr for WallpaperMcpPrompts {
    type Err = UnknownPromptError;

    fn from_str(prompt: &str) -> Result<Self, Self::Err> {
        match prompt {
            "wallpaper_guide" => Ok(Self::WallpaperGuide),
            _ => Err(UnknownPromptError::new(prompt)),
        }
    }
}

impl Display for WallpaperMcpPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
