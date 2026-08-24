use elysium_core::math::{Mat4, Vec3, Vec4};
use std::collections::HashMap;

/// Cascaded Shadow Map sistemi için cascade yapılandırması
#[derive(Debug, Clone)]
pub struct CascadeConfig {
    pub split_lambda: f32,
    pub resolution: u32,
    pub num_cascades: usize,
    pub depth_bias: f32,
}

impl Default for CascadeConfig {
    fn default() -> Self {
        Self {
            split_lambda: 0.75,
            resolution: 1024,
            num_cascades: 4,
            depth_bias: 0.0005,
        }
    }
}

/// Cascade bilgilerini tutar
#[derive(Debug, Clone)]
pub struct CascadeInfo {
    pub view_projection_matrix: Mat4,
    pub world_split_distances: [f32; 2],
    pub texel_size: f32,
}

/// Cascaded Shadow Map sistemini uygular
pub struct CascadedShadowMap {
    pub cascades: Vec<CascadeInfo>,
    pub config: CascadeConfig,
}

impl CascadedShadowMap {
    /// Yeni bir CSM sistemi oluşturur
    pub fn new(config: CascadeConfig) -> Self {
        let cascades = vec![CascadeInfo {
            view_projection_matrix: Mat4::IDENTITY,
            world_split_distances: [0.0, 0.0],
            texel_size: 0.0,
        }; config.num_cascades];
        
        Self { cascades, config }
    }

    /// Cascade uzaklıklarını hesaplar
    pub fn calculate_cascade_splits(&mut self, near_plane: f32, far_plane: f32) {
        for i in 0..self.config.num_cascades {
            let ni = i as f32;
            let nf = self.config.num_cascades as f32;
            
            let log_split = near_plane * (far_plane / near_plane).powf(ni / nf);
            let uniform_split = near_plane + (far_plane - near_plane) * (ni / nf);
            
            let split_distance = self.config.split_lambda * (log_split - uniform_split) + uniform_split;
            self.cascades[i].world_split_distances[0] = if i == 0 { near_plane } else { self.cascades[i - 1].world_split_distances[1] };
            self.cascades[i].world_split_distances[1] = split_distance;
        }
    }

    /// Cascade view-projection matrislerini hesaplar
    pub fn update_cascades(&mut self, light_view: Mat4, projection: Mat4, camera_pos: Vec3, frustum_corners: &[Vec3; 8]) {
        for i in 0..self.config.num_cascades {
            let cascade_near = self.cascades[i].world_split_distances[0];
            let cascade_far = self.cascades[i].world_split_distances[1];
            
            // Cascade için frustum köşe noktalarını hesapla
            let mut cascade_corners = [Vec3::ZERO; 8];
            for j in 0..8 {
                cascade_corners[j] = frustum_corners[j];
            }
            
            // Light space'e projekte et
            let light_space_points: Vec<Vec3> = cascade_corners.iter()
                .map(|&corner| {
                    let world_point = projection.inverse().transform_point3(corner);
                    light_view.transform_point3(world_point)
                })
                .collect();
            
            // AABB hesapla
            let min_bound = light_space_points.iter().fold(Vec3::splat(f32::MAX), |min, &point| {
                Vec3::new(point.x.min(min.x), point.y.min(min.y), point.z.min(min.z))
            });
            
            let max_bound = light_space_points.iter().fold(Vec3::splat(f32::MIN), |max, &point| {
                Vec3::new(point.x.max(max.x), point.y.max(max.y), point.z.max(max.z))
            });
            
            // Cascade haritası çözünürlüğüne göre texel boyutunu hesapla
            let cascade_width = max_bound.x - min_bound.x;
            let cascade_height = max_bound.y - min_bound.y;
            self.cascades[i].texel_size = cascade_width / self.config.resolution as f32;
            
            // Ortalama bir pozisyon oluştur
            let center = (min_bound + max_bound) * 0.5;
            
            // Cascade view-projection matrisini oluştur
            let cascade_view = Mat4::from_translation(-center);
            let cascade_proj = Mat4::orthographic_rh_gl(
                min_bound.x, max_bound.x,
                min_bound.y, max_bound.y,
                min_bound.z, max_bound.z
            );
            
            self.cascades[i].view_projection_matrix = cascade_proj * cascade_view;
        }
    }
}

/// Shadow mapping için gerekli sabitleri içerir
pub struct ShadowConstants {
    pub light_space_matrices: [[f32; 16]; 4], // Max 4 cascades
    pub cascade_splits: [f32; 4],
    pub shadow_params: Vec4, // x: depth_bias, y: normal_offset_scale, z: cascade_count, w: padding
}

impl Default for ShadowConstants {
    fn default() -> Self {
        Self {
            light_space_matrices: [[0.0; 16]; 4],
            cascade_splits: [0.0; 4],
            shadow_params: Vec4::new(0.0005, 1.0, 4.0, 0.0),
        }
    }
}