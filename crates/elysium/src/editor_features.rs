//! editor_features.rs — Editör için 6 yeni özellik
//!
//! 1. Parçacık Sistemi Editörü
//! 2. Terrain Editörü (fırça tabanlı)
//! 3. Material Editörü (canlı önizleme)
//! 4. Sahne Arama/Filtreleme
//! 5. Keyframe Animation Timeline
//! 6. Sahne Notları/Annotation

use glam::Vec3;
use crate::renderer::*;

// ═══════════════════════════════════════════════════════════ 1. PARÇACIK SİSTEMİ EDİTÖRÜ

/// Parçacık emitter türü
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EmitterType {
    Point,
    Sphere { radius: f32 },
    Cone { angle: f32, radius: f32 },
    Box { size: [f32; 3] },
    Ring { inner_radius: f32, outer_radius: f32 },
}

/// Parçacık Life Over Time eğrisi
#[derive(Clone, Debug)]
pub struct ParticleCurve {
    pub keys: Vec<(f32, f32)>, // (zaman [0-1], değer)
}

impl ParticleCurve {
    pub fn linear() -> Self {
        Self { keys: vec![(0.0, 1.0), (1.0, 0.0)] }
    }
    pub fn ease_out() -> Self {
        Self { keys: vec![(0.0, 1.0), (0.3, 0.8), (1.0, 0.0)] }
    }
    pub fn pulse() -> Self {
        Self { keys: vec![(0.0, 0.0), (0.1, 1.0), (0.3, 0.5), (0.5, 1.0), (1.0, 0.0)] }
    }
    pub fn evaluate(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        if self.keys.len() < 2 { return t; }
        for i in 0..self.keys.len() - 1 {
            let (t0, v0) = self.keys[i];
            let (t1, v1) = self.keys[i + 1];
            if t >= t0 && t <= t1 {
                let f = if (t1 - t0).abs() < 0.0001 { 0.0 } else { (t - t0) / (t1 - t0) };
                return v0 + (v1 - v0) * f;
            }
        }
        self.keys.last().unwrap().1
    }
}

/// Parçacık Sistemi
#[derive(Clone, Debug)]
pub struct ParticleSystem {
    pub name: String,
    pub emitter: EmitterType,
    pub max_particles: u32,
    pub emission_rate: f32,
    pub particle_lifetime: (f32, f32), // (min, max)
    pub start_speed: (f32, f32),
    pub start_size: (f32, f32),
    pub start_color: ([f32; 4], [f32; 4]), // (başlangıç min/max RGBA)
    pub gravity: Vec3,
    pub size_over_lifetime: ParticleCurve,
    pub alpha_over_lifetime: ParticleCurve,
    pub color_over_lifetime: ParticleCurve,
    pub rotation_speed: (f32, f32),
    pub active: bool,
    pub world_space: bool,
    pub loop_emission: bool,
    pub duration: f32,
    pub time: f32,
    pub sort_order: i32,
    pub blend_mode: BlendMode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlendMode {
    Alpha,
    Additive,
    Multiply,
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self {
            name: "ParticleSystem".into(),
            emitter: EmitterType::Sphere { radius: 0.5 },
            max_particles: 500,
            emission_rate: 50.0,
            particle_lifetime: (0.5, 2.0),
            start_speed: (2.0, 5.0),
            start_size: (0.1, 0.3),
            start_color: ([1.0, 0.8, 0.2, 1.0], [1.0, 0.4, 0.1, 0.8]),
            gravity: Vec3::new(0.0, -3.0, 0.0),
            size_over_lifetime: ParticleCurve::ease_out(),
            alpha_over_lifetime: ParticleCurve::linear(),
            color_over_lifetime: ParticleCurve::linear(),
            rotation_speed: (-180.0, 180.0),
            active: true,
            world_space: false,
            loop_emission: true,
            duration: 5.0,
            time: 0.0,
            sort_order: 0,
            blend_mode: BlendMode::Alpha,
        }
    }
}

impl ParticleSystem {
    /// Fire efekti
    pub fn fire() -> Self {
        Self {
            name: "Fire".into(),
            emitter: EmitterType::Cone { angle: 25.0, radius: 0.2 },
            max_particles: 800,
            emission_rate: 100.0,
            particle_lifetime: (0.3, 1.2),
            start_speed: (3.0, 7.0),
            start_size: (0.15, 0.4),
            start_color: ([1.0, 0.9, 0.1, 1.0], [1.0, 0.3, 0.0, 0.9]),
            gravity: Vec3::new(0.0, 1.0, 0.0),
            size_over_lifetime: ParticleCurve { keys: vec![(0.0, 0.5), (0.2, 1.0), (1.0, 0.0)] },
            alpha_over_lifetime: ParticleCurve::ease_out(),
            color_over_lifetime: ParticleCurve { keys: vec![(0.0, 1.0), (0.4, 0.8), (1.0, 0.2)] },
            blend_mode: BlendMode::Additive,
            ..Default::default()
        }
    }

