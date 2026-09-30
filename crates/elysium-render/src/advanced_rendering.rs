use elysium_core::math::{Mat4, Vec2, Vec3};
use wgpu::Device;

// =============================================================================
// Mevcut içerik — GBuffer, DeferredShading, LightCulling, Light, SSR, TAA,
// VolumetricFog, TextureHandle, Format, AdvancedRenderingSystems
// =============================================================================

/// Deferred shading için gbuffer yapıları
#[derive(Debug, Clone)]
pub struct GBuffer {
    pub position: TextureHandle,
    pub normal: TextureHandle,
    pub albedo: TextureHandle,
    pub material_properties: TextureHandle,
}

/// Deferred shading sistemi
#[derive(Debug, Clone)]
pub struct DeferredShading {
    pub gbuffer: GBuffer,
    pub light_culling: LightCulling,
}

impl DeferredShading {
    pub fn new(gbuffer_resolution: (u32, u32)) -> Self {
        Self {
            gbuffer: GBuffer {
                position: TextureHandle::new(gbuffer_resolution.0, gbuffer_resolution.1, Format::Rgba32Float),
                normal: TextureHandle::new(gbuffer_resolution.0, gbuffer_resolution.1, Format::Rgba32Float),
                albedo: TextureHandle::new(gbuffer_resolution.0, gbuffer_resolution.1, Format::Rgba32Float),
                material_properties: TextureHandle::new(gbuffer_resolution.0, gbuffer_resolution.1, Format::Rgba32Float),
            },
            light_culling: LightCulling::new(),
        }
    }
}

/// Işık kümeleme sistemi
#[derive(Debug, Clone)]
pub struct LightCulling {
    pub lights: Vec<Light>,
    pub tile_size: u32,
}

impl LightCulling {
    pub fn new() -> Self {
        Self {
            lights: Vec::new(),
            tile_size: 16,
        }
    }

    pub fn add_light(&mut self, light: Light) {
        self.lights.push(light);
    }

    pub fn cull_lights(&self, _view_proj: Mat4) -> Vec<Light> {
        self.lights.clone()
    }
}

/// Işık türleri
#[derive(Debug, Clone)]
pub enum Light {
    Point(PointLight),
    Spot(SpotLight),
    Directional(DirectionalLight),
}

#[derive(Debug, Clone)]
pub struct PointLight {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub range: f32,
}

#[derive(Debug, Clone)]
pub struct SpotLight {
    pub position: Vec3,
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub range: f32,
    pub inner_cone_angle: f32,
    pub outer_cone_angle: f32,
}

#[derive(Debug, Clone)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

/// SSR (Screen Space Reflections) sistemi
#[derive(Debug, Clone)]
pub struct ScreenSpaceReflections {
    pub ray_step_size: f32,
    pub max_ray_distance: f32,
    pub thickness: f32,
    pub jitter: bool,
}

impl Default for ScreenSpaceReflections {
    fn default() -> Self {
        Self {
            ray_step_size: 0.5,
            max_ray_distance: 100.0,
            thickness: 0.1,
            jitter: true,
        }
    }
}

/// TAA (Temporal Anti-Aliasing) sistemi
#[derive(Debug, Clone)]
pub struct TemporalAntiAliasing {
    pub history_buffer: TextureHandle,
    pub blend_factor: f32,
    pub sharpen_factor: f32,
}

impl TemporalAntiAliasing {
    pub fn new(resolution: (u32, u32)) -> Self {
        Self {
            history_buffer: TextureHandle::new(resolution.0, resolution.1, Format::Rgba32Float),
            blend_factor: 0.9,
            sharpen_factor: 0.5,
        }
    }
}

/// Hacimsel sis sistemi
#[derive(Debug, Clone)]
pub struct VolumetricFog {
    pub density: f32,
    pub scattering_coefficient: f32,
    pub extinction_coefficient: f32,
    pub light_scattering: f32,
    pub max_density_distance: f32,
}

impl Default for VolumetricFog {
    fn default() -> Self {
        Self {
            density: 0.02,
            scattering_coefficient: 0.8,
            extinction_coefficient: 1.0,
            light_scattering: 0.1,
            max_density_distance: 1000.0,
        }
    }
}

/// Render sisteminde kullanılan geçici doku tanımı
#[derive(Debug, Clone)]
pub struct TextureHandle {
    pub width: u32,
    pub height: u32,
    pub format: Format,
}

#[derive(Debug, Clone)]
pub enum Format {
    Rgba32Float,
    Rgb32Float,
    Rg32Float,
    R32Float,
    Depth32Float,
}

impl TextureHandle {
    pub fn new(width: u32, height: u32, format: Format) -> Self {
        Self { width, height, format }
    }
}

/// Gelişmiş rendering sistemleri koleksiyonu
#[derive(Debug, Clone)]
pub struct AdvancedRenderingSystems {
    pub deferred_shading: Option<DeferredShading>,
    pub ssr: Option<ScreenSpaceReflections>,
    pub taa: Option<TemporalAntiAliasing>,
    pub volumetric_fog: Option<VolumetricFog>,
}

impl AdvancedRenderingSystems {
    pub fn new() -> Self {
        Self {
            deferred_shading: None,
            ssr: None,
            taa: None,
            volumetric_fog: None,
        }
    }

    pub fn enable_deferred_shading(&mut self, resolution: (u32, u32)) {
        self.deferred_shading = Some(DeferredShading::new(resolution));
    }

    pub fn enable_ssr(&mut self) {
        self.ssr = Some(ScreenSpaceReflections::default());
    }

    pub fn enable_taa(&mut self, resolution: (u32, u32)) {
        self.taa = Some(TemporalAntiAliasing::new(resolution));
    }

    pub fn enable_volumetric_fog(&mut self) {
        self.volumetric_fog = Some(VolumetricFog::default());
    }

    pub fn update(&mut self) {
        // Sistemleri güncelleme mantığı buraya
    }
}

// =============================================================================
// Ray Tracing (stub)
// =============================================================================

