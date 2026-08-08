# Concept: Pet Service & Widget

This document describes the concept for a **Pet Service** and a **Pet Widget** in the *Smearor Swipe Launcher*. The pet is an omniscient, demanding virtual
companion — similar to a Tamagotchi — that reacts to system conditions and interacts with the user. When something does not meet its high standards, it becomes
unhappy, complains, and holds up protest signs.

The system follows the decoupled SOA architecture:

1. **Model Crate (`model/pet`):** Shared structs, enums, topics, and message formats.
2. **Service Crate (`services/pet`):** Singleton background service that subscribes to sysinfo and network topics, evaluates the pet's mood, and broadcasts pet
   status updates.
3. **Widget Crate (`plugins/pet`):** Pure GTK4 UI that subscribes to pet status and renders the pet character with expressions, speech bubbles, and protest
   signs.

---

## 1. System Architecture & Data Flow

```
+--------------------------+          +--------------------------+
| Sysinfo Service          |          | Network Service          |
| (Singleton)              |          | (Singleton)              |
|                          |          |                          |
| TOPIC_CPU                |          | TOPIC_STATUS             |
| TOPIC_MEMORY             |          |   (NetworkStatusMessage) |
| TOPIC_DISKS              |          |                          |
+--------------------------+          +--------------------------+
         |                                     |
         |  sysinfo status messages            |  network status messages
         v                                     v
+----------------------------------------------------------+
| Pet Service (Singleton)                                  |
|                                                          |
|  +-----------+    +-----------+    +-----------+         |
|  | Mood      |    | Complaint |    | Protest   |         |
|  | Engine    |--->| Generator |--->| Sign      |         |
|  +-----------+    +-----------+    +-----------+         |
|       ^                  |               |               |
|       | thresholds       |               |               |
|       | config           |               |               |
+-------|------------------|---------------|---------------+
        |                  |               |
        |  TOPIC_PET_STATUS|               |
        |  (PetStatusMessage)              |
        v                                  v
+----------------------------------------------------------+
| Pet Widget                                               |
| (subscribed to service.pet.status)                       |
|                                                          |
|  +----------+   +----------+   +----------+             |
|  | Pet      |   | Speech   |   | Protest  |             |
|  | Sprite   |   | Bubble   |   | Sign     |             |
|  +----------+   +----------+   +----------+             |
+----------------------------------------------------------+
```

The Pet Service acts as an aggregator: it subscribes to existing sysinfo and network topics, evaluates all conditions against configurable thresholds, computes
a mood level, generates complaints, and broadcasts a unified `PetStatusMessage`. The Pet Widget renders the pet based on this single message.

---

## 2. Crate Structure

Following the workspace conventions (`AGENTS.md`), the feature is split into three crates:

| Crate       | Path            | Responsibility                                                           |
|-------------|-----------------|--------------------------------------------------------------------------|
| **Model**   | `model/pet/`    | Shared structs, enums, topics, and message formats                       |
| **Service** | `services/pet/` | Mood evaluation, complaint generation, protest sign selection, broadcast |
| **Widget**  | `plugins/pet/`  | GTK4 user interface, pet sprite, speech bubbles, protest signs           |

---

## 3. Pet Triggers

The pet reacts to the following system conditions. Each trigger has a configurable threshold and a corresponding complaint message.

| Trigger                | Data Source                            | Topic                             | Condition                      | Complaint Example                      |
|------------------------|----------------------------------------|-----------------------------------|--------------------------------|----------------------------------------|
| High CPU usage         | `CpuStatusMessage.cpu_usage`           | `service.sysinfo.cpu.status`      | `cpu_usage > threshold`        | "My brain is overheating!"             |
| High GPU usage         | `GpuStatusMessage.gpu_usage`           | `service.sysinfo.gpu.status`      | `gpu_usage > threshold`        | "The graphics card is burning!"        |
| Low memory             | `MemoryStatusMessage.memory_available` | `service.sysinfo.memory.status`   | `memory_available < threshold` | "I'm suffocating in here!"             |
| Low disk free          | `DisksStatusMessage.mounts`            | `service.sysinfo.disks.status`    | `available < threshold`        | "I'm running out of room!"             |
| No internet connection | `NetworkStatusMessage`                 | `service.network.status`          | No connected interface         | "I feel so isolated without internet!" |
| Pending updates        | `PackageUpdateMessage`                 | `service.sysinfo.packages.status` | `pending_count > threshold`    | "So many updates! You never clean up!" |

### 3.1 New Sysinfo Extensions

Two new metric categories are required that do not yet exist in the sysinfo service:

#### GPU Metrics

A new `GpuStatusMessage` and `TOPIC_GPU` are added to `model/sysinfo`. The sysinfo service collects GPU usage and temperature via one of the following methods,
depending on availability:

| Method          | Data Source                                                                            | GPU Vendor |
|-----------------|----------------------------------------------------------------------------------------|------------|
| `nvidia-smi`    | `nvidia-smi --query-gpu=utilization.gpu,temperature.gpu --format=csv,noheader,nounits` | NVIDIA     |
| `rocm-smi`      | `rocm-smi --showuse --showtemp`                                                        | AMD        |
| `intel_gpu_top` | `intel_gpu_top -J`                                                                     | Intel      |

