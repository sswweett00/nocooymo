//! Enterprise math primitives: vectors, matrices, quaternions, AABB, rays,
//! planes, frusta, and interpolation.
//!
//! Uses `glam` for SIMD-accelerated linear algebra under the hood, but exposes
//! engine-friendly types and additional geometric utilities.

pub use glam::{
    Affine3A, BVec2, BVec3, BVec4, IVec2, IVec3, IVec4, Mat2, Mat3, Mat4, Quat, UVec2, UVec3,
    UVec4, Vec2, Vec3, Vec4,
};

/// A generational handle into a sparse resource (entity ids, etc.).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Handle {
    pub index: u32,
    pub generation: u32,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const PI: f32 = std::f32::consts::PI;
pub const TAU: f32 = std::f32::consts::TAU;
pub const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
pub const EPSILON: f32 = 1e-6;
pub const DEG_TO_RAD: f32 = PI / 180.0;
pub const RAD_TO_DEG: f32 = 180.0 / PI;

// ---------------------------------------------------------------------------
// Angle utilities
// ---------------------------------------------------------------------------

#[inline]
pub fn deg_to_rad(deg: f32) -> f32 {
    deg * DEG_TO_RAD
}

#[inline]
pub fn rad_to_deg(rad: f32) -> f32 {
    rad * RAD_TO_DEG
}

/// Linear interpolation between two floats.
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Smoothstep interpolation: `t*t*(3-2*t)`.
#[inline]
pub fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Smootherstep interpolation: `t*t*t*(t*(t*6-15)+10)`.
#[inline]
pub fn smootherstep(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Clamp a value to a range.
#[inline]
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.clamp(min, max)
}

/// Clamp value to [0, 1].
#[inline]
pub fn saturate(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

/// Map `x` from `[in_min, in_max]` to `[out_min, out_max]`.
#[inline]
pub fn remap(x: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    let t = (x - in_min) / (in_max - in_min);
    lerp(out_min, out_max, t)
}

/// Move `current` toward `target` by at most `max_delta`.
#[inline]
pub fn move_toward(current: f32, target: f32, max_delta: f32) -> f32 {
    if current < target {
        (current + max_delta).min(target)
    } else {
        (current - max_delta).max(target)
    }
}

/// Frame-rate independent exponential damping.
/// `damping` is the fraction remaining after 1 second (e.g. 0.001 = 99.9% gone).
#[inline]
pub fn damp(current: f32, target: f32, damping: f32, dt: f32) -> f32 {
    let t = 1.0 - damping.powf(dt);
    lerp(current, target, t)
}

// ---------------------------------------------------------------------------
// Color utilities
// ---------------------------------------------------------------------------

/// Linear RGB color with alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);
    pub const CYAN: Self = Self::rgb(0.0, 1.0, 1.0);
    pub const MAGENTA: Self = Self::rgb(1.0, 0.0, 1.0);
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create from 0–255 sRGB bytes.
    pub fn from_srgb_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::rgba(
            srgb_to_linear(r as f32 / 255.0),
            srgb_to_linear(g as f32 / 255.0),
            srgb_to_linear(b as f32 / 255.0),
            a as f32 / 255.0,
        )
    }

    pub fn to_u32_rgba(self) -> u32 {
        let r = (saturate(self.r) * 255.0) as u32;
        let g = (saturate(self.g) * 255.0) as u32;
        let b = (saturate(self.b) * 255.0) as u32;
        let a = (saturate(self.a) * 255.0) as u32;
        (a << 24) | (b << 16) | (g << 8) | r
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn to_vec4(self) -> Vec4 {
        Vec4::new(self.r, self.g, self.b, self.a)
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self::rgba(
            lerp(self.r, other.r, t),
            lerp(self.g, other.g, t),
            lerp(self.b, other.b, t),
            lerp(self.a, other.a, t),
        )
    }

    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    pub fn luminance(self) -> f32 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }
}

/// sRGB to linear conversion.
#[inline]
pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear to sRGB conversion.
#[inline]
pub fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

// ---------------------------------------------------------------------------
// AABB
// ---------------------------------------------------------------------------

/// Axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub const EMPTY: Self = Self {
        min: Vec3::splat(f32::INFINITY),
        max: Vec3::splat(f32::NEG_INFINITY),
    };

    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_center_half_extents(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    pub fn extents(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn volume(&self) -> f32 {
        let e = self.extents();
        e.x * e.y * e.z
    }

    pub fn surface_area(&self) -> f32 {
        let e = self.extents();
        2.0 * (e.x * e.y + e.y * e.z + e.z * e.x)
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.min.cmple(other.max).all() && self.max.cmpge(other.min).all()
    }

    pub fn merged(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn expand(&self, point: Vec3) -> Self {
        Self {
            min: self.min.min(point),
            max: self.max.max(point),
        }
    }

    pub fn transform(&self, mat: Mat4) -> Self {
        let corners = [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ];
        let mut result = Aabb::EMPTY;
        for c in corners {
            let t = mat.transform_point3(c);
            result = result.expand(t);
        }
        result
    }

    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }
}

impl Default for Aabb {
    fn default() -> Self {
        Self::EMPTY
    }
}

// ---------------------------------------------------------------------------
// Ray
// ---------------------------------------------------------------------------

/// A ray with origin and direction (normalised).
#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, dir: Vec3) -> Self {
        Self {
            origin,
            dir: dir.normalize_or_zero(),
        }
    }

    pub fn point_at(&self, t: f32) -> Vec3 {
        self.origin + self.dir * t
    }

    /// Ray-AABB intersection using the slab method.  Returns `(tmin, tmax)` if hit.
    pub fn intersect_aabb(&self, aabb: &Aabb) -> Option<(f32, f32)> {
        let inv_dir = Vec3::new(
            1.0 / self.dir.x,
            1.0 / self.dir.y,
            1.0 / self.dir.z,
        );

        let t1 = (aabb.min - self.origin) * inv_dir;
        let t2 = (aabb.max - self.origin) * inv_dir;

        let tmin = t1.min(t2);
        let tmax = t1.max(t2);

        let t_enter = tmin.x.max(tmin.y).max(tmin.z);
        let t_exit = tmax.x.min(tmax.y).min(tmax.z);

        if t_enter <= t_exit && t_exit >= 0.0 {
            Some((t_enter.max(0.0), t_exit))
        } else {
            None
        }
    }

    /// Ray-triangle intersection (Möller–Trumbore).  Returns `t` if hit.
    pub fn intersect_triangle(&self, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let h = self.dir.cross(edge2);
        let a = edge1.dot(h);
        if a.abs() < EPSILON {
            return None; // parallel
        }
        let f = 1.0 / a;
        let s = self.origin - v0;
        let u = f * s.dot(h);
        if u < 0.0 || u > 1.0 {
            return None;
        }
        let q = s.cross(edge1);
        let v = f * self.dir.dot(q);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = f * edge2.dot(q);
        if t > EPSILON {
            Some(t)
        } else {
            None
        }
    }

    /// Ray-sphere intersection.  Returns nearest `t` if hit.
    pub fn intersect_sphere(&self, center: Vec3, radius: f32) -> Option<f32> {
        let oc = self.origin - center;
        let a = self.dir.dot(self.dir);
        let b = 2.0 * oc.dot(self.dir);
        let c = oc.dot(oc) - radius * radius;
        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 {
            return None;
        }
        let sqrt_d = discriminant.sqrt();
        let t1 = (-b - sqrt_d) / (2.0 * a);
        let t2 = (-b + sqrt_d) / (2.0 * a);
        if t1 > EPSILON {
            Some(t1)
        } else if t2 > EPSILON {
            Some(t2)
        } else {
            None
        }
    }

    /// Ray-plane intersection.  Plane defined by point `p` and normal `n`.
    pub fn intersect_plane(&self, p: Vec3, n: Vec3) -> Option<f32> {
        let denom = n.dot(self.dir);
        if denom.abs() < EPSILON {
            return None;
        }
        let t = (p - self.origin).dot(n) / denom;
        if t >= 0.0 {
            Some(t)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Plane
// ---------------------------------------------------------------------------

/// A plane defined by normal and distance from origin.
#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub normal: Vec3,
    pub d: f32,
}

impl Plane {
    pub fn new(normal: Vec3, d: f32) -> Self {
        Self {
            normal: normal.normalize_or_zero(),
            d,
        }
    }

    pub fn from_point_normal(point: Vec3, normal: Vec3) -> Self {
        let n = normal.normalize_or_zero();
        Self {
            normal: n,
            d: -n.dot(point),
        }
    }

    pub fn distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.d
    }

    pub fn is_front_facing(&self, dir: Vec3) -> bool {
        self.normal.dot(dir) < 0.0
    }
}

// ---------------------------------------------------------------------------
// Frustum
// ---------------------------------------------------------------------------

