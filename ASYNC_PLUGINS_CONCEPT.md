# Async Plugin Architecture Concept

## 1. Problem Statement

The current plugin system suffers from several architectural bottlenecks that compound as the number of plugins grows. Every existing plugin (clock, audio,
MPRIS, notifications, app-launcher, button) exhibits the same patterns, and every future plugin would inherit the same problems.

### 1.1 JSON Serialization Overhead

All inter-plugin communication passes through `FfiEnvelope`, which carries the payload as a serialized JSON string (`RString`). Even though the Host and the
Plugin share the exact same `model` crate and therefore know the exact memory layout of the message types, the current architecture forces:

- **Serialization** on the sender side (`serde_json::to_string`)
- **Deserialization** on the receiver side (`serde_json::from_str`)
- **String allocation** for every message

This is entirely unnecessary when both sides share the same type definitions. For high-frequency messages (e.g., MPRIS position updates, audio volume changes,
notification status broadcasts) this overhead dominates the actual business logic.

### 1.2 Blocking Threads and Polling

Services currently spawn dedicated OS threads via `std::thread::spawn`. Inside these threads:

- zbus (a fully async library) is forced into blocking patterns
- The notification service uses `std::sync::mpsc::channel` with `recv_timeout(Duration::from_millis(500))`
- Widgets use `glib::timeout_add_local(Duration::from_millis(50..100))` to poll `std::sync::mpsc::Receiver::try_recv()`

This creates a constant background load of context switches and wasted CPU cycles even when no messages are flowing. The system is **polling-driven** rather
than **event-driven**.

### 1.3 No Access to the Host Async Runtime

The Host already runs a `tokio::runtime::Handle`, but Plugins have no access to it. This means:

- Plugins cannot spawn tasks onto the Host's thread pool
- Async I/O libraries (zbus, libpulse) cannot be used idiomatically inside Plugins
- Every Plugin that needs async I/O must either spawn its own thread or block the GTK main thread

### 1.4 abi_stable Limitations

While `abi_stable` provides stable ABI types (`RString`, `RResult`, `RRef`), it does not provide a stable ABI for **Futures**. This makes it impossible to pass
async closures or `BoxFuture` across the FFI boundary safely. The ecosystem is also effectively maintenance-only; the Rust community is converging on `stabby`
for new projects requiring ABI stability with modern Rust features.

---

## 2. Architecture Concept

The new architecture is built on three pillars:

1. **`stabby` for all FFI boundaries** — ABI-stable types, trait objects, and Futures
2. **Shared `stabby` types in `model` crates** — Zero-copy, zero-serialization message passing
3. **`PluginExecutor` — Host runtime delegation** — Plugins spawn Futures onto the Host's Tokio runtime

### 2.1 stabby FFI Traits

The `plugin-api` crate replaces manual VTables with `#[stabby::stabby]` traits. stabby automatically generates ABI-stable v-tables (`PluginDyn`, `ServiceDyn`)
and provides `dynptr!`/`Dyn`/`DynRef` for type-safe trait objects across FFI.

> **Note:** `SharedMessage` cannot be a stabby trait because stabby traits do not support trait-object parameters in their methods. Therefore, message passing
> continues to use raw pointers with `type_id` for manual down-casting. The `Plugin` and `Service` traits below are conceptual; the actual implementation uses
> manual `#[repr(C)]` VTables.

```rust
#[stabby::stabby]
pub trait Plugin {
    extern "C" fn id(&self) -> stabby::string::String;
    extern "C" fn display_name(&self) -> stabby::string::String;
    extern "C" fn icon_name(&self) -> stabby::option::Option<stabby::string::String>;
    extern "C" fn build_widget(&mut self) -> FfiWidget;
    extern "C" fn on_message(&mut self, message: *mut core::ffi::c_void, type_id: u64);
}

#[stabby::stabby]
pub trait Service {
    extern "C" fn id(&self) -> stabby::string::String;
    extern "C" fn display_name(&self) -> stabby::string::String;
    extern "C" fn on_message(&mut self, message: *mut core::ffi::c_void, type_id: u64);
}
```

**Why traits instead of manual VTables?**

