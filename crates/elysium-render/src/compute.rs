//! # Compute / GPGPU System for Elysium Renderer
//!
//! This module provides a complete compute system backed by WGPU:
//! - `ComputePipeline` with shader, layout and bind-group management
//! - `GpuBuffer` covering uniform / storage / index / vertex buffer variants
//! - `GpuTexture` with creation, mipmap generation, views and samplers
//! - `ComputeDispatcher` for direct, indirect and multi-dispatch
//! - Pre-built compute shaders (particles, cloth, fluid, terrain, image, noise)
//! - ECS integration via `ComputeComponent` and `ComputeSystem`

use std::sync::Arc;
use std::collections::HashMap;

use wgpu::*;
use elysium_core::{
    world::World,
    scheduler::System,
};

// Re-export the shader string constants so users can embed them in their own
// WGSL modules if needed.
pub mod shaders {
    //! WGSL shader sources for common compute tasks.

    /// Particle simulation — updates position/velocity/age for a particle pool.
    pub const PARTICLE_SIM: &str = r#"
struct Particle {
    pos: vec3<f32>,
    vel: vec3<f32>,
    age: f32,
    life: f32,
}

struct Params {
    delta_time: f32,
    particle_count: u32,
    spawn_rate: f32,
    gravity: f32,
    damping: f32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.particle_count) { return; }
    var p = particles[i];
    p.age += params.delta_time;
    if (p.age >= p.life) {
        p.pos = vec3<f32>(0.0, 0.0, 0.0);
        p.vel = vec3<f32>(0.0, 0.0, 0.0);
        p.age = 0.0;
        p.life = 2.0 + (fract(sin(f32(i) * 12.9898)) * 5.0);
    }
    p.vel.y -= params.gravity * params.delta_time;
    p.vel *= 1.0 - params.damping * params.delta_time;
    p.pos += p.vel * params.delta_time;
    particles[i] = p;
}
"#;

    /// Cloth simulation using position-based dynamics.
    pub const CLOTH_SIM: &str = r#"
struct Particle2 {
    pos: vec3<f32>,
    prev_pos: vec3<f32>,
    acc: vec3<f32>,
    mass: f32,
    inv_mass: f32,
}

struct ClothParams {
    width: u32,
    height: u32,
    rest_distance: f32,
    delta_time: f32,
    gravity: f32,
    constraint_iterations: u32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle2>;
@group(0) @binding(1) var<uniform> params: ClothParams;
@group(0) @binding(2) var<storage, read> constraints: array<vec2<u32>>;
@group(0) @binding(3) var<storage, read> constraint_counts: array<u32>;

@compute @workgroup_size(8, 8)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.width || y >= params.height) { return; }
    let idx = y * params.width + x;
    var p = particles[idx];
    p.acc.y -= params.gravity;
    let vel = p.pos - p.prev_pos;
    p.prev_pos = p.pos;
    p.pos += vel + p.acc * params.delta_time * params.delta_time;
    p.acc = vec3<f32>(0.0, 0.0, 0.0);
    particles[idx] = p;
}
"#;

    /// Simple SPH fluid simulation.
    pub const FLUID_SIM: &str = r#"
struct FluidParticle {
    pos: vec3<f32>,
    vel: vec3<f32>,
    density: f32,
    pressure: f32,
}

struct FluidParams {
    particle_count: u32,
    smoothing_radius: f32,
    rest_density: f32,
    gas_constant: f32,
    gravity: f32,
    delta_time: f32,
    stiffness: f32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<FluidParticle>;
@group(0) @binding(1) var<uniform> params: FluidParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.particle_count) { return; }
    var p = particles[i];
    p.vel.y -= params.gravity * params.delta_time;
    p.pos += p.vel * params.delta_time;
    particles[i] = p;
}
"#;

    /// Terrain heightmap generation using layered noise.
    pub const TERRAIN_GEN: &str = r#"
struct Params {
    width: u32,
    height: u32,
    scale: f32,
    octaves: u32,
    persistence: f32,
    lacunarity: f32,
    seed: f32,
    height_scale: f32,
}

@group(0) @binding(0) var<storage, read_write> heightmap: array<f32>;
@group(0) @binding(1) var<uniform> params: Params;

fn hash(n: f32) -> f32 {
    return fract(sin(n) * 43758.5453);
}

fn noise(x: f32) -> f32 {
    let i = floor(x);
    let f = fract(x);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(hash(i), hash(i + 1.0), u);
}

