use serde::Deserialize;
use std::str::FromStr;

/// Defines the transition animation for area changes
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum AreaTransition {
    /// No transition, instant change
    None,
    /// Fade in/out transition
    Fade,
    /// Slide from left
    SlideLeft,
    /// Slide from right
    SlideRight,
    /// Slide from top
    SlideUp,
    /// Slide from bottom
    SlideDown,
    /// Pop in/out effect
    Pop,
    /// Scale in/out effect
    Scale,
}

impl AreaTransition {
    pub fn css_class(&self) -> &str {
        match self {
            AreaTransition::None => "transition-none",
            AreaTransition::Fade => "transition-fade",
            AreaTransition::SlideLeft => "transition-slide-left",
            AreaTransition::SlideRight => "transition-slide-right",
            AreaTransition::SlideUp => "transition-slide-up",
            AreaTransition::SlideDown => "transition-slide-down",
            AreaTransition::Pop => "transition-pop",
            AreaTransition::Scale => "transition-scale",
        }
    }
}

impl Default for AreaTransition {
    fn default() -> Self {
        AreaTransition::Fade
    }
}

impl FromStr for AreaTransition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(AreaTransition::None),
            "fade" => Ok(AreaTransition::Fade),
            "slide_left" => Ok(AreaTransition::SlideLeft),
            "slide_right" => Ok(AreaTransition::SlideRight),
            "slide_up" => Ok(AreaTransition::SlideUp),
            "slide_down" => Ok(AreaTransition::SlideDown),
            "pop" => Ok(AreaTransition::Pop),
            "scale" => Ok(AreaTransition::Scale),
            _ => Err(format!("Invalid transition: {}", s)),
        }
    }
}
