//! Narrow phase — primitive vs. primitive contact generation (Mimari §3.1 F12).
//!
//! Converts a broad-phase candidate pair into a [`ContactManifold`] of world
//! contact points. Supports sphere / box / capsule pairs analytically.

use crate::body::RigidBody;
use crate::collider::ColliderShape;
use crate::contact::{ContactManifold, ContactPoint};
use crate::math::{Quat, Vec3};

/// Generate a manifold for a candidate body pair, or `None` if they do not touch.
pub fn collide(a: &RigidBody, b: &RigidBody, ia: u32, ib: u32) -> Option<ContactManifold> {
    let ca = a.collider.as_ref()?;
    let cb = b.collider.as_ref()?;
    if !crate::collider::collides_group(ca, cb) {
        return None;
    }
    let a_center = ca.shape_world_center(a.pos, a.rot);
    let b_center = cb.shape_world_center(b.pos, b.rot);

    let points = match (ca.shape, cb.shape) {
        (ColliderShape::Sphere { radius: ra }, ColliderShape::Sphere { radius: rb }) => {
            sphere_sphere(a_center, ra, b_center, rb).map(|c| vec![orient(c, a_center, b_center)])
        }
        (ColliderShape::Sphere { radius: ra }, ColliderShape::Box { half_extents: he }) => {
            sphere_box_points(a_center, ra, b_center, b.rot, he)
                .map(|p| vec![orient(p, a_center, b_center)])
        }
        (ColliderShape::Box { half_extents: he }, ColliderShape::Sphere { radius: rb }) => {
            sphere_box_points(b_center, rb, a_center, a.rot, he)
                .map(|p| vec![orient_reverse(p, a_center, b_center)])
        }
        (ColliderShape::Sphere { radius: ra }, ColliderShape::Capsule { radius, height }) => {
            sphere_capsule_points(a_center, ra, b_center, b.rot, radius, height)
                .map(|p| vec![orient(p, a_center, b_center)])
        }
        (ColliderShape::Capsule { radius, height }, ColliderShape::Sphere { radius: rb }) => {
            sphere_capsule_points(b_center, rb, a_center, a.rot, radius, height)
                .map(|p| vec![orient_reverse(p, a_center, b_center)])
        }
        (ColliderShape::Box { half_extents: hea }, ColliderShape::Box { half_extents: heb }) => {
            box_box_points(a_center, a.rot, hea, b_center, b.rot, heb)
                .map(|p| vec![orient(p, a_center, b_center)])
        }
        (ColliderShape::Capsule { radius, height }, ColliderShape::Box { half_extents: he }) => {
            capsule_box_points(a_center, a.rot, radius, height, b_center, b.rot, he)
                .map(|p| vec![orient(p, a_center, b_center)])
        }
        (ColliderShape::Box { half_extents: he }, ColliderShape::Capsule { radius, height }) => {
            capsule_box_points(b_center, b.rot, radius, height, a_center, a.rot, he)
                .map(|p| vec![orient_reverse(p, a_center, b_center)])
        }
        (ColliderShape::Capsule { radius: ra, height: ha }, ColliderShape::Capsule { radius: rb, height: hb }) => {
            capsule_capsule_points(a_center, a.rot, ra, ha, b_center, b.rot, rb, hb)
                .map(|p| vec![orient(p, a_center, b_center)])
        }
    }?;

    let friction = crate::collider::Material::combine_friction(ca.material.friction, cb.material.friction);
    let restitution = crate::collider::Material::combine_restitution(ca.material.restitution, cb.material.restitution);
    let mut manifold = ContactManifold::new(ia, ib);
    manifold.friction = friction;
    manifold.restitution = restitution;
    manifold.points = points;
    manifold.active = true;
    Some(manifold)
}

/// A raw contact: point on A, point on B, normal A->B, and signed separation
/// (positive when apart, negative when penetrating).
struct Contact {
    pa: Vec3,
    pb: Vec3,
    normal: Vec3,
    separation: f32,
}

/// Ensure the normal points from body A toward body B (for oriented manifolds).
fn orient(p: Contact, center_a: Vec3, center_b: Vec3) -> ContactPoint {
    let mut n = p.normal;
    if n.dot(&(center_b - center_a)) < 0.0 {
        n = -n;
    }
    contact_point(p, n)
}

fn orient_reverse(p: Contact, center_a: Vec3, center_b: Vec3) -> ContactPoint {
    let mut n = -p.normal;
    if n.dot(&(center_b - center_a)) < 0.0 {
        n = -n;
    }
    contact_point(p, n)
}

fn contact_point(p: Contact, n: Vec3) -> ContactPoint {
    ContactPoint::new(p.pa, p.pb, n, p.separation)
}

// ---------------------------------------------------------------------------
// Primitive contact routines (return signed separation: negative = overlap)
// ---------------------------------------------------------------------------

