use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::{Entity, Transform, Component};

/// Fiziksel nesne türü
#[derive(Debug, Clone, PartialEq)]
pub enum PhysicsBodyType {
    Static,      // Statik nesne (hareket etmez)
    Dynamic,     // Dinamik nesne (fizik etkileriyle hareket eder)
    Kinematic,   // Kinematik nesne (kodla hareket ettirilir, ama çarpışmaları vardır)
}

/// Çarpışma şekli
#[derive(Debug, Clone, PartialEq)]
pub enum CollisionShape {
    Sphere { radius: f32 },
    Box { half_extents: crate::math::Vec3 },
    Capsule { radius: f32, height: f32 },
    Mesh { vertices: Vec<crate::math::Vec3> }, // Sadece statik nesneler için
}

/// Fiziksel nesne bileşeni
#[derive(Debug, Clone)]
pub struct PhysicsBody {
    pub body_type: PhysicsBodyType,
    pub shape: CollisionShape,
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32, // Sekme katsayısı
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub is_sensor: bool, // Sadece tetikleyici olarak çalışan nesne
    pub collision_groups: u32,
    pub collision_masks: u32,
}

impl Default for PhysicsBody {
    fn default() -> Self {
        Self {
            body_type: PhysicsBodyType::Dynamic,
            shape: CollisionShape::Sphere { radius: 1.0 },
            mass: 1.0,
            friction: 0.5,
            restitution: 0.2,
            linear_damping: 0.0,
            angular_damping: 0.0,
            is_sensor: false,
            collision_groups: 1,
            collision_masks: 0xFFFFFFFF, // Tüm gruplarla çarpışır
        }
    }
}

/// Kuvvet türleri
#[derive(Debug, Clone)]
pub enum ForceType {
    Impulse,
    Force,
    Torque,
}

/// Uygulanan kuvvet
#[derive(Debug, Clone)]
pub struct AppliedForce {
    pub force: crate::math::Vec3,
    pub force_type: ForceType,
    pub position: crate::math::Vec3, // Sadece tork için kullanılır
}

/// Fiziksel nesne durumu
#[derive(Debug, Clone)]
pub struct PhysicsState {
    pub position: crate::math::Vec3,
    pub rotation: crate::math::Quat,
    pub linear_velocity: crate::math::Vec3,
    pub angular_velocity: crate::math::Vec3,
    pub forces: Vec<AppliedForce>,
    pub is_sleeping: bool,
    pub sleep_threshold: f32,
}

impl Default for PhysicsState {
    fn default() -> Self {
        Self {
            position: crate::math::Vec3::ZERO,
            rotation: crate::math::Quat::IDENTITY,
            linear_velocity: crate::math::Vec3::ZERO,
            angular_velocity: crate::math::Vec3::ZERO,
            forces: Vec::new(),
            is_sleeping: false,
            sleep_threshold: 0.01,
        }
    }
}

/// Çarpışma verisi
#[derive(Debug, Clone)]
pub struct CollisionData {
    pub entity_a: Entity,
    pub entity_b: Entity,
    pub contact_points: Vec<crate::math::Vec3>,
    pub normal: crate::math::Vec3,
    pub penetration_depth: f32,
    pub impulse: f32,
}

/// Fiziksel dünya yapılandırması
#[derive(Debug, Clone)]
pub struct PhysicsWorldConfig {
    pub gravity: crate::math::Vec3,
    pub substeps: u32,
    pub max_velocity_iterations: u32,
    pub max_position_iterations: u32,
    pub sleep_enabled: bool,
    pub broad_phase_algorithm: BroadPhaseAlgorithm,
}

impl Default for PhysicsWorldConfig {
    fn default() -> Self {
        Self {
            gravity: crate::math::Vec3::new(0.0, -9.81, 0.0),
            substeps: 1,
            max_velocity_iterations: 8,
            max_position_iterations: 3,
            sleep_enabled: true,
            broad_phase_algorithm: BroadPhaseAlgorithm::AabbTree,
        }
    }
}