/// Ray tracing pipeline konfigürasyonu
#[derive(Debug, Clone)]
pub struct RayTracingPipeline {
    pub max_ray_depth: u32,
    pub max_ray_recursion: u32,
    pub fallback_to_rasterization: bool,
}

impl Default for RayTracingPipeline {
    fn default() -> Self {
        Self {
            max_ray_depth: 4,
            max_ray_recursion: 2,
            fallback_to_rasterization: true,
        }
    }
}

impl RayTracingPipeline {
    pub fn new(max_ray_depth: u32, max_ray_recursion: u32) -> Self {
        Self {
            max_ray_depth,
            max_ray_recursion,
            fallback_to_rasterization: true,
        }
    }

    pub fn dispatch(&self, _width: u32, _height: u32) {
        if self.fallback_to_rasterization {
            tracing::info!("Ray tracing not available — falling back to rasterization");
            return;
        }
        // Gerçek ray tracing dispatch burada gerçekleşir.
        tracing::info!("Dispatching ray tracing pass (depth={}, recursion={})",
                       self.max_ray_depth, self.max_ray_recursion);
    }
}

/// Acceleration structure türleri
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccelerationStructureType {
    Blas,
    Tlas,
    Bvh,
}

/// Acceleration structure tanımı
#[derive(Debug, Clone)]
pub struct AccelerationStructure {
    pub as_type: AccelerationStructureType,
    pub size_bytes: u64,
    pub handle: u64,
}

impl AccelerationStructure {
    pub fn new(as_type: AccelerationStructureType, size_bytes: u64) -> Self {
        Self {
            as_type,
            size_bytes,
            handle: 0,
        }
    }
}

/// Ray tracing shader kaynakları
pub mod rt_shaders {
    /// Ray generation shader
    pub const RAY_GEN: &str = r#"
@group(0) @binding(0) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> params: RayTracingParams;

struct RayTracingParams {
    width: u32,
    height: u32,
    frame: u32,
    max_depth: u32,
}

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

fn get_camera_ray(uv: vec2<f32>) -> Ray {
    var r = Ray(vec3(0.0), normalize(vec3(uv * 2.0 - 1.0, -1.0)));
    return r;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.width || y >= params.height) { return; }
    let uv = vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) / vec2<f32>(f32(params.width), f32(params.height));
    let ray = get_camera_ray(uv);
    // Placeholder — gerçek BLAS/TLAS traversal burada yapılır
    output[y * params.width + x] = vec4(0.1, 0.1, 0.2, 1.0);
}
"#;

    /// Closest-hit shader stub
    pub const CLOSEST_HIT: &str = r#"
@group(0) @binding(0) var<storage, read> materials: array<f32>;
@group(0) @binding(1) var<uniform> params: vec4<f32>;

@anyhit
fn main() {
    // closest-hit stub
}
"#;

    /// Miss shader stub
    pub const MISS: &str = r#"
@group(0) @binding(0) var sky_tex: texture_cube<f32>;
@group(0) @binding(1) var sky_sampler: sampler;

@anyhit
fn main() {
    // miss stub — sky fallback
}
"#;
}

// =============================================================================
// Path Tracing (stub)
// =============================================================================

/// Path tracing konfigürasyonu
#[derive(Debug, Clone)]
pub struct PathTracer {
    pub max_bounces: u32,
    pub samples_per_frame: u32,
    pub enable_denoising: bool,
    pub progressive: bool,
    pub sample_count: u64,
    pub resolution: (u32, u32),
    pub output_texture: TextureHandle,
    pub history_texture: TextureHandle,
}

impl PathTracer {
    pub fn new(resolution: (u32, u32), max_bounces: u32, samples_per_frame: u32) -> Self {
        Self {
            max_bounces,
            samples_per_frame,
            enable_denoising: true,
            progressive: true,
            sample_count: 0,
            resolution,
            output_texture: TextureHandle::new(resolution.0, resolution.1, Format::Rgba32Float),
            history_texture: TextureHandle::new(resolution.0, resolution.1, Format::Rgba32Float),
        }
    }

    pub fn accumulate(&mut self) {
        self.sample_count += self.samples_per_frame as u64;
    }

    pub fn reset(&mut self) {
        self.sample_count = 0;
    }

    pub fn dispatch(&self, _device: &wgpu::Device) {
        tracing::info!("Path tracing dispatch: {}x{} samples={} bounces={}",
                       self.resolution.0, self.resolution.1,
                       self.sample_count, self.max_bounces);
    }
}

/// Denoiser türleri
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DenoiserType {
    Spatial,
    Temporal,
    Bilateral,
}

/// Denoising parametreleri
#[derive(Debug, Clone)]
pub struct DenoiserParams {
    pub denoiser_type: DenoiserType,
    pub spatial_radius: u32,
    pub temporal_alpha: f32,
    pub sigma_color: f32,
    pub sigma_spatial: f32,
}

impl Default for DenoiserParams {
    fn default() -> Self {
        Self {
            denoiser_type: DenoiserType::Spatial,
            spatial_radius: 3,
            temporal_alpha: 0.1,
            sigma_color: 0.5,
            sigma_spatial: 2.0,
        }
    }
}

/// Path tracing shader kaynakları
pub mod pt_shaders {
    /// Path tracing compute shader
    pub const PATH_TRACE: &str = r#"
struct PathTraceParams {
    width: u32,
    height: u32,
    frame: u32,
    max_bounces: u32,
    seed: u32,
}

@group(0) @binding(0) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> params: PathTraceParams;
@group(0) @binding(2) var<storage, read> bvh_nodes: array<u32>;
@group(0) @binding(3) var<storage, read> triangles: array<vec4<f32>>;

fn rand(seed: u32) -> f32 {
    let n = seed * 747796405u + 2891336453u;
    let word = ((n >> ((n >> 28u) + 4u)) ^ n) * 277803737u;
    return f32((word >> 22u) ^ word) / 4294967296.0;
}

fn cosine_hemisphere(n: vec3<f32>, seed: u32) -> vec3<f32> {
    let r1 = rand(seed) * 6.2831853;
    let r2 = rand(seed ^ 0xDEADBEEFu);
    let r = sqrt(r2);
    let x = r * cos(r1);
    let y = r * sin(r1);
    let z = sqrt(1.0 - r2);
    let tangent = normalize(cross(abs(n.x) < 0.9 ? vec3(1.0, 0.0, 0.0) : vec3(0.0, 1.0, 0.0), n));
    let bitangent = cross(n, tangent);
    return normalize(tangent * x + bitangent * y + n * z);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.width || y >= params.height) { return; }
    let idx = y * params.width + x;
    let seed = params.seed + idx;
    var ray_dir = cosine_hemisphere(vec3(0.0, 1.0, 0.0), seed);
    // Placeholder — gerçek path tracing mantığı burada yürütülür
    output[idx] = vec4(ray_dir * 0.5 + 0.5, 1.0);
}
"#;

    /// Spatial denoiser
    pub const DENOISE_SPATIAL: &str = r#"