/// A view frustum defined by 6 planes.
#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Extract frustum planes from a view-projection matrix.
    /// Planes are ordered: left, right, bottom, top, near, far.
    pub fn from_view_proj(vp: Mat4) -> Self {
        let m = vp.to_cols_array();
        // glam Mat4 is column-major: m[0..4] = col0, m[4..8] = col1, ...
        let (m00, m01, m02, m03) = (m[0], m[1], m[2], m[3]);
        let (m10, m11, m12, m13) = (m[4], m[5], m[6], m[7]);
        let (m20, m21, m22, m23) = (m[8], m[9], m[10], m[11]);
        let (m30, m31, m32, m33) = (m[12], m[13], m[14], m[15]);

        let normalize = |n: Vec3, d: f32| -> Plane {
            let len = n.length();
            if len > EPSILON {
                Plane::new(n / len, d / len)
            } else {
                Plane::new(n, d)
            }
        };

        // Left: m30 + m00
        let left = normalize(
            Vec3::new(m30 + m00, m31 + m01, m32 + m02),
            m33 + m03,
        );
        // Right: m30 - m00
        let right = normalize(
            Vec3::new(m30 - m00, m31 - m01, m32 - m02),
            m33 - m03,
        );
        // Bottom: m30 + m10
        let bottom = normalize(
            Vec3::new(m30 + m10, m31 + m11, m32 + m12),
            m33 + m13,
        );
        // Top: m30 - m10
        let top = normalize(
            Vec3::new(m30 - m10, m31 - m11, m32 - m12),
            m33 - m13,
        );
        // Near: m30 + m20
        let near = normalize(
            Vec3::new(m30 + m20, m31 + m21, m32 + m22),
            m33 + m23,
        );
        // Far: m30 - m20
        let far = normalize(
            Vec3::new(m30 - m20, m31 - m21, m32 - m22),
            m33 - m23,
        );

        Self {
            planes: [left, right, bottom, top, near, far],
        }
    }

    /// Test whether an AABB is inside or intersects the frustum.
    pub fn intersects_aabb(&self, aabb: &Aabb) -> bool {
        // For each plane, check if all 8 corners are on the negative side.
        let corners = [
            Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z),
            Vec3::new(aabb.max.x, aabb.min.y, aabb.min.z),
            Vec3::new(aabb.min.x, aabb.max.y, aabb.min.z),
            Vec3::new(aabb.max.x, aabb.max.y, aabb.min.z),
            Vec3::new(aabb.min.x, aabb.min.y, aabb.max.z),
            Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z),
            Vec3::new(aabb.min.x, aabb.max.y, aabb.max.z),
            Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z),
        ];
        for plane in &self.planes {
            let mut out = 0;
            for c in &corners {
                if plane.distance(*c) < 0.0 {
                    out += 1;
                }
            }
            if out == 8 {
                return false;
            }
        }
        true
    }

    /// Test whether a point is inside the frustum.
    pub fn contains_point(&self, point: Vec3) -> bool {
        self.planes.iter().all(|p| p.distance(point) >= 0.0)
    }
}

// ---------------------------------------------------------------------------
// Random number generation (Xoshiro256**)
// ---------------------------------------------------------------------------

/// A fast, non-cryptographic PRNG.
pub struct Rng {
    state: [u64; 4],
}

impl Rng {
    pub fn from_seed(seed: u64) -> Self {
        // SplitMix64 to expand seed
        let mut z = seed;
        let mut next = || {
            z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58_4276_4B67_A9CB);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_5C53_0A2D);
            x ^ (x >> 31)
        };
        Self {
            state: [next(), next(), next(), next()],
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.state[1]
            .wrapping_mul(5)
            .rotate_left(7)
            .wrapping_mul(9);
        let t = self.state[1] << 17;
        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];
        self.state[2] ^= t;
        self.state[3] = self.state[3].rotate_left(45);
        result
    }

    pub fn next_f32(&mut self) -> f32 {
        let bits = (self.next_u64() >> 40) as u32;
        (bits as f32) / (1u64 << 24) as f32
    }

    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }

    pub fn range_i32(&mut self, min: i32, max: i32) -> i32 {
        min + (self.next_u64() % (max - min + 1) as u64) as i32
    }

    pub fn unit_vec3(&mut self) -> Vec3 {
        let z = self.next_f32() * 2.0 - 1.0;
        let a = self.next_f32() * TAU;
        let r = (1.0 - z * z).max(0.0).sqrt();
        Vec3::new(r * a.cos(), r * a.sin(), z)
    }
}

// ---------------------------------------------------------------------------
// Easing functions
// ---------------------------------------------------------------------------

pub mod ease {
    use super::*;

