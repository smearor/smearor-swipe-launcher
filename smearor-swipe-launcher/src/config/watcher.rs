use async_channel::Receiver;
use async_channel::unbounded;
use dashmap::DashMap;
use notify::Config;
use notify::Event;
use notify::EventKind;
use notify::RecommendedWatcher;
use notify::RecursiveMode;
use notify::Watcher;
use notify::event::AccessKind;
use notify::event::ModifyKind;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tracing::debug;
use tracing::error;
use tracing::warn;

/// Debounce duration for file change events.
///
/// Text editors often perform multiple writes in quick succession
/// (e.g. atomic save via temp file + rename). This delay coalesces
/// bursts of events into a single reload.
const DEBOUNCE_DURATION: Duration = Duration::from_millis(500);

/// A request to reload a launcher instance from its config file.
///
/// Sent by the watcher when a config file (or one of its include files)
/// has been modified. The receiver should call
/// `LauncherHost::reload_instance` on the GTK main context.
pub struct ConfigReloadRequest {
    /// The instance ID to reload.
    pub instance_id: String,
    /// The absolute path to the main launcher config file.
    pub config_path: PathBuf,
}

/// Watches launcher configuration files for changes and emits reload requests.
///
/// Maintains a mapping from config file paths (and include file paths) to
/// instance IDs. When a watched file changes, the corresponding instance
/// is scheduled for hot-reload via a debounced channel.
///
/// # Example
///
/// ```no_run
/// use smearor_swipe_launcher::config::watcher::ConfigWatcher;
///
/// let watcher = ConfigWatcher::new();
/// watcher.add_config(std::path::Path::new("config.toml"), "main", &[]);
/// let reload_rx = watcher.start();
/// // Drain `reload_rx` on the GTK main context and call `reload_instance`.
/// ```
#[derive(Clone)]
pub struct ConfigWatcher {
    /// Maps any watched file path (main config or include) to instance ID.
    path_to_instance: DashMap<PathBuf, String>,
    /// Maps instance ID to the main config path (used for reload).
    instance_to_config: DashMap<String, PathBuf>,
    /// Maps watched file path to its last known content hash.
    file_hashes: DashMap<PathBuf, u64>,
    /// Flag to stop the debounce task.
    cancel_flag: Arc<AtomicBool>,
}

