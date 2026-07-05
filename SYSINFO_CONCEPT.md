# Concept: Sysinfo Service & Widgets

This document describes the concept for a central **Sysinfo Service**, a dedicated **Sysinfo Widget** crate, and the extension of the **Button Widget** with
dynamic topic subscriptions. All components follow the decoupled architecture of the *Smearor Swipe Launcher*.

---

## 1. Sysinfo Service

### 1.1 Overview

The Sysinfo Service is a singleton background service that collects system metrics at regular intervals and publishes each metric on its own topic via the
central message broker. It contains no GTK code and is therefore completely decoupled from the user interface.

### 1.2 Collected Metrics

| Metric              | Unit                  | Topic                            | Description                                             |
|---------------------|-----------------------|----------------------------------|---------------------------------------------------------|
| CPU usage           | percent (0.0 - 100.0) | `service.sysinfo.cpu.status`     | Current total utilization across all CPU cores          |
| CPU temperature     | degrees Celsius (°C)  | `service.sysinfo.cpu.status`     | CPU temperature from the first available thermal sensor |
| Memory usage        | percent (0.0 - 100.0) | `service.sysinfo.memory.status`  | Share of occupied physical RAM                          |
| Memory total        | bytes (u64)           | `service.sysinfo.memory.status`  | Total size of physical RAM                              |
| Memory used         | bytes (u64)           | `service.sysinfo.memory.status`  | Actually occupied physical RAM                          |
| Memory available    | bytes (u64)           | `service.sysinfo.memory.status`  | Free RAM **without** page cache                         |
| Battery level       | percent (0.0 - 100.0) | `service.sysinfo.battery.status` | Remaining battery charge                                |
| Battery status      | enum                  | `service.sysinfo.battery.status` | Charging, discharging, full, unknown                    |
| Disk usage          | percent (0.0 - 100.0) | `service.sysinfo.disks.status`   | Per-mount usage percentage                              |
| Disk read           | bytes per second      | `service.sysinfo.disks.status`   | Aggregate disk read throughput                          |
| Disk write          | bytes per second      | `service.sysinfo.disks.status`   | Aggregate disk write throughput                         |
| Network received    | bytes per second      | `service.sysinfo.network.status` | Aggregate inbound throughput                            |
| Network transmitted | bytes per second      | `service.sysinfo.network.status` | Aggregate outbound throughput                           |
| Uptime              | seconds (u64)         | `service.sysinfo.uptime.status`  | Time since system boot                                  |
| Load average        | tuple of f32          | `service.sysinfo.uptime.status`  | 1-minute, 5-minute, and 15-minute load averages         |

### 1.3 Crate Structure

The sysinfo functionality is split into four separate crates:

| Crate                | Path                | Responsibility                                                  |
|----------------------|---------------------|-----------------------------------------------------------------|
| **Model**            | `model/sysinfo/`    | Shared structs, enums, and message formats for all topics       |
| **Service**          | `services/sysinfo/` | Backend logic, collection, and broadcast of metrics             |
| **Widgets**          | `plugins/sysinfo/`  | Dedicated widgets: CPU, Memory, Battery, Disks, Network, Uptime |
| **Button Extension** | `plugins/button/`   | Dynamic label subscription via topic                            |

---

## 2. Model Crate (`model/sysinfo`)

### 2.1 Message Topics

Each metric category publishes on its own topic. Widgets subscribe only to the topics they actually need.

```rust
pub const TOPIC_CPU: &str = "service.sysinfo.cpu.status";
pub const TOPIC_MEMORY: &str = "service.sysinfo.memory.status";
pub const TOPIC_BATTERY: &str = "service.sysinfo.battery.status";
pub const TOPIC_DISKS: &str = "service.sysinfo.disks.status";
pub const TOPIC_NETWORK: &str = "service.sysinfo.network.status";
pub const TOPIC_UPTIME: &str = "service.sysinfo.uptime.status";
```

### 2.2 CPU Message

```rust
/// Status message for CPU metrics.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct CpuStatusMessage {
    /// CPU usage in percent (0.0 - 100.0).
    pub cpu_usage: f32,
    /// CPU temperature in degrees Celsius.
    pub cpu_temperature: Option<f32>,
}
```

### 2.3 Memory Message

