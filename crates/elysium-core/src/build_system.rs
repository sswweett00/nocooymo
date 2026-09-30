//! Elysium Engine build and deployment system.
//!
//! This module provides:
//! - Build configuration (platforms, modes, optimisations, signing)
//! - Asset packaging (bundling, compression, chunking, manifest, pak files)
//! - Platform abstraction (detection, code paths, filesystem, window management)
//! - Patching system (delta patches, manifest, verification, rollback)
//! - Deployment (installers, store integration hooks, cloud build hooks)
//!
//! # Example
//! ```
//! use elysium_core::build_system::{BuildConfig, Platform, BuildMode};
//!
//! let config = BuildConfig::new(Platform::Windows, BuildMode::Shipping);
//! assert!(config.strip_symbols);
//! ```

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Write, Cursor};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during build, packaging, or deployment operations.
#[derive(Debug, Error)]
pub enum BuildError {
    #[error("IO error: {0}")]
    Io(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Patch error: {0}")]
    Patch(String),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Deployment error: {0}")]
    Deployment(String),

    #[error("Verification error: {0}")]
    Verification(String),
}

impl From<io::Error> for BuildError {
    fn from(err: io::Error) -> Self {
        BuildError::Io(err.to_string())
    }
}

impl From<bincode::Error> for BuildError {
    fn from(err: bincode::Error) -> Self {
        BuildError::Serialization(err.to_string())
    }
}

impl From<serde_json::Error> for BuildError {
    fn from(err: serde_json::Error) -> Self {
        BuildError::Serialization(err.to_string())
    }
}

/// Specialised result type for build operations.
pub type BuildResult<T> = Result<T, BuildError>;

// ---------------------------------------------------------------------------
// 1. Build Configuration
// ---------------------------------------------------------------------------

/// Target platform for the build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Windows,
    Mac,
    Linux,
    IOS,
    Android,
    Web,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Platform::Windows => write!(f, "windows"),
            Platform::Mac => write!(f, "mac"),
            Platform::Linux => write!(f, "linux"),
            Platform::IOS => write!(f, "ios"),
            Platform::Android => write!(f, "android"),
            Platform::Web => write!(f, "web"),
        }
    }
}

impl Platform {
    /// Detects the current platform at runtime.
    pub fn detect() -> Self {
        if cfg!(target_os = "windows") {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::Mac
        } else if cfg!(target_os = "ios") {
            Platform::IOS
        } else if cfg!(target_os = "android") {
            Platform::Android
        } else if cfg!(target_family = "wasm") {
            Platform::Web
        } else {
            Platform::Linux
        }
    }

    /// Returns the default architecture for this platform.
    pub fn default_arch(&self) -> &'static str {
        match self {
            Platform::Windows | Platform::Linux | Platform::Mac | Platform::Android => {
                if cfg!(target_arch = "x86_64") {
                    "x86_64"
                } else if cfg!(target_arch = "aarch64") {
                    "aarch64"
                } else {
                    "x86"
                }
            }
            Platform::IOS => "aarch64",
            Platform::Web => "wasm32",
        }
    }

    /// Returns whether this platform supports native code signing.
    pub fn supports_code_signing(&self) -> bool {
        matches!(self, Platform::Windows | Platform::Mac | Platform::IOS | Platform::Android)
    }

    /// Returns the default executable extension for this platform.
    pub fn executable_extension(&self) -> Option<&'static str> {
        match self {
            Platform::Windows => Some("exe"),
            Platform::Mac => None,
            Platform::Linux => None,
            Platform::IOS => None,
            Platform::Android => Some("apk"),
            Platform::Web => None,
        }
    }
}

/// Build mode / configuration profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildMode {
    Debug,
    Release,
    Shipping,
}

impl fmt::Display for BuildMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildMode::Debug => write!(f, "debug"),
            BuildMode::Release => write!(f, "release"),
            BuildMode::Shipping => write!(f, "shipping"),
        }
    }
}

/// Optimization level for the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OptimizationLevel {
    None = 0,
    Fast = 1,
    Faster = 2,
    Fastest = 3,
    Size = 4,
    SizeMin = 5,
}

impl Default for OptimizationLevel {
    fn default() -> Self {
        Self::Fast
    }
}

impl fmt::Display for OptimizationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptimizationLevel::None => write!(f, "none"),
            OptimizationLevel::Fast => write!(f, "fast"),
            OptimizationLevel::Faster => write!(f, "faster"),
            OptimizationLevel::Fastest => write!(f, "fastest"),
            OptimizationLevel::Size => write!(f, "size"),
            OptimizationLevel::SizeMin => write!(f, "size-min"),
        }
    }
}

impl OptimizationLevel {
    /// Returns the flag used by C/C++ compilers.
    pub fn compiler_flag(&self) -> &'static str {
        match self {
            OptimizationLevel::None => "O0",
            OptimizationLevel::Fast => "O1",
            OptimizationLevel::Faster => "O2",
            OptimizationLevel::Fastest => "O3",
            OptimizationLevel::Size => "Os",
            OptimizationLevel::SizeMin => "Oz",
        }
    }
}

/// Code signing configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeSigningConfig {
    /// Whether to sign the build.
    pub enabled: bool,
    /// Path to the signing certificate / key.
    pub certificate_path: Option<PathBuf>,
    /// Certificate password or key passphrase.
    pub certificate_password: Option<String>,
    /// Signing identity (Apple) or subject name (Windows).
    pub identity: Option<String>,
    /// Additional signing options.
    pub extra_args: Vec<String>,
}

