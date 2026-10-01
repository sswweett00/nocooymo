//! # VFX / Efekt Sistemi
//!
//! Tam kapsamlı bir VFX sistemi sağlar:
//! - Parçacık sistemi (emission, forces, collision, sub-emitters, LOD)
//! - Yollar/Ribbonlar (TrailRenderer)
//! - Dijital etkiler (DecalSystem)
//! - Işık şaftları/God Rays (hacimsel ve ekran-uzayı)
//! - Hava efektleri (yağmur, kar, şimşek, hacimsel sis)
//! - Son işlem efektleri (lens flare, chromatic aberration, vignette, film grain, color grading)
//! - Efekt editörü (Cascade benzeri, eğriler, rastgele tohum kontrolü)

use crate::advanced_rendering::TextureHandle;
use elysium_core::math::{Aabb, Mat4, Vec2, Vec3, Vec4};
use glam::{Quat, UVec2};
use parking_lot::RwLock;
use rand::{
    distributions::{Distribution, Uniform},
    SeedableRng, Rng,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Ortak Yardımcı Tipler
// ---------------------------------------------------------------------------

/// Zamanlayıcı ve kullanılabilir rastgele tohum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RandomSeed {
    pub seed: u64,
}

impl RandomSeed {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    /// Belirli bir tohumdan rastgele 32-bit değer döndürür.
    pub fn next_u32(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.seed >> 32) as u32
    }

    /// Belirli bir tohumdan rastgele 64-bit değer döndürür (saf).
    pub fn next_u64(&mut self) -> u64 {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.seed
    }

    /// `rand` crate tabanlı RNG oluşturur.
    pub fn into_rng(self) -> rand::rngs::StdRng {
        rand::rngs::StdRng::seed_from_u64(self.seed)
    }
}

// ---------------------------------------------------------------------------
// EĞRİLER (Curve / Parametre Analizi)
// ---------------------------------------------------------------------------

/// Örneklenmiş Float eğrileri — VFX modülleri için.
#[derive(Debug, Clone, Default)]
pub struct FloatCurve {
    pub samples: Vec<(f32, f32)>,
}

impl FloatCurve {
    pub fn linear() -> Self {
        Self { samples: vec![(0.0, 0.0), (1.0, 1.0)] }
    }

    pub fn constant(v: f32) -> Self {
        Self { samples: vec![(0.0, v), (1.0, v)] }
    }

    pub fn add_key(&mut self, t: f32, v: f32) {
        self.samples.push((t, v));
        self.samples.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    pub fn sample(&self, t: f32) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        if self.samples.len() == 1 || t <= self.samples[0].0 {
            return self.samples[0].1;
        }
        if t >= self.samples[self.samples.len() - 1].0 {
            return self.samples[self.samples.len() - 1].1;
        }
        for i in 0..self.samples.len() - 1 {
            let (t0, v0) = self.samples[i];
            let (t1, v1) = self.samples[i + 1];
            if t >= t0 && t <= t1 {
                let f = (t - t0) / (t1 - t0).max(1e-6);
                return v0 + (v1 - v0) * f;
            }
        }
        self.samples[self.samples.len() - 1].1
    }
}

/// 3-bileşenli renk eğrileri.
#[derive(Debug, Clone, Default)]
pub struct ColorCurve {
    pub r: FloatCurve,
    pub g: FloatCurve,
    pub b: FloatCurve,
    pub a: FloatCurve,
}

impl ColorCurve {
    pub fn sample(&self, t: f32) -> Vec4 {
        Vec4::new(self.r.sample(t), self.g.sample(t), self.b.sample(t), self.a.sample(t))
    }
}

// ---------------------------------------------------------------------------
// PARÇACIK SİSTEMİ — TEMEL TİPLER
// ---------------------------------------------------------------------------

/// Tek bir parçacık özelliği.
#[derive(Debug, Clone, Default)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub acceleration: Vec3,
    pub color: Vec4,
    pub size: Vec2,
    pub rotation: f32,
    pub lifetime: f32,
    pub age: f32,
    pub alive: bool,
    pub seed: u32,
    pub padding: u32,
}

impl Particle {
    pub fn t(&self) -> f32 {
        (self.age / self.lifetime.max(1e-6)).clamp(0.0, 1.0)
    }
}

/// Parçacık havuzu — sıkıştırılmış dizi (SoA benzeri).
#[derive(Debug, Clone, Default)]
pub struct ParticlePool {
    pub positions: Vec<Vec3>,
    pub velocities: Vec<Vec3>,
    pub accelerations: Vec<Vec3>,
    pub colors: Vec<Vec4>,
    pub sizes: Vec<Vec2>,
    pub rotations: Vec<f32>,
    pub lifetimes: Vec<f32>,
    pub ages: Vec<f32>,
    pub alive_flags: Vec<bool>,
    pub seeds: Vec<u32>,
    pub capacity: usize,
    pub count: usize,
}

impl ParticlePool {
    pub fn new(capacity: usize) -> Self {
        Self {
            positions: vec![Vec3::ZERO; capacity],
            velocities: vec![Vec3::ZERO; capacity],
            accelerations: vec![Vec3::ZERO; capacity],
            colors: vec![Vec4::ONE; capacity],
            sizes: vec![Vec2::ONE; capacity],
            rotations: vec![0.0; capacity],
            lifetimes: vec![0.0; capacity],
            ages: vec![0.0; capacity],
            alive_flags: vec![false; capacity],
            seeds: vec![0; capacity],
            capacity,
            count: 0,
        }
    }

    pub fn spawn(&mut self, p: &Particle) -> Option<usize> {
        for i in 0..self.capacity {
            if !self.alive_flags[i] {
                self.positions[i] = p.position;
                self.velocities[i] = p.velocity;
                self.accelerations[i] = p.acceleration;
                self.colors[i] = p.color;
                self.sizes[i] = p.size;
                self.rotations[i] = p.rotation;
                self.lifetimes[i] = p.lifetime.max(1e-6);
                self.ages[i] = 0.0;
                self.alive_flags[i] = true;
                self.seeds[i] = p.seed;
                self.count = self.count.max(i + 1);
                return Some(i);
            }
        }
        None
    }

    pub fn kill(&mut self, index: usize) {
        if index < self.capacity {
            self.alive_flags[index] = false;
        }
    }

    pub fn alive_count(&self) -> usize {
        self.alive_flags.iter().filter(|a| **a).count()
    }

    pub fn is_alive(&self, index: usize) -> bool {
        index < self.capacity && self.alive_flags[index]
    }

