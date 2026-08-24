use crate::{Component, Transform};
use crate::math::Vec3;
use std::time::Duration;

// Partikül veri yapısı
#[derive(Debug, Clone)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub color: [f32; 4],
    pub size: f32,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub mass: f32,
    pub rotation: f32,
    pub rotation_speed: f32,
}

impl Particle {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::new(0.0, 0.0, 0.0),
            color: [1.0, 1.0, 1.0, 1.0],
            size: 1.0,
            lifetime: 0.0,
            max_lifetime: 5.0,
            mass: 1.0,
            rotation: 0.0,
            rotation_speed: 0.0,
        }
    }
    
    pub fn is_alive(&self) -> bool {
        self.lifetime < self.max_lifetime
    }
    
    pub fn age(&self) -> f32 {
        self.lifetime / self.max_lifetime
    }
    
    pub fn normalized_age(&self) -> f32 {
        self.age().clamp(0.0, 1.0)
    }
}

// Partikül emitör modları
#[derive(Debug, Clone)]
pub enum EmissionMode {
    Continuous { rate: f32 }, // Partiküllerin saniyedeki sayısı
    Burst { count: u32 },     // Tek seferde çıkan partikül sayısı
}

// Emitör şekil modları
#[derive(Debug, Clone)]
pub enum EmitterShape {
    Point,
    Sphere { radius: f32 },
    Hemisphere { radius: f32 },
    Cone { angle: f32, radius: f32 }, // Yön yukarı (+Y)
    Box { size: Vec3 },
    Circle { radius: f32 },
}

// Renk gradient'ı
#[derive(Debug, Clone)]
pub struct ColorGradient {
    pub keys: Vec<(f32, [f32; 4])>, // (zaman, renk) çiftleri
}

impl ColorGradient {
    pub fn new() -> Self {
        Self {
            keys: vec![(0.0, [1.0, 1.0, 1.0, 1.0]), (1.0, [1.0, 1.0, 1.0, 0.0])],
        }
    }
    
    pub fn add_key(&mut self, time: f32, color: [f32; 4]) {
        self.keys.push((time, color));
        self.keys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }
    
    pub fn evaluate(&self, time: f32) -> [f32; 4] {
        let clamped_time = time.clamp(0.0, 1.0);
        
        // Uygun aralığı bul
        for i in 0..self.keys.len() - 1 {
            if clamped_time >= self.keys[i].0 && clamped_time <= self.keys[i + 1].0 {
                let t = (clamped_time - self.keys[i].0) / (self.keys[i + 1].0 - self.keys[i].0);
                
                let c1 = self.keys[i].1;
                let c2 = self.keys[i + 1].1;
                
                return [
                    c1[0] + t * (c2[0] - c1[0]),
                    c1[1] + t * (c2[1] - c1[1]),
                    c1[2] + t * (c2[2] - c1[2]),
                    c1[3] + t * (c2[3] - c1[3]),
                ];
            }
        }
        
        // Varsayılan olarak son renk
        self.keys.last().map(|(_, color)| *color).unwrap_or([1.0, 1.0, 1.0, 1.0])
    }
}

// Boyut eğrisi
#[derive(Debug, Clone)]
pub struct SizeCurve {
    pub keys: Vec<(f32, f32)>, // (zaman, boyut çarpanı)
}

impl SizeCurve {
    pub fn new() -> Self {
        Self {
            keys: vec![(0.0, 1.0), (1.0, 1.0)],
        }
    }
    
    pub fn add_key(&mut self, time: f32, size: f32) {
        self.keys.push((time, size));
        self.keys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }
    
    pub fn evaluate(&self, time: f32) -> f32 {
        let clamped_time = time.clamp(0.0, 1.0);
        
        for i in 0..self.keys.len() - 1 {
            if clamped_time >= self.keys[i].0 && clamped_time <= self.keys[i + 1].0 {
                let t = (clamped_time - self.keys[i].0) / (self.keys[i + 1].0 - self.keys[i].0);
                return self.keys[i].1 + t * (self.keys[i + 1].1 - self.keys[i].1);
            }
        }
        
        self.keys.last().map(|(_, size)| *size).unwrap_or(1.0)
    }
}

