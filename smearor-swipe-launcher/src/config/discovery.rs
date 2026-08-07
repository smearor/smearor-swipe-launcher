use miette::Result;
use miette::miette;
use std::path::Path;
use std::path::PathBuf;
use tracing::debug;
use tracing::info;

/// Filenames excluded from launcher config auto-discovery in the working directory.
const EXCLUDED_TOML_FILES: &[&str] = &["services.toml", "wallpaper.toml"];

/// Service for discovering configuration files from CLI arguments, working directory, and XDG config directory.
///
/// Implements the fallback loading order for launcher configs, services config, and wallpaper config:
/// - Launcher configs: CLI `--config` > working dir `*.toml` (excluding `services.toml`/`wallpaper.toml`) > `~/.config/smearor/launcher/*.toml` > `/usr/share/smearor/launcher/*.toml`
/// - Services config: CLI `--services-config` > working dir `services.toml` > `~/.config/smearor/services/services.toml` > `/usr/share/smearor/services/services.toml`
/// - Wallpaper config: working dir `wallpaper.toml` > `~/.config/smearor/services/wallpaper.toml` > `/usr/share/smearor/services/wallpaper.toml`
pub struct ConfigDiscoveryService;

impl ConfigDiscoveryService {
    /// Creates a new `ConfigDiscoveryService`.
    pub fn new() -> Self {
        Self
    }

