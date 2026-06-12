use thiserror::Error;

/// Errors that can occur during configuration validation
#[derive(Error, Debug)]
pub enum ConfigValidationError {
    #[error("Area '{area_id}' listed in areas but not found in configuration")]
    AreaNotFound { area_id: String },

    #[error("Area '{area_id}' configuration is invalid: {reason}")]
    InvalidAreaConfig { area_id: String, reason: String },

    #[error("No areas defined in configuration")]
    NoAreasDefined,

    #[error("Duplicate area ID '{area_id}' found in areas list")]
    DuplicateAreaId { area_id: String },

    #[error("Plugin ID '{plugin_id}' is duplicated across areas")]
    DuplicatePluginId { plugin_id: String },

    #[error("Invalid area type '{area_type}' for area '{area_id}'")]
    InvalidAreaType { area_id: String, area_type: String },

    #[error("Fixed area '{area_id}' must have either width or width_percent specified")]
    MissingWidthSpec { area_id: String },

    #[error("Invalid width_percent {percent} for area '{area_id}': must be between 0.0 and 1.0")]
    InvalidWidthPercent { area_id: String, percent: f32 },

    #[error("Invalid transition '{transition}' for area '{area_id}'")]
    InvalidTransition { area_id: String, transition: String },

    #[error("Layout trigger validation failed: {reason}")]
    InvalidLayoutTrigger { reason: String },
}
