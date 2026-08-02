use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP resources exposed by the wallpaper service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WallpaperMcpResources {
    /// Current wallpaper status including selected theme, running processes, and theme list.
    Status,
    /// Available wallpaper themes.
    Themes,
}

impl AsRef<str> for WallpaperMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::Status => "wallpaper://status",
            Self::Themes => "wallpaper://themes",
        }
    }
}

impl FromStr for WallpaperMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri {
            "wallpaper://status" => Ok(Self::Status),
            "wallpaper://themes" => Ok(Self::Themes),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for WallpaperMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
