# Concept: Network Widget View-Based Redesign (Milestone 2)

This document describes the **second milestone** of the Network Menu Widget. It refactors the widget from a single large panel into a compact, view-based tile
that mirrors the architecture and styling of the **Weather Widget**. The detailed network interactions (WLAN scan list, QR code, VPN toggles, airplane mode) are
moved into the separate `network_area` scroll menu, which already contains app launchers like `etherape`, `gnome_network_displays`, `digger`, and
`ping_monitor`.

---

## 1. Motivation

The first milestone (`NETWORK_MENU_CONCEPT.md`) implemented the network widget as a full-size panel displaying all information at once: status overview, WLAN
scan list, VPN toggles, airplane mode, throughput sparkline, and QR code button. On a 32-inch touch display with multiple widgets in a scroll band, this panel
is too large and visually overwhelming.

The Weather Widget solved a similar problem by splitting information into **views** that the user cycles through with swipe gestures. The network widget should
follow the same pattern:

- **Compact tile** with three labels (icon, value, info) — identical layout to the Weather Widget.
- **Swipe up/down** cycles through network views (status, throughput, WiFi scan summary, VPN summary, airplane mode).
- **Click** opens the `network_area` scroll menu with detailed controls.
- **Long-press** opens the `network_area` (configurable, can trigger a refresh command instead).

---

## 2. Changes from Milestone 1

### 2.1 What Changed

| Aspect                         | Milestone 1                                                                                                                    | Milestone 2                                                                                                                                      |
|--------------------------------|--------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------|
| **Widget layout**              | Full panel (280x400px) with multiple sections rendered simultaneously                                                          | Compact tile (~100x100px) with three labels, one view visible at a time                                                                          |
| **View switching**             | No views; all sections visible at once                                                                                         | Swipe up/down cycles through `NetworkView` variants (like `WeatherView`)                                                                         |
| **Config flags**               | `show_status`, `show_scan_list`, `show_airplane_toggle`, `show_vpn_toggles`, `show_throughput`, `show_qr_code` booleans        | `views: Vec<NetworkView>` list (replaces all `show_*` booleans); `spacing`, `button_size`, `icon_size` retained                                  |
| **Widget size**                | 280x400 default                                                                                                                | 100x100 default (matches Weather Widget tile size)                                                                                               |
| **Status views**               | Single `show_status` section showing the primary interface                                                                     | Split into `WifiStatus` and `EthernetStatus` views; each is independently clickable to toggle                                                    |
| **Scan list**                  | Rendered inside the widget as GTK boxes, buttons, and overlays                                                                 | Moved to `network_area` scroll menu as separate button widgets                                                                                   |
| **Airplane mode**              | Rendered as a toggle button inside the widget panel                                                                            | Retained as a clickable toggle inside the tile; clicking the tile in `Airplane` view sends a `ToggleRadio` command                               |
| **QR code**                    | Rendered as a button-triggered overlay inside the widget                                                                       | Retained as a dedicated `QrCode` view; the QR code is rendered directly in the tile via `DrawingArea`                                            |
| **VPN toggle**                 | Rendered as a toggle list inside the widget panel                                                                              | Retained as a `Vpn` view; clicking the tile toggles the first VPN profile. VPN configuration is done via a NetworkManager tool in `network_area` |
| **Throughput sparkline**       | `gtk4::DrawingArea` inside the widget                                                                                          | Removed from tile; throughput shown as text in the `Throughput` view                                                                             |
| **Password dialog**            | In-widget GTK dialog                                                                                                           | Handled by a dedicated button in `network_area` (future)                                                                                         |
| **Gesture handling**           | `GestureClick` only (click to open area)                                                                                       | `GestureClick` + `GestureDrag` (swipe to cycle views) + `GestureLongPress` (configurable action)                                                 |
| **Message interaction fields** | `click_topic`, `click_payload` only                                                                                            | `click_topic`, `click_payload`, `click_instance`, `longpress_topic`, `longpress_payload`, `longpress_instance` (parity with Button Widget)       |
| **CSS classes**                | `network-widget`, `network-status`, `network-scan-*`, `network-vpn-*`, `network-airplane`, `network-sparkline`, `network-qr-*` | `network-widget`, `network-icon`, `network-value`, `network-info` (mirrors Weather Widget CSS)                                                   |
| **Icon rendering**             | `Label` with hardcoded Unicode codepoints                                                                                      | `gtk4::Image` with `resolve_gtk_nerd_icon`; all icons configurable via Nerd Font names (like Wallpaper Widget)                                   |

### 2.2 What Stayed the Same

- **Model crate** (`model/network`): All existing message types, topics, enums, and icon functions remain unchanged.
- **Service crate** (`services/network`): The D-Bus service, status broadcasts, scan results, VPN profiles, throughput sampling, and MCP tools remain unchanged.
- **Message topics**: `TOPIC_STATUS`, `TOPIC_SCAN_RESULTS`, `TOPIC_VPN_PROFILES`, `TOPIC_COMMAND` remain unchanged.
- **FFI types**: All `#[stabby::stabby]` types in the model crate remain unchanged.

### 2.3 What Was Added

- **`NetworkView` enum** in `model/network` — defines which data category the tile displays.
- **`views` config field** in `NetworkWidgetConfig` — list of views to cycle through.
- **`click_instance` / `longpress_instance`** config fields — target instance support for message routing.
- **Swipe gesture handling** in the widget — `GestureDrag` for view switching.
- **`network_area` scroll menu** in `config.toml` — contains the close button, app launchers, and (future) detailed network control buttons.

---

## 3. New Model Addition: `NetworkView`

A new enum is added to `model/network` to define the available views, mirroring `WeatherView` in `model/weather`.

### 3.1 File: `model/network/src/messages/view.rs`

```rust
use serde::Deserialize;
use serde::Serialize;

/// Available network views that the widget can display.
/// Each variant corresponds to a data category rendered in the widget tile.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum NetworkView {
    /// WiFi status: SSID, signal strength, and IP of the most recently connected WiFi device.
    /// Clicking the tile in this view toggles WiFi radio on/off.
    #[default]
    WifiStatus,
    /// Ethernet status: interface name and IP of the primary Ethernet device.
    /// Clicking the tile in this view toggles the Ethernet connection on/off.
    EthernetStatus,
    /// Aggregate throughput: download and upload rates.
    Throughput,
    /// WLAN scan summary: count of available networks and strongest signal.
    WifiScan,
    /// VPN summary: shows the first VPN profile's name and state. Clicking the tile in this view toggles the first VPN profile.
    Vpn,
    /// Airplane mode status: on or off. Clicking the tile in this view toggles airplane mode.
    Airplane,
    /// WiFi QR code: renders a scannable QR code for the current WiFi connection directly in the tile.
    QrCode,
}
```

### 3.2 Re-export in `model/network/src/lib.rs`

```rust
pub use messages::view::NetworkView;
```

