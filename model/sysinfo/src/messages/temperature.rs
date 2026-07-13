/// A single temperature component reading.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct TemperatureComponent {
    /// Human-readable label of the component (e.g. "k10temp Tctl").
    pub label: stabby::string::String,
    /// Identifier of the component (e.g. "hwmon0_1" or "thermal_zone0").
    pub id: stabby::string::String,
    /// Temperature in degrees Celsius.
    pub temperature: stabby::option::Option<f32>,
    /// Maximum observed temperature in degrees Celsius.
    pub max_temperature: stabby::option::Option<f32>,
    /// Critical temperature threshold in degrees Celsius.
    pub critical_temperature: stabby::option::Option<f32>,
}

impl TemperatureComponent {
    /// Creates a new temperature component.
    pub fn new(
        label: impl Into<stabby::string::String>,
        id: impl Into<stabby::string::String>,
        temperature: stabby::option::Option<f32>,
        max_temperature: stabby::option::Option<f32>,
        critical_temperature: stabby::option::Option<f32>,
    ) -> Self {
        Self {
            label: label.into(),
            id: id.into(),
            temperature,
            max_temperature,
            critical_temperature,
        }
    }
}
