# MPRIS Widget & Service

This document describes the architecture for the **MPRIS Widget Plugin** and **MPRIS Service Plugin** of the *Smearor Swipe Launcher*. The system uses the
D-Bus MPRIS standard (`org.mpris.MediaPlayer2`) to control and display information from media players like Spotify, VLC, Firefox, etc.

The concept cleanly separates UI interactions from system logic using the ABI-stable plugin crate architecture.

---

## 1. Crate Structure

The MPRIS feature is split into three separate crates following the project architecture:

| Crate       | Path              | Responsibility                             |
|-------------|-------------------|--------------------------------------------|
| **Model**   | `model/mpris/`    | Shared structs, enums, and message formats |
| **Service** | `services/mpris/` | Backend logic, D-Bus MPRIS communication   |
| **Widget**  | `plugins/mpris/`  | GTK4 user interface, gesture handling      |

---

## 2. Model Crate (`model/mpris`)

Contains all message types and shared data structures used by both the service and widget.

### 2.1 Message Topics

```rust
pub const TOPIC_COMMAND: &str = "service.mpris.command";
pub const TOPIC_STATUS: &str = "service.mpris.status";
```

### 2.2 Command Messages (Widget -> Service)

```rust
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum MprisCommandAction {
    #[default]
    /// Start or resume playback
    Play,
    /// Pause playback
    Pause,
    /// Toggle between play and pause
    TogglePlayPause,
    /// Stop playback
    Stop,
    /// Skip to the next track
    NextTrack,
    /// Go back to the previous track
    PreviousTrack,
    /// Seek forward or backward by an offset in microseconds
    Seek,
    /// Set the playback position to an absolute value in microseconds
    SetPosition,
    /// Cycle loop mode (None -> Track -> Playlist)
    CycleLoop,
    /// Toggle shuffle on/off
    ToggleShuffle,
    /// Switch to the next active player
    NextPlayer,
    /// Switch to the previous active player
    PreviousPlayer,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MprisCommandMessage {
    /// The action to execute
    pub action: MprisCommandAction,
    /// Optional seek offset in microseconds (positive or negative)
    pub seek_offset: Option<i64>,
    /// Optional absolute position in microseconds
    pub position: Option<i64>,
    /// Optional player bus name to target a specific player
    pub player_bus_name: Option<String>,
}

impl MprisCommandMessage {
    pub fn play() -> Self { ... }
    pub fn pause() -> Self { ... }
    pub fn toggle_play_pause() -> Self { ... }
    pub fn stop() -> Self { ... }
    pub fn next_track() -> Self { ... }
    pub fn previous_track() -> Self { ... }
    pub fn seek(offset: i64) -> Self { ... }
    pub fn set_position(position: i64) -> Self { ... }
    pub fn cycle_loop() -> Self { ... }
    pub fn toggle_shuffle() -> Self { ... }
    pub fn next_player() -> Self { ... }
    pub fn previous_player() -> Self { ... }
}
```

### 2.3 Status Messages (Service -> Widget)

```rust
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct MprisPlayerInfo {
    /// D-Bus bus name of the player (e.g. "org.mpris.MediaPlayer2.spotify")
    pub bus_name: String,
    /// Human-readable player name
    pub name: String,
    /// Whether this is the currently active player
    pub is_active: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum MprisPlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum MprisLoopStatus {
    None,
    Track,
    Playlist,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct MprisTrackMetadata {
    /// Track title
    pub title: String,
    /// Track artist(s)
    pub artist: String,
    /// Album name
    pub album: String,
    /// Track length in microseconds
    pub length: i64,
    /// Cover art URL or local path
    pub art_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct MprisStatusMessage {
    /// Whether any player is currently active
    pub has_player: bool,
    /// The currently active player
    pub active_player: Option<MprisPlayerInfo>,
    /// List of all available players
    pub players: Vec<MprisPlayerInfo>,
    /// Current playback status
    pub playback_status: MprisPlaybackStatus,
    /// Metadata of the current track
    pub metadata: Option<MprisTrackMetadata>,
    /// Current playback position in microseconds
    pub position: i64,
    /// Current loop mode
    pub loop_status: MprisLoopStatus,
    /// Whether shuffle is enabled
    pub shuffle: bool,
    /// Player volume (0.0 to 1.0)
    pub volume: f32,
}
```