```rust
/// Status message for memory metrics.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct MemoryStatusMessage {
    /// Memory usage in percent (0.0 - 100.0).
    pub memory_usage: f32,
    /// Total physical memory in bytes.
    pub memory_total: u64,
    /// Used physical memory in bytes.
    pub memory_used: u64,
    /// Available physical memory in bytes (without cache).
    pub memory_available: u64,
}
```

### 2.4 Battery Message

```rust
/// Charging state of the battery.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub enum BatteryStatus {
    /// State is unknown.
    #[default]
    Unknown,
    /// Battery is discharging.
    Discharging,
    /// Battery is charging.
    Charging,
    /// Battery is fully charged.
    Full,
}

/// Status message for battery metrics.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct BatteryStatusMessage {
    /// Battery charge level in percent (0.0 - 100.0).
    pub level: f32,
    /// Current charging state.
    pub status: BatteryStatus,
}
```

### 2.5 Disks Message

```rust
/// Usage information for a single mount point.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[stabby::stabby]
pub struct DiskUsage {
    /// Mount point path.
    pub mount_point: String,
    /// Usage percentage (0.0 - 100.0).
    pub usage: f32,
    /// Total capacity in bytes.
    pub total: u64,
    /// Used space in bytes.
    pub used: u64,
    /// Available space in bytes.
    pub available: u64,
}

/// Status message for disk metrics.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct DisksStatusMessage {
    /// Per-mount usage information.
    pub mounts: Vec<DiskUsage>,
    /// Aggregate read throughput in bytes per second.
    pub read_bytes_per_second: u64,
    /// Aggregate write throughput in bytes per second.
    pub write_bytes_per_second: u64,
}
```

### 2.6 Network Message

```rust
/// Status message for network metrics.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct NetworkStatusMessage {
    /// Aggregate inbound throughput in bytes per second.
    pub received_bytes_per_second: u64,
    /// Aggregate outbound throughput in bytes per second.
    pub transmitted_bytes_per_second: u64,
}
```

### 2.7 Uptime Message

```rust
/// Status message for uptime and load average.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct UptimeStatusMessage {
    /// Time since system boot in seconds.
    pub uptime_seconds: u64,
    /// 1-minute load average.
    pub load_average_1_minute: f32,
    /// 5-minute load average.
    pub load_average_5_minute: f32,
    /// 15-minute load average.
    pub load_average_15_minute: f32,
}
```

---

## 3. Service Crate (`services/sysinfo`)

### 3.1 File Structure

- `service.rs` - `SysinfoService` struct and trait implementations
- `config.rs` - `SysinfoServiceConfig` struct and parsing
- `collector.rs` - Metric collection logic
- `lib.rs` - `service_plugin!` macro invocation

### 3.2 Service Implementation

```rust
pub struct SysinfoService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: SysinfoServiceConfig,
    pub cpu_state: Arc<RwLock<CpuStatusMessage>>,
    pub memory_state: Arc<RwLock<MemoryStatusMessage>>,
    pub battery_state: Arc<RwLock<BatteryStatusMessage>>,
    pub disks_state: Arc<RwLock<DisksStatusMessage>>,
    pub network_state: Arc<RwLock<NetworkStatusMessage>>,
    pub uptime_state: Arc<RwLock<UptimeStatusMessage>>,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<SysinfoCommandMessage>>` - Processes optional control commands
- `MessageBroadcaster` - Broadcasts messages to the broker
- `MessageTopicBroadcaster` - Broadcasts to specific topic subscribers
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context

### 3.3 Configuration

```rust
/// Configuration for the sysinfo service.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SysinfoServiceConfig {
    /// Query interval for metrics in milliseconds.
    pub update_interval_ms: u64,
    /// Whether CPU temperature should be collected.
    pub enable_cpu_temperature: bool,
    /// Whether battery metrics should be collected.
    pub enable_battery: bool,
    /// Whether disk metrics should be collected.
    pub enable_disks: bool,
    /// Whether network metrics should be collected.
    pub enable_network: bool,
}

impl Default for SysinfoServiceConfig {
    fn default() -> Self {
        Self {
            update_interval_ms: 1000,
            enable_cpu_temperature: true,
            enable_battery: true,
            enable_disks: true,
            enable_network: true,
        }
    }
}
```

### 3.4 Metric Collection

