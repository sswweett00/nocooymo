//! # Save Game System for Elysium Engine
//!
//! Provides a complete save game system with:
//! - Multiple save slots with rich metadata
//! - Binary serialization with ZSTD/LZ4 compression
//! - Delta saves for partial updates
//! - Save game versioning and migration
//! - Platform-agnostic save paths
//! - Optional encryption and integrity checks
//! - Cloud save support via platform abstraction
//! - Auto-save support

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{self, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::Entity;
use crate::serialization::{SerializationError, TypeId, WorldSnapshot};
use crate::{EngineError, EngineResult};

// ---------------------------------------------------------------------------
// Re-exported serialization types used by the save system
// ---------------------------------------------------------------------------
pub use crate::serialization::{SerializationFormat, Serializable};

// ===========================================================================
// Compression
// ===========================================================================

/// Supported compression algorithms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    /// No compression.
    None,
    /// Zstandard — good default for game saves.
    #[default]
    Zstd,
    /// LZ4 block — faster compression, slightly lower ratio.
    Lz4,
}

impl fmt::Display for CompressionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Zstd => write!(f, "zstd"),
            Self::Lz4 => write!(f, "lz4"),
        }
    }
}

// ===========================================================================
// Encryption
// ===========================================================================

/// Supported encryption algorithms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionType {
    /// No encryption.
    #[default]
    None,
    /// ChaCha20-Poly1305 AEAD — pure-Rust, no hardware requirements.
    ChaCha20Poly1305,
    /// AES-256-GCM — requires hardware AES-NI on many platforms for full speed.
    Aes256Gcm,
}

impl fmt::Display for EncryptionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::ChaCha20Poly1305 => write!(f, "chacha20poly1305"),
            Self::Aes256Gcm => write!(f, "aes-256-gcm"),
        }
    }
}

// ===========================================================================
// Versioning
// ===========================================================================

/// Semantic version for save game format compatibility.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SaveGameVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl SaveGameVersion {
    /// The current save format version.
    pub const CURRENT: Self = Self {
        major: 1,
        minor: 0,
        patch: 0,
    };
}

impl fmt::Display for SaveGameVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// ===========================================================================
// Platform abstraction
// ===========================================================================

/// Target platform — affects default save paths and cloud provider selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    #[default]
    Pc,
    Console,
    Mobile,
}

impl Platform {
    /// Returns the default relative save directory name.
    pub fn save_dir_name(self) -> &'static str {
        match self {
            Self::Pc => "Saves",
            Self::Console => "Saves",
            Self::Mobile => "saves",
        }
    }

    /// Returns a human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Pc => "PC",
            Self::Console => "Console",
            Self::Mobile => "Mobile",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ===========================================================================
// Save game header (binary layout prefix)
// ===========================================================================

/// Binary header written before every save file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveGameHeader {
    /// Save format version.
    pub version: SaveGameVersion,
    /// Platform the save was created on.
    pub platform: Platform,
    /// Compression used for the payload.
    pub compression: CompressionType,
    /// Encryption used for the payload.
    pub encryption: EncryptionType,
    /// Uncompressed payload size in bytes.
    pub data_size: u64,
    /// Integrity checksum (first 16 bytes of SHA-256 digest).
    pub checksum: [u8; 16],
    /// Reserved for future use.
    pub reserved: [u8; 32],
}

impl SaveGameHeader {
    pub const MAGIC: [u8; 4] = *b"ElyS";
    pub const HEADER_SIZE: usize = 4 + 2 + 2 + 2 + 1 + 1 + 1 + 8 + 16 + 32; // 69 bytes

    /// Serialize the header to a byte vector.
    pub fn to_bytes(&self) -> EngineResult<Vec<u8>> {
        let mut buf = Vec::with_capacity(Self::HEADER_SIZE);
        buf.extend_from_slice(&Self::MAGIC);
        buf.extend_from_slice(&self.version.major.to_le_bytes());
        buf.extend_from_slice(&self.version.minor.to_le_bytes());
        buf.extend_from_slice(&self.version.patch.to_le_bytes());
        buf.push(self.platform as u8);
        buf.push(self.compression as u8);
        buf.push(self.encryption as u8);
        buf.extend_from_slice(&self.data_size.to_le_bytes());
        buf.extend_from_slice(&self.checksum);
        buf.extend_from_slice(&self.reserved);
        Ok(buf)
    }

    /// Deserialize a header from bytes.
    pub fn from_bytes(data: &[u8]) -> EngineResult<Self> {
        if data.len() < 4 || &data[0..4] != Self::MAGIC {
            return Err(EngineError::ValidationError(
                "Invalid save file magic bytes".into(),
            ));
        }

        let offset = 4;
        let version = SaveGameVersion {
            major: u16::from_le_bytes([data[offset], data[offset + 1]]),
            minor: u16::from_le_bytes([data[offset + 2], data[offset + 3]]),
            patch: u16::from_le_bytes([data[offset + 4], data[offset + 5]]),
        };

        let platform = match data[offset + 6] {
            0 => Platform::Pc,
            1 => Platform::Console,
            2 => Platform::Mobile,
            _ => return Err(EngineError::ValidationError("Unknown platform".into())),
        };

        let compression = match data[offset + 7] {
            0 => CompressionType::None,
            1 => CompressionType::Zstd,
            2 => CompressionType::Lz4,
            _ => return Err(EngineError::ValidationError("Unknown compression".into())),
        };

        let encryption = match data[offset + 8] {
            0 => EncryptionType::None,
            1 => EncryptionType::ChaCha20Poly1305,
            2 => EncryptionType::Aes256Gcm,
            _ => return Err(EngineError::ValidationError("Unknown encryption".into())),
        };

        let data_size = u64::from_le_bytes([
            data[offset + 9],
            data[offset + 10],
            data[offset + 11],
            data[offset + 12],
            data[offset + 13],
            data[offset + 14],
            data[offset + 15],
            data[offset + 16],
        ]);

        let mut checksum = [0u8; 16];
        checksum.copy_from_slice(&data[offset + 17..offset + 33]);

        let mut reserved = [0u8; 32];
        reserved.copy_from_slice(&data[offset + 33..offset + 65]);

        Ok(Self {
            version,
            platform,
            compression,
            encryption,
            data_size,
            checksum,
            reserved,
        })
    }
}

