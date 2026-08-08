# Concept: File Browser Widget

This document describes the concept for a **File Browser Widget** — a Multi-Span Widget designed exclusively for MacroPad devices. It allows the user to browse
directories and open files using physical buttons. Each button in the visible area represents one entry in the current directory: directories can be entered,
files can be opened with the system's default application, and a dedicated "Parent" button navigates back to the parent directory.

This concept builds upon the **Multi-Span Widget** framework and **Input Triggers** defined in `MACROPAD_ATOMIC_WIDGETS.md`. The File Browser Widget was briefly
outlined in Section 8 of that document; this paper expands it into a full implementation specification.

The system follows the decoupled SOA architecture:

1. **Model Crate (`model/file-browser`):** Shared structs, enums, topics, and message formats for directory entries and navigation actions.
2. **Widget Crate (`plugins/file-browser`):** Headless-only widget that reads directory contents, renders entries to pixel buffers, and handles navigation and
   file opening via button presses.

No service crate is required — the widget reads the filesystem directly using `std::fs` and opens files via `gio::AppInfo::launch_default_for_uri`. There is no
long-running background process, no periodic polling, and no state to broadcast beyond the widget's own internal directory state.

---

## 1. Problem & Motivation

On a MacroPad device, the user interacts exclusively through physical buttons. There is no touch surface, no mouse, and no file manager window. To browse and
open files without switching to a desktop environment, the MacroPad itself must present directory contents on its button displays and map button presses to
navigation and file-launching actions.

The File Browser Widget turns the MacroPad button grid into a miniature file manager:

- **Directories** are displayed as folder icons with labels. Pressing a directory button enters that directory and re-renders the grid with its contents.
- **Files** are displayed with type-specific icons and labels. Pressing a file button opens it with the system's default application.
- **Parent navigation** is always available as the first button, allowing the user to traverse back up the directory tree.
- **Pagination** handles directories with more entries than available buttons.

The widget only makes sense on a MacroPad with enough buttons to display a useful number of entries. On a 5×3 grid (15 buttons), one button is reserved for
"Parent", leaving 14 entries per page. On smaller grids (e.g. 3×2 = 6 buttons), the widget is still functional but can only show 5 entries per page.

---

## 2. Feature Scope

| Feature                      | Description                                                                                                               |
|------------------------------|---------------------------------------------------------------------------------------------------------------------------|
| **Directory Browsing**       | Navigate into directories by pressing the corresponding button.                                                           |
| **Parent Navigation**        | A dedicated "Parent" button (always first) navigates to the parent directory.                                             |
| **File Opening**             | Pressing a file button opens the file with the system's default application (`gio::AppInfo::launch_default_for_uri`).     |
| **Pagination**               | Directories with more entries than available buttons are paginated. A "More" button on the last slot shows the next page. |
| **File Type Icons**          | Each file type maps to a Nerd Font icon (images, videos, audio, PDF, text, etc.).                                         |
| **Hidden File Filtering**    | Hidden files (dotfiles) are hidden by default, configurable via `show_hidden`.                                            |
| **Configurable Home Folder** | The starting directory is configurable via `home_folder`.                                                                 |
| **Longpress Actions**        | Longpress on a file shows file info (size, modified date). Longpress on a directory opens it in the system file manager.  |
| **Headless-Only**            | The widget is designed for MacroPad (`InstanceType::Headless`). It does not implement `WidgetBuilder` for GTK.            |

---

## 3. Navigation Flow

```
Home Folder (Start: ~/)
┌────────┬────────┬────────┐
│ Parent │ Bilder │ Videos │
│  📁↑   │  📁   │  📁   │
├────────┼────────┼────────┤
│ Musik  │ Docs   │ More   │
│  📁   │  📁   │  →    │
└────────┴────────┴────────┘

User presses "Bilder" (enter directory):
┌────────┬────────┬────────┐
│ Parent │ BGs    │ More   │
│  📁↑   │  📁   │  →    │
├────────┼────────┼────────┤
│ img1   │ img2   │ img3   │
│  🖼️   │  🖼️   │  🖼️   │
└────────┴────────┴────────┘

User presses "BGs" (enter subdirectory):
┌────────┬────────┬────────┐
│ Parent │ img.png│ vid.mp4│
│  📁↑   │  🖼️   │  🎬   │
├────────┼────────┼────────┤
│ aud.mp3│ ...    │ ...    │
│  🎵   │        │        │
└────────┴────────┴────────┘

User presses "img.png" (open file):
→ gio::AppInfo::launch_default_for_uri("file:///home/user/Bilder/BGs/img.png")
→ Opens default image viewer

User presses "Parent" (navigate back):
┌────────┬────────┬────────┐
│ Parent │ BGs    │ More   │
│  📁↑   │  📁   │  →    │
├────────┼────────┼────────┤
│ img1   │ img2   │ img3   │
│  🖼️   │  🖼️   │  🖼️   │
└────────┴────────┴────────┘
```

### 3.1 Detailed Navigation Example

