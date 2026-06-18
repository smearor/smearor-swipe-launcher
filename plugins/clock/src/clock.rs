use crate::config::ClockConfig;
use crate::config::DEFAULT_FORMAT;
use crate::config::DEFAULT_FORMAT_2;
use time::OffsetDateTime;
use time::format_description::parse;

#[derive(Debug)]
pub(crate) struct Clock {
    pub(crate) config: ClockConfig,
}

impl Clock {
    pub(crate) fn new(config: ClockConfig) -> Self {
        Self { config }
    }

    pub(crate) fn get_current_time_1(&self) -> String {
        Self::format_current_time(&self.config.format, DEFAULT_FORMAT, self.get_timezone())
    }

    pub(crate) fn get_current_time_2(&self) -> Option<String> {
        let Some(format_2) = self.config.format_2.clone() else {
            return None;
        };
        Some(Self::format_current_time(&format_2, DEFAULT_FORMAT_2, self.get_timezone()))
    }

    pub(crate) fn get_timezone(&self) -> OffsetDateTime {
        let Some(timezone) = self.config.timezone.clone() else {
            return OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
        };
        match timezone.to_lowercase().as_str() {
            "utc" => OffsetDateTime::now_utc(),
            "local" => OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc()),
            // TODO: parse timezone string
            _ => OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc()),
        }
    }

    fn format_current_time(format: &str, default_format: &str, timezone: OffsetDateTime) -> String {
        let format_desc = parse(format).unwrap_or_else(|_| parse(default_format).unwrap());
        timezone.format(&format_desc).unwrap_or_else(|_| "Invalid Format".to_string())
    }
}