    /// Su efekti
    pub fn water_splash() -> Self {
        Self {
            name: "WaterSplash".into(),
            emitter: EmitterType::Ring { inner_radius: 0.3, outer_radius: 0.8 },
            max_particles: 200,
            emission_rate: 30.0,
            particle_lifetime: (0.5, 1.5),
            start_speed: (4.0, 8.0),
            start_size: (0.05, 0.15),
            start_color: ([0.4, 0.7, 1.0, 0.8], [0.6, 0.9, 1.0, 0.6]),
            gravity: Vec3::new(0.0, -12.0, 0.0),
            blend_mode: BlendMode::Alpha,
            ..Default::default()
        }
    }

    /// Yaprak efekti
    pub fn leaves() -> Self {
        Self {
            name: "Leaves".into(),
            emitter: EmitterType::Box { size: [10.0, 0.0, 10.0] },
            max_particles: 100,
            emission_rate: 8.0,
            particle_lifetime: (3.0, 6.0),
            start_speed: (0.5, 1.5),
            start_size: (0.1, 0.25),
            start_color: ([0.2, 0.7, 0.1, 0.9], [0.8, 0.6, 0.0, 0.8]),
            gravity: Vec3::new(0.0, -0.5, 0.0),
            rotation_speed: (-90.0, 90.0),
            ..Default::default()
        }
    }

