use crate::{Entity, World};
use nalgebra::{Vector2, Vector3};
use std::sync::Arc;

/// Arazi parçasının (Chunk) 2D tam sayı koordinatları.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct TerrainCoord {
    pub x: i32,
    pub z: i32,
}

impl TerrainCoord {
    #[inline]
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// Bu chunk'ın dünya matrisindeki sol-alt (minimum) köşe koordinatlarını hesaplar.
    #[inline]
    pub fn to_world_position(&self, chunk_size: f32) -> Vector3<f32> {
        Vector3::new(self.x as f32 * chunk_size, 0.0, self.z as f32 * chunk_size)
    }

    /// Dünya koordinatındaki bir XZ noktasının hangi chunk koordinatına denk geldiğini bulur.
    #[inline]
    pub fn from_world_position(world_pos: Vector2<f32>, chunk_size: f32) -> Self {
        Self {
            x: (world_pos.x / chunk_size).floor() as i32,
            z: (world_pos.y / chunk_size).floor() as i32,
        }
    }
}

/// Eksen Hizalamalı Kutu (Axis-Aligned Bounding Box). 
/// Culling (Kırpma) ve Görünürlük testleri için zorunludur.
#[derive(Clone, Copy, Debug, Default)]
pub struct Aabb {
    pub min: Vector3<f32>,
    pub max: Vector3<f32>,
}

/// Arazinin global yapılandırma ayarları.
#[derive(Clone, Debug)]
pub struct TerrainDescriptor {
    /// Her bir chunk'ın dünya birimi cinsinden genişliği/uzunluğu (Örn: 128.0 metre).
    pub chunk_size: f32,
    /// Arazinin maksimum yüksekliği (Dikey ölçeklendirme).
    pub height_scale: f32,
    /// Vertex yoğunluğu (Örn: 64 ise chunk başına 64x64 kuadratik yüzey üretilir).
    pub tessellation: u32,
    /// Oyuncunun etrafında kaç chunk ötesinin hafızada tutulacağı (Stream yarıçapı).
    pub stream_distance: u32,
    /// İş parçacığı (Worker thread) sayısı sınırlandırmaları için havuz boyutu.
    pub max_loading_threads: usize,
    /// Dünya genişliği (metre cinsinden).
    pub world_size: f32,
}

impl Default for TerrainDescriptor {
    fn default() -> Self {
        Self {
            chunk_size: 128.0,
            height_scale: 256.0,
            tessellation: 64,
            stream_distance: 4,
            max_loading_threads: 4,
            world_size: 2048.0,
        }
    }
}

/// Arazi yükseklik haritası (Heightmap) sağlayıcı arayüzü.
/// İş parçacıkları arasında güvenle taşınabilmesi ve paylaşılabilmesi için `Send + Sync` zorunludur.
pub trait HeightProvider: Send + Sync {
    /// Belirli bir dünya XZ koordinatındaki yüksekliği senkronize veya prosedürel (Noise) olarak döner.
    fn sample(&self, world_xz: Vector2<f32>) -> f32;
    
    /// Belirli bir alanın (AABB) minimum ve maksimum yükseklik sınırlarını optimize şekilde hesaplar.
    /// Bu sayede her vertex'i tek tek taramadan hızlıca Bounds (AABB) çıkarılabilir.
    fn compute_bounds(&self, coord: TerrainCoord, chunk_size: f32) -> (f32, f32);
}

/// Dinamik Detay Seviyesi (Level of Detail) durum yönetimi.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum LodLevel {
    #[default]
    Ultra = 0, // En yüksek vertex yoğunluğu (Kameraya en yakın)
    High = 1,
    Medium = 2,
    Low = 3,   // En düşük vertex yoğunluğu (En uzak)
}

/// Chunk bileşeni (Component). Artık sadece veri taşımıyor, kendi yaşam döngüsünü yönetiyor.
#[derive(Clone, Debug)]
pub struct TerrainChunk {
    pub coord: TerrainCoord,
    pub lod: LodLevel,
    pub bounds: Aabb,
    pub state: ChunkState,
}

/// Chunk'ın asenkron yüklenme ve GPU durum makinesi.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChunkState {
    #[default]
    PendingCpu,  // Arka planda CPU yükseklik verisi hesaplanıyor/okunuyor
    PendingGpu,  // CPU verisi hazır, GPU'ya (VBO/IBO) upload edilmeyi bekliyor
    Active,      // GPU'da aktif ve çizime hazır
    Unloading,   // Bellekten silinme aşamasında
}

