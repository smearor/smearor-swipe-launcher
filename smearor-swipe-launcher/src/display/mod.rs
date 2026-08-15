mod area_size;
mod calculation;
mod monitor;

pub use area_size::AreaSize;
pub use area_size::DEFAULT_HEIGHT;
pub use calculation::calculate_area_size_for_monitor;
pub use monitor::resolve_monitor;
pub use monitor::validate_monitor_index;
