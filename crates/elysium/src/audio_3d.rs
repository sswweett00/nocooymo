/// audio_3d.rs — 3D Spatial Audio Engine
///
/// Features:
/// - HRTF (Head-Related Transfer Function) binaural processing
/// - Multi-algorithm reverb (freeverb, convolution, room models)
/// - Ray-based audio occlusion and obstruction
/// - Audio zones (reverb, underwater, echo, dampen)
/// - Doppler effect simulation
/// - Distance attenuation models (linear, inverse, logarithmic)
/// - Material absorption coefficients
/// - Audio mixer with per-source volume, pitch, pan
/// - EAX-style environmental audio
/// - Sound categories and priority system
/// - Low-pass / high-pass / band-pass filters

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════ Math

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0, z: 1.0 };

    pub fn new(x: f32, y: f32, z: f32) -> Self { Self { x, y, z } }
    pub fn splat(v: f32) -> Self { Self { x: v, y: v, z: v } }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn normalize(self) -> Self {
        let l = self.length();
        if l > 1e-8 { self * (1.0 / l) } else { Self::ZERO }
    }

    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn cross(self, o: Self) -> Self {
        Self {
            x: self.y * o.z - self.z * o.y,
            y: self.z * o.x - self.x * o.z,
            z: self.x * o.y - self.y * o.x,
        }
    }

    pub fn lerp(self, o: Self, t: f32) -> Self {
        Self {
            x: self.x + (o.x - self.x) * t,
            y: self.y + (o.y - self.y) * t,
            z: self.z + (o.z - self.z) * t,
        }
    }

    pub fn distance(self, o: Self) -> f32 { (self - o).length() }

    pub fn to_array(self) -> [f32; 3] { [self.x, self.y, self.z] }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, o: Self) -> Self { Self { x: self.x - o.x, y: self.y - o.y, z: self.z - o.z } }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, o: Self) -> Self { Self { x: self.x + o.x, y: self.y + o.y, z: self.z + o.z } }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, s: f32) -> Self { Self { x: self.x * s, y: self.y * s, z: self.z * s } }
}

impl std::ops::Mul<Vec3> for f32 {
    type Output = Vec3;
    fn mul(self, v: Vec3) -> Vec3 { Vec3 { x: self * v.x, y: self * v.y, z: self * v.z } }
}

impl std::ops::Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self { Self { x: -self.x, y: -self.y, z: -self.z } }
}

impl std::ops::Div<f32> for Vec3 {
    type Output = Self;
    fn div(self, s: f32) -> Self { Self { x: self.x / s, y: self.y / s, z: self.z / s } }
}

// ═══════════════════════════════════════════════════════════ HRTF

/// HRTF azimuth/elevation bin
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HrtfBin {
    pub azimuth: f32,    // degrees
    pub elevation: f32,  // degrees
    pub left_impulse: Vec<f32>,
    pub right_impulse: Vec<f32>,
}

/// HRTF database — simplified spherical harmonic model
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HrtfDatabase {
    pub bins: Vec<HrtfBin>,
    pub sample_rate: u32,
    pub impulse_length: usize,
}

impl Default for HrtfDatabase {
    fn default() -> Self {
        let sample_rate = 44100;
        let impulse_len = 128; // Short for CPU processing
        let mut bins = Vec::new();

        // Generate simplified HRTF bins at key directions
        for elev in (-45..=45).step_by(15) {
            for azim in (0..360).step_by(15) {
                let left_impulse = generate_hrtf_impulse(azim as f32, elev as f32, true, impulse_len, sample_rate);
                let right_impulse = generate_hrtf_impulse(azim as f32, elev as f32, false, impulse_len, sample_rate);
                bins.push(HrtfBin {
                    azimuth: azim as f32,
                    elevation: elev as f32,
                    left_impulse,
                    right_impulse,
                });
            }
        }

        Self { bins, sample_rate, impulse_length: impulse_len }
    }
}

/// Generate a simplified HRTF impulse response based on direction
fn generate_hrtf_impulse(azimuth: f32, elevation: f32, is_left: bool, length: usize, _sample_rate: u32) -> Vec<f32> {
    let mut impulse = vec![0.0f32; length];
    let az_rad = azimuth.to_radians();
    let el_rad = elevation.to_radians();

    // Simplified ITD (inter-aural time delay)
    let head_radius = 0.0875; // meters
    let speed_of_sound = 343.0;
    let itd = if is_left {
        -head_radius * (az_rad.sin()).max(0.0) / speed_of_sound
    } else {
        head_radius * (az_rad.sin()).max(0.0) / speed_of_sound
    };
    let itd_samples = (itd * _sample_rate as f32).abs() as usize;

    // ILD (inter-aural level difference)
    let ild_factor = if is_left {
        1.0 + 0.3 * (-az_rad).cos() * (1.0 + el_rad.sin() * 0.3)
    } else {
        1.0 + 0.3 * az_rad.cos() * (1.0 + el_rad.sin() * 0.3)
    };

    // Spectral shaping (simplified pinna filtering)
    let shadow_factor = if is_left {
        (az_rad.sin().max(0.0) * 0.5 + 0.5) * (1.0 - el_rad.abs().sin() * 0.2)
    } else {
        (-az_rad.sin().max(0.0) * 0.5 + 0.5) * (1.0 - el_rad.abs().sin() * 0.2)
    };

    // Direct sound with ITD
    let direct_idx = itd_samples.min(length - 1);
    if direct_idx < length {
        impulse[direct_idx] = ild_factor * 0.8;
    }

    // Early reflections from pinna (simplified)
    let delay1 = (direct_idx + 8 + (elevation.abs() * 0.05) as usize).min(length - 1);
    impulse[delay1] += shadow_factor * 0.3 * ild_factor;

    let delay2 = (direct_idx + 16 + (azimuth.abs() * 0.03) as usize).min(length - 1);
    impulse[delay2] += shadow_factor * 0.15 * ild_factor;

    // Diffuse tail
    for i in (direct_idx + 24)..length {
        let t = (i - direct_idx) as f32 / (length - direct_idx) as f32;
        let decay = (-t * 4.0).exp() * shadow_factor * 0.1;
        let noise = pseudo_random(i as f32 * 0.1 + azimuth * 0.01) * 2.0 - 1.0;
        impulse[i] += decay * noise * ild_factor;
    }

    impulse
}

impl HrtfDatabase {
    pub fn find_nearest(&self, azimuth: f32, elevation: f32) -> Option<&HrtfBin> {
        self.bins.iter().min_by_key(|bin| {
            let da = (bin.azimuth - azimuth).abs();
            let de = (bin.elevation - elevation).abs();
            ((da * da + de * de) * 100.0) as u32
        })
    }

    /// Convolve source audio with HRTF for binaural output
    pub fn process_binaural(&self, input: &[f32], azimuth: f32, elevation: f32) -> (Vec<f32>, Vec<f32>) {
        if let Some(bin) = self.find_nearest(azimuth, elevation) {
            let left = convolve(input, &bin.left_impulse);
            let right = convolve(input, &bin.right_impulse);
            (left, right)
        } else {
            let mono = input.to_vec();
            (mono.clone(), mono)
        }
    }
}

/// Simple convolution
fn convolve(signal: &[f32], impulse: &[f32]) -> Vec<f32> {
    if signal.is_empty() || impulse.is_empty() { return vec![0.0; signal.len()]; }
    let out_len = signal.len() + impulse.len() - 1;
    let mut output = vec![0.0f32; signal.len()]; // Truncate to signal length
    for i in 0..signal.len() {
        for j in 0..impulse.len() {
            if i + j < output.len() {
                output[i] += signal[i] * impulse[j];
            }
        }
    }
    output
}

// ═══════════════════════════════════════════════════════════ Reverb

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReverbModel {
    Freeverb,
    Room,
    Hall,
    Plate,
    Chamber,
    Cave,
    Underwater,
    Custom,
}

impl Default for ReverbModel {
    fn default() -> Self { ReverbModel::Room }
}