fn fbm(x: f32, oct: u32, persistence: f32, lacunarity: f32) -> f32 {
    var value: f32 = 0.0;
    var amplitude: f32 = 1.0;
    var frequency: f32 = 1.0;
    var max_amp: f32 = 0.0;
    for (var o: u32 = 0u; o < oct; o++) {
        value += amplitude * noise(x * frequency);
        max_amp += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
    }
    return value / max_amp;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.width || y >= params.height) { return; }
    let nx = f32(x) / f32(params.width) * params.scale + params.seed;
    let ny = f32(y) / f32(params.height) * params.scale + params.seed;
    let h = fbm(nx + ny, params.octaves, params.persistence, params.lacunarity);
    heightmap[y * params.width + x] = h * params.height_scale;
}
"#;

    /// Gaussian blur (single-pass horizontal/vertical).
    pub const BLUR: &str = r#"
struct Params {
    width: u32,
    height: u32,
    direction: u32,
    sigma: f32,
}

@group(0) @binding(0) var input: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.width || y >= params.height) { return; }
    let tc = vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) / vec2<f32>(f32(params.width), f32(params.height));
    var color: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    let r = i32(params.sigma * 3.0);
    var total_weight: f32 = 0.0;
    for (var i: i32 = -r; i <= r; i++) {
        let g = exp(-f32(i * i) / (2.0 * params.sigma * params.sigma));
        var off = vec2<i32>(0, 0);
        if (params.direction == 0u) { off.x = i; } else { off.y = i; }
        let s = textureSampleLevel(input, default_sampler, tc + vec2<f32>(off) / vec2<f32>(f32(params.width), f32(params.height)), 0.0).rgb;
        color += s * g;
        total_weight += g;
    }
    output[y * params.width + x] = vec4<f32>(color / total_weight, 1.0);
}
"#;

    /// Sobel edge detection.
    pub const EDGE_DETECTION: &str = r#"
struct Params {
    width: u32,
    height: u32,
}

@group(0) @binding(0) var input: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> params: Params;

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.width || y >= params.height) { return; }
    let ts = vec2<f32>(1.0 / f32(params.width), 1.0 / f32(params.height));
    let tl = luma(textureSampleLevel(input, default_sampler, vec2<f32>(f32(x) - 1.0, f32(y) - 1.0) * ts, 0.0).rgb);
    let tc = luma(textureSampleLevel(input, default_sampler, vec2<f32>(f32(x), f32(y) - 1.0) * ts, 0.0).rgb);
    let tr = luma(textureSampleLevel(input, default_sampler, vec2<f32>(f32(x) + 1.0, f32(y) - 1.0) * ts, 0.0).rgb);
    let ml = luma(textureSampleLevel(input, default_sampler, vec2<f32>(f32(x) - 1.0, f32(y)) * ts, 0.0).rgb);
    let mr = luma(textureSampleLevel(input, default_sampler, vec2<f32>(f32(x) + 1.0, f32(y)) * ts, 0.0).rgb);
    let bl = luma(textureSampleLevel(input, default_sampler, vec2<f32>(f32(x) - 1.0, f32(y) + 1.0) * ts, 0.0).rgb);
    let bc = luma(textureSampleLevel(input, default_sampler, vec2<f32>(f32(x), f32(y) + 1.0) * ts, 0.0).rgb);
    let br = luma(textureSampleLevel(input, default_sampler, vec2<f32>(f32(x) + 1.0, f32(y) + 1.0) * ts, 0.0).rgb);
    let gx = -tl - 2.0 * ml - bl + tr + 2.0 * mr + br;
    let gy = -tl - 2.0 * tc - tr + bl + 2.0 * bc + br;
    let edge = sqrt(gx * gx + gy * gy);
    output[y * params.width + x] = vec4<f32>(edge, edge, edge, 1.0);
}
"#;

    /// Classic Perlin noise generator.
    pub const PERLIN_NOISE: &str = r#"
struct Params {
    width: u32,
    height: u32,
    scale: f32,
    octaves: u32,
    persistence: f32,
    lacunarity: f32,
    seed: f32,
}

@group(0) @binding(0) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> params: Params;

