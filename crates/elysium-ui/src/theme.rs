//! Tema sistemi — renk paleti ve stiller.

use serde::{Serialize, Deserialize};

/// Tema rengi
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ThemeColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ThemeColor {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

/// Koyu tema (varsayılan)
pub struct DarkTheme;

impl DarkTheme {
    pub const BG_PRIMARY: ThemeColor = ThemeColor::rgb(30, 30, 32);
    pub const BG_SECONDARY: ThemeColor = ThemeColor::rgb(45, 45, 48);
    pub const BG_HEADER: ThemeColor = ThemeColor::rgb(51, 51, 55);
    pub const BG_PANEL: ThemeColor = ThemeColor::rgb(37, 37, 38);
    pub const BG_BUTTON: ThemeColor = ThemeColor::rgb(55, 55, 58);
    pub const BG_HOVER: ThemeColor = ThemeColor::rgb(62, 62, 66);
    pub const BG_PRESSED: ThemeColor = ThemeColor::rgb(25, 65, 110);
    pub const BG_SELECTED: ThemeColor = ThemeColor::rgb(9, 71, 113);

    pub const TEXT_PRIMARY: ThemeColor = ThemeColor::rgb(240, 240, 240);
    pub const TEXT_SECONDARY: ThemeColor = ThemeColor::rgb(150, 150, 150);
    pub const TEXT_ACCENT: ThemeColor = ThemeColor::rgb(100, 180, 255);
    pub const TEXT_SUCCESS: ThemeColor = ThemeColor::rgb(120, 220, 140);
    pub const TEXT_WARNING: ThemeColor = ThemeColor::rgb(240, 180, 60);
    pub const TEXT_ERROR: ThemeColor = ThemeColor::rgb(220, 80, 80);

    pub const BORDER: ThemeColor = ThemeColor::rgb(60, 60, 64);
    pub const ACCENT: ThemeColor = ThemeColor::rgb(60, 130, 220);
    pub const SUCCESS: ThemeColor = ThemeColor::rgb(40, 160, 70);
    pub const WARNING: ThemeColor = ThemeColor::rgb(200, 150, 30);
    pub const ERROR: ThemeColor = ThemeColor::rgb(180, 50, 40);
}

/// Açık tema
pub struct LightTheme;

impl LightTheme {
    pub const BG_PRIMARY: ThemeColor = ThemeColor::rgb(240, 240, 240);
    pub const BG_SECONDARY: ThemeColor = ThemeColor::rgb(220, 220, 225);
    pub const BG_HEADER: ThemeColor = ThemeColor::rgb(200, 200, 205);
    pub const TEXT_PRIMARY: ThemeColor = ThemeColor::rgb(30, 30, 32);
    pub const TEXT_SECONDARY: ThemeColor = ThemeColor::rgb(100, 100, 105);
    pub const BORDER: ThemeColor = ThemeColor::rgb(180, 180, 185);
}

/// Köşe yuvarlaklığı stili
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CornerStyle {
    None,
    Small,   // 2px
    Medium,  // 4px
    Large,   // 8px
    Round,   // 12px
}

/// Border stili
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BorderStyle {
    None,
    Thin,
    Thick,
    Dashed,
}

/// Widget stili
#[derive(Clone, Debug)]
pub struct WidgetStyle {
    pub bg_color: ThemeColor,
    pub text_color: ThemeColor,
    pub border_color: ThemeColor,
    pub border_style: BorderStyle,
    pub corner_style: CornerStyle,
    pub padding: [i32; 4],
    pub margin: [i32; 4],
}

impl Default for WidgetStyle {
    fn default() -> Self {
        Self {
            bg_color: DarkTheme::BG_SECONDARY,
            text_color: DarkTheme::TEXT_PRIMARY,
            border_color: DarkTheme::BORDER,
            border_style: BorderStyle::None,
            corner_style: CornerStyle::None,
            padding: [4, 8, 4, 8],
            margin: [0; 4],
        }
    }
}

impl WidgetStyle {
    pub fn button() -> Self {
        Self {
            bg_color: DarkTheme::BG_BUTTON,
            text_color: DarkTheme::TEXT_PRIMARY,
            corner_style: CornerStyle::Small,
            padding: [6, 12, 6, 12],
            ..Default::default()
        }
    }

    pub fn panel() -> Self {
        Self {
            bg_color: DarkTheme::BG_PANEL,
            border_color: DarkTheme::BORDER,
            border_style: BorderStyle::Thin,
            corner_style: CornerStyle::Small,
            padding: [8, 8, 8, 8],
            ..Default::default()
        }
    }

    pub fn header() -> Self {
        Self {
            bg_color: DarkTheme::BG_HEADER,
            text_color: DarkTheme::TEXT_PRIMARY,
            padding: [6, 12, 6, 12],
            ..Default::default()
        }
    }
}
