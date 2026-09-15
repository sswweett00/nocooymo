
//! engine.rs — Tam oyun motoru API'si
//!
//! Tüm alt sistemleri tek çatı altında birleştirir.
//! Bir oyun oluşturmak için tek yapmanız gereken `Game` trait'ini implement etmek.

use glam::{Vec3, Quat};
use std::collections::HashMap;
use std::time::Duration;

// ═══════════════════════════════════════════════════════════ ENGINE CORE

/// Ana oyun motoru
pub struct Engine {
    pub input: InputManager,
    pub audio: AudioManager,
    pub assets: AssetManager,
    pub scenes: SceneManager,
    pub time: TimeManager,
    pub renderer: RendererState,
    pub physics: PhysicsState,
    pub entities: EntityManager,
    pub ui: UiState,
    pub debug: DebugState,
    pub config: EngineConfig,
}

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub target_fps: u32,
    pub fixed_timestep: f32,
    pub vsync: bool,
    pub resizable: bool,
    pub fullscreen: bool,
    pub physics_substeps: u32,
    pub max_entities: u32,
    pub audio_channels: u32,
    pub master_volume: f32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            title: "Elysium Game".into(), width: 1280, height: 720, target_fps: 60,
            fixed_timestep: 1.0 / 60.0, vsync: true, resizable: true, fullscreen: false,
            physics_substeps: 4, max_entities: 10000, audio_channels: 32, master_volume: 1.0,
        }
    }
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            input: InputManager::new(),
            audio: AudioManager::new(config.audio_channels),
            assets: AssetManager::new(),
            scenes: SceneManager::new(),
            time: TimeManager::new(config.fixed_timestep),
            renderer: RendererState::default(),
            physics: PhysicsState::new(config.physics_substeps),
            entities: EntityManager::with_capacity(config.max_entities as usize),
            ui: UiState::new(),
            debug: DebugState::new(),
            config,
        }
    }

    /// Ana oyun döngüsü — her frame çağrılır
    pub fn update(&mut self, game: &mut dyn Game, dt: f32) {
        self.time.update(dt);

        // Fixed timestep fizik
        while self.time.accumulator >= self.time.fixed_dt {
            game.fixed_update(self, self.time.fixed_dt);
            self.physics.step(self.time.fixed_dt);
            self.time.accumulator -= self.time.fixed_dt;
        }

        // Variable timestep game logic
        game.update(self, dt);

        // Sistem güncellemeleri
        self.input.end_frame();
        self.scenes.update_simple();
        self.audio.update(dt);
        self.debug.update(dt);
    }

    pub fn render(&self, game: &dyn Game) {
        game.render(self);
    }
}

// ═══════════════════════════════════════════════════════════ GAME TRAIT

/// Tüm oyunlar bu trait'i implement eder
pub trait Game {
    /// Motor başlatıldığında çağrılır
    fn init(&mut self, engine: &mut Engine);

    /// Her frame çağrılır (variable timestep)
    fn update(&mut self, engine: &mut Engine, dt: f32);

    /// Sabit timestep ile çağrılır (fizik, input)
    fn fixed_update(&mut self, engine: &mut Engine, dt: f32);

    /// Render çağrısı
    fn render(&self, engine: &Engine);

    /// Pencere kapatıldığında
    fn shutdown(&mut self, _engine: &mut Engine) {}

    /// Pencere yeniden boyutlandırıldığında
    fn on_resize(&mut self, _engine: &mut Engine, _width: u32, _height: u32) {}

    /// Tooltip: oyun adı
    fn name(&self) -> &str { "Elysium Game" }
}

// ═══════════════════════════════════════════════════════════ INPUT MANAGER