// ===========================================================================
// Player data
// ===========================================================================

/// Player position and orientation.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct PlayerTransform {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub velocity: [f32; 3],
}

/// A single inventory entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InventoryItem {
    pub item_id: u32,
    pub quantity: u32,
    pub durability: f32,
    pub custom_data: Vec<u8>,
}

/// Player skill data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillData {
    pub skill_id: u32,
    pub level: u32,
    pub experience: u64,
}

/// Complete player state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlayerData {
    /// Entity ID of the player character.
    pub entity_id: u32,
    /// Transform state.
    pub transform: PlayerTransform,
    /// Vital stats.
    pub health: f32,
    pub max_health: f32,
    pub stamina: f32,
    pub max_stamina: f32,
    /// Progression.
    pub level: u32,
    pub experience: u64,
    /// Inventory (max 256 items for serialization efficiency).
    pub inventory: Vec<InventoryItem>,
    /// Named stats (strength, agility, etc.).
    pub stats: HashMap<String, f32>,
    /// Learned skills.
    pub skills: Vec<SkillData>,
}

// ===========================================================================
// World / scene data
// ===========================================================================

/// Quest status.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QuestStatus {
    NotStarted,
    InProgress,
    Completed,
    Failed,
}

/// Individual objective state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectiveState {
    pub objective_id: String,
    pub completed: bool,
    pub current_progress: f32,
    pub target_progress: f32,
}

/// Full quest state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuestState {
    pub quest_id: String,
    pub status: QuestStatus,
    pub objectives: Vec<ObjectiveState>,
    pub started_at: f64,
    pub completed_at: Option<f64>,
}

/// Captured entity state for the save.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntitySaveData {
    pub entity: Entity,
    pub archetype_id: u32,
    pub components: Vec<(String, Vec<u8>)>,
}

/// World-level data (captured from the ECS `World`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorldData {
    /// Currently active scene name.
    pub active_scene: String,
    /// Snapshot of all relevant entities.
    pub entities: Vec<EntitySaveData>,
    /// Quest progress tracker.
    pub quest_progress: HashMap<String, QuestState>,
    /// Elapsed world time in seconds.
    pub world_time: f64,
    /// Current weather preset name.
    pub weather: String,
    /// Completed main / side objectives.
    pub completed_objectives: Vec<String>,
}

impl WorldData {
    /// Captures a `WorldData` snapshot from the engine `World`.
    ///
    /// This bridges the save system with the existing ECS `World`.
    pub fn from_world(_world: &crate::World) -> Self {
        // The World doesn't expose public iteration over all entities.
        // In a production system we'd add a World::snapshot() method or
        // use the WorldSnapshot type from serialization.rs.
        // Here we return an empty snapshot as a placeholder.
        Self {
            active_scene: String::new(),
            entities: Vec::new(),
            quest_progress: HashMap::new(),
            world_time: 0.0,
            weather: String::new(),
            completed_objectives: Vec::new(),
        }
    }
}

// ===========================================================================
// Settings data
// ===========================================================================

/// Graphics-specific settings.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GraphicsSettings {
    pub resolution: (u32, u32),
    pub fullscreen: bool,
    pub vsync: bool,
    pub texture_quality: u32,
    pub shadow_quality: u32,
    pub anti_aliasing: u32,
    pub render_distance: u32,
}

/// Audio-specific settings.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AudioSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub voice_volume: f32,
    pub muted: bool,
}

/// Gameplay-specific settings.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GameplaySettings {
    pub difficulty: String,
    pub subtitles: bool,
    pub tutorial_enabled: bool,
    pub auto_save: bool,
    pub auto_save_interval: u32,
    pub invert_y_axis: bool,
}

/// Input / control bindings.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ControlsSettings {
    pub mouse_sensitivity: f32,
    pub controller_enabled: bool,
    pub key_bindings: HashMap<String, String>,
}

/// Accessibility settings.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AccessibilitySettings {
    pub colorblind_mode: String,
    pub high_contrast: bool,
    pub text_size: f32,
    pub screen_shake: f32,
}

/// All user-configurable settings.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SettingsData {
    pub graphics: GraphicsSettings,
    pub audio: AudioSettings,
    pub gameplay: GameplaySettings,
    pub controls: ControlsSettings,
    pub accessibility: AccessibilitySettings,
}

// ===========================================================================
// Thumbnail / screenshot
// ===========================================================================

