/// elysium_core::math — glam türlerinin yeniden ihracatı.
/// elysium-render ve diğer crate'lerin `use elysium_core::math::*` ile
/// kullanabileceği merkezi matematik modülü.

pub use glam::{
    EulerRot,
    Vec2, Vec3, Vec4,
    Mat2, Mat3, Mat4,
    Quat,
    IVec2, IVec3, IVec4,
    UVec2, UVec3, UVec4,
    BVec2, BVec3, BVec4,
    vec2, vec3, vec4,
    mat2, mat3, mat4,
    quat,
};

/// Axis-aligned bounding box (3D).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    pub fn contains_point(&self, p: Vec3) -> bool {
        p.cmpge(self.min).all() && p.cmple(self.max).all()
    }

    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.cmple(other.max).all() && self.max.cmpge(other.min).all()
    }

    pub fn expand_by_point(&self, p: Vec3) -> Self {
        Self {
            min: self.min.min(p),
            max: self.max.max(p),
        }
    }

    pub fn merge(&self, other: &Aabb) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

impl Default for Aabb {
    fn default() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }
}

/// 3D frustum - kamera görüş alanı.
#[derive(Debug, Clone, Copy)]
pub struct Frustum {
    /// [left, right, bottom, top, near, far]
    pub planes: [Vec4; 6],
}

impl Frustum {
    /// MVP matrisinden frustum düzlemlerini çıkar (Gribb-Hartmann yöntemi).
    pub fn from_view_projection(vp: Mat4) -> Self {
        let m = vp.to_cols_array_2d();
        // Her sütun: m[col][row]
        let row0 = Vec4::new(m[0][0], m[1][0], m[2][0], m[3][0]);
        let row1 = Vec4::new(m[0][1], m[1][1], m[2][1], m[3][1]);
        let row2 = Vec4::new(m[0][2], m[1][2], m[2][2], m[3][2]);
        let row3 = Vec4::new(m[0][3], m[1][3], m[2][3], m[3][3]);

        let planes = [
            (row3 + row0).normalize(), // left
            (row3 - row0).normalize(), // right
            (row3 + row1).normalize(), // bottom
            (row3 - row1).normalize(), // top
            (row3 + row2).normalize(), // near
            (row3 - row2).normalize(), // far
        ];

        Self { planes }
    }

    /// Küre frustum ile kesişiyor mu?
    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            let dist = plane.truncate().dot(center) + plane.w;
            if dist < -radius {
                return false;
            }
        }
        true
    }

    /// AABB frustum ile kesişiyor mu?
    pub fn intersects_aabb(&self, aabb: &Aabb) -> bool {
        for plane in &self.planes {
            let n = plane.truncate();
            // En pozitif köşeyi seç
            let p = Vec3::new(
                if n.x >= 0.0 { aabb.max.x } else { aabb.min.x },
                if n.y >= 0.0 { aabb.max.y } else { aabb.min.y },
                if n.z >= 0.0 { aabb.max.z } else { aabb.min.z },
            );
            if n.dot(p) + plane.w < 0.0 {
                return false;
            }
        }
        true
    }
}

/// Işın (3D ray).
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3, // normalize edilmiş
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }

    /// AABB ile kesişim testi. Kesişiyorsa `Some(t_min)` döner — sıfır yön bileşenlerini güvenli işler.
    pub fn intersects_aabb(&self, aabb: &Aabb) -> Option<f32> {
        const EPS: f32 = 1e-9;
        let mut t_min = f32::NEG_INFINITY;
        let mut t_max = f32::INFINITY;
        for i in 0..3 {
            let dir = self.direction[i];
            let origin = self.origin[i];
            let min = aabb.min[i];
            let max = aabb.max[i];
            if dir.abs() < EPS {
                if origin < min || origin > max {
                    return None;
                }
            } else {
                let inv = 1.0 / dir;
                let mut t1 = (min - origin) * inv;
                let mut t2 = (max - origin) * inv;
                if t1 > t2 {
                    std::mem::swap(&mut t1, &mut t2);
                }
                t_min = t_min.max(t1);
                t_max = t_max.min(t2);
                if t_min > t_max {
                    return None;
                }
            }
        }
        if t_max >= 0.0 {
            Some(t_min.max(0.0))
        } else {
            None
        }
    }

    /// Üçgen ile Möller–Trumbore kesişim testi.
    pub fn intersects_triangle(&self, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
        const EPSILON: f32 = 1e-7;
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let h = self.direction.cross(edge2);
        let a = edge1.dot(h);
        if a.abs() < EPSILON {
            return None;
        }
        let f = 1.0 / a;
        let s = self.origin - v0;
        let u = f * s.dot(h);
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let q = s.cross(edge1);
        let v = f * self.direction.dot(q);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = f * edge2.dot(q);
        if t > EPSILON { Some(t) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_intersects() {
        let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let b = Aabb::new(Vec3::splat(0.5), Vec3::splat(1.5));
        assert!(a.intersects(&b));
        let c = Aabb::new(Vec3::splat(2.0), Vec3::splat(3.0));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn test_ray_aabb() {
        let ray = Ray::new(Vec3::new(0.0, 0.5, -5.0), Vec3::Z);
        let aabb = Aabb::new(Vec3::new(-1.0, 0.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(ray.intersects_aabb(&aabb).is_some());
    }

    #[test]
    fn test_ray_triangle() {
        let ray = Ray::new(Vec3::new(0.3, 0.3, -1.0), Vec3::Z);
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);
        assert!(ray.intersects_triangle(v0, v1, v2).is_some());
    }
}