---

## 4. Widget Config Changes

### 4.1 Updated `NetworkWidgetConfig`

The `show_*` boolean flags are replaced by a `views` list. The `click_instance` and `longpress_*` fields are added for parity with the Button Widget.

```rust
pub const DEFAULT_WIDTH: i32 = 100;
pub const DEFAULT_HEIGHT: i32 = 100;
pub const DEFAULT_SPACING: i32 = 0;
pub const DEFAULT_BUTTON_SIZE: i32 = 48;
pub const DEFAULT_ICON_SIZE: i32 = 36;

// Default Nerd Font icon names (configurable via config.toml)
pub const DEFAULT_ICON_WIFI_STRENGTH_4: &str = "nf-md-wifi_strength_4";
pub const DEFAULT_ICON_WIFI_STRENGTH_3: &str = "nf-md-wifi_strength_3";
pub const DEFAULT_ICON_WIFI_STRENGTH_2: &str = "nf-md-wifi_strength_2";
pub const DEFAULT_ICON_WIFI_STRENGTH_1: &str = "nf-md-wifi_strength_1";
pub const DEFAULT_ICON_WIFI_STRENGTH_OFF: &str = "nf-md-wifi_strength_off";
pub const DEFAULT_ICON_ETHERNET_ON: &str = "nf-md-network_outline";
pub const DEFAULT_ICON_ETHERNET_OFF: &str = "nf-md-network_off";
pub const DEFAULT_ICON_VPN_ON: &str = "nf-md-shield_key";
pub const DEFAULT_ICON_VPN_OFF: &str = "nf-md-shield_off";
pub const DEFAULT_ICON_AIRPLANE_ON: &str = "nf-md-airplane";
pub const DEFAULT_ICON_AIRPLANE_OFF: &str = "nf-md-airplane_off";
pub const DEFAULT_ICON_THROUGHPUT: &str = "nf-md-swap_vertical";
pub const DEFAULT_ICON_WIFI_SCAN: &str = "nf-md-wifi_strength_4";
pub const DEFAULT_ICON_QR_CODE: &str = "nf-md-qrcode";

/// Configuration for the network menu widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct NetworkWidgetConfig {
    /// Width of the widget tile in pixels.
    #[builder(default = DEFAULT_WIDTH)]
    pub(crate) width: i32,

    /// Height of the widget tile in pixels.
    #[builder(default = DEFAULT_HEIGHT)]
    pub(crate) height: i32,

    /// Spacing between elements (icon, value, info labels) in pixels.
    #[builder(default = DEFAULT_SPACING)]
    pub(crate) spacing: i32,

    /// Button size in pixels (used for touch target sizing).
    #[builder(default = DEFAULT_BUTTON_SIZE)]
    pub(crate) button_size: i32,

    /// Icon size in pixels (pixel size for Nerd Font icon images).
    #[builder(default = DEFAULT_ICON_SIZE)]
    pub(crate) icon_size: i32,

    /// WiFi icon: signal strength 4 (>75%).
    #[builder(default = DEFAULT_ICON_WIFI_STRENGTH_4.to_string())]
    #[serde(default = "default_icon_wifi_strength_4")]
    pub(crate) icon_wifi_strength_4: String,

    /// WiFi icon: signal strength 3 (>50%).
    #[builder(default = DEFAULT_ICON_WIFI_STRENGTH_3.to_string())]
    #[serde(default = "default_icon_wifi_strength_3")]
    pub(crate) icon_wifi_strength_3: String,

    /// WiFi icon: signal strength 2 (>25%).
    #[builder(default = DEFAULT_ICON_WIFI_STRENGTH_2.to_string())]
    #[serde(default = "default_icon_wifi_strength_2")]
    pub(crate) icon_wifi_strength_2: String,

    /// WiFi icon: signal strength 1 (>0%).
    #[builder(default = DEFAULT_ICON_WIFI_STRENGTH_1.to_string())]
    #[serde(default = "default_icon_wifi_strength_1")]
    pub(crate) icon_wifi_strength_1: String,

    /// WiFi icon: WiFi off / no signal.
    #[builder(default = DEFAULT_ICON_WIFI_STRENGTH_OFF.to_string())]
    #[serde(default = "default_icon_wifi_strength_off")]
    pub(crate) icon_wifi_strength_off: String,

    /// Ethernet icon: connected.
    #[builder(default = DEFAULT_ICON_ETHERNET_ON.to_string())]
    #[serde(default = "default_icon_ethernet_on")]
    pub(crate) icon_ethernet_on: String,

    /// Ethernet icon: disconnected.
    #[builder(default = DEFAULT_ICON_ETHERNET_OFF.to_string())]
    #[serde(default = "default_icon_ethernet_off")]
    pub(crate) icon_ethernet_off: String,

    /// VPN icon: active.
    #[builder(default = DEFAULT_ICON_VPN_ON.to_string())]
    #[serde(default = "default_icon_vpn_on")]
    pub(crate) icon_vpn_on: String,

    /// VPN icon: inactive.
    #[builder(default = DEFAULT_ICON_VPN_OFF.to_string())]
    #[serde(default = "default_icon_vpn_off")]
    pub(crate) icon_vpn_off: String,

    /// Airplane icon: airplane mode on.
    #[builder(default = DEFAULT_ICON_AIRPLANE_ON.to_string())]
    #[serde(default = "default_icon_airplane_on")]
    pub(crate) icon_airplane_on: String,

    /// Airplane icon: airplane mode off.
    #[builder(default = DEFAULT_ICON_AIRPLANE_OFF.to_string())]
    #[serde(default = "default_icon_airplane_off")]
    pub(crate) icon_airplane_off: String,

    /// Throughput view icon.
    #[builder(default = DEFAULT_ICON_THROUGHPUT.to_string())]
    #[serde(default = "default_icon_throughput")]
    pub(crate) icon_throughput: String,

    /// WiFi scan view icon.
    #[builder(default = DEFAULT_ICON_WIFI_SCAN.to_string())]
    #[serde(default = "default_icon_wifi_scan")]
    pub(crate) icon_wifi_scan: String,

    /// QR code view icon.
    #[builder(default = DEFAULT_ICON_QR_CODE.to_string())]
    #[serde(default = "default_icon_qr_code")]
    pub(crate) icon_qr_code: String,

    /// Views to cycle through on swipe up/down.
    #[builder(default)]
    pub(crate) views: Vec<NetworkView>,

    /// Maximum number of access points to summarize in the WifiScan view.
    #[builder(default = 10)]
    pub(crate) max_access_points: usize,

    /// Message topic for single-click (opens the network menu area).
    #[serde(default)]
    pub click_topic: Option<String>,

    /// Message payload for single-click.
    #[serde(default)]
    pub click_payload: Option<Value>,

    /// Target instance for single-click message.
    #[serde(default)]
    pub click_instance: Option<String>,

    /// Message topic for long-press.
    #[serde(default)]
    pub longpress_topic: Option<String>,

    /// Message payload for long-press (JSON/TOML).
    #[serde(default)]
    pub longpress_payload: Option<Value>,

    /// Target instance for long-press message.
    #[serde(default)]
    pub longpress_instance: Option<String>,
}

impl Default for NetworkWidgetConfig {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            spacing: DEFAULT_SPACING,
            button_size: DEFAULT_BUTTON_SIZE,
            icon_size: DEFAULT_ICON_SIZE,
            icon_wifi_strength_4: DEFAULT_ICON_WIFI_STRENGTH_4.to_string(),
            icon_wifi_strength_3: DEFAULT_ICON_WIFI_STRENGTH_3.to_string(),
            icon_wifi_strength_2: DEFAULT_ICON_WIFI_STRENGTH_2.to_string(),
            icon_wifi_strength_1: DEFAULT_ICON_WIFI_STRENGTH_1.to_string(),
            icon_wifi_strength_off: DEFAULT_ICON_WIFI_STRENGTH_OFF.to_string(),
            icon_ethernet_on: DEFAULT_ICON_ETHERNET_ON.to_string(),
            icon_ethernet_off: DEFAULT_ICON_ETHERNET_OFF.to_string(),
            icon_vpn_on: DEFAULT_ICON_VPN_ON.to_string(),
            icon_vpn_off: DEFAULT_ICON_VPN_OFF.to_string(),
            icon_airplane_on: DEFAULT_ICON_AIRPLANE_ON.to_string(),
            icon_airplane_off: DEFAULT_ICON_AIRPLANE_OFF.to_string(),
            icon_throughput: DEFAULT_ICON_THROUGHPUT.to_string(),
            icon_wifi_scan: DEFAULT_ICON_WIFI_SCAN.to_string(),
            icon_qr_code: DEFAULT_ICON_QR_CODE.to_string(),
            views: vec![
                NetworkView::WifiStatus,
                NetworkView::EthernetStatus,
                NetworkView::Throughput,
                NetworkView::WifiScan,
                NetworkView::Vpn,
                NetworkView::Airplane,
                NetworkView::QrCode,
            ],
            max_access_points: 10,
            click_topic: None,
            click_payload: None,
            click_instance: None,
            longpress_topic: None,
            longpress_payload: None,
            longpress_instance: None,
        }
    }
}

fn default_icon_wifi_strength_4() -> String { DEFAULT_ICON_WIFI_STRENGTH_4.to_string() }
fn default_icon_wifi_strength_3() -> String { DEFAULT_ICON_WIFI_STRENGTH_3.to_string() }
fn default_icon_wifi_strength_2() -> String { DEFAULT_ICON_WIFI_STRENGTH_2.to_string() }
fn default_icon_wifi_strength_1() -> String { DEFAULT_ICON_WIFI_STRENGTH_1.to_string() }
fn default_icon_wifi_strength_off() -> String { DEFAULT_ICON_WIFI_STRENGTH_OFF.to_string() }
fn default_icon_ethernet_on() -> String { DEFAULT_ICON_ETHERNET_ON.to_string() }
fn default_icon_ethernet_off() -> String { DEFAULT_ICON_ETHERNET_OFF.to_string() }
fn default_icon_vpn_on() -> String { DEFAULT_ICON_VPN_ON.to_string() }
fn default_icon_vpn_off() -> String { DEFAULT_ICON_VPN_OFF.to_string() }
fn default_icon_airplane_on() -> String { DEFAULT_ICON_AIRPLANE_ON.to_string() }
fn default_icon_airplane_off() -> String { DEFAULT_ICON_AIRPLANE_OFF.to_string() }
fn default_icon_throughput() -> String { DEFAULT_ICON_THROUGHPUT.to_string() }
fn default_icon_wifi_scan() -> String { DEFAULT_ICON_WIFI_SCAN.to_string() }
fn default_icon_qr_code() -> String { DEFAULT_ICON_QR_CODE.to_string() }
```

