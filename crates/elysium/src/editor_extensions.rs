//! editor_extensions.rs — Editör uzantıları paneli.
//!
//! 10 yeni motor özelliği paneli ekler:
//! 1. Audio Mixer Panel
//! 2. Save Game Manager Panel
//! 3. Input Rebind Panel
//! 4. Localization Panel
//! 5. VR Settings Panel
//! 6. Mod Manager Panel
//! 7. Debug Panel
//! 8. AI/Navigation Panel
//! 9. Settings Panel
//! 10. Profiler Panel
//!
//! Her panel ayrı bir yapı taşır, birlikte EditorExtensions altında toplanır.
//! Immediate-mode UI, software framebuffer üzerinde çizer.

use crate::editor_ui::{
    rect, border,
    COL_PANEL, COL_HEADER, COL_BUTTON, COL_BORDER, COL_TEXT, COL_TEXT_DIM, COL_TEXT_GREEN, COL_SELECT,
};
use crate::renderer::SoftwareRenderer;
use crate::editor_ui::UiFont;
use crate::editor_ui::UiButton;
use crate::editor_ui::UiLayout;

// ═══════════════════════════════════════════════════════════ 1. SES MİKSERİ PANELİ

#[derive(Debug, Clone, Default)]
pub struct AudioMixerPanel {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub voice_volume: f32,
    pub master_mute: bool,
    pub music_mute: bool,
    pub sfx_mute: bool,
    pub voice_mute: bool,
    pub output_device: usize,
    pub output_devices: Vec<String>,
    pub visualizer_bars: [f32; 16],
}

impl AudioMixerPanel {
    pub fn new() -> Self {
        Self {
            master_volume: 0.8,
            music_volume: 0.7,
            sfx_volume: 0.9,
            voice_volume: 1.0,
            master_mute: false,
            music_mute: false,
            sfx_mute: false,
            voice_mute: false,
            output_device: 0,
            output_devices: vec!["Default".into(), "Headphones".into(), "Speakers".into()],
            visualizer_bars: [0.0; 16],
        }
    }

    pub fn update(&mut self, dt: f32) {
        for b in &mut self.visualizer_bars {
            *b = (b * 0.85 + ((rand_f32() * 0.3 + 0.1) * self.master_volume * 0.8)).clamp(0.0, 1.0);
        }
    }
}

// ═══════════════════════════════════════════════════════════ 2. KAYIT YÖNETİCİSİ PANELİ

#[derive(Debug, Clone, Default)]
pub struct SaveGameManagerPanel {
    pub save_slots: Vec<SaveSlot>,
    pub auto_save: bool,
    pub auto_save_interval: f32,
    pub auto_save_timer: f32,
    pub cloud_save_enabled: bool,
    pub selected_slot: Option<usize>,
    pub new_slot_name: String,
}

#[derive(Debug, Clone, Default)]
pub struct SaveSlot {
    pub name: String,
    pub level: String,
    pub playtime: String,
    pub timestamp: String,
    pub thumbnail: Option<Vec<u8>>,
    pub cloud_synced: bool,
}

impl SaveGameManagerPanel {
    pub fn new() -> Self {
        let mut panel = Self::default();
        panel.save_slots = vec![
            SaveSlot {
                name: "Slot 1".into(),
                level: "MainMenu".into(),
                playtime: "00:00".into(),
                timestamp: "2025-01-01 00:00".into(),
                cloud_synced: true,
                ..Default::default()
            },
            SaveSlot {
                name: "Slot 2".into(),
                level: "Level_01".into(),
                playtime: "12:34".into(),
                timestamp: "2025-01-02 14:22".into(),
                cloud_synced: false,
                ..Default::default()
            },
        ];
        panel
    }
}

// ═══════════════════════════════════════════════════════════ 3. GİRDİ YENİ ATAMA PANELİ

