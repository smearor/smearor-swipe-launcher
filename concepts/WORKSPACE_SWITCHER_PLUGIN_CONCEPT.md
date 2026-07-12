# Workspace Switcher Widget Concept

This document describes the concept for a **Workspace Switcher Widget** in the *Smearor Swipe Launcher*. The widget provides a compact, touch-optimized way to
switch between workspaces using swipe gestures — one workspace per view, analogous to how the Weather Widget rotates through data categories.

The system follows the decoupled SOA architecture:

1. **Model Crate (`model/workspace` — `smearor-model-compositor`):** Shared structs, enums, topics, and message formats for workspace snapshot, switch, and
   create commands. The new types are added to the existing compositor model crate alongside `WorkspaceChangedEvent` and `WorkspaceLifecycleEvent`.
2. **Service Extensions (`services/hyprland`, `services/gnome`, `services/wayland`):** Each compositor service gains handlers for compositor-unified switch and
   create commands, and broadcasts a workspace snapshot on startup.
3. **Widget Crate (`plugins/workspace-switcher`):** Pure GTK4 UI that subscribes to workspace events, maintains a dynamic view list, and sends switch/create
   commands on swipe.

---

## 1. Motivation

The launcher already receives `WorkspaceChangedEvent` and `WorkspaceLifecycleEvent` from all three compositor services. The launcher core uses these for layout
profile re-evaluation. However, there is no widget that exposes workspace switching to the user.

The Workspace Switcher Widget fills this gap: a single tile in the launcher bar that shows the current workspace and allows the user to swipe through all
available workspaces. Swiping past the first or last workspace creates a new workspace dynamically.

---

## 2. Scope

- **In scope:**
    - New message types in the existing `model/workspace` (`smearor-model-compositor`) crate
    - Widget crate `plugins/workspace-switcher` with swipe-based view rotation
    - Service extensions in all three compositor services to handle switch and create commands
    - Workspace snapshot broadcasting for initial state
    - Dynamic workspace creation when swiping past the edges
- **Out of scope:**
    - Workspace renaming or deletion from the widget (future enhancement)
    - Monitor control widget (separate future concept)
    - Per-monitor workspace filtering (all workspaces across all monitors are shown)

---

## 3. Current State

### 3.1 Existing Events (`model/workspace`)

The following events are already defined and broadcast by all three compositor services:

```rust
/// Broadcast when the active workspace changes on a monitor.
pub struct WorkspaceChangedEvent {
    pub workspace_name: stabby::string::String,
    pub workspace_id: i32,
    pub monitor_index: u32,
}

/// Broadcast when a workspace is created or destroyed.
pub struct WorkspaceLifecycleEvent {
    pub workspace_name: stabby::string::String,
    pub workspace_id: i32,
    pub monitor_index: u32,
    pub lifecycle_type: WorkspaceLifecycleType,
}
```

### 3.2 Existing Dispatch (`model/hyprland`)

Hyprland already has a `WorkspaceDispatchMessage` that the `HyprlandService` handles via `MessageHandler<WorkspaceDispatchMessage>`. This message uses
`HyprlandWorkspaceIdentifierWithSpecial` which supports `Id`, `Name`, `Relative`, `Previous`, `Empty`, `Special`, etc.

However, this is Hyprland-specific. GNOME and Wayland services have no dispatch mechanism yet.

### 3.3 Widget Patterns

The Weather Widget (`plugins/weather`) and Network Widget (`plugins/network`) demonstrate the swipe-based view rotation pattern:

- `GestureDrag` with `SWIPE_THRESHOLD: 50.0` detects vertical swipes
- Swipe-up calls `next_view()`, swipe-down calls `prev_view()`
- Views are defined as an enum (`WeatherView`) and rendered via a `render_view()` function
- The widget maintains `current_view: Rc<RefCell<usize>>` and `latest_status: Rc<RefCell<Option<...>>>`

The Workspace Switcher adapts this pattern but with **dynamic views** (one per workspace) instead of a static enum.

### 3.4 Nerd Font Icons

The launcher uses `resolve_gtk_nerd_icon()` to map Nerd Font CSS class names (e.g. `"nf-md-numeric-1"`) to GTK resource paths. The Network Widget demonstrates
this in `set_icon_image()`. The Workspace Switcher will use the same mechanism for per-workspace icons.

---

## 4. Architecture

### 4.1 Overview

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                        Workspace Switcher Widget                             │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐    │
│  │  Icon (Nerd Font)  │  Label (workspace name)  │  Dot Indicator      │    │
│  │  nf-md-numeric-3   │  "3"                    │  ○○●○○              │    │
│  └──────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  Swipe-Up    → next workspace (or create new after last)                     │
│  Swipe-Down  → previous workspace (or create new before first)               │
│  Long-Press  → optional: broadcast custom topic (e.g. workspace overview)    │
│  Click       → optional: broadcast custom topic                              │
└──────────────────────────────────────────────────────────────────────────────┘
                    │                                       ▲
                    │  SwitchWorkspaceMessage                │  WorkspaceSnapshotMessage
                    │  CreateWorkspaceMessage                │  WorkspaceChangedEvent
                    ▼                                       │  WorkspaceLifecycleEvent
┌──────────────────────────────────────────────────────────────────────────────┐
│                        Compositor Service                                    │
│                                                                              │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐           │
│  │  HyprlandService │  │  WaylandService  │  │  GnomeService    │           │
│  │  (IPC socket)    │  │  (ext-workspace) │  │  (D-Bus)         │           │
│  │                  │  │                  │  │                  │           │
│  │  Handles:        │  │  Handles:        │  │  Handles:        │           │
│  │  SwitchWorkspace │  │  SwitchWorkspace │  │  SwitchWorkspace │           │
│  │  CreateWorkspace │  │  CreateWorkspace │  │  CreateWorkspace │           │
│  │                  │  │                  │  │                  │           │
│  │  Broadcasts:     │  │  Broadcasts:     │  │  Broadcasts:     │           │
│  │  Snapshot        │  │  Snapshot        │  │  Snapshot        │           │
│  │  ChangedEvent    │  │  ChangedEvent    │  │  ChangedEvent    │           │
│  │  LifecycleEvent  │  │  LifecycleEvent  │  │  LifecycleEvent  │           │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘           │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Event Flow

