# Concept: Config Files Loading

This document describes the loading order and discovery mechanism for the three configuration file categories used by the *Smearor Swipe Launcher*: launcher
configs, `services.toml`, and `wallpaper.toml`. It also defines the reorganization of example/template config files into dedicated `configs/` subdirectories.

---

## 1. Current State

### 1.1 CLI Arguments

Defined in `smearor-swipe-launcher/src/args/launcher.rs`:

| Argument | Short | Long                | Default         | Type              | Description                                     |
|----------|-------|---------------------|-----------------|-------------------|-------------------------------------------------|
| Config   | `-c`  | `--config`          | `config.toml`   | `Vec<PathBuf>`    | Launcher instance config files (one per window) |
| Services | `-s`  | `--services-config` | `services.toml` | `Option<PathBuf>` | Shared background services config               |
| Instance | `-i`  | `--instance-id`     | *(none)*        | `Vec<String>`     | Optional instance IDs for each `--config`       |

There is **no CLI argument** for `wallpaper.toml`. The wallpaper service receives its `config_path` from the `[wallpaper]` section inside `services.toml`, then
loads themes from that path at service construction time (`services/wallpaper/src/service.rs`).

### 1.2 Config Files in the Repository Root

| File             | Purpose                                     |
|------------------|---------------------------------------------|
| `config.toml`    | Default launcher instance config            |
| `services.toml`  | Shared background services config           |
| `wallpaper.toml` | Wallpaper themes config (loaded by service) |

### 1.3 Example Configs

| Directory   | Files                                                                                                                                                                                            |
|-------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `config/`   | `config-streamcontrollerx.toml`, `config-streamdeck.toml`, `config-web.toml`                                                                                                                     |
| `examples/` | `config-90.toml`, `config-example-bottom.toml`, `config-example-left.toml`, `config-example-right.toml`, `config-example-top.toml`, `config.example.toml`, `config_layout_profiles_example.toml` |

### 1.2 Loading Flow in `main.rs`

1. `SwipeLauncherArguments::parse()` — clap parses CLI args.
2. `args.load_services_config()` — reads `services.toml` from the path given by `--services-config` (default: `services.toml` in the working directory).
3. `host.load_services(&services_config)` — services are loaded and started.
4. For each path in `args.config`: `args.load_config_from_file(config_path)` — reads and parses the launcher config, then `host.create_instance(...)` creates a
   launcher instance.

There is **no fallback discovery** — if the default file does not exist, the launcher errors out. There is **no XDG config directory** support.

---

## 2. Target State: Loading Order

### 2.1 Launcher Configs (Requirement 1)

When no launcher config files are specified via CLI (`--config` is not passed), the launcher discovers configs automatically from the following locations, in
priority order:

| Priority | Source                                                                             | Behavior                                                                                     |
|----------|------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------|
| 1        | CLI parameter `--config`                                                           | If one or more paths are given, **only** these are loaded. No auto-discovery occurs.         |
| 2        | `*.toml` in the working directory (excluding `services.toml` and `wallpaper.toml`) | All matching files are loaded, sorted alphabetically by filename. Each becomes one instance. |
| 3        | `~/.config/smearor/launcher/*.toml`                                                | All matching files are loaded, sorted alphabetically. Each becomes one instance.             |

**Rules:**

- If CLI `--config` is provided, auto-discovery is **completely skipped**. Only the explicitly listed files are loaded.
- If no `*.toml` files are found in the working directory (after excluding `services.toml` and `wallpaper.toml`), the launcher proceeds to the XDG config
  directory.
- If no config files are found at any location, the launcher exits with an error message indicating no configuration was found.
- The exclusion of `services.toml` and `wallpaper.toml` from working-directory globbing prevents the services config and wallpaper themes config from being
  mistakenly loaded as launcher instance configs.
- Instance IDs default to the config file stem (current behavior, unchanged).

### 2.2 `services.toml` (Requirement 2)