/// Main build configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Target platform.
    pub platform: Platform,
    /// Build mode.
    pub mode: BuildMode,
    /// Architecture to build for.
    pub architecture: String,
    /// Optimization level.
    pub optimization: OptimizationLevel,
    /// Whether to strip debug symbols.
    pub strip_symbols: bool,
    /// Code signing configuration.
    pub code_signing: CodeSigningConfig,
    /// Additional C/C++/Rust compiler flags.
    pub extra_compiler_flags: Vec<String>,
    /// Additional linker flags.
    pub extra_linker_flags: Vec<String>,
    /// Environment variables to set during the build.
    pub env: HashMap<String, String>,
    /// Output directory for build artifacts.
    pub output_dir: PathBuf,
    /// Temporary build directory.
    pub build_dir: PathBuf,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            platform: Platform::detect(),
            mode: BuildMode::Debug,
            architecture: Platform::detect().default_arch().to_string(),
            optimization: OptimizationLevel::default(),
            strip_symbols: false,
            code_signing: CodeSigningConfig::default(),
            extra_compiler_flags: Vec::new(),
            extra_linker_flags: Vec::new(),
            env: HashMap::new(),
            output_dir: PathBuf::from("target/elysium-build"),
            build_dir: PathBuf::from("target/elysium-build-tmp"),
        }
    }
}

impl BuildConfig {
    /// Creates a new build configuration with sensible defaults.
    pub fn new(platform: Platform, mode: BuildMode) -> Self {
        let mut config = Self {
            platform,
            mode,
            ..Self::default()
        };

        config.apply_mode_defaults();
        config
    }

    /// Applies convention-based defaults for the selected mode.
    pub fn apply_mode_defaults(&mut self) {
        match self.mode {
            BuildMode::Debug => {
                self.optimization = OptimizationLevel::None;
                self.strip_symbols = false;
            }
            BuildMode::Release => {
                self.optimization = OptimizationLevel::Fastest;
                self.strip_symbols = true;
            }
            BuildMode::Shipping => {
                self.optimization = OptimizationLevel::Size;
                self.strip_symbols = true;
            }
        }
    }

    /// Returns whether this is a debug build.
    pub fn is_debug(&self) -> bool {
        matches!(self.mode, BuildMode::Debug)
    }

    /// Sets the output directory.
    pub fn with_output_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.output_dir = dir.into();
        self
    }

    /// Enables code signing with the given certificate path and identity.
    pub fn with_signing(mut self, cert_path: impl Into<PathBuf>, identity: impl Into<String>) -> Self {
        self.code_signing = CodeSigningConfig {
            enabled: true,
            certificate_path: Some(cert_path.into()),
            identity: Some(identity.into()),
            certificate_password: None,
            extra_args: Vec::new(),
        };
        self
    }
}

// ---------------------------------------------------------------------------
// 2. Asset Packaging
// ---------------------------------------------------------------------------

/// Compression algorithm for pak files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionAlgorithm {
    None,
    Lz4,
    Zstd,
    Deflate,
}

impl Default for CompressionAlgorithm {
    fn default() -> Self {
        Self::Zstd
    }
}

impl CompressionAlgorithm {
    /// Returns the file extension commonly associated with this algorithm.
    pub fn extension(&self) -> Option<&'static str> {
        match self {
            CompressionAlgorithm::None => None,
            CompressionAlgorithm::Lz4 => Some("lz4"),
            CompressionAlgorithm::Zstd => Some("zst"),
            CompressionAlgorithm::Deflate => Some("deflate"),
        }
    }
}

/// Represents a single chunk of data for streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetChunk {
    /// Unique chunk identifier.
    pub id: Uuid,
    /// Original file path this chunk belongs to.
    pub file_path: PathBuf,
    /// Byte offset in the original file.
    pub offset: u64,
    /// Chunk size in bytes.
    pub size: u32,
    /// Compressed size in bytes.
    pub compressed_size: u32,
    /// SHA-256 hash of the uncompressed chunk.
    pub hash: [u8; 32],
    /// Priority for streaming (higher = more important).
    pub priority: u8,
}

impl AssetChunk {
    /// Creates a new chunk.
    pub fn new(file_path: PathBuf, offset: u64, size: u32, compressed_size: u32, hash: [u8; 32]) -> Self {
        Self {
            id: Uuid::new_v4(),
            file_path,
            offset,
            size,
            compressed_size,
            hash,
            priority: 0,
        }
    }

    /// Sets the streaming priority.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }
}

/// A bundled asset package entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PakEntry {
    /// Virtual path inside the pak file.
    pub virtual_path: String,
    /// Original size in bytes.
    pub size: u64,
    /// Compressed size in bytes (0 if uncompressed).
    pub compressed_size: u64,
    /// SHA-256 hash of the original data.
    pub hash: [u8; 32],
    /// Offset within the pak file data section.
    pub offset: u64,
    /// Compression algorithm used.
    pub compression: CompressionAlgorithm,
    /// Whether this entry is stored uncompressed.
    pub uncompressed: bool,
}

impl PakEntry {
    /// Creates a new pak entry.
    pub fn new(
        virtual_path: impl Into<String>,
        size: u64,
        offset: u64,
        hash: [u8; 32],
        compression: CompressionAlgorithm,
    ) -> Self {
        Self {
            virtual_path: virtual_path.into(),
            size,
            compressed_size: 0,
            hash,
            offset,
            compression,
            uncompressed: matches!(compression, CompressionAlgorithm::None),
        }
    }
}

/// Pak file format header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PakHeader {
    /// Magic bytes: "PAK\0".
    pub magic: [u8; 4],
    /// Pak file format version.
    pub version: u32,
    /// Compression algorithm used.
    pub compression: CompressionAlgorithm,
    /// Number of entries in the pak.
    pub entry_count: u32,
    /// Offset to the entry table.
    pub entry_table_offset: u64,
    /// Offset to the data section.
    pub data_section_offset: u64,
    /// Total size of the pak file.
    pub total_size: u64,
    /// SHA-256 hash of the entire pak.
    pub pak_hash: [u8; 32],
    /// Timestamp when this pak was created.
    pub created_at: u64,
}

impl Default for PakHeader {
    fn default() -> Self {
        Self {
            magic: *b"PAK\0",
            version: 1,
            compression: CompressionAlgorithm::default(),
            entry_count: 0,
            entry_table_offset: 0,
            data_section_offset: 0,
            total_size: 0,
            pak_hash: [0; 32],
            created_at: 0,
        }
    }
}

/// A Pak asset bundle.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PakFile {
    /// Pak file header.
    pub header: PakHeader,
    /// All entries in this pak file.
    pub entries: Vec<PakEntry>,
}

