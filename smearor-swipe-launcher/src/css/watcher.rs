use dashmap::DashMap;
use gtk4::CssProvider;
use gtk4::gdk::Display;
use gtk4::gio::File;
use notify::Config;
use notify::Event;
use notify::EventKind;
use notify::RecommendedWatcher;
use notify::RecursiveMode;
use notify::Watcher;
use notify::event::AccessKind;
use notify::event::ModifyKind;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tracing::debug;
use tracing::warn;

/// Debounce duration for CSS file change events.
///
/// Text editors often perform atomic saves (temp file + rename), producing
/// rapid bursts of events. This delay coalesces them into a single reload.
const DEBOUNCE_DURATION: Duration = Duration::from_millis(500);

/// Priority for per-instance CSS providers (overrides global user CSS).
const INSTANCE_CSS_PRIORITY: u32 = gtk4::STYLE_PROVIDER_PRIORITY_USER + 1;

/// Priority for the global user CSS provider.
const GLOBAL_CSS_PRIORITY: u32 = gtk4::STYLE_PROVIDER_PRIORITY_USER;

/// A CSS file to watch, with its associated priority and provider.
#[derive(Clone)]
struct WatchedCss {
    /// GTK priority level for this provider.
    priority: u32,
    /// The registered `CssProvider`, kept alive so it can be removed later.
    provider: CssProvider,
}

/// Watches CSS files for changes and hot-reloads them on the GTK main thread.
///
/// Handles both the global user CSS (`~/.config/smearor/style.css`) and
/// per-instance CSS files (`{config_stem}.css`). Uses `notify` for file
/// system events, debounces changes (500ms), and dispatches all GTK
/// operations via `glib::MainContext::default().spawn_local()` to ensure
/// thread safety.
///
/// # Atomic Saves
///
/// Many editors write to a temp file and rename it over the original,
/// replacing the inode. The watcher handles this by checking `path.exists()`
/// after the debounce interval — if the file still exists (already re-created
/// by the atomic swap), it is treated as a modification. Only if the file is
/// genuinely gone is the provider removed.
///
/// # inotify Fallback
///
/// On Linux, `notify` cannot watch a non-existent file. If a CSS file does
/// not exist at startup, the parent directory is watched instead. When the
/// file is created, it is loaded immediately and the watcher switches to
/// direct file watching. Directory watches are deduplicated across all
/// CSS files sharing the same parent.
#[derive(Clone)]
pub struct CssWatcher {
    /// Maps CSS file path to its watched entry (provider + priority).
    watched: DashMap<PathBuf, WatchedCss>,
    /// Maps directory path to the set of expected CSS filenames within it.
    /// Used for the inotify fallback when a CSS file doesn't exist yet.
    directory_watches: DashMap<PathBuf, HashSet<PathBuf>>,
    /// Flag to stop the debounce task.
    cancel_flag: Arc<AtomicBool>,
}