| Metric             | Data Source                             | Implementation                                             |
|--------------------|-----------------------------------------|------------------------------------------------------------|
| CPU usage          | `/proc/stat`                            | Difference between idle and total ticks across two queries |
| CPU temperature    | `/sys/class/thermal/thermal_zone*/temp` | First available thermal sensor, divided by 1000            |
| Memory total       | `/proc/meminfo`                         | `MemTotal`                                                 |
| Memory used        | `/proc/meminfo`                         | `MemTotal - MemAvailable`                                  |
| Memory available   | `/proc/meminfo`                         | `MemAvailable`                                             |
| Memory usage       | Calculated                              | `(memory_used / memory_total) * 100.0`                     |
| Battery level      | `/sys/class/power_supply/BAT*/capacity` | Read capacity file                                         |
| Battery status     | `/sys/class/power_supply/BAT*/status`   | Map status string to enum                                  |
| Disk usage         | `statvfs`                               | Per-mount usage                                            |
| Disk throughput    | `/proc/diskstats`                       | Delta between two reads                                    |
| Network throughput | `/proc/net/dev`                         | Delta between two reads                                    |
| Uptime             | `/proc/uptime`                          | First floating-point value                                 |
| Load average       | `/proc/loadavg`                         | First three values                                         |

### 3.5 Background Update Loop

On initialization, the service spawns one asynchronous Tokio task. In this task, all collectors are called in the configured interval, and each result is
broadcast to its dedicated topic.

```rust
async fn run_update_loop(
    config: SysinfoServiceConfig,
    state: SysinfoState,
    broadcaster: Box<dyn MessageTopicBroadcaster>,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(config.update_interval_ms));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        let cpu = Self::collect_cpu(&config).await;
        let memory = Self::collect_memory().await;
        let battery = Self::collect_battery(&config).await;
        let disks = Self::collect_disks(&config).await;
        let network = Self::collect_network(&config).await;
        let uptime = Self::collect_uptime().await;

        state.cpu_state.write().await.clone_from(&cpu);
        state.memory_state.write().await.clone_from(&memory);
        state.battery_state.write().await.clone_from(&battery);
        state.disks_state.write().await.clone_from(&disks);
        state.network_state.write().await.clone_from(&network);
        state.uptime_state.write().await.clone_from(&uptime);

        broadcaster.broadcast_topic(TOPIC_CPU, cpu);
        broadcaster.broadcast_topic(TOPIC_MEMORY, memory);
        broadcaster.broadcast_topic(TOPIC_BATTERY, battery);
        broadcaster.broadcast_topic(TOPIC_DISKS, disks);
        broadcaster.broadcast_topic(TOPIC_NETWORK, network);
        broadcaster.broadcast_topic(TOPIC_UPTIME, uptime);
    }
}
```

### 3.6 Message Flow

```
+-------------------------+        +-------------------------+
| SysinfoService          |        | CpuWidget               |
| (Singleton)             |        | (subscribed to          |
|                         |=======>|   service.sysinfo.cpu)  |
| - /proc/stat            |        |                         |
| - /sys/class/thermal    |        +-------------------------+
+-------------------------+        +-------------------------+
|                         |        | MemoryWidget            |
| - /proc/meminfo         |=======>| (subscribed to          |
|                         |        |   service.sysinfo.memory|
+-------------------------+        +-------------------------+
|                         |        | BatteryWidget           |
| - /sys/class/power      |=======>| (subscribed to          |
|   supply/BAT*           |        |   service.sysinfo.battery|
+-------------------------+        +-------------------------+
|                         |        | DisksWidget             |
| - /proc/diskstats       |=======>| (subscribed to          |
| - statvfs               |        |   service.sysinfo.disks) |
+-------------------------+        +-------------------------+
|                         |        | NetworkWidget           |
| - /proc/net/dev         |=======>| (subscribed to          |
|                         |        |   service.sysinfo.network|
+-------------------------+        +-------------------------+
|                         |        | UptimeWidget            |
| - /proc/uptime          |=======>| (subscribed to          |
| - /proc/loadavg         |        |   service.sysinfo.uptime)
+-------------------------+        +-------------------------+
```

---

## 4. Sysinfo Widget Crate (`plugins/sysinfo`)

### 4.1 Overview

The `plugins/sysinfo` crate provides a set of dedicated widgets, one for each metric category. Each widget is a separate plugin that can be instantiated
independently. CPU and Memory widgets support two visual representations: **Bar** and **Gauge**.

### 4.2 File Structure

