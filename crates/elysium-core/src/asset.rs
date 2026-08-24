use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::any::Any;
use std::fmt::Debug;

/// Asset ID tipi
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(u64);

impl AssetId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
    
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// Asset türleri
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssetType {
    Model,
    Texture,
    Audio,
    Shader,
    Material,
    Animation,
    Font,
    Scene,
    Prefab,
}

/// Asset meta verisi
#[derive(Debug, Clone)]
pub struct AssetMetadata {
    pub asset_type: AssetType,
    pub file_path: PathBuf,
    pub size: u64,
    pub created_at: std::time::SystemTime,
    pub modified_at: std::time::SystemTime,
    pub dependencies: Vec<AssetId>,
}

/// Asset durumu
#[derive(Debug, Clone, PartialEq)]
pub enum AssetStatus {
    Loading,
    Loaded,
    Error(String),
    Unloaded,
}

/// Asset trait'i
pub trait Asset: Any + Send + Sync + Debug {
    fn as_any(&self) -> &dyn Any;
    fn asset_type(&self) -> AssetType;
    fn metadata(&self) -> &AssetMetadata;
    fn status(&self) -> AssetStatus;
}

/// Trait for getting default asset type (used in generic contexts)
pub trait AssetTypeDefault {
    fn asset_type_default() -> AssetType;
}

/// Asset handle
#[derive(Debug, Clone)]
pub struct AssetHandle<T: Asset> {
    pub id: AssetId,
    pub asset: Arc<RwLock<Option<T>>>,
}

impl<T: Asset> AssetHandle<T> {
    pub fn new(id: AssetId, asset: T) -> Self {
        Self {
            id,
            asset: Arc::new(RwLock::new(Some(asset))),
        }
    }
    
    pub fn get(&self) -> Option<std::sync::RwLockReadGuard<Option<T>>> {
        self.asset.read().ok()
    }
    
    pub fn get_mut(&self) -> Option<std::sync::RwLockWriteGuard<Option<T>>> {
        self.asset.write().ok()
    }
}

/// Asset loader trait
pub trait AssetLoader: Send + Sync {
    fn load(&self, path: &PathBuf) -> Result<Box<dyn Asset>, Box<dyn std::error::Error>>;
    fn can_load(&self, path: &PathBuf) -> bool;
    fn asset_type(&self) -> AssetType;
}

