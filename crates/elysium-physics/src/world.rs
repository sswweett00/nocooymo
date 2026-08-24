//! The [`PhysicsWorld`] — top-level orchestrator of the Tremor engine.
//!
//! Owns all bodies, joints, fluids and vehicles, runs the broad/narrow phase,
//! calls the solver, and exposes queries (raycast, sphere overlap) plus a
//! contact event stream for gameplay logic.

use std::collections::HashSet;

use crate::body::{BodyHandle, RigidBody};
use crate::broadphase::UniformGrid;
use crate::contact::ContactManifold;
use crate::fluid::SPHFluid;
use crate::joints::Joint;
use crate::math::{Aabb, Vec3};
use crate::narrowphase;
use crate::raycast::{Ray, RayHit};
use crate::solver::TremorSolver;
use crate::vehicle::Vehicle;

/// High-level runtime configuration for the world.
#[derive(Debug, Clone)]
pub struct PhysicsConfig {
    pub gravity: Vec3,
    pub substeps: u32,
    pub sleep_threshold: f32,
    pub fluid_bounds: Aabb,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravity: Vec3::new(0.0, -9.81, 0.0),
            substeps: 4,
            sleep_threshold: 0.05,
            fluid_bounds: Aabb::new(Vec3::new(-25.0, -25.0, -25.0), Vec3::new(25.0, 25.0, 25.0)),
        }
    }
}

/// A gameplay-observable contact lifecycle event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactEvent {
    Begin { a: u32, b: u32 },
    End { a: u32, b: u32 },
}

/// The main physics simulation world.
#[derive(Debug, Clone)]
pub struct PhysicsWorld {
    pub bodies: Vec<RigidBody>,
    pub joints: Vec<Joint>,
    pub fluids: Vec<SPHFluid>,
    pub vehicles: Vec<Vehicle>,
    pub gravity: Vec3,
    pub substeps: u32,
    pub sleep_threshold: f32,
    pub fluid_bounds: Aabb,
    pub solver: TremorSolver,
    broad: UniformGrid,
    prev_pairs: HashSet<(u32, u32)>,
    pub events: Vec<ContactEvent>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new(PhysicsConfig::default())
    }
}

impl PhysicsWorld {
    pub fn new(config: PhysicsConfig) -> Self {
        Self {
            bodies: Vec::new(),
            joints: Vec::new(),
            fluids: Vec::new(),
            vehicles: Vec::new(),
            gravity: config.gravity,
            substeps: config.substeps.max(1),
            sleep_threshold: config.sleep_threshold,
            fluid_bounds: config.fluid_bounds,
            solver: TremorSolver::default(),
            broad: UniformGrid::default(),
            prev_pairs: HashSet::new(),
            events: Vec::new(),
        }
    }

    pub fn body(&self, h: BodyHandle) -> Option<&RigidBody> {
        self.bodies.get(h.0 as usize)
    }
    pub fn body_mut(&mut self, h: BodyHandle) -> Option<&mut RigidBody> {
        self.bodies.get_mut(h.0 as usize)
    }

    /// Add a body; returns its stable handle (contiguous slot index).
    pub fn add_body(&mut self, body: RigidBody) -> BodyHandle {
        self.bodies.push(body);
        BodyHandle((self.bodies.len() - 1) as u32)
    }

    pub fn add_joint(&mut self, joint: Joint) {
        self.joints.push(joint);
    }
    pub fn add_fluid(&mut self, fluid: SPHFluid) -> &mut SPHFluid {
        self.fluids.push(fluid);
        self.fluids.last_mut().unwrap()
    }
    pub fn add_vehicle(&mut self, vehicle: Vehicle) -> &mut Vehicle {
        self.vehicles.push(vehicle);
        self.vehicles.last_mut().unwrap()
    }

    /// Advance the simulation by `dt` seconds.
    pub fn step(&mut self, dt: f32) {
        self.events.clear();
        let sub_dt = dt / self.substeps as f32;
        let mut manifolds: Vec<ContactManifold> = Vec::new();

        for _ in 0..self.substeps {
            // 1) Forces + integration.
            for b in self.bodies.iter_mut() {
                if b.body_type == crate::body::BodyType::Dynamic && !b.sleeping {
                    b.force += self.gravity * b.mass * b.gravity_scale;
                    b.integrate(sub_dt);
                }
            }

            // 2) Broad phase.
            let aabbs: Vec<Aabb> = self.bodies.iter().map(RigidBody::aabb).collect();
            self.broad.rebuild(&aabbs);
            let pairs = self.broad.pairs(&aabbs);

            // 3) Narrow phase.
            manifolds.clear();
            for (a, b) in pairs {
                let (ia, ib) = (a as usize, b as usize);
                if let Some(m) = narrowphase::collide(&self.bodies[ia], &self.bodies[ib], a, b) {
                    manifolds.push(m);
                }
            }

            // 4) Solve contacts, then joints.
            self.solver.solve_contacts(&mut self.bodies, &mut manifolds);
            for _ in 0..4 {
                for j in self.joints.iter_mut() {
                    j.solve(&mut self.bodies, sub_dt);
                }
            }

            for b in self.bodies.iter_mut() {
                b.update_sleep(sub_dt, self.sleep_threshold);
            }
        }

        // 5) Continuous subsystems run once per frame.
        for f in self.fluids.iter_mut() {
            f.update(dt, &self.fluid_bounds);
        }
        for v in self.vehicles.iter_mut() {
            let bounds = self.fluid_bounds;
            v.simulate(&mut self.bodies, dt, |_x, _z| bounds.min.y);
        }

        // 6) Contact lifecycle events.
        let mut active: HashSet<(u32, u32)> = HashSet::new();
        for m in &manifolds {
            if !m.points.is_empty() {
                active.insert((m.a, m.b));
            }
        }
        for p in active.difference(&self.prev_pairs) {
            self.events.push(ContactEvent::Begin { a: p.0, b: p.1 });
        }
        for p in self.prev_pairs.difference(&active) {
            self.events.push(ContactEvent::End { a: p.0, b: p.1 });
        }
        self.prev_pairs = active;
    }

