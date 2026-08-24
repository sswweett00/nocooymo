pub mod ely;
pub mod cook;
pub mod pipeline;

pub use ely::*;
pub use cook::*;
pub use pipeline::*;

use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadata {
    pub id: String,
    pub asset_type: AssetType,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
    pub import_settings: ImportSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetType {
    Mesh,
    Texture,
    Material,
    Animation,
    Audio,
    Scene,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSettings {
    pub compression: CompressionType,
    pub quality: f32,
    pub mipmap_levels: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    LZ4,
    Zstd,
    None,
}

pub struct AssetManager {
    pub assets: HashMap<String, AssetMetadata>,
    pub loader: AssetLoader,
    pub cooker: Cooker,
}

pub struct AssetLoader;

impl AssetLoader {
    pub async fn load_asset(&self, path: &PathBuf) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let data = tokio::fs::read(path).await?;
        Ok(data)
    }
}

pub struct Cooker;

impl Cooker {
    pub fn cook_mesh(&self, raw_data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Convert raw mesh data to engine format
        // This would involve parsing the mesh data and converting to the engine's internal format
        Ok(raw_data.to_vec())
    }

    pub fn cook_texture(&self, raw_data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Convert raw texture data to engine format
        // This would involve decompressing, converting formats, generating mipmaps, etc.
        Ok(raw_data.to_vec())
    }

    pub fn cook_material(&self, raw_data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Convert raw material data to engine format
        Ok(raw_data.to_vec())
    }
}

pub struct AssetPipeline {
    pub asset_manager: AssetManager,
    pub cooker: Cooker,
}

impl AssetPipeline {
    pub fn new() -> Self {
        Self {
            asset_manager: AssetManager {
                assets: HashMap::new(),
                loader: AssetLoader,
                cooker: Cooker,
            },
            cooker: Cooker,
        }
    }

    pub async fn import_asset(&mut self, path: PathBuf, asset_type: AssetType) -> Result<String, Box<dyn std::error::Error>> {
        // Load the raw asset
        let raw_data = self.asset_manager.loader.load_asset(&path).await?;
        
        // Cook the asset based on its type
        let cooked_data = match asset_type {
            AssetType::Mesh => self.cooker.cook_mesh(&raw_data)?,
            AssetType::Texture => self.cooker.cook_texture(&raw_data)?,
            AssetType::Material => self.cooker.cook_material(&raw_data)?,
            _ => raw_data,
        };
        
        // Generate an ID for the asset — deterministik hash (md5 yerine)
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        cooked_data.hash(&mut hasher);
        path.hash(&mut hasher);
        let hash_string = format!("{:016x}", hasher.finish());
        let id = format!("asset_{}", hash_string);
        
        // Create metadata
        let metadata = AssetMetadata {
            id: id.clone(),
            asset_type,
            path,
            dependencies: Vec::new(),
            import_settings: ImportSettings {
                compression: CompressionType::None,
                quality: 1.0,
                mipmap_levels: 1,
            },
        };
        
        // Store the asset
        self.asset_manager.assets.insert(id.clone(), metadata);
        
        // Save the cooked asset to the .ely archive format
        ElyArchive::save_asset(&id, &cooked_data)?;
        
        Ok(id)
    }
}
