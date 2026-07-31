# Concept: MacroPad 2D Span Groups

This document describes the concept for **2D Span Groups** — a generalisation of the existing horizontal-only Multi-Span Widget system to support arbitrary grid
layouts (1×N, N×1, M×N) on MacroPad devices. The current implementation in `MACROPAD_ATOMIC_WIDGETS.md` Phase 8 only supports horizontal 1×N spans. This concept
extends the host rendering pipeline, device metadata, and plugin configuration model to support vertical and rectangular grid spans.

---

## 1. Problem Statement

### 1.1 Current State

Multi-Span Widgets were introduced in `MACROPAD_ATOMIC_WIDGETS.md` Phase 8. The host rendering pipeline in `application.rs` groups plugins by `span_group`,
sorts them by `span_index`, renders the first member at combined dimensions (`key_width * group_size`, `key_height`), and splits the pixel buffer into
horizontal slices using `extract_horizontal_slice()`.

This approach is limited to **horizontal 1×N spans**:

- `combined_width = key_width * group_size` — only width is multiplied.
- `combined_height = key_height` — height stays at a single button.
- `extract_horizontal_slice()` cuts vertical strips from the combined buffer.
- `button_index` increments linearly — no 2D position mapping.

### 1.2 What Is Missing

- **Vertical spans (N×1)**: A volume slider spanning two buttons vertically, or a tall clock display.
- **Grid spans (M×N)**: A 2×2 system monitor showing CPU, memory, temperature, and disk across four buttons in a square layout.
- **Device grid awareness**: The host does not know how many columns the device's button grid has, which is required to map 2D span positions to physical button
  indices.

### 1.3 Why Generalise

A general 2D grid implementation with `span_rows` and `span_cols` parameters covers all cases:

| Configuration  | `span_rows` | `span_cols` | Buttons | Example                  |
|----------------|-------------|-------------|---------|--------------------------|
| 1×2 (existing) | 1           | 2           | 2       | Horizontal volume slider |
| 1×3            | 1           | 3           | 3       | Wide digital clock       |
| 2×1            | 2           | 1           | 2       | Vertical volume slider   |
| 3×1            | 3           | 1           | 3       | Tall countdown timer     |
| 2×2            | 2           | 2           | 4       | System monitor grid      |
| 3×3            | 3           | 3           | 9       | Full-grid file browser   |

The existing 1×N spans become the special case `span_rows = 1`.

---

## 2. Goals

- Support arbitrary rectangular span layouts (M×N) on MacroPad devices.
- Maintain full backward compatibility with existing 1×N span configurations.
- Provide device grid metadata (column count) to the host for correct physical button mapping.
- Handle alignment validation — prevent spans from overflowing row boundaries.
- Keep widget implementations unchanged — widgets already render to arbitrary dimensions via `render_graphic(width, height)`.

## 3. Non-Goals

- Supporting non-rectangular span shapes (L-shapes, T-shapes).
- Supporting overlapping span groups on the same buttons.
- Changing the `GraphicRenderer` trait or `AtomicGraphicRenderer` trait.
- Changing the `widget_factory_plugin_graphic!` macro.
- Supporting 2D spans on GTK or Web instances (GTK uses its own layout system).

---

## 4. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                        MacroPad Device                                │
│  (e.g. Stream Deck: 5 columns × 3 rows = 15 keys)                    │
│                                                                       │
│  ┌────────┬────────┬────────┬────────┬────────┐                      │
│  │  B0    │  B1    │  B2    │  B3    │  B4    │                      │
│  ├────────┼────────┼────────┼────────┼────────┤                      │
│  │  B5    │  B6    │  B7    │  B8    │  B9    │                      │
│  ├────────┼────────┼────────┼────────┼────────┤                      │
│  │  B10   │  B11   │  B12   │  B13   │  B14   │                      │
│  └────────┴────────┴────────┴────────┴────────┘                      │
│                                                                       │
│  2×2 Span Group starting at B6:                                       │
│  ┌────────┬────────┬────────┬────────┬────────┐                      │
│  │  B0    │  B1    │  B2    │  B3    │  B4    │                      │
│  ├────────┼────────┼────────┼────────┼────────┤                      │
│  │  B5    │ Span0  │ Span1  │  B8    │  B9    │                      │
│  ├────────┼────────┼────────┼────────┼────────┤                      │
│  │  B10   │ Span2  │ Span3  │  B13   │  B14   │                      │
│  └────────┴────────┴────────┴────────┴────────┘                      │
│                                                                       │
│  Physical button mapping:                                             │
│    Span0 → B6  (row=1, col=1) → base + 1*5 + 1 = 6                   │
│    Span1 → B7  (row=1, col=2) → base + 1*5 + 2 = 7                   │
│    Span2 → B11 (row=2, col=1) → base + 2*5 + 1 = 11                  │
│    Span3 → B12 (row=2, col=2) → base + 2*5 + 2 = 12                  │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 5. Changes by Layer

