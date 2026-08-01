# Concept: `smearor-swipe-launcher`

**Touch-optimized, rotatable, and modular scrolling app launcher for Wayland**

---

## 1. System Architecture & Layer Model

The system is based on a strictly decoupled, three-tier architecture. This ensures that the core of the launcher remains stable, while visual components and
logic can be flexibly replaced or extended without having to recompile the entire application.

* **The Core (`smearor-swipe-launcher`):** The main program manages the application window under Wayland, controls spatial orientation (rotation), intercepts
  global gestures, and provides the layout framework.
* **The Interface (`smearor-plugin-api`):** A minimal, stable connection element that defines the contract between the Core and the plugins. It governs how
  widgets are handed over to the main window and how interactions (clicks, holding) are reported back to the plugins.
* **The Extensions (Widget Plugins):** Independent, self-contained code packages compiled into native system libraries (`.so` files). Each plugin encapsulates
  its own logic (e.g., time updates, DBus communication for music or notifications) and generates a specific visual element.

---

## 2. Core Window & UI Layout

The main program is responsible for seamless integration into the Wayland desktop environment and is specifically optimized for physically immersive workspaces.

### Primary Use Case: Table-Top Smart-Desk (65" 4K-Touch)

The primary purpose of the launcher is to provide interactive menu ribbons on all four edges of a horizontal Table-Top Smart-Desk with a **65" 4K touchscreen**.
In this environment, users sit together on different sides of the table and interact collaboratively with the system. This imposes special requirements on the
user interface:

* **Orientation Precision:** The menu ribbons must be perfectly rendered on each edge (0°, 90°, 180°, 270°) and oriented toward the respective side of the
  table, allowing users to read the elements upright from their respective seating positions.
* **Large-scale Touch Optimization:** Due to the massive 65" screen real estate, all touch targets are designed with generous dimensions (Fitts's Law). Icons,
  buttons, and spacing are sized to ensure error-free targeting, even with fast gestures while standing or sitting.
* **4K Scalability:** Fonts and vector graphics are completely scalable and high-resolution, remaining razor-sharp up close as well as from a distance.

### Integration of `smearor-wrot-rotation`

In order to flexibly rotate widgets along the edges of the Smart-Desk, the Core integrates the GTK 4 widget `RotationWidget` from the external library *
*`smearor-wrot-rotation`**:

* **Visual Transformation:** The `RotationWidget` uses high-performance GSK transforms (`GskTransform`) to rotate the entire rendering widget layout in space
  without loss of quality (e.g., by 90° on the left edge or 180° on the top edge).
* **Input Transformation (Input/Output Mapping):** A conventional rotation transform in CSS or GTK often causes touch and mouse coordinates to mismatch with the
  visual elements. `RotationWidget` transforms all touch, mouse, and keyboard events bidirectionally. As a result, swipe and drag gestures on rotated widgets
  behave exactly as they would in the default orientation on every edge.

### Window Management & Positioning

The launcher uses the Wayland Layer Shell protocol. It has no window decorations (borders, title bars) and behaves like a native system panel. The position on
the screen is directly coupled to the display's rotation state:

* **0° (Bottom Table Edge):** The window anchors to the bottom edge and expands horizontally.
* **90° (Left Table Edge):** The window moves to the left edge and expands vertically.
* **180° (Top Table Edge):** The window attaches to the top edge and expands horizontally.
* **270° (Right Table Edge):** The window moves to the right edge and expands vertically.

### The UI Hierarchy (The "Ribbon" Principle)

The window is internally divided into three areas:

1. **Left Static Area:** Accommodates permanent widgets that should not scroll into view or be pushed away (e.g., a permanent clock or menu buttons).
2. **Central Scroll Ribbon:** A high-performance, virtualized container. To guarantee fluid swiping at a constant 120 Hz, even with hundreds of installed apps,
   the scroll ribbon utilizes modern virtualized GTK 4 list widgets (`GtkListView` or `GtkGridView`) in combination with `GskTransform`. Instead of keeping a
   permanent widget in memory for every entry, only the currently visible widgets are created and dynamically recycled during scrolling (widget recycling).
3. **Right Static Area:** Similar to the left side, for permanent elements at the other end of the launcher (e.g., a notification indicator).

---

## 3. Navigation & Gesture Logic

Interaction with the launcher follows an intuitive pattern optimized for touchscreens and keyboard shortcuts:

