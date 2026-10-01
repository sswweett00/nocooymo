
/// Post-processing efektlerini tanımlar
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PostProcessEffect {
    Bloom,
    Ssao,
    MotionBlur,
    Fxaa,
    Taa,
    ToneMapping,
}

/// Bloom efekti için parametreler
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

/// SSAO efekti için parametreler
#[derive(Debug, Clone)]
pub struct SsaoParams {
    pub sample_radius: f32,
    pub bias: f32,
    pub intensity: f32,
    pub power: f32,
    pub quality: u32,
}

impl Default for SsaoParams {
    fn default() -> Self {
        Self {
            sample_radius: 0.5,
            bias: 0.05,
            intensity: 1.0,
            power: 2.0,
            quality: 8,
        }
    }
}

/// Motion blur efekti için parametreler
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

/// Post-processing pipeline yapılandırması
#[derive(Debug, Clone)]
pub struct PostProcessPipeline {
    pub enabled_effects: Vec<PostProcessEffect>,
    pub bloom_params: BloomParams,
    pub ssao_params: SsaoParams,
    pub motion_blur_params: MotionBlurParams,
    pub fxaa_enabled: bool,
    pub taa_enabled: bool,
    pub tone_mapping_enabled: bool,
}

impl Default for PostProcessPipeline {
    fn default() -> Self {
        Self {
            enabled_effects: vec![PostProcessEffect::Bloom, PostProcessEffect::Ssao],
            bloom_params: BloomParams::default(),
            ssao_params: SsaoParams::default(),
            motion_blur_params: MotionBlurParams::default(),
            fxaa_enabled: true,
            taa_enabled: false,
            tone_mapping_enabled: true,
        }
    }
}

impl PostProcessPipeline {
    pub fn enable_effect(&mut self, effect: PostProcessEffect) {
        if !self.enabled_effects.contains(&effect) {
            self.enabled_effects.push(effect);
        }
    }

    pub fn disable_effect(&mut self, effect: PostProcessEffect) {
        self.enabled_effects.retain(|&e| e != effect);
    }

    pub fn is_effect_enabled(&self, effect: PostProcessEffect) -> bool {
        self.enabled_effects.contains(&effect)
    }

    /// WGSL shader kaynak kodlarını döndürür
    pub fn get_bloom_shader_wgsl() -> &'static str {
        r#"
@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

struct BloomParams {
    intensity: f32,
    threshold: f32,
    soft_knee: f32,
    radius: f32,
}
@group(0) @binding(2) var<uniform> params: BloomParams;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0)
    );
    return vec4(pos[idx], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(source_tex);
    let uv = pos.xy / vec2<f32>(f32(dims.x), f32(dims.y));
    let color = textureSample(source_tex, source_sampler, uv).rgb;
    let brightness = max(color - params.threshold, vec3(0.0));
    let knee = brightness * smoothstep(vec3(0.0), vec3(params.soft_knee), brightness);
    return vec4(knee * params.intensity, 1.0);
}
"#
    }

    pub fn get_ssao_shader_wgsl() -> &'static str {
        r#"
@group(0) @binding(0) var position_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var noise_tex: texture_2d<f32>;
@group(0) @binding(3) var tex_sampler: sampler;

struct SsaoParams {
    sample_radius: f32,
    bias: f32,
    intensity: f32,
    power: f32,
}
@group(0) @binding(4) var<uniform> params: SsaoParams;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0)
    );
    return vec4(pos[idx], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) f32 {
    let dims = textureDimensions(position_tex);
    let uv = pos.xy / vec2<f32>(f32(dims.x), f32(dims.y));
    let frag_pos = textureSample(position_tex, tex_sampler, uv).xyz;
    let normal = normalize(textureSample(normal_tex, tex_sampler, uv).xyz);
    return 1.0; // placeholder — tam kernel değerleri GPU'da doldurulur
}
"#
    }

    pub fn get_fxaa_shader_wgsl() -> &'static str {
        r#"
