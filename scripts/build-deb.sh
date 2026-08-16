#!/bin/bash
set -euo pipefail

# Build script for all Debian packages
# Produces 37 .deb files in target/debian/:
#   1 main package + 16 widget plugins + 19 services + 1 metapackage
#
# Usage:
#   ./scripts/build-deb.sh              # CPU-only (default)
#   ./scripts/build-deb.sh vulkan       # Vulkan GPU support (AMD APU, e.g. Ryzen 5 8500G)
#   ./scripts/build-deb.sh hipblas      # HIPBLAS GPU support (AMD discrete, e.g. Ryzen 9 9950X3D)
#   ./scripts/build-deb.sh default       # CPU-only (explicit)

cd "$(dirname "$0")/.."

BUILD_VARIANT="${1:-default}"

case "$BUILD_VARIANT" in
    default)
        VOICE_ASSISTANT_FEATURES=""
        VARIANT_LABEL="CPU-only"
        ;;
    vulkan)
        VOICE_ASSISTANT_FEATURES="ryzen-5-8500g"
        VARIANT_LABEL="Vulkan GPU"
        ;;
    hipblas)
        VOICE_ASSISTANT_FEATURES="ryzen-9-9950x3d-hipblas"
        VARIANT_LABEL="HIPBLAS GPU"
        ;;
    *)
        echo "Usage: $0 [default|vulkan|hipblas]"
        echo "  default  - CPU-only build (default)"
        echo "  vulkan  - Vulkan GPU support (AMD APU, e.g. Ryzen 5 8500G)"
        echo "  hipblas - HIPBLAS GPU support (AMD discrete, e.g. Ryzen 9 9950X3D)"
        exit 1
        ;;
esac

echo "=== Build variant: $VARIANT_LABEL ==="
echo "=== Voice Assistant features: ${VOICE_ASSISTANT_FEATURES:-none} ==="
echo ""

rm -f target/debian/smearor-service-voice-assistant*.deb

echo "=== Phase 1: Release build for entire workspace ==="
if [ -n "$VOICE_ASSISTANT_FEATURES" ]; then
    cargo build --release --workspace --features "smearor-voice-assistant-service/$VOICE_ASSISTANT_FEATURES"
else
    cargo build --release --workspace
fi

echo "=== Phase 2: Build main package ==="
cargo deb -p smearor-swipe-launcher

echo "=== Phase 3: Build all widget plugin packages ==="
WIDGETS=(
    smearor-app-launcher-widget
    smearor-audio-widget
    smearor-button-widget
    smearor-clock-widget
    smearor-doa-widget
    smearor-mpris-widget
    smearor-network-widget
    smearor-notifications-widget
    smearor-power-widget
    smearor-sysinfo-widget
    smearor-theme-widget
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
    smearor-doa-service
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
    smearor-theme-service
    smearor-terminal-command-service
    smearor-voice-assistant-service
    smearor-wallpaper-service
    smearor-wayland-service
    smearor-weather-service
)
for pkg in "${SERVICES[@]}"; do
    echo "  -> $pkg"
    if [ "$pkg" = "smearor-voice-assistant-service" ]; then
        if [ -n "$VOICE_ASSISTANT_FEATURES" ]; then
            cargo deb -p "$pkg" --variant "$BUILD_VARIANT" -- --features "$VOICE_ASSISTANT_FEATURES"
        else
            cargo deb -p "$pkg"
        fi
    else
        cargo deb -p "$pkg"
    fi
done

echo "=== Phase 5: Build metapackage ==="
cargo deb -p smearor-swipe-launcher-full

echo "=== Done! ($VARIANT_LABEL) ==="
echo "All .deb files in target/debian/"
ls -1 target/debian/*.deb 2>/dev/null | wc -l
echo "packages built"