    /// Parçacık istatistikleri
    pub fn stats(&self) -> ParticleStats {
        ParticleStats {
            max: self.max_particles,
            emission_rate: self.emission_rate,
            lifetime_avg: (self.particle_lifetime.0 + self.particle_lifetime.1) / 2.0,
            active: self.active,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParticleStats {
    pub max: u32,
    pub emission_rate: f32,
    pub lifetime_avg: f32,
    pub active: bool,
}

// ═══════════════════════════════════════════════════════════ 2. TERRAIN EDİTÖRÜ

/// Terrain fırça türü
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TerrainBrush {
    Raise,
    Lower,
    Flatten,
    Smooth,
    Paint { material_id: u32 },
}

/// Terrain editörü
#[derive(Clone, Debug)]
pub struct TerrainEditor {
    pub enabled: bool,
    pub brush: TerrainBrush,
    pub brush_radius: f32,
    pub brush_strength: f32,
    pub height_data: Vec<Vec<f32>>,
    pub resolution: usize,
    pub terrain_size: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub layers: Vec<TerrainLayer>,
    pub active_layer: usize,
}

#[derive(Clone, Debug)]
pub struct TerrainLayer {
    pub name: String,
    pub color: [u8; 3],
    pub texture_id: Option<u32>,
    pub opacity: f32,
}

impl Default for TerrainEditor {
    fn default() -> Self {
        let res = 64;
        Self {
            enabled: false,
            brush: TerrainBrush::Raise,
            brush_radius: 3.0,
            brush_strength: 0.5,
            height_data: vec![vec![0.0; res]; res],
            resolution: res,
            terrain_size: 20.0,
            min_height: -5.0,
            max_height: 10.0,
            layers: vec![
                TerrainLayer { name: "Grass".into(), color: [60, 140, 40], texture_id: None, opacity: 1.0 },
                TerrainLayer { name: "Dirt".into(), color: [140, 100, 60], texture_id: None, opacity: 1.0 },
                TerrainLayer { name: "Rock".into(), color: [120, 120, 120], texture_id: None, opacity: 1.0 },
                TerrainLayer { name: "Sand".into(), color: [210, 190, 140], texture_id: None, opacity: 1.0 },
                TerrainLayer { name: "Snow".into(), color: [240, 240, 250], texture_id: None, opacity: 1.0 },
            ],
            active_layer: 0,
        }
    }
}

impl TerrainEditor {
    /// Fırça ile yüksekliği değiştir
    pub fn apply_brush(&mut self, world_x: f32, world_z: f32, dt: f32) {
        let cx = ((world_x / self.terrain_size + 0.5) * self.resolution as f32) as i32;
        let cz = ((world_z / self.terrain_size + 0.5) * self.resolution as f32) as i32;
        let r = (self.brush_radius / self.terrain_size * self.resolution as f32) as i32;

        for dz in -r..=r {
            for dx in -r..=r {
                let x = cx + dx;
                let z = cz + dz;
                if x < 0 || z < 0 || x >= self.resolution as i32 || z >= self.resolution as i32 { continue; }

                let dist = ((dx * dx + dz * dz) as f32).sqrt();
                let falloff = (1.0 - dist / (r as f32 + 0.001)).max(0.0);
                let strength = self.brush_strength * falloff * dt * 10.0;

                let xi = x as usize;
                let zi = z as usize;

                match self.brush {
                    TerrainBrush::Raise => {
                        self.height_data[zi][xi] += strength;
                        self.height_data[zi][xi] = self.height_data[zi][xi].min(self.max_height);
                    }
                    TerrainBrush::Lower => {
                        self.height_data[zi][xi] -= strength;
                        self.height_data[zi][xi] = self.height_data[zi][xi].max(self.min_height);
                    }
                    TerrainBrush::Flatten => {
                        let clamp_x = (cx.max(0) as usize).min(self.resolution-1);
                        let clamp_z = (cz.max(0) as usize).min(self.resolution-1);
                        let target = self.height_data[clamp_z][clamp_x];
                        self.height_data[zi][xi] += (target - self.height_data[zi][xi]) * strength;
                    }
                    TerrainBrush::Smooth => {
                        let mut avg = 0.0;
                        let mut count = 0;
                        for sz in -1..=1 {
                            for sx in -1..=1 {
                                let nx = x + sx;
                                let nz = z + sz;
                                if nx >= 0 && nz >= 0 && nx < self.resolution as i32 && nz < self.resolution as i32 {
                                    avg += self.height_data[nz as usize][nx as usize];
                                    count += 1;
                                }
                            }
                        }
                        if count > 0 {
                            avg /= count as f32;
                            self.height_data[zi][xi] += (avg - self.height_data[zi][xi]) * strength;
                        }
                    }
                    TerrainBrush::Paint { .. } => {
                        // Texture painting — şimdilik noop
                    }
                }
            }
        }
    }

    /// Yüksekliği world pozisyonundan oku
    pub fn get_height(&self, world_x: f32, world_z: f32) -> f32 {
        let fx = (world_x / self.terrain_size + 0.5) * self.resolution as f32;
        let fz = (world_z / self.terrain_size + 0.5) * self.resolution as f32;
        let x0 = fx.floor() as i32;
        let z0 = fz.floor() as i32;
        if x0 < 0 || z0 < 0 || x0 + 1 >= self.resolution as i32 || z0 + 1 >= self.resolution as i32 {
            return 0.0;
        }
        let tx = fx - fx.floor();
        let tz = fz - fz.floor();
        let h00 = self.height_data[z0 as usize][x0 as usize];
        let h10 = self.height_data[z0 as usize][(x0 + 1) as usize];
        let h01 = self.height_data[(z0 + 1) as usize][x0 as usize];
        let h11 = self.height_data[(z0 + 1) as usize][(x0 + 1) as usize];
        let h0 = h00 + (h10 - h00) * tx;
        let h1 = h01 + (h11 - h01) * tx;
        h0 + (h1 - h0) * tz
    }

    /// Yüzey normalini hesapla
    pub fn get_normal(&self, world_x: f32, world_z: f32) -> Vec3 {
        let delta = 0.1;
        let h = self.get_height(world_x, world_z);
        let hx = self.get_height(world_x + delta, world_z);
        let hz = self.get_height(world_x, world_z + delta);
        Vec3::new(h - hx, delta, h - hz).normalize()
    }

    /// Perlin benzeri prosedürel terrain oluştur
    pub fn generate_procedural(&mut self, seed: u32, octaves: u32, persistence: f32) {
        for z in 0..self.resolution {
            for x in 0..self.resolution {
                let nx = x as f32 / self.resolution as f32;
                let nz = z as f32 / self.resolution as f32;
                let mut height = 0.0;
                let mut amplitude = 1.0;
                let mut frequency = 1.0;
                for _ in 0..octaves {
                    let val = self.noise2d(nx * frequency + seed as f32, nz * frequency + seed as f32 * 1.3);
                    height += val * amplitude;
                    amplitude *= persistence;
                    frequency *= 2.0;
                }
                self.height_data[z][x] = height * 3.0;
            }
        }
    }

    fn noise2d(&self, x: f32, y: f32) -> f32 {
        let ix = x.floor() as i32;
        let iy = y.floor() as i32;
        let fx = x - x.floor();
        let fy = y - y.floor();
        let fx = fx * fx * (3.0 - 2.0 * fx);
        let fy = fy * fy * (3.0 - 2.0 * fy);

        let hash = |x: i32, y: i32| -> f32 {
            let h = (x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263))) as u32;
            ((h >> 13) ^ h) as f32 / 2147483648.0
        };

        let v00 = hash(ix, iy);
        let v10 = hash(ix + 1, iy);
        let v01 = hash(ix, iy + 1);
        let v11 = hash(ix + 1, iy + 1);
        let v0 = v00 + (v10 - v00) * fx;
        let v1 = v01 + (v11 - v01) * fx;
        (v0 + (v1 - v0) * fy) * 2.0 - 1.0
    }
}

// ═══════════════════════════════════════════════════════════ 3. MATERIAL EDİTÖRÜ

/// Material slot tanımı
#[derive(Clone, Debug)]
pub struct MaterialSlot {
    pub name: String,
    pub value_type: MaterialValueType,
    pub min: f32,
    pub max: f32,
    pub step: f32,
}

#[derive(Clone, Debug)]
pub enum MaterialValueType {
    Float(f32),
    Color([f32; 3]),
    Texture(Option<u32>),
}

/// Material editörü
#[derive(Clone, Debug)]
pub struct MaterialEditor {
    pub enabled: bool,
    pub selected_object: Option<usize>,
    pub preview_size: u32,
    pub slots: Vec<MaterialSlot>,
    pub needs_update: bool,
}

impl Default for MaterialEditor {
    fn default() -> Self {
        Self {
            enabled: false,
            selected_object: None,
            preview_size: 128,
            slots: vec![
                MaterialSlot { name: "Albedo".into(), value_type: MaterialValueType::Color([0.8, 0.8, 0.8]), min: 0.0, max: 1.0, step: 0.01 },
                MaterialSlot { name: "Metallic".into(), value_type: MaterialValueType::Float(0.0), min: 0.0, max: 1.0, step: 0.01 },
                MaterialSlot { name: "Roughness".into(), value_type: MaterialValueType::Float(0.5), min: 0.0, max: 1.0, step: 0.01 },
                MaterialSlot { name: "AO".into(), value_type: MaterialValueType::Float(1.0), min: 0.0, max: 1.0, step: 0.01 },
                MaterialSlot { name: "Emissive".into(), value_type: MaterialValueType::Color([0.0, 0.0, 0.0]), min: 0.0, max: 5.0, step: 0.1 },
                MaterialSlot { name: "Emissive Strength".into(), value_type: MaterialValueType::Float(0.0), min: 0.0, max: 10.0, step: 0.1 },
                MaterialSlot { name: "Albedo Texture".into(), value_type: MaterialValueType::Texture(None), min: 0.0, max: 0.0, step: 0.0 },
                MaterialSlot { name: "Normal Map".into(), value_type: MaterialValueType::Texture(None), min: 0.0, max: 0.0, step: 0.0 },
            ],
            needs_update: false,
        }
    }
}

impl MaterialEditor {
    /// Seçili nesnenin materyalini slot'lardan oku
    pub fn sync_from_object(&mut self, obj: &SceneObject) {
        self.slots[0].value_type = MaterialValueType::Color(obj.material.albedo.to_array());
        self.slots[1].value_type = MaterialValueType::Float(obj.material.metallic);
        self.slots[2].value_type = MaterialValueType::Float(obj.material.roughness);
        self.slots[3].value_type = MaterialValueType::Float(obj.material.ao);
        self.slots[4].value_type = MaterialValueType::Color(obj.material.emissive.to_array());
        self.slots[5].value_type = MaterialValueType::Float(obj.material.emissive_strength);
        self.slots[6].value_type = MaterialValueType::Texture(obj.material.albedo_texture);
        self.slots[7].value_type = MaterialValueType::Texture(obj.material.normal_texture);
    }