    pub fn clear(&mut self) {
        self.alive_flags.fill(false);
        self.count = 0;
    }
}

// ---------------------------------------------------------------------------
// PARÇACIK MODÜLLERİ — Zaman İçinde Değişimler
// ---------------------------------------------------------------------------

/// Hız zaman içinde değişimini tanımlar.
#[derive(Debug, Clone, Default)]
pub struct VelocityOverLifetime {
    pub velocity: Vec3,
    pub random_velocity: Vec3,
    pub curve: FloatCurve,
}

impl VelocityOverLifetime {
    /// Havuzun `index` numaralı parçacığına uygula.
    pub fn apply(&self, pool: &mut ParticlePool, index: usize, t: f32, seed: u32) {
        let mut rng = RandomSeed::new(seed as u64).into_rng();
        let scale = self.curve.sample(t);
        let rx = rng.gen_range(-1.0..1.0);
        let ry = rng.gen_range(-1.0..1.0);
        let rz = rng.gen_range(-1.0..1.0);
        let rnd = Vec3::new(rx, ry, rz) * self.random_velocity;
        pool.velocities[index] += (self.velocity * scale + rnd) * 1.0 / 60.0;
    }
}

/// Renk zaman içinde değişimini tanımlar.
#[derive(Debug, Clone, Default)]
pub struct ColorOverLifetime {
    pub curve: ColorCurve,
    pub alpha_curve: FloatCurve,
}

impl ColorOverLifetime {
    pub fn apply(&self, pool: &mut ParticlePool, index: usize, t: f32) {
        let c = self.curve.sample(t);
        pool.colors[index] = c;
        pool.colors[index].w *= self.alpha_curve.sample(t);
    }
}

/// Boyut zaman içinde değişimini tanımlar.
#[derive(Debug, Clone, Default)]
pub struct SizeOverLifetime {
    pub size_multiplier: FloatCurve,
}

impl SizeOverLifetime {
    pub fn apply(&self, pool: &mut ParticlePool, index: usize, t: f32) {
        let s = self.size_multiplier.sample(t);
        pool.sizes[index] *= s;
    }
}

/// Dönüş zaman içinde değişimini tanımlar.
#[derive(Debug, Clone, Default)]
pub struct RotationOverLifetime {
    pub angular_velocity: FloatCurve,
}

impl RotationOverLifetime {
    pub fn apply(&self, pool: &mut ParticlePool, index: usize, t: f32, dt: f32) {
        pool.rotations[index] += self.angular_velocity.sample(t) * dt;
    }
}

// ---------------------------------------------------------------------------
// GÜÇLER
// ---------------------------------------------------------------------------

/// Yerçekimi gücü.
#[derive(Debug, Clone, Default)]
pub struct GravityForce {
    pub gravity: Vec3,
}

impl GravityForce {
    pub fn apply(&self, pool: &mut ParticlePool, index: usize, _seed: u32, dt: f32) {
        pool.velocities[index] += self.gravity * dt;
        pool.accelerations[index] += self.gravity;
    }
}

/// Rüzgar gücü.
#[derive(Debug, Clone, Default)]
pub struct WindForce {
    pub wind: Vec3,
    pub turbulence: f32,
    pub seed: u64,
}

impl WindForce {
    pub fn apply(&self, pool: &mut ParticlePool, index: usize, _seed: u32, dt: f32) {
        let age = pool.ages[index];
        let mut rng = RandomSeed::new(self.seed.wrapping_add(age as u64)).into_rng();
        let turb = Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0))
            * self.turbulence;
        pool.velocities[index] += (self.wind + turb) * dt;
    }
}

/// Sürüklenme gücü.
#[derive(Debug, Clone, Default)]
pub struct DragForce {
    pub drag_coefficient: f32,
}

impl DragForce {
    pub fn apply(&self, pool: &mut ParticlePool, index: usize, _seed: u32, dt: f32) {
        let speed = pool.velocities[index].length();
        if speed > 1e-6 {
            let drag = -self.drag_coefficient * speed * pool.velocities[index].normalize();
            pool.velocities[index] += drag * dt;
        }
    }
}

/// Girdap (vortex) gücü — merkez etrafında döndürür.
#[derive(Debug, Clone, Default)]
pub struct VortexForce {
    pub center: Vec3,
    pub strength: f32,
    pub radius: f32,
}

impl VortexForce {
    pub fn apply(&self, pool: &mut ParticlePool, index: usize, _seed: u32, dt: f32) {
        let delta = pool.positions[index] - self.center;
        let dist = delta.length();
        if dist < self.radius && dist > 1e-6 {
            let falloff = 1.0 - (dist / self.radius);
            let up = Vec3::Y;
            let tangent = delta.cross(up).normalize_or(Vec3::X);
            pool.velocities[index] += tangent * self.strength * falloff * dt;
        }
    }
}

/// Kaos/turbülans gücü — basit Perlin tarzı gürültü.
#[derive(Debug, Clone, Default)]
pub struct TurbulenceForce {
    pub frequency: Vec3,
    pub amplitude: f32,
    pub seed: u64,
}

impl TurbulenceForce {
    pub fn sample_noise(&self, p: Vec3, seed: u64) -> Vec3 {
        let mut rng = RandomSeed::new(seed.wrapping_add((p.x * 100.0) as u64)).into_rng();
        let x = rng.gen_range(-1.0..1.0);
        let y = rng.gen_range(-1.0..1.0);
        let z = rng.gen_range(-1.0..1.0);
        Vec3::new(x, y, z) * self.amplitude
    }

    pub fn apply(&self, pool: &mut ParticlePool, index: usize, seed: u32, dt: f32) {
        let n = self.sample_noise(pool.positions[index], self.seed.wrapping_add(seed as u64));
        pool.velocities[index] += n * dt;
    }
}

// ---------------------------------------------------------------------------
// ÇARPIŞMA (Collision)
// ---------------------------------------------------------------------------

/// Çarpışma yanıtı.
#[derive(Debug, Clone, Copy, Default)]
pub struct CollisionResponse {
    pub bounce: f32,
    pub friction: f32,
    pub lifetime_loss: f32,
}

/// Dünya çarpışması — basit AABB tabanlı.
#[derive(Debug, Clone, Default)]
pub struct WorldCollision {
    pub aabbs: Vec<Aabb>,
    pub response: CollisionResponse,
}