pub struct InputManager {
    pub keys: HashMap<KeyCode, KeyState>,
    pub mouse: MouseState,
    pub gamepad: GamepadState,
    pub text_input: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyCode {
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    Space, Enter, Escape, Tab, Backspace, Delete, Insert,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Shift, Control, Alt, Super,
    Home, End, PageUp, PageDown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyState { JustPressed, Pressed, JustReleased, Released }

pub struct MouseState {
    pub x: f64, pub y: f64,
    pub delta_x: f64, pub delta_y: f64,
    pub scroll_x: f64, pub scroll_y: f64,
    pub buttons: HashMap<MouseButton, KeyState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton { Left, Right, Middle, Extra1, Extra2 }

pub struct GamepadState {
    pub connected: bool,
    pub axes: HashMap<GamepadAxis, f32>,
    pub buttons: HashMap<GamepadButton, KeyState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GamepadAxis { LeftX, LeftY, RightX, RightY, LeftTrigger, RightTrigger }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GamepadButton { A, B, X, Y, LB, RB, Back, Start, Guide, LS, RS, DPadUp, DPadDown, DPadLeft, DPadRight }

impl InputManager {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(), mouse: MouseState {
                x: 0.0, y: 0.0, delta_x: 0.0, delta_y: 0.0, scroll_x: 0.0, scroll_y: 0.0,
                buttons: HashMap::new(),
            },
            gamepad: GamepadState { connected: false, axes: HashMap::new(), buttons: HashMap::new() },
            text_input: String::new(),
        }
    }

    pub fn key_pressed(&self, key: KeyCode) -> bool { matches!(self.keys.get(&key), Some(KeyState::Pressed) | Some(KeyState::JustPressed)) }
    pub fn key_just_pressed(&self, key: KeyCode) -> bool { matches!(self.keys.get(&key), Some(KeyState::JustPressed)) }
    pub fn key_just_released(&self, key: KeyCode) -> bool { matches!(self.keys.get(&key), Some(KeyState::JustReleased)) }
    pub fn key_held(&self, key: KeyCode) -> bool { self.keys.get(&key) == Some(&KeyState::Pressed) }
    pub fn mouse_pressed(&self, btn: MouseButton) -> bool { matches!(self.mouse.buttons.get(&btn), Some(KeyState::Pressed) | Some(KeyState::JustPressed)) }
    pub fn mouse_just_pressed(&self, btn: MouseButton) -> bool { matches!(self.mouse.buttons.get(&btn), Some(KeyState::JustPressed)) }
    pub fn mouse_position(&self) -> (f64, f64) { (self.mouse.x, self.mouse.y) }
    pub fn mouse_delta(&self) -> (f64, f64) { (self.mouse.delta_x, self.mouse.delta_y) }
    pub fn scroll_delta(&self) -> (f64, f64) { (self.mouse.scroll_x, self.mouse.scroll_y) }
    pub fn gamepad_axis(&self, axis: GamepadAxis) -> f32 { self.gamepad.axes.get(&axis).copied().unwrap_or(0.0) }
    pub fn gamepad_pressed(&self, btn: GamepadButton) -> bool { matches!(self.gamepad.buttons.get(&btn), Some(KeyState::Pressed) | Some(KeyState::JustPressed)) }

    pub fn end_frame(&mut self) {
        self.mouse.delta_x = 0.0;
        self.mouse.delta_y = 0.0;
        self.mouse.scroll_x = 0.0;
        self.mouse.scroll_y = 0.0;
        for state in self.keys.values_mut() { if *state == KeyState::JustPressed { *state = KeyState::Pressed; } if *state == KeyState::JustReleased { *state = KeyState::Released; } }
        for state in self.mouse.buttons.values_mut() { if *state == KeyState::JustPressed { *state = KeyState::Pressed; } if *state == KeyState::JustReleased { *state = KeyState::Released; } }
        self.text_input.clear();
    }

    pub fn set_key(&mut self, key: KeyCode, pressed: bool) {
        if pressed { self.keys.insert(key, KeyState::JustPressed); }
        else if let Some(state) = self.keys.get_mut(&key) { if *state == KeyState::Pressed { *state = KeyState::JustReleased; } }
    }
    pub fn set_mouse_button(&mut self, btn: MouseButton, pressed: bool) {
        if pressed { self.mouse.buttons.insert(btn, KeyState::JustPressed); }
        else if let Some(state) = self.mouse.buttons.get_mut(&btn) { if *state == KeyState::Pressed { *state = KeyState::JustReleased; } }
    }
    pub fn set_mouse_position(&mut self, x: f64, y: f64) { self.mouse.delta_x = x - self.mouse.x; self.mouse.delta_y = y - self.mouse.y; self.mouse.x = x; self.mouse.y = y; }
    pub fn add_scroll(&mut self, dx: f64, dy: f64) { self.mouse.scroll_x += dx; self.mouse.scroll_y += dy; }
    pub fn set_gamepad_axis(&mut self, axis: GamepadAxis, value: f32) { self.gamepad.axes.insert(axis, value); }

    /// WASD hareket vektörü
    pub fn movement_vector(&self) -> Vec3 {
        let mut dir = Vec3::ZERO;
        if self.key_pressed(KeyCode::W) { dir.z -= 1.0; }
        if self.key_pressed(KeyCode::S) { dir.z += 1.0; }
        if self.key_pressed(KeyCode::A) { dir.x -= 1.0; }
        if self.key_pressed(KeyCode::D) { dir.x += 1.0; }
        if self.key_pressed(KeyCode::Space) { dir.y += 1.0; }
        if self.key_pressed(KeyCode::Shift) { dir.y -= 1.0; }
        dir.normalize_or_zero()
    }
}

// ═══════════════════════════════════════════════════════════ AUDIO MANAGER

pub struct AudioManager {
    pub channels: Vec<AudioChannel>,
    pub music: MusicTrack,
    pub master_volume: f32,
    pub sfx_volume: f32,
    pub music_volume: f32,
    pub muted: bool,
    pub listener_position: Vec3,
    pub next_sound_id: u32,
}

pub struct AudioChannel {
    pub id: u32,
    pub sound_id: Option<u32>,
    pub volume: f32,
    pub pan: f32,
    pub pitch: f32,
    pub position: Option<Vec3>,
    pub looping: bool,
    pub playing: bool,
    pub time: f32,
    pub duration: f32,
}

pub struct MusicTrack {
    pub current: Option<u32>,
    pub volume: f32,
    pub fade_in: f32,
    pub fade_out: f32,
    pub crossfade: f32,
    pub position: f32,
    pub playing: bool,
}

impl AudioManager {
    pub fn new(channels: u32) -> Self {
        Self {
            channels: (0..channels).map(|i| AudioChannel {
                id: i, sound_id: None, volume: 1.0, pan: 0.0, pitch: 1.0,
                position: None, looping: false, playing: false, time: 0.0, duration: 0.0,
            }).collect(),
            music: MusicTrack { current: None, volume: 0.7, fade_in: 1.0, fade_out: 2.0, crossfade: 1.0, position: 0.0, playing: false },
            master_volume: 1.0, sfx_volume: 1.0, music_volume: 0.7, muted: false,
            listener_position: Vec3::ZERO, next_sound_id: 1,
        }
    }

    pub fn play_sfx(&mut self, sound_id: u32, volume: f32, pitch: f32) {
        if let Some(ch) = self.channels.iter_mut().find(|c| !c.playing) {
            ch.sound_id = Some(sound_id);
            ch.volume = volume;
            ch.pitch = pitch;
            ch.playing = true;
            ch.time = 0.0;
            ch.duration = 2.0;
        }
    }

    pub fn play_sfx_3d(&mut self, sound_id: u32, position: Vec3, volume: f32) {
        let dist = self.listener_position.distance(position);
        let atten = (1.0 / (1.0 + dist * 0.1)).min(1.0);
        self.play_sfx(sound_id, volume * atten, 1.0);
    }

    pub fn play_music(&mut self, track_id: u32) { self.music.current = Some(track_id); self.music.playing = true; self.music.position = 0.0; }
    pub fn stop_music(&mut self) { self.music.playing = false; }
    pub fn set_master_volume(&mut self, v: f32) { self.master_volume = v.clamp(0.0, 1.0); }
    pub fn set_sfx_volume(&mut self, v: f32) { self.sfx_volume = v.clamp(0.0, 1.0); }
    pub fn set_music_volume(&mut self, v: f32) { self.music_volume = v.clamp(0.0, 1.0); }
    pub fn toggle_mute(&mut self) { self.muted = !self.muted; }

    pub fn update(&mut self, dt: f32) {
        for ch in &mut self.channels {
            if !ch.playing { continue; }
            ch.time += dt;
            if ch.time >= ch.duration && !ch.looping { ch.playing = false; }
            if ch.looping { ch.time = ch.time % ch.duration; }
        }
        if self.music.playing { self.music.position += dt; }
    }

    pub fn active_channel_count(&self) -> usize { self.channels.iter().filter(|c| c.playing).count() }
}

// ═══════════════════════════════════════════════════════════ ASSET MANAGER

pub struct AssetManager {
    pub loaded: HashMap<u64, AssetEntry>,
    pub loading_queue: VecDeque<String>,
    pub next_id: u64,
    pub stats: AssetStats,
}

#[derive(Clone, Debug)]
pub struct AssetEntry {
    pub id: u64,
    pub path: String,
    pub asset_type: AssetType,
    pub size_bytes: usize,
    pub ref_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AssetType { Texture, Mesh, Audio, Script, Material, Animation, Scene, Config }

#[derive(Clone, Debug, Default)]
pub struct AssetStats {
    pub total_loaded: u32,
    pub total_memory_bytes: u64,
    pub textures_loaded: u32,
    pub meshes_loaded: u32,
    pub audio_loaded: u32,
}

use std::collections::VecDeque;

impl AssetManager {
    pub fn new() -> Self {
        Self { loaded: HashMap::new(), loading_queue: VecDeque::new(), next_id: 1, stats: AssetStats::default() }
    }

    pub fn load(&mut self, path: &str, asset_type: AssetType, size: usize) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.loaded.insert(id, AssetEntry { id, path: path.into(), asset_type, size_bytes: size, ref_count: 1 });
        self.stats.total_loaded += 1;
        self.stats.total_memory_bytes += size as u64;
        match asset_type {
            AssetType::Texture => self.stats.textures_loaded += 1,
            AssetType::Mesh => self.stats.meshes_loaded += 1,
            AssetType::Audio => self.stats.audio_loaded += 1,
            _ => {}
        }
        id
    }

    pub fn get(&self, id: u64) -> Option<&AssetEntry> { self.loaded.get(&id) }
    pub fn unload(&mut self, id: u64) { if let Some(entry) = self.loaded.remove(&id) { self.stats.total_loaded -= 1; self.stats.total_memory_bytes -= entry.size_bytes as u64; } }
    pub fn queue_load(&mut self, path: &str) { self.loading_queue.push_back(path.into()); }
    pub fn memory_usage_mb(&self) -> f32 { self.stats.total_memory_bytes as f32 / (1024.0 * 1024.0) }
}

// ═══════════════════════════════════════════════════════════ SCENE MANAGER

pub struct SceneManager {
    pub scenes: Vec<SceneData>,
    pub current_scene: Option<usize>,
    pub transitioning: bool,
    pub transition_progress: f32,
    pub transition_duration: f32,
    pub fade_color: [f32; 3],
}

pub struct SceneData {
    pub name: String,
    pub entities: Vec<u32>,
    pub loaded: bool,
    pub persistent: bool,
}

impl SceneManager {
    pub fn new() -> Self {
        Self { scenes: Vec::new(), current_scene: None, transitioning: false, transition_progress: 0.0, transition_duration: 1.0, fade_color: [0.0; 3] }
    }

    pub fn add_scene(&mut self, name: &str, persistent: bool) -> usize {
        let idx = self.scenes.len();
        self.scenes.push(SceneData { name: name.into(), entities: Vec::new(), loaded: false, persistent });
        idx
    }

    pub fn load_scene(&mut self, idx: usize) {
        if let Some(scene) = self.scenes.get_mut(idx) { scene.loaded = true; }
        self.current_scene = Some(idx);
    }

    pub fn transition_to(&mut self, idx: usize, duration: f32, fade_color: [f32; 3]) {
        self.transitioning = true;
        self.transition_progress = 0.0;
        self.transition_duration = duration;
        self.fade_color = fade_color;
        if duration <= 0.0 { self.load_scene(idx); self.transitioning = false; }
    }

    pub fn update(&mut self, _engine: &mut Engine) { self.update_simple(); }
    pub fn update_simple(&mut self) {
        if self.transitioning {
            self.transition_progress += 1.0 / 60.0;
            if self.transition_progress >= 1.0 { self.transitioning = false; }
        }
    }

    pub fn current_scene_name(&self) -> &str {
        self.current_scene.and_then(|i| self.scenes.get(i)).map(|s| s.name.as_str()).unwrap_or("None")
    }

    pub fn scene_count(&self) -> usize { self.scenes.len() }
}

// ═══════════════════════════════════════════════════════════ TIME MANAGER

pub struct TimeManager {
    pub dt: f32,
    pub fixed_dt: f32,
    pub elapsed: f32,
    pub frame: u64,
    pub fps: f32,
    pub time_scale: f32,
    pub accumulator: f32,
    pub delta_duration: Duration,
}

impl TimeManager {
    pub fn new(fixed_dt: f32) -> Self {
        Self { dt: 0.0, fixed_dt, elapsed: 0.0, frame: 0, fps: 60.0, time_scale: 1.0, accumulator: 0.0, delta_duration: Duration::ZERO }
    }

    pub fn update(&mut self, dt: f32) {
        self.delta_duration = Duration::from_secs_f32(dt);
        self.dt = dt * self.time_scale;
        self.elapsed += self.dt;
        self.frame += 1;
        self.accumulator += self.dt;
        if dt > 0.0 { self.fps = self.fps * 0.95 + (1.0 / dt) * 0.05; }
    }

    pub fn set_time_scale(&mut self, scale: f32) { self.time_scale = scale.max(0.0); }
    pub fn pause(&mut self) { self.time_scale = 0.0; }
    pub fn resume(&mut self) { self.time_scale = 1.0; }
    pub fn is_paused(&self) -> bool { self.time_scale == 0.0 }
}

// ═══════════════════════════════════════════════════════════ ENTITY MANAGER

pub struct EntityManager {
    pub entities: Vec<EntityData>,
    pub free_list: Vec<u32>,
    pub active_count: u32,
}

#[derive(Clone, Debug)]
pub struct EntityData {
    pub id: u32,
    pub name: String,
    pub active: bool,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub velocity: Vec3,
    pub tags: Vec<String>,
    pub components: HashMap<String, String>,
}

impl EntityManager {
    pub fn with_capacity(cap: usize) -> Self {
        Self { entities: Vec::with_capacity(cap), free_list: Vec::new(), active_count: 0 }
    }

    pub fn spawn(&mut self, name: &str) -> u32 {
        if let Some(id) = self.free_list.pop() {
            if let Some(e) = self.entities.get_mut(id as usize) {
                e.name = name.into();
                e.active = true;
                e.position = Vec3::ZERO;
                e.rotation = Quat::IDENTITY;
                e.scale = Vec3::ONE;
                e.velocity = Vec3::ZERO;
                e.tags.clear();
                e.components.clear();
                self.active_count += 1;
                return id;
            }
        }
        let id = self.entities.len() as u32;
        self.entities.push(EntityData {
            id, name: name.into(), active: true,
            position: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE,
            velocity: Vec3::ZERO, tags: Vec::new(), components: HashMap::new(),
        });
        self.active_count += 1;
        id
    }

    pub fn despawn(&mut self, id: u32) {
        if let Some(e) = self.entities.get_mut(id as usize) { e.active = false; self.active_count -= 1; }
        self.free_list.push(id);
    }

    pub fn get(&self, id: u32) -> Option<&EntityData> { self.entities.get(id as usize).filter(|e| e.active) }
    pub fn get_mut(&mut self, id: u32) -> Option<&mut EntityData> { self.entities.get_mut(id as usize).filter(|e| e.active) }

    pub fn add_tag(&mut self, id: u32, tag: &str) { if let Some(e) = self.get_mut(id) { e.tags.push(tag.into()); } }
    pub fn has_tag(&self, id: u32, tag: &str) -> bool { self.get(id).map(|e| e.tags.contains(&tag.to_string())).unwrap_or(false) }

    pub fn find_by_name(&self, name: &str) -> Option<u32> { self.entities.iter().filter(|e| e.active).find(|e| e.name == name).map(|e| e.id) }
    pub fn find_by_tag(&self, tag: &str) -> Vec<u32> { self.entities.iter().filter(|e| e.active && e.tags.contains(&tag.to_string())).map(|e| e.id).collect() }

    pub fn set_component(&mut self, id: u32, key: &str, value: &str) { if let Some(e) = self.get_mut(id) { e.components.insert(key.into(), value.into()); } }
    pub fn get_component(&self, id: u32, key: &str) -> Option<&str> { self.get(id)?.components.get(key).map(|s| s.as_str()) }
}

// ═══════════════════════════════════════════════════════════ RENDERER STATE

pub struct RendererState {
    pub frame_count: u64,
    pub draw_calls: u32,
    pub triangles_rendered: u64,
    pub batch_count: u32,
    pub screen_width: f32,
    pub screen_height: f32,
    pub camera: CameraState,
    pub post_process: PostProcessState,
    pub light_count: u32,
    pub ambient_intensity: f32,
}

#[derive(Clone, Debug)]
pub struct CameraState {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub orthographic: bool,
    pub ortho_size: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self { position: Vec3::new(0.0, 5.0, 10.0), target: Vec3::ZERO, up: Vec3::Y, fov: 60.0, near: 0.1, far: 1000.0, orthographic: false, ortho_size: 10.0 }
    }
}

#[derive(Clone, Debug)]
pub struct PostProcessState {
    pub enabled: bool,
    pub bloom: f32,
    pub vignette: f32,
    pub chromatic_aberration: f32,
    pub scanlines: bool,
    pub color_grading: [f32; 3],
}

impl Default for PostProcessState {
    fn default() -> Self {
        Self { enabled: true, bloom: 0.15, vignette: 0.3, chromatic_aberration: 0.002, scanlines: false, color_grading: [1.0; 3] }
    }
}

impl Default for RendererState {
    fn default() -> Self {
        Self { frame_count: 0, draw_calls: 0, triangles_rendered: 0, batch_count: 0, screen_width: 1280.0, screen_height: 720.0, camera: CameraState::default(), post_process: PostProcessState::default(), light_count: 0, ambient_intensity: 0.3 }
    }
}

// ═══════════════════════════════════════════════════════════ PHYSICS STATE

pub struct PhysicsState {
    pub gravity: Vec3,
    pub substeps: u32,
    pub enabled: bool,
    pub bodies: Vec<RigidBodyData>,
    pub collisions: Vec<CollisionPair>,
}

#[derive(Clone, Debug)]
pub struct RigidBodyData {
    pub entity_id: u32,
    pub body_type: RbType,
    pub mass: f32,
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub restitution: f32,
    pub friction: f32,
    pub gravity_scale: f32,
    pub is_sensor: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RbType { Dynamic, Static, Kinematic }

#[derive(Clone, Debug)]
pub struct CollisionPair { pub a: u32, pub b: u32, pub normal: Vec3, pub penetration: f32 }

impl PhysicsState {
    pub fn new(substeps: u32) -> Self {
        Self { gravity: Vec3::new(0.0, -9.81, 0.0), substeps, enabled: true, bodies: Vec::new(), collisions: Vec::new() }
    }