impl CssWatcher {
    /// Creates a new `CssWatcher` with no files registered.
    pub fn new() -> Self {
        Self {
            watched: DashMap::new(),
            directory_watches: DashMap::new(),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Registers and loads the global user CSS file (`~/.config/smearor/style.css`)
    /// if it exists, and starts watching it for changes.
    ///
    /// If the file does not exist, the parent directory is watched for creation.
    pub fn watch_global_css(&self) {
        let Some(config_dir) = dirs::config_dir() else {
            return;
        };
        let global_css_path = config_dir.join("smearor").join("style.css");
        self.watch_css_file(&global_css_path, GLOBAL_CSS_PRIORITY);
    }

    /// Registers and loads a per-instance CSS file if it exists, and starts
    /// watching it for changes.
    ///
    /// The CSS path is resolved from the TOML config path by replacing the
    /// `.toml` extension with `.css`. If the CSS file does not exist, the
    /// parent directory is watched for creation.
    pub fn watch_instance_css(&self, config_path: &Path) {
        let Some(css_path) = resolve_instance_css_path(config_path) else {
            return;
        };
        self.watch_css_file(&css_path, INSTANCE_CSS_PRIORITY);
    }

    /// Removes a per-instance CSS provider and stops watching its file.
    ///
    /// Called during `stop_instance()` to clean up CSS resources. The CSS
    /// path is resolved from the TOML config path. If the CSS file was
    /// watched directly, its provider is removed from the display. If it
    /// was tracked via a directory watch (file didn't exist), the entry
    /// is removed from the directory-watch map.
    pub fn remove_instance_css(&self, config_path: &Path) {
        let Some(css_path) = resolve_instance_css_path(config_path) else {
            return;
        };
        let canonical = std::fs::canonicalize(&css_path).unwrap_or_else(|_| css_path.to_path_buf());

        // Remove from direct watch map and remove provider from display.
        if self.watched.contains_key(&canonical) {
            self.remove_css_provider(&canonical);
            debug!("CssWatcher: removed instance CSS for {}", canonical.display());
        }

        // Remove from directory-watch map (inotify fallback for non-existent files).
        if let Some(parent) = canonical.parent() {
            if let Some(mut entry) = self.directory_watches.get_mut(parent) {
                entry.remove(&canonical);
                if entry.is_empty() {
                    drop(entry);
                    self.directory_watches.remove(parent);
                }
                debug!("CssWatcher: removed instance CSS directory watch for {}", canonical.display());
            }
        }
    }

    /// Registers a CSS file for watching and loads it if it exists.
    ///
    /// If the file does not exist, the parent directory is watched instead
    /// (inotify fallback). When the file is created, it will be loaded
    /// automatically.
    fn watch_css_file(&self, css_path: &Path, priority: u32) {
        let canonical = std::fs::canonicalize(css_path).unwrap_or_else(|_| css_path.to_path_buf());

        if canonical.exists() {
            if let Some(display) = Display::default() {
                let provider = CssProvider::new();
                provider.load_from_file(&File::for_path(&canonical));
                gtk4::style_context_add_provider_for_display(&display, &provider, priority);
                debug!("Loaded CSS from {} (priority {})", canonical.display(), priority);

                self.watched.insert(canonical.clone(), WatchedCss { priority, provider });
            }
        } else {
            debug!("CSS file {} does not exist, watching parent directory", canonical.display());
            self.register_directory_watch(&canonical);
        }
    }

    /// Registers a directory watch for a CSS file that doesn't exist yet.
    ///
    /// Maintains a `directory -> Set<css_paths>` map for deduplication.
    fn register_directory_watch(&self, css_path: &Path) {
        let Some(parent) = css_path.parent() else {
            return;
        };
        self.directory_watches.entry(parent.to_path_buf()).or_default().insert(css_path.to_path_buf());
    }

    /// Starts the file watcher background task.
    ///
    /// This spawns a tokio task that watches all registered CSS files and
    /// directories. On file change, the CSS is reloaded on the GTK main
    /// thread via `glib::MainContext::default().spawn_local()`.
    pub fn start(&self) {
        if self.watched.is_empty() && self.directory_watches.is_empty() {
            debug!("CssWatcher has no files to watch, skipping watcher startup");
            return;
        }

        let cancel_flag = Arc::clone(&self.cancel_flag);

        // Channel for raw file system events (notify -> debouncer).
        let (event_tx, event_rx) = async_channel::unbounded::<PathBuf>();

        // Channel for debounced events (debouncer -> GTK main thread).
        let (reload_tx, reload_rx) = async_channel::unbounded::<PathBuf>();

        let event_tx_clone = event_tx.clone();
        let mut watcher = match RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| match result {
                Ok(event) => {
                    let is_relevant = matches!(
                        event.kind,
                        EventKind::Modify(ModifyKind::Data(_))
                            | EventKind::Modify(ModifyKind::Any)
                            | EventKind::Modify(ModifyKind::Name(_))
                            | EventKind::Create(_)
                            | EventKind::Remove(_)
                            | EventKind::Access(AccessKind::Close(_))
                    );
                    if !is_relevant {
                        return;
                    }
                    debug!("CssWatcher: raw event: kind={:?} paths={:?}", event.kind, event.paths);
                    for path in &event.paths {
                        let _ = event_tx_clone.try_send(path.clone());
                    }
                }
                Err(e) => {
                    warn!("CssWatcher: watcher error: {}", e);
                }
            },
            Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                warn!("Failed to create CSS file watcher: {}", e);
                return;
            }
        };

        // Watch existing CSS files directly.
        for entry in self.watched.iter() {
            let path = entry.key();
            if let Err(e) = watcher.watch(path, RecursiveMode::NonRecursive) {
                warn!("Failed to watch CSS file {}: {}", path.display(), e);
            } else {
                debug!("Watching CSS file: {}", path.display());
            }
        }

        // Watch parent directories for CSS files that don't exist yet.
        // Also watch parent directories of existing files to catch atomic saves.
        let mut watched_dirs = HashSet::new();
        for entry in self.watched.iter() {
            if let Some(parent) = entry.key().parent() {
                if watched_dirs.insert(parent.to_path_buf()) {
                    if let Err(e) = watcher.watch(parent, RecursiveMode::NonRecursive) {
                        warn!("Failed to watch CSS directory {}: {}", parent.display(), e);
                    } else {
                        debug!("Watching CSS directory (for existing file): {}", parent.display());
                    }
                }
            }
        }

        for entry in self.directory_watches.iter() {
            let dir = entry.key();
            if let Err(e) = watcher.watch(dir, RecursiveMode::NonRecursive) {
                warn!("Failed to watch CSS directory {}: {}", dir.display(), e);
            } else {
                debug!("Watching CSS directory (for pending file): {}", dir.display());
            }
        }

        // Debouncer task: runs on tokio, only handles paths (no GTK types).
        tokio::spawn(async move {
            let _watcher = watcher;
            let mut pending: Option<PathBuf> = None;

            debug!("CssWatcher debouncer started");

            loop {
                tokio::select! {
                    _ = async {
                        while !cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                    } => {
                        debug!("CssWatcher debouncer cancelled, stopping");
                        break;
                    }
                    Ok(path) = event_rx.recv() => {
                        debug!("CssWatcher: received event for path: {}", path.display());
                        pending = Some(path);
                    }
                    _ = tokio::time::sleep(DEBOUNCE_DURATION), if pending.is_some() => {
                        if let Some(path) = pending.take() {
                            let _ = reload_tx.try_send(path);
                        }
                    }
                }
            }

            debug!("CssWatcher debouncer stopped");
        });

        // GTK main thread handler: receives debounced paths and reloads CSS.
        let main_watcher = self.clone();
        let main_context = gtk4::glib::MainContext::default();
        main_context.spawn_local(async move {
            debug!("CssWatcher main-thread handler started");
            while let Ok(path) = reload_rx.recv().await {
                main_watcher.handle_css_event(&path);
            }
            debug!("CssWatcher main-thread handler stopped");
        });
    }