### 5.1 Model: `MacroPadConnectionStatus`

**File**: `model/macropad/src/connection_status.rs`

Add a `key_columns` field to the connection status message so the host knows the grid layout of the device.

```rust
/// Number of columns in the device's button grid.
///
/// Used by the host to map 2D span group positions to physical button
/// indices. For devices with a single row, this equals `key_count`.
pub key_columns: u8,
```

The `MacroPadConnectionStatus::new()` function signature gains a `key_columns` parameter.

### 5.2 Model: `MacroPadDeviceMetadata`

**File**: `smearor-swipe-launcher/src/instance/macropad_metadata.rs`

Add `key_columns` to the host-side device metadata struct.

```rust
/// Number of columns in the device's button grid.
pub key_columns: u8,
```

### 5.3 Model: `PluginEntry`

**File**: `model/plugin/src/plugin.rs`

Add two new optional fields to `PluginEntry`:

```rust
/// Number of rows this span group occupies in the button grid.
///
/// Defaults to `1` (horizontal span). Used together with `span_cols`
/// to determine the combined render dimensions and physical button
/// mapping for 2D span groups.
#[serde(default)]
pub span_rows: Option<u32>,
/// Number of columns this span group occupies in the button grid.
///
/// Defaults to `1` (vertical span or single button). Used together
/// with `span_rows` to determine the combined render dimensions and
/// physical button mapping for 2D span groups.
#[serde(default)]
pub span_cols: Option<u32>,
```

The `PluginEntryStabby` struct and its `From`/`Into` implementations do **not** need changes — `span_rows` and `span_cols` are host-side only fields, not passed
through FFI to plugins.

### 5.4 Services: Driver Reporting

Each MacroPad driver service must report `key_columns` when broadcasting `MacroPadConnectionStatus`.

**Stream Deck** (`services/streamdeck/src/service.rs`):

The `streamdeck` crate's `DeviceKind` does not expose columns/rows directly, but the key count and device type imply the layout. A lookup table or heuristic can
derive columns from the known device types:

| Device Kind | Keys | Columns | Rows |
|-------------|------|---------|------|
| Original V2 | 15   | 5       | 3    |
| Mini        | 6    | 3       | 2    |
| XL          | 32   | 8       | 4    |
| MK2         | 15   | 5       | 3    |
| Plus        | 8    | 4       | 2    |
| Pedal       | 3    | 3       | 1    |

Alternatively, the `streamdeck` crate may be patched to expose `columns()` on `DeviceKind`. The lookup table approach is preferred initially to avoid upstream
dependency changes.

**Loupedeck** (`services/loupedeck/src/service.rs`):

The loupedeck driver already has `layout.columns` and `layout.rows` from `device.layout()`. Simply pass `layout.columns as u8` as `key_columns`.

### 5.5 Host: `render_buttons_to_device`

**File**: `smearor-swipe-launcher/src/application.rs`, function `render_buttons_to_device` (line ~1457)

#### Current Logic (1×N only)

```
combined_width = key_width * group_size
render_graphic(combined_width, key_height)
for each member at index i:
    x_offset = i * slice_width
    extract_horizontal_slice(...)
    send to button_index++
```

#### New Logic (2D Grid)

```
span_rows = group_members[0].span_rows.unwrap_or(1)
span_cols = group_members[0].span_cols.unwrap_or(1)
group_size = span_rows * span_cols

// Validate: group_members.len() must equal group_size
// Validate: span must not overflow device row boundary

combined_width = key_width * span_cols
combined_height = key_height * span_rows
render_graphic(combined_width, combined_height)

for each member at span_index i:
    row = i / span_cols
    col = i % span_cols
    x_offset = col * key_width
    y_offset = row * key_height
    slice = extract_grid_slice(pixels, combined_width, combined_height,
                               x_offset, y_offset, key_width, key_height)
    physical_button = base_button + row * key_columns + col
    send_button_image(physical_button, ...)

button_index += group_size
```

