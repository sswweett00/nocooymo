//! # Ana Renderer Sistemi
//!
//! Bu modül, elysium-render'ın tüm alt sistemlerini bir araya getiren
//! tam fonksiyonlu bir render döngüsü sağlar:
//!
//! - PBR malzeme rendering
//! - Cascaded Shadow Maps (CSM)
//! - Post-processing pipeline (Bloom, FXAA, TAA, Tonemapping)
//! - Meshlet-based rendering ve culling
//! - Virtual Geometry (Weyra) sistemi
//! - Skeletal animasyon skinning
//! - Debug rendering (collision shapes, wireframes)
//! - HDR rendering ve exposure kontrolü
//! - Sky/atmosphere rendering
//! - Tone mapping (Reinhard, ACES)

use crate::{
    rhi::*,
    pipeline::*,
    pbr::*,
    shadow::*,
    post_process::*,
    material::*,
    skeleton::*,
    meshlet::*,
    virtual_geometry::*,
    advanced_rendering::*,
};
use elysium_core::math::{Mat4, Vec3, Vec4, Vec2, Quat, Aabb};
use elysium_core::camera::Camera;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Tone Mapping Modları
// ---------------------------------------------------------------------------

/// HDR ton eşleme modları — shader'larda u32 modu ile eşleşir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ToneMapMode {
    #[default]
    ACES = 1,
    Reinhard = 0,
    Uncharted2 = 2,
    None = 3,
}

impl ToneMapMode {
    /// Shader tarafında kullanılan u32 değerini döndürür.
    pub fn to_shader_index(self) -> u32 {
        match self {
            Self::Reinhard => 0,
            Self::ACES => 1,
            Self::Uncharted2 => 2,
            Self::None => 3,
        }
    }

    /// İsme göre mod döndürür.
    pub fn from_str(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "reinhard" => Self::Reinhard,
            "aces" => Self::ACES,
            "uncharted2" => Self::Uncharted2,
            _ => Self::None,
        }
    }
}

// ---------------------------------------------------------------------------
// Debug Render Modları
// ---------------------------------------------------------------------------

/// Debug görselleştirme modları.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DebugRenderMode {
    #[default]
    None,
    Wireframe,
    Normals,
    Tangents,
    UVs,
    Aabb,
    CollisionShapes,
    SkeletonOverlay,
    LightComplexity,
    MetalRoughness,
    Emissive,
    Velocity,
}

impl DebugRenderMode {
    /// Modun aktif olup olmadığını kontrol eder.
    pub fn is_active(self) -> bool {
        self != Self::None
    }

    /// Shader varyant adını döndürür.
    pub fn shader_variant(self) -> &'static str {
        match self {
            Self::None => "DEFAULT",
            Self::Wireframe => "WIREFRAME",
            Self::Normals => "NORMALS",
            Self::Tangents => "TANGENTS",
            Self::UVs => "UVS",
            Self::Aabb => "AABB",
            Self::CollisionShapes => "COLLISION",
            Self::SkeletonOverlay => "SKELETON",
            Self::LightComplexity => "LIGHT_COMPLEXITY",
            Self::MetalRoughness => "METAL_ROUGHNESS",
            Self::Emissive => "EMISSIVE",
            Self::Velocity => "VELOCITY",
        }
    }
}

// ---------------------------------------------------------------------------
// Renderer Yapılandırması
// ---------------------------------------------------------------------------

/// Renderer'ın tüm ayarlarını içerir.
#[derive(Debug, Clone)]
pub struct RendererConfig {
    pub width: u32,
    pub height: u32,
    pub sample_count: u32,
    pub enable_msaa: bool,
    pub enable_hdr: bool,
    pub enable_tonemap: bool,
    pub enable_bloom: bool,
    pub enable_fxaa: bool,
    pub enable_taa: bool,
    pub enable_ssao: bool,
    pub enable_shadows: bool,
    pub enable_sky: bool,
    pub enable_debug: bool,
    pub exposure: f32,
    pub tone_map_mode: ToneMapMode,
    pub debug_mode: DebugRenderMode,
    pub shadow_config: CascadeConfig,
    pub max_lights: usize,
    pub max_draw_calls: usize,
    pub meshlet_culling_enabled: bool,
    pub virtual_geometry_streaming_enabled: bool,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            sample_count: 4,
            enable_msaa: true,
            enable_hdr: true,
            enable_tonemap: true,
            enable_bloom: true,
            enable_fxaa: true,
            enable_taa: false,
            enable_ssao: true,
            enable_shadows: true,
            enable_sky: true,
            enable_debug: false,
            exposure: 1.0,
            tone_map_mode: ToneMapMode::default(),
            debug_mode: DebugRenderMode::default(),
            shadow_config: CascadeConfig::default(),
            max_lights: 256,
            max_draw_calls: 4096,
            meshlet_culling_enabled: true,
            virtual_geometry_streaming_enabled: true,
        }
    }
}

