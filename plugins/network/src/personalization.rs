use smearor_personalization_model::MeasurementSystem;
use smearor_swipe_launcher_plugin_api::Locale;

/// Personalization override data for the network widget.
///
/// Stores measurement system and locale received from the personalization service.
/// When available, measurement system determines bandwidth formatting (metric vs imperial)
/// and locale determines label translations.
#[derive(Clone, Debug, Default)]
pub struct PersonalizationOverride {
    /// Preferred measurement system (metric or imperial).
    pub measurement_system: Option<MeasurementSystem>,
    /// Locale for label translations.
    pub locale: Locale,
}

impl PersonalizationOverride {
    /// Returns the effective measurement system, falling back to metric.
    pub fn effective_measurement_system(&self) -> MeasurementSystem {
        self.measurement_system.clone().unwrap_or_default()
    }

    /// Returns the effective locale, falling back to default (English).
    pub fn effective_locale(&self) -> Locale {
        self.locale
    }

    /// Formats a bandwidth value in bytes per second using the effective measurement system.
    ///
    /// Metric uses decimal prefixes (KB, MB, GB).
    /// Imperial uses binary prefixes (KiB, MiB, GiB).
    pub fn format_bandwidth(&self, bytes_per_second: u64) -> String {
        let bps = bytes_per_second as f64;
        match self.effective_measurement_system() {
            MeasurementSystem::Metric => {
                if bps >= 1_073_741_824.0 {
                    format!("{:.1} GB/s", bps / 1_073_741_824.0)
                } else if bps >= 1_048_576.0 {
                    format!("{:.1} MB/s", bps / 1_048_576.0)
                } else if bps >= 1024.0 {
                    format!("{:.1} KB/s", bps / 1024.0)
                } else {
                    format!("{} B/s", bytes_per_second)
                }
            }
            MeasurementSystem::Imperial => {
                if bps >= 1_073_741_824.0 {
                    format!("{:.1} GiB/s", bps / 1_073_741_824.0)
                } else if bps >= 1_048_576.0 {
                    format!("{:.1} MiB/s", bps / 1_048_576.0)
                } else if bps >= 1024.0 {
                    format!("{:.1} KiB/s", bps / 1024.0)
                } else {
                    format!("{} B/s", bytes_per_second)
                }
            }
        }
    }
}