- **No boilerplate:** `destroy`, `get_id`, `on_message` — all generated automatically by stabby
- **Type safety:** `stabby::dynptr!(Box<dyn Plugin>)` carries the v-table inline; no raw `*mut ()` pointers
- **Drop handled automatically:** When the stabby trait object goes out of scope, the destructor is called via the v-table

### 2.2 PluginExecutor — Delegated Async Runtime

At plugin initialization the Host passes a `PluginExecutor` struct. This gives the Plugin a stable function pointer to spawn `stabby::future::DynFuture` values
onto the Host's Tokio worker threads:

```rust
#[stabby::stabby]
pub struct PluginExecutor {
    /// Opaque pointer to the Host's `tokio::runtime::Handle`.
    /// The Host guarantees this pointer remains valid for the entire plugin lifetime.
    pub context: *const core::ffi::c_void,
    /// Spawn a future on the Host's async runtime.
    /// `stabby::future::DynFuture` already implies `Send + Sync`.
    /// (Note: `stabby::compiler::Send` does not exist in stabby 72.1.8;
    ///  stabby uses separate type aliases instead: `DynFuture` = Send+Sync,
    ///  `DynFutureUnsync` = Send, `DynFutureUnsend` = no bound.)
    pub spawn: extern "C" fn(
        context: *const core::ffi::c_void,
        future: stabby::future::DynFuture<'static, ()>,
    ),
}
```

**Host-side implementation:**

```rust
unsafe extern "C" fn host_spawn(
    context: *const core::ffi::c_void,
    future: stabby::future::DynFuture<'static, ()>,
) {
    let handle = &*(context as *const tokio::runtime::Handle);
    handle.spawn(async move { future.await });
}
```

When a Plugin calls `(executor.spawn)(...)`, the Future seamlessly executes on the Host's Tokio worker threads. The Host initializes the executor during plugin
load. The handle is stored in a `Box` on the heap; the pointer remains valid for the plugin's entire lifetime and is reclaimed when the Host shuts down:

```rust
let host_runtime_handle: Box<tokio::runtime::Handle> = Box::new(runtime.handle().clone());
let executor = PluginExecutor {
context: Box::into_raw(host_runtime_handle) as * const c_void,
spawn: host_spawn,
};
```

### 2.3 Zero-Copy Messages (Ansatz A)

Since the Host and every Plugin share the same `model` crates, messages are passed **directly as stabby types** without any serialization layer.

#### 2.3.1 Model Crate Changes

Every `model/<plugin>` crate migrates from `std` collections to `stabby` collections:

```rust
// Before (model/notifications)
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NotificationInfo {
    pub app_name: String,
    pub actions: Vec< < NotificationAction>,
    pub icon: Option<String>,
}

// After (model/notifications)
#[stabby::stabby]
#[derive(Clone, Debug)]
pub struct NotificationInfo {
    pub app_name: stabby::string::String,
    pub actions: stabby::vec::Vec< < NotificationAction>,
    pub icon: stabby::option::Option<stabby::string::String>,
}
```

Unit enums (e.g., `AudioCommandAction`, `MprisPlaybackStatus`) become `#[stabby::stabby]` enums with explicit discriminant sizes.

#### 2.3.2 Message Broker Changes

The Host's message broker stops serializing and instead passes a type-erased stable trait object:

```rust
#[stabby::stabby]
pub trait SharedMessage {
    fn topic(&self) -> stabby::string::String;
    /// A stable type identifier (e.g., a hash of the type name) defined in the shared `model` crate.
    fn type_id(&self) -> u64;
}
```

The broker routes `FfiEnvelope` (containing a raw pointer + `type_id`) from sender to receiver. The receiver down-casts using the stable `type_id` and accesses
the
message fields directly from memory:

```rust
extern "C" fn on_message(plugin: *mut (), msg: *mut core::ffi::c_void, type_id: u64) {
    if type_id == NOTIFICATION_STATUS_TYPE_ID {
        let status = unsafe { &*(msg as *const NotificationStatusMessage) };
        // Use directly — no copy, no deserialization
    }
}
```

**No JSON. No postcard. No rkyv. Pure pointer passing.**

### 2.4 Async Plugin Lifecycle

#### Initialization Flow

1. Host loads the plugin `.so` via `libloading`
2. Host calls `smearor_plugin_create` (or `smearor_service_create`), passing:
    - `PluginConfig` (stabby-compatible config struct)
    - `PluginExecutor` (access to Host Tokio runtime)
    - `MessageBrokerHandle` (stable broker reference for sending messages)