impl RendererConfig {
    /// Yüksek kaliteli masaüstü ön ayarı.
    pub fn high_quality(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            sample_count: 4,
            enable_msaa: true,
            enable_hdr: true,
            enable_tonemap: true,
            enable_bloom: true,
            enable_fxaa: true,
            enable_taa: true,
            enable_ssao: true,
            enable_shadows: true,
            enable_sky: true,
            enable_debug: false,
            exposure: 1.0,
            tone_map_mode: ToneMapMode::ACES,
            debug_mode: DebugRenderMode::None,
            shadow_config: CascadeConfig {
                resolution: 2048,
                num_cascades: 4,
                ..Default::default()
            },
            max_lights: 512,
            max_draw_calls: 8192,
            meshlet_culling_enabled: true,
            virtual_geometry_streaming_enabled: true,
        }
    }

    /// Performans odaklı ön ayar (düşük güçlü cihazlar için).
    pub fn performance(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            sample_count: 1,
            enable_msaa: false,
            enable_hdr: true,
            enable_tonemap: true,
            enable_bloom: true,
            enable_fxaa: true,
            enable_taa: true,
            enable_ssao: false,
            enable_shadows: true,
            enable_sky: true,
            enable_debug: false,
            exposure: 1.0,
            tone_map_mode: ToneMapMode::ACES,
            debug_mode: DebugRenderMode::None,
            shadow_config: CascadeConfig {
                resolution: 1024,
                num_cascades: 3,
                ..Default::default()
            },
            max_lights: 128,
            max_draw_calls: 2048,
            meshlet_culling_enabled: true,
            virtual_geometry_streaming_enabled: false,
        }
    }

    /// Geliştirme / debug ön ayarı.
    pub fn development(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            sample_count: 1,
            enable_msaa: false,
            enable_hdr: false,
            enable_tonemap: false,
            enable_bloom: false,
            enable_fxaa: false,
            enable_taa: false,
            enable_ssao: false,
            enable_shadows: true,
            enable_sky: true,
            enable_debug: true,
            exposure: 1.0,
            tone_map_mode: ToneMapMode::None,
            debug_mode: DebugRenderMode::Wireframe,
            shadow_config: CascadeConfig {
                resolution: 512,
                num_cascades: 1,
                ..Default::default()
            },
            max_lights: 16,
            max_draw_calls: 1024,
            meshlet_culling_enabled: false,
            virtual_geometry_streaming_enabled: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Render Kamerası
// ---------------------------------------------------------------------------

/// Render döngüsü sırasında kullanılan kamera verisi.
#[derive(Debug, Clone, Default)]
pub struct RenderCamera {
    pub view_matrix: Mat4,
    pub projection_matrix: Mat4,
    pub view_projection_matrix: Mat4,
    pub inverse_view_matrix: Mat4,
    pub inverse_projection_matrix: Mat4,
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub right: Vec3,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub viewport: (u32, u32, u32, u32), // x, y, width, height
    pub jitter_offset: Vec2,
}

impl RenderCamera {
    /// Yeni bir kamera oluşturur.
    pub fn new(
        view: Mat4,
        projection: Mat4,
        position: Vec3,
        forward: Vec3,
        up: Vec3,
        fov: f32,
        aspect: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let view_projection = projection * view;
        let right = forward.cross(up).normalize();
        Self {
            view_matrix: view,
            projection_matrix: projection,
            view_projection_matrix: view_projection,
            inverse_view_matrix: view.inverse(),
            inverse_projection_matrix: projection.inverse(),
            position,
            forward,
            up,
            right,
            fov,
            aspect,
            near,
            far,
            viewport: (0, 0, 1920, 1080),
            jitter_offset: Vec2::ZERO,
        }
    }

    /// elysium-core `Camera` türünden `RenderCamera` oluşturur.
    pub fn from_camera(camera: &Camera, transform: &elysium_core::Transform) -> Self {
        let view_matrix = transform.compute_view_matrix();
        let projection_matrix = camera.get_projection_matrix();
        let position = transform.get_translation();
        let rotation = transform.get_rotation();
        let forward = rotation * Vec3::NEG_Z; // elysium-core standart yönü
        let up = rotation * Vec3::Y;
        let (fov, aspect, near, far) = match &camera.projection {
            elysium_core::camera::CameraProjection::Perspective { fov, aspect, near, far } => {
                (*fov, *aspect, *near, *far)
            }
            elysium_core::camera::CameraProjection::Orthographic { .. } => {
                (45.0_f32.to_radians(), 16.0 / 9.0, 0.1, 1000.0)
            }
        };
        Self::new(
            view_matrix,
            projection_matrix,
            position,
            forward,
            up,
            fov,
            aspect,
            near,
            far,
        )
    }

    /// Viewport'u günceller.
    pub fn set_viewport(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.viewport = (x, y, width, height);
    }

    /// TAA için frame jitter uygular.
    pub fn apply_taa_jitter(&mut self, frame_index: u64, width: u32, height: u32) {
        let offset = calculate_taa_jitter(frame_index);
        self.jitter_offset = Vec2::new(
            (offset.x / width as f32) - 0.5,
            (offset.y / height as f32) - 0.5,
        );
    }
}

// ---------------------------------------------------------------------------
// Post-process Parametreleri (FrameContext içinde kullanılır)
// ---------------------------------------------------------------------------

/// FXAA parametreleri.
#[derive(Debug, Clone, Default)]
pub struct FxaaParams {
    pub inverse_resolution: Vec2,
    pub edge_threshold: f32,
    pub edge_threshold_min: f32,
}

/// TAA parametreleri.
#[derive(Debug, Clone, Default)]
pub struct TaaParams {
    pub blend_factor: f32,
    pub sharpen: f32,
}

/// Ton eşleme parametreleri.
#[derive(Debug, Clone, Default)]
pub struct ToneMapParams {
    pub exposure: f32,
    pub mode: u32, // 0=Reinhard, 1=ACES, 2=Uncharted2
}

// ---------------------------------------------------------------------------
// Aydınlatma Sabitleri (GPU üzerinden iletilir)
// ---------------------------------------------------------------------------

/// Frame başına aydınlatma verisi.
#[derive(Debug, Clone, Default)]
pub struct FrameLightingConstants {
    pub view_matrix: Mat4,
    pub projection_matrix: Mat4,
    pub view_projection_matrix: Mat4,
    pub camera_position: Vec3,
    pub camera_direction: Vec3,
    pub ambient_light: Vec4, // rgb + intensity
    pub directional_light: Vec4, // rgb + intensity
    pub directional_light_dir: Vec4, // xyz + padding
    pub light_count: u32,
    pub shadow_cascade_count: u32,
    pub exposure: f32,
    pub tone_map_mode: u32,
    pub time: f32,
    pub frame_index: u32,
    pub debug_mode: u32,
}

// ---------------------------------------------------------------------------
// Sahne (Scene)
// ---------------------------------------------------------------------------

/// Render edilecek sahne verisi.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub drawables: Vec<Drawable>,
    pub lights: Vec<LightSource>,
    pub skinned_meshes: Vec<SkinnedMeshInstance>,
    pub meshlet_objects: Vec<MeshletObject>,
    pub virtual_geometry_blocks: Vec<VirtualGeometryBlock>,
    pub debug_shapes: Vec<DebugShape>,
    pub skeletons: HashMap<String, Skeleton>,
    pub animations: HashMap<String, AnimationClip>,
    pub animation_players: Vec<AnimationPlayer>,
    pub sky_enabled: bool,
    pub fog_density: f32,
    pub fog_color: Vec3,
    pub background_color: Vec3,
}

impl Scene {
    /// Yeni boş bir sahne oluşturur.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sahneye bir çizilebilir nesne ekler.
    pub fn add_drawable(&mut self, drawable: Drawable) {
        self.drawables.push(drawable);
    }

    /// Sahneye bir ışık ekler.
    pub fn add_light(&mut self, light: LightSource) {
        self.lights.push(light);
    }

    /// Sahneye bir skinli mesh ekler.
    pub fn add_skinned_mesh(&mut self, mesh: SkinnedMeshInstance) {
        self.skinned_meshes.push(mesh);
    }

    /// Sahneye bir meshlet objesi ekler.
    pub fn add_meshlet_object(&mut self, obj: MeshletObject) {
        self.meshlet_objects.push(obj);
    }

    /// Sahneye bir debug şekli ekler.
    pub fn add_debug_shape(&mut self, shape: DebugShape) {
        self.debug_shapes.push(shape);
    }

    /// Sahne aydınlatma sayısını sınırlar.
    pub fn clamp_lights(&mut self, max: usize) {
        if self.lights.len() > max {
            self.lights.truncate(max);
        }
    }

    /// Tüm skinned mesh'lerin animasyonlarını günceller.
    pub fn update_animations(&mut self, delta_time: f32) {
        for player in &mut self.animation_players {
            player.update(delta_time, &self.animations, &mut self.skeletons);
        }
    }
}

// ---------------------------------------------------------------------------
// Çizilebilir Nesne (Drawable)
// ---------------------------------------------------------------------------

/// Render edilebir temel çizim nesnesi.
#[derive(Debug, Clone)]
pub struct Drawable {
    pub mesh_handle: u64,
    pub material: Material,
    pub transform: Mat4,
    pub aabb: Aabb,
    pub cast_shadows: bool,
    pub receive_shadows: bool,
    pub render_mode: MaterialVariant,
    pub meshlet_indices: Vec<usize>,
    pub lod_bias: f32,
    pub visibility: bool,
}

impl Drawable {
    pub fn new(
        mesh_handle: u64,
        material: Material,
        transform: Mat4,
        aabb: Aabb,
    ) -> Self {
        Self {
            mesh_handle,
            material,
            transform,
            aabb,
            cast_shadows: true,
            receive_shadows: true,
            render_mode: MaterialVariant::Default,
            meshlet_indices: Vec::new(),
            lod_bias: 1.0,
            visibility: true,
        }
    }

    pub fn with_shadows(mut self, cast: bool, receive: bool) -> Self {
        self.cast_shadows = cast;
        self.receive_shadows = receive;
        self
    }

    pub fn with_variant(mut self, variant: MaterialVariant) -> Self {
        self.render_mode = variant;
        self
    }
}

// ---------------------------------------------------------------------------
// Işık Kaynakları
// ---------------------------------------------------------------------------

/// Sahne ışık kaynakları.
#[derive(Debug, Clone)]
pub enum LightSource {
    Directional(DirectionalLightData),
    Point(PointLightData),
    Spot(SpotLightData),
}