    /// Slot değerini değiştir
    pub fn set_slot_value(&mut self, index: usize, value: MaterialValueType) {
        if index < self.slots.len() {
            self.slots[index].value_type = value;
            self.needs_update = true;
        }
    }

    /// Slot değerini nesneye uygula
    pub fn apply_to_object(&self, obj: &mut SceneObject) {
        if let MaterialValueType::Color(c) = self.slots[0].value_type {
            obj.material.albedo = Vec3::new(c[0], c[1], c[2]);
        }
        if let MaterialValueType::Float(v) = self.slots[1].value_type {
            obj.material.metallic = v;
        }
        if let MaterialValueType::Float(v) = self.slots[2].value_type {
            obj.material.roughness = v;
        }
        if let MaterialValueType::Float(v) = self.slots[3].value_type {
            obj.material.ao = v;
        }
        if let MaterialValueType::Color(c) = self.slots[4].value_type {
            obj.material.emissive = Vec3::new(c[0], c[1], c[2]);
        }
        if let MaterialValueType::Float(v) = self.slots[5].value_type {
            obj.material.emissive_strength = v;
        }
        if let MaterialValueType::Texture(t) = self.slots[6].value_type {
            obj.material.albedo_texture = t;
        }
        if let MaterialValueType::Texture(t) = self.slots[7].value_type {
            obj.material.normal_texture = t;
        }
    }

