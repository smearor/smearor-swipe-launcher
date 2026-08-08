/// Default width for the launcher area in pixels.
pub const DEFAULT_WIDTH: i32 = 800;

/// Default height for the launcher area in pixels.
pub const DEFAULT_HEIGHT: i32 = 100;

/// The rendered size of a launcher area on a monitor.
///
/// Computed from the monitor geometry and rotation, or falling back to
/// `DEFAULT_WIDTH` × `DEFAULT_HEIGHT` when no monitor is available.
#[derive(Debug, Clone, Copy)]
pub struct AreaSize {
    /// Width in pixels.
    pub width: i32,
    /// Height in pixels.
    pub height: i32,
}

impl AreaSize {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
}

impl Default for AreaSize {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
        }
    }
}