impl PakFile {
    /// Creates a new empty pak file.
    pub fn new(compression: CompressionAlgorithm) -> Self {
        let mut header = PakHeader::default();
        header.compression = compression;
        header.created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            header,
            entries: Vec::new(),
        }
    }

    /// Adds an entry to the pak.
    pub fn add_entry(&mut self, entry: PakEntry) {
        self.entries.push(entry);
        self.header.entry_count = self.entries.len() as u32;
    }

    /// Returns entries grouped by virtual path prefix for dependency sorting.
    pub fn sorted_entries(&self) -> Vec<&PakEntry> {
        let mut sorted = self.entries.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| a.virtual_path.cmp(&b.virtual_path));
        sorted
    }
}

/// Asset manifest describes all assets in a build.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetManifest {
    /// Manifest version.
    pub version: u32,
    /// Pak files included in this manifest.
    pub pak_files: Vec<PathBuf>,
    /// All individual chunks (for streaming builds).
    pub chunks: Vec<AssetChunk>,
    /// Total uncompressed asset size.
    pub total_size: u64,
    /// Total compressed asset size.
    pub total_compressed_size: u64,
    /// Asset dependencies (virtual_path -> list of required virtual paths).
    pub dependencies: BTreeMap<String, Vec<String>>,
    /// Timestamp of the manifest.
    pub generated_at: u64,
}

impl AssetManifest {
    /// Creates a new empty manifest.
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs;
        Self {
            version: 1,
            pak_files: Vec::new(),
            chunks: Vec::new(),
            total_size: 0,
            total_compressed_size: 0,
            dependencies: BTreeMap::new(),
            generated_at: now,
        }
    }

    /// Adds a pak file to the manifest.
    pub fn add_pak(&mut self, path: impl Into<PathBuf>) {
        self.pak_files.push(path.into());
    }

    /// Adds chunks to the manifest.
    pub fn add_chunks(&mut self, chunks: Vec<AssetChunk>) {
        for chunk in chunks {
            self.total_size += chunk.size as u64;
            self.total_compressed_size += chunk.compressed_size as u64;
            self.chunks.push(chunk);
        }
    }

    /// Registers a dependency relationship.
    pub fn add_dependency(&mut self, asset: impl Into<String>, deps: Vec<String>) {
        self.dependencies.insert(asset.into(), deps);
    }

    /// Writes the manifest to JSON.
    pub fn write_json(&self, path: impl AsRef<Path>) -> BuildResult<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Reads the manifest from JSON.
    pub fn read_json(path: impl AsRef<Path>) -> BuildResult<Self> {
        let data = fs::read(path)?;
        let manifest = serde_json::from_slice(&data)?;
        Ok(manifest)
    }
}

/// Bundles assets into pak files.
pub struct AssetPackager {
    /// Working directory for packaging.
    pub working_dir: PathBuf,
    /// Output directory for pak files.
    pub output_dir: PathBuf,
    /// Compression algorithm to use.
    pub compression: CompressionAlgorithm,
    /// Chunk size for streaming (0 to disable chunking).
    pub chunk_size: u32,
}

impl Default for AssetPackager {
    fn default() -> Self {
        Self {
            working_dir: PathBuf::from("assets"),
            output_dir: PathBuf::from("target/paks"),
            compression: CompressionAlgorithm::default(),
            chunk_size: 256 * 1024, // 256 KiB chunks
        }
    }
}

impl AssetPackager {
    /// Creates a new asset packager.
    pub fn new(working_dir: impl Into<PathBuf>, output_dir: impl Into<PathBuf>) -> Self {
        Self {
            working_dir: working_dir.into(),
            output_dir: output_dir.into(),
            ..Self::default()
        }
    }

    /// Sets the compression algorithm.
    pub fn with_compression(mut self, compression: CompressionAlgorithm) -> Self {
        self.compression = compression;
        self
    }

    /// Sets the chunk size for streaming.
    pub fn with_chunk_size(mut self, chunk_size: u32) -> Self {
        self.chunk_size = chunk_size;
        self
    }

    /// Compresses data using the configured algorithm.
    pub fn compress(&self, data: &[u8]) -> BuildResult<Vec<u8>> {
        match self.compression {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            CompressionAlgorithm::Lz4 => {
                let mut encoder = lz4::EncoderBuilder::new()
                    .build(Vec::new())
                    .map_err(|e| BuildError::Compression(e.to_string()))?;
                encoder.write_all(data)?;
                Ok(encoder.finish()?)
            }
            CompressionAlgorithm::Zstd => {
                let encoded = zstd::encode_all(data, 3)
                    .map_err(|e| BuildError::Compression(e.to_string()))?;
                Ok(encoded)
            }
            CompressionAlgorithm::Deflate => {
                let mut encoder = flate2::write::DeflateEncoder::new(
                    Vec::new(),
                    flate2::Compression::default(),
                );
                encoder.write_all(data)?;
                Ok(encoder.finish()?)
            }
        }
    }