---

## 3. Service Plugin (`services/mpris`)

The service plugin is a singleton that runs in the background. It handles all MPRIS D-Bus communication and broadcasts state updates to all subscribed widgets.

### 3.1 File Structure

- `service.rs` - `MprisService` struct and trait implementations
- `config.rs` - `MprisServiceConfig` struct and parsing
- `lib.rs` - `service_plugin!` macro invocation

### 3.2 Service Implementation

```rust
pub struct MprisService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: MprisServiceConfig,
    pub command_sender: Sender<MprisCommand>,
    pub status_receiver: Arc<Mutex<Receiver<MprisStatusMessage>>>,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<MprisCommandMessage>>` - Processes commands from widgets
- `MessageBroadcaster<MprisStatusMessage>` - Broadcasts status updates
- `MessageTopicBroadcaster<MprisStatusMessage>` - Broadcasts to topic subscribers
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to core context

### 3.3 Command Handling

The service listens on `service.mpris.command` and handles:

| Command           | Action                                               |
|-------------------|------------------------------------------------------|
| `Play`            | Starts or resumes playback on the active player      |
| `Pause`           | Pauses playback on the active player                 |
| `TogglePlayPause` | Toggles play/pause state                             |
| `Stop`            | Stops playback                                       |
| `NextTrack`       | Skips to the next track                              |
| `PreviousTrack`   | Returns to the previous track                        |
| `Seek`            | Seeks forward or backward by the provided offset     |
| `SetPosition`     | Sets the playback position to an absolute value      |
| `CycleLoop`       | Cycles through loop modes: None -> Track -> Playlist |
| `ToggleShuffle`   | Toggles shuffle on/off                               |
| `NextPlayer`      | Switches to the next available player                |
| `PreviousPlayer`  | Switches to the previous available player            |

After each command, the service updates the MPRIS state and broadcasts an `MprisStatusMessage` to all widgets.

### 3.4 MPRIS / D-Bus Integration

The service communicates with media players via D-Bus:

1. **Initialization:** Scans for available MPRIS players on startup
2. **Event Subscription:** Listens to `PropertiesChanged` signals from all players
3. **Command Execution:** Sends method calls to the active player via D-Bus
4. **Player Rotation:** Cycles through available players when `NextPlayer`/`PreviousPlayer` is received

---

## 4. Widget Plugin (`plugins/mpris`)

The widget plugin provides the GTK4 user interface. Multiple MPRIS widget instances can exist, all synchronized via the service's status broadcasts.

### 4.1 File Structure

- `widget.rs` - `MprisWidget` struct and trait implementations
- `config.rs` - `MprisWidgetConfig` struct and parsing
- `lib.rs` - `widget_plugin!` macro invocation

### 4.2 Widget Implementation

```rust
pub struct MprisWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: MprisWidgetConfig,
    pub(crate) status_sender: Sender<MprisStatusMessage>,
    pub(crate) status_receiver: Option<Receiver<MprisStatusMessage>>,
    pub(crate) last_command_time: Arc<Mutex<Instant>>,
    pub(crate) album_art: Arc<Mutex<Option<Image>>>,
    pub(crate) progress_bar: Arc<Mutex<Option<LevelBar>>>,
    pub(crate) title_label: Arc<Mutex<Option<Label>>>,
    pub(crate) artist_label: Arc<Mutex<Option<Label>>>,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<MprisStatusMessage>>` - Receives status updates from the service
- `MessageBroadcaster<MprisCommandMessage>` - Sends commands to the service
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to core context
- `WidgetBuilder` - Builds the GTK4 widget UI

### 4.3 UI Layout

The widget is wrapped in a `gtk4::Button` with CSS classes `scroll-item` and `menu-button` for consistent appearance with the App-Launcher and Audio widgets.

- **Album Art:** Cover image of the current track rendered via `gtk4::Image`, sized at 48x48 px (matching the icon size of the App-Launcher and Audio widgets).
  The image is loaded asynchronously from the `art_url` in `MprisTrackMetadata`. If no `art_url` is available, a fallback symbolic icon (
  `audio-x-generic-symbolic`) is displayed.
