use std::env;
use std::fs;
use std::path::PathBuf;
use tracing::error;
use tracing::trace;

/// Ensures the Hyprland socket can be found by the `hyprland` crate.
///
/// The crate reads `HYPRLAND_INSTANCE_SIGNATURE` to build the socket path.
/// If the variable is missing, this function tries to find a single Hyprland
/// instance in `/tmp/hypr` and sets the variable accordingly.
pub(crate) fn ensure_hyprland_instance_signature() {
    if let Ok(instance_signature) = env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        trace!("Found HYPRLAND_INSTANCE_SIGNATURE: '{instance_signature}'");
        return;
    }

    let runtime_dir = env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/run/user/1000"));

    let hypr_dir = runtime_dir.join("hypr");

    let entries = match fs::read_dir(&hypr_dir) {
        Ok(entries) => entries,
        Err(e) => {
            error!("Could not read Hyprland runtime directory '{:?}': {}", hypr_dir, e);
            return;
        }
    };

    let mut signatures: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_dir() {
                if let Ok(name) = entry.file_name().into_string() {
                    signatures.push(name);
                }
            }
        }
    }

    if signatures.len() > 1 {
        error!("Multiple HYPRLAND_INSTANCE_SIGNATUREs found in {:?}: {:?}", hypr_dir, signatures);
    }

    match signatures.first() {
        None => {
            error!("No HYPRLAND_INSTANCE_SIGNATURE found in {:?}", hypr_dir);
        }
        Some(signature) => {
            error!("HYPRLAND_INSTANCE_SIGNATURE not set, using detected signature: {signature}");
            unsafe {
                env::set_var("HYPRLAND_INSTANCE_SIGNATURE", signature);
            }
        }
    }
}