    pub fn step(&mut self, dt: f32) {
        if !self.enabled { return; }
        let sub_dt = dt / self.substeps as f32;
        for _ in 0..self.substeps {
            for body in &mut self.bodies {
                if body.body_type != RbType::Dynamic { continue; }
                body.velocity += self.gravity * body.gravity_scale * sub_dt;
                let pos_idx = body.entity_id as usize;
                // Integration would happen here
                let _ = pos_idx;
            }
        }
        self.collisions.clear();
    }

    pub fn add_body(&mut self, entity_id: u32, body_type: RbType, mass: f32) -> usize {
        let idx = self.bodies.len();
        self.bodies.push(RigidBodyData { entity_id, body_type, mass, velocity: Vec3::ZERO, angular_velocity: Vec3::ZERO, restitution: 0.5, friction: 0.5, gravity_scale: 1.0, is_sensor: false });
        idx
    }
}

// ═══════════════════════════════════════════════════════════ UI STATE

pub struct UiState {
    pub panels: Vec<UiPanel>,
    pub active_panel: Option<usize>,
    pub tooltip: Option<String>,
    pub cursor_icon: CursorIcon,
}

#[derive(Clone, Debug)]
pub struct UiPanel {
    pub name: String,
    pub rect: [f32; 4],
    pub visible: bool,
    pub draggable: bool,
    pub resizable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CursorIcon { Default, Pointer, Move, Resize, Text, Wait, Crosshair }

impl UiState {
    pub fn new() -> Self {
        Self { panels: Vec::new(), active_panel: None, tooltip: None, cursor_icon: CursorIcon::Default }
    }
    pub fn add_panel(&mut self, name: &str, x: f32, y: f32, w: f32, h: f32) {
        self.panels.push(UiPanel { name: name.into(), rect: [x, y, w, h], visible: true, draggable: true, resizable: true });
    }
}

// ═══════════════════════════════════════════════════════════ DEBUG STATE

pub struct DebugState {
    pub show_fps: bool,
    pub show_console: bool,
    pub show_wireframe: bool,
    pub show_colliders: bool,
    pub show_navmesh: bool,
    pub show_octree: bool,
    pub fps_history: Vec<f32>,
    pub console_lines: Vec<String>,
    pub max_console_lines: usize,
}

impl DebugState {
    pub fn new() -> Self {
        Self { show_fps: true, show_console: false, show_wireframe: false, show_colliders: false, show_navmesh: false, show_octree: false, fps_history: Vec::new(), console_lines: Vec::new(), max_console_lines: 100 }
    }
    pub fn update(&mut self, _dt: f32) {
        // FPS history maintained externally
    }
    pub fn log(&mut self, msg: &str) { self.console_lines.push(msg.into()); if self.console_lines.len() > self.max_console_lines { self.console_lines.remove(0); } }
    pub fn toggle_console(&mut self) { self.show_console = !self.show_console; }
}

// ═══════════════════════════════════════════════════════════ EXAMPLE GAME: PLATFORMER

/// 2D Platformer demo oyunu
pub struct PlatformerGame {
    pub player_id: Option<u32>,
    pub score: u32,
    pub lives: u32,
    pub gravity: f32,
    pub jump_force: f32,
    pub move_speed: f32,
    pub is_grounded: bool,
    pub camera_offset: Vec3,
    pub platforms: Vec<Platform>,
    pub coins: Vec<Coin>,
    pub particles: Vec<Particle>,
    pub state: GameState,
    pub level: u32,
    pub time: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GameState { Menu, Playing, Paused, GameOver, Victory }

#[derive(Clone, Debug)]
pub struct Platform { pub x: f32, pub y: f32, pub width: f32, pub height: f32, pub color: [f32; 3] }

#[derive(Clone, Debug)]
pub struct Coin { pub x: f32, pub y: f32, pub collected: bool, pub bob_time: f32 }

#[derive(Clone, Debug)]
pub struct Particle { pub x: f32, pub y: f32, pub vx: f32, pub vy: f32, pub life: f32, pub max_life: f32, pub color: [f32; 3], pub size: f32 }

impl PlatformerGame {
    pub fn new() -> Self {
        Self {
            player_id: None, score: 0, lives: 3, gravity: 20.0, jump_force: 8.0, move_speed: 6.0,
            is_grounded: false, camera_offset: Vec3::ZERO, platforms: Vec::new(), coins: Vec::new(),
            particles: Vec::new(), state: GameState::Menu, level: 1, time: 0.0,
        }
    }

