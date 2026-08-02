use crate::labels::NotificationLabel;
use smearor_personalization_model::DateFormat;
use smearor_personalization_model::TimeFormat;
use smearor_swipe_launcher_plugin_api::Locale;

/// Personalization override data for the notifications widget.
///
/// Stores time format, date format, and locale received from the personalization service.
/// When available, these determine timestamp formatting and label translations.
#[derive(Clone, Debug, Default)]
pub struct PersonalizationOverride {
    /// Preferred time format (12h/24h).
    pub time_format: TimeFormat,
    /// Preferred date format.
    pub date_format: DateFormat,
    /// Locale for label translations.
    pub locale: Locale,
}

impl PersonalizationOverride {
    /// Returns the effective locale, falling back to default (English).
    pub fn effective_locale(&self) -> Locale {
        self.locale
    }

    /// Formats a Unix epoch millisecond timestamp according to the effective time and date formats.
    ///
    /// Produces strings like `DD.MM.YYYY HH:MM`, `MM/DD/YYYY h:MM AM/PM`, or `YYYY-MM-DD HH:MM`.
    pub fn format_timestamp(&self, timestamp_millis: u64) -> String {
        let secs = timestamp_millis / 1000;
        let mins = (secs / 60) % 60;
        let hours = (secs / 3600) % 24;
        let day_of_month = (secs / 86400) as i64;
        let (year, month, day) = epoch_to_ymd(day_of_month);

        let time_str = match self.time_format {
            TimeFormat::Hour24 => format!("{:02}:{:02}", hours, mins),
            TimeFormat::Hour12 => {
                let h12 = if hours == 0 {
                    12
                } else if hours > 12 {
                    hours - 12
                } else {
                    hours
                };
                let ampm = if hours >= 12 { "PM" } else { "AM" };
                format!("{}:{:02} {}", h12, mins, ampm)
            }
        };

        let date_str = match self.date_format {
            DateFormat::Dmy => format!("{:02}.{:02}.{}", day, month, year),
            DateFormat::Mdy => format!("{:02}/{:02}/{}", month, day, year),
            DateFormat::Ymd => format!("{}-{:02}-{:02}", year, month, day),
        };

        format!("{} {}", date_str, time_str)
    }

    /// Formats a relative time string (e.g. "5 min ago") based on the locale.
    ///
    /// Falls back to English relative time strings.
    pub fn format_relative_time(&self, timestamp_millis: u64, now_millis: u64) -> String {
        let locale = self.effective_locale();
        let diff_secs = now_millis.saturating_sub(timestamp_millis) / 1000;

        if diff_secs < 60 {
            return NotificationLabel::JustNow.localized_label(locale).to_string();
        }
        let diff_mins = diff_secs / 60;
        if diff_mins < 60 {
            return format!("{} {}", diff_mins, NotificationLabel::MinutesAgo.localized_label(locale));
        }
        let diff_hours = diff_mins / 60;
        if diff_hours < 24 {
            return format!("{} {}", diff_hours, NotificationLabel::HoursAgo.localized_label(locale));
        }
        let diff_days = diff_hours / 24;
        format!("{} {}", diff_days, NotificationLabel::DaysAgo.localized_label(locale))
    }
}

/// Converts days since Unix epoch (1970-01-01) to (year, month, day).
///
/// Uses the proleptic Gregorian calendar algorithm.
fn epoch_to_ymd(days_since_epoch: i64) -> (i64, u32, u32) {
    let days = days_since_epoch + 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy as u32 - (153 * mp as u32 + 2) / 5 + 1;
    let m = if mp < 10 { mp as u32 + 3 } else { mp as u32 - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}