#### Initial State

1. Widget starts with an empty workspace list and renders a **placeholder state** (loading icon `nf-md-loading`, label `"..."`).
2. Widget broadcasts `WorkspaceSnapshotRequestMessage`.
3. Service receives the request and queries the compositor for all workspaces.
4. Service broadcasts `WorkspaceSnapshotMessage` containing the full workspace list and the active workspace.
5. Widget populates its view list, replaces the placeholder, and renders the active workspace.

**Important note (Bootstrapping):** Until the service response arrives, a few milliseconds pass. During this time the workspace list is empty. The widget must
render the placeholder state (see section 7.3.1) instead of drawing an empty layout or crashing on the first render attempt. Swipe gestures are ignored in this
state (`ws_list.is_empty()` → early return).

#### Swipe to Next Workspace

1. User swipes up on the widget.
2. Widget calls `next_view()`.
3. If not at the last workspace: widget sends `SwitchWorkspaceMessage { workspace_id }` and optimistically updates the UI.
4. Service receives the message, translates it to the compositor-specific dispatch, and switches the workspace.
5. Compositor fires a workspace change event.
6. Service broadcasts `WorkspaceChangedEvent`.
7. Widget receives the event and confirms the active workspace.

#### Swipe Past Last Workspace (Create New)

1. User swipes up on the last workspace.
2. Widget calls `next_view()` and detects it is at the edge.
3. Widget sends `CreateWorkspaceMessage { relative_to: last_workspace_id, position: After }`.
4. Service receives the message, creates a new workspace via the compositor API.
5. Compositor fires a workspace lifecycle event (Created).
6. Service broadcasts `WorkspaceLifecycleEvent { lifecycle_type: Created }`.
7. Widget adds the new workspace to its view list.
8. Widget sends `SwitchWorkspaceMessage { workspace_id: new_workspace_id }`.
9. Service switches to the new workspace.
10. Compositor fires a workspace change event.
11. Service broadcasts `WorkspaceChangedEvent`.
12. Widget updates the active workspace.

#### Swipe Past First Workspace (Create New Before)

1. User swipes down on the first workspace.
2. Widget calls `prev_view()` and detects it is at the edge.
3. Widget sends `CreateWorkspaceMessage { relative_to: first_workspace_id, position: Before }`.
4. Service creates a new workspace before the first one.
5. Compositor fires lifecycle + change events.
6. Widget prepends the new workspace and switches to it.

---

## 5. Crate Structure

Following the workspace conventions (`AGENTS.md`), the feature is split into three crates:

| Crate        | Path                                            | Responsibility                                                                |
|--------------|-------------------------------------------------|-------------------------------------------------------------------------------|
| **Model**    | `model/workspace/` (`smearor-model-compositor`) | Shared structs, enums, topics, and message formats (existing crate, extended) |
| **Services** | `services/hyprland/` etc.                       | Handlers for switch/create commands, snapshot broadcasting                    |
| **Widget**   | `plugins/workspace-switcher/`                   | GTK4 user interface, swipe gestures, dynamic view list                        |

---

## 6. Model Crate (`model/workspace` — `smearor-model-compositor`)

The new message types are added to the existing `smearor-model-compositor` crate (located at `model/workspace/`), alongside the already-defined
`WorkspaceChangedEvent` and `WorkspaceLifecycleEvent`. No new model crate is created.

### 6.1 Message Topics

```rust
/// Topic for workspace snapshot requests (Widget -> Service).
pub const TOPIC_WORKSPACE_SNAPSHOT_REQUEST: &str = "compositor::workspace_snapshot_request";

/// Topic for workspace snapshot responses (Service -> Widget).
pub const TOPIC_WORKSPACE_SNAPSHOT: &str = "compositor::workspace_snapshot";

/// Topic for workspace switch commands (Widget -> Service).
pub const TOPIC_SWITCH_WORKSPACE: &str = "compositor::switch_workspace";

/// Topic for workspace creation commands (Widget -> Service).
pub const TOPIC_CREATE_WORKSPACE: &str = "compositor::create_workspace";
```

### 6.2 Workspace Info

A single workspace entry, used in snapshots and internal widget state.

```rust
/// Information about a single workspace.
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub struct WorkspaceInfo {
    /// The workspace ID (numeric, as reported by the compositor).
    /// Set to `-1` for special workspaces.
    pub workspace_id: i32,
    /// The workspace name or number as a string.
    pub workspace_name: stabby::string::String,
    /// The monitor index (0-based, matching GDK display order) on which the
    /// workspace is located.
    pub monitor_index: u32,
    /// Whether this workspace is currently active.
    pub is_active: bool,
}
```

### 6.3 Workspace Snapshot Message

Broadcast by the service in response to a snapshot request, or automatically on startup.

```rust
/// Snapshot of all workspaces, sent from the service to the widget.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct WorkspaceSnapshotMessage {
    /// All known workspaces, ordered by workspace ID.
    pub workspaces: stabby::vec::Vec<WorkspaceInfo>,
    /// The currently active workspace ID.
    pub active_workspace_id: i32,
    /// The monitor index on which the active workspace is located.
    pub active_monitor_index: u32,
}

impl TypedMessage for WorkspaceSnapshotMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_workspace_switcher_model::WorkspaceSnapshotMessage");
}

impl MessageTopic for WorkspaceSnapshotMessage {
    fn topic() -> &'static str {
        TOPIC_WORKSPACE_SNAPSHOT
    }
}

impl SharedMessage for WorkspaceSnapshotMessage {
    fn topic(&self) -> &'static str {
        TOPIC_WORKSPACE_SNAPSHOT
    }
}
```

### 6.4 Workspace Snapshot Request Message

Sent by the widget on startup to request the current workspace list.

```rust
/// Request a workspace snapshot from the active compositor service.
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub struct WorkspaceSnapshotRequestMessage {
    /// The monitor index the widget is interested in.
    /// Set to `0` if the widget does not filter by monitor.
    pub monitor_index: u32,
}

impl TypedMessage for WorkspaceSnapshotRequestMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_workspace_switcher_model::WorkspaceSnapshotRequestMessage");
}

impl MessageTopic for WorkspaceSnapshotRequestMessage {
    fn topic() -> &'static str {
        TOPIC_WORKSPACE_SNAPSHOT_REQUEST
    }
}

impl SharedMessage for WorkspaceSnapshotRequestMessage {
    fn topic(&self) -> &'static str {
        TOPIC_WORKSPACE_SNAPSHOT_REQUEST
    }
}
```

