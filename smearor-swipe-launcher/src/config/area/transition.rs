use serde::Deserialize;
use std::str::FromStr;

pub struct AreaTransitionParams {
    pub(crate) start_opacity: f64,
    pub(crate) target_opacity: f64,
    pub(crate) start_margin_x: i32,
    pub(crate) target_margin_x: i32,
    pub(crate) start_margin_y: i32,
    pub(crate) target_margin_y: i32,
    pub(crate) start_scale: f64,
    pub(crate) target_scale: f64,
}

impl AreaTransitionParams {
    pub fn new(
        start_opacity: f64,
        target_opacity: f64,
        start_margin_x: i32,
        target_margin_x: i32,
        start_margin_y: i32,
        target_margin_y: i32,
        start_scale: f64,
        target_scale: f64,
    ) -> Self {
        Self {
            start_opacity,
            target_opacity,
            start_margin_x,
            target_margin_x,
            start_margin_y,
            target_margin_y,
            start_scale,
            target_scale,
        }
    }
}

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

    pub fn transition_enter_css_class(&self) -> Option<String> {
        match self {
            AreaTransition::None => None,
            _ => Some(format!("{}-enter", self.css_class())),
        }
    }

    pub fn transition_exit_css_class(&self) -> Option<String> {
        match self {
            AreaTransition::None => None,
            _ => Some(format!("{}-exit", self.css_class())),
        }
    }

    pub fn transition_active_css_class(&self) -> Option<String> {
        match self {
            AreaTransition::None => None,
            _ => Some(format!("{}-active", self.css_class())),
        }
    }

    pub fn animation_parameters(&self) -> Option<AreaTransitionParams> {
        Some(match self {
            AreaTransition::None => return None,
            AreaTransition::Fade => AreaTransitionParams::new(0.0, 1.0, 0, 0, 0, 0, 1.0, 1.0),
            AreaTransition::SlideLeft => AreaTransitionParams::new(0.0, 1.0, 200, 0, 0, 0, 1.0, 1.0),
            AreaTransition::SlideRight => AreaTransitionParams::new(0.0, 1.0, -200, 0, 0, 0, 1.0, 1.0),
            AreaTransition::SlideUp => AreaTransitionParams::new(0.0, 1.0, 0, 0, 200, 0, 1.0, 1.0),
            AreaTransition::SlideDown => AreaTransitionParams::new(0.0, 1.0, 0, 0, -200, 0, 1.0, 1.0),
            AreaTransition::Pop => AreaTransitionParams::new(0.0, 1.0, 0, 0, 0, 0, 0.5, 1.0),
            AreaTransition::Scale => AreaTransitionParams::new(0.0, 1.0, 0, 0, 0, 0, 0.0, 1.0),
        })
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
        match s.to_lowercase().replace("_", "").replace("-", "").as_str() {
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