```
Home Folder (Start: ~/)
┌──────────────────────────────────────────────────────────┐
│                                                          │
│  Bilder (Press → Enter Directory → Show Contents)        │
│  ├── Parent (Press → Parent Directory → Show Home)       │
│  ├── Backgrounds (Press → Enter Directory)               │
│  │   ├── Parent (Press → Parent → Show Bilder)           │
│  │   └── image.png (Press → Open default PNG viewer)     │
│  │                                                        │
│  Videos (Press → Enter Directory → Show Contents)        │
│  ├── Parent (Press → Parent Directory → Show Home)       │
│  ├── Backgrounds (Press → Enter Directory)               │
│  │   ├── Parent (Press → Parent → Show Videos)           │
│  │   └── video.mp4 (Press → Open default video player)   │
│  │                                                        │
│  Music (Press → Enter Directory → Show Contents)         │
│  ├── Parent (Press → Parent Directory → Show Home)       │
│  ├── Backgrounds (Press → Enter Directory)               │
│  │   ├── Parent (Press → Parent → Show Music)            │
│  │   └── audio.mp3 (Press → Open default audio player)   │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

---

## 4. System Architecture & Data Flow

```
+--------------------------+
| File Browser Widget      |
| (Headless / MacroPad)    |
|                          |
|  1. Read directory       |
|     std::fs::read_dir    |
|                          |
|  2. Sort entries         |
|     directories first,   |
|     then files,          |
|     alphabetical         |
|                          |
|  3. Render to buttons    |
|     render_graphic()     |
|     per-button pixel     |
|     buffer               |
|                          |
|  4. Handle button press  |
|     InvokeToolMessage    |
|     → enter / parent /   |
|       open / next_page / |
|       prev_page          |
|                          |
|  5. Open file            |
|     gio::AppInfo::       |
|     launch_default_for_  |
|     uri                  |
+--------------------------+
         |
         | WidgetUpdateMessage
         | Topic: "widget.update"
         v
+--------------------------+
| Host (host/mod.rs)    |
|                          |
|  Receives widget.update  |
|  → render_buttons_to_    |
|    device()              |
|  → SetButtonImage per    |
|    button                |
+--------------------------+
```

No service crate is involved. The widget is self-contained: it reads the filesystem, manages its own navigation state, renders pixel buffers, and opens files.
The only external interaction is broadcasting `WidgetUpdateMessage` to trigger re-rendering on the MacroPad device.

---

## 5. Crate Structure

Following the workspace conventions (`AGENTS.md`), the feature is split into two crates:

| Crate      | Path                    | Responsibility                                                          |
|------------|-------------------------|-------------------------------------------------------------------------|
| **Model**  | `model/file-browser/`   | Shared structs, enums, topics, file type icon mapping                   |
| **Widget** | `plugins/file-browser/` | Headless widget: directory reading, rendering, navigation, file opening |

No service crate is needed — the widget has no background logic, no periodic tasks, and no state to broadcast to other components.

---

## 6. Model Crate (`model/file-browser`)

### 6.1 Message Topics

```rust
/// Topic for widget update notifications (re-render trigger).
/// The widget broadcasts this when the directory view changes.
pub const TOPIC_WIDGET_UPDATE: &str = "widget.update";
```

### 6.2 File Browser Action Enum

```rust
/// Actions the file browser widget can perform in response to button presses.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[stabby::stabby]
pub enum FileBrowserAction {
    /// Enter a directory (payload contains the directory path).
    EnterDirectory,
    /// Navigate to the parent directory.
    #[default]
    Parent,
    /// Open a file with the default application (payload contains the file path).
    OpenFile,
    /// Show the next page of directory entries.
    NextPage,
    /// Show the previous page of directory entries.
    PrevPage,
}
```

### 6.3 Directory Entry Struct

```rust
/// A single entry in a directory listing.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[stabby::stabby]
pub struct DirectoryEntry {
    /// Display name of the entry (filename without path).
    pub name: stabby::string::String,
    /// Full absolute path of the entry.
    pub path: stabby::string::String,
    /// Whether the entry is a directory.
    pub is_directory: bool,
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// Last modified time as Unix timestamp (seconds since epoch).
    pub modified: u64,
    /// File extension (lowercase, without dot). Empty for directories or files without extension.
    pub extension: stabby::string::String,
}
```

### 6.4 File Browser State Struct

```rust
/// Current navigation state of the file browser widget.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct FileBrowserState {
    /// Current directory path being displayed.
    pub current_path: stabby::string::String,
    /// Current page index (0-based).
    pub current_page: u32,
    /// Total number of pages.
    pub total_pages: u32,
    /// Entries on the current page.
    pub entries: stabby::vec::Vec<DirectoryEntry>,
    /// Whether the "Parent" button is available (false at filesystem root).
    pub has_parent: bool,
}
```

### 6.5 File Type Icon Mapping

Each file extension maps to a Nerd Font Material Design icon for consistent rendering:

| Category  | Extensions                                                                  | Icon | Nerd Font Name        |
|-----------|-----------------------------------------------------------------------------|------|-----------------------|
| Image     | png, jpg, jpeg, gif, bmp, webp, svg, tiff                                   | 🖼️   | `nf-md-file_image`    |
| Video     | mp4, mkv, webm, avi, mov, flv, wmv                                          | 🎬   | `nf-md-file_video`    |
| Audio     | mp3, flac, wav, ogg, m4a, aac                                               | 🎵   | `nf-md-file_music`    |
| PDF       | pdf                                                                         | 📄   | `nf-md-file_pdf`      |
| Text      | txt, md, rst                                                                | 📝   | `nf-md-file_document` |
| Code      | rs, py, js, ts, go, c, cpp, h, java, kt, rb, sh, toml, json, yaml, yml, xml | 📋   | `nf-md-code_braces`   |
| Archive   | zip, tar, gz, bz2, xz, 7z, rar                                              | 📦   | `nf-md-zip_box`       |
| Directory | (directories)                                                               | 📁   | `nf-md-folder`        |
| Parent    | (parent entry)                                                              | 📁↑  | `nf-md-folder_upload` |
| More      | (pagination)                                                                | →    | `nf-md-chevron_right` |
| Default   | (unknown extensions)                                                        | 📄   | `nf-md-file`          |

The mapping is defined in the model crate as a utility function:

```rust
/// Returns the Nerd Font icon name for a file extension.
pub fn file_type_icon(extension: &str) -> &'static str {
    match extension.to_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "tiff" => "nf-md-file_image",
        "mp4" | "mkv" | "webm" | "avi" | "mov" | "flv" | "wmv" => "nf-md-file_video",
        "mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac" => "nf-md-file_music",
        "pdf" => "nf-md-file_pdf",
        "txt" | "md" | "rst" => "nf-md-file_document",
        "rs" | "py" | "js" | "ts" | "go" | "c" | "cpp" | "h" | "java" | "kt" | "rb" | "sh"
        | "toml" | "json" | "yaml" | "yml" | "xml" => "nf-md-code_braces",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "nf-md-zip_box",
        _ => "nf-md-file",
    }
}