    /// Preset materyaller
    pub fn apply_preset(&mut self, preset: &str) {
        match preset {
            "gold" => {
                self.slots[0].value_type = MaterialValueType::Color([1.0, 0.84, 0.0]);
                self.slots[1].value_type = MaterialValueType::Float(1.0);
                self.slots[2].value_type = MaterialValueType::Float(0.2);
            }
            "chrome" => {
                self.slots[0].value_type = MaterialValueType::Color([0.9, 0.9, 0.9]);
                self.slots[1].value_type = MaterialValueType::Float(1.0);
                self.slots[2].value_type = MaterialValueType::Float(0.05);
            }
            "plastic" => {
                self.slots[0].value_type = MaterialValueType::Color([0.2, 0.5, 0.9]);
                self.slots[1].value_type = MaterialValueType::Float(0.0);
                self.slots[2].value_type = MaterialValueType::Float(0.4);
            }
            "glass" => {
                self.slots[0].value_type = MaterialValueType::Color([0.8, 0.9, 1.0]);
                self.slots[1].value_type = MaterialValueType::Float(0.0);
                self.slots[2].value_type = MaterialValueType::Float(0.0);
                self.slots[3].value_type = MaterialValueType::Float(0.0);
            }
            "rubber" => {
                self.slots[0].value_type = MaterialValueType::Color([0.1, 0.1, 0.1]);
                self.slots[1].value_type = MaterialValueType::Float(0.0);
                self.slots[2].value_type = MaterialValueType::Float(0.9);
            }
            "emissive_blue" => {
                self.slots[0].value_type = MaterialValueType::Color([0.0, 0.2, 0.8]);
                self.slots[1].value_type = MaterialValueType::Float(0.0);
                self.slots[2].value_type = MaterialValueType::Float(0.5);
                self.slots[4].value_type = MaterialValueType::Color([0.0, 0.3, 1.0]);
                self.slots[5].value_type = MaterialValueType::Float(3.0);
            }
            _ => {}
        }
        self.needs_update = true;
    }
}

// ═══════════════════════════════════════════════════════════ 4. SAHNE ARAMA/FİLTRELEME

/// Arama/filtre durumu
#[derive(Clone, Debug)]
pub struct SceneSearch {
    pub enabled: bool,
    pub query: String,
    pub filter_type: SearchFilter,
    pub results: Vec<usize>,
    pub result_index: usize,
    pub match_count: usize,
    pub case_sensitive: bool,
    pub search_in_tags: bool,
    pub search_in_children: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SearchFilter {
    All,
    ByType(GeometryType),
    ByTeam(EntityTeam),
    ByName,
    WithPhysics,
    WithSkeleton,
    WithLight,
    Selected,
}

impl Default for SceneSearch {
    fn default() -> Self {
        Self {
            enabled: false,
            query: String::new(),
            filter_type: SearchFilter::All,
            results: Vec::new(),
            result_index: 0,
            match_count: 0,
            case_sensitive: false,
            search_in_tags: true,
            search_in_children: true,
        }
    }
}

impl SceneSearch {
    /// Sahneyi ara
    pub fn search(&mut self, scene: &Scene) {
        self.results.clear();
        let query_lower = if self.case_sensitive { self.query.clone() } else { self.query.to_lowercase() };

        for obj in &scene.objects {
            // İsim araması
            let name_match = if self.query.is_empty() {
                true
            } else if self.case_sensitive {
                obj.name.contains(&self.query)
            } else {
                obj.name.to_lowercase().contains(&query_lower)
            };

            // Tip filtresi
            let type_match = match self.filter_type {
                SearchFilter::All => true,
                SearchFilter::ByType(ref t) => std::mem::discriminant(&obj.geometry) == std::mem::discriminant(t),
                SearchFilter::ByTeam(team) => obj.team == team,
                SearchFilter::ByName => name_match,
                SearchFilter::WithPhysics => obj.rigid_body.is_some(),
                SearchFilter::WithSkeleton => obj.skeleton_id.is_some(),
                SearchFilter::WithLight => obj.is_light,
                SearchFilter::Selected => false, // handled separately
            };

            // Tag araması
            let tag_match = if self.search_in_tags && !self.query.is_empty() {
                obj.tags.iter().any(|t| {
                    if self.case_sensitive { t.contains(&self.query) } else { t.to_lowercase().contains(&query_lower) }
                })
            } else {
                false
            };

            if (name_match || tag_match) && type_match {
                self.results.push(obj.id);
            }
        }

        self.match_count = self.results.len();
        self.result_index = 0;
    }

    /// Sonraki sonuca geç
    pub fn next_result(&mut self) -> Option<usize> {
        if self.results.is_empty() { return None; }
        self.result_index = (self.result_index + 1) % self.results.len();
        Some(self.results[self.result_index])
    }

    /// Önceki sonuca geç
    pub fn prev_result(&mut self) -> Option<usize> {
        if self.results.is_empty() { return None; }
        if self.result_index == 0 { self.result_index = self.results.len() - 1; }
        else { self.result_index -= 1; }
        Some(self.results[self.result_index])
    }