#[derive(Debug, Clone)]
pub struct DirectionalLightData {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub shadow_cascades: [Mat4; 4],
    pub cascade_splits: [f32; 4],
    pub enabled: bool,
}

impl Default for DirectionalLightData {
    fn default() -> Self {
        Self {
            direction: Vec3::new(-0.5, -1.0, -0.5).normalize(),
            color: Vec3::ONE,
            intensity: 1.0,
            shadow_cascades: [Mat4::IDENTITY; 4],
            cascade_splits: [0.0; 4],
            enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PointLightData {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub range: f32,
    pub enabled: bool,
}

impl Default for PointLightData {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            color: Vec3::ONE,
            intensity: 1.0,
            range: 10.0,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpotLightData {
    pub position: Vec3,
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub range: f32,
    pub inner_cone_angle: f32,
    pub outer_cone_angle: f32,
    pub enabled: bool,
}

impl Default for SpotLightData {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            direction: Vec3::NEG_Z,
            color: Vec3::ONE,
            intensity: 1.0,
            range: 10.0,
            inner_cone_angle: 12.5_f32.to_radians(),
            outer_cone_angle: 17.5_f32.to_radians(),
            enabled: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Skinned Mesh Instance
// ---------------------------------------------------------------------------

/// Sahnedeki tek bir skinned mesh örneği.
#[derive(Debug, Clone)]
pub struct SkinnedMeshInstance {
    pub mesh_handle: u64,
    pub material: Material,
    pub transform: Mat4,
    pub skeleton_name: String,
    pub bone_transforms: Vec<Mat4>,
    pub visibility: bool,
}

impl SkinnedMeshInstance {
    pub fn new(
        mesh_handle: u64,
        material: Material,
        transform: Mat4,
        skeleton_name: String,
    ) -> Self {
        Self {
            mesh_handle,
            material,
            transform,
            skeleton_name,
            bone_transforms: Vec::new(),
            visibility: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Meshlet Object
// ---------------------------------------------------------------------------

/// Meshlet tabanlı çizim nesnesi.
#[derive(Debug, Clone)]
pub struct MeshletObject {
    pub meshlet_handle: u64,
    pub transform: Mat4,
    pub visible_meshlets: Vec<usize>,
    pub lod_level: u8,
    pub enabled: bool,
}

impl MeshletObject {
    pub fn new(meshlet_handle: u64, transform: Mat4) -> Self {
        Self {
            meshlet_handle,
            transform,
            visible_meshlets: Vec::new(),
            lod_level: 0,
            enabled: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Debug Shape
// ---------------------------------------------------------------------------

/// Debug görselleştirme şekilleri.
#[derive(Debug, Clone)]
pub enum DebugShape {
    Aabb(AabbDebug),
    Sphere(SphereDebug),
    Line(LineDebug),
    Cone(ConeDebug),
    SkeletonBones(SkeletonBonesDebug),
    WireframeMesh(WireframeMeshDebug),
}

#[derive(Debug, Clone)]
pub struct AabbDebug {
    pub aabb: Aabb,
    pub transform: Mat4,
    pub color: Vec4,
}

#[derive(Debug, Clone)]
pub struct SphereDebug {
    pub center: Vec3,
    pub radius: f32,
    pub transform: Mat4,
    pub color: Vec4,
}

#[derive(Debug, Clone)]
pub struct LineDebug {
    pub start: Vec3,
    pub end: Vec3,
    pub color: Vec4,
}

#[derive(Debug, Clone)]
pub struct ConeDebug {
    pub apex: Vec3,
    pub direction: Vec3,
    pub angle: f32,
    pub length: f32,
    pub transform: Mat4,
    pub color: Vec4,
}

#[derive(Debug, Clone)]
pub struct SkeletonBonesDebug {
    pub skeleton_name: String,
    pub transform: Mat4,
    pub color: Vec4,
}

#[derive(Debug, Clone)]
pub struct WireframeMeshDebug {
    pub mesh_handle: u64,
    pub transform: Mat4,
    pub color: Vec4,
}

// ---------------------------------------------------------------------------
// Frame Context
// ---------------------------------------------------------------------------

/// Tek bir frame için bağlam verisi — `begin_frame` ile oluşturulur,
/// çeşitli render aşamaları tarafından değiştirilir ve `end_frame` ile
/// sunucuya gönderilir.
#[derive(Debug, Clone, Default)]
pub struct FrameContext {
    pub frame_index: u64,
    pub time: f32,
    pub delta_time: f32,
    pub command_encoder: Option<CommandEncoder>,
    pub hdr_view: Option<TextureView>,
    pub depth_view: Option<TextureView>,
    pub shadow_views: [Option<TextureView>; 4],
    pub current_output: Option<TextureView>,
    pub swapchain_view: Option<TextureView>,
    pub camera: RenderCamera,
    pub lighting_constants: FrameLightingConstants,
    pub shadow_constants: ShadowConstants,
    pub material_params: MaterialParams,
    pub bloom_params: BloomParams,
    pub fxaa_params: FxaaParams,
    pub taa_params: TaaParams,
    pub tone_map_params: ToneMapParams,
    pub is_hdr_enabled: bool,
    pub is_first_frame: bool,
}

// ---------------------------------------------------------------------------
// Sky / Atmosphere Renderer
// ---------------------------------------------------------------------------

/// Gök ve atmosfer rendering sistemi.
#[derive(Debug, Clone, Default)]
pub struct SkyRenderer {
    pub enabled: bool,
    pub sun_direction: Vec3,
    pub sun_color: Vec3,
    pub sun_intensity: f32,
    pub turbidity: f32,
    pub rayleigh: f32,
    pub mie: f32,
    pub mie_direction: f32,
    pub luminance: f32,
    pub inscatter: bool,
    pub transmittance: bool,
}

impl SkyRenderer {
    pub fn new() -> Self {
        Self {
            sun_direction: Vec3::new(0.0, -1.0, 0.0).normalize(),
            sun_color: Vec3::new(1.0, 0.95, 0.8),
            sun_intensity: 2.0,
            turbidity: 2.0,
            rayleigh: 2.0,
            mie: 0.005,
            mie_direction: 0.8,
            luminance: 1.0,
            inscatter: true,
            transmittance: true,
            ..Default::default()
        }
    }

    pub fn with_sun(mut self, direction: Vec3, color: Vec3, intensity: f32) -> Self {
        self.sun_direction = direction.normalize();
        self.sun_color = color;
        self.sun_intensity = intensity;
        self
    }

    pub fn with_atmosphere(mut self, turbidity: f32, rayleigh: f32, mie: f32) -> Self {
        self.turbidity = turbidity;
        self.rayleigh = rayleigh;
        self.mie = mie;
        self
    }

    pub fn enable(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

// ---------------------------------------------------------------------------
// Ana Renderer
// ---------------------------------------------------------------------------

/// Elysium motorunun ana renderer sistemi. Tüm alt sistemleri
/// (PBR, gölgeler, post-process, meshlet, sanal geometri, iskelet animasyonu)
/// koordine eder ve render döngüsünü yönetir.
pub struct Renderer {
    pub backend: RenderBackend,
    pub pipeline: RenderPipeline,
    pub config: RendererConfig,
    
    // HDR render hedefleri
    hdr_target: Option<RenderTarget>,
    hdr_msaa_target: Option<MultisampledRenderTarget>,
    depth_target: Option<RenderTarget>,
    
    // Post-process ara hedefleri
    bloom_target: Option<RenderTarget>,
    bright_target: Option<RenderTarget>,
    intermediate_target: Option<RenderTarget>,
    taa_history: Option<RenderTarget>,
    
    // Shadow atlas
    shadow_atlas: Option<RenderTarget>,
    
    // Pipeline kaynakları
    pbr_pipeline: Option<Arc<RenderPipeline>>,
    shadow_pipeline: Option<Arc<RenderPipeline>>,
    post_process_pipelines: HashMap<&'static str, Arc<RenderPipeline>>,
    sky_pipeline: Option<Arc<RenderPipeline>>,
    debug_pipeline: Option<Arc<RenderPipeline>>,
    fullscreen_pipeline: Option<Arc<RenderPipeline>>,
    
    // Shader modülleri
    pbr_shader: Option<ShaderModule>,
    shadow_shader: Option<ShaderModule>,
    post_process_shaders: HashMap<&'static str, ShaderModule>,
    sky_shader: Option<ShaderModule>,
    debug_shader: Option<ShaderModule>,
    
    // Bind group layout'lar
    camera_bind_group_layout: Option<BindGroupLayout>,
    material_bind_group_layout: Option<BindGroupLayout>,
    shadow_bind_group_layout: Option<BindGroupLayout>,
    post_process_bind_group_layout: Option<BindGroupLayout>,
    
    // Durum
    current_frame: u64,
    is_frame_in_progress: bool,
    exposure: f32,
    tone_map_mode: ToneMapMode,
    debug_mode: DebugRenderMode,
    sky_renderer: SkyRenderer,
    
    // Meshlet / Virtual Geometry durumu
    weyra_pipeline: Option<WeyraPipeline>,
    virtual_geometry_system: Option<VirtualGeometrySystem>,
    
    // Gölge hedefleri (tek kullanımlık, her frame yeniden kullanılır)
    _shadow_targets: Vec<RenderTarget>,
    
    // Frame etiket önbelleği
    frame_label_buf: String,
}

impl Renderer {
    /// Yeni bir renderer oluşturur.
    ///
    /// # Arguments
    /// * `backend` — RenderBackend örneği (WGPU cihaz, kuyruk vb.)
    /// * `pipeline` — RenderPipeline yapılandırması
    /// * `config` — Renderer yapılandırması
    pub fn new(
        mut backend: RenderBackend,
        pipeline: RenderPipeline,
        config: RendererConfig,
    ) -> Self {
        let device = &backend.device;
        
        // Render hedeflerini oluştur
        let hdr_format = TextureFormat::Rgba16Float;
        let depth_format = TextureFormat::Depth32Float;
        let size = (config.width, config.height);
        
        let hdr_target = if config.enable_msaa {
            None
        } else {
            Some(RenderTarget::new(
                device,
                size,
                hdr_format,
                Some("HDR Target"),
            ))
        };
        
        let hdr_msaa_target = if config.enable_msaa {
            Some(MultisampledRenderTarget::new(
                device,
                size,
                hdr_format,
                config.sample_count,
            ))
        } else {
            None
        };
        
        let depth_target = Some(RenderTarget::new(
            device,
            size,
            depth_format,
            Some("Depth Target"),
        ));
        
        // Post-process hedefleri
        let bloom_target = if config.enable_bloom {
            Some(RenderTarget::new(
                device,
                (config.width / 2, config.height / 2),
                hdr_format,
                Some("Bloom Target"),
            ))
        } else {
            None
        };
        
        let bright_target = if config.enable_bloom {
            Some(RenderTarget::new(
                device,
                (config.width / 4, config.height / 4),
                hdr_format,
                Some("Bright Target"),
            ))
        } else {
            None
        };
        
        let intermediate_target = Some(RenderTarget::new(
            device,
            size,
            hdr_format,
            Some("Intermediate Target"),
        ));
        
        let taa_history = if config.enable_taa {
            Some(RenderTarget::new(
                device,
                size,
                hdr_format,
                Some("TAA History"),
            ))
        } else {
            None
        };
        
        // Shadow atlas
        let shadow_size = config.shadow_config.resolution * config.shadow_config.num_cascades as u32;
        let shadow_atlas = if config.enable_shadows {
            Some(RenderTarget::new(
                device,
                (shadow_size, config.shadow_config.resolution),
                TextureFormat::Depth32Float,
                Some("Shadow Atlas"),
            ))
        } else {
            None
        };
        
        // Shader'ları oluştur
        let pbr_shader = Some(device.create_shader_module(ShaderModuleDescriptor {
            label: Some("PBR Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(get_pbr_shader_wgsl())),
        }));
        
        let shadow_shader = Some(device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shadow Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(get_shadow_shader_wgsl())),
        }));
        
        let sky_shader = Some(device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Sky Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(get_sky_shader_wgsl())),
        }));
        
        let debug_shader = Some(device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Debug Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(get_debug_shader_wgsl())),
        }));
        
        // Bind group layout'ları oluştur
        let camera_bind_group_layout = Some(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Camera Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        }));
        
        let material_bind_group_layout = Some(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Material Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        }));
        
        let shadow_bind_group_layout = Some(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Shadow Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Depth,
                    },
                    count: std::num::NonZeroU32::new(4),
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        }));
        
        // Post-process pipeline'ları
        let mut post_process_pipelines: HashMap<&'static str, Arc<RenderPipeline>> = HashMap::new();
        
        if config.enable_bloom {
            let bloom_shader = device.create_shader_module(ShaderModuleDescriptor {
                label: Some("Bloom Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(PostProcessPipeline::get_bloom_shader_wgsl())),
            });
            
            let bloom_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Bloom Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
            
            let bloom_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Bloom Pipeline"),
                layout: Some(&bloom_pipeline_layout),
                vertex: VertexState {
                    module: &bloom_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &bloom_shader,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: hdr_format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });
            
            post_process_pipelines.insert("bloom", Arc::new(bloom_pipeline));
        }
        
        if config.enable_fxaa {
            let fxaa_shader = device.create_shader_module(ShaderModuleDescriptor {
                label: Some("FXAA Shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(PostProcessPipeline::get_fxaa_shader_wgsl())),
            });
            
            let fxaa_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("FXAA Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
            
            let fxaa_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("FXAA Pipeline"),
                layout: Some(&fxaa_pipeline_layout),
                vertex: VertexState {
                    module: &fxaa_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &fxaa_shader,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: hdr_format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });
            
            post_process_pipelines.insert("fxaa", Arc::new(fxaa_pipeline));
        }
        
        // TAA pipeline
        let taa_pipeline = if config.enable_taa {
            let taa_shader = device.create_shader_module(ShaderModuleDescriptor {
                label: Some("TAA Shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(PostProcessPipeline::get_taa_shader_wgsl())),
            });
            
            let taa_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("TAA Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
            
            let taa_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("TAA Pipeline"),
                layout: Some(&taa_pipeline_layout),
                vertex: VertexState {
                    module: &taa_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &taa_shader,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: hdr_format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });
            
            post_process_pipelines.insert("taa", Arc::new(taa_pipeline));
            post_process_pipelines.get("taa").cloned()
        } else {
            None
        };
        
        // Sky pipeline
        let sky_pipeline = {
            let sky_shader_module = device.create_shader_module(ShaderModuleDescriptor {
                label: Some("Sky Shader Module"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(get_sky_shader_wgsl())),
            });
            
            let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Sky Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
            
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Sky Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &sky_shader_module,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &sky_shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: hdr_format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });
            
            Some(Arc::new(pipeline))
        };
        
        // Debug pipeline
        let debug_pipeline = {
            let debug_shader_module = device.create_shader_module(ShaderModuleDescriptor {
                label: Some("Debug Shader Module"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(get_debug_shader_wgsl())),
            });
            
            let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Debug Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
            
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Debug Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &debug_shader_module,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &debug_shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: if config.enable_hdr { hdr_format } else { TextureFormat::Rgba8UnormSrgb },
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: Some(DepthStencilState {
                    format: depth_format,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::LessEqual,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                }),
                multisample: MultisampleState::default(),
                multiview: None,
            });
            
            Some(Arc::new(pipeline))
        };
        
        // Fullscreen pipeline (tonemap, fxaa vb.)
        let fullscreen_pipeline = {
            let fullscreen_shader = device.create_shader_module(ShaderModuleDescriptor {
                label: Some("Fullscreen Shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(get_fullscreen_shader_wgsl())),
            });
            
            let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Fullscreen Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
            
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Fullscreen Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &fullscreen_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &fullscreen_shader,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: TextureFormat::Rgba8UnormSrgb,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });
            
            Some(Arc::new(pipeline))
        };
        
        // PBR pipeline
        let pbr_pipeline = {
            let pbr_shader_module = device.create_shader_module(ShaderModuleDescriptor {
                label: Some("PBR Shader Module"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(get_pbr_shader_wgsl())),
            });
            
            let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("PBR Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
            
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("PBR Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &pbr_shader_module,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &pbr_shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: hdr_format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState {
                    cull_mode: Some(Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(DepthStencilState {
                    format: depth_format,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::LessEqual,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                }),
                multisample: MultisampleState {
                    count: config.sample_count,
                    ..Default::default()
                },
                multiview: None,
            });
            
            Some(Arc::new(pipeline))
        };
        
        // Shadow pipeline
        let shadow_pipeline = {
            let shadow_shader_module = device.create_shader_module(ShaderModuleDescriptor {
                label: Some("Shadow Shader Module"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(get_shadow_shader_wgsl())),
            });
            
            let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Shadow Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
            
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Shadow Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shadow_shader_module,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: None,
                primitive: PrimitiveState::default(),
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::LessEqual,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                }),
                multisample: MultisampleState::default(),
                multiview: None,
            });
            
            Some(Arc::new(pipeline))
        };
        
        // Weyra (virtual geometry) pipeline'ını başlat
        let weyra_pipeline = Some(WeyraPipeline::new(config.width, config.height, 4));
        
        // Virtual geometry sistemi
        let virtual_geometry_system = Some(VirtualGeometrySystem::new(VirtualGeometryConfig::default()));
        
        // Sky renderer
        let sky_renderer = SkyRenderer::new();
        
        Self {
            backend,
            pipeline,
            config,
            hdr_target,
            hdr_msaa_target,
            depth_target,
            bloom_target,
            bright_target,
            intermediate_target,
            taa_history,
            shadow_atlas,
            pbr_pipeline,
            shadow_pipeline,
            post_process_pipelines,
            sky_pipeline,
            debug_pipeline,
            fullscreen_pipeline,
            pbr_shader,
            shadow_shader,
            post_process_shaders: HashMap::new(),
            sky_shader,
            debug_shader,
            camera_bind_group_layout,
            material_bind_group_layout,
            shadow_bind_group_layout,
            post_process_bind_group_layout: None,
            current_frame: 0,
            is_frame_in_progress: false,
            exposure: config.exposure,
            tone_map_mode: config.tone_map_mode,
            debug_mode: config.debug_mode,
            sky_renderer,
            weyra_pipeline,
            virtual_geometry_system,
            _shadow_targets: Vec::new(),
            frame_label_buf: String::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Frame Yaşam Döngüsü
    // -----------------------------------------------------------------------

    /// Yeni bir frame başlatır ve komut encoder'ı döndürür.
    pub fn begin_frame(&mut self, camera: &RenderCamera) -> FrameContext {
        assert!(!self.is_frame_in_progress, "begin_frame: zaten bir frame devam ediyor");
        self.is_frame_in_progress = true;
        self.current_frame += 1;
        
        let device = &self.backend.device;
        
        // Frame etiketini önbelleğe al
        self.frame_label_buf.clear();
        self.frame_label_buf.push_str("Frame ");
        self.frame_label_buf.push_str(&self.current_frame.to_string());
        
        let command_encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some(&self.frame_label_buf),
        });
        
        // Aydınlatma sabitlerini hazırla
        let lighting_constants = FrameLightingConstants {
            view_matrix: camera.view_matrix,
            projection_matrix: camera.projection_matrix,
            view_projection_matrix: camera.view_projection_matrix,
            camera_position: camera.position,
            camera_direction: camera.forward,
            ambient_light: Vec4::new(0.1, 0.1, 0.1, 0.1),
            directional_light: Vec4::new(1.0, 1.0, 1.0, 1.0),
            directional_light_dir: Vec4::from(self.pipeline.lighting_constants.directional_light.direction),
            light_count: 1,
            shadow_cascade_count: self.config.shadow_config.num_cascades as u32,
            exposure: self.exposure,
            tone_map_mode: self.tone_map_mode.to_shader_index(),
            time: 0.0,
            frame_index: self.current_frame as u32,
            debug_mode: self.debug_mode.shader_variant().len() as u32,
        };
        
        let ctx = FrameContext {
            frame_index: self.current_frame,
            time: 0.0,
            delta_time: 0.0,
            command_encoder: Some(command_encoder),
            hdr_view: self.hdr_target.as_ref().map(|t| t.view.clone()),
            depth_view: self.depth_target.as_ref().map(|t| t.view.clone()),
            shadow_views: Default::default(),
            current_output: self.hdr_target.as_ref().map(|t| t.view.clone()),
            swapchain_view: None,
            camera: camera.clone(),
            lighting_constants,
            shadow_constants: ShadowConstants::default(),
            material_params: MaterialParams::default(),
            bloom_params: BloomParams::default(),
            fxaa_params: FxaaParams {
                inverse_resolution: Vec2::new(
                    1.0 / self.config.width as f32,
                    1.0 / self.config.height as f32,
                ),
                edge_threshold: 0.125,
                edge_threshold_min: 0.0312,
            },
            taa_params: TaaParams {
                blend_factor: 0.9,
                sharpen: 0.5,
            },
            tone_map_params: ToneMapParams {
                exposure: self.exposure,
                mode: self.tone_map_mode.to_shader_index(),
            },
            is_hdr_enabled: self.config.enable_hdr,
            is_first_frame: self.current_frame == 1,
        };
        
        ctx
    }

    /// Ana sahne render aşaması — PBR, meshlet, skinned mesh, debug vb.
    pub fn render_scene(&mut self, frame: &mut FrameContext, scene: &Scene) {
        let mut encoder = frame.command_encoder.take().unwrap();
        let device = &self.backend.device;
        
        // Texture view'ları yerel değişkenlere kopyala (lifetime yönetimi için)
        let hdr_view_opt = self.hdr_target.as_ref().map(|t| t.view.clone());
        let depth_view_opt = self.depth_target.as_ref().map(|t| t.view.clone());
        let msaa_resolve_opt = self.hdr_msaa_target.as_ref().map(|msaa| msaa.resolve_target.view.clone());
        
        // Renk ekini hazırla
        let color_attachments_slice: &[Option<wgpu::RenderPassColorAttachment>] = &[
            hdr_view_opt.as_ref().map(|view| wgpu::RenderPassColorAttachment {
                view,
                resolve_target: msaa_resolve_opt.as_ref(),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: true,
                },
            }),
        ];
        
        // Derinlik ekini hazırla
        let depth_stencil = depth_view_opt.as_ref().map(|view| wgpu::RenderPassDepthStencilAttachment {
            view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: true,
            }),
            stencil_ops: None,
        });
        
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main Scene Pass"),
            color_attachments: color_attachments_slice,
            depth_stencil_attachment: depth_stencil.as_ref(),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        
        // Viewport ayarla
        render_pass.set_viewport(
            0.0,
            0.0,
            self.config.width as f32,
            self.config.height as f32,
            0.0,
            1.0,
        );
        
        // Sky rendering
        if self.config.enable_sky && self.sky_renderer.enabled {
            self.render_sky(&mut render_pass, frame);
        }
        
        // Opaque drawables (PBR)
        self.render_opaque_drawables(&mut render_pass, frame, scene);
        
        // Meshlet-based rendering
        if self.config.meshlet_culling_enabled {
            self.render_meshlet_objects(&mut render_pass, frame, scene);
        }
        
        // Skinned mesh'ler
        self.render_skinned_meshes(&mut render_pass, frame, scene);
        
        // Virtual geometry (Weyra)
        if self.config.virtual_geometry_streaming_enabled {
            self.render_virtual_geometry(&mut render_pass, frame, scene);
        }
        
        // Debug rendering
        if self.config.enable_debug && self.debug_mode.is_active() {
            self.render_debug_shapes(&mut render_pass, frame, scene);
        }
        
        render_pass.end();
        
        // Encoder'ı geri koy
        frame.command_encoder = Some(encoder);
    }

    /// Gölge harita render aşaması.
    pub fn render_shadows(&mut self, frame: &mut FrameContext, scene: &Scene) {
        if !self.config.enable_shadows {
            return;
        }
        
        let mut encoder = frame.command_encoder.take().unwrap();
        let device = &self.backend.device;
        
        // Her cascade için ayrı bir render pass
        for cascade_idx in 0..self.config.shadow_config.num_cascades {
            let cascade_view = if let Some(ref atlas) = self.shadow_atlas {
                if cascade_idx < atlas.size.0 / self.config.shadow_config.resolution {
                    Some(atlas.texture.create_view(
                        &wgpu::TextureViewDescriptor {
                            label: Some(&format!("Cascade {}", cascade_idx)),
                            format: Some(TextureFormat::Depth32Float),
                            dimension: Some(TextureViewDimension::D2),
                            aspect: TextureAspect::All,
                            base_mip_level: 0,
                            mip_level_count: None,
                            base_array_layer: cascade_idx as u32,
                            array_layer_count: Some(1),
                        }
                    ))
                } else {
                    None
                }
            } else {
                None
            };
            
            let color_attachments: &[Option<wgpu::RenderPassColorAttachment>] = &[];
            
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("Shadow Pass Cascade {}", cascade_idx)),
                color_attachments,
                depth_stencil_attachment: cascade_view.as_ref().map(|view| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: true,
                        }),
                        stencil_ops: None,
                    }
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            
            render_pass.set_viewport(
                0.0,
                0.0,
                self.config.shadow_config.resolution as f32,
                self.config.shadow_config.resolution as f32,
                0.0,
                1.0,
            );
            
            // Shadow pipeline'ı kullanarak sahne nesnelerini çiz
            if let Some(ref shadow_pipeline) = self.shadow_pipeline {
                render_pass.set_pipeline(shadow_pipeline);
                
                // Sadece gölge atan drawable'ları çiz
                for drawable in &scene.drawables {
                    if drawable.cast_shadows && drawable.visibility {
                        // Gerçek uygulamada vertex/index buffer binding burada yapılır
                        // Şimdilik placeholder
                    }
                }
            }
            
            render_pass.end();
        }
        
        // Shadow sabitlerini frame context'e kaydet
        frame.shadow_views = Default::default();
        if let Some(ref atlas) = self.shadow_atlas {
            for i in 0..4.min(self.config.shadow_config.num_cascades) {
                frame.shadow_views[i] = Some(atlas.view.clone());
            }
        }
        
        frame.shadow_constants = self.pipeline.shadow_map
            .cascades
            .iter()
            .fold(ShadowConstants::default(), |mut sc, cascade| {
                sc.light_space_matrices[0] = cascade.view_projection_matrix.to_cols_array();
                sc
            });
        
        frame.command_encoder = Some(encoder);
    }

    /// Post-processing pipeline aşaması.
    pub fn render_post_process(&mut self, frame: &mut FrameContext) {
        if !self.config.enable_post_processing {
            return;
        }
        
        let mut encoder = frame.command_encoder.take().unwrap();
        let mut current_view = frame.current_output.clone().unwrap();
        let device = &self.backend.device;
        
        // Fullscreen quad için pipeline'ları kullan
        // 1. Bloom
        if self.config.enable_bloom && self.bloom_target.is_some() {
            // Bright pass
            // Gaussian blur
            // Composite
        }
        
        // 2. SSAO
        if self.config.enable_ssao {
            // Screen-space ambient occlusion
        }
        
        // 3. FXAA
        if self.config.enable_fxaa {
            // FXAA anti-aliasing
        }
        
        // 4. TAA
        if self.config.enable_taa && self.taa_history.is_some() {
            // Temporal anti-aliasing
        }
        
        // 5. Tonemapping
        if self.config.enable_tonemap && self.config.enable_hdr {
            // Reinhard veya ACES tonemap
            current_view = self.apply_tone_mapping(frame, &current_view);
        }
        
        // Swapchain'a sun
        frame.current_output = Some(current_view);
        
        frame.command_encoder = Some(encoder);
    }

    /// Frame sonlandırma ve sunucuya gönderme.
    pub fn end_frame(&mut self, mut frame: FrameContext) {
        assert!(self.is_frame_in_progress, "end_frame: aktif frame yok");
        
        let encoder = frame.command_encoder.take().unwrap();
        let command_buffers = vec![encoder.finish()];
        
        self.backend.submit_commands(command_buffers);
        self.is_frame_in_progress = false;
        
        // TAA history'yi güncelle
        if self.config.enable_taa {
            if let (Some(ref taa_history), Some(ref current_output)) =
                (&self.taa_history, &frame.current_output)
            {
                // History texture'ı güncelle
            }
        }
    }

    // -----------------------------------------------------------------------
    // Yardımcı Render Metotları
    // -----------------------------------------------------------------------

    /// Gökyüzü / atmosferi render eder.
    fn render_sky(&self, _render_pass: &mut wgpu::RenderPass, _frame: &FrameContext) {
        if let Some(ref sky_pipeline) = self.sky_pipeline {
            // _render_pass.set_pipeline(sky_pipeline);
            // Sky shader parametreleri:
            // - sun_direction
            // - turbidity, rayleigh, mie
            // Gerçek uygulamada burada sky mesh çizilir
        }
    }

    /// Opaque çizilebilir nesneleri PBR ile render eder.
    fn render_opaque_drawables(
        &self,
        _render_pass: &mut wgpu::RenderPass,
        _frame: &FrameContext,
        scene: &Scene,
    ) {
        if let Some(ref pbr_pipeline) = self.pbr_pipeline {
            // _render_pass.set_pipeline(pbr_pipeline);
            
            for drawable in &scene.drawables {
                if drawable.visibility && !drawable.material.is_transparent() {
                    // Vertex/index buffer binding
                    // Material bind group
                    // Transform uniform
                    // _render_pass.draw_indexed(...)
                }
            }
        }
    }

    /// Meshlet objelerini culling ile render eder.
    fn render_meshlet_objects(
        &self,
        _render_pass: &mut wgpu::RenderPass,
        frame: &FrameContext,
        scene: &Scene,
    ) {
        for obj in &scene.meshlet_objects {
            if !obj.enabled {
                continue;
            }
            
            // BVH culling
            if let Some(ref weyra) = self.weyra_pipeline {
                let _visible = weyra.test_meshlet_visibility(
                    &VirtualGeometryBlock {
                        id: obj.meshlet_handle,
                        position: Vec3::ZERO,
                        size: Vec3::ONE,
                        lod_level: obj.lod_level,
                        is_loaded: true,
                        bounding_box: BoundingBox::new(Vec3::ZERO, Vec3::ONE),
                    },
                    &frame.camera.view_projection_matrix,
                );
                
                // Visible meshlet'leri çiz
            }
            
            // Meshlet culling (frustum + backface)
            if let Some(ref culler) = scene.meshlet_objects.first() {
                // MeshletCulling kullan
            }
        }
    }

    /// Skinned mesh'leri render eder.
    fn render_skinned_meshes(
        &self,
        _render_pass: &mut wgpu::RenderPass,
        _frame: &FrameContext,
        scene: &Scene,
    ) {
        for skinned in &scene.skinned_meshes {
            if !skinned.visibility {
                continue;
            }
            
            // Skeleton'dan bone transforms'ı al
            let skeleton = scene.skeletons.get(&skinned.skeleton_name);
            let bone_transforms = skeleton.map(|s| s.get_bone_transforms()).unwrap_or_default();
            
            // Skin shader ile çiz
            // Bone transform buffer bind
            // Vertex shader'da skinning uygula
        }
    }

    /// Virtual geometry bloklarını render eder.
    fn render_virtual_geometry(
        &self,
        _render_pass: &mut wgpu::RenderPass,
        frame: &FrameContext,
        scene: &Scene,
    ) {
        if let Some(ref vgs) = self.virtual_geometry_system {
            // Kamera konumunu güncelle
            // vgs.update_camera_position(frame.camera.position);
            
            // Frustum culling
            // let visible = vgs.cull_by_frustum(&frame.camera_frustum);
            
            // LOD'a göre grupla
            // let lod_groups = vgs.group_by_lod();
            
            // Her LOD seviyesi için farklı mesh verisi çiz
        }
    }

    /// Debug şekillerini render eder.
    fn render_debug_shapes(
        &self,
        _render_pass: &mut wgpu::RenderPass,
        _frame: &FrameContext,
        scene: &Scene,
    ) {
        if let Some(ref debug_pipeline) = self.debug_pipeline {
            // _render_pass.set_pipeline(debug_pipeline);
            
            for shape in &scene.debug_shapes {
                match shape {
                    DebugShape::Aabb(debug) => {
                        // AABB wireframe çiz
                    }
                    DebugShape::Sphere(debug) => {
                        // Sphere wireframe çiz
                    }
                    DebugShape::Line(debug) => {
                        // Line çiz
                    }
                    DebugShape::Cone(debug) => {
                        // Cone wireframe çiz
                    }
                    DebugShape::SkeletonBones(debug) => {
                        // Skeleton bone çizgileri
                    }
                    DebugShape::WireframeMesh(debug) => {
                        // Mesh wireframe çiz
                    }
                }
            }
        }
    }

    /// Ton eşleme uygular (Reinhard / ACES).
    fn apply_tone_mapping(&self, _frame: &FrameContext, input_view: &TextureView) -> TextureView {
        // Gerçek uygulamada fullscreen quad ile tonemap shader çalıştırılır
        // Şimdilik aynı view'ı döndür
        input_view.clone()
    }

    /// HDR exposure değerini ayarlar.
    pub fn set_exposure(&mut self, exposure: f32) {
        self.exposure = exposure.clamp(0.01, 10.0);
    }

    /// Ton eşleme modunu ayarlar.
    pub fn set_tone_map_mode(&mut self, mode: ToneMapMode) {
        self.tone_map_mode = mode;
    }

    /// Debug render modunu ayarlar.
    pub fn set_debug_mode(&mut self, mode: DebugRenderMode) {
        self.debug_mode = mode;
        self.config.debug_mode = mode;
    }

    /// Gökyüzü renderer'ını ayarlar.
    pub fn set_sky_renderer(&mut self, sky_renderer: SkyRenderer) {
        self.sky_renderer = sky_renderer;
    }

    /// Weyra (virtual geometry) pipeline'ını yeniden boyutlandırır.
    pub fn resize_weyra(&mut self, width: u32, height: u32) {
        if let Some(ref mut weyra) = self.weyra_pipeline {
            weyra.resize(width, height);
        }
    }

    /// Shadow atlas'ı yeniden boyutlandırır.
    pub fn resize_shadow_atlas(&mut self, resolution: u32, cascade_count: usize) {
        let device = &self.backend.device;
        let shadow_size = resolution * cascade_count as u32;
        
        self.shadow_atlas = Some(RenderTarget::new(
            device,
            (shadow_size, resolution),
            TextureFormat::Depth32Float,
            Some("Shadow Atlas (Resized)"),
        ));
    }

    /// Renderer'ı yeni çözünürlüğe göre yeniden boyutlandırır.
    pub fn resize(&mut self, width: u32, height: u32) {
        let device = &self.backend.device;
        let hdr_format = TextureFormat::Rgba16Float;
        let depth_format = TextureFormat::Depth32Float;
        
        self.config.width = width;
        self.config.height = height;
        
        // HDR hedefleri yeniden oluştur
        self.hdr_target = Some(RenderTarget::new(
            device,
            (width, height),
            hdr_format,
            Some("HDR Target (Resized)"),
        ));
        
        if self.config.enable_msaa {
            self.hdr_msaa_target = Some(MultisampledRenderTarget::new(
                device,
                (width, height),
                hdr_format,
                self.config.sample_count,
            ));
        }
        
        self.depth_target = Some(RenderTarget::new(
            device,
            (width, height),
            depth_format,
            Some("Depth Target (Resized)"),
        ));
        
        // Post-process hedefleri
        if self.config.enable_bloom {
            self.bloom_target = Some(RenderTarget::new(
                device,
                (width / 2, height / 2),
                hdr_format,
                Some("Bloom Target (Resized)"),
            ));
            self.bright_target = Some(RenderTarget::new(
                device,
                (width / 4, height / 4),
                hdr_format,
                Some("Bright Target (Resized)"),
            ));
        }
        
        self.intermediate_target = Some(RenderTarget::new(
            device,
            (width, height),
            hdr_format,
            Some("Intermediate Target (Resized)"),
        ));
        
        if self.config.enable_taa {
            self.taa_history = Some(RenderTarget::new(
                device,
                (width, height),
                hdr_format,
                Some("TAA History (Resized)"),
            ));
        }
        
        // Weyra pipeline'ı yeniden boyutlandır
        self.resize_weyra(width, height);
    }

    /// Frame sıfırlama ve kaynak geri yükleme.
    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.is_frame_in_progress = false;
        self.exposure = self.config.exposure;
        self.tone_map_mode = self.config.tone_map_mode;
        self.debug_mode = self.config.debug_mode;
        
        if let Some(ref mut weyra) = self.weyra_pipeline {
            weyra.clear_visibility_buffer();
        }
    }

    /// Renderer istatistiklerini döndürür.
    pub fn get_stats(&self) -> RendererStats {
        RendererStats {
            frame_count: self.current_frame,
            is_hdr_enabled: self.config.enable_hdr,
            exposure: self.exposure,
            tone_map_mode: self.tone_map_mode,
            debug_mode: self.debug_mode,
            resolution: (self.config.width, self.config.height),
            sample_count: self.config.sample_count,
        }
    }
}

