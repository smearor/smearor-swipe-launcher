use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the wallpaper service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WallpaperMcpTools {
    /// Select a wallpaper theme by name.
    SelectTheme,
    /// Start the selected wallpaper process.
    StartSelectedProcess,
    /// Stop the current wallpaper process.
    StopCurrentProcess,
    /// Add a new wallpaper theme.
    AddTheme,
    /// Remove a wallpaper theme by name.
    RemoveTheme,
}

impl AsRef<str> for WallpaperMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::SelectTheme => "select_wallpaper_theme",
            Self::StartSelectedProcess => "start_selected_wallpaper_process",
            Self::StopCurrentProcess => "stop_current_wallpaper_process",
            Self::AddTheme => "add_wallpaper_theme",
            Self::RemoveTheme => "remove_wallpaper_theme",
        }
    }
}

impl FromStr for WallpaperMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "select_wallpaper_theme" => Ok(Self::SelectTheme),
            "start_selected_wallpaper_process" => Ok(Self::StartSelectedProcess),
            "stop_current_wallpaper_process" => Ok(Self::StopCurrentProcess),
            "add_wallpaper_theme" => Ok(Self::AddTheme),
            "remove_wallpaper_theme" => Ok(Self::RemoveTheme),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for WallpaperMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