### 4.2 Removed Config Fields

The following fields are removed from `NetworkWidgetConfig` because they are replaced by the `views` list:

- `show_status`
- `show_scan_list`
- `show_airplane_toggle`
- `show_vpn_toggles`
- `show_throughput`
- `show_qr_code`

### 4.3 Retained Config Fields

The following sizing fields are retained from Milestone 1 and control the visual layout of the tile:

- `spacing` — spacing between the icon, value, and info labels in the vertical `GtkBox` (default: 0px)
- `button_size` — touch target size in pixels, used for gesture hit area sizing (default: 48px)
- `icon_size` — pixel size for Nerd Font icon images rendered via `gtk4::Image` (default: 36px)
- `icon_wifi_strength_4` — WiFi icon for signal >75% (default: `nf-md-wifi_strength_4`)
- `icon_wifi_strength_3` — WiFi icon for signal >50% (default: `nf-md-wifi_strength_3`)
- `icon_wifi_strength_2` — WiFi icon for signal >25% (default: `nf-md-wifi_strength_2`)
- `icon_wifi_strength_1` — WiFi icon for signal >0% (default: `nf-md-wifi_strength_1`)
- `icon_wifi_strength_off` — WiFi icon for WiFi off (default: `nf-md-wifi_strength_off`)
- `icon_ethernet_on` — Ethernet connected icon (default: `nf-md-network_outline`)
- `icon_ethernet_off` — Ethernet disconnected icon (default: `nf-md-network_off`)
- `icon_vpn_on` — VPN active icon (default: `nf-md-shield_key`)
- `icon_vpn_off` — VPN inactive icon (default: `nf-md-shield_off`)
- `icon_airplane_on` — Airplane mode on icon (default: `nf-md-airplane`)
- `icon_airplane_off` — Airplane mode off icon (default: `nf-md-airplane_off`)
- `icon_throughput` — Throughput view icon (default: `nf-md-swap_vertical`)
- `icon_wifi_scan` — WiFi scan view icon (default: `nf-md-wifi_strength_4`)
- `icon_qr_code` — QR code view icon (default: `nf-md-qrcode`)

All icon fields accept Nerd Font icon names (e.g., `nf-md-wifi_strength_4`). Icons are resolved at runtime via `resolve_gtk_nerd_icon` and rendered as
`gtk4::Image` from GResource SVGs, identical to the Wallpaper and Button widgets.

---

## 5. Widget Redesign

### 5.1 Widget Struct

The widget struct mirrors the Weather Widget, but uses a `gtk4::Image` for the icon (instead of a `Label`) to support Nerd Font icon resolution via
`resolve_gtk_nerd_icon`:

```rust
pub struct NetworkWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: NetworkWidgetConfig,
    pub icon_image: Rc<RefCell<Option<gtk4::Image>>>,
    pub value_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub qr_drawing_area: Rc<RefCell<Option<DrawingArea>>>,
    pub current_view: Rc<RefCell<usize>>,
    pub latest_status: Rc<RefCell<Option<NetworkStatusMessage>>>,
    pub latest_scan: Rc<RefCell<Option<ScanResultsMessage>>>,
    pub latest_vpn: Rc<RefCell<Option<VpnProfilesMessage>>>,
}
```

### 5.2 View Rendering

A `render_view` function produces `(icon_name, value, info)` for each view, where `icon_name` is a Nerd Font icon name (e.g., `nf-md-wifi_strength_4`) resolved
from config. The icon is rendered as a `gtk4::Image` via `resolve_gtk_nerd_icon`, not as a text label:

```rust
fn render_view(
    status: &NetworkStatusMessage,
    scan: Option<&ScanResultsMessage>,
    vpn: Option<&VpnProfilesMessage>,
    config: &NetworkWidgetConfig,
    view: NetworkView,
) -> (String, String, String) {
    match view {
        NetworkView::WifiStatus => {
            let wifi_iface = status.interfaces.iter().find(|iface| iface.interface_type == NetworkInterfaceType::Wifi);
            match wifi_iface {
                Some(iface) => {
                    let signal = iface.signal.as_ref().map(|s| *s).unwrap_or(0);
                    let icon = if !status.wifi_enabled {
                        config.icon_wifi_strength_off.clone()
                    } else if signal > 75 {
                        config.icon_wifi_strength_4.clone()
                    } else if signal > 50 {
                        config.icon_wifi_strength_3.clone()
                    } else if signal > 25 {
                        config.icon_wifi_strength_2.clone()
                    } else if signal > 0 {
                        config.icon_wifi_strength_1.clone()
                    } else {
                        config.icon_wifi_strength_off.clone()
                    };
                    let ssid = iface.ssid.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "Unknown".to_string());
                    let value = format!("{ssid} {signal}%");
                    let info = iface.ipv4_address.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "No IP".to_string());
                    (icon, value, info)
                }
                None => (config.icon_wifi_strength_off.clone(), "Off".to_string(), "No WiFi".to_string()),
            }
        }
        NetworkView::EthernetStatus => {
            let eth_iface = status.interfaces.iter().find(|iface| iface.interface_type == NetworkInterfaceType::Ethernet);
            match eth_iface {
                Some(iface) => {
                    let icon = if iface.state == NetworkConnectionState::Connected { config.icon_ethernet_on.clone() } else { config.icon_ethernet_off.clone() };
                    let value = if iface.state == NetworkConnectionState::Connected { "Connected".to_string() } else { "Disconnected".to_string() };
                    let info = iface.ipv4_address.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "No IP".to_string());
                    (icon, value, info)
                }
                None => (config.icon_ethernet_off.clone(), "--".to_string(), "No Ethernet".to_string()),
            }
        }
        NetworkView::Throughput => {
            let rx = format_bytes(status.received_bytes_per_second);
            let tx = format_bytes(status.transmitted_bytes_per_second);
            (config.icon_throughput.clone(), format!("{rx}/s"), format!("{tx}/s"))
        }
        NetworkView::WifiScan => {
            match scan {
                Some(scan) => {
                    let count = scan.access_points.len();
                    let strongest = scan.access_points.first().map(|ap| ap.signal).unwrap_or(0);
                    (config.icon_wifi_scan.clone(), format!("{count} networks"), format!("Strongest: {strongest}%"))
                }
                None => (config.icon_wifi_scan.clone(), "--".to_string(), "No scan".to_string()),
            }
        }
        NetworkView::Vpn => {
            match vpn {
                Some(vpn) => {
                    let first = vpn.profiles.first();
                    match first {
                        Some(profile) => {
                            let name = profile.name.to_string();
                            let state = if profile.is_active { "ON" } else { "OFF" };
                            let icon = if profile.is_active { config.icon_vpn_on.clone() } else { config.icon_vpn_off.clone() };
                            (icon, name, format!("VPN {state}"))
                        }
                        None => (config.icon_vpn_off.clone(), "--".to_string(), "No VPN".to_string()),
                    }
                }
                None => (config.icon_vpn_off.clone(), "--".to_string(), "No VPN".to_string()),
            }
        }
        NetworkView::Airplane => {
            if status.airplane_mode {
                (config.icon_airplane_on.clone(), "ON".to_string(), "Airplane".to_string())
            } else {
                (config.icon_airplane_off.clone(), "OFF".to_string(), "Airplane".to_string())
            }
        }
        NetworkView::QrCode => {
            // The QR code view is special: instead of updating the icon/value/info,
            // the widget shows the DrawingArea (which renders the QR code) and
            // hides the labels. See section 5.3a for layout switching details.
            (config.icon_qr_code.clone(), "QR".to_string(), "Scan to connect".to_string())
        }
    }
}
```

### 5.3 View Switching

View switching is handled by `GestureDrag`, identical to the Weather Widget pattern:

```rust
let drag_gesture = GestureDrag::new();
drag_gesture.set_propagation_phase(PropagationPhase::Capture);
drag_gesture.connect_drag_end( move | gesture, offset_x, offset_y| {
const SWIPE_THRESHOLD: f64 = 50.0;
if offset_y.abs() > offset_x.abs() & & offset_y.abs() > SWIPE_THRESHOLD {
gesture.set_state(EventSequenceState::Claimed);
if offset_y < 0.0 {
widget_self.next_view();
} else {
widget_self.prev_view();
}
}
});
outer_box.add_controller(drag_gesture);
```

### 5.3a Layout Switching for QrCode View

The `QrCode` view is special: instead of showing the icon image and text labels, the widget shows a `gtk4::DrawingArea` that renders the QR code directly. The
`update_ui` method handles this layout switch:

```rust
fn update_ui(&self, status: &NetworkStatusMessage) {
    let view_index = *self.current_view.borrow();
    let view = self.config.views.get(view_index).copied().unwrap_or(NetworkView::WifiStatus);

    let icon_image = self.icon_image.clone();
    let value_label = self.value_label.clone();
    let info_label = self.info_label.clone();
    let qr_area = self.qr_drawing_area.clone();
    let config = self.config.clone();
    let status = status.clone();

    MainContext::default().spawn_local(async move {
        if view == NetworkView::QrCode {
            // Show QR code DrawingArea, hide icon image and text labels
            if let Some(ref area) = *qr_area.borrow() {
                area.set_visible(true);
                area.queue_draw();
            }
            if let Some(ref img) = *icon_image.borrow() { img.set_visible(false); }
            if let Some(ref label) = *value_label.borrow() { label.set_visible(false); }
            if let Some(ref label) = *info_label.borrow() { label.set_visible(false); }
        } else {
            // Show icon image and text labels, hide QR code DrawingArea
            if let Some(ref area) = *qr_area.borrow() { area.set_visible(false); }
            if let Some(ref img) = *icon_image.borrow() { img.set_visible(true); }
            if let Some(ref label) = *value_label.borrow() { label.set_visible(true); }
            if let Some(ref label) = *info_label.borrow() { label.set_visible(true); }

            let (icon_name, value_text, info_text) = render_view(&status, None, None, &config, view);

            // Update icon image via resolve_gtk_nerd_icon (no set_text on Image)
            if let Some(ref img) = *icon_image.borrow() {
                if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(&icon_name) {
                    let resource_path = format!("/com/nerd/icons/{}.svg", gtk_icon_name);
                    if gio::resources_lookup_data(&resource_path, gio::ResourceLookupFlags::NONE).is_ok() {
                        img.set_resource(Some(&resource_path));
                    } else {
                        img.set_from_icon_name(Some(&gtk_icon_name));
                    }
                }
            }

            if let Some(ref label) = *value_label.borrow() { label.set_text(&value_text); }
            if let Some(ref label) = *info_label.borrow() { label.set_text(&info_text); }
        }
    });
}
```