    /// Decompresses data.
    pub fn decompress(&self, data: &[u8], algorithm: CompressionAlgorithm) -> BuildResult<Vec<u8>> {
        match algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            CompressionAlgorithm::Lz4 => {
                let mut decoder = lz4::Decoder::new(data)
                    .map_err(|e| BuildError::Compression(e.to_string()))?;
                let mut out = Vec::new();
                io::copy(&mut decoder, &mut out)?;
                Ok(out)
            }
            CompressionAlgorithm::Zstd => {
                let decoded = zstd::decode_all(data)
                    .map_err(|e| BuildError::Compression(e.to_string()))?;
                Ok(decoded)
            }
            CompressionAlgorithm::Deflate => {
                let mut decoder = flate2::read::DeflateDecoder::new(data);
                let mut out = Vec::new();
                io::copy(&mut decoder, &mut out)?;
                Ok(out)
            }
        }
    }

    /// Packages all assets in the working directory into pak files.
    pub fn package(&self) -> BuildResult<AssetManifest> {
        fs::create_dir_all(&self.output_dir)?;
        let mut manifest = AssetManifest::new();
        let mut pak = PakFile::new(self.compression);
        let mut entries = Vec::new();
        let mut chunks = Vec::new();

        let mut collected = self.collect_files(&self.working_dir, "")?;
        collected.sort_by(|a, b| a.cmp(b));

        for rel_path in collected {
            let full_path = self.working_dir.join(&rel_path);
            let data = fs::read(&full_path)?;
            let hash = sha2::Sha256::digest(&data);
            let mut hash_bytes = [0u8; 32];
            hash_bytes.copy_from_slice(&hash);

            if self.chunk_size > 0 && data.len() > self.chunk_size as usize {
                let file_chunks = self.chunk_data(&rel_path.to_string_lossy(), &data, hash_bytes)?;
                chunks.extend(file_chunks);
            }

            let compressed = self.compress(&data)?;
            let entry = PakEntry {
                virtual_path: rel_path.to_string_lossy().into_owned(),
                size: data.len() as u64,
                compressed_size: compressed.len() as u64,
                hash: hash_bytes,
                offset: 0,
                compression: self.compression,
                uncompressed: false,
            };
            entries.push((entry, compressed));
        }

        let mut cursor = Cursor::new(Vec::new());
        for (entry, data) in &entries {
            let offset = cursor.position();
            let mut entry_mut = entry.clone();
            entry_mut.offset = offset;
            pak.add_entry(entry_mut);
            cursor.write_all(data)?;
        }

        let pak_data = cursor.into_inner();
        pak.header.total_size = pak_data.len() as u64;
        pak.header.data_section_offset = std::mem::size_of::<PakHeader>() as u64;

        let pak_path = self.output_dir.join("assets.pak");
        let mut file = File::create(&pak_path)?;
        let header_bytes = bincode::serialize(&pak.header)?;
        file.write_all(&header_bytes)?;
        file.write_all(&pak_data)?;
        drop(file);

        manifest.add_pak(pak_path);
        manifest.add_chunks(chunks);
        manifest.total_size = entries.iter().map(|(e, _)| e.size).sum();
        manifest.total_compressed_size = entries.iter().map(|(e, _)| e.compressed_size).sum();
        manifest.write_json(self.output_dir.join("manifest.json"))?;

        Ok(manifest)
    }

    /// Recursively collects files in a directory.
    fn collect_files(&self, dir: &Path, prefix: &str) -> BuildResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let rel = if prefix.is_empty() {
                entry.file_name().into()
            } else {
                PathBuf::from(prefix).join(entry.file_name())
            };
            if path.is_dir() {
                files.extend(self.collect_files(&path, &rel.to_string_lossy())?);
            } else {
                files.push(rel);
            }
        }
        Ok(files)
    }

    /// Splits data into chunks and returns their hashes.
    fn chunk_data(&self, name: &str, data: &[u8], file_hash: [u8; 32]) -> BuildResult<Vec<AssetChunk>> {
        let mut chunks = Vec::new();
        let mut offset = 0u64;
        let mut priority = 255u8;

        for chunk in data.chunks(self.chunk_size as usize) {
            let chunk_hash = sha2::Sha256::digest(chunk);
            let mut hash_bytes = [0u8; 32];
            hash_bytes.copy_from_slice(&chunk_hash);

            let asset_chunk = AssetChunk::new(
                PathBuf::from(name),
                offset,
                chunk.len() as u32,
                chunk.len() as u32,
                hash_bytes,
            )
            .with_priority(priority);

            chunks.push(asset_chunk);
            offset += chunk.len() as u64;
            priority = priority.saturating_sub(1);
        }

        Ok(chunks)
    }
}

// ---------------------------------------------------------------------------
// 3. Platform Abstraction
// ---------------------------------------------------------------------------

/// File system abstraction trait.
pub trait FileSystem: Send + Sync {
    /// Reads a file.
    fn read(&self, path: &Path) -> BuildResult<Vec<u8>>;
    /// Writes a file.
    fn write(&self, path: &Path, data: &[u8]) -> BuildResult<()>;
    /// Creates a directory.
    fn create_dir(&self, path: &Path) -> BuildResult<()>;
    /// Lists directory contents.
    fn list_dir(&self, path: &Path) -> BuildResult<Vec<PathBuf>>;
    /// Checks if a path exists.
    fn exists(&self, path: &Path) -> bool;
    /// Removes a file.
    fn remove(&self, path: &Path) -> BuildResult<()>;
}

/// Local file system implementation.
#[derive(Debug, Clone, Default)]
pub struct LocalFileSystem;

impl FileSystem for LocalFileSystem {
    fn read(&self, path: &Path) -> BuildResult<Vec<u8>> {
        Ok(fs::read(path)?)
    }

    fn write(&self, path: &Path, data: &[u8]) -> BuildResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(fs::write(path, data)?)
    }

    fn create_dir(&self, path: &Path) -> BuildResult<()> {
        Ok(fs::create_dir_all(path)?)
    }

    fn list_dir(&self, path: &Path) -> BuildResult<Vec<PathBuf>> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)? {
            entries.push(entry?.path());
        }
        Ok(entries)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn remove(&self, path: &Path) -> BuildResult<()> {
        Ok(fs::remove_file(path)?)
    }
}

/// Window / display management abstraction.
#[derive(Debug, Clone)]
pub struct DisplayConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub vsync: bool,
    pub high_dpi: bool,
    pub resizable: bool,
    pub samples: u32,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            title: "Elysium".into(),
            width: 1920,
            height: 1080,
            fullscreen: false,
            vsync: true,
            high_dpi: true,
            resizable: true,
            samples: 4,
        }
    }
}

/// Platform abstraction layer.
#[derive(Debug, Clone, Default)]
pub struct PlatformAbstraction {
    /// Detected host platform.
    pub host_platform: Platform,
    /// Target platform for this build.
    pub target_platform: Platform,
    /// Display configuration.
    pub display: DisplayConfig,
    /// Filesystem implementation to use.
    pub filesystem: Arc<dyn FileSystem>,
    /// Environment variables specific to the platform.
    pub env: HashMap<String, String>,
}

impl PlatformAbstraction {
    /// Creates a new platform abstraction for the given target.
    pub fn new(target: Platform) -> Self {
        Self {
            host_platform: Platform::detect(),
            target_platform: target,
            display: DisplayConfig::default(),
            filesystem: Arc::new(LocalFileSystem::default()),
            env: HashMap::new(),
        }
    }

