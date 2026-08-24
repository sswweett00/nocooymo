//! Rigid bodies — the dynamic actors of the physics world.

use crate::collider::Collider;
use crate::math::{Mat3, Quat, Vec3};
use nalgebra::Matrix3;

/// Physical behaviour class of a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyType {
    /// Infinite mass; never moves but participates in collisions.
    Static,
    /// Finite mass; integrated and acted on by forces/impulses.
    Dynamic,
    /// Infinite mass; user-driven motion with full collision response.
    Kinematic,
}

/// A lightweight, generation-safe handle referencing a body in a [`crate::World`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyHandle(pub u32);

/// A rigid body owning its own collider, transform and velocity state.
#[derive(Debug, Clone)]
pub struct RigidBody {
    pub body_type: BodyType,
    pub mass: f32,
    pub inv_mass: f32,
    pub inertia_local: Mat3,
    pub inv_inertia_local: Mat3,
    pub pos: Vec3,
    pub rot: Quat,
    pub lin_vel: Vec3,
    pub ang_vel: Vec3,
    /// Accumulated linear force (cleared after integration).
    pub force: Vec3,
    /// Accumulated torque (cleared after integration).
    pub torque: Vec3,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub gravity_scale: f32,
    /// Kinematic bodies move by explicitly setting velocity each frame.
    pub allow_sleep: bool,
    pub sleeping: bool,
    sleep_time: f32,
    pub collider: Option<Collider>,
}

impl RigidBody {
    /// Create a dynamic body with the given mass (0 -> static).
    pub fn dynamic(mass: f32) -> Self {
        let inv = if mass > 0.0 && mass.is_finite() { 1.0 / mass } else { 0.0 };
        let body_type = if inv > 0.0 { BodyType::Dynamic } else { BodyType::Static };
        Self {
            body_type,
            mass: if inv > 0.0 { mass } else { 0.0 },
            inv_mass: inv,
            inertia_local: Matrix3::zeros(),
            inv_inertia_local: Matrix3::zeros(),
            pos: Vec3::zeros(),
            rot: Quat::identity(),
            lin_vel: Vec3::zeros(),
            ang_vel: Vec3::zeros(),
            force: Vec3::zeros(),
            torque: Vec3::zeros(),
            linear_damping: 0.0,
            angular_damping: 0.0,
            gravity_scale: 1.0,
            allow_sleep: true,
            sleeping: false,
            sleep_time: 0.0,
            collider: None,
        }
    }

    pub fn static_body() -> Self {
        let mut b = Self::dynamic(0.0);
        b.body_type = BodyType::Static;
        b
    }

    pub fn kinematic() -> Self {
        let mut b = Self::dynamic(0.0);
        b.body_type = BodyType::Kinematic;
        b.allow_sleep = false;
        b
    }

    /// Attach a collider and recompute mass properties from its density.
    pub fn with_collider(mut self, collider: Collider) -> Self {
        self.attach_collider(collider);
        self
    }

    pub fn attach_collider(&mut self, collider: Collider) {
        self.collider = Some(collider);
        self.recompute_mass_properties();
    }

    /// Recompute mass and inertia from the collider density/shape, unless the
    /// body is already static or kinematic with an explicit mass set by the user.
    pub fn recompute_mass_properties(&mut self) {
        if self.body_type != BodyType::Dynamic {
            emit_static_inertia(self);
            return;
        }
        if let Some(c) = &self.collider {
            let density = c.material.density;
            let vol = c.shape.volume();
            let mass = vol * density;
            if mass > 0.0 && mass.is_finite() {
                self.mass = mass;
                self.inv_mass = 1.0 / mass;
                self.inertia_local = c.shape.unit_inertia() * density;
                self.inv_inertia_local = self.inertia_local.try_inverse().unwrap_or_else(|| {
                    // Degenerate shape (zero volume) -> treat angularly as static.
                    Matrix3::zeros()
                });
                return;
            }
        }
        emit_static_inertia(self);
    }

    pub fn is_static(&self) -> bool {
        self.body_type != BodyType::Dynamic
    }

    /// World-space inertia of a body; used for torque integration.
    pub fn world_inertia(&self) -> Mat3 {
        self.rot.to_rotation_matrix().into_inner() * self.inertia_local
            * self.rot.to_rotation_matrix().into_inner().transpose()
    }

    pub fn world_inv_inertia(&self) -> Mat3 {
        self.rot.to_rotation_matrix().into_inner() * self.inv_inertia_local
            * self.rot.to_rotation_matrix().into_inner().transpose()
    }

    /// Apply an instantaneous impulse at a world-space point.
    pub fn apply_impulse_at(&mut self, impulse: Vec3, point: Vec3) {
        if self.is_static() {
            return;
        }
        self.lin_vel += impulse * self.inv_mass;
        let r = point - self.pos;
        self.ang_vel += self.world_inv_inertia() * r.cross(&impulse);
    }

    pub fn apply_force(&mut self, force: Vec3) {
        if !self.is_static() {
            self.force += force;
        }
    }

