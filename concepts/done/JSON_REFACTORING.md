# Concept: JSON Converter Refactoring — Migration to serde_json

## 1. Problem Statement

All `model/*/src/json_converters.rs` files contain hand-written parsing functions that manually extract fields from `serde_json::Value` and construct message
structs. This approach has several problems:

- **Bug-prone**: Manual match arms for enums frequently miss variants (e.g. `parse_mpris_command_action` was missing `"Play"` and `"Refresh"`;
  `AudioCommandAction::From<&str>` mapped `"Refresh"` to `PreviousDevice`).
- **Redundant**: `serde` derive (`Serialize, Deserialize`) can handle the same conversion automatically, including nested types, `stabby::option::Option`, and
  `stabby::vec::Vec`.
- **Maintenance burden**: Every new field or variant requires updating the manual parser in addition to the struct definition.
- **Code duplication**: Each model crate repeats the same pattern: `parse_*` helper functions, `impl_json_convertible!` with a hand-written closure, and a
  `register_json_converters` function.

### 1.1 Already Completed

The refactoring has been applied to `model/audio` and `model/mpris` as pilot crates. The pattern is proven and compiles cleanly across model, service, and
widget crates.

### 1.2 Scope

This concept covers the remaining model crates that still use manual `json_converters.rs` files, plus the area system in the core application
(`smearor-swipe-launcher/src/area/area_manager.rs`).

---

## 2. Refactoring Pattern

The proven pattern from `model/audio` and `model/mpris` consists of four steps per crate:

### 2.1 Cargo.toml — Enable stabby serde feature

```toml
stabby = { workspace = true, features = ["serde"] }
```

### 2.2 Message types — Add derives

Add `Serialize, Deserialize` to all message structs and enums. Add `Default` to structs that need `unwrap_or_default()` as a deserialization fallback.

### 2.3 lib.rs — Replace json_converters module

Replace the `mod json_converters` and `pub use json_converters::register_json_converters` with direct `impl_json_convertible!` macro calls using
`serde_json::from_value`:

```rust
smearor_swipe_launcher_plugin_api::impl_json_convertible!(
    FooCommandMessageConverter,
    FooCommandMessage,
    |json: serde_json::Value| serde_json::from_value(json).unwrap_or_default()
);
```

Keep `register_json_converters()` as a public function for backward compatibility with service crates.

### 2.4 Delete json_converters.rs

The entire file is removed. All manual `parse_*` functions are eliminated.

---

## 3. Affected Crates

### 3.1 Model crates with `json_converters.rs`

| Crate                    | `json_converters.rs` lines | Stabby serde feature | Stabby wrapper types                                                 | Notes                                                                              |
|--------------------------|----------------------------|----------------------|----------------------------------------------------------------------|------------------------------------------------------------------------------------|
| `model/app-launcher`     | 74                         | Missing              | `DesktopFileCommandMessageStabby`, `DesktopFileStatusMessageStabby`  | Dual converter pairs (native + stabby)                                             |
| `model/area`             | 63                         | Missing              | `AddAreaMessageStabby`                                               | Also has `register_json_converters_in_registry` for core                           |
| `model/hyprland`         | 1117                       | Missing              | Multiple `*Stabby` wrappers                                          | Largest crate; complex nested enums                                                |
| `model/instance-control` | —                          | Missing              | —                                                                    | Needs investigation                                                                |
| `model/macropad`         | —                          | Missing              | —                                                                    | Needs investigation                                                                |
| `model/network`          | 201                        | Missing              | —                                                                    | 4 converters                                                                       |
| `model/notifications`    | 104                        | Missing              | —                                                                    | Nested `NotificationInfo`, `NotificationAction`                                    |
| `model/personalization`  | 134                        | Missing              | —                                                                    | Complex enum `PersonalizationCommandAction` with data                              |
| `model/power`            | 112                        | Missing              | —                                                                    | Nested `PowerCapabilities`, `InhibitorInfo`, `ScheduledActionInfo`                 |
| `model/terminal_command` | 69                         | Missing              | `TerminalCommandMessageStabby`, `TerminalCommandStatusMessageStabby` | Dual converter pairs                                                               |
| `model/voice_assistant`  | 86                         | Missing              | `VoiceCommandMessageStabby`, `AssistantStatusMessageStabby`          | Dual converter pairs                                                               |
| `model/wallpaper`        | 105                        | Missing              | —                                                                    | Nested `MonitorProcess`, `WallpaperThemeInfo`                                      |
| `model/weather`          | 149                        | Missing              | —                                                                    | Complex nested types (`CurrentWeatherData`, `DailyForecastData`, `AirQualityData`) |
| `model/widget`           | 15                         | Missing              | —                                                                    | Single converter, trivial                                                          |

### 3.2 Core area system