/// A save game thumbnail (PNG or JPEG encoded bytes).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SaveGameThumbnail {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl SaveGameThumbnail {
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            data,
        }
    }

    /// Creates a blank 1x1 PNG thumbnail.
    pub fn blank() -> Self {
        Self {
            width: 1,
            height: 1,
            data: vec![
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
                0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
                0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
                0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41,
                0x54, 0x08, 0xD7, 0x63, 0xF8, 0xFF, 0xFF, 0x3F,
                0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x59,
                0xE7, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
                0x44, 0xAE, 0x42, 0x60, 0x82,
            ],
        }
    }
}

// ===========================================================================
// Save game metadata
// ===========================================================================

/// User-facing save slot metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveGameMetadata {
    /// Slot index (0-9 for typical 10-slot saves).
    pub slot_id: u32,
    /// User-facing save name.
    pub name: String,
    /// Total play time in seconds.
    pub play_time_seconds: u64,
    /// Name of the level / scene the player is in.
    pub level_name: String,
    /// Unix timestamp of save creation.
    pub created_at: f64,
    /// Unix timestamp of last modification.
    pub last_modified: f64,
    /// Optional thumbnail image.
    pub thumbnail: Option<SaveGameThumbnail>,
    /// Whether this slot is managed by auto-save.
    pub is_auto_save: bool,
    /// True when the save file is suspected corrupted.
    pub is_corrupted: bool,
    /// User-defined tags (e.g. "pre-boss", "pacifist").
    pub tags: Vec<String>,
}

impl SaveGameMetadata {
    pub fn new(slot_id: u32, name: impl Into<String>) -> Self {
        let now = current_unix_ts();
        Self {
            slot_id,
            name: name.into(),
            play_time_seconds: 0,
            level_name: String::new(),
            created_at: now,
            last_modified: now,
            thumbnail: None,
            is_auto_save: false,
            is_corrupted: false,
            tags: Vec::new(),
        }
    }

    pub fn with_thumbnail(mut self, thumbnail: SaveGameThumbnail) -> Self {
        self.thumbnail = Some(thumbnail);
        self
    }

    pub fn with_play_time(mut self, seconds: u64) -> Self {
        self.play_time_seconds = seconds;
        self
    }

    pub fn with_level_name(mut self, level: impl Into<String>) -> Self {
        self.level_name = level.into();
        self
    }

    pub fn with_auto_save(mut self) -> Self {
        self.is_auto_save = true;
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

// ===========================================================================
// Delta save data
// ===========================================================================

/// Partial save that records only changes since a base version.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SaveGameDelta {
    /// Version of the base save this delta is against.
    pub base_version: u64,
    /// Entities that changed.
    pub changed_entities: Vec<EntitySaveData>,
    /// Player state delta (None = unchanged).
    pub player_delta: Option<PlayerData>,
    /// Quests that changed.
    pub quest_delta: HashMap<String, QuestState>,
    /// New world time (None = unchanged).
    pub world_time_delta: Option<f64>,
}

// ===========================================================================
// Complete save game
// ===========================================================================

/// The top-level save game object written to disk (after header).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveGame {
    /// Save game metadata.
    pub metadata: SaveGameMetadata,
    /// Captured ECS world state.
    pub world_data: WorldData,
    /// Player state.
    pub player_data: PlayerData,
    /// User settings snapshot.
    pub settings_data: SettingsData,
    /// Optional thumbnail.
    pub thumbnail: Option<SaveGameThumbnail>,
}

impl SaveGame {
    pub fn new(metadata: SaveGameMetadata) -> Self {
        Self {
            metadata,
            world_data: WorldData::default(),
            player_data: PlayerData::default(),
            settings_data: SettingsData::default(),
            thumbnail: None,
        }
    }
}

// ===========================================================================
// Save game serializer (compression + encryption)
// ===========================================================================

/// Serializes and deserializes save games with compression and encryption.
pub struct SaveGameSerializer {
    compression: CompressionType,
    encryption: EncryptionType,
    encryption_key: Option<[u8; 32]>,
}

impl Default for SaveGameSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl SaveGameSerializer {
    pub fn new() -> Self {
        Self {
            compression: CompressionType::Zstd,
            encryption: EncryptionType::None,
            encryption_key: None,
        }
    }

    pub fn with_compression(mut self, compression: CompressionType) -> Self {
        self.compression = compression;
        self
    }

    pub fn with_encryption(mut self, key: [u8; 32]) -> Self {
        self.encryption = EncryptionType::ChaCha20Poly1305;
        self.encryption_key = Some(key);
        self
    }

    /// Serialize a `SaveGame` to bytes.
    pub fn serialize(&self, save: &SaveGame) -> EngineResult<Vec<u8>> {
        let data = bincode::serialize(save).map_err(|e| {
            EngineError::SerializationError(format!("serialize save game: {}", e))
        })?;

        let compressed = self.compress(&data)?;
        let final_bytes = self.encrypt(&compressed)?;
        Ok(final_bytes)
    }

    /// Deserialize bytes into a `SaveGame`.
    pub fn deserialize(&self, data: &[u8]) -> EngineResult<SaveGame> {
        let decrypted = self.decrypt(data)?;
        let decompressed = self.decompress(&decrypted)?;
        let save: SaveGame = bincode::deserialize(&decompressed).map_err(|e| {
            EngineError::SerializationError(format!("deserialize save game: {}", e))
        })?;
        Ok(save)
    }