| Priority | Source                                     | Behavior                                                              |
|----------|--------------------------------------------|-----------------------------------------------------------------------|
| 1        | CLI parameter `--services-config`          | If a path is given, **only** this file is loaded. No fallback occurs. |
| 2        | `services.toml` in the working directory   | Loaded if present.                                                    |
| 3        | `~/.config/smearor/services/services.toml` | Loaded if the working directory file is absent.                       |

**Rules:**

- If `--services-config` is provided, only that file is used. No fallback.
- If no `services.toml` is found at any location, the launcher starts with an empty/default `ServicesConfig` (no services, default MCP and web config). This is
  a graceful degradation — the launcher can run without background services.

### 2.3 `wallpaper.toml` (Requirement 3)

The wallpaper config is **not** loaded directly by the launcher host. It is loaded by the wallpaper service at construction time, using the `config_path` field
from the `[wallpaper]` section in `services.toml`. The discovery logic for `wallpaper.toml` applies when `config_path` is not explicitly set in `services.toml`.

| Priority | Source                                      | Behavior                                        |
|----------|---------------------------------------------|-------------------------------------------------|
| 1        | `wallpaper.toml` in the working directory   | Loaded if present.                              |
| 2        | `~/.config/smearor/services/wallpaper.toml` | Loaded if the working directory file is absent. |

**Rules:**

- If `config_path` is explicitly set in the `[wallpaper]` section of `services.toml`, that path is used directly. No fallback discovery occurs.
- If `config_path` is **not** set (or is empty), the wallpaper service applies the fallback discovery above.
- If no `wallpaper.toml` is found at any location, the wallpaper service starts with an empty themes list (current behavior, unchanged).

---

## 3. Implementation Plan

### 3.1 Phase 1: Config Discovery Module

Create a new module `smearor-swipe-launcher/src/config/discovery.rs` that implements the fallback logic for all three config categories.

```rust
/// Discovers launcher config files based on CLI args and fallback locations.
///
/// If `cli_configs` is non-empty, returns only those paths (no discovery).
/// Otherwise, scans the working directory and `~/.config/smearor/launcher/` for `*.toml` files,
/// excluding `services.toml` and `wallpaper.toml`.
pub fn discover_launcher_configs(cli_configs: &[PathBuf]) -> Result<Vec<PathBuf>>;

/// Discovers the services config file based on CLI arg and fallback locations.
///
/// If `cli_services_config` is provided, returns only that path.
/// Otherwise, checks the working directory for `services.toml`, then `~/.config/smearor/services/services.toml`.
pub fn discover_services_config(cli_services_config: Option<&PathBuf>) -> Result<Option<PathBuf>>;

/// Discovers the wallpaper config file based on fallback locations.
///
/// Checks the working directory for `wallpaper.toml`, then `~/.config/smearor/services/wallpaper.toml`.
/// Returns `None` if no file is found (graceful degradation).
pub fn discover_wallpaper_config() -> Option<PathBuf>;
```

**Key implementation details:**

- Use `std::env::current_dir()` for the working directory.
- Use `dirs::config_dir()` (or `std::env::var("XDG_CONFIG_HOME")` with fallback to `~/.config`) for the XDG config directory.
- Working-directory globbing uses `std::fs::read_dir()` with filename suffix filtering (`.toml`).
- Results are sorted alphabetically for deterministic loading order.
- The `services.toml` and `wallpaper.toml` exclusion in launcher discovery uses filename comparison (not path comparison), so only files named exactly
  `services.toml` or `wallpaper.toml` in the working directory root are excluded.

### 3.2 Phase 2: CLI Argument Changes

Modify `smearor-swipe-launcher/src/args/launcher.rs`:

- **`--config`**: Change `default_value` from `Some("config.toml")` to `None`. When the user does not pass `--config`, the vector is empty, triggering
  auto-discovery. When the user passes `--config` one or more times, only those files are loaded.