| File                | Responsibility                         |
|---------------------|----------------------------------------|
| `lib.rs`            | `widget_plugin!` macro invocations     |
| `config.rs`         | Shared configuration structs and enums |
| `widget_cpu.rs`     | `CpuWidget` implementation             |
| `widget_memory.rs`  | `MemoryWidget` implementation          |
| `widget_battery.rs` | `BatteryWidget` implementation         |
| `widget_disks.rs`   | `DisksWidget` implementation           |
| `widget_network.rs` | `NetworkWidget` implementation         |
| `widget_uptime.rs`  | `UptimeWidget` implementation          |

### 4.3 Shared Configuration

```rust
/// Visual representation for percentage-based widgets.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub enum DisplayMode {
    /// Horizontal or vertical progress bar.
    #[default]
    Bar,
    /// Circular or semicircular gauge.
    Gauge,
}

/// Orientation of a bar display.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub enum BarOrientation {
    /// Horizontal bar.
    #[default]
    Horizontal,
    /// Vertical bar.
    Vertical,
}

/// Configuration shared by percentage-based widgets.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct PercentageWidgetConfig {
    /// Visual display mode.
    pub display_mode: DisplayMode,
    /// Orientation when display_mode is Bar.
    pub bar_orientation: BarOrientation,
    /// Whether to show the numeric value as text.
    pub show_value: bool,
    /// Whether to show an icon next to the value.
    pub show_icon: bool,
    /// Width of the widget in pixels.
    pub width: i32,
    /// Height of the widget in pixels.
    pub height: i32,
    /// Optional icon name.
    pub icon: Option<String>,
    /// Format string for the numeric label.
    pub value_format: String,
    /// Color threshold for warning state.
    pub warning_threshold: f32,
    /// Color threshold for critical state.
    pub critical_threshold: f32,
}
```

### 4.4 CPU Widget

The CPU Widget subscribes to `service.sysinfo.cpu` and displays CPU usage and optionally temperature.

```rust
/// Configuration for the CPU widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct CpuWidgetConfig {
    /// Shared percentage widget configuration.
    pub percentage: PercentageWidgetConfig,
    /// Whether to display the CPU temperature.
    pub show_temperature: bool,
}

/// CPU widget state and UI elements.
pub struct CpuWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: CpuWidgetConfig,
    pub container: Arc<RwLock<Option<gtk4::Box>>>,
    pub value_bar: Arc<RwLock<Option<gtk4::LevelBar>>>,
    pub value_gauge: Arc<RwLock<Option<gtk4::DrawingArea>>>,
    pub label: Arc<RwLock<Option<gtk4::Label>>>,
}
```

**Display modes:**

- **Bar:** A `gtk4::LevelBar` filled according to `cpu_usage`.
- **Gauge:** A `gtk4::DrawingArea` with a custom draw function rendering a circular or semicircular arc proportional to `cpu_usage`.

### 4.5 Memory Widget

The Memory Widget subscribes to `service.sysinfo.memory` and displays memory usage.

```rust
/// Configuration for the memory widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct MemoryWidgetConfig {
    /// Shared percentage widget configuration.
    pub percentage: PercentageWidgetConfig,
    /// Whether to display the absolute used memory in bytes.
    pub show_used_bytes: bool,
    /// Whether to display the available memory in bytes.
    pub show_available_bytes: bool,
}

/// Memory widget state and UI elements.
pub struct MemoryWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: MemoryWidgetConfig,
    pub container: Arc<RwLock<Option<gtk4::Box>>>,
    pub value_bar: Arc<RwLock<Option<gtk4::LevelBar>>>,
    pub value_gauge: Arc<RwLock<Option<gtk4::DrawingArea>>>,
    pub label: Arc<RwLock<Option<gtk4::Label>>>,
}
```

**Display modes:**

- **Bar:** A `gtk4::LevelBar` filled according to `memory_usage`.
- **Gauge:** A `gtk4::DrawingArea` with a custom draw function rendering a circular or semicircular arc proportional to `memory_usage`.

### 4.6 Battery Widget

The Battery Widget subscribes to `service.sysinfo.battery` and displays the charge level and charging state.

```rust
/// Configuration for the battery widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct BatteryWidgetConfig {
    /// Shared percentage widget configuration.
    pub percentage: PercentageWidgetConfig,
    /// Whether to display the charging status as text.
    pub show_status_text: bool,
    /// Whether to change the icon based on charging status.
    pub animate_icon: bool,
}
```

### 4.7 Disks Widget

The Disks Widget subscribes to `service.sysinfo.disks` and displays per-mount usage and optionally aggregate throughput.