The `DrawingArea` is created in `build_widget` with a draw function that generates the QR code from the current WiFi status:

```rust
let qr_area = DrawingArea::builder()
.css_classes(["network-qr".to_string()])
.width_request(80)
.height_request(80)
.halign(Align::Center)
.valign(Align::Center)
.visible(false)  // Hidden by default, shown only in QrCode view
.build();

let status_for_qr = self .latest_status.clone();
qr_area.set_draw_func( move | _, cr, width, height| {
let status = status_for_qr.borrow();
if let Some( ref status) = * status {
let ssid = status.primary_interface.ssid.as_ref()
.map( | s | s.to_string())
.unwrap_or_else( | | "Unknown".to_string());
let qr_string = generate_wifi_qr_string( & ssid, "", "WPA");
if let Ok(qr_code) = qrcode::QrCode::new(qr_string.as_bytes()) {
draw_qr_code(cr, width, height, & qr_code);
}
}
});
outer_box.append( & qr_area);
* self .qr_drawing_area.borrow_mut() = Some(qr_area);
```

### 5.4 Click and Long-Press Handling

The click behavior is **view-dependent**:

- `WifiStatus` — toggles WiFi radio on/off via `ToggleRadio("wifi", !current_state)`
- `EthernetStatus` — toggles the Ethernet connection on/off via `Disconnect`/reconnect
- `Airplane` — toggles airplane mode via `ToggleRadio("all", !current_state)`
- `Vpn` — toggles the first VPN profile via `ToggleVpn(first_profile, !state)`
- All other views (`Throughput`, `WifiScan`, `QrCode`) — broadcast the configured `click_topic` (typically opening `network_area`)

```rust
// Click: view-dependent action
let click_gesture = GestureClick::builder()
.button(0)
.propagation_phase(PropagationPhase::Capture)
.build();
click_gesture.connect_released( move | gesture, _n_press, _x, _y| {
let view_index = * current_view.borrow();
let view = views.get(view_index).copied().unwrap_or(NetworkView::WifiStatus);

if view == NetworkView::WifiStatus {
// Toggle WiFi radio: send ToggleRadio command to service
let wifi_on = latest_status.borrow().as_ref().map( | s | s.wifi_enabled).unwrap_or(false);
let command = NetworkCommandMessage::toggle_radio("wifi", ! wifi_on);
broadcaster.broadcast_message_to_topic(command);
} else if view == NetworkView::EthernetStatus {
// Toggle Ethernet: disconnect or reconnect the primary Ethernet device
let eth_iface = latest_status.borrow().as_ref()
.and_then( | s | s.interfaces.iter().find( | i| i.interface_type == NetworkInterfaceType::Ethernet));
if let Some(iface) = eth_iface {
if iface.state == NetworkConnectionState::Connected {
let command = NetworkCommandMessage::disconnect(iface.interface_name.to_string());
broadcaster.broadcast_message_to_topic(command);
} else {
// Reconnect by activating the Ethernet connection profile
let command = NetworkCommandMessage::connect(iface.interface_name.to_string());
broadcaster.broadcast_message_to_topic(command);
}
}
} else if view == NetworkView::Airplane {
// Toggle airplane mode: send ToggleRadio command to service
let is_airplane_on = latest_status.borrow().as_ref().map( | s | s.airplane_mode).unwrap_or(false);
let command = NetworkCommandMessage::toggle_radio("all", ! is_airplane_on);
broadcaster.broadcast_message_to_topic(command);
} else if view == NetworkView::Vpn {
// Toggle first VPN profile: send ToggleVpn command to service
let first_vpn = latest_vpn.borrow().as_ref()
.and_then( | vpn | vpn.profiles.first())
.map( |p | (p.name.to_string(), p.is_active));
if let Some((name, is_active)) = first_vpn {
let command = NetworkCommandMessage::toggle_vpn( & name, ! is_active);
broadcaster.broadcast_message_to_topic(command);
}
} else if let (Some(topic), Some(payload)) = (click_topic.clone(), click_payload.clone()) {
let payload_str = payload.to_string();
if let Some(instance) = click_instance.clone() {
broadcaster.broadcast_string_to_instance( & instance, & topic, & payload_str);
} else {
broadcaster.broadcast_string( & topic, & payload_str);
}
}
gesture.set_state(EventSequenceState::Claimed);
});
outer_box.add_controller(click_gesture);

// Long-press: broadcast to longpress_topic
let longpress_gesture = GestureLongPress::builder()
.button(0)
.propagation_phase(PropagationPhase::Capture)
.build();
longpress_gesture.connect_pressed( move | gesture, _x, _y| {
if let (Some(topic), Some(payload)) = (longpress_topic.clone(), longpress_payload.clone()) {
let payload_str = payload.to_string();
if let Some(instance) = longpress_instance.clone() {
broadcaster.broadcast_string_to_instance( & instance, & topic, & payload_str);
} else {
broadcaster.broadcast_string( & topic, & payload_str);
}
}
gesture.set_state(EventSequenceState::Claimed);
});
outer_box.add_controller(longpress_gesture);
```

### 5.5 Widget Layout

The widget builds a vertical `GtkBox` with an `Image` (for the Nerd Font icon) and two `Label`s (for value and info), similar to the Weather Widget but with an
`Image` instead of an icon `Label`. The `spacing` config value controls the gap between elements, and `icon_size` controls the pixel size of the `Image`:

```rust
let main_box = gtk4::Box::builder()
.orientation(gtk4::Orientation::Vertical)
.spacing(config.spacing)
.halign(gtk4::Align::Center)
.valign(gtk4::Align::Center)
.css_classes(["network-widget".to_string()])
.build();

let icon_image = gtk4::Image::builder()
.css_classes(["network-icon".to_string()])
.pixel_size(config.icon_size)
.build();

let value_label = gtk4::Label::builder()
.css_classes(["network-value".to_string()])
.build();

let info_label = gtk4::Label::builder()
.css_classes(["network-info".to_string()])
.build();

main_box.append( & icon_image);
main_box.append( & value_label);
main_box.append( & info_label);
```