/// Renderer istatistikleri.
#[derive(Debug, Clone, Default)]
pub struct RendererStats {
    pub frame_count: u64,
    pub is_hdr_enabled: bool,
    pub exposure: f32,
    pub tone_map_mode: ToneMapMode,
    pub debug_mode: DebugRenderMode,
    pub resolution: (u32, u32),
    pub sample_count: u32,
}

// ---------------------------------------------------------------------------
// TAA Jitter Hesaplama
// ---------------------------------------------------------------------------

/// Halton sequence kullanarak TAA için frame jitter hesaplar.
pub fn calculate_taa_jitter(frame_index: u64) -> Vec2 {
    let x = halton(frame_index, 2) - 0.5;
    let y = halton(frame_index, 3) - 0.5;
    Vec2::new(x * 2.0, y * 2.0)
}

fn halton(index: u64, base: u64) -> f32 {
    let mut result = 0.0;
    let mut f = 1.0;
    let mut i = index;
    
    while i > 0 {
        f /= base as f32;
        result += f * (i % base) as f32;
        i /= base;
    }
    
    result
}

// ---------------------------------------------------------------------------
// WGSL Shader Kaynakları
// ---------------------------------------------------------------------------

/// PBR vertex/fragment shader kaynağı.
fn get_pbr_shader_wgsl() -> &'static str {
    r#"
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
    @location(4) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) color: vec4<f32>,
    @location(6) shadow_coords: vec4<f32>,
}

