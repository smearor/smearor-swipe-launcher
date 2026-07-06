use openmeteo_rs::AirQualityHourlyVar;
use openmeteo_rs::Client;
use openmeteo_rs::CurrentVar;
use openmeteo_rs::DailyVar;
use openmeteo_rs::ForecastResponse;
use openmeteo_rs::TimeFormat;
use openmeteo_rs::Timezone;
use openmeteo_rs::WindSpeedUnit;
use smearor_weather_model::AirQualityData;
use smearor_weather_model::CurrentWeather;
use smearor_weather_model::DailyForecast;
use smearor_weather_model::DailyForecastData;
use stabby::option::Option as StabbyOption;

use crate::config::WeatherServiceConfig;

/// Fetches weather data from Open-Meteo and maps it to model types.
pub struct WeatherFetcher {
    client: Client,
}

impl WeatherFetcher {
    /// Creates a new fetcher with a default Open-Meteo client.
    pub fn new() -> Self {
        Self { client: Client::new() }
    }

    /// Fetches forecast and air quality data, returning a combined result.
    pub async fn fetch(&self, config: &WeatherServiceConfig) -> Result<FetchResult, String> {
        let forecast = self.fetch_forecast(config).await?;
        let air_quality = if config.enable_air_quality {
            self.fetch_air_quality(config).await.ok()
        } else {
            None
        };
        Ok(FetchResult { forecast, air_quality })
    }

    async fn fetch_forecast(&self, config: &WeatherServiceConfig) -> Result<ForecastResponse, String> {
        let timezone = if config.timezone == "auto" || config.timezone.is_empty() {
            Timezone::Auto
        } else {
            Timezone::Iana(config.timezone.clone())
        };

        let response = self
            .client
            .forecast(config.latitude, config.longitude)
            .current([
                CurrentVar::Temperature2m,
                CurrentVar::CloudCover,
                CurrentVar::RelativeHumidity2m,
                CurrentVar::WindSpeed10m,
                CurrentVar::WindDirection10m,
                CurrentVar::SurfacePressure,
                CurrentVar::UvIndex,
                CurrentVar::WeatherCode,
                CurrentVar::IsDay,
            ])
            .daily([
                DailyVar::WeatherCode,
                DailyVar::Temperature2mMax,
                DailyVar::Temperature2mMin,
                DailyVar::CloudCoverMean,
                DailyVar::Sunrise,
                DailyVar::Sunset,
                DailyVar::UvIndexMax,
                DailyVar::WindSpeed10mMax,
                DailyVar::PrecipitationSum,
            ])
            .wind_speed_unit(WindSpeedUnit::Kmh)
            .timeformat(TimeFormat::Iso8601)
            .timezone(timezone)
            .forecast_days(config.forecast_days)
            .send()
            .await
            .map_err(|e| format!("Forecast request failed: {e}"))?;

        Ok(response)
    }

    async fn fetch_air_quality(&self, config: &WeatherServiceConfig) -> Result<AirQualityData, String> {
        let timezone = if config.timezone == "auto" || config.timezone.is_empty() {
            Timezone::Auto
        } else {
            Timezone::Iana(config.timezone.clone())
        };

        let response = self
            .client
            .air_quality(config.latitude, config.longitude)
            .current([
                AirQualityHourlyVar::EuropeanAqi,
                AirQualityHourlyVar::Pm10,
                AirQualityHourlyVar::Pm2_5,
                AirQualityHourlyVar::Ozone,
                AirQualityHourlyVar::NitrogenDioxide,
                AirQualityHourlyVar::SulphurDioxide,
                AirQualityHourlyVar::CarbonMonoxide,
            ])
            .timeformat(TimeFormat::Iso8601)
            .timezone(timezone)
            .send()
            .await
            .map_err(|e| format!("Air quality request failed: {e}"))?;

        let current = match &response.current {
            Some(c) => c,
            None => return Ok(AirQualityData::default()),
        };

        Ok(map_air_quality(current))
    }
}

/// Result of a weather data fetch.
pub struct FetchResult {
    /// Forecast response from Open-Meteo.
    pub forecast: ForecastResponse,
    /// Air quality data, if fetched.
    pub air_quality: Option<AirQualityData>,
}

fn extract_current_value_f32(current: &openmeteo_rs::CurrentData, var: CurrentVar) -> StabbyOption<f32> {
    current
        .get_var(var)
        .and_then(|s| s.values_f32())
        .and_then(|v| v.first().copied().flatten())
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None())
}

