use smearor_personalization_model::MeasurementSystem;
use smearor_personalization_model::TemperatureUnit;
use smearor_personalization_model::TimeFormat;
use smearor_personalization_model::WindSpeedUnit;
use smearor_swipe_launcher_plugin_api::Locale;

/// Personalization override data for the weather widget.
///
/// Stores unit preferences and locale received from the personalization service.
/// When available, these values override the default metric formatting.
#[derive(Clone, Debug, Default)]
pub struct PersonalizationOverride {
    /// Preferred temperature unit (Celsius or Fahrenheit).
    pub temperature_unit: Option<TemperatureUnit>,
    /// Preferred wind speed unit (km/h, mph, or m/s).
    pub wind_speed_unit: Option<WindSpeedUnit>,
    /// Preferred measurement system (metric or imperial).
    pub measurement_system: Option<MeasurementSystem>,
    /// Preferred time format (12h or 24h).
    pub time_format: Option<TimeFormat>,
    /// Locale for label translations.
    pub locale: Locale,
}

impl PersonalizationOverride {
    /// Returns the effective temperature unit, falling back to Celsius.
    pub fn effective_temperature_unit(&self) -> TemperatureUnit {
        self.temperature_unit.clone().unwrap_or_default()
    }

    /// Returns the effective wind speed unit, falling back to km/h.
    pub fn effective_wind_speed_unit(&self) -> WindSpeedUnit {
        self.wind_speed_unit.clone().unwrap_or_default()
    }

    /// Returns the effective measurement system, falling back to metric.
    pub fn effective_measurement_system(&self) -> MeasurementSystem {
        self.measurement_system.clone().unwrap_or_default()
    }

    /// Returns the effective time format, falling back to 24h.
    pub fn effective_time_format(&self) -> TimeFormat {
        self.time_format.as_ref().cloned().unwrap_or_default()
    }

    /// Formats a temperature value in Celsius using the effective unit.
    /// Input is always in Celsius (as delivered by the API).
    pub fn format_temperature(&self, celsius: f32) -> String {
        self.format_temperature_with_unit(celsius)
    }

    /// Formats a temperature value with unit suffix for forecast views.
    /// Input is always in Celsius (as delivered by the API).
    pub fn format_temperature_with_unit(&self, celsius: f32) -> String {
        let c = celsius as f64;
        match self.effective_temperature_unit() {
            TemperatureUnit::Celsius => format!("{:.0}\u{b0}C", c),
            TemperatureUnit::Fahrenheit => format!("{:.0}\u{b0}F", c * 9.0 / 5.0 + 32.0),
        }
    }

    /// Formats a wind speed value in km/h using the effective unit.
    /// Input is always in km/h (as delivered by the API).
    pub fn format_wind_speed(&self, kmh: f32) -> String {
        let k = kmh as f64;
        match self.effective_wind_speed_unit() {
            WindSpeedUnit::Kmh => format!("{:.0} km/h", k),
            WindSpeedUnit::Mph => format!("{:.0} mph", k * 0.621371),
            WindSpeedUnit::Ms => format!("{:.0} m/s", k / 3.6),
        }
    }

    /// Formats a precipitation value in mm using the effective measurement system.
    /// Input is always in mm (as delivered by the API).
    pub fn format_precipitation(&self, mm: f32) -> String {
        let m = mm as f64;
        match self.effective_measurement_system() {
            MeasurementSystem::Metric => format!("{:.1} mm", m),
            MeasurementSystem::Imperial => format!("{:.2} in", m / 25.4),
        }
    }

    /// Formats a pressure value in hPa using the effective measurement system.
    /// Input is always in hPa (as delivered by the API).
    pub fn format_pressure(&self, hpa: f32) -> String {
        let h = hpa as f64;
        match self.effective_measurement_system() {
            MeasurementSystem::Metric => format!("{:.0} hPa", h),
            MeasurementSystem::Imperial => format!("{:.1} inHg", h / 33.8639),
        }
    }

    /// Formats a time string (ISO format from API) using the effective time format.
    /// Expects input in the form "YYYY-MM-DDTHH:MM" or "HH:MM".
    /// Falls back to the input string if parsing fails.
    pub fn format_time(&self, iso_time: &str) -> String {
        let time_part = iso_time.split('T').last().unwrap_or(iso_time);
        let hhmm = time_part.get(..5).unwrap_or("");
        let parts: Vec<&str> = hhmm.split(':').collect();
        if parts.len() < 2 {
            return iso_time.to_string();
        }
        let hour: u8 = match parts[0].parse() {
            Ok(h) => h,
            Err(_) => return iso_time.to_string(),
        };
        let minute: u8 = match parts[1].parse() {
            Ok(m) => m,
            Err(_) => return iso_time.to_string(),
        };
        match self.effective_time_format() {
            TimeFormat::Hour12 => {
                let hour12 = if hour == 0 {
                    12
                } else if hour > 12 {
                    hour - 12
                } else {
                    hour
                };
                let am_pm = if hour >= 12 { "PM" } else { "AM" };
                format!("{}:{:02} {}", hour12, minute, am_pm)
            }
            TimeFormat::Hour24 => format!("{:02}:{:02}", hour, minute),
        }
    }
}
