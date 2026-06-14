use serde::Deserialize;
use std::str::FromStr;

pub struct AreaTransitionParams {
    pub(crate) start_opacity: f64,
    pub(crate) target_opacity: f64,
    pub(crate) start_margin_x: i32,
    pub(crate) target_margin_x: i32,
    pub(crate) start_margin_y: i32,
    pub(crate) target_margin_y: i32,
}

impl AreaTransitionParams {
    pub fn new(start_opacity: f64, target_opacity: f64, start_margin_x: i32, target_margin_x: i32, start_margin_y: i32, target_margin_y: i32) -> Self {
        Self {
            start_opacity,
            target_opacity,
            start_margin_x,
            target_margin_x,
            start_margin_y,
            target_margin_y,
        }
    }
}

/// Defines the transition animation for area changes
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum AreaTransition {
    /// No transition, instant change
    None,
    /// Fade in (opacity 0 -> 1)
    FadeIn,
    /// Fade out (opacity 1 -> 0)
    FadeOut,
    /// Slide from left
    SlideLeft,
    /// Slide from right
    SlideRight,
    /// Slide from top
    SlideUp,
    /// Slide from bottom
    SlideDown,
    /// Pop in effect (scale 0.5 -> 1.0 with bounce)
    PopIn,
    /// Pop out effect (scale 1.0 -> 0.5)
    PopOut,
    /// Scale in (scale 0.0 -> 1.0)
    ScaleIn,
    /// Scale out (scale 1.0 -> 0.0)
    ScaleOut,
}

impl AreaTransition {
    pub fn opposite(&self) -> AreaTransition {
        match self {
            AreaTransition::None => AreaTransition::None,
            AreaTransition::FadeIn => AreaTransition::FadeOut,
            AreaTransition::FadeOut => AreaTransition::FadeIn,
            AreaTransition::SlideLeft => AreaTransition::SlideRight,
            AreaTransition::SlideRight => AreaTransition::SlideLeft,
            AreaTransition::SlideUp => AreaTransition::SlideDown,
            AreaTransition::SlideDown => AreaTransition::SlideUp,
            AreaTransition::PopIn => AreaTransition::PopOut,
            AreaTransition::PopOut => AreaTransition::PopIn,
            AreaTransition::ScaleIn => AreaTransition::ScaleOut,
            AreaTransition::ScaleOut => AreaTransition::ScaleIn,
        }
    }
    pub fn css_class(&self) -> &str {
        match self {
            AreaTransition::None => "transition-none",
            AreaTransition::FadeIn => "transition-fade-in",
            AreaTransition::FadeOut => "transition-fade-out",
            AreaTransition::SlideLeft => "transition-slide-left",
            AreaTransition::SlideRight => "transition-slide-right",
            AreaTransition::SlideUp => "transition-slide-up",
            AreaTransition::SlideDown => "transition-slide-down",
            AreaTransition::PopIn => "transition-pop-in",
            AreaTransition::PopOut => "transition-pop-out",
            AreaTransition::ScaleIn => "transition-scale-in",
            AreaTransition::ScaleOut => "transition-scale-out",
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

    pub fn enter_transition_parameters(&self) -> Option<AreaTransitionParams> {
        Some(match self {
            AreaTransition::None => return None,
            AreaTransition::FadeIn => AreaTransitionParams::new(0.001, 1.0, 0, 0, 0, 0),
            AreaTransition::FadeOut => AreaTransitionParams::new(1.0, 0.0, 0, 0, 0, 0),
            AreaTransition::SlideLeft => AreaTransitionParams::new(0.001, 1.0, 200, 0, 0, 0),
            AreaTransition::SlideRight => AreaTransitionParams::new(0.001, 1.0, -200, 0, 0, 0),
            AreaTransition::SlideUp => AreaTransitionParams::new(0.001, 1.0, 0, 0, 200, 0),
            AreaTransition::SlideDown => AreaTransitionParams::new(0.001, 1.0, 0, 0, -200, 0),
            AreaTransition::PopIn => AreaTransitionParams::new(0.001, 1.0, 0, 0, 0, 0),
            AreaTransition::PopOut => AreaTransitionParams::new(1.0, 0.0, 0, 0, 0, 0),
            AreaTransition::ScaleIn => AreaTransitionParams::new(0.001, 1.0, 0, 0, 0, 0),
            AreaTransition::ScaleOut => AreaTransitionParams::new(1.0, 0.0, 0, 0, 0, 0),
        })
    }

    pub fn exit_transition_parameters(&self) -> Option<AreaTransitionParams> {
        Some(match self {
            AreaTransition::None => return None,
            AreaTransition::FadeIn => AreaTransitionParams::new(1.0, 0.0, 0, 0, 0, 0),
            AreaTransition::FadeOut => AreaTransitionParams::new(1.0, 0.0, 0, 0, 0, 0),
            AreaTransition::SlideLeft => AreaTransitionParams::new(1.0, 0.0, 0, -200, 0, 0),
            AreaTransition::SlideRight => AreaTransitionParams::new(1.0, 0.0, 0, 200, 0, 0),
            AreaTransition::SlideUp => AreaTransitionParams::new(1.0, 0.0, 0, 0, 0, -200),
            AreaTransition::SlideDown => AreaTransitionParams::new(1.0, 0.0, 0, 0, 0, 200),
            AreaTransition::PopIn => AreaTransitionParams::new(1.0, 0.0, 0, 0, 0, 0),
            AreaTransition::PopOut => AreaTransitionParams::new(1.0, 0.0, 0, 0, 0, 0),
            AreaTransition::ScaleIn => AreaTransitionParams::new(1.0, 0.0, 0, 0, 0, 0),
            AreaTransition::ScaleOut => AreaTransitionParams::new(1.0, 0.0, 0, 0, 0, 0),
        })
    }
}

impl Default for AreaTransition {
    fn default() -> Self {
        AreaTransition::FadeIn
    }
}

impl FromStr for AreaTransition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace("_", "").replace("-", "").as_str() {
            "none" => Ok(AreaTransition::None),
            "fadein" => Ok(AreaTransition::FadeIn),
            "fadeout" => Ok(AreaTransition::FadeOut),
            "slideleft" => Ok(AreaTransition::SlideLeft),
            "slideright" => Ok(AreaTransition::SlideRight),
            "slideup" => Ok(AreaTransition::SlideUp),
            "slidedown" => Ok(AreaTransition::SlideDown),
            "popin" => Ok(AreaTransition::PopIn),
            "popout" => Ok(AreaTransition::PopOut),
            "scalein" => Ok(AreaTransition::ScaleIn),
            "scaleout" => Ok(AreaTransition::ScaleOut),
            _ => Err(format!("Invalid transition: {}", s)),
        }
    }
}