// Partikül sistem bileşeni
pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    pub max_particles: usize,
    pub emission_rate: f32, // saniyede
    pub burst_count: u32,
    pub lifetime: f32,
    pub lifetime_randomness: f32,
    pub start_speed: Vec3,
    pub start_speed_randomness: Vec3,
    pub start_size: f32,
    pub start_size_randomness: f32,
    pub start_color: [f32; 4],
    pub start_rotation: f32,
    pub start_rotation_randomness: f32,
    pub gravity: Vec3,
    pub drag: f32,
    pub color_gradient: ColorGradient,
    pub size_curve: SizeCurve,
    pub emitter_shape: EmitterShape,
    pub emission_mode: EmissionMode,
    pub enabled: bool,
    pub simulation_space: SimulationSpace,
    pub last_emit_time: f32,
    pub accumulated_emit: f32,
}

#[derive(Debug, Clone)]
pub enum SimulationSpace {
    Local,  // Transform ile birlikte hareket eder
    World,  // Dünya koordinatlarında kalır
}

impl ParticleSystem {
    pub fn new(max_particles: usize) -> Self {
        Self {
            particles: Vec::with_capacity(max_particles),
            max_particles,
            emission_rate: 10.0,
            burst_count: 10,
            lifetime: 5.0,
            lifetime_randomness: 0.2,
            start_speed: Vec3::new(0.0, 5.0, 0.0),
            start_speed_randomness: Vec3::new(1.0, 1.0, 1.0),
            start_size: 1.0,
            start_size_randomness: 0.3,
            start_color: [1.0, 1.0, 1.0, 1.0],
            start_rotation: 0.0,
            start_rotation_randomness: 0.5,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            drag: 0.01,
            color_gradient: ColorGradient::new(),
            size_curve: SizeCurve::new(),
            emitter_shape: EmitterShape::Sphere { radius: 1.0 },
            emission_mode: EmissionMode::Continuous { rate: 10.0 },
            enabled: true,
            simulation_space: SimulationSpace::Local,
            last_emit_time: 0.0,
            accumulated_emit: 0.0,
        }
    }
    
    pub fn emit_particles(&mut self, transform: &Transform, count: u32) {
        for _ in 0..count {
            if self.particles.len() >= self.max_particles {
                break;
            }
            
            let mut particle = self.create_particle(transform);
            
            // Rastgele yaşam süresi
            let randomness = (rand::random::<f32>() - 0.5) * 2.0 * self.lifetime_randomness;
            particle.max_lifetime = (self.lifetime + randomness).max(0.1);
            
            // Rastgele başlangıç hızı
            let speed_rand = Vec3::new(
                (rand::random::<f32>() - 0.5) * 2.0,
                (rand::random::<f32>() - 0.5) * 2.0,
                (rand::random::<f32>() - 0.5) * 2.0,
            );
            particle.velocity = self.start_speed + self.start_speed_randomness * speed_rand;
            
            // Rastgele başlangıç boyutu
            let size_rand = (rand::random::<f32>() - 0.5) * 2.0 * self.start_size_randomness;
            particle.size = (self.start_size + size_rand).max(0.01);
            
            // Rastgele başlangıç rotasyonu
            let rotation_rand = (rand::random::<f32>() - 0.5) * 2.0 * self.start_rotation_randomness;
            particle.rotation = self.start_rotation + rotation_rand;
            
            self.particles.push(particle);
        }
    }
    