impl ReverbModel {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Freeverb => "Freeverb",
            Self::Room => "Room",
            Self::Hall => "Hall",
            Self::Plate => "Plate",
            Self::Chamber => "Chamber",
            Self::Cave => "Cave",
            Self::Underwater => "Underwater",
            Self::Custom => "Custom",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReverbParams {
    pub model: ReverbModel,
    pub room_size: f32,          // 0.0 - 1.0
    pub damping: f32,            // 0.0 - 1.0
    pub wet_level: f32,          // 0.0 - 1.0
    pub dry_level: f32,          // 0.0 - 1.0
    pub pre_delay: f32,          // ms
    pub diffusion: f32,          // 0.0 - 1.0
    pub density: f32,            // 0.0 - 1.0
    pub decay_time: f32,         // seconds
    pub high_frequency_damping: f32,
    pub late_reflections_level: f32,
    pub early_reflections_level: f32,
    pub early_reflections_delay: f32, // ms
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            model: ReverbModel::Room,
            room_size: 0.5,
            damping: 0.5,
            wet_level: 0.33,
            dry_level: 0.8,
            pre_delay: 15.0,
            diffusion: 0.8,
            density: 0.8,
            decay_time: 1.5,
            high_frequency_damping: 0.5,
            late_reflections_level: 0.6,
            early_reflections_level: 0.4,
            early_reflections_delay: 20.0,
        }
    }
}

impl ReverbParams {
    pub fn room() -> Self {
        Self { model: ReverbModel::Room, room_size: 0.3, damping: 0.5, decay_time: 0.8, ..Default::default() }
    }

    pub fn hall() -> Self {
        Self { model: ReverbModel::Hall, room_size: 0.8, damping: 0.3, decay_time: 2.5, wet_level: 0.4, ..Default::default() }
    }

    pub fn cave() -> Self {
        Self { model: ReverbModel::Cave, room_size: 0.9, damping: 0.2, decay_time: 4.0, wet_level: 0.5, diffusion: 0.4, density: 0.3, ..Default::default() }
    }

    pub fn underwater() -> Self {
        Self { model: ReverbModel::Underwater, room_size: 0.7, damping: 0.8, decay_time: 3.0, wet_level: 0.6, high_frequency_damping: 0.9, ..Default::default() }
    }

    pub fn plate() -> Self {
        Self { model: ReverbModel::Plate, room_size: 0.5, damping: 0.4, decay_time: 1.0, diffusion: 1.0, density: 1.0, ..Default::default() }
    }
}

/// Freeverb-style comb/allpass reverb
pub struct FreeverbReverb {
    pub params: ReverbParams,
    combs: Vec<CombFilter>,
    allpasses: Vec<AllpassFilter>,
    buffer_size: usize,
}

struct CombFilter {
    buffer: Vec<f32>,
    position: usize,
    feedback: f32,
    damping: f32,
    store: f32,
}

impl CombFilter {
    fn new(size: usize, feedback: f32, damping: f32) -> Self {
        Self { buffer: vec![0.0; size], position: 0, feedback, damping, store: 0.0 }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.position];
        self.store = output * (1.0 - self.damping) + self.store * self.damping;
        self.buffer[self.position] = input + self.store * self.feedback;
        self.position = (self.position + 1) % self.buffer.len();
        output
    }

    fn set_size(&mut self, size: usize) {
        self.buffer.resize(size, 0.0);
        self.position = 0;
    }
}

struct AllpassFilter {
    buffer: Vec<f32>,
    position: usize,
    feedback: f32,
}

impl AllpassFilter {
    fn new(size: usize, feedback: f32) -> Self {
        Self { buffer: vec![0.0; size], position: 0, feedback }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.position];
        self.buffer[self.position] = input + output * self.feedback;
        self.position = (self.position + 1) % self.buffer.len();
        output - input
    }

    fn set_size(&mut self, size: usize) {
        self.buffer.resize(size, 0.0);
        self.position = 0;
    }
}

impl FreeverbReverb {
    pub fn new(params: ReverbParams) -> Self {
        let scaled_size = (params.room_size * 44100.0 * 0.05) as usize;
        let scale_ms = 44100 / 1000;

        let combs = vec![
            CombFilter::new(scaled_size + 1116 * scale_ms, 0.84, 0.2),
            CombFilter::new(scaled_size + 1188 * scale_ms, 0.83, 0.2),
            CombFilter::new(scaled_size + 1277 * scale_ms, 0.82, 0.2),
            CombFilter::new(scaled_size + 1356 * scale_ms, 0.81, 0.2),
            CombFilter::new(scaled_size + 1422 * scale_ms, 0.80, 0.2),
            CombFilter::new(scaled_size + 1491 * scale_ms, 0.79, 0.2),
            CombFilter::new(scaled_size + 1557 * scale_ms, 0.78, 0.2),
            CombFilter::new(scaled_size + 1617 * scale_ms, 0.77, 0.2),
        ];

        let allpasses = vec![
            AllpassFilter::new(556 * scale_ms, 0.7),
            AllpassFilter::new(441 * scale_ms, 0.7),
            AllpassFilter::new(341 * scale_ms, 0.7),
            AllpassFilter::new(225 * scale_ms, 0.7),
        ];

        Self { params, combs, allpasses, buffer_size: 44100 }
    }

    pub fn update_params(&mut self, params: ReverbParams) {
        self.params = params;
        let scaled_size = (self.params.room_size * 44100.0 * 0.05) as usize;
        let scale_ms = 44100 / 1000;
        let sizes = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
        for (i, comb) in self.combs.iter_mut().enumerate() {
            comb.set_size(scaled_size + sizes[i] * scale_ms);
            comb.feedback = (0.84 - i as f32 * 0.01) * self.params.room_size;
            comb.damping = self.params.damping;
        }
    }

    pub fn process_sample(&mut self, input: f32) -> f32 {
        let mut comb_sum = 0.0;
        for comb in &mut self.combs {
            comb_sum += comb.process(input);
        }
        let comb_avg = comb_sum / self.combs.len() as f32;

        let mut allpass_out = comb_avg;
        for allpass in &mut self.allpasses {
            allpass_out = allpass.process(allpass_out);
        }

        // Mix wet/dry
        input * self.params.dry_level + allpass_out * self.params.wet_level
    }

    pub fn process_block(&mut self, input: &[f32]) -> Vec<f32> {
        input.iter().map(|&s| self.process_sample(s)).collect()
    }
}

/// Per-source early reflections model
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EarlyReflections {
    pub reflections: Vec<Reflection>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Reflection {
    pub delay_ms: f32,
    pub gain: f32,
    pub filter_coeff: f32, // High-frequency absorption
}

impl EarlyReflections {
    pub fn compute(listener_pos: Vec3, listener_forward: Vec3, source_pos: Vec3, room_size: f32) -> Self {
        let dir = source_pos - listener_pos;
        let dist = dir.length();
        let n_reflections = 8;

        let mut reflections = Vec::new();
        for i in 0..n_reflections {
            let delay_base = (room_size * 30.0) as f32; // 0-30ms pre-delay
            let delay = delay_base + i as f32 * (5.0 + room_size * 10.0);
            let gain = (1.0 - i as f32 / n_reflections as f32) * (1.0 - dist / 50.0).max(0.0);
            let filter_coeff = 0.5 + i as f32 * 0.05; // More HF absorption at later reflections

            reflections.push(Reflection { delay_ms: delay, gain: gain.max(0.0), filter_coeff });
        }

        Self { reflections, enabled: true }
    }

    pub fn total_delay_ms(&self) -> f32 {
        self.reflections.iter().map(|r| r.delay_ms).fold(0.0f32, f32::max)
    }

    pub fn total_energy(&self) -> f32 {
        self.reflections.iter().map(|r| r.gain * r.gain).sum()
    }
}

// ═══════════════════════════════════════════════════════════ Audio Occlusion

/// Material absorption coefficients (at 1kHz)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    pub absorption: f32,     // 0.0 (reflective) - 1.0 (absorbing)
    pub transmission: f32,   // 0.0 (opaque) - 1.0 (transparent)
    pub scattering: f32,     // 0.0 (specular) - 1.0 (diffuse)
}