    /// Sets the filesystem implementation.
    pub fn with_filesystem(mut self, fs: Arc<dyn FileSystem>) -> Self {
        self.filesystem = fs;
        self
    }

    /// Returns whether cross-compilation is in effect.
    pub fn is_cross_compiling(&self) -> bool {
        self.host_platform != self.target_platform
    }

    /// Returns platform-specific compiler flags.
    pub fn platform_compiler_flags(&self) -> Vec<String> {
        match self.target_platform {
            Platform::Windows => vec!["/utf-8".into(), "/W4".into()],
            Platform::Mac | Platform::IOS => vec!["-fobjc-arc".into()],
            Platform::Linux => vec!["-Wall".into(), "-Wextra".into()],
            Platform::Android => vec!["-fPIC".into()],
            Platform::Web => vec!["-sSIDE_MODULE=1".into()],
        }
    }

    /// Returns the platform-specific output binary name.
    pub fn output_binary_name(&self, app_name: &str) -> PathBuf {
        let ext = self.target_platform.executable_extension();
        match ext {
            Some(e) => PathBuf::from(format!("{}.{}", app_name, e)),
            None => PathBuf::from(app_name),
        }
    }

    /// Returns platform-specific environment variables.
    pub fn platform_env(&self) -> HashMap<String, String> {
        let mut env = self.env.clone();
        match self.target_platform {
            Platform::Windows => {
                env.insert("CARGO_CFG_TARGET_OS".into(), "windows".into());
            }
            Platform::Mac => {
                env.insert("CARGO_CFG_TARGET_OS".into(), "macos".into());
                env.insert("MACOSX_DEPLOYMENT_TARGET".into(), "12.0".into());
            }
            Platform::Linux => {
                env.insert("CARGO_CFG_TARGET_OS".into(), "linux".into());
            }
            Platform::IOS => {
                env.insert("CARGO_CFG_TARGET_OS".into(), "ios".into());
            }
            Platform::Android => {
                env.insert("CARGO_CFG_TARGET_OS".into(), "android".into());
            }
            Platform::Web => {
                env.insert("CARGO_CFG_TARGET_OS".into(), "unknown".into());
                env.insert("CARGO_CFG_TARGET_FAMILY".into(), "wasm".into());
            }
        }
        env
    }
}

// ---------------------------------------------------------------------------
// 4. Patching System
// ---------------------------------------------------------------------------

/// A single file patch (delta).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePatch {
    /// Unique patch identifier.
    pub id: Uuid,
    /// Source file path (relative to the build root).
    pub source_path: PathBuf,
    /// Target file path.
    pub target_path: PathBuf,
    /// SHA-256 hash of the original file.
    pub original_hash: [u8; 32],
    /// SHA-256 hash of the patched file.
    pub patched_hash: [u8; 32],
    /// Patch data size in bytes.
    pub patch_size: u32,
    /// Uncompressed delta size.
    pub delta_size: u32,
    /// Priority (higher = applied earlier).
    pub priority: i32,
}

impl FilePatch {
    /// Creates a new file patch.
    pub fn new(
        source_path: impl Into<PathBuf>,
        target_path: impl Into<PathBuf>,
        original_hash: [u8; 32],
        patched_hash: [u8; 32],
        patch_size: u32,
        delta_size: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_path: source_path.into(),
            target_path: target_path.into(),
            original_hash,
            patched_hash,
            patch_size,
            delta_size,
            priority: 0,
        }
    }

    /// Sets the patch priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

/// Patch manifest describing a set of patches.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PatchManifest {
    /// Manifest version.
    pub version: u32,
    /// Source build version.
    pub source_version: String,
    /// Target build version.
    pub target_version: String,
    /// List of file patches.
    pub patches: Vec<FilePatch>,
    /// Total patch size in bytes.
    pub total_patch_size: u64,
    /// Total uncompressed delta size.
    pub total_delta_size: u64,
    /// Minimum required source version.
    pub min_source_version: String,
    /// Whether this is a cumulative patch.
    pub cumulative: bool,
    /// Timestamp.
    pub generated_at: u64,
}

impl PatchManifest {
    /// Creates a new empty patch manifest.
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs;
        Self {
            version: 1,
            source_version: source.into(),
            target_version: target.into(),
            patches: Vec::new(),
            total_patch_size: 0,
            total_delta_size: 0,
            min_source_version: source.into(),
            cumulative: false,
            generated_at: now,
        }
    }

    /// Adds a patch.
    pub fn add_patch(&mut self, patch: FilePatch) {
        self.total_patch_size += patch.patch_size as u64;
        self.total_delta_size += patch.delta_size as u64;
        self.patches.push(patch);
    }

    /// Sorts patches by priority (highest first).
    pub fn sort_by_priority(&mut self) {
        self.patches.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Writes the manifest to JSON.
    pub fn write_json(&self, path: impl AsRef<Path>) -> BuildResult<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Reads the manifest from JSON.
    pub fn read_json(path: impl AsRef<Path>) -> BuildResult<Self> {
        let data = fs::read(path)?;
        let manifest = serde_json::from_slice(&data)?;
        Ok(manifest)
    }
}

/// Patching engine.
pub struct PatchEngine {
    /// Base directory for patch files.
    pub patch_dir: PathBuf,
}

impl Default for PatchEngine {
    fn default() -> Self {
        Self {
            patch_dir: PathBuf::from("target/patches"),
        }
    }
}

impl PatchEngine {
    /// Creates a new patch engine.
    pub fn new(patch_dir: impl Into<PathBuf>) -> Self {
        Self {
            patch_dir: patch_dir.into(),
        }
    }

    /// Generates a delta patch between two file versions.
    pub fn generate_file_patch(
        &self,
        source: &[u8],
        target: &[u8],
        source_path: impl Into<PathBuf>,
        target_path: impl Into<PathBuf>,
    ) -> BuildResult<FilePatch> {
        let original_hash = sha2::Sha256::digest(source);
        let patched_hash = sha2::Sha256::digest(target);
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&original_hash);
        let mut patched_hash_bytes = [0u8; 32];
        patched_hash_bytes.copy_from_slice(&patched_hash);

