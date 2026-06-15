# Bidirectional Communication & Widget System Architecture

This document describes the design and architecture for a flexible, loose-coupled, multi-threaded, and asynchronous bidirectional communication system between
the Core Application (`smearor-swipe-launcher`) and dynamically loaded plugins over the FFI boundary.

---

## 1. Core Architectural Goals

To support modern, highly interactive widgets running on high-refresh-rate touchscreens (120 Hz on 65" tables), the communication layer must meet the following
criteria:

* **Loose Coupling:** Plugins and the Core should not depend on each other's concrete Rust structures or compile-time enum schemas. Adding a new widget or
  message type must not require changing or recompiling existing components.
* **Extensible & Flexible Message Types:** Message schemas must support complex payloads that can evolve over time.
* **Multi-threaded & Asynchronous:** Heavy tasks (e.g., networking, database/DBus, file parsing) must run concurrently on a thread pool and never block the GTK
  main UI thread.
* **FFI Safety:** Message passing across library boundaries (`libloading`) must respect Rust's ownership model, prevent allocator mismatches, and utilize
  ABI-stable data types via `abi_stable`.

---

## 2. The Solution: Topic-Based JSON Pub/Sub over FFI

Instead of rigid Rust enums, we implement a **Topic-Based Event Bus** utilizing an ABI-stable message envelope containing a string-based topic and a serialized
JSON payload.

### 2.1 The Message Envelope

We define an ABI-stable structure for cross-FFI message passing:

```rust
use abi_stable::StableAbi;
use abi_stable::std_types::RString;

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct FfiEnvelope {
    pub sender_id: RString,
    pub topic: RString,
    pub payload: RString, // Serialized JSON payload
}
```

* **`sender_id`:** Uniquely identifies the message originator (e.g., `"clock_widget_1"` or `"core"`).
* **`topic`:** A slash-separated channel string indicating the message destination/meaning (e.g., `"core/window"`, `"plugins/broadcast/settings"`,
  `"plugin/app_launcher/launch"`).
* **`payload`:** A flexible, schema-free JSON payload. This lets plugins define their own data structures entirely, achieving maximum loose coupling.

---

## 3. Bidirectional Interface Contract

To enable bidirectional communication, the Core and the Plugins register callbacks across the FFI boundary.

### 3.1 Plugin-to-Core (Downstream)

When a plugin is instantiated, the Core passes a `FfiCoreContext` which exposes an FFI function to send messages downstream to the Core's event broker.

```rust
#[repr(C)]
#[derive(StableAbi)]
pub struct CoreContextVTable {
    pub publish_message: unsafe extern "C" fn(context: *mut (), message: FfiEnvelope),
}
```

### 3.2 Core-to-Plugin (Upstream)

To allow the Core (or other plugins) to send messages upstream, we extend the `PluginVTable` to include an `on_message` callback:

```rust
#[repr(C)]
#[derive(StableAbi)]
pub struct PluginVTable {
    pub get_id: unsafe extern "C" fn(instance: *mut ()) -> RString,
    pub build_widget: unsafe extern "C" fn(instance: *mut ()) -> FfiWidget,
    pub on_message: unsafe extern "C" fn(instance: *mut (), message: FfiEnvelope),
    pub on_primary_action: unsafe extern "C" fn(instance: *mut (), rotation: u32) -> i32,
    pub on_secondary_action: unsafe extern "C" fn(instance: *mut (), rotation: u32) -> i32,
    pub destroy: unsafe extern "C" fn(instance: *mut ()),
}
```

When the Core receives an envelope, the central broker routes it to all subscribed plugins by invoking their respective `on_message` FFI function.

---

## 4. Scenario Mapping (Examples 1-5)

### Example 1: Plugin wants the Launcher to close

* **Originator:** A widget plugin (e.g., app-launcher after starting an app).
* **Action:** Publishes an envelope to the Core control topic.
* **Envelope:**
  ```json
  {
    "sender_id": "app_launcher_1",
    "topic": "core/control",
    "payload": "{ \"action\": \"RequestClose\" }"
  }
  ```
* **Handling:** The Core's Event Broker receives the envelope, extracts the `"action"`, and triggers `gtk_app.quit()` safely.

### Example 2: Plugin wants the Launcher to scroll to its Widget

* **Originator:** A widget plugin (e.g., timer widget whose countdown finishes).
* **Action:** Publishes an envelope to request focus on its slot.
* **Envelope:**
  ```json
  {
    "sender_id": "timer_widget_1",
    "topic": "core/layout",
    "payload": "{ \"action\": \"FocusWidget\", \"plugin_id\": \"timer_widget_1\" }"
  }
  ```
* **Handling:** The Core's Event Broker receives the message on `core/layout` and schedules a task on the GTK main loop. The GTK thread finds the widget
  associated with `timer_widget_1` and calls `.scroll_to_child(widget)` on the `ScrolledWindow`.

### Example 3: Core wants to send a setting to a specific Plugin

* **Originator:** The Core application (e.g., reading a config update or CLI parameter).
* **Action:** Core directly invokes the targeted plugin's `on_message` FFI function.
* **Envelope:**
  ```json
  {
    "sender_id": "core",
    "topic": "plugin/clock_widget_left/settings",
    "payload": "{ \"format\": \"[hour]:[minute]\", \"timezone\": \"CET\" }"
  }
  ```
* **Handling:** The clock widget receives the update, parses the payload, updates its internal state, and schedules an interface refresh.

### Example 4: Core wants to send a setting to ALL Plugins

* **Originator:** The Core application (e.g., a global configuration change, theme swap, or system-wide scale update).
* **Action:** Core broadcasts the envelope to all active plugins via their `on_message` callbacks.
* **Envelope:**
  ```json
  {
    "sender_id": "core",
    "topic": "plugins.broadcast.settings",
    "payload": "{ \"theme\": \"dark\", \"animations_enabled\": true }"
  }
  ```
* **Handling:** Every loaded plugin receives the message, parses the shared setting, and instantly re-skins its widget.

### Example 5: Plugin A wants to signal Plugin B

* **Originator:** Plugin A (e.g., a music-controller plugin).
* **Action:** Publishes an envelope targeting Plugin B's topic (or a shared system topic).
* **Envelope:**
  ```json
  {
    "sender_id": "music_controller_1",
    "topic": "plugin/clock_widget_left/visual_sync",
    "payload": "{ \"action\": \"bounce_on_beat\", \"bpm\": 120.0 }"
  }
  ```
* **Handling:** The Core Event Broker acts as a router. It receives the envelope, matches the topic, and calls `on_message` on Plugin B (`clock_widget_left`).
  The plugins remain completely decoupled; Plugin A does not need to link against Plugin B.

---

## 5. Async Runtime & Library Evaluation

To implement this design, we evaluate several options for the async engine and messaging model:

| Metric / Library    | Option A: Tokio Channels (Recommended)                          | Option B: Actix / Actor Model                   | Option C: EventBus Crates |
|:--------------------|:----------------------------------------------------------------|:------------------------------------------------|:--------------------------|
| **Performance**     | **Extremely High** (Lock-free channels, zero-overhead)          | **High**                                        | Medium                    |
| **Complexity**      | **Very Low** (Simple mpsc/broadcast, minimal boilerplate)       | **High** (Large actor macros, heavy lifecycles) | Low                       |
| **Multi-threading** | **Excellent** (Built-in multi-threaded work-stealing scheduler) | **Excellent**                                   | Medium                    |
| **Thread-Safety**   | **Perfect** (Send/Sync traits enforced at compile-time)         | **Perfect**                                     | High                      |
| **GTK Integration** | **Seamless** (Via GLib async channels/context)                  | Moderate (Requires custom runner integration)   | Easy                      |

### Recommendation: Tokio + Standard Channels + DashMap

We recommend utilizing a **Multi-threaded Tokio Runtime** inside the Core, paired with **`tokio::sync::broadcast`** for broadcasting and **`dashmap`** for
thread-safe subscription routing.

* **Tokio:** The industry standard for asynchronous Rust. Running a multi-threaded Tokio runtime allows plugins to spawn background tasks (such as D-Bus
  watchers, network polling, and disk I/O) that run concurrently without ever affecting the UI.
* **DashMap:** A lightning-fast, concurrent hash map used by the central broker to maintain active subscriptions to specific topic patterns.
* **Why not Actix?** While Actix is an excellent framework, the actor model introduces substantial boilerplate (defining message types, implementing
  `Handler<M>`, setting up addresses). Additionally, Actix is tightly coupled to its own thread-affinity runtime (`actix-rt`), which can conflict with GTK's
  strict single-threaded main-loop requirements. Tokio channels are lightweight, highly transparent, and integrate beautifully with the GTK main loop.

---

## 6. GTK & Multi-Threading Integration (How They Coordinate)

Since GTK 4 is single-threaded, any UI manipulation must occur on the main thread. To prevent background task threads from causing undefined behavior, we
coordinate them using standard GLib main-context channels:

```
  +-------------------------------------------------------+
  |                   GTK MAIN THREAD                     |
  |                                                       |
  |  [ GTK Main Loop ] <======== (glib::Receiver) <====+  |
  |          |                                         |  |
  |          | (UI Events)                             |  |
  +----------|-----------------------------------------|--+
             |                                         |
             v                                         | (Safe Cross-Thread Bridge)
  +-------------------------------------------------+  |
  |             TOKIO ASYNC RUNTIME                 |  |
  |                                                 |  |
  |  [ Multi-Threaded Async Thread Pool ]           |  |
  |                                                 |  |
  |  +-------------------------------------------+  |  |
  |  |            CENTRAL EVENT BROKER           |  |  |
  |  |                                           |  |  |
  |  |  Subscribers:                             |--|--+
  |  |  - Core Receiver (routes to UI/Layout)    |  |
  |  |  - Plugins (routes to `on_message` FFI)   |  |
  |  +-------------------------------------------+  |
  |                                                 |
  +-------------------------------------------------+
```

### 6.1 The Communication Pipeline

1. **Background Tasks:** A plugin spawns an async task on the Tokio runtime (e.g., listening for a D-Bus signal).
2. **FFI Downstream:** When a D-Bus signal arrives, the plugin constructs an `FfiEnvelope` and calls the Core's `publish_message` callback.
3. **Async Processing:** The Core's Event Broker receives the envelope asynchronously in the Tokio worker thread pool.
4. **Thread-Safe UI Update:** If the message targets a UI update (e.g., scrolling), the broker forwards the data over a `glib::MainContext::channel()`.
5. **GTK Execution:** The receiver on the GTK main thread intercepts the message and executes the UI update safely.

---

## 7. Next Steps for Implementation

To make the codebase fully support this architecture, we will implement the following changes:

1. **Extend API:** Add `FfiEnvelope` in `smearor-plugin-api/src/messages.rs`.
2. **Upgrade `PluginVTable` & `CoreContextVTable`:** Add bidirectional functions.
3. **Build Core Broker:** Create a thread-safe message broker using Tokio and DashMap inside `smearor-swipe-launcher/src/controller.rs` or a new `broker.rs`
   file.
4. **Setup Tokio Runtime:** Integrate `#[tokio::main(flavor = "multi_thread")]` in `main.rs` to spawn the async background threads.
