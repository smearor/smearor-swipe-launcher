#!/bin/bash
set -euo pipefail

# Build script for all Debian packages
# Produces 33 .deb files in target/debian/:
#   1 main package + 13 widget plugins + 18 services + 1 metapackage

cd "$(dirname "$0")/.."

echo "=== Phase 1: Release build for entire workspace ==="
cargo build --release --workspace

echo "=== Phase 2: Build main package ==="
cargo deb -p smearor-swipe-launcher

echo "=== Phase 3: Build all widget plugin packages ==="
WIDGETS=(
    smearor-app-launcher-widget
    smearor-audio-widget
    smearor-button-widget
    smearor-clock-widget
    smearor-mpris-widget
    smearor-network-widget
    smearor-notifications-widget
    smearor-power-widget
    smearor-sysinfo-widget
    smearor-voice-assistant-widget
    smearor-wallpaper-widget
    smearor-weather-widget
    smearor-workspace-switcher
)
for pkg in "${WIDGETS[@]}"; do
    echo "  -> $pkg"
    cargo deb -p "$pkg"
done

echo "=== Phase 4: Build all service packages ==="
SERVICES=(
    smearor-app-launcher-service
    smearor-audio-service
    smearor-gnome-service
    smearor-http-service
    smearor-hyprland-service
    smearor-loupedeck-service
    smearor-mpris-service
    smearor-network-service
    smearor-notifications-service
    smearor-personalization-service
    smearor-power-service
    smearor-streamdeck-service
    smearor-sysinfo-service
    smearor-terminal-command-service
    smearor-voice-assistant-service
    smearor-wallpaper-service
    smearor-wayland-service
    smearor-weather-service
)
for pkg in "${SERVICES[@]}"; do
    echo "  -> $pkg"
    cargo deb -p "$pkg"
done

echo "=== Phase 5: Build metapackage ==="
cargo deb -p smearor-swipe-launcher-full

echo "=== Done! ==="
echo "All .deb files in target/debian/"
ls -1 target/debian/*.deb 2>/dev/null | wc -l
echo "packages built"