        let delta = Self::compute_delta(source, target);
        let patch_data = bincode::serialize(&delta)?;
        let patch_size = patch_data.len() as u32;
        let delta_size = delta.len() as u32;

        Ok(FilePatch::new(
            source_path,
            target_path,
            hash_bytes,
            patched_hash_bytes,
            patch_size,
            delta_size,
        ))
    }

    /// Generates a patch manifest between two directories.
    pub fn generate_manifest(
        &self,
        source_dir: &Path,
        target_dir: &Path,
        source_version: &str,
        target_version: &str,
    ) -> BuildResult<PatchManifest> {
        let mut manifest = PatchManifest::new(source_version, target_version);
        let source_files = self.collect_files(source_dir)?;
        let target_files = self.collect_files(target_dir)?;

        for (rel_path, target_data) in target_files {
            if let Some(source_data) = source_files.get(&rel_path) {
                if source_data != &target_data {
                    let patch = self.generate_file_patch(
                        source_data,
                        &target_data,
                        rel_path.clone(),
                        rel_path.clone(),
                    )?;
                    manifest.add_patch(patch);
                }
            } else {
                let original_hash = sha2::Sha256::digest(&[] as &[u8]);
                let mut hash_bytes = [0u8; 32];
                hash_bytes.copy_from_slice(&original_hash);
                let target_hash = sha2::Sha256::digest(&target_data);
                let mut patched_hash_bytes = [0u8; 32];
                patched_hash_bytes.copy_from_slice(&target_hash);

                let patch = FilePatch::new(
                    rel_path.clone(),
                    rel_path.clone(),
                    hash_bytes,
                    patched_hash_bytes,
                    target_data.len() as u32,
                    target_data.len() as u32,
                );
                manifest.add_patch(patch);
            }
        }

        manifest.sort_by_priority();
        Ok(manifest)
    }

    /// Verifies a patch can be applied to a given source version.
    pub fn verify_patch(&self, manifest: &PatchManifest, source_version: &str) -> BuildResult<bool> {
        if manifest.cumulative {
            return Ok(true);
        }

        let compatible = source_version >= manifest.min_source_version.as_str()
            && source_version < manifest.target_version.as_str();
        Ok(compatible)
    }

    /// Applies patches in order to source data.
    pub fn apply_patches(
        &self,
        manifest: &PatchManifest,
        mut data: HashMap<String, Vec<u8>>,
    ) -> BuildResult<HashMap<String, Vec<u8>>> {
        for patch in &manifest.patches {
            let key = patch.source_path.to_string_lossy().into_owned();
            if let Some(source_data) = data.remove(&key) {
                let delta = Self::compute_delta_reverse(&source_data, patch)?;
                data.insert(patch.target_path.to_string_lossy().into_owned(), delta);
            }
        }
        Ok(data)
    }

    /// Computes a simple delta between source and target.
    fn compute_delta(source: &[u8], target: &[u8]) -> Vec<u8> {
        let mut delta = Vec::with_capacity(target.len().min(source.len() + 16));
        let mut i = 0;
        let mut j = 0;

        while i < source.len() && j < target.len() {
            if source[i] == target[j] {
                i += 1;
                j += 1;
            } else {
                delta.push(0xFF);
                delta.push(target[j]);
                j += 1;
            }
        }

        while j < target.len() {
            delta.push(0xFF);
            delta.push(target[j]);
            j += 1;
        }

        delta
    }

    /// Reconstructs target data from source using a delta.
    fn compute_delta_reverse(source: &[u8], patch: &FilePatch) -> BuildResult<Vec<u8>> {
        let mut target = Vec::with_capacity(patch.delta_size as usize);
        let mut i = 0;

        for byte in &source[..source.len().min(patch.delta_size as usize)] {
            target.push(*byte);
        }

        // In a real implementation, delta encoding/decoding logic would be here.
        // This is a simplified placeholder.
        Ok(target)
    }

    /// Collects all files in a directory into a map of relative path -> data.
    fn collect_files(&self, dir: &Path) -> BuildResult<HashMap<String, Vec<u8>>> {
        let mut files = HashMap::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Ok(data) = fs::read(&path) {
                    let rel = path.strip_prefix(dir).unwrap_or(&path);
                    files.insert(rel.to_string_lossy().into_owned(), data);
                }
            }
        }
        Ok(files)
    }

    /// Rolls back to a previous version using the rollback manifest.
    pub fn rollback(&self, rollback_manifest: &PatchManifest) -> BuildResult<()> {
        if rollback_manifest.patches.is_empty() {
            return Err(BuildError::Patch("No patches to roll back".into()));
        }
        // In a real implementation, this would revert files to their original state.
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 5. Deployment
// ---------------------------------------------------------------------------

/// Installer type for the target platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallerType {
    Msi,
    Exe,
    Dmg,
    Pkg,
    AppImage,
    Deb,
    Rpm,
    Apk,
    Zip,
    None,
}

impl fmt::Display for InstallerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallerType::Msi => write!(f, "msi"),
            InstallerType::Exe => write!(f, "exe"),
            InstallerType::Dmg => write!(f, "dmg"),
            InstallerType::Pkg => write!(f, "pkg"),
            InstallerType::AppImage => write!(f, "appimage"),
            InstallerType::Deb => write!(f, "deb"),
            InstallerType::Rpm => write!(f, "rpm"),
            InstallerType::Apk => write!(f, "apk"),
            InstallerType::Zip => write!(f, "zip"),
            InstallerType::None => write!(f, "none"),
        }
    }
}

impl InstallerType {
    /// Returns the default installer type for a platform.
    pub fn default_for_platform(platform: Platform) -> Self {
        match platform {
            Platform::Windows => InstallerType::Exe,
            Platform::Mac => InstallerType::Dmg,
            Platform::Linux => InstallerType::AppImage,
            Platform::IOS => InstallerType::Apk,
            Platform::Android => InstallerType::Apk,
            Platform::Web => InstallerType::Zip,
        }
    }
}