- **`--services-config`**: Change `default_value` from `Some("services.toml")` to `None`. When the user does not pass `--services-config`, fallback discovery is
  used. When the user passes it, only that file is loaded.
- **No new CLI argument** for `wallpaper.toml` — the wallpaper config path continues to be configured via `services.toml` `[wallpaper] config_path`, with
  fallback discovery when unset.

```rust
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct SwipeLauncherArguments {
    /// Configuration file for shared background services.
    /// If omitted, discovered from working directory or ~/.config/smearor/services/services.toml.
    #[arg(short = 's', long)]
    pub(crate) services_config: Option<PathBuf>,

    /// Configuration files for each launcher instance window.
    /// Specify multiple times for multiple windows.
    /// If omitted, discovered from working directory or ~/.config/smearor/launcher/*.toml.
    #[arg(short, long)]
    pub(crate) config: Vec<PathBuf>,

    /// Optional instance IDs corresponding to each --config.
    /// If omitted, the config file stem is used as the instance ID.
    #[arg(short = 'i', long)]
    pub(crate) instance_id: Vec<String>,

    #[command(flatten)]
    pub(crate) rotation: RotationArguments,

    #[command(flatten)]
    pub(crate) layer: LayerArguments,
}
```

### 3.3 Phase 3: Main Loading Flow Changes

Modify `smearor-swipe-launcher/src/main.rs` to use the discovery module:

```rust
// Discover launcher configs
let config_paths = discover_launcher_configs( & args.config) ?;
if config_paths.is_empty() {
return Err(miette ! ("No launcher configuration files found. \
        Specify via --config, or place *.toml files in the working directory or ~/.config/smearor/launcher/"));
}

// Discover services config
let services_config_path = discover_services_config( & args.services_config) ?;
let services_config = match services_config_path {
Some(path) => {
let config_content = std::fs::read_to_string( &path).into_diagnostic() ?;
toml::from_str::< ServicesConfig > ( & config_content).into_diagnostic() ?
}
None => {
debug ! ("No services config found, starting with default config");
ServicesConfig::default()
}
};
```

### 3.4 Phase 4: Wallpaper Service Fallback

Modify `services/wallpaper/src/service.rs` and/or `services/wallpaper/src/config.rs`:

- When `service_config.config_path` is empty or the file does not exist, call `discover_wallpaper_config()` to find `wallpaper.toml` in the fallback locations.
- If no file is found, proceed with an empty themes list (current behavior).

### 3.5 Phase 5: File Reorganization

Move example/template config files into a structured `configs/` directory:

#### 5.1 Launcher Configs (Requirement 4)

Move all launcher example configs from the repository root and `config/` and `examples/` into `configs/launcher/`:

| Source                                         | Destination                               |
|------------------------------------------------|-------------------------------------------|
| `config.toml`                                  | `configs/launcher/config.toml`            |
| `config/config-streamcontrollerx.toml`         | `configs/launcher/streamcontrollerx.toml` |
| `config/config-streamdeck.toml`                | `configs/launcher/streamdeck.toml`        |
| `config/config-web.toml`                       | `configs/launcher/web.toml`               |
| `examples/config-90.toml`                      | `configs/launcher/rotated-90.toml`        |
| `examples/config-example-bottom.toml`          | `configs/launcher/example-bottom.toml`    |
| `examples/config-example-left.toml`            | `configs/launcher/example-left.toml`      |
| `examples/config-example-right.toml`           | `configs/launcher/example-right.toml`     |
| `examples/config-example-top.toml`             | `configs/launcher/example-top.toml`       |
| `examples/config.example.toml`                 | `configs/launcher/minimal.toml`           |
| `examples/config_layout_profiles_example.toml` | `configs/launcher/layout_profiles.toml`   |

After the move, the `examples/` directory is removed if it contained only config files.

#### 5.2 `services.toml` (Requirement 5)

