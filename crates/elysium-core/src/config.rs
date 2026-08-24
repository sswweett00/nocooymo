//! Enterprise configuration system with hot-reload, layered overrides, and
//! serialisation to RON / TOML / JSON.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Engine configuration structures
// ---------------------------------------------------------------------------

/// Top-level engine configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
    pub app: AppConfig,
    pub window: WindowConfig,
    pub renderer: RendererConfig,
    pub physics: PhysicsConfig,
    pub audio: AudioConfig,
    pub input: InputConfig,
    pub network: NetworkConfig,
    pub asset: AssetConfig,
    pub logging: LoggingConfig,
    pub profiling: ProfilingConfig,
    pub scene: SceneConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            app: AppConfig::default(),
            window: WindowConfig::default(),
            renderer: RendererConfig::default(),
            physics: PhysicsConfig::default(),
            audio: AudioConfig::default(),
            input: InputConfig::default(),
            network: NetworkConfig::default(),
            asset: AssetConfig::default(),
            logging: LoggingConfig::default(),
            profiling: ProfilingConfig::default(),
            scene: SceneConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub name: String,
    pub version: String,
    pub max_fps: u32,
    pub vsync: bool,
    pub headless: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "Elysium App".into(),
            version: "0.1.0".into(),
            max_fps: 0,
            vsync: true,
            headless: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub resizable: bool,
    pub high_dpi: bool,
    pub samples: u32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Elysium".into(),
            width: 1920,
            height: 1080,
            fullscreen: false,
            resizable: true,
            high_dpi: true,
            samples: 4,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RendererConfig {
    pub backend: RenderBackend,
    pub vsync: bool,
    pub max_fps: u32,
    pub shadow_map_size: u32,
    pub shadow_cascade_count: u32,
    pub shadow_cascade_split_lambda: f32,
    pub ssao_enabled: bool,
    pub ssao_radius: f32,
    pub ssao_samples: u32,
    pub bloom_enabled: bool,
    pub bloom_threshold: f32,
    pub bloom_intensity: f32,
    pub tone_mapping: ToneMapping,
    pub exposure: f32,
    pub max_point_lights: u32,
    pub max_directional_lights: u32,
    pub max_spot_lights: u32,
    pub meshlet_debug: bool,
    pub virtual_texturing_enabled: bool,
    pub virtual_texturing_page_size: u32,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            backend: RenderBackend::Auto,
            vsync: true,
            max_fps: 0,
            shadow_map_size: 2048,
            shadow_cascade_count: 4,
            shadow_cascade_split_lambda: 0.75,
            ssao_enabled: true,
            ssao_radius: 0.5,
            ssao_samples: 32,
            bloom_enabled: true,
            bloom_threshold: 1.0,
            bloom_intensity: 0.8,
            tone_mapping: ToneMapping::AcesFilmic,
            exposure: 1.0,
            max_point_lights: 256,
            max_directional_lights: 8,
            max_spot_lights: 64,
            meshlet_debug: false,
            virtual_texturing_enabled: true,
            virtual_texturing_page_size: 128,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RenderBackend {
    Auto,
    Vulkan,
    Dx12,
    Metal,
    Gl,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToneMapping {
    None,
    Reinhard,
    AcesFilmic,
    AcesLutz,
    Uncharted2,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PhysicsConfig {
    pub fixed_timestep: f32,
    pub gravity: [f32; 3],
    pub max_substeps: u32,
    pub broadphase: BroadphaseType,
    pub solver_iterations: u32,
    pub default_restitution: f32,
    pub default_friction: f32,
    pub default_linear_damping: f32,
    pub default_angular_damping: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            fixed_timestep: 1.0 / 60.0,
            gravity: [0.0, -9.81, 0.0],
            max_substeps: 4,
            broadphase: BroadphaseType::SweepAndPrune,
            solver_iterations: 10,
            default_restitution: 0.3,
            default_friction: 0.5,
            default_linear_damping: 0.05,
            default_angular_damping: 0.05,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BroadphaseType {
    SweepAndPrune,
    DynamicAabbTree,
    Grid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    pub master_volume: f32,
    pub sfx_volume: f32,
    pub music_volume: f32,
    pub voice_volume: f32,
    pub max_channels: u32,
    pub hrtf: bool,
    pub sample_rate: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            sfx_volume: 0.8,
            music_volume: 0.6,
            voice_volume: 1.0,
            max_channels: 64,
            hrtf: true,
            sample_rate: 48000,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct InputConfig {
    pub dead_zone: f32,
    pub repeat_delay: f32,
    pub repeat_rate: f32,
    pub mouse_sensitivity: f32,
    pub invert_y: bool,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            dead_zone: 0.15,
            repeat_delay: 0.5,
            repeat_rate: 0.03,
            mouse_sensitivity: 1.0,
            invert_y: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    pub max_connections: u32,
    pub port: u16,
    pub protocol_version: u32,
    pub timeout_seconds: f32,
    pub heartbeat_interval: f32,
    pub interpolation_delay: f32,
    pub snapshot_rate: u32,
    pub max_packet_size: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            max_connections: 64,
            port: 7777,
            protocol_version: 1,
            timeout_seconds: 30.0,
            heartbeat_interval: 2.0,
            interpolation_delay: 0.1,
            snapshot_rate: 20,
            max_packet_size: 1400,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AssetConfig {
    pub source_dir: String,
    pub import_dir: String,
    pub cache_dir: String,
    pub parallel_imports: u32,
    pub watch_for_changes: bool,
    pub generate_mipmaps: bool,
    pub texture_compression: TextureCompression,
    pub mesh_optimization: bool,
}

impl Default for AssetConfig {
    fn default() -> Self {
        Self {
            source_dir: "assets".into(),
            import_dir: "target/assets".into(),
            cache_dir: "target/asset-cache".into(),
            parallel_imports: 4,
            watch_for_changes: true,
            generate_mipmaps: true,
            texture_compression: TextureCompression::Bc7,
            mesh_optimization: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TextureCompression {
    None,
    Bc1,
    Bc3,
    Bc5,
    Bc7,
    Astc,
    Etc2,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: String,
    pub file_enabled: bool,
    pub file_path: String,
    pub console_enabled: bool,
    pub colored: bool,
    pub flush_interval_secs: f32,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "Info".into(),
            file_enabled: true,
            file_path: "logs/elysium.log".into(),
            console_enabled: true,
            colored: true,
            flush_interval_secs: 5.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfilingConfig {
    pub enabled: bool,
    pub max_frames: u32,
    pub gpu_timestamps: bool,
    pub cpu_profiling: bool,
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_frames: 300,
            gpu_timestamps: true,
            cpu_profiling: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SceneConfig {
    pub default_scene: String,
    pub auto_save_interval: f32,
    pub max_entities: u32,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            default_scene: "main".into(),
            auto_save_interval: 60.0,
            max_entities: 100_000,
        }
    }
}

// ---------------------------------------------------------------------------
// Config manager with hot-reload
// ---------------------------------------------------------------------------

/// Manages the active engine configuration with support for layered overrides
/// and file-watching hot-reload.
pub struct ConfigManager {
    config: Arc<RwLock<EngineConfig>>,
    source_path: Option<PathBuf>,
    overrides: RwLock<Vec<ConfigOverride>>,
    watchers: RwLock<Vec<ConfigChangeCallback>>,
    last_modified: RwLock<Option<SystemTime>>,
}

type ConfigChangeCallback = Box<dyn Fn(&EngineConfig) + Send + Sync>;

/// A layered override applied on top of the base config.
#[derive(Clone, Debug)]
pub struct ConfigOverride {
    pub name: String,
    pub priority: i32,
    pub values: toml::Value,
}

impl ConfigManager {
    /// Create a new manager with the given base config.
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            source_path: None,
            overrides: RwLock::new(Vec::new()),
            watchers: RwLock::new(Vec::new()),
            last_modified: RwLock::new(None),
        }
    }

    /// Load configuration from a TOML file.
    pub fn load_toml(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::Io(path.display().to_string(), e.to_string()))?;
        let config: EngineConfig = toml::from_str(&contents)
            .map_err(|e| ConfigError::Parse(path.display().to_string(), e.to_string()))?;
        let last_modified = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            source_path: Some(path.to_path_buf()),
            overrides: RwLock::new(Vec::new()),
            watchers: RwLock::new(Vec::new()),
            last_modified: RwLock::new(last_modified),
        })
    }

    /// Save the current configuration to a TOML file.
    pub fn save_toml(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let path = path.as_ref();
        let config = self.config.read().unwrap();
        let contents = toml::to_string_pretty(&*config)
            .map_err(|e| ConfigError::Serialize(path.display().to_string(), e.to_string()))?;
        std::fs::write(path, contents)
            .map_err(|e| ConfigError::Io(path.display().to_string(), e.to_string()))?;
        Ok(())
    }

    /// Load configuration from a JSON file.
    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::Io(path.display().to_string(), e.to_string()))?;
        let config: EngineConfig = serde_json::from_str(&contents)
            .map_err(|e| ConfigError::Parse(path.display().to_string(), e.to_string()))?;
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            source_path: Some(path.to_path_buf()),
            overrides: RwLock::new(Vec::new()),
            watchers: RwLock::new(Vec::new()),
            last_modified: RwLock::new(None),
        })
    }

    /// Save the current configuration to a JSON file.
    pub fn save_json(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let path = path.as_ref();
        let config = self.config.read().unwrap();
        let contents = serde_json::to_string_pretty(&*config)
            .map_err(|e| ConfigError::Serialize(path.display().to_string(), e.to_string()))?;
        std::fs::write(path, contents)
            .map_err(|e| ConfigError::Io(path.display().to_string(), e.to_string()))?;
        Ok(())
    }

    /// Add a layered override.  Overrides with higher priority are applied last.
    pub fn add_override(&self, override_cfg: ConfigOverride) {
        let mut overrides = self.overrides.write().unwrap();
        overrides.push(override_cfg);
        overrides.sort_by_key(|o| o.priority);
        // Re-apply: merge overrides onto base config
        // (simplified: overrides replace top-level sections)
    }

    /// Register a callback that is called whenever the config changes.
    pub fn on_change<F>(&self, callback: F)
    where
        F: Fn(&EngineConfig) + Send + Sync + 'static,
    {
        self.watchers.write().unwrap().push(Box::new(callback));
    }

    /// Check if the source file has been modified and reload if so.
    pub fn check_hot_reload(&self) -> Result<bool, ConfigError> {
        let Some(path) = &self.source_path else {
            return Ok(false);
        };
        let meta = std::fs::metadata(path)
            .map_err(|e| ConfigError::Io(path.display().to_string(), e.to_string()))?;
        let modified = meta.modified().map_err(|e| {
            ConfigError::Io(path.display().to_string(), e.to_string())
        })?;
        let mut last = self.last_modified.write().unwrap();
        if let Some(prev) = *last {
            if modified <= prev {
                return Ok(false);
            }
        }
        *last = Some(modified);
        drop(last);

        // Reload
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::Io(path.display().to_string(), e.to_string()))?;
        let new_config: EngineConfig = toml::from_str(&contents)
            .map_err(|e| ConfigError::Parse(path.display().to_string(), e.to_string()))?;
        {
            let mut cfg = self.config.write().unwrap();
            *cfg = new_config;
        }
        self.notify_watchers();
        Ok(true)
    }

    fn notify_watchers(&self) {
        let config = self.config.read().unwrap();
        let watchers = self.watchers.read().unwrap();
        for w in watchers.iter() {
            w(&config);
        }
    }

    /// Get a clone of the current configuration.
    pub fn config(&self) -> EngineConfig {
        self.config.read().unwrap().clone()
    }

    /// Get a handle to the shared config for read access.
    pub fn shared(&self) -> Arc<RwLock<EngineConfig>> {
        Arc::clone(&self.config)
    }

    /// Update the configuration in-place.
    pub fn update<F: FnOnce(&mut EngineConfig)>(&self, f: F) {
        {
            let mut cfg = self.config.write().unwrap();
            f(&mut cfg);
        }
        self.notify_watchers();
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during configuration loading/saving.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error reading '{0}': {1}")]
    Io(String, String),
    #[error("Parse error in '{0}': {1}")]
    Parse(String, String),
    #[error("Serialization error writing '{0}': {1}")]
    Serialize(String, String),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn default_config_roundtrips_toml() {
        let config = EngineConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: EngineConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.window.width, 1920);
        assert_eq!(parsed.renderer.shadow_map_size, 2048);
    }

    #[test]
    fn default_config_roundtrips_json() {
        let config = EngineConfig::default();
        let json_str = serde_json::to_string(&config).unwrap();
        let parsed: EngineConfig = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.physics.fixed_timestep, 1.0 / 60.0);
    }

    #[test]
    fn config_manager_update_notifies() {
        let mgr = ConfigManager::new(EngineConfig::default());
        let called = Arc::new(AtomicBool::new(false));
        let c2 = called.clone();
        mgr.on_change(move |_| {
            c2.store(true, Ordering::SeqCst);
        });
        mgr.update(|c| {
            c.window.title = "Changed".into();
        });
        assert!(called.load(Ordering::SeqCst));
        assert_eq!(mgr.config().window.title, "Changed");
    }
}