    pub fn apply_force_at(&mut self, force: Vec3, point: Vec3) {
        if self.is_static() {
            return;
        }
        self.force += force;
        self.torque += (point - self.pos).cross(&force);
    }

    pub fn apply_torque(&mut self, torque: Vec3) {
        if !self.is_static() {
            self.torque += torque;
        }
    }

    pub fn set_linear_velocity(&mut self, v: Vec3) -> &mut Self {
        self.lin_vel = v;
        self.sleep_time = 0.0;
        self.sleeping = false;
        self
    }

    pub fn with_position(mut self, pos: Vec3) -> Self {
        self.pos = pos;
        self
    }
    pub fn with_rotation(mut self, rot: Quat) -> Self {
        self.rot = rot;
        self
    }
    pub fn with_velocity(mut self, v: Vec3) -> Self {
        self.set_linear_velocity(v);
        self
    }
    pub fn with_angular_velocity(mut self, w: Vec3) -> Self {
        self.ang_vel = w;
        self
    }
    pub fn set_position(&mut self, pos: Vec3) -> &mut Self {
        self.pos = pos;
        self
    }
    pub fn set_rotation(&mut self, rot: Quat) -> &mut Self {
        self.rot = rot;
        self
    }

    /// Semi-implicit (symplectic) Euler integration of linear/angular state.
    pub fn integrate(&mut self, dt: f32) {
        if self.body_type != BodyType::Dynamic || self.sleeping {
            return;
        }
        self.lin_vel += self.force * self.inv_mass * dt;
        if self.linear_damping > 0.0 {
            self.lin_vel *= self.linear_damping.powf(dt.max(0.0));
        }

        let inv_i_world = self.world_inv_inertia();
        let ang_accel = inv_i_world * self.torque;
        self.ang_vel += ang_accel * dt;
        if self.angular_damping > 0.0 {
            self.ang_vel *= self.angular_damping.powf(dt.max(0.0));
        }

        self.pos += self.lin_vel * dt;
        // Integrate angular velocity into the quaternion.
        if self.ang_vel.norm_squared() > 1e-14 {
            let dq = Quat::from_scaled_axis(self.ang_vel * dt);
            self.rot *= dq;
        }

        self.force = Vec3::zeros();
        self.torque = Vec3::zeros();
    }

    /// Velocity of a point on the body in world space.
    pub fn velocity_at_point(&self, point: Vec3) -> Vec3 {
        self.lin_vel + self.ang_vel.cross(&(point - self.pos))
    }

    /// Compute the world-space AABB for broad phase.
    pub fn aabb(&self) -> crate::math::Aabb {
        match &self.collider {
            Some(c) => c.shape.aabb(c.shape_world_center(self.pos, self.rot), self.rot),
            None => crate::math::Aabb::new(self.pos, self.pos),
        }
    }

    /// Advance the sleep timer; sets `sleeping` once below threshold.
    pub fn update_sleep(&mut self, dt: f32, threshold: f32) {
        if self.body_type != BodyType::Dynamic || !self.allow_sleep {
            return;
        }
        if self.lin_vel.norm() < threshold && self.ang_vel.norm() < threshold {
            self.sleep_time += dt;
            if self.sleep_time > 0.5 {
                self.sleeping = true;
            }
        } else {
            self.sleep_time = 0.0;
            self.sleeping = false;
        }
    }
}

fn emit_static_inertia(b: &mut RigidBody) {
    b.mass = 0.0;
    b.inv_mass = 0.0;
    b.inertia_local = Matrix3::zeros();
    b.inv_inertia_local = Matrix3::zeros();
}

/// Convenience: build a static ground-plane body with a friction box collider.
pub fn static_plane(y: f32) -> RigidBody {
    RigidBody::static_body().with_collider(Collider::box_collider(Vec3::new(1000.0, 0.5, 1000.0))
        .with_offset(Vec3::new(0.0, y - 0.5, 0.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_integrates_velocity() {
        let mut b = RigidBody::dynamic(1.0);
        b.force = Vec3::new(0.0, -9.81, 0.0);
        b.integrate(1.0);
        assert!((b.lin_vel.y - (-9.81)).abs() < 1e-4);
        // Semi-implicit Euler: position uses the *updated* velocity.
        assert!((b.pos.y - (-9.81)).abs() < 1e-3);
    }

    #[test]
    fn static_body_ignores_forces() {
        let mut b = RigidBody::static_body();
        b.apply_force(Vec3::new(100.0, 0.0, 0.0));
        b.integrate(0.5);
        assert_eq!(b.lin_vel, Vec3::zeros());
        assert_eq!(b.pos, Vec3::zeros());
    }

    #[test]
    fn mass_from_density() {
        // A unit cube of density 2 -> mass 16.
        let b = RigidBody::dynamic(1.0).with_collider(
            Collider::box_collider(Vec3::new(1.0, 1.0, 1.0))
                .with_material(crate::collider::Material::new(0.5, 0.2, 2.0)),
        );
        assert!((b.mass - 16.0).abs() < 1e-3);
    }
}
