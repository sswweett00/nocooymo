//! Ray casting against colliders (Mimari — picking, line-of-sight, vehicles).

use crate::body::RigidBody;
use crate::collider::ColliderShape;
use crate::math::Vec3;

/// A world-space ray.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, dir: Vec3) -> Self {
        Self { origin, dir }
    }

    pub fn point_at(&self, t: f32) -> Vec3 {
        self.origin + self.dir * t
    }
}

/// The nearest ray hit against a single body's collider.
#[derive(Debug, Clone, Copy)]
pub struct RayHit {
    pub body: u32,
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
}

/// Cast against one body; returns the closest hit parameter `t` if any.
pub fn cast_body(ray: &Ray, body: &RigidBody, max_t: f32) -> Option<(f32, Vec3)> {
    let c = body.collider.as_ref()?;
    let center = c.shape_world_center(body.pos, body.rot);
    let hit = match c.shape {
        ColliderShape::Sphere { radius } => {
            ray_sphere(ray, center, radius, max_t)?
        }
        ColliderShape::Box { half_extents } => {
            ray_box(ray, center, body.rot, half_extents, max_t)?
        }
        ColliderShape::Capsule { radius, height } => {
            ray_capsule(ray, center, body.rot, radius, height, max_t)?
        }
    };
    Some(hit)
}

/// Cast against all bodies, returning the closest hit.
pub fn cast_world(bodies: &[RigidBody], ray: &Ray, max_t: f32) -> Option<RayHit> {
    let mut best: Option<RayHit> = None;
    for (i, body) in bodies.iter().enumerate() {
        let c = body.collider.as_ref()?;
        let _ = c;
        if let Some((t, normal)) = cast_body(ray, body, max_t) {
            let replace = match &best {
                Some(b) => t < b.t,
                None => true,
            };
            if replace {
                best = Some(RayHit {
                    body: i as u32,
                    t,
                    point: ray.point_at(t),
                    normal,
                });
            }
        }
    }
    best
}

fn ray_sphere(ray: &Ray, center: Vec3, r: f32, max_t: f32) -> Option<(f32, Vec3)> {
    let oc = ray.origin - center;
    let a = ray.dir.dot(&ray.dir);
    let b = 2.0 * oc.dot(&ray.dir);
    let cc = oc.dot(&oc) - r * r;
    let disc = b * b - 4.0 * a * cc;
    if disc < 0.0 {
        return None;
    }
    let sqrt_d = disc.sqrt();
    let mut t = (-b - sqrt_d) / (2.0 * a);
    if t < 0.0 {
        t = (-b + sqrt_d) / (2.0 * a);
    }
    if t < 0.0 || t > max_t {
        return None;
    }
    let point = ray.point_at(t);
    let n = (point - center).normalize();
    Some((t, n))
}

/// Ray vs oriented box using the slab method in box-local space.
fn ray_box(ray: &Ray, center: Vec3, rot: crate::math::Quat, he: Vec3, max_t: f32) -> Option<(f32, Vec3)> {
    let inv = rot.inverse();
    let local_origin = inv * (ray.origin - center);
    let local_dir = inv * ray.dir;
    let mut tmin = 0.0f32;
    let mut tmax = max_t;
    let mut normal = Vec3::zeros();
    for i in 0..3 {
        if local_dir[i].abs() < 1e-9 {
            if local_origin[i] < -he[i] || local_origin[i] > he[i] {
                return None;
            }
        } else {
            let inv_d = 1.0 / local_dir[i];
            let mut t0 = (-he[i] - local_origin[i]) * inv_d;
            let mut t1 = (he[i] - local_origin[i]) * inv_d;
            let mut n_sign = 1.0f32;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
                n_sign = -1.0;
            }
            if t0 > tmin {
                tmin = t0;
                normal = Vec3::zeros();
                normal[i] = n_sign;
            }
            if t1 < tmax {
                tmax = t1;
            }
            if tmin > tmax {
                return None;
            }
        }
    }
    if tmin < 0.0 {
        tmin = tmax;
    }
    if tmin < 0.0 || tmin > max_t {
        return None;
    }
    Some((tmin, rot * normal))
}

/// Ray vs capsule as two spheres + a rounded cylinder.
fn ray_capsule(ray: &Ray, center: Vec3, rot: crate::math::Quat, r: f32, height: f32, max_t: f32) -> Option<(f32, Vec3)> {
    let half = height * 0.5;
    let c0 = center + rot * Vec3::new(0.0, -half, 0.0);
    let c1 = center + rot * Vec3::new(0.0, half, 0.0);
    let axis = c1 - c0;
    let axis_len = axis.norm();
    let axis_dir = if axis_len > 1e-9 { axis / axis_len } else { Vec3::new(0.0, 1.0, 0.0) };

    let mut result: Option<(f32, Vec3)> = None;
    // Cylinder (infinite) clipped to the segment.
    if let Some((t, n)) = ray_infinite_cylinder(ray, c0, axis_dir, r, max_t) {
        let p = ray.point_at(t);
        let proj = (p - c0).dot(&axis_dir);
        if proj >= -r && proj <= axis_len + r {
            result = Some((t, n));
        }
    }
    for cap in [c0, c1] {
        if let Some((t, n)) = ray_sphere(ray, cap, r, max_t) {
            let better = match result {
                Some((bt, _)) => t < bt,
                None => true,
            };
            if better {
                result = Some((t, n));
            }
        }
    }
    result
}

fn ray_infinite_cylinder(ray: &Ray, base: Vec3, dir: Vec3, r: f32, max_t: f32) -> Option<(f32, Vec3)> {
    let oc = ray.origin - base;
    let a = ray.dir - dir * ray.dir.dot(&dir);
    let b = oc - dir * oc.dot(&dir);
    let aa = a.dot(&a);
    if aa < 1e-9 {
        return None;
    }
    let bb = a.dot(&b);
    let cc = b.dot(&b) - r * r;
    let disc = bb * bb - aa * cc;
    if disc < 0.0 {
        return None;
    }
    let sqrt_d = disc.sqrt();
    let mut t = (-bb - sqrt_d) / aa;
    if t < 0.0 {
        t = (-bb + sqrt_d) / aa;
    }
    if t < 0.0 || t > max_t {
        return None;
    }
    let point = ray.point_at(t);
    let radial = point - base - dir * (point - base).dot(&dir);
    let n = radial.normalize();
    Some((t, n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::RigidBody;
    use crate::collider::Collider;

    #[test]
    fn ray_hits_sphere() {
        let body = RigidBody::dynamic(1.0)
            .with_collider(Collider::sphere(1.0))
            .with_position(Vec3::new(0.0, 0.0, 0.0));
        let ray = Ray::new(Vec3::new(-10.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
        let (t, n) = cast_body(&ray, &body, 20.0).expect("hit");
        assert!((t - 9.0).abs() < 1e-3);
        assert!((n + Vec3::new(1.0, 0.0, 0.0)).norm() < 1e-3);
    }

    #[test]
    fn ray_misses_sphere() {
        let body = RigidBody::static_body()
            .with_collider(Collider::sphere(1.0));
        let ray = Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
        assert!(cast_body(&ray, &body, 100.0).is_none());
    }
}