/// Asset manager
pub struct AssetManager {
    assets: HashMap<AssetId, Box<dyn Asset>>,
    loaders: HashMap<AssetType, Box<dyn AssetLoader>>,
    handles: HashMap<AssetId, Arc<dyn Any + Send + Sync>>,
    path_to_id: HashMap<PathBuf, AssetId>,
    next_id: u64,
    cache_size_limit: usize,
    current_cache_size: usize,
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetManager {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
            loaders: HashMap::new(),
            handles: HashMap::new(),
            path_to_id: HashMap::new(),
            next_id: 0,
            cache_size_limit: 1024 * 1024 * 512, // 512 MB
            current_cache_size: 0,
        }
    }

    /// Yeni bir asset loader ekler
    pub fn register_loader(&mut self, asset_type: AssetType, loader: Box<dyn AssetLoader>) {
        self.loaders.insert(asset_type, loader);
    }

    /// Asset yükler
    pub fn load_asset<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<AssetId, Box<dyn std::error::Error>> {
        let path = path.as_ref().to_path_buf();
        
        // Zaten yüklü mü kontrol et
        if let Some(&existing_id) = self.path_to_id.get(&path) {
            return Ok(existing_id);
        }
        
        // Doğru loader'ı bul
        let loader = self.find_appropriate_loader(&path)?;
        
        // Asset'i yükle
        let asset = loader.load(&path)?;
        let asset_type = asset.asset_type();
        
        // Yeni ID oluştur
        let id = AssetId::new(self.next_id);
        self.next_id += 1;
        
        // Meta veri oluştur
        let metadata = AssetMetadata {
            asset_type,
            file_path: path.clone(),
            size: 0, // Gerçek boyutu asset'ten almak gerekir
            created_at: std::fs::metadata(&path).map(|m| m.created().unwrap_or_else(|_| std::time::SystemTime::now())).unwrap_or_else(|_| std::time::SystemTime::now()),
            modified_at: std::fs::metadata(&path).map(|m| m.modified().unwrap_or_else(|_| std::time::SystemTime::now())).unwrap_or_else(|_| std::time::SystemTime::now()),
            dependencies: Vec::new(),
        };
        
        // Asset'i ekle
        self.assets.insert(id, asset);
        self.path_to_id.insert(path, id);
        
        Ok(id)
    }

    /// Asset ID'sine göre asset al
    pub fn get_asset(&self, id: AssetId) -> Option<&dyn Asset> {
        self.assets.get(&id).map(|asset| asset.as_ref())
    }

    /// Asset ID'sine göre asset al (mutable)
    pub fn get_asset_mut(&mut self, id: AssetId) -> Option<&mut dyn Asset> {
        self.assets.get_mut(&id).map(|asset| asset.as_mut())
    }

    /// Asset ID'sine göre asset handle al
    pub fn get_handle<T: Asset + AssetTypeDefault>(&self, id: AssetId) -> Option<AssetHandle<T>> {
        if let Some(asset) = self.assets.get(&id) {
            if asset.asset_type() == T::asset_type_default() {
                // Burada aslında doğru tür dönüşümü yapılması gerekir
                // Bu basitleştirilmiş bir versiyon
                None
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Asset sil
    pub fn unload_asset(&mut self, id: AssetId) -> bool {
        if self.assets.remove(&id).is_some() {
            // Path mapping'i kaldır - collect paths first to avoid borrow issues
            let paths_to_remove: Vec<PathBuf> = self.path_to_id
                .iter()
                .filter(|(_, &stored_id)| stored_id == id)
                .map(|(path, _)| path.clone())
                .collect();
            
            for path in paths_to_remove {
                self.path_to_id.remove(&path);
            }
            
            // Handle'ı kaldır
            self.handles.remove(&id);
            
            true
        } else {
            false
        }
    }

    /// Asset ID'sini dosya yoluna göre al
    pub fn get_asset_id_by_path<P: AsRef<std::path::Path>>(&self, path: P) -> Option<AssetId> {
        self.path_to_id.get(path.as_ref()).copied()
    }

    /// Asset türüne göre loader bul
    fn find_appropriate_loader(&self, path: &PathBuf) -> Result<&Box<dyn AssetLoader>, Box<dyn std::error::Error>> {
        for (_, loader) in &self.loaders {
            if loader.can_load(path) {
                return Ok(loader);
            }
        }
        
        Err(format!("No suitable loader found for path: {:?}", path).into())
    }

    /// Asset preload
    pub fn preload_directory<P: AsRef<std::path::Path>>(&mut self, directory: P) -> Result<Vec<AssetId>, Box<dyn std::error::Error>> {
        let mut loaded_assets = Vec::new();
        
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Ok(id) = self.load_asset(&path) {
                    loaded_assets.push(id);
                }
            }
        }
        
        Ok(loaded_assets)
    }

    /// Asset sayısını al
    pub fn asset_count(&self) -> usize {
        self.assets.len()
    }

    /// Cache limitini ayarla (bayt cinsinden)
    pub fn set_cache_size_limit(&mut self, limit: usize) {
        self.cache_size_limit = limit;
    }

    /// Geçerli cache boyutunu al
    pub fn current_cache_size(&self) -> usize {
        self.current_cache_size
    }
}

/// Basit bir örnek asset tanımı
#[derive(Debug)]
pub struct TextureAsset {
    pub metadata: AssetMetadata,
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub data: Vec<u8>,
}

impl TextureAsset {
    pub fn new(metadata: AssetMetadata, width: u32, height: u32, channels: u8, data: Vec<u8>) -> Self {
        Self {
            metadata,
            width,
            height,
            channels,
            data,
        }
    }
}

impl Asset for TextureAsset {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn asset_type(&self) -> AssetType {
        AssetType::Texture
    }
    
    fn metadata(&self) -> &AssetMetadata {
        &self.metadata
    }
    
    fn status(&self) -> AssetStatus {
        AssetStatus::Loaded
    }
}

impl AssetTypeDefault for TextureAsset {
    fn asset_type_default() -> AssetType {
        AssetType::Texture
    }
}

/// Basit bir model asset tanımı
#[derive(Debug)]
pub struct ModelAsset {
    pub metadata: AssetMetadata,
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub materials: Vec<AssetId>,
}

impl ModelAsset {
    pub fn new(metadata: AssetMetadata, vertices: Vec<f32>, indices: Vec<u32>, materials: Vec<AssetId>) -> Self {
        Self {
            metadata,
            vertices,
            indices,
            materials,
        }
    }
}

impl Asset for ModelAsset {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn asset_type(&self) -> AssetType {
        AssetType::Model
    }
    
    fn metadata(&self) -> &AssetMetadata {
        &self.metadata
    }
    
    fn status(&self) -> AssetStatus {
        AssetStatus::Loaded
    }
}

impl AssetTypeDefault for ModelAsset {
    fn asset_type_default() -> AssetType {
        AssetType::Model
    }
}