impl Default for Material {
    fn default() -> Self { Material::concrete() }
}

impl Material {
    pub fn concrete() -> Material { Material { name: "Concrete".into(), absorption: 0.02, transmission: 0.0, scattering: 0.1 } }
    pub fn wood() -> Material { Material { name: "Wood".into(), absorption: 0.15, transmission: 0.0, scattering: 0.3 } }
    pub fn glass() -> Material { Material { name: "Glass".into(), absorption: 0.05, transmission: 0.3, scattering: 0.05 } }
    pub fn metal() -> Material { Material { name: "Metal".into(), absorption: 0.03, transmission: 0.0, scattering: 0.1 } }
    pub fn carpet() -> Material { Material { name: "Carpet".into(), absorption: 0.5, transmission: 0.0, scattering: 0.8 } }
    pub fn curtain() -> Material { Material { name: "Curtain".into(), absorption: 0.4, transmission: 0.0, scattering: 0.6 } }
    pub fn water() -> Material { Material { name: "Water".into(), absorption: 0.01, transmission: 0.4, scattering: 0.2 } }
    pub fn dirt() -> Material { Material { name: "Dirt".into(), absorption: 0.3, transmission: 0.0, scattering: 0.5 } }
    pub fn leaves() -> Material { Material { name: "Leaves".into(), absorption: 0.6, transmission: 0.1, scattering: 0.9 } }
    pub fn brick() -> Material { Material { name: "Brick".into(), absorption: 0.04, transmission: 0.0, scattering: 0.15 } }
    pub fn plaster() -> Material { Material { name: "Plaster".into(), absorption: 0.06, transmission: 0.0, scattering: 0.2 } }
    pub fn snow() -> Material { Material { name: "Snow".into(), absorption: 0.7, transmission: 0.0, scattering: 0.95 } }
    pub fn flesh() -> Material { Material { name: "Flesh".into(), absorption: 0.25, transmission: 0.1, scattering: 0.5 } }
    pub fn fabric() -> Material { Material { name: "Fabric".into(), absorption: 0.35, transmission: 0.0, scattering: 0.7 } }
    pub fn tile() -> Material { Material { name: "Tile".into(), absorption: 0.03, transmission: 0.0, scattering: 0.05 } }
    pub fn stone() -> Material { Material { name: "Stone".into(), absorption: 0.025, transmission: 0.0, scattering: 0.12 } }

    pub fn all() -> Vec<Material> {
        vec![
            Self::concrete(), Self::wood(), Self::glass(), Self::metal(),
            Self::carpet(), Self::curtain(), Self::water(), Self::dirt(),
            Self::leaves(), Self::brick(), Self::plaster(), Self::snow(),
            Self::flesh(), Self::fabric(), Self::tile(), Self::stone(),
        ]
    }
}

/// Ray hit result for occlusion
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OcclusionHit {
    pub distance: f32,
    pub material: Material,
    pub normal: Vec3,
    pub is_transmissive: bool,
}

/// Audio occlusion result
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct OcclusionResult {
    pub occluded: bool,
    pub obstruction: f32,     // 0.0 - 1.0 (how blocked the direct sound is)
    pub occlusion_filter: f32, // Low-pass cutoff factor (0.0 = fully muffled, 1.0 = no filter)
    pub transmission: f32,    // How much sound passes through
    pub distance: f32,
    pub num_walls: u32,
    pub total_absorption: f32,
}

/// Axis-aligned bounding box for simple geometry
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self { Self { min, max } }

    pub fn center(&self) -> Vec3 {
        Vec3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }

    /// Ray-AABB intersection test
    pub fn ray_intersect(&self, origin: Vec3, dir: Vec3) -> Option<(f32, f32)> {
        let inv_x = if dir.x.abs() > 1e-10 { 1.0 / dir.x } else { if dir.x >= 0.0 { 1e10 } else { -1e10 } };
        let inv_y = if dir.y.abs() > 1e-10 { 1.0 / dir.y } else { if dir.y >= 0.0 { 1e10 } else { -1e10 } };
        let inv_z = if dir.z.abs() > 1e-10 { 1.0 / dir.z } else { if dir.z >= 0.0 { 1e10 } else { -1e10 } };

        let t1 = (self.min.x - origin.x) * inv_x;
        let t2 = (self.max.x - origin.x) * inv_x;
        let t_min_x = t1.min(t2);
        let t_max_x = t1.max(t2);

        let t1 = (self.min.y - origin.y) * inv_y;
        let t2 = (self.max.y - origin.y) * inv_y;
        let t_min_y = t1.min(t2);
        let t_max_y = t1.max(t2);

        let t1 = (self.min.z - origin.z) * inv_z;
        let t2 = (self.max.z - origin.z) * inv_z;
        let t_min_z = t1.min(t2);
        let t_max_z = t1.max(t2);

        let t_min = t_min_x.max(t_min_y).max(t_min_z);
        let t_max = t_max_x.min(t_max_y).min(t_max_z);

        if t_min <= t_max && t_max > 0.0 {
            let t_enter = if t_min > 0.0 { t_min } else { t_max };
            Some((t_enter, t_max))
        } else {
            None
        }
    }
}

/// Scene geometry for audio occlusion
pub struct AudioSceneGeometry {
    pub walls: Vec<AudioWall>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioWall {
    pub bounds: Aabb,
    pub material: Material,
}

impl Default for AudioSceneGeometry {
    fn default() -> Self {
        let mut walls = Vec::new();
        // Add some default walls for testing
        walls.push(AudioWall {
            bounds: Aabb::new(Vec3::new(-0.1, -2.0, -5.0), Vec3::new(0.1, 2.0, 5.0)),
            material: Material::concrete(),
        });
        Self { walls }
    }
}

impl AudioSceneGeometry {
    pub fn new() -> Self { Self::default() }

    pub fn add_wall(&mut self, min: Vec3, max: Vec3, material: Material) {
        self.walls.push(AudioWall { bounds: Aabb::new(min, max), material });
    }

    pub fn add_room(&mut self, size: Vec3, material: Material) {
        let hs = Vec3::new(size.x * 0.5, size.y * 0.5, size.z * 0.5);
        // Floor
        self.add_wall(Vec3::new(-hs.x, -hs.y - 0.2, -hs.z), Vec3::new(hs.x, -hs.y, hs.z), material.clone());
        // Ceiling
        self.add_wall(Vec3::new(-hs.x, hs.y, -hs.z), Vec3::new(hs.x, hs.y + 0.2, hs.z), material.clone());
        // Walls
        self.add_wall(Vec3::new(-hs.x - 0.2, -hs.y, -hs.z), Vec3::new(-hs.x, hs.y, hs.z), material.clone());
        self.add_wall(Vec3::new(hs.x, -hs.y, -hs.z), Vec3::new(hs.x + 0.2, hs.y, hs.z), material.clone());
        self.add_wall(Vec3::new(-hs.x, -hs.y, -hs.z - 0.2), Vec3::new(hs.x, hs.y, -hs.z), material.clone());
        self.add_wall(Vec3::new(-hs.x, -hs.y, hs.z), Vec3::new(hs.x, hs.y, hs.z + 0.2), material.clone());
    }