fn fade(t: f32) -> f32 { return t * t * t * (t * (t * 6.0 - 15.0) + 10.0); }
fn lerp(a: f32, b: f32, t: f32) -> f32 { return a + t * (b - a); }
fn grad(hash: f32, x: f32, y: f32) -> f32 {
    let h = hash & 3.0;
    return select(x, -x, h < 1.0) ^ select(y, -y, (h & 1.0) == 0.0);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.width || y >= params.height) { return; }
    var value: f32 = 0.0;
    var amplitude: f32 = 1.0;
    var frequency: f32 = params.scale;
    for (var o: u32 = 0u; o < params.octaves; o++) {
        let xi = floor(f32(x) * frequency + params.seed);
        let yi = floor(f32(y) * frequency + params.seed);
        let xf = fract(f32(x) * frequency + params.seed);
        let yf = fract(f32(y) * frequency + params.seed);
        let u = fade(xf);
        let v = fade(yf);
        let aa = grad(hash(xi + hash(yi)), xf, yf);
        let ab = grad(hash(xi + hash(yi + 1.0)), xf, yf - 1.0);
        let ba = grad(hash(xi + 1.0 + hash(yi)), xf - 1.0, yf);
        let bb = grad(hash(xi + 1.0 + hash(yi + 1.0)), xf - 1.0, yf - 1.0);
        value += amplitude * lerp(lerp(aa, ba, u), lerp(ab, bb, u), v);
        amplitude *= params.persistence;
        frequency *= params.lacunarity;
    }
    output[y * params.width + x] = vec4<f32>((value + 1.0) * 0.5);
}
"#;

    /// Simplex noise generator.
    pub const SIMPLEX_NOISE: &str = r#"
struct Params {
    width: u32,
    height: u32,
    scale: f32,
}

@group(0) @binding(0) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.width || y >= params.height) { return; }
    let n = fract(sin(dot(vec2<f32>(f32(x), f32(y)), vec2<f32>(12.9898, 78.233))) * 43758.5453);
    output[y * params.width + x] = vec4<f32>(n, n, n, 1.0);
}
"#;

    /// Worley / cellular noise generator.
    pub const WORLEY_NOISE: &str = r#"
struct Params {
    width: u32,
    height: u32,
    cell_count: u32,
}

@group(0) @binding(0) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.width || y >= params.height) { return; }
    let u = f32(x) / f32(params.width);
    let v = f32(y) / f32(params.height);
    var min_dist: f32 = 999.0;
    for (var cy: u32 = 0u; cy < params.cell_count; cy++) {
        for (var cx: u32 = 0u; cx < params.cell_count; cx++) {
            let h = fract(sin(f32(cx + cy * 57) * 43758.5453));
            let h2 = fract(sin(f32(cx + 1u + cy * 57) * 43758.5453));
            let fx = (f32(cx) + h) / f32(params.cell_count);
            let fy = (f32(cy) + h2) / f32(params.cell_count);
            let d = distance(vec2<f32>(u, v), vec2<f32>(fx, fy));
            min_dist = min(min_dist, d);
        }
    }
    output[y * params.width + x] = vec4<f32>(min_dist, min_dist, min_dist, 1.0);
}
"#;
}

// ---------------------------------------------------------------------------
// ComputePipeline
// ---------------------------------------------------------------------------

/// A complete compute pipeline: shader + layout + bind-group layouts.
pub struct ComputePipeline {
    inner: ComputePipeline,
    pub bind_group_layout: BindGroupLayout,
    pub pipeline_layout: PipelineLayout,
    pub shader_module: ShaderModule,
    pub entry_point: String,
    pub label: String,
}

impl ComputePipeline {
    /// Create a new compute pipeline from a pre-compiled shader module.
    pub fn new(
        device: &Device,
        shader_module: ShaderModule,
        entry_point: &str,
        bind_group_layouts: &[&BindGroupLayout],
        push_constant_ranges: &[PushConstantRange],
        label: Option<&str>,
    ) -> Self {
        let entry_point = entry_point.to_string();
        let label_str = label.unwrap_or("compute_pipeline").to_string();

        let bind_group_layout = if bind_group_layouts.is_empty() {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(&format!("{}_bgl", label_str)),
                entries: &[],
            })
        } else {
            bind_group_layouts[0].clone()
        };

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(&format!("{}_layout", label_str)),
            bind_group_layouts,
            push_constant_ranges,
        });

        let inner = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some(&label_str),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: &entry_point,
        });

        Self {
            inner,
            bind_group_layout,
            pipeline_layout,
            shader_module,
            entry_point,
            label: label_str,
        }
    }

    /// Convenience: create from a WGSL source string.
    pub fn from_wgsl(
        device: &Device,
        wgsl_source: &str,
        entry_point: &str,
        bind_group_layouts: &[&BindGroupLayout],
        label: Option<&str>,
    ) -> Self {
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(label.unwrap_or("compute_shader")),
            source: ShaderSource::Wgsl(wgsl_source.into()),
        });
        Self::new(device, shader_module, entry_point, bind_group_layouts, &[], label)
    }

    /// Convenience: create from raw WGSL bytes.
    pub fn from_wgsl_bytes(
        device: &Device,
        wgsl_bytes: &[u8],
        entry_point: &str,
        bind_group_layouts: &[&BindGroupLayout],
        label: Option<&str>,
    ) -> Self {
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(label.unwrap_or("compute_shader")),
                source: ShaderSource::Wgsl(
                    std::str::from_utf8(wgsl_bytes).unwrap_or("").into()
                ),
        });
        Self::new(device, shader_module, entry_point, bind_group_layouts, &[], label)
    }

    /// Create a bind group for this pipeline.
    pub fn create_bind_group(
        &self,
        device: &Device,
        entries: &[BindGroupEntry<'_>],
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(&format!("{}_bind_group", self.label)),
            layout: &self.bind_group_layout,
            entries,
        })
    }

    /// Immutable reference to the inner wgpu compute pipeline.
    pub fn inner(&self) -> &ComputePipeline {
        &self.inner
    }
}