/// Returns the Nerd Font icon name for a directory entry.
pub fn directory_icon() -> &'static str {
    "nf-md-folder"
}

/// Returns the Nerd Font icon name for the parent directory entry.
pub fn parent_icon() -> &'static str {
    "nf-md-folder_upload"
}

/// Returns the Nerd Font icon name for the "More" (pagination) entry.
pub fn more_icon() -> &'static str {
    "nf-md-chevron_right"
}
```

### 6.6 Model Crate `lib.rs`

```rust
mod icon;
mod json_converters;
mod messages;

pub use icon::file_type_icon;
pub use icon::directory_icon;
pub use icon::more_icon;
pub use icon::parent_icon;
pub use messages::action::FileBrowserAction;
pub use messages::entry::DirectoryEntry;
pub use messages::state::FileBrowserState;
pub use json_converters::register_json_converters;
```

### 6.7 File Structure

```
model/file-browser/
  Cargo.toml
  src/
    lib.rs
    json_converters.rs
    icon.rs                       # file_type_icon, directory_icon, parent_icon, more_icon
    messages/
      mod.rs
      action.rs                   # FileBrowserAction
      entry.rs                    # DirectoryEntry
      state.rs                    # FileBrowserState
```

---

## 7. Widget Crate (`plugins/file-browser`)

### 7.1 File Structure

- `widget.rs` - `FileBrowserWidget` struct and trait implementations
- `config.rs` - `FileBrowserWidgetConfig` struct and parsing
- `lib.rs` - `widget_plugin!` macro invocation

### 7.2 Widget Struct

```rust
/// File browser widget for MacroPad devices.
/// Displays directory contents on the button grid and handles
/// navigation and file opening via button presses.
pub struct FileBrowserWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: FileBrowserWidgetConfig,
    pub state: Arc<RwLock<FileBrowserState>>,
    pub broker: MessageBrokerHandle,
}
```

**Trait Implementations:**

- `MessageHandler<FfiEnvelopePayload<InvokeToolMessage>>` - Handles button press actions (enter, parent, open, next_page, prev_page)
- `MessageBroadcaster` - Broadcasts `WidgetUpdateMessage` on directory change to trigger re-render
- `PluginMetaGetter` - Returns plugin metadata
- `AsRef<Option<FfiCoreContext>>` - Provides access to the core context
- `GraphicRenderer` - Renders directory entries to per-button pixel buffers (headless only)

The widget does **not** implement `WidgetBuilder` — it is headless-only and not intended for GTK instances.

### 7.3 Widget Configuration

```rust
/// Configuration for the file browser widget.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FileBrowserWidgetConfig {
    /// Starting directory (absolute path or ~ for home).
    /// Defaults to "~" (home directory).
    pub home_folder: Option<String>,
    /// Whether to show hidden files (dotfiles).
    /// Defaults to false.
    pub show_hidden: Option<bool>,
    /// Maximum number of entries to show per page.
    /// Defaults to the button count of the MacroPad grid.
    pub max_entries: Option<u32>,
    /// Custom icon overrides per file extension.
    /// Keys are lowercase extensions without dot, values are Nerd Font icon names.
    /// Example: { "png" = "nf-md-image", "mp4" = "nf-md-movie" }
    pub icons: Option<std::collections::HashMap<String, String>>,
}