struct CameraUniforms {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
}

struct MaterialUniforms {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    normal_scale: f32,
    emissive: vec4<f32>,
    alpha_cutoff: f32,
}

struct ShadowUniforms {
    light_space_matrices: array<mat4x4<f32>, 4>,
    cascade_splits: vec4<f32>,
    shadow_params: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> material: MaterialUniforms;
@group(1) @binding(0) var<uniform> shadow: ShadowUniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_pos = vec4<f32>(input.position, 1.0);
    output.clip_position = camera.projection * camera.view * world_pos;
    output.world_position = input.position;
    output.world_normal = input.normal;
    output.uv = input.uv;
    output.tangent = input.tangent.xyz;
    output.bitangent = cross(input.normal, input.tangent.xyz) * input.tangent.w;
    output.color = input.color;
    output.shadow_coords = shadow.light_space_matrices[0] * world_pos;
    return output;
}

fn fresnel_schlick(f0: vec3<f32>, cos_theta: f32) -> vec3<f32> {
    let x = 1.0 - cos_theta;
    let x2 = x * x;
    let x5 = x2 * x2 * x;
    return f0 * (1.0 - x5) + vec3<f32>(1.0) * x5;
}

fn distribution_ggx(alpha: f32, cos_theta: f32) -> f32 {
    let alpha_sq = alpha * alpha;
    let denom = cos_theta * cos_theta * (alpha_sq - 1.0) + 1.0;
    return alpha_sq / (3.14159265 * denom * denom);
}