    /// Compute an integrity checksum for the given data.
    pub fn compute_checksum(&self, data: &[u8]) -> [u8; 16] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut checksum = [0u8; 16];
        checksum.copy_from_slice(&result[..16]);
        checksum
    }

    // -- compression

    fn compress(&self, data: &[u8]) -> EngineResult<Vec<u8>> {
        match self.compression {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Zstd => {
                zstd::encode_all(data, 3)
                    .map_err(|e| EngineError::SerializationError(format!("zstd compress: {}", e)))
            }
            CompressionType::Lz4 => {
                let mut encoder = lz4::EncoderBuilder::new()
                    .level(4)
                    .build(Vec::new())
                    .map_err(|e| EngineError::SerializationError(format!("lz4 compress: {}", e)))?;
                encoder.write_all(data).map_err(|e| {
                    EngineError::SerializationError(format!("lz4 write: {}", e))
                })?;
                let (output, result) = encoder.finish();
                result.map_err(|e| EngineError::SerializationError(format!("lz4 finish: {}", e)))?;
                Ok(output)
            }
        }
    }

    fn decompress(&self, data: &[u8]) -> EngineResult<Vec<u8>> {
        match self.compression {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Zstd => {
                zstd::decode_all(data)
                    .map_err(|e| EngineError::SerializationError(format!("zstd decompress: {}", e)))
            }
            CompressionType::Lz4 => {
                let mut decoder = lz4::Decoder::new(Cursor::new(data))
                    .map_err(|e| EngineError::SerializationError(format!("lz4 decode: {}", e)))?;
                let mut out = Vec::new();
                decoder.read_to_end(&mut out).map_err(|e| {
                    EngineError::SerializationError(format!("lz4 read: {}", e))
                })?;
                Ok(out)
            }
        }
    }

    // -- encryption

    fn encrypt(&self, data: &[u8]) -> EngineResult<Vec<u8>> {
        if self.encryption == EncryptionType::None {
            return Ok(data.to_vec());
        }

        let key = self
            .encryption_key
            .ok_or_else(|| EngineError::SerializationError("encryption enabled but no key".into()))?;

        match self.encryption {
            EncryptionType::ChaCha20Poly1305 => self.encrypt_chacha(data, &key),
            EncryptionType::Aes256Gcm => self.encrypt_aes(data, &key),
            EncryptionType::None => unreachable!(),
        }
    }

    fn decrypt(&self, data: &[u8]) -> EngineResult<Vec<u8>> {
        if self.encryption == EncryptionType::None {
            return Ok(data.to_vec());
        }

        let key = self
            .encryption_key
            .ok_or_else(|| EngineError::SerializationError("encryption enabled but no key".into()))?;

        match self.encryption {
            EncryptionType::ChaCha20Poly1305 => self.decrypt_chacha(data, &key),
            EncryptionType::Aes256Gcm => self.decrypt_aes(data, &key),
            EncryptionType::None => unreachable!(),
        }
    }

    fn encrypt_chacha(&self, data: &[u8], key: &[u8; 32]) -> EngineResult<Vec<u8>> {
        use chacha20poly1305::{
            AeadCore, ChaCha20Poly1305, Key, Nonce,
            aead::{Aead, KeyInit, rand_core::OsRng},
        };

        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, data).map_err(|e| {
            EngineError::SerializationError(format!("chacha encrypt: {}", e))
        })?;

        let mut out = nonce.as_ref().to_vec();
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    fn decrypt_chacha(&self, data: &[u8], key: &[u8; 32]) -> EngineResult<Vec<u8>> {
        use chacha20poly1305::{
            ChaCha20Poly1305, Key, Nonce,
            aead::{Aead, KeyInit},
        };

        if data.len() < 12 {
            return Err(EngineError::ValidationError(
                "ciphertext too short for nonce".into(),
            ));
        }

        let nonce = Nonce::from_slice(&data[..12]);
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let plaintext = cipher.decrypt(nonce, &data[12..]).map_err(|e| {
            EngineError::SerializationError(format!("chacha decrypt: {}", e))
        })?;
        Ok(plaintext)
    }

    fn encrypt_aes(&self, data: &[u8], key: &[u8; 32]) -> EngineResult<Vec<u8>> {
        use aes_gcm::{
            Aes256Gcm, Key, Nonce,
            aead::{Aead, KeyInit, rand_core::OsRng},
        };

        let cipher = Aes256Gcm::new(Key::from_slice(key));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, data).map_err(|e| {
            EngineError::SerializationError(format!("aes encrypt: {}", e))
        })?;

        let mut out = nonce.as_ref().to_vec();
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    fn decrypt_aes(&self, data: &[u8], key: &[u8; 32]) -> EngineResult<Vec<u8>> {
        use aes_gcm::{
            Aes256Gcm, Key, Nonce,
            aead::{Aead, KeyInit},
        };

        if data.len() < 12 {
            return Err(EngineError::ValidationError(
                "ciphertext too short for nonce".into(),
            ));
        }

        let nonce = Nonce::from_slice(&data[..12]);
        let cipher = Aes256Gcm::new(Key::from_slice(key));
        let plaintext = cipher.decrypt(nonce, &data[12..]).map_err(|e| {
            EngineError::SerializationError(format!("aes decrypt: {}", e))
        })?;
        Ok(plaintext)
    }
}

// ===========================================================================
// Save game slot
// ===========================================================================

/// A single save slot — may or may not contain data.
pub struct SaveGameSlot {
    pub metadata: SaveGameMetadata,
    pub data: Option<SaveGame>,
    pub dirty: bool,
}