3. Plugin stores the executor and broker handle in its state
4. Plugin may immediately spawn background tasks via `(executor.spawn)(...)`

#### Why pass `PluginExecutor` to the constructor?

The constructor (`new`) receives the `PluginExecutor` directly rather than a separate `init` phase. This is simpler: the plugin has everything it needs from the
first line of its own code.

**Are there disadvantages?**

| Concern                                              | Reality                                                                                                                                                                                                                                       |
|------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Constructor cannot be `async`                        | True, but this is inherent to FFI constructors regardless of the Executor pattern. Async initialization is deferred to a spawned task.                                                                                                        |
| Simple plugins (e.g., clock) don't need the Executor | The parameter can be ignored; it is a zero-cost parameter if unused.                                                                                                                                                                          |
| Constructor signature grows                          | The gain (single entry point, no `init` boilerplate) outweighs the cosmetic cost.                                                                                                                                                             |
| Spawning in `new` then failing                       | If `new` spawns a task and then returns `Err`, the Host leaks the task. **Mitigation:** do not spawn in `new`. Store the `Executor` in the struct and spawn in a dedicated `start()` method called by the Host after successful construction. |

**Revised pattern:**

```rust
impl NotificationService {
    pub fn new(
        config: PluginConfig,
        executor: PluginExecutor,
        broker: MessageBrokerHandle,
    ) -> Result<Self, PluginInitError> {
        // Construction only — no spawning
        Ok(Self { meta: PluginMeta::try_from(&config)?, executor, broker, state: ... })
    }

    pub fn start(&self) {
        // Now safe to spawn — construction succeeded
        (self.executor.spawn)(
            self.executor.context,
            DynFuture::new(async move {
                self.run_dbus_listener().await;
            }),
        );
    }
}
```

#### Widget Build Flow

1. Host calls `build_widget` on the trait object (`dyn Plugin`)
2. Plugin returns an `FfiWidget` (raw GTK pointer, unchanged from current architecture)
3. GTK widgets remain bound to the GTK main thread; async updates are bridged via `glib::MainContext::default().spawn_local`

#### Message Send Flow

1. Plugin constructs a message using shared `model` types
2. Plugin boxes the message and calls `broker.send(FfiEnvelope { type_id: T::TYPE_ID, payload: Box::into_raw(...), ... })`
3. Host broker routes to target plugin(s)
4. Receiving plugin receives raw pointer + `type_id`, down-casts, and handles

### 2.5 Service Architecture

Services become pure Tokio tasks spawned via the `PluginExecutor`:

```rust
pub struct NotificationService {
    meta: PluginMeta,
    executor: PluginExecutor,
    broker: MessageBrokerHandle,
}

impl NotificationService {
    pub fn new(
        config: PluginConfig,
        executor: PluginExecutor,
        broker: MessageBrokerHandle,
    ) -> stabby::result::Result<Self, PluginInitError> {
        // Construction only — no async spawning yet
        Ok(Self {
            meta: PluginMeta::try_from(&config)?,
            executor,
            broker,
        })
    }

    pub fn start(&self) {
        // Spawn the zbus listener directly on the Host's Tokio runtime.
        // DynFuture already implies Send + Sync; no explicit bound needed.
        (self.executor.spawn)(
            self.executor.context,
            stabby::future::DynFuture::new(async move {
                self.run_dbus_listener().await;
            }),
        );
    }

    async fn run_dbus_listener(&self) {
        // Pure async zbus code — no std::thread::spawn, no polling
        let connection = zbus::Connection::session().await.unwrap();
        // ...
    }
}
```

### 2.6 Macro Implementation with `stabby::export` and `stabby::libloading`

stabby provides `#[stabby::export]` to generate `#[no_mangle]` entry points with automatic type-report verification. Plugins export their constructors with
`#[stabby::export]`, and the Host loads them via `stabby::libloading::StabbyLibrary::get_stabbied`, which verifies the type reports at runtime before returning
the function pointer. This eliminates manual ABI mismatches and prevents segfaults from incompatible plugins.

#### Plugin-side macro (`widget_plugin!`)