If no GPU monitoring tool is available, GPU metrics are reported as `None` and the pet ignores GPU-related triggers.

#### Package Update Metrics

A new `PackageUpdateMessage` and `TOPIC_PACKAGES` are added to `model/sysinfo`. The sysinfo service counts pending package updates by invoking the
distribution's package manager:

| Distribution  | Command                                           |
|---------------|---------------------------------------------------|
| Arch Linux    | `checkupdates` (from `pacman-contrib`)            |
| Debian/Ubuntu | `apt list --upgradable` (filtered for upgradable) |
| Fedora        | `dnf check-update --quiet`                        |

The count is refreshed at a slower interval (default: 30 minutes) to avoid excessive subprocess spawning.

---

## 4. Model Crate (`model/pet`)

### 4.1 Message Topics

```rust
/// Topic for pet status broadcasts.
pub const TOPIC_PET_STATUS: &str = "service.pet.status";

/// Topic for pet commands (e.g., acknowledge, pet, feed).
pub const TOPIC_PET_COMMAND: &str = "service.pet.command";
```

### 4.2 Mood Enum

The pet has a discrete mood level that drives its visual expression and behavior.

```rust
/// The current mood of the pet.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum PetMood {
    /// The pet is happy and content.
    #[default]
    Happy,
    /// The pet is slightly annoyed but not yet protesting.
    Annoyed,
    /// The pet is unhappy and holding up a protest sign.
    Unhappy,
    /// The pet is furious and holding up multiple protest signs.
    Furious,
}
```

### 4.3 Pet Trigger Enum

Each active trigger is represented as an enum variant so the widget knows exactly what the pet is complaining about.

```rust
/// A single trigger that causes pet dissatisfaction.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum PetTrigger {
    /// CPU usage exceeds the configured threshold.
    HighCpuUsage,
    /// GPU usage exceeds the configured threshold.
    HighGpuUsage,
    /// Available memory falls below the configured threshold.
    LowMemory,
    /// Available disk space falls below the configured threshold.
    LowDiskFree,
    /// No network interface is connected.
    NoInternetConnection,
    /// Pending package updates exceed the configured threshold.
    PendingUpdates,
}
```

### 4.4 Pet Status Message

```rust
/// Status message broadcast by the pet service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct PetStatusMessage {
    /// Current mood of the pet.
    pub mood: PetMood,
    /// All currently active triggers.
    pub active_triggers: stabby::vec::Vec<PetTrigger>,
    /// The complaint text the pet is currently expressing.
    pub complaint: stabby::string::String,
    /// The protest sign text the pet is holding (empty if none).
    pub protest_sign: stabby::string::String,
    /// Whether the pet is currently interacting with the user.
    pub is_interacting: bool,
}
```

### 4.5 Pet Command Message

```rust
/// Actions the user can perform on the pet.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum PetCommandAction {
    /// Acknowledge the pet's complaint (temporarily calms it down).
    #[default]
    Acknowledge,
    /// Pet the pet (increases happiness temporarily).
    Pet,
    /// Feed the pet (increases happiness temporarily).
    Feed,
}

/// Command message sent to the pet service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct PetCommandMessage {
    /// The action to perform.
    pub action: PetCommandAction,
}
```

### 4.6 File Structure

| File                      | Responsibility                               |
|---------------------------|----------------------------------------------|
| `src/lib.rs`              | Module declarations and `pub use` re-exports |
| `src/topics.rs`           | Topic constants                              |
| `src/messages/mood.rs`    | `PetMood` enum                               |
| `src/messages/trigger.rs` | `PetTrigger` enum                            |
| `src/messages/status.rs`  | `PetStatusMessage`                           |
| `src/messages/command.rs` | `PetCommandAction` and `PetCommandMessage`   |
| `src/messages/mod.rs`     | Module declarations for messages             |

---

## 5. Service Crate (`services/pet`)

### 5.1 File Structure

| File                            | Responsibility                                |
|---------------------------------|-----------------------------------------------|
| `src/lib.rs`                    | `service_plugin!` macro invocation            |
| `src/config.rs`                 | `PetServiceConfig` struct and parsing         |
| `src/mood.rs`                   | Mood evaluation logic                         |
| `src/complaint.rs`              | Complaint and protest sign text generation    |
| `src/service/loaded_service.rs` | `PetService` struct and trait implementations |

### 5.2 Configuration