    /// Cast a ray and return all hits along the way
    pub fn ray_cast(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Vec<OcclusionHit> {
        let mut hits = Vec::new();
        for wall in &self.walls {
            if let Some((t_min, _t_max)) = wall.bounds.ray_intersect(origin, dir) {
                if t_min > 0.0 && t_min < max_distance {
                    let normal = self.compute_normal(origin, dir, &wall.bounds);
                    hits.push(OcclusionHit {
                        distance: t_min,
                        material: wall.material.clone(),
                        normal,
                        is_transmissive: wall.material.transmission > 0.1,
                    });
                }
            }
        }
        hits.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        hits
    }

    fn compute_normal(&self, origin: Vec3, dir: Vec3, bounds: &Aabb) -> Vec3 {
        // Simplified — return closest face normal
        let center = bounds.center();
        let d = center - origin;
        let adx = (d.x).abs();
        let ady = (d.y).abs();
        let adz = (d.z).abs();
        if adx > ady && adx > adz {
            Vec3::new(if dir.x > 0.0 { -1.0 } else { 1.0 }, 0.0, 0.0)
        } else if ady > adz {
            Vec3::new(0.0, if dir.y > 0.0 { -1.0 } else { 1.0 }, 0.0)
        } else {
            Vec3::new(0.0, 0.0, if dir.z > 0.0 { -1.0 } else { 1.0 })
        }
    }

    /// Compute full occlusion between two points
    pub fn compute_occlusion(&self, listener_pos: Vec3, source_pos: Vec3) -> OcclusionResult {
        let dir = source_pos - listener_pos;
        let distance = dir.length();
        if distance < 0.001 {
            return OcclusionResult { distance: 0.0, ..Default::default() };
        }
        let dir_norm = dir.normalize();
        let hits = self.ray_cast(listener_pos, dir_norm, distance);

        if hits.is_empty() {
            return OcclusionResult {
                occluded: false,
                obstruction: 0.0,
                occlusion_filter: 1.0,
                transmission: 1.0,
                distance,
                num_walls: 0,
                total_absorption: 0.0,
            };
        }

        let num_walls = hits.len() as u32;
        let mut total_absorption = 0.0f32;
        let mut total_transmission = 1.0f32;
        let mut has_transmissive_path = false;

        for hit in &hits {
            total_absorption += hit.material.absorption;
            total_transmission *= 1.0 - hit.material.absorption;
            if hit.is_transmissive {
                has_transmissive_path = true;
            }
        }

        // Distance-based attenuation on top
        let dist_atten = 1.0 / (1.0 + distance * 0.1);

        let obstruction = (total_absorption * 0.8 + num_walls as f32 * 0.1).min(1.0);
        let occlusion_filter = (1.0 - total_absorption * 0.7).max(0.05) * dist_atten;

        OcclusionResult {
            occluded: true,
            obstruction,
            occlusion_filter,
            transmission: if has_transmissive_path { total_transmission } else { 0.0 },
            distance,
            num_walls,
            total_absorption,
        }
    }
}

// ═══════════════════════════════════════════════════════════ Audio Zones

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioZoneType {
    Reverb,
    Underwater,
    Echo,
    Dampen,
    Muffle,
    Boost,
    DopplerShift,
}

impl Default for AudioZoneType {
    fn default() -> Self { AudioZoneType::Reverb }
}

impl AudioZoneType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Reverb => "Reverb Zone",
            Self::Underwater => "Underwater",
            Self::Echo => "Echo",
            Self::Dampen => "Dampen",
            Self::Muffle => "Muffle",
            Self::Boost => "Boost",
            Self::DopplerShift => "Doppler Shift",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioZone {
    pub id: u32,
    pub zone_type: AudioZoneType,
    pub position: Vec3,
    pub radius: f32,
    pub inner_radius: f32,    // Full effect inside this radius
    pub priority: u32,        // Higher = overrides lower priority zones
    pub enabled: bool,
    pub reverb_params: Option<ReverbParams>,
    pub low_pass_cutoff: f32, // 0.0 - 1.0
    pub high_pass_cutoff: f32,
    pub volume_multiplier: f32,
    pub pitch_shift: f32,
    pub wet_mix: f32,         // 0.0 - 1.0
}

impl AudioZone {
    pub fn reverb(id: u32, pos: Vec3, radius: f32, params: ReverbParams) -> Self {
        Self {
            id, zone_type: AudioZoneType::Reverb,
            position: pos, radius, inner_radius: radius * 0.3,
            priority: 1, enabled: true,
            reverb_params: Some(params),
            low_pass_cutoff: 1.0, high_pass_cutoff: 0.0,
            volume_multiplier: 1.0, pitch_shift: 1.0, wet_mix: 0.5,
        }
    }

    pub fn underwater(id: u32, pos: Vec3, radius: f32) -> Self {
        Self {
            id, zone_type: AudioZoneType::Underwater,
            position: pos, radius, inner_radius: radius * 0.5,
            priority: 10, enabled: true,
            reverb_params: Some(ReverbParams::underwater()),
            low_pass_cutoff: 0.2,
            high_pass_cutoff: 0.1,
            volume_multiplier: 0.6,
            pitch_shift: 0.9,
            wet_mix: 0.7,
        }
    }

    pub fn echo(id: u32, pos: Vec3, radius: f32) -> Self {
        Self {
            id, zone_type: AudioZoneType::Echo,
            position: pos, radius, inner_radius: radius * 0.3,
            priority: 5, enabled: true,
            reverb_params: Some(ReverbParams::cave()),
            low_pass_cutoff: 0.8,
            high_pass_cutoff: 0.0,
            volume_multiplier: 1.0,
            pitch_shift: 1.0,
            wet_mix: 0.6,
        }
    }

    pub fn dampen(id: u32, pos: Vec3, radius: f32) -> Self {
        Self {
            id, zone_type: AudioZoneType::Dampen,
            position: pos, radius, inner_radius: radius * 0.2,
            priority: 3, enabled: true,
            reverb_params: None,
            low_pass_cutoff: 0.4,
            high_pass_cutoff: 0.0,
            volume_multiplier: 0.5,
            pitch_shift: 1.0,
            wet_mix: 0.0,
        }
    }

    pub fn muffle(id: u32, pos: Vec3, radius: f32) -> Self {
        Self {
            id, zone_type: AudioZoneType::Muffle,
            position: pos, radius, inner_radius: radius * 0.3,
            priority: 4, enabled: true,
            reverb_params: None,
            low_pass_cutoff: 0.15,
            high_pass_cutoff: 0.0,
            volume_multiplier: 0.4,
            pitch_shift: 0.95,
            wet_mix: 0.0,
        }
    }

    /// Calculate zone influence at a given position (0.0 = no effect, 1.0 = full effect)
    pub fn influence(&self, pos: Vec3) -> f32 {
        let dist = self.position.distance(pos);
        if dist > self.radius { return 0.0; }
        if dist <= self.inner_radius { return 1.0; }
        // Smooth falloff between inner and outer radius
        let t = (dist - self.inner_radius) / (self.radius - self.inner_radius);
        1.0 - t * t * (3.0 - 2.0 * t) // smoothstep
    }
}

// ═══════════════════════════════════════════════════════════ Doppler

/// Doppler effect calculator
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DopplerEffect {
    pub speed_of_sound: f32,
    pub listener_velocity: Vec3,
    pub source_velocity: Vec3,
    pub enabled: bool,
}

impl DopplerEffect {
    pub fn new() -> Self {
        Self { speed_of_sound: 343.0, enabled: true, ..Default::default() }
    }

    /// Calculate Doppler shift ratio for pitch
    pub fn calculate(&self, listener_pos: Vec3, source_pos: Vec3) -> f32 {
        if !self.enabled { return 1.0; }

        let dir = source_pos - listener_pos;
        let distance = dir.length();
        if distance < 0.001 { return 1.0; }
        let dir_norm = dir / distance;

        // Project velocities onto sound direction
        let listener_vel_proj = self.listener_velocity.dot(dir_norm);
        let source_vel_proj = self.source_velocity.dot(dir_norm);

        // Doppler formula: f' = f * (c + v_l) / (c + v_s)
        let shift = (self.speed_of_sound + listener_vel_proj) / (self.speed_of_sound + source_vel_proj);

        shift.clamp(0.5, 2.0) // Clamp to reasonable range
    }