    fn create_particle(&mut self, transform: &Transform) -> Particle {
        let position = match self.emitter_shape {
            EmitterShape::Point => transform.translation,
            EmitterShape::Sphere { radius } => {
                let r = rand::random::<f32>().cbrt() * radius;
                let theta = rand::random::<f32>() * 2.0 * std::f32::consts::PI;
                let phi = (rand::random::<f32>() * 2.0 - 1.0).acos();
                
                let x = r * phi.sin() * theta.cos();
                let y = r * phi.sin() * theta.sin();
                let z = r * phi.cos();
                
                transform.translation + Vec3::new(x, y, z)
            }
            EmitterShape::Hemisphere { radius } => {
                let r = rand::random::<f32>().cbrt() * radius;
                let theta = rand::random::<f32>() * 2.0 * std::f32::consts::PI;
                let phi = rand::random::<f32>() * std::f32::consts::PI / 2.0;
                
                let x = r * phi.sin() * theta.cos();
                let y = r * phi.sin() * theta.sin();
                let z = r * phi.cos();
                
                transform.translation + Vec3::new(x, y, z)
            }
            EmitterShape::Cone { angle, radius } => {
                let r = rand::random::<f32>() * radius;
                let height = r * angle.to_radians().tan();
                
                transform.translation + Vec3::new(
                    (rand::random::<f32>() - 0.5) * 2.0 * r,
                    height,
                    (rand::random::<f32>() - 0.5) * 2.0 * r,
                )
            }
            EmitterShape::Box { size } => {
                transform.translation + Vec3::new(
                    (rand::random::<f32>() - 0.5) * size.x,
                    (rand::random::<f32>() - 0.5) * size.y,
                    (rand::random::<f32>() - 0.5) * size.z,
                )
            }
            EmitterShape::Circle { radius } => {
                let r = rand::random::<f32>().sqrt() * radius;
                let theta = rand::random::<f32>() * 2.0 * std::f32::consts::PI;
                
                transform.translation + Vec3::new(
                    r * theta.cos(),
                    0.0,
                    r * theta.sin(),
                )
            }
        };
        
        let mut particle = Particle::new(position);
        particle.color = self.start_color;
        particle.max_lifetime = self.lifetime;
        
        particle
    }
    
    pub fn update(&mut self, transform: &Transform, delta_time: f32) {
        if !self.enabled {
            return;
        }
        
        // Yeni partikülleri oluştur
        match &self.emission_mode {
            EmissionMode::Continuous { rate } => {
                self.accumulated_emit += rate * delta_time;
                let emit_count = self.accumulated_emit as u32;
                if emit_count > 0 {
                    self.emit_particles(transform, emit_count);
                    self.accumulated_emit -= emit_count as f32;
                }
            }
            EmissionMode::Burst { count } => {
                if self.last_emit_time == 0.0 {
                    self.emit_particles(transform, *count);
                    self.last_emit_time = 1.0; // Tek seferlik patlama
                }
            }
        }
        
        // Mevcut partikülleri güncelle
        self.particles.retain_mut(|particle| {
            // Yaşam süresini güncelle
            particle.lifetime += delta_time;
            
            // Fiziksel etkileri uygula
            particle.velocity += self.gravity * delta_time;
            particle.velocity *= (1.0 - self.drag).powf(delta_time);
            
            // Konumu güncelle
            particle.position += particle.velocity * delta_time;
            
            // Rotasyonu güncelle
            particle.rotation += particle.rotation_speed * delta_time;
            
            // Henüz yaşıyorsa true döndür (silinmesin diye)
            particle.is_alive()
        });
    }
    
    pub fn burst(&mut self) {
        if let EmissionMode::Burst { count } = self.emission_mode {
            self.emit_particles(&Transform::default(), count);
        }
    }
    
    pub fn reset(&mut self) {
        self.particles.clear();
        self.last_emit_time = 0.0;
        self.accumulated_emit = 0.0;
    }
}

// Partikül efekt türleri
#[derive(Debug, Clone)]
pub enum ParticleEffectType {
    Fire,
    Smoke,
    Explosion,
    Rain,
    Snow,
    Magic,
    Energy,
    Custom { name: String },
}