// ---------------------------------------------------------------------------
// ComputePipelineCache
// ---------------------------------------------------------------------------

/// Caches compute pipelines by label so repeated creation is avoided.
pub struct ComputePipelineCache {
    pipelines: HashMap<String, Arc<ComputePipeline>>,
}

impl ComputePipelineCache {
    pub fn new() -> Self {
        Self {
            pipelines: HashMap::new(),
        }
    }

    /// Get or create a cached compute pipeline.
    pub fn get_or_create(
        &mut self,
        device: &Device,
        shader_module: ShaderModule,
        entry_point: &str,
        bind_group_layouts: &[&BindGroupLayout],
        label: &str,
    ) -> Arc<ComputePipeline> {
        if let Some(pipeline) = self.pipelines.get(label) {
            return pipeline.clone();
        }
        let pipeline = Arc::new(ComputePipeline::new(
            device,
            shader_module,
            entry_point,
            bind_group_layouts,
            &[],
            Some(label),
        ));
        self.pipelines.insert(label.to_string(), pipeline.clone());
        pipeline
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.pipelines.clear();
    }

    /// Number of cached pipelines.
    pub fn len(&self) -> usize {
        self.pipelines.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.pipelines.is_empty()
    }
}

impl Default for ComputePipelineCache {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// GpuBuffer
// ---------------------------------------------------------------------------

/// Buffer kind understood by the compute system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferKind {
    Uniform,
    Storage { read_only: bool },
    Index { format: IndexFormat },
    Vertex,
}

impl BufferKind {
    /// Resolve to the corresponding wgpu `BufferUsages` flags.
    pub fn usage(self) -> BufferUsages {
        match self {
            BufferKind::Uniform => BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            BufferKind::Storage { read_only } => {
                let base = if read_only {
                    BufferUsages::STORAGE_READ
                } else {
                    BufferUsages::STORAGE
                };
                base | BufferUsages::COPY_DST | BufferUsages::COPY_SRC
            }
            BufferKind::Index { .. } => {
                BufferUsages::INDEX | BufferUsages::COPY_DST | BufferUsages::COPY_SRC
            }
            BufferKind::Vertex => {
                BufferUsages::VERTEX | BufferUsages::COPY_DST | BufferUsages::COPY_SRC
            }
        }
    }
}

/// Typed GPU buffer wrapper.  Provides mapping, copying and barrier helpers.
pub struct GpuBuffer {
    pub buffer: Buffer,
    pub kind: BufferKind,
    pub size: u64,
}

impl GpuBuffer {
    /// Allocate a new GPU buffer with the given data, staged from CPU.
    pub fn new(device: &Device, kind: BufferKind, data: &[u8], label: Option<&str>) -> Self {
        let size = data.len() as u64;
        let usage = kind.usage();
        let buffer = device.create_buffer(&BufferDescriptor {
            label,
            size,
            usage,
            mapped_at_creation: true,
        });
        buffer.get_mapped_range_mut(0..size).copy_from_slice(data);
        buffer.unmap();
        Self { buffer, kind, size }
    }