The `update_ui` method resolves the icon name returned by `render_view` and updates the `Image`:

```rust
fn update_ui(&self) {
    let status = self.latest_status.borrow();
    let scan = self.latest_scan.borrow();
    let vpn = self.latest_vpn.borrow();
    let view = self.config.views.get(*self.current_view.borrow()).copied().unwrap_or(NetworkView::WifiStatus);

    let (icon_name, value, info) = render_view(
        status.as_ref().unwrap_or(&Default::default()),
        scan.as_ref(),
        vpn.as_ref(),
        &self.config,
        view,
    );

    // Update icon image via resolve_gtk_nerd_icon
    if let Some(ref img) = *self.icon_image.borrow() {
        if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(&icon_name) {
            let resource_path = format!("/com/nerd/icons/{}.svg", gtk_icon_name);
            if gio::resources_lookup_data(&resource_path, gio::ResourceLookupFlags::NONE).is_ok() {
                img.set_resource(Some(&resource_path));
            } else {
                img.set_from_icon_name(Some(&gtk_icon_name));
            }
        }
    }

    // Update value and info labels
    if let Some(ref label) = *self.value_label.borrow() {
        label.set_text(&value);
    }
    if let Some(ref label) = *self.info_label.borrow() {
        label.set_text(&info);
    }

    // Toggle visibility for QrCode view
    let is_qr = view == NetworkView::QrCode;
    if let Some(ref img) = *self.icon_image.borrow() { img.set_visible(!is_qr); }
    if let Some(ref label) = *self.value_label.borrow() { label.set_visible(!is_qr); }
    if let Some(ref label) = *self.info_label.borrow() { label.set_visible(!is_qr); }
    if let Some(ref area) = *self.qr_drawing_area.borrow() {
        area.set_visible(is_qr);
        if is_qr { area.queue_draw(); }
    }
}
```

### 5.6 Removed Widget Features

The following features are removed from the widget crate and are expected to be reimplemented as separate button widgets in `network_area`:

- **WLAN scan list** (`scan_list.rs`) — becomes a dedicated button widget in `network_area`
- **Throughput sparkline** (`sparkline.rs`) — removed; throughput is shown as text in the `Throughput` view
- **Password dialog** — becomes a dedicated button widget in `network_area`
- **VPN configuration UI** — VPN profile configuration (adding, editing, removing profiles) is handled by a NetworkManager tool (e.g., `nm-connection-editor`)
  launched from `network_area`, not in the widget

### 5.7 Retained Widget Features

The following features are retained from Milestone 1 and adapted to the view-based tile:

- **WiFi status & toggle** — the `WifiStatus` view shows the SSID, signal strength, and IP of the most recently connected WiFi device. Clicking the tile
  sends `ToggleRadio("wifi", !current_state)` to the service. The service selects the WiFi device as the most recently connected WiFi interface.
- **Ethernet status & toggle** — the `EthernetStatus` view shows the connection state and IP of the primary Ethernet device. Clicking the tile disconnects
  or reconnects the Ethernet interface.
- **Airplane mode toggle** — when the current view is `Airplane`, clicking the tile sends a `NetworkCommandMessage::toggle_radio("all", !current_state)` command
  to the service. The icon and value label update on the next status broadcast. No separate button widget is needed; the tile itself acts as the toggle.
- **QR code generator** — retained as a dedicated `QrCode` view. Instead of a button-triggered overlay, the QR code is rendered directly in the tile via a
  `gtk4::DrawingArea` with custom draw function. The `DrawingArea` is hidden in all other views and shown only when the `QrCode` view is active. The QR code
  encodes the current WiFi SSID in `WIFI:S:<SSID>;T:WPA;P:;;` format.
- **VPN toggle** — retained as a `Vpn` view. The view shows the first VPN profile's name and ON/OFF state. Clicking the tile in this view sends a
  `NetworkCommandMessage::toggle_vpn(first_profile_name, !current_state)` command to the service. Only the first VPN profile is toggled; managing multiple VPN
  profiles is done via a NetworkManager tool (e.g., `nm-connection-editor`) launched from `network_area`.

### 5.8 File Structure (Updated)

- `widget.rs` — `NetworkWidget` struct, view rendering, gesture handling, trait implementations
- `config.rs` — `NetworkWidgetConfig` with `views` list
- `lib.rs` — `widget_plugin!` macro invocation

Removed files:

- `status_view.rs` (merged into `widget.rs` as `render_view`)
- `scan_list.rs`
- `sparkline.rs`

---

## 6. CSS Styling

New CSS classes mirror the Weather Widget styling:

```css
/* Network widget */
.network-widget {
    padding: 4px 8px;
}

.network-widget:hover {
    background-color: #00a1e433;
}

.network-icon {
    color: white;
    filter: brightness(0.8) drop-shadow(0 0 4px #00a1e4ff) drop-shadow(0 0 8px #00a1e4cc);
}

.network-value {
    font-family: "JetBrains Mono", "Fira Code", "Monospace", sans-serif;
    font-size: 16px;
    font-weight: bold;
    color: white;
    text-shadow: 0 0 4px rgba(0, 170, 255, 0.6);
}

.network-info {
    font-size: 0.8em;
    color: #88ccff;
    text-shadow: 0 0 4px rgba(136, 204, 255, 0.4);
}

.network-qr {
    background-color: white;
    border-radius: 4px;
    padding: 2px;
}
```

---

## 7. Configuration Example

### 7.1 Widget Configuration in `config.toml`

```toml
[network_menu_widget]
width = 100
height = 100
spacing = 0
button_size = 48
icon_size = 36
views = ["WifiStatus", "EthernetStatus", "Throughput", "WifiScan", "Vpn", "Airplane", "QrCode"]
max_access_points = 10
# All icon fields are optional and default to the Nerd Font names below.
# Override any icon by specifying its Nerd Font name:
# icon_wifi_strength_4 = "nf-md-wifi_strength_4"
# icon_wifi_strength_3 = "nf-md-wifi_strength_3"
# icon_wifi_strength_2 = "nf-md-wifi_strength_2"
# icon_wifi_strength_1 = "nf-md-wifi_strength_1"
# icon_wifi_strength_off = "nf-md-wifi_strength_off"
# icon_ethernet_on = "nf-md-network_outline"
# icon_ethernet_off = "nf-md-network_off"
# icon_vpn_on = "nf-md-shield_key"
# icon_vpn_off = "nf-md-shield_off"
# icon_airplane_on = "nf-md-airplane"
# icon_airplane_off = "nf-md-airplane_off"
# icon_throughput = "nf-md-swap_vertical"
# icon_wifi_scan = "nf-md-wifi_strength_4"
# icon_qr_code = "nf-md-qrcode"
click_topic = "area.open"
click_payload = { area_id = "network_area" }
longpress_topic = "area.open"
longpress_payload = { area_id = "network_area" }
```

