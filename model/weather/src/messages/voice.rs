use crate::AirQualityData;
use crate::AirQualityLevel;
use crate::CurrentWeather;
use crate::DailyForecast;
use crate::model::cloud_cover_level::CloudCoverLevel;
use crate::model::humidity_level::HumidityLevel;
use crate::model::particulate_matter_level::ParticulateMatterLevel;
use crate::model::precipitation_amount_level::PrecipitationAmountLevel;
use crate::model::precipitation_intensity::PrecipitationIntensity;
use crate::model::precipitation_probability_level::PrecipitationProbabilityLevel;
use crate::model::pressure_level::PressureLevel;
use crate::model::sunshine_level::SunshineLevel;
use crate::model::temperature_level::TemperatureLevel;
use crate::model::uv_index_level::UvIndexLevel;
use crate::model::wind_direction::WindDirection;
use crate::model::wind_speed_level::WindSpeedLevel;

/// Trait for weather data that can produce a German voice-assistant summary.
pub trait VoiceDescribable {
    /// Returns a human-readable German summary suitable for text-to-speech.
    fn voice_summary(&self) -> String;
}

impl CurrentWeather {
    /// Returns the wind direction as a compass enum, if available.
    pub fn wind_direction(&self) -> Option<WindDirection> {
        self.wind_direction.as_ref().copied().map(WindDirection::from)
    }

    /// Returns the temperature perception level, if available.
    pub fn temperature_level(&self) -> Option<TemperatureLevel> {
        self.temperature.as_ref().copied().map(TemperatureLevel::from)
    }

    /// Returns the cloud cover level, if available.
    pub fn cloud_cover_level(&self) -> Option<CloudCoverLevel> {
        self.cloud_cover.as_ref().copied().map(CloudCoverLevel::from)
    }

    /// Returns the humidity perception level, if available.
    pub fn humidity_level(&self) -> Option<HumidityLevel> {
        self.relative_humidity.as_ref().copied().map(HumidityLevel::from)
    }

    /// Returns the pressure level, if available.
    pub fn pressure_level(&self) -> Option<PressureLevel> {
        self.pressure.as_ref().copied().map(PressureLevel::from)
    }

    /// Returns the UV index level, if available.
    pub fn uv_index_level(&self) -> Option<UvIndexLevel> {
        self.uv_index.as_ref().copied().map(UvIndexLevel::from)
    }

    /// Returns the wind speed level, if available.
    pub fn wind_speed_level(&self) -> Option<WindSpeedLevel> {
        self.wind_speed.as_ref().copied().map(WindSpeedLevel::from)
    }

    /// Returns the precipitation intensity, if available.
    pub fn precipitation_intensity(&self) -> Option<PrecipitationIntensity> {
        self.precipitation.as_ref().copied().map(PrecipitationIntensity::from)
    }
}

impl VoiceDescribable for CurrentWeather {
    fn voice_summary(&self) -> String {
        let temp = self.temperature.as_ref().copied().unwrap_or(0.0);
        let clouds = self.cloud_cover.as_ref().copied().unwrap_or(0.0);
        let wind_speed = self.wind_speed.as_ref().copied().unwrap_or(0.0);
        let wind_dir = self.wind_direction.as_ref().copied().unwrap_or(0.0);
        let is_day = self.is_day.as_ref().copied().unwrap_or(true);
        let tag_nacht = if is_day { "Tag" } else { "Nacht" };

        format!(
            "Es ist {tag_nacht}. Die Temperatur beträgt {:.0} Grad bei {}. Der Wind weht als {} aus {}.",
            temp,
            CloudCoverLevel::from(clouds),
            WindSpeedLevel::from(wind_speed),
            WindDirection::from(wind_dir),
        )
    }
}

impl DailyForecast {
    /// Returns the sunshine level, if both sunshine and daylight durations are available.
    pub fn sunshine_level(&self) -> Option<SunshineLevel> {
        let sunshine = self.sunshine_duration.as_ref().copied()?;
        let daylight = self.daylight_duration.as_ref().copied()?;
        Some(SunshineLevel::from_durations(sunshine, daylight))
    }

    pub fn sunshine_hours(&self) -> Option<f32> {
        self.sunshine_duration.as_ref().copied().map(|s| s / 3600.0)
    }

    /// Returns the precipitation probability level, if available.
    pub fn precipitation_probability_level(&self) -> Option<PrecipitationProbabilityLevel> {
        self.precipitation_probability_max.as_ref().copied().map(PrecipitationProbabilityLevel::from)
    }

    /// Returns the precipitation amount level, if available.
    pub fn precipitation_amount_level(&self) -> Option<PrecipitationAmountLevel> {
        self.precipitation_sum.as_ref().copied().map(PrecipitationAmountLevel::from)
    }
}

impl VoiceDescribable for DailyForecast {
    fn voice_summary(&self) -> String {
        let temp_max = self.temperature_max.as_ref().copied().unwrap_or(0.0);
        let temp_min = self.temperature_min.as_ref().copied().unwrap_or(0.0);

        let mut parts = vec![format!("Höchstwert {:.0} Grad, Tiefstwert {:.0} Grad", temp_max, temp_min)];

        if let Some(sunshine) = self.sunshine_level() {
            parts.push(sunshine.to_string());
        }

        if let Some(prob) = self.precipitation_probability_level() {
            parts.push(prob.to_string());
        }

        parts.join(". ")
    }
}

impl AirQualityData {
    /// Returns the air quality level, if available.
    pub fn air_quality_level(&self) -> Option<AirQualityLevel> {
        self.european_aqi.as_ref().copied().map(AirQualityLevel::from)
    }

    /// Returns the PM2.5 particulate matter level, if available.
    pub fn particulate_matter_level(&self) -> Option<ParticulateMatterLevel> {
        self.pm2_5.as_ref().copied().map(ParticulateMatterLevel::from)
    }
}

impl VoiceDescribable for AirQualityData {
    fn voice_summary(&self) -> String {
        let aqi = self.european_aqi.as_ref().copied().unwrap_or(0.0);
        let pm2_5 = self.pm2_5.as_ref().copied().unwrap_or(0.0);
        format!(
            "Die Luftqualität ist aktuell {}. Feinstaub liegt bei {}.",
            AirQualityLevel::from(aqi),
            ParticulateMatterLevel::from(pm2_5),
        )
    }
}