> **Note:** The original concept used `stabby::dyn::DynBox<dyn Plugin>`, but `DynBox` does not exist in stabby 72.1.8. The actual implementation uses manual
`#[repr(C)]` VTables with version checking.

```rust
#[macro_export]
macro_rules! widget_plugin {
    ($widget_type:ty) => {
        #[stabby::export]
        pub extern "C" fn smearor_plugin_create(
            config_json: *const i8,
            config_len: usize,
            executor: $crate::PluginExecutor,
            broker: $crate::MessageBrokerHandle,
        ) -> stabby::result::Result<
            $crate::PluginContainer,
            $crate::PluginConstructionErrorWrapper,
        > {
            let config = $crate::PluginConfig::new(config_json, config_len)?;
            let widget = <$widget_type>::new(config, executor, broker)?;
            Ok($crate::PluginContainer {
                instance: Box::into_raw(Box::new(widget)) as *mut core::ffi::c_void,
                vtable: &$crate::PluginVTable {
                    destroy: destroy_fn,
                    build_widget: build_widget_fn,
                    on_message: on_message_fn,
                    start: start_fn,
                },
                vtable_version: $crate::PLUGIN_VTABLE_VERSION,
            })
        }
    };
}
```

What `#[stabby::export]` generates automatically:

- `smearor_plugin_create` — the actual constructor (`#[no_mangle]`, `extern "C"`)
- `smearor_plugin_create_stabbied` — type-report checker for runtime ABI verification
- `smearor_plugin_create_stabbied_report` — debug info if types mismatch

#### Service-side macro (`service_plugin!`)

> **Note:** The original concept used `stabby::dyn::DynBox<dyn Service>`, but `DynBox` does not exist in stabby 72.1.8. The actual implementation uses manual
`#[repr(C)]` VTables with version checking.

```rust
#[macro_export]
macro_rules! service_plugin {
    ($service_type:ty) => {
        #[stabby::export]
        pub extern "C" fn smearor_service_create(
            config_json: *const i8,
            config_len: usize,
            executor: $crate::PluginExecutor,
            broker: $crate::MessageBrokerHandle,
        ) -> stabby::result::Result<
            $crate::ServiceContainer,
            $crate::PluginConstructionErrorWrapper,
        > {
            let config = $crate::PluginConfig::new(config_json, config_len)?;
            let service = <$service_type>::new(config, executor, broker)?;
            Ok($crate::ServiceContainer {
                instance: Box::into_raw(Box::new(service)) as *mut core::ffi::c_void,
                vtable: &$crate::ServiceVTable {
                    destroy: destroy_fn,
                    get_id: get_id_fn,
                    on_message: on_message_fn,
                    start: start_fn,
                },
                vtable_version: $crate::SERVICE_VTABLE_VERSION,
            })
        }
    };
}
```

#### Host-side plugin loading (`stabby::libloading`)

The Host uses `stabby::libloading::StabbyLibrary` to load plugin constructors at runtime with automatic ABI verification:

```rust
use stabby::libloading::StabbyLibrary;
use smearor_swipe_launcher_plugin_api::PluginConstructor;

let library = libloading::Library::new("libplugin.so") ?;
let constructor = unsafe {
library.get_stabbied::<PluginConstructor>(b"smearor_plugin_create")
}.map_err( | e| LauncherError::PluginStabbiedLoadError(e.to_string())) ?;
```

`get_stabbied` looks up `smearor_plugin_create_stabbied_v3` (generated by `#[stabby::export]`) and compares the type report of the plugin's constructor with the
Host's expected type report. If they mismatch (e.g., different parameter order, incompatible stabby version, or different compiler settings), the load fails
gracefully with a detailed error message instead of a segfault.

For services, the same pattern is used with `ServiceConstructor`:

```rust
let constructor = unsafe {
library.get_stabbied::<ServiceConstructor>(b"smearor_service_create")
}.map_err( | e| LauncherError::PluginStabbiedLoadError(e.to_string())) ?;
```

---

## 3. Advantages, Disadvantages, and Problem Resolution

### 3.1 Advantages

