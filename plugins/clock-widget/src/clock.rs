use crate::config::ClockConfig;
use crate::config::DEFAULT_FORMAT;
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

    pub(crate) fn get_current_time(&self) -> String {
        let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
        let format_desc = parse(&self.config.format).unwrap_or_else(|_| parse(DEFAULT_FORMAT).unwrap());
        now.format(&format_desc).unwrap_or_else(|_| "Invalid Format".to_string())
    }
}
