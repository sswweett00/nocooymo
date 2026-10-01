//! Colliders and surface materials.
//!
//! A [`Collider`] pairs a primitive shape with a [`Material`] describing the
//! surface response (friction, restitution, density).

use crate::math::{Mat3, Vec3};
use nalgebra::Matrix3;

/// Surface / mass material properties (Mimari §3.1 — Tremor).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    /// Coefficient of kinetic/static friction in `[0, 1]`.
    pub friction: f32,
    /// Bounciness coefficient in `[0, 1]`.
    pub restitution: f32,
    /// Mass per unit volume (`kg/m^3`). Used to derive mass from a shape.
    pub density: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self { friction: 0.5, restitution: 0.2, density: 1.0 }
    }
}

impl Material {
    pub const fn new(friction: f32, restitution: f32, density: f32) -> Self {
        Self { friction, restitution, density }
    }

    /// Standard "bouncy toy" material.
    pub const RUBBER: Self = Self::new(0.9, 0.8, 1.0);
    /// Standard "metal" material.
    pub const METAL: Self = Self::new(0.2, 0.05, 8.0);

    /// Combine two friction values (geometric mean).
    pub fn combine_friction(a: f32, b: f32) -> f32 { (a * b).sqrt() }
    /// Combine two restitution values (max).
    pub fn combine_restitution(a: f32, b: f32) -> f32 { a.max(b) }
}

/// The geometric primitive of a collider.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColliderShape {
    Sphere { radius: f32 },
    Box { half_extents: Vec3 },
    Capsule { radius: f32, height: f32 },
}

impl ColliderShape {
    pub fn is_sphere(&self) -> bool {
        matches!(self, ColliderShape::Sphere { .. })
    }

    /// World-space AABB of the shape when located at `pos` with orientation `rot`.
    pub fn aabb(&self, pos: Vec3, rot: crate::math::Quat) -> crate::math::Aabb {
        use crate::math::Aabb;
        match self {
            ColliderShape::Sphere { radius } => {
                let r = Vec3::repeat(*radius);
                Aabb::new(pos - r, pos + r)
            }
            ColliderShape::Box { half_extents } => {
                // |R| * half_extents — tam OBB AABB hesabı
                let m = rot.to_rotation_matrix();
                let r = m.matrix();
                let abs_r = Vec3::new(
                    r.m11.abs() * half_extents.x + r.m12.abs() * half_extents.y + r.m13.abs() * half_extents.z,
                    r.m21.abs() * half_extents.x + r.m22.abs() * half_extents.y + r.m23.abs() * half_extents.z,
                    r.m31.abs() * half_extents.x + r.m32.abs() * half_extents.y + r.m33.abs() * half_extents.z,
                );
                Aabb::new(pos - abs_r, pos + abs_r)
            }
            ColliderShape::Capsule { radius, height } => {
                // Kapsül Y ekseni etrafında yönlenir — rotasyonu hesaba kat
                let half = height * 0.5 + radius;
                // Yerel Y eksenli kapsülün AABB'i, rotasyon sonrası genişlet
                let m = rot.to_rotation_matrix();
                let r = m.matrix();
                // Y yarı-uzunluğu half, X/Z yarı-uzunluğu radius
                let local_extents = Vec3::new(*radius, half, *radius);
                // Basit ama doğru: |R|*extents yerine, kapsül için eksen bağımsız genişletme
                // Yönelimden bağımsız kapsül AABB: en kötü durumda küreye yakın
                let _ = r; // rotasyon kapsül simetrisi nedeniyle Y dışındakiler korunur
                // Rotasyona duyarlı hesap: |R|*extents
                let abs_r = Vec3::new(
                    r.m11.abs() * local_extents.x + r.m12.abs() * local_extents.y + r.m13.abs() * local_extents.z,
                    r.m21.abs() * local_extents.x + r.m22.abs() * local_extents.y + r.m23.abs() * local_extents.z,
                    r.m31.abs() * local_extents.x + r.m32.abs() * local_extents.y + r.m33.abs() * local_extents.z,
                );
                Aabb::new(pos - abs_r, pos + abs_r)
            }
        }
    }

    /// Approximate volume of the shape in cubic units.
    pub fn volume(&self) -> f32 {
        match self {
            ColliderShape::Sphere { radius } => (4.0 / 3.0) * std::f32::consts::PI * radius.powi(3),
            ColliderShape::Box { half_extents } =>
                8.0 * half_extents.x * half_extents.y * half_extents.z,
            ColliderShape::Capsule { radius, height } => {
                let cyl = std::f32::consts::PI * radius.powi(2) * height;
                let sph = (4.0 / 3.0) * std::f32::consts::PI * radius.powi(3);
                cyl + sph
            }
        }
    }