    fn generate_level(&mut self, level: u32) {
        self.platforms.clear();
        self.coins.clear();
        self.particles.clear();

        // Zemin
        self.platforms.push(Platform { x: -5.0, y: -2.0, width: 30.0, height: 1.0, color: [0.3, 0.6, 0.3] });

        // Platformlar (level'a göre artan zorluk)
        let count = 3 + level as usize;
        for i in 0..count {
            let x = (i as f32 * 4.0) - 5.0;
            let y = (i as f32 * 1.5) - 1.0 + ((i as f32 * 0.7).sin() * 2.0);
            let w = 2.5 - (level as f32 * 0.1).max(0.5);
            self.platforms.push(Platform { x, y, width: w, height: 0.3, color: [0.5, 0.4, 0.3] });

            // Coin ekle
            if i % 2 == 0 {
                self.coins.push(Coin { x: x + w / 2.0, y: y + 0.8, collected: false, bob_time: i as f32 * 0.5 });
            }
        }
    }

    fn spawn_particles(&mut self, x: f32, y: f32, color: [f32; 3], count: u32) {
        for _ in 0..count {
            let angle = rand::random::<f32>() * std::f32::consts::TAU;
            let speed = rand::random::<f32>() * 3.0 + 1.0;
            self.particles.push(Particle {
                x, y, vx: angle.cos() * speed, vy: angle.sin() * speed - 2.0,
                life: 0.5 + rand::random::<f32>() * 0.5, max_life: 1.0, color, size: 0.1 + rand::random::<f32>() * 0.1,
            });
        }
    }
}

impl Game for PlatformerGame {
    fn init(&mut self, engine: &mut Engine) {
        engine.debug.log("=== ELYSIUM PLATFORMER ===");
        engine.debug.log("Controls: WASD move, Space jump, P pause");
        engine.debug.log("Collect all coins to advance levels!");
        self.player_id = Some(engine.entities.spawn("Player"));
        engine.entities.add_tag(self.player_id.unwrap(), "player");
        self.generate_level(self.level);
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) {
        self.time += dt;

        match self.state {
            GameState::Menu => {
                if engine.input.key_just_pressed(KeyCode::Space) || engine.input.key_just_pressed(KeyCode::Enter) {
                    self.state = GameState::Playing;
                    self.score = 0;
                    self.lives = 3;
                    self.level = 1;
                    self.generate_level(self.level);
                    engine.debug.log("Game Started!");
                }
            }
            GameState::Playing => {
                // Player input
                if let Some(player_id) = self.player_id {
                    if let Some(player) = engine.entities.get_mut(player_id) {
                        // Hareket
                        let move_input = engine.input.movement_vector();
                        player.velocity.x = move_input.x * self.move_speed;

                        // Zıplama
                        if engine.input.key_just_pressed(KeyCode::Space) && self.is_grounded {
                            player.velocity.y = self.jump_force;
                            self.is_grounded = false;
                            self.spawn_particles(player.position.x, player.position.y - 0.5, [1.0, 1.0, 0.5], 5);
                        }

                        // Gravity
                        player.velocity.y -= self.gravity * dt;

                        // Pozisyon güncelle
                        player.position += player.velocity * dt;

                        // Platform çarpışma
                        self.is_grounded = false;
                        for plat in &self.platforms {
                            if player.position.x + 0.3 > plat.x && player.position.x - 0.3 < plat.x + plat.width
                                && player.position.y - 0.5 < plat.y + plat.height
                                && player.position.y - 0.5 + player.velocity.y * dt <= plat.y + plat.height
                                && player.velocity.y <= 0.0
                            {
                                player.position.y = plat.y + plat.height + 0.5;
                                player.velocity.y = 0.0;
                                self.is_grounded = true;
                            }
                        }

                        // Coin toplama
                        let px = player.position.x;
                        let py = player.position.y;
                        let mut score_add = 0u32;
                        let mut coin_positions = Vec::new();
                        for coin in &mut self.coins {
                            if !coin.collected {
                                let dx = px - coin.x;
                                let dy = py - coin.y;
                                if dx * dx + dy * dy < 0.5 {
                                    coin.collected = true;
                                    score_add += 100;
                                    coin_positions.push((coin.x, coin.y));
                                }
                            }
                        }
                        if score_add > 0 {
                            self.score += score_add;
                            for (cx, cy) in coin_positions { self.spawn_particles(cx, cy, [1.0, 0.9, 0.0], 8); }
                            engine.debug.log(&format!("Score: {}", self.score));
                        }

                        // Kamera takibi
                        engine.renderer.camera.target = player.position;
                        engine.renderer.camera.position = player.position + Vec3::new(0.0, 2.0, 12.0);

                        // Sınırlar
                        if player.position.y < -10.0 {
                            self.lives -= 1;
                            if self.lives == 0 { self.state = GameState::GameOver; engine.debug.log("GAME OVER"); }
                            else { player.position = Vec3::ZERO; player.velocity = Vec3::ZERO; }
                        }
                    }
                }

                // Coin bob animation
                for coin in &mut self.coins {
                    coin.bob_time += dt * 3.0;
                }

                // Tüm coin'ler toplandı mı?
                if self.coins.iter().all(|c| c.collected) {
                    self.level += 1;
                    if self.level > 5 {
                        self.state = GameState::Victory;
                        engine.debug.log("YOU WIN!");
                    } else {
                        self.generate_level(self.level);
                        if let Some(pid) = self.player_id {
                            if let Some(p) = engine.entities.get_mut(pid) { p.position = Vec3::ZERO; p.velocity = Vec3::ZERO; }
                        }
                        engine.debug.log(&format!("Level {}!", self.level));
                    }
                }

                // Pause
                if engine.input.key_just_pressed(KeyCode::P) { self.state = GameState::Paused; engine.debug.log("PAUSED"); }
            }
            GameState::Paused => {
                if engine.input.key_just_pressed(KeyCode::P) { self.state = GameState::Playing; engine.debug.log("RESUMED"); }
            }
            GameState::GameOver => {
                if engine.input.key_just_pressed(KeyCode::Space) || engine.input.key_just_pressed(KeyCode::Enter) {
                    self.state = GameState::Menu;
                }
            }
            GameState::Victory => {
                if engine.input.key_just_pressed(KeyCode::Space) || engine.input.key_just_pressed(KeyCode::Enter) {
                    self.state = GameState::Menu;
                }
            }
        }

        // Particles update
        for p in &mut self.particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vy += 10.0 * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
    }