/// Geniş faz algoritması
#[derive(Debug, Clone, PartialEq)]
pub enum BroadPhaseAlgorithm {
    BruteForce,
    AabbTree,
    SpatialHash,
}

/// Dar faz algoritması
#[derive(Debug, Clone, PartialEq)]
pub enum NarrowPhaseAlgorithm {
    Gjk,
    Minkowski,
}

/// Fizik motoru durumu
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PhysicsEngineStatus {
    Running,
    Paused,
    Stepping,
}

/// Fiziksel dünya
pub struct PhysicsWorld {
    pub bodies: HashMap<Entity, PhysicsBody>,
    pub states: HashMap<Entity, PhysicsState>,
    pub config: PhysicsWorldConfig,
    pub status: PhysicsEngineStatus,
    pub frame_count: u64,
    pub collision_pairs: Vec<CollisionData>,
    pub broad_phase_tree: AabbTree,
    pub gravity: crate::math::Vec3,
    pub timestep: f32,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self {
            bodies: HashMap::new(),
            states: HashMap::new(),
            config: PhysicsWorldConfig::default(),
            status: PhysicsEngineStatus::Running,
            frame_count: 0,
            collision_pairs: Vec::new(),
            broad_phase_tree: AabbTree::new(),
            gravity: crate::math::Vec3::new(0.0, -9.81, 0.0),
            timestep: 1.0 / 60.0, // 60 FPS
        }
    }

    /// Yeni bir fiziksel nesne oluşturur
    pub fn create_body(&mut self, entity: Entity, body: PhysicsBody, state: PhysicsState) {
        self.bodies.insert(entity, body);
        self.states.insert(entity, state);
    }

    /// Fiziksel nesneyi alır
    pub fn get_body(&self, entity: Entity) -> Option<&PhysicsBody> {
        self.bodies.get(&entity)
    }

    /// Fiziksel nesneyi değiştirir
    pub fn set_body(&mut self, entity: Entity, body: PhysicsBody) -> Option<PhysicsBody> {
        self.bodies.insert(entity, body)
    }

    /// Fiziksel durumu alır
    pub fn get_state(&self, entity: Entity) -> Option<&PhysicsState> {
        self.states.get(&entity)
    }

    /// Fiziksel durumu değiştirir
    pub fn set_state(&mut self, entity: Entity, state: PhysicsState) -> Option<PhysicsState> {
        self.states.insert(entity, state)
    }

    /// Kuvvet uygular
    pub fn apply_force(&mut self, entity: Entity, force: crate::math::Vec3, force_type: ForceType) {
        if let Some(state) = self.states.get_mut(&entity) {
            state.forces.push(AppliedForce {
                force,
                force_type,
                position: crate::math::Vec3::ZERO, // Varsayılan olarak merkez
            });
        }
    }

    /// Belirli bir pozisyona kuvvet uygular (tork oluşturur)
    pub fn apply_force_at_point(&mut self, entity: Entity, force: crate::math::Vec3, point: crate::math::Vec3) {
        if let Some(state) = self.states.get_mut(&entity) {
            state.forces.push(AppliedForce {
                force,
                force_type: ForceType::Force,
                position: point,
            });
        }
    }

    /// Fiziksel dünyayı günceller
    pub fn step(&mut self, delta_time: f32) {
        if self.status != PhysicsEngineStatus::Running && self.status != PhysicsEngineStatus::Stepping {
            return;
        }

        // Alt adımlar için zamanı böl
        let substep_dt = delta_time / self.config.substeps as f32;

        for _ in 0..self.config.substeps {
            // Kuvvetleri uygula
            self.apply_forces(substep_dt);

            // Entegrasyon (konum ve hız güncelleme)
            self.integrate(substep_dt);

            // Çarpışma tespiti
            self.detect_collisions();

            // Çarpışma çözümleme
            self.resolve_collisions();

            // Uyku durumu kontrolü
            if self.config.sleep_enabled {
                self.update_sleep_states();
            }
        }

        self.frame_count += 1;
    }

    /// Kuvvetleri uygular
    fn apply_forces(&mut self, dt: f32) {
        for (entity, state) in self.states.iter_mut() {
            if let Some(body) = self.bodies.get(entity) {
                // Gravitasyon uygula
                if body.body_type == PhysicsBodyType::Dynamic {
                    state.linear_velocity += self.gravity * dt;
                }

                // Uygulanan kuvvetleri ekle
                for applied_force in &state.forces {
                    match applied_force.force_type {
                        ForceType::Force => {
                            if body.body_type == PhysicsBodyType::Dynamic {
                                let acceleration = applied_force.force / body.mass;
                                state.linear_velocity += acceleration * dt;
                            }
                        }
                        ForceType::Impulse => {
                            if body.body_type == PhysicsBodyType::Dynamic {
                                let velocity_change = applied_force.force / body.mass;
                                state.linear_velocity += velocity_change;
                            }
                        }
                        ForceType::Torque => {
                            // Basit tork uygulaması (gerçek uygulama daha karmaşıktır)
                            if body.body_type == PhysicsBodyType::Dynamic {
                                state.angular_velocity += applied_force.force * dt;
                            }
                        }
                    }
                }

                // Damping uygula
                state.linear_velocity *= (1.0 - body.linear_damping * dt).max(0.0);
                state.angular_velocity *= (1.0 - body.angular_damping * dt).max(0.0);
            }

            // Kuvvet listesini temizle
            state.forces.clear();
        }
    }

    /// Entegrasyon (konum ve hız güncelleme)
    fn integrate(&mut self, dt: f32) {
        for (entity, state) in self.states.iter_mut() {
            if let Some(body) = self.bodies.get(entity) {
                if body.body_type != PhysicsBodyType::Static {
                    // Euler integrasyonu
                    state.position += state.linear_velocity * dt;
                    state.rotation *= crate::math::Quat::from_axis_angle(
                        state.angular_velocity.normalize(),
                        state.angular_velocity.length() * dt
                    ).normalize();
                }
            }
        }
    }

    /// Çarpışma tespiti
    fn detect_collisions(&mut self) {
        // Geniş faz: potansiyel çarpışan çiftleri bul
        let potential_pairs = self.broad_phase_collision_detection();

        // Dar faz: gerçek çarpışmaları tespit et
        for (entity_a, entity_b) in potential_pairs {
            if let (Some(body_a), Some(body_b)) = (self.bodies.get(&entity_a), self.bodies.get(&entity_b)) {
                // Çarpışma filtreleme kontrolü
                if (body_a.collision_groups & body_b.collision_masks) == 0 ||
                   (body_b.collision_groups & body_a.collision_masks) == 0 {
                    continue;
                }

                // Gerçek çarpışma tespiti (basit implementasyon)
                if let Some(collision_data) = self.narrow_phase_collision_detection(entity_a, entity_b, body_a, body_b) {
                    self.collision_pairs.push(collision_data);
                }
            }
        }
    }

    /// Geniş faz çarpışma tespiti
    fn broad_phase_collision_detection(&self) -> Vec<(Entity, Entity)> {
        let entities: Vec<_> = self.bodies.keys().cloned().collect();
        let mut pairs = Vec::new();

        // Basit brute-force yaklaşımı
        for i in 0..entities.len() {
            for j in (i + 1)..entities.len() {
                pairs.push((entities[i], entities[j]));
            }
        }

        pairs
    }

    /// Dar faz çarpışma tespiti
    fn narrow_phase_collision_detection(
        &self,
        entity_a: Entity,
        entity_b: Entity,
        body_a: &PhysicsBody,
        body_b: &PhysicsBody,
    ) -> Option<CollisionData> {
        // Basit küre-küre çarpışma tespiti
        if let (CollisionShape::Sphere { radius: rad_a }, CollisionShape::Sphere { radius: rad_b }) = (&body_a.shape, &body_b.shape) {
            if let (Some(state_a), Some(state_b)) = (self.states.get(&entity_a), self.states.get(&entity_b)) {
                let dist_squared = (state_a.position - state_b.position).length_squared();
                let min_dist = rad_a + rad_b;

                if dist_squared < min_dist * min_dist {
                    let dist = dist_squared.sqrt();
                    let normal = if dist > 0.0 {
                        (state_a.position - state_b.position) / dist
                    } else {
                        crate::math::Vec3::X // Varsayılan normal
                    };

                    return Some(CollisionData {
                        entity_a,
                        entity_b,
                        contact_points: vec![(state_a.position + state_b.position) * 0.5],
                        normal,
                        penetration_depth: min_dist - dist,
                        impulse: 0.0, // Basit implementasyonda sıfır
                    });
                }
            }
        }

        None
    }

    /// Çarpışma çözümleme
    fn resolve_collisions(&mut self) {
        for collision in &self.collision_pairs {
            // Clone body types before borrowing
            let (body_type_a, body_type_b) = {
                let body_a = self.bodies.get(&collision.entity_a);
                let body_b = self.bodies.get(&collision.entity_b);
                match (body_a, body_b) {
                    (Some(a), Some(b)) => (a.body_type.clone(), b.body_type.clone()),
                    _ => continue,
                }
            };
            
            // Sadece dinamik nesneleri etkile
            if body_type_a == PhysicsBodyType::Static && body_type_b == PhysicsBodyType::Static {
                continue;
            }

            // Get states separately to avoid borrow issues
            let entity_a = collision.entity_a;
            let entity_b = collision.entity_b;
            
            // Check if both states exist
            let has_both_states = self.states.contains_key(&entity_a) && self.states.contains_key(&entity_b);
            if !has_both_states {
                continue;
            }
            
            // Get bodies for mass calculation
            let (body_a, body_b) = {
                let body_a = self.bodies.get(&entity_a);
                let body_b = self.bodies.get(&entity_b);
                match (body_a, body_b) {
                    (Some(a), Some(b)) => (a.clone(), b.clone()),
                    _ => continue,
                }
            };
            
            // Basit momentum transferi
            // Basit çarpışma çözümü
            let total_mass = match (body_type_a, body_type_b) {
                (PhysicsBodyType::Static, PhysicsBodyType::Static) => 0.0,
                (PhysicsBodyType::Static, PhysicsBodyType::Dynamic) => body_b.mass,
                (PhysicsBodyType::Dynamic, PhysicsBodyType::Static) => body_a.mass,
                (PhysicsBodyType::Dynamic, PhysicsBodyType::Dynamic) => body_a.mass + body_b.mass,
                _ => body_a.mass + body_b.mass,
            };

            if total_mass > 0.0 {
                // Pozisyon düzeltmesi
                let correction = collision.normal * (collision.penetration_depth * 0.5);
                if body_a.body_type != PhysicsBodyType::Static {
                    if let Some(state_a) = self.states.get_mut(&entity_a) {
                        state_a.position += correction;
                    }
                }
                if body_b.body_type != PhysicsBodyType::Static {
                    if let Some(state_b) = self.states.get_mut(&entity_b) {
                        state_b.position -= correction;
                    }
                }
            }
        }

        // Çarpışma listesini temizle
        self.collision_pairs.clear();
    }

    /// Uyku durumlarını günceller
    fn update_sleep_states(&mut self) {
        for (entity, state) in self.states.iter_mut() {
            if let Some(body) = self.bodies.get(entity) {
                if body.body_type != PhysicsBodyType::Dynamic {
                    continue;
                }

                let velocity_squared = state.linear_velocity.length_squared() + state.angular_velocity.length_squared();
                state.is_sleeping = velocity_squared < state.sleep_threshold * state.sleep_threshold;
            }
        }
    }

    /// Fiziksel nesneyi siler
    pub fn remove_body(&mut self, entity: Entity) -> bool {
        self.bodies.remove(&entity).is_some() && self.states.remove(&entity).is_some()
    }

    /// Statik bir nesne mi diye kontrol eder
    pub fn is_static_body(&self, entity: Entity) -> bool {
        if let Some(body) = self.bodies.get(&entity) {
            body.body_type == PhysicsBodyType::Static
        } else {
            false
        }
    }

    /// Dinamik bir nesne mi diye kontrol eder
    pub fn is_dynamic_body(&self, entity: Entity) -> bool {
        if let Some(body) = self.bodies.get(&entity) {
            body.body_type == PhysicsBodyType::Dynamic
        } else {
            false
        }
    }
}