### 6.5 Switch Workspace Message

Compositor-unified command to switch to a specific workspace by ID.

```rust
/// Command to switch to a specific workspace.
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub struct SwitchWorkspaceMessage {
    /// The workspace ID to switch to.
    pub workspace_id: i32,
}

impl TypedMessage for SwitchWorkspaceMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_workspace_switcher_model::SwitchWorkspaceMessage");
}

impl MessageTopic for SwitchWorkspaceMessage {
    fn topic() -> &'static str {
        TOPIC_SWITCH_WORKSPACE
    }
}

impl SharedMessage for SwitchWorkspaceMessage {
    fn topic(&self) -> &'static str {
        TOPIC_SWITCH_WORKSPACE
    }
}
```

### 6.6 Create Workspace Message

Compositor-unified command to create a new workspace relative to an existing one.

```rust
/// Position for creating a new workspace relative to a reference workspace.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorkspaceCreatePosition {
    /// Create the new workspace before the reference workspace.
    #[default]
    Before,
    /// Create the new workspace after the reference workspace.
    After,
}

/// Command to create a new workspace.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct CreateWorkspaceMessage {
    /// The workspace ID of the reference workspace.
    pub relative_to: i32,
    /// Whether to create the new workspace before or after the reference.
    pub position: WorkspaceCreatePosition,
}

impl TypedMessage for CreateWorkspaceMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_workspace_switcher_model::CreateWorkspaceMessage");
}

impl MessageTopic for CreateWorkspaceMessage {
    fn topic() -> &'static str {
        TOPIC_CREATE_WORKSPACE
    }
}

impl SharedMessage for CreateWorkspaceMessage {
    fn topic(&self) -> &'static str {
        TOPIC_CREATE_WORKSPACE
    }
}
```

### 6.7 Updated `lib.rs` (existing crate — add new exports)

The existing `model/workspace/src/lib.rs` is extended with the new module and re-exports:

```rust
// Existing modules remain unchanged
pub mod monitor;
pub mod workspace;

// New module for workspace switcher message types
pub mod switcher;

// Existing re-exports remain unchanged
// ...

// New re-exports for workspace switcher types
pub use switcher::CreateWorkspaceMessage;
pub use switcher::SwitchWorkspaceMessage;
pub use switcher::TOPIC_CREATE_WORKSPACE;
pub use switcher::TOPIC_SWITCH_WORKSPACE;
pub use switcher::TOPIC_WORKSPACE_SNAPSHOT;
pub use switcher::TOPIC_WORKSPACE_SNAPSHOT_REQUEST;
pub use switcher::WorkspaceCreatePosition;
pub use switcher::WorkspaceInfo;
pub use switcher::WorkspaceSnapshotMessage;
pub use switcher::WorkspaceSnapshotRequestMessage;
```

---

## 7. Widget Crate (`plugins/workspace-switcher`)

### 7.1 Configuration (`config.rs`)

```rust
pub const DEFAULT_WIDTH: i32 = 120;
pub const DEFAULT_HEIGHT: i32 = 80;

/// Configuration for the workspace switcher widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct WorkspaceSwitcherConfig {
    /// The width of the widget in pixels.
    #[builder(default, setter(into))]
    pub(crate) width: Option<i32>,

    /// The height of the widget in pixels.
    #[builder(default, setter(into))]
    pub(crate) height: Option<i32>,

    /// Whether to show the workspace label (name or number).
    #[builder(default = true)]
    pub(crate) show_label: bool,

    /// Whether to show the dot indicator (position in workspace list).
    #[builder(default = true)]
    pub(crate) show_dot_indicator: bool,

    /// Icon size in pixels for the workspace icon.
    #[builder(default = 32)]
    pub(crate) icon_size: i32,

    /// Map of workspace IDs (as strings) to Nerd Font icon class names.
    /// Example: `{ "1" = "nf-md-numeric-1", "2" = "nf-md-numeric-2" }`
    #[builder(default)]
    pub(crate) icon_map: HashMap<String, String>,

    /// Default icon class name for workspaces not in `icon_map`.
    #[builder(default = "nf-md-monitor")]
    pub(crate) default_icon: String,

    /// Message topic for single-click action.
    #[serde(default)]
    pub click_topic: Option<String>,

    /// Message payload for single-click action (JSON/TOML).
    #[serde(default)]
    pub click_payload: Option<Value>,

    /// Message topic for long-press.
    #[serde(default)]
    pub longpress_topic: Option<String>,

    /// Message payload for long-press (JSON/TOML).
    #[serde(default)]
    pub longpress_payload: Option<Value>,
}

impl Default for WorkspaceSwitcherConfig {
    fn default() -> Self {
        Self {
            width: Some(DEFAULT_WIDTH),
            height: Some(DEFAULT_HEIGHT),
            show_label: true,
            show_dot_indicator: true,
            icon_size: 32,
            icon_map: HashMap::new(),
            default_icon: "nf-md-monitor".to_string(),
            click_topic: None,
            click_payload: None,
            longpress_topic: None,
            longpress_payload: None,
        }
    }
}
```

### 7.2 Widget Struct (`widget.rs`)

```rust
pub struct WorkspaceSwitcherWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: WorkspaceSwitcherConfig,
    /// Dynamic list of workspaces (one per view).
    pub workspaces: Rc<RefCell<Vec<WorkspaceInfo>>>,
    /// Index of the currently displayed workspace in the view list.
    pub current_view: Rc<RefCell<usize>>,
    /// The icon image widget.
    pub icon_image: Rc<RefCell<Option<Image>>>,
    /// The label widget showing the workspace name.
    pub label_widget: Rc<RefCell<Option<Label>>>,
    /// The dot indicator container.
    pub dot_container: Rc<RefCell<Option<GtkBox>>>,
}
```

### 7.3 Swipe Logic

Unlike the Weather Widget (which wraps around with `% views.len()`), the Workspace Switcher uses **edge-detection** to create new workspaces.

