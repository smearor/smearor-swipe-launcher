# Development Plan: `smearor-swipe-launcher`

## Phase 1: The Foundation & The Interface (Minimum Viable Product)

*Focus: Setting up the architecture, displaying a window, and loading the first plugin at runtime.*

### 🏁 Milestone 1.1: The API & The Test Plugin

* [x] **Set up Project Workspace:** Creation of the Rust workspace structure with the core launcher, the API library, and a directory for plugins.
* [x] **Define the Plugin API (State Separation):** Design of the core trait with a strict separation between the data model (state) and UI generation (
  preparing for virtualization), including passing a `CoreContext` / `EventChannel` to the constructor.
* [x] **C-ABI Memory Protection at the FFI Boundary:** Integration of a safe allocation and destruction pattern (e.g., FFI export of `_smearor_plugin_destroy`)
  into the API definition, so plugins can clean up their own memory cleanly.
* [x] **The First Functional Plugin:** Development of a minimal time or text widget as a dynamic library (`.so`) implementing the interface.

### 🏁 Milestone 1.2: The Plugin Loader in the Core

* [x] **Dynamic Loading (`libloading`):** Implementation of the logic in the core launcher that opens a `.so` file at runtime, calls the constructor, passes the
  `CoreContext` to the plugin, and holds the plugin safely in memory.
* [x] **Memory Safety & Allocator Safety:** Implementation of a registration and cleanup system in the Core that passes the pointer back to the plugin-internal
  FFI destructor function when a plugin is closed to avoid allocator mismatches.
* [x] **First Visual Life Sign:** A simple GTK 4 window successfully opens and displays plugin widgets. Plugins supply data (state) over FFI, Core creates UI
  based on this data (state separation implemented).

---

## Phase 2: Wayland Integration & The "Ribbon" Layout

*Focus: Adapting the window to the system and building the touch-optimized navigation.*

### 🏁 Milestone 2.1: The Scrollable Ribbon & Gesture Recognition

* [x] **Three-Part Layout:** Building the UI structure in the main window (Left static area, central scroll container, right static area).
* [x] **Centralized Gesture Handling:** The Core intercepts clicks (taps) and long presses on the dynamic widgets and routes them correctly to the primary and
  secondary functions of the respective plugin.

---

## Phase 3: Dynamic Configuration & Rotation

*Focus: Making the launcher controllable via a file and reacting to hardware changes.*

### 🏁 Milestone 3.1: TOML Parser & JSON Bridge

* [x] **Configuration Engine:** Implementation of a parser (via Serde) that reads the central `config.toml`.
* [x] **Dynamic UI Building:** The Core iterates through the lists for left, right, and the scroll ribbon, loads the defined plugins, and inserts them into the
  correct positions in the layout.
* [x] **Parameter Passing:** The Core extracts the specific settings blocks from the TOML file and passes them as a JSON interface to the respective plugin
  during loading.

### 🏁 Milestone 3.2: Rotation & Position Synchronization

* [x] **Integration of `smearor-wrot-rotation`:** Integration of the `RotationWidget` from the external library to cleanly rotate the widgets on the respective
  table edges (90°, 180°, 270°) both visually and interactively (input/output coordinate mapping).
* [x] **Rotation Parameter Propagation:** Extension of the primary action so that the current display orientation is determined and passed to the plugin upon
  execution (preparing for the `--rotation` parameter on app launch).

---

## Phase 4: Core Widgets (The App Ecosystem Basis)

*Focus: Developing the primary plugins that make the launcher useful.*

### 🏁 Milestone 4.1: The App Launcher Plugin

* [ ] **Non-blocking Desktop Entry Parser:** Development of an asynchronous plugin that reads system `.desktop` files in the background (Tokio / GLib channel).
  Meanwhile, the UI displays a shimmer/placeholder skeleton that is updated in a non-blocking manner after loading.
* [ ] **Two-Way App Execution:** Implementation of the launch logic. On click, the app is launched – taking the rotation parameter into account. Upon successful
  launch, the plugin sends a `RequestClose` signal to the Core via the event channel to close the launcher.