| Source          | Destination                      |
|-----------------|----------------------------------|
| `services.toml` | `configs/services/services.toml` |

#### 5.3 `wallpaper.toml` (Requirement 6)

| Source           | Destination                       |
|------------------|-----------------------------------|
| `wallpaper.toml` | `configs/services/wallpaper.toml` |

#### 5.4 Resulting Directory Structure

```
configs/
├── launcher/
│   ├── config.toml
│   ├── streamcontrollerx.toml
│   ├── streamdeck.toml
│   ├── web.toml
│   ├── rotated-90.toml
│   ├── example-bottom.toml
│   ├── example-left.toml
│   ├── example-right.toml
│   ├── example-top.toml
│   ├── minimal.toml
│   └── layout_profiles.toml
└── services/
    ├── services.toml
    └── wallpaper.toml
```

### 3.6 Phase 6: Documentation and References

Update the following references after the file moves:

- `README.md` — update any references to config file locations.
- `CHANGELOG.md` — document the new loading order and file reorganization.
- `services.toml` `[wallpaper]` section — update the default `config_path` comment to reflect the new fallback behavior.
- Any documentation in `docs/` that references config file paths.

---

## 4. Edge Cases and Error Handling

| Scenario                                                             | Behavior                                                                            |
|----------------------------------------------------------------------|-------------------------------------------------------------------------------------|
| No `--config` and no `*.toml` in any location                        | Exit with error: no launcher config found.                                          |
| `--config` points to non-existent file                               | Exit with error: file not found (current behavior, unchanged).                      |
| No `--services-config` and no `services.toml`                        | Start with default `ServicesConfig` (no services, default MCP/web).                 |
| `--services-config` points to non-existent file                      | Exit with error: file not found.                                                    |
| No `wallpaper.toml` and no `config_path` set                         | Wallpaper service starts with empty themes list.                                    |
| `wallpaper.toml` exists in both locations                            | Working directory takes priority; XDG path is not loaded.                           |
| Multiple `*.toml` in working directory                               | All are loaded (sorted alphabetically), each becomes one instance.                  |
| `services.toml` in working directory is also a valid launcher config | Excluded from launcher discovery by filename. It is only loaded as services config. |

---

## 5. Dependencies

- `dirs` crate (or equivalent) for XDG config directory resolution. Add to `smearor-swipe-launcher/Cargo.toml` if not already present.
- No other new dependencies required.

---

## 6. Testing

### 6.1 Unit Tests (`config/discovery.rs`)

- Test `discover_launcher_configs` with empty CLI list and mock working directory containing `*.toml` files (including `services.toml` and `wallpaper.toml`
  which should be excluded).
- Test `discover_launcher_configs` with non-empty CLI list — should return only CLI paths.
- Test `discover_services_config` with no CLI arg and `services.toml` in working directory.
- Test `discover_services_config` with no CLI arg and no working directory file — should fall back to XDG path.
- Test `discover_wallpaper_config` with file in working directory.
- Test `discover_wallpaper_config` with no file in working directory — should fall back to XDG path.

### 6.2 Integration Tests

- Start the launcher with no `--config` and verify it discovers configs from the working directory.
- Start the launcher with no `--services-config` and verify it discovers `services.toml` or starts with defaults.
- Verify that `services.toml` and `wallpaper.toml` are never loaded as launcher instance configs.

---

## 7. References

- **CLI arguments**: `smearor-swipe-launcher/src/args/launcher.rs` — current argument definitions.
- **Main loading flow**: `smearor-swipe-launcher/src/main.rs` — current config loading in `main()`.
- **Services config**: `smearor-swipe-launcher/src/config/services.rs` — `ServicesConfig` struct.
- **Wallpaper service**: `services/wallpaper/src/service.rs` — wallpaper config loading via `config_path`.
- **Wallpaper config**: `services/wallpaper/src/config.rs` — `load_themes()` function.