### 5.6 Host: `render_single_button_to_device`

**File**: `smearor-swipe-launcher/src/application.rs`, function `render_single_button_to_device` (line ~1639)

Same generalisation as `render_buttons_to_device` — read `span_rows`/`span_cols` from the first group member, compute combined dimensions, use
`extract_grid_slice()`, and map each member to its physical button via `base + row * key_columns + col`.

### 5.7 Host: `extract_grid_slice`

**File**: `smearor-swipe-launcher/src/application.rs`

New helper function replacing `extract_horizontal_slice` for 2D spans:

```rust
/// Extract a rectangular slice from an RGBA pixel buffer.
///
/// Used to split a 2D span group's combined render into individual
/// button images. Crops the region (x_offset, y_offset) to
/// (slice_width, slice_height) from the source buffer.
fn extract_grid_slice(
    pixels: &[u8],
    src_width: u32,
    src_height: u32,
    x_offset: u32,
    y_offset: u32,
    slice_width: u32,
    slice_height: u32,
) -> Vec<u8> {
    let mut result = Vec::with_capacity((slice_width * slice_height * 4) as usize);
    for y in y_offset..(y_offset + slice_height) {
        let start = ((y * src_width + x_offset) * 4) as usize;
        let end = start + (slice_width * 4) as usize;
        result.extend_from_slice(&pixels[start..end]);
    }
    result
}
```

The existing `extract_horizontal_slice` can be replaced by `extract_grid_slice` with `y_offset = 0` and `slice_height = src_height`, or kept as a convenience
wrapper.

### 5.8 Host: Alignment Validation

Before rendering a 2D span group, the host validates that the span fits within the device grid:

```
base_col = base_button % key_columns
base_row = base_button / key_columns

// Check column overflow
if base_col + span_cols > key_columns:
    log warning "Span group '{span_group}' would overflow row boundary
        (base_col={base_col}, span_cols={span_cols}, device_columns={key_columns})"
    // Option A: Skip rendering (strict)
    // Option B: Advance base to next row start (pragmatic)
    base_col = 0
    base_button = (base_row + 1) * key_columns

// Check row overflow
if base_row + span_rows > (key_count / key_columns):
    log warning "Span group '{span_group}' would overflow device bottom"
    skip rendering
```

The **pragmatic** approach (Option B) is recommended — advance `base_button` to the next row if the span would overflow the current row. This matches user
expectations: if a 2×2 span is placed after 4 single buttons on a 5-column device, it starts at the next row (button 5) rather than overflowing.

### 5.9 Host: Compound Longpress

**File**: `smearor-swipe-launcher/src/application.rs`, function `get_span_group_for_button` (line ~483)

No changes needed. This function already collects all button indices in a span group by filtering on `span_group` name. It works for 2D layouts because it does
not assume linear ordering — it returns all buttons in the group regardless of their physical position.

---

## 6. Configuration Format

### 6.1 1×2 Horizontal Span (Existing, Backward Compatible)

```toml
plugins = [
    { id = "vol_span_0", widget = "audio_volume_span",
        span_group = "vol_span", span_index = 0 },
    { id = "vol_span_1", widget = "audio_volume_span",
        span_group = "vol_span", span_index = 1 },
]
```

No `span_rows`/`span_cols` specified — defaults to `1×2` (inferred from member count for backward compatibility).

### 6.2 2×1 Vertical Span

```toml
plugins = [
    { id = "vol_span_0", widget = "audio_volume_span",
        span_group = "vol_span", span_index = 0,
        span_rows = 2, span_cols = 1 },
    { id = "vol_span_1", widget = "audio_volume_span",
        span_group = "vol_span", span_index = 1,
        span_rows = 2, span_cols = 1 },
]
```

### 6.3 2×2 Grid Span