```rust
/// Configuration for the pet service.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PetServiceConfig {
    /// CPU usage percentage above which the pet complains.
    pub cpu_usage_threshold: f32,
    /// GPU usage percentage above which the pet complains.
    pub gpu_usage_threshold: f32,
    /// Available memory in bytes below which the pet complains.
    pub memory_available_threshold: u64,
    /// Available disk space in bytes below which the pet complains.
    pub disk_available_threshold: u64,
    /// Number of pending updates above which the pet complains.
    pub pending_updates_threshold: u32,
    /// Duration in seconds for which an acknowledge command calms the pet.
    pub acknowledge_duration_seconds: u64,
    /// Duration in seconds for which a pet or feed command boosts happiness.
    pub interaction_boost_seconds: u64,
    /// Whether GPU monitoring is enabled.
    pub enable_gpu_monitoring: bool,
    /// Whether package update monitoring is enabled.
    pub enable_package_monitoring: bool,
}

impl Default for PetServiceConfig {
    fn default() -> Self {
        Self {
            cpu_usage_threshold: 85.0,
            gpu_usage_threshold: 90.0,
            memory_available_threshold: 512 * 1024 * 1024, // 512 MB
            disk_available_threshold: 5 * 1024 * 1024 * 1024, // 5 GB
            pending_updates_threshold: 50,
            acknowledge_duration_seconds: 60,
            interaction_boost_seconds: 30,
            enable_gpu_monitoring: true,
            enable_package_monitoring: true,
        }
    }
}
```

### 5.3 Service Implementation

```rust
pub struct PetService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: PetServiceConfig,
    pub current_status: Arc<RwLock<PetStatusMessage>>,
    /// Cached CPU status from the last sysinfo broadcast.
    pub cpu_status: Arc<RwLock<Option<CpuStatusMessage>>>,
    /// Cached GPU status from the last sysinfo broadcast.
    pub gpu_status: Arc<RwLock<Option<GpuStatusMessage>>>,
    /// Cached memory status from the last sysinfo broadcast.
    pub memory_status: Arc<RwLock<Option<MemoryStatusMessage>>>,
    /// Cached disk status from the last sysinfo broadcast.
    pub disk_status: Arc<RwLock<Option<DisksStatusMessage>>>,
    /// Cached network status from the last network service broadcast.
    pub network_status: Arc<RwLock<Option<NetworkStatusMessage>>>,
    /// Cached package update status from the last sysinfo broadcast.
    pub package_status: Arc<RwLock<Option<PackageUpdateMessage>>>,
    /// Timestamp until which the pet is calmed by an acknowledge command.
    pub acknowledge_until: Arc<RwLock<Option<Instant>>>,
    /// Timestamp until which the pet is boosted by a pet or feed command.
    pub interaction_boost_until: Arc<RwLock<Option<Instant>>>,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<PetCommandMessage>>` - Processes user commands (acknowledge, pet, feed)
- `MessageHandler<FfiEnvelopePayload<CpuStatusMessage>>` - Receives CPU metrics
- `MessageHandler<FfiEnvelopePayload<GpuStatusMessage>>` - Receives GPU metrics
- `MessageHandler<FfiEnvelopePayload<MemoryStatusMessage>>` - Receives memory metrics
- `MessageHandler<FfiEnvelopePayload<DisksStatusMessage>>` - Receives disk metrics
- `MessageHandler<FfiEnvelopePayload<NetworkStatusMessage>>` - Receives network status
- `MessageHandler<FfiEnvelopePayload<PackageUpdateMessage>>` - Receives package update counts
- `MessageBroadcaster` - Broadcasts messages to the broker
- `MessageTopicBroadcaster` - Broadcasts to specific topic subscribers
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context

### 5.4 Mood Evaluation

The mood engine evaluates all cached system metrics against the configured thresholds and computes a `PetMood` and a list of active triggers.

```rust
fn evaluate_mood(&self) -> (PetMood, Vec<PetTrigger>) {
    let now = Instant::now();
    let mut triggers = Vec::new();

    // Check CPU usage
    if let Some(cpu) = self.cpu_status.read().clone() {
        if cpu.cpu_usage > self.config.cpu_usage_threshold {
            triggers.push(PetTrigger::HighCpuUsage);
        }
    }

    // Check GPU usage
    if self.config.enable_gpu_monitoring {
        if let Some(gpu) = self.gpu_status.read().clone() {
            if gpu.gpu_usage > self.config.gpu_usage_threshold {
                triggers.push(PetTrigger::HighGpuUsage);
            }
        }
    }

    // Check memory
    if let Some(memory) = self.memory_status.read().clone() {
        if memory.memory_available < self.config.memory_available_threshold {
            triggers.push(PetTrigger::LowMemory);
        }
    }

    // Check disk free
    if let Some(disks) = self.disk_status.read().clone() {
        for mount in &disks.mounts {
            if mount.available < self.config.disk_available_threshold {
                triggers.push(PetTrigger::LowDiskFree);
                break;
            }
        }
    }

    // Check internet connection
    if let Some(network) = self.network_status.read().clone() {
        let has_connected = network
            .primary_interface
            .connection_state
            == NetworkConnectionState::Connected;
        if !has_connected {
            triggers.push(PetTrigger::NoInternetConnection);
        }
    }

    // Check pending updates
    if self.config.enable_package_monitoring {
        if let Some(packages) = self.package_status.read().clone() {
            if packages.pending_count > self.config.pending_updates_threshold {
                triggers.push(PetTrigger::PendingUpdates);
            }
        }
    }

    // Determine mood based on trigger count and acknowledge/boost timers
    let is_acknowledged = self
        .acknowledge_until
        .read()
        .map_or(false, |until| now < until);

    let is_boosted = self
        .interaction_boost_until
        .read()
        .map_or(false, |until| now < until);

    let effective_trigger_count = if is_acknowledged || is_boosted {
        triggers.len().saturating_sub(1)
    } else {
        triggers.len()
    };

    let mood = match effective_trigger_count {
        0 => PetMood::Happy,
        1 => PetMood::Annoyed,
        2 => PetMood::Unhappy,
        _ => PetMood::Furious,
    };

    (mood, triggers)
}
```

