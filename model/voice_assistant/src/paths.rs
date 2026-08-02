/// Returns the XDG model directory: `$XDG_DATA_HOME/smearor/models/`.
/// Falls back to `~/.local/share/smearor/models/` when `XDG_DATA_HOME` is unset.
pub fn xdg_models_dir() -> String {
    let base = dirs::data_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{home}/.local/share")
    });
    format!("{base}/smearor/models")
}

/// Returns the path to a file in the XDG config directory: `$XDG_CONFIG_HOME/smearor/<filename>`.
/// Falls back to `~/.config/smearor/<filename>` when `XDG_CONFIG_HOME` is unset.
pub fn xdg_config_path(filename: &str) -> String {
    let base = dirs::config_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{home}/.config")
    });
    format!("{base}/smearor/{filename}")
}