impl WorldCollision {
    pub fn apply(&self, pool: &mut ParticlePool, index: usize) {
        for aabb in &self.aabbs {
            if aabb.contains_point(pool.positions[index]) {
                let center = aabb.center();
                let normal = (pool.positions[index] - center).normalize_or(Vec3::Y);
                let dot = pool.velocities[index].dot(normal);
                if dot < 0.0 {
                    pool.velocities[index] -= (1.0 + self.response.bounce) * dot * normal;
                    pool.velocities[index] *= 1.0 - self.response.friction;
                    pool.lifetimes[index] -= self.response.lifetime_loss;
                }
            }
        }
    }
}

/// Derinlik tamponu çarpışması — örnekleme tabanlı.
#[derive(Debug, Clone, Default)]
pub struct DepthBufferCollision {
    pub depth_texture: Option<TextureHandle>,
    pub response: CollisionResponse,
    pub world_to_screen: Mat4,
    pub screen_size: UVec2,
}

impl DepthBufferCollision {
    pub fn apply(&self, _pool: &mut ParticlePool, _index: usize) {
        // Derinlik tamponu okuma ve ekran-uzayı dönüşümü burada uygulanır.
        // Gerçek uygulama render hedefine bağlıdır.
    }
}

// ---------------------------------------------------------------------------
// ALT EMİTTERLER (Sub-emitters)
// ---------------------------------------------------------------------------

/// Alt emöter tetikleme kuralı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubEmitterTrigger {
    #[default]
    None,
    OnSpawn,
    OnDeath,
    OnCollision,
}

/// Alt emöter — bir parçacık olayında yeni efektler üretir.
#[derive(Debug, Clone, Default)]
pub struct SubEmitter {
    pub trigger: SubEmitterTrigger,
    pub effect_template_index: u32,
    pub probability: f32,
    pub count: u32,
}

// ---------------------------------------------------------------------------
// PARÇACIK SİSTEMİ — Yapılandırma
// ---------------------------------------------------------------------------

/// Emisyon modu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmissionMode {
    #[default]
    Continuous,
    Burst,
    Distance,
    Collision,
}

/// Tek emisyon olayı — burst için.
#[derive(Debug, Clone, Copy, Default)]
pub struct BurstEmission {
    pub time: f32,
    pub count: u32,
}

/// Uzaklık emisyonu — her x birimde yeni parçacık.
#[derive(Debug, Clone, Default)]
pub struct DistanceEmission {
    pub distance: f32,
    pub count: u32,
}

/// Çarpışma emisyonu — çarpışma anında parçacık üret.
#[derive(Debug, Clone, Default)]
pub struct CollisionEmission {
    pub min_impulse: f32,
    pub count: u32,
}

/// Parçacık sistemi LOD ayarları.
#[derive(Debug, Clone, Default)]
pub struct ParticleLOD {
    pub high_distance: f32,
    pub medium_distance: f32,
    pub low_distance: f32,
    pub max_particles_high: u32,
    pub max_particles_medium: u32,
    pub max_particles_low: u32,
}

/// Parçacık sistemi yapılandırması.
#[derive(Debug, Clone, Default)]
pub struct ParticleSystemDesc {
    pub name: String,
    pub max_particles: u32,
    pub emission_mode: EmissionMode,
    pub rate_per_second: f32,
    pub burst_emissions: Vec<BurstEmission>,
    pub distance_emission: Option<DistanceEmission>,
    pub collision_emission: Option<CollisionEmission>,
    pub initial_velocity: Vec3,
    pub initial_velocity_random: Vec3,
    pub lifetime_min: f32,
    pub lifetime_max: f32,
    pub size_min: Vec2,
    pub size_max: Vec2,
    pub color_start: Vec4,
    pub color_end: Vec4,
    pub gravity: Vec3,
    pub drag: f32,
    pub vortex: Option<VortexForce>,
    pub turbulence: Option<TurbulenceForce>,
    pub world_collision: Option<WorldCollision>,
    pub depth_collision: Option<DepthBufferCollision>,
    pub velocity_over_lifetime: Option<VelocityOverLifetime>,
    pub color_over_lifetime: Option<ColorOverLifetime>,
    pub size_over_lifetime: Option<SizeOverLifetime>,
    pub rotation_over_lifetime: Option<RotationOverLifetime>,
    pub sub_emitters: Vec<SubEmitter>,
    pub lod: ParticleLOD,
    pub loop_enabled: bool,
    pub prewarm: bool,
    pub prewarm_steps: u32,
    pub seed: u64,
}

// ---------------------------------------------------------------------------
// PARÇACIK SİSTEMİ — Çalışma Zamanı
// ---------------------------------------------------------------------------

/// Aktif parçacık sistemi örneği.
#[derive(Debug, Clone, Default)]
pub struct ParticleSystem {
    pub desc: Arc<ParticleSystemDesc>,
    pub pool: Arc<RwLock<ParticlePool>>,
    pub transform: Mat4,
    pub enabled: bool,
    pub time_since_last_emission: f32,
    pub distance_since_last_emission: f32,
    pub local_seed: RandomSeed,
    pub active: bool,
    pub current_lod: u32,
}

impl ParticleSystem {
    pub fn new(desc: ParticleSystemDesc) -> Self {
        let pool = Arc::new(RwLock::new(ParticlePool::new(desc.max_particles as usize)));
        let local_seed = RandomSeed::new(desc.seed);
        let active = !desc.prewarm;
        Self {
            desc: Arc::new(desc),
            pool,
            transform: Mat4::IDENTITY,
            enabled: true,
            time_since_last_emission: 0.0,
            distance_since_last_emission: 0.0,
            local_seed,
            active,
            current_lod: 0,
        }
    }

    pub fn set_transform(&mut self, transform: Mat4) {
        self.transform = transform;
    }