@group(0) @binding(0) var color_tex: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;

struct FxaaParams {
    inverse_resolution: vec2<f32>,
    edge_threshold: f32,
    edge_threshold_min: f32,
}
@group(0) @binding(2) var<uniform> params: FxaaParams;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0)
    );
    return vec4(pos[idx], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(color_tex);
    let uv = pos.xy / vec2<f32>(f32(dims.x), f32(dims.y));
    let inv = params.inverse_resolution;

    let rgbNW = textureSample(color_tex, tex_sampler, uv + inv * vec2(-1.0, -1.0)).rgb;
    let rgbNE = textureSample(color_tex, tex_sampler, uv + inv * vec2( 1.0, -1.0)).rgb;
    let rgbSW = textureSample(color_tex, tex_sampler, uv + inv * vec2(-1.0,  1.0)).rgb;
    let rgbSE = textureSample(color_tex, tex_sampler, uv + inv * vec2( 1.0,  1.0)).rgb;
    let rgbM  = textureSample(color_tex, tex_sampler, uv).rgb;

    let luma = vec3(0.299, 0.587, 0.114);
    let lumaNW = dot(rgbNW, luma);
    let lumaNE = dot(rgbNE, luma);
    let lumaSW = dot(rgbSW, luma);
    let lumaSE = dot(rgbSE, luma);
    let lumaM  = dot(rgbM,  luma);

    let lumaMin = min(lumaM, min(min(lumaNW, lumaNE), min(lumaSW, lumaSE)));
    let lumaMax = max(lumaM, max(max(lumaNW, lumaNE), max(lumaSW, lumaSE)));
    let lumaRange = lumaMax - lumaMin;

    if lumaRange < max(params.edge_threshold_min, lumaMax * params.edge_threshold) {
        return vec4(rgbM, 1.0);
    }

    var dir: vec2<f32>;
    dir.x = -((lumaNW + lumaNE) - (lumaSW + lumaSE));
    dir.y =  ((lumaNW + lumaSW) - (lumaNE + lumaSE));

    let dir_reduce = max((lumaNW + lumaNE + lumaSW + lumaSE) * 0.25 * params.edge_threshold, 0.25);
    let rcp_dir_min = 1.0 / (min(abs(dir.x), abs(dir.y)) + dir_reduce);
    dir = clamp(dir * rcp_dir_min, vec2(-8.0), vec2(8.0)) * inv;

    let rgbA = 0.5 * (
        textureSample(color_tex, tex_sampler, uv + dir * (1.0/3.0 - 0.5)).rgb +
        textureSample(color_tex, tex_sampler, uv + dir * (2.0/3.0 - 0.5)).rgb
    );
    let rgbB = rgbA * 0.5 + 0.25 * (
        textureSample(color_tex, tex_sampler, uv + dir * (-0.5)).rgb +
        textureSample(color_tex, tex_sampler, uv + dir *  0.5).rgb
    );

    let lumaB = dot(rgbB, luma);
    if lumaB < lumaMin || lumaB > lumaMax {
        return vec4(rgbA, 1.0);
    }
    return vec4(rgbB, 1.0);
}
"#
    }

    pub fn get_tonemap_shader_wgsl() -> &'static str {
        r#"
@group(0) @binding(0) var hdr_tex: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;

struct ToneMapParams {
    exposure: f32,
    mode: u32, // 0=Reinhard, 1=ACES, 2=Uncharted2
}
@group(0) @binding(2) var<uniform> params: ToneMapParams;

const A = 2.51;
const B = 0.03;
const C = 2.43;
const D = 0.59;
const E = 0.14;

fn aces(c: vec3<f32>) -> vec3<f32> {
    return clamp((c * (A * c + B)) / (c * (C * c + D) + E), vec3(0.0), vec3(1.0));
}

