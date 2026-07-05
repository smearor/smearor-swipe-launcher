# Concept: Multiple Widgets per Widget Plugin

## Problem

The `widget_plugin!` macro in `plugin-api/src/macro.rs` generates one FFI export function with the fixed symbol name `smearor_plugin_create` per invocation (
`@/home/aschaeffer/git/smearor-swipe-launcher/plugin-api/src/macro.rs:176-177`). A dynamic library (`.so`) may export this symbol only once. When multiple
widgets use the same macro in one crate, the compiler emits:

```
error[E0428]: the name `smearor_plugin_create` is defined multiple times
```

For the current `plugins/sysinfo` crate, a single `.so` should provide multiple widgets (CPU, Memory, Battery, Disks, Network, Uptime).

## Goals

- A widget plugin crate can provide multiple widget implementations.
- The host still loads a single `.so` and still calls only `smearor_plugin_create`.
- Each widget implementation remains a separate Rust struct with its own `WidgetBuilder` / `Plugin` implementation.
- The widget selection is driven by the plugin configuration.
- No duplication of shared code (e.g. `config.rs`, `shared.rs` in `plugins/sysinfo`).

## Non-Goals

- Services (`smearor_service_create`) are not changed; this concept is for widgets only.
- The fundamental host/plugin architecture (VTable, stabby, FFI container) remains unchanged.
- No automatic widget registration without explicit configuration is introduced.
- No unit tests are added as part of this change.

## AGENTS.md Compliance

The implementation follows the project guidelines from `AGENTS.md`:

- **Documentation**: All new public items (the `widget_factory_plugin!` macro, `PluginEntry` field, and any new error variants) receive rustdoc comments with
  semantic descriptions.
- **Naming**: Descriptive names without abbreviations; the macro and its parameters are self-explanatory.
- **Error handling**: The factory constructor returns `stabby::result::Result` with a descriptive `PluginConstructionErrorWrapper` for unknown widget names or
  construction failures.
- **Single responsibility**: The macro generates one constructor that dispatches; per-widget destruction, message handling, and build functions remain separate
  and small.
- **No panics**: The generated code avoids `unwrap()` / `expect()`; missing or unknown `widget` values produce a recoverable error.

## Proposed Solution

### New Macro Variant: `widget_factory_plugin!`

A new macro is added in `plugin-api/src/macro.rs`:

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

The macro generates:

1. A unique VTable per widget type (as before via `paste::paste!`).
2. A single `#[stabby::export] pub extern "C" fn smearor_plugin_create(...)`.
3. Inside that function, the `widget` field is read from the configuration.
4. A `match` on the value selects the correct widget implementation and returns the corresponding `PluginContainer`.

### Host-Side Configuration

A new `widget` field is added per plugin instance in `config.toml`. Example:

```toml
[left_area]
area_type = "fixed"
width = 200
align = "right"
plugins = [
    { id = "sysinfo_cpu", path = "target/release/libsmearor_sysinfo_widget.so", widget = "cpu" },
    { id = "sysinfo_memory", path = "target/release/libsmearor_sysinfo_widget.so", widget = "memory" },
]

[sysinfo_cpu]
show_temperature = true

[sysinfo_memory]
show_used_bytes = true
```

### Host Code Changes

In `smearor-swipe-launcher/src/plugin.rs`, the plugin configuration is extended by the `widget` field. The field must reach the JSON config passed to
`smearor_plugin_create`. The current code already injects the `id` (`@/home/aschaeffer/git/smearor-swipe-launcher/smearor-swipe-launcher/src/plugin.rs:42-44`).
The `widget` field is part of the original `config.config` and is therefore serialized automatically as long as it is present in `PluginEntry`.

The `PluginEntry` struct in `model/plugin/src/plugin.rs` is extended by an optional `widget` field:

```rust
pub struct PluginEntry {
    pub id: String,
    pub path: String,
    pub widget: Option<String>,
}
```

The `PluginEntryStabby` version is adjusted accordingly (optional, because pure identification is not ABI-relevant, but consistency matters).

### Plugin Code Changes

In `plugins/sysinfo/src/lib.rs`, the six `widget_plugin!` calls are replaced by a single `widget_factory_plugin!` call:

```rust
use crate::widget_battery::BatteryWidget;
use crate::widget_cpu::CpuWidget;
use crate::widget_disks::DisksWidget;
use crate::widget_memory::MemoryWidget;
use crate::widget_network::NetworkWidget;
use crate::widget_uptime::UptimeWidget;
use smearor_swipe_launcher_plugin_api::widget_factory_plugin;

widget_factory_plugin! {
    "cpu" => CpuWidget,
    "memory" => MemoryWidget,
    "battery" => BatteryWidget,
    "disks" => DisksWidget,
    "network" => NetworkWidget,
    "uptime" => UptimeWidget,
};
```

The individual widget modules (`widget_cpu.rs`, `widget_memory.rs`, etc.) remain unchanged.

### Macro Implementation Sketch