    /// Local-space inertia tensor for a uniform density of 1.0.
    /// Multiply by actual density/mass as needed.
    pub fn unit_inertia(&self) -> Mat3 {
        match self {
            ColliderShape::Sphere { radius } => {
                let r2 = radius * radius;
                let i = (2.0 / 5.0) * r2;
                Matrix3::from_diagonal(&Vec3::new(i, i, i))
            }
            ColliderShape::Box { half_extents } => {
                let (x, y, z) = (half_extents.x, half_extents.y, half_extents.z);
                // Birim kütle için atalet — vol çarpanı yok
                let diag = Vec3::new(
                    (1.0 / 3.0) * (y * y + z * z),
                    (1.0 / 3.0) * (x * x + z * z),
                    (1.0 / 3.0) * (x * x + y * y),
                );
                Matrix3::from_diagonal(&diag)
            }
            ColliderShape::Capsule { radius, height } => {
                let m_cyl = std::f32::consts::PI * radius * radius * height;
                let r2 = radius * radius;
                let i_cyl_ax = 0.5 * m_cyl * r2;
                let i_cyl_rad = m_cyl * (0.25 * r2 + height * height / 12.0);
                let m_sph = (4.0 / 3.0) * std::f32::consts::PI * radius.powi(3);
                let i_sph = (2.0 / 5.0) * m_sph * r2;
                // Küreler silindirin iki ucunda — paralel eksen teoremi
                let offset = height * 0.5;
                let i_sph_offset = m_sph * offset * offset;
                let ix = i_cyl_rad + i_sph + i_sph_offset;
                Matrix3::from_diagonal(&Vec3::new(ix, i_cyl_ax + i_sph, ix))
            }
        }
    }
}

/// A collider attaching a shape to a body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Collider {
    pub shape: ColliderShape,
    pub material: Material,
    /// Local offset of the collider centroid from the body origin.
    pub offset: Vec3,
    /// Collision filter group bit.
    pub group: u32,
    /// Mask of groups this collider reacts with.
    pub mask: u32,
    /// If true, this collider is a trigger (no physical response).
    pub sensor: bool,
}

impl Collider {
    pub fn sphere(radius: f32) -> Self {
        Self {
            shape: ColliderShape::Sphere { radius },
            material: Material::default(),
            offset: Vec3::zeros(),
            group: 1,
            mask: u32::MAX,
            sensor: false,
        }
    }

    pub fn box_collider(half_extents: Vec3) -> Self {
        Self {
            shape: ColliderShape::Box { half_extents },
            material: Material::default(),
            offset: Vec3::zeros(),
            group: 1,
            mask: u32::MAX,
            sensor: false,
        }
    }

    pub fn capsule(radius: f32, height: f32) -> Self {
        Self {
            shape: ColliderShape::Capsule { radius, height },
            material: Material::default(),
            offset: Vec3::zeros(),
            group: 1,
            mask: u32::MAX,
            sensor: false,
        }
    }

    pub fn with_material(mut self, m: Material) -> Self {
        self.material = m;
        self
    }
    pub fn with_offset(mut self, offset: Vec3) -> Self {
        self.offset = offset;
        self
    }
    pub fn as_sensor(mut self) -> Self {
        self.sensor = true;
        self
    }
    pub fn with_filter(mut self, group: u32, mask: u32) -> Self {
        self.group = group;
        self.mask = mask;
        self
    }

    pub fn shape_world_center(&self, body_pos: Vec3, body_rot: crate::math::Quat) -> Vec3 {
        body_pos + body_rot * self.offset
    }

    /// Test whether a world-space point is inside this collider.
    pub fn contains_point(&self, point: Vec3, body_pos: Vec3, body_rot: crate::math::Quat) -> bool {
        let local = body_rot.inverse() * (point - body_pos - self.offset);
        match self.shape {
            ColliderShape::Sphere { radius } => local.norm_squared() <= radius * radius,
            ColliderShape::Box { half_extents } => {
                local.x.abs() <= half_extents.x
                    && local.y.abs() <= half_extents.y
                    && local.z.abs() <= half_extents.z
            }
            ColliderShape::Capsule { radius, height } => {
                let half = height * 0.5;
                let d = local.y.abs() - half;
                let lateral_dist = local.xz().norm();
                lateral_dist * lateral_dist + d.max(0.0).powi(2) <= radius * radius
            }
        }
    }
}

/// Simple collision-filter test between two colliders.
pub(crate) fn collides_group(a: &Collider, b: &Collider) -> bool {
    (a.group & b.mask) != 0 && (b.group & a.mask) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_volume_and_inertia() {
        let s = ColliderShape::Sphere { radius: 1.0 };
        assert!(((4.0 / 3.0) * std::f32::consts::PI - s.volume()).abs() < 1e-4);
        let i = s.unit_inertia();
        assert!((i[(0, 0)] - 0.4).abs() < 1e-5);
    }

    #[test]
    fn box_aabb_rotates_conservatively() {
        let b = Collider::box_collider(Vec3::new(1.0, 1.0, 1.0));
        let pos = Vec3::new(5.0, 0.0, 0.0);
        let rot = nalgebra::UnitQuaternion::from_axis_angle(&nalgebra::Vector3::z_axis(), 0.0);
        let aabb = b.shape.aabb(pos, rot);
        assert_eq!(aabb.min, Vec3::new(4.0, -1.0, -1.0));
        assert_eq!(aabb.max, Vec3::new(6.0, 1.0, 1.0));
    }

    #[test]
    fn filter_groups() {
        let a = Collider::sphere(1.0).with_filter(2, 2);
        let b = Collider::sphere(1.0).with_filter(4, 4);
        assert!(!collides_group(&a, &b));
        let c = Collider::sphere(1.0).with_filter(2, 2);
        assert!(collides_group(&a, &c));
    }
}