fn geometry_smith(alpha: f32, cos_theta: f32) -> f32 {
    let k = (alpha + 1.0) * (alpha + 1.0) / 8.0;
    return cos_theta / (cos_theta * (1.0 - k) + k);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(input.world_normal);
    let v = normalize(camera.camera_position.xyz - input.world_position);
    let albedo = material.base_color.rgb;
    let metallic = material.metallic;
    let roughness = max(material.roughness, 0.04);
    let f0 = mix(vec3<f32>(0.04), albedo, metallic);
    
    // Basit tek ışık PBR hesaplaması
    let l = normalize(-shadow.light_space_matrices[0][2].xyz);
    let h = normalize(v + l);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 0.0);
    let n_dot_h = max(dot(n, h), 0.0);
    let v_dot_h = max(dot(v, h), 0.0);
    
    let f = fresnel_schlick(f0, v_dot_h);
    let d = distribution_ggx(roughness * roughness, n_dot_h);
    let g = geometry_smith(roughness, n_dot_v) * geometry_smith(roughness, n_dot_l);
    
    let numerator = f * d * g;
    let denominator = 4.0 * n_dot_v * n_dot_l + 0.001;
    let specular = numerator / denominator;
    
    let k_s = f;
    let k_d = (vec3<f32>(1.0) - k_s) * (1.0 - metallic);
    let irradiance = albedo / 3.14159265;
    let diffuse = k_d * irradiance * n_dot_l;
    
    var color = diffuse + specular * n_dot_l;
    color += material.emissive.rgb * material.emissive.a;
    
    return vec4<f32>(color, material.base_color.a);
}
"#
}

