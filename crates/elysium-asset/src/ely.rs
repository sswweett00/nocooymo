use std::collections::HashMap;
use std::path::PathBuf;
use std::io::{Read, Write, Cursor};
use serde::{Deserialize, Serialize};
use crate::{AssetType, ImportSettings};

const ELY_MAGIC: &[u8; 4] = b"ELY\0";
const ELY_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElyHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub num_assets: u32,
    pub index_offset: u64,
    pub data_offset: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElyAssetEntry {
    pub id: String,
    pub asset_type: AssetType,
    pub data_offset: u64,
    pub data_size: u64,
    pub compressed_size: u64,
    pub compression_type: CompressionType,
    pub import_settings: ImportSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq)]
pub enum CompressionType {
    None,
    LZ4,
    Zstd,
}

pub struct ElyArchive {
    pub header: ElyHeader,
    pub entries: HashMap<String, ElyAssetEntry>,
    pub data: Vec<u8>,
}

impl ElyArchive {
    pub fn new() -> Self {
        Self {
            header: ElyHeader {
                magic: *ELY_MAGIC,
                version: 1,
                num_assets: 0,
                index_offset: 0,
                data_offset: 0,
            },
            entries: HashMap::new(),
            data: Vec::new(),
        }
    }

    pub fn create_from_directory(dir_path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let mut archive = Self::new();
        
        // Walk through the directory and add all assets
        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                let ext = path.extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                
                let asset_type = match ext.as_str() {
                    "obj" | "gltf" | "glb" | "fbx" => AssetType::Mesh,
                    "png" | "jpg" | "jpeg" | "dds" | "tga" => AssetType::Texture,
                    "mat" | "mtl" => AssetType::Material,
                    "anim" | "smd" => AssetType::Animation,
                    "wav" | "mp3" | "ogg" => AssetType::Audio,
                    "scene" | "ecs" => AssetType::Scene,
                    _ => continue, // Skip unsupported file types
                };
                
                // Load and add the asset
                let asset_data = std::fs::read(&path)?;
                archive.add_asset(&path, asset_data, asset_type)?;
            }
        }
        
        Ok(archive)
    }

    pub fn add_asset(&mut self, path: &PathBuf, data: Vec<u8>, asset_type: AssetType) -> Result<(), Box<dyn std::error::Error>> {
        // Compress the data with the specified compression type
        let compression_type = CompressionType::Zstd; // Default to Zstd for better compression
        let compressed_data = self.compress_data(&data, compression_type)?;
        
        let id = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let entry = ElyAssetEntry {
            id: id.clone(),
            asset_type,
            data_offset: self.data.len() as u64,
            data_size: data.len() as u64,
            compressed_size: compressed_data.len() as u64,
            compression_type,
            import_settings: ImportSettings {
                compression: crate::CompressionType::Zstd,
                quality: 1.0,
                mipmap_levels: 1,
            },
        };
        
        // Add compressed data to our data buffer
        self.data.extend_from_slice(&compressed_data);
        
        // Add entry to our map
        self.entries.insert(id, entry);
        
        // Update header
        self.header.num_assets = self.entries.len() as u32;
        
        Ok(())
    }

    fn compress_data(&self, data: &[u8], compression_type: CompressionType) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        match compression_type {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::LZ4 => self.compress_lz4(data),
            CompressionType::Zstd => self.compress_zstd(data),
        }
    }

    fn compress_lz4(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // LZ4 sıkıştırma - basit implementasyon
        // Gerçek implementasyonda lz4 crate'i kullanılabilir
        use std::io::Write;
        
        let mut encoder = lz4::EncoderBuilder::new()
            .build(Vec::new())
            .map_err(|e| format!("LZ4 encoder creation failed: {}", e))?;
        
        encoder.write_all(data)
            .map_err(|e| format!("LZ4 write failed: {}", e))?;
        
        let (compressed, _) = encoder.finish();
        Ok(compressed)
    }

    fn compress_zstd(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // ZSTD sıkıştırma - basit implementasyon
        // Gerçek implementasyonda zstd crate'i kullanılabilir
        use zstd;
        
        let compressed = zstd::encode_all(Cursor::new(data), 3) // Compression level 3
            .map_err(|e| format!("ZSTD compression failed: {}", e))?;
        
        Ok(compressed)
    }

    pub fn save_to_file(&self, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs::File;
        use std::io::BufWriter;
        
        let mut file = BufWriter::new(File::create(file_path)?);
        
        // Write header
        let header_bytes = bincode::serialize(&self.header)?;
        file.write_all(&header_bytes)?;
        
        // Write entries
        let entries_bytes = bincode::serialize(&self.entries)?;
        file.write_all(&entries_bytes)?;
        
        // Write data
        file.write_all(&self.data)?;
        
        file.flush()?;
        Ok(())
    }

    pub fn load_from_file(file_path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        use std::fs::File;
        use std::io::BufReader;
        
        let mut file = BufReader::new(File::open(file_path)?);
        
        // Read header
        let mut header_bytes = vec![0u8; std::mem::size_of::<ElyHeader>()];
        file.read_exact(&mut header_bytes)?;
        let header: ElyHeader = bincode::deserialize(&header_bytes)?;
        
        // Verify magic
        if &header.magic != ELY_MAGIC {
            return Err("Invalid ELY file".into());
        }
        
        // Read entries
        let mut entries_bytes = vec![0u8; header.index_offset as usize - std::mem::size_of::<ElyHeader>()];
        file.read_exact(&mut entries_bytes)?;
        let entries: HashMap<String, ElyAssetEntry> = bincode::deserialize(&entries_bytes)?;
        
        // Read data
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        
        Ok(Self {
            header,
            entries,
            data,
        })
    }

    pub fn extract_asset(&self, id: &str) -> Option<Vec<u8>> {
        if let Some(entry) = self.entries.get(id) {
            let start = entry.data_offset as usize;
            let end = start + entry.data_size as usize;
            
            if end <= self.data.len() {
                let compressed_data = &self.data[start..end];
                
                // Decompress the data
                match self.decompress_data(compressed_data, &entry.compression_type) {
                    Ok(decompressed) => Some(decompressed),
                    Err(_) => None,
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn decompress_data(&self, data: &[u8], compression_type: &CompressionType) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        match compression_type {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::LZ4 => self.decompress_lz4(data),
            CompressionType::Zstd => self.decompress_zstd(data),
        }
    }

    fn decompress_lz4(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // LZ4 decompression
        use std::io::Read;
        
        let mut decoder = lz4::Decoder::new(data)
            .map_err(|e| format!("LZ4 decoder creation failed: {}", e))?;
        
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| format!("LZ4 read failed: {}", e))?;
        
        Ok(decompressed)
    }

    fn decompress_zstd(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // ZSTD decompression
        use zstd;
        
        let decompressed = zstd::decode_all(Cursor::new(data))
            .map_err(|e| format!("ZSTD decompression failed: {}", e))?;
        
        Ok(decompressed)
    }

    pub fn save_asset(asset_id: &str, asset_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs::File;
        use std::io::BufWriter;
        
        let file_path = PathBuf::from(format!("assets/{}.ely", asset_id));
        
        // Ensure the assets directory exists
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let mut file = BufWriter::new(File::create(file_path)?);
        
        // Write magic
        file.write_all(ELY_MAGIC)?;
        
        // Write asset data length
        let len = asset_data.len() as u32;
        file.write_all(&len.to_le_bytes())?;
        
        // Write asset data
        file.write_all(asset_data)?;
        
        file.flush()?;
        Ok(())
    }
}

// Add bincode as a dependency since we're using it for serialization