// Hazır partikül efekt fabrikası
pub struct ParticleEffectFactory;

impl ParticleEffectFactory {
    pub fn create_fire_effect() -> ParticleSystem {
        let mut ps = ParticleSystem::new(100);
        
        ps.emission_rate = 20.0;
        ps.lifetime = 3.0;
        ps.lifetime_randomness = 0.3;
        ps.start_speed = Vec3::new(0.0, 2.0, 0.0);
        ps.start_speed_randomness = Vec3::new(0.5, 1.0, 0.5);
        ps.start_size = 0.5;
        ps.start_size_randomness = 0.2;
        ps.gravity = Vec3::new(0.0, 0.0, 0.0);
        ps.drag = 0.05;
        
        // Ateş rengi gradient'ı: altta kırmızı/orange, üstte sarı
        ps.color_gradient = ColorGradient {
            keys: vec![
                (0.0, [1.0, 0.3, 0.1, 1.0]),  // Kırmızımsı alt
                (0.5, [1.0, 0.6, 0.1, 0.8]),  // Orange orta
                (0.8, [1.0, 1.0, 0.2, 0.5]),  // Sarı üst
                (1.0, [1.0, 1.0, 0.2, 0.0]),  // Saydam sarı
            ],
        };
        
        ps.size_curve = SizeCurve {
            keys: vec![
                (0.0, 0.5),  // Küçük başlangıç
                (0.5, 1.0),  // Orta boy
                (1.0, 0.8),  // Az küçülme
            ],
        };
        
        ps.emitter_shape = EmitterShape::Circle { radius: 0.3 };
        
        ps
    }
    
    pub fn create_smoke_effect() -> ParticleSystem {
        let mut ps = ParticleSystem::new(150);
        
        ps.emission_rate = 15.0;
        ps.lifetime = 8.0;
        ps.lifetime_randomness = 0.4;
        ps.start_speed = Vec3::new(0.0, 1.0, 0.0);
        ps.start_speed_randomness = Vec3::new(0.3, 0.5, 0.3);
        ps.start_size = 1.0;
        ps.start_size_randomness = 0.5;
        ps.gravity = Vec3::new(0.0, 0.1, 0.0);  // Hafif yukarı yönlü
        ps.drag = 0.02;
        
        // Duman rengi gradient'ı: gri tonları
        ps.color_gradient = ColorGradient {
            keys: vec![
                (0.0, [0.8, 0.8, 0.8, 0.7]),  // Gri opak
                (0.3, [0.7, 0.7, 0.7, 0.5]),  // Daha şeffaf
                (0.7, [0.6, 0.6, 0.6, 0.3]),  // Daha da şeffaf
                (1.0, [0.5, 0.5, 0.5, 0.0]),  // Tamamen şeffaf
            ],
        };
        
        ps.size_curve = SizeCurve {
            keys: vec![
                (0.0, 0.3),  // Küçük başlangıç
                (0.3, 1.0),  // Genişleme
                (1.0, 2.0),  // Büyüme
            ],
        };
        
        ps.emitter_shape = EmitterShape::Sphere { radius: 0.5 };
        
        ps
    }
    
    pub fn create_explosion_effect() -> ParticleSystem {
        let mut ps = ParticleSystem::new(200);
        
        ps.emission_mode = EmissionMode::Burst { count: 100 };
        ps.lifetime = 2.0;
        ps.lifetime_randomness = 0.2;
        ps.start_speed = Vec3::new(0.0, 0.0, 0.0);
        ps.start_speed_randomness = Vec3::new(8.0, 8.0, 8.0);
        ps.start_size = 0.3;
        ps.start_size_randomness = 0.2;
        ps.gravity = Vec3::new(0.0, -9.81, 0.0);
        ps.drag = 0.1;
        
        // Patlama rengi gradient'ı: parlak renkler
        ps.color_gradient = ColorGradient {
            keys: vec![
                (0.0, [1.0, 0.7, 0.2, 1.0]),  // Parlak turuncu
                (0.3, [1.0, 0.4, 0.1, 0.8]),  // Daha koyu turuncu
                (0.7, [0.8, 0.2, 0.1, 0.5]),  // Kırmızımsı
                (1.0, [0.5, 0.1, 0.1, 0.0]),  // Karanlık kırmızı
            ],
        };
        
        ps.size_curve = SizeCurve {
            keys: vec![
                (0.0, 1.0),   // Başlangıç boyutu
                (0.2, 1.2),   // Hafif büyüme
                (1.0, 0.1),   // Hızlı küçülme
            ],
        };
        
        ps.emitter_shape = EmitterShape::Sphere { radius: 0.1 };
        
        ps
    }
    