    pub fn in_quad(t: f32) -> f32 { t * t }
    pub fn out_quad(t: f32) -> f32 { 1.0 - (1.0 - t) * (1.0 - t) }
    pub fn in_out_quad(t: f32) -> f32 {
        if t < 0.5 { 2.0 * t * t } else { 1.0 - 2.0 * (1.0 - t) * (1.0 - t) }
    }
    pub fn in_cubic(t: f32) -> f32 { t * t * t }
    pub fn out_cubic(t: f32) -> f32 { 1.0 - (1.0 - t).powi(3) }
    pub fn in_out_cubic(t: f32) -> f32 {
        if t < 0.5 { 4.0 * t * t * t } else { 1.0 - 4.0 * (1.0 - t).powi(3) }
    }
    pub fn in_quart(t: f32) -> f32 { t * t * t * t }
    pub fn out_quart(t: f32) -> f32 { 1.0 - (1.0 - t).powi(4) }
    pub fn in_out_quart(t: f32) -> f32 {
        if t < 0.5 { 8.0 * t * t * t * t } else { 1.0 - 8.0 * (1.0 - t).powi(4) }
    }
    pub fn in_sine(t: f32) -> f32 { 1.0 - (t * HALF_PI).cos() }
    pub fn out_sine(t: f32) -> f32 { (t * HALF_PI).sin() }
    pub fn in_out_sine(t: f32) -> f32 { 0.5 - 0.5 * (t * PI).cos() }
    pub fn in_expo(t: f32) -> f32 { if t == 0.0 { 0.0 } else { 2.0_f32.powf(10.0 * t - 10.0) } }
    pub fn out_expo(t: f32) -> f32 { if t == 1.0 { 1.0 } else { 1.0 - 2.0_f32.powf(-10.0 * t) } }
    pub fn in_back(t: f32) -> f32 { 2.70158 * t * t * t - 1.70158 * t * t }
    pub fn out_back(t: f32) -> f32 { 1.0 + 2.70158 * (t - 1.0).powi(3) + 1.70158 * (t - 1.0).powi(2) }
    pub fn out_bounce(t: f32) -> f32 {
        if t < 1.0 / 2.75 { 7.5625 * t * t }
        else if t < 2.0 / 2.75 { 7.5625 * (t - 1.5/2.75) * (t - 1.5/2.75) + 0.75 }
        else if t < 2.5 / 2.75 { 7.5625 * (t - 2.25/2.75) * (t - 2.25/2.75) + 0.9375 }
        else { 7.5625 * (t - 2.625/2.75) * (t - 2.625/2.75) + 0.984375 }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_basic() {
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < EPSILON);
    }

    #[test]
    fn color_lerp() {
        let c = Color::RED.lerp(Color::BLUE, 0.5);
        assert!((c.r - 0.5).abs() < EPSILON);
        assert!((c.b - 0.5).abs() < EPSILON);
    }

    #[test]
    fn aabb_intersect() {
        let a = Aabb::new(Vec3::ZERO, Vec3::new(2.0, 2.0, 2.0));
        let b = Aabb::new(Vec3::new(1.0, 1.0, 1.0), Vec3::new(3.0, 3.0, 3.0));
        assert!(a.intersects(&b));
    }

    #[test]
    fn ray_aabb_hit() {
        let ray = Ray::new(Vec3::new(-5.0, 0.5, 0.5), Vec3::new(1.0, 0.0, 0.0));
        let aabb = Aabb::new(Vec3::ZERO, Vec3::new(2.0, 2.0, 2.0));
        assert!(ray.intersect_aabb(&aabb).is_some());
    }

    #[test]
    fn ray_triangle_hit() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        let v0 = Vec3::new(-1.0, -1.0, 0.0);
        let v1 = Vec3::new(1.0, -1.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);
        let t = ray.intersect_triangle(v0, v1, v2);
        assert!(t.is_some());
        assert!((t.unwrap() - 5.0).abs() < 0.01);
    }

    #[test]
    fn ray_sphere_hit() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        let t = ray.intersect_sphere(Vec3::ZERO, 1.0);
        assert!(t.is_some());
        assert!((t.unwrap() - 4.0).abs() < 0.01);
    }

    #[test]
    fn rng_range() {
        let mut rng = Rng::from_seed(42);
        for _ in 0..1000 {
            let v = rng.range(0.0, 1.0);
            assert!(v >= 0.0 && v < 1.0);
        }
    }

    #[test]
    fn frustum_from_proj() {
        let proj = Mat4::perspective_rh(deg_to_rad(90.0), 1.0, 0.1, 100.0);
        let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
        let f = Frustum::from_view_proj(proj * view);
        assert!(f.contains_point(Vec3::ZERO));
        // Far beyond far plane should be outside
        assert!(!f.contains_point(Vec3::new(0.0, 0.0, -200.0)));
    }

    #[test]
    fn ease_bounce() {
        assert!((ease::out_bounce(0.0)).abs() < EPSILON);
        assert!((ease::out_bounce(1.0) - 1.0).abs() < EPSILON);
    }
}