### 5.5 Complaint Generation

Each trigger has a pool of complaint messages. The service selects one randomly to keep the pet's behavior varied and engaging.

```rust
fn generate_complaint(triggers: &[PetTrigger]) -> String {
    if triggers.is_empty() {
        return String::from("Everything is fine!");
    }

    let complaints: &[(&PetTrigger, &[&str])] = &[
        (&PetTrigger::HighCpuUsage, &[
            "My brain is overheating!",
            "Too much thinking! I need a break!",
            "The CPU is on fire! Put me out!",
        ]),
        (&PetTrigger::HighGpuUsage, &[
            "The graphics card is burning!",
            "My eyes hurt from all these pixels!",
            "GPU overload! I can't see straight!",
        ]),
        (&PetTrigger::LowMemory, &[
            "I'm suffocating in here!",
            "So cramped! I need more memory!",
            "I can't breathe! Free some RAM!",
        ]),
        (&PetTrigger::LowDiskFree, &[
            "I'm running out of room!",
            "Clean up your files! I'm claustrophobic!",
            "No space left! I feel trapped!",
        ]),
        (&PetTrigger::NoInternetConnection, &[
            "I feel so isolated without internet!",
            "No connection? How am I supposed to know anything?",
            "I'm disconnected from the world!",
        ]),
        (&PetTrigger::PendingUpdates, &[
            "So many updates! You never clean up!",
            "Pending packages are piling up! Do something!",
            "Update me already! This is embarrassing!",
        ]),
    ];

    // Pick a random complaint for the first active trigger
    let trigger = triggers[0];
    for (t, pool) in complaints {
        if **t == trigger {
            let index = rand::random::<usize>() % pool.len();
            return pool[index].to_string();
        }
    }

    String::from("Something is wrong!")
}
```

### 5.6 Protest Sign Generation

When the pet is `Unhappy` or `Furious`, it holds up a protest sign with a short, punchy text.

```rust
fn generate_protest_sign(mood: PetMood, triggers: &[PetTrigger]) -> String {
    if mood == PetMood::Happy || mood == PetMood::Annoyed {
        return String::new();
    }

    let signs: &[(&PetTrigger, &[&str])] = &[
        (&PetTrigger::HighCpuUsage, &["TOO HOT", "CPU = BAD", "SLOW DOWN"]),
        (&PetTrigger::HighGpuUsage, &["GPU ON FIRE", "PIXELS HURT", "STOP RENDERING"]),
        (&PetTrigger::LowMemory, &["MORE RAM!", "I'M SUFFOCATING", "FREE ME"]),
        (&PetTrigger::LowDiskFree, &["CLEAN UP!", "NO SPACE", "DELETE STUFF"]),
        (&PetTrigger::NoInternetConnection, &["I WANT INTERNET", "DISCONNECTED", "NO WEB = SAD"]),
        (&PetTrigger::PendingUpdates, &["UPDATE ME!", "TOO MANY UPDATES", "CLEAN UP PKGS"]),
    ];

    let trigger = triggers[0];
    for (t, pool) in signs {
        if **t == trigger {
            let index = rand::random::<usize>() % pool.len();
            return pool[index].to_string();
        }
    }

    String::from("PROTEST!")
}
```

### 5.7 Background Update Loop

The service spawns an asynchronous Tokio task that periodically re-evaluates the mood and broadcasts a new `PetStatusMessage`. This is necessary because the
acknowledge and interaction boost timers expire over time, even if no new sysinfo messages arrive.

```rust
async fn run_update_loop(
    config: PetServiceConfig,
    state: PetState,
    broadcaster: Box<dyn MessageTopicBroadcaster>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        let (mood, triggers) = Self::evaluate_mood(&state, &config).await;
        let complaint = Self::generate_complaint(&triggers);
        let protest_sign = Self::generate_protest_sign(mood, &triggers);

        let status = PetStatusMessage {
            mood,
            active_triggers: triggers.into_iter().collect(),
            complaint: complaint.into(),
            protest_sign: protest_sign.into(),
            is_interacting: false,
        };

        *state.current_status.write().await = status.clone();
        broadcaster.broadcast_topic(TOPIC_PET_STATUS, status);
    }
}
```

### 5.8 Command Handling

When the user sends a command, the service updates the acknowledge or interaction boost timer. The next evaluation cycle will reflect the new mood.

```rust
async fn handle_command(&self, action: PetCommandAction) {
    let now = Instant::now();
    match action {
        PetCommandAction::Acknowledge => {
            *self.acknowledge_until.write().await =
                Some(now + Duration::from_secs(self.config.acknowledge_duration_seconds));
        }
        PetCommandAction::Pet | PetCommandAction::Feed => {
            *self.interaction_boost_until.write().await =
                Some(now + Duration::from_secs(self.config.interaction_boost_seconds));
        }
    }
}
```

