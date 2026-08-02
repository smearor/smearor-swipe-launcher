use smearor_personalization_model::DateFormat;
use smearor_personalization_model::TimeFormat;
use smearor_swipe_launcher_plugin_api::Locale;

/// Personalization override data for the button widget.
///
/// Stores time format, date format, and locale received from the
/// personalization service. When available, these values can be used
/// to format dynamic date/time values embedded in `main_text` or `info_text`.
#[derive(Clone, Debug, Default)]
pub struct PersonalizationOverride {
    /// Preferred time format (12h or 24h).
    pub time_format: Option<TimeFormat>,
    /// Preferred date format.
    pub date_format: Option<DateFormat>,
    /// Locale for label translations.
    pub locale: Locale,
}

impl PersonalizationOverride {
    /// Returns the effective time format, falling back to 24h.
    pub fn effective_time_format(&self) -> TimeFormat {
        self.time_format.as_ref().cloned().unwrap_or_default()
    }

    /// Returns the effective date format, falling back to DMY.
    pub fn effective_date_format(&self) -> DateFormat {
        self.date_format.as_ref().cloned().unwrap_or_default()
    }

    /// Formats a time from hour and minute components according to the effective time format.
    ///
    /// `Hour24` -> `HH:MM`
    /// `Hour12` -> `h:MM AM/PM`
    pub fn format_time(&self, hour: u32, minute: u32) -> String {
        match self.effective_time_format() {
            TimeFormat::Hour24 => format!("{:02}:{:02}", hour, minute),
            TimeFormat::Hour12 => {
                let (h, suffix) = match hour {
                    0 => (12, "AM"),
                    1..=11 => (hour, "AM"),
                    12 => (12, "PM"),
                    13..=23 => (hour - 12, "PM"),
                    _ => (hour, "AM"),
                };
                format!("{}:{:02} {}", h, minute, suffix)
            }
        }
    }

    /// Formats a date from day, month, year components according to the effective date format.
    ///
    /// `Dmy` -> `DD.MM.YYYY`
    /// `Mdy` -> `MM/DD/YYYY`
    /// `Ymd` -> `YYYY-MM-DD`
    pub fn format_date(&self, day: u32, month: u32, year: i32) -> String {
        match self.effective_date_format() {
            DateFormat::Dmy => format!("{:02}.{:02}.{}", day, month, year),
            DateFormat::Mdy => format!("{:02}/{:02}/{:04}", month, day, year),
            DateFormat::Ymd => format!("{:04}-{:02}-{:02}", year, month, day),
        }
    }
}
