use crate::{
    Material, PbrLighting, CascadedShadowMap, PostProcessPipeline, 
    ShadowConstants, CascadeConfig
};
use elysium_core::math::{Mat4, Vec3, Vec4};

/// Temel rendering pipeline yapılandırması
#[derive(Debug, Clone)]
pub struct RenderPipelineConfig {
    pub enable_pbr: bool,
    pub enable_shadows: bool,
    pub enable_post_processing: bool,
    pub shadow_config: CascadeConfig,
}

impl Default for RenderPipelineConfig {
    fn default() -> Self {
        Self {
            enable_pbr: true,
            enable_shadows: true,
            enable_post_processing: true,
            shadow_config: CascadeConfig::default(),
        }
    }
}

/// Ana rendering pipeline sistemi
pub struct RenderPipeline {
    pub config: RenderPipelineConfig,
    pub pbr_system: PbrLighting,
    pub shadow_map: CascadedShadowMap,
    pub post_process_pipeline: PostProcessPipeline,
    pub lighting_constants: LightingConstants,
}

/// Aydınlatma sabitlerini içerir
#[derive(Debug, Clone)]
pub struct LightingConstants {
    pub view_matrix: Mat4,
    pub projection_matrix: Mat4,
    pub view_projection_matrix: Mat4,
    pub camera_position: Vec3,
    pub ambient_light: Vec3,
    pub directional_light: DirectionalLight,
}

#[derive(Debug, Clone)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vec3::new(-0.5, -1.0, -0.5).normalize(),
            color: Vec3::ONE,
            intensity: 1.0,
        }
    }
}

impl Default for LightingConstants {
    fn default() -> Self {
        Self {
            view_matrix: Mat4::IDENTITY,
            projection_matrix: Mat4::IDENTITY,
            view_projection_matrix: Mat4::IDENTITY,
            camera_position: Vec3::ZERO,
            ambient_light: Vec3::splat(0.1),
            directional_light: DirectionalLight::default(),
        }
    }
}

impl RenderPipeline {
    /// Yeni bir rendering pipeline sistemi oluşturur
    pub fn new(config: RenderPipelineConfig) -> Self {
        let shadow_map = CascadedShadowMap::new(config.shadow_config.clone());
        let post_process_pipeline = PostProcessPipeline::default();
        
        Self {
            config,
            pbr_system: PbrLighting,
            shadow_map,
            post_process_pipeline,
            lighting_constants: LightingConstants::default(),
        }
    }

    /// Render pipeline'ı günceller
    pub fn update(&mut self) {
        // Aydınlatma sabitlerini güncelle
        self.update_lighting_constants();
        
        // Gerekiyorsa gölge haritalarını güncelle
        if self.config.enable_shadows {
            self.update_shadow_maps();
        }
    }

    /// Aydınlatma sabitlerini günceller
    fn update_lighting_constants(&mut self) {
        self.lighting_constants.view_projection_matrix = 
            self.lighting_constants.projection_matrix * self.lighting_constants.view_matrix;
    }

    /// Gölge haritalarını günceller
    fn update_shadow_maps(&mut self) {
        // Cascade uzaklıklarını hesapla
        self.shadow_map.calculate_cascade_splits(0.1, 1000.0);
        
        // Frustum köşe noktalarını varsayalım
        let frustum_corners = [
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(-1.0, 1.0, 0.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
        ];
        
        // Light view matrisini varsayalım
        let light_view = Mat4::look_at_rh(
            self.lighting_constants.directional_light.direction * 50.0,
            Vec3::ZERO,
            Vec3::Y
        );
        
        self.shadow_map.update_cascades(
            light_view,
            self.lighting_constants.projection_matrix,
            self.lighting_constants.camera_position,
            &frustum_corners
        );
    }

    /// PBR rendering hesaplamalarını yapar
    pub fn calculate_pbr_lighting(
        &self,
        normal: Vec3,
        view_dir: Vec3,
        light_dir: Vec3,
        albedo: Vec3,
        metallic: f32,
        roughness: f32,
        f0: Vec3,
    ) -> Vec3 {
        if self.config.enable_pbr {
            PbrLighting::calculate_lighting(
                normal,
                view_dir,
                light_dir,
                albedo,
                metallic,
                roughness,
                f0,
            )
        } else {
            // Basit lambert aydınlatması
            let n_dot_l = normal.dot(light_dir).max(0.0);
            albedo * n_dot_l
        }
    }

    /// Render pipeline için gerekli tüm sabitleri döndürür
    pub fn get_render_constants(&self) -> (LightingConstants, ShadowConstants) {
        let lighting = self.lighting_constants.clone();
        
        let mut shadows = ShadowConstants::default();
        for (i, cascade) in self.shadow_map.cascades.iter().enumerate() {
            if i < 4 {
                shadows.light_space_matrices[i] = cascade.view_projection_matrix.to_cols_array();
            }
        }
        
        // Cascade bölünmelerini ayarla
        for (i, cascade) in self.shadow_map.cascades.iter().enumerate() {
            if i < 4 {
                shadows.cascade_splits[i] = cascade.world_split_distances[1];
            }
        }
        
        shadows.shadow_params.z = self.shadow_map.cascades.len().min(4) as f32;
        
        (lighting, shadows)
    }

    /// Material shading için gerekli parametreleri hazırlar
    pub fn prepare_material_params(&self, material: &Material) -> MaterialParams {
        MaterialParams {
            base_color: material.base_color,
            metallic: material.metallic,
            roughness: material.roughness,
            normal_scale: material.normal_scale,
            emissive: material.emissive,
            alpha_cutoff: material.alpha_cutoff,
        }
    }
}

/// Material parametrelerini shader'lara göndermek için kullanılır
#[derive(Debug, Clone)]
pub struct MaterialParams {
    pub base_color: Vec4,
    pub metallic: f32,
    pub roughness: f32,
    pub normal_scale: f32,
    pub emissive: Vec3,
    pub alpha_cutoff: f32,
}