```rust
/// Configuration for the disks widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct DisksWidgetConfig {
    /// Maximum number of mount points to display.
    pub max_mount_points: usize,
    /// Mount points to display (empty means all).
    pub include_mount_points: Vec<String>,
    /// Whether to show read/write throughput.
    pub show_throughput: bool,
    /// Whether to display the widget as a list or a single bar for the root mount.
    pub display_mode: DiskDisplayMode,
}

/// Display mode for the disks widget.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub enum DiskDisplayMode {
    /// Show the root mount point only.
    #[default]
    RootOnly,
    /// Show a list of configured mount points.
    List,
}
```

### 4.8 Network Widget

The Network Widget subscribes to `service.sysinfo.network` and displays inbound and outbound throughput.

```rust
/// Configuration for the network widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct NetworkWidgetConfig {
    /// Whether to show received bytes per second.
    pub show_received: bool,
    /// Whether to show transmitted bytes per second.
    pub show_transmitted: bool,
    /// Whether to show a small sparkline history.
    pub show_history: bool,
    /// Number of history samples to keep.
    pub history_length: usize,
}
```

### 4.9 Uptime Widget

The Uptime Widget subscribes to `service.sysinfo.uptime` and displays system uptime and load averages.

```rust
/// Configuration for the uptime widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct UptimeWidgetConfig {
    /// Whether to show the uptime as a human-readable duration.
    pub show_uptime: bool,
    /// Whether to show the 1-minute load average.
    pub show_load_average_1_minute: bool,
    /// Whether to show the 5-minute load average.
    pub show_load_average_5_minute: bool,
    /// Whether to show the 15-minute load average.
    pub show_load_average_15_minute: bool,
}
```

### 4.10 Widget Crate Registration

All widgets are registered in a single factory in `plugins/sysinfo/src/lib.rs`:

```rust
widget_factory_plugin! {
    "cpu" => CpuWidget,
    "memory" => MemoryWidget,
    "battery" => BatteryWidget,
    "disks" => DisksWidget,
    "network" => NetworkWidget,
    "uptime" => UptimeWidget,
}
```

The `widget` field in the plugin configuration selects which widget is instantiated.

---

## 5. Button Widget: Dynamic Label Subscription

### 5.1 Motivation

Currently, the Button Widget always displays a static text from `config.toml`. For dynamic displays (e.g., current CPU load or available memory), the Button
Widget should optionally be able to subscribe to a topic and derive its label text from incoming messages.

### 5.2 Extended Configuration

The existing `ButtonConfig` is extended with three optional fields:

```toml
[sysinfo_button]
icon = "nf-fae-chip"
icon_only = false

# Instead of a fixed label, subscribe to a topic
label_topic = "service.sysinfo.cpu.status"
label_format = "CPU {cpu_usage:.1}%"
label_fallback = "loading..."

click_topic = "area.open"
click_payload = { area_id = "sysinfo_area" }
```

```rust
/// Configuration for a button widget.
#[derive(Debug, Clone, Deserialize)]
pub struct ButtonConfig {
    // ... existing fields ...

    /// Topic whose messages control the label text.
    #[serde(default)]
    pub label_topic: Option<String>,
    /// Format string for the label display (JSON values via serde_json).
    #[serde(default)]
    pub label_format: Option<String>,
    /// Fallback text when the topic has not yet delivered a message.
    #[serde(default)]
    pub label_fallback: Option<String>,
}
```

### 5.3 Behavior

- If `label_topic` is not set, the Button Widget behaves as before: it displays `config.text`.
- If `label_topic` is set, the widget subscribes to this topic and updates the label text on every incoming message.
- The text can be formatted from the JSON payload of the received message. `label_format` supports simple string interpolation or a JSON template (e.g.,
  `"CPU: {cpu_usage:.1}%"`).
- Until a message arrives, `label_fallback` is displayed. If that is also not set, the label remains empty.

### 5.4 Trait Implementation

`AcceptTopic` and `MessageHandler` of the Button Widget must be adjusted so that messages on the `label_topic` are accepted and processed:

```rust
impl AcceptTopic<FfiEnvelope> for ButtonWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        if let Some(label_topic) = &self.config.label_topic {
            return topic == label_topic;
        }
        false
    }
}

impl MessageHandler<FfiEnvelope> for ButtonWidget {
    fn handle_message(&self, message: FfiEnvelope, _sender_id: &str) {
        if let Some(label_topic) = &self.config.label_topic {
            if message.topic == *label_topic {
                self.update_label_from_message(&message.payload);
            }
        }
    }
}
```