| Advantage                           | How It Solves the Problem                                                                                                                                               |
|-------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Zero serialization overhead**     | `stabby` types pass directly through FFI as stable memory layouts. No JSON, no postcard, no rkyv. The sender writes to memory; the receiver reads from the same layout. |
| **Event-driven instead of polling** | `tokio::sync::mpsc` channels replace `timeout_add_local` + `try_recv`. CPU usage drops to zero when idle.                                                               |
| **Unified async runtime**           | Plugins reuse the Host's Tokio runtime via `PluginExecutor`. No more `std::thread::spawn` per service.                                                                  |
| **Idiomatic async I/O in plugins**  | zbus, libpulse, and other async libraries can be used natively. No blocking wrappers or manual thread management.                                                       |
| **Scalable architecture**           | Every new plugin automatically inherits the async, zero-copy, event-driven foundation. No repeated `timeout_add_local` boilerplate.                                     |
| **Future-proof ABI**                | `stabby` supports async/await, generics, and trait objects across FFI — capabilities that `abi_stable` lacks.                                                           |

### 3.2 Disadvantages and Risks

| Disadvantage                                 | Mitigation                                                                                                                                                                                                                                                                                    |
|----------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **`stabby` is pre-1.0**                      | Pin to a specific minor version via `Cargo.lock`. Monitor the `stabby` repository for breaking changes. The API surface we use (structs, enums, `DynFuture`) is stable in practice. (`DynBox` does not exist in stabby 72.1.8; we use manual VTables instead.)                                |
| **Breaking change for all plugins**          | All 8 model crates, 6 plugins, and 4 services must migrate. Mitigate by migrating one plugin pair (service + widget) as a Proof of Concept before touching the others.                                                                                                                        |
| **GTK main thread constraint remains**       | Widgets must still update GTK objects on the main thread. Use `glib::MainContext::default().spawn_local` inside async handlers. This is a GTK limitation, not an architecture flaw.                                                                                                           |
| **Conversion overhead for external APIs**    | Libraries like zbus return `std::string::String`; these must be converted to `stabby::string::String` before crossing the model boundary. This is a one-time cost at the service edge, not per message.                                                                                       |
| **Lifetime management of `context` pointer** | Using `Arc::into_raw` would leak memory on plugin unload. Instead, store the `Handle` in a `Box` on the Host heap and pass `Box::into_raw`. Reclaim with `Box::from_raw` during Host shutdown.                                                                                                |
| **Thread-safety of plugin Futures (`Send`)** | `stabby::future::DynFuture` already implies `Send + Sync` at the type level. Plugin developers cannot accidentally capture non-`Send` state in futures spawned on the Host runtime. (Note: `stabby::compiler::Send` does not exist in stabby 72.1.8; separate type aliases are used instead.) |
| **GTK main context bridge deadlock risk**    | Never use `block_on` or synchronous waits inside `spawn_local`. The flow must always be unidirectional: Tokio task → async channel → `spawn_local` → GTK update. No back-pressure that blocks the GTK thread.                                                                                 |

### 3.3 Why This Solves the Current Problems

| Current Problem                                 | New Architecture Solution                                                                          |
|-------------------------------------------------|----------------------------------------------------------------------------------------------------|
| JSON serialization dominates message cost       | Eliminated entirely — messages are stabby types passed by pointer                                  |
| `std::thread::spawn` + `recv_timeout` polling   | Replaced by Tokio tasks + async channels — event-driven, no polling                                |
| `glib::timeout_add_local(50ms)` polling widgets | Replaced by `tokio::sync::mpsc` receivers mapped to `glib::MainContext::spawn_local`               |
| Plugins cannot use Host Tokio runtime           | `PluginExecutor` provides a stable FFI function pointer that delegates `spawn` to the Host runtime |
| `abi_stable` cannot express async FFI           | `stabby::future::DynFuture` is a stable ABI replacement for `BoxFuture`                            |

### 3.4 Implementation Gotchas

Three specific problems must be handled carefully during implementation:

**Gotcha A — `Arc::into_raw` Memory Leak:** Wrapping the `tokio::runtime::Handle` in an `Arc` and calling `Arc::into_raw` leaks memory on plugin unload because
the reference count never reaches zero. **Fix:** Store the handle in a `Box` and pass `Box::into_raw`. The Host reclaims it with `Box::from_raw` on shutdown.