* **Horizontal Swiping (Left / Right):** Smoothly moves the content of the central scroll ribbon with a finger or mouse.
* **Keyboard Control:** A global keyboard shortcut (e.g., `SUPER + ARROW KEYS`) allows the ribbon to be scrolled incrementally via code if no touchscreen is
  used.
* **Vertical Swiping (Up / Down):** Swiping up opens a parent menu, while swiping down minimizes or closes the launcher.

---

## 4. The Plugin System & Widget Lifecycle

The UI elements are created completely dynamically at runtime based on a high-performance, robust, and secure architecture.

### 4.1 Loading, Instantiation & Two-Way Communication

At startup, the Core reads the configuration. For each entry, the launcher loads the corresponding `.so` file and calls a standardized constructor function of
the plugin.

* **Two-Way Communication (Event Bus):** To enable bidirectional interaction, the Core passes a secure handle to the Core context (`CoreContext` or an
  `EventChannel`) during instantiation. Through this return channel, the plugin can asynchronously send standardized messages to the Core, such as:
    * `RequestClose`: Close the launcher after successfully starting an app.
    * `TriggerParentMenu`: Open a parent menu or move the ribbon.
    * `EmitNotification`: Trigger a system notification.
* **State Separation (Virtualization Interface):** To support the UI virtualization of the scroll ribbon, the API strictly separates the plugin's pure data
  structure (state) from the actual UI generation. The plugin provides the Core with a lightweight data object (model), whereas binding to the real, recycled
  GTK widget only occurs when it becomes visible on the screen.

### 4.2 Non-Blocking Threading Model

GTK 4 operates strictly single-threaded on the main thread (UI thread). Any blocking operation of a plugin (e.g., reading a `.desktop` file from a slow disk,
waiting for a DBus response via MPRIS, or network requests) would freeze the entire user interface, including all animations.

* **Asynchronous Loading & Shimmer Widgets:** The plugin trait prohibits time-consuming calculations in the initialization and widget creation steps. Instead,
  plugins immediately build an empty skeleton widget (shimmer/placeholder/skeleton).
* **Background Tasks:** Heavy I/O or computational tasks are offloaded to asynchronous tasks (e.g., via the Tokio runtime or using a
  `glib::MainContext::channel`).
* **UI Updates in the GTK Main Thread:** Once the data is ready, the async task sends a message back to the GTK main thread, which fills the skeleton widget
  with the real data safely and without blocking.

### 4.3 Interaction Pipeline

Touch detection is handled centrally by the Core launcher to guarantee consistent system behavior:

* **Primary Action (Short Tap / Click):** The Core detects the release of the finger and signals the plugin to execute its primary function (e.g., launching an
  app or pausing music). The current rotation is communicated to the plugin, allowing launched apps to be invoked with the appropriate rotation parameter.
* **Secondary Action (Long Press / Long touch):** The Core registers the holding of an element and instructs the plugin to execute the secondary operation (
  e.g., opening a context menu or settings dialog).

### 4.4 Memory Integrity at the FFI Boundary

Passing Rust trait objects across dynamic library boundaries (`libloading`) carries significant risks of memory errors (undefined behavior) if conventional
`Box::into_raw` and `Box::from_raw` patterns are used:

1. **Allocator Mismatch:** If the Core launcher and the plugin are compiled with different compiler versions, optimization levels, or allocators (e.g.,
   `jemalloc` in one and the system allocator in the other), deallocating the memory in the main program will cause an immediate crash.
2. **Lack of Destructor Control:** The Core must not arbitrarily free a plugin's memory within its own context.

* **Solution (Plugin-Controlled Deallocation):** In addition to its constructor, each plugin exports a dedicated FFI function:
  ```rust
  unsafe extern "C" fn _smearor_plugin_destroy(ptr: *mut dyn MenuEntry)
  ```
  When the launcher unloads or closes a plugin, it passes the pointer back to this plugin function. This ensures that the plugin frees its own resources using
  the exact same compiler and allocator that created them. Alternatively, memory handling is delegated to a C-compatible `ffi::CustomDeleter` pattern.

---

## 5. Configuration Design

The entire interface is configured via a single, central text file (e.g., in TOML format). This file is divided into three sections, reflecting the UI layout:

* Global settings (animation speeds, default rotation).
* Lists for the respective positions (left persistent, middle scrollable, right persistent).

Each entry defines only the system path to the desired plugin library and an arbitrary block of variables. The Core launcher does not need to understand the
widgets' logic; it simply passes the specific parameters blindly to the respective plugin. This allows users to add new widgets or change parameters without
modifying the launcher's source code.