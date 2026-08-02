use smearor_swipe_launcher_plugin_api::Locale;

/// Weather widget labels that can be localized.
#[derive(Copy, Clone, Debug)]
pub enum WeatherLabel {
    Today,
    Tomorrow,
    Humidity,
    Sunrise,
    Sunset,
    Pressure,
    UvIndex,
    Sunshine,
    RainChance,
    RainAmount,
    Precipitation,
}

impl WeatherLabel {
    /// Returns a localized label for the given key and locale.
    /// Falls back to English when the locale is not supported.
    pub fn localized_label(&self, locale: Locale) -> &'static str {
        match locale {
            Locale::DeDe => self.german(),
            Locale::FrFr => self.french(),
            Locale::ItIt => self.italian(),
            Locale::EsEs => self.spanish(),
            _ => self.english(),
        }
    }

    fn english(&self) -> &'static str {
        match self {
            WeatherLabel::Today => "Today",
            WeatherLabel::Tomorrow => "Tomorrow",
            WeatherLabel::Humidity => "Humidity",
            WeatherLabel::Sunrise => "Sunrise",
            WeatherLabel::Sunset => "Sunset",
            WeatherLabel::Pressure => "Pressure",
            WeatherLabel::UvIndex => "UV Index",
            WeatherLabel::Sunshine => "Sunshine",
            WeatherLabel::RainChance => "Rain Chance",
            WeatherLabel::RainAmount => "Rain Amount",
            WeatherLabel::Precipitation => "Precipitation",
        }
    }

    fn german(&self) -> &'static str {
        match self {
            WeatherLabel::Today => "Heute",
            WeatherLabel::Tomorrow => "Morgen",
            WeatherLabel::Humidity => "Luftfeuchtigkeit",
            WeatherLabel::Sunrise => "Sonnenaufgang",
            WeatherLabel::Sunset => "Sonnenuntergang",
            WeatherLabel::Pressure => "Luftdruck",
            WeatherLabel::UvIndex => "UV-Index",
            WeatherLabel::Sunshine => "Sonnenstunden",
            WeatherLabel::RainChance => "Regenwahrsch.",
            WeatherLabel::RainAmount => "Regenmenge",
            WeatherLabel::Precipitation => "Niederschlag",
        }
    }

    fn french(&self) -> &'static str {
        match self {
            WeatherLabel::Today => "Aujourd'hui",
            WeatherLabel::Tomorrow => "Demain",
            WeatherLabel::Humidity => "Humidit\u{e9}",
            WeatherLabel::Sunrise => "Lever du soleil",
            WeatherLabel::Sunset => "Coucher du soleil",
            WeatherLabel::Pressure => "Pression",
            WeatherLabel::UvIndex => "Indice UV",
            WeatherLabel::Sunshine => "Ensoleillement",
            WeatherLabel::RainChance => "Risque de pluie",
            WeatherLabel::RainAmount => "Pr\u{e9}cipitations",
            WeatherLabel::Precipitation => "Pr\u{e9}cipitation",
        }
    }

    fn spanish(&self) -> &'static str {
        match self {
            WeatherLabel::Today => "Hoy",
            WeatherLabel::Tomorrow => "Ma\u{f1}ana",
            WeatherLabel::Humidity => "Humedad",
            WeatherLabel::Sunrise => "Amanecer",
            WeatherLabel::Sunset => "Atardecer",
            WeatherLabel::Pressure => "Presi\u{f3}n",
            WeatherLabel::UvIndex => "\u{cd}ndice UV",
            WeatherLabel::Sunshine => "Horas de sol",
            WeatherLabel::RainChance => "Prob. de lluvia",
            WeatherLabel::RainAmount => "Cantidad de lluvia",
            WeatherLabel::Precipitation => "Precipitaci\u{f3}n",
        }
    }

    fn italian(&self) -> &'static str {
        match self {
            WeatherLabel::Today => "Oggi",
            WeatherLabel::Tomorrow => "Domani",
            WeatherLabel::Humidity => "Umidit\u{e0}",
            WeatherLabel::Sunrise => "Alba",
            WeatherLabel::Sunset => "Tramonto",
            WeatherLabel::Pressure => "Pressione",
            WeatherLabel::UvIndex => "Indice UV",
            WeatherLabel::Sunshine => "Ore di sole",
            WeatherLabel::RainChance => "Prob. di pioggia",
            WeatherLabel::RainAmount => "Quantit\u{e0} di pioggia",
            WeatherLabel::Precipitation => "Precipitazione",
        }
    }
}