/// Endüstri standardı Katmanlı Arazi Malzemesi (PBR Splatmapping).
/// Sadece tek bir renk yerine, dokuların (Texture) ağırlık haritalarına göre karışmasını sağlar.
#[derive(Clone, Debug)]
pub struct TerrainMaterial {
    pub base_color_tint: [f32; 4],
    /// PBR parametreleri: [Roughness, Metallic, Ambient Occlusion, Cavity]
    pub pbr_factors: [f32; 4],
    /// Normal haritasının keskinliği/etki oranı.
    pub normal_scale: f32,
    /// Splatmap (Doku karışım) katmanlarının indeksleri veya handle'ları.
    pub layer_weights_handle: Option<u32>,
}

impl Default for TerrainMaterial {
    fn default() -> Self {
        Self {
            base_color_tint: [1.0, 1.0, 1.0, 1.0],
            pbr_factors: [0.8, 0.0, 1.0, 0.0],
            normal_scale: 1.0,
            layer_weights_handle: None,
        }
    }
}

/// Tüm arazi sistemini koordine eden merkezi Streaming / LOD Yöneticisi.
pub struct TerrainManager {
    pub descriptor: TerrainDescriptor,
    pub provider: Arc<dyn HeightProvider>,
    pub active_chunks: std::collections::HashMap<TerrainCoord, Entity>,
}

impl TerrainManager {
    pub fn new(descriptor: TerrainDescriptor, provider: Arc<dyn HeightProvider>) -> Self {
        Self {
            descriptor,
            provider,
            active_chunks: std::collections::HashMap::new(),
        }
    }

    /// Kameranın veya oyuncunun pozisyonuna göre hangi chunk'ların yüklenmesi, 
    /// hangilerinin silinmesi ve hangi LOD seviyesine geçmesi gerektiğini hesaplar.
    pub fn update_streaming(&mut self, _world: &mut World, camera_pos: Vector3<f32>) {
        let camera_xz = Vector2::new(camera_pos.x, camera_pos.z);
        let center_coord = TerrainCoord::from_world_position(camera_xz, self.descriptor.chunk_size);
        
        // 1. Yeni yüklenmesi gereken chunk'ları tespit et (Stream Radius içi)
        let radius = self.descriptor.stream_distance as i32;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let coord = TerrainCoord::new(center_coord.x + dx, center_coord.z + dz);
                
                if !self.active_chunks.contains_key(&coord) {
                    // Mesafe tabanlı LOD Hesaplama
                    let distance = f32::sqrt((dx * dx + dz * dz) as f32);
                    let _lod = if distance < 2.0 {
                        0
                    } else if distance < 5.0 {
                        1
                    } else {
                        2
                    };

                    // Yükseklik sınırlarını (Bounds) güvenle hesapla
                    let (_min_h, _max_h) = self.provider.compute_bounds(coord, self.descriptor.chunk_size);
                    let _world_min = coord.to_world_position(self.descriptor.chunk_size);
                    
                    let _bounds = Aabb {
                        min: Vector3::new(dx as f32 * self.descriptor.chunk_size - self.descriptor.world_size / 2.0, -10.0, dz as f32 * self.descriptor.chunk_size - self.descriptor.world_size / 2.0),
                        max: Vector3::new((dx as f32 + 1.0) * self.descriptor.chunk_size - self.descriptor.world_size / 2.0, 10.0, (dz as f32 + 1.0) * self.descriptor.chunk_size - self.descriptor.world_size / 2.0),
                    };

                    // ECS üzerinde yeni bir Chunk Entity'si oluştur (Temsili)
                    // let entity = world.spawn().insert(TerrainChunk { ... }).id();
                    // self.active_chunks.insert(coord, entity);
                }
            }
        }

        // 2. Menzil dışı kalan (Out of bounds) chunk'ları evict et / temizle (Hafıza optimizasyonu)
        // (Gerçek projede active_chunks üzerinde retain/drain çalıştırılır)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockNoiseProvider;
    impl HeightProvider for MockNoiseProvider {
        fn sample(&self, _world_xz: Vector2<f32>) -> f32 { 0.5 }
        fn compute_bounds(&self, _coord: TerrainCoord, _chunk_size: f32) -> (f32, f32) { (0.0, 1.0) }
    }

    #[test]
    fn test_terrain_coord_conversion() {
        let chunk_size = 100.0;
        let p = Vector2::new(150.0, -50.0);
        let coord = TerrainCoord::from_world_position(p, chunk_size);
        
        assert_eq!(coord.x, 1);
        assert_eq!(coord.z, -1);
        
        let world_back = coord.to_world_position(chunk_size);
        assert_eq!(world_back.x, 100.0);
        assert_eq!(world_back.z, -100.0);
    }
}