struct DenoiseParams {
    sigma_color: f32,
    sigma_spatial: f32,
    radius: u32,
    width: u32,
    height: u32,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var tex_sampler: sampler;
@group(0) @binding(3) var<uniform> params: DenoiseParams;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.width || y >= params.height) { return; }
    let center = textureSample(input_tex, tex_sampler, vec2<f32>(f32(x), f32(y)) / vec2<f32>(f32(params.width), f32(params.height))).rgb;
    var sum = vec3(0.0);
    var wsum = 0.0;
    for (var dy: i32 = -i32(params.radius); dy <= i32(params.radius); dy = dy + 1) {
        for (var dx: i32 = -i32(params.radius); dx <= i32(params.radius); dx = dx + 1) {
            let sx = i32(x) + dx;
            let sy = i32(y) + dy;
            if (sx < 0 || sy < 0 || sx >= i32(params.width) || sy >= i32(params.height)) { continue; }
            let s = textureSample(input_tex, tex_sampler, vec2<f32>(f32(sx), f32(sy)) / vec2<f32>(f32(params.width), f32(params.height))).rgb;
            let spatial = length(vec2<f32>(f32(dx), f32(dy)));
            let color_diff = length(s - center);
            let w = exp(-spatial * spatial / (params.sigma_spatial * params.sigma_spatial)) *
                    exp(-color_diff * color_diff / (params.sigma_color * params.sigma_color));
            sum = sum + s * w;
            wsum = wsum + w;
        }
    }
    textureStore(output_tex, vec2<i32>(x, y), vec4(sum / wsum, 1.0));
}
"#;

    /// Temporal denoiser / accumulation
    pub const DENOISE_TEMPORAL: &str = r#"
struct TemporalDenoiseParams {
    blend_alpha: f32,
}

@group(0) @binding(0) var current_tex: texture_2d<f32>;
@group(0) @binding(1) var history_tex: texture_2d<f32>;
@group(0) @binding(2) var output_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var tex_sampler: sampler;
@group(0) @binding(4) var<uniform> params: TemporalDenoiseParams;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(current_tex);
    let x = gid.x;
    let y = gid.y;
    if (x >= dims.x || y >= dims.y) { return; }
    let uv = vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) / vec2<f32>(f32(dims.x), f32(dims.y));
    let current = textureSample(current_tex, tex_sampler, uv).rgb;
    let history = textureSample(history_tex, tex_sampler, uv).rgb;
    let min_luma = min(luminance(current), luminance(history));
    let max_luma = max(luminance(current), luminance(history));
    let clipped = clamp(history, min(current, history), max(current, history));
    let result = mix(current, clipped, params.blend_alpha);
    textureStore(output_tex, vec2<i32>(x, y), vec4(result, 1.0));
}

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3(0.2126, 0.7152, 0.0722));
}
"#;
}

// =============================================================================
// Global Illumination (stub)
// =============================================================================

/// Light probe (irradiance volume) hücresi
#[derive(Debug, Clone)]
pub struct LightProbe {
    pub position: Vec3,
    pub irradiance: Vec3,
    pub irradiance_sh: [Vec3; 9],
    pub visibility: f32,
}

impl LightProbe {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            irradiance: Vec3::ZERO,
            irradiance_sh: [Vec3::ZERO; 9],
            visibility: 1.0,
        }
    }
}

/// Irradiance volume
#[derive(Debug, Clone)]
pub struct IrradianceVolume {
    pub origin: Vec3,
    pub cell_size: Vec3,
    pub grid_dimensions: (u32, u32, u32),
    pub probes: Vec<LightProbe>,
}

impl IrradianceVolume {
    pub fn new(origin: Vec3, cell_size: Vec3, grid_dimensions: (u32, u32, u32)) -> Self {
        let count = (grid_dimensions.0 * grid_dimensions.1 * grid_dimensions.2) as usize;
        let mut probes = Vec::with_capacity(count);
        for z in 0..grid_dimensions.2 {
            for y in 0..grid_dimensions.1 {
                for x in 0..grid_dimensions.0 {
                    let pos = origin
                        + Vec3::new(
                            x as f32 * cell_size.x,
                            y as f32 * cell_size.y,
                            z as f32 * cell_size.z,
                        );
                    probes.push(LightProbe::new(pos));
                }
            }
        }
        Self {
            origin,
            cell_size,
            grid_dimensions,
            probes,
        }
    }

    pub fn sample(&self, _position: Vec3) -> Vec3 {
        // Basit trilinear interpolasyon placeholder
        self.probes.get(0).map(|p| p.irradiance).unwrap_or(Vec3::ZERO)
    }
}

/// Reflection probe
#[derive(Debug, Clone)]
pub struct ReflectionProbe {
    pub position: Vec3,
    pub radius: f32,
    pub cubemap: TextureHandle,
    pub mip_levels: u32,
}

impl ReflectionProbe {
    pub fn new(position: Vec3, radius: f32, resolution: u32) -> Self {
        Self {
            position,
            radius,
            cubemap: TextureHandle::new(resolution, resolution, Format::Rgba32Float),
            mip_levels: 5,
        }
    }
}