---

## 6. Widget Crate (`plugins/pet`)

### 6.1 Overview

The Pet Widget subscribes to `service.pet.status` and renders the pet as an animated character. The visual representation changes based on the pet's mood:

| Mood    | Expression         | Protest Sign | Speech Bubble         |
|---------|--------------------|--------------|-----------------------|
| Happy   | Smiling, relaxed   | None         | "Everything is fine!" |
| Annoyed | Frowning, tapping  | None         | Complaint text        |
| Unhappy | Angry, one sign    | One sign     | Complaint text        |
| Furious | Furious, two signs | Two signs    | Complaint text        |

### 6.2 File Structure

| File            | Responsibility                                  |
|-----------------|-------------------------------------------------|
| `src/lib.rs`    | `widget_plugin!` macro invocation               |
| `src/config.rs` | `PetWidgetConfig` struct and parsing            |
| `src/widget.rs` | `PetWidget` struct and trait implementations    |
| `src/sprite.rs` | Pet sprite rendering (CSS-based or image-based) |
| `src/bubble.rs` | Speech bubble rendering                         |
| `src/sign.rs`   | Protest sign rendering                          |

### 6.3 Configuration

```rust
/// Configuration for the pet widget.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PetWidgetConfig {
    /// Width of the widget in pixels.
    pub width: i32,
    /// Height of the widget in pixels.
    pub height: i32,
    /// Whether to show the speech bubble.
    pub show_speech_bubble: bool,
    /// Whether to show the protest sign.
    pub show_protest_sign: bool,
    /// Whether to show the mood as a text label.
    pub show_mood_label: bool,
    /// Topic to publish when the pet is clicked.
    pub click_topic: Option<String>,
    /// Payload to publish when the pet is clicked.
    pub click_payload: Option<serde_json::Value>,
    /// Topic to publish when the pet is long-pressed.
    pub longpress_topic: Option<String>,
    /// Payload to publish when the pet is long-pressed.
    pub longpress_payload: Option<serde_json::Value>,
}

impl Default for PetWidgetConfig {
    fn default() -> Self {
        Self {
            width: 80,
            height: 80,
            show_speech_bubble: true,
            show_protest_sign: true,
            show_mood_label: false,
            click_topic: None,
            click_payload: None,
            longpress_topic: None,
            longpress_payload: None,
        }
    }
}
```

### 6.4 Widget Implementation

```rust
pub struct PetWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: PetWidgetConfig,
    pub container: Arc<RwLock<Option<gtk4::Box>>>,
    pub sprite_widget: Arc<RwLock<Option<gtk4::Box>>>,
    pub bubble_widget: Arc<RwLock<Option<gtk4::Label>>>,
    pub sign_widget: Arc<RwLock<Option<gtk4::Label>>>,
    pub mood_label: Arc<RwLock<Option<gtk4::Label>>>,
    pub current_status: Arc<RwLock<PetStatusMessage>>,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<PetStatusMessage>>` - Receives pet status updates
- `MessageBroadcaster` - Publishes click and long-press commands
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `WidgetBuilder` - Builds the GTK widget tree

### 6.5 Rendering

The pet sprite is rendered using CSS classes applied to a `gtk4::Box` container. The CSS classes change based on the mood:

```css
.pet-sprite {
    background-size: contain;
    background-repeat: no-repeat;
    background-position: center;
    transition: all 0.3s ease;
}

.pet-sprite-happy {
    /* Happy pet sprite */
}

.pet-sprite-annoyed {
    /* Annoyed pet sprite */
}

.pet-sprite-unhappy {
    /* Unhappy pet sprite with animation */
    animation: shake 0.5s infinite;
}

.pet-sprite-furious {
    /* Furious pet sprite with intense animation */
    animation: rage 0.3s infinite;
}
```

The speech bubble is a `gtk4::Label` with a CSS class `pet-speech-bubble` that displays the complaint text. The protest sign is a `gtk4::Label` with a CSS class
`pet-protest-sign` that displays the protest sign text, styled to look like a hand-held cardboard sign.

### 6.6 Interaction

- **Click (short tap):** Publishes `click_topic` / `click_payload` (default: `service.pet.command` with `PetCommandAction::Pet`). This gives the user a way to
  interact with the pet and temporarily boost its happiness.
- **Long press:** Publishes `longpress_topic` / `longpress_payload` (default: `service.pet.command` with `PetCommandAction::Feed`).

### 6.7 UI Updates

All UI updates happen in the GTK main thread via `glib::MainContext::spawn_local`. When a `PetStatusMessage` is received, the widget updates the sprite CSS
class, speech bubble text, and protest sign visibility.

