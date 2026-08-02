use smearor_personalization_model::TimeFormat;
use smearor_swipe_launcher_plugin_api::Locale;

/// Personalization override data for the MPRIS widget.
///
/// Stores locale and time format received from the personalization service.
/// When available, locale determines label translations and time format
/// determines progress timestamp formatting.
#[derive(Clone, Debug, Default)]
pub struct PersonalizationOverride {
    /// Locale for label translations.
    pub locale: Locale,
    /// Time format for progress timestamps.
    pub time_format: TimeFormat,
}

impl PersonalizationOverride {
    /// Returns the effective locale, falling back to default (English).
    pub fn effective_locale(&self) -> Locale {
        self.locale
    }

    /// Formats a duration in microseconds as a time string.
    ///
    /// `Hour24` → `MM:SS` or `HH:MM:SS`
    /// `Hour12` → `M:SS` or `H:MM:SS`
    pub fn format_duration(&self, micros: i64) -> String {
        if micros <= 0 {
            return "0:00".to_string();
        }
        let total_seconds = micros / 1_000_000;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        match self.time_format {
            TimeFormat::Hour24 => {
                if hours > 0 {
                    format!("{}:{:02}:{:02}", hours, minutes, seconds)
                } else {
                    format!("{:02}:{:02}", minutes, seconds)
                }
            }
            TimeFormat::Hour12 => {
                if hours > 0 {
                    format!("{}:{:02}:{:02}", hours, minutes, seconds)
                } else {
                    format!("{}:{:02}", minutes, seconds)
                }
            }
        }
    }
}