fn extract_current_value_u16(current: &openmeteo_rs::CurrentData, var: CurrentVar) -> StabbyOption<u16> {
    current
        .get_var(var)
        .and_then(|s| s.values_f32())
        .and_then(|v| v.first().copied().flatten())
        .map(|v| v as u16)
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None())
}

fn extract_current_bool(current: &openmeteo_rs::CurrentData, var: CurrentVar) -> StabbyOption<bool> {
    current
        .get_var(var)
        .and_then(|s| s.values_f32())
        .and_then(|v| v.first().copied().flatten())
        .map(|v| v != 0.0)
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None())
}

/// Maps the Open-Meteo forecast response to the model's current weather struct.
pub fn map_current_weather(response: &ForecastResponse) -> CurrentWeather {
    let Some(current) = &response.current else {
        return CurrentWeather::default();
    };
    CurrentWeather {
        temperature: extract_current_value_f32(current, CurrentVar::Temperature2m),
        cloud_cover: extract_current_value_f32(current, CurrentVar::CloudCover),
        relative_humidity: extract_current_value_f32(current, CurrentVar::RelativeHumidity2m),
        wind_speed: extract_current_value_f32(current, CurrentVar::WindSpeed10m),
        wind_direction: extract_current_value_f32(current, CurrentVar::WindDirection10m),
        pressure: extract_current_value_f32(current, CurrentVar::SurfacePressure),
        uv_index: extract_current_value_f32(current, CurrentVar::UvIndex),
        weather_code: extract_current_value_u16(current, CurrentVar::WeatherCode),
        is_day: extract_current_bool(current, CurrentVar::IsDay),
    }
}

fn extract_daily_value_f32(daily: &openmeteo_rs::DailyData, var: DailyVar, index: usize) -> StabbyOption<f32> {
    daily
        .get_var(var)
        .and_then(|s| s.values_f32())
        .and_then(|v| v.get(index).copied().flatten())
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None())
}

fn extract_daily_value_u16(daily: &openmeteo_rs::DailyData, var: DailyVar, index: usize) -> StabbyOption<u16> {
    daily
        .get_var(var)
        .and_then(|s| s.values_f32())
        .and_then(|v| v.get(index).copied().flatten())
        .map(|v| v as u16)
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None())
}

fn extract_daily_string(daily: &openmeteo_rs::DailyData, var: DailyVar, index: usize) -> StabbyOption<stabby::string::String> {
    daily
        .get_var(var)
        .and_then(|s| s.values_str())
        .and_then(|v| v.get(index).cloned().flatten())
        .map(|s| stabby::string::String::from(s.as_str()))
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None())
}

fn map_daily_forecast(daily: &openmeteo_rs::DailyData, index: usize) -> DailyForecast {
    DailyForecast {
        temperature_max: extract_daily_value_f32(daily, DailyVar::Temperature2mMax, index),
        temperature_min: extract_daily_value_f32(daily, DailyVar::Temperature2mMin, index),
        cloud_cover: extract_daily_value_f32(daily, DailyVar::CloudCoverMean, index),
        sunrise: extract_daily_string(daily, DailyVar::Sunrise, index),
        sunset: extract_daily_string(daily, DailyVar::Sunset, index),
        uv_index_max: extract_daily_value_f32(daily, DailyVar::UvIndexMax, index),
        wind_speed_max: extract_daily_value_f32(daily, DailyVar::WindSpeed10mMax, index),
        precipitation_sum: extract_daily_value_f32(daily, DailyVar::PrecipitationSum, index),
        weather_code: extract_daily_value_u16(daily, DailyVar::WeatherCode, index),
    }
}

/// Maps the Open-Meteo forecast response to the model's daily forecast data.
pub fn map_daily_forecast_data(response: &ForecastResponse) -> DailyForecastData {
    let Some(daily) = &response.daily else {
        return DailyForecastData::default();
    };
    DailyForecastData {
        today: map_daily_forecast(daily, 0),
        tomorrow: map_daily_forecast(daily, 1),
    }
}

fn map_air_quality(current: &openmeteo_rs::CurrentData) -> AirQualityData {
    let get = |api_name: &str| -> StabbyOption<f32> {
        current
            .get(api_name)
            .and_then(|s| s.values_f32())
            .and_then(|v| v.first().copied().flatten())
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None())
    };
    AirQualityData {
        european_aqi: get("european_aqi"),
        pm10: get("pm10"),
        pm2_5: get("pm2_5"),
        ozone: get("ozone"),
        nitrogen_dioxide: get("nitrogen_dioxide"),
        sulphur_dioxide: get("sulphur_dioxide"),
        carbon_monoxide: get("carbon_monoxide"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_fetcher_new_creates_client() {
        let _fetcher = WeatherFetcher::new();
    }
}