```rust
fn render_status(&self, status: &PetStatusMessage) {
    let sprite_weak = self.sprite_widget.clone();
    let bubble_weak = self.bubble_widget.clone();
    let sign_weak = self.sign_widget.clone();
    let mood = status.mood;
    let complaint = status.complaint.to_string();
    let protest_sign = status.protest_sign.to_string();

    glib::MainContext::default().spawn_local(async move {
        if let Some(sprite) = sprite_weak.write().await.as_ref() {
            sprite.remove_css_class("pet-sprite-happy");
            sprite.remove_css_class("pet-sprite-annoyed");
            sprite.remove_css_class("pet-sprite-unhappy");
            sprite.remove_css_class("pet-sprite-furious");
            sprite.add_css_class(match mood {
                PetMood::Happy => "pet-sprite-happy",
                PetMood::Annoyed => "pet-sprite-annoyed",
                PetMood::Unhappy => "pet-sprite-unhappy",
                PetMood::Furious => "pet-sprite-furious",
            });
        }

        if let Some(bubble) = bubble_weak.write().await.as_ref() {
            bubble.set_text(&complaint);
        }

        if let Some(sign) = sign_weak.write().await.as_ref() {
            if protest_sign.is_empty() {
                sign.set_visible(false);
            } else {
                sign.set_text(&protest_sign);
                sign.set_visible(true);
            }
        }
    });
}
```

---

## 7. Sysinfo Model Extensions

### 7.1 GPU Status Message

Added to `model/sysinfo/src/messages/gpu.rs`:

```rust
/// Status message for GPU metrics.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct GpuStatusMessage {
    /// GPU usage in percent (0.0 - 100.0).
    pub gpu_usage: f32,
    /// GPU temperature in degrees Celsius.
    pub gpu_temperature: stabby::option::Option<f32>,
    /// Name of the GPU device.
    pub gpu_name: stabby::string::String,
}
```

New topic in `model/sysinfo/src/topics.rs`:

```rust
/// Topic for GPU metrics, including usage and temperature.
pub const TOPIC_GPU: &str = "service.sysinfo.gpu.status";
```

### 7.2 Package Update Message

Added to `model/sysinfo/src/messages/packages.rs`:

```rust
/// Status message for pending package updates.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct PackageUpdateMessage {
    /// Number of pending updates.
    pub pending_count: u32,
    /// Names of the pending packages (up to a configured maximum).
    pub pending_packages: stabby::vec::Vec<stabby::string::String>,
    /// Name of the package manager detected on the system.
    pub package_manager: stabby::string::String,
}
```

New topic in `model/sysinfo/src/topics.rs`:

```rust
/// Topic for pending package update metrics.
pub const TOPIC_PACKAGES: &str = "service.sysinfo.packages.status";
```

---

## 8. Configuration Example

### 8.1 Service Configuration

```toml
[services]
load = ["sysinfo", "network", "pet"]

[pet]
cpu_usage_threshold = 85.0
gpu_usage_threshold = 90.0
memory_available_threshold = 536870912
disk_available_threshold = 5368709120
pending_updates_threshold = 50
acknowledge_duration_seconds = 60
interaction_boost_seconds = 30
enable_gpu_monitoring = true
enable_package_monitoring = true
```

### 8.2 Widget Configuration

```toml
[[scroll_band.plugins]]
id = "pet_widget"
path = "target/debug/libpet_widget.so"

[pet_widget]
width = 80
height = 80
show_speech_bubble = true
show_protest_sign = true
show_mood_label = false
click_topic = "service.pet.command"
click_payload = { action = "Pet" }
longpress_topic = "service.pet.command"
longpress_payload = { action = "Feed" }
```

---

## 9. Roadmap

This roadmap defines the recommended order, dependencies, and deliverables for implementing the Pet feature. The order is chosen so that each layer is built on
top of already-tested foundations.

### Phase 1: Foundation — Sysinfo Model Extensions (`model/sysinfo`)

**Goal:** Add GPU and package update message types to the existing sysinfo model.

**Dependencies:** None (extends existing `model/sysinfo` crate).

**Order:**

1. Add `TOPIC_GPU` and `TOPIC_PACKAGES` to `model/sysinfo/src/topics.rs`.
2. Create `src/messages/gpu.rs` with `GpuStatusMessage`.
3. Create `src/messages/packages.rs` with `PackageUpdateMessage`.
4. Add `#[stabby::stabby]` to all new FFI-relevant types.
5. Implement `TypedMessage`, `MessageTopic`, and `SharedMessage` for both new messages.
6. Re-export the new types in `src/lib.rs`.
7. Run `cargo check` and `cargo test` for the model crate.

**Exit criteria:**

- The crate compiles without warnings.
- Every new public struct and enum has English rustdoc documentation.
- `cargo test` passes with serialization/deserialization tests for both new messages.

---

### Phase 2: Backend — Sysinfo Service Extensions (`services/sysinfo`)

**Goal:** Collect GPU metrics and pending package update counts, and broadcast them on the new topics.

**Dependencies:** Phase 1 must be complete.

**Order:**

1. Add GPU collector to `services/sysinfo/src/collector.rs`:
    - Detect available GPU monitoring tool (`nvidia-smi`, `rocm-smi`, `intel_gpu_top`).
    - Parse output and produce `GpuStatusMessage`.
    - Return `None` fields if no tool is available.
2. Add package update collector to `services/sysinfo/src/collector.rs`:
    - Detect distribution's package manager.
    - Run the appropriate command and parse the pending count.
    - Produce `PackageUpdateMessage`.