impl FileBrowserWidgetConfig {
    pub fn parse(config_json: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config_json.clone())
    }

    /// Returns the home folder path, expanding ~ to the user's home directory.
    pub fn resolved_home_folder(&self) -> PathBuf {
        let raw = self.home_folder.as_deref().unwrap_or("~");
        expand_tilde(raw)
    }

    /// Returns whether hidden files should be shown.
    pub fn should_show_hidden(&self) -> bool {
        self.show_hidden.unwrap_or(false)
    }

    /// Returns the max entries per page, or None to use the button count.
    pub fn resolved_max_entries(&self) -> Option<u32> {
        self.max_entries
    }
}
```

### 7.4 Directory Reading and Sorting

The widget reads directory contents using `std::fs::read_dir` and sorts entries:

```rust
/// Reads directory contents and returns sorted entries.
fn read_directory(
    path: &Path,
    show_hidden: bool,
) -> Result<Vec<DirectoryEntry>, FileBrowserError> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files if not enabled
        if !show_hidden && name.starts_with('.') {
            continue;
        }

        let extension = entry
            .path()
            .extension()
            .map(|ext| ext.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or(0);

        entries.push(DirectoryEntry {
            name: name.clone(),
            path: entry.path().to_string_lossy().to_string(),
            is_directory: metadata.is_dir(),
            size: if metadata.is_file() { metadata.len() } else { 0 },
            modified,
            extension,
        });
    }

    // Sort: directories first (alphabetical), then files (alphabetical)
    entries.sort_by(|a, b| {
        match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(entries)
}
```

### 7.5 Pagination

The widget paginates directory entries to fit the available button count:

```rust
/// Calculates the entries to display on the current page.
fn calculate_page_entries(
    all_entries: &[DirectoryEntry],
    page: u32,
    max_entries: u32,
    has_parent: bool,
) -> (Vec<DirectoryEntry>, bool) {
    // Reserve one button for "Parent" if applicable
    let available_slots = if has_parent {
        max_entries.saturating_sub(1)
    } else {
        max_entries
    };

    // Reserve one button for "More" if there are more pages
    let total_pages = (all_entries.len() as u32 + available_slots - 1) / available_slots;
    let has_more = page + 1 < total_pages;

    let slots_for_entries = if has_more {
        available_slots.saturating_sub(1)
    } else {
        available_slots
    };

    let start = (page * slots_for_entries) as usize;
    let end = std::cmp::min(start + slots_for_entries as usize, all_entries.len());

    let page_entries = all_entries[start..end].to_vec();
    (page_entries, has_more)
}
```

### 7.6 Rendering

The widget implements `GraphicRenderer` to render each button's pixel buffer. Each button displays:

- **Icon**: Centered Nerd Font icon (40×40 px), type-specific for files, folder icon for directories.
- **Label**: Bottom-aligned filename (10–12 px font, max 8 characters, truncated with ellipsis).

Rendering uses the shared utilities from `plugins/render-utils` (defined in `HEADLESS_WIDGETS_CONCEPT.md`):

```rust
impl GraphicRenderer for FileBrowserWidget {
    fn render_graphic(&self, width: u32, height: u32) -> Option<FfiGraphic> {
        let state = self.state.read().ok()?;
        let mut image = RgbaImage::new(width, height);

        fill_background(&mut image, COLOR_BACKGROUND);

        // Determine which entry this button represents
        let button_index = /* provided by host context */;
        let entry = self.entry_for_button(button_index, &state)?;

        match entry {
            EntrySlot::Parent => {
                draw_icon(&mut image, parent_icon(), 40, COLOR_TEXT);
                draw_label(&mut image, "Parent", COLOR_TEXT);
            }
            EntrySlot::Directory(dir) => {
                draw_icon(&mut image, directory_icon(), 40, COLOR_TEXT);
                draw_label(&mut image, &truncate(&dir.name, 8), COLOR_TEXT);
            }
            EntrySlot::File(file) => {
                let icon = self.config.icon_for_extension(&file.extension);
                draw_icon(&mut image, icon, 40, COLOR_TEXT);
                draw_label(&mut image, &truncate(&file.name, 8), COLOR_TEXT);
            }
            EntrySlot::More => {
                draw_icon(&mut image, more_icon(), 40, COLOR_TEXT);
                draw_label(&mut image, "More", COLOR_TEXT);
            }
            EntrySlot::Empty => {
                // Clear button — no icon, no label
            }
        }

        Some(FfiGraphic::from_image(&image))
    }
}
```

### 7.7 Button Press Handling

The widget handles `InvokeToolMessage` actions for navigation and file opening:

```rust
impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for FileBrowserWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>) {
        let action_payload = &message.payload;
        let action_str = action_payload.action.as_str();

        match action_str {
            "enter" => {
                // Payload contains the directory path to enter
                let path = action_payload.path.as_str();
                self.navigate_to(PathBuf::from(path));
            }
            "parent" => {
                self.navigate_to_parent();
            }
            "open" => {
                // Payload contains the file path to open
                let path = action_payload.path.as_str();
                self.open_file(PathBuf::from(path));
            }
            "next_page" => {
                self.next_page();
            }
            "prev_page" => {
                self.prev_page();
            }
            _ => {
                debug!("Unknown file browser action: {action_str}");
            }
        }
    }
}
```

### 7.8 File Opening

Files are opened using `gio::AppInfo::launch_default_for_uri`, which invokes the system's default application for the file type:

```rust
/// Opens a file with the system's default application.
fn open_file(&self, path: PathBuf) {
    let uri = format!("file://{}", path.display());
    glib::MainContext::default().spawn_local(async move {
        match gio::AppInfo::launch_default_for_uri_future(&uri, None::<&gio::AppLaunchContext>).await {
            Ok(_) => debug!("Opened file: {uri}"),
            Err(error) => tracing::error!("Failed to open file {uri}: {error}"),
        }
    });
}
```

### 7.9 Navigation Methods

```rust
/// Navigates to a directory, reads its contents, and triggers re-render.
fn navigate_to(&self, path: PathBuf) {
    let entries = match read_directory(&path, self.config.should_show_hidden()) {
        Ok(entries) => entries,
        Err(error) => {
            tracing::error!("Failed to read directory {}: {error}", path.display());
            return;
        }
    };

    let has_parent = path.parent().is_some();

    {
        let mut state = self.state.write().expect("state lock poisoned");
        state.current_path = path.to_string_lossy().to_string();
        state.current_page = 0;
        state.entries = entries;
        state.has_parent = has_parent;
        state.total_pages = calculate_total_pages(&state.entries, self.max_entries(), has_parent);
    }

    self.broadcast_update();
}