    /// Tüm sonuçların ID'lerini döndür
    pub fn all_results(&self) -> &[usize] {
        &self.results
    }
}

// ═══════════════════════════════════════════════════════════ 5. KEYFRAME ANİMASYON ZAMAN ÇİZELGESİ

/// Keyframe
#[derive(Clone, Debug)]
pub struct Keyframe {
    pub time: f32,
    pub property: String,
    pub value: KeyframeValue,
    pub easing: EasingType,
    pub selected: bool,
}

#[derive(Clone, Debug)]
pub enum KeyframeValue {
    Float(f32),
    Vec3(Vec3),
    Color([f32; 3]),
    Bool(bool),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EasingType {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Step,
}

/// Animation timeline
#[derive(Clone, Debug)]
pub struct AnimationTimeline {
    pub enabled: bool,
    pub duration: f32,
    pub current_time: f32,
    pub playing: bool,
    pub loop_playback: bool,
    pub fps: f32,
    pub keyframes: Vec<Keyframe>,
    pub selected_keyframe: Option<usize>,
    pub zoom: f32,
    pub scroll_offset: f32,
    pub visible_start: f32,
    pub visible_end: f32,
    pub snap_to_grid: bool,
    pub grid_size: f32,
    pub records: bool,
}

impl Default for AnimationTimeline {
    fn default() -> Self {
        Self {
            enabled: false,
            duration: 5.0,
            current_time: 0.0,
            playing: false,
            loop_playback: true,
            fps: 30.0,
            keyframes: Vec::new(),
            selected_keyframe: None,
            zoom: 1.0,
            scroll_offset: 0.0,
            visible_start: 0.0,
            visible_end: 5.0,
            snap_to_grid: true,
            grid_size: 1.0 / 30.0,
            records: false,
        }
    }
}

impl AnimationTimeline {
    /// Keyframe ekle
    pub fn add_keyframe(&mut self, time: f32, property: &str, value: KeyframeValue) {
        if self.snap_to_grid {
            let snapped = (time / self.grid_size).round() * self.grid_size;
            self.keyframes.push(Keyframe {
                time: snapped, property: property.to_string(), value,
                easing: EasingType::Linear, selected: false,
            });
        } else {
            self.keyframes.push(Keyframe {
                time, property: property.to_string(), value,
                easing: EasingType::Linear, selected: false,
            });
        }
        self.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Keyframe sil
    pub fn remove_selected(&mut self) {
        if let Some(idx) = self.selected_keyframe {
            self.keyframes.remove(idx);
            self.selected_keyframe = None;
        }
    }

    /// Zaman çizelgesinde zamanı ilerlet
    pub fn update(&mut self, dt: f32) {
        if self.playing {
            self.current_time += dt;
            if self.current_time >= self.duration {
                if self.loop_playback {
                    self.current_time = 0.0;
                } else {
                    self.current_time = self.duration;
                    self.playing = false;
                }
            }
        }
    }

    /// Belirli bir zamandaki değeri hesapla (interpolasyon)
    pub fn sample(&self, time: f32, property: &str) -> Option<KeyframeValue> {
        let relevant: Vec<&Keyframe> = self.keyframes.iter()
            .filter(|k| k.property == property)
            .collect();

        if relevant.is_empty() { return None; }
        if relevant.len() == 1 { return Some(relevant[0].value.clone()); }

        // Önceki ve sonraki keyframe'leri bul
        let mut prev = relevant[0];
        let mut next = relevant[relevant.len() - 1];

        for i in 0..relevant.len() - 1 {
            if time >= relevant[i].time && time <= relevant[i + 1].time {
                prev = relevant[i];
                next = relevant[i + 1];
                break;
            }
        }

        if prev.time == next.time { return Some(prev.value.clone()); }

        let t = ((time - prev.time) / (next.time - prev.time)).clamp(0.0, 1.0);

        match (&prev.value, &next.value) {
            (KeyframeValue::Float(a), KeyframeValue::Float(b)) => {
                Some(KeyframeValue::Float(a + (b - a) * t))
            }
            (KeyframeValue::Vec3(a), KeyframeValue::Vec3(b)) => {
                Some(KeyframeValue::Vec3(a.lerp(*b, t)))
            }
            _ => Some(prev.value.clone()),
        }
    }

    /// Belirli bir property için tüm keyframe zamanlarını döndür
    pub fn get_keyframe_times(&self, property: &str) -> Vec<f32> {
        self.keyframes.iter()
            .filter(|k| k.property == property)
            .map(|k| k.time)
            .collect()
    }

    /// Tüm property isimlerini döndür (benzersiz)
    pub fn properties(&self) -> Vec<String> {
        let mut props: Vec<String> = self.keyframes.iter().map(|k| k.property.clone()).collect();
        props.sort();
        props.dedup();
        props
    }
}

// ═══════════════════════════════════════════════════════════ 6. SAHNE NOTLARI/ANNOTATION

/// Not rengi
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoteColor {
    Yellow,
    Red,
    Green,
    Blue,
    Purple,
    Orange,
}

impl NoteColor {
    pub fn rgba(&self) -> [u8; 4] {
        match self {
            Self::Yellow => [255, 230, 80, 220],
            Self::Red => [240, 80, 80, 220],
            Self::Green => [80, 200, 120, 220],
            Self::Blue => [80, 150, 240, 220],
            Self::Purple => [180, 100, 240, 220],
            Self::Orange => [240, 160, 60, 220],
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Yellow => "Sarı",
            Self::Red => "Kırmızı",
            Self::Green => "Yeşil",
            Self::Blue => "Mavi",
            Self::Purple => "Mor",
            Self::Orange => "Turuncu",
        }
    }
}

/// Sahne notu
#[derive(Clone, Debug)]
pub struct SceneNote {
    pub id: usize,
    pub text: String,
    pub position: Vec3,
    pub color: NoteColor,
    pub attached_object: Option<usize>,
    pub created_at: String,
    pub priority: NotePriority,
    pub visible: bool,
    pub icon: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NotePriority {
    Low,
    Medium,
    High,
    Critical,
}

impl NotePriority {
    pub fn icon(&self) -> &str {
        match self {
            Self::Low => "📝",
            Self::Medium => "📌",
            Self::High => "⚠️",
            Self::Critical => "🔴",
        }
    }
}

/// Annotation sistemi
#[derive(Clone, Debug)]
pub struct AnnotationSystem {
    pub enabled: bool,
    pub notes: Vec<SceneNote>,
    pub next_id: usize,
    pub selected_note: Option<usize>,
    pub show_in_viewport: bool,
    pub show_in_hierarchy: bool,
    pub filter_color: Option<NoteColor>,
    pub filter_priority: Option<NotePriority>,
    pub total_notes: usize,
}

impl Default for AnnotationSystem {
    fn default() -> Self {
        Self {
            enabled: false,
            notes: Vec::new(),
            next_id: 1,
            selected_note: None,
            show_in_viewport: true,
            show_in_hierarchy: true,
            filter_color: None,
            filter_priority: None,
            total_notes: 0,
        }
    }
}

impl AnnotationSystem {
    /// Yeni not ekle
    pub fn add_note(&mut self, text: &str, position: Vec3, color: NoteColor) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.notes.push(SceneNote {
            id,
            text: text.to_string(),
            position,
            color,
            attached_object: None,
            created_at: format!("note_{}", id),
            priority: NotePriority::Medium,
            visible: true,
            icon: "📝".into(),
        });
        self.total_notes = self.notes.len();
        id
    }