```toml
plugins = [
    { id = "sysmon_0", widget = "sysinfo_cpu",
        span_group = "sysmon_grid", span_index = 0,
        span_rows = 2, span_cols = 2 },
    { id = "sysmon_1", widget = "sysinfo_memory",
        span_group = "sysmon_grid", span_index = 1,
        span_rows = 2, span_cols = 2 },
    { id = "sysmon_2", widget = "sysinfo_cpu_temperature",
        span_group = "sysmon_grid", span_index = 2,
        span_rows = 2, span_cols = 2 },
    { id = "sysmon_3", widget = "sysinfo_disk",
        span_group = "sysmon_grid", span_index = 3,
        span_rows = 2, span_cols = 2 },
]
```

### 6.4 Backward Compatibility

When `span_rows` and `span_cols` are both absent, the host falls back to the current behaviour:

- `span_rows = 1` (implicit)
- `span_cols = group_size` (inferred from member count)
- `combined_width = key_width * group_size`
- `combined_height = key_height`
- Linear `button_index` increment

This ensures all existing 1×N span configurations continue to work without modification.

### 6.5 Span Index to Grid Position Mapping

`span_index` is a linear index into the grid, row-major order:

```
span_index = row * span_cols + col

Example (2×2 grid):
  span_index 0 → row 0, col 0 (top-left)
  span_index 1 → row 0, col 1 (top-right)
  span_index 2 → row 1, col 0 (bottom-left)
  span_index 3 → row 1, col 1 (bottom-right)
```

---

## 7. Rendering Pipeline

### 7.1 Full Area Render (`render_buttons_to_device`)

```
1. Iterate plugin entries in order, tracking button_index.

2. For each entry with a span_group:
   a. Collect all consecutive entries with the same span_group.
   b. Sort by span_index.
   c. Read span_rows and span_cols from the first member.
      - If both absent: span_rows=1, span_cols=member_count (backward compat).
      - If present: validate member_count == span_rows * span_cols.
   d. Validate alignment against device key_columns.
   e. Compute combined dimensions:
      - combined_width = key_width * span_cols
      - combined_height = key_height * span_rows
   f. Call render_graphic(combined_width, combined_height) on the first member.
   g. For each member at span_index i:
      - row = i / span_cols
      - col = i % span_cols
      - x_offset = col * key_width
      - y_offset = row * key_height
      - slice = extract_grid_slice(...)
      - physical_button = base_button + row * key_columns + col
      - send_button_image(physical_button, slice)
   h. Advance button_index by span_rows * span_cols.

3. For entries without a span_group: render individually (unchanged).
```

### 7.2 Single Button Update (`render_single_button_to_device`)

```
1. Find the plugin entry for the updated plugin_id.

2. If it has a span_group:
   a. Collect all group members, sort by span_index.
   b. Read span_rows and span_cols from the first member.
   c. Compute combined dimensions (same as above).
   d. Find the base_button (position of the first member in the plugin list).
   e. Re-render the entire group at combined dimensions.
   f. Split and send each slice to its physical button (same as above).

3. If no span_group: render individually (unchanged).
```

---

## 8. Implementation Phases

### Phase 1: Device Grid Metadata

**Order**: First — everything else depends on knowing the device column count.

**Changes**:

- Add `key_columns: u8` to `MacroPadConnectionStatus` in `model/macropad/src/connection_status.rs`.
- Add `key_columns: u8` to `MacroPadDeviceMetadata` in `smearor-swipe-launcher/src/instance/macropad_metadata.rs`.
- Update `MacroPadConnectionStatus::new()` signature.
- Update Stream Deck service to report `key_columns` via lookup table or `DeviceKind` extension.
- Update Loupedeck service to report `layout.columns as u8`.
- Update host connection handler to store `key_columns` in device metadata.

**Exit Criteria**: Both Stream Deck and Loupedeck services broadcast `key_columns` on connect. Host stores it in `MacroPadDeviceMetadata`.

### Phase 2: Plugin Entry Extension

**Order**: After Phase 1.

**Changes**:

- Add `span_rows: Option<u32>` and `span_cols: Option<u32>` to `PluginEntry` in `model/plugin/src/plugin.rs`.
- Do not add to `PluginEntryStabby` (host-side only fields).

**Exit Criteria**: `PluginEntry` parses `span_rows` and `span_cols` from TOML. Existing configs without these fields continue to work.

### Phase 3: Grid Slice Helper

**Order**: After Phase 2.

**Changes**:

- Add `extract_grid_slice()` function in `application.rs`.
- Optionally refactor `extract_horizontal_slice()` to delegate to `extract_grid_slice()`.