/// Navigates to the parent of the current directory.
fn navigate_to_parent(&self) {
    let current = self.state.read().expect("state lock poisoned").current_path.clone();
    let parent = PathBuf::from(current.as_str()).parent();
    if let Some(parent_path) = parent {
        self.navigate_to(parent_path.to_path_buf());
    }
}

/// Shows the next page of entries.
fn next_page(&self) {
    {
        let mut state = self.state.write().expect("state lock poisoned");
        if state.current_page + 1 < state.total_pages {
            state.current_page += 1;
        } else {
            return;
        }
    }
    self.broadcast_update();
}

/// Shows the previous page of entries.
fn prev_page(&self) {
    {
        let mut state = self.state.write().expect("state lock poisoned");
        if state.current_page > 0 {
            state.current_page -= 1;
        } else {
            return;
        }
    }
    self.broadcast_update();
}

/// Broadcasts a WidgetUpdateMessage to trigger re-rendering on the device.
fn broadcast_update(&self) {
    let message = WidgetUpdateMessage {
        plugin_id: self.meta.id.clone(),
        instance_id: /* current instance id */,
    };
    self.broker.broadcast(TOPIC_WIDGET_UPDATE, message);
}
```

### 7.10 Longpress Actions

Longpress provides context-specific secondary actions:

| Entry Type    | Click Action                  | Longpress Action                                            |
|---------------|-------------------------------|-------------------------------------------------------------|
| **Parent**    | Navigate to parent directory  | —                                                           |
| **Directory** | Enter directory               | Open directory in system file manager (`xdg-open`)          |
| **File**      | Open with default application | Show file info (size, modified date) as a temporary overlay |
| **More**      | Next page                     | —                                                           |
| **Empty**     | —                             | —                                                           |

File info overlay: on longpress of a file, the widget temporarily renders the file size and modified date on all buttons for 3 seconds, then reverts to the
directory listing. This uses an internal timer and a `ViewMode` enum:

```rust
/// Current view mode of the file browser widget.
enum ViewMode {
    /// Normal directory listing.
    Directory,
    /// File info overlay (temporary, shown after longpress on a file).
    FileInfo,
}
```

---

## 8. Configuration TOML

### 8.1 Widget Configuration (MacroPad)

```toml
[scroll_band]
area_type = "scroll"
plugins = [
    { id = "file_browser", path = "target/release/libsmearor_file_browser_widget.so" },
]

[file_browser]
defaults = "menu_button"
# Starting directory (absolute path or ~ for home)
home_folder = "~"
# Show hidden files (default: false)
show_hidden = false
# Maximum entries per page (default: button count of the grid)
# max_entries = 15