/// Shadow map vertex shader kaynağı.
fn get_shadow_shader_wgsl() -> &'static str {
    r#"
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
}

struct ShadowUniforms {
    light_view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> shadow: ShadowUniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_pos = vec4<f32>(input.position, 1.0);
    output.clip_position = shadow.light_view_proj * world_pos;
    output.world_position = input.position;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
"#
}

/// Gökyüzü shader kaynağı.
fn get_sky_shader_wgsl() -> &'static str {
    r#"
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) direction: vec3<f32>,
}

struct SkyUniforms {
    sun_direction: vec4<f32>,
    sun_color: vec4<f32>,
    turbidity: f32,
    rayleigh: f32,
    mie: f32,
    mie_direction: f32,
}

@group(0) @binding(0) var<uniform> sky: SkyUniforms;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0)
    );
    var output: VertexOutput;
    output.clip_position = vec4<f32>(positions[idx], 0.999, 1.0);
    output.direction = output.clip_position.xyz;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(input.direction);
    let sun_dir = normalize(sky.sun_direction.xyz);
    let sun_fade = 1.0 - clamp(1.0 - sun_dir.y, 0.0, 1.0);
    let rayleigh_coeff = sky.rayleigh * (1.0 + sun_fade);
    
    var color = vec3<f32>(0.0);
    let zenith = vec3<f32>(0.0, 0.0, 1.0);
    let mu = dot(dir, zenith);
    
    // Basit gökyüzü gradienti
    let day_blue = vec3<f32>(0.3, 0.6, 1.0);
    let horizon = vec3<f32>(0.8, 0.85, 0.9);
    let ground = vec3<f32>(0.2, 0.15, 0.1);
    
    if mu > 0.0 {
        color = mix(horizon, day_blue, pow(mu, 0.5));
    } else {
        color = mix(horizon, ground, pow(-mu, 0.5));
    }
    
    // Güneş parlaklığı
    let sun_dot = dot(dir, sun_dir);
    if sun_dot > 0.0 {
        let sun_intensity = pow(sun_dot, 500.0) * sky.sun_color.rgb * sky.sun_color.a;
        color += sun_intensity;
    }
    
    return vec4<f32>(color, 1.0);
}
"#
}

