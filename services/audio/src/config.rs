use serde::Deserialize;
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct AudioServiceConfig {
    /// Volume change step as a ratio (e.g. 0.05 for 5%)
    #[builder(default = 0.05)]
    pub(crate) volume_step: f32,
    /// Maximum allowed volume ratio (e.g. 1.5 for 150% overdrive)
    #[builder(default = 1.5)]
    pub(crate) max_volume: f32,
}

impl Default for AudioServiceConfig {
    fn default() -> Self {
        Self {
            volume_step: 0.05,
            max_volume: 1.5,
        }
    }
}