    /// Nesneye not ekle
    pub fn attach_to_object(&mut self, note_id: usize, object_id: usize) {
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == note_id) {
            note.attached_object = Some(object_id);
        }
    }

    /// Not sil
    pub fn remove_note(&mut self, note_id: usize) {
        self.notes.retain(|n| n.id != note_id);
        self.total_notes = self.notes.len();
    }

    /// Filtrelenmiş notları döndür
    pub fn filtered_notes(&self) -> Vec<&SceneNote> {
        self.notes.iter().filter(|n| {
            if let Some(color) = self.filter_color {
                if n.color != color { return false; }
            }
            if let Some(priority) = self.filter_priority {
                if n.priority != priority { return false; }
            }
            true
        }).collect()
    }

    /// Nesneye ait notları bul
    pub fn notes_for_object(&self, object_id: usize) -> Vec<&SceneNote> {
        self.notes.iter().filter(|n| n.attached_object == Some(object_id)).collect()
    }

    /// Öncelik sayımı
    pub fn priority_counts(&self) -> (usize, usize, usize, usize) {
        let mut low = 0; let mut med = 0; let mut high = 0; let mut crit = 0;
        for n in &self.notes {
            match n.priority {
                NotePriority::Low => low += 1,
                NotePriority::Medium => med += 1,
                NotePriority::High => high += 1,
                NotePriority::Critical => crit += 1,
            }
        }
        (low, med, high, crit)
    }
}

// ═══════════════════════════════════════════════════════════ Testler

#[cfg(test)]
mod tests {
    use super::*;