**Important note (Borrow-Checker):** Data must be extracted from the `RefCell` borrow before performing UI updates, since `borrow()` and `borrow_mut()` cannot
be held simultaneously. All UI widgets require their own `Rc` clones for the async block. The pattern is: borrow data in a limited scope → extract values →
borrow ends → perform UI update with extracted values.

```rust
fn next_view(&self) {
    let workspaces = self.workspaces.clone();
    let current_view = self.current_view.clone();
    let broadcaster = self.get_broadcaster();

    // Clone UI elements for the async block
    let icon_image = self.icon_image.clone();
    let label_widget = self.label_widget.clone();
    let dot_container = self.dot_container.clone();
    let config = self.config.clone();

    glib::MainContext::default().spawn_local(async move {
        // Borrow data in a limited scope and extract values.
        // The borrow ends as soon as the scope is exited.
        let (idx, has_next, next_id, last_id) = {
            let ws_list = workspaces.borrow();
            if ws_list.is_empty() {
                return;
            }
            let idx = *current_view.borrow();
            let has_next = idx + 1 < ws_list.len();
            let next_id = if has_next { ws_list[idx + 1].workspace_id } else { -1 };
            let last_id = ws_list[idx].workspace_id;
            (idx, has_next, next_id, last_id)
        }; // ws_list borrow ends here

        if has_next {
            let msg = SwitchWorkspaceMessage { workspace_id: next_id };
            broadcaster.broadcast_message_to_topic(msg);
            *current_view.borrow_mut() = idx + 1;
            Self::update_ui_internal(
                &workspaces,
                &current_view,
                &icon_image,
                &label_widget,
                &dot_container,
                &config,
            );
        } else {
            // At the last workspace — create a new workspace after it.
            let msg = CreateWorkspaceMessage {
                relative_to: last_id,
                position: WorkspaceCreatePosition::After,
            };
            broadcaster.broadcast_message_to_topic(msg);
        }
    });
}

fn prev_view(&self) {
    let workspaces = self.workspaces.clone();
    let current_view = self.current_view.clone();
    let broadcaster = self.get_broadcaster();

    // Clone UI elements for the async block
    let icon_image = self.icon_image.clone();
    let label_widget = self.label_widget.clone();
    let dot_container = self.dot_container.clone();
    let config = self.config.clone();

    glib::MainContext::default().spawn_local(async move {
        // Borrow data in a limited scope and extract values.
        let (idx, has_prev, prev_id, first_id) = {
            let ws_list = workspaces.borrow();
            if ws_list.is_empty() {
                return;
            }
            let idx = *current_view.borrow();
            let has_prev = idx > 0;
            let prev_id = if has_prev { ws_list[idx - 1].workspace_id } else { -1 };
            let first_id = ws_list[idx].workspace_id;
            (idx, has_prev, prev_id, first_id)
        }; // ws_list borrow ends here

        if has_prev {
            let msg = SwitchWorkspaceMessage { workspace_id: prev_id };
            broadcaster.broadcast_message_to_topic(msg);
            *current_view.borrow_mut() = idx - 1;
            Self::update_ui_internal(
                &workspaces,
                &current_view,
                &icon_image,
                &label_widget,
                &dot_container,
                &config,
            );
        } else {
            // At the first workspace — create a new workspace before it.
            let msg = CreateWorkspaceMessage {
                relative_to: first_id,
                position: WorkspaceCreatePosition::Before,
            };
            broadcaster.broadcast_message_to_topic(msg);
        }
    });
}
```

### 7.3.1 Placeholder State (Bootstrapping)

Until the `WorkspaceSnapshotMessage` response from the service arrives, a few milliseconds pass. During this time the workspace list is empty (
`ws_list.is_empty()`). The widget must render a **placeholder state** instead of drawing an empty layout or crashing on the first render attempt:

```rust
fn update_ui_internal(
    workspaces: &Rc<RefCell<Vec<WorkspaceInfo>>>,
    current_view: &Rc<RefCell<usize>>,
    icon_image: &Rc<RefCell<Option<Image>>>,
    label_widget: &Rc<RefCell<Option<Label>>>,
    dot_container: &Rc<RefCell<Option<GtkBox>>>,
    config: &WorkspaceSwitcherConfig,
) {
    let ws_list = workspaces.borrow();

    if ws_list.is_empty() {
        // Placeholder: show loading icon until the snapshot arrives.
        if let Some(ref image) = *icon_image.borrow() {
            set_workspace_icon(image, "nf-md-loading", config.icon_size);
        }
        if let Some(ref label) = *label_widget.borrow() {
            label.set_text("...");
        }
        if let Some(ref container) = *dot_container.borrow() {
            update_dot_indicator(container, 0, 0);
        }
        return;
    }

    let idx = *current_view.borrow();
    let ws = &ws_list[idx];

    if let Some(ref image) = *icon_image.borrow() {
        let icon_class = resolve_workspace_icon(config, ws.workspace_id);
        set_workspace_icon(image, &icon_class, config.icon_size);
    }
    if let Some(ref label) = *label_widget.borrow() {
        if config.show_label {
            label.set_text(&ws.workspace_name.to_string());
            label.set_visible(true);
        } else {
            label.set_visible(false);
        }
    }
    if let Some(ref container) = *dot_container.borrow() {
        if config.show_dot_indicator {
            update_dot_indicator(container, ws_list.len(), idx);
            container.set_visible(true);
        } else {
            container.set_visible(false);
        }
    }
}
```

The loading icon (`nf-md-loading`) uses the same `resolve_gtk_nerd_icon()` mechanism as all other icons. Optionally, a CSS animation (`@keyframes spin`) can be
defined for a rotating loading indicator.

### 7.4 Event Handling

The widget subscribes to three topics via `AcceptTopic`:

```rust
impl AcceptTopic<FfiEnvelope> for WorkspaceSwitcherWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_WORKSPACE_SNAPSHOT
            || topic == TOPIC_WORKSPACE_CHANGED
            || topic == TOPIC_WORKSPACE_LIFECYCLE
    }
}
```