```rust
#[macro_export]
macro_rules! widget_factory_plugin {
    (
        $(
            $name:literal => $widget_type:ty
        ),+ $(,)?
    ) => {
        paste::paste! {
            $(
                // per widget type: destroy, build_widget, on_message, start, VTable
                unsafe extern "C" fn [<destroy_ $widget_type:snake>](instance: *mut core::ffi::c_void) { ... }
                unsafe extern "C" fn [<build_widget_ $widget_type:snake>](instance: *mut core::ffi::c_void) -> $crate::FfiWidget { ... }
                unsafe extern "C" fn [<on_message_ $widget_type:snake>](instance: *mut core::ffi::c_void, message: *mut core::ffi::c_void) { ... }
                unsafe extern "C" fn [<start_ $widget_type:snake>](instance: *mut core::ffi::c_void) { ... }

                static [<VTABLE_ $widget_type:snake>]: $crate::PluginVTable = $crate::PluginVTable {
                    destroy: [<destroy_ $widget_type:snake>],
                    build_widget: [<build_widget_ $widget_type:snake>],
                    on_message: [<on_message_ $widget_type:snake>],
                    start: [<start_ $widget_type:snake>],
                };
            )+

            #[stabby::export]
            pub extern "C" fn smearor_plugin_create(
                config_json: *const i8,
                config_len: usize,
                core_context: *mut core::ffi::c_void,
            ) -> stabby::result::Result<
                *mut core::ffi::c_void,
                $crate::PluginConstructionErrorWrapper,
            > {
                // ... common setup (tracing, PluginConfig, ffi_context) ...

                let widget_name = config.config.get("widget")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();

                match widget_name {
                    $(
                        $name => {
                            match <$widget_type>::new(config.clone(), ffi_context.clone()) {
                                Ok(widget) => {
                                    let container = $crate::PluginContainer {
                                        instance: Box::into_raw(Box::new(widget)) as *mut core::ffi::c_void,
                                        vtable: & [<VTABLE_ $widget_type:snake>],
                                        vtable_version: $crate::PLUGIN_VTABLE_VERSION,
                                    };
                                    stabby::result::Result::Ok(
                                        Box::into_raw(Box::new(container)) as *mut core::ffi::c_void
                                    )
                                }
                                Err(error) => stabby::result::Result::Err(error),
                            }
                        }
                    )+
                    _ => stabby::result::Result::Err(
                        $crate::PluginConstructionErrorWrapper::new(
                            $crate::PluginConstructionError::FailedToParseWidgetConfig,
                            format!("unknown widget: {}", widget_name).into(),
                        )
                    ),
                }
            }
        }
    };
}
```

Note: `PluginConfig` and `FfiCoreContext` must be cloned in each match arm because `new` consumes them. `FfiCoreContext` is generally `Clone` or can be copied
because it is created from a raw pointer.

## Impact on Existing Plugins

Existing single-widget plugins (e.g. `plugins/clock`, `plugins/button`) remain unchanged and continue to use `widget_plugin!`. The new `widget_factory_plugin!`
is an **extension**, not a replacement API.

## Risks and Considerations

1. **FFI symbol collision**: The factory macro defines `smearor_plugin_create` only once, fully resolving the original problem.
2. **Configuration field `widget`**: If `widget` is missing, a clear error message must be returned ("unknown widget").
3. **PluginConfig cloning**: Because `new` consumes the config value, it must be cloned in each match arm. This is harmless because `PluginConfig` is small (
   `serde_json::Value`).
4. **`FfiCoreContext`**: Must implement `Clone` or be copyable. If not, the pointer can be parsed and reconstructed instead.
5. **VTable version**: Remains at `PLUGIN_VTABLE_VERSION = 1` because the `PluginVTable` structure does not change.

## Implementation Phases

1. **Phase 1: Macro in `plugin-api`**
    - Add `widget_factory_plugin!` to `plugin-api/src/macro.rs`.
    - Add documentation and an example in the macro comment.

2. **Phase 2: Host Configuration**
    - Add `widget: Option<String>` to `PluginEntry` and `PluginEntryStabby` in `model/plugin/src/plugin.rs`.
    - Verify in `smearor-swipe-launcher/src/plugin.rs` that the field is automatically included in the JSON config.

3. **Phase 3: Migrate `plugins/sysinfo`**
    - Replace `widget_plugin!` with `widget_factory_plugin!` in `plugins/sysinfo/src/lib.rs`.
    - Ensure all `new` functions accept `PluginConfig` and `Option<FfiCoreContext>`.

4. **Phase 4: Configuration and Verification**
    - Add example config in `config.toml` (e.g. `sysinfo_cpu` and `sysinfo_memory` in `left_area`).
    - Run `cargo build` and `cargo clippy` for the affected crates.
    - Verify runtime behavior: the host loads `libsmearor_sysinfo_widget.so` twice with different `widget` values and creates the correct widgets.

## Exit Criteria

- `cargo build` in the workspace succeeds.
- `plugins/sysinfo` exports only one `smearor_plugin_create` symbol.
- The host can instantiate multiple different widgets from the same `.so`.
- Existing single-widget plugins continue to work.

---

Please review the concept and approve it if I should proceed with the implementation.