/// AABB (Axis-Aligned Bounding Box) ağacı için basit implementasyon
pub struct AabbTree {
    nodes: Vec<AabbNode>,
    root: Option<usize>,
}

#[derive(Debug, Clone)]
struct AabbNode {
    bounds: Aabb,
    entity: Option<Entity>,
    children: [Option<usize>; 2],
}

#[derive(Debug, Clone)]
struct Aabb {
    min: crate::math::Vec3,
    max: crate::math::Vec3,
}

impl Aabb {
    fn new(min: crate::math::Vec3, max: crate::math::Vec3) -> Self {
        Self { min, max }
    }

    fn contains_point(&self, point: crate::math::Vec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y &&
        point.z >= self.min.z && point.z <= self.max.z
    }

    fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x &&
        self.min.y <= other.max.y && self.max.y >= other.min.y &&
        self.min.z <= other.max.z && self.max.z >= other.min.z
    }

    fn combine(&self, other: &Self) -> Self {
        Aabb::new(
            crate::math::Vec3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            crate::math::Vec3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        )
    }
}

impl AabbTree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: None,
        }
    }

    pub fn insert(&mut self, bounds: Aabb, entity: Entity) {
        let node = AabbNode {
            bounds,
            entity: Some(entity),
            children: [None, None],
        };

        let node_index = self.nodes.len();
        self.nodes.push(node);

        if self.root.is_none() {
            self.root = Some(node_index);
        } else {
            self.insert_recursive(self.root.unwrap(), node_index);
        }
    }

    fn insert_recursive(&mut self, current_index: usize, new_index: usize) {
        let current_bounds = self.nodes[current_index].bounds.clone();
        let new_bounds = self.nodes[new_index].bounds.clone();

        // Hangi çocuğu büyüteceğimize karar ver (daha az büyüyen)
        let extend_left = if let Some(left_child) = self.nodes[current_index].children[0] {
            let combined = current_bounds.combine(&self.nodes[left_child].bounds);
            combined.volume() - current_bounds.volume()
        } else {
            f32::INFINITY
        };

        let extend_right = if let Some(right_child) = self.nodes[current_index].children[1] {
            let combined = current_bounds.combine(&self.nodes[right_child].bounds);
            combined.volume() - current_bounds.volume()
        } else {
            f32::INFINITY
        };

        if extend_left <= extend_right {
            if self.nodes[current_index].children[0].is_none() {
                self.nodes[current_index].children[0] = Some(new_index);
            } else {
                self.insert_recursive(self.nodes[current_index].children[0].unwrap(), new_index);
            }
        } else {
            if self.nodes[current_index].children[1].is_none() {
                self.nodes[current_index].children[1] = Some(new_index);
            } else {
                self.insert_recursive(self.nodes[current_index].children[1].unwrap(), new_index);
            }
        }

        // Üst düğüm sınırlarını güncelle
        if let Some(left_child) = self.nodes[current_index].children[0] {
            if let Some(right_child) = self.nodes[current_index].children[1] {
                self.nodes[current_index].bounds = 
                    self.nodes[left_child].bounds.combine(&self.nodes[right_child].bounds);
            }
        }
    }
}

impl Aabb {
    fn volume(&self) -> f32 {
        let size = self.max - self.min;
        size.x * size.y * size.z
    }
}