### 5.5 Label Update

The widget holds a weak reference to the `gtk4::Label`. When a message is received, `glib::MainContext::spawn_local` is used to update the label text in the GTK
main thread.

```rust
fn update_label_from_message(&self, payload: &str) {
    let format = self.config.label_format.clone();
    let label_weak = self.label_widget.downgrade();

    glib::MainContext::default().spawn_local(async move {
        if let Some(label) = label_weak.upgrade() {
            let text = format_label(payload, format.as_deref());
            label.set_text(&text);
        }
    });
}
```

### 5.6 Formatting

Formatting can happen in two ways:

1. **Simple interpolation** from the JSON payload (e.g., `"{cpu_usage:.1}%"`).
2. **Fixed format string without placeholders**, where the complete payload string is used as the label (useful for services that already send formatted
   strings).

Example: Received message on `service.sysinfo.cpu`:

```json
{
  "cpu_usage": 42.5,
  "cpu_temperature": 55.0
}
```

With `label_format = "CPU {cpu_usage:.1}%"`, the label becomes:

```
CPU 42.5%
```

---

## 6. Configuration Example

### 6.1 Service Configuration

```toml
[services]
load = ["sysinfo"]

[sysinfo]
update_interval_ms = 1000
enable_cpu_temperature = true
enable_battery = true
enable_disks = true
enable_network = true
```

### 6.2 CPU Widget Configuration

```toml
[[scroll_band.plugins]]
id = "cpu_widget"
path = "target/debug/libsysinfo_widget.so"

[cpu_widget]
widget = "Cpu"
display_mode = "Gauge"
bar_orientation = "Horizontal"
show_value = true
show_icon = true
show_temperature = true
icon = "nf-fae-chip"
value_format = "{cpu_usage:.0}%"
warning_threshold = 70.0
critical_threshold = 90.0
```

### 6.3 Memory Widget Configuration

```toml
[[scroll_band.plugins]]
id = "memory_widget"
path = "target/debug/libsysinfo_widget.so"

[memory_widget]
widget = "Memory"
display_mode = "Bar"
bar_orientation = "Vertical"
show_value = true
show_icon = true
show_used_bytes = true
show_available_bytes = false
icon = "nf-fae-memory"
value_format = "{memory_usage:.0}%"
warning_threshold = 75.0
critical_threshold = 90.0
```

### 6.4 Button Widget Configuration with Dynamic Label

```toml
[[scroll_band.plugins]]
id = "sysinfo_button"
path = "target/debug/libbutton_widget.so"

[sysinfo_button]
icon = "nf-fae-chip"
icon_only = false
label_topic = "service.sysinfo.cpu.status"
label_format = "CPU {cpu_usage:.1}%"
label_fallback = "loading..."

click_topic = "area.open"
click_payload = { area_id = "sysinfo_area" }
```

---

## 7. Roadmap

This roadmap defines the recommended order, dependencies, and deliverables for implementing the Sysinfo feature. The order is chosen so that each layer is built
on top of already-tested foundations.

### Phase 1: Foundation — Model Crate (`model/sysinfo`)

**Goal:** Define all shared messages, topics, and configuration types.

**Order:**

1. Create the crate `model/sysinfo` with a `Cargo.toml` that depends on `serde`, `stabby`, and the project plugin API.
2. Create `src/topics.rs` and declare `TOPIC_CPU`, `TOPIC_MEMORY`, `TOPIC_BATTERY`, `TOPIC_DISKS`, `TOPIC_NETWORK`, `TOPIC_UPTIME`.
3. Create one file per message struct:
    - `src/messages/cpu.rs` → `CpuStatusMessage`
    - `src/messages/memory.rs` → `MemoryStatusMessage`
    - `src/messages/battery.rs` → `BatteryStatus` and `BatteryStatusMessage`
    - `src/messages/disks.rs` → `DiskUsage` and `DisksStatusMessage`
    - `src/messages/network.rs` → `NetworkStatusMessage`
    - `src/messages/uptime.rs` → `UptimeStatusMessage`
4. Add `#[stabby::stabby]` to all FFI-relevant types.
5. Re-export all public types in `src/lib.rs`.
6. Run `cargo check` and `cargo test` for the model crate.

**Exit criteria:**

- The crate compiles without warnings.
- Every public struct and enum has English rustdoc documentation.
- `cargo test` passes with at least serialization/deserialization tests for each message.