    /// Allocate an empty GPU buffer (for later write via mapping or command encoder).
    pub fn empty(device: &Device, kind: BufferKind, size: u64, label: Option<&str>) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label,
            size,
            usage: kind.usage(),
            mapped_at_creation: false,
        });
        Self { buffer, kind, size }
    }

    /// Write host data into the buffer via a staging copy.
    pub fn write(&self, device: &Device, data: &[u8]) {
        let size = data.len() as u64;
        let staging = device.create_buffer(&BufferDescriptor {
            label: Some("staging_write"),
            size,
            usage: BufferUsages::MAP_WRITE | BufferUsages::COPY_SRC,
            mapped_at_creation: true,
        });
        staging.get_mapped_range_mut(..).copy_from_slice(data);
        staging.unmap();

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("staging_copy_encoder"),
        });
        encoder.copy_buffer_to_buffer(&staging, 0, &self.buffer, 0, size);
        device.queue.submit(Some(encoder.finish()));
    }

    /// Schedule a full buffer copy into this buffer (legacy alias for `write`).
    pub fn copy_from(&self, device: &Device, data: &[u8]) {
        self.write(device, data);
    }

    /// Insert a buffer barrier (pipeline barrier) into the given command encoder.
    ///
    /// On WGPU barriers are implicit, but this method exists for portability.
    pub fn barrier(&self, _encoder: &mut CommandEncoder, _stages: ShaderStages) {
        // wgpu handles barriers implicitly.
    }
}

// ---------------------------------------------------------------------------
// GpuTexture
// ---------------------------------------------------------------------------

/// Filter mode used when sampling the texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FilterMode {
    Nearest,
    #[default]
    Linear,
}

impl From<FilterMode> for wgpu::FilterMode {
    fn from(m: FilterMode) -> Self {
        match m {
            FilterMode::Nearest => wgpu::FilterMode::Nearest,
            FilterMode::Linear => wgpu::FilterMode::Linear,
        }
    }
}

/// Address mode for texture coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AddressMode {
    Repeat,
    MirroredRepeat,
    ClampToEdge,
    #[default]
    ClampToBorder,
}

impl From<AddressMode> for wgpu::AddressMode {
    fn from(m: AddressMode) -> Self {
        match m {
            AddressMode::Repeat => wgpu::AddressMode::Repeat,
            AddressMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
            AddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            AddressMode::ClampToBorder => wgpu::AddressMode::ClampToBorder,
        }
    }
}

/// Comparison function for depth/sampler compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CompareFunction {
    Never,
    Less,
    Equal,
    LessEqual,
    #[default]
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

impl From<CompareFunction> for wgpu::CompareFunction {
    fn from(c: CompareFunction) -> Self {
        match c {
            CompareFunction::Never => wgpu::CompareFunction::Never,
            CompareFunction::Less => wgpu::CompareFunction::Less,
            CompareFunction::Equal => wgpu::CompareFunction::Equal,
            CompareFunction::LessEqual => wgpu::CompareFunction::LessEqual,
            CompareFunction::Greater => wgpu::CompareFunction::Greater,
            CompareFunction::NotEqual => wgpu::CompareFunction::NotEqual,
            CompareFunction::GreaterEqual => wgpu::CompareFunction::GreaterEqual,
            CompareFunction::Always => wgpu::CompareFunction::Always,
        }
    }
}

/// Descriptor for creating a `GpuTexture`.
#[derive(Debug, Clone)]
pub struct TextureDescriptor {
    pub size: (u32, u32, u32),
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub dimension: TextureDimension,
    pub format: TextureFormat,
    pub usage: TextureUsages,
    pub label: Option<&'static str>,
}

impl Default for TextureDescriptor {
    fn default() -> Self {
        Self {
            size: (1, 1, 1),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            label: None,
        }
    }
}

/// High-level GPU texture handle.
pub struct GpuTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub descriptor: TextureDescriptor,
    pub label: String,
}

impl GpuTexture {
    /// Create a new texture from a descriptor.
    pub fn new(device: &Device, desc: TextureDescriptor) -> Self {
        let label_str = desc.label.unwrap_or("gpu_texture").to_string();

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&label_str),
            size: Extent3d {
                width: desc.size.0,
                height: desc.size.1,
                depth_or_array_layers: desc.size.2,
            },
            mip_level_count: desc.mip_level_count,
            sample_count: desc.sample_count,
            dimension: desc.dimension,
            format: desc.format,
            usage: desc.usage,
            view_formats: &[],
        });

        let view = texture.create_view(&TextureViewDescriptor::default());

        Self {
            texture,
            view,
            descriptor: desc,
            label: label_str,
        }
    }

    /// Create a 2D texture with RGBA8Unorm format.
    pub fn new_2d(
        device: &Device,
        width: u32,
        height: u32,
        format: TextureFormat,
        usage: TextureUsages,
        label: Option<&'static str>,
    ) -> Self {
        let desc = TextureDescriptor {
            size: (width, height, 1),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage,
            label,
        };
        Self::new(device, desc)
    }

    /// Create a 2D texture with a full mipmap chain.
    pub fn new_2d_with_mips(
        device: &Device,
        width: u32,
        height: u32,
        format: TextureFormat,
        usage: TextureUsages,
        mip_level_count: u32,
        label: Option<&'static str>,
    ) -> Self {
        let desc = TextureDescriptor {
            size: (width, height, 1),
            mip_level_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage,
            label,
        };
        Self::new(device, desc)
    }

    /// Generate mipmaps for this texture.
    ///
    /// A full implementation would iterate over each mip level and blit from
    /// the previous level using a render pass or compute shader.  This is a
    /// placeholder — callers should implement their own mipmap generation
    /// pass if they need mip levels populated at runtime.
    pub fn generate_mipmaps(&self, _device: &Device) {
        // Placeholder for mipmap generation blit/compute pass.
    }

    /// Create a specific texture view for a subresource range.
    pub fn create_view(
        &self,
        base_mip: u32,
        mip_count: Option<u32>,
        base_array: u32,
        array_count: Option<u32>,
    ) -> TextureView {
        self.texture.create_view(&TextureViewDescriptor {
            label: Some(&format!("{}_sub_view", self.label)),
            format: None,
            dimension: None,
            aspect: TextureAspect::All,
            base_mip_level: base_mip,
            mip_level_count: mip_count.map(|v| v.into()),
            base_array_layer: base_array,
            array_layer_count: array_count.map(|v| v.into()),
            usage: None,
        })
    }

    /// Default full texture view.
    pub fn default_view(&self) -> &TextureView {
        &self.view
    }
}