    /// Cast a ray and return the nearest hit.
    pub fn raycast(&self, ray: &Ray, max_t: f32) -> Option<RayHit> {
        crate::raycast::cast_world(&self.bodies, ray, max_t)
    }

    /// Return bodies whose collider AABB overlaps the query sphere.
    pub fn query_sphere(&self, center: Vec3, radius: f32) -> Vec<BodyHandle> {
        let mut out = Vec::new();
        let q = Aabb::new(center - Vec3::repeat(radius), center + Vec3::repeat(radius));
        for (i, b) in self.bodies.iter().enumerate() {
            if b.aabb().intersects(&q) {
                out.push(BodyHandle(i as u32));
            }
        }
        out
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::body::RigidBody;
    use crate::collider::Collider;

    fn ground(world: &mut PhysicsWorld) {
        world.add_body(
            RigidBody::static_body().with_collider(
                Collider::box_collider(Vec3::new(20.0, 0.5, 20.0))
                    .with_offset(Vec3::new(0.0, -0.5, 0.0)),
            ),
        );
    }

    #[test]
    fn ball_drops_and_rests_on_ground() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        ground(&mut world);
        let ball = world.add_body(
            RigidBody::dynamic(1.0)
                .with_collider(Collider::sphere(0.5))
                .with_position(Vec3::new(0.0, 5.0, 0.0)),
        );
        for _ in 0..240 {
            world.step(1.0 / 60.0);
        }
        let b = world.body(ball).unwrap();
        assert!((b.pos.y - 0.5).abs() < 0.05, "ball resting y={}", b.pos.y);
        assert!(b.lin_vel.norm() < 0.2, "ball velocity={}", b.lin_vel);
    }

    #[test]
    fn box_stack_stays_stable() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        ground(&mut world);
        let mut handles = Vec::new();
        for i in 0..4 {
            handles.push(world.add_body(
                RigidBody::dynamic(1.0)
                    .with_collider(Collider::box_collider(Vec3::new(0.5, 0.5, 0.5)))
                    .with_position(Vec3::new(0.0, 0.5 + i as f32 * 1.0, 0.0)),
            ));
        }
        for _ in 0..360 {
            world.step(1.0 / 60.0);
        }
        let top = world.body(handles[3]).unwrap();
        assert!(top.pos.y > 3.0, "stack collapsed, top y={}", top.pos.y);
        assert!(top.pos.x.abs() < 0.5, "stack drifted, top x={}", top.pos.x);
    }

    #[test]
    fn raycast_finds_stack() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        ground(&mut world);
        world.add_body(
            RigidBody::static_body().with_collider(Collider::sphere(1.0))
                .with_position(Vec3::new(0.0, 3.0, 0.0)),
        );
        let ray = Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let hit = world.raycast(&ray, 20.0).unwrap();
        // The sphere's top is at y = 4.0.
        assert!((hit.point.y - 4.0).abs() < 0.2, "hit point y={}", hit.point.y);
    }

    #[test]
    fn joint_keeps_bodies_connected() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        ground(&mut world);
        let a = world.add_body(
            RigidBody::static_body()
                .with_collider(Collider::sphere(0.3))
                .with_position(Vec3::new(0.0, 1.6, 0.0)),
        );
        let b = world.add_body(
            RigidBody::dynamic(1.0)
                .with_collider(Collider::sphere(0.3))
                .with_position(Vec3::new(0.6, 1.6, 0.0)),
        );
        world.add_joint(Joint::distance(a, b));
        for _ in 0..240 {
            world.step(1.0 / 60.0);
        }
        let ba = world.body(a).unwrap();
        let bb = world.body(b).unwrap();
        let dist = (bb.pos - ba.pos).norm();
        // The rod length must be conserved (snapped to the initial 0.6).
        assert!((dist - 0.6).abs() < 0.03, "joint lost, dist={}", dist);
        // ...and it must hang below the anchor (not float).
        assert!(bb.pos.y < ba.pos.y, "joint not hanging, b.y={}", bb.pos.y);
    }
}