### 7.2 Network Area in `config.toml`

The `network_area` scroll menu contains the close button, app launchers, and (future) detailed network control buttons:

```toml
[network_area]
include = "areas/scroll_menu.toml"
open_transition = "SlideUp"
css_classes = ["network-area-bg"]
plugins = [
    { id = "network_close_button", path = "target/release/libsmearor_button_widget.so" },
    { id = "etherape", path = "target/release/libsmearor_app_launcher_widget.so" },
    { id = "gnome_network_displays", path = "target/release/libsmearor_app_launcher_widget.so" },
    { id = "digger", path = "target/release/libsmearor_app_launcher_widget.so" },
    { id = "ping_monitor", path = "target/release/libsmearor_app_launcher_widget.so" }
]

[network_close_button]
defaults = "close_button"
click_payload = { area_id = "network_area" }
```

---

## 8. Architecture Diagram

```
+-------------------+     swipe up/down      +-------------------+
| Network Widget    |-----> cycle views ---->|                   |
| (compact tile)    |                        |                   |
| icon_label        |     click              |  network_area     |
| value_label       |-----> open area ------>|  (scroll menu)    |
| info_label        |     (Throughput/       |                   |
| qr_drawing_area   |      WifiScan/QrCode)  |  - Close button   |
+-------------------+                        |  - Etherape       |
          |                                  |  - GNOME Net Disp |
          |     click (WifiStatus)           |  - Digger         |
          |-----> toggle WiFi radio -------->|  - Ping Monitor   |
          |                                  |  - (future) WLAN  |
          |     click (EthernetStatus)       |  - (future) nm-   |
          |-----> toggle Ethernet ---------->|    connection-    |
          |                                  |    editor         |
          |     click (Airplane view)        |                   |
          |-----> toggle radio ------------->|                   |
          |     click (Vpn view)             |                   |
          |-----> toggle first VPN --------->|                   |
          |  subscribes to                   |                   |
          v                                  +-------------------+
+-------------------+
| Network Service   |
| (D-Bus / zbus)    |
+-------------------+
```

---

## 9. Roadmap

### Phase 1: Model Addition — `NetworkView` Enum

**Goal:** Add the `NetworkView` enum to `model/network`.

**Order:**

1. Create `model/network/src/messages/view.rs` with the `NetworkView` enum.
2. Add `pub use messages::view::NetworkView;` to `model/network/src/lib.rs`.
3. Run `cargo check -p smearor-network-model`.

**Exit criteria:**

- The enum compiles with `#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]`.
- All variants have English rustdoc comments.

---

### Phase 2: Widget Config Refactor

**Goal:** Replace `show_*` booleans with `views` list and add `click_instance` / `longpress_instance`.

**Dependencies:** Phase 1 must be complete.

**Order:**

1. Update `plugins/network/src/config.rs` with the new `NetworkWidgetConfig` struct.
2. Remove `show_status`, `show_scan_list`, `show_airplane_toggle`, `show_vpn_toggles`, `show_throughput`, `show_qr_code`, `spacing`, `button_size`, `icon_size`
   fields.
3. Add `views: Vec<NetworkView>` field with default containing all views.
4. Add `click_instance`, `longpress_topic`, `longpress_payload`, `longpress_instance` fields.
5. Add icon configuration fields (`icon_wifi_strength_*`, `icon_ethernet_on/off`, `icon_vpn_on/off`, `icon_airplane_on/off`, `icon_throughput`,
   `icon_wifi_scan`, `icon_qr_code`) with serde defaults.
6. Update `Default` impl.
7. Run `cargo check -p smearor-network-widget`.

**Exit criteria:**

- Config deserializes correctly from TOML.
- Default config contains all seven views.
- `cargo check` passes.

---

### Phase 3: Widget Redesign

**Goal:** Rewrite `widget.rs` to use the view-based tile pattern.

**Dependencies:** Phase 2 must be complete.

**Order:**

1. Rewrite `NetworkWidget` struct to hold `icon_image` (`gtk4::Image`), `value_label`, `info_label`, `current_view`, `latest_status`, `latest_scan`,
   `latest_vpn`.
2. Implement `render_view` function for all seven `NetworkView` variants, returning Nerd Font icon names from config.
3. Implement `update_ui` method: resolve icon names via `resolve_gtk_nerd_icon` and update `Image` from GResource; update value/info labels.
4. Implement `next_view`, `prev_view` methods (mirroring Weather Widget).
5. Implement `build_widget` with `Image` + two `Label`s in a vertical `GtkBox`.
6. Add `GestureClick` with `click_topic` / `click_payload` / `click_instance` support.
7. Add `GestureDrag` for swipe-based view switching.
8. Add `GestureLongPress` with `longpress_topic` / `longpress_payload` / `longpress_instance` support.
9. Update message handlers to store latest status, scan, and VPN messages and trigger `update_ui`.
10. Remove all unused code: scan list rendering, VPN list rendering, sparkline, password dialog, airplane toggle button.
11. Run `cargo check` and `cargo clippy`.

**Exit criteria:**

- Widget compiles and loads as a plugin.
- `Image` + two `Label`s are rendered in a vertical box.
- Icons resolve correctly via `resolve_gtk_nerd_icon` from config.
- Swipe up/down cycles through views.
- Click broadcasts `click_topic` with `click_payload`.
- Long-press broadcasts `longpress_topic` with `longpress_payload`.
- `click_instance` and `longpress_instance` are respected when set.
- No `unwrap`, `expect`, or `panic` in new code.
- `cargo clippy` is clean.

---

### Phase 4: CSS Styling

**Goal:** Add CSS classes for the network widget tile.

**Dependencies:** Phase 3 must be complete.

**Order:**

1. Add `.network-widget`, `.network-icon`, `.network-value`, `.network-info` CSS rules to `resources/style.css`.
2. Add `.network-widget:hover` rule for hover effect.
3. Add `.network-area-bg` CSS rule for the network area background.

**Exit criteria:**

- Widget tile matches Weather Widget visual style (glow, font, spacing).
- Hover effect is applied.

---

### Phase 5: Configuration Wiring

**Goal:** Update `config.toml` with the new widget config and verify the `network_area` scroll menu.

**Dependencies:** Phase 4 must be complete.

**Order:**

1. Update `[network_menu_widget]` in `config.toml` with `views` list and `click_topic` / `longpress_topic`.
2. Verify `network_area` plugins list includes close button and all app launchers.
3. Build the project in release mode.
4. Run the application and verify the widget renders.

**Exit criteria:**

- Widget renders as a compact tile in the scroll band.
- Swipe cycles through views.
- Click opens `network_area`.
- Long-press opens `network_area` (or triggers configured action).

---

### Phase 6: Validation

**Goal:** End-to-end verification.

**Dependencies:** Phase 5 must be complete.

