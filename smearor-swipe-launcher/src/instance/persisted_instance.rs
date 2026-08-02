use tracing::error;

/// A persisted instance entry in the state file.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct PersistedInstance {
    pub instance_id: String,
    pub config_path: String,
    pub instance_type: String,
}

/// Returns the path to the instances state file.
pub fn get_instances_state_path() -> std::path::PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    config_dir.join("smearor").join("instances.toml")
}

/// Read the instances state file.
pub fn read_instances_state(path: &std::path::Path) -> Vec<PersistedInstance> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    #[derive(serde::Deserialize)]
    struct StateFile {
        #[serde(default)]
        instances: Vec<PersistedInstance>,
    }
    toml::from_str::<StateFile>(&content).map(|s| s.instances).unwrap_or_default()
}

/// Write the instances state file atomically (write to temp, then rename).
pub fn write_instances_state(path: &std::path::Path, entries: &[PersistedInstance]) {
    #[derive(serde::Serialize)]
    struct StateFile<'a> {
        instances: &'a [PersistedInstance],
    }
    let state = StateFile { instances: entries };
    let content = match toml::to_string_pretty(&state) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to serialize instances state: {}", e);
            return;
        }
    };
    let header = "# Persisted dynamic launcher instances.\n# Automatically managed by the launcher — do not edit manually.\n\n";
    let full_content = format!("{}{}", header, content);

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp_path = path.with_extension("toml.tmp");
    if std::fs::write(&tmp_path, &full_content).is_err() {
        error!("Failed to write instances state temp file");
        return;
    }
    if std::fs::rename(&tmp_path, path).is_err() {
        error!("Failed to rename instances state file");
        let _ = std::fs::remove_file(&tmp_path);
    }
}