/// Debug shader kaynağı (wireframe, normals vb.).
fn get_debug_shader_wgsl() -> &'static str {
    r#"
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
}

struct CameraUniforms {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    camera_position: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> debug_params: vec4<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_pos = vec4<f32>(input.position, 1.0);
    output.clip_position = camera.projection * camera.view * world_pos;
    output.world_position = input.position;
    output.world_normal = input.normal;
    output.uv = input.uv;
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let mode = i32(debug_params.x);
    
    if mode == 0 {
        // Wireframe / default
        return vec4<f32>(input.color.rgb, 1.0);
    } else if mode == 1 {
        // Normals
        return vec4<f32>(normalize(input.world_normal) * 0.5 + 0.5, 1.0);
    } else if mode == 2 {
        // Tangents
        return vec4<f32>(0.5, 0.5, 0.5, 1.0);
    } else if mode == 3 {
        // UVs
        return vec4<f32>(input.uv, 0.0, 1.0);
    } else if mode == 4 {
        // AABB
        return vec4<f32>(0.0, 1.0, 0.0, 1.0);
    } else if mode == 5 {
        // Light complexity
        return vec4<f32>(0.5, 0.5, 0.5, 1.0);
    } else if mode == 6 {
        // Metal/Roughness
        return vec4<f32>(0.5, 0.5, 0.5, 1.0);
    } else {
        return vec4<f32>(input.color.rgb, 1.0);
    }
}
"#
}

/// Fullscreen shader kaynağı (tonemap, fxaa vb. için).
fn get_fullscreen_shader_wgsl() -> &'static str {
    r#"
@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0)
    );
    return vec4(pos[idx], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    // Gerçek uygulamada texture sampling yapılır
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
"#
}

// ---------------------------------------------------------------------------
// Drop — temizlik
// ---------------------------------------------------------------------------

impl Drop for Renderer {
    fn drop(&mut self) {
        // GPU kaynakları otomatik olarak wgpu tarafından temizlenir
        // Ek temizlik gerekirse burada yapılır
    }
}