### 🏁 Milestone 4.2: The MPRIS Media Plugin

* [ ] **Asynchronous MPRIS Plugin:** Development of a media widget that attaches asynchronously to active players via DBus. DBus communication runs in a
  background task and never blocks the GTK UI thread (ensuring no stuttering while swiping).
* [ ] **Touch Media Controls:** Clear widget layout showing album art and large touch buttons for play/pause/skip, optimized for table edge interaction.

### 🏁 Milestone 4.3: The Time & Calendar Widget

* [ ] **Precise Time Widget:** Integration of a high-performance, non-blocking time display widget with configurable layouts (analog/digital) and time zones.
* [ ] **Skeleton Calendar Overview:** Touch interaction on the clock opens a small, smoothly rotated calendar overview with upcoming calendar events, loaded
  asynchronously in the background.
* [ ] **Table Scaling:** Optimization of font size and contrast to make the time readable on the 65" table from any direction.

### 🏁 Milestone 4.4: The Notification Widget

* [ ] **DBus Notification Daemon Listener:** Integration of an asynchronous receiver for the standard `org.freedesktop.Notifications` DBus service inside the
  plugin.
* [ ] **Notification Banner & Badge:** Display of a notification counter in the permanent area and smooth slide-in of new banners on the menu ribbon.
* [ ] **Touch Gesture Interaction:** Swipe-to-dismiss gestures to close notifications, specifically designed for the large touchscreen.

### 🏁 Milestone 4.5: Layer-Shell & Positioning

* [ ] **Layer-Shell-Integration:** Integration of the Wayland protocol to completely remove window decorations and define the launcher as a system panel.
* [ ] **Exclusive Zones:** Configuration of the window so that it is fixed to the bottom of the screen by default and pushes other open application windows
  aside when necessary.
* [ ] **Dynamic Layer Adjustment:** Implementation of logic that shifts the window to the correct screen edge at runtime when a rotation change occurs (0°
  bottom, 90° left, etc.) and mirrors the layout from horizontal to vertical.

### 🏁 Milestone 4.6: Virtualization of the Scroll Ribbon (Performance)

* [ ] **GtkListView/GridView Integration:** Use of modern GTK 4 list widgets (`GtkListView`/`GtkGridView`) in combination with `GskTransform` in the central
  scroll area to achieve highly efficient widget recycling with large amounts of data (e.g., 200+ apps).

### 🏁 Milestone 4.7: Touch Optimization for 65" 4K Smart-Desks

* [ ] **Touch Optimization for 65" 4K Smart-Desks:** Sizing all widget dimensions, icons, and spacing for the 65-inch touch experience (Fitts's Law) and
  ensuring razor-sharp scalability.

---

## Phase 5: Polishing, Performance & Fine-Tuning

*Focus: Refining gestures, optimizing performance, and making the system ready for daily use.*

### 🏁 Milestone 5.1: Advanced Gestures & Shortcuts

* [ ] **Vertical Swipes:** Implementation of detection for swipe gestures upwards (to invoke the Parent Menu) and downwards (to minimize the launcher).
* [ ] **Keyboard Navigation:** Registration of global shortcuts (`SUPER + ARROW KEYS`) to control the ribbon precisely even without a touchscreen.

### 🏁 Milestone 5.2: CSS Styling & Robustness

* [ ] **Hot-Reloading CSS:** Integration of a GTK CSS structure that allows custom styling of the launcher's appearance (colors, spacing, rounded corners) via
  an external stylesheet file, ideally with live reloading upon changes.
* [ ] **Performance Fine-Tuning of Virtualization:** Profiling and optimizing swipe animations at 120 Hz under maximum load with hundreds of virtualized list
  entries.
* [ ] **Error Encapsulation (Panic Handling):** Protecting the core launcher against faulty third-party plugins. If a widget crashes during operation, the Core
  intercepts it, hides the widget, and prevents the entire launcher from crashing.