fn reinhard(c: vec3<f32>) -> vec3<f32> {
    return c / (c + 1.0);
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0)
    );
    return vec4(pos[idx], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(hdr_tex);
    let uv = pos.xy / vec2<f32>(f32(dims.x), f32(dims.y));
    var color = textureSample(hdr_tex, tex_sampler, uv).rgb * params.exposure;
    if params.mode == 1u {
        color = aces(color);
    } else {
        color = reinhard(color);
    }
    // Gamma correction
    color = pow(color, vec3(1.0 / 2.2));
    return vec4(color, 1.0);
}
"#
    }

    /// Returns a list of enabled effect names and their WGSL shader source.
    pub fn process_effects(&self) -> Vec<(&'static str, &'static str)> {
        let mut out = Vec::new();
        for effect in &self.enabled_effects {
            match effect {
                PostProcessEffect::Bloom => {
                    out.push(("Bloom", Self::get_bloom_shader_wgsl()));
                }
                PostProcessEffect::Ssao => {
                    out.push(("Ssao", Self::get_ssao_shader_wgsl()));
                }
                PostProcessEffect::MotionBlur => {
                    out.push(("MotionBlur", Self::get_motion_blur_shader_wgsl()));
                }
                PostProcessEffect::Fxaa => {
                    if self.fxaa_enabled {
                        out.push(("Fxaa", Self::get_fxaa_shader_wgsl()));
                    }
                }
                PostProcessEffect::Taa => {
                    if self.taa_enabled {
                        out.push(("Taa", Self::get_taa_shader_wgsl()));
                    }
                }
                PostProcessEffect::ToneMapping => {
                    if self.tone_mapping_enabled {
                        out.push(("ToneMapping", Self::get_tonemap_shader_wgsl()));
                    }
                }
            }
        }
        out
    }

    pub fn get_motion_blur_shader_wgsl() -> &'static str {
        r#"
@group(0) @binding(0) var color_tex: texture_2d<f32>;
@group(0) @binding(1) var vel_tex: texture_2d<f32>;
@group(0) @binding(2) var tex_sampler: sampler;

struct MotionBlurParams {
    shutter_speed: f32,
    max_velocity: f32,
    sample_count: f32,
}
@group(0) @binding(3) var<uniform> params: MotionBlurParams;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0)
    );
    return vec4(pos[idx], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(color_tex);
    let uv = pos.xy / vec2<f32>(f32(dims.x), f32(dims.y));
    let vel = textureSample(vel_tex, tex_sampler, uv).xy;
    let speed = length(vel) / params.max_velocity;
    let samples = u32(params.sample_count);
    var color = vec4<f32>(0.0);
    for (var i: u32; i < samples; i = i + 1u) {
        let t = (f32(i) + 0.5) / f32(samples);
        let offset = vel * params.shutter_speed * (t - 0.5);
        color = color + textureSample(color_tex, tex_sampler, uv - offset);
    }
    return color / f32(samples);
}
"#
    }

    pub fn get_taa_shader_wgsl() -> &'static str {
        r#"
@group(0) @binding(0) var current_tex: texture_2d<f32>;
@group(0) @binding(1) var history_tex: texture_2d<f32>;
@group(0) @binding(2) var tex_sampler: sampler;

struct TaaParams {
    blend_factor: f32,
    sharpen: f32,
}
@group(0) @binding(3) var<uniform> params: TaaParams;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0)
    );
    return vec4(pos[idx], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(current_tex);
    let uv = pos.xy / vec2<f32>(f32(dims.x), f32(dims.y));
    let current = textureSample(current_tex, tex_sampler, uv);
    let history = textureSample(history_tex, tex_sampler, uv);
    let min_luma = min(current.rgb, history.rgb);
    let max_luma = max(current.rgb, history.rgb);
    let clipped = clamp(history.rgb, min_luma, max_luma);
    return vec4(mix(current.rgb, clipped, params.blend_factor), 1.0);
}
"#
    }
}