# Optional: custom icon overrides per extension
[file_browser.icons]
png = "nf-md-file_image"
jpg = "nf-md-file_image"
mp4 = "nf-md-file_video"
mp3 = "nf-md-file_music"
pdf = "nf-md-file_pdf"
txt = "nf-md-file_document"
default = "nf-md-file"
```

### 8.2 Multi-Span Configuration

The File Browser Widget uses the full button grid as a logical unit. Unlike other Multi-Span Widgets that use `span_group` and `span_index`, the File Browser
Widget occupies all buttons in its area as a single plugin. The host allocates all visible buttons to this widget:

```toml
[scroll_band]
area_type = "scroll"
plugins = [
    { id = "file_browser", path = "target/release/libsmearor_file_browser_widget.so" },
]
```

The host recognises that the File Browser Widget is the only plugin in the area and assigns all button slots to it. The widget's `render_graphic()` is called
once per button with the button's index, and the widget determines which entry to render based on the index.

---

## 9. Implementation Phases

### Phase 1: Foundation — Model Crate (`model/file-browser`)

**Goal:** Define all shared messages, enums, and icon mappings.

**Order:**

1. Create the crate `model/file-browser` with a `Cargo.toml` that depends on `serde`, `stabby`, and the project plugin API.
2. Create `src/messages/action.rs` and declare the `FileBrowserAction` enum.
3. Create `src/messages/entry.rs` and declare the `DirectoryEntry` struct.
4. Create `src/messages/state.rs` and declare the `FileBrowserState` struct.
5. Create `src/icon.rs` and implement the `file_type_icon`, `directory_icon`, `parent_icon`, and `more_icon` functions.
6. Add `#[stabby::stabby]` to all FFI-relevant types.
7. Create `src/json_converters.rs` and implement `register_json_converters`.
8. Re-export all public types in `src/lib.rs`.
9. Run `cargo check` and `cargo test` for the model crate.

**Exit criteria:**

- The crate compiles without warnings.
- Every public struct and enum has English rustdoc documentation.
- `cargo test` passes with serialization/deserialization tests for each message type.
- The icon mapping function returns correct icon names for all supported extensions.

---

### Phase 2: Widget Crate (`plugins/file-browser`)

**Goal:** Implement the headless widget with directory reading, rendering, navigation, and file opening.

**Dependencies:** Phase 1 must be complete. `MACROPAD_ATOMIC_WIDGETS.md` Phase 1 (Input Triggers) and Phase 2 (Span Group Support) must be complete.

**Order:**

1. Create the crate `plugins/file-browser` with a `Cargo.toml` that depends on `model/file-browser`, the project plugin API, `render-utils`, `gio`, `glib`,
   `tokio`, and `tracing`.
2. Create `src/config.rs` with `FileBrowserWidgetConfig` and its `parse` method.
3. Create `src/widget.rs` with `FileBrowserWidget` and all required trait implementations.
4. Implement `read_directory()` to read and sort directory entries.
5. Implement pagination logic (`calculate_page_entries`, `calculate_total_pages`).
6. Implement `GraphicRenderer::render_graphic()` to render per-button pixel buffers.
7. Implement `MessageHandler` for `InvokeToolMessage` with actions: `enter`, `parent`, `open`, `next_page`, `prev_page`.
8. Implement `navigate_to()`, `navigate_to_parent()`, `next_page()`, `prev_page()`.
9. Implement `open_file()` using `gio::AppInfo::launch_default_for_uri`.
10. Implement `broadcast_update()` to send `WidgetUpdateMessage` on directory change.
11. Implement longpress actions: directory → open in file manager, file → file info overlay.
12. Wire `widget_plugin!` in `src/lib.rs`.
13. Add unit tests for directory reading, sorting, and pagination.

**Exit criteria:**

- The widget compiles and loads as a plugin.
- Directory reading correctly lists and sorts entries (directories first, then files, alphabetical).
- Pagination correctly splits entries across pages.
- `render_graphic()` produces correct pixel buffers for each entry type (parent, directory, file, more, empty).
- Button press actions navigate directories and open files correctly.
- `WidgetUpdateMessage` is broadcast on every directory change.
- No `unwrap`, `expect`, or `panic` remains in the new code.

---

### Phase 3: Host Integration

**Goal:** Connect the widget to the host's rendering and input pipeline.

**Dependencies:** Phase 2 must be complete.

**Order:**

1. Add the `model/file-browser` and `plugins/file-browser` crates to the workspace `Cargo.toml`.
2. Ensure the host recognises the File Browser Widget as a full-grid widget (all buttons in the area belong to this single plugin).
3. Ensure `render_buttons_to_device()` calls `render_graphic()` per button with the correct button index.
4. Ensure `InvokeToolMessage` actions from button presses are routed to the widget's `handle_message()`.
5. Add a sample configuration block in `config.toml` for the File Browser Widget.
6. Create a config example: `config-macropad-file-browser.toml`.

**Exit criteria:**

- The workspace compiles with `cargo build`.
- The File Browser Widget loads and displays the home directory contents on startup.
- Button presses navigate directories and open files.
- Pagination works when a directory has more entries than buttons.

---

### Phase 4: Validation — Integration and Tests

**Goal:** Verify end-to-end behavior and stability.

**Dependencies:** Phase 3 must be complete.

**Order:**