- **`WorkspaceSnapshotMessage`**: Replaces the entire workspace list and sets the active view.
- **`WorkspaceChangedEvent`**: Updates the active workspace in the view list and updates the UI.
- **`WorkspaceLifecycleEvent`**: Adds or removes a workspace from the view list (`Created` → append/insert, `Destroyed` → remove).

### 7.5 Icon Resolution

The widget uses `resolve_gtk_nerd_icon()` to resolve Nerd Font class names to GTK resource paths:

```rust
fn resolve_workspace_icon(config: &WorkspaceSwitcherConfig, workspace_id: i32) -> String {
    let key = workspace_id.to_string();
    config
        .icon_map
        .get(&key)
        .cloned()
        .unwrap_or_else(|| config.default_icon.clone())
}

fn set_workspace_icon(image: &Image, icon_class: &str, icon_size: i32) {
    if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_class) {
        let resource_path = format!("/com/nerd/icons/{}.svg", gtk_icon_name);
        if gtk4::gio::resources_lookup_data(&resource_path, gtk4::gio::ResourceLookupFlags::NONE).is_ok() {
            image.set_resource(Some(&resource_path));
        } else {
            image.set_icon_name(Some(&gtk_icon_name));
        }
    }
    image.set_pixel_size(icon_size);
}
```

### 7.6 Dot Indicator

A row of small dots below the icon/label showing the position in the workspace list:

```rust
fn update_dot_indicator(container: &GtkBox, total: usize, current: usize) {
    while container.first_child().is_some() {
        container.remove(&container.first_child().unwrap());
    }
    for i in 0..total {
        let dot = Label::builder()
            .css_classes(if i == current {
                vec!["workspace-dot-active".to_string()]
            } else {
                vec!["workspace-dot-inactive".to_string()]
            })
            .build();
        dot.set_text("\u{2022}"); // Bullet character
        container.append(&dot);
    }
}
```

### 7.7 Widget Layout

```
┌─────────────────────────────────────────┐
│            Vertical Box                 │
│                                         │
│    ┌─────────────────────────────┐      │
│    │     Icon (Image)            │      │
│    │     nf-md-numeric-3         │      │
│    └─────────────────────────────┘      │
│                                         │
│    ┌─────────────────────────────┐      │
│    │     Label                   │      │
│    │     "3"                     │      │
│    └─────────────────────────────┘      │
│                                         │
│    ┌─────────────────────────────┐      │
│    │     Dot Indicator           │      │
│    │     ○ ○ ● ○ ○               │      │
│    └─────────────────────────────┘      │
│                                         │
└─────────────────────────────────────────┘
```

### 7.8 `lib.rs`

```rust
pub(crate) mod config;
pub(crate) mod widget;

use crate::widget::WorkspaceSwitcherWidget;
use smearor_swipe_launcher_plugin_api::widget_plugin;

widget_plugin!(WorkspaceSwitcherWidget);
```

---

## 8. Service Extensions

Each compositor service (`services/hyprland`, `services/gnome`, `services/wayland`) must handle the new compositor-unified command messages and broadcast
workspace snapshots.

### 8.1 Hyprland Service (`services/hyprland`)

#### Switch Workspace

The service already handles `WorkspaceDispatchMessage`. Add a `MessageHandler<SwitchWorkspaceMessage>` that translates the compositor-unified message into a
Hyprland dispatch:

```rust
impl MessageHandler<FfiEnvelopePayload<SwitchWorkspaceMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<SwitchWorkspaceMessage>, _sender_id: &str) {
        let dispatch = WorkspaceDispatchMessage {
            identifier: HyprlandWorkspaceIdentifierWithSpecial {
                kind: HyprlandWorkspaceIdentifierKind::Id,
                id: message.0.workspace_id,
                ..Default::default()
            },
        };
        let _ = self.command_sender.send(HyprlandCommand::Dispatch(
            HyprlandDispatchMessage {
                kind: HyprlandDispatchActionKind::Workspace,
                workspace: StabbyOption::Some(dispatch.into()),
                ..Default::default()
            },
        ));
    }
}
```

#### Create Workspace

Hyprland creates workspaces implicitly when switching to a non-existent workspace ID. To create a workspace before or after a reference:

```rust
impl MessageHandler<FfiEnvelopePayload<CreateWorkspaceMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<CreateWorkspaceMessage>, _sender_id: &str) {
        let new_id = match message.0.position {
            WorkspaceCreatePosition::Before => message.0.relative_to - 1,
            WorkspaceCreatePosition::After => message.0.relative_to + 1,
        };
        let dispatch = WorkspaceDispatchMessage {
            identifier: HyprlandWorkspaceIdentifierWithSpecial {
                kind: HyprlandWorkspaceIdentifierKind::Id,
                id: new_id,
                ..Default::default()
            },
        };
        let _ = self.command_sender.send(HyprlandCommand::Dispatch(/* ... */));
    }
}
```

#### Workspace Snapshot

On startup or on `WorkspaceSnapshotRequestMessage`, query `hyprland::data::Workspaces` and `hyprland::data::Monitors` to build the snapshot:

```rust
async fn build_workspace_snapshot() -> WorkspaceSnapshotMessage {
    let workspaces = hyprland::data::Workspaces::get().unwrap_or_default();
    let monitors = hyprland::data::Monitors::get().unwrap_or_default();
    let active_monitor = monitors.first();

    let mut ws_list: Vec<WorkspaceInfo> = workspaces
        .iter()
        .map(|ws| WorkspaceInfo {
            workspace_id: ws.id,
            workspace_name: ws.name.clone().into(),
            monitor_index: /* resolve from monitors */,
            is_active: /* match active workspace */,
        })
        .collect();
    ws_list.sort_by_key(|w| w.workspace_id);

    WorkspaceSnapshotMessage {
        workspaces: ws_list.into(),
        active_workspace_id: /* current active */,
        active_monitor_index: /* current monitor */,
    }
}
```

### 8.2 GNOME Service (`services/gnome`)

GNOME manages workspaces via the `org.gnome.Mutter.DisplayConfig` interface or directly through the Shell interface. The GNOME service uses the `zbus` crate for
asynchronous D-Bus communication.

#### Querying the Workspace List & Active Workspace

Via the `org.gnome.Shell` destination and the `/org/gnome/Shell` object, the workspace count and active workspace index can be queried through JavaScript
evaluation (`Eval` method) or the standard Mutter interfaces:

```rust
/// Query the total number of workspaces.
async fn query_workspace_count(shell_proxy: &ShellProxy<'_>) -> Result<i32, zbus::Error> {
    let js = "global.workspace_manager.get_n_workspaces()";
    let (success, result) = shell_proxy.eval(js).await?;
    if success {
        Ok(result.trim().parse::<i32>().unwrap_or(0))
    } else {
        Ok(0)
    }
}

/// Query the active workspace index.
async fn query_active_workspace(shell_proxy: &ShellProxy<'_>) -> Result<i32, zbus::Error> {
    let js = "global.workspace_manager.get_active_workspace_index()";
    let (success, result) = shell_proxy.eval(js).await?;
    if success {
        Ok(result.trim().parse::<i32>().unwrap_or(0))
    } else {
        Ok(0)
    }
}

/// Query the name of a workspace by index.
async fn query_workspace_name(shell_proxy: &ShellProxy<'_>, index: i32) -> Option<String> {
    let js = format!(
        "global.workspace_manager.get_workspace_by_index({}).title() || '{}'",
        index, index
    );
    let (success, result) = shell_proxy.eval(&js).await.ok()?;
    if success {
        Some(result.trim().trim_matches('\'').to_string())
    } else {
        None
    }
}
```

Additionally, `org.gnome.desktop.wm.preferences.num-workspaces` can be read via D-Bus (`org.freedesktop.DBus.Properties.Get`) to determine the configured number
of workspaces.

#### Switching (Dispatch)

Sending a `SwitchWorkspaceMessage { workspace_id }` is intercepted by the GNOME service, which issues the D-Bus call:

```rust
impl MessageHandler<FfiEnvelopePayload<SwitchWorkspaceMessage>> for GnomeService {
    fn handle_message(&self, message: FfiEnvelopePayload<SwitchWorkspaceMessage>, _sender_id: &str) {
        let workspace_id = message.0.workspace_id;
        let broadcaster = self.get_broadcaster();
        let core_context = self.core_context.clone();

        // Spawn async task to perform D-Bus call
        let runtime = self.runtime.clone();
        runtime.spawn(async move {
            let connection = match zbus::Connection::session().await {
                Ok(conn) => conn,
                Err(error) => {
                    error!("GNOME service: failed to connect to D-Bus: {error}");
                    return;
                }
            };
            let proxy = ShellProxy::new(&connection).await;

            let js = format!(
                "global.workspace_manager.get_workspace_by_index({}).activate(global.display.get_current_time())",
                workspace_id
            );

            if let Err(error) = proxy.eval(&js).await {
                error!("GNOME service: failed to switch workspace: {error}");
            }
        });
    }
}
```

#### Create Workspace

GNOME has (in contrast to Hyprland) a **strict, index-based, and sequential workspace model**. Inserting before a specific index without shifting is not
possible. Workspaces are dynamically created when a window is moved beyond the last workspace. The service can simulate workspace creation by calling
`append_new_workspace`:

```rust
async fn create_workspace(shell_proxy: &ShellProxy<'_>, position: WorkspaceCreatePosition, relative_to: i32) {
    match position {
        WorkspaceCreatePosition::After => {
            // Append a new workspace at the end.
            let js = "global.workspace_manager.append_new_workspace(false, global.display.get_current_time())";
            let _ = shell_proxy.eval(js).await;
        }
        WorkspaceCreatePosition::Before => {
            // GNOME does not support inserting before a specific index.
            // The strict, sequential model only allows `append_new_workspace`.
            //
            // The service could try to append the new workspace and then
            // reorder workspaces by moving windows. However, this is
            // error-prone and not recommended.
            //
            // GNOME-specific limitation: `Before` is implemented as best-effort
            // (append + switch). The workspace order may not match the request.
            // The widget should account for this on GNOME compositors and
            // potentially hide or disable the `Before` case.
            warn!(
                "GNOME service: WorkspaceCreatePosition::Before is not natively supported \
                 by GNOME's sequential workspace model. Falling back to append."
            );
            let js = "global.workspace_manager.append_new_workspace(false, global.display.get_current_time())";
            let _ = shell_proxy.eval(js).await;
        }
    }
}
```

**GNOME-specific limitation:** Inserting a workspace *before* a specific index is not natively supported by GNOME. The service implements this as best-effort (
append + switch). The widget should account for the `Before` case on GNOME compositors and potentially disable it.

#### Monitor Mapping (GNOME)

GNOME has two modes for workspace-to-monitor mapping in its settings (`org.gnome.mutter`):

1. **Workspaces on primary monitor only** (default): `workspaces-only-on-primary = true`
2. **Workspaces span all monitors**: `workspaces-only-on-primary = false`

The service must query this mode and communicate it to the widget. When `workspaces-only-on-primary` is active, the widget on non-primary monitors should be
grayed out or control the primary monitor instead of simulating an incorrect `monitor_index: 0`.

```rust
/// Query whether GNOME is configured to show workspaces only on the primary monitor.
async fn query_workspaces_only_on_primary(connection: &zbus::Connection) -> bool {
    let proxy = zbus::proxy::fdo::PropertiesProxy::new(
        connection,
        "org.gnome.Mutter",
        "/org/gnome/Mutter",
    )
        .await
        .ok();

    if let Some(proxy) = proxy {
        if let Ok(value) = proxy.get("org.gnome.Mutter", "workspaces-only-on-primary").await {
            if let zvariant::Value::Bool(b) = value {
                return b;
            }
        }
    }

    // Default: true (GNOME standard behavior)
    true
}
```

The snapshot takes this mode into account:

```rust
async fn build_workspace_snapshot(
    shell_proxy: &ShellProxy<'_>,
    connection: &zbus::Connection,
) -> WorkspaceSnapshotMessage {
    let count = query_workspace_count(shell_proxy).await.unwrap_or(0);
    let active = query_active_workspace(shell_proxy).await.unwrap_or(0);
    let only_on_primary = query_workspaces_only_on_primary(connection).await;

    // When `workspaces-only-on-primary = true`, the relevant monitor is the primary monitor.
    // When `false`, workspaces span all monitors (monitor_index stays 0).
    let monitor_index = if only_on_primary {
        // Query primary monitor via DisplayConfig.
        query_primary_monitor_index(connection).await.unwrap_or(0)
    } else {
        0
    };

    let mut ws_list = Vec::new();
    for i in 0..count {
        let name = query_workspace_name(shell_proxy, i).await.unwrap_or_else(|| i.to_string());
        ws_list.push(WorkspaceInfo {
            workspace_id: i,
            workspace_name: name.into(),
            monitor_index,
            is_active: i == active,
        });
    }

    WorkspaceSnapshotMessage {
        workspaces: ws_list.into(),
        active_workspace_id: active,
        active_monitor_index: monitor_index,
    }
}
```

**Widget recommendation:** When `workspaces-only-on-primary = true` and the widget is running on a non-primary monitor, it should either be grayed out or
control the primary monitor. This requires the widget to know its own monitor position (via GDK `Display::monitor_at_surface()`).

### 8.3 Wayland Service (`services/wayland`)

The relevant protocol is `ext-workspace-v1` (or the older `wlr-foreign-toplevel-management-unstable-v1`). This protocol allows external panels/launchers to
inspect the compositor's workspaces, read their names, and signal the compositor to focus a specific workspace. The service uses the `wayland-client` crate for
this.

#### Switch Workspace

The `ext-workspace-v1` protocol supports activating a workspace via `ExtWorkspaceHandleV1::activate()`. The service stores workspace handles in `WaylandState`
and can activate them on demand:

```rust
/// Activate a workspace by its numeric ID.
///
/// The service iterates over all tracked workspaces in `WaylandState` and
/// calls `activate()` on the matching `ExtWorkspaceHandleV1`.
fn switch_workspace(state: &mut WaylandState, workspace_id: i32) {
    for (_, ws) in &state.workspaces {
        let id_num = ws.id.parse::<i32>().unwrap_or(-1);
        if id_num == workspace_id {
            if let Some(handle) = &ws.handle {
                debug!("Wayland service: activating workspace {} ({})", workspace_id, ws.name);
                handle.activate();
            } else {
                warn!("Wayland service: no handle for workspace {}", workspace_id);
            }
            return;
        }
    }
    warn!("Wayland service: workspace {} not found", workspace_id);
}
```

The `MessageHandler` in the service forwards the `SwitchWorkspaceMessage` to this function. Since the Wayland event loop runs in a separate thread, the request
is sent via a channel to the event loop:

```rust
impl MessageHandler<FfiEnvelopePayload<SwitchWorkspaceMessage>> for WaylandService {
    fn handle_message(&self, message: FfiEnvelopePayload<SwitchWorkspaceMessage>, _sender_id: &str) {
        let workspace_id = message.0.workspace_id;
        let _ = self.command_sender.send(WaylandCommand::SwitchWorkspace(workspace_id));
    }
}
```

#### Create Workspace

The `ext-workspace-v1` protocol does not have a dedicated "create workspace" request. Workspaces are compositor-managed. For sway-compatible compositors,
workspaces can be created implicitly by switching to a non-existent workspace name:

```rust
/// Create a new workspace by switching to a non-existent workspace name.
///
/// For sway-compatible compositors, switching to an unknown workspace name
/// causes the compositor to create the workspace implicitly.
/// This is compositor-specific and may not work on all Wayland compositors.
fn create_workspace(state: &mut WaylandState, relative_to: i32, position: WorkspaceCreatePosition) {
    let new_id = match position {
        WorkspaceCreatePosition::Before => relative_to - 1,
        WorkspaceCreatePosition::After => relative_to + 1,
    };
    let new_name = format!("workspace_{}", new_id);

    // Attempt to find a workspace handle and activate with the new name.
    // For compositors that support workspace creation by name, this will
    // create the workspace. For others, the activation will silently fail.
    debug!(
        "Wayland service: creating workspace {} ({}), relative_to={}, position={:?}",
        new_id, new_name, relative_to, position
    );

    // The actual implementation depends on the compositor's capabilities.
    // The service should handle the failure gracefully and log a warning
    // if the workspace cannot be created.
    warn!(
        "Wayland service: workspace creation via ext-workspace-v1 is compositor-dependent. \
         The new workspace may not be created on all compositors."
    );
}
```

This is compositor-specific and may not work on all Wayland compositors. The service should handle the failure gracefully and log a warning.

#### Workspace Snapshot

The service already tracks all workspaces in `WaylandState::workspaces`. On snapshot request, the current state is serialized:

```rust
/// Build a workspace snapshot from the current `WaylandState`.
///
/// Iterates over all tracked workspaces, resolves their monitor index from
/// the workspace group's output mapping, and constructs a `WorkspaceSnapshotMessage`.
fn build_snapshot(state: &WaylandState) -> WorkspaceSnapshotMessage {
    let mut ws_list: Vec<WorkspaceInfo> = state
        .workspaces
        .iter()
        .map(|(_, ws)| {
            let monitor_index = ws
                .group_id
                .as_ref()
                .and_then(|gid| state.groups.get(gid))
                .and_then(|g| g.output_ids.first())
                .and_then(|oid| state.output_to_index.get(oid))
                .copied()
                .unwrap_or(0);

            WorkspaceInfo {
                workspace_id: ws.id.parse::<i32>().unwrap_or(-1),
                workspace_name: ws.name.clone().into(),
                monitor_index,
                is_active: ws.is_active,
            }
        })
        .collect();
    ws_list.sort_by_key(|w| w.workspace_id);

    let (active_id, active_monitor) = state
        .last_active
        .as_ref()
        .map(|(_, id, mon)| (*id, *mon))
        .unwrap_or((-1, 0));

    WorkspaceSnapshotMessage {
        workspaces: ws_list.into(),
        active_workspace_id: active_id,
        active_monitor_index: active_monitor,
    }
}
```

---

## 9. Configuration

### 9.1 Service Configuration

Each service needs to handle the new message types. No additional config flags are required — if `enable_workspace_tracking` is `true`, the service already
tracks workspaces and can respond to snapshot requests and switch/create commands.

### 9.2 Widget Configuration