fn sphere_sphere(ca: Vec3, ra: f32, cb: Vec3, rb: f32) -> Option<Contact> {
    let d = cb - ca;
    let dist = d.norm();
    let sum = ra + rb;
    if dist >= sum || dist < 1e-9 {
        return None;
    }
    let n = d / dist;
    Some(Contact {
        pa: ca + n * ra,
        pb: cb - n * rb,
        normal: n,
        separation: dist - sum,
    })
}

/// Sphere (A) vs box (B). Returns normal pointing sphere -> box.
fn sphere_box_points(sc: Vec3, r: f32, bc: Vec3, brot: Quat, he: Vec3) -> Option<Contact> {
    let inv = brot.inverse();
    let local = inv * (sc - bc);
    let closest = Vec3::new(
        local.x.clamp(-he.x, he.x),
        local.y.clamp(-he.y, he.y),
        local.z.clamp(-he.z, he.z),
    );
    let diff = local - closest;
    let dist2 = diff.norm_squared();

    if dist2 > 1e-9 {
        // Sphere center outside the box.
        let dist = dist2.sqrt();
        if dist >= r {
            return None;
        }
        // direction from box to sphere center (world).
        let dir_world = brot * (diff / dist);
        // normal sphere -> box.
        let n_ab = -dir_world;
        let pa = sc + n_ab * r;
        let pb = bc + brot * closest;
        Some(Contact { pa, pb, normal: n_ab, separation: dist - r })
    } else {
        // Sphere center inside the box: push out along the least-penetration axis.
        let mut axis = 0usize;
        let mut min_pen = f32::MAX;
        for i in 0..3 {
            let pen = he[i] + r - local[i].abs();
            if pen < min_pen {
                min_pen = pen;
                axis = i;
            }
        }
        let sign = if local[axis] >= 0.0 { 1.0 } else { -1.0 };
        let mut outward = Vec3::zeros();
        outward[axis] = sign;
        let outward_world = brot * outward;
        let n_ab = -outward_world;
        let mut on_box = closest;
        on_box[axis] = he[axis] * sign;
        let pa = sc + n_ab * r;
        let pb = bc + brot * on_box;
        Some(Contact { pa, pb, normal: n_ab, separation: -min_pen })
    }
}

/// Sphere (A) vs capsule (B). Capsule is a segment (axis = local Y) of half
/// length `height/2` with rounded radius `r_b`.
fn sphere_capsule_points(
    sc: Vec3,
    ra: f32,
    cc: Vec3,
    crot: Quat,
    rb: f32,
    height: f32,
) -> Option<Contact> {
    let half = height * 0.5;
    let seg_start = cc + crot * Vec3::new(0.0, -half, 0.0);
    let seg_end = cc + crot * Vec3::new(0.0, half, 0.0);
    let closest = closest_point_on_segment(sc, seg_start, seg_end);
    let d = sc - closest;
    let dist = d.norm();
    if dist >= ra + rb || dist < 1e-9 {
        return None;
    }
    let n = d / dist; // points from capsule surface to sphere: sphere <- segment
    let n_ab = -n;   // sphere -> capsule
    let pa = sc + n_ab * ra;
    let pb = closest + n * rb;
    Some(Contact { pa, pb, normal: n_ab, separation: dist - (ra + rb) })
}

fn closest_point_on_segment(p: Vec3, a: Vec3, b: Vec3) -> Vec3 {
    let ab = b - a;
    let t = ((p - a).dot(&ab) / ab.norm_squared()).clamp(0.0, 1.0);
    a + ab * t
}

/// Capsule (A) vs capsule (B).
#[allow(clippy::too_many_arguments)]
fn capsule_capsule_points(
    ac: Vec3, arot: Quat, ra: f32, ha: f32,
    bc: Vec3, brot: Quat, rb: f32, hb: f32,
) -> Option<Contact> {
    let a0 = ac + arot * Vec3::new(0.0, -ha * 0.5, 0.0);
    let a1 = ac + arot * Vec3::new(0.0, ha * 0.5, 0.0);
    let b0 = bc + brot * Vec3::new(0.0, -hb * 0.5, 0.0);
    let b1 = bc + brot * Vec3::new(0.0, hb * 0.5, 0.0);
    let (pa, pb) = closest_point_between_segments(a0, a1, b0, b1);
    let d = pb - pa;
    let dist = d.norm();
    if dist >= ra + rb || dist < 1e-9 {
        return None;
    }
    let n = d / dist; // from capsule A surface to capsule B surface
    let pa_w = pa + n * ra;
    let pb_w = pb - n * rb;
    Some(Contact { pa: pa_w, pb: pb_w, normal: n, separation: dist - (ra + rb) })
}