1. Load a headless instance with the File Browser Widget and verify the home directory is displayed.
2. Press a directory button and verify the grid updates with the directory's contents.
3. Press the "Parent" button and verify the grid returns to the parent directory.
4. Press a file button and verify the system's default application opens the file.
5. Navigate to a directory with more entries than buttons and verify pagination.
6. Press the "More" button and verify the next page is displayed.
7. Longpress a directory button and verify the system file manager opens.
8. Longpress a file button and verify the file info overlay is displayed.
9. Verify hidden files are not shown by default.
10. Enable `show_hidden = true` and verify hidden files appear.
11. Run `cargo test` for both crates.
12. Run `cargo clippy` and `cargo fmt` and fix any issues.

**Exit criteria:**

- All tests pass.
- The widget navigates directories, opens files, and paginates correctly.
- No `unwrap`, `expect`, or `panic` remains in the new code.
- `rustfmt` and `clippy` are clean.

---

### Summary of Order

```
Phase 1: model/file-browser
    |
    v
Phase 2: plugins/file-browser
    |
    v
Phase 3: host integration and config
    |
    v
Phase 4: integration and tests
```

### Rationale

- **Model first:** Message formats, enums, and icon mappings must exist before the widget can use them.
- **Widget second:** The widget is the core implementation — directory reading, rendering, navigation, and file opening.
- **Host integration third:** The host must wire up rendering and input routing for the full-grid widget.
- **Tests last:** End-to-end validation closes the loop.

---

## 10. File Changes Summary

| File                                         | Change                                               |
|----------------------------------------------|------------------------------------------------------|
| `model/file-browser/Cargo.toml`              | **New** — model crate manifest                       |
| `model/file-browser/src/lib.rs`              | **New** — re-exports                                 |
| `model/file-browser/src/json_converters.rs`  | **New** — JSON converter registration                |
| `model/file-browser/src/icon.rs`             | **New** — file type icon mapping                     |
| `model/file-browser/src/messages/mod.rs`     | **New** — message module                             |
| `model/file-browser/src/messages/action.rs`  | **New** — `FileBrowserAction` enum                   |
| `model/file-browser/src/messages/entry.rs`   | **New** — `DirectoryEntry` struct                    |
| `model/file-browser/src/messages/state.rs`   | **New** — `FileBrowserState` struct                  |
| `plugins/file-browser/Cargo.toml`            | **New** — widget crate manifest                      |
| `plugins/file-browser/src/lib.rs`            | **New** — `widget_plugin!` macro                     |
| `plugins/file-browser/src/widget.rs`         | **New** — `FileBrowserWidget` struct and trait impls |
| `plugins/file-browser/src/config.rs`         | **New** — `FileBrowserWidgetConfig` and parsing      |
| `Cargo.toml` (workspace)                     | Add `model/file-browser` and `plugins/file-browser`  |
| `config.toml`                                | Add sample File Browser Widget configuration         |
| `examples/config-macropad-file-browser.toml` | **New** — MacroPad config example                    |

---

## 11. Dependencies

### New Crates

| Crate                  | Purpose                                                                 |
|------------------------|-------------------------------------------------------------------------|
| `model/file-browser`   | Shared messages, enums, icon mapping for the File Browser Widget        |
| `plugins/file-browser` | Headless widget: directory reading, rendering, navigation, file opening |

### Per-Crate Dependencies

| Crate                  | Additional Dependencies                                                                                                                         |
|------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------|
| `model/file-browser`   | `serde`, `serde_json`, `stabby`, `smearor-swipe-launcher-plugin-api`                                                                            |
| `plugins/file-browser` | `smearor-model-file-browser`, `smearor-plugin-api`, `smearor-render-utils`, `gio`, `glib`, `tokio`, `tracing`, `image`, `ab_glyph`, `imageproc` |

No new external dependencies — all required crates (`gio`, `glib`, `image`, `ab_glyph`, `imageproc`, `tokio`, `tracing`) are already in the workspace.

---

## 12. Technical Notes

- **No service crate:** The File Browser Widget is self-contained. It reads the filesystem directly using `std::fs::read_dir` and opens files via
  `gio::AppInfo::launch_default_for_uri`. There is no background process, no periodic polling, and no inter-service communication. The widget's state is
  entirely internal (current directory, current page, entries).

- **Headless-only:** The widget implements `GraphicRenderer` for MacroPad pixel buffers. It does not implement `WidgetBuilder` for GTK — the File Browser is a
  MacroPad-exclusive feature. On GTK instances, the user has a full file manager available.

- **Full-grid widget:** Unlike standard Multi-Span Widgets that use `span_group` and `span_index`, the File Browser Widget is a single plugin that occupies all
  buttons in its area. The host assigns all button slots to this widget and calls `render_graphic()` per button with the button's index. The widget determines
  which entry to render based on the index.

- **File opening:** Uses `gio::AppInfo::launch_default_for_uri` (GLib async API) to open files with the system's default application. This is the same mechanism
  used by GNOME file managers and respects the user's MIME type associations. The call is spawned via `glib::MainContext::default().spawn_local` to avoid
  blocking the widget's message handler.

- **Directory sorting:** Entries are sorted with directories first (alphabetical), then files (alphabetical). This matches the convention of most file managers
  and makes navigation predictable.