```toml
[[plugins.workspace-switcher]]
show_label = true
show_dot_indicator = true
icon_size = 32
default_icon = "nf-md-monitor"

[plugins.workspace-switcher.icon_map]
"1" = "nf-md-numeric-1"
"2" = "nf-md-numeric-2"
"3" = "nf-md-numeric-3"
"4" = "nf-md-numeric-4"
"5" = "nf-md-numeric-5"
"6" = "nf-md-numeric-6"
"7" = "nf-md-numeric-7"
"8" = "nf-md-numeric-8"
"9" = "nf-md-numeric-9"
"10" = "nf-md-numeric-10"
```

### 9.3 CSS Classes

```css
.workspace-switcher-widget {
    padding: 4px;
}

.workspace-icon {
    font-size: 32px;
}

.workspace-label {
    font-size: 12px;
    font-weight: bold;
}

.workspace-dot-active {
    color: @theme_selected_bg_color;
    font-size: 10px;
}

.workspace-dot-inactive {
    color: @theme_unfocused_fg_color;
    font-size: 10px;
}
```

---

## 10. Implementation Roadmap

### Phase 1: Model Extensions (`model/workspace` — `smearor-model-compositor`)

Add the new message types, topics, and `TypedMessage` / `SharedMessage` implementations to the existing `smearor-model-compositor` crate.

**Deliverables:**

- New module `model/workspace/src/switcher.rs` (or extend `messages.rs`) with `WorkspaceInfo`, `WorkspaceSnapshotMessage`, `WorkspaceSnapshotRequestMessage`,
  `SwitchWorkspaceMessage`, `CreateWorkspaceMessage`, `WorkspaceCreatePosition`
- Update `model/workspace/src/lib.rs` with `pub use` re-exports
- All types carry `#[stabby::stabby]`
- `generate_type_id` for each `TypedMessage` impl

**Exit criteria:** `cargo build -p smearor-model-compositor` succeeds.

### Phase 2: Service Extensions

Extend each compositor service with `MessageHandler` implementations for `SwitchWorkspaceMessage`, `CreateWorkspaceMessage`, and
`WorkspaceSnapshotRequestMessage`. Add workspace snapshot broadcasting.

**Deliverables:**

- `services/hyprland`: `MessageHandler<SwitchWorkspaceMessage>`, `MessageHandler<CreateWorkspaceMessage>`, `MessageHandler<WorkspaceSnapshotRequestMessage>`,
  snapshot builder
- `services/gnome`: Same handlers, D-Bus-based implementation
- `services/wayland`: Same handlers, protocol-based implementation
- Each service registers JSON converters for the new message types

**Exit criteria:** `cargo build` succeeds for all three service crates. Manual test: send a `SwitchWorkspaceMessage` via the message system and verify the
compositor switches workspaces.

### Phase 3: Widget Crate (`plugins/workspace-switcher`)

Implement the GTK4 widget with swipe gestures, dynamic view list, icon resolution, dot indicator, and event handling.

**Deliverables:**

- `plugins/workspace-switcher/Cargo.toml`
- `plugins/workspace-switcher/src/lib.rs`
- `plugins/workspace-switcher/src/config.rs`
- `plugins/workspace-switcher/src/widget.rs`
- Swipe-up/swipe-down with edge-detection for workspace creation
- `AcceptTopic` for `TOPIC_WORKSPACE_SNAPSHOT`, `TOPIC_WORKSPACE_CHANGED`, `TOPIC_WORKSPACE_LIFECYCLE`
- `MessageHandler` implementations for all three event types
- Icon resolution via `resolve_gtk_nerd_icon()`
- Dot indicator

**Exit criteria:** `cargo build -p smearor-workspace-switcher` succeeds. Widget loads in the launcher, displays the current workspace, and swipe gestures switch
workspaces.

### Phase 4: Integration & Testing

Wire the widget into the launcher configuration, test with all three compositors, and verify edge cases.

**Deliverables:**

- Launcher config example with workspace-switcher plugin
- Test: swipe through all workspaces
- Test: swipe past last workspace creates a new one
- Test: swipe past first workspace creates a new one before
- Test: workspace snapshot on startup
- Test: workspace lifecycle events update the view list
- Test: icon map resolution

**Exit criteria:** All tests pass on at least one compositor (Hyprland recommended for initial testing).

---

## 11. Dependencies

### 11.1 Model Crate

| Crate                               | Purpose                                                             |
|-------------------------------------|---------------------------------------------------------------------|
| `stabby`                            | ABI-stable types for FFI                                            |
| `smearor-swipe-launcher-plugin-api` | `TypedMessage`, `SharedMessage`, `MessageTopic`, `generate_type_id` |

### 11.2 Widget Crate

| Crate                               | Purpose                                                                                                                                             |
|-------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------|
| `gtk4`                              | GTK4 widgets                                                                                                                                        |
| `glib`                              | GLib utilities (`MainContext::spawn_local`)                                                                                                         |
| `smearor-swipe-launcher-plugin-api` | Plugin API, `WidgetBuilder`, `MessageBroadcaster`, `resolve_gtk_nerd_icon`                                                                          |
| `smearor-model-compositor`          | `WorkspaceChangedEvent`, `WorkspaceLifecycleEvent`, `WorkspaceInfo`, `SwitchWorkspaceMessage`, `CreateWorkspaceMessage`, `WorkspaceSnapshotMessage` |
| `serde`                             | Configuration deserialization                                                                                                                       |
| `typed-builder`                     | Typed builder for config                                                                                                                            |
| `tracing`                           | Logging                                                                                                                                             |

### 11.3 Service Extensions

No new dependencies — the services already have all required crates (`hyprland`, `zbus`, `wayland-client`, `tokio`, etc.).

---

## 12. Future Enhancements

- **Workspace renaming**: Long-press on a workspace to rename it (requires compositor support).
- **Workspace deletion**: Swipe a workspace away with a horizontal gesture to destroy it.
- **Per-monitor filtering**: Only show workspaces on the monitor the launcher instance is placed on.
- **Workspace thumbnails**: Show a mini preview of the workspace content instead of just an icon.
- **Drag-to-reorder**: Long-press and drag to reorder workspaces (requires compositor support for workspace reordering).
- **Scroll gesture**: Add `EventControllerScroll` for mouse wheel workspace switching.
- **Workspace groups**: Support for workspace groups (Hyprland) or workspace sets (GNOME 46+).