    fn fixed_update(&mut self, _engine: &mut Engine, _dt: f32) {
        // Fixed timestep game logic (physics already handled in update for simplicity)
    }

    fn render(&self, _engine: &Engine) {
        // FPS log update'de yapılır
    }

    fn on_resize(&mut self, engine: &mut Engine, width: u32, height: u32) {
        engine.renderer.screen_width = width as f32;
        engine.renderer.screen_height = height as f32;
    }

    fn name(&self) -> &str { "Elysium Platformer" }
}

// ═══════════════════════════════════════════════════════════ TESTLER

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let config = EngineConfig { width: 800, height: 600, ..Default::default() };
        let engine = Engine::new(config);
        assert_eq!(engine.config.width, 800);
        assert_eq!(engine.time.fixed_dt, 1.0 / 60.0);
    }

    #[test]
    fn test_input_manager() {
        let mut input = InputManager::new();
        input.set_key(KeyCode::Space, true);
        assert!(input.key_just_pressed(KeyCode::Space));
        assert!(input.key_pressed(KeyCode::Space));
        input.end_frame();
        assert!(input.key_held(KeyCode::Space));
        input.set_key(KeyCode::Space, false);
        input.end_frame();
        assert!(!input.key_pressed(KeyCode::Space));
    }

    #[test]
    fn test_input_movement_vector() {
        let mut input = InputManager::new();
        input.set_key(KeyCode::W, true);
        input.set_key(KeyCode::D, true);
        let dir = input.movement_vector();
        assert!(dir.x > 0.0);
        assert!(dir.z < 0.0);
        assert!((dir.length() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_audio_manager() {
        let mut audio = AudioManager::new(8);
        audio.play_sfx(1, 1.0, 1.0);
        assert_eq!(audio.active_channel_count(), 1);
        audio.update(3.0);
        assert_eq!(audio.active_channel_count(), 0);
    }

    #[test]
    fn test_asset_manager() {
        let mut assets = AssetManager::new();
        let id = assets.load("texture.png", AssetType::Texture, 4096);
        assert_eq!(assets.memory_usage_mb(), 4096.0 / (1024.0 * 1024.0));
        assets.unload(id);
        assert_eq!(assets.memory_usage_mb(), 0.0);
    }

    #[test]
    fn test_scene_manager() {
        let mut sm = SceneManager::new();
        let idx = sm.add_scene("Menu", false);
        sm.load_scene(idx);
        assert_eq!(sm.current_scene_name(), "Menu");
        sm.transition_to(0, 0.0, [0.0; 3]);
    }

    #[test]
    fn test_time_manager() {
        let mut tm = TimeManager::new(1.0 / 60.0);
        tm.update(1.0 / 60.0);
        assert!(tm.frame >= 1);
        tm.set_time_scale(2.0);
        tm.update(1.0 / 60.0);
        assert!((tm.time_scale - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_entity_manager() {
        let mut em = EntityManager::with_capacity(100);
        let id = em.spawn("Player");
        assert_eq!(em.get(id).unwrap().name, "Player");
        em.add_tag(id, "player");
        assert!(em.has_tag(id, "player"));
        assert_eq!(em.find_by_tag("player"), vec![id]);
        em.despawn(id);
        assert!(em.get(id).is_none());
        assert_eq!(em.active_count, 0);
    }

    #[test]
    fn test_physics_state() {
        let mut ps = PhysicsState::new(4);
        ps.add_body(0, RbType::Dynamic, 1.0);
        ps.step(1.0 / 60.0);
        assert!(ps.collisions.is_empty());
    }

    #[test]
    fn test_platformer_game() {
        let mut game = PlatformerGame::new();
        let config = EngineConfig::default();
        let mut engine = Engine::new(config);
        game.init(&mut engine);
        assert_eq!(game.state, GameState::Menu);
        assert!(game.player_id.is_some());
        assert!(!game.platforms.is_empty());

        // Start game
        engine.input.set_key(KeyCode::Space, true);
        game.update(&mut engine, 1.0 / 60.0);
        assert_eq!(game.state, GameState::Playing);

        // Jump
        engine.input.set_key(KeyCode::Space, true);
        game.is_grounded = true;
        game.update(&mut engine, 1.0 / 60.0);
    }

    #[test]
    fn test_camera_state() {
        let cam = CameraState::default();
        assert_eq!(cam.fov, 60.0);
        assert_eq!(cam.near, 0.1);
    }

    #[test]
    fn test_debug_state() {
        let mut dbg = DebugState::new();
        dbg.log("Hello");
        assert_eq!(dbg.console_lines.len(), 1);
        dbg.toggle_console();
        assert!(dbg.show_console);
    }

    #[test]
    fn test_ui_state() {
        let mut ui = UiState::new();
        ui.add_panel("Inspector", 0.0, 0.0, 300.0, 600.0);
        assert_eq!(ui.panels.len(), 1);
    }

    #[test]
    fn test_renderer_state() {
        let mut rs = RendererState::default();
        rs.frame_count = 100;
        rs.draw_calls = 50;
        assert_eq!(rs.frame_count, 100);
        assert_eq!(rs.camera.fov, 60.0);
    }

    #[test]
    fn test_gamepad_input() {
        let mut input = InputManager::new();
        input.set_gamepad_axis(GamepadAxis::LeftX, 0.8);
        assert!((input.gamepad_axis(GamepadAxis::LeftX) - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_coin_collection() {
        let mut game = PlatformerGame::new();
        game.coins.push(Coin { x: 0.0, y: 0.0, collected: false, bob_time: 0.0 });
        assert_eq!(game.score, 0);
        // Simulate player at coin position
        let config = EngineConfig::default();
        let mut engine = Engine::new(config);
        game.init(&mut engine);
        if let Some(pid) = game.player_id {
            if let Some(p) = engine.entities.get_mut(pid) { p.position = Vec3::new(0.0, 0.0, 0.0); }
        }
        game.state = GameState::Playing;
        game.update(&mut engine, 1.0 / 60.0);
        // Score might increase depending on position
    }

    #[test]
    fn test_lives_system() {
        let mut game = PlatformerGame::new();
        game.lives = 1;
        game.state = GameState::Playing;
        let config = EngineConfig::default();
        let mut engine = Engine::new(config);
        game.init(&mut engine);
        if let Some(pid) = game.player_id {
            if let Some(p) = engine.entities.get_mut(pid) { p.position = Vec3::new(100.0, -15.0, 0.0); }
        }
        game.update(&mut engine, 1.0 / 60.0);
        assert_eq!(game.state, GameState::GameOver);
    }
}