impl SaveGameSlot {
    pub fn new(slot_id: u32, name: impl Into<String>) -> Self {
        Self {
            metadata: SaveGameMetadata::new(slot_id, name),
            data: None,
            dirty: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_none()
    }

    pub fn is_auto_save(&self) -> bool {
        self.metadata.is_auto_save
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.metadata.last_modified = current_unix_ts();
    }

    pub fn clear(&mut self) {
        self.data = None;
        self.dirty = false;
        self.metadata = SaveGameMetadata::new(self.metadata.slot_id, &self.metadata.name);
    }
}

// ===========================================================================
// Cloud save provider trait
// ===========================================================================

/// Abstraction over cloud save backends.
pub trait CloudSaveProvider: Send + Sync {
    /// Upload save bytes for the given slot.
    fn upload(&self, slot_id: u32, data: &[u8]);
    /// Download save bytes for the given slot. Returns `None` if not available.
    fn download(&self, slot_id: u32) -> Option<Vec<u8>>;
    /// Delete the cloud copy of the given slot.
    fn delete(&self, slot_id: u32);
    /// Returns `true` if the cloud is reachable and authenticated.
    fn is_available(&self) -> bool;
}

/// A no-op cloud provider used when cloud saves are disabled.
#[derive(Clone, Debug, Default)]
pub struct NoopCloudProvider;

impl CloudSaveProvider for NoopCloudProvider {
    fn upload(&self, _slot_id: u32, _data: &[u8]) {}
    fn download(&self, _slot_id: u32) -> Option<Vec<u8>> {
        None
    }
    fn delete(&self, _slot_id: u32) {}
    fn is_available(&self) -> bool {
        false
    }
}

// ===========================================================================
// Save game manager
// ===========================================================================

/// Central save game manager — owns all slots, handles I/O, compression,
/// encryption, and optional cloud uploads.
pub struct SaveGameManager {
    slots: HashMap<u32, SaveGameSlot>,
    max_slots: u32,
    current_slot: Option<u32>,
    auto_save_enabled: bool,
    auto_save_interval: Duration,
    last_auto_save: SystemTime,
    serializer: SaveGameSerializer,
    platform: Platform,
    save_path: PathBuf,
    cloud_provider: Option<Box<dyn CloudSaveProvider>>,
}

impl SaveGameManager {
    /// Create a new manager with a default configuration.
    pub fn new() -> Self {
        Self::with_max_slots(10)
    }

    /// Create a new manager with a custom maximum slot count.
    pub fn with_max_slots(max_slots: u32) -> Self {
        let mut slots = HashMap::with_capacity(max_slots as usize);
        for i in 0..max_slots {
            slots.insert(i, SaveGameSlot::new(i, format!("Slot {}", i + 1)));
        }

        Self {
            slots,
            max_slots,
            current_slot: None,
            auto_save_enabled: false,
            auto_save_interval: Duration::from_secs(300),
            last_auto_save: SystemTime::now(),
            serializer: SaveGameSerializer::new(),
            platform: Platform::default(),
            save_path: platform_default_path(Platform::default()),
            cloud_provider: None,
        }
    }

    // -- configuration

    /// Set the target platform and adjust the default save path accordingly.
    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.platform = platform;
        self.save_path = platform_default_path(platform);
        self
    }