**Gotcha B — Thread-Safety of Spawned Futures:** `tokio::runtime::Handle::spawn` may execute the future on any worker thread. If the plugin captures non-`Send`
state (e.g., GTK widgets, `Rc<RefCell<T>>`), this is undefined behavior. **Fix:** Use `stabby::future::DynFuture` in the `PluginExecutor`
signature. This type alias already implies `Send + Sync`; the compiler rejects non-thread-safe futures.
/// NOTE: `stabby::compiler::Send` does not exist in stabby 72.1.8. Stabby uses separate type aliases
/// (`DynFuture`, `DynFutureUnsync`, `DynFutureUnsend`) rather than generic Send bounds.

**Gotcha C — GTK Main Context Deadlock:** If a `spawn_local` task on the GTK main thread ever blocks waiting for a Tokio future (e.g., `block_on`, `.await` on a
non-`LocalSet` future), the GTK thread deadlocks because Tokio cannot complete the future without the GTK thread making progress. **Fix:** The bridge must be
strictly unidirectional:

```
Tokio worker thread          GTK main thread
        |                          |
        |-- async channel -------->|
        |                          |-- glib::MainContext::spawn_local
        |                          |   update GTK widgets
```

Never use `block_on`, never `.await` a Tokio future inside `spawn_local`.

---

## 4. Implementation Roadmap

### Phase 1 — Foundation: Plugin-API on stabby

**Scope:** `plugin-api` crate only. No model or plugin changes yet.