/// SDF-based GI parametreleri
#[derive(Debug, Clone)]
pub struct SdfGiParams {
    pub voxel_size: f32,
    pub world_size: Vec3,
    pub num_cones: u32,
    pub max_distance: f32,
}

impl Default for SdfGiParams {
    fn default() -> Self {
        Self {
            voxel_size: 1.0,
            world_size: Vec3::new(128.0, 128.0, 128.0),
            num_cones: 5,
            max_distance: 50.0,
        }
    }
}

/// Global illumination sistemi
#[derive(Debug, Clone)]
pub struct GlobalIllumination {
    pub irradiance_volume: Option<IrradianceVolume>,
    pub reflection_probes: Vec<ReflectionProbe>,
    pub sdf_params: Option<SdfGiParams>,
    pub enabled: bool,
    pub irradiance_intensity: f32,
}

impl Default for GlobalIllumination {
    fn default() -> Self {
        Self {
            irradiance_volume: None,
            reflection_probes: Vec::new(),
            sdf_params: Some(SdfGiParams::default()),
            enabled: false,
            irradiance_intensity: 1.0,
        }
    }
}

impl GlobalIllumination {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_irradiance_volume(&mut self, volume: IrradianceVolume) {
        self.irradiance_volume = Some(volume);
    }

    pub fn add_reflection_probe(&mut self, probe: ReflectionProbe) {
        self.reflection_probes.push(probe);
    }

    pub fn sample_irradiance(&self, position: Vec3) -> Vec3 {
        self.irradiance_volume
            .as_ref()
            .map(|v| v.sample(position))
            .unwrap_or(Vec3::ZERO)
            * self.irradiance_intensity
    }
}

// =============================================================================
// Advanced Shading
// =============================================================================

/// Subsurface scattering profili
#[derive(Debug, Clone)]
pub struct SubsurfaceScatteringParams {
    pub radius: Vec3,
    pub falloff_color: Vec3,
    pub strength: f32,
}

impl Default for SubsurfaceScatteringParams {
    fn default() -> Self {
        Self {
            radius: Vec3::new(1.0, 0.2, 0.1),
            falloff_color: Vec3::new(1.0, 0.3, 0.2),
            strength: 1.0,
        }
    }
}

/// Hair shading modeli
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HairShadingModel {
    KajiyaKay,
    Marschner,
}

/// Hair malzeme parametreleri
#[derive(Debug, Clone)]
pub struct HairMaterialParams {
    pub base_color: Vec3,
    pub roughness: f32,
    pub shift: f32,
    pub ior: f32,
    pub model: HairShadingModel,
}

impl Default for HairMaterialParams {
    fn default() -> Self {
        Self {
            base_color: Vec3::new(0.8, 0.4, 0.2),
            roughness: 0.5,
            shift: 0.3,
            ior: 1.55,
            model: HairShadingModel::KajiyaKay,
        }
    }
}

/// Cloth shading modeli
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClothShadingModel {
    Ward,
    AshikhminShirley,
}

/// Cloth malzeme parametreleri
#[derive(Debug, Clone)]
pub struct ClothMaterialParams {
    pub base_color: Vec3,
    pub roughness_u: f32,
    pub roughness_v: f32,
    pub model: ClothShadingModel,
}

impl Default for ClothMaterialParams {
    fn default() -> Self {
        Self {
            base_color: Vec3::new(0.9, 0.1, 0.1),
            roughness_u: 0.8,
            roughness_v: 0.8,
            model: ClothShadingModel::AshikhminShirley,
        }
    }
}

/// Clearcoat katmanı parametreleri
#[derive(Debug, Clone)]
pub struct ClearcoatParams {
    pub clearcoat: f32,
    pub clearcoat_roughness: f32,
    pub clearcoat_normal: Vec3,
}

impl Default for ClearcoatParams {
    fn default() -> Self {
        Self {
            clearcoat: 0.0,
            clearcoat_roughness: 0.1,
            clearcoat_normal: Vec3::Y,
        }
    }
}

/// Sheen katmanı parametreleri
#[derive(Debug, Clone)]
pub struct SheenParams {
    pub sheen: f32,
    pub sheen_roughness: f32,
    pub sheen_color: Vec3,
}

