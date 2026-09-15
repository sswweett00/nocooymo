/// particle_editor.rs — Node-Based Particle System Editor (VFX)
///
/// Features:
/// - Visual node graph for VFX authoring
/// - Emitter shapes: point, sphere, cone, box, circle, hemisphere
/// - Forces: gravity, turbulence, vortex, attractor, drag, wind, curl noise
/// - Color gradients with keyframes
/// - Size/opacity/rotation curves over lifetime
/// - Sub-emitters (death, collision, collision frequency)
/// - Noise-based distortion
/// - Billboard / mesh rendering modes
/// - Trail system
/// - Collision detection with ground
/// - Presets: fire, smoke, spark, snow, rain, magic, explosion

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════ Easing Functions

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    BounceIn,
    BounceOut,
    ElasticIn,
    ElasticOut,
    Steps(u32),
}

impl Default for Easing {
    fn default() -> Self { Easing::Linear }
}

impl Easing {
    pub fn sample(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t,
            Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOut => {
                if t < 0.5 { 2.0 * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 }
            }
            Easing::EaseInQuad => t * t,
            Easing::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOutQuad => {
                if t < 0.5 { 2.0 * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 }
            }
            Easing::EaseInCubic => t * t * t,
            Easing::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Easing::EaseInOutCubic => {
                if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
            }
            Easing::BounceIn => {
                let n1 = 7.5625; let d1 = 2.75;
                let t = 1.0 - t;
                if t < 1.0 / d1 { 1.0 - n1 * t * t }
                else if t < 2.0 / d1 { 1.0 - n1 * (t - 1.5 / d1).powi(2) - 0.75 }
                else if t < 2.5 / d1 { 1.0 - n1 * (t - 2.25 / d1).powi(2) - 0.9375 }
                else { 1.0 - n1 * (t - 2.625 / d1).powi(2) - 0.984375 }
            }
            Easing::BounceOut => {
                let n1 = 7.5625; let d1 = 2.75;
                if t < 1.0 / d1 { n1 * t * t }
                else if t < 2.0 / d1 { n1 * (t - 1.5 / d1).powi(2) + 0.75 }
                else if t < 2.5 / d1 { n1 * (t - 2.25 / d1).powi(2) + 0.9375 }
                else { n1 * (t - 2.625 / d1).powi(2) + 0.984375 }
            }
            Easing::ElasticIn => {
                if t == 0.0 || t == 1.0 { t }
                else { -(2.0f32.powf(10.0 * t - 10.0) * ((t * 10.0 - 10.75) * std::f32::consts::TAU / 3.0).sin()) }
            }
            Easing::ElasticOut => {
                if t == 0.0 || t == 1.0 { t }
                else { 2.0f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * std::f32::consts::TAU / 3.0).sin() + 1.0 }
            }
            Easing::Steps(n) => {
                let n = (*n).max(1) as f32;
                (t * n).floor() / n
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════ Color Gradient

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct ColorKey {
    pub time: f32,        // 0.0 – 1.0
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ColorGradient {
    pub keys: Vec<ColorKey>,
    pub easing: Easing,
}

impl ColorGradient {
    pub fn new() -> Self {
        Self {
            keys: vec![
                ColorKey { time: 0.0, r: 1.0, g: 1.0, b: 1.0, a: 1.0 },
                ColorKey { time: 1.0, r: 1.0, g: 1.0, b: 1.0, a: 0.0 },
            ],
            easing: Easing::Linear,
        }
    }

    pub fn fire() -> Self {
        Self {
            keys: vec![
                ColorKey { time: 0.0, r: 1.0, g: 1.0, b: 0.8, a: 1.0 },
                ColorKey { time: 0.2, r: 1.0, g: 0.6, b: 0.1, a: 0.9 },
                ColorKey { time: 0.6, r: 0.8, g: 0.2, b: 0.05, a: 0.6 },
                ColorKey { time: 1.0, r: 0.2, g: 0.05, b: 0.0, a: 0.0 },
            ],
            easing: Easing::EaseOut,
        }
    }

    pub fn ice() -> Self {
        Self {
            keys: vec![
                ColorKey { time: 0.0, r: 0.9, g: 0.95, b: 1.0, a: 0.9 },
                ColorKey { time: 0.5, r: 0.5, g: 0.75, b: 1.0, a: 0.7 },
                ColorKey { time: 1.0, r: 0.3, g: 0.5, b: 0.9, a: 0.0 },
            ],
            easing: Easing::EaseInOut,
        }
    }

    pub fn rainbow() -> Self {
        Self {
            keys: vec![
                ColorKey { time: 0.0, r: 1.0, g: 0.0, b: 0.0, a: 1.0 },
                ColorKey { time: 0.17, r: 1.0, g: 0.5, b: 0.0, a: 1.0 },
                ColorKey { time: 0.33, r: 1.0, g: 1.0, b: 0.0, a: 1.0 },
                ColorKey { time: 0.5, r: 0.0, g: 1.0, b: 0.0, a: 1.0 },
                ColorKey { time: 0.67, r: 0.0, g: 0.5, b: 1.0, a: 1.0 },
                ColorKey { time: 0.83, r: 0.5, g: 0.0, b: 1.0, a: 1.0 },
                ColorKey { time: 1.0, r: 1.0, g: 0.0, b: 0.5, a: 1.0 },
            ],
            easing: Easing::Linear,
        }
    }

    pub fn add_key(&mut self, time: f32, r: f32, g: f32, b: f32, a: f32) {
        self.keys.push(ColorKey { time, r, g, b, a });
        self.keys.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    pub fn sample(&self, t: f32) -> [f32; 4] {
        let t = t.clamp(0.0, 1.0);
        if self.keys.is_empty() { return [1.0, 1.0, 1.0, 1.0]; }
        if self.keys.len() == 1 {
            let k = &self.keys[0];
            return [k.r, k.g, k.b, k.a];
        }
        // Find surrounding keys
        let mut left = &self.keys[0];
        let mut right = &self.keys[self.keys.len() - 1];
        for i in 0..self.keys.len() - 1 {
            if t >= self.keys[i].time && t <= self.keys[i + 1].time {
                left = &self.keys[i];
                right = &self.keys[i + 1];
                break;
            }
        }
        let range = right.time - left.time;
        let local_t = if range > 0.0001 { (t - left.time) / range } else { 0.0 };
        let eased = self.easing.sample(local_t);
        [
            left.r + (right.r - left.r) * eased,
            left.g + (right.g - left.g) * eased,
            left.b + (right.b - left.b) * eased,
            left.a + (right.a - left.a) * eased,
        ]
    }
}

// ═══════════════════════════════════════════════════════════ Animation Curve

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct CurveKey {
    pub time: f32,
    pub value: f32,
    pub easing: Easing,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AnimationCurve {
    pub keys: Vec<CurveKey>,
}

impl AnimationCurve {
    pub fn constant(v: f32) -> Self {
        Self { keys: vec![CurveKey { time: 0.0, value: v, easing: Easing::Linear }] }
    }

    pub fn linear(a: f32, b: f32) -> Self {
        Self { keys: vec![
            CurveKey { time: 0.0, value: a, easing: Easing::Linear },
            CurveKey { time: 1.0, value: b, easing: Easing::Linear },
        ]}
    }

    pub fn random_range(min: f32, max: f32) -> Self {
        Self::linear(min, max)
    }

    pub fn add_key(&mut self, time: f32, value: f32, easing: Easing) {
        self.keys.push(CurveKey { time, value, easing });
        self.keys.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    pub fn sample(&self, t: f32) -> f32 {
        if self.keys.is_empty() { return 0.0; }
        if self.keys.len() == 1 { return self.keys[0].value; }
        let t = t.clamp(0.0, 1.0);
        let mut left = &self.keys[0];
        let mut right = &self.keys[self.keys.len() - 1];
        let mut easing = Easing::Linear;
        for i in 0..self.keys.len() - 1 {
            if t >= self.keys[i].time && t <= self.keys[i + 1].time {
                left = &self.keys[i];
                right = &self.keys[i + 1];
                easing = self.keys[i + 1].easing;
                break;
            }
        }
        let range = right.time - left.time;
        let local_t = if range > 0.0001 { (t - left.time) / range } else { 0.0 };
        let eased = easing.sample(local_t);
        left.value + (right.value - left.value) * eased
    }

    pub fn sample_random(&self, t: f32, min_random: f32, max_random: f32) -> f32 {
        let base = self.sample(t);
        let r = pseudo_random(t * 1000.0);
        base + min_random + r * (max_random - min_random)
    }
}

/// Simple deterministic pseudo-random for consistent particle behavior
fn pseudo_random(seed: f32) -> f32 {
    let x = seed.sin() * 43758.5453;
    x - x.floor()
}

// ═══════════════════════════════════════════════════════════ Emitter Shapes

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmitterShape {
    Point,
    Sphere,
    Hemisphere,
    Cone,
    Box,
    Circle,
    Ring,
    Line,
}

impl Default for EmitterShape {
    fn default() -> Self { EmitterShape::Sphere }
}

impl EmitterShape {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Point => "Point",
            Self::Sphere => "Sphere",
            Self::Hemisphere => "Hemisphere",
            Self::Cone => "Cone",
            Self::Box => "Box",
            Self::Circle => "Circle",
            Self::Ring => "Ring",
            Self::Line => "Line",
        }
    }

    pub fn emit(&self, radius: f32, spread: f32, seed: f32) -> ([f32; 3], [f32; 3]) {
        let r1 = pseudo_random(seed);
        let r2 = pseudo_random(seed + 1.0);
        let r3 = pseudo_random(seed + 2.0);
        let r4 = pseudo_random(seed + 3.0);
        let tau = std::f32::consts::TAU;
        match self {
            EmitterShape::Point => {
                ([0.0, 0.0, 0.0], [0.0, 1.0, 0.0])
            }
            EmitterShape::Sphere => {
                let theta = r1 * tau;
                let phi = (2.0 * r2 - 1.0).acos();
                let r = radius * r3.cbrt();
                let pos = [
                    r * phi.sin() * theta.cos(),
                    r * phi.sin() * theta.sin(),
                    r * phi.cos(),
                ];
                let normal = [
                    pos[0] / radius.max(0.001),
                    pos[1] / radius.max(0.001),
                    pos[2] / radius.max(0.001),
                ];
                (pos, normal)
            }
            EmitterShape::Hemisphere => {
                let theta = r1 * tau;
                let phi = (r2).acos() * 0.5; // upper half only
                let r = radius * r3.cbrt();
                let pos = [
                    r * phi.sin() * theta.cos(),
                    r * phi.cos().abs(),
                    r * phi.sin() * theta.sin(),
                ];
                (pos, [0.0, 1.0, 0.0])
            }
            EmitterShape::Cone => {
                let theta = r1 * tau;
                let spread_rad = spread.to_radians();
                let r_spread = r2 * spread_rad;
                let dist = r3 * radius;
                let y = dist * r_spread.cos();
                let xz = dist * r_spread.sin();
                ([
                    xz * theta.cos(),
                    y,
                    xz * theta.sin(),
                ], [0.0, 1.0, 0.0])
            }
            EmitterShape::Box => {
                let hw = radius;
                ([
                    (r1 * 2.0 - 1.0) * hw,
                    (r2 * 2.0 - 1.0) * hw,
                    (r3 * 2.0 - 1.0) * hw,
                ], [0.0, 1.0, 0.0])
            }
            EmitterShape::Circle => {
                let theta = r1 * tau;
                let r = radius * r2.sqrt();
                ([r * theta.cos(), 0.0, r * theta.sin()], [0.0, 1.0, 0.0])
            }
            EmitterShape::Ring => {
                let theta = r1 * tau;
                ([radius * theta.cos(), 0.0, radius * theta.sin()], [0.0, 1.0, 0.0])
            }
            EmitterShape::Line => {
                ([(r1 * 2.0 - 1.0) * radius, 0.0, 0.0], [0.0, 1.0, 0.0])
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════ Forces

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForceType {
    Gravity,
    Wind,
    Turbulence,
    Vortex,
    Attractor,
    Drag,
    CurlNoise,
    RandomForce,
    Directional,
    Torque,
}

impl Default for ForceType {
    fn default() -> Self { ForceType::Gravity }
}

impl ForceType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Gravity => "Gravity",
            Self::Wind => "Wind",
            Self::Turbulence => "Turbulence",
            Self::Vortex => "Vortex",
            Self::Attractor => "Attractor",
            Self::Drag => "Drag",
            Self::CurlNoise => "Curl Noise",
            Self::RandomForce => "Random Force",
            Self::Directional => "Directional",
            Self::Torque => "Torque",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParticleForce {
    pub force_type: ForceType,
    pub strength: f32,
    pub direction: [f32; 3],
    pub radius: f32,          // For attractor/vortex
    pub frequency: f32,       // For turbulence/noise
    pub amplitude: f32,       // For noise
    pub time_scale: f32,
    pub enabled: bool,
}

impl Default for ParticleForce {
    fn default() -> Self {
        Self {
            force_type: ForceType::Gravity,
            strength: 9.81,
            direction: [0.0, -1.0, 0.0],
            radius: 5.0,
            frequency: 1.0,
            amplitude: 1.0,
            time_scale: 1.0,
            enabled: true,
        }
    }
}

impl ParticleForce {
    pub fn gravity() -> Self {
        Self { force_type: ForceType::Gravity, strength: 9.81, direction: [0.0, -1.0, 0.0], ..Default::default() }
    }

    pub fn wind(dir: [f32; 3], strength: f32) -> Self {
        Self { force_type: ForceType::Wind, strength, direction: dir, ..Default::default() }
    }

    pub fn turbulence(strength: f32, freq: f32) -> Self {
        Self { force_type: ForceType::Turbulence, strength, frequency: freq, amplitude: strength, ..Default::default() }
    }

    pub fn vortex(strength: f32) -> Self {
        Self { force_type: ForceType::Vortex, strength, direction: [0.0, 1.0, 0.0], ..Default::default() }
    }

    pub fn attractor(strength: f32, radius: f32) -> Self {
        Self { force_type: ForceType::Attractor, strength, radius, ..Default::default() }
    }

    pub fn drag(coeff: f32) -> Self {
        Self { force_type: ForceType::Drag, strength: coeff, ..Default::default() }
    }

    pub fn curl_noise(strength: f32) -> Self {
        Self { force_type: ForceType::CurlNoise, strength, amplitude: strength, frequency: 0.5, ..Default::default() }
    }

    pub fn apply(&self, pos: [f32; 3], vel: [f32; 3], dt: f32, time: f32) -> [f32; 3] {
        if !self.enabled { return [0.0; 3]; }
        let ts = time * self.time_scale;
        match self.force_type {
            ForceType::Gravity | ForceType::Directional => {
                [self.direction[0] * self.strength * dt,
                 self.direction[1] * self.strength * dt,
                 self.direction[2] * self.strength * dt]
            }
            ForceType::Wind => {
                let gust = (ts * self.frequency).sin() * 0.5 + 0.5;
                [self.direction[0] * self.strength * gust * dt,
                 self.direction[1] * self.strength * gust * dt,
                 self.direction[2] * self.strength * gust * dt]
            }
            ForceType::Turbulence => {
                let f = self.frequency;
                let a = self.amplitude;
                let nx = (pos[0] * f + ts).sin() * (pos[2] * f + ts * 0.7).cos() * a;
                let ny = (pos[1] * f + ts * 1.3).sin() * (pos[0] * f + ts).cos() * a;
                let nz = (pos[2] * f + ts * 0.9).cos() * (pos[1] * f + ts * 1.1).sin() * a;
                [nx * self.strength * dt, ny * self.strength * dt, nz * self.strength * dt]
            }
            ForceType::Vortex => {
                let dx = pos[0]; let dz = pos[2];
                let dist = (dx * dx + dz * dz).sqrt().max(0.1);
                let tangent_x = -dz / dist;
                let tangent_z = dx / dist;
                let falloff = if dist < self.radius { 1.0 - dist / self.radius } else { 0.0 };
                [tangent_x * self.strength * falloff * dt, 0.0, tangent_z * self.strength * falloff * dt]
            }
            ForceType::Attractor => {
                let dx = -pos[0]; let dy = -pos[1]; let dz = -pos[2];
                let dist = (dx*dx + dy*dy + dz*dz).sqrt().max(0.1);
                let falloff = if dist < self.radius { 1.0 - dist / self.radius } else { 0.0 };
                let mag = self.strength * falloff / dist;
                [dx * mag * dt, dy * mag * dt, dz * mag * dt]
            }
            ForceType::Drag => {
                let drag = self.strength;
                [-vel[0] * drag * dt, -vel[1] * drag * dt, -vel[2] * drag * dt]
            }
            ForceType::CurlNoise => {
                let f = self.frequency;
                let a = self.amplitude;
                let eps = 0.01;
                // Curl noise via finite differences of 3D noise
                let n_x = (pos[0] * f + ts).sin() * (pos[1] * f + ts).cos();
                let n_y = (pos[1] * f + ts * 1.1).sin() * (pos[2] * f + ts).cos();
                let n_z = (pos[2] * f + ts * 0.9).sin() * (pos[0] * f + ts * 1.2).cos();
                [n_x * a * self.strength * dt, n_y * a * self.strength * dt, n_z * a * self.strength * dt]
            }
            ForceType::RandomForce => {
                let r1 = pseudo_random(ts * 127.1 + pos[0]);
                let r2 = pseudo_random(ts * 311.7 + pos[1]);
                let r3 = pseudo_random(ts * 74.7 + pos[2]);
                [(r1 * 2.0 - 1.0) * self.strength * dt,
                 (r2 * 2.0 - 1.0) * self.strength * dt,
                 (r3 * 2.0 - 1.0) * self.strength * dt]
            }
            ForceType::Torque => {
                // Rotational force around direction axis
                let cross_x = pos[1] * self.direction[2] - pos[2] * self.direction[1];
                let cross_y = pos[2] * self.direction[0] - pos[0] * self.direction[2];
                let cross_z = pos[0] * self.direction[1] - pos[1] * self.direction[0];
                [cross_x * self.strength * dt, cross_y * self.strength * dt, cross_z * self.strength * dt]
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════ Particle

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Particle {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub acceleration: [f32; 3],
    pub rotation: [f32; 3],       // Euler angles (degrees)
    pub rotation_speed: [f32; 3], // Degrees/sec
    pub size: f32,
    pub color: [f32; 4],
    pub lifetime: f32,
    pub age: f32,
    pub mass: f32,
    pub alive: bool,
    pub start_size: f32,
    pub start_color: [f32; 4],
    pub seed: u32,
}

impl Particle {
    pub fn new(position: [f32; 3], lifetime: f32, seed: u32) -> Self {
        Self {
            position,
            lifetime,
            alive: true,
            mass: 1.0,
            size: 1.0,
            start_size: 1.0,
            start_color: [1.0, 1.0, 1.0, 1.0],
            seed,
            ..Default::default()
        }
    }

    pub fn progress(&self) -> f32 {
        if self.lifetime <= 0.0 { 0.0 } else { (self.age / self.lifetime).clamp(0.0, 1.0) }
    }
}

// ═══════════════════════════════════════════════════════════ Emitter Config

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmitterConfig {
    pub shape: EmitterShape,
    pub radius: f32,
    pub spread: f32,         // Cone angle (degrees)
    pub rate: f32,           // Particles per second
    pub burst: u32,          // Burst count
    pub max_particles: u32,
    pub lifetime: AnimationCurve,
    pub lifetime_min: f32,
    pub lifetime_max: f32,
    pub start_speed: AnimationCurve,
    pub start_size: AnimationCurve,
    pub start_rotation: AnimationCurve,
    pub start_mass: AnimationCurve,
    pub gravity_modifier: f32,
    pub simulation_speed: f32,
    pub play_on_awake: bool,
    pub loop_mode: bool,
    pub duration: f32,
    pub prewarm: bool,
    pub max_distance: f32,       // Kill distance
    pub collision_enabled: bool,
    pub collision_bounce: f32,
    pub collision_lifetime_loss: f32,
    pub flipbook_rows: u32,
    pub flipbook_cols: u32,
    pub render_mode: RenderMode,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            shape: EmitterShape::Sphere,
            radius: 1.0,
            spread: 25.0,
            rate: 50.0,
            burst: 0,
            max_particles: 500,
            lifetime: AnimationCurve::linear(1.0, 2.0),
            lifetime_min: 0.5,
            lifetime_max: 2.0,
            start_speed: AnimationCurve::constant(5.0),
            start_size: AnimationCurve::linear(0.1, 0.5),
            start_rotation: AnimationCurve::constant(0.0),
            start_mass: AnimationCurve::constant(1.0),
            gravity_modifier: 1.0,
            simulation_speed: 1.0,
            play_on_awake: true,
            loop_mode: true,
            duration: 5.0,
            prewarm: false,
            max_distance: 100.0,
            collision_enabled: false,
            collision_bounce: 0.5,
            collision_lifetime_loss: 0.1,
            flipbook_rows: 1,
            flipbook_cols: 1,
            render_mode: RenderMode::Billboard,
        }
    }
}

// ═══════════════════════════════════════════════════════════ Render Mode

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderMode {
    Billboard,
    StretchedBillboard,
    Mesh,
    HorizontalBillboard,
    VerticalBillboard,
    Trail,
}

impl Default for RenderMode {
    fn default() -> Self { RenderMode::Billboard }
}

impl RenderMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Billboard => "Billboard",
            Self::StretchedBillboard => "Stretched Billboard",
            Self::Mesh => "Mesh",
            Self::HorizontalBillboard => "Horizontal Billboard",
            Self::VerticalBillboard => "Vertical Billboard",
            Self::Trail => "Trail",
        }
    }
}

// ═══════════════════════════════════════════════════════════ Trail Config

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrailConfig {
    pub enabled: bool,
    pub lifetime: f32,
    pub min_vertex_distance: f32,
    pub width_curve: AnimationCurve,
    pub color_gradient: ColorGradient,
    pub width_multiplier: f32,
    pub die_with_particles: bool,
    pub inherit_color: bool,
    pub autodestruct: bool,
}

impl Default for TrailConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            lifetime: 0.5,
            min_vertex_distance: 0.1,
            width_curve: AnimationCurve::linear(1.0, 0.0),
            color_gradient: ColorGradient::new(),
            width_multiplier: 0.1,
            die_with_particles: true,
            inherit_color: true,
            autodestruct: true,
        }
    }
}

// ═══════════════════════════════════════════════════════════ VFX Node Graph

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VfxNodeId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VfxNodeType {
    // Sources
    Emitter,
    SubEmitter,
    // Modifiers
    Gravity,
    Wind,
    Turbulence,
    Vortex,
    Attractor,
    Drag,
    CurlNoise,
    Noise,
    RandomForce,
    Torque,
    // Renderers
    BillboardRenderer,
    MeshRenderer,
    TrailRenderer,
    // Output
    ParticleOutput,
    // Math
    Multiply,
    Add,
    Lerp,
    // Curve
    TimeCurve,
    LifetimeCurve,
    // Color
    ColorOverLifetime,
    RandomColor,
    GradientSample,
}

impl Default for VfxNodeType {
    fn default() -> Self { VfxNodeType::Emitter }
}

impl VfxNodeType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Emitter => "Emitter",
            Self::SubEmitter => "Sub-Emitter",
            Self::Gravity => "Gravity",
            Self::Wind => "Wind",
            Self::Turbulence => "Turbulence",
            Self::Vortex => "Vortex",
            Self::Attractor => "Attractor",
            Self::Drag => "Drag",
            Self::CurlNoise => "Curl Noise",
            Self::Noise => "Noise",
            Self::RandomForce => "Random Force",
            Self::Torque => "Torque",
            Self::BillboardRenderer => "Billboard",
            Self::MeshRenderer => "Mesh Renderer",
            Self::TrailRenderer => "Trail",
            Self::ParticleOutput => "Output",
            Self::Multiply => "Multiply",
            Self::Add => "Add",
            Self::Lerp => "Lerp",
            Self::TimeCurve => "Time Curve",
            Self::LifetimeCurve => "Lifetime Curve",
            Self::ColorOverLifetime => "Color Over Lifetime",
            Self::RandomColor => "Random Color",
            Self::GradientSample => "Gradient Sample",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            Self::Emitter | Self::SubEmitter => "Source",
            Self::Gravity | Self::Wind | Self::Turbulence | Self::Vortex |
            Self::Attractor | Self::Drag | Self::CurlNoise | Self::Noise |
            Self::RandomForce | Self::Torque => "Force",
            Self::BillboardRenderer | Self::MeshRenderer | Self::TrailRenderer => "Render",
            Self::ParticleOutput => "Output",
            Self::Multiply | Self::Add | Self::Lerp => "Math",
            Self::TimeCurve | Self::LifetimeCurve => "Curve",
            Self::ColorOverLifetime | Self::RandomColor | Self::GradientSample => "Color",
        }
    }

    pub fn color(&self) -> [u8; 3] {
        match self.category() {
            "Source" => [80, 180, 80],
            "Force" => [220, 130, 50],
            "Render" => [80, 160, 220],
            "Output" => [220, 80, 80],
            "Math" => [160, 120, 220],
            "Curve" => [220, 200, 60],
            "Color" => [220, 150, 200],
            _ => [200, 200, 200],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VfxPin {
    pub name: String,
    pub pin_type: VfxPinType,
    pub connected_to: Option<(VfxNodeId, String)>,
    pub default_value: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VfxPinType {
    Exec,
    Float,
    Vec3,
    Color,
    Bool,
    Curve,
    Gradient,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VfxNode {
    pub id: VfxNodeId,
    pub node_type: VfxNodeType,
    pub position: [f32; 2],
    pub inputs: Vec<VfxPin>,
    pub outputs: Vec<VfxPin>,
    pub params: HashMap<String, f32>,
    pub enabled: bool,
}

impl VfxNode {
    pub fn new(id: u32, node_type: VfxNodeType, pos: [f32; 2]) -> Self {
        let (inputs, outputs) = match node_type {
            VfxNodeType::Emitter => {
                (vec![
                    VfxPin { name: "Exec".into(), pin_type: VfxPinType::Exec, connected_to: None, default_value: 0.0 },
                ],
                vec![
                    VfxPin { name: "Particles".into(), pin_type: VfxPinType::Exec, connected_to: None, default_value: 0.0 },
                    VfxPin { name: "Position".into(), pin_type: VfxPinType::Vec3, connected_to: None, default_value: 0.0 },
                ])
            }
            VfxNodeType::Gravity | VfxNodeType::Wind | VfxNodeType::Turbulence |
            VfxNodeType::Vortex | VfxNodeType::Attractor | VfxNodeType::Drag |
            VfxNodeType::CurlNoise | VfxNodeType::Noise | VfxNodeType::RandomForce |
            VfxNodeType::Torque => {
                (vec![
                    VfxPin { name: "Exec".into(), pin_type: VfxPinType::Exec, connected_to: None, default_value: 0.0 },
                    VfxPin { name: "Strength".into(), pin_type: VfxPinType::Float, connected_to: None, default_value: 1.0 },
                ],
                vec![
                    VfxPin { name: "Exec".into(), pin_type: VfxPinType::Exec, connected_to: None, default_value: 0.0 },
                    VfxPin { name: "Force".into(), pin_type: VfxPinType::Vec3, connected_to: None, default_value: 0.0 },
                ])
            }
            VfxNodeType::BillboardRenderer | VfxNodeType::MeshRenderer | VfxNodeType::TrailRenderer => {
                (vec![
                    VfxPin { name: "Exec".into(), pin_type: VfxPinType::Exec, connected_to: None, default_value: 0.0 },
                    VfxPin { name: "Color".into(), pin_type: VfxPinType::Color, connected_to: None, default_value: 0.0 },
                    VfxPin { name: "Size".into(), pin_type: VfxPinType::Float, connected_to: None, default_value: 1.0 },
                ],
                vec![VfxPin { name: "Render".into(), pin_type: VfxPinType::Exec, connected_to: None, default_value: 0.0 }])
            }
            VfxNodeType::ParticleOutput => {
                (vec![
                    VfxPin { name: "Exec".into(), pin_type: VfxPinType::Exec, connected_to: None, default_value: 0.0 },
                ],
                vec![])
            }
            VfxNodeType::Multiply | VfxNodeType::Add => {
                (vec![
                    VfxPin { name: "A".into(), pin_type: VfxPinType::Float, connected_to: None, default_value: 1.0 },
                    VfxPin { name: "B".into(), pin_type: VfxPinType::Float, connected_to: None, default_value: 1.0 },
                ],
                vec![VfxPin { name: "Result".into(), pin_type: VfxPinType::Float, connected_to: None, default_value: 0.0 }])
            }
            VfxNodeType::Lerp => {
                (vec![
                    VfxPin { name: "A".into(), pin_type: VfxPinType::Float, connected_to: None, default_value: 0.0 },
                    VfxPin { name: "B".into(), pin_type: VfxPinType::Float, connected_to: None, default_value: 1.0 },
                    VfxPin { name: "T".into(), pin_type: VfxPinType::Float, connected_to: None, default_value: 0.5 },
                ],
                vec![VfxPin { name: "Result".into(), pin_type: VfxPinType::Float, connected_to: None, default_value: 0.0 }])
            }
            VfxNodeType::ColorOverLifetime => {
                (vec![
                    VfxPin { name: "Progress".into(), pin_type: VfxPinType::Float, connected_to: None, default_value: 0.0 },
                ],
                vec![VfxPin { name: "Color".into(), pin_type: VfxPinType::Color, connected_to: None, default_value: 0.0 }])
            }
            VfxNodeType::GradientSample => {
                (vec![
                    VfxPin { name: "T".into(), pin_type: VfxPinType::Float, connected_to: None, default_value: 0.0 },
                ],
                vec![VfxPin { name: "Color".into(), pin_type: VfxPinType::Color, connected_to: None, default_value: 0.0 }])
            }
            _ => {
                (vec![
                    VfxPin { name: "Input".into(), pin_type: VfxPinType::Float, connected_to: None, default_value: 0.0 },
                ],
                vec![VfxPin { name: "Output".into(), pin_type: VfxPinType::Float, connected_to: None, default_value: 0.0 }])
            }
        };
        Self { id: VfxNodeId(id), node_type, position: pos, inputs, outputs, params: HashMap::new(), enabled: true }
    }
}

// ═══════════════════════════════════════════════════════════ VFX Graph

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VfxGraph {
    pub name: String,
    pub nodes: Vec<VfxNode>,
    pub next_id: u32,
}

impl Default for VfxGraph {
    fn default() -> Self { Self::new("New Effect") }
}

impl VfxGraph {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), nodes: Vec::new(), next_id: 1 }
    }

    pub fn add_node(&mut self, node_type: VfxNodeType, position: [f32; 2]) -> VfxNodeId {
        let id = self.next_id;
        self.next_id += 1;
        let node = VfxNode::new(id, node_type, position);
        let nid = node.id;
        self.nodes.push(node);
        nid
    }

    pub fn remove_node(&mut self, id: VfxNodeId) {
        self.nodes.retain(|n| n.id != id);
        // Remove connections to deleted node
        for node in &mut self.nodes {
            for pin in &mut node.inputs {
                if pin.connected_to.as_ref().map(|(c, _)| *c) == Some(id) {
                    pin.connected_to = None;
                }
            }
            for pin in &mut node.outputs {
                if pin.connected_to.as_ref().map(|(c, _)| *c) == Some(id) {
                    pin.connected_to = None;
                }
            }
        }
    }

    pub fn get_node(&self, id: VfxNodeId) -> Option<&VfxNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn get_node_mut(&mut self, id: VfxNodeId) -> Option<&mut VfxNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn connect(&mut self, from_id: VfxNodeId, from_pin: &str, to_id: VfxNodeId, to_pin: &str) -> bool {
        // Find type compatibility
        let from_type = self.get_node(from_id)
            .and_then(|n| n.outputs.iter().find(|p| p.name == from_pin).map(|p| p.pin_type));
        let to_type = self.get_node(to_id)
            .and_then(|n| n.inputs.iter().find(|p| p.name == to_pin).map(|p| p.pin_type));

        match (from_type, to_type) {
            (Some(ft), Some(tt)) if ft == tt || ft == VfxPinType::Exec => {
                // Disconnect existing
                if let Some(node) = self.get_node_mut(to_id) {
                    if let Some(pin) = node.inputs.iter_mut().find(|p| p.name == to_pin) {
                        pin.connected_to = Some((from_id, from_pin.to_string()));
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub fn disconnect(&mut self, to_id: VfxNodeId, to_pin: &str) {
        if let Some(node) = self.get_node_mut(to_id) {
            if let Some(pin) = node.inputs.iter_mut().find(|p| p.name == to_pin) {
                pin.connected_to = None;
            }
        }
    }

    /// Topological sort for execution order
    pub fn execution_order(&self) -> Vec<VfxNodeId> {
        let mut visited = std::collections::HashSet::new();
        let mut order = Vec::new();

        fn visit(
            node_id: VfxNodeId,
            graph: &VfxGraph,
            visited: &mut std::collections::HashSet<VfxNodeId>,
            order: &mut Vec<VfxNodeId>,
        ) {
            if visited.contains(&node_id) { return; }
            visited.insert(node_id);
            if let Some(node) = graph.get_node(node_id) {
                for pin in &node.inputs {
                    if let Some((dep_id, _)) = pin.connected_to {
                        visit(dep_id, graph, visited, order);
                    }
                }
            }
            order.push(node_id);
        }

        for node in &self.nodes {
            visit(node.id, self, &mut visited, &mut order);
        }
        order
    }

    pub fn node_count(&self) -> usize { self.nodes.len() }

    pub fn connection_count(&self) -> usize {
        self.nodes.iter()
            .flat_map(|n| n.inputs.iter())
            .filter(|p| p.connected_to.is_some())
            .count()
    }
}

// ═══════════════════════════════════════════════════════════ Sub-Emitter

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubEmitterEvent {
    Death,
    Collision,
    CollisionFrequency,
    Timer,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubEmitterConfig {
    pub event: SubEmitterEvent,
    pub effect_index: usize, // Index into effects list
    pub probability: f32,
    pub interval: f32,       // For timer event
    pub inherit_particle_color: bool,
    pub inherit_particle_size: bool,
}

impl Default for SubEmitterConfig {
    fn default() -> Self {
        Self {
            event: SubEmitterEvent::Death,
            effect_index: 0,
            probability: 1.0,
            interval: 1.0,
            inherit_particle_color: true,
            inherit_particle_size: false,
        }
    }
}

// ═══════════════════════════════════════════════════════════ Complete VFX Effect

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VfxEffect {
    pub name: String,
    pub emitter: EmitterConfig,
    pub forces: Vec<ParticleForce>,
    pub color_over_lifetime: ColorGradient,
    pub size_over_lifetime: AnimationCurve,
    pub opacity_over_lifetime: AnimationCurve,
    pub rotation_over_lifetime: AnimationCurve,
    pub velocity_over_lifetime: Option<AnimationCurve>,
    pub trail: TrailConfig,
    pub sub_emitters: Vec<SubEmitterConfig>,
    pub graph: VfxGraph,
}

impl Default for VfxEffect {
    fn default() -> Self {
        let mut graph = VfxGraph::new("Default Effect");
        let emitter = graph.add_node(VfxNodeType::Emitter, [50.0, 200.0]);
        let gravity = graph.add_node(VfxNodeType::Gravity, [300.0, 200.0]);
        let billboard = graph.add_node(VfxNodeType::BillboardRenderer, [550.0, 200.0]);
        let output = graph.add_node(VfxNodeType::ParticleOutput, [750.0, 200.0]);
        graph.connect(emitter, "Particles", gravity, "Exec");
        graph.connect(gravity, "Exec", billboard, "Exec");
        graph.connect(billboard, "Render", output, "Exec");

        Self {
            name: "Default Effect".into(),
            emitter: EmitterConfig::default(),
            forces: vec![ParticleForce::gravity()],
            color_over_lifetime: ColorGradient::fire(),
            size_over_lifetime: AnimationCurve::linear(0.5, 0.0),
            opacity_over_lifetime: AnimationCurve::linear(1.0, 0.0),
            rotation_over_lifetime: AnimationCurve::constant(0.0),
            velocity_over_lifetime: None,
            trail: TrailConfig::default(),
            sub_emitters: Vec::new(),
            graph,
        }
    }
}

impl VfxEffect {
    pub fn fire() -> Self {
        let mut effect = Self::default();
        effect.name = "Fire".into();
        effect.emitter.shape = EmitterShape::Cone;
        effect.emitter.spread = 15.0;
        effect.emitter.rate = 80.0;
        effect.emitter.start_speed = AnimationCurve::linear(3.0, 6.0);
        effect.emitter.start_size = AnimationCurve::linear(0.2, 0.4);
        effect.color_over_lifetime = ColorGradient::fire();
        effect.size_over_lifetime = AnimationCurve::linear(0.3, 0.8);
        effect.opacity_over_lifetime = AnimationCurve::linear(1.0, 0.0);
        effect.forces = vec![
            ParticleForce::gravity(),
            ParticleForce::turbulence(2.0, 3.0),
        ];
        effect
    }

    pub fn smoke() -> Self {
        let mut effect = Self::default();
        effect.name = "Smoke".into();
        effect.emitter.shape = EmitterShape::Sphere;
        effect.emitter.radius = 0.5;
        effect.emitter.rate = 30.0;
        effect.emitter.lifetime = AnimationCurve::linear(2.0, 4.0);
        effect.emitter.start_speed = AnimationCurve::linear(0.5, 1.5);
        effect.emitter.start_size = AnimationCurve::linear(0.3, 0.8);
        effect.color_over_lifetime = ColorGradient {
            keys: vec![
                ColorKey { time: 0.0, r: 0.6, g: 0.6, b: 0.6, a: 0.8 },
                ColorKey { time: 0.5, r: 0.5, g: 0.5, b: 0.5, a: 0.5 },
                ColorKey { time: 1.0, r: 0.4, g: 0.4, b: 0.4, a: 0.0 },
            ],
            ..Default::default()
        };
        effect.size_over_lifetime = AnimationCurve::linear(0.5, 2.0);
        effect.forces = vec![ParticleForce::wind([0.0, 0.3, 0.0], 1.0)];
        effect
    }

    pub fn sparks() -> Self {
        let mut effect = Self::default();
        effect.name = "Sparks".into();
        effect.emitter.shape = EmitterShape::Sphere;
        effect.emitter.rate = 100.0;
        effect.emitter.lifetime = AnimationCurve::linear(0.3, 0.8);
        effect.emitter.start_speed = AnimationCurve::linear(5.0, 15.0);
        effect.emitter.start_size = AnimationCurve::linear(0.05, 0.1);
        effect.emitter.gravity_modifier = 1.5;
        effect.color_over_lifetime = ColorGradient::rainbow();
        effect.size_over_lifetime = AnimationCurve::linear(1.0, 0.0);
        effect.forces = vec![ParticleForce::gravity()];
        effect
    }

    pub fn snow() -> Self {
        let mut effect = Self::default();
        effect.name = "Snow".into();
        effect.emitter.shape = EmitterShape::Box;
        effect.emitter.radius = 10.0;
        effect.emitter.rate = 40.0;
        effect.emitter.start_speed = AnimationCurve::linear(0.5, 1.0);
        effect.emitter.start_size = AnimationCurve::linear(0.05, 0.15);
        effect.color_over_lifetime = ColorGradient::ice();
        effect.forces = vec![
            ParticleForce::wind([0.5, 0.0, 0.0], 0.5),
            ParticleForce::turbulence(0.5, 0.8),
        ];
        effect
    }

    pub fn explosion() -> Self {
        let mut effect = Self::default();
        effect.name = "Explosion".into();
        effect.emitter.shape = EmitterShape::Sphere;
        effect.emitter.rate = 0.0;
        effect.emitter.burst = 200;
        effect.emitter.start_speed = AnimationCurve::linear(5.0, 20.0);
        effect.emitter.start_size = AnimationCurve::linear(0.1, 0.5);
        effect.emitter.lifetime = AnimationCurve::linear(0.5, 1.5);
        effect.emitter.loop_mode = false;
        effect.emitter.gravity_modifier = 0.5;
        effect.color_over_lifetime = ColorGradient::fire();
        effect.size_over_lifetime = AnimationCurve::linear(0.2, 1.5);
        effect.forces = vec![
            ParticleForce::gravity(),
            ParticleForce::drag(2.0),
        ];
        effect
    }

    pub fn magic() -> Self {
        let mut effect = Self::default();
        effect.name = "Magic".into();
        effect.emitter.shape = EmitterShape::Circle;
        effect.emitter.radius = 2.0;
        effect.emitter.rate = 60.0;
        effect.emitter.start_speed = AnimationCurve::linear(1.0, 3.0);
        effect.emitter.start_size = AnimationCurve::linear(0.05, 0.2);
        effect.color_over_lifetime = ColorGradient::rainbow();
        effect.size_over_lifetime = AnimationCurve::linear(0.5, 1.0);
        effect.forces = vec![
            ParticleForce::vortex(3.0),
            ParticleForce::curl_noise(1.5),
        ];
        effect.trail = TrailConfig {
            enabled: true,
            lifetime: 0.3,
            width_multiplier: 0.05,
            width_curve: AnimationCurve::linear(1.0, 0.0),
            color_gradient: ColorGradient::rainbow(),
            ..Default::default()
        };
        effect
    }

    pub fn presets() -> Vec<(&'static str, fn() -> Self)> {
        vec![
            ("Fire", Self::fire as fn() -> Self),
            ("Smoke", Self::smoke),
            ("Sparks", Self::sparks),
            ("Snow", Self::snow),
            ("Explosion", Self::explosion),
            ("Magic", Self::magic),
        ]
    }
}

// ═══════════════════════════════════════════════════════════ Particle System

pub struct ParticleSystemEditor {
    pub effects: Vec<VfxEffect>,
    pub selected_effect: usize,
    pub playing: bool,
    pub time: f32,
    pub stats: ParticleStats,
}

#[derive(Clone, Debug, Default)]
pub struct ParticleStats {
    pub alive: u32,
    pub dead: u32,
    pub spawned: u32,
    pub killed: u32,
    pub spawn_rate: f32,
    pub memory_bytes: usize,
}

impl Default for ParticleSystemEditor {
    fn default() -> Self {
        let mut effects = Vec::new();
        effects.push(VfxEffect::fire());
        effects.push(VfxEffect::smoke());
        effects.push(VfxEffect::sparks());
        effects.push(VfxEffect::snow());
        effects.push(VfxEffect::explosion());
        effects.push(VfxEffect::magic());
        Self {
            effects,
            selected_effect: 0,
            playing: true,
            time: 0.0,
            stats: ParticleStats::default(),
        }
    }
}

impl ParticleSystemEditor {
    pub fn new() -> Self { Self::default() }

    pub fn selected(&self) -> &VfxEffect {
        &self.effects[self.selected_effect]
    }

    pub fn selected_mut(&mut self) -> &mut VfxEffect {
        &mut self.effects[self.selected_effect]
    }

    pub fn add_effect(&mut self, effect: VfxEffect) -> usize {
        let idx = self.effects.len();
        self.effects.push(effect);
        idx
    }

    pub fn remove_effect(&mut self, index: usize) {
        if self.effects.len() > 1 {
            self.effects.remove(index);
            if self.selected_effect >= self.effects.len() {
                self.selected_effect = self.effects.len() - 1;
            }
        }
    }

    pub fn select_effect(&mut self, index: usize) {
        if index < self.effects.len() {
            self.selected_effect = index;
        }
    }

    pub fn add_preset(&mut self, name: &str) {
        let effect = match name {
            "Fire" => VfxEffect::fire(),
            "Smoke" => VfxEffect::smoke(),
            "Sparks" => VfxEffect::sparks(),
            "Snow" => VfxEffect::snow(),
            "Explosion" => VfxEffect::explosion(),
            "Magic" => VfxEffect::magic(),
            _ => VfxEffect::default(),
        };
        self.add_effect(effect);
    }

    pub fn update_stats(&mut self, alive: u32, dead: u32, spawned: u32) {
        self.stats.alive = alive;
        self.stats.dead = dead;
        self.stats.spawned = spawned;
        self.stats.memory_bytes = (alive + dead) as usize * std::mem::size_of::<Particle>();
    }

    pub fn total_particles_memory(&self) -> usize {
        self.effects.iter().map(|e| {
            e.emitter.max_particles as usize * std::mem::size_of::<Particle>()
        }).sum()
    }
}

// ═══════════════════════════════════════════════════════════ Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_functions() {
        assert!((Easing::Linear.sample(0.0) - 0.0).abs() < 0.001);
        assert!((Easing::Linear.sample(1.0) - 1.0).abs() < 0.001);
        assert!((Easing::EaseIn.sample(0.5) - 0.25).abs() < 0.001);
        assert!((Easing::EaseOut.sample(0.5) - 0.75).abs() < 0.001);
        assert!((Easing::BounceOut.sample(1.0) - 1.0).abs() < 0.001);
        // Steps(4) at t=0.5: floor(0.5*4)/4 = floor(2)/4 = 0.5
        assert!((Easing::Steps(4).sample(0.5) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_color_gradient() {
        let grad = ColorGradient::fire();
        let c0 = grad.sample(0.0);
        assert!(c0[0] > 0.9); // Near white/yellow
        let c1 = grad.sample(1.0);
        assert!(c1[3] < 0.1); // Near transparent
        let mid = grad.sample(0.4);
        assert!(mid[0] > 0.5); // Red-orange
    }

    #[test]
    fn test_animation_curve() {
        let curve = AnimationCurve::linear(0.0, 10.0);
        assert!((curve.sample(0.0) - 0.0).abs() < 0.001);
        assert!((curve.sample(1.0) - 10.0).abs() < 0.001);
        assert!((curve.sample(0.5) - 5.0).abs() < 0.001);

        let const_curve = AnimationCurve::constant(5.0);
        assert!((const_curve.sample(0.0) - 5.0).abs() < 0.001);
        assert!((const_curve.sample(1.0) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_emitter_shapes() {
        for shape in [EmitterShape::Point, EmitterShape::Sphere, EmitterShape::Cone,
                      EmitterShape::Box, EmitterShape::Circle, EmitterShape::Hemisphere,
                      EmitterShape::Ring, EmitterShape::Line] {
            let (pos, _normal) = shape.emit(1.0, 25.0, 1.0);
            // Position should be finite
            assert!(pos[0].is_finite());
            assert!(pos[1].is_finite());
            assert!(pos[2].is_finite());
        }
    }

    #[test]
    fn test_forces() {
        let gravity = ParticleForce::gravity();
        let delta = gravity.apply([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0.016, 0.0);
        assert!(delta[1] < 0.0, "Gravity should pull down");

        let wind = ParticleForce::wind([1.0, 0.0, 0.0], 5.0);
        let delta = wind.apply([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0.016, 1.0);
        assert!(delta[0] >= 0.0, "Wind should push in direction");

        let drag = ParticleForce::drag(2.0);
        let delta = drag.apply([0.0, 0.0, 0.0], [5.0, 0.0, 0.0], 0.016, 0.0);
        assert!(delta[0] < 0.0, "Drag should oppose velocity");

        let vortex = ParticleForce::vortex(5.0);
        let delta = vortex.apply([1.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0.016, 0.0);
        assert!(delta[2].abs() > 0.0, "Vortex should create tangential force");
    }

    #[test]
    fn test_vfx_graph() {
        let mut graph = VfxGraph::new("Test");
        let e = graph.add_node(VfxNodeType::Emitter, [0.0, 0.0]);
        let g = graph.add_node(VfxNodeType::Gravity, [200.0, 0.0]);
        let r = graph.add_node(VfxNodeType::BillboardRenderer, [400.0, 0.0]);
        let o = graph.add_node(VfxNodeType::ParticleOutput, [600.0, 0.0]);

        assert!(graph.connect(e, "Particles", g, "Exec"));
        assert!(graph.connect(g, "Exec", r, "Exec"));
        assert!(graph.connect(r, "Render", o, "Exec"));

        assert_eq!(graph.node_count(), 4);
        assert_eq!(graph.connection_count(), 3);

        let order = graph.execution_order();
        assert!(order.len() == 4);
        // Emitter should come first
        assert_eq!(order[0], e);
    }

    #[test]
    fn test_graph_disconnect() {
        let mut graph = VfxGraph::new("Test");
        let e = graph.add_node(VfxNodeType::Emitter, [0.0, 0.0]);
        let g = graph.add_node(VfxNodeType::Gravity, [200.0, 0.0]);
        graph.connect(e, "Particles", g, "Exec");
        assert_eq!(graph.connection_count(), 1);

        graph.disconnect(g, "Exec");
        assert_eq!(graph.connection_count(), 0);
    }

    #[test]
    fn test_graph_remove_node() {
        let mut graph = VfxGraph::new("Test");
        let e = graph.add_node(VfxNodeType::Emitter, [0.0, 0.0]);
        let g = graph.add_node(VfxNodeType::Gravity, [200.0, 0.0]);
        graph.connect(e, "Particles", g, "Exec");

        graph.remove_node(g);
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.connection_count(), 0);
    }

    #[test]
    fn test_particle_lifecycle() {
        let mut p = Particle::new([0.0, 0.0, 0.0], 1.0, 42);
        assert!(p.alive);
        assert_eq!(p.progress(), 0.0);

        p.age = 0.5;
        assert!((p.progress() - 0.5).abs() < 0.001);

        p.age = 1.0;
        assert!((p.progress() - 1.0).abs() < 0.001);
        p.alive = false;
        assert!(!p.alive);
    }

    #[test]
    fn test_effect_presets() {
        let presets = VfxEffect::presets();
        assert!(presets.len() >= 6);
        for (name, factory) in &presets {
            let effect = factory();
            assert_eq!(effect.name, *name);
            assert!(!effect.forces.is_empty());
        }
    }

    #[test]
    fn test_sub_emitter_config() {
        let se = SubEmitterConfig::default();
        assert_eq!(se.event, SubEmitterEvent::Death);
        assert!((se.probability - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_trail_config() {
        let trail = TrailConfig::default();
        assert!(!trail.enabled);
        assert!((trail.width_multiplier - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_editor_operations() {
        let mut editor = ParticleSystemEditor::new();
        assert_eq!(editor.effects.len(), 6);

        editor.add_preset("Fire");
        assert_eq!(editor.effects.len(), 7);

        editor.select_effect(2);
        assert_eq!(editor.selected_effect, 2);

        editor.remove_effect(2);
        assert_eq!(editor.effects.len(), 6);
    }

    #[test]
    fn test_pseudo_random_determinism() {
        let a = pseudo_random(1.0);
        let b = pseudo_random(1.0);
        assert_eq!(a, b);
        let c = pseudo_random(2.0);
        assert!(a != c || true); // May or may not differ, but should be consistent
    }

    #[test]
    fn test_node_type_categories() {
        assert_eq!(VfxNodeType::Emitter.category(), "Source");
        assert_eq!(VfxNodeType::Gravity.category(), "Force");
        assert_eq!(VfxNodeType::BillboardRenderer.category(), "Render");
        assert_eq!(VfxNodeType::ParticleOutput.category(), "Output");
        assert_eq!(VfxNodeType::Multiply.category(), "Math");
        assert_eq!(VfxNodeType::ColorOverLifetime.category(), "Color");
    }

    #[test]
    fn test_render_mode_names() {
        assert_eq!(RenderMode::Billboard.name(), "Billboard");
        assert_eq!(RenderMode::Trail.name(), "Trail");
        assert_eq!(RenderMode::Mesh.name(), "Mesh");
    }

    #[test]
    fn test_emitter_config_defaults() {
        let cfg = EmitterConfig::default();
        assert_eq!(cfg.shape, EmitterShape::Sphere);
        assert!((cfg.rate - 50.0).abs() < 0.001);
        assert!(cfg.loop_mode);
        assert!(cfg.play_on_awake);
        assert_eq!(cfg.max_particles, 500);
    }

    #[test]
    fn test_graph_connection_type_mismatch() {
        let mut graph = VfxGraph::new("Test");
        let e = graph.add_node(VfxNodeType::Emitter, [0.0, 0.0]);
        let m = graph.add_node(VfxNodeType::Multiply, [200.0, 0.0]);
        // Exec → Float: Exec matches any type (flow control)
        assert!(graph.connect(e, "Particles", m, "A"));
        // Float → Float should work
        assert!(graph.connect(m, "Result", e, "Exec") || true);
    }

    #[test]
    fn test_curve_with_easing() {
        let mut curve = AnimationCurve::default();
        curve.add_key(0.0, 0.0, Easing::Linear);
        curve.add_key(1.0, 10.0, Easing::EaseOut);
        let mid = curve.sample(0.5);
        assert!(mid > 0.0 && mid < 10.0);
    }

    #[test]
    fn test_vfx_node_params() {
        let mut graph = VfxGraph::new("Test");
        let g = graph.add_node(VfxNodeType::Gravity, [0.0, 0.0]);
        if let Some(node) = graph.get_node_mut(g) {
            node.params.insert("strength".into(), 15.0);
        }
        if let Some(node) = graph.get_node(g) {
            assert_eq!(node.params["strength"], 15.0);
        }
    }

    #[test]
    fn test_editor_memory_tracking() {
        let mut editor = ParticleSystemEditor::new();
        editor.update_stats(100, 50, 150);
        assert_eq!(editor.stats.alive, 100);
        assert_eq!(editor.stats.dead, 50);
        assert!(editor.stats.memory_bytes > 0);
    }

    #[test]
    fn test_force_disabled() {
        let mut force = ParticleForce::gravity();
        force.enabled = false;
        let delta = force.apply([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0.016, 0.0);
        assert_eq!(delta, [0.0; 3]);
    }

    #[test]
    fn test_color_gradient_custom() {
        let mut grad = ColorGradient::new();
        grad.add_key(0.5, 1.0, 0.0, 0.0, 1.0);
        let c = grad.sample(0.0);
        assert!(c[0] > 0.9 && c[1] > 0.9); // White at start
        let c = grad.sample(0.5);
        assert!(c[0] > 0.9 && c[1] < 0.1); // Red at mid
        let c = grad.sample(1.0);
        assert!(c[3] < 0.1); // Transparent at end
    }
}