- [x] Add `stabby` dependency to workspace `Cargo.toml`
- [x] Define `Plugin` and `Service` traits (normal Rust traits; `#[stabby::stabby]` traits blocked by stabby's trait-object limitation)
- [x] Define `PluginExecutor`, `SharedMessage` trait, and `MessageBrokerHandle`
- [x] Rewrite `widget_plugin!` macro using `#[stabby::export]` for the constructor
- [x] Rewrite `service_plugin!` macro using `#[stabby::export]` for the constructor
- [x] Define `new`/`start` pattern: constructor receives `PluginExecutor` + `MessageBrokerHandle`, `start()` spawns async tasks
- [x] Host-side: update plugin loader to use `stabby::libloading::StabbyLibrary::get_stabbied` with ABI verification
- [x] Remove `abi_stable` from `plugin-api` dependencies

**Estimated effort:** Medium. The trait-to-FFI transition and macro rewrite are the most complex parts.

### Phase 2 — Model Migration

**Scope:** All `model/<name>` crates.

- [x] Convert `String` → `stabby::string::String`
- [x] Convert `Vec<T>` → `stabby::vec::Vec<T>`
- [x] Convert `Option<T>` → `stabby::option::Option<T>`
- [x] Add `#[stabby::stabby]` to all message structs and enums
- [x] Remove `serde::Serialize` / `serde::Deserialize` derives from FFI-facing types (keep for config parsing if needed)
- [x] Ensure `MessageTopic` trait is compatible with stabby strings

**Status:** ✅ **Complete.** All FFI-facing types in `model/audio`, `model/area`, `model/plugin`, `model/notifications`, `model/mpris`, and `model/app-launcher`
have `#[stabby::stabby]` or `#[stabby::stabby(no_opt)]`. Non-stabby types (`AreaConfig`, `AreaTransition`, `PluginEntry`) are Host-side config types with
`serde` for JSON parsing and do not cross FFI.

#### Why two structs per type?

Some model crates define **two versions** of the same concept: a normal Rust struct (`AreaConfig`) and an ABI-stable counterpart (`AreaConfigStabby`). This is
intentional:

- **Normal Rust structs** (`#[derive(Serialize, Deserialize)]`) are used for **Host-side JSON config parsing** (e.g., `config.json` → `AreaConfig`). These never
  cross FFI boundaries and use standard Rust types (`String`, `Vec<T>`, `Option<T>`).
- **Stabby structs** (`#[stabby::stabby]`) are used for **cross-plugin FFI messaging** (e.g., `AudioStatusMessage` sent from service to widget). These use
  `stabby::string::String`, `stabby::vec::Vec<T>`, `stabby::option::Option<T>` and have a stable ABI layout.
- **Conversion traits** (`From<X> for XStabby` and vice versa) bridge the two worlds at the service/widget edge.

This separation is necessary because `#[stabby::stabby]` and `#[derive(Serialize, Deserialize)]` cannot coexist on the same struct — stabby controls the memory
layout, while serde expects standard Rust layout. Mixing them would lead to compilation errors or undefined behavior.

**Estimated effort:** Low per crate. Purely mechanical refactoring.

### Phase 3 — Host Broker Rewrite

**Scope:** `smearor-swipe-launcher/src/` message broker and plugin loading.

- [x] Replace `FfiEnvelope` JSON payload with raw pointer + `type_id` (zero-copy, no serialization)
- [x] Rewrite `PluginManager` and `ServiceManager` to use stabby traits (manual `#[repr(C)]` VTables with version checking; stabby trait-objects blocked by
  limitations)

> **Note:** `DynBox` does not exist in stabby 72.1.8. The trait-object approach via `stabby::dynptr!` is possible, but requires `SharedMessage` to be a
`#[stabby::stabby]` trait — which is blocked by stabby's limitation on trait-object parameters in trait methods. The current manual VTable + raw pointer
> approach is the practical solution.

- [x] Implement `host_spawn` and pass `PluginExecutor` during plugin init
- [ ] Remove `serde_json` dependency from the Host (where only used for plugin messages)

**Estimated effort:** Medium. The broker routing logic changes significantly.

### Phase 4 — Proof of Concept: Notifications

**Scope:** `services/notifications` + `plugins/notifications` + `model/notifications`.

- [x] Migrate `model/notifications` to stabby types
- [x] Rewrite `NotificationService` with `new`/`start` pattern — `new` stores `PluginExecutor`, `start` spawns zbus listener via Host Tokio
- [x] Use idiomatic async zbus inside the service task
- [x] Rewrite `NotificationWidget` to use `tokio::sync::mpsc` + `glib::MainContext::spawn_local` instead of `timeout_add_local`
- [x] Verify end-to-end: notification arrives via zbus → service → broker → widget → GTK update

**Estimated effort:** Medium. This validates the entire stack.

### Phase 5 — Plugin-by-Plugin Migration

**Scope:** Remaining plugins and services.

Order of migration (simplest to most complex):

1. `clock` — simplest, no external I/O
2. `button` — simple commands
3. `audio` — libpulse (has async bindings)
4. `app-launcher` — desktop file monitoring
5. `mpris` — complex zbus interactions

For each:

- [x] Migrate `model/<name>` to stabby
- [x] Rewrite service (if applicable) as async Tokio task
- [x] Rewrite widget to remove polling loops
- [x] Verify functionality end-to-end

**Estimated effort:** Low per plugin after the PoC pattern is established.

### Phase 6 — Cleanup

**Scope:** Repository-wide.

- [x] Remove `abi_stable` from workspace dependencies
- [x] Remove `serde_json` from crates where no longer needed (still required for Host-side config parsing)
- [x] Audit and remove all `std::thread::spawn` from plugins (all services now use Tokio)
- [x] Audit and remove all `glib::timeout_add_local` polling loops (replaced with event-driven `spawn_local`)
- [x] Reclaim Host runtime handle via `Box::from_raw` during Host shutdown (`Drop` impls in `LoadedPlugin`/`LoadedService`)
- [x] Update documentation (`WIDGET_SYSTEM.md`, `SERVICE_PLUGIN_CONCEPT.md`, `ASYNC_PLUGINS_ARCHITECTURE.md`, etc.)
- [x] Update `AGENTS.md` with new architecture guidelines

**Estimated effort:** Low. Cleanup and documentation.

---

## 5. Decision Summary

| Aspect                | Decision                   | Rationale                                             |
|-----------------------|----------------------------|-------------------------------------------------------|
| FFI framework         | `stabby`                   | Native async support, modern, actively developed      |
| Serialization         | **None**                   | Shared `model` crates allow zero-copy pointer passing |
| Intermediary format   | Not needed                 | `postcard`/`rkyv` would be unnecessary indirection    |
| Plugin runtime access | `PluginExecutor`           | Stable FFI delegation to Host Tokio runtime           |
| Async I/O in plugins  | Native async               | zbus, libpulse, etc. work idiomatically               |
| GTK bridge            | `MainContext::spawn_local` | GTK's single-threaded requirement is respected        |

This architecture eliminates the root causes of the current performance problems rather than treating the symptoms. Every new plugin built on this foundation
automatically benefits from event-driven messaging, zero-copy communication, and idiomatic async I/O.