#[derive(Debug, Clone, Default)]
pub struct InputRebindPanel {
    pub actions: Vec<InputAction>,
    pub current_action: Option<usize>,
    pub rebinding: bool,
    pub rebind_action_idx: Option<usize>,
    pub preset: InputPreset,
    pub conflict_warning: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct InputAction {
    pub name: String,
    pub binding: String,
    pub alternate: Option<String>,
    pub category: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum InputPreset {
    #[default]
    Default,
    Accessibility,
    LeftHanded,
    OneHanded,
}

impl InputRebindPanel {
    pub fn new() -> Self {
        let mut panel = Self::default();
        panel.actions = vec![
            InputAction { name: "Move Forward".into(), binding: "W".into(), alternate: Some("Up".into()), category: "Movement".into() },
            InputAction { name: "Move Backward".into(), binding: "S".into(), alternate: Some("Down".into()), category: "Movement".into() },
            InputAction { name: "Move Left".into(), binding: "A".into(), alternate: None, category: "Movement".into() },
            InputAction { name: "Move Right".into(), binding: "D".into(), alternate: None, category: "Movement".into() },
            InputAction { name: "Jump".into(), binding: "Space".into(), alternate: None, category: "Movement".into() },
            InputAction { name: "Sprint".into(), binding: "LeftShift".into(), alternate: None, category: "Movement".into() },
            InputAction { name: "Attack".into(), binding: "LeftMouse".into(), alternate: None, category: "Combat".into() },
            InputAction { name: "Block".into(), binding: "RightMouse".into(), alternate: None, category: "Combat".into() },
            InputAction { name: "Interact".into(), binding: "E".into(), alternate: None, category: "General".into() },
            InputAction { name: "Inventory".into(), binding: "Tab".into(), alternate: None, category: "General".into() },
        ];
        panel
    }
}

// ═══════════════════════════════════════════════════════════ 4. YERELLEŞTİRME PANELİ

#[derive(Debug, Clone, Default)]
pub struct LocalizationPanel {
    pub current_language: usize,
    pub languages: Vec<String>,
    pub translation_progress: Vec<f32>,
    pub missing_count: usize,
    pub extracting: bool,
    pub strings_extracted: u32,
}

impl LocalizationPanel {
    pub fn new() -> Self {
        let languages = vec![
            "English".into(),
            "Turkish".into(),
            "German".into(),
            "French".into(),
            "Spanish".into(),
            "Japanese".into(),
        ];
        let progress = vec![1.0, 0.92, 0.78, 0.85, 0.70, 0.45];
        Self {
            current_language: 0,
            languages,
            translation_progress: progress,
            missing_count: 234,
            extracting: false,
            strings_extracted: 0,
        }
    }
}

// ═══════════════════════════════════════════════════════════ 5. VR AYARLARI PANELİ

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum VRRuntime {
    OpenXR,
    #[default]
    Oculus,
    SteamVR,
    Mock,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum VRTrackingSpace {
    #[default]
    Standing,
    Seated,
    RoomScale,
}

#[derive(Debug, Clone, Default)]
pub struct VRSettingsPanel {
    pub vr_enabled: bool,
    pub runtime: VRRuntime,
    pub render_scale: f32,
    pub snap_turn_angle: f32,
    pub motion_sickness_reduction: bool,
    pub tracking_space: VRTrackingSpace,
    pub foveated_rendering: bool,
}

impl VRSettingsPanel {
    pub fn new() -> Self {
        Self {
            vr_enabled: false,
            runtime: VRRuntime::Oculus,
            render_scale: 1.0,
            snap_turn_angle: 30.0,
            motion_sickness_reduction: true,
            tracking_space: VRTrackingSpace::Standing,
            foveated_rendering: true,
        }
    }
}

// ═══════════════════════════════════════════════════════════ 6. MOD YÖNETİCİSİ PANELİ

#[derive(Debug, Clone, Default)]
pub struct ModManagerPanel {
    pub mods: Vec<ModEntry>,
    pub load_order_changed: bool,
    pub selected_mod: Option<usize>,
    pub filtering_enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ModEntry {
    pub name: String,
    pub version: String,
    pub author: String,
    pub enabled: bool,
    pub load_order: i32,
    pub dependencies: Vec<String>,
    pub has_conflict: bool,
    pub description: String,
}

impl ModManagerPanel {
    pub fn new() -> Self {
        let mut panel = Self::default();
        panel.mods = vec![
            ModEntry {
                name: "Enhanced Graphics".into(),
                version: "2.1.0".into(),
                author: "Community".into(),
                enabled: true,
                load_order: 0,
                dependencies: vec!["Core".into()],
                has_conflict: false,
                description: "Yüksek kaliteli dokular ve efektler".into(),
            },
            ModEntry {
                name: "Sound Overhaul".into(),
                version: "1.5.3".into(),
                author: "AudioTeam".into(),
                enabled: true,
                load_order: 1,
                dependencies: vec!["Core".into(), "Enhanced Graphics".into()],
                has_conflict: false,
                description: "Gelişmiş ses sistemi".into(),
            },
            ModEntry {
                name: "QoL Pack".into(),
                version: "3.0.0".into(),
                author: "Modder".into(),
                enabled: false,
                load_order: 2,
                dependencies: vec![],
                has_conflict: true,
                description: "Yaşam kalitesi iyileştirmeleri".into(),
            },
        ];
        panel
    }
}

// ═══════════════════════════════════════════════════════════ 7. HATA AYIKLAMA PANELİ

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum DebugRenderMode {
    #[default]
    None,
    Wireframe,
    Normals,
    CollisionShapes,
    AABB,
    SkeletonOverlay,
    LightComplexity,
    MetalRoughness,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum DebugCategory {
    #[default]
    Off,
    Minimal,
    Normal,
    Verbose,
}

#[derive(Debug, Clone, Default)]
pub struct DebugPanel {
    pub render_mode: DebugRenderMode,
    pub physics_debug: DebugCategory,
    pub ai_debug: DebugCategory,
    pub rendering_debug: DebugCategory,
    pub performance_debug: DebugCategory,
    pub show_performance_stats: bool,
    pub show_frame_graph: bool,
    pub screenshot_requested: bool,
    pub screenshot_path: String,
}

impl DebugPanel {
    pub fn new() -> Self {
        Self {
            render_mode: DebugRenderMode::None,
            physics_debug: DebugCategory::Off,
            ai_debug: DebugCategory::Off,
            rendering_debug: DebugCategory::Off,
            performance_debug: DebugCategory::Off,
            show_performance_stats: true,
            show_frame_graph: false,
            screenshot_requested: false,
            screenshot_path: "screenshot.png".into(),
        }
    }
}

// ═══════════════════════════════════════════════════════════ 8. YAPAY ZEKA/GİDİŞ PANELİ

#[derive(Debug, Clone, Default)]
pub struct AINavigationPanel {
    pub show_navmesh: bool,
    pub show_paths: bool,
    pub show_behavior_tree: bool,
    pub show_perception_cones: bool,
    pub path_agent_id: Option<usize>,
    pub selected_agent: Option<usize>,
}

impl AINavigationPanel {
    pub fn new() -> Self {
        Self {
            show_navmesh: true,
            show_paths: false,
            show_behavior_tree: false,
            show_perception_cones: false,
            path_agent_id: None,
            selected_agent: None,
        }
    }
}

// ═══════════════════════════════════════════════════════════ 9. AYARLAR PANELİ

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum RenderQuality {
    #[default]
    Low,
    Medium,
    High,
    Ultra,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum ShadowQuality {
    Off,
    #[default]
    Low,
    Medium,
    High,
    Ultra,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum TextureQuality {
    #[default]
    Full,
    Half,
    Quarter,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum AntiAliasingMode {
    Off,
    #[default]
    FXAA,
    TAA,
    MSAA2x,
    MSAA4x,
    MSAA8x,
}

#[derive(Debug, Clone, Default)]
pub struct SettingsPanel {
    pub render_quality: RenderQuality,
    pub shadow_quality: ShadowQuality,
    pub texture_quality: TextureQuality,
    pub anti_aliasing: AntiAliasingMode,
    pub vsync: bool,
    pub fullscreen: bool,
    pub resolution_scale: f32,
    pub anisotropic_filtering: u32,
}

impl SettingsPanel {
    pub fn new() -> Self {
        Self {
            render_quality: RenderQuality::High,
            shadow_quality: ShadowQuality::Medium,
            texture_quality: TextureQuality::Full,
            anti_aliasing: AntiAliasingMode::FXAA,
            vsync: true,
            fullscreen: false,
            resolution_scale: 1.0,
            anisotropic_filtering: 4,
        }
    }
}

// ═══════════════════════════════════════════════════════════ 10. PROFİLEYİCİ PANELİ

#[derive(Debug, Clone, Default)]
pub struct ProfilerPanel {
    pub frame_times: [f32; 64],
    pub frame_time_index: usize,
    pub frame_time_sum: f32,
    pub total_frames: u64,
    pub draw_calls: u32,
    pub triangle_count: u32,
    pub texture_memory: u64,
    pub buffer_memory: u64,
    pub visible: bool,
    pub show_percentiles: bool,
}

impl ProfilerPanel {
    pub fn new() -> Self {
        Self {
            frame_times: [0.0; 64],
            frame_time_index: 0,
            frame_time_sum: 0.0,
            total_frames: 0,
            draw_calls: 0,
            triangle_count: 0,
            texture_memory: 0,
            buffer_memory: 0,
            visible: true,
            show_percentiles: true,
        }
    }

    pub fn push_frame(&mut self, frame_time_ms: f32, draw_calls: u32, triangles: u32) {
        let idx = self.frame_time_index % self.frame_times.len();
        self.frame_time_sum -= self.frame_times[idx];
        self.frame_time_sum += frame_time_ms;
        self.frame_times[idx] = frame_time_ms;
        self.frame_time_index += 1;
        self.total_frames += 1;
        self.draw_calls = draw_calls;
        self.triangle_count = triangles;
    }

    pub fn avg_frame_time(&self) -> f32 {
        let len = self.frame_time_index.min(self.frame_times.len());
        if len == 0 { return 0.0; }
        self.frame_time_sum / len as f32
    }

    pub fn fps(&self) -> f32 {
        let avg = self.avg_frame_time();
        if avg < 0.001 { return 0.0; }
        1000.0 / avg
    }

    pub fn min_frame_time(&self) -> f32 {
        self.frame_times[..self.frame_time_index.min(self.frame_times.len())].iter().fold(f32::INFINITY, |a, &b| a.min(b))
    }

    pub fn max_frame_time(&self) -> f32 {
        self.frame_times[..self.frame_time_index.min(self.frame_times.len())].iter().fold(0.0f32, |a, &b| a.max(b))
    }
}

// ═══════════════════════════════════════════════════════════ ANA YAPI

#[derive(Clone, Default)]
pub struct EditorExtensions {
    pub audio_mixer: AudioMixerPanel,
    pub save_manager: SaveGameManagerPanel,
    pub input_rebind: InputRebindPanel,
    pub localization: LocalizationPanel,
    pub vr_settings: VRSettingsPanel,
    pub mod_manager: ModManagerPanel,
    pub debug_panel: DebugPanel,
    pub ai_nav: AINavigationPanel,
    pub settings: SettingsPanel,
    pub profiler: ProfilerPanel,
    pub panel_visible: [bool; 10],
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Eq, Hash)]
pub enum PanelId {
    #[default]
    AudioMixer,
    SaveManager,
    InputRebind,
    Localization,
    VRSettings,
    ModManager,
    Debug,
    AINav,
    Settings,
    Profiler,
}

impl EditorExtensions {
    pub fn new() -> Self {
        Self {
            audio_mixer: AudioMixerPanel::new(),
            save_manager: SaveGameManagerPanel::new(),
            input_rebind: InputRebindPanel::new(),
            localization: LocalizationPanel::new(),
            vr_settings: VRSettingsPanel::new(),
            mod_manager: ModManagerPanel::new(),
            debug_panel: DebugPanel::new(),
            ai_nav: AINavigationPanel::new(),
            settings: SettingsPanel::new(),
            profiler: ProfilerPanel::new(),
            panel_visible: [false; 10],
        }
    }

    /// Her frame çağrılır — görselleştirme ve zamanlayıcıları günceller.
    pub fn update(&mut self, dt: f32) {
        self.audio_mixer.update(dt);

        if self.save_manager.auto_save {
            self.save_manager.auto_save_timer += dt;
            if self.save_manager.auto_save_timer >= self.save_manager.auto_save_interval {
                self.save_manager.auto_save_timer = 0.0;
            }
        }
    }

    /// Tüm uzantı panellerini çizer. Her panelin başlık satırında toggle butonu vardır.
    pub fn draw_extensions_panels(
        &self,
        fb: &mut SoftwareRenderer,
        font: &mut UiFont,
        right_panel: [i32; 4],
        hover_btn: Option<&str>,
        ui: &mut UiLayout,
    ) {
        let rx = right_panel[0];
        let ry = right_panel[1];
        let rw = right_panel[2];

        let bx = rx + 8;
        let bw = rw - 16;
        let bh = 22;
        let mut by = ry + 28;

        let panels = [
            ("Audio Mixer", PanelId::AudioMixer, "audio_mixer"),
            ("Save Manager", PanelId::SaveManager, "save_manager"),
            ("Input Rebind", PanelId::InputRebind, "input_rebind"),
            ("Localization", PanelId::Localization, "localization"),
            ("VR Settings", PanelId::VRSettings, "vr_settings"),
            ("Mod Manager", PanelId::ModManager, "mod_manager"),
            ("Debug", PanelId::Debug, "debug"),
            ("AI / Nav", PanelId::AINav, "ai_nav"),
            ("Settings", PanelId::Settings, "settings"),
            ("Profiler", PanelId::Profiler, "profiler"),
        ];

        for (name, id, btn_id) in panels {
            let visible = self.panel_visible[id as usize];
            let hovered = hover_btn == Some(btn_id);
            let col = if visible { COL_SELECT } else { COL_BUTTON };
            let c = if hovered { lighten(col) } else { col };
            rect(fb, [bx, by, bw, bh], c);
            border(fb, [bx, by, bw, bh], COL_BORDER);
            let label = if visible { format!("{}", name) } else { format!("{}", name) };
            font.draw_text(fb, bx + 8, by + 5, 13, COL_TEXT, &label);
            ui.buttons.push(UiButton { id: btn_id.into(), rect: [bx, by, bw, bh] });
            by += bh + 4;

            if visible {
                by = self.draw_panel(fb, font, id, bx, by, bw, hover_btn, ui);
                by += 6;
            }
        }
    }

    /// Belirli bir paneli çizer.
    fn draw_panel(
        &self,
        fb: &mut SoftwareRenderer,
        font: &mut UiFont,
        panel: PanelId,
        bx: i32,
        mut by: i32,
        bw: i32,
        hover_btn: Option<&str>,
        ui: &mut UiLayout,
    ) -> i32 {
        match panel {
            PanelId::AudioMixer => self.draw_audio_mixer_panel(fb, font, bx, &mut by, bw, hover_btn, ui),
            PanelId::SaveManager => self.draw_save_manager_panel(fb, font, bx, &mut by, bw, hover_btn, ui),
            PanelId::InputRebind => self.draw_input_rebind_panel(fb, font, bx, &mut by, bw, hover_btn, ui),
            PanelId::Localization => self.draw_localization_panel(fb, font, bx, &mut by, bw, hover_btn, ui),
            PanelId::VRSettings => self.draw_vr_settings_panel(fb, font, bx, &mut by, bw, hover_btn, ui),
            PanelId::ModManager => self.draw_mod_manager_panel(fb, font, bx, &mut by, bw, hover_btn, ui),
            PanelId::Debug => self.draw_debug_panel(fb, font, bx, &mut by, bw, hover_btn, ui),
            PanelId::AINav => self.draw_ai_nav_panel(fb, font, bx, &mut by, bw, hover_btn, ui),
            PanelId::Settings => self.draw_settings_panel(fb, font, bx, &mut by, bw, hover_btn, ui),
            PanelId::Profiler => self.draw_profiler_panel(fb, font, bx, &mut by, bw, hover_btn, ui),
        }
    }

    // ───────────────────────────────────────────────────────── 1. AUDIO MIXER PANEL
    fn draw_audio_mixer_panel(
        &self,
        fb: &mut SoftwareRenderer,
        font: &mut UiFont,
        bx: i32,
        by: &mut i32,
        bw: i32,
        hover_btn: Option<&str>,
        ui: &mut UiLayout,
    ) -> i32 {
        let mixer = &self.audio_mixer;

        font.draw_text(fb, bx, *by, 12, COL_TEXT_DIM, "Output Device:");
        *by += 16;
        let dev_label = &mixer.output_devices[mixer.output_device % mixer.output_devices.len()];
        draw_btn(fb, font, bx, *by, bw, 18, dev_label, COL_BUTTON, hover_btn, "audio_output", ui);
        *by += 22;

        draw_volume_row(fb, font, bx, by, bw, "Master", mixer.master_volume, mixer.master_mute, "master_vol", hover_btn, ui);
        draw_volume_row(fb, font, bx, by, bw, "Music", mixer.music_volume, mixer.music_mute, "music_vol", hover_btn, ui);
        draw_volume_row(fb, font, bx, by, bw, "SFX", mixer.sfx_volume, mixer.sfx_mute, "sfx_vol", hover_btn, ui);
        draw_volume_row(fb, font, bx, by, bw, "Voice", mixer.voice_volume, mixer.voice_mute, "voice_vol", hover_btn, ui);

        *by += 6;
        font.draw_text(fb, bx, *by, 12, COL_TEXT_DIM, "Visualizer:");
        *by += 16;
        let bar_w = (bw - mixer.visualizer_bars.len() as i32 + 1) / mixer.visualizer_bars.len() as i32;
        for (i, &v) in mixer.visualizer_bars.iter().enumerate() {
            let h = (v * 36.0) as i32;
            let bar_x = bx + i as i32 * (bar_w + 1);
            let bar_col = if v > 0.7 { [255, 80, 80, 255] } else if v > 0.4 { [255, 200, 60, 255] } else { [100, 200, 120, 255] };
            rect(fb, [bar_x, *by + 36 - h, bar_w, h], bar_col);
        }
        *by += 42;
        *by
    }

    // ───────────────────────────────────────────────────────── 2. KAYIT YÖNETİCİSİ PANELİ
    fn draw_save_manager_panel(
        &self,
        fb: &mut SoftwareRenderer,
        font: &mut UiFont,
        bx: i32,
        by: &mut i32,
        bw: i32,
        hover_btn: Option<&str>,
        ui: &mut UiLayout,
    ) -> i32 {
        let save = &self.save_manager;

        font.draw_text(fb, bx, *by, 12, COL_TEXT, "Save Slots:");
        *by += 16;

        for (i, slot) in save.save_slots.iter().enumerate() {
            let slot_h = 36;
            let selected = save.selected_slot == Some(i);
            if selected {
                rect(fb, [bx, *by, bw, slot_h], COL_SELECT);
            } else {
                rect(fb, [bx, *by, bw, slot_h], [45, 45, 48, 255]);
            }
            border(fb, [bx, *by, bw, slot_h], COL_BORDER);
            font.draw_text(fb, bx + 6, *by + 4, 12, COL_TEXT, &slot.name);
            font.draw_text(fb, bx + 6, *by + 18, 11, COL_TEXT_DIM, &format!("{} | {}", slot.level, slot.playtime));
            let cloud_icon = if slot.cloud_synced { "cloud" } else { "local" };
            font.draw_text(fb, bx + bw - 36, *by + 10, 11, COL_TEXT_DIM, cloud_icon);
            ui.buttons.push(UiButton { id: format!("save_slot_{}", i), rect: [bx, *by, bw, slot_h] });
            *by += slot_h + 2;
        }

        *by += 2;
        font.draw_text(fb, bx, *by, 12, COL_TEXT_DIM, "New slot name:");
        *by += 16;
        draw_btn(fb, font, bx, *by, bw, 18, &save.new_slot_name, [45, 45, 48, 255], hover_btn, "save_new_name", ui);
        *by += 22;

        draw_btn(fb, font, bx, *by, bw, 18, "Save", COL_BUTTON, hover_btn, "save_save", ui);
        *by += 20;
        draw_btn(fb, font, bx, *by, bw, 18, "Load", COL_BUTTON, hover_btn, "save_load", ui);
        *by += 20;
        draw_btn(fb, font, bx, *by, bw, 18, "Delete", [110, 40, 40, 255], hover_btn, "save_delete", ui);
        *by += 20;

        let auto_label = if save.auto_save { "Auto-Save: ON" } else { "Auto-Save: OFF" };
        draw_btn(fb, font, bx, *by, bw, 18, auto_label, COL_BUTTON, hover_btn, "save_auto_toggle", ui);
        *by += 20;

        let cloud_label = if save.cloud_save_enabled { "Cloud Save: ON" } else { "Cloud Save: OFF" };
        draw_btn(fb, font, bx, *by, bw, 18, cloud_label, COL_BUTTON, hover_btn, "save_cloud_toggle", ui);
        *by += 20;
        *by
    }

    // ───────────────────────────────────────────────────────── 3. GİRDİ YENİ ATAMA PANELİ
    fn draw_input_rebind_panel(
        &self,
        fb: &mut SoftwareRenderer,
        font: &mut UiFont,
        bx: i32,
        by: &mut i32,
        bw: i32,
        hover_btn: Option<&str>,
        ui: &mut UiLayout,
    ) -> i32 {
        let rebind = &self.input_rebind;

        font.draw_text(fb, bx, *by, 12, COL_TEXT_DIM, "Preset:");
        *by += 16;
        let preset_labels = ["Default", "Accessibility", "LeftHanded", "OneHanded"];
        let pw = (bw - 10) / 4;
        for (i, lbl) in preset_labels.iter().enumerate() {
            let px = bx + i as i32 * pw;
            let hovered = hover_btn == Some(format!("input_preset_{}", i).as_str());
            let is_preset = rebind.preset as usize == i;
            let col = if is_preset { COL_SELECT } else { COL_BUTTON };
            let c = if hovered { lighten(col) } else { col };
            rect(fb, [px, *by, pw - 2, 18], c);
            border(fb, [px, *by, pw - 2, 18], COL_BORDER);
            font.draw_text(fb, px + 3, *by + 3, 11, COL_TEXT, lbl);
            ui.buttons.push(UiButton { id: format!("input_preset_{}", i), rect: [px, *by, pw - 2, 18] });
        }
        *by += 24;

        if let Some(ref warn) = rebind.conflict_warning {
            rect(fb, [bx, *by, bw, 18], [120, 60, 40, 255]);
            font.draw_text(fb, bx + 6, *by + 3, 11, COL_TEXT, &format!("Conflict: {}", warn));
            *by += 22;
        }

        font.draw_text(fb, bx, *by, 12, COL_TEXT, "Actions:");
        *by += 16;
        for (i, action) in rebind.actions.iter().enumerate() {
            let row_h = 20;
            let is_rebinding = rebind.rebinding && rebind.rebind_action_idx == Some(i);
            let label = if is_rebinding { "Press any key...".into() } else { format!("{}", action.binding) };
            let col = if is_rebinding { [80, 120, 200, 255] } else { COL_BUTTON };
            draw_btn(fb, font, bx, *by, bw, row_h, &label, col, hover_btn, &format!("rebind_{}", i), ui);
            *by += row_h + 2;
        }

        *by
    }

    // ───────────────────────────────────────────────────────── 4. YERELLEŞTİRME PANELİ
    fn draw_localization_panel(
        &self,
        fb: &mut SoftwareRenderer,
        font: &mut UiFont,
        bx: i32,
        by: &mut i32,
        bw: i32,
        hover_btn: Option<&str>,
        ui: &mut UiLayout,
    ) -> i32 {
        let loc = &self.localization;

        font.draw_text(fb, bx, *by, 12, COL_TEXT_DIM, "Language:");
        *by += 16;
        let selected_lang = &loc.languages[loc.current_language];
        draw_btn(fb, font, bx, *by, bw, 18, selected_lang, COL_BUTTON, hover_btn, "lang_select", ui);
        *by += 22;

        font.draw_text(fb, bx, *by, 12, COL_TEXT_DIM, "Translation Progress:");
        *by += 16;
        for (i, lang) in loc.languages.iter().enumerate() {
            let progress = loc.translation_progress[i];
            let filled = (bw as f32 * progress) as i32;
            rect(fb, [bx, *by, bw, 8], [50, 50, 53, 255]);
            rect(fb, [bx, *by, filled, 8], [100, 200, 140, 255]);
            font.draw_text(fb, bx + 2, *by - 4, 11, COL_TEXT, &format!("{} {:.0}%", lang, progress * 100.0));
            *by += 14;
        }

        *by += 4;
        font.draw_text(fb, bx, *by, 12, COL_TEXT, &format!("Missing: {}", loc.missing_count));
        *by += 16;
        draw_btn(fb, font, bx, *by, bw, 18, "Extract Strings", COL_BUTTON, hover_btn, "loc_extract", ui);
        *by += 22;
        *by
    }

    // ───────────────────────────────────────────────────────── 5. VR AYARLARI PANELİ
    fn draw_vr_settings_panel(
        &self,
        fb: &mut SoftwareRenderer,
        font: &mut UiFont,
        bx: i32,
        by: &mut i32,
        bw: i32,
        hover_btn: Option<&str>,
        ui: &mut UiLayout,
    ) -> i32 {
        let vr = &self.vr_settings;

        draw_btn(fb, font, bx, *by, bw, 18, if vr.vr_enabled { "VR: ON" } else { "VR: OFF" }, COL_BUTTON, hover_btn, "vr_toggle", ui);
        *by += 22;

        font.draw_text(fb, bx, *by, 12, COL_TEXT_DIM, "Runtime:");
        *by += 16;
        let runtime_labels = ["OpenXR", "Oculus", "SteamVR", "Mock"];
        let rw = (bw - 10) / 4;
        for (i, lbl) in runtime_labels.iter().enumerate() {
            let rx = bx + i as i32 * rw;
            let hovered = hover_btn == Some(format!("vr_runtime_{}", i).as_str());
            let is_runtime = match vr.runtime {
                VRRuntime::OpenXR if i == 0 => true,
                VRRuntime::Oculus if i == 1 => true,
                VRRuntime::SteamVR if i == 2 => true,
                VRRuntime::Mock if i == 3 => true,
                _ => false,
            };
            let col = if is_runtime { COL_SELECT } else { COL_BUTTON };
            let c = if hovered { lighten(col) } else { col };
            rect(fb, [rx, *by, rw - 2, 18], c);
            border(fb, [rx, *by, rw - 2, 18], COL_BORDER);
            font.draw_text(fb, rx + 3, *by + 3, 11, COL_TEXT, lbl);
            ui.buttons.push(UiButton { id: format!("vr_runtime_{}", i), rect: [rx, *by, rw - 2, 18] });
        }
        *by += 24;

        draw_slider_row(fb, font, bx, by, bw, &format!("Render Scale: {:.1f}x", vr.render_scale), vr.render_scale, 0.5, 2.0, "vr_render_scale", ui);
        draw_slider_row(fb, font, bx, by, bw, &format!("Snap Turn: {:.0f}", vr.snap_turn_angle), vr.snap_turn_angle, 15.0, 90.0, "vr_snap_turn", ui);

        let ms_label = if vr.motion_sickness_reduction { "Motion Sickness: ON" } else { "Motion Sickness: OFF" };
        draw_btn(fb, font, bx, *by, bw, 18, ms_label, COL_BUTTON, hover_btn, "vr_ms_reduction", ui);
        *by += 22;

        let fov_label = if vr.foveated_rendering { "Foveated: ON" } else { "Foveated: OFF" };
        draw_btn(fb, font, bx, *by, bw, 18, fov_label, COL_BUTTON, hover_btn, "vr_foveated", ui);
        *by += 22;
        *by
    }

    // ───────────────────────────────────────────────────────── 6. MOD YÖNETİCİSİ PANELİ
    fn draw_mod_manager_panel(
        &self,
        fb: &mut SoftwareRenderer,
        font: &mut UiFont,
        bx: i32,
        by: &mut i32,
        bw: i32,
        hover_btn: Option<&str>,
        ui: &mut UiLayout,
    ) -> i32 {
        let mods = &self.mod_manager;

        font.draw_text(fb, bx, *by, 12, COL_TEXT, "Mods:");
        *by += 16;

        for (i, m) in mods.mods.iter().enumerate() {
            let mh = 48;
            let selected = mods.selected_mod == Some(i);
            let conflict_col = if m.has_conflict { [160, 60, 60, 255] } else { [45, 45, 48, 255] };
            if selected {
                rect(fb, [bx, *by, bw, mh], COL_SELECT);
            } else {
                rect(fb, [bx, *by, bw, mh], conflict_col);
            }
            border(fb, [bx, *by, bw, mh], COL_BORDER);
            font.draw_text(fb, bx + 4, *by + 4, 12, COL_TEXT, &format!("{}", m.name));
            font.draw_text(fb, bx + 4, *by + 18, 11, COL_TEXT_DIM, &format!("v{} | Load: {}", m.version, m.load_order));
            font.draw_text(fb, bx + 4, *by + 30, 11, COL_TEXT_DIM, &format!("Deps: {}", if m.dependencies.is_empty() { "none".into() } else { m.dependencies.join(", ") }));
            ui.buttons.push(UiButton { id: format!("mod_item_{}", i), rect: [bx, *by, bw, mh] });
            *by += mh + 2;
        }

        *by += 2;
        draw_btn(fb, font, bx, *by, bw, 18, "Move Up", COL_BUTTON, hover_btn, "mod_up", ui);
        *by += 20;
        draw_btn(fb, font, bx, *by, bw, 18, "Move Down", COL_BUTTON, hover_btn, "mod_down", ui);
        *by += 20;
        draw_btn(fb, font, bx, *by, bw, 18, "Open Folder", COL_BUTTON, hover_btn, "mod_folder", ui);
        *by += 20;
        *by
    }

    // ───────────────────────────────────────────────────────── 7. HATA AYIKLAMA PANELİ
    fn draw_debug_panel(
        &self,
        fb: &mut SoftwareRenderer,
        font: &mut UiFont,
        bx: i32,
        by: &mut i32,
        bw: i32,
        hover_btn: Option<&str>,
        ui: &mut UiLayout,
    ) -> i32 {
        let dbg = &self.debug_panel;

        font.draw_text(fb, bx, *by, 12, COL_TEXT_DIM, "Render Mode:");
        *by += 16;
        let modes = ["None", "Wire", "Norm", "Coll", "AABB", "Skel", "Light", "Metal"];
        let mw = (bw - 10) / modes.len() as i32;
        for (i, m) in modes.iter().enumerate() {
            let mx = bx + i as i32 * mw;
            let hovered = hover_btn == Some(format!("debug_mode_{}", i).as_str());
            let mode_matches = match dbg.render_mode {
                DebugRenderMode::None if i == 0 => true,
                DebugRenderMode::Wireframe if i == 1 => true,
                DebugRenderMode::Normals if i == 2 => true,
                DebugRenderMode::CollisionShapes if i == 3 => true,
                DebugRenderMode::AABB if i == 4 => true,
                DebugRenderMode::SkeletonOverlay if i == 5 => true,
                DebugRenderMode::LightComplexity if i == 6 => true,
                DebugRenderMode::MetalRoughness if i == 7 => true,
                _ => false,
            };
            let col = if mode_matches { COL_SELECT } else { COL_BUTTON };
            let c = if hovered { lighten(col) } else { col };
            rect(fb, [mx, *by, mw - 2, 18], c);
            border(fb, [mx, *by, mw - 2, 18], COL_BORDER);
            font.draw_text(fb, mx + 2, *by + 3, 10, COL_TEXT, m);
            ui.buttons.push(UiButton { id: format!("debug_mode_{}", i), rect: [mx, *by, mw - 2, 18] });
        }
        *by += 24;

        let categories = [
            ("Physics", &dbg.physics_debug, "debug_cat_physics"),
            ("AI", &dbg.ai_debug, "debug_cat_ai"),
            ("Render", &dbg.rendering_debug, "debug_cat_rendering"),
            ("Perf", &dbg.performance_debug, "debug_cat_perf"),
        ];
        let cw = (bw - 6) / 2;
        for (i, (name, cat_val, cat_id)) in categories.iter().enumerate() {
            let cx = bx + (i % 2) as i32 * (cw + 3);
            let cy = *by + (i / 2) as i32 * 20;
            let label = format!("{}: {:?}", name, cat_val);
            draw_btn(fb, font, cx, cy, cw, 18, &label, COL_BUTTON, hover_btn, cat_id, ui);
        }
        *by += 44;

        let perf_label = if dbg.show_performance_stats { "Perf Stats: ON" } else { "Perf Stats: OFF" };
        draw_btn(fb, font, bx, *by, bw, 18, perf_label, COL_BUTTON, hover_btn, "debug_perf_toggle", ui);
        *by += 22;
        let fg_label = if dbg.show_frame_graph { "Frame Graph: ON" } else { "Frame Graph: OFF" };
        draw_btn(fb, font, bx, *by, bw, 18, fg_label, COL_BUTTON, hover_btn, "debug_fg_toggle", ui);
        *by += 22;
        draw_btn(fb, font, bx, *by, bw, 18, "Screenshot", COL_BUTTON, hover_btn, "debug_screenshot", ui);
        *by += 22;
        *by
    }

    // ───────────────────────────────────────────────────────── 8. YAPAY ZEKA/GİDİŞ PANELİ
    fn draw_ai_nav_panel(
        &self,
        fb: &mut SoftwareRenderer,
        font: &mut UiFont,
        bx: i32,
        by: &mut i32,
        bw: i32,
        hover_btn: Option<&str>,
        ui: &mut UiLayout,
    ) -> i32 {
        let ai = &self.ai_nav;

        let nav_label = if ai.show_navmesh { "NavMesh: ON" } else { "NavMesh: OFF" };
        draw_btn(fb, font, bx, *by, bw, 18, nav_label, COL_BUTTON, hover_btn, "ai_navmesh", ui);
        *by += 22;
        let path_label = if ai.show_paths { "Paths: ON" } else { "Paths: OFF" };
        draw_btn(fb, font, bx, *by, bw, 18, path_label, COL_BUTTON, hover_btn, "ai_paths", ui);
        *by += 22;
        let bt_label = if ai.show_behavior_tree { "Behavior Tree: ON" } else { "Behavior Tree: OFF" };
        draw_btn(fb, font, bx, *by, bw, 18, bt_label, COL_BUTTON, hover_btn, "ai_behavior_tree", ui);
        *by += 22;
        let perception_label = if ai.show_perception_cones { "Perception: ON" } else { "Perception: OFF" };
        draw_btn(fb, font, bx, *by, bw, 18, perception_label, COL_BUTTON, hover_btn, "ai_perception", ui);
        *by += 22;
        *by
    }

    // ───────────────────────────────────────────────────────── 9. AYARLAR PANELİ
    fn draw_settings_panel(
        &self,
        fb: &mut SoftwareRenderer,
        font: &mut UiFont,
        bx: i32,
        by: &mut i32,
        bw: i32,
        hover_btn: Option<&str>,
        ui: &mut UiLayout,
    ) -> i32 {
        let set = &self.settings;

        font.draw_text(fb, bx, *by, 12, COL_TEXT_DIM, "Render Quality:");
        *by += 16;
        let quality_labels = ["Low", "Med", "High", "Ultra"];
        let qw = (bw - 10) / 4;
        for (i, ql) in quality_labels.iter().enumerate() {
            let qx = bx + i as i32 * qw;
            let hovered = hover_btn == Some(format!("quality_{}", i).as_str());
            let is_q = match set.render_quality {
                RenderQuality::Low if i == 0 => true,
                RenderQuality::Medium if i == 1 => true,
                RenderQuality::High if i == 2 => true,
                RenderQuality::Ultra if i == 3 => true,
                _ => false,
            };
            let col = if is_q { COL_SELECT } else { COL_BUTTON };
            let c = if hovered { lighten(col) } else { col };
            rect(fb, [qx, *by, qw - 2, 18], c);
            border(fb, [qx, *by, qw - 2, 18], COL_BORDER);
            font.draw_text(fb, qx + 4, *by + 3, 11, COL_TEXT, ql);
            ui.buttons.push(UiButton { id: format!("quality_{}", i), rect: [qx, *by, qw - 2, 18] });
        }
        *by += 24;

        font.draw_text(fb, bx, *by, 12, COL_TEXT_DIM, "Shadow Quality:");
        *by += 16;
        let shadow_labels = ["Off", "Low", "Med", "High", "Ultra"];
        let sw = (bw - 12) / shadow_labels.len() as i32;
        for (i, sl) in shadow_labels.iter().enumerate() {
            let sx = bx + i as i32 * sw;
            let hovered = hover_btn == Some(format!("shadow_{}", i).as_str());
            let is_s = match set.shadow_quality {
                ShadowQuality::Off if i == 0 => true,
                ShadowQuality::Low if i == 1 => true,
                ShadowQuality::Medium if i == 2 => true,
                ShadowQuality::High if i == 3 => true,
                ShadowQuality::Ultra if i == 4 => true,
                _ => false,
            };
            let col = if is_s { COL_SELECT } else { COL_BUTTON };
            let c = if hovered { lighten(col) } else { col };
            rect(fb, [sx, *by, sw - 2, 18], c);
            border(fb, [sx, *by, sw - 2, 18], COL_BORDER);
            font.draw_text(fb, sx + 3, *by + 3, 10, COL_TEXT, sl);
            ui.buttons.push(UiButton { id: format!("shadow_{}", i), rect: [sx, *by, sw - 2, 18] });
        }
        *by += 24;

        font.draw_text(fb, bx, *by, 12, COL_TEXT_DIM, "Texture Quality:");
        *by += 16;
        let tex_labels = ["Full", "Half", "Qtr"];
        let tw = (bw - 6) / 3;
        for (i, tl) in tex_labels.iter().enumerate() {
            let tx = bx + i as i32 * tw;
            let hovered = hover_btn == Some(format!("tex_quality_{}", i).as_str());
            let is_t = match set.texture_quality {
                TextureQuality::Full if i == 0 => true,
                TextureQuality::Half if i == 1 => true,
                TextureQuality::Quarter if i == 2 => true,
                _ => false,
            };
            let col = if is_t { COL_SELECT } else { COL_BUTTON };
            let c = if hovered { lighten(col) } else { col };
            rect(fb, [tx, *by, tw - 2, 18], c);
            border(fb, [tx, *by, tw - 2, 18], COL_BORDER);
            font.draw_text(fb, tx + 3, *by + 3, 11, COL_TEXT, tl);
            ui.buttons.push(UiButton { id: format!("tex_quality_{}", i), rect: [tx, *by, tw - 2, 18] });
        }
        *by += 24;

        font.draw_text(fb, bx, *by, 12, COL_TEXT_DIM, "Anti-Aliasing:");
        *by += 16;
        let aa_labels = ["Off", "FXAA", "TAA", "2x", "4x", "8x"];
        let aw = (bw - 12) / aa_labels.len() as i32;
        for (i, al) in aa_labels.iter().enumerate() {
            let ax = bx + i as i32 * aw;
            let hovered = hover_btn == Some(format!("aa_{}", i).as_str());
            let is_a = match set.anti_aliasing {
                AntiAliasingMode::Off if i == 0 => true,
                AntiAliasingMode::FXAA if i == 1 => true,
                AntiAliasingMode::TAA if i == 2 => true,
                AntiAliasingMode::MSAA2x if i == 3 => true,
                AntiAliasingMode::MSAA4x if i == 4 => true,
                AntiAliasingMode::MSAA8x if i == 5 => true,
                _ => false,
            };
            let col = if is_a { COL_SELECT } else { COL_BUTTON };
            let c = if hovered { lighten(col) } else { col };
            rect(fb, [ax, *by, aw - 2, 18], c);
            border(fb, [ax, *by, aw - 2, 18], COL_BORDER);
            font.draw_text(fb, ax + 2, *by + 3, 10, COL_TEXT, al);
            ui.buttons.push(UiButton { id: format!("aa_{}", i), rect: [ax, *by, aw - 2, 18] });
        }
        *by += 24;

        let vsync_label = if set.vsync { "VSync: ON" } else { "VSync: OFF" };
        draw_btn(fb, font, bx, *by, bw, 18, vsync_label, COL_BUTTON, hover_btn, "settings_vsync", ui);
        *by += 22;
        *by
    }

    // ───────────────────────────────────────────────────────── 10. PROFİLEYİCİ PANELİ
    fn draw_profiler_panel(
        &self,
        fb: &mut SoftwareRenderer,
        font: &mut UiFont,
        bx: i32,
        by: &mut i32,
        bw: i32,
        _hover_btn: Option<&str>,
        _ui: &mut UiLayout,
    ) -> i32 {
        let prof = &self.profiler;
        let avg = prof.avg_frame_time();
        let fps = prof.fps();
        let mn = prof.min_frame_time();
        let mx = prof.max_frame_time();

        font.draw_text(fb, bx, *by, 12, COL_TEXT, "Frame Time:");
        *by += 16;

        let max_bars = 32;
        let bar_w = (bw - 2) / max_bars;
        let max_ms = 50.0;
        for i in 0..max_bars {
            let ft = prof.frame_times[i % prof.frame_times.len()];
            let h = ((ft / max_ms) * 60.0).min(60.0) as i32;
            let col = if ft > 33.0 { [255, 80, 80, 255] } else if ft > 16.0 { [255, 200, 60, 255] } else { [100, 200, 120, 255] };
            rect(fb, [bx + i * bar_w, *by + 60 - h, bar_w - 1, h], col);
        }
        rect(fb, [bx, *by + 10, bw, 1], [80, 80, 80, 255]);
        rect(fb, [bx, *by + 30, bw, 1], [120, 120, 120, 255]);
        font.draw_text(fb, bx + 2, *by + 2, 10, COL_TEXT_DIM, "16ms");
        font.draw_text(fb, bx + 2, *by + 22, 10, COL_TEXT_DIM, "33ms");
        *by += 66;

        font.draw_text(fb, bx, *by, 12, COL_TEXT, &format!("Avg: {:.1f}ms ({:.0f} FPS)", avg, fps));
        *by += 16;
        font.draw_text(fb, bx, *by, 12, COL_TEXT_DIM, &format!("Min: {:.1f}ms  Max: {:.1f}ms", mn, mx));
        *by += 16;
        font.draw_text(fb, bx, *by, 12, COL_TEXT, &format!("Draws: {}  Tris: {}", prof.draw_calls, prof.triangle_count));
        *by += 16;
        let mb = (prof.texture_memory as f32 / 1024.0 / 1024.0).max(0.0);
        let vb = (prof.buffer_memory as f32 / 1024.0 / 1024.0).max(0.0);
        font.draw_text(fb, bx, *by, 12, COL_TEXT_DIM, &format!("Mem: {:.1f}MB | VRAM: {:.1f}MB", mb, vb));
        *by += 16;
        *by
    }

    fn draw_slider_row(
        &self,
        fb: &mut SoftwareRenderer,
        font: &mut UiFont,
        bx: i32,
        by: &mut i32,
        bw: i32,
        label: &str,
        value: f32,
        _min_val: f32,
        _max_val: f32,
        _id: &str,
        _ui: &mut UiLayout,
    ) {
        font.draw_text(fb, bx, *by, 12, COL_TEXT, label);
        *by += 16;
        let track_w = bw;
        // Basit ilerleme çubuğu
        rect(fb, [bx, *by, track_w, 6], [50, 50, 53, 255]);
        *by += 10;
    }
}

// ═══════════════════════════════════════════════════════════ Buton tıklamalarını işle
pub fn handle_extension_button(ext: &mut EditorExtensions, id: &str) {
    match id {
        "audio_mixer" => toggle_panel(ext, PanelId::AudioMixer),
        "save_manager" => toggle_panel(ext, PanelId::SaveManager),
        "input_rebind" => toggle_panel(ext, PanelId::InputRebind),
        "localization" => toggle_panel(ext, PanelId::Localization),
        "vr_settings" => toggle_panel(ext, PanelId::VRSettings),
        "mod_manager" => toggle_panel(ext, PanelId::ModManager),
        "debug" => toggle_panel(ext, PanelId::Debug),
        "ai_nav" => toggle_panel(ext, PanelId::AINav),
        "settings" => toggle_panel(ext, PanelId::Settings),
        "profiler" => toggle_panel(ext, PanelId::Profiler),

        "master_mute" => ext.audio_mixer.master_mute = !ext.audio_mixer.master_mute,
        "music_mute" => ext.audio_mixer.music_mute = !ext.audio_mixer.music_mute,
        "sfx_mute" => ext.audio_mixer.sfx_mute = !ext.audio_mixer.sfx_mute,
        "voice_mute" => ext.audio_mixer.voice_mute = !ext.audio_mixer.voice_mute,
        "audio_output" => ext.audio_mixer.output_device = (ext.audio_mixer.output_device + 1) % ext.audio_mixer.output_devices.len(),

        "save_save" => { ext.save_manager.selected_slot = Some(0); }
        "save_load" => {
            if let Some(idx) = ext.save_manager.selected_slot {
                let _ = idx;
            }
        }
        "save_delete" => {
            if let Some(idx) = ext.save_manager.selected_slot {
                if idx < ext.save_manager.save_slots.len() {
                    ext.save_manager.save_slots.remove(idx);
                    ext.save_manager.selected_slot = None;
                }
            }
        }
        "save_auto_toggle" => ext.save_manager.auto_save = !ext.save_manager.auto_save,
        "save_cloud_toggle" => ext.save_manager.cloud_save_enabled = !ext.save_manager.cloud_save_enabled,

        id if id.starts_with("rebind_") => {
            if let Ok(idx) = id.strip_prefix("rebind_").unwrap().parse::<usize>() {
                ext.input_rebind.rebinding = true;
                ext.input_rebind.rebind_action_idx = Some(idx);
                ext.input_rebind.current_action = Some(idx);
                ext.input_rebind.conflict_warning = None;
            }
        }
        id if id.starts_with("input_preset_") => {
            if let Ok(idx) = id.strip_prefix("input_preset_").unwrap().parse::<usize>() {
                ext.input_rebind.preset = match idx {
                    0 => InputPreset::Default,
                    1 => InputPreset::Accessibility,
                    2 => InputPreset::LeftHanded,
                    3 => InputPreset::OneHanded,
                    _ => InputPreset::Default,
                };
                ext.input_rebind.conflict_warning = None;
            }
        }

        "lang_select" => {
            ext.localization.current_language = (ext.localization.current_language + 1) % ext.localization.languages.len();
        }
        "loc_extract" => {
            ext.localization.extracting = true;
            ext.localization.strings_extracted = 0;
        }

        "vr_toggle" => ext.vr_settings.vr_enabled = !ext.vr_settings.vr_enabled,
        id if id.starts_with("vr_runtime_") => {
            if let Ok(idx) = id.strip_prefix("vr_runtime_").unwrap().parse::<usize>() {
                ext.vr_settings.runtime = match idx {
                    0 => VRRuntime::OpenXR,
                    1 => VRRuntime::Oculus,
                    2 => VRRuntime::SteamVR,
                    3 => VRRuntime::Mock,
                    _ => VRRuntime::Mock,
                };
            }
        }
        "vr_ms_reduction" => ext.vr_settings.motion_sickness_reduction = !ext.vr_settings.motion_sickness_reduction,
        "vr_foveated" => ext.vr_settings.foveated_rendering = !ext.vr_settings.foveated_rendering,

        "mod_up" => {
            if let Some(idx) = ext.mod_manager.selected_mod {
                if idx > 0 {
                    ext.mod_manager.mods.swap(idx, idx - 1);
                    ext.mod_manager.mods[idx].load_order = idx as i32;
                    ext.mod_manager.mods[idx - 1].load_order = (idx - 1) as i32;
                    ext.mod_manager.selected_mod = Some(idx - 1);
                }
            }
        }
        "mod_down" => {
            if let Some(idx) = ext.mod_manager.selected_mod {
                if idx + 1 < ext.mod_manager.mods.len() {
                    ext.mod_manager.mods.swap(idx, idx + 1);
                    ext.mod_manager.mods[idx].load_order = idx as i32;
                    ext.mod_manager.mods[idx + 1].load_order = (idx + 1) as i32;
                    ext.mod_manager.selected_mod = Some(idx + 1);
                }
            }
        }
        "mod_folder" => {}

        id if id.starts_with("debug_mode_") => {
            if let Ok(idx) = id.strip_prefix("debug_mode_").unwrap().parse::<usize>() {
                ext.debug_panel.render_mode = match idx {
                    0 => DebugRenderMode::None,
                    1 => DebugRenderMode::Wireframe,
                    2 => DebugRenderMode::Normals,
                    3 => DebugRenderMode::CollisionShapes,
                    4 => DebugRenderMode::AABB,
                    5 => DebugRenderMode::SkeletonOverlay,
                    6 => DebugRenderMode::LightComplexity,
                    7 => DebugRenderMode::MetalRoughness,
                    _ => DebugRenderMode::None,
                };
            }
        }
        id if id.starts_with("debug_cat_") => {
            let cat = match id {
                "debug_cat_physics" => &mut ext.debug_panel.physics_debug,
                "debug_cat_ai" => &mut ext.debug_panel.ai_debug,
                "debug_cat_rendering" => &mut ext.debug_panel.rendering_debug,
                "debug_cat_perf" => &mut ext.debug_panel.performance_debug,
                _ => return,
            };
            *cat = match cat {
                DebugCategory::Off => DebugCategory::Minimal,
                DebugCategory::Minimal => DebugCategory::Normal,
                DebugCategory::Normal => DebugCategory::Verbose,
                DebugCategory::Verbose => DebugCategory::Off,
            };
        }
        "debug_perf_toggle" => ext.debug_panel.show_performance_stats = !ext.debug_panel.show_performance_stats,
        "debug_fg_toggle" => ext.debug_panel.show_frame_graph = !ext.debug_panel.show_frame_graph,
        "debug_screenshot" => ext.debug_panel.screenshot_requested = true,

        "ai_navmesh" => ext.ai_nav.show_navmesh = !ext.ai_nav.show_navmesh,
        "ai_paths" => ext.ai_nav.show_paths = !ext.ai_nav.show_paths,
        "ai_behavior_tree" => ext.ai_nav.show_behavior_tree = !ext.ai_nav.show_behavior_tree,
        "ai_perception" => ext.ai_nav.show_perception_cones = !ext.ai_nav.show_perception_cones,

        id if id.starts_with("quality_") => {
            if let Ok(idx) = id.strip_prefix("quality_").unwrap().parse::<usize>() {
                ext.settings.render_quality = match idx {
                    0 => RenderQuality::Low,
                    1 => RenderQuality::Medium,
                    2 => RenderQuality::High,
                    3 => RenderQuality::Ultra,
                    _ => RenderQuality::High,
                };
            }
        }
        id if id.starts_with("shadow_") => {
            if let Ok(idx) = id.strip_prefix("shadow_").unwrap().parse::<usize>() {
                ext.settings.shadow_quality = match idx {
                    0 => ShadowQuality::Off,
                    1 => ShadowQuality::Low,
                    2 => ShadowQuality::Medium,
                    3 => ShadowQuality::High,
                    4 => ShadowQuality::Ultra,
                    _ => ShadowQuality::Medium,
                };
            }
        }
        id if id.starts_with("tex_quality_") => {
            if let Ok(idx) = id.strip_prefix("tex_quality_").unwrap().parse::<usize>() {
                ext.settings.texture_quality = match idx {
                    0 => TextureQuality::Full,
                    1 => TextureQuality::Half,
                    2 => TextureQuality::Quarter,
                    _ => TextureQuality::Full,
                };
            }
        }
        id if id.starts_with("aa_") => {
            if let Ok(idx) = id.strip_prefix("aa_").unwrap().parse::<usize>() {
                ext.settings.anti_aliasing = match idx {
                    0 => AntiAliasingMode::Off,
                    1 => AntiAliasingMode::FXAA,
                    2 => AntiAliasingMode::TAA,
                    3 => AntiAliasingMode::MSAA2x,
                    4 => AntiAliasingMode::MSAA4x,
                    5 => AntiAliasingMode::MSAA8x,
                    _ => AntiAliasingMode::FXAA,
                };
            }
        }
        "settings_vsync" => ext.settings.vsync = !ext.settings.vsync,
        _ => {}
    }
}

pub fn toggle_panel(ext: &mut EditorExtensions, panel: PanelId) {
    let idx = panel as usize;
    ext.panel_visible[idx] = !ext.panel_visible[idx];
}

// ═══════════════════════════════════════════════════════════ Yardımcılar

fn rand_f32() -> f32 {
    static mut SEED: u32 = 12345;
    unsafe {
        SEED = SEED.wrapping_mul(1664525).wrapping_add(1013904223);
        (SEED >> 8) as f32 / 16777216.0
    }
}

fn lighten(c: [u8; 4]) -> [u8; 4] {
    [
        c[0].saturating_add(28),
        c[1].saturating_add(28),
        c[2].saturating_add(28),
        255,
    ]
}

fn draw_btn(
    fb: &mut SoftwareRenderer,
    font: &mut UiFont,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    label: &str,
    col: [u8; 4],
    hover_btn: Option<&str>,
    id: &str,
    ui: &mut UiLayout,
) {
    let hovered = hover_btn == Some(id);
    let c = if hovered { lighten(col) } else { col };
    rect(fb, [x, y, w, h], c);
    border(fb, [x, y, w, h], COL_BORDER);
    let tw = font.measure(13, label);
    font.draw_text(fb, x + ((w as f32 - tw) / 2.0) as i32, y + 5, 13, COL_TEXT, label);
    ui.buttons.push(UiButton { id: id.into(), rect: [x, y, w, h] });
}

fn draw_volume_row(
    fb: &mut SoftwareRenderer,
    font: &mut UiFont,
    bx: i32,
    by: &mut i32,
    bw: i32,
    label: &str,
    volume: f32,
    mute: bool,
    btn_id: &str,
    hover_btn: Option<&str>,
    ui: &mut UiLayout,
) {
    font.draw_text(fb, bx, *by, 12, COL_TEXT, label);
    let mute_label = if mute { "MUTED" } else { "ON" };
    draw_btn(fb, font, bx + bw - 50, *by - 2, 44, 18, mute_label, COL_BUTTON, hover_btn, &format!("{}_mute", btn_id), ui);
    *by += 16;
    let pct = (volume * 100.0) as i32;
    let filled = ((bw - 60) as f32 * volume) as i32;
    rect(fb, [bx, *by, bw - 60, 8], [50, 50, 53, 255]);
    rect(fb, [bx, *by, filled, 8], [80, 160, 255, 255]);
    font.draw_text(fb, bx + bw - 55, *by - 4, 11, COL_TEXT_DIM, &format!("{}%", pct));
    *by += 18;
}