    // Particle tests
    #[test]
    fn test_particle_curve() {
        let curve = ParticleCurve::linear();
        assert!((curve.evaluate(0.0) - 1.0).abs() < 0.01);
        assert!((curve.evaluate(0.5) - 0.5).abs() < 0.01);
        assert!((curve.evaluate(1.0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_particle_curve_pulse() {
        let curve = ParticleCurve::pulse();
        assert!(curve.evaluate(0.1) > 0.9);
        assert!(curve.evaluate(0.5) > 0.9);
    }

    #[test]
    fn test_particle_system_presets() {
        let fire = ParticleSystem::fire();
        assert_eq!(fire.name, "Fire");
        assert_eq!(fire.blend_mode, BlendMode::Additive);

        let water = ParticleSystem::water_splash();
        assert_eq!(water.name, "WaterSplash");
    }

    // Terrain tests
    #[test]
    fn test_terrain_get_height() {
        let mut te = TerrainEditor::default();
        te.height_data[32][32] = 5.0;
        let h = te.get_height(0.0, 0.0);
        assert!((h - 5.0).abs() < 0.5);
    }

    #[test]
    fn test_terrain_brush() {
        let mut te = TerrainEditor::default();
        let before = te.get_height(0.0, 0.0);
        te.brush = TerrainBrush::Raise;
        te.brush_strength = 1.0;
        te.apply_brush(0.0, 0.0, 0.1);
        let after = te.get_height(0.0, 0.0);
        assert!(after > before);
    }

    #[test]
    fn test_terrain_procedural() {
        let mut te = TerrainEditor::default();
        te.generate_procedural(42, 4, 0.5);
        let h = te.get_height(0.0, 0.0);
        let _ = h; // Sadece çökmesin
    }

    // Material tests
    #[test]
    fn test_material_editor_sync() {
        let mut me = MaterialEditor::default();
        let mut obj = SceneObject {
            id: 1, name: "Test".into(), geometry: GeometryType::Cube,
            transform: Transform::default(), color: [0.8, 0.8, 0.8], visible: true,
            team: EntityTeam::Neutral, health: 100.0, max_health: 100.0, damage: 10.0,
            material: PbrMaterial { metallic: 0.7, ..Default::default() },
            is_light: false, light_id: None, rigid_body: None,
            collider_radius: 1.0, tags: Vec::new(), skeleton_id: None,
        };
        me.sync_from_object(&obj);
        if let MaterialValueType::Float(v) = me.slots[1].value_type {
            assert!((v - 0.7).abs() < 0.01);
        } else { panic!("Expected float"); }

        me.apply_preset("gold");
        me.apply_to_object(&mut obj);
        assert!((obj.material.metallic - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_material_presets() {
        let mut me = MaterialEditor::default();
        me.apply_preset("chrome");
        me.apply_preset("glass");
        me.apply_preset("rubber");
        assert!(me.needs_update);
    }

    // Search tests
    #[test]
    fn test_scene_search() {
        let mut scene = Scene::default();
        scene.add_object("Player".into(), GeometryType::Capsule, EntityTeam::Player);
        scene.add_object("Enemy1".into(), GeometryType::Cube, EntityTeam::Enemy);
        scene.add_object("Enemy2".into(), GeometryType::Cube, EntityTeam::Enemy);
        scene.add_object("Ground".into(), GeometryType::Plane, EntityTeam::Neutral);

        let mut search = SceneSearch::default();
        search.query = "Enemy".into();
        search.search(&scene);
        assert_eq!(search.match_count, 2);

        search.query = "Player".into();
        search.search(&scene);
        assert_eq!(search.match_count, 1);
    }

    #[test]
    fn test_search_navigation() {
        let mut scene = Scene::default();
        for i in 0..5 {
            scene.add_object(format!("Item {}", i), GeometryType::Cube, EntityTeam::Neutral);
        }

        let mut search = SceneSearch::default();
        search.search(&scene);
        assert_eq!(search.match_count, 5);

        let first = search.next_result();
        assert!(first.is_some());
        let second = search.next_result();
        assert!(second.is_some());
        assert_ne!(first, second);
    }

    // Timeline tests
    #[test]
    fn test_timeline_keyframe() {
        let mut tl = AnimationTimeline::default();
        tl.add_keyframe(0.0, "position.x", KeyframeValue::Float(0.0));
        tl.add_keyframe(1.0, "position.x", KeyframeValue::Float(5.0));

        let val = tl.sample(0.5, "position.x");
        assert!(val.is_some());
        if let Some(KeyframeValue::Float(v)) = val {
            assert!((v - 2.5).abs() < 0.01);
        }
    }

    #[test]
    fn test_timeline_update() {
        let mut tl = AnimationTimeline::default();
        tl.playing = true;
        tl.duration = 2.0;
        tl.update(0.5);
        assert!((tl.current_time - 0.5).abs() < 0.01);
        tl.update(2.0);
        assert!((tl.current_time - 0.0).abs() < 0.01); // Loop
    }

    #[test]
    fn test_timeline_properties() {
        let mut tl = AnimationTimeline::default();
        tl.add_keyframe(0.0, "position.x", KeyframeValue::Float(0.0));
        tl.add_keyframe(0.0, "rotation.y", KeyframeValue::Float(0.0));
        tl.add_keyframe(1.0, "position.x", KeyframeValue::Float(5.0));

        let props = tl.properties();
        assert_eq!(props.len(), 2, "Should have 2 unique properties, got {:?}", props);
    }

    // Annotation tests
    #[test]
    fn test_annotation_add() {
        let mut ann = AnnotationSystem::default();
        let id = ann.add_note("Test note", Vec3::ZERO, NoteColor::Yellow);
        assert_eq!(ann.total_notes, 1);
        assert_eq!(ann.notes[0].id, id);
    }

    #[test]
    fn test_annotation_filter() {
        let mut ann = AnnotationSystem::default();
        ann.add_note("Note 1", Vec3::ZERO, NoteColor::Red);
        ann.add_note("Note 2", Vec3::ZERO, NoteColor::Blue);
        ann.add_note("Note 3", Vec3::ZERO, NoteColor::Red);

        ann.filter_color = Some(NoteColor::Red);
        let filtered = ann.filtered_notes();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_annotation_object_attachment() {
        let mut ann = AnnotationSystem::default();
        let id = ann.add_note("Attached", Vec3::ZERO, NoteColor::Green);
        ann.attach_to_object(id, 42);
        let notes = ann.notes_for_object(42);
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn test_note_color_rgba() {
        let colors = [NoteColor::Yellow, NoteColor::Red, NoteColor::Green, NoteColor::Blue, NoteColor::Purple, NoteColor::Orange];
        for c in &colors {
            let rgba = c.rgba();
            assert_eq!(rgba[3], 220); // Alpha always 220
        }
    }
}