    /// Stops all watchers and cleans up resources.
    ///
    /// This cancels the debounce task. The `notify::Watcher` is dropped
    /// when the tokio task exits, automatically unregistering all
    /// kernel-level watches.
    pub fn shutdown(&self) {
        debug!("CssWatcher shutting down");
        self.cancel_flag.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Handles a debounced CSS file event.
    ///
    /// Determines whether the event represents a modification, creation, or
    /// deletion, and dispatches the appropriate GTK operation on the main thread.
    fn handle_css_event(&self, path: &Path) {
        // Check if this is a known CSS file (direct watch).
        let is_watched = self.watched.contains_key(path);

        // Check if this path matches a pending CSS file in a directory watch.
        let is_pending = self.directory_watches.iter().any(|entry| entry.value().contains(path));

        if !is_watched && !is_pending {
            // Check if the path is in a watched directory (could be the CSS file
            // with a different canonical path due to atomic save).
            if let Some(parent) = path.parent() {
                if let Some(entry) = self.directory_watches.get(parent) {
                    if entry.iter().any(|expected| expected.file_name() == path.file_name()) {
                        drop(entry);
                        if path.exists() {
                            debug!("CssWatcher: CSS file created at {}", path.display());
                            load_css_on_main_thread(path, INSTANCE_CSS_PRIORITY);
                        }
                        return;
                    }
                }
            }
            debug!("CssWatcher: path {} not in watch map, ignoring", path.display());
            return;
        }

        if path.exists() {
            // File exists — reload (handles both modification and atomic save).
            debug!("CssWatcher: reloading CSS from {}", path.display());
            let priority = self.get_css_priority(path);
            self.reload_css(path.to_path_buf(), priority);
        } else {
            // File is genuinely gone — remove provider.
            debug!("CssWatcher: CSS file {} removed, removing provider", path.display());
            self.remove_css_provider(path);
        }
    }

    /// Gets the CSS priority for a watched file.
    fn get_css_priority(&self, path: &Path) -> u32 {
        self.watched.get(path).map(|entry| entry.priority).unwrap_or(INSTANCE_CSS_PRIORITY)
    }

    /// Reloads a CSS file on the GTK main thread.
    ///
    /// Removes the old provider and registers a new one with the same priority.
    fn reload_css(&self, path: PathBuf, priority: u32) {
        let watched = self.watched.clone();
        let main_context = gtk4::glib::MainContext::default();
        main_context.spawn_local(async move {
            let Some(display) = Display::default() else {
                return;
            };

            // Remove old provider.
            let old_provider = watched.remove(&path).map(|(_, w)| w.provider);
            if let Some(ref old) = old_provider {
                gtk4::style_context_remove_provider_for_display(&display, old);
            }

            // Load and register new provider.
            let provider = CssProvider::new();
            provider.load_from_file(&File::for_path(&path));
            gtk4::style_context_add_provider_for_display(&display, &provider, priority);
            debug!("CssWatcher: reloaded CSS from {} (priority {})", path.display(), priority);

            watched.insert(path.clone(), WatchedCss { priority, provider });
        });
    }

    /// Removes a CSS provider from the display on the GTK main thread.
    fn remove_css_provider(&self, path: &Path) {
        let path = path.to_path_buf();
        let watched = self.watched.clone();
        let main_context = gtk4::glib::MainContext::default();
        main_context.spawn_local(async move {
            let provider = watched.remove(&path).map(|(_, w)| w.provider);
            if let Some(provider) = provider {
                if let Some(display) = Display::default() {
                    gtk4::style_context_remove_provider_for_display(&display, &provider);
                    debug!("CssWatcher: removed CSS provider for {}", path.display());
                }
            }
        });
    }
}

impl Default for CssWatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Loads a CSS file on the GTK main thread (for newly created files).
fn load_css_on_main_thread(path: &Path, priority: u32) {
    let path = path.to_path_buf();
    let main_context = gtk4::glib::MainContext::default();
    main_context.spawn_local(async move {
        let Some(display) = Display::default() else {
            return;
        };
        let provider = CssProvider::new();
        provider.load_from_file(&File::for_path(&path));
        gtk4::style_context_add_provider_for_display(&display, &provider, priority);
        debug!("CssWatcher: loaded newly created CSS from {} (priority {})", path.display(), priority);
    });
}

/// Resolves the CSS file path for a given TOML config path by replacing
/// the `.toml` extension with `.css`. Returns `None` if the input path
/// has no `.toml` extension.
fn resolve_instance_css_path(config_path: &Path) -> Option<PathBuf> {
    let extension = config_path.extension()?;
    if extension != "toml" {
        return None;
    }
    let stem = config_path.file_stem()?;
    let parent = config_path.parent().unwrap_or_else(|| Path::new(""));
    Some(parent.join(format!("{}.css", stem.to_string_lossy())))
}
