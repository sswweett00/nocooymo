//! Contact manifolds and per-point warm-starting state.

use crate::math::Vec3;

/// One persistent contact point between two bodies.
#[derive(Debug, Clone, Copy)]
pub struct ContactPoint {
    /// Point on body A in world space.
    pub world_a: Vec3,
    /// Point on body B in world space.
    pub world_b: Vec3,
    /// Contact normal pointing from A toward B.
    pub normal: Vec3,
    /// Penetration depth (positive when interpenetrating).
    pub separation: f32,
    /// Accumulated normal impulse (warm starting).
    pub normal_impulse: f32,
    /// Accumulated tangent impulse components (warm starting).
    pub tangent_impulse: [f32; 2],
    /// Effective normal mass (precomputed for the solver).
    pub normal_mass: f32,
    /// Effective tangent mass.
    pub tangent_mass: f32,
    /// Relative normal velocity at the point.
    pub rel_normal_vel: f32,
}

impl ContactPoint {
    pub fn new(world_a: Vec3, world_b: Vec3, normal: Vec3, separation: f32) -> Self {
        Self {
            world_a,
            world_b,
            normal,
            separation,
            normal_impulse: 0.0,
            tangent_impulse: [0.0, 0.0],
            normal_mass: 0.0,
            tangent_mass: 0.0,
            rel_normal_vel: 0.0,
        }
    }
}

/// A manifold is the resolved contact set between exactly two bodies.
#[derive(Debug, Clone)]
pub struct ContactManifold {
    pub a: u32,
    pub b: u32,
    pub points: Vec<ContactPoint>,
    pub friction: f32,
    pub restitution: f32,
    /// True when the pair is awake and needs solving.
    pub active: bool,
}

impl ContactManifold {
    pub fn new(a: u32, b: u32) -> Self {
        Self {
            a,
            b,
            points: Vec::new(),
            friction: 0.5,
            restitution: 0.0,
            active: false,
        }
    }

    pub fn is_sensor(&self) -> bool {
        self.points.is_empty()
    }
}