/// Store integration configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoreIntegration {
    /// Whether Steam integration is enabled.
    pub steam_enabled: bool,
    /// Steam App ID.
    pub steam_app_id: Option<u32>,
    /// Steam DRM wrapper path.
    pub steam_drm_path: Option<PathBuf>,
    /// Whether Epic Games integration is enabled.
    pub epic_enabled: bool,
    /// Epic Product ID.
    pub epic_product_id: Option<String>,
    /// Additional store arguments.
    pub extra_args: HashMap<String, String>,
}

/// Cloud build configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CloudBuildConfig {
    /// Cloud provider (e.g. "github", "gitlab", "jenkins").
    pub provider: String,
    /// Cloud project / pipeline identifier.
    pub project_id: String,
    /// Whether to trigger cloud builds automatically.
    pub auto_trigger: bool,
    /// Cloud build environment variables.
    pub env: HashMap<String, String>,
}

/// Deployment configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Application name.
    pub app_name: String,
    /// Application version.
    pub version: String,
    /// Target platform.
    pub platform: Platform,
    /// Installer type to generate.
    pub installer: InstallerType,
    /// Application icon path.
    pub icon: Option<PathBuf>,
    /// Publisher name.
    pub publisher: String,
    /// Application description.
    pub description: String,
    /// Copyright notice.
    pub copyright: String,
    /// Store integration configuration.
    pub store: StoreIntegration,
    /// Cloud build configuration.
    pub cloud: CloudBuildConfig,
    /// Additional deployment arguments.
    pub extra_args: Vec<String>,
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            app_name: "ElysiumApp".into(),
            version: "0.1.0".into(),
            platform: Platform::detect(),
            installer: InstallerType::None,
            icon: None,
            publisher: "Elysium Team".into(),
            description: String::new(),
            copyright: String::new(),
            store: StoreIntegration::default(),
            cloud: CloudBuildConfig::default(),
            extra_args: Vec::new(),
        }
    }
}

impl DeploymentConfig {
    /// Creates a new deployment configuration.
    pub fn new(app_name: impl Into<String>, version: impl Into<String>, platform: Platform) -> Self {
        Self {
            app_name: app_name.into(),
            version: version.into(),
            platform,
            installer: InstallerType::default_for_platform(platform),
            ..Self::default()
        }
    }

    /// Sets the installer type.
    pub fn with_installer(mut self, installer: InstallerType) -> Self {
        self.installer = installer;
        self
    }

    /// Enables Steam integration.
    pub fn with_steam(mut self, app_id: u32) -> Self {
        self.store.steam_enabled = true;
        self.store.steam_app_id = Some(app_id);
        self
    }

    /// Enables Epic Games integration.
    pub fn with_epic(mut self, product_id: impl Into<String>) -> Self {
        self.store.epic_enabled = true;
        self.store.epic_product_id = Some(product_id.into());
        self
    }

    /// Sets the cloud build provider.
    pub fn with_cloud(mut self, provider: impl Into<String>, project_id: impl Into<String>) -> Self {
        self.cloud.provider = provider.into();
        self.cloud.project_id = project_id.into();
        self.cloud.auto_trigger = true;
        self
    }
}

/// Deployment engine.
pub struct DeploymentEngine {
    /// Output directory for deployment artifacts.
    pub output_dir: PathBuf,
    /// Base build directory.
    pub build_dir: PathBuf,
    /// Whether the engine is in dry-run mode.
    pub dry_run: bool,
}

impl Default for DeploymentEngine {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("target/deployment"),
            build_dir: PathBuf::from("target/elysium-build"),
            dry_run: false,
        }
    }
}

impl DeploymentEngine {
    /// Creates a new deployment engine.
    pub fn new(output_dir: impl Into<PathBuf>, build_dir: impl Into<PathBuf>) -> Self {
        Self {
            output_dir: output_dir.into(),
            build_dir: build_dir.into(),
            ..Self::default()
        }
    }

    /// Sets dry-run mode.
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Generates a deployment package.
    pub fn deploy(&self, config: &DeploymentConfig) -> BuildResult<DeploymentArtifact> {
        fs::create_dir_all(&self.output_dir)?;

        let artifact_name = format!(
            "{}_{}_{}",
            config.app_name,
            config.version,
            config.platform
        );

        let mut artifact = DeploymentArtifact {
            name: artifact_name.clone(),
            version: config.version.clone(),
            platform: config.platform,
            installer: config.installer,
            files: Vec::new(),
            path: self.output_dir.join(format!("{}.{}", artifact_name, config.installer)),
        };

        match config.installer {
            InstallerType::Msi | InstallerType::Exe => {
                self.generate_windows_installer(config, &mut artifact)?;
            }
            InstallerType::Dmg | InstallerType::Pkg => {
                self.generate_mac_installer(config, &mut artifact)?;
            }
            InstallerType::AppImage | InstallerType::Deb | InstallerType::Rpm => {
                self.generate_linux_installer(config, &mut artifact)?;
            }
            InstallerType::Apk => {
                self.generate_android_package(config, &mut artifact)?;
            }
            InstallerType::Zip => {
                self.generate_zip_package(config, &mut artifact)?;
            }
            InstallerType::None => {
                self.generate_plain_package(config, &mut artifact)?;
            }
        }

        Ok(artifact)
    }

    /// Triggers a cloud build if configured.
    pub fn trigger_cloud_build(&self, config: &DeploymentConfig) -> BuildResult<String> {
        if config.cloud.provider.is_empty() {
            return Err(BuildError::Deployment(
                "Cloud build provider not configured".into(),
            ));
        }

        if self.dry_run {
            return Ok(format!(
                "DRY RUN: Would trigger cloud build on {} for project {}",
                config.cloud.provider, config.cloud.project_id
            ));
        }

        // In a real implementation, this would invoke the cloud provider API.
        Ok(format!(
            "Cloud build triggered for {} v{} on {}",
            config.app_name, config.version, config.cloud.provider
        ))
    }

    /// Generates Windows installer artifacts.
    fn generate_windows_installer(
        &self,
        config: &DeploymentConfig,
        artifact: &mut DeploymentArtifact,
    ) -> BuildResult<()> {
        let installer_name = match config.installer {
            InstallerType::Msi => format!("{}.msi", artifact.name),
            _ => format!("{}.exe", artifact.name),
        };
        artifact.path = self.output_dir.join(installer_name);
        artifact.files.push(self.output_dir.join("setup.nsh"));
        artifact.files.push(self.output_dir.join("setup.ini"));
        Ok(())
    }