The area system lives in `smearor-swipe-launcher/src/area/area_manager.rs` and calls
`smearor_model_area::register_json_converters_in_registry(&json_converter_registry)` at startup. The area model crate (`model/area`) has a dual registration
pattern:

- `register_json_converters(context)` — FFI callback for plugins
- `register_json_converters_in_registry(registry)` — Direct registry for core

Both must be preserved after refactoring.

### 3.3 Service crates (no changes needed)

Service crates call `register_json_converters(core_context)` during initialization. This function signature remains unchanged after refactoring, so no service
crate modifications are required.

---

## 4. Phased Plan

### Phase 1: Simple model crates

**Goal**: Refactor the simplest model crates to establish the pattern at scale.

**Crates** (ordered by complexity):

1. `model/widget` — 1 converter, 15 lines
2. `model/terminal_command` — 4 converters, dual stabby pairs
3. `model/app-launcher` — 4 converters, dual stabby pairs, `SmearorWindowRotationWrapper` dependency
4. `model/voice_assistant` — 4 converters, dual stabby pairs

**Steps per crate**:

1. Add `features = ["serde"]` to `stabby` in `Cargo.toml`
2. Add `Serialize, Deserialize` (and `Default` where needed) to all message types
3. Remove redundant `From<&str>` / `AsRef<str>` impls if present
4. Replace `json_converters` module in `lib.rs` with `impl_json_convertible!` calls
5. Delete `json_converters.rs`
6. Verify: `cargo check -p <model-crate> && cargo check -p <service-crate> && cargo check -p <widget-crate>`

**Exit criteria**: All 4 crates compile, no `json_converters.rs` files remain.

### Phase 2: Medium model crates

**Goal**: Refactor crates with moderate complexity (nested structs, multiple converters).

**Crates** (ordered by complexity):

1. `model/wallpaper` — 2 converters, nested `MonitorProcess`, `WallpaperThemeInfo`
2. `model/notifications` — 2 converters, nested `NotificationInfo`, `NotificationAction`, `UrgencyLevel`
3. `model/power` — 2 converters, nested `PowerCapabilities`, `InhibitorInfo`, `ScheduledActionInfo`, `PowerAction`
4. `model/network` — 4 converters, nested `InterfaceStatus`, `AccessPointInfo`, `VpnProfileInfo`
5. `model/personalization` — 2 converters, complex enum `PersonalizationCommandAction` with data variants (`UpdateLocation(GeoCoordinates)`,
   `UpdateLocale(String)`)

**Special considerations**:

- `PersonalizationCommandAction` has data-carrying enum variants. serde serializes these as tagged unions (`{"UpdateLocation": {...}}`). The manual parser used
  a flat JSON shape. A `#[serde(tag = "action", content = "data")]` attribute may be needed to preserve the existing JSON contract.
- `model/network` has 4 converters (`NetworkCommandMessage`, `NetworkStatusMessage`, `ScanResultsMessage`, `VpnProfilesMessage`).

**Exit criteria**: All 5 crates compile, services and widgets unchanged.

### Phase 3: Weather model

**Goal**: Refactor `model/weather` as a standalone phase due to its complex nested type hierarchy.

**Types to derive**: `WeatherCommandAction`, `WeatherCommandMessage`, `WeatherStatusMessage`, `CurrentWeatherData`, `DailyForecastData`, `DailyForecastEntry`,
`AirQualityData`, and all sub-types (level enums, precipitation types, etc.).

**Special considerations**:

- Many sub-types in `model/weather/src/model/` directory (e.g. `precipitation_amount_level.rs`, `particulate_matter_level.rs`, `sunshine_level.rs`)
- Some types may already have `Serialize, Deserialize` — verify before adding
- `WeatherStatusMessage` has many fields with nested structs and `stabby::option::Option`

**Exit criteria**: `model/weather` compiles, `services/weather` and `plugins/weather` unchanged.

### Phase 4: Hyprland model

**Goal**: Refactor `model/hyprland` — the largest and most complex model crate (1117 lines of `json_converters.rs`).

**Types**: 30+ message types, 20+ helper enums and structs (`HyprlandDirection`, `HyprlandWorkspaceIdentifier`, `HyprlandMonitorIdentifier`,
`HyprlandWindowIdentifier`, `HyprlandPropType`, etc.).

**Special considerations**:

- `HyprlandWindowIdentifier` is a data-carrying enum (`Address(String)`, `ClassRegularExpression(String)`, `Title(String)`, `ProcessId(u32)`). serde serializes
  these as tagged unions by default.
- `HyprlandWorkspaceIdentifier` is also a data-carrying enum with unit variants (`Empty()`, `Previous()`) and data variants (`Id(i32)`, `Name(String)`, etc.).
  Unit-like variants with `()` may need `#[serde(untagged)]` or custom attributes.
- Many `*Stabby` wrapper types with `From` conversions
- This phase should be approached carefully, potentially splitting into sub-tasks

