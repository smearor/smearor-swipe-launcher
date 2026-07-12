# Async Plugin Architecture for Smearor Swipe Launcher

**Status:** Phase 1 Complete (plugin-api crate migrated to stabby)
**Last Updated:** 2026-06-18

---

## 1. Overview

This document records architectural decisions and implementation details for the async plugin system in the Smearor Swipe Launcher. It is a living document that
will be updated as implementation progresses through each phase.

### Goals

- Replace `abi_stable` with `stabby` for ABI stability and native async support
- Eliminate JSON serialization overhead for plugin-to-plugin communication
- Enable plugins to spawn async tasks on the Host's Tokio runtime
- Maintain type safety across the FFI boundary where possible
- Provide a versioning mechanism to prevent runtime crashes on API mismatches

---

## 2. Core Architecture Decisions

### 2.1 Why stabby instead of abi_stable?

| Feature                | abi_stable        | stabby                                       |
|------------------------|-------------------|----------------------------------------------|
| Trait objects          | Manual VTables    | Generated VTables (with limitations)         |
| Async support          | None              | `DynFuture` for ABI-safe futures             |
| Type safety            | Runtime checks    | Compile-time `IStable` validation            |
| String/Vec types       | `RString`, `RVec` | `stabby::string::String`, `stabby::vec::Vec` |
| Constructor validation | None              | `stabby::export` generates signature reports |

**Decision:** Use stabby for constructor ABI validation and standard types, but fall back to manual VTables for plugin runtime API due to stabby trait
limitations.

### 2.2 Hybrid Architecture: stabby + Manual VTables

After extensive analysis of the stabby crate (v72.1), we discovered a fundamental limitation:

> **stabby traits (`#[stabby::stabby]`) do not support trait-object parameters in their methods.**
> The macro panics with `"trait objects are not ABI stable"` whenever `dyn Trait` appears in a method signature.

This makes it impossible to model `Plugin::on_message(&mut self, message: DynRef<dyn SharedMessage>)` as a stabby trait.

**Solution:** A hybrid approach:

1. **Constructor functions** use `#[stabby::export]` for ABI-validated entry points
2. **Plugin/Service runtime API** uses manual `#[repr(C)]` VTables with version checking
3. **Shared types** (strings, options, results) use stabby's ABI-safe equivalents

```
┌─────────────────────────────────────────────────────────────┐
│                        HOST                                  │
│  ┌─────────────────┐    ┌─────────────────┐                  │
│  │ Plugin Loader   │    │ Plugin Loader   │                  │
│  │ (stabby::import)│    │ (stabby::import)│                  │
│  │                 │    │                 │                  │
│  │ Validates:      │    │ Validates:      │                  │
│  │ - signature     │    │ - signature     │                  │
│  │ - type reports  │    │ - type reports  │                  │
│  └────────┬────────┘    └────────┬────────┘                  │
│           │                        │                           │
│           ▼                        ▼                           │
│  ┌─────────────────┐    ┌─────────────────┐                  │
│  │ PluginContainer   │    │ ServiceContainer │                  │
│  │ {instance, vtable,│    │ {instance, vtable,│                  │
│  │  vtable_version} │    │  vtable_version} │                  │
│  └─────────────────┘    └─────────────────┘                  │
│           │                        │                           │
│           ▼                        ▼                           │
│  ┌─────────────────┐    ┌─────────────────┐                  │
│  │ Manual VTable   │    │ Manual VTable   │                  │
│  │ calls           │    │ calls           │                  │
│  └─────────────────┘    └─────────────────┘                  │
└─────────────────────────────────────────────────────────────┘
```

### 2.3 VTable Versioning Strategy

**The Problem:** With manual `#[repr(C)]` VTables, adding a new method in the middle shifts all subsequent field offsets. An old plugin loaded into a new Host
would have misaligned function pointers, causing segfaults.

**The Solution:** Each VTable has a version constant:

```rust
// plugin-api/src/plugin.rs
pub const PLUGIN_VTABLE_VERSION: u32 = 1;

#[repr(C)]
pub struct PluginVTable {
    pub destroy: extern "C" fn(*mut c_void),
    pub build_widget: extern "C" fn(*mut c_void) -> FfiWidget,
    pub on_message: extern "C" fn(*mut c_void, *mut c_void),
    pub start: extern "C" fn(*mut c_void),
}

#[repr(C)]
pub struct PluginContainer {
    pub instance: *mut c_void,
    pub vtable: *const PluginVTable,
    pub vtable_version: u32,  // Must match PLUGIN_VTABLE_VERSION
}
```

**Rules:**

- **Never reorder or remove existing fields**
- **New methods are appended at the end**
- **Increment `PLUGIN_VTABLE_VERSION` on every VTable change**
- **Host rejects plugins with mismatched `vtable_version`**

This is similar to COM's `Vtbl` versioning or Vulkan's extension mechanism.

---

## 3. Plugin API Crate Structure

### 3.1 File Organization

