use smearor_personalization_model::TimeFormat;
use smearor_swipe_launcher_plugin_api::Locale;

/// Personalization override data for the power widget.
///
/// Stores time format and locale received from the personalization service.
/// When available, these values override the default 24h formatting and
/// English labels.
#[derive(Clone, Debug, Default)]
pub struct PersonalizationOverride {
    /// Preferred time format (12h or 24h).
    pub time_format: Option<TimeFormat>,
    /// Locale for label translations.
    pub locale: Locale,
}

impl PersonalizationOverride {
    /// Returns the effective time format, falling back to 24h.
    pub fn effective_time_format(&self) -> TimeFormat {
        self.time_format.as_ref().cloned().unwrap_or_default()
    }

    /// Formats a countdown timer based on the effective time format.
    ///
    /// `Hour24` -> `HH:MM:SS` countdown display
    /// `Hour12` -> `h:MM:SS AM/PM` countdown display (if countdown spans midnight)
    pub fn format_countdown(&self, remaining_seconds: u64) -> String {
        let hours = remaining_seconds / 3600;
        let minutes = (remaining_seconds % 3600) / 60;
        let seconds = remaining_seconds % 60;
        match self.effective_time_format() {
            TimeFormat::Hour24 => format!("{:02}:{:02}:{:02}", hours, minutes, seconds),
            TimeFormat::Hour12 => {
                if hours == 0 {
                    format!("12:{:02}:{:02} AM", minutes, seconds)
                } else if hours < 12 {
                    format!("{}:{:02}:{:02} AM", hours, minutes, seconds)
                } else if hours == 12 {
                    format!("12:{:02}:{:02} PM", minutes, seconds)
                } else {
                    format!("{}:{:02}:{:02} PM", hours - 12, minutes, seconds)
                }
            }
        }
    }
}