fn closest_point_between_segments(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> (Vec3, Vec3) {
    // Standard segment-segment closest point (Ericson, RTCD Ch.5).
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;
    let a = d1.dot(&d1);
    let e = d2.dot(&d2);
    let f = d2.dot(&r);
    let c = d1.dot(&r);
    let b = d1.dot(&d2);
    let denom = a * e - b * b;
    let mut s = 0.0;
    let mut t = 0.0;
    if denom > 1e-12 {
        s = (b * f - c * e) / denom;
        s = s.clamp(0.0, 1.0);
        t = (b * s + f) / e;
        t = t.clamp(0.0, 1.0);
        s = (b * t - c) / a;
        s = s.clamp(0.0, 1.0);
    }
    (p1 + d1 * s, p2 + d2 * t)
}

/// Capsule (A) as a rounded segment vs box (B), via two-sphere approximation.
fn capsule_box_points(
    ac: Vec3, arot: Quat, ra: f32, height: f32,
    bc: Vec3, brot: Quat, he: Vec3,
) -> Option<Contact> {
    let half = height * 0.5;
    let c0 = ac + arot * Vec3::new(0.0, -half, 0.0);
    let c1 = ac + arot * Vec3::new(0.0, half, 0.0);
    let mut best: Option<Contact> = None;
    let mut best_sep = f32::MAX;
    for c in [c0, c1] {
        if let Some(ct) = sphere_box_points(c, ra, bc, brot, he) {
            if ct.separation < best_sep {
                best_sep = ct.separation;
                best = Some(ct);
            }
        }
    }
    best
}

/// Box (A) vs box (B) using the Separating Axis Theorem. Returns a single
/// representative contact (normal A->B, negative separation when penetrating).
fn box_box_points(
    ac: Vec3, arot: Quat, hea: Vec3,
    bc: Vec3, brot: Quat, heb: Vec3,
) -> Option<Contact> {
    let ra = arot.to_rotation_matrix().into_inner();
    let rb = brot.to_rotation_matrix().into_inner();

    // Build the 15 candidate axes.
    let mut axes: Vec<Vec3> = Vec::with_capacity(15);
    for i in 0..3 { axes.push(ra.column(i).into_owned()); }
    for i in 0..3 { axes.push(rb.column(i).into_owned()); }
    for i in 0..3 {
        for j in 0..3 {
            let c = ra.column(i).cross(&rb.column(j));
            if c.norm_squared() > 1e-9 {
                axes.push(c.normalize());
            }
        }
    }

    let mut min_depth = f32::MAX;
    let mut mtv = Vec3::zeros();
    let cdiff = bc - ac;
    for axis in axes {
        let (mina, maxa) = box_projection(ac, &ra, hea, &axis);
        let (minb, maxb) = box_projection(bc, &rb, heb, &axis);
        // Overlap along this axis; zero/negative means the boxes are separated.
        let overlap = maxa.min(maxb) - mina.max(minb);
        if overlap <= 1e-9 {
            return None;
        }
        if overlap < min_depth {
            min_depth = overlap;
            mtv = axis;
        }
    }

    // Orient MTV to point from A to B.
    if mtv.dot(&cdiff) < 0.0 {
        mtv = -mtv;
    }

    // Stable support point: project A's center onto B's near face, so that
    // face-to-face rests (e.g. box stacks) do not tip around a corner contact.
    let n = mtv;
    let n_local_b = brot.inverse() * n;
    let extent_b = heb.x * n_local_b.x.abs()
        + heb.y * n_local_b.y.abs()
        + heb.z * n_local_b.z.abs();
    let face_b = bc - n * extent_b;
    let contact = ac - n * ((ac - face_b).dot(&n));
    Some(Contact {
        pa: contact,
        pb: contact,
        normal: n,
        separation: -min_depth,
    })
}

fn box_projection(c: Vec3, r: &nalgebra::Matrix3<f32>, he: Vec3, axis: &Vec3) -> (f32, f32) {
    let center = c.dot(axis);
    let mut radius = 0.0;
    for i in 0..3 {
        radius += he[i] * (r.column(i).dot(axis)).abs();
    }
    (center - radius, center + radius)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::RigidBody;
    use crate::collider::Collider;

    fn sphere_body(pos: Vec3, r: f32) -> RigidBody {
        RigidBody::dynamic(1.0).with_collider(Collider::sphere(r)).with_position(pos)
    }

    #[test]
    fn sphere_sphere_contact() {
        let a = sphere_body(Vec3::new(0.0, 0.0, 0.0), 1.0);
        let b = sphere_body(Vec3::new(1.5, 0.0, 0.0), 1.0);
        let m = collide(&a, &b, 0, 1).expect("touching");
        assert!(m.points[0].separation < 0.0);
        // Normal should point from A toward B (+X).
        assert!(m.points[0].normal.x > 0.9);
        // Too far -> no contact.
        let c = sphere_body(Vec3::new(10.0, 0.0, 0.0), 1.0);
        assert!(collide(&a, &c, 0, 2).is_none());
    }

    #[test]
    fn sphere_box_contact() {
        let s = sphere_body(Vec3::new(1.4, 0.0, 0.0), 1.0);
        let box_body = RigidBody::static_body().with_collider(Collider::box_collider(Vec3::new(1.0, 1.0, 1.0)));
        let m = collide(&s, &box_body, 0, 1).expect("touching");
        assert!(m.points[0].separation < 0.0);
        // Normal points from sphere (A) to box (B): -X.
        assert!(m.points[0].normal.x < -0.9);
    }
}