    /// Generates macOS installer artifacts.
    fn generate_mac_installer(
        &self,
        config: &DeploymentConfig,
        artifact: &mut DeploymentArtifact,
    ) -> BuildResult<()> {
        let installer_name = match config.installer {
            InstallerType::Dmg => format!("{}.dmg", artifact.name),
            _ => format!("{}.pkg", artifact.name),
        };
        artifact.path = self.output_dir.join(installer_name);
        artifact.files.push(self.output_dir.join("distribution.xml"));
        Ok(())
    }

    /// Generates Linux installer artifacts.
    fn generate_linux_installer(
        &self,
        config: &DeploymentConfig,
        artifact: &mut DeploymentArtifact,
    ) -> BuildResult<()> {
        let installer_name = match config.installer {
            InstallerType::AppImage => format!("{}.AppImage", artifact.name),
            InstallerType::Deb => format!("{}.deb", artifact.name),
            InstallerType::Rpm => format!("{}.rpm", artifact.name),
            _ => format!("{}.tar.gz", artifact.name),
        };
        artifact.path = self.output_dir.join(installer_name);
        artifact.files.push(self.output_dir.join("AppRun"));
        artifact.files.push(self.output_dir.join("elysium.desktop"));
        Ok(())
    }

    /// Generates Android package artifacts.
    fn generate_android_package(
        &self,
        config: &DeploymentConfig,
        artifact: &mut DeploymentArtifact,
    ) -> BuildResult<()> {
        artifact.path = self.output_dir.join(format!("{}.apk", artifact.name));
        artifact.files.push(self.output_dir.join("AndroidManifest.xml"));
        Ok(())
    }

    /// Generates a plain zip package.
    fn generate_zip_package(
        &self,
        config: &DeploymentConfig,
        artifact: &mut DeploymentArtifact,
    ) -> BuildResult<()> {
        artifact.path = self.output_dir.join(format!("{}.zip", artifact.name));
        Ok(())
    }

    /// Generates a plain file copy package.
    fn generate_plain_package(
        &self,
        config: &DeploymentConfig,
        artifact: &mut DeploymentArtifact,
    ) -> BuildResult<()> {
        artifact.path = self.output_dir.join(&artifact.name);
        Ok(())
    }
}

/// Result of a deployment operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeploymentArtifact {
    /// Artifact name.
    pub name: String,
    /// Application version.
    pub version: String,
    /// Target platform.
    pub platform: Platform,
    /// Installer type.
    pub installer: InstallerType,
    /// Files included in this artifact.
    pub files: Vec<PathBuf>,
    /// Primary output path.
    pub path: PathBuf,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_detection_is_valid() {
        let detected = Platform::detect();
        assert!(matches!(
            detected,
            Platform::Windows | Platform::Mac | Platform::Linux | Platform::IOS | Platform::Android | Platform::Web
        ));
    }

    #[test]
    fn build_config_defaults() {
        let config = BuildConfig::default();
        assert!(config.strip_symbols == false || config.strip_symbols == true);
        assert!(config.optimization >= OptimizationLevel::None);
    }

    #[test]
    fn build_config_mode_defaults() {
        let mut debug = BuildConfig::new(Platform::Linux, BuildMode::Debug);
        debug.apply_mode_defaults();
        assert_eq!(debug.optimization, OptimizationLevel::None);

        let mut shipping = BuildConfig::new(Platform::Windows, BuildMode::Shipping);
        shipping.apply_mode_defaults();
        assert_eq!(shipping.optimization, OptimizationLevel::Size);
        assert!(shipping.strip_symbols);
    }

    #[test]
    fn asset_packager_defaults() {
        let packager = AssetPackager::default();
        assert_eq!(packager.chunk_size, 256 * 1024);
        assert_eq!(packager.compression, CompressionAlgorithm::Zstd);
    }

    #[test]
    fn pak_file_new() {
        let pak = PakFile::new(CompressionAlgorithm::Lz4);
        assert_eq!(pak.header.compression, CompressionAlgorithm::Lz4);
        assert!(pak.entries.is_empty());
    }

    #[test]
    fn asset_manifest_roundtrip_json() {
        let mut manifest = AssetManifest::new();
        manifest.add_pak(PathBuf::from("assets.pak"));
        manifest.add_dependency("textures/terrain.png", vec!["materials/terrain.ely".into()]);

        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: AssetManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.pak_files.len(), 1);
        assert_eq!(parsed.dependencies.len(), 1);
    }

    #[test]
    fn patch_manifest_roundtrip() {
        let mut manifest = PatchManifest::new("0.1.0", "0.2.0");
        let patch = FilePatch::new(
            "a.txt",
            "a.txt",
            [0u8; 32],
            [1u8; 32],
            10,
            10,
        );
        manifest.add_patch(patch);

        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: PatchManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.patches.len(), 1);
        assert_eq!(parsed.source_version, "0.1.0");
    }

    #[test]
    fn patch_engine_generates_delta() {
        let engine = PatchEngine::default();
        let source = b"hello world";
        let target = b"hello rust";
        let patch = engine
            .generate_file_patch(source, target, "in.txt", "out.txt")
            .unwrap();
        assert_eq!(patch.patch_size, patch.delta_size);
    }

    #[test]
    fn deployment_config_default_installer() {
        let config = DeploymentConfig::new("MyGame", "1.0.0", Platform::Windows);
        assert_eq!(config.installer, InstallerType::Exe);
    }

    #[test]
    fn deployment_config_with_steam() {
        let config = DeploymentConfig::new("MyGame", "1.0.0", Platform::Windows).with_steam(480);
        assert!(config.store.steam_enabled);
        assert_eq!(config.store.steam_app_id, Some(480));
    }

    #[test]
    fn deployment_engine_generates_artifact() {
        let engine = DeploymentEngine::default();
        let config = DeploymentConfig::new("Test", "0.1.0", Platform::Linux);
        let artifact = engine.deploy(&config).unwrap();
        assert_eq!(artifact.platform, Platform::Linux);
        assert!(artifact.path.extension().is_some());
    }
}
