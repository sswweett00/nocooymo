//! Joint constraints (Mimari §3.1 — Tremor).
//!
//! Joints couple two bodies with velocity-level impulses. Supported types:
//! distance (springy rod) and point-to-point / hinge anchors.

use crate::body::{BodyHandle, RigidBody};
use crate::math::Vec3;

/// A constraint linking two bodies.
#[derive(Debug, Clone)]
pub enum Joint {
    /// Keeps two world-space points at a target distance (like a springy rod).
    Distance {
        a: BodyHandle,
        b: BodyHandle,
        local_anchor_a: Vec3,
        local_anchor_b: Vec3,
        /// `None` snaps to the initial distance on first solve.
        rest_length: Option<f32>,
        /// Constraint stiffness (0..1-ish). Higher = stiffer.
        stiffness: f32,
        /// Velocity damping factor applied along the rod.
        damping: f32,
    },
    /// Holds an anchor point on each body together (ball-and-socket / hinge base).
    PointToPoint {
        a: BodyHandle,
        b: BodyHandle,
        local_anchor_a: Vec3,
        local_anchor_b: Vec3,
        stiffness: f32,
    },
}

impl Joint {
    pub fn a(&self) -> BodyHandle {
        match self {
            Joint::Distance { a, .. } | Joint::PointToPoint { a, .. } => *a,
        }
    }

    pub fn b(&self) -> BodyHandle {
        match self {
            Joint::Distance { b, .. } | Joint::PointToPoint { b, .. } => *b,
        }
    }

    pub fn distance(a: BodyHandle, b: BodyHandle) -> Self {
        Joint::Distance {
            a,
            b,
            local_anchor_a: Vec3::zeros(),
            local_anchor_b: Vec3::zeros(),
            rest_length: None,
            stiffness: 0.3,
            damping: 0.3,
        }
    }

    pub fn pin(a: BodyHandle, b: BodyHandle) -> Self {
        Joint::PointToPoint {
            a,
            b,
            local_anchor_a: Vec3::zeros(),
            local_anchor_b: Vec3::zeros(),
            stiffness: 1.0,
        }
    }

    /// Solve this joint for one solver iteration.
    pub fn solve(&mut self, bodies: &mut [RigidBody], dt: f32) {
        match self {
            Joint::Distance {
                a,
                b,
                local_anchor_a,
                local_anchor_b,
                rest_length,
                stiffness,
                damping,
            } => solve_distance(
                bodies, a.0, b.0, *local_anchor_a, *local_anchor_b,
                rest_length, *stiffness, *damping, dt,
            ),
            Joint::PointToPoint {
                a,
                b,
                local_anchor_a,
                local_anchor_b,
                stiffness,
            } => solve_point_to_point(
                bodies, a.0, b.0, *local_anchor_a, *local_anchor_b, *stiffness, dt,
            ),
        }
    }
}

fn two_mut<T>(s: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
    if a < b {
        let (l, r) = s.split_at_mut(b);
        (&mut l[a], &mut r[0])
    } else {
        let (l, r) = s.split_at_mut(a);
        (&mut r[0], &mut l[b])
    }
}

fn effective_mass(a: &RigidBody, b: &RigidBody, ra: Vec3, rb: Vec3, dir: Vec3) -> f32 {
    let mut k = a.inv_mass + b.inv_mass;
    if !a.is_static() {
        let t = a.world_inv_inertia() * ra.cross(&dir);
        k += t.cross(&ra).dot(&dir);
    }
    if !b.is_static() {
        let t = b.world_inv_inertia() * rb.cross(&dir);
        k += t.cross(&rb).dot(&dir);
    }
    k
}

fn apply_impulse(body: &mut RigidBody, p: Vec3, r: Vec3) {
    if body.is_static() {
        return;
    }
    body.lin_vel += p * body.inv_mass;
    body.ang_vel += body.world_inv_inertia() * r.cross(&p);
}

#[allow(clippy::too_many_arguments)]
fn solve_distance(
    bodies: &mut [RigidBody],
    a: u32, b: u32,
    local_a: Vec3, local_b: Vec3,
    rest_length: &mut Option<f32>,
    stiffness: f32, damping: f32, dt: f32,
) {
    let (ba, bb) = two_mut(bodies, a as usize, b as usize);
    if ba.is_static() && bb.is_static() {
        return;
    }
    let pa = ba.pos + ba.rot * local_a;
    let pb = bb.pos + bb.rot * local_b;
    let d = pb - pa;
    let dist = d.norm();
    if dist < 1e-9 {
        return;
    }
    let n = d / dist;
    if rest_length.is_none() {
        *rest_length = Some(dist);
    }
    let rest = rest_length.unwrap();
    let (ra, rb) = (pa - ba.pos, pb - bb.pos);

    let km = effective_mass(ba, bb, ra, rb, n);
    if km < 1e-12 {
        return;
    }
    let va = ba.lin_vel + ba.ang_vel.cross(&ra);
    let vb = bb.lin_vel + bb.ang_vel.cross(&rb);
    let rv = vb - va;
    let vn = rv.dot(&n);

    // Positional error -> bias velocity (stiffness-scaled, XPBD-style compliance).
    let c = dist - rest;
    let bias = -(stiffness * c / dt.max(1e-6)).clamp(-2.0, 2.0);
    let mut d = -(vn - bias) * (1.0 / km);
    // Damp the correction and cap per-solve impulse to avoid explosions.
    d *= 1.0 - damping;
    d = d.clamp(-20.0, 20.0);
    let impulse = n * d;
    apply_impulse(ba, -impulse, ra);
    apply_impulse(bb, impulse, rb);
    let _ = dt;
}

fn solve_point_to_point(
    bodies: &mut [RigidBody],
    a: u32, b: u32,
    local_a: Vec3, local_b: Vec3,
    stiffness: f32, dt: f32,
) {
    let (ba, bb) = two_mut(bodies, a as usize, b as usize);
    if ba.is_static() && bb.is_static() {
        return;
    }
    let pa = ba.pos + ba.rot * local_a;
    let pb = bb.pos + bb.rot * local_b;
    let d = pb - pa;
    let dist = d.norm();
    let n = if dist < 1e-9 { Vec3::new(1.0, 0.0, 0.0) } else { d / dist };
    let (ra, rb) = (pa - ba.pos, pb - bb.pos);

    let km = effective_mass(ba, bb, ra, rb, n);
    if km < 1e-12 {
        return;
    }
    let va = ba.lin_vel + ba.ang_vel.cross(&ra);
    let vb = bb.lin_vel + bb.ang_vel.cross(&rb);
    let vn = (vb - va).dot(&n);
    let velocity_bias = -stiffness * dist / dt.max(1e-6);
    let mut d = -(vn - velocity_bias) / km;
    // Impulse clamp — patlamayı engelle
    d = d.clamp(-5.0 / dt.max(1e-6), 5.0 / dt.max(1e-6));
    let impulse = n * d;
    apply_impulse(ba, -impulse, ra);
    apply_impulse(bb, impulse, rb);
}
