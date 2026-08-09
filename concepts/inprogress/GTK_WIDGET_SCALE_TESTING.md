# Phase 5: Widget Scale Testing Checklist

This checklist documents the verification steps for the global GTK widget scaling system. Unit tests cover the pure-logic components (`sanitize_scale`, scaled
accessors, `register_css_once`). The remaining tests require a running GTK display and must be verified manually.

## Unit Tests (Automated)

All unit tests are in `plugin-api/src/widget/` and run via `cargo test -p smearor-swipe-launcher-plugin-api`.

### `sanitize_scale` (css.rs)

- [x] `scale = 1.0` → returns `1.0` (no-op)
- [x] `scale = 0.0` → clamped to `0.5` (SCALE_MIN)
- [x] `scale = -1.0` → clamped to `0.5`
- [x] `scale = 0.49` → clamped to `0.5`
- [x] `scale = 3.5` → clamped to `3.0` (SCALE_MAX)
- [x] `scale = 10.0` → clamped to `3.0`
- [x] `scale = NaN` → returns `1.0`
- [x] `scale = +Infinity` → returns `1.0`
- [x] `scale = -Infinity` → returns `1.0`
- [x] `scale = 0.5` → returns `0.5` (boundary)
- [x] `scale = 3.0` → returns `3.0` (boundary)
- [x] `scale = 1.5, 2.0, 0.75` → unchanged (mid-range)

### `WidgetDimensions` scaled accessors (dimensions.rs)

- [x] `width_scaled(1.0)` with `None` → `DEFAULT_WIDGET_WIDTH` (100)
- [x] `width_scaled(2.0)` with `None` → 200
- [x] `width_scaled(0.5)` with `None` → 50
- [x] `width_scaled` with `Some(150)` → 150, 300, 75 at scales 1.0, 2.0, 0.5
- [x] `height_scaled` with `None` → `DEFAULT_WIDGET_HEIGHT` (100), 200, 50
- [x] `height_scaled` with `Some(80)` → 80, 120 at scales 1.0, 1.5
- [x] `max_width_scaled(Wide, 1.0)` with `None` → `DEFAULT_WIDE_MODE_WIDGET_WIDTH` (300)
- [x] `max_width_scaled(Compact, 1.0)` with `None` → `DEFAULT_WIDGET_WIDTH` (100)
- [x] `max_width_scaled` with `Some(250)` → 250, 500 at scales 1.0, 2.0
- [x] `effective_width_scaled` → `min(width_scaled, max_width_scaled)`
- [x] `scale` field defaults to `None`
- [x] `scale` field round-trips through serde
- [x] `scale` absent in JSON → `None`

### `WidgetLayout` scaled accessors (layout.rs)

- [x] `spacing_scaled(1.0)` with `None` → `DEFAULT_WIDGET_SPACING` (0)
- [x] `spacing_scaled` with `Some(10)` → 10, 20, 15, 5 at scales 1.0, 2.0, 1.5, 0.5
- [x] `spacing_scaled` rounds correctly (7 * 1.5 = 11, 7 * 0.75 = 5)

### `WidgetIcon` scaled accessors (icons.rs)

- [x] `icon_size_scaled(1.0)` → `DEFAULT_ICON_SIZE` (36)
- [x] `icon_size_scaled(2.0)` → 72
- [x] `icon_size_scaled(0.5)` → 18
- [x] `icon_size_scaled` with custom `icon_size = 24` → 24, 36, 72 at scales 1.0, 1.5, 3.0
- [x] `icon_size_scaled` rounds correctly (17 * 1.5 = 26, 17 * 0.75 = 13)

### `register_css_once` deduplication (css.rs)

- [x] Does not panic without a GDK display (headless)
- [x] Calling twice with the same key does not panic (deduplication)

## Visual Tests (Manual — requires running launcher)

These tests require launching the launcher with different `scale` values and verifying the visual output. Run with:

```bash
# Set scale in config.toml under [launcher]:
# scale = 1.5

# Then launch:
cargo run --bin smearor-swipe-launcher
```

### Scale = 1.0 (No Regression)

- [x] All widgets render identically to pre-scaling behavior
- [x] No visual artifacts or layout shifts
- [x] Font sizes match `style.css` defaults (14px main, 10px info, 1.5em icons, 32px clock)

### Scale = 0.5 (Compact)

- [x] All widget dimensions are approximately half size
- [x] Icons are half size
- [x] Spacing between elements is halved
- [x] Font sizes are visibly smaller but readable
- [x] No GTK panics or critical warnings

### Scale = 1.5 (High-DPI)

- [x] All widget dimensions are 1.5x larger
- [x] Icons are 1.5x larger
- [x] Spacing is 1.5x
- [x] Font sizes are visibly larger
- [x] No overlap or clipping
- [x] No GTK panics or critical warnings

### Scale = 2.0 (Accessibility)

- [x] All widget dimensions are doubled
- [x] Icons are doubled
- [x] Font sizes are doubled
- [x] Layout remains usable (may need area width adjustment)
- [x] No GTK panics or critical warnings

### Per-Widget Scale Override

```toml
[launcher]
scale = 1.5

[my_small_widget]
scale = 1.0  # overrides global

[my_large_widget]
scale = 2.0  # overrides global
```

- [x] Widget with `scale = 1.0` renders at default size despite global `1.5`
- [x] Widget with `scale = 2.0` renders at 2x despite global `1.5`
- [x] Per-widget override also scales CSS font sizes (scoped `.scale-100` / `.scale-200` class applied)
- [x] Other widgets remain at global `1.5`

### Edge Cases

- [ ] `scale = 0.0` in config → clamped to `0.5`, no crash
- [ ] `scale = -5.0` in config → clamped to `0.5`, no crash
- [ ] `scale = NaN` in config → falls back to `1.0`, no crash
- [ ] `scale = 999.0` in config → clamped to `3.0`, no crash

### Max-Width CSS Scaling (Wide Mode)

- [ ] Widget with `max_width` in Wide mode scales the CSS `max-width` constraint correctly
- [ ] No duplicate CSS providers after widget rebuild (change layout / reload config)

### Atomic Widgets (Not Affected)

- [ ] Stream Deck / Loupedeck widgets are unaffected by `scale` (physical dimensions)
- [ ] Atomic widget rendering uses device dimensions, not `WidgetDimensions`

### CSS Provider Lifecycle

- [ ] Rebuild a widget multiple times (e.g. by switching layouts)
- [ ] Verify no duplicate `CssProvider` instances accumulate (check with GTK inspector)
- [ ] `register_css_once` prevents duplicate registration for the same key