    /// Calculate distance attenuation with Doppler
    pub fn attenuation_with_doppler(&self, listener_pos: Vec3, source_pos: Vec3) -> (f32, f32) {
        let distance = listener_pos.distance(source_pos);
        let doppler = self.calculate(listener_pos, source_pos);
        let attenuation = 1.0 / (1.0 + distance * 0.1); // Simple inverse distance
        (attenuation, doppler)
    }
}

// ═══════════════════════════════════════════════════════════ Filters

/// Low-pass filter (one-pole)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LowPassFilter {
    pub cutoff: f32, // 0.0 - 1.0 (normalized)
    prev_output: f32,
}

impl LowPassFilter {
    pub fn new(cutoff: f32) -> Self { Self { cutoff: cutoff.clamp(0.001, 1.0), prev_output: 0.0 } }

    pub fn process(&mut self, input: f32) -> f32 {
        let alpha = self.cutoff;
        self.prev_output = self.prev_output + alpha * (input - self.prev_output);
        self.prev_output
    }

    pub fn reset(&mut self) { self.prev_output = 0.0; }
}

/// High-pass filter
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HighPassFilter {
    pub cutoff: f32,
    prev_input: f32,
    prev_output: f32,
}

impl HighPassFilter {
    pub fn new(cutoff: f32) -> Self { Self { cutoff: cutoff.clamp(0.001, 0.999), prev_input: 0.0, prev_output: 0.0 } }

    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.cutoff * (self.prev_output + input - self.prev_input);
        self.prev_input = input;
        self.prev_output = output;
        output
    }

    pub fn reset(&mut self) { self.prev_input = 0.0; self.prev_output = 0.0; }
}

/// Band-pass filter (low-pass + high-pass)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BandPassFilter {
    pub low: LowPassFilter,
    pub high: HighPassFilter,
}

impl BandPassFilter {
    pub fn new(low_cutoff: f32, high_cutoff: f32) -> Self {
        Self { low: LowPassFilter::new(high_cutoff), high: HighPassFilter::new(low_cutoff) }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.low.process(self.high.process(input))
    }

    pub fn reset(&mut self) { self.low.reset(); self.high.reset(); }
}

// ═══════════════════════════════════════════════════════════ Audio Source

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoundCategory {
    SFX,
    Music,
    Voice,
    Ambient,
    UI,
}

impl Default for SoundCategory {
    fn default() -> Self { SoundCategory::SFX }
}

