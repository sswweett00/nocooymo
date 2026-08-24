//! Tremor constraint solver.
//!
//! Implements a sequential-impulse contact solver with warm starting,
//! restitution and Coulomb friction, followed by linear positional correction
//! to remove residual penetration (sinking). Joint constraints reuse the same
//! impulse machinery via [`crate::joints`].

use crate::body::RigidBody;
use crate::contact::ContactManifold;
use crate::math::Vec3;

/// Tunables controlling solver fidelity and stability.
#[derive(Debug, Clone, Copy)]
pub struct SolverSettings {
    /// Number of velocity iterations per step.
    pub iterations: u32,
    /// Number of positional-correction passes.
    pub position_iterations: u32,
    /// Allowed penetration before positional correction kicks in.
    pub slop: f32,
    /// Fraction of penetration corrected per position pass (0..1).
    pub baumgarte: f32,
    /// Relative normal speed above which restitution is applied.
    pub restitution_threshold: f32,
}

impl Default for SolverSettings {
    fn default() -> Self {
        Self {
            iterations: 10,
            position_iterations: 3,
            slop: 0.005,
            baumgarte: 0.2,
            restitution_threshold: 1.0,
        }
    }
}

/// Simultaneously borrow two distinct bodies from a slice.
fn two_mut<T>(s: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
    assert!(a != b, "contact/joint between a body and itself");
    if a < b {
        let (l, r) = s.split_at_mut(b);
        (&mut l[a], &mut r[0])
    } else {
        let (l, r) = s.split_at_mut(a);
        (&mut r[0], &mut l[b])
    }
}

/// Apply a world-space impulse at an offset `r` about a body's origin.
fn apply_impulse(body: &mut RigidBody, p: Vec3, r: Vec3) {
    if body.is_static() {
        return;
    }
    body.lin_vel += p * body.inv_mass;
    body.ang_vel += body.world_inv_inertia() * r.cross(&p);
}

/// Right-handed tangent basis from a normal.
pub(crate) fn tangent_basis(n: Vec3) -> (Vec3, Vec3) {
    let x = if n.x.abs() < 0.9 { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, 1.0, 0.0) };
    let t1 = n.cross(&x);
    let t1 = if t1.norm_squared() > 1e-9 { t1.normalize() } else { Vec3::new(1.0, 0.0, 0.0) };
    let t2 = n.cross(&t1).normalize();
    (t1, t2)
}

/// Effective mass of a two-body constraint along `dir`.
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

/// The solver — stateless apart from its settings.
#[derive(Debug, Clone, Copy)]
pub struct TremorSolver {
    pub settings: SolverSettings,
}

impl Default for TremorSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl TremorSolver {
    pub fn new() -> Self {
        Self { settings: SolverSettings::default() }
    }

    pub fn with_settings(settings: SolverSettings) -> Self {
        Self { settings }
    }

    /// Solve velocity contacts, then correct residual penetration.
    pub fn solve_contacts(&self, bodies: &mut [RigidBody], manifolds: &mut [ContactManifold]) {
        self.warm_start(bodies, manifolds);
        for _ in 0..self.settings.iterations {
            self.velocity_iteration(bodies, manifolds);
        }
        for _ in 0..self.settings.position_iterations {
            self.position_correction(bodies, manifolds);
        }
    }

    fn warm_start(&self, bodies: &mut [RigidBody], manifolds: &mut [ContactManifold]) {
        for m in manifolds.iter() {
            if !m.active || m.points.is_empty() {
                continue;
            }
            let (ia, ib) = (m.a as usize, m.b as usize);
            let (ba, bb) = two_mut(bodies, ia, ib);
            for p in &m.points {
                if p.normal_impulse == 0.0 && p.tangent_impulse == [0.0, 0.0] {
                    continue;
                }
                let (ra, rb) = (p.world_a - ba.pos, p.world_b - bb.pos);
                let (t1, t2) = tangent_basis(p.normal);
                let total = p.normal * p.normal_impulse
                    + t1 * p.tangent_impulse[0]
                    + t2 * p.tangent_impulse[1];
                apply_impulse(ba, -total, ra);
                apply_impulse(bb, total, rb);
            }
        }
    }

    fn velocity_iteration(&self, bodies: &mut [RigidBody], manifolds: &mut [ContactManifold]) {
        for m in manifolds.iter_mut() {
            if !m.active || m.points.is_empty() {
                continue;
            }
            let (ia, ib) = (m.a as usize, m.b as usize);
            let (ba, bb) = two_mut(bodies, ia, ib);
            for p in m.points.iter_mut() {
                let (ra, rb) = (p.world_a - ba.pos, p.world_b - bb.pos);
                let n = p.normal;
                let km = effective_mass(ba, bb, ra, rb, n);
                if km < 1e-12 {
                    continue;
                }
                let normal_mass = 1.0 / km;

                let va = ba.lin_vel + ba.ang_vel.cross(&ra);
                let vb = bb.lin_vel + bb.ang_vel.cross(&rb);
                let rv = vb - va;
                let vn = rv.dot(&n);
                let bias = if vn < -self.settings.restitution_threshold {
                    -m.restitution * vn
                } else {
                    0.0
                };

                let mut d = -(vn - bias) * normal_mass;
                let old = p.normal_impulse;
                let new = (old + d).max(0.0);
                d = new - old;
                p.normal_impulse = new;
                let impulse = n * d;
                apply_impulse(ba, -impulse, ra);
                apply_impulse(bb, impulse, rb);

                // Friction (Coulomb): two tangent directions, capped by mu * Pn.
                let (t1, t2) = tangent_basis(n);
                for (t, slot) in [(t1, 0usize), (t2, 1usize)] {
                    let kt = effective_mass(ba, bb, ra, rb, t);
                    if kt < 1e-12 {
                        continue;
                    }
                    let va = ba.lin_vel + ba.ang_vel.cross(&ra);
                    let vb = bb.lin_vel + bb.ang_vel.cross(&rb);
                    let rv = vb - va;
                    let vt = rv.dot(&t);
                    let max_f = m.friction * p.normal_impulse;
                    let mut d = -vt / kt;
                    let old = p.tangent_impulse[slot];
                    let new = (old + d).clamp(-max_f, max_f);
                    d = new - old;
                    p.tangent_impulse[slot] = new;
                    let impulse = t * d;
                    apply_impulse(ba, -impulse, ra);
                    apply_impulse(bb, impulse, rb);
                }
            }
        }
    }

    fn position_correction(&self, bodies: &mut [RigidBody], manifolds: &mut [ContactManifold]) {
        for m in manifolds.iter() {
            if !m.active || m.points.is_empty() {
                continue;
            }
            let (ia, ib) = (m.a as usize, m.b as usize);
            let (ba, bb) = two_mut(bodies, ia, ib);
            let sep = m.points[0].separation;
            if sep >= -self.settings.slop {
                continue;
            }
            let n = m.points[0].normal;
            let correction = self.settings.baumgarte * (sep + self.settings.slop).abs() * n;
            let total_inv = ba.inv_mass + bb.inv_mass;
            if total_inv <= 1e-12 {
                continue;
            }
            if ba.inv_mass > 0.0 {
                ba.pos -= correction * (ba.inv_mass / total_inv);
            }
            if bb.inv_mass > 0.0 {
                bb.pos += correction * (bb.inv_mass / total_inv);
            }
        }
    }
}