    /// Havuzun indeks numaralı parçacığını oluşturur.
    fn spawn_into_pool(&mut self, pool: &mut ParticlePool) -> Option<usize> {
        let desc = self.desc.as_ref();
        let seed = self.local_seed.next_u64();
        let mut rng = RandomSeed::new(seed).into_rng();
        let lifetime = rng.gen_range(desc.lifetime_min..desc.lifetime_max);
        let size = Vec2::new(
            rng.gen_range(desc.size_min.x..desc.size_max.x),
            rng.gen_range(desc.size_min.y..desc.size_max.y),
        );
        let vel_rnd = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        );
        let position = self.transform.col(3).truncate();
        let velocity = desc.initial_velocity + vel_rnd * desc.initial_velocity_random;
        let p = Particle {
            position,
            velocity,
            acceleration: Vec3::ZERO,
            color: desc.color_start,
            size,
            rotation: 0.0,
            lifetime,
            age: 0.0,
            alive: true,
            seed: self.local_seed.next_u32(),
            padding: 0,
        };
        pool.spawn(&p)
    }

    pub fn update(&mut self, delta_time: f32, _camera_position: Vec3) {
        if !self.enabled || !self.active {
            return;
        }
        let desc = self.desc.as_ref();
        let mut pool = self.pool.write();

        // Emisyon
        match desc.emission_mode {
            EmissionMode::Continuous => {
                self.time_since_last_emission += delta_time;
                let interval = 1.0 / desc.rate_per_second.max(1e-6);
                while self.time_since_last_emission >= interval {
                    self.time_since_last_emission -= interval;
                    if let Some(idx) = self.spawn_into_pool(&mut pool) {
                        let _ = idx;
                    }
                }
            }
            EmissionMode::Burst => {
                let current = self.time_since_last_emission;
                let next = current + delta_time;
                for burst in &desc.burst_emissions {
                    if current < burst.time && next >= burst.time {
                        for _ in 0..burst.count {
                            let _ = self.spawn_into_pool(&mut pool);
                        }
                    }
                }
                self.time_since_last_emission = next;
            }
            EmissionMode::Distance => {
                self.distance_since_last_emission += delta_time;
                if let Some(dist) = desc.distance_emission {
                    let interval = 1.0 / dist.distance.max(1e-6);
                    while self.distance_since_last_emission >= interval {
                        self.distance_since_last_emission -= interval;
                        let _ = self.spawn_into_pool(&mut pool);
                    }
                }
            }
            EmissionMode::Collision => {
                // Çarpışma tetiklemesi dinamik olarak çağrılır; burada pasif.
            }
        }

        let count = pool.capacity;

        // Parçacıkları güncelle
        for i in 0..count {
            if !pool.alive_flags[i] {
                continue;
            }
            let t = (pool.ages[i] / pool.lifetimes[i]).clamp(0.0, 1.0);
            let seed = pool.seeds[i];
            pool.accelerations[i] = Vec3::ZERO;

            // Kuvvetler
            let gravity = GravityForce { gravity: desc.gravity };
            gravity.apply(&mut pool, i, seed, delta_time);

            if let Some(vortex) = &desc.vortex {
                vortex.apply(&mut pool, i, seed, delta_time);
            }
            let drag = DragForce { drag_coefficient: desc.drag };
            drag.apply(&mut pool, i, seed, delta_time);

            if let Some(turb) = &desc.turbulence {
                turb.apply(&mut pool, i, seed, delta_time);
            }

            // Hız zaman içinde modülü
            if let Some(ref vel_mod) = desc.velocity_over_lifetime {
                vel_mod.apply(&mut pool, i, t, seed);
            }

            // Renk / boyut / dönüş modülleri
            if let Some(ref col_mod) = desc.color_over_lifetime {
                col_mod.apply(&mut pool, i, t);
            }
            if let Some(ref size_mod) = desc.size_over_lifetime {
                size_mod.apply(&mut pool, i, t);
            }
            if let Some(ref rot_mod) = desc.rotation_over_lifetime {
                rot_mod.apply(&mut pool, i, t, delta_time);
            }

            // Konum güncelle
            pool.positions[i] += pool.velocities[i] * delta_time;
            pool.ages[i] += delta_time;

            // Yaşam süresi dolunca öldür / loop
            if pool.ages[i] >= pool.lifetimes[i] {
                if desc.loop_enabled {
                    pool.ages[i] = 0.0;
                    pool.positions[i] = self.transform.col(3).truncate();
                    pool.velocities[i] = desc.initial_velocity;
                    pool.colors[i] = desc.color_start;
                    pool.sizes[i] = Vec2::new(
                        rng_float(seed, desc.size_min.x, desc.size_max.x),
                        rng_float(seed.wrapping_add(1), desc.size_min.y, desc.size_max.y),
                    );
                } else {
                    pool.alive_flags[i] = false;
                }
            }
        }

        // Dünya çarpışması
        if let Some(ref wc) = desc.world_collision {
            for i in 0..count {
                if pool.alive_flags[i] {
                    wc.apply(&mut pool, i);
                }
            }
        }

        // Derinlik tamponu çarpışması
        if let Some(ref dc) = desc.depth_collision {
            for i in 0..count {
                if pool.alive_flags[i] {
                    dc.apply(&mut pool, i);
                }
            }
        }

        // Alt emiter tetikleme (basit: ölüm anında)
        for i in 0..count {
            if pool.alive_flags[i] && pool.ages[i] >= pool.lifetimes[i] {
                for sub in &desc.sub_emitters {
                    if matches!(sub.trigger, SubEmitterTrigger::OnDeath) {
                        let mut rng = self.local_seed.clone().into_rng();
                        if rng.gen_range(0.0..1.0) < sub.probability {
                            for _ in 0..sub.count {
                                let _ = self.spawn_into_pool(&mut pool);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn alive_count(&self) -> usize {
        self.pool.read().alive_count()
    }

    pub fn reset(&mut self) {
        self.pool.write().clear();
        self.time_since_last_emission = 0.0;
        self.distance_since_last_emission = 0.0;
        self.local_seed = RandomSeed::new(self.desc.seed);
    }
}

/// Seed tabanlı basit float üretici.
fn rng_float(seed: u64, min: f32, max: f32) -> f32 {
    let mut rng = RandomSeed::new(seed).into_rng();
    rng.gen_range(min..max)
}

// ---------------------------------------------------------------------------
// YOL / RIBBON — TrailRenderer
// ---------------------------------------------------------------------------

/// Yol noktası — zaman içinde konum ve genişlik.
#[derive(Debug, Clone, Default)]
pub struct TrailPoint {
    pub position: Vec3,
    pub width: f32,
    pub color: Vec4,
    pub age: f32,
    pub lifetime: f32,
}

/// Trail/Ribbon oluşturucu.
#[derive(Debug, Clone, Default)]
pub struct TrailRenderer {
    pub points: Vec<TrailPoint>,
    pub max_points: usize,
    pub width_over_lifetime: FloatCurve,
    pub color_over_lifetime: ColorCurve,
    pub min_distance: f32,
    pub loop_enabled: bool,
    pub smooth: bool,
    pub subdivision: u32,
    pub last_position: Option<Vec3>,
}

impl TrailRenderer {
    pub fn new(max_points: usize) -> Self {
        Self {
            max_points,
            min_distance: 0.1,
            smooth: true,
            subdivision: 2,
            ..Default::default()
        }
    }

    pub fn push_point(&mut self, pos: Vec3) {
        if let Some(last) = self.last_position {
            if pos.distance_squared(last) < self.min_distance.powi(2) {
                return;
            }
        }
        self.points.push(TrailPoint {
            position: pos,
            width: 1.0,
            color: Vec4::ONE,
            age: 0.0,
            lifetime: 1.0,
        });
        if self.points.len() > self.max_points {
            self.points.remove(0);
        }
        self.last_position = Some(pos);
    }

    pub fn update(&mut self, delta_time: f32) {
        for pt in &mut self.points {
            pt.age += delta_time;
        }
        self.points.retain(|pt| pt.age < pt.lifetime);
    }

    /// Ribbon üretir — iki dizi (sol kenar, sağ kenar veya sıralı üçgenler).
    pub fn generate_ribbon(&self) -> Vec<Vec3> {
        if self.points.len() < 2 {
            return Vec::new();
        }
        let mut verts = Vec::new();
        for i in 0..self.points.len() - 1 {
            let p0 = &self.points[i];
            let p1 = &self.points[i + 1];
            let t0 = (p0.age / p0.lifetime).clamp(0.0, 1.0);
            let t1 = (p1.age / p1.lifetime).clamp(0.0, 1.0);
            let w0 = p0.width * self.width_over_lifetime.sample(t0);
            let w1 = p1.width * self.width_over_lifetime.sample(t1);
            let dir = (p1.position - p0.position).normalize_or(Vec3::Z);
            let right = dir.cross(Vec3::Y).normalize_or(Vec3::X);
            for s in 0..=self.subdivision {
                let f = s as f32 / self.subdivision.max(1) as f32;
                let pos = p0.position.lerp(p1.position, f);
                let width = w0.lerp(w1, f);
                let offset = right * width * 0.5;
                verts.push(pos + offset);
                verts.push(pos - offset);
            }
        }
        verts
    }
}

// ---------------------------------------------------------------------------
// DECAL SİSTEMİ
// ---------------------------------------------------------------------------

/// Dekal projeksiyon modu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DecalProjectionMode {
    #[default]
    World,
    Oriented,
}

/// Dekal — bir yüzeye yansıtılan etki.
#[derive(Debug, Clone, Default)]
pub struct Decal {
    pub position: Vec3,
    pub rotation: Quat,
    pub size: Vec3,
    pub color: Vec4,
    pub lifetime: f32,
    pub age: f32,
    pub material_override: Option<u64>,
    pub projection_mode: DecalProjectionMode,
    pub fade_in: f32,
    pub fade_out: f32,
    pub alive: bool,
}

impl Decal {
    pub fn is_alive(&self) -> bool {
        self.alive && self.age < self.lifetime
    }

    pub fn alpha(&self) -> f32 {
        let t = (self.age / self.lifetime.max(1e-6)).clamp(0.0, 1.0);
        if t < self.fade_in {
            t / self.fade_in
        } else if t > 1.0 - self.fade_out {
            (1.0 - t) / self.fade_out
        } else {
            1.0
        }
    }

    pub fn projection_matrix(&self) -> Mat4 {
        let scale = Mat4::from_scale(self.size);
        let rot = Mat4::from_quat(self.rotation);
        let trans = Mat4::from_translation(self.position);
        trans * rot * scale
    }
}

/// Dekal havuzu.
#[derive(Debug, Clone, Default)]
pub struct DecalPool {
    pub decals: Vec<Decal>,
    pub capacity: usize,
}

impl DecalPool {
    pub fn new(capacity: usize) -> Self {
        Self {
            decals: (0..capacity).map(|_| Decal::default()).collect(),
            capacity,
        }
    }

    pub fn spawn(&mut self, decal: Decal) -> Option<usize> {
        for i in 0..self.capacity {
            if !self.decals[i].alive {
                self.decals[i] = decal;
                self.decals[i].alive = true;
                return Some(i);
            }
        }
        None
    }

    pub fn update(&mut self, delta_time: f32) {
        for decal in &mut self.decals {
            if decal.alive {
                decal.age += delta_time;
                if decal.age >= decal.lifetime {
                    decal.alive = false;
                }
            }
        }
    }

    pub fn alive_decal_indices(&self) -> Vec<usize> {
        self.decals
            .iter()
            .enumerate()
            .filter(|(_, d)| d.alive)
            .map(|(i, _)| i)
            .collect()
    }
}

/// Dekal sistemi.
#[derive(Debug, Clone, Default)]
pub struct DecalSystem {
    pub pool: DecalPool,
    pub max_decal_distance: f32,
}

impl DecalSystem {
    pub fn new(max_decals: usize) -> Self {
        Self {
            pool: DecalPool::new(max_decals),
            max_decal_distance: 50.0,
        }
    }

    pub fn add_decal(&mut self, decal: Decal) -> Option<usize> {
        self.pool.spawn(decal)
    }

    pub fn update(&mut self, delta_time: f32) {
        self.pool.update(delta_time);
    }

    /// Dünya uzayında yüzey normaline göre dekal projeksiyonu üretir.
    pub fn project_world(&self, position: Vec3, size: Vec3, normal: Vec3) -> Mat4 {
        let right = normal.cross(Vec3::Y).normalize_or(Vec3::X);
        let up = right.cross(normal).normalize();
        let scale = Mat4::from_scale(size);
        let rot = Mat4::from_cols(
            right.extend(0.0),
            up.extend(0.0),
            normal.extend(0.0),
            Vec4::ZERO,
        );
        let trans = Mat4::from_translation(position);
        trans * rot * scale
    }
}

// ---------------------------------------------------------------------------
// IŞIK ŞAFTLARI / GOD RAYS
// ---------------------------------------------------------------------------

/// Hacimsel ışık şaftı.
#[derive(Debug, Clone, Default)]
pub struct VolumetricLightShaft {
    pub origin: Vec3,
    pub direction: Vec3,
    pub length: f32,
    pub radius: f32,
    pub density: f32,
    pub intensity: f32,
    pub color: Vec3,
    pub cookie_texture: Option<TextureHandle>,
    pub enabled: bool,
}

impl VolumetricLightShaft {
    pub fn update(&mut self, delta_time: f32) {
        if self.enabled {
            let _ = delta_time;
        }
    }
}

/// Ekran-uzayı ışık şaftları (God Rays).
#[derive(Debug, Clone, Default)]
pub struct ScreenSpaceLightShafts {
    pub enabled: bool,
    pub intensity: f32,
    pub decay: f32,
    pub density: f32,
    pub weight: f32,
    pub exposure: f32,
    pub num_samples: u32,
    pub light_screen_position: Vec2,
    pub cookie: Option<TextureHandle>,
}

impl ScreenSpaceLightShafts {
    pub fn new() -> Self {
        Self {
            enabled: true,
            num_samples: 60,
            intensity: 1.0,
            decay: 0.95,
            density: 0.9,
            weight: 0.3,
            exposure: 0.25,
            light_screen_position: Vec2::new(0.5, 0.5),
            cookie: None,
        }
    }
}

// ---------------------------------------------------------------------------
// HAVA EFEKTLERİ — Yağmur, Kar, Şimşek, Sis
// ---------------------------------------------------------------------------

/// Yağmur sistemi.
#[derive(Debug, Clone, Default)]
pub struct RainSystem {
    pub particle_system: ParticleSystem,
    pub wind: Vec3,
    pub splash_chance: f32,
    pub max_splashes: u32,
    pub intensity: f32,
}

impl RainSystem {
    pub fn new(desc: ParticleSystemDesc) -> Self {
        let ps = ParticleSystem::new(desc);
        Self {
            particle_system: ps,
            wind: Vec3::new(2.0, 0.0, 0.0),
            splash_chance: 0.1,
            max_splashes: 64,
            intensity: 1.0,
        }
    }

    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity;
    }

    pub fn update(&mut self, delta_time: f32, camera_position: Vec3) {
        self.particle_system.update(delta_time, camera_position);
    }
}

/// Kar sistemi.
#[derive(Debug, Clone, Default)]
pub struct SnowSystem {
    pub particle_system: ParticleSystem,
    pub wind: Vec3,
    pub turbulence_scale: f32,
    pub turbulence_strength: f32,
    pub intensity: f32,
}

impl SnowSystem {
    pub fn new(desc: ParticleSystemDesc) -> Self {
        let ps = ParticleSystem::new(desc);
        Self {
            particle_system: ps,
            wind: Vec3::ZERO,
            turbulence_scale: 1.0,
            turbulence_strength: 0.5,
            intensity: 1.0,
        }
    }

    pub fn update(&mut self, delta_time: f32, camera_position: Vec3) {
        self.particle_system.update(delta_time, camera_position);
    }
}

/// Şimşek dalı.
#[derive(Debug, Clone, Default)]
pub struct LightningBolt {
    pub start: Vec3,
    pub end: Vec3,
    pub segments: u32,
    pub thickness: f32,
    pub color: Vec3,
    pub lifetime: f32,
    pub age: f32,
    pub alive: bool,
    pub branches: Vec<LightningBolt>,
}

/// Şimşek sistemi.
#[derive(Debug, Clone, Default)]
pub struct LightningSystem {
    pub bolts: Vec<LightningBolt>,
    pub max_bolts: usize,
    pub frequency: f32,
    pub time_since_last: f32,
    pub seed: u64,
}

impl LightningSystem {
    pub fn new(max_bolts: usize) -> Self {
        Self {
            max_bolts,
            frequency: 0.2,
            time_since_last: 0.0,
            seed: 7,
            ..Default::default()
        }
    }

    pub fn spawn_bolt(&mut self, start: Vec3, end: Vec3) {
        if self.bolts.len() >= self.max_bolts {
            self.bolts.remove(0);
        }
        let mut bolt = LightningBolt {
            start,
            end,
            segments: 8,
            thickness: 0.5,
            color: Vec3::new(0.7, 0.8, 1.0),
            lifetime: 0.2,
            age: 0.0,
            alive: true,
            branches: Vec::new(),
        };
        let mut rng = RandomSeed::new(self.seed).into_rng();
        if rng.gen_bool(0.4) {
            let branch_end = start + (end - start) * rng.gen_range(0.4..0.8)
                + Vec3::new(rng.gen_range(-2.0..2.0), 0.0, rng.gen_range(-2.0..2.0));
            bolt.branches.push(LightningBolt {
                start,
                end: branch_end,
                segments: 4,
                thickness: 0.25,
                color: Vec3::new(0.6, 0.7, 1.0),
                lifetime: 0.15,
                age: 0.0,
                alive: true,
                branches: Vec::new(),
            });
        }
        self.bolts.push(bolt);
    }

    pub fn update(&mut self, delta_time: f32) {
        self.time_since_last += delta_time;
        for bolt in &mut self.bolts {
            bolt.age += delta_time;
            for branch in &mut bolt.branches {
                branch.age += delta_time;
            }
        }
        self.bolts.retain(|b| b.alive);
    }
}

/// Hacimsel hava sis/sıcaklık sistemi.
#[derive(Debug, Clone, Default)]
pub struct VolumetricWeatherFog {
    pub density: f32,
    pub scattering: Vec3,
    pub extinction: Vec3,
    pub light_anisotropy: f32,
    pub max_distance: f32,
}

impl VolumetricWeatherFog {
    pub fn new() -> Self {
        Self {
            density: 0.02,
            scattering: Vec3::new(1.0, 1.0, 1.0),
            extinction: Vec3::new(1.0, 1.0, 1.0),
            light_anisotropy: 0.8,
            max_distance: 100.0,
        }
    }
}

// ---------------------------------------------------------------------------
// SON İŞLEM EFEKTLERİ
// ---------------------------------------------------------------------------

/// Lens flaş (Lens Flare).
#[derive(Debug, Clone, Default)]
pub struct LensFlare {
    pub enabled: bool,
    pub intensity: f32,
    pub threshold: f32,
    pub chromatic_aberration: f32,
    pub ghost_count: u32,
    pub halo_radius: f32,
    pub artifacts: Vec<TextureHandle>,
}

impl LensFlare {
    pub fn new() -> Self {
        Self {
            ghost_count: 5,
            halo_radius: 0.4,
            ..Default::default()
        }
    }
}

/// Kromatik aberasyon.
#[derive(Debug, Clone, Default)]
pub struct ChromaticAberration {
    pub enabled: bool,
    pub intensity: f32,
    pub radial_strength: f32,
    pub direction: Vec2,
}

impl ChromaticAberration {
    pub fn new() -> Self {
        Self {
            intensity: 0.01,
            radial_strength: 1.0,
            direction: Vec2::ZERO,
            ..Default::default()
        }
    }
}

/// Vinjet (Vignette).
#[derive(Debug, Clone, Default)]
pub struct Vignette {
    pub enabled: bool,
    pub intensity: f32,
    pub slope: f32,
    pub color: Vec3,
    pub rounded: bool,
}

impl Vignette {
    pub fn new() -> Self {
        Self {
            intensity: 0.5,
            slope: 2.0,
            color: Vec3::ZERO,
            rounded: false,
            ..Default::default()
        }
    }
}

/// Film tanesi (Film Grain).
#[derive(Debug, Clone, Default)]
pub struct FilmGrain {
    pub enabled: bool,
    pub intensity: f32,
    pub speed: f32,
    pub colored: bool,
    pub seed: u64,
}

impl FilmGrain {
    pub fn new() -> Self {
        Self {
            intensity: 0.08,
            speed: 1.0,
            colored: false,
            seed: 13,
            ..Default::default()
        }
    }
}

/// Renk düzeltmesi (Color Grading).
#[derive(Debug, Clone, Default)]
pub struct ColorGrading {
    pub enabled: bool,
    pub temperature: f32,
    pub tint: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub brightness: f32,
    pub lift: Vec3,
    pub gamma: Vec3,
    pub gain: Vec3,
    pub shadows_color: Vec3,
    pub midtones_color: Vec3,
    pub highlights_color: Vec3,
}

impl ColorGrading {
    pub fn new() -> Self {
        Self {
            temperature: 0.0,
            tint: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            brightness: 0.0,
            lift: Vec3::ZERO,
            gamma: Vec3::ONE,
            gain: Vec3::ONE,
            shadows_color: Vec3::ZERO,
            midtones_color: Vec3::ZERO,
            highlights_color: Vec3::ZERO,
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// VFX PIPELINE — Tüm efektlerin koordinasyonu
// ---------------------------------------------------------------------------

/// Tüm VFX sistemini koordine eden ana pipeline.
#[derive(Debug, Clone, Default)]
pub struct VfxPipeline {
    pub particle_systems: Vec<ParticleSystem>,
    pub trail_renderers: Vec<TrailRenderer>,
    pub decal_system: DecalSystem,
    pub light_shafts: Vec<VolumetricLightShaft>,
    pub screen_shafts: ScreenSpaceLightShafts,
    pub rain_system: Option<RainSystem>,
    pub snow_system: Option<SnowSystem>,
    pub lightning_system: LightningSystem,
    pub volumetric_fog: VolumetricWeatherFog,
    pub lens_flare: LensFlare,
    pub chromatic_aberration: ChromaticAberration,
    pub vignette: Vignette,
    pub film_grain: FilmGrain,
    pub color_grading: ColorGrading,
    pub time: f32,
    pub max_total_particles: usize,
    pub max_decals: usize,
    pub enabled: bool,
}

impl VfxPipeline {
    pub fn new(max_total_particles: usize, max_decals: usize) -> Self {
        Self {
            particle_systems: Vec::new(),
            trail_renderers: Vec::new(),
            decal_system: DecalSystem::new(max_decals),
            light_shafts: Vec::new(),
            screen_shafts: ScreenSpaceLightShafts::new(),
            rain_system: None,
            snow_system: None,
            lightning_system: LightningSystem::new(16),
            volumetric_fog: VolumetricWeatherFog::new(),
            lens_flare: LensFlare::new(),
            chromatic_aberration: ChromaticAberration::new(),
            vignette: Vignette::new(),
            film_grain: FilmGrain::new(),
            color_grading: ColorGrading::new(),
            time: 0.0,
            max_total_particles,
            max_decals,
            enabled: true,
        }
    }

    pub fn add_particle_system(&mut self, ps: ParticleSystem) {
        self.particle_systems.push(ps);
    }

    pub fn add_trail_renderer(&mut self, tr: TrailRenderer) {
        self.trail_renderers.push(tr);
    }

    pub fn add_light_shaft(&mut self, shaft: VolumetricLightShaft) {
        self.light_shafts.push(shaft);
    }

    pub fn enable_rain(&mut self, desc: ParticleSystemDesc) {
        self.rain_system = Some(RainSystem::new(desc));
    }

    pub fn enable_snow(&mut self, desc: ParticleSystemDesc) {
        self.snow_system = Some(SnowSystem::new(desc));
    }

    pub fn spawn_lightning(&mut self, start: Vec3, end: Vec3) {
        self.lightning_system.spawn_bolt(start, end);
    }

    pub fn update(&mut self, delta_time: f32, camera_position: Vec3) {
        if !self.enabled {
            return;
        }
        self.time += delta_time;
        for ps in &mut self.particle_systems {
            ps.update(delta_time, camera_position);
        }
        for tr in &mut self.trail_renderers {
            tr.update(delta_time);
        }
        self.decal_system.update(delta_time);
        for shaft in &mut self.light_shafts {
            shaft.update(delta_time);
        }
        if let Some(ref mut rain) = self.rain_system {
            rain.update(delta_time, camera_position);
        }
        if let Some(ref mut snow) = self.snow_system {
            snow.update(delta_time, camera_position);
        }
        self.lightning_system.update(delta_time);
    }

    pub fn total_particle_count(&self) -> usize {
        self.particle_systems
            .iter()
            .map(|ps| ps.alive_count())
            .sum()
    }

    pub fn reset(&mut self) {
        for ps in &mut self.particle_systems {
            ps.reset();
        }
        for tr in &mut self.trail_renderers {
            tr.points.clear();
        }
        self.decal_system = DecalSystem::new(self.max_decals);
        self.light_shafts.clear();
        self.rain_system = None;
        self.snow_system = None;
        self.lightning_system = LightningSystem::new(16);
        self.time = 0.0;
    }
}

// ---------------------------------------------------------------------------
// EFEKT EDİTÖRÜ — Cascade benzeri grafik editör
// ---------------------------------------------------------------------------

/// Modül çıktısı — bir parçacık modülünün bir anlık değeri.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModuleOutput {
    Float(f32),
    Vec3(Vec3),
    Vec4(Vec4),
    Bool(bool),
}

/// Efekt modülü — editördeki bir düğüm.
#[derive(Debug, Clone, Default)]
pub struct EffectModule {
    pub id: u32,
    pub name: String,
    pub module_type: EffectModuleType,
    pub enabled: bool,
    pub inputs: HashMap<String, ModuleOutput>,
    pub outputs: Vec<String>,
}

/// Modül türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EffectModuleType {
    #[default]
    Spawn,
    VelocityOverLifetime,
    ColorOverLifetime,
    SizeOverLifetime,
    RotationOverLifetime,
    Force,
    Collision,
    SubEmitter,
    Render,
}

/// Efekt grafiği — modüller ve bağlantılar.
#[derive(Debug, Clone, Default)]
pub struct EffectGraph {
    pub modules: Vec<EffectModule>,
    pub connections: Vec<(u32, String, u32, String)>,
    pub seed: RandomSeed,
    pub loop_enabled: bool,
    pub duration: f32,
}

impl EffectGraph {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            connections: Vec::new(),
            seed: RandomSeed::new(42),
            loop_enabled: true,
            duration: 5.0,
        }
    }

    pub fn add_module(&mut self, module: EffectModule) -> u32 {
        let id = module.id;
        self.modules.push(module);
        id
    }

    pub fn connect(&mut self, from: u32, from_output: &str, to: u32, to_input: &str) {
        self.connections
            .push((from, from_output.into(), to, to_input.into()));
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.modules.is_empty() {
            return Err("Effect graph has no modules".into());
        }
        Ok(())
    }
}

/// Efekt şablonu — kaydedilebilir VFX tanımı.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EffectTemplate {
    pub name: String,
    pub graph: EffectGraph,
    pub desc: ParticleSystemDesc,
}

// ---------------------------------------------------------------------------
// VFX EDİTÖRÜ — Gerçek Zaman Önizleme + Parametre Eğrileri
// ---------------------------------------------------------------------------

/// Efekt editör durumu.
#[derive(Debug, Clone, Default)]
pub struct EffectEditorState {
    pub selected_module: Option<u32>,
    pub preview_playing: bool,
    pub preview_time: f32,
    pub preview_seed: RandomSeed,
    pub curve_editor_open: bool,
    pub selected_curve: Option<String>,
}

impl EffectEditorState {
    pub fn new() -> Self {
        Self {
            preview_seed: RandomSeed::new(42),
            preview_playing: true,
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// ECS BİLEŞENLERİ — Oyun tarafından doğrudan kullanılabilir.
// ---------------------------------------------------------------------------

/// VFX komponenti — bir nesneye eklenebilir.
#[derive(Debug, Clone, Default)]
pub struct VfxComponent {
    pub particle_systems: Vec<ParticleSystem>,
    pub trails: Vec<TrailRenderer>,
    pub decals: Vec<Decal>,
    pub light_shafts: Vec<VolumetricLightShaft>,
    pub enabled: bool,
}

impl VfxComponent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_particle_system(&mut self, ps: ParticleSystem) {
        self.particle_systems.push(ps);
    }

    pub fn add_trail(&mut self, trail: TrailRenderer) {
        self.trails.push(trail);
    }

    pub fn add_light_shaft(&mut self, shaft: VolumetricLightShaft) {
        self.light_shafts.push(shaft);
    }

    pub fn update(&mut self, delta_time: f32, camera_position: Vec3) {
        if !self.enabled {
            return;
        }
        for ps in &mut self.particle_systems {
            ps.update(delta_time, camera_position);
        }
        for tr in &mut self.trails {
            tr.update(delta_time);
        }
    }
}

// ---------------------------------------------------------------------------
// Dışa Açılan API (Kullanıcı dostu)
// ---------------------------------------------------------------------------

/// VFX yardımcıları — hızlı efekt oluşturma.
pub mod presets {
    use super::*;

    /// Basit ateş efekti tanımı.
    pub fn fire() -> ParticleSystemDesc {
        ParticleSystemDesc {
            name: "Fire".into(),
            max_particles: 512,
            emission_mode: EmissionMode::Continuous,
            rate_per_second: 80.0,
            lifetime_min: 0.5,
            lifetime_max: 1.2,
            initial_velocity: Vec3::Y * 2.0,
            initial_velocity_random: Vec3::new(1.0, 0.5, 1.0),
            color_start: Vec4::new(1.0, 0.9, 0.3, 1.0),
            color_end: Vec4::new(0.6, 0.1, 0.0, 0.0),
            size_min: Vec2::new(0.2, 0.2),
            size_max: Vec2::new(0.5, 0.5),
            gravity: Vec3::ZERO,
            drag: 0.5,
            turbulence: Some(TurbulenceForce {
                frequency: Vec3::new(2.0, 2.0, 2.0),
                amplitude: 1.5,
                seed: 101,
            }),
            ..Default::default()
        }
    }

    /// Basit patlama efekti tanımı.
    pub fn explosion() -> ParticleSystemDesc {
        ParticleSystemDesc {
            name: "Explosion".into(),
            max_particles: 1024,
            emission_mode: EmissionMode::Burst,
            rate_per_second: 0.0,
            burst_emissions: vec![BurstEmission { time: 0.0, count: 200 }],
            lifetime_min: 0.3,
            lifetime_max: 1.5,
            initial_velocity: Vec3::ZERO,
            initial_velocity_random: Vec3::splat(15.0),
            color_start: Vec4::new(1.0, 0.8, 0.4, 1.0),
            color_end: Vec4::new(0.2, 0.1, 0.0, 0.0),
            size_min: Vec2::new(0.3, 0.3),
            size_max: Vec2::new(0.8, 0.8),
            gravity: Vec3::ZERO,
            drag: 2.0,
            ..Default::default()
        }
    }

    /// Basit yağmur efekti.
    pub fn rain() -> ParticleSystemDesc {
        ParticleSystemDesc {
            name: "Rain".into(),
            max_particles: 4096,
            emission_mode: EmissionMode::Continuous,
            rate_per_second: 1000.0,
            lifetime_min: 0.5,
            lifetime_max: 1.5,
            initial_velocity: Vec3::new(0.0, -20.0, 0.0),
            initial_velocity_random: Vec3::new(1.0, 1.0, 1.0),
            color_start: Vec4::new(0.7, 0.8, 1.0, 0.6),
            color_end: Vec4::new(0.7, 0.8, 1.0, 0.0),
            size_min: Vec2::new(0.05, 0.4),
            size_max: Vec2::new(0.1, 0.8),
            gravity: Vec3::ZERO,
            drag: 0.0,
            ..Default::default()
        }
    }

    /// Basit kar efekti.
    pub fn snow() -> ParticleSystemDesc {
        ParticleSystemDesc {
            name: "Snow".into(),
            max_particles: 2048,
            emission_mode: EmissionMode::Continuous,
            rate_per_second: 400.0,
            lifetime_min: 4.0,
            lifetime_max: 8.0,
            initial_velocity: Vec3::new(0.0, -1.0, 0.0),
            initial_velocity_random: Vec3::new(0.5, 0.5, 0.5),
            color_start: Vec4::new(1.0, 1.0, 1.0, 1.0),
            color_end: Vec4::new(1.0, 1.0, 1.0, 0.0),
            size_min: Vec2::new(0.05, 0.05),
            size_max: Vec2::new(0.15, 0.15),
            gravity: Vec3::ZERO,
            turbulence: Some(TurbulenceForce {
                frequency: Vec3::new(0.5, 0.5, 0.5),
                amplitude: 0.3,
                seed: 202,
            }),
            ..Default::default()
        }
    }
}