- **Progress Bar:** A `gtk4::LevelBar` positioned directly below the album art, visually identical to the volume bar in the Audio widget, filling proportionally
  to the current playback position (0-100%).
- **Title & Artist:** Text labels (`gtk4::Label`) below the progress bar showing the current track title and artist, with text ellipsization for overflow.

### 4.4 Interaction Mapping

The interaction mapping mirrors the Audio widget exactly, replacing audio-specific actions with MPRIS equivalents:

| Action             | Touch Input        | Mouse Input        | Generated Message                          |
|--------------------|--------------------|--------------------|--------------------------------------------|
| **Play / Pause**   | Short tap (center) | Left click         | `MprisCommandMessage::toggle_play_pause()` |
| **Next Track**     | Scroll up          | Scroll up          | `MprisCommandMessage::next_track()`        |
| **Previous Track** | Scroll down        | Scroll down        | `MprisCommandMessage::previous_track()`    |
| **Next Player**    | Long press (2s)    | Double right click | `MprisCommandMessage::next_player()`       |

When an interaction occurs, the widget creates the appropriate `MprisCommandMessage` and broadcasts it via the message broker to the service.

### 4.5 State Synchronization

All MPRIS widgets subscribe to `service.mpris.status`. When the service broadcasts a status update, all widgets update their UI simultaneously:

- Album art updates to the new track cover
- Title and artist labels refresh
- Play/pause button icon changes based on playback state
- Progress bar updates to current position

### 4.6 Icons

- `media-seek-forward-symbolic`
- `media-seek-backward-symbolic`
- `media-skip-forward-symbolic`
- `media-skip-backward-symbolic`
- `media-playback-start-symbolic`
- `media-playback-pause-symbolic`
- `media-playback-stop-symbolic`
- `media-playlist-consecutive-symbolic`
- `media-playlist-no-repeat-symbolic`
- `media-playlist-no-shuffle-symbolic`
- `media-playlist-repeat-one-symbolic`
- `media-playlist-repeat-song-symbolic`
- `media-playlist-repeat-symbolic`
- `media-playlist-shuffle-symbolic`

---

## 5. Message Flow

```
+-------------------+         +-------------------+         +-------------------+
|   MPRIS Widget 1  |<--------|                   |-------->|   MPRIS Widget 2  |
| (Control Panel)   |  Status |   Event Broker    |  Status | (Mini Player)     |
+---------+---------+  Broadcast               Broadcast  +---------+---------+
          |                                                 ^
          | Command                                         | Status
          v                                                 |
+---------+-------------------+                             |
|      MprisService           |                             |
|   (D-Bus MPRIS Bridge)      |-----------------------------+
+-----------------------------+       Status Broadcast
```

---

## 6. Configuration

### 6.1 Widget Configuration (`config.rs`)

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MprisWidgetConfig {
    /// Widget width in pixels
    #[serde(default = "default_width")]
    pub width: i32,
    /// Widget height in pixels
    #[serde(default = "default_height")]
    pub height: i32,
    /// Whether to show the album art
    #[serde(default = "default_show_album_art")]
    pub show_album_art: bool,
    /// Whether to show the progress bar
    #[serde(default = "default_show_progress_bar")]
    pub show_progress_bar: bool,
    /// Whether to show the player name label
    #[serde(default = "default_show_player_label")]
    pub show_player_label: bool,
    /// List of allowed player bus names (empty = all players)
    #[serde(default)]
    pub player_filter: Vec<String>,
}
```

### 6.2 Service Configuration

```rust
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct MprisServiceConfig {
    /// D-Bus address override (optional)
    #[builder(default, setter(into))]
    pub dbus_address: Option<String>,
}
```

---

## 7. Technical Considerations

- **D-Bus Signal Handling:** The service listens to `PropertiesChanged` signals from all MPRIS players to avoid polling.
- **Player Disconnection:** When a player closes, the service automatically removes it from the player list and falls back to the next available player.
- **Multiple Players:** The service maintains a list of all active players and allows cycling through them.
- **Async D-Bus Communication:** All D-Bus operations run asynchronously to avoid blocking the GTK4 main thread.
- **Singleton Service:** Only one `MprisService` instance exists, shared by all MPRIS widgets, preventing redundant D-Bus connections.