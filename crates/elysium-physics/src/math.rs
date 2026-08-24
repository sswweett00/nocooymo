//! Core math primitives for the Tremor physics engine.
//!
//! The engine standardises on `nalgebra` single-precision linear algebra.
//! Position and direction are `Vec3`; orientation is a unit quaternion `Quat`.
//! `Aabb` is the axis-aligned bounding box used by the broad phase.

use nalgebra::{Matrix3, UnitQuaternion, Vector3};

/// A 3D vector (translation, velocity, force, …).
pub type Vec3 = Vector3<f32>;

/// A unit quaternion encoding orientation.
pub type Quat = UnitQuaternion<f32>;

/// A 3x3 matrix (inertia tensors, …).
pub type Mat3 = Matrix3<f32>;

/// Zero vector constant.
pub const ZERO: Vec3 = Vector3::new(0.0, 0.0, 0.0);

/// Axis-aligned bounding box (world space).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// An empty / inverted box used as an accumulator identity.
    pub fn empty() -> Self {
        Self {
            min: Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            max: Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    pub fn contains_point(&self, p: &Vec3) -> bool {
        self.min.x <= p.x
            && p.x <= self.max.x
            && self.min.y <= p.y
            && p.y <= self.max.y
            && self.min.z <= p.z
            && p.z <= self.max.z
    }

    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && other.min.x <= self.max.x
            && self.min.y <= other.max.y
            && other.min.y <= self.max.y
            && self.min.z <= other.max.z
            && other.min.z <= self.max.z
    }

    /// Grow the box to also contain `other`.
    pub fn merge(&mut self, other: &Aabb) {
        self.min = self.min.inf(&other.min);
        self.max = self.max.sup(&other.max);
    }

    /// Inflate the box by `margin` on every side.
    pub fn fatten(&self, margin: f32) -> Aabb {
        let m = Vec3::repeat(margin);
        Aabb::new(self.min - m, self.max + m)
    }

    pub fn surface_area(&self) -> f32 {
        let d = self.max - self.min;
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }

    /// The cell coordinate range this box covers for a grid of `cell_size`.
    pub fn cells(&self, cell_size: f32) -> (Vec3, Vec3) {
        let min = Vec3::new(
            (self.min.x / cell_size).floor(),
            (self.min.y / cell_size).floor(),
            (self.min.z / cell_size).floor(),
        );
        let max = Vec3::new(
            (self.max.x / cell_size).floor(),
            (self.max.y / cell_size).floor(),
            (self.max.z / cell_size).floor(),
        );
        (min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aabb_intersection() {
        let a = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let b = Aabb::new(Vec3::new(0.5, 0.5, 0.5), Vec3::new(2.0, 2.0, 2.0));
        let c = Aabb::new(Vec3::new(5.0, 5.0, 5.0), Vec3::new(6.0, 6.0, 6.0));
        assert!(a.intersects(&b));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn aabb_merge_and_fatten() {
        let a = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let mut m = a;
        m.merge(&Aabb::new(Vec3::new(2.0, 2.0, 2.0), Vec3::new(3.0, 3.0, 3.0)));
        assert_eq!(m.max, Vec3::new(3.0, 3.0, 3.0));
        let fat = a.fatten(0.1);
        assert!((fat.min.x + 0.1).abs() < 1e-6);
    }
}