**Exit Criteria**: `extract_grid_slice()` correctly extracts rectangular regions from pixel buffers.

### Phase 4: Host Rendering Pipeline

**Order**: After Phase 3.

**Changes**:

- Update `render_buttons_to_device()` to use `span_rows`/`span_cols` for combined dimensions, `extract_grid_slice()` for slicing, and
  `base + row * key_columns + col` for physical button mapping.
- Update `render_single_button_to_device()` with the same logic.
- Add alignment validation (row overflow → advance to next row, bottom overflow → skip with warning).
- Implement backward compatibility fallback: when `span_rows`/`span_cols` are absent, use `span_rows=1, span_cols=member_count`.

**Exit Criteria**: 2D span groups render correctly across multiple rows and columns. Existing 1×N spans continue to work. Alignment validation prevents broken
layouts.

### Phase 5: Configuration & Testing

**Order**: After Phase 4.

**Changes**:

- Add example 2×2 span configuration to `streamdeck.toml` and `streamcontrollerx.toml`.
- Verify all span variants: 1×2, 1×3, 2×1, 3×1, 2×2.
- Test alignment edge cases (span at row boundary, span at device bottom).

**Exit Criteria**: All span configurations render correctly on both Stream Deck and Stream Controller X. Edge cases are handled gracefully with warnings.

---

## 9. Edge Cases

### 9.1 Span at Row Boundary

A 2×2 span starting at button 4 on a 5-column device would overflow:

```
┌────┬────┬────┬────┬────┐
│ B0 │ B1 │ B2 │ B3 │B4  │
├────┼────┼────┼────┼────┤
│ B5 │ B6 │ B7 │ B8 │ B9 │
└────┴────┴────┴────┴────┘

B4 is at col=4, but span_cols=2 needs cols 4 and 5 — col 5 does not exist.
```

**Handling**: Advance `base_button` to the next row start (B5). Log a warning.

### 9.2 Span at Device Bottom

A 2×2 span starting at button 10 on a 5-column, 3-row device:

```
┌────┬────┬────┬────┬────┐
│ B0 │ B1 │ B2 │ B3 │ B4 │
├────┼────┼────┼────┼────┤
│ B5 │ B6 │ B7 │ B8 │ B9 │
├────┼────┼────┼────┼────┤
│B10 │B11 │B12 │B13 │B14 │
└────┴────┴────┴────┴────┘

B10 is at row=2, but span_rows=2 needs rows 2 and 3 — row 3 does not exist.
```

**Handling**: Skip rendering. Log a warning.

### 9.3 Member Count Mismatch

If `span_rows * span_cols` does not match the number of plugin entries in the group:

**Handling**: Log an error. Fall back to the backward-compatible 1×N behaviour (treat as horizontal span with `span_cols = member_count`).

### 9.4 Inconsistent Span Dimensions

If different members of the same group have different `span_rows`/`span_cols` values:

**Handling**: Read the values from the first member only (lowest `span_index`). Log a warning if other members have inconsistent values.

### 9.5 Single-Button Span Group

A span group with only one member (`span_rows=1, span_cols=1`):

**Handling**: Render as a normal single button. No slicing needed.

---

## 10. Security Considerations

No new security concerns. Span group configuration is read from the same TOML config files that are already trusted. No user-supplied input is processed at
runtime for span layout.

---

## 11. Performance Considerations

- `extract_grid_slice()` allocates a `Vec<u8>` per button slice — same as `extract_horizontal_slice()`. No additional allocation overhead.
- Combined render dimensions are larger for 2D spans, but `render_graphic()` is called once per group, not per button. The pixel buffer is then split in-memory,
  which is O (width × height) — negligible compared to rendering.
- Alignment validation is O (1) per span group.

---

## 12. Dependencies

| Dependency                      | Type           | Required For                         |
|---------------------------------|----------------|--------------------------------------|
| `model/macropad`                | Existing crate | `MacroPadConnectionStatus` extension |
| `model/plugin`                  | Existing crate | `PluginEntry` extension              |
| `smearor-swipe-launcher` (host) | Existing crate | Rendering pipeline changes           |
| `services/streamdeck`           | Existing crate | `key_columns` reporting              |
| `services/loupedeck`            | Existing crate | `key_columns` reporting              |
| `plugins/render-utils`          | Existing crate | No changes needed                    |

No new crate dependencies are introduced.