| File          | Purpose                             | stabby Usage                                                                                                              |
|---------------|-------------------------------------|---------------------------------------------------------------------------------------------------------------------------|
| `error.rs`    | Error types                         | `#[stabby::stabby]` on `PluginConstructionErrorWrapper`; `#[repr(u8)]` + `#[stabby::stabby]` on `PluginConstructionError` |
| `meta.rs`     | Plugin metadata                     | `#[stabby::stabby]` on `PluginMeta`; uses `stabby::string::String`, `stabby::option::Option`                              |
| `config.rs`   | Config parsing                      | Standard Rust; no stabby types in struct itself                                                                           |
| `context.rs`  | Host handles                        | `#[stabby::stabby]` on `PluginExecutor`; `#[repr(C)]` on `MessageBrokerHandle`                                            |
| `messages.rs` | Message traits                      | Normal Rust traits (not stabby); raw pointers for FFI                                                                     |
| `plugin.rs`   | Widget plugin API                   | Normal Rust `Plugin` trait; `#[repr(C)]` VTable + Container                                                               |
| `service.rs`  | Service plugin API                  | Normal Rust `Service` trait; `#[repr(C)]` VTable + Container                                                              |
| `widget.rs`   | GTK widget wrapper                  | `#[repr(C)]` on `FfiWidget`                                                                                               |
| `macro.rs`    | `widget_plugin!`, `service_plugin!` | `#[stabby::export]` on constructor functions                                                                              |

### 3.2 Why Some Types Use `#[stabby::stabby]` and Others `#[repr(C)]`