---

### Phase 2: Backend — Service Crate (`services/sysinfo`)

**Goal:** Collect system metrics and publish them on the per-metric topics.

**Dependencies:** Phase 1 must be complete.

**Order:**

1. Create the crate `services/sysinfo` with a `Cargo.toml` that depends on the `model/sysinfo` crate and the project plugin API.
2. Create `src/config.rs` with `SysinfoServiceConfig` and its default values.
3. Create `src/collector.rs` and implement collectors in this order:
    - CPU and temperature (`/proc/stat`, `/sys/class/thermal`)
    - Memory (`/proc/meminfo`)
    - Uptime and load average (`/proc/uptime`, `/proc/loadavg`)
    - Battery (`/sys/class/power_supply`)
    - Disks (`statvfs`, `/proc/diskstats`)
    - Network (`/proc/net/dev`)
4. Create `src/service.rs` with `SysinfoService` and all required trait implementations.
5. Implement `run_update_loop` in `src/service.rs` to broadcast each metric on its dedicated topic.
6. Wire `service_plugin!` in `src/lib.rs`.
7. Add unit tests for each collector.

**Exit criteria:**

- The service compiles and loads as a plugin.
- Unit tests for collectors produce plausible values on the current system.
- Running the service broadcasts each topic at least once per interval.

---

### Phase 3: Display — Sysinfo Widget Crate (`plugins/sysinfo`)

**Goal:** Provide dedicated widgets for each metric category.

**Dependencies:** Phase 1 and Phase 2 must be complete.

**Order:**

1. Create the crate `plugins/sysinfo` with a `Cargo.toml` that depends on `model/sysinfo`, the project plugin API, `gtk4`, and `glib`.
2. Create `src/config.rs` with shared types:
    - `DisplayMode` (Bar, Gauge)
    - `BarOrientation` (Horizontal, Vertical)
    - `PercentageWidgetConfig`
    - `DiskDisplayMode` (RootOnly, List)
3. Implement `CpuWidget` in `src/widget_cpu.rs`:
    - Start with `Bar` mode using `gtk4::LevelBar`.
    - Add `Gauge` mode using a `gtk4::DrawingArea` custom draw function.
    - Subscribe to `TOPIC_CPU`.
4. Implement `MemoryWidget` in `src/widget_memory.rs` with the same two modes.
    - Subscribe to `TOPIC_MEMORY`.
5. Implement `BatteryWidget` in `src/widget_battery.rs`.
    - Subscribe to `TOPIC_BATTERY`.
6. Implement `DisksWidget` in `src/widget_disks.rs`.
    - Subscribe to `TOPIC_DISKS`.
7. Implement `NetworkWidget` in `src/widget_network.rs`.
    - Subscribe to `TOPIC_NETWORK`.
8. Implement `UptimeWidget` in `src/widget_uptime.rs`.
    - Subscribe to `TOPIC_UPTIME`.
9. Wire all widgets in `src/lib.rs` using `widget_factory_plugin!`.
10. Add one integration test per widget that verifies the widget accepts its topic.

**Exit criteria:**

- Each widget compiles and can be loaded independently.
- CPU and Memory widgets can be switched between Bar and Gauge modes via configuration.
- Each widget updates its UI when the corresponding topic is broadcasted.

---

### Phase 4: Reusability — Button Widget Extension (`plugins/button`)

**Goal:** Allow the existing Button Widget to display dynamic text from any topic.

**Dependencies:** Phase 1 must be complete.

**Order:**

1. Extend `ButtonConfig` in `plugins/button/src/config.rs` with `label_topic`, `label_format`, and `label_fallback`.
2. Update `AcceptTopic` in `plugins/button/src/widget.rs` to accept the configured `label_topic`.
3. Update `MessageHandler` to process `label_topic` messages and call a label update function.
4. Store a weak reference to the label widget created in `build_widget`.
5. Implement `update_label_from_message` using `glib::MainContext::spawn_local`.
6. Add a formatter that supports simple JSON interpolation and fallback text.
7. Add a unit test for the formatter.

**Exit criteria:**

- A button configured with `label_topic = "service.sysinfo.cpu.status"` updates its label when the topic is broadcasted.
- A button without `label_topic` still behaves exactly as before.

---

### Phase 5: Wiring — Configuration and Registration

**Goal:** Connect all new crates to the main application.

**Dependencies:** Phase 2, Phase 3, and Phase 4 must be complete.

