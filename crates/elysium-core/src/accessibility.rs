use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::Color;

// ============================================================================
// Colorblind Support
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorblindType {
    None,
    Protanopia,
    Deuteranopia,
    Tritanopia,
    Achromatopsia,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ContrastMode {
    Normal,
    High,
    UltraHigh,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ColorblindCorrectionMode {
    None,
    Simulate,
    Correct,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorblindSettings {
    pub colorblind_type: ColorblindType,
    pub contrast_mode: ContrastMode,
    pub correction_mode: ColorblindCorrectionMode,
    pub simulation_strength: f32,
    pub correction_strength: f32,
    pub use_colorblind_palette: bool,
}

impl Default for ColorblindSettings {
    fn default() -> Self {
        Self {
            colorblind_type: ColorblindType::None,
            contrast_mode: ContrastMode::Normal,
            correction_mode: ColorblindCorrectionMode::None,
            simulation_strength: 1.0,
            correction_strength: 1.0,
            use_colorblind_palette: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorblindPalette {
    pub name: String,
    pub colors: HashMap<String, Color>,
    pub accessible_for: Vec<ColorblindType>,
}

impl Default for ColorblindPalette {
    fn default() -> Self {
        let mut colors = HashMap::new();
        colors.insert("primary".to_string(), Color::rgb(0.0, 0.47, 0.83));
        colors.insert("secondary".to_string(), Color::rgb(0.98, 0.6, 0.0));
        colors.insert("success".to_string(), Color::rgb(0.0, 0.62, 0.38));
        colors.insert("warning".to_string(), Color::rgb(0.98, 0.72, 0.0));
        colors.insert("danger".to_string(), Color::rgb(0.85, 0.15, 0.15));
        colors.insert("info".to_string(), Color::rgb(0.0, 0.53, 0.71));
        Self {
            name: "default".to_string(),
            colors,
            accessible_for: vec![ColorblindType::None],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColorblindSimulationFilter {
    pub matrix: [[f32; 3]; 3],
}

impl ColorblindSimulationFilter {
    pub fn new(colorblind_type: ColorblindType) -> Self {
        let matrix = match colorblind_type {
            ColorblindType::Protanopia => {
                [[0.567, 0.433, 0.0], [0.558, 0.442, 0.0], [0.0, 0.242, 0.758]]
            }
            ColorblindType::Deuteranopia => {
                [[0.625, 0.375, 0.0], [0.7, 0.3, 0.0], [0.0, 0.3, 0.7]]
            }
            ColorblindType::Tritanopia => {
                [[0.95, 0.05, 0.0], [0.0, 0.433, 0.567], [0.0, 0.475, 0.525]]
            }
            ColorblindType::Achromatopsia => {
                [[0.299, 0.587, 0.114], [0.299, 0.587, 0.114], [0.299, 0.587, 0.114]]
            }
            ColorblindType::None => {
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
            }
        };
        Self { matrix }
    }

    pub fn apply(&self, color: &Color) -> Color {
        let r = color.r;
        let g = color.g;
        let b = color.b;
        Color::rgb(
            self.matrix[0][0] * r + self.matrix[0][1] * g + self.matrix[0][2] * b,
            self.matrix[1][0] * r + self.matrix[1][1] * g + self.matrix[1][2] * b,
            self.matrix[2][0] * r + self.matrix[2][1] * g + self.matrix[2][2] * b,
        )
    }
}

pub fn simulate_colorblind(color: &Color, colorblind_type: ColorblindType, strength: f32) -> Color {
    if colorblind_type == ColorblindType::None || strength == 0.0 {
        return *color;
    }
    let filter = ColorblindSimulationFilter::new(colorblind_type);
    let simulated = filter.apply(color);
    Color::rgb(
        color.r * (1.0 - strength) + simulated.r * strength,
        color.g * (1.0 - strength) + simulated.g * strength,
        color.b * (1.0 - strength) + simulated.b * strength,
    )
}

pub fn get_colorblind_friendly_color(
    original: &Color,
    colorblind_type: ColorblindType,
) -> Color {
    match colorblind_type {
        ColorblindType::Protanopia | ColorblindType::Deuteranopia => {
            let diff = (original.r - original.g).abs();
            if diff < 0.3 {
                Color::rgb(
                    original.r,
                    original.g,
                    if original.b > 0.5 { original.b } else { original.b + 0.4 },
                )
            } else {
                *original
            }
        }
        ColorblindType::Tritanopia => {
            let diff = (original.b - original.r).abs();
            if diff < 0.3 {
                Color::rgb(
                    if original.r > 0.5 { original.r } else { original.r + 0.4 },
                    original.g,
                    original.b,
                )
            } else {
                *original
            }
        }
        ColorblindType::Achromatopsia => {
            let lum = 0.299 * original.r + 0.587 * original.g + 0.114 * original.b;
            Color::rgb(lum, lum, lum)
        }
        ColorblindType::None => *original,
    }
}

// ============================================================================
// Subtitle System
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubtitleSize {
    Small,
    Medium,
    Large,
    ExtraLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SubtitlePosition {
    Top,
    Center,
    Bottom,
    Custom { x: f32, y: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleStyle {
    pub font_size: SubtitleSize,
    pub text_color: Color,
    pub background_color: Color,
    pub background_opacity: f32,
    pub outline_color: Color,
    pub outline_width: f32,
    pub padding: f32,
    pub max_width: Option<f32>,
    pub line_spacing: f32,
}

impl Default for SubtitleStyle {
    fn default() -> Self {
        Self {
            font_size: SubtitleSize::Medium,
            text_color: Color::rgb(1.0, 1.0, 1.0),
            background_color: Color::rgb(0.0, 0.0, 0.0),
            background_opacity: 0.75,
            outline_color: Color::rgb(0.0, 0.0, 0.0),
            outline_width: 2.0,
            padding: 8.0,
            max_width: Some(800.0),
            line_spacing: 1.2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptionStyle {
    pub show_speaker_labels: bool,
    pub speaker_label_color: Color,
    pub show_sound_descriptions: bool,
    pub sound_description_color: Color,
    pub music_description_color: Color,
    pub bracket_style: bool,
}

impl Default for CaptionStyle {
    fn default() -> Self {
        Self {
            show_speaker_labels: true,
            speaker_label_color: Color::rgb(1.0, 0.9, 0.3),
            show_sound_descriptions: true,
            sound_description_color: Color::rgb(0.6, 0.85, 1.0),
            music_description_color: Color::rgb(0.85, 0.6, 1.0),
            bracket_style: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubtitleLanguage {
    English,
    Turkish,
    Spanish,
    French,
    German,
    Japanese,
    Korean,
    ChineseSimplified,
    ChineseTraditional,
    Portuguese,
    Russian,
    Arabic,
    Hindi,
    Custom(String),
}

impl SubtitleLanguage {
    pub fn as_str(&self) -> &str {
        match self {
            SubtitleLanguage::English => "en",
            SubtitleLanguage::Turkish => "tr",
            SubtitleLanguage::Spanish => "es",
            SubtitleLanguage::French => "fr",
            SubtitleLanguage::German => "de",
            SubtitleLanguage::Japanese => "ja",
            SubtitleLanguage::Korean => "ko",
            SubtitleLanguage::ChineseSimplified => "zh-Hans",
            SubtitleLanguage::ChineseTraditional => "zh-Hant",
            SubtitleLanguage::Portuguese => "pt",
            SubtitleLanguage::Russian => "ru",
            SubtitleLanguage::Arabic => "ar",
            SubtitleLanguage::Hindi => "hi",
            SubtitleLanguage::Custom(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleEntry {
    pub id: String,
    pub start_time: Duration,
    pub end_time: Duration,
    pub speaker: Option<String>,
    pub text: String,
    pub is_sound_description: bool,
    pub is_music_description: bool,
    pub language: SubtitleLanguage,
}

impl SubtitleEntry {
    pub fn is_active(&self, time: Duration) -> bool {
        time >= self.start_time && time <= self.end_time
    }

    pub fn display_text(&self, caption_style: &CaptionStyle) -> String {
        let mut result = String::new();
        if caption_style.show_speaker_labels {
            if let Some(ref speaker) = self.speaker {
                if caption_style.bracket_style {
                    result.push_str(&format!("[{}] ", speaker));
                } else {
                    result.push_str(&format!("{}: ", speaker));
                }
            }
        }
        if self.is_sound_description {
            if caption_style.bracket_style {
                result.push_str(&format!("[{}] ", self.text));
            } else {
                result.push_str(&self.text);
            }
        } else if self.is_music_description {
            result.push_str(&format!("*{}* ", self.text));
        } else {
            result.push_str(&self.text);
        }
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleTrack {
    pub language: SubtitleLanguage,
    pub entries: Vec<SubtitleEntry>,
    pub enabled: bool,
}

impl Default for SubtitleTrack {
    fn default() -> Self {
        Self {
            language: SubtitleLanguage::English,
            entries: Vec::new(),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleSettings {
    pub enabled: bool,
    pub preferred_language: SubtitleLanguage,
    pub style: SubtitleStyle,
    pub caption_style: CaptionStyle,
    pub position: SubtitlePosition,
    pub custom_position: (f32, f32),
    pub min_display_duration: Duration,
    pub max_display_duration: Duration,
    pub tracks: Vec<SubtitleTrack>,
    pub font_scale: f32,
}

impl Default for SubtitleSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            preferred_language: SubtitleLanguage::English,
            style: SubtitleStyle::default(),
            caption_style: CaptionStyle::default(),
            position: SubtitlePosition::Bottom,
            custom_position: (0.5, 0.9),
            min_display_duration: Duration::from_secs(1),
            max_display_duration: Duration::from_secs(7),
            tracks: Vec::new(),
            font_scale: 1.0,
        }
    }
}

// ============================================================================
// Screen Reader Support
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScreenReaderEventType {
    FocusGained,
    FocusLost,
    ValueChanged,
    StateChanged,
    SelectionChanged,
    NavigationComplete,
    Alert,
    Announcement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenReaderEvent {
    pub event_type: ScreenReaderEventType,
    pub element_id: String,
    pub role: AriaRole,
    pub name: String,
    pub description: Option<String>,
    pub value: Option<String>,
    pub priority: AnnouncementPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnnouncementPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AriaRole {
    Button,
    Checkbox,
    Combobox,
    Dialog,
    Heading,
    Image,
    Link,
    Listbox,
    Menu,
    Menuitem,
    Option,
    Progressbar,
    Radiogroup,
    Scrollbar,
    Searchbox,
    Slider,
    Spinbutton,
    Status,
    Switch,
    Tab,
    Tablist,
    Tabpanel,
    Textbox,
    Timer,
    Toolbar,
    Tree,
    Treeitem,
    Generic,
    Custom(String),
}

impl AriaRole {
    pub fn as_str(&self) -> &str {
        match self {
            AriaRole::Button => "button",
            AriaRole::Checkbox => "checkbox",
            AriaRole::Combobox => "combobox",
            AriaRole::Dialog => "dialog",
            AriaRole::Heading => "heading",
            AriaRole::Image => "image",
            AriaRole::Link => "link",
            AriaRole::Listbox => "listbox",
            AriaRole::Menu => "menu",
            AriaRole::Menuitem => "menuitem",
            AriaRole::Option => "option",
            AriaRole::Progressbar => "progressbar",
            AriaRole::Radiogroup => "radiogroup",
            AriaRole::Scrollbar => "scrollbar",
            AriaRole::Searchbox => "searchbox",
            AriaRole::Slider => "slider",
            AriaRole::Spinbutton => "spinbutton",
            AriaRole::Status => "status",
            AriaRole::Switch => "switch",
            AriaRole::Tab => "tab",
            AriaRole::Tablist => "tablist",
            AriaRole::Tabpanel => "tabpanel",
            AriaRole::Textbox => "textbox",
            AriaRole::Timer => "timer",
            AriaRole::Toolbar => "toolbar",
            AriaRole::Tree => "tree",
            AriaRole::Treeitem => "treeitem",
            AriaRole::Generic => "generic",
            AriaRole::Custom(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AriaAttributes {
    pub role: AriaRole,
    pub label: String,
    pub description: Option<String>,
    pub value: Option<String>,
    pub min_value: Option<f32>,
    pub max_value: Option<f32>,
    pub is_required: bool,
    pub is_disabled: bool,
    pub is_hidden: bool,
    pub is_expanded: Option<bool>,
    pub is_selected: Option<bool>,
    pub is_checked: Option<bool>,
    pub is_pressed: Option<bool>,
    pub level: Option<u32>,
    pub live_region: Option<LiveRegionMode>,
    pub controls: Vec<String>,
    pub described_by: Vec<String>,
    pub labelled_by: Vec<String>,
    pub custom_attributes: HashMap<String, String>,
}

impl Default for AriaAttributes {
    fn default() -> Self {
        Self {
            role: AriaRole::Generic,
            label: String::new(),
            description: None,
            value: None,
            min_value: None,
            max_value: None,
            is_required: false,
            is_disabled: false,
            is_hidden: false,
            is_expanded: None,
            is_selected: None,
            is_checked: None,
            is_pressed: None,
            level: None,
            live_region: None,
            controls: Vec::new(),
            described_by: Vec::new(),
            labelled_by: Vec::new(),
            custom_attributes: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LiveRegionMode {
    Off,
    Polite,
    Assertive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TextToSpeechVoice {
    Default,
    Male,
    Female,
    Child,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToSpeechSettings {
    pub enabled: bool,
    pub voice: TextToSpeechVoice,
    pub speech_rate: f32,
    pub speech_pitch: f32,
    pub speech_volume: f32,
    pub preferred_locale: crate::Locale,
    pub verbosity: SpeechVerbosity,
    pub auto_read_navigation: bool,
    pub auto_read_alerts: bool,
    pub auto_read_status: bool,
    pub punctuation_level: PunctuationLevel,
}

impl Default for TextToSpeechSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            voice: TextToSpeechVoice::Default,
            speech_rate: 1.0,
            speech_pitch: 1.0,
            speech_volume: 1.0,
            preferred_locale: crate::Locale::default(),
            verbosity: SpeechVerbosity::Normal,
            auto_read_navigation: true,
            auto_read_alerts: true,
            auto_read_status: false,
            punctuation_level: PunctuationLevel::Some,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpeechVerbosity {
    Minimal,
    Normal,
    Verbose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PunctuationLevel {
    None,
    Some,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenReaderSettings {
    pub enabled: bool,
    pub text_to_speech: TextToSpeechSettings,
    pub announce_focus_changes: bool,
    pub announce_state_changes: bool,
    pub announce_value_changes: bool,
    pub keyboard_navigation: bool,
    pub focus_ring_visible: bool,
    pub focus_ring_color: Color,
    pub focus_ring_width: f32,
    pub focus_trap_dialogs: bool,
    pub bypass_mode: bool,
    pub element_queue: Vec<String>,
    pub reading_paused: bool,
}

impl Default for ScreenReaderSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            text_to_speech: TextToSpeechSettings::default(),
            announce_focus_changes: true,
            announce_state_changes: true,
            announce_value_changes: true,
            keyboard_navigation: true,
            focus_ring_visible: true,
            focus_ring_color: Color::rgb(1.0, 0.9, 0.0),
            focus_ring_width: 3.0,
            focus_trap_dialogs: true,
            bypass_mode: false,
            element_queue: Vec::new(),
            reading_paused: false,
        }
    }
}

// ============================================================================
// Input Accessibility
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputRemapProfile {
    Standard,
    Accessible,
    OneHanded,
    LeftHanded,
    RightHanded,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputRemapEntry {
    pub action: String,
    pub primary_binding: crate::InputBinding,
    pub secondary_binding: Option<crate::InputBinding>,
    pub hold_time: Option<Duration>,
    pub toggle_enabled: bool,
    pub auto_repeat: bool,
    pub auto_repeat_delay: Duration,
    pub auto_repeat_rate: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAccessibilitySettings {
    pub one_handed_mode: bool,
    pub one_handed_layout: crate::OneHandedLayout,
    pub button_behavior: crate::ButtonBehavior,
    pub auto_repeat_enabled: bool,
    pub auto_repeat_delay: Duration,
    pub auto_repeat_rate: Duration,
    pub sticky_keys: bool,
    pub sticky_keys_timeout: Duration,
    pub slow_keys: bool,
    pub slow_keys_delay: Duration,
    pub bounce_keys: bool,
    pub bounce_keys_delay: Duration,
    pub input_remap_profile: InputRemapProfile,
    pub remap_overrides: Vec<InputRemapEntry>,
    pub motion_sickness_reduction: bool,
    pub foveated_rendering: bool,
    pub snap_turn_enabled: bool,
    pub snap_turn_angle: f32,
    pub vignette_intensity: f32,
    pub teleport_movement: bool,
    pub comfort_mode: bool,
}

impl Default for InputAccessibilitySettings {
    fn default() -> Self {
        Self {
            one_handed_mode: false,
            one_handed_layout: crate::OneHandedLayout::Left,
            button_behavior: crate::ButtonBehavior::Hold,
            auto_repeat_enabled: false,
            auto_repeat_delay: Duration::from_millis(500),
            auto_repeat_rate: Duration::from_millis(30),
            sticky_keys: false,
            sticky_keys_timeout: Duration::from_secs(5),
            slow_keys: false,
            slow_keys_delay: Duration::from_millis(500),
            bounce_keys: false,
            bounce_keys_delay: Duration::from_millis(300),
            input_remap_profile: InputRemapProfile::Standard,
            remap_overrides: Vec::new(),
            motion_sickness_reduction: false,
            foveated_rendering: false,
            snap_turn_enabled: false,
            snap_turn_angle: 45.0,
            vignette_intensity: 0.5,
            teleport_movement: false,
            comfort_mode: false,
        }
    }
}

// ============================================================================
// Visual Accessibility
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TextScalingMode {
    System,
    Custom(f32),
}

impl Default for TextScalingMode {
    fn default() -> Self {
        Self::System
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum UIScalingMode {
    System,
    Custom(f32),
}

impl Default for UIScalingMode {
    fn default() -> Self {
        Self::System
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReducedMotionLevel {
    None,
    Some,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualAccessibilitySettings {
    pub text_scaling_mode: TextScalingMode,
    pub text_scale_factor: f32,
    pub ui_scaling_mode: UIScalingMode,
    pub ui_scale_factor: f32,
    pub high_contrast_ui: bool,
    pub high_contrast_color_scheme: HighContrastColorScheme,
    pub reduced_motion: ReducedMotionLevel,
    pub reduced_transparency: bool,
    pub screen_reader_mode: bool,
    pub large_cursor: bool,
    pub cursor_scale: f32,
    pub cursor_color: Color,
    pub cursor_outline: bool,
    pub cursor_outline_color: Color,
    pub bold_text: bool,
    pub increased_decorative_elements: bool,
    pub show_ui_animations: bool,
    pub show_hover_states: bool,
    pub show_focus_indicators: bool,
    pub flash_effects_reduced: bool,
}

impl Default for VisualAccessibilitySettings {
    fn default() -> Self {
        Self {
            text_scaling_mode: TextScalingMode::System,
            text_scale_factor: 1.0,
            ui_scaling_mode: UIScalingMode::System,
            ui_scale_factor: 1.0,
            high_contrast_ui: false,
            high_contrast_color_scheme: HighContrastColorScheme::Light,
            reduced_motion: ReducedMotionLevel::None,
            reduced_transparency: false,
            screen_reader_mode: false,
            large_cursor: false,
            cursor_scale: 1.0,
            cursor_color: Color::rgb(1.0, 1.0, 1.0),
            cursor_outline: false,
            cursor_outline_color: Color::rgb(0.0, 0.0, 0.0),
            bold_text: false,
            increased_decorative_elements: false,
            show_ui_animations: true,
            show_hover_states: true,
            show_focus_indicators: true,
            flash_effects_reduced: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HighContrastColorScheme {
    Light,
    Dark,
    Custom,
}

// ============================================================================
// Audio Accessibility
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AudioChannelMode {
    Stereo,
    Mono,
    LeftOnly,
    RightOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDuckingSettings {
    pub enabled: bool,
    pub duck_on_speech: bool,
    pub duck_on_music: bool,
    pub duck_on_effects: bool,
    pub duck_amount: f32,
    pub duck_attack_time: Duration,
    pub duck_release_time: Duration,
    pub restore_on_silence: bool,
    pub silence_threshold: f32,
}

impl Default for AudioDuckingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            duck_on_speech: true,
            duck_on_music: false,
            duck_on_effects: false,
            duck_amount: 0.5,
            duck_attack_time: Duration::from_millis(200),
            duck_release_time: Duration::from_millis(500),
            restore_on_silence: false,
            silence_threshold: 0.01,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualAudioIndicator {
    pub source_id: String,
    pub position: Option<[f32; 3]>,
    pub min_volume: f32,
    pub max_volume: f32,
    pub indicator_color: Color,
    pub show_waveform: bool,
    pub show_spectrum: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioAccessibilitySettings {
    pub channel_mode: AudioChannelMode,
    pub mono_mix_factor: f32,
    pub left_channel_volume: f32,
    pub right_channel_volume: f32,
    pub ducking: AudioDuckingSettings,
    pub visual_indicators: Vec<VisualAudioIndicator>,
    pub show_visual_indicators: bool,
    pub visual_indicator_position: (f32, f32),
    pub subtitles_for_audio_cues: bool,
    pub audio_cue_threshold: f32,
    pub important_only_subtitles: bool,
    pub show_speaker_volume_levels: bool,
    pub dynamic_range_compression: bool,
    pub compression_threshold: f32,
    pub compression_ratio: f32,
    pub speech_enhancement: bool,
    pub speech_enhancement_amount: f32,
    pub background_noise_reduction: bool,
    pub noise_reduction_amount: f32,
    pub frequency_emphasis: Option<FrequencyRange>,
}

impl Default for AudioAccessibilitySettings {
    fn default() -> Self {
        Self {
            channel_mode: AudioChannelMode::Stereo,
            mono_mix_factor: 0.5,
            left_channel_volume: 1.0,
            right_channel_volume: 1.0,
            ducking: AudioDuckingSettings::default(),
            visual_indicators: Vec::new(),
            show_visual_indicators: false,
            visual_indicator_position: (0.0, 0.0),
            subtitles_for_audio_cues: false,
            audio_cue_threshold: 0.3,
            important_only_subtitles: true,
            show_speaker_volume_levels: false,
            dynamic_range_compression: false,
            compression_threshold: -20.0,
            compression_ratio: 4.0,
            speech_enhancement: false,
            speech_enhancement_amount: 0.5,
            background_noise_reduction: false,
            noise_reduction_amount: 0.5,
            frequency_emphasis: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FrequencyRange {
    pub low_hz: f32,
    pub high_hz: f32,
    pub gain_db: f32,
}

// ============================================================================
// Master Accessibility Settings
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilitySettings {
    pub colorblind: ColorblindSettings,
    pub subtitles: SubtitleSettings,
    pub screen_reader: ScreenReaderSettings,
    pub input: InputAccessibilitySettings,
    pub visual: VisualAccessibilitySettings,
    pub audio: AudioAccessibilitySettings,
    pub enabled: bool,
    pub profile_name: String,
    pub apply_on_change: bool,
}

impl Default for AccessibilitySettings {
    fn default() -> Self {
        Self {
            colorblind: ColorblindSettings::default(),
            subtitles: SubtitleSettings::default(),
            screen_reader: ScreenReaderSettings::default(),
            input: InputAccessibilitySettings::default(),
            visual: VisualAccessibilitySettings::default(),
            audio: AudioAccessibilitySettings::default(),
            enabled: false,
            profile_name: "default".to_string(),
            apply_on_change: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessibilityProfile {
    Standard,
    ColorblindFriendly,
    HighContrast,
    LowVision,
    DeafHardOfHearing,
    MotorImpaired,
    CognitiveAccessible,
    Custom(String),
}

impl AccessibilityProfile {
    pub fn as_str(&self) -> &str {
        match self {
            AccessibilityProfile::Standard => "standard",
            AccessibilityProfile::ColorblindFriendly => "colorblind_friendly",
            AccessibilityProfile::HighContrast => "high_contrast",
            AccessibilityProfile::LowVision => "low_vision",
            AccessibilityProfile::DeafHardOfHearing => "deaf_hard_of_hearing",
            AccessibilityProfile::MotorImpaired => "motor_impaired",
            AccessibilityProfile::CognitiveAccessible => "cognitive_accessible",
            AccessibilityProfile::Custom(s) => s.as_str(),
        }
    }

    pub fn apply(&self, settings: &mut AccessibilitySettings) {
        let mut new_settings = match self {
            AccessibilityProfile::Standard => AccessibilitySettings::default(),
            AccessibilityProfile::ColorblindFriendly => {
                let mut s = AccessibilitySettings::default();
                s.colorblind.colorblind_type = ColorblindType::Deuteranopia;
                s.colorblind.correction_mode = ColorblindCorrectionMode::Correct;
                s.colorblind.use_colorblind_palette = true;
                s
            }
            AccessibilityProfile::HighContrast => {
                let mut s = AccessibilitySettings::default();
                s.colorblind.contrast_mode = ContrastMode::UltraHigh;
                s.visual.high_contrast_ui = true;
                s.visual.high_contrast_color_scheme = HighContrastColorScheme::Dark;
                s.colorblind.colorblind_type = ColorblindType::None;
                s.colorblind.correction_mode = ColorblindCorrectionMode::None;
                s.visual.bold_text = true;
                s.visual.large_cursor = true;
                s.visual.cursor_scale = 1.5;
                s
            }
            AccessibilityProfile::LowVision => {
                let mut s = AccessibilitySettings::default();
                s.visual.text_scale_factor = 1.5;
                s.visual.ui_scale_factor = 1.3;
                s.visual.high_contrast_ui = true;
                s.visual.high_contrast_color_scheme = HighContrastColorScheme::Dark;
                s.visual.screen_reader_mode = true;
                s.visual.bold_text = true;
                s.visual.large_cursor = true;
                s.visual.cursor_scale = 1.5;
                s.screen_reader.enabled = true;
                s.subtitles.enabled = true;
                s.subtitles.style.font_size = SubtitleSize::ExtraLarge;
                s.subtitles.font_scale = 1.3;
                s
            }
            AccessibilityProfile::DeafHardOfHearing => {
                let mut s = AccessibilitySettings::default();
                s.subtitles.enabled = true;
                s.subtitles.caption_style.show_speaker_labels = true;
                s.subtitles.caption_style.show_sound_descriptions = true;
                s.audio.visual_indicators = vec![VisualAudioIndicator {
                    source_id: "default".to_string(),
                    position: Some([0.0, 0.0, 0.0]),
                    min_volume: 0.01,
                    max_volume: 1.0,
                    indicator_color: Color::rgb(1.0, 0.9, 0.0),
                    show_waveform: true,
                    show_spectrum: true,
                }];
                s.audio.show_visual_indicators = true;
                s.audio.subtitles_for_audio_cues = true;
                s.audio.channel_mode = AudioChannelMode::Stereo;
                s
            }
            AccessibilityProfile::MotorImpaired => {
                let mut s = AccessibilitySettings::default();
                s.input.one_handed_mode = true;
                s.input.one_handed_layout = crate::OneHandedLayout::Right;
                s.input.button_behavior = crate::ButtonBehavior::Toggle;
                s.input.auto_repeat_enabled = true;
                s.input.sticky_keys = true;
                s.input.slow_keys = true;
                s.input.slow_keys_delay = Duration::from_millis(800);
                s.input.bounce_keys = true;
                s.input.bounce_keys_delay = Duration::from_millis(500);
                s.input.motion_sickness_reduction = true;
                s.input.snap_turn_enabled = true;
                s.input.snap_turn_angle = 30.0;
                s.input.vignette_intensity = 0.8;
                s.input.teleport_movement = true;
                s.input.comfort_mode = true;
                s
            }
            AccessibilityProfile::CognitiveAccessible => {
                let mut s = AccessibilitySettings::default();
                s.visual.text_scale_factor = 1.2;
                s.visual.ui_scale_factor = 1.2;
                s.visual.high_contrast_ui = true;
                s.visual.high_contrast_color_scheme = HighContrastColorScheme::Light;
                s.visual.reduced_motion = ReducedMotionLevel::All;
                s.visual.reduced_transparency = true;
                s.visual.bold_text = true;
                s.subtitles.enabled = true;
                s.subtitles.font_scale = 1.2;
                s.screen_reader.enabled = true;
                s.screen_reader.text_to_speech.verbosity = SpeechVerbosity::Normal;
                s.screen_reader.text_to_speech.auto_read_navigation = true;
                s
            }
            AccessibilityProfile::Custom(_) => AccessibilitySettings::default(),
        };
        *settings = new_settings;
    }
}

// ============================================================================
// Accessibility Manager
// ============================================================================

#[derive(Debug, Clone)]
pub struct AccessibilityManager {
    pub settings: AccessibilitySettings,
    pub active_profile: AccessibilityProfile,
    pub colorblind_filters: HashMap<ColorblindType, ColorblindSimulationFilter>,
    pub current_subtitle_track: Option<SubtitleTrack>,
    pub pending_screen_reader_announcements: Vec<ScreenReaderEvent>,
    pub saved_profiles: HashMap<String, AccessibilitySettings>,
}

impl Default for AccessibilityManager {
    fn default() -> Self {
        let mut colorblind_filters = HashMap::new();
        colorblind_filters.insert(
            ColorblindType::Protanopia,
            ColorblindSimulationFilter::new(ColorblindType::Protanopia),
        );
        colorblind_filters.insert(
            ColorblindType::Deuteranopia,
            ColorblindSimulationFilter::new(ColorblindType::Deuteranopia),
        );
        colorblind_filters.insert(
            ColorblindType::Tritanopia,
            ColorblindSimulationFilter::new(ColorblindType::Tritanopia),
        );
        colorblind_filters.insert(
            ColorblindType::Achromatopsia,
            ColorblindSimulationFilter::new(ColorblindType::Achromatopsia),
        );
        Self {
            settings: AccessibilitySettings::default(),
            active_profile: AccessibilityProfile::Standard,
            colorblind_filters,
            current_subtitle_track: None,
            pending_screen_reader_announcements: Vec::new(),
            saved_profiles: HashMap::new(),
        }
    }
}

impl AccessibilityManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_profile(mut self, profile: AccessibilityProfile) -> Self {
        self.apply_profile(profile);
        self
    }

    pub fn apply_profile(&mut self, profile: AccessibilityProfile) {
        profile.apply(&mut self.settings);
        self.active_profile = profile;
        self.settings.profile_name = profile.as_str().to_string();
    }

    pub fn set_colorblind_type(&mut self, colorblind_type: ColorblindType) {
        self.settings.colorblind.colorblind_type = colorblind_type;
        self.active_profile = AccessibilityProfile::Custom("custom".to_string());
        self.settings.profile_name = "custom".to_string();
    }

    pub fn set_contrast_mode(&mut self, contrast_mode: ContrastMode) {
        self.settings.colorblind.contrast_mode = contrast_mode;
        self.active_profile = AccessibilityProfile::Custom("custom".to_string());
        self.settings.profile_name = "custom".to_string();
    }

    pub fn enable_subtitles(&mut self, enabled: bool) {
        self.settings.subtitles.enabled = enabled;
    }

    pub fn set_subtitle_language(&mut self, language: SubtitleLanguage) {
        self.settings.subtitles.preferred_language = language;
        if let Some(track) = self
            .settings
            .subtitles
            .tracks
            .iter()
            .find(|t| t.language == language)
        {
            self.current_subtitle_track = Some(track.clone());
        }
    }

    pub fn set_subtitle_style(&mut self, style: SubtitleStyle) {
        self.settings.subtitles.style = style;
    }

    pub fn set_subtitle_position(&mut self, position: SubtitlePosition) {
        self.settings.subtitles.position = position;
    }

    pub fn get_subtitle_track(&self, language: SubtitleLanguage) -> Option<&SubtitleTrack> {
        self.settings
            .subtitles
            .tracks
            .iter()
            .find(|t| t.language == language)
    }

    pub fn get_subtitle_at_time(&self, time: Duration) -> Option<&SubtitleEntry> {
        if let Some(ref track) = self.current_subtitle_track {
            track.entries.iter().find(|e| e.is_active(time))
        } else {
            self.settings
                .subtitles
                .tracks
                .iter()
                .find(|t| t.language == self.settings.subtitles.preferred_language)
                .and_then(|track| track.entries.iter().find(|e| e.is_active(time)))
        }
    }

    pub fn enable_screen_reader(&mut self, enabled: bool) {
        self.settings.screen_reader.enabled = enabled;
    }

    pub fn queue_announcement(&mut self, event: ScreenReaderEvent) {
        self.pending_screen_reader_announcements.push(event);
    }

    pub fn get_next_announcement(&mut self) -> Option<ScreenReaderEvent> {
        if self.pending_screen_reader_announcements.is_empty() {
            None
        } else {
            Some(self.pending_screen_reader_announcements.remove(0))
        }
    }

    pub fn set_input_accessibility(&mut self, settings: InputAccessibilitySettings) {
        self.settings.input = settings;
    }

    pub fn set_visual_accessibility(&mut self, settings: VisualAccessibilitySettings) {
        self.settings.visual = settings;
    }

    pub fn set_audio_accessibility(&mut self, settings: AudioAccessibilitySettings) {
        self.settings.audio = settings;
    }

    pub fn set_text_scale(&mut self, scale: f32) {
        self.settings.visual.text_scale_factor = scale.clamp(0.5, 3.0);
        self.settings.visual.text_scaling_mode = TextScalingMode::Custom(scale);
    }

    pub fn set_ui_scale(&mut self, scale: f32) {
        self.settings.visual.ui_scale_factor = scale.clamp(0.5, 3.0);
        self.settings.visual.ui_scaling_mode = UIScalingMode::Custom(scale);
    }

    pub fn set_reduced_motion(&mut self, level: ReducedMotionLevel) {
        self.settings.visual.reduced_motion = level;
    }

    pub fn set_motion_sickness_reduction(&mut self, enabled: bool) {
        self.settings.input.motion_sickness_reduction = enabled;
        if enabled {
            self.settings.input.snap_turn_enabled = true;
            self.settings.input.teleport_movement = true;
            self.settings.input.comfort_mode = true;
            self.settings.input.vignette_intensity = self.settings.input.vignette_intensity.max(0.5);
        }
    }

    pub fn set_audio_channel_mode(&mut self, mode: AudioChannelMode) {
        self.settings.audio.channel_mode = mode;
    }

    pub fn process_color_for_colorblind(&self, color: &Color) -> Color {
        let settings = &self.settings.colorblind;
        match settings.correction_mode {
            ColorblindCorrectionMode::None => *color,
            ColorblindCorrectionMode::Simulate => simulate_colorblind(
                color,
                settings.colorblind_type,
                settings.simulation_strength,
            ),
            ColorblindCorrectionMode::Correct => {
                if settings.use_colorblind_palette {
                    get_colorblind_friendly_color(color, settings.colorblind_type)
                } else {
                    simulate_colorblind(
                        color,
                        settings.colorblind_type,
                        settings.simulation_strength,
                    )
                }
            }
        }
    }

    pub fn process_color_for_contrast(&self, color: &Color) -> Color {
        let mode = self.settings.colorblind.contrast_mode;
        match mode {
            ContrastMode::Normal => *color,
            ContrastMode::High => {
                let lum = 0.299 * color.r + 0.587 * color.g + 0.114 * color.b;
                if lum > 0.5 {
                    Color::rgb(1.0, 1.0, 1.0)
                } else {
                    Color::rgb(1.0, 1.0, 0.0)
                }
            }
            ContrastMode::UltraHigh => {
                let lum = 0.299 * color.r + 0.587 * color.g + 0.114 * color.b;
                if lum > 0.5 {
                    Color::rgb(1.0, 1.0, 1.0)
                } else {
                    Color::rgb(0.0, 0.0, 0.0)
                }
            }
        }
    }

    pub fn process_color(&self, color: &Color) -> Color {
        let colorblind = self.process_color_for_colorblind(color);
        self.process_color_for_contrast(&colorblind)
    }

    pub fn get_text_scale(&self) -> f32 {
        match self.settings.visual.text_scaling_mode {
            TextScalingMode::System => self.settings.visual.text_scale_factor,
            TextScalingMode::Custom(scale) => scale,
        }
    }

    pub fn get_ui_scale(&self) -> f32 {
        match self.settings.visual.ui_scaling_mode {
            UIScalingMode::System => self.settings.visual.ui_scale_factor,
            UIScalingMode::Custom(scale) => scale,
        }
    }

    pub fn is_subtitle_enabled(&self) -> bool {
        self.settings.subtitles.enabled
    }

    pub fn is_screen_reader_enabled(&self) -> bool {
        self.settings.screen_reader.enabled
    }

    pub fn is_reduced_motion(&self) -> bool {
        self.settings.visual.reduced_motion != ReducedMotionLevel::None
    }

    pub fn is_motion_sickness_reduction(&self) -> bool {
        self.settings.input.motion_sickness_reduction
    }

    pub fn get_audio_channel_mode(&self) -> AudioChannelMode {
        self.settings.audio.channel_mode
    }

    pub fn save_profile(&mut self, name: impl Into<String>) {
        self.saved_profiles
            .insert(name.into(), self.settings.clone());
    }

    pub fn load_profile(&mut self, name: &str) -> bool {
        if let Some(settings) = self.saved_profiles.get(name) {
            self.settings = settings.clone();
            true
        } else {
            false
        }
    }

    pub fn has_saved_profile(&self, name: &str) -> bool {
        self.saved_profiles.contains_key(name)
    }

    pub fn saved_profile_names(&self) -> Vec<&String> {
        self.saved_profiles.keys().collect()
    }

    pub fn reset_to_default(&mut self) {
        self.settings = AccessibilitySettings::default();
        self.active_profile = AccessibilityProfile::Standard;
        self.current_subtitle_track = None;
        self.pending_screen_reader_announcements.clear();
    }

    pub fn is_enabled(&self) -> bool {
        self.settings.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.settings.enabled = enabled;
    }
}