**Exit criteria**: `model/hyprland` compiles, `services/hyprland` and any widgets unchanged.

### Phase 5: Area system (core)

**Goal**: Refactor `model/area` and update the core area manager.

**Types**: `OpenAreaMessage`, `CloseAreaMessage`, `RemoveAreaMessage`, `AddAreaMessage`, `AddAreaMessageStabby`, `AreaConfigStabby`, `AreaTypeStabby`,
`AreaTransitionStabby`.

**Special considerations**:

- `AddAreaMessageStabby` has a manual converter that constructs a default `AreaConfigStabby` — this may need `Default` derives on `AreaConfigStabby`,
  `AreaTypeStabby`, and `AreaTransitionStabby`
- `model/area` has two registration functions:
    - `register_json_converters(context)` — used by plugins via FFI
    - `register_json_converters_in_registry(registry)` — used by core `AreaManager::new`
- Both must be preserved
- `smearor-swipe-launcher/src/area/area_manager.rs:57` calls `register_json_converters_in_registry` — no change needed
- `smearor-swipe-launcher/src/main.rs` calls `register_json_converters` for other models — no change needed

**Exit criteria**: `model/area` compiles, core application compiles and starts.

### Phase 6: Remaining model crates

**Goal**: Refactor any remaining model crates that have `json_converters.rs` files but were not covered by earlier phases.

**Crates to verify**:

- `model/instance-control` — check for `json_converters.rs`
- `model/macropad` — check for `json_converters.rs`
- `model/clock` — check for `json_converters.rs`
- `model/sysinfo` — check for `json_converters.rs`
- `model/workspace` — check for `json_converters.rs`
- `model/http` — check for `json_converters.rs`

**Exit criteria**: No `json_converters.rs` files remain in any `model/*/src/` directory.

### Phase 7: AGENTS.md update and cleanup

**Goal**: Update `AGENTS.md` to mandate serde derives for all future model crates.

**Changes**:

1. Add a new requirement to **Project-Specific Requirements > Requirements**:
   > 15. All message types in `model` crates must derive `Serialize, Deserialize` from `serde`. The `stabby` dependency must include the `serde` feature. JSON
         converters must use `impl_json_convertible!` with `serde_json::from_value(json).unwrap_or_default()` — manual `parse_*` functions are forbidden.
2. Update the **Model example** in AGENTS.md to show the serde derive pattern
3. Update **Dependencies** section to mention `stabby` with `serde` feature
4. Update **Key Features to Implement** to mention `serde_json` for JSON conversion

**Exit criteria**: AGENTS.md updated, future model crates follow the serde pattern from the start.

---

## 5. Bug Fixes Discovered During Pilot

The manual parsers contained several bugs that are fixed by the serde migration:

| Crate         | Bug                                                                                     | Fix                                  |
|---------------|-----------------------------------------------------------------------------------------|--------------------------------------|
| `model/audio` | `From<&str>` for `AudioCommandAction` mapped `"Refresh"` to `PreviousDevice`            | serde handles all variants correctly |
| `model/mpris` | `parse_mpris_command_action` missing `"Play"` and `"Refresh"` variants                  | serde handles all variants correctly |
| `model/audio` | `parse_audio_command_action` (commented out) missing `VolumeUp`, `SetVolume`, `Refresh` | serde handles all variants correctly |

---

## 6. Verification Strategy

After each phase:

1. `cargo check -p <model-crate>` — model compiles
2. `cargo check -p <service-crate>` — service compiles (no changes expected)
3. `cargo check -p <widget-crate>` — widget compiles (no changes expected)
4. `cargo fmt` — formatting is consistent
5. `cargo clippy -p <model-crate>` — no new warnings

After all phases:

6. `cargo build` — full workspace compiles
7. `grep -r "json_converters" model/` — no remaining references
8. `find model/ -name "json_converters.rs"` — no remaining files

---

## 7. Risks and Mitigations

| Risk                                                                                      | Mitigation                                                                                                             |
|-------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------|
| serde enum representation differs from manual parser JSON shape                           | Use `#[serde(tag = "action")]` or `#[serde(rename_all = "PascalCase")]` attributes to preserve existing JSON contracts |
| `stabby::option::Option` / `stabby::vec::Vec` serde support incomplete                    | Already proven in `model/audio` and `model/mpris` — stabbys `serde` feature works correctly                            |
| `AddAreaMessageStabby` manual converter constructs default `AreaConfigStabby`             | Add `Default` derives to `AreaConfigStabby` and related types                                                          |
| Hyprland crate complexity (1117 lines)                                                    | Phase 4 is isolated; approach incrementally, potentially one message group at a time                                   |
| Data-carrying enums (e.g. `PersonalizationCommandAction::UpdateLocation(GeoCoordinates)`) | serde tagged union representation may differ from flat manual parser; verify JSON contract compatibility               |