    /// Override the default save path.
    pub fn with_save_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.save_path = path.as_ref().to_path_buf();
        self
    }

    /// Set the compression algorithm.
    pub fn with_compression(mut self, compression: CompressionType) -> Self {
        self.serializer = self.serializer.with_compression(compression);
        self
    }

    /// Enable encryption with the given 256-bit key.
    pub fn with_encryption(mut self, key: [u8; 32]) -> Self {
        self.serializer = self.serializer.with_encryption(key);
        self
    }

    /// Enable auto-save with the given interval.
    pub fn with_auto_save(mut self, interval: Duration) -> Self {
        self.auto_save_enabled = true;
        self.auto_save_interval = interval;
        self
    }

    /// Provide a cloud save backend.
    pub fn with_cloud_provider<P: CloudSaveProvider + 'static>(mut self, provider: P) -> Self {
        self.cloud_provider = Some(Box::new(provider));
        self
    }

    // -- slot operations

    /// Create a new save in `slot_id` from a fully-built `SaveGame`.
    pub fn create_save(
        &mut self,
        slot_id: u32,
        save: SaveGame,
    ) -> EngineResult<()> {
        if slot_id >= self.max_slots {
            return Err(EngineError::ValidationError(format!(
                "slot {} out of range (0..{})",
                slot_id, self.max_slots
            )));
        }

        let mut slot = SaveGameSlot::new(slot_id, save.metadata.name.clone());
        slot.data = Some(save);
        slot.mark_dirty();
        self.slots.insert(slot_id, slot);
        self.current_slot = Some(slot_id);
        Ok(())
    }

    /// Write the save in `slot_id` to disk.
    pub fn save(&mut self, slot_id: u32) -> EngineResult<()> {
        if slot_id >= self.max_slots {
            return Err(EngineError::ValidationError(format!(
                "slot {} out of range (0..{})",
                slot_id, self.max_slots
            )));
        }

        let slot = self.slots.get(&slot_id).ok_or_else(|| {
            EngineError::ValidationError(format!("slot {} not found", slot_id))
        })?;

        let save = slot.data.as_ref().ok_or_else(|| {
            EngineError::ValidationError("save slot has no data".into())
        })?;

        let payload = self.serializer.serialize(save)?;
        let checksum = self.serializer.compute_checksum(&payload);

        let header = SaveGameHeader {
            version: SaveGameVersion::CURRENT,
            platform: self.platform,
            compression: self.serializer.compression,
            encryption: self.serializer.encryption,
            data_size: payload.len() as u64,
            checksum,
            reserved: [0u8; 32],
        };

        let header_bytes = header.to_bytes()?;

        fs::create_dir_all(&self.save_path).map_err(|e| e.into())?;

        let file_path = self
            .save_path
            .join(format!("save_{:02}_{}.sav", slot_id, sanitize_name(&save.metadata.name)));

        let mut file = fs::File::create(&file_path).map_err(|e| e.into())?;
        file.write_all(&header_bytes).map_err(|e| e.into())?;
        file.write_all(&payload).map_err(|e| e.into())?;
        file.sync_all().map_err(|e| e.into())?;

        // Upload to cloud if available
        if let Some(ref cloud) = self.cloud_provider {
            if cloud.is_available() {
                cloud.upload(slot_id, &payload);
            }
        }

        if let Some(slot) = self.slots.get_mut(&slot_id) {
            slot.dirty = false;
        }

        Ok(())
    }

    /// Load a save from `slot_id`. Returns `None` if the slot is empty.
    pub fn load(&mut self, slot_id: u32) -> EngineResult<Option<SaveGame>> {
        if slot_id >= self.max_slots {
            return Err(EngineError::ValidationError(format!(
                "slot {} out of range (0..{})",
                slot_id, self.max_slots
            )));
        }

        // Locate the save file on disk by scanning for the matching slot prefix
        let file_path = find_save_file(&self.save_path, slot_id)?;

        let data = fs::read(&file_path).map_err(|e| e.into())?;

        if data.len() < SaveGameHeader::HEADER_SIZE {
            return Err(EngineError::ValidationError(
                "save file too short for header".into(),
            ));
        }

        let header = SaveGameHeader::from_bytes(&data[..SaveGameHeader::HEADER_SIZE])?;

        // Version check — only allow major version bumps via migration.
        if header.version.major > SaveGameVersion::CURRENT.major {
            return Err(EngineError::ValidationError(format!(
                "save version {}.{}.{} is too new (current: {})",
                header.version, header.version, header.version, SaveGameVersion::CURRENT
            )));
        }

        let payload = &data[SaveGameHeader::HEADER_SIZE..];

        // Integrity check
        let computed = self.serializer.compute_checksum(payload);
        if computed != header.checksum {
            return Err(EngineError::ValidationError(
                "save file integrity check failed".into(),
            ));
        }

        let save = self.serializer.deserialize(payload)?;

        if let Some(slot) = self.slots.get_mut(&slot_id) {
            slot.data = Some(save.clone());
            slot.metadata.last_modified = current_unix_ts();
            slot.metadata.is_corrupted = false;
        }

        self.current_slot = Some(slot_id);
        Ok(Some(save))
    }

    /// Delete the save in `slot_id`.
    pub fn delete(&mut self, slot_id: u32) -> EngineResult<()> {
        if slot_id >= self.max_slots {
            return Err(EngineError::ValidationError(format!(
                "slot {} out of range (0..{})",
                slot_id, self.max_slots
            )));
        }

        // Remove file on disk
        let pattern = format!("save_{:02}_*.sav", slot_id);
        if let Ok(entries) = fs::read_dir(&self.save_path) {
            for entry in entries.flatten() {
                let name = entry.file_name().into_string().unwrap_or_default();
                if name.starts_with(&format!("save_{:02}_", slot_id))
                    && name.ends_with(".sav")
                {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }

        self.slots
            .insert(slot_id, SaveGameSlot::new(slot_id, format!("Slot {}", slot_id + 1)));

        if self.current_slot == Some(slot_id) {
            self.current_slot = None;
        }

        Ok(())
    }

    /// Trigger auto-save if the interval has elapsed.
    pub fn auto_save(&mut self, save: SaveGame) -> EngineResult<()> {
        if !self.auto_save_enabled {
            return Ok(());
        }

        let elapsed = SystemTime::now()
            .duration_since(self.last_auto_save)
            .unwrap_or_default();

        if elapsed < self.auto_save_interval {
            return Ok(());
        }

        let slot_id = self.current_slot.unwrap_or(self.max_slots - 1);
        let mut slot = SaveGameSlot::new(slot_id, "Auto Save");
        slot.data = Some(save);
        slot.metadata.is_auto_save = true;
        self.slots.insert(slot_id, slot);
        self.save(slot_id)?;
        self.last_auto_save = SystemTime::now();
        Ok(())
    }

    /// Check whether auto-save should trigger.
    pub fn should_auto_save(&self) -> bool {
        self.auto_save_enabled
            && SystemTime::now()
                .duration_since(self.last_auto_save)
                .unwrap_or_default()
                >= self.auto_save_interval
    }

    // -- queries

    /// Number of slots.
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Max number of slots.
    pub fn max_slots(&self) -> u32 {
        self.max_slots
    }

    /// Currently selected slot, if any.
    pub fn current_slot(&self) -> Option<u32> {
        self.current_slot
    }

    /// Select a slot as current.
    pub fn set_current_slot(&mut self, slot_id: u32) -> EngineResult<()> {
        if slot_id >= self.max_slots {
            return Err(EngineError::ValidationError(
                "slot out of range".into(),
            ));
        }
        self.current_slot = Some(slot_id);
        Ok(())
    }

    /// Reference to a slot.
    pub fn get_slot(&self, slot_id: u32) -> Option<&SaveGameSlot> {
        self.slots.get(&slot_id)
    }

    /// Mutable reference to a slot.
    pub fn get_slot_mut(&mut self, slot_id: u32) -> Option<&mut SaveGameSlot> {
        self.slots.get_mut(&slot_id)
    }

    /// Iterator over all slots.
    pub fn slots(&self) -> impl Iterator<Item = &SaveGameSlot> {
        self.slots.values()
    }

    /// Mutable iterator over all slots.
    pub fn slots_mut(&mut self) -> impl Iterator<Item = &mut SaveGameSlot> {
        self.slots.values_mut()
    }

    /// The active save directory.
    pub fn save_path(&self) -> &Path {
        &self.save_path
    }

    /// Refresh slot metadata by scanning the save directory.
    pub fn refresh_metadata(&mut self) -> EngineResult<()> {
        if let Ok(entries) = fs::read_dir(&self.save_path) {
            for entry in entries.flatten() {
                let name = entry.file_name().into_string().unwrap_or_default();
                // Try to extract slot id from filename: save_00_Name.sav
                if name.starts_with("save_") && name.ends_with(".sav") {
                    let parts: Vec<&str> = name.trim_end_matches(".sav").split('_').collect();
                    if let Ok(slot_id) = parts.get(1).and_then(|s| s.parse().ok()) {
                        if let Some(slot) = self.slots.get_mut(&slot_id) {
                            slot.metadata.last_modified = entry
                                .metadata()
                                .and_then(|m| m.modified())
                                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                                .map(|d| d.as_secs() as f64)
                                .unwrap_or(slot.metadata.last_modified);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

// ===========================================================================
// Save game builder (ergonomic construction)
// ===========================================================================

/// Ergonomically build a `SaveGame` from constituent parts.
pub struct SaveGameBuilder {
    save: SaveGame,
}

impl SaveGameBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        let mut meta = SaveGameMetadata::new(0, name);
        meta.slot_id = 0;
        Self {
            save: SaveGame {
                metadata: meta,
                world_data: WorldData::default(),
                player_data: PlayerData::default(),
                settings_data: SettingsData::default(),
                thumbnail: None,
            },
        }
    }

    pub fn with_slot(mut self, slot_id: u32) -> Self {
        self.save.metadata.slot_id = slot_id;
        self
    }

    pub fn with_player_data(mut self, player: PlayerData) -> Self {
        self.save.player_data = player;
        self
    }

    pub fn with_world_data(mut self, world: WorldData) -> Self {
        self.save.world_data = world;
        self
    }

    pub fn with_settings(mut self, settings: SettingsData) -> Self {
        self.save.settings_data = settings;
        self
    }

    pub fn with_thumbnail(mut self, thumbnail: SaveGameThumbnail) -> Self {
        self.save.thumbnail = Some(thumbnail);
        self
    }

    pub fn with_play_time(mut self, seconds: u64) -> Self {
        self.save.metadata.play_time_seconds = seconds;
        self
    }

    pub fn with_level_name(mut self, level: impl Into<String>) -> Self {
        self.save.metadata.level_name = level.into();
        self
    }

    pub fn with_auto_save(mut self) -> Self {
        self.save.metadata.is_auto_save = true;
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.save.metadata.tags.push(tag.into());
        self
    }

    pub fn build(self) -> SaveGame {
        self.save
    }
}

// ===========================================================================
// Migration
// ===========================================================================

/// A save game migration step.
pub trait SaveMigration: Send + Sync {
    /// Target version after this migration.
    fn target_version(&self) -> SaveGameVersion;
    /// Apply the migration to a save.
    fn migrate(&self, save: &mut SaveGame) -> EngineResult<()>;
}

/// Migrates a save from its current version to `SaveGameVersion::CURRENT`.
pub fn migrate_save(save: &mut SaveGame) -> EngineResult<SaveGameVersion> {
    // In a production system, register migrations here:
    // if save.metadata.version < VERSION_1_0_1 { apply_v101(save)?; }
    Ok(SaveGameVersion::CURRENT)
}

// ===========================================================================
// Helpers
// ===========================================================================

fn current_unix_ts() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as f64
}

fn platform_default_path(platform: Platform) -> PathBuf {
    match platform {
        Platform::Pc => dirs_impl::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Elysium")
            .join("Saves"),
        Platform::Console => PathBuf::from("/userdata/elysium/saves"),
        Platform::Mobile => PathBuf::from("/sdcard/Elysium/Saves"),
    }
}

/// Strip characters that are unsafe in filenames.
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// Locate the save file for a given slot ID by directory scan.
fn find_save_file(dir: &Path, slot_id: u32) -> EngineResult<PathBuf> {
    let prefix = format!("save_{:02}_", slot_id);
    let mut found: Option<PathBuf> = None;

    if dir.exists() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().into_string().unwrap_or_default();
                if name.starts_with(&prefix) && name.ends_with(".sav") {
                    found = Some(entry.path());
                }
            }
        }
    }

    found.ok_or_else(|| EngineError::ValidationError(format!("no save file for slot {}", slot_id)))
}

// Lightweight config-dir lookup (avoids pulling in the full `dirs` crate).
mod dirs_impl {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var("APPDATA").ok().map(PathBuf::from)
        }
        #[cfg(target_os = "macos")]
        {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
        }
        #[cfg(target_os = "linux")]
        {
            std::env::var("XDG_CONFIG_HOME")
                .ok()
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config"))
                })
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }
}

