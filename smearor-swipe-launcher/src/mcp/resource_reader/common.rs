use crate::area::instance_area_manager::InstanceAreaManager;
use crate::host::LauncherHost;

/// Helper to access the first available area manager.
pub(super) fn with_first_area_manager<F, T>(host: &LauncherHost, callback: F) -> Result<T, String>
where
    F: FnOnce(&InstanceAreaManager) -> Result<T, String>,
{
    let instances = host.instances.lock().map_err(|_| "Failed to lock instances")?;
    let first_instance = instances.values().next().ok_or("No launcher instance available")?;
    let area_manager = first_instance.area_manager.lock().map_err(|_| "Failed to lock area manager")?;
    callback(&area_manager)
}