**Order:**

1. Verify WifiStatus view shows SSID, signal %, and IP of the most recently connected WiFi device, with the correct `nf-md-wifi_strength_*` icon based on signal
   level.
2. Verify EthernetStatus view shows connection state and IP of the primary Ethernet device, with `nf-md-network_outline` (on) or `nf-md-network_off` (off).
3. Verify throughput view shows download/upload rates with `nf-md-swap_vertical` icon.
4. Verify WiFi scan view shows network count and strongest signal.
5. Verify VPN view shows first VPN profile name and ON/OFF state, with `nf-md-shield_key` (on) or `nf-md-shield_off` (off).
6. Verify airplane view shows on/off state, with `nf-md-airplane` (on) or `nf-md-airplane_off` (off).
7. Verify QR code view renders a scannable QR code for the current WiFi connection.
8. Verify swipe gestures cycle through all configured views.
9. Verify click opens `network_area` for Throughput/WifiScan/QrCode views.
10. Verify click toggles WiFi radio when in `WifiStatus` view (sends `ToggleRadio("wifi", !state)` command).
11. Verify click toggles Ethernet connection when in `EthernetStatus` view (disconnects or reconnects).
12. Verify click toggles airplane mode when in `Airplane` view (sends `ToggleRadio` command).
13. Verify click toggles the first VPN profile when in `Vpn` view (sends `ToggleVpn` command).
14. Verify long-press triggers configured action.
15. Verify custom icon names in `config.toml` override the default Nerd Font icons.
16. Run `cargo test`, `cargo clippy`, and `cargo fmt`.

**Exit criteria:**

- All views render correct data from service messages.
- View switching is smooth and responsive.
- No crashes or panics.
- `cargo test`, `cargo clippy`, and `cargo fmt` are clean.

---

## 10. Summary of Order

```
Phase 1: model/network — NetworkView enum
    |
    v
Phase 2: plugins/network — config refactor
    |
    v
Phase 3: plugins/network — widget redesign
    |
    v
Phase 4: resources/style.css — CSS styling
    |
    v
Phase 5: config.toml — configuration wiring
    |
    v
Phase 6: validation — end-to-end tests
```

---

## 11. Compliance with `AGENTS.md`

- **Crate separation:** Model (`model/network`), Service (`services/network`), Widget (`plugins/network`) remain separate crates.
- **One struct/enum per file:** `NetworkView` lives in its own file `messages/view.rs`.
- **Widget traits:** The widget implements `MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>`, and `WidgetBuilder`.
- **Async runtime:** The service uses `tokio::sync::mpsc` and the `PluginExecutor`. The widget uses `glib::MainContext::spawn_local` for GTK updates.
- **Event-driven:** The widget updates exclusively through incoming messages. No polling loops.
- **FFI stability:** `NetworkView` is not an FFI type (it is widget-internal config), so `#[stabby::stabby]` is not required. All existing FFI types remain
  unchanged.
- **No panic:** The implementation uses `Result` and `Option` for error handling.
- **Naming:** All names are descriptive and follow Rust naming conventions (`snake_case` for fields, `PascalCase` for types).
- **Documentation:** All public structs, enums, and fields have English rustdoc comments.
- **Import organization:** One import per line, alphabetical ordering, no star imports (except `gtk4::prelude::*`).
- **Formatting:** Code is formatted with `rustfmt` and checked with `clippy`.

---

## 12. Technical Notes

- **View rendering is text + Image (except QrCode):** Most views show a Nerd Font icon (rendered as `gtk4::Image` via `resolve_gtk_nerd_icon`) and two text
  labels. The `QrCode` view is the exception: it hides the icon/value/info and shows a `gtk4::DrawingArea` that renders the QR code directly in the tile. This
  keeps the widget lightweight while retaining the QR sharing feature.
- **Icon rendering via `resolve_gtk_nerd_icon`:** All icons are rendered as `gtk4::Image` from GResource SVGs, using the same mechanism as the Wallpaper and
  Button widgets. The `render_view` function returns a Nerd Font icon name (e.g., `nf-md-wifi_strength_4`) from the config, and `update_ui` resolves it to a
  GResource path via `resolve_gtk_nerd_icon`. This replaces the Milestone 1 approach of hardcoding Unicode codepoints in a `Label`.
- **Configurable icons:** Every icon is configurable via `config.toml` using Nerd Font icon names. Defaults: WiFi uses `nf-md-wifi_strength_4`/`3`/`2`/`1`/`off`
  based on signal; Ethernet uses `nf-md-network_outline`/`network_off`; VPN uses `nf-md-shield_key`/`shield_off`; Airplane uses `nf-md-airplane`/`airplane_off`.
- **QR code layout switching:** The `DrawingArea` is always present in the widget's `GtkBox` but hidden by default (`visible = false`). When the `QrCode` view
  becomes active, `update_ui` toggles visibility: labels are hidden, the `DrawingArea` is shown and `queue_draw()` is called. When switching away, the reverse
  happens. This avoids creating/destroying widgets on each view change.
- **Scan data in tile is summary-only:** The `WifiScan` view shows the count of networks and the strongest signal percentage. Detailed scan lists are in
  `network_area` (future button widgets).
- **Throughput is text-only:** The `Throughput` view shows download and upload rates as formatted strings (e.g., "1.2 MB/s"). The sparkline is removed from the
  tile.
- **Gesture priority:** `GestureDrag` uses `PropagationPhase::Capture` to intercept swipe gestures before child widgets. `GestureClick` and `GestureLongPress`
  also use `PropagationPhase::Capture` and check `EventSequenceState` to avoid conflicts.
- **VPN toggle is first-profile-only:** The `Vpn` view shows and toggles only the first VPN profile from `VpnProfilesMessage`. Managing multiple VPN profiles (
  adding, editing, removing) is done via `nm-connection-editor` or a similar NetworkManager tool launched from `network_area`.
- **View-dependent click behavior:** The click action depends on the current view: `WifiStatus` sends `ToggleRadio("wifi", !state)`, `EthernetStatus` sends
  `Disconnect`/reconnect, `Airplane` sends `ToggleRadio("all", !state)`, `Vpn` sends `ToggleVpn(first_profile, !state)`, all other views broadcast the
  configured `click_topic` (typically opening `network_area`). This keeps interactive toggles inline without separate button widgets.
- **WiFi device selection:** The `WifiStatus` view shows the most recently connected WiFi device. The service should prioritize WiFi interfaces by recency of
  connection (last connected = first in the list). This ensures the user sees the WiFi network they most recently joined, even if multiple WiFi adapters exist.
- **`network_area` is the detail view:** The scroll menu area contains app launchers and will be extended with dedicated button widgets for WLAN connection and
  `nm-connection-editor` in a future milestone. Airplane mode, QR code, and VPN toggle are NOT in `network_area` — they stay in the widget tile.

---

*End of document.*