    pub fn create_rain_effect() -> ParticleSystem {
        let mut ps = ParticleSystem::new(500);
        
        ps.emission_rate = 200.0;
        ps.lifetime = 4.0;
        ps.lifetime_randomness = 0.1;
        ps.start_speed = Vec3::new(0.0, -15.0, 0.0);
        ps.start_speed_randomness = Vec3::new(0.5, 2.0, 0.5);
        ps.start_size = 0.1;
        ps.start_size_randomness = 0.05;
        ps.gravity = Vec3::new(0.0, 0.0, 0.0);  // Hava direnci etkisi
        ps.drag = 0.0;
        
        // Yağmur rengi: saydam mavi
        ps.color_gradient = ColorGradient {
            keys: vec![
                (0.0, [0.5, 0.5, 1.0, 0.7]),  // Mavi ton
                (1.0, [0.5, 0.5, 1.0, 0.0]),  // Şeffaf mavi
            ],
        };
        
        ps.size_curve = SizeCurve::new();  // Sabit boyut
        
        ps.emitter_shape = EmitterShape::Box { 
            size: Vec3::new(20.0, 1.0, 20.0)  // Geniş alan
        };
        
        ps
    }
    
    pub fn create_custom_effect(effect_type: ParticleEffectType) -> ParticleSystem {
        match effect_type {
            ParticleEffectType::Fire => Self::create_fire_effect(),
            ParticleEffectType::Smoke => Self::create_smoke_effect(),
            ParticleEffectType::Explosion => Self::create_explosion_effect(),
            ParticleEffectType::Rain => Self::create_rain_effect(),
            ParticleEffectType::Snow => {
                let mut ps = ParticleSystem::new(300);
                
                ps.emission_rate = 50.0;
                ps.lifetime = 10.0;
                ps.lifetime_randomness = 0.3;
                ps.start_speed = Vec3::new(0.0, -2.0, 0.0);
                ps.start_speed_randomness = Vec3::new(0.3, 0.5, 0.3);
                ps.start_size = 0.2;
                ps.start_size_randomness = 0.1;
                ps.gravity = Vec3::new(0.0, -1.0, 0.0);
                ps.drag = 0.05;
                
                // Kar rengi: beyaz
                ps.color_gradient = ColorGradient {
                    keys: vec![
                        (0.0, [1.0, 1.0, 1.0, 0.9]),  // Beyaz opak
                        (1.0, [1.0, 1.0, 1.0, 0.0]),  // Şeffaf
                    ],
                };
                
                ps.size_curve = SizeCurve::new();
                
                ps.emitter_shape = EmitterShape::Box { 
                    size: Vec3::new(30.0, 1.0, 30.0) 
                };
                
                ps
            },
            ParticleEffectType::Magic => {
                let mut ps = ParticleSystem::new(100);
                
                ps.emission_rate = 10.0;
                ps.lifetime = 4.0;
                ps.lifetime_randomness = 0.2;
                ps.start_speed = Vec3::new(0.0, 1.0, 0.0);
                ps.start_speed_randomness = Vec3::new(1.0, 1.0, 1.0);
                ps.start_size = 0.15;
                ps.start_size_randomness = 0.05;
                ps.gravity = Vec3::new(0.0, 0.0, 0.0);
                ps.drag = 0.01;
                
                // Sihir rengi: mor/efekt
                ps.color_gradient = ColorGradient {
                    keys: vec![
                        (0.0, [0.8, 0.2, 1.0, 0.8]),  // Mor
                        (0.5, [0.2, 0.8, 1.0, 0.6]),  // Mavi
                        (1.0, [0.2, 1.0, 0.8, 0.0]),  // Yeşilimsi
                    ],
                };
                
                ps.size_curve = SizeCurve {
                    keys: vec![
                        (0.0, 0.5),  // Küçük
                        (0.5, 1.0),  // Büyü
                        (1.0, 0.3),  // Küçül
                    ],
                };
                
                ps.emitter_shape = EmitterShape::Sphere { radius: 2.0 };
                
                ps
            },
            ParticleEffectType::Energy => {
                let mut ps = ParticleSystem::new(80);
                
                ps.emission_rate = 20.0;
                ps.lifetime = 2.0;
                ps.lifetime_randomness = 0.1;
                ps.start_speed = Vec3::new(0.0, 0.0, 0.0);
                ps.start_speed_randomness = Vec3::new(3.0, 3.0, 3.0);
                ps.start_size = 0.25;
                ps.start_size_randomness = 0.1;
                ps.gravity = Vec3::new(0.0, 0.0, 0.0);
                ps.drag = 0.02;
                
                // Enerji rengi: elektrik mavisi
                ps.color_gradient = ColorGradient {
                    keys: vec![
                        (0.0, [0.2, 0.8, 1.0, 1.0]),  // Parlayan mavi
                        (0.3, [0.5, 0.9, 1.0, 0.8]),  // Açık mavi
                        (0.7, [0.8, 0.9, 1.0, 0.5]),  // Soluk mavi
                        (1.0, [0.9, 1.0, 1.0, 0.0]),  // Beyaz şeffaf
                    ],
                };
                
                ps.size_curve = SizeCurve {
                    keys: vec![
                        (0.0, 1.0),  // Büyük başlangıç
                        (0.2, 0.7),  // Hızlı küçülme
                        (1.0, 0.1),  // Minimal
                    ],
                };
                
                ps.emitter_shape = EmitterShape::Point;
                
                ps
            },
            ParticleEffectType::Custom { name } => {
                // Özel efekt için varsayılan bir sistem döndür
                println!("Creating custom particle effect: {}", name);
                ParticleSystem::new(50)
            }
        }
    }
}