3. Extend `SysinfoServiceConfig` with `enable_gpu` and `enable_packages` flags.
4. Extend the update loop in `services/sysinfo/src/service/loaded_service.rs` to collect and broadcast GPU and package metrics.
5. Add unit tests for both new collectors with mock command outputs.

**Exit criteria:**

- The service compiles and broadcasts `TOPIC_GPU` and `TOPIC_PACKAGES` at the configured intervals.
- GPU collector returns `None` fields gracefully when no GPU tool is available.
- Package update collector returns `pending_count = 0` when no package manager is detected.
- Unit tests pass with mock outputs.

---

### Phase 3: Foundation — Pet Model Crate (`model/pet`)

**Goal:** Define all shared messages, topics, and configuration types for the pet feature.

**Dependencies:** Phase 1 must be complete (pet service needs to reference sysinfo model types).

**Order:**

1. Create the crate `model/pet` with a `Cargo.toml` that depends on `serde`, `stabby`, and the project plugin API.
2. Create `src/topics.rs` and declare `TOPIC_PET_STATUS` and `TOPIC_PET_COMMAND`.
3. Create one file per type:
    - `src/messages/mood.rs` -> `PetMood` enum
    - `src/messages/trigger.rs` -> `PetTrigger` enum
    - `src/messages/status.rs` -> `PetStatusMessage`
    - `src/messages/command.rs` -> `PetCommandAction` and `PetCommandMessage`
4. Add `#[stabby::stabby]` to all FFI-relevant types.
5. Implement `TypedMessage`, `MessageTopic`, and `SharedMessage` for `PetStatusMessage` and `PetCommandMessage`.
6. Re-export all public types in `src/lib.rs`.
7. Run `cargo check` and `cargo test` for the model crate.

**Exit criteria:**

- The crate compiles without warnings.
- Every public struct and enum has English rustdoc documentation.
- `cargo test` passes with serialization/deserialization tests for each message.

---

### Phase 4: Backend — Pet Service Crate (`services/pet`)

**Goal:** Evaluate system conditions, compute pet mood, generate complaints, and broadcast pet status.

**Dependencies:** Phase 1, Phase 2, and Phase 3 must be complete.

**Order:**

1. Create the crate `services/pet` with a `Cargo.toml` that depends on `model/pet`, `model/sysinfo`, `model/network`, the project plugin API, `tokio`, and
   `tracing`.
2. Create `src/config.rs` with `PetServiceConfig` and its default values.
3. Create `src/mood.rs` and implement the mood evaluation logic.
4. Create `src/complaint.rs` and implement complaint and protest sign text generation.
5. Create `src/service/loaded_service.rs` with `PetService` and all required trait implementations.
6. Implement message handlers for all subscribed topics (CPU, GPU, memory, disks, network, packages).
7. Implement command handling for `PetCommandMessage`.
8. Implement `run_update_loop` to periodically re-evaluate and broadcast `PetStatusMessage`.
9. Wire `service_plugin!` in `src/lib.rs`.
10. Add unit tests for mood evaluation and complaint generation.

**Exit criteria:**

- The service compiles and loads as a plugin.
- The service subscribes to all sysinfo and network topics and caches incoming messages.
- Mood evaluation produces correct `PetMood` values for given trigger combinations.
- Complaint generation produces non-empty strings for each trigger.
- Acknowledge and interaction boost timers correctly reduce the effective trigger count.
- Running the service broadcasts `TOPIC_PET_STATUS` at least every 5 seconds.

---

### Phase 5: Display — Pet Widget Crate (`plugins/pet`)

**Goal:** Provide an animated pet character that reacts to status messages.

**Dependencies:** Phase 3 and Phase 4 must be complete.

**Order:**

1. Create the crate `plugins/pet` with a `Cargo.toml` that depends on `model/pet`, the project plugin API, `gtk4`, and `glib`.
2. Create `src/config.rs` with `PetWidgetConfig`.
3. Create `src/sprite.rs` and implement the pet sprite rendering with mood-based CSS classes.
4. Create `src/bubble.rs` and implement the speech bubble rendering.
5. Create `src/sign.rs` and implement the protest sign rendering.
6. Create `src/widget.rs` with `PetWidget` and all required trait implementations.
7. Subscribe to `TOPIC_PET_STATUS` and update the UI on every message.
8. Implement click handling: publish `click_topic` / `click_payload`.
9. Implement long-press handling: publish `longpress_topic` / `longpress_payload`.
10. Wire `widget_plugin!` in `src/lib.rs`.
11. Add CSS styles for the pet sprite, speech bubble, and protest sign to `resources/style.css`.
12. Add an integration test that verifies the widget accepts `TOPIC_PET_STATUS` and updates its UI.

**Exit criteria:**

- The widget compiles and can be loaded as a plugin.
- The pet sprite changes its CSS class based on `PetMood`.
- The speech bubble displays the complaint text.
- The protest sign is visible only when `mood` is `Unhappy` or `Furious`.
- Click and long-press events publish the configured topics.

---

### Phase 6: Wiring — Configuration and Registration

**Goal:** Connect all new crates to the main application.

**Dependencies:** Phase 2, Phase 4, and Phase 5 must be complete.

**Order:**