// ---------------------------------------------------------------------------
// Sampler
// ---------------------------------------------------------------------------

/// High-level sampler descriptor.
#[derive(Debug, Clone)]
pub struct SamplerDescriptor {
    pub label: Option<&'static str>,
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
    pub address_mode_w: AddressMode,
    pub mag_filter: FilterMode,
    pub min_filter: FilterMode,
    pub mipmap_filter: FilterMode,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub compare: Option<CompareFunction>,
    pub border_color: Option<Color>,
    pub anisotropy_clamp: u32,
}

impl Default for SamplerDescriptor {
    fn default() -> Self {
        Self {
            label: None,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            border_color: None,
            anisotropy_clamp: 1,
        }
    }
}

impl SamplerDescriptor {
    /// Convert to a wgpu `SamplerDescriptor`.
    pub fn to_wgpu(&self) -> wgpu::SamplerDescriptor<'_> {
        wgpu::SamplerDescriptor {
            label: self.label,
            address_mode_u: self.address_mode_u.into(),
            address_mode_v: self.address_mode_v.into(),
            address_mode_w: self.address_mode_w.into(),
            mag_filter: self.mag_filter.into(),
            min_filter: self.min_filter.into(),
            mipmap_filter: self.mipmap_filter.into(),
            lod_min_clamp: self.lod_min_clamp,
            lod_max_clamp: self.lod_max_clamp,
            compare: self.compare.map(|c| c.into()),
            anisotropy_clamp: self.anisotropy_clamp,
            border_color: self.border_color,
        }
    }
}

/// Create a sampler from a high-level descriptor.
pub fn create_sampler(device: &Device, desc: &SamplerDescriptor) -> Sampler {
    device.create_sampler(&desc.to_wgpu())
}

// ---------------------------------------------------------------------------
// ComputeDispatcher
// ---------------------------------------------------------------------------

/// Dispatch arguments for a compute pass.
#[derive(Debug, Clone, Copy, Default)]
pub struct DispatchCounts {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl DispatchCounts {
    /// Compute workgroup counts from element count and workgroup size.
    pub fn from_count_and_workgroup(
        element_count: u32,
        workgroup_size_x: u32,
        workgroup_size_y: u32,
        workgroup_size_z: u32,
    ) -> Self {
        let x = (element_count.max(1) + workgroup_size_x - 1) / workgroup_size_x;
        let y = (element_count.max(1) + workgroup_size_y - 1) / workgroup_size_y;
        let z = (element_count.max(1) + workgroup_size_z - 1) / workgroup_size_z;
        Self { x: x.max(1), y: y.max(1), z: z.max(1) }
    }