- **`#[stabby::stabby]`** — Used when all fields are types that implement `IStable` (stabby's ABI-stability trait). stabby computes the layout at compile time
  and generates `IStable` impls.
- **`#[repr(C)]`** — Used when fields contain types that are NOT `IStable` (e.g., `GtkWidget`, function pointer structs like `PluginVTable`, or opaque
  pointers).

### 3.3 stabby Types Used

| Standard Rust     | stabby Equivalent              | Used For                             |
|-------------------|--------------------------------|--------------------------------------|
| `String`          | `stabby::string::String`       | Plugin metadata, error messages      |
| `Option<T>`       | `stabby::option::Option<T>`    | Optional icon names                  |
| `Result<T, E>`    | `stabby::result::Result<T, E>` | Constructor return values            |
| `Box<dyn Future>` | `stabby::future::DynFuture`    | Spawning async tasks on Host runtime |

---

## 4. Plugin Lifecycle

### 4.1 Construction Phase (stabby-validated)

```rust
// Generated by service_plugin!(MyService) or widget_plugin!(MyWidget)
#[stabby::export]
pub extern "C" fn smearor_service_create(
    config_json: *const i8,
    config_len: usize,
    executor: PluginExecutor,
    broker: MessageBrokerHandle,
) -> stabby::result::Result<ServiceContainer, PluginConstructionErrorWrapper>
```

1. Host calls `smearor_service_create` via `stabby::import`
2. stabby validates the type report of the function signature
3. If reports match, the function pointer is returned
4. Plugin parses config, allocates instance, returns `ServiceContainer`

### 4.2 Runtime Phase (manual VTable)

```rust
// Host-side pseudo-code
let container = plugin_constructor(...);
assert_eq!(container.vtable_version, PLUGIN_VTABLE_VERSION);

// Call methods through VTable
(container.vtable.build_widget)(container.instance);
(container.vtable.on_message)(container.instance, message_ptr);
(container.vtable.start)(container.instance);
```

### 4.3 Destruction Phase

```rust
// Host calls destroy before unloading the shared library
(container.vtable.destroy)(container.instance);
```

---

## 5. Message Passing Architecture

### 5.1 Design Decision: Raw Pointers with Type ID

We evaluated several approaches for cross-plugin message passing:

| Approach                                 | Pros                      | Cons                                |
|------------------------------------------|---------------------------|-------------------------------------|
| JSON serialization                       | Simple, language-agnostic | Slow, allocations, parsing overhead |
| stabby `DynBox<dyn SharedMessage>`       | Type-safe                 | Not supported in stabby traits      |
| `stabby::DynRef<vtable!(SharedMessage)>` | Zero-copy                 | Requires lifetime coordination      |
| **Raw pointers + `type_id`**             | **Simple, fast, works**   | **Manual down-casting**             |

**Selected:** Raw pointers with manual `type_id` down-casting.

### 5.2 Message Flow

```
Plugin A                    Host Broker                  Plugin B
   │                          │                            │
   │  broadcast_message(msg)  │                            │
   │ ────────────────────────>│                            │
   │                          │  route(topic, msg_ptr)     │
   │                          │ ──────────────────────────>│
   │                          │                            │
   │                          │                            │  on_message(msg_ptr)
   │                          │                            │  // down-cast via type_id
```

### 5.3 SharedMessage Trait

```rust
// Normal Rust trait (NOT #[stabby::stabby])
pub trait SharedMessage: Send {
    fn topic(&self) -> &'static str;
    fn type_id(&self) -> u64;
}
```

Each message type in a `model` crate implements this trait with a unique `type_id` constant. The receiver down-casts using this ID.

---

## 6. Async Task Delegation

### 6.1 PluginExecutor

```rust
#[stabby::stabby]
#[derive(Clone, Copy)]
pub struct PluginExecutor {
    pub context: *const c_void,  // tokio::runtime::Handle
    pub spawn: extern "C" fn(
        context: *const c_void,
        future: DynFuture<'static, ()>,
    ),
}
```

### 6.2 How Plugins Spawn Async Tasks

```rust
// Inside a plugin
let executor = /* received from Host */;
let future = stabby::future::DynFuture::new(async move {
    // async work here
});
(executor.spawn)(executor.context, future);
```

### 6.3 Host-Side Spawn Implementation

```rust
// Host implements this function pointer
extern "C" fn spawn_on_runtime(
    context: *const c_void,
    future: DynFuture<'static, ()>,
) {
    let handle = unsafe { &*(context as *const tokio::runtime::Handle) };
    handle.spawn(async move {
        future.await;
    });
}
```

---

## 7. Macro Implementation

### 7.1 `service_plugin!` Macro

The macro generates:

1. Static `ServiceVTable` with function pointers to trampoline functions
2. Trampoline functions that cast `*mut c_void` back to `&mut MyService`
3. `#[stabby::export]` constructor function returning `ServiceContainer`

```rust
macro_rules! service_plugin {
    ($service_type:ty) => {
        paste::paste! {
            unsafe extern "C" fn [<destroy_ $service_type:snake>](instance: *mut c_void) { ... }
            unsafe extern "C" fn [<on_message_ $service_type:snake>](instance: *mut c_void, msg: *mut c_void) { ... }
            unsafe extern "C" fn [<start_ $service_type:snake>](instance: *mut c_void) { ... }

            static VTABLE: ServiceVTable = ServiceVTable {
                destroy: destroy_my_service,
                on_message: on_message_my_service,
                start: start_my_service,
            };

            #[stabby::export]
            pub extern "C" fn smearor_service_create(...) -> Result<ServiceContainer, Error> {
                // construct, return container with vtable + version
            }
        }
    };
}
```

### 7.2 `widget_plugin!` Macro

Identical structure, but generates `PluginVTable` and `smearor_plugin_create`.

---

## 8. Host-Side Plugin Loading (Phase 2)

### 8.1 Planned Implementation

The Host will use `stabby::import` to load plugin constructors:

```rust
#[stabby::import(name = "my_plugin")]
extern "C" {
    fn smearor_plugin_create(
        config_json: *const i8,
        config_len: usize,
        executor: PluginExecutor,
        broker: MessageBrokerHandle,
    ) -> stabby::result::Result<PluginContainer, PluginConstructionErrorWrapper>;
}
```

`stabby::import` lazy-initializes the symbol by calling `smearor_plugin_create_stabbied`, which validates the type report before returning the function pointer.

### 8.2 Version Checking

```rust
let container = smearor_plugin_create(...)?;
if container.vtable_version != PLUGIN_VTABLE_VERSION {
    return Err("Plugin VTable version mismatch".into());
}
```

---

## 9. Known Limitations and Trade-offs

### 9.1 stabby Trait Limitations

- Cannot use `dyn Trait` in stabby trait methods
- Cannot use `stabby::dynptr!` in stabby struct definitions (expands to `std::boxed::Box`)
- `stabby::vtable!` macro requires explicit lifetime parameters in some contexts

### 9.2 Manual VTable Limitations

- No automatic ABI validation by stabby for VTable struct layout
- Relies on discipline: append-only, version bump on change
- Host must explicitly check `vtable_version`

### 9.3 Raw Pointer Messages

- No compile-time type safety for messages crossing FFI
- `type_id` collisions possible (mitigated by using UUID-based constants)
- Memory ownership of messages must be carefully documented

---

## 10. Migration Roadmap

| Phase   | Status       | Description                                                                                          |
|---------|--------------|------------------------------------------------------------------------------------------------------|
| Phase 1 | **Complete** | Migrate `plugin-api` crate to stabby types, define traits + VTables, rewrite macros                  |
| Phase 2 | **Complete** | Implement Host-side plugin loader with `stabby::libloading::StabbyLibrary`                           |
| Phase 3 | **Complete** | Migrate existing plugins (clock, app-launcher, audio, mpris, notifications, button) to new macro API |
| Phase 4 | **Complete** | Implement message broker with raw pointer routing (FfiEnvelope + type_id)                            |
| Phase 5 | **Complete** | Add async task spawning (PluginExecutor with DynFuture) and test with real plugins                   |
| Phase 6 | **Complete** | Remove `abi_stable` dependency entirely from workspace                                               |

---

## 11. References

- [stabby GitHub Repository](https://github.com/ZettaScaleLabs/stabby)
- [stabby Examples](https://github.com/ZettaScaleLabs/stabby/tree/main/examples)
- `ASYNC_PLUGINS_CONCEPT.md` — Original concept document

---

*This document is a living artifact. Update it whenever architectural decisions change or new phases are completed.*