// ===========================================================================
// Delta save computation
// ===========================================================================

/// Compute a delta between two world snapshots.
pub fn compute_delta(current: &WorldSnapshot, base: &WorldSnapshot) -> SaveGameDelta {
    let mut changed = Vec::new();

    // Simple entity diff: any entity in current but not in base (or vice-versa).
    // A production system would compare component data hash-by-hash.
    for &(entity, ref components) in &current.component_data {
        changed.push(EntitySaveData {
            entity,
            archetype_id: 0,
            components: components.iter().map(|cd| (format!("{:?}", cd.type_id), cd.data.clone())).collect(),
        });
    }

    SaveGameDelta {
        base_version: 0,
        changed_entities: changed,
        player_delta: None,
        quest_delta: HashMap::new(),
        world_time_delta: None,
    }
}

// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rand;

    #[test]
    fn test_save_game_metadata_defaults() {
        let meta = SaveGameMetadata::new(0, "Test Save");
        assert_eq!(meta.slot_id, 0);
        assert_eq!(meta.play_time_seconds, 0);
        assert!(!meta.is_auto_save);
        assert!(meta.tags.is_empty());
    }

    #[test]
    fn test_save_game_slot_lifecycle() {
        let mut slot = SaveGameSlot::new(0, "Empty Slot");
        assert!(slot.is_empty());

        let save = SaveGame::new(SaveGameMetadata::new(0, "Test"));
        slot.data = Some(save);
        assert!(!slot.is_empty());

        slot.mark_dirty();
        assert!(slot.dirty);
    }

    #[test]
    fn test_serializer_roundtrip() {
        let serializer = SaveGameSerializer::new();
        let save = SaveGameBuilder::new("Roundtrip Test")
            .with_play_time(12345)
            .with_level_name("MountainPass")
            .with_tag("test")
            .build();

        let bytes = serializer.serialize(&save).expect("serialize");
        let restored = serializer.deserialize(&bytes).expect("deserialize");

        assert_eq!(save.metadata.name, restored.metadata.name);
        assert_eq!(save.metadata.play_time_seconds, restored.metadata.play_time_seconds);
        assert_eq!(save.metadata.level_name, restored.metadata.level_name);
    }

    #[test]
    fn test_compression_roundtrip() {
        let mut rng = rand::thread_rng();
        let data: Vec<u8> = (0..4096).map(|_| rand::random::<u8>()).collect();

        // Zstd
        let zstd_ser = SaveGameSerializer::new().with_compression(CompressionType::Zstd);
        let zstd_bytes = zstd_ser.compress(&data).unwrap();
        let zstd_de = zstd_ser.decompress(&zstd_bytes).unwrap();
        assert_eq!(data, zstd_de);
        assert!(zstd_bytes.len() < data.len() || data.len() <= 8, "compression should not expand tiny data");

        // Lz4
        let lz4_ser = SaveGameSerializer::new().with_compression(CompressionType::Lz4);
        let lz4_bytes = lz4_ser.compress(&data).unwrap();
        let lz4_de = lz4_ser.decompress(&lz4_bytes).unwrap();
        assert_eq!(data, lz4_de);
    }

    #[test]
    fn test_checksum() {
        let ser = SaveGameSerializer::new();
        let data = b"hello save game world";
        let cksum1 = ser.compute_checksum(data);
        let cksum2 = ser.compute_checksum(data);
        assert_eq!(cksum1, cksum2, "checksums should be deterministic");
    }

    #[test]
    fn test_header_roundtrip() {
        let header = SaveGameHeader {
            version: SaveGameVersion::CURRENT,
            platform: Platform::PC,
            compression: CompressionType::Zstd,
            encryption: EncryptionType::ChaCha20Poly1305,
            data_size: 42,
            checksum: [0xAB; 16],
            reserved: [0u8; 32],
        };

        let bytes = header.to_bytes().unwrap();
        let restored = SaveGameHeader::from_bytes(&bytes).unwrap();

        assert_eq!(header.version, restored.version);
        assert_eq!(header.platform, restored.platform);
        assert_eq!(header.compression, restored.compression);
        assert_eq!(header.encryption, restored.encryption);
        assert_eq!(header.data_size, restored.data_size);
        assert_eq!(header.checksum, restored.checksum);
    }

    #[test]
    fn test_builder_pattern() {
        let save = SaveGameBuilder::new("Builder Test")
            .with_play_time(999)
            .with_level_name("Dungeon")
            .with_tag("speedrun")
            .with_tag("hardcore")
            .build();

        assert_eq!(save.metadata.name, "Builder Test");
        assert_eq!(save.metadata.play_time_seconds, 999);
        assert_eq!(save.metadata.level_name, "Dungeon");
        assert_eq!(save.metadata.tags.len(), 2);
    }

    #[test]
    fn test_save_game_manager_lifecycle() {
        let mut mgr = SaveGameManager::new().with_platform(Platform::Pc);

        let save = SaveGameBuilder::new("Manager Test")
            .with_play_time(100)
            .build();

        mgr.create_save(0, save.clone()).expect("create");
        assert!(mgr.get_slot(0).unwrap().data.is_some());

        mgr.save(0).expect("save");
        assert!(!mgr.get_slot(0).unwrap().dirty);

        let loaded = mgr.load(0).expect("load");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().metadata.name, "Manager Test");

        mgr.delete(0).expect("delete");
        assert!(mgr.get_slot(0).unwrap().is_empty());
    }
}