// Partikül sistem kontrolörü
pub struct ParticleSystemController {
    pub systems: Vec<ParticleSystem>,
    pub active_systems: Vec<bool>,
    pub auto_update: bool,
}

impl ParticleSystemController {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            active_systems: Vec::new(),
            auto_update: true,
        }
    }
    
    pub fn add_system(&mut self, system: ParticleSystem) {
        self.systems.push(system);
        self.active_systems.push(true);
    }
    
    pub fn remove_system(&mut self, index: usize) {
        if index < self.systems.len() {
            self.systems.remove(index);
            self.active_systems.remove(index);
        }
    }
    
    pub fn enable_system(&mut self, index: usize, enabled: bool) {
        if index < self.active_systems.len() {
            self.active_systems[index] = enabled;
        }
    }
    
    pub fn trigger_burst(&mut self, index: usize) {
        if index < self.systems.len() {
            self.systems[index].burst();
        }
    }
    
    pub fn update_all(&mut self, transform: &Transform, delta_time: f32) {
        if !self.auto_update {
            return;
        }
        
        for (i, system) in self.systems.iter_mut().enumerate() {
            if i < self.active_systems.len() && self.active_systems[i] {
                system.update(transform, delta_time);
            }
        }
    }
    
    pub fn reset_all(&mut self) {
        for system in self.systems.iter_mut() {
            system.reset();
        }
    }
}