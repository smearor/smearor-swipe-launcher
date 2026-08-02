use crate::config::ClockConfig;
use crate::localized_weekday::LocalizedWeekday;
use serde_json::json;
use smearor_personalization_model::DateFormat;
use smearor_personalization_model::TimeFormat;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::WidgetMode;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// Personalization data that overrides config values when available.
#[derive(Debug, Clone, Default)]
pub(crate) struct PersonalizationOverride {
    /// IANA timezone identifier (e.g. "Europe/Berlin") from personalization service.
    pub(crate) timezone: Option<String>,
    /// Locale from personalization service.
    pub(crate) locale: Locale,
    /// Time format (12h/24h) from personalization service.
    pub(crate) time_format: Option<TimeFormat>,
    /// Date format from personalization service.
    pub(crate) date_format: Option<DateFormat>,
}

#[derive(Debug)]
pub(crate) struct Clock {
    pub(crate) config: ClockConfig,
    pub(crate) personalization: std::sync::RwLock<PersonalizationOverride>,
}

impl Clock {
    pub(crate) fn new(config: ClockConfig) -> Self {
        Self {
            config,
            personalization: std::sync::RwLock::new(PersonalizationOverride::default()),
        }
    }

    /// Updates the personalization override data.
    pub(crate) fn update_personalization(&self, override_data: PersonalizationOverride) {
        if let Ok(mut guard) = self.personalization.write() {
            *guard = override_data;
        }
    }

    /// Returns the effective timezone string: personalization override or config fallback.
    fn effective_timezone(&self) -> Option<String> {
        if let Ok(guard) = self.personalization.read()
            && let Some(ref tz) = guard.timezone
        {
            return Some(tz.clone());
        }
        self.config.timezone.clone()
    }

    /// Returns the effective locale: personalization override or config fallback.
    fn effective_locale(&self) -> Locale {
        if let Ok(guard) = self.personalization.read() {
            return guard.locale;
        }
        Locale::Unknown
    }

    /// Returns the effective time format: personalization override or default (24h).
    fn effective_time_format(&self) -> TimeFormat {
        if let Ok(guard) = self.personalization.read()
            && let Some(tf) = guard.time_format.as_ref()
        {
            return tf.clone();
        }
        TimeFormat::Hour24
    }

    /// Returns the effective date format: personalization override or default (Dmy).
    fn effective_date_format(&self) -> DateFormat {
        if let Ok(guard) = self.personalization.read()
            && let Some(df) = guard.date_format.as_ref()
        {
            return df.clone();
        }
        DateFormat::Dmy
    }

    /// Returns the current time as a formatted string.
    /// Respects TimeFormat from personalization (12h/24h).
    /// Compact mode: HH:MM, Wide mode: HH:MM:SS.
    pub(crate) fn get_time_string(&self) -> String {
        let now = self.get_timezone();
        let time_format = self.effective_time_format();
        match time_format {
            TimeFormat::Hour12 => {
                let (hour12, am_pm) = to_12h(now.hour());
                match self.config.mode {
                    WidgetMode::Compact => format!("{}:{:02} {}", hour12, now.minute(), am_pm),
                    WidgetMode::Wide => format!("{}:{:02}:{:02} {}", hour12, now.minute(), now.second(), am_pm),
                }
            }
            TimeFormat::Hour24 => match self.config.mode {
                WidgetMode::Compact => format!("{:02}:{:02}", now.hour(), now.minute()),
                WidgetMode::Wide => format!("{:02}:{:02}:{:02}", now.hour(), now.minute(), now.second()),
            },
        }
    }

    /// Returns the current date formatted according to DateFormat from personalization.
    pub(crate) fn get_date_string(&self) -> String {
        let now = self.get_timezone();
        let date_format = self.effective_date_format();
        match date_format {
            DateFormat::Mdy => format!("{:02}/{:02}/{}", now.month() as u8, now.day(), now.year()),
            DateFormat::Ymd => format!("{}-{:02}-{:02}", now.year(), now.month() as u8, now.day()),
            DateFormat::Dmy => format!("{:02}.{:02}.{}", now.day(), now.month() as u8, now.year()),
        }
    }

    /// Returns the date split into (day_month, year) parts for multi-line rendering.
    pub(crate) fn get_date_parts(&self) -> (String, String) {
        let now = self.get_timezone();
        let date_format = self.effective_date_format();
        match date_format {
            DateFormat::Mdy => (format!("{:02}/{:02}", now.month() as u8, now.day()), format!("{}", now.year())),
            DateFormat::Ymd => (format!("{:02}-{:02}", now.month() as u8, now.day()), format!("{}", now.year())),
            DateFormat::Dmy => (format!("{:02}.{:02}.", now.day(), now.month() as u8), format!("{}", now.year())),
        }
    }

    /// Returns the current weekday as a `LocalizedWeekday`.
    pub(crate) fn get_weekday_localized(&self) -> LocalizedWeekday {
        let now = self.get_timezone();
        LocalizedWeekday::from_time_weekday(now.weekday())
    }

    /// Returns the weekday name in the language determined by the effective locale.
    pub(crate) fn get_weekday_name(&self) -> &'static str {
        self.get_weekday_localized().localized(self.effective_locale())
    }

    /// Returns a structured JSON object with rich time information for LLM consumption.
    /// Includes ISO timestamp, timezone, DST status, day of week, time of day context,
    /// and workday status.
    pub(crate) fn get_time_info_json(&self) -> Option<String> {
        let now = self.get_timezone();
        let timestamp_iso = now.format(&Rfc3339).unwrap_or_else(|_| "unknown".to_string());
        let date = self.get_date_string();
        let time = format!("{:02}:{:02}", now.hour(), now.minute());
        let timezone_name = self.effective_timezone().unwrap_or_else(|| "local".to_string());
        let utc_offset_seconds = now.offset().whole_seconds();
        let is_summer_time = utc_offset_seconds > 3600;
        let timezone_label = match utc_offset_seconds {
            0 => "UTC",
            3600 => "CET",
            7200 => "CEST",
            -28800 => "PST",
            -25200 => "PDT",
            -18000 => "EST",
            -14400 => "EDT",
            _ => "local",
        };
        let weekday = LocalizedWeekday::from_time_weekday(now.weekday());
        let day_of_week = weekday.english();
        let hour = now.hour();
        let time_of_day_context = match hour {
            0..=5 => "late_night",
            6..=8 => "early_morning",
            9..=11 => "morning",
            12..=13 => "noon",
            14..=17 => "afternoon",
            18..=21 => "evening",
            _ => "late_night",
        };
        let workday_status = match now.weekday() {
            time::Weekday::Saturday | time::Weekday::Sunday => "weekend",
            _ => "mid_week",
        };
        let payload = json!({
            "timestamp_iso": timestamp_iso,
            "date": date,
            "time": time,
            "timezone": timezone_name,
            "timezone_label": timezone_label,
            "is_summer_time": is_summer_time,
            "day_of_week": day_of_week,
            "time_of_day_context": time_of_day_context,
            "workday_status": workday_status
        });
        Some(payload.to_string())
    }

    pub(crate) fn get_timezone(&self) -> OffsetDateTime {
        let Some(timezone) = self.effective_timezone() else {
            return OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
        };
        match timezone.to_lowercase().as_str() {
            "utc" => OffsetDateTime::now_utc(),
            "local" => OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc()),
            // IANA timezone names are not directly supported by the `time` crate.
            // Fall back to local time for now.
            _ => OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc()),
        }
    }
}

/// Converts a 24-hour value (0–23) to 12-hour format.
/// Returns (hour12, "AM"/"PM").
fn to_12h(hour: u8) -> (u8, &'static str) {
    match hour {
        0 => (12, "AM"),
        1..=11 => (hour, "AM"),
        12 => (12, "PM"),
        13..=23 => (hour - 12, "PM"),
        _ => (12, "AM"),
    }
}