impl Default for SheenParams {
    fn default() -> Self {
        Self {
            sheen: 0.0,
            sheen_roughness: 0.5,
            sheen_color: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

/// Anizotropik parametreler
#[derive(Debug, Clone)]
pub struct AnisotropyParams {
    pub anisotropy: f32,
    pub rotation: f32,
    pub tangent: Vec3,
}

impl Default for AnisotropyParams {
    fn default() -> Self {
        Self {
            anisotropy: 0.0,
            rotation: 0.0,
            tangent: Vec3::X,
        }
    }
}

// =============================================================================
// Volumetric Rendering
// =============================================================================

/// Bulut parametreleri
#[derive(Debug, Clone)]
pub struct VolumetricCloudParams {
    pub coverage: f32,
    pub density: f32,
    pub absorption: f32,
    pub wind_offset: Vec3,
    pub detail_scale: f32,
    pub detail_strength: f32,
}

impl Default for VolumetricCloudParams {
    fn default() -> Self {
        Self {
            coverage: 0.5,
            density: 0.3,
            absorption: 0.4,
            wind_offset: Vec3::ZERO,
            detail_scale: 4.0,
            detail_strength: 0.6,
        }
    }
}

/// God ray parametreleri
#[derive(Debug, Clone)]
pub struct GodRayParams {
    pub weight: f32,
    pub decay: f32,
    pub density: f32,
    pub exposure: f32,
    pub samples: u32,
    pub light_position: Vec3,
}

impl Default for GodRayParams {
    fn default() -> Self {
        Self {
            weight: 0.4,
            decay: 0.96,
            density: 0.5,
            exposure: 0.5,
            samples: 60,
            light_position: Vec3::new(0.0, 100.0, 0.0),
        }
    }
}

/// Volumetric fog (detaylı) — raymarched
#[derive(Debug, Clone)]
pub struct VolumetricFogRaymarched {
    pub density: f32,
    pub scattering: f32,
    pub extinction: f32,
    pub anisotropy_g: f32,
    pub max_steps: u32,
    pub max_distance: f32,
    pub light_position: Vec3,
    pub light_color: Vec3,
    pub light_intensity: f32,
}

impl Default for VolumetricFogRaymarched {
    fn default() -> Self {
        Self {
            density: 0.02,
            scattering: 0.8,
            extinction: 1.0,
            anisotropy_g: 0.5,
            max_steps: 32,
            max_distance: 200.0,
            light_position: Vec3::new(0.0, 50.0, 0.0),
            light_color: Vec3::new(1.0, 0.95, 0.8),
            light_intensity: 2.0,
        }
    }
}

/// Light scattering phase fonksiyonları
pub mod scattering {
    /// Rayleigh phase fonksiyonu
    pub fn rayleigh(cos_theta: f32) -> f32 {
        let k = 3.0 / (16.0 * std::f32::consts::PI);
        k * (1.0 + cos_theta * cos_theta)
    }

    /// Henyey-Greenstein phase fonksiyonu
    pub fn henyey_greenstein(cos_theta: f32, g: f32) -> f32 {
        let g2 = g * g;
        let denom = (1.0 + g2 - 2.0 * g * cos_theta).sqrt();
        1.0 / (4.0 * std::f32::consts::PI * denom * denom * denom * denom)
    }

    /// Dual Henyey-Greenstein phase fonksiyonu
    pub fn dual_henyey_greenstein(cos_theta: f32, g1: f32, g2: f32, w: f32) -> f32 {
        w * henyey_greenstein(cos_theta, g1) + (1.0 - w) * henyey_greenstein(cos_theta, g2)
    }
}

/// Volumetric rendering shader kaynakları
pub mod vol_shaders {
    /// Volumetric fog raymarching compute shader
    pub const VOLUMETRIC_FOG: &str = r#"
struct VolumetricFogParams {
    density: f32,
    scattering: f32,
    extinction: f32,
    anisotropy_g: f32,
    max_steps: u32,
    max_distance: f32,
    light_position: vec3<f32>,
    light_color: vec3<f32>,
    light_intensity: f32,
}

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var tex_sampler: sampler;
@group(0) @binding(3) var<uniform> params: VolumetricFogParams;
@group(0) @binding(4) var<uniform> view: mat4x4<f32>;
@group(0) @binding(5) var<uniform> proj: mat4x4<f32>;

fn henyey_greenstein(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    return 1.0 / (4.0 * 3.14159265 * denom * sqrt(denom) * denom);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(depth_tex);
    let x = gid.x;
    let y = gid.y;
    if (x >= dims.x || y >= dims.y) { return; }
    let uv = vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) / vec2<f32>(f32(dims.x), f32(dims.y));
    let depth = textureSample(depth_tex, tex_sampler, uv).x;
    let ray_start = vec3(0.0, 0.0, 0.0);
    let ray_dir = normalize(vec3(uv * 2.0 - 1.0, -1.0));
    let ray_end = ray_start + ray_dir * depth * params.max_distance;
    let step_size = params.max_distance / f32(params.max_steps);
    var transmittance = 1.0;
    var scattered_light = vec3(0.0);
    for (var i: u32 = 0u; i < params.max_steps; i = i + 1u) {
        let t = f32(i) / f32(params.max_steps);
        let pos = mix(ray_start, ray_end, t);
        let density = params.density * (1.0 - smoothstep(0.0, 20.0, length(pos - params.light_position)));
        if (density <= 0.0) { continue; }
        let light_dir = normalize(params.light_position - pos);
        let phase = henyey_greenstein(dot(ray_dir, light_dir), params.anisotropy_g);
        let inscatter = params.light_color * params.light_intensity * phase * params.scattering * density * step_size;
        scattered_light = scattered_light + transmittance * inscatter;
        transmittance = transmittance * exp(-params.extinction * density * step_size);
        if (transmittance < 0.01) { break; }
    }
    textureStore(output_tex, vec2<i32>(x, y), vec4(scattered_light, 1.0 - transmittance));
}
"#;

    /// Volumetric clouds compute shader (raymarched)
    pub const VOLUMETRIC_CLOUDS: &str = r#"
struct CloudParams {
    coverage: f32,
    density: f32,
    absorption: f32,
    wind_offset: vec3<f32>,
    detail_scale: f32,
    detail_strength: f32,
}

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var tex_sampler: sampler;
@group(0) @binding(3) var<uniform> params: CloudParams;
@group(0) @binding(4) var<uniform> view: mat4x4<f32>;
@group(0) @binding(5) var<uniform> proj: mat4x4<f32>;

fn hash(p: vec3<f32>) -> f32 {
    let n = fract(sin(dot(p, vec3(12.9898, 78.233, 45.164))) * 43758.5453);
    return n;
}

fn noise3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(mix(hash(i), hash(i + vec3(1.0, 0.0, 0.0)), u.x),
            mix(hash(i + vec3(0.0, 1.0, 0.0)), hash(i + vec3(1.0, 1.0, 0.0)), u.x), u.y),
        mix(mix(hash(i + vec3(0.0, 0.0, 1.0)), hash(i + vec3(1.0, 0.0, 1.0)), u.x),
            mix(hash(i + vec3(0.0, 1.0, 1.0)), hash(i + vec3(1.0, 1.0, 1.0)), u.x), u.y), u.z
    );
}

fn fbm(p: vec3<f32>, scale: f32) -> f32 {
    var value = 0.0;
    var amp = 0.5;
    var pos = p * scale;
    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        value = value + amp * noise3d(pos);
        pos = pos * 2.0;
        amp = amp * 0.5;
    }
    return value;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(depth_tex);
    let x = gid.x;
    let y = gid.y;
    if (x >= dims.x || y >= dims.y) { return; }
    let uv = vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) / vec2<f32>(f32(dims.x), f32(dims.y));
    let depth = textureSample(depth_tex, tex_sampler, uv).x;
    let ray_dir = normalize(vec3(uv * 2.0 - 1.0, -1.0));
    var pos = vec3(0.0) + ray_dir * 800.0;
    let cloud_start = 1000.0;
    let cloud_end = 1500.0;
    var transmittance = 1.0;
    var scattered = vec3(0.0);
    let steps = 16u;
    let step_size = (cloud_end - cloud_start) / f32(steps);
    for (var i: u32 = 0u; i < steps; i = i + 1u) {
        let t = f32(i) / f32(steps);
        let sample_pos = pos + ray_dir * (cloud_start + t * (cloud_end - cloud_start));
        let n = fbm(sample_pos + params.wind_offset, params.detail_scale);
        let detail = fbm(sample_pos * 4.0 + params.wind_offset, 8.0);
        let density = smoothstep(params.coverage, params.coverage + 0.2, n + detail * params.detail_strength) * params.density;
        if (density > 0.0) {
            let beer = exp(-params.absorption * density * step_size);
            transmittance = transmittance * beer;
            scattered = scattered + transmittance * vec3(1.0) * density * step_size;
        }
    }
    textureStore(output_tex, vec2<i32>(x, y), vec4(scattered, 1.0 - transmittance));
}
"#;

    /// God ray (light shaft) compute shader
    pub const GOD_RAYS: &str = r#"
struct GodRayParams {
    weight: f32,
    decay: f32,
    density: f32,
    exposure: f32,
    samples: u32,
    light_screen_pos: vec2<f32>,
}

@group(0) @binding(0) var occlusion_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var tex_sampler: sampler;
@group(0) @binding(3) var<uniform> params: GodRayParams;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(occlusion_tex);
    let x = gid.x;
    let y = gid.y;
    if (x >= dims.x || y >= dims.y) { return; }
    let uv = vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) / vec2<f32>(f32(dims.x), f32(dims.y));
    let delta_uv = (uv - params.light_screen_pos) / f32(params.samples);
    var illumination_decay = 1.0;
    var color = vec3(0.0);
    var cur_uv = uv;
    for (var i: u32 = 0u; i < params.samples; i = i + 1u) {
        let sample_tex = textureSample(occlusion_tex, tex_sampler, cur_uv);
        color = color + sample_tex.rgb * illumination_decay * params.weight;
        illumination_decay = illumination_decay * params.decay;
        cur_uv = cur_uv - delta_uv;
    }
    textureStore(output_tex, vec2<i32>(x, y), vec4(color * params.exposure, 1.0));
}
"#;
}

// =============================================================================
// Post-Process Effects
// =============================================================================

/// SMAA edge detection modu
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SmaaEdgeDetectionMode {
    Luma,
    Color,
    Depth,
}

/// SMAA parametreleri
#[derive(Debug, Clone)]
pub struct SmaaParams {
    pub edge_detection_mode: SmaaEdgeDetectionMode,
    pub threshold: f32,
    pub max_search_steps: u32,
    pub max_search_steps_diagonal: u32,
    pub corner_rounding: u32,
}

impl Default for SmaaParams {
    fn default() -> Self {
        Self {
            edge_detection_mode: SmaaEdgeDetectionMode::Luma,
            threshold: 0.1,
            max_search_steps: 16,
            max_search_steps_diagonal: 8,
            corner_rounding: 25,
        }
    }
}

/// Depth of field parametreleri
#[derive(Debug, Clone)]
pub struct DepthOfFieldParams {
    pub focus_distance: f32,
    pub aperture: f32,
    pub focal_length: f32,
    pub max_blur: f32,
    pub bokeh_shape: u32,
}

impl Default for DepthOfFieldParams {
    fn default() -> Self {
        Self {
            focus_distance: 10.0,
            aperture: 2.8,
            focal_length: 50.0,
            max_blur: 1.0,
            bokeh_shape: 0,
        }
    }
}

/// Lens flare parametreleri
#[derive(Debug, Clone)]
pub struct LensFlareParams {
    pub intensity: f32,
    pub ghost_count: u32,
    pub ghost_spacing: f32,
    pub threshold: f32,
    pub chromatic_aberration: f32,
}

impl Default for LensFlareParams {
    fn default() -> Self {
        Self {
            intensity: 1.0,
            ghost_count: 4,
            ghost_spacing: 0.3,
            threshold: 0.8,
            chromatic_aberration: 0.5,
        }
    }
}

/// Chromatic aberration parametreleri
#[derive(Debug, Clone)]
pub struct ChromaticAberrationParams {
    pub offset: Vec2,
    pub strength: f32,
}

impl Default for ChromaticAberrationParams {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            strength: 0.005,
        }
    }
}

/// Vignette parametreleri
#[derive(Debug, Clone)]
pub struct VignetteParams {
    pub intensity: f32,
    pub slope: f32,
    pub roundness: f32,
}

impl Default for VignetteParams {
    fn default() -> Self {
        Self {
            intensity: 0.4,
            slope: 12.0,
            roundness: 1.0,
        }
    }
}

/// Film grain parametreleri
#[derive(Debug, Clone)]
pub struct FilmGrainParams {
    pub intensity: f32,
    pub speed: f32,
    pub time: f32,
}

impl Default for FilmGrainParams {
    fn default() -> Self {
        Self {
            intensity: 0.05,
            speed: 1.0,
            time: 0.0,
        }
    }
}

/// Color grading LUT parametreleri
#[derive(Debug, Clone)]
pub struct ColorGradingLutParams {
    pub lut_size: u32,
    pub intensity: f32,
    pub saturation: f32,
    pub contrast: f32,
    pub gamma: Vec3,
}

impl Default for ColorGradingLutParams {
    fn default() -> Self {
        Self {
            lut_size: 32,
            intensity: 1.0,
            saturation: 1.0,
            contrast: 1.0,
            gamma: Vec3::new(1.0 / 2.2, 1.0 / 2.2, 1.0 / 2.2),
        }
    }
}

/// Post-process shader kaynakları
pub mod pp_shaders {
    /// SMAA luma edge detection
    pub const SMAA_EDGE_DETECTION: &str = r#"
struct SmaaParams {
    threshold: f32,
}

@group(0) @binding(0) var color_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var tex_sampler: sampler;
@group(0) @binding(3) var<uniform> params: SmaaParams;

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3(0.2126, 0.7152, 0.0722));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(color_tex);
    let x = gid.x;
    let y = gid.y;
    if (x >= dims.x || y >= dims.y) { return; }
    let uv = vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) / vec2<f32>(f32(dims.x), f32(dims.y));
    let c = textureSample(color_tex, tex_sampler, uv).rgb;
    let c_left  = textureSampleOffset(color_tex, tex_sampler, uv, vec2<i32>(-1, 0)).rgb;
    let c_top   = textureSampleOffset(color_tex, tex_sampler, uv, vec2<i32>( 0, 1)).rgb;
    let delta_c = abs(luma(c) - luma(c_left)) + abs(luma(c) - luma(c_top));
    let edge = select(vec2(0.0, 1.0), vec2(1.0, 0.0), delta_c >= params.threshold);
    textureStore(output_tex, vec2<i32>(x, y), vec4(edge, 0.0, 1.0));
}
"#;

    /// Depth of field (bokeh) shader
    pub const DEPTH_OF_FIELD: &str = r#"
struct DofParams {
    focus_distance: f32,
    aperture: f32,
    max_blur: f32,
}

@group(0) @binding(0) var color_tex: texture_2d<f32>;
@group(0) @binding(1) var depth_tex: texture_2d<f32>;
@group(0) @binding(2) var output_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var tex_sampler: sampler;
@group(0) @binding(4) var<uniform> params: DofParams;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(color_tex);
    let x = gid.x;
    let y = gid.y;
    if (x >= dims.x || y >= dims.y) { return; }
    let uv = vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) / vec2<f32>(f32(dims.x), f32(dims.y));
    let depth = textureSample(depth_tex, tex_sampler, uv).x;
    let coc = abs(depth - params.focus_distance) * params.aperture;
    var color = vec3(0.0);
    let samples = 8u;
    for (var i: u32 = 0u; i < samples; i = i + 1u) {
        let angle = f32(i) / f32(samples) * 6.2831853;
        let offset = vec2<f32>(cos(angle), sin(angle)) * coc * params.max_blur;
        color = color + textureSample(color_tex, tex_sampler, uv + offset).rgb;
    }
    textureStore(output_tex, vec2<i32>(x, y), vec4(color / f32(samples), 1.0));
}
"#;

    /// Lens flare / ghosting compute shader
    pub const LENS_FLARE: &str = r#"
struct LensFlareParams {
    intensity: f32,
    ghost_count: u32,
    ghost_spacing: f32,
    threshold: f32,
    chromatic_aberration: f32,
}

@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var tex_sampler: sampler;
@group(0) @binding(3) var<uniform> params: LensFlareParams;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(source_tex);
    let x = gid.x;
    let y = gid.y;
    if (x >= dims.x || y >= dims.y) { return; }
    let uv = vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) / vec2<f32>(f32(dims.x), f32(dims.y));
    let center = vec2(0.5, 0.5);
    var ghost_uv = uv - center;
    var color = vec3(0.0);
    for (var i: u32 = 0u; i < params.ghost_count; i = i + 1u) {
        let t = f32(i) * params.ghost_spacing;
        let sample_uv = center + ghost_uv * (1.0 + t);
        let aberration = params.chromatic_aberration * t;
        let r = textureSample(source_tex, tex_sampler, sample_uv + vec2(aberration, 0.0)).r;
        let g = textureSample(source_tex, tex_sampler, sample_uv).g;
        let b = textureSample(source_tex, tex_sampler, sample_uv - vec2(aberration, 0.0)).b;
        color = color + vec3(r, g, b) * params.intensity / f32(params.ghost_count);
    }
    textureStore(output_tex, vec2<i32>(x, y), vec4(color, 1.0));
}
"#;

    /// Chromatic aberration shader
    pub const CHROMATIC_ABERRATION: &str = r#"
struct ChromaticParams {
    strength: f32,
}

@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var tex_sampler: sampler;
@group(0) @binding(3) var<uniform> params: ChromaticParams;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(source_tex);
    let x = gid.x;
    let y = gid.y;
    if (x >= dims.x || y >= dims.y) { return; }
    let uv = vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) / vec2<f32>(f32(dims.x), f32(dims.y));
    let dir = uv - vec2(0.5, 0.5);
    let dist = length(dir);
    let offset = dir * dist * params.strength;
    let r = textureSample(source_tex, tex_sampler, uv + offset).r;
    let g = textureSample(source_tex, tex_sampler, uv).g;
    let b = textureSample(source_tex, tex_sampler, uv - offset).b;
    textureStore(output_tex, vec2<i32>(x, y), vec4(r, g, b, 1.0));
}
"#;

    /// Vignette shader
    pub const VIGNETTE: &str = r#"
struct VignetteParams {
    intensity: f32,
    slope: f32,
    roundness: f32,
}

@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var tex_sampler: sampler;
@group(0) @binding(3) var<uniform> params: VignetteParams;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(source_tex);
    let x = gid.x;
    let y = gid.y;
    if (x >= dims.x || y >= dims.y) { return; }
    let uv = vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) / vec2<f32>(f32(dims.x), f32(dims.y));
    let d = distance(uv, vec2(0.5));
    let vignette = 1.0 - params.intensity * pow(d * params.slope, params.roundness);
    let color = textureSample(source_tex, tex_sampler, uv).rgb * vignette;
    textureStore(output_tex, vec2<i32>(x, y), vec4(color, 1.0));
}
"#;

    /// Film grain shader
    pub const FILM_GRAIN: &str = r#"
struct FilmGrainParams {
    intensity: f32,
    time: f32,
}

@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var tex_sampler: sampler;
@group(0) @binding(3) var<uniform> params: FilmGrainParams;

fn hash12(p: vec2<f32>) -> f32 {
    let n = fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
    return n;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(source_tex);
    let x = gid.x;
    let y = gid.y;
    if (x >= dims.x || y >= dims.y) { return; }
    let uv = vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) / vec2<f32>(f32(dims.x), f32(dims.y));
    let grain = hash12(uv + vec2(params.time)) * 2.0 - 1.0;
    let color = textureSample(source_tex, tex_sampler, uv).rgb + grain * params.intensity;
    textureStore(output_tex, vec2<i32>(x, y), vec4(color, 1.0));
}
"#;

    /// Color grading LUT shader
    pub const COLOR_GRADING_LUT: &str = r#"
struct ColorGradingParams {
    lut_size: u32,
    intensity: f32,
    saturation: f32,
    contrast: f32,
    gamma: vec3<f32>,
}

@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var lut_tex: texture_2d<f32>;
@group(0) @binding(2) var output_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var tex_sampler: sampler;
@group(0) @binding(4) var<uniform> params: ColorGradingParams;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(source_tex);
    let x = gid.x;
    let y = gid.y;
    if (x >= dims.x || y >= dims.y) { return; }
    let uv = vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) / vec2<f32>(f32(dims.x), f32(dims.y));
    var color = textureSample(source_tex, tex_sampler, uv).rgb;
    color = (color - 0.5) * params.contrast + 0.5;
    let gray = dot(color, vec3(0.2126, 0.7152, 0.0722));
    color = mix(vec3(gray), color, params.saturation);
    color = pow(clamp(color, vec3(0.0), vec3(1.0)), params.gamma);
    let lut_size = f32(params.lut_size);
    let blue_idx = floor(color.b * (lut_size - 1.0));
    let lut_uv = (color.rg * (lut_size - 1.0 - blue_idx) + blue_idx) / (lut_size * lut_size);
    let graded = textureSample(lut_tex, tex_sampler, lut_uv).rgb;
    textureStore(output_tex, vec2<i32>(x, y), vec4(mix(color, graded, params.intensity), 1.0));
}
"#;
}

// =============================================================================
// Advanced Shading WGSL Shaders
// =============================================================================

pub mod shading_shaders {
    /// Subsurface scattering compute shader (stub)
    pub const SUBSURFACE_SCATTERING: &str = r#"
struct SssParams {
    radius: vec3<f32>,
    strength: f32,
}

@group(0) @binding(0) var thickness_tex: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;
@group(0) @binding(2) var<uniform> params: SssParams;

fn profile(r: vec3<f32>, d: f32) -> vec3<f32> {
    return exp(-d * r) / (r + 1.0);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(thickness_tex);
    let x = gid.x;
    let y = gid.y;
    if (x >= dims.x || y >= dims.y) { return; }
    let uv = vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) / vec2<f32>(f32(dims.x), f32(dims.y));
    let thickness = textureSample(thickness_tex, tex_sampler, uv).r;
    let sss = profile(params.radius, thickness) * params.strength;
    // Placeholder
}
"#;

    /// Kajiya-Kay hair shading compute shader (stub)
    pub const HAIR_KAJIYA_KAY: &str = r#"
struct HairParams {
    base_color: vec3<f32>,
    roughness: f32,
    shift: f32,
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // Kajiya-Kay stub
}
"#;

    /// Marschner hair shading compute shader (stub)
    pub const HAIR_MARSCHNER: &str = r#"
struct MarschnerParams {
    base_color: vec3<f32>,
    roughness: f32,
    ior: f32,
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // Marschner stub
}
"#;

    /// Ward cloth shading compute shader (stub)
    pub const CLOTH_WARD: &str = r#"
struct WardParams {
    base_color: vec3<f32>,
    roughness_u: f32,
    roughness_v: f32,
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // Ward stub
}
"#;

    /// Ashikhmin-Shirley cloth shading compute shader (stub)
    pub const CLOTH_ASHIKHMIN_SHIRLEY: &str = r#"
struct AshikhminShirleyParams {
    base_color: vec3<f32>,
    roughness_u: f32,
    roughness_v: f32,
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // Ashikhmin-Shirley stub
}
"#;

    /// Clearcoat shading compute shader (stub)
    pub const CLEARCOAT: &str = r#"
struct ClearcoatParams {
    clearcoat: f32,
    clearcoat_roughness: f32,
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // Clearcoat stub
}
"#;

    /// Sheen shading compute shader (stub)
    pub const SHEEN: &str = r#"
struct SheenParams {
    sheen: f32,
    sheen_roughness: f32,
    sheen_color: vec3<f32>,
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // Sheen stub
}
"#;

    /// Anisotropic shading compute shader (stub)
    pub const ANISOTROPY: &str = r#"
struct AnisotropyParams {
    anisotropy: f32,
    rotation: f32,
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // Anisotropy stub
}
"#;
}

// =============================================================================
// Extra Post-Process Effect Structs
// =============================================================================

/// FXAA parametreleri (shader tanımı post_process.rs'de mevcut)
#[derive(Debug, Clone)]
pub struct FxaaParams {
    pub edge_threshold: f32,
    pub edge_threshold_min: f32,
}

impl Default for FxaaParams {
    fn default() -> Self {
        Self {
            edge_threshold: 0.125,
            edge_threshold_min: 0.0312,
        }
    }
}

/// Bloom parametreleri (shader tanımı post_process.rs'de mevcut)
#[derive(Debug, Clone)]
pub struct BloomParams {
    pub intensity: f32,
    pub threshold: f32,
    pub soft_knee: f32,
    pub radius: f32,
}

impl Default for BloomParams {
    fn default() -> Self {
        Self {
            intensity: 1.0,
            threshold: 1.0,
            soft_knee: 0.5,
            radius: 5.0,
        }
    }
}

/// Motion blur parametreleri (shader tanımı post_process.rs'de mevcut)
#[derive(Debug, Clone)]
pub struct MotionBlurParams {
    pub shutter_speed: f32,
    pub max_velocity: f32,
    pub sample_count: u32,
}

impl Default for MotionBlurParams {
    fn default() -> Self {
        Self {
            shutter_speed: 0.016,
            max_velocity: 1.0,
            sample_count: 8,
        }
    }
}

// =============================================================================
// Re-exports
// =============================================================================

pub use rt_shaders::*;
pub use pt_shaders::*;
pub use vol_shaders::*;
pub use pp_shaders::*;
pub use shading_shaders::*;
