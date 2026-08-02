use smearor_personalization_model::MeasurementSystem;
use smearor_personalization_model::TemperatureUnit;
use smearor_swipe_launcher_plugin_api::Locale;

/// Personalization override data for sysinfo widgets.
///
/// Stores unit preferences and locale received from the personalization service.
/// When available, these values override the default metric formatting.
#[derive(Clone, Debug, Default)]
pub struct PersonalizationOverride {
    /// Preferred temperature unit (Celsius or Fahrenheit).
    pub temperature_unit: Option<TemperatureUnit>,
    /// Preferred measurement system (metric or imperial).
    pub measurement_system: Option<MeasurementSystem>,
    /// Locale for label translations.
    pub locale: Locale,
}

impl PersonalizationOverride {
    /// Returns the effective temperature unit, falling back to Celsius.
    pub fn effective_temperature_unit(&self) -> TemperatureUnit {
        self.temperature_unit.clone().unwrap_or_default()
    }

    /// Returns the effective measurement system, falling back to metric.
    pub fn effective_measurement_system(&self) -> MeasurementSystem {
        self.measurement_system.clone().unwrap_or_default()
    }

    /// Formats a temperature value in Celsius using the effective unit.
    /// Input is always in Celsius (as delivered by the sysinfo service).
    pub fn format_temperature(&self, celsius: f32) -> String {
        let c = celsius as f64;
        match self.effective_temperature_unit() {
            TemperatureUnit::Celsius => format!("{:.0}\u{b0}C", c),
            TemperatureUnit::Fahrenheit => format!("{:.0}\u{b0}F", c * 9.0 / 5.0 + 32.0),
        }
    }

    /// Formats a temperature value with no unit suffix, using the effective unit.
    /// Input is always in Celsius (as delivered by the sysinfo service).
    pub fn format_temperature_value(&self, celsius: f32) -> String {
        let c = celsius as f64;
        match self.effective_temperature_unit() {
            TemperatureUnit::Celsius => format!("{:.0}", c),
            TemperatureUnit::Fahrenheit => format!("{:.0}", c * 9.0 / 5.0 + 32.0),
        }
    }

    /// Formats bytes as a human-readable string using the effective measurement system.
    pub fn format_bytes(&self, bytes: u64) -> String {
        match self.effective_measurement_system() {
            MeasurementSystem::Metric => {
                let units = ["B", "KB", "MB", "GB", "TB"];
                let mut value = bytes as f64;
                let mut unit_index = 0;
                while value >= 1000.0 && unit_index < units.len() - 1 {
                    value /= 1000.0;
                    unit_index += 1;
                }
                format!("{:.1} {}", value, units[unit_index])
            }
            MeasurementSystem::Imperial => {
                let units = ["B", "KiB", "MiB", "GiB", "TiB"];
                let mut value = bytes as f64;
                let mut unit_index = 0;
                while value >= 1024.0 && unit_index < units.len() - 1 {
                    value /= 1024.0;
                    unit_index += 1;
                }
                format!("{:.1} {}", value, units[unit_index])
            }
        }
    }

    /// Formats a data rate (bytes per second) using the effective measurement system.
    pub fn format_data_rate(&self, bytes_per_second: u64) -> String {
        format!("{}/s", self.format_bytes(bytes_per_second))
    }
}