**Order:**

1. Add the `model/sysinfo` and `services/sysinfo` crates to the workspace `Cargo.toml`.
2. Register the service in `services.toml`.
3. Add a sample configuration block for `sysinfo` in `config.toml`.
4. Add sample widget configurations for CPU, Memory, Battery, Disks, Network, and Uptime widgets.
5. Add a sample button configuration that uses `label_topic`.

**Exit criteria:**

- The workspace compiles with `cargo build`.
- The service is loaded at application startup.
- All configured widgets receive messages and render correctly.

---

### Phase 6: Validation — Integration and Tests

**Goal:** Verify end-to-end behavior and stability.

**Dependencies:** Phase 5 must be complete.

**Order:**

1. Run the application and verify that each topic appears on the message broker.
2. Verify CPU and Memory widgets in both Bar and Gauge modes.
3. Verify that the Battery, Disks, Network, and Uptime widgets display reasonable data.
4. Verify the Button Widget label updates dynamically.
5. Run `cargo test` for all four crates.
6. Run `cargo clippy` and `cargo fmt` and fix any issues.
7. Measure resource usage of the service to confirm the loop is lightweight.

**Exit criteria:**

- All tests pass.
- All widgets render correctly.
- No `unwrap`, `expect`, or `panic` remains in the new code.
- `rustfmt` and `clippy` are clean.

---

### Summary of Order

```
Phase 1: model/sysinfo
    |
    v
Phase 2: services/sysinfo
    |
    v
Phase 3: plugins/sysinfo
    |
    v
Phase 4: plugins/button
    |
    v
Phase 5: workspace wiring and config
    |
    v
Phase 6: integration and tests
```

### Rationale

- **Model first:** Message formats must exist before the service or widgets can use them.
- **Service second:** Widgets need a running publisher to test against.
- **Widgets third:** Display widgets depend on the service topics.
- **Button fourth:** The button extension only needs the topic infrastructure, so it can be built after the model crate.
- **Wiring fifth:** Final integration only makes sense when all components are ready.
- **Tests last:** End-to-end validation closes the loop.

---

## 8. Technical Notes

- **No polling in the widget:** Widgets update exclusively through incoming messages. Regular polling only happens in the service.
- **Fault tolerance:** If a collector fails (e.g., no battery present), the corresponding metric is set to a sensible default or `None` and the remaining topics
  are still broadcasted.
- **Performance:** `/proc/stat`, `/proc/meminfo`, `/proc/diskstats`, `/proc/net/dev`, `/proc/uptime`, and `/proc/loadavg` are virtual files in RAM; reading them
  is fast and non-blocking.
- **Throughput calculation:** Disk and network throughput require keeping the previous sample to compute a delta.
- **Per-metric topics:** This design allows widgets to subscribe only to data they actually display, reducing unnecessary message traffic and CPU load in the
  broker.

---

## 9. Compliance with `AGENTS.md`

The proposed implementation follows the project guidelines in `AGENTS.md`:

- **Crate separation:** The feature is split into `model/sysinfo`, `services/sysinfo`, `plugins/sysinfo`, and `plugins/button`.
- **One struct per file:** Each widget and each message struct lives in its own file.
- **Service traits:** The service implements `MessageHandler`, `MessageBroadcaster`, `MessageTopicBroadcaster`, `PluginMetaGetter`, and
  `AsRef<Option<FfiCoreContext>>`.
- **Widget traits:** Each widget implements `MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>`, and `WidgetBuilder`.
- **Async runtime:** The service uses `tokio::sync::mpsc` and spawns async tasks via the `PluginExecutor`.
- **GTK updates:** Widgets use `glib::MainContext::spawn_local` for GTK updates and `tokio::sync::mpsc` for message reception.
- **Event-driven:** Widgets are updated by incoming messages, not by polling loops.
- **FFI stability:** All FFI-relevant types in the model carry `#[stabby::stabby]`.
- **No panic:** The implementation uses `Result` and `Option` for error handling; no `unwrap()`, `expect()`, or `panic!`.
- **Naming:** All names are descriptive and follow Rust naming conventions.
- **Documentation:** All public structs, enums, and fields are documented in English.
- **Formatting:** Code is formatted with `rustfmt` and checked with `clippy`.
- **Dependencies:** The model uses `serde` and `stabby`; the service uses `tokio` and `tracing`; the widgets use `gtk4` and `glib`.

---

*End of document.*