impl ConfigWatcher {
    /// Creates a new, empty `ConfigWatcher`.
    pub fn new() -> Self {
        Self {
            path_to_instance: DashMap::new(),
            instance_to_config: DashMap::new(),
            file_hashes: DashMap::new(),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Register a launcher config file and its include files for watching.
    ///
    /// Both the main config path and any include paths are canonicalized
    /// before being added to the watch set. If a path does not exist yet,
    /// the non-canonical form is used.
    pub fn add_config(&self, config_path: &Path, instance_id: &str, include_paths: &[PathBuf]) {
        let canonical = std::fs::canonicalize(config_path).unwrap_or_else(|_| config_path.to_path_buf());
        self.store_hash(&canonical);
        self.path_to_instance.insert(canonical.clone(), instance_id.to_string());
        self.instance_to_config.insert(instance_id.to_string(), canonical);

        for include in include_paths {
            let canonical_include = std::fs::canonicalize(include).unwrap_or_else(|_| include.clone());
            self.store_hash(&canonical_include);
            self.path_to_instance.insert(canonical_include, instance_id.to_string());
        }
    }

    /// Removes all watch entries for a specific instance.
    ///
    /// Called during `stop_instance()` to stop watching the instance's
    /// config and include files. This prevents reload requests for
    /// non-existent instances.
    pub fn remove_instance(&self, instance_id: &str) {
        // Remove the main config mapping.
        if let Some((_, config_path)) = self.instance_to_config.remove(instance_id) {
            self.path_to_instance.remove(&config_path);
            self.file_hashes.remove(&config_path);
            debug!("ConfigWatcher: removed config watch for instance '{}' ({})", instance_id, config_path.display());
        }

        // Remove any remaining path-to-instance entries (include files).
        let paths_to_remove: Vec<PathBuf> = self
            .path_to_instance
            .iter()
            .filter(|entry| entry.value() == instance_id)
            .map(|entry| entry.key().clone())
            .collect();
        for path in paths_to_remove {
            self.path_to_instance.remove(&path);
            self.file_hashes.remove(&path);
            debug!("ConfigWatcher: removed include watch '{}' for instance '{}'", path.display(), instance_id);
        }
    }

    /// Returns the config path for a given instance ID, if registered.
    pub fn get_config_path(&self, instance_id: &str) -> Option<PathBuf> {
        self.instance_to_config.get(instance_id).map(|entry| entry.value().clone())
    }

    /// Compute and store the content hash for a file path.
    fn store_hash(&self, path: &Path) {
        if let Ok(content) = std::fs::read(path) {
            let mut hasher = DefaultHasher::new();
            hasher.write(&content);
            self.file_hashes.insert(path.to_path_buf(), hasher.finish());
        }
    }

    /// Start watching all registered config files.
    ///
    /// Returns an `async_channel::Receiver` that yields `ConfigReloadRequest`s.
    /// The caller should drain this receiver on the GTK main context and
    /// call `LauncherHost::reload_instance` for each request.
    ///
    /// The watcher runs in a background tokio task. File change events are
    /// debounced: if multiple events arrive within `DEBOUNCE_DURATION`,
    /// only a single reload request is emitted.
    pub fn start(&self) -> Receiver<ConfigReloadRequest> {
        let (reload_tx, reload_rx) = unbounded::<ConfigReloadRequest>();
        let (event_tx, event_rx) = unbounded::<PathBuf>();

        if self.path_to_instance.is_empty() {
            debug!("ConfigWatcher has no files to watch, skipping watcher startup");
            return reload_rx;
        }

        let event_tx_clone = event_tx.clone();
        let mut watcher = match RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| match result {
                Ok(event) => {
                    let is_content_change = matches!(
                        event.kind,
                        EventKind::Modify(ModifyKind::Data(_))
                            | EventKind::Modify(ModifyKind::Any)
                            | EventKind::Modify(ModifyKind::Name(_))
                            | EventKind::Create(_)
                            | EventKind::Access(AccessKind::Close(_))
                    );
                    if !is_content_change {
                        return;
                    }
                    debug!("ConfigWatcher: raw event: kind={:?} paths={:?}", event.kind, event.paths);
                    for path in &event.paths {
                        let _ = event_tx_clone.try_send(path.clone());
                    }
                }
                Err(e) => {
                    warn!("ConfigWatcher: watcher error: {}", e);
                }
            },
            Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to create config file watcher: {}", e);
                return reload_rx;
            }
        };

        // Watch each config file directly. This catches in-place modifications
        // but not atomic saves (temp file + rename), which replace the inode.
        for entry in self.path_to_instance.iter() {
            let path = entry.key();
            if let Err(e) = watcher.watch(path, RecursiveMode::NonRecursive) {
                warn!("Failed to watch config file {}: {}", path.display(), e);
            } else {
                debug!("Watching config file: {}", path.display());
            }
        }

        // Also watch parent directories to catch atomic-save events (IN_MOVED_TO).
        // Modern editors write to a temp file then rename it over the original,
        // replacing the inode. The file-level watch becomes stale, but the
        // directory-level watch still fires for the rename.
        let mut watched_dirs = std::collections::HashSet::new();
        for entry in self.path_to_instance.iter() {
            if let Some(parent) = entry.key().parent() {
                if watched_dirs.insert(parent.to_path_buf()) {
                    if let Err(e) = watcher.watch(parent, RecursiveMode::NonRecursive) {
                        warn!("Failed to watch config directory {}: {}", parent.display(), e);
                    } else {
                        debug!("Watching config directory: {}", parent.display());
                    }
                }
            }
        }

        let path_to_instance = self.path_to_instance.clone();
        let instance_to_config = self.instance_to_config.clone();
        let file_hashes = self.file_hashes.clone();
        let cancel_flag = Arc::clone(&self.cancel_flag);
        let watcher_count = path_to_instance.len();

        tokio::spawn(async move {
            let _watcher = watcher;
            let mut pending: Option<(String, PathBuf)> = None;

            debug!("ConfigWatcher debouncer started, watching {} file(s)", watcher_count);

            loop {
                if pending.is_some() {
                    tokio::select! {
                        _ = async {
                            while !cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                tokio::time::sleep(Duration::from_millis(50)).await;
                            }
                        } => {
                            debug!("ConfigWatcher debouncer cancelled, stopping");
                            break;
                        }
                        Ok(path) = event_rx.recv() => {
                            debug!("ConfigWatcher: debouncer received event for path: {}", path.display());
                            if let Some(instance_id) = path_to_instance.get(&path) {
                                let instance_id = instance_id.clone();
                                if let Some(config_path) = instance_to_config.get(&instance_id) {
                                    let config_path = config_path.clone();
                                    if has_content_changed(&file_hashes, &path) {
                                        pending = Some((instance_id, config_path));
                                    }
                                } else {
                                    debug!("ConfigWatcher: no config_path for instance '{}'", instance_id);
                                }
                            } else {
                                debug!("ConfigWatcher: path {} not in watch map, ignoring", path.display());
                            }
                        }
                        _ = tokio::time::sleep(DEBOUNCE_DURATION) => {
                            if let Some((instance_id, config_path)) = pending.take() {
                                debug!(
                                    "Config file changed, sending reload request for instance '{}' (config: {})",
                                    instance_id,
                                    config_path.display()
                                );
                                let _ = reload_tx.try_send(ConfigReloadRequest {
                                    instance_id,
                                    config_path,
                                });
                            }
                        }
                    }
                } else {
                    tokio::select! {
                        _ = async {
                            while !cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                tokio::time::sleep(Duration::from_millis(50)).await;
                            }
                        } => {
                            debug!("ConfigWatcher debouncer cancelled, stopping");
                            break;
                        }
                        recv_result = event_rx.recv() => {
                            match recv_result {
                                Ok(path) => {
                                    debug!("ConfigWatcher: debouncer received event for path: {}", path.display());
                                    if let Some(instance_id) = path_to_instance.get(&path) {
                                        let instance_id = instance_id.clone();
                                        if let Some(config_path) = instance_to_config.get(&instance_id) {
                                            let config_path = config_path.clone();
                                            if has_content_changed(&file_hashes, &path) {
                                                pending = Some((instance_id, config_path));
                                            }
                                        } else {
                                            debug!("ConfigWatcher: no config_path for instance '{}'", instance_id);
                                        }
                                    } else {
                                        debug!("ConfigWatcher: path {} not in watch map, ignoring", path.display());
                                    }
                                }
                                Err(_) => {
                                    debug!("ConfigWatcher event channel closed, stopping debouncer");
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            debug!("ConfigWatcher debouncer stopped");
        });

        reload_rx
    }

    /// Stops all watchers and cancels the debounce task.
    ///
    /// The `notify::Watcher` is dropped when the tokio task exits,
    /// automatically unregistering all kernel-level watches.
    pub fn shutdown(&self) {
        debug!("ConfigWatcher shutting down");
        self.cancel_flag.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Default for ConfigWatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a file's content has changed compared to the stored hash.
/// Updates the stored hash when the content has changed.
/// Returns `true` if the content differs (or the file is new/unreadable), `false` if unchanged.
fn has_content_changed(file_hashes: &DashMap<PathBuf, u64>, path: &Path) -> bool {
    let Ok(content) = std::fs::read(path) else {
        debug!("ConfigWatcher: could not read {}, treating as changed", path.display());
        return true;
    };
    let mut hasher = DefaultHasher::new();
    hasher.write(&content);
    let new_hash = hasher.finish();
    match file_hashes.get(path) {
        Some(stored) if *stored == new_hash => false,
        Some(stored) => {
            debug!("DEBUG: has_content_changed: {} CHANGED (stored={}, new={})", path.display(), *stored, new_hash);
            file_hashes.insert(path.to_path_buf(), new_hash);
            true
        }
        None => {
            debug!("DEBUG: has_content_changed: {} NEW (new={})", path.display(), new_hash);
            file_hashes.insert(path.to_path_buf(), new_hash);
            true
        }
    }
}
