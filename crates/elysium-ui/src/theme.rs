use gpui::{rgb, hsl, white};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
    pub spacing: Spacing,
    pub typography: Typography,
    pub effects: Effects,
    pub animations: Animations,
    pub transitions: Transitions,
}

#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub primary: gpui::Hsla,
    pub secondary: gpui::Hsla,
    pub background: gpui::Hsla,
    pub surface: gpui::Hsla,
    pub error: gpui::Hsla,
    pub warning: gpui::Hsla,
    pub success: gpui::Hsla,
    pub text_primary: gpui::Hsla,
    pub text_secondary: gpui::Hsla,
    pub border: gpui::Hsla,
    pub accent: gpui::Hsla,
    pub disabled: gpui::Hsla,
}

#[derive(Debug, Clone)]
pub struct Spacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            xs: 4.0,
            sm: 8.0,
            md: 12.0,
            lg: 16.0,
            xl: 24.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Typography {
    pub font_family: String,
    pub sizes: FontSizes,
    pub weights: FontWeights,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            font_family: "Inter".to_string(), // Popüler açık kaynak font
            sizes: FontSizes::default(),
            weights: FontWeights::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FontSizes {
    pub xs: f32,
    pub sm: f32,
    pub base: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
}

impl Default for FontSizes {
    fn default() -> Self {
        Self {
            xs: 10.0,
            sm: 12.0,
            base: 14.0,
            lg: 16.0,
            xl: 18.0,
            xxl: 24.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FontWeights {
    pub light: u16,
    pub regular: u16,
    pub medium: u16,
    pub bold: u16,
}

impl Default for FontWeights {
    fn default() -> Self {
        Self {
            light: 300,
            regular: 400,
            medium: 500,
            bold: 700,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Effects {
    pub shadow: Shadow,
    pub radius: Radius,
    pub opacity: Opacity,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            shadow: Shadow::default(),
            radius: Radius::default(),
            opacity: Opacity::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Shadow {
    pub small: String,
    pub medium: String,
    pub large: String,
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            small: "0px 1px 2px rgba(0, 0, 0, 0.05)".to_string(),
            medium: "0px 4px 6px -1px rgba(0, 0, 0, 0.1), 0px 2px 4px -1px rgba(0, 0, 0, 0.06)".to_string(),
            large: "0px 10px 15px -3px rgba(0, 0, 0, 0.1), 0px 4px 6px -2px rgba(0, 0, 0, 0.05)".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Radius {
    pub small: f32,
    pub medium: f32,
    pub large: f32,
    pub full: f32,
}

impl Default for Radius {
    fn default() -> Self {
        Self {
            small: 4.0,
            medium: 8.0,
            large: 12.0,
            full: 9999.0, // Tam yuvarlak
        }
    }
}

#[derive(Debug, Clone)]
pub struct Opacity {
    pub low: f32,
    pub medium: f32,
    pub high: f32,
    pub full: f32,
}

impl Default for Opacity {
    fn default() -> Self {
        Self {
            low: 0.25,
            medium: 0.5,
            high: 0.75,
            full: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Animations {
    pub duration_short: Duration,
    pub duration_medium: Duration,
    pub duration_long: Duration,
    pub easing_functions: EasingFunctions,
}

impl Default for Animations {
    fn default() -> Self {
        Self {
            duration_short: Duration::from_millis(150),
            duration_medium: Duration::from_millis(300),
            duration_long: Duration::from_millis(500),
            easing_functions: EasingFunctions::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EasingFunctions {
    pub ease_in: String,
    pub ease_out: String,
    pub ease_in_out: String,
    pub linear: String,
    pub bounce: String,
}

impl Default for EasingFunctions {
    fn default() -> Self {
        Self {
            ease_in: "cubic-bezier(0.42, 0, 1, 1)".to_string(),
            ease_out: "cubic-bezier(0, 0, 0.58, 1)".to_string(),
            ease_in_out: "cubic-bezier(0.42, 0, 0.58, 1)".to_string(),
            linear: "cubic-bezier(0, 0, 1, 1)".to_string(),
            bounce: "cubic-bezier(0.5, 0.075, 0.2, 0.9)".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Transitions {
    pub fade: Transition,
    pub slide: Transition,
    pub scale: Transition,
    pub color: Transition,
}

impl Default for Transitions {
    fn default() -> Self {
        Self {
            fade: Transition::new(Duration::from_millis(200), "ease-in-out"),
            slide: Transition::new(Duration::from_millis(300), "ease-out"),
            scale: Transition::new(Duration::from_millis(150), "ease-in-out"),
            color: Transition::new(Duration::from_millis(250), "linear"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub duration: Duration,
    pub easing: String,
}

impl Transition {
    pub fn new(duration: Duration, easing: &str) -> Self {
        Self {
            duration,
            easing: easing.to_string(),
        }
    }
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            name: "Dark".to_string(),
            colors: ThemeColors {
                primary: rgb(0x6366F1), // indigo-500
                secondary: rgb(0x8B5CF6), // violet-500
                background: rgb(0x0F172A), // slate-900
                surface: rgb(0x1E293B), // slate-800
                error: rgb(0xEF4444), // red-500
                warning: rgb(0xF59E0B), // amber-500
                success: rgb(0x10B981), // emerald-500
                text_primary: rgb(0xF1F5F9), // slate-100
                text_secondary: rgb(0x94A3B8), // slate-400
                border: rgb(0x475569), // slate-600
                accent: rgb(0x0EA5E9), // sky-500
                disabled: rgb(0x64748B), // slate-500
            },
            spacing: Spacing::default(),
            typography: Typography::default(),
            effects: Effects::default(),
            animations: Animations::default(),
            transitions: Transitions::default(),
        }
    }

    pub fn light() -> Self {
        Self {
            name: "Light".to_string(),
            colors: ThemeColors {
                primary: rgb(0x3B82F6), // blue-500
                secondary: rgb(0x8B5CF6), // violet-500
                background: rgb(0xF8FAFC), // slate-50
                surface: rgb(0xFFFFFF), // white
                error: rgb(0xEF4444), // red-500
                warning: rgb(0xF59E0B), // amber-500
                success: rgb(0x10B981), // emerald-500
                text_primary: rgb(0x0F172A), // slate-900
                text_secondary: rgb(0x64748B), // slate-500
                border: rgb(0xE2E8F0), // slate-200
                accent: rgb(0x0284C7), // sky-600
                disabled: rgb(0x94A3B8), // slate-400
            },
            spacing: Spacing::default(),
            typography: Typography::default(),
            effects: Effects::default(),
            animations: Animations::default(),
            transitions: Transitions::default(),
        }
    }

    pub fn high_contrast() -> Self {
        Self {
            name: "High Contrast".to_string(),
            colors: ThemeColors {
                primary: rgb(0xFF0000), // Red for better contrast
                secondary: rgb(0x00FF00), // Green
                background: rgb(0x000000), // Black
                surface: rgb(0x222222), // Very dark gray
                error: rgb(0xFF4444), // Bright red
                warning: rgb(0xFFFF00), // Yellow
                success: rgb(0x00FF00), // Bright green
                text_primary: rgb(0xFFFFFF), // White
                text_secondary: rgb(0xDDDDDD), // Light gray
                border: rgb(0xAAAAAA), // Medium gray
                accent: rgb(0x00FFFF), // Cyan
                disabled: rgb(0x666666), // Gray
            },
            spacing: Spacing {
                xs: 6.0,
                sm: 12.0,
                md: 18.0,
                lg: 24.0,
                xl: 36.0,
            }, // Larger spacing for accessibility
            typography: Typography {
                font_family: "Arial".to_string(), // More accessible font
                sizes: FontSizes {
                    xs: 12.0,
                    sm: 14.0,
                    base: 16.0,
                    lg: 18.0,
                    xl: 22.0,
                    xxl: 32.0,
                }, // Larger fonts for readability
                weights: FontWeights {
                    light: 400,
                    regular: 500,
                    medium: 600,
                    bold: 700,
                },
            },
            effects: Effects {
                shadow: Shadow {
                    small: "0px 2px 4px rgba(255, 255, 255, 0.2)".to_string(),
                    medium: "0px 4px 8px rgba(255, 255, 255, 0.3)".to_string(),
                    large: "0px 8px 16px rgba(255, 255, 255, 0.4)".to_string(),
                },
                radius: Radius {
                    small: 0.0, // Sharp corners for better visibility
                    medium: 0.0,
                    large: 0.0,
                    full: 0.0,
                },
                opacity: Opacity::default(),
            },
            animations: Animations {
                duration_short: Duration::from_millis(0), // Disable animations for some users
                duration_medium: Duration::from_millis(0),
                duration_long: Duration::from_millis(0),
                easing_functions: EasingFunctions::default(),
            },
            transitions: Transitions::default(),
        }
    }

    pub fn get_transition_for_state(&self, state: &str) -> &Transition {
        match state {
            "hover" | "focus" => &self.transitions.fade,
            "active" => &self.transitions.scale,
            "disabled" => &self.transitions.color,
            _ => &self.transitions.slide,
        }
    }

    pub fn get_animation_duration(&self, animation_type: &str) -> Duration {
        match animation_type {
            "quick" => self.animations.duration_short,
            "normal" => self.animations.duration_medium,
            "slow" => self.animations.duration_long,
            _ => self.animations.duration_medium,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            colors: ThemeColors::default(),
            spacing: Spacing::default(),
            typography: Typography::default(),
            effects: Effects::default(),
            animations: Animations::default(),
            transitions: Transitions::default(),
        }
    }
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            primary: rgb(0x3B82F6), // blue-500
            secondary: rgb(0x8B5CF6), // violet-500
            background: rgb(0xF8FAFC), // slate-50
            surface: rgb(0xFFFFFF), // white
            error: rgb(0xEF4444), // red-500
            warning: rgb(0xF59E0B), // amber-500
            success: rgb(0x10B981), // emerald-500
            text_primary: rgb(0x0F172A), // slate-900
            text_secondary: rgb(0x64748B), // slate-500
            border: rgb(0xE2E8F0), // slate-200
            accent: rgb(0x0284C7), // sky-600
            disabled: rgb(0x94A3B8), // slate-400
        }
    }
}