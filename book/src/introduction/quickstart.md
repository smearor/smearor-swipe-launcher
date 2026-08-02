# Quick Start

## Prerequisites

- Rust (Edition 2024, `rustc >= 1.95`)
- GTK 4 and development headers
- `gtk4-layer-shell`
- Wayland compositor (e.g. Hyprland, GNOME on Wayland)
- `libpulse` (for the audio plugin)
- `zbus` (for MPRIS, notifications, GNOME)

## Installation

### From Debian packages

```bash
# Install everything (main launcher + all standard widgets and services)
sudo apt install smearor-swipe-launcher-full

# Or install individual components
sudo apt install smearor-swipe-launcher
sudo apt install smearor-plugin-clock smearor-service-personalization
```

Enable the systemd user service for your compositor:

```bash
systemctl --user enable --now smearor-swipe-launcher-hyprland
# or
systemctl --user enable --now smearor-swipe-launcher-gnome
```

On first launch, the launcher copies default configs from `/usr/share/smearor/` to `~/.config/smearor/` automatically.

See [Debian Packaging](../packaging/debian-packaging.md) for details.

### From source

## Build

```bash
cargo build --release
```

The plugin libraries (`.so` files) are placed in `target/release/`.

## Configuration

The main configuration is located at `configs/launcher/config.toml`. It defines:

- The areas and their order
- Which plugins are loaded in which area
- Plugin-specific configurations
- Layout profiles (e.g. per workspace)

See [Launcher Configuration](../configuration/launcher-config.md) for details.

Services are configured in `configs/services/services.toml`. See [Services Configuration](../configuration/services-config.md).

## Launch

```bash
cargo run --release
```

or directly:

```bash
./target/release/smearor-swipe-launcher
```

## First Steps

1. Open `configs/launcher/config.toml` and adjust the areas
2. Add plugins (see the [plugin list](../plugins/app-launcher.md))
3. Configure services in `configs/services/services.toml`
4. Restart the launcher

## Building This Book

To build this documentation as HTML:

```bash
cargo install mdbook mdbook-mermaid
mdbook-mermaid install .
mdbook serve --open
```