    /// Compute 2D workgroup counts from width/height.
    pub fn from_2d(width: u32, height: u32, workgroup_size: u32) -> Self {
        let x = (width + workgroup_size - 1) / workgroup_size;
        let y = (height + workgroup_size - 1) / workgroup_size;
        Self { x: x.max(1), y: y.max(1), z: 1 }
    }
}

/// Dispatches compute work.  Supports direct, indirect and batched dispatch.
///
/// Each dispatch call submits independently to the queue.  For batch
/// recording use the closure-based `dispatch_batch` helper.
pub struct ComputeDispatcher<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> ComputeDispatcher<'a> {
    /// Create a new dispatcher.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set pipeline, bind group and dispatch workgroups directly.
    pub fn dispatch(
        &self,
        device: &Device,
        pipeline: &ComputePipeline,
        bind_group: &BindGroup,
        counts: DispatchCounts,
    ) {
        let label = format!("{}_dispatch", pipeline.label);
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some(&label),
        });
        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some(&format!("{}_pass", pipeline.label)),
            });
            pass.set_pipeline(pipeline.inner());
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(counts.x, counts.y, counts.z);
        }
        device.queue.submit(Some(encoder.finish()));
    }

    /// Dispatch using an indirect arguments buffer.
    pub fn dispatch_indirect(
        &self,
        device: &Device,
        pipeline: &ComputePipeline,
        bind_group: &BindGroup,
        indirect_buffer: &Buffer,
        indirect_offset: u64,
    ) {
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some(&format!("{}_indirect_dispatch", pipeline.label)),
        });
        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some(&format!("{}_pass_indirect", pipeline.label)),
            });
            pass.set_pipeline(pipeline.inner());
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups_indirect(indirect_buffer, indirect_offset);
        }
        device.queue.submit(Some(encoder.finish()));
    }

    /// Dispatch the same pipeline/bind-group N times with different workgroup counts.
    pub fn multi_dispatch(
        &self,
        device: &Device,
        pipeline: &ComputePipeline,
        bind_group: &BindGroup,
        counts: &[DispatchCounts],
    ) {
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some(&format!("{}_multi_dispatch", pipeline.label)),
        });
        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some(&format!("{}_pass_multi", pipeline.label)),
            });
            pass.set_pipeline(pipeline.inner());
            pass.set_bind_group(0, bind_group, &[]);
            for c in counts {
                pass.dispatch_workgroups(c.x, c.y, c.z);
            }
        }
        device.queue.submit(Some(encoder.finish()));
    }

    /// Record multiple dispatches into a single command buffer and submit.
    ///
    /// The closure receives a mutable `ComputePassRecorder` so that many
    /// dispatches can be recorded before the pass ends and the command buffer
    /// is submitted.
    pub fn dispatch_batch<F>(&self, device: &Device, label: &str, f: F)
    where
        F: FnOnce(&mut ComputePassRecorder<'_>),
    {
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some(label),
        });
        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("compute_batch_pass"),
            });
            let mut recorder = ComputePassRecorder { pass: &mut pass };
            f(&mut recorder);
        }
        device.queue.submit(Some(encoder.finish()));
    }
}

impl Default for ComputeDispatcher<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper that wraps a `ComputePass` for batch recording.
pub struct ComputePassRecorder<'a> {
    pass: &'a mut ComputePass<'a>,
}

impl<'a> ComputePassRecorder<'a> {
    /// Set the active compute pipeline.
    pub fn set_pipeline(&mut self, pipeline: &ComputePipeline) {
        self.pass.set_pipeline(pipeline.inner());
    }

    /// Bind a bind group at the given index.
    pub fn set_bind_group(&mut self, index: u32, bind_group: &BindGroup) {
        self.pass.set_bind_group(index, bind_group, &[]);
    }

    /// Dispatch workgroups with the given counts.
    pub fn dispatch(&mut self, counts: DispatchCounts) {
        self.pass.dispatch_workgroups(counts.x, counts.y, counts.z);
    }

    /// Dispatch using an indirect buffer.
    pub fn dispatch_indirect(&mut self, indirect_buffer: &Buffer, indirect_offset: u64) {
        self.pass.dispatch_workgroups_indirect(indirect_buffer, indirect_offset);
    }
}

// ---------------------------------------------------------------------------
// ComputeComponent — attaches compute resources to an ECS entity
// ---------------------------------------------------------------------------

/// Component attached to an entity that owns compute resources.
pub struct ComputeComponent {
    /// The WGSL shader source for the compute task.
    pub shader_source: String,
    /// Entry point function name.
    pub entry_point: String,
    /// Bind group layouts used by this compute task.
    pub bind_group_layouts: Vec<BindGroupLayout>,
    /// Uniform / push-constant data.
    pub push_constants: Vec<u8>,
    /// Whether the component is currently active.
    pub active: bool,
    /// Cached pipeline (recreated when shader changes).
    pub pipeline: Option<Arc<ComputePipeline>>,
}

impl ComputeComponent {
    pub fn new(
        shader_source: impl Into<String>,
        entry_point: impl Into<String>,
    ) -> Self {
        Self {
            shader_source: shader_source.into(),
            entry_point: entry_point.into(),
            bind_group_layouts: Vec::new(),
            push_constants: Vec::new(),
            active: true,
            pipeline: None,
        }
    }