- **Pagination:** When a directory has more entries than available buttons, the last button slot becomes a "More" button. Pressing "More" advances to the next
  page. The "Parent" button is always first (when not at filesystem root), reducing the available slots by one. The "More" button (when present) reduces the
  available slots by one more.

- **Hidden files:** Dotfiles (entries starting with `.`) are hidden by default. The `show_hidden` config field controls this behaviour. This matches the
  convention of most file managers.

- **Tilde expansion:** The `home_folder` config field accepts `~` as a shorthand for the user's home directory. The `resolved_home_folder()` method expands this
  using the `HOME` environment variable.

- **Error handling:** Directory read errors are logged via `tracing::error!` and do not crash the widget. The widget remains on the current directory and the
  user can navigate elsewhere. File opening errors are similarly logged without crashing.

- **Security:** The widget only reads directories and opens files — it does not modify, delete, or create files. File opening uses the system's default
  application handler, which runs in the user's session with the user's permissions. The widget can navigate above the home folder via "Parent", but this is the
  same as opening a file manager. No additional restrictions are imposed.

- **FFI string types:** All `String` and `Option<String>` fields in `#[stabby::stabby]` structs use `stabby::string::String` and
  `stabby::option::Option<stabby::string::String>` respectively, to maintain ABI stability across compiler invocations. This is consistent with the existing
  pattern in other model crates.

- **File info overlay:** Longpress on a file temporarily shows the file's size and modified date on all buttons for 3 seconds. This is implemented via an
  internal `ViewMode` enum and a `glib::MainContext::spawn_local` timer that reverts to `Directory` mode after the timeout. No polling loop is used — the timer
  is a single async delay.

---

## 13. Risks and Considerations

1. **Button count limitation:** On small MacroPad grids (e.g. 3×2 = 6 buttons), the File Browser can only show 5 entries per page (one slot reserved for
   "Parent"). This is functional but may require frequent pagination. Mitigation: the widget is designed for grids with at least 8–15 buttons.

2. **Filename truncation:** MacroPad button displays are small (72×72 px). Filenames longer than 8 characters are truncated with ellipsis. This may make it
   difficult to distinguish files with similar names. Mitigation: longpress shows the full filename in the file info overlay.

3. **Large directories:** Directories with hundreds or thousands of entries (e.g. `/usr/bin`) will require many pages. Mitigation: the widget paginates
   efficiently and does not load all entries into memory at once — it reads the full listing once, sorts it, and stores it in state. For extremely large
   directories, a future optimisation could lazy-load pages.

4. **Symlinks:** The widget follows symlinks for directory entry (entering a symlinked directory navigates to the target). This matches the behaviour of most
   file managers. Symlink loops are protected by the filesystem's own loop detection.

5. **File opening latency:** `gio::AppInfo::launch_default_for_uri` is asynchronous, but the application launch itself may take time. The widget does not block
   during file opening — the call is spawned on the GLib main context and the widget remains responsive.

6. **No write access:** The widget is read-only — it cannot rename, move, copy, delete, or create files. This is a deliberate safety measure. File management
   operations are left to the system file manager (opened via longpress on a directory).

7. **Concurrent access:** The widget's state is protected by `Arc<RwLock<FileBrowserState>>`. Directory reads are synchronous (`std::fs::read_dir`) and happen
   in the message handler thread. This is acceptable because directory reads are fast (typically < 10 ms for a few hundred entries) and do not block the
   MacroPad input loop significantly.

---

## 14. Compliance with `AGENTS.md`

The proposed implementation follows the project guidelines in `AGENTS.md`:

- **Crate separation:** The feature is split into `model/file-browser` and `plugins/file-browser`. No service crate is needed.
- **One struct per file:** Each message struct and each enum lives in its own file.
- **Widget traits:** The widget implements `MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`, `AsRef<Option<FfiCoreContext>>`, and `GraphicRenderer`.
- **GTK updates:** File opening uses `glib::MainContext::spawn_local` for async operation. The file info overlay timer uses `glib::MainContext::spawn_local`.
- **Event-driven:** The widget is updated by incoming `InvokeToolMessage` actions, not by polling loops. Directory reads happen only on navigation.
- **FFI stability:** All FFI-relevant types in the model carry `#[stabby::stabby]`. String fields use `stabby::string::String`.
- **No panic:** The implementation uses `Result` and `Option` for error handling; no `unwrap()`, `expect()`, or `panic!`.
- **Naming:** All names are descriptive and follow Rust naming conventions (`snake_case` for functions and variables, `PascalCase` for types).
- **Documentation:** All public structs, enums, and fields are documented in English.
- **Formatting:** Code is formatted with `rustfmt` and checked with `clippy`.
- **Dependencies:** The model uses `serde` and `stabby`; the widget uses `gio`, `glib`, `render-utils`, `tokio`, and `tracing`.
- **Rust Edition 2024:** The crate uses the latest edition features.
- **Import organization:** Imports are individual, alphabetically ordered, with `crate::` first, then external crates, then `std::`.

---

*End of document.*