    /// Returns the XDG config directory for smearor (`~/.config/smearor`).
    fn xdg_config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("smearor"))
    }

    /// Discovers launcher config files based on CLI args and fallback locations.
    ///
    /// If `cli_configs` is non-empty, returns only those paths (no discovery).
    /// Otherwise, scans the working directory and `~/.config/smearor/launcher/` for `*.toml` files,
    /// excluding `services.toml` and `wallpaper.toml`.
    pub fn discover_launcher_configs(&self, cli_configs: &[PathBuf]) -> Result<Vec<PathBuf>> {
        if !cli_configs.is_empty() {
            return Ok(cli_configs.to_vec());
        }

        let mut configs = Vec::new();

        // Priority 2: *.toml in working directory (excluding services.toml and wallpaper.toml)
        if let Ok(cwd) = std::env::current_dir() {
            configs.extend(Self::collect_toml_files(&cwd, EXCLUDED_TOML_FILES));
        }

        if !configs.is_empty() {
            debug!("Discovered {} launcher config(s) in working directory", configs.len());
            return Ok(configs);
        }

        // Priority 3: ~/.config/smearor/launcher/*.toml
        if let Some(xdg_dir) = Self::xdg_config_dir() {
            let launcher_dir = xdg_dir.join("launcher");
            configs.extend(Self::collect_toml_files(&launcher_dir, &[]));
        }

        if !configs.is_empty() {
            debug!("Discovered {} launcher config(s) in XDG config directory", configs.len());
            return Ok(configs);
        }

        // Priority 4: /usr/share/smearor/launcher/*.toml (system default)
        let system_dir = Path::new("/usr/share/smearor/launcher");
        configs.extend(Self::collect_toml_files(system_dir, &[]));

        if !configs.is_empty() {
            debug!("Discovered {} launcher config(s) in system directory", configs.len());
        }

        Ok(configs)
    }

    /// Discovers the services config file based on CLI arg and fallback locations.
    ///
    /// If `cli_services_config` is provided, returns only that path.
    /// Otherwise, checks the working directory for `services.toml`, then `~/.config/smearor/services/services.toml`,
    /// then `/usr/share/smearor/services/services.toml`.
    pub fn discover_services_config(&self, cli_services_config: Option<&PathBuf>) -> Result<Option<PathBuf>> {
        if let Some(path) = cli_services_config {
            return Ok(Some(path.clone()));
        }

        // Priority 2: services.toml in working directory
        if let Ok(cwd) = std::env::current_dir() {
            let path = cwd.join("services.toml");
            if path.is_file() {
                debug!("Discovered services config in working directory: {}", path.display());
                return Ok(Some(path));
            }
        }

        // Priority 3: ~/.config/smearor/services/services.toml
        if let Some(xdg_dir) = Self::xdg_config_dir() {
            let path = xdg_dir.join("services").join("services.toml");
            if path.is_file() {
                debug!("Discovered services config in XDG config directory: {}", path.display());
                return Ok(Some(path));
            }
        }

        // Priority 4: /usr/share/smearor/services/services.toml (system default)
        let system_path = Path::new("/usr/share/smearor/services/services.toml");
        if system_path.is_file() {
            debug!("Discovered services config in system directory: {}", system_path.display());
            return Ok(Some(system_path.to_path_buf()));
        }

        debug!("No services config found, starting with default config");
        Ok(None)
    }

    /// Discovers the wallpaper config file (`wallpaper.toml`) using fallback locations.
    ///
    /// Checks the working directory first, then `~/.config/smearor/services/wallpaper.toml`,
    /// then `/usr/share/smearor/services/wallpaper.toml` (system default).
    /// Returns `None` if no file is found.
    pub fn discover_wallpaper_config(&self) -> Option<PathBuf> {
        // Priority 1: wallpaper.toml in working directory
        if let Ok(cwd) = std::env::current_dir() {
            let path = cwd.join("wallpaper.toml");
            if path.is_file() {
                debug!("Discovered wallpaper config in working directory: {}", path.display());
                return Some(path);
            }
        }

        // Priority 2: ~/.config/smearor/services/wallpaper.toml
        if let Some(xdg_dir) = Self::xdg_config_dir() {
            let path = xdg_dir.join("services").join("wallpaper.toml");
            if path.is_file() {
                debug!("Discovered wallpaper config in XDG config directory: {}", path.display());
                return Some(path);
            }
        }

        // Priority 3: /usr/share/smearor/services/wallpaper.toml (system default)
        let system_path = Path::new("/usr/share/smearor/services/wallpaper.toml");
        if system_path.is_file() {
            debug!("Discovered wallpaper config in system directory: {}", system_path.display());
            return Some(system_path.to_path_buf());
        }

        debug!("No wallpaper config found");
        None
    }

    /// Validates that all paths in the given list exist and are readable files.
    /// Returns an error with the first non-existent path.
    pub fn validate_config_paths(&self, paths: &[PathBuf]) -> Result<()> {
        for path in paths {
            if !path.is_file() {
                return Err(miette!("Configuration file not found: {}", path.display()));
            }
        }
        Ok(())
    }

    /// Bootstraps user config files from system-wide defaults on first run.
    ///
    /// Copies default configs from `/usr/share/smearor/` to `~/.config/smearor/`
    /// if they don't already exist. This ensures a fresh user account gets working
    /// configs after first launch without requiring manual setup.
    pub fn bootstrap_user_configs(&self) {
        let Some(xdg_dir) = Self::xdg_config_dir() else {
            debug!("Cannot determine XDG config directory, skipping bootstrap");
            return;
        };

        let system_dir = Path::new("/usr/share/smearor");

        let launcher_dir = xdg_dir.join("launcher");
        if Self::collect_toml_files(&launcher_dir, &[]).is_empty() {
            Self::bootstrap_file(system_dir.join("launcher/config.toml"), launcher_dir.join("config.toml"));
        }

        Self::bootstrap_file(system_dir.join("services/services.toml"), xdg_dir.join("services/services.toml"));

        Self::bootstrap_file(system_dir.join("services/wallpaper.toml"), xdg_dir.join("services/wallpaper.toml"));
    }

    /// Copies a file from `source` to `destination` if the destination does not exist.
    /// Creates parent directories as needed.
    fn bootstrap_file(source: PathBuf, destination: PathBuf) {
        if destination.is_file() {
            return;
        }

        if !source.is_file() {
            debug!("System default not found, skipping bootstrap: {}", source.display());
            return;
        }

        if let Some(parent) = destination.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                debug!("Failed to create directory {}: {}", parent.display(), e);
                return;
            }
        }

        match std::fs::copy(&source, &destination) {
            Ok(_) => {
                info!("Bootstrapped config: {} -> {}", source.display(), destination.display());
            }
            Err(e) => {
                debug!("Failed to bootstrap config {} -> {}: {}", source.display(), destination.display(), e);
            }
        }
    }

    /// Collects all `*.toml` files in a directory, sorted alphabetically by filename.
    /// Files whose names match any entry in `excluded` are skipped.
    fn collect_toml_files(dir: &Path, excluded: &[&str]) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };

        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let path = e.path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "toml") {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if !excluded.contains(&name) {
                            return Some(path);
                        }
                    }
                }
                None
            })
            .collect();

        files.sort();
        files
    }
}

impl Default for ConfigDiscoveryService {
    fn default() -> Self {
        Self::new()
    }
}

/// Bootstrap user configs from system defaults and discover launcher configuration files.
///
/// This is the top-level entry point for config discovery: it bootstraps user
/// configs on first run, then discovers and validates launcher config files
/// using the standard fallback order (CLI > working dir > XDG > system default).
pub fn bootstrap_configs(args: &crate::SwipeLauncherArguments) -> Result<Vec<PathBuf>> {
    let discovery_service = ConfigDiscoveryService::new();
    discovery_service.bootstrap_user_configs();

    let config_paths = discovery_service.discover_launcher_configs(&args.config)?;
    if config_paths.is_empty() {
        return Err(miette!(
            "No launcher configuration files found. \
            Specify via --config, or place *.toml files in the working directory or ~/.config/smearor/launcher/"
        ));
    }
    discovery_service.validate_config_paths(&config_paths)?;
    debug!("Starting smearor-swipe-launcher with config files: {:?}", config_paths);
    Ok(config_paths)
}