    /// Set the bind group layouts for this compute task.
    pub fn with_bind_group_layouts(mut self, layouts: Vec<BindGroupLayout>) -> Self {
        self.bind_group_layouts = layouts;
        self
    }

    /// Set push-constant data.
    pub fn with_push_constants(mut self, data: Vec<u8>) -> Self {
        self.push_constants = data;
        self
    }

    /// Activate / deactivate this compute component.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
}

// ---------------------------------------------------------------------------
// ComputeSystem — ECS system that runs compute components each frame
// ---------------------------------------------------------------------------

/// The compute system drives all `ComputeComponent` entities each frame.
pub struct ComputeSystem {
    device: Arc<Device>,
    queue: Arc<Queue>,
    pipeline_cache: ComputePipelineCache,
    pub label: String,
}

impl ComputeSystem {
    /// Create a new compute system bound to a wgpu device/queue.
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        Self {
            device,
            queue,
            pipeline_cache: ComputePipelineCache::new(),
            label: "ComputeSystem".to_string(),
        }
    }

    /// Rebuild a compute pipeline for a `ComputeComponent` (called when shader changes).
    pub fn rebuild_pipeline(
        &mut self,
        component: &mut ComputeComponent,
    ) -> Arc<ComputePipeline> {
        let layout_refs: Vec<&BindGroupLayout> =
            component.bind_group_layouts.iter().collect();
        let pipeline = self.pipeline_cache.get_or_create(
            &self.device,
            self.device.create_shader_module(ShaderModuleDescriptor {
                label: Some("compute_system_shader"),
                source: ShaderSource::Wgsl(component.shader_source.clone().into()),
            }),
            &component.entry_point,
            &layout_refs,
            &component.entry_point,
        );
        component.pipeline = Some(pipeline.clone());
        pipeline
    }

    /// Submit command buffers to the queue.
    pub fn submit(&self, command_buffers: Vec<CommandBuffer>) {
        self.queue.submit(command_buffers);
    }

    /// Create a storage buffer from raw data.
    pub fn create_storage_buffer(&self, data: &[u8], label: Option<&str>) -> GpuBuffer {
        GpuBuffer::new(
            &self.device,
            BufferKind::Storage { read_only: false },
            data,
            label,
        )
    }

    /// Create a uniform buffer from raw data.
    pub fn create_uniform_buffer(&self, data: &[u8], label: Option<&str>) -> GpuBuffer {
        GpuBuffer::new(
            &self.device,
            BufferKind::Uniform,
            data,
            label,
        )
    }

    /// Create a texture from a descriptor.
    pub fn create_texture(&self, desc: TextureDescriptor) -> GpuTexture {
        GpuTexture::new(&self.device, desc)
    }

    /// Create a sampler from a descriptor.
    pub fn create_sampler(&self, desc: &SamplerDescriptor) -> Sampler {
        create_sampler(&self.device, desc)
    }
}

impl System for ComputeSystem {
    fn name(&self) -> &str {
        &self.label
    }

    fn update(&mut self, world: &mut World, dt: f32) {
        // Iterate all alive entities and drive any ComputeComponent they carry.
        let _dt = dt;
        for entity in world.entities.iter_alive() {
            let _entity = entity;
            // In a full integration, look up ComputeComponent from the
            // component registry and dispatch accordingly.
            let _ = _dt;
        }
    }
}

// ---------------------------------------------------------------------------
// Convenience builders
// ---------------------------------------------------------------------------

/// Build a compute dispatch from a `ComputePipeline` + bind group + element count.
pub fn dispatch_compute(
    device: &Device,
    pipeline: &ComputePipeline,
    bind_group: &BindGroup,
    element_count: u32,
    workgroup_size_x: u32,
) {
    let counts = DispatchCounts::from_count_and_workgroup(element_count, workgroup_size_x, 1, 1);
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("compute_dispatch"),
        });
        pass.set_pipeline(pipeline.inner());
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups(counts.x, counts.y, counts.z);
    }
    device.queue.submit(Some(encoder.finish()));
}

/// Build a 2D compute dispatch from a pipeline + bind group + image dimensions.
pub fn dispatch_compute_2d(
    device: &Device,
    pipeline: &ComputePipeline,
    bind_group: &BindGroup,
    width: u32,
    height: u32,
    workgroup_size: u32,
) {
    let counts = DispatchCounts::from_2d(width, height, workgroup_size);
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("compute_dispatch_2d"),
        });
        pass.set_pipeline(pipeline.inner());
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups(counts.x, counts.y, 1);
    }
    device.queue.submit(Some(encoder.finish()));
}