1. Add the `model/pet`, `services/pet`, and `plugins/pet` crates to the workspace `Cargo.toml`.
2. Register the pet service in `services.toml`.
3. Add a sample configuration block for the pet service in `config.toml`.
4. Add a sample widget configuration for the pet widget.
5. Add CSS styles for the pet widget to `resources/style.css`.

**Exit criteria:**

- The workspace compiles with `cargo build`.
- The pet service is loaded at application startup.
- The pet widget renders and updates based on system conditions.

---

### Phase 7: Validation — Integration and Tests

**Goal:** Verify end-to-end behavior and stability.

**Dependencies:** Phase 6 must be complete.

**Order:**

1. Run the application and verify that the pet status is broadcast on the message broker.
2. Simulate high CPU usage and verify the pet becomes annoyed or unhappy.
3. Simulate low memory and verify the pet complains about suffocating.
4. Disconnect the network and verify the pet complains about no internet.
5. Send an acknowledge command and verify the pet temporarily calms down.
6. Send a pet or feed command and verify the pet's happiness is temporarily boosted.
7. Run `cargo test` for all new and modified crates.
8. Run `cargo clippy` and `cargo fmt` and fix any issues.

**Exit criteria:**

- All tests pass.
- The pet widget renders correctly for all four mood states.
- No `unwrap`, `expect`, or `panic` remains in the new code.
- `rustfmt` and `clippy` are clean.

---

### Summary of Order

```
Phase 1: model/sysinfo (GPU + packages extensions)
    |
    v
Phase 2: services/sysinfo (GPU + packages collectors)
    |
    +---> Phase 3: model/pet
    |         |
    |         v
    |     Phase 4: services/pet
    |         |
    |         v
    |     Phase 5: plugins/pet
    |         |
    v         v
Phase 6: workspace wiring and config
    |
    v
Phase 7: integration and tests
```

### Rationale

- **Sysinfo model first:** GPU and package update message types must exist before the sysinfo service can broadcast them.
- **Sysinfo service second:** The pet service needs GPU and package data to evaluate all triggers.
- **Pet model third:** Pet message formats must exist before the pet service or widget can use them.
- **Pet service fourth:** The widget needs a running publisher to test against.
- **Pet widget fifth:** The display depends on the service's status broadcasts.
- **Wiring sixth:** Final integration only makes sense when all components are ready.
- **Tests last:** End-to-end validation closes the loop.

---

## 10. Technical Notes

- **No polling in the widget:** The widget updates exclusively through incoming `PetStatusMessage` broadcasts. The periodic re-evaluation happens in the
  service.
- **Fault tolerance:** If a sysinfo topic is never received (e.g., no GPU tool available), the corresponding cached status remains `None` and the trigger is
  skipped.
- **Graceful degradation:** If the sysinfo service is not loaded, the pet has no data to evaluate and remains in its default `Happy` mood.
- **Randomness:** Complaint and protest sign selection uses a random index to keep the pet's behavior varied. The randomness is seeded from the system entropy.
- **Timer expiration:** The acknowledge and interaction boost timers are checked on every evaluation cycle, not via a separate timer task. This keeps the
  architecture simple.
- **Performance:** The pet service's update loop runs every 5 seconds and performs only in-memory comparisons. No I/O or subprocess calls happen in the pet
  service itself.
- **Package update interval:** The sysinfo service collects package updates at a slower interval (default: 30 minutes) to avoid excessive subprocess spawning.
  The pet service uses the last cached value.

---

## 11. Compliance with `AGENTS.md`

The proposed implementation follows the project guidelines in `AGENTS.md`:

- **Crate separation:** The feature is split into `model/pet`, `services/pet`, and `plugins/pet`. Sysinfo extensions are made in `model/sysinfo` and
  `services/sysinfo`.
- **One struct per file:** Each widget component, message struct, and enum lives in its own file.
- **Service traits:** The service implements `MessageHandler`, `MessageBroadcaster`, `MessageTopicBroadcaster`, `PluginMetaGetter`, and
  `AsRef<Option<FfiCoreContext>>`.
- **Widget traits:** The widget implements `MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>`, and `WidgetBuilder`.
- **Async runtime:** The service uses `tokio::sync::mpsc` and spawns async tasks via the `PluginExecutor`.
- **GTK updates:** The widget uses `glib::MainContext::spawn_local` for GTK updates and `tokio::sync::mpsc` for message reception.
- **Event-driven:** The widget is updated by incoming messages, not by polling loops. The service's periodic re-evaluation is a lightweight in-memory check, not
  a polling loop in the widget.
- **FFI stability:** All FFI-relevant types in the model carry `#[stabby::stabby]`.
- **No panic:** The implementation uses `Result` and `Option` for error handling; no `unwrap()`, `expect()`, or `panic!`.
- **Naming:** All names are descriptive and follow Rust naming conventions.
- **Documentation:** All public structs, enums, and fields are documented in English.
- **Formatting:** Code is formatted with `rustfmt` and checked with `clippy`.
- **Dependencies:** The model uses `serde` and `stabby`; the service uses `tokio` and `tracing`; the widget uses `gtk4` and `glib`.

---

*End of document.*