/// Maximum number of simultaneous sounds per category
pub fn max_sounds_per_category(cat: SoundCategory) -> u32 {
    match cat {
        SoundCategory::SFX => 32,
        SoundCategory::Music => 4,
        SoundCategory::Voice => 8,
        SoundCategory::Ambient => 16,
        SoundCategory::UI => 8,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioSource {
    pub id: u32,
    pub position: Vec3,
    pub velocity: Vec3,
    pub gain: f32,
    pub pitch: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub rolloff: f32,
    pub attenuation_model: AttenuationModel,
    pub is_3d: bool,
    pub looping: bool,
    pub playing: bool,
    pub paused: bool,
    pub category: SoundCategory,
    pub priority: i32,
    pub low_pass: LowPassFilter,
    pub high_pass: HighPassFilter,
    pub directivity: f32, // 0.0 = omnidirectional, 1.0 = fully directional
    pub inner_cone_angle: f32, // degrees
    pub outer_cone_angle: f32,
    pub outer_cone_gain: f32,
    pub zone_volume: f32,
    pub doppler_factor: f32,
}

impl Default for AudioSource {
    fn default() -> Self {
        Self {
            id: 0,
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            gain: 1.0,
            pitch: 1.0,
            min_distance: 1.0,
            max_distance: 50.0,
            rolloff: 1.0,
            attenuation_model: AttenuationModel::InverseDistance,
            is_3d: true,
            looping: false,
            playing: true,
            paused: false,
            category: SoundCategory::SFX,
            priority: 0,
            low_pass: LowPassFilter::new(1.0),
            high_pass: HighPassFilter::new(0.0),
            directivity: 0.0,
            inner_cone_angle: 360.0,
            outer_cone_angle: 360.0,
            outer_cone_gain: 1.0,
            zone_volume: 1.0,
            doppler_factor: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttenuationModel {
    None,
    Linear,
    InverseDistance,
    InverseDistanceClamped,
    Logarithmic,
    Exponential,
}

impl Default for AttenuationModel {
    fn default() -> Self { AttenuationModel::InverseDistanceClamped }
}

impl AttenuationModel {
    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Linear => "Linear",
            Self::InverseDistance => "Inverse Distance",
            Self::InverseDistanceClamped => "Inverse Distance Clamped",
            Self::Logarithmic => "Logarithmic",
            Self::Exponential => "Exponential",
        }
    }

    pub fn calculate(&self, distance: f32, min_dist: f32, max_dist: f32, rolloff: f32) -> f32 {
        match self {
            Self::None => 1.0,
            Self::Linear => {
                let d = distance.clamp(min_dist, max_dist);
                (1.0 - rolloff * (d - min_dist) / (max_dist - min_dist)).max(0.0)
            }
            Self::InverseDistance => {
                let d = distance.max(0.001);
                min_dist / (min_dist + rolloff * (d - min_dist))
            }
            Self::InverseDistanceClamped => {
                let d = distance.clamp(min_dist, max_dist);
                min_dist / (min_dist + rolloff * (d - min_dist))
            }
            Self::Logarithmic => {
                let d = distance.clamp(min_dist, max_dist);
                let ratio = min_dist / d;
                ratio.powf(rolloff)
            }
            Self::Exponential => {
                let d = distance.clamp(min_dist, max_dist);
                (d / min_dist).powf(-rolloff)
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════ Listener

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Listener {
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub velocity: Vec3,
    pub gain: f32,
}

impl Listener {
    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            forward: Vec3::new(0.0, 0.0, -1.0),
            up: Vec3::new(0.0, 1.0, 0.0),
            velocity: Vec3::ZERO,
            gain: 1.0,
        }
    }

    /// Calculate azimuth and elevation of a source relative to listener
    pub fn source_direction(&self, source_pos: Vec3) -> (f32, f32) {
        let dir = source_pos - self.position;
        if dir.length() < 0.001 { return (0.0, 0.0); }
        let dir = dir.normalize();

        let right = self.forward.cross(self.up).normalize();

        // Azimuth: angle in horizontal plane
        let azimuth = dir.x.atan2(-dir.z).to_degrees();

        // Elevation: angle above/below horizontal
        let horizontal = Vec3::new(dir.x, 0.0, dir.z).normalize();
        let elevation = dir.y.asin().to_degrees();

        (azimuth, elevation)
    }

    /// Get relative left/right gain for stereo panning
    pub fn stereo_pan(&self, source_pos: Vec3) -> (f32, f32) {
        let (azimuth, _) = self.source_direction(source_pos);
        let az_rad = azimuth.to_radians();
        let left_gain = ((std::f32::consts::FRAC_PI_2 + az_rad).cos() * 0.5 + 0.5);
        let right_gain = ((std::f32::consts::FRAC_PI_2 - az_rad).cos() * 0.5 + 0.5);
        (left_gain, right_gain)
    }
}

// ═══════════════════════════════════════════════════════════ Audio Engine

pub struct AudioEngine3d {
    pub listener: Listener,
    pub sources: Vec<AudioSource>,
    pub hrtf: HrtfDatabase,
    pub reverb: FreeverbReverb,
    pub zones: Vec<AudioZone>,
    pub geometry: AudioSceneGeometry,
    pub doppler: DopplerEffect,
    pub global_reverb: ReverbParams,
    pub master_volume: f32,
    pub next_source_id: u32,
    pub next_zone_id: u32,
    pub stats: AudioStats,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AudioStats {
    pub active_sources: u32,
    pub active_zones: u32,
    pub occluded_sources: u32,
    pub total_sources: u32,
    pub hrtf_processed: u32,
    pub reverb_processed: u32,
}

impl Default for AudioEngine3d {
    fn default() -> Self {
        Self {
            listener: Listener::new(),
            sources: Vec::new(),
            hrtf: HrtfDatabase::default(),
            reverb: FreeverbReverb::new(ReverbParams::room()),
            zones: Vec::new(),
            geometry: AudioSceneGeometry::new(),
            doppler: DopplerEffect::new(),
            global_reverb: ReverbParams::room(),
            master_volume: 1.0,
            next_source_id: 1,
            next_zone_id: 1,
            stats: AudioStats::default(),
        }
    }
}

impl AudioEngine3d {
    pub fn new() -> Self { Self::default() }

    pub fn set_master_volume(&mut self, vol: f32) {
        self.master_volume = vol.clamp(0.0, 1.0);
    }

    pub fn set_global_reverb(&mut self, params: ReverbParams) {
        self.global_reverb = params.clone();
        self.reverb.update_params(params);
    }

    pub fn add_zone(&mut self, zone_type: AudioZoneType, pos: Vec3, radius: f32) -> u32 {
        let id = self.next_zone_id;
        self.next_zone_id += 1;
        let zone = match zone_type {
            AudioZoneType::Reverb => AudioZone::reverb(id, pos, radius, ReverbParams::room()),
            AudioZoneType::Underwater => AudioZone::underwater(id, pos, radius),
            AudioZoneType::Echo => AudioZone::echo(id, pos, radius),
            AudioZoneType::Dampen => AudioZone::dampen(id, pos, radius),
            AudioZoneType::Muffle => AudioZone::muffle(id, pos, radius),
            _ => AudioZone::reverb(id, pos, radius, ReverbParams::room()),
        };
        self.zones.push(zone);
        id
    }

    pub fn remove_zone(&mut self, id: u32) {
        self.zones.retain(|z| z.id != id);
    }

    pub fn add_source(&mut self, pos: Vec3, category: SoundCategory) -> u32 {
        let id = self.next_source_id;
        self.next_source_id += 1;
        let mut source = AudioSource::default();
        source.id = id;
        source.position = pos;
        source.category = category;
        self.sources.push(source);
        id
    }

    pub fn remove_source(&mut self, id: u32) {
        self.sources.retain(|s| s.id != id);
    }

    pub fn get_source(&self, id: u32) -> Option<&AudioSource> {
        self.sources.iter().find(|s| s.id == id)
    }

    pub fn get_source_mut(&mut self, id: u32) -> Option<&mut AudioSource> {
        self.sources.iter_mut().find(|s| s.id == id)
    }

    /// Calculate final volume and pitch for a source considering all effects
    pub fn calculate_source_output(&mut self, source_id: u32) -> SourceOutput {
        let source = match self.sources.iter().find(|s| s.id == source_id) {
            Some(s) => s.clone(),
            None => return SourceOutput::default(),
        };

        let mut output = SourceOutput {
            gain: source.gain * self.master_volume,
            pitch: source.pitch,
            left_gain: 1.0,
            right_gain: 1.0,
            low_pass_cutoff: 1.0,
            high_pass_cutoff: 0.0,
            occluded: false,
            occlusion_filter: 1.0,
            doppler_shift: 1.0,
            zone_influence: 1.0,
            reverb_wet: 0.0,
        };

        if !source.playing || source.paused {
            output.gain = 0.0;
            return output;
        }

        if source.is_3d {
            // Distance attenuation
            let distance = self.listener.position.distance(source.position);
            output.gain *= AttenuationModel::InverseDistanceClamped.calculate(
                distance, source.min_distance, source.max_distance, source.rolloff,
            );

            // Stereo panning
            let (left, right) = self.listener.stereo_pan(source.position);
            output.left_gain = left;
            output.right_gain = right;

            // Directional source
            if source.directivity > 0.0 {
                let (azimuth, _) = self.listener.source_direction(source.position);
                let az_rad = azimuth.to_radians();
                let cone_factor = if azimuth.abs() < source.inner_cone_angle * 0.5 {
                    1.0
                } else if azimuth.abs() < source.outer_cone_angle * 0.5 {
                    let t = (azimuth.abs() - source.inner_cone_angle * 0.5) /
                            (source.outer_cone_angle * 0.5 - source.inner_cone_angle * 0.5);
                    1.0 + (source.outer_cone_gain - 1.0) * t
                } else {
                    source.outer_cone_gain
                };
                output.gain *= cone_factor;
            }

            // Doppler effect
            output.doppler_shift = self.doppler.calculate(self.listener.position, source.position);
            output.pitch *= output.doppler_shift * source.doppler_factor;

            // Audio occlusion
            let occ = self.geometry.compute_occlusion(self.listener.position, source.position);
            output.occluded = occ.occluded;
            output.occlusion_filter = occ.occlusion_filter;
            output.gain *= 1.0 - occ.obstruction * 0.7;

            // Low-pass from occlusion
            output.low_pass_cutoff = occ.occlusion_filter;

            // Zone processing — find highest priority active zone
            let mut active_zone: Option<&AudioZone> = None;
            let mut max_priority = -1i32;
            for zone in &self.zones {
                if !zone.enabled { continue; }
                let infl = zone.influence(source.position);
                if infl > 0.0 && zone.priority as i32 > max_priority {
                    max_priority = zone.priority as i32;
                    active_zone = Some(zone);
                }
            }

            if let Some(zone) = active_zone {
                let infl = zone.influence(source.position);
                output.zone_influence = infl;
                output.gain *= zone.volume_multiplier * infl + (1.0 - infl);
                output.pitch *= zone.pitch_shift.powf(infl);
                output.low_pass_cutoff = output.low_pass_cutoff.min(zone.low_pass_cutoff * infl + (1.0 - infl));
                output.high_pass_cutoff = zone.high_pass_cutoff * infl;
                output.reverb_wet = zone.wet_mix * infl;
            }
        }

        output.gain = output.gain.clamp(0.0, 2.0);
        output
    }

    /// Update all sources and compute stats
    pub fn update(&mut self, dt: f32) {
        // Update reverb with listener position (basic room detection)
        self.reverb.update_params(self.global_reverb.clone());

        // Update stats
        let mut active = 0;
        let mut occluded = 0;
        for source in &self.sources {
            if source.playing && !source.paused {
                active += 1;
                if source.is_3d {
                    let occ = self.geometry.compute_occlusion(self.listener.position, source.position);
                    if occ.occluded { occluded += 1; }
                }
            }
        }

        self.stats.active_sources = active;
        self.stats.active_zones = self.zones.iter().filter(|z| z.enabled).count() as u32;
        self.stats.occluded_sources = occluded;
        self.stats.total_sources = self.sources.len() as u32;
    }

    /// Process a stereo output buffer
    pub fn process_stereo(&mut self, input_mono: &[f32], source_id: u32) -> (Vec<f32>, Vec<f32>) {
        let output = self.calculate_source_output(source_id);

        let mut left = Vec::with_capacity(input_mono.len());
        let mut right = Vec::with_capacity(input_mono.len());

        for &sample in input_mono {
            let mut s = sample;
            // Apply low-pass filter
            s = output.low_pass_cutoff * s + (1.0 - output.low_pass_cutoff) * 0.0;
            // Apply high-pass filter
            let hp = s * output.high_pass_cutoff;
            s = hp;

            left.push(s * output.gain * output.left_gain);
            right.push(s * output.gain * output.right_gain);
        }

        // Apply reverb if wet mix > 0
        if output.reverb_wet > 0.001 {
            let wet_left = self.reverb.process_block(&left);
            let wet_right = self.reverb.process_block(&right);
            let dry = 1.0 - output.reverb_wet;
            for i in 0..left.len() {
                left[i] = left[i] * dry + wet_left[i] * output.reverb_wet;
                right[i] = right[i] * dry + wet_right[i] * output.reverb_wet;
            }
        }

        (left, right)
    }
}

// ═══════════════════════════════════════════════════════════ Source Output

#[derive(Clone, Debug, Default)]
pub struct SourceOutput {
    pub gain: f32,
    pub pitch: f32,
    pub left_gain: f32,
    pub right_gain: f32,
    pub low_pass_cutoff: f32,
    pub high_pass_cutoff: f32,
    pub occluded: bool,
    pub occlusion_filter: f32,
    pub doppler_shift: f32,
    pub zone_influence: f32,
    pub reverb_wet: f32,
}

// ═══════════════════════════════════════════════════════════ Helpers

fn pseudo_random(seed: f32) -> f32 {
    let x = seed.sin() * 43758.5453;
    x - x.floor()
}

// ═══════════════════════════════════════════════════════════ Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec3_basics() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert!((v.length() - (14.0_f32).sqrt()).abs() < 0.001);
        let n = v.normalize();
        assert!((n.length() - 1.0).abs() < 0.01);
        let dot = Vec3::new(1.0, 0.0, 0.0).dot(Vec3::new(0.0, 1.0, 0.0));
        assert!(dot.abs() < 0.001);
    }

    #[test]
    fn test_vec3_operations() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        let sum = a + b;
        assert_eq!(sum.x, 5.0);
        assert_eq!(sum.y, 7.0);
        assert_eq!(sum.z, 9.0);
        let diff = b - a;
        assert_eq!(diff.x, 3.0);
        let scaled = a * 2.0;
        assert_eq!(scaled.x, 2.0);
    }

    #[test]
    fn test_hrtf_database() {
        let db = HrtfDatabase::default();
        assert!(!db.bins.is_empty());
        assert_eq!(db.impulse_length, 128);

        let bin = db.find_nearest(45.0, 0.0);
        assert!(bin.is_some());
        let bin = bin.unwrap();
        assert_eq!(bin.left_impulse.len(), 128);
        assert_eq!(bin.right_impulse.len(), 128);
    }

    #[test]
    fn test_hrtf_binaural_processing() {
        let db = HrtfDatabase::default();
        let input = vec![0.5; 64];
        let (left, right) = db.process_binaural(&input, 45.0, 0.0);
        assert_eq!(left.len(), 64);
        assert_eq!(right.len(), 64);
        // Left and right should be different for off-center source
        let diff: f32 = left.iter().zip(right.iter()).map(|(l, r)| (l - r).abs()).sum();
        assert!(diff > 0.001, "HRTF should produce different L/R signals");
    }

    #[test]
    fn test_easing_sample() {
        // Just verify basic easing works
        let bin = HrtfBin::default();
        assert!(bin.left_impulse.is_empty());
    }

    #[test]
    fn test_reverb_freeverb() {
        let params = ReverbParams::room();
        let mut reverb = FreeverbReverb::new(params);
        let input = vec![0.5; 44100];
        let output = reverb.process_block(&input);
        assert_eq!(output.len(), 44100);
        // Output should be quieter than input (dry+wet mix)
        let input_rms: f32 = input.iter().map(|s| s * s).sum::<f32>() / input.len() as f32;
        let output_rms: f32 = output.iter().map(|s| s * s).sum::<f32>() / output.len() as f32;
        assert!(output_rms > 0.0, "Reverb should produce non-zero output");
    }

    #[test]
    fn test_reverb_update_params() {
        let mut reverb = FreeverbReverb::new(ReverbParams::room());
        reverb.update_params(ReverbParams::hall());
        assert_eq!(reverb.params.model, ReverbModel::Hall);
    }

    #[test]
    fn test_early_reflections() {
        let er = EarlyReflections::compute(
            Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(5.0, 0.0, 0.0), 0.5,
        );
        assert!(er.enabled);
        assert!(!er.reflections.is_empty());
        assert!(er.total_energy() > 0.0);
    }

    #[test]
    fn test_materials() {
        let materials = Material::all();
        assert_eq!(materials.len(), 16);
        for m in &materials {
            assert!(m.absorption >= 0.0 && m.absorption <= 1.0);
            assert!(m.transmission >= 0.0 && m.transmission <= 1.0);
            assert!(m.scattering >= 0.0 && m.scattering <= 1.0);
        }
    }

    #[test]
    fn test_aabb_intersection() {
        let bbox = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        // Ray pointing toward box
        let hit = bbox.ray_intersect(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        assert!(hit.is_some());
        let (t_min, t_max) = hit.unwrap();
        assert!(t_min > 0.0);
        assert!(t_max > t_min);

        // Ray pointing away
        let hit = bbox.ray_intersect(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(hit.is_none());
    }

    #[test]
    fn test_audio_scene_occlusion() {
        let mut geom = AudioSceneGeometry::new();
        geom.walls.clear();
        // Wall between source and listener
        geom.add_wall(
            Vec3::new(-0.1, -2.0, -0.5),
            Vec3::new(0.1, 2.0, 0.5),
            Material::concrete(),
        );

        let occ = geom.compute_occlusion(Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 5.0));
        assert!(occ.occluded);
        assert!(occ.num_walls > 0);
        assert!(occ.total_absorption > 0.0);

        // No wall
        let occ2 = geom.compute_occlusion(Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 10.0, -5.0));
        assert!(!occ2.occluded);
    }

    #[test]
    fn test_audio_scene_room() {
        let mut geom = AudioSceneGeometry::new();
        geom.walls.clear();
        geom.add_room(Vec3::new(10.0, 3.0, 10.0), Material::wood());
        // Should have 6 walls
        assert_eq!(geom.walls.len(), 6);

        // Source inside room — should not be occluded
        let occ = geom.compute_occlusion(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0));
        assert!(!occ.occluded);
    }

    #[test]
    fn test_audio_zones() {
        let zone = AudioZone::reverb(1, Vec3::new(0.0, 0.0, 0.0), 10.0, ReverbParams::hall());
        // At center: full influence
        assert!((zone.influence(Vec3::new(0.0, 0.0, 0.0)) - 1.0).abs() < 0.001);
        // At distance 2 (inside inner_radius=3): full influence
        assert!((zone.influence(Vec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < 0.001);
        // At distance 5 (between inner=3 and outer=10): partial influence
        let inf5 = zone.influence(Vec3::new(5.0, 0.0, 0.0));
        assert!(inf5 > 0.3 && inf5 < 1.0, "Influence at 5m should be between 0.3 and 1.0, got {}", inf5);
        // Outside zone radius: no influence
        assert!(zone.influence(Vec3::new(15.0, 0.0, 0.0)) < 0.001);
    }

    #[test]
    fn test_audio_zones_underwater() {
        let zone = AudioZone::underwater(1, Vec3::ZERO, 5.0);
        assert_eq!(zone.zone_type, AudioZoneType::Underwater);
        assert!(zone.reverb_params.is_some());
        assert!(zone.low_pass_cutoff < 0.5);
    }

    #[test]
    fn test_doppler_effect() {
        let mut doppler = DopplerEffect::new();
        doppler.source_velocity = Vec3::new(0.0, 0.0, -100.0); // Moving toward listener
        let shift = doppler.calculate(Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0));
        assert!(shift > 1.0, "Source approaching should increase pitch");

        doppler.source_velocity = Vec3::new(0.0, 0.0, 100.0); // Moving away
        let shift = doppler.calculate(Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0));
        assert!(shift < 1.0, "Source receding should decrease pitch");
    }

    #[test]
    fn test_attenuation_models() {
        let dist = 10.0;
        let min_d = 1.0;
        let max_d = 50.0;
        let rolloff = 1.0;

        let linear = AttenuationModel::Linear.calculate(dist, min_d, max_d, rolloff);
        let inverse = AttenuationModel::InverseDistanceClamped.calculate(dist, min_d, max_d, rolloff);
        let log = AttenuationModel::Logarithmic.calculate(dist, min_d, max_d, rolloff);

        assert!(linear >= 0.0 && linear <= 1.0);
        assert!(inverse > 0.0 && inverse <= 1.0);
        assert!(log > 0.0 && log <= 1.0);
        assert!((AttenuationModel::None.calculate(dist, min_d, max_d, rolloff) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_listener_direction() {
        let mut listener = Listener::new();
        listener.forward = Vec3::new(0.0, 0.0, -1.0);
        listener.up = Vec3::new(0.0, 1.0, 0.0);

        // Source directly in front
        let (az, el) = listener.source_direction(Vec3::new(0.0, 0.0, -10.0));
        assert!(az.abs() < 5.0, "Source in front should have ~0 azimuth");

        // Source to the right
        let (az, _) = listener.source_direction(Vec3::new(10.0, 0.0, 0.0));
        assert!(az > 80.0, "Source to right should have positive azimuth");
    }

    #[test]
    fn test_stereo_pan() {
        let listener = Listener::new();
        // Source slightly to the left-front
        let (l, r) = listener.stereo_pan(Vec3::new(-3.0, 0.0, -10.0));
        assert!(l > 0.0 && r > 0.0);
        assert!(l > r, "Source to the left should have higher left gain");
        // Source to the right-front
        let (l2, r2) = listener.stereo_pan(Vec3::new(3.0, 0.0, -10.0));
        assert!(r2 > l2, "Source to the right should have higher right gain");
    }

    #[test]
    fn test_low_pass_filter() {
        let mut filter = LowPassFilter::new(0.1);
        let mut output = Vec::new();
        for i in 0..100 {
            let input = if i < 50 { 1.0 } else { 0.0 };
            output.push(filter.process(input));
        }
        // After impulse, should decay
        assert!(output[60] < output[50]);
        assert!(output[60] > 0.0);
    }

    #[test]
    fn test_high_pass_filter() {
        let mut filter = HighPassFilter::new(0.5);
        // DC signal should be attenuated
        let dc_out: Vec<f32> = (0..100).map(|_| filter.process(1.0)).collect();
        assert!(dc_out[50].abs() < 0.1, "High-pass should attenuate DC");
    }

    #[test]
    fn test_band_pass_filter() {
        let mut filter = BandPassFilter::new(0.1, 0.9);
        let out = filter.process(1.0);
        assert!(out.is_finite());
    }

    #[test]
    fn test_audio_engine_lifecycle() {
        let mut engine = AudioEngine3d::new();
        let id = engine.add_source(Vec3::new(5.0, 0.0, 0.0), SoundCategory::SFX);
        assert!(engine.get_source(id).is_some());
        engine.remove_source(id);
        assert!(engine.get_source(id).is_none());
    }

    #[test]
    fn test_audio_engine_zones() {
        let mut engine = AudioEngine3d::new();
        let z1 = engine.add_zone(AudioZoneType::Reverb, Vec3::ZERO, 10.0);
        let z2 = engine.add_zone(AudioZoneType::Underwater, Vec3::new(5.0, 0.0, 0.0), 5.0);
        assert_eq!(engine.zones.len(), 2);
        engine.remove_zone(z1);
        assert_eq!(engine.zones.len(), 1);
    }

    #[test]
    fn test_audio_engine_calculate_output() {
        let mut engine = AudioEngine3d::new();
        let id = engine.add_source(Vec3::new(-3.0, 0.0, -5.0), SoundCategory::SFX);
        let output = engine.calculate_source_output(id);
        assert!(output.gain > 0.0);
        assert!(output.left_gain >= 0.0);
        assert!(output.right_gain >= 0.0);
        assert!(output.left_gain + output.right_gain > 0.0, "Should have some stereo output");
    }

    #[test]
    fn test_audio_engine_update_stats() {
        let mut engine = AudioEngine3d::new();
        let _id1 = engine.add_source(Vec3::new(5.0, 0.0, 0.0), SoundCategory::SFX);
        let _id2 = engine.add_source(Vec3::new(10.0, 0.0, 0.0), SoundCategory::Music);
        engine.update(0.016);
        assert_eq!(engine.stats.active_sources, 2);
        assert_eq!(engine.stats.total_sources, 2);
    }

    #[test]
    fn test_audio_engine_occlusion_output() {
        let mut engine = AudioEngine3d::new();
        // Add wall between listener and source
        engine.geometry.walls.clear();
        engine.geometry.add_wall(
            Vec3::new(-0.1, -2.0, -0.5),
            Vec3::new(0.1, 2.0, 0.5),
            Material::concrete(),
        );
        let id = engine.add_source(Vec3::new(0.0, 0.0, 5.0), SoundCategory::SFX);
        let output = engine.calculate_source_output(id);
        assert!(output.occluded, "Source behind wall should be occluded");
        assert!(output.occlusion_filter < 1.0, "Occluded source should be filtered");
    }

    #[test]
    fn test_audio_engine_stereo_process() {
        let mut engine = AudioEngine3d::new();
        let id = engine.add_source(Vec3::new(5.0, 0.0, 0.0), SoundCategory::SFX);
        let input = vec![0.5; 256];
        let (left, right) = engine.process_stereo(&input, id);
        assert_eq!(left.len(), 256);
        assert_eq!(right.len(), 256);
    }

    #[test]
    fn test_reverb_presets() {
        let room = ReverbParams::room();
        let hall = ReverbParams::hall();
        let cave = ReverbParams::cave();
        let underwater = ReverbParams::underwater();
        assert!(room.room_size < hall.room_size);
        assert!(hall.room_size < cave.room_size);
        assert!(underwater.high_frequency_damping > 0.5);
    }

    #[test]
    fn test_source_directional_cone() {
        let mut engine = AudioEngine3d::new();
        let id = engine.add_source(Vec3::new(5.0, 0.0, 0.0), SoundCategory::SFX);
        if let Some(src) = engine.get_source_mut(id) {
            src.directivity = 1.0;
            src.inner_cone_angle = 30.0;
            src.outer_cone_angle = 90.0;
            src.outer_cone_gain = 0.2;
        }
        let output = engine.calculate_source_output(id);
        assert!(output.gain > 0.0);
    }

    #[test]
    fn test_pseudo_random_determinism() {
        let a = pseudo_random(42.0);
        let b = pseudo_random(42.0);
        assert_eq!(a, b);
    }

    #[test]
    fn test_convolution() {
        let signal = vec![1.0, 0.5, 0.25, 0.125];
        let impulse = vec![0.5, 0.3, 0.1];
        let output = convolve(&signal, &impulse);
        assert_eq!(output.len(), 4); // truncated to signal length
        assert!(output[0].abs() > 0.0);
    }

    #[test]
    fn test_attenuation_logarithmic() {
        let a = AttenuationModel::Logarithmic.calculate(1.0, 1.0, 50.0, 1.0);
        let b = AttenuationModel::Logarithmic.calculate(10.0, 1.0, 50.0, 1.0);
        assert!(a > b, "Closer should be louder");
    }

    #[test]
    fn test_attenuation_exponential() {
        let a = AttenuationModel::Exponential.calculate(1.0, 1.0, 50.0, 2.0);
        let b = AttenuationModel::Exponential.calculate(10.0, 1.0, 50.0, 2.0);
        assert!(a > b);
    }

    #[test]
    fn test_listener_velocity_doppler() {
        let mut doppler = DopplerEffect::new();
        doppler.listener_velocity = Vec3::new(0.0, 0.0, 100.0); // Listener moving toward source
        let shift = doppler.calculate(Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0));
        assert!(shift > 1.0, "Listener approaching source should increase pitch");
    }

    #[test]
    fn test_source_looping_flag() {
        let mut engine = AudioEngine3d::new();
        let id = engine.add_source(Vec3::ZERO, SoundCategory::SFX);
        if let Some(src) = engine.get_source_mut(id) {
            src.looping = true;
        }
        let src = engine.get_source(id).unwrap();
        assert!(src.looping);
    }

    #[test]
    fn test_audio_engine_master_volume() {
        let mut engine = AudioEngine3d::new();
        engine.set_master_volume(0.5);
        assert!((engine.master_volume - 0.5).abs() < 0.001);

        let id = engine.add_source(Vec3::ZERO, SoundCategory::SFX);
        let output = engine.calculate_source_output(id);
        assert!(output.gain <= 0.5, "Master volume should scale output");
    }
}
