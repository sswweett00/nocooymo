//! Wheeled vehicle simulation (Mimari — ray-cast suspension model).
//!
//! Each wheel ray-casts to a ground height, derives a spring + damper
//! suspension force, and applies engine / braking traction and lateral
//! friction to the chassis body.

use crate::body::RigidBody;
use crate::math::Vec3;

/// Configuration of a single wheel.
#[derive(Debug, Clone, Copy)]
pub struct VehicleWheel {
    /// Local position relative to the chassis body origin.
    pub local_position: Vec3,
    pub radius: f32,
    pub suspension_rest: f32,
    pub suspension_stiffness: f32,
    pub damping_compression: f32,
    pub damping_relaxation: f32,
    /// Front wheels steer; driven wheels receive engine torque.
    pub is_front: bool,
    pub is_driven: bool,
}

impl VehicleWheel {
    pub fn new(local_position: Vec3, radius: f32) -> Self {
        Self {
            local_position,
            radius,
            suspension_rest: 0.3,
            suspension_stiffness: 30.0,
            damping_compression: 3.5,
            damping_relaxation: 2.5,
            is_front: true,
            is_driven: false,
        }
    }
}

/// A wheeled vehicle driving a single chassis body.
#[derive(Debug, Clone)]
pub struct Vehicle {
    pub body: u32,
    pub wheels: Vec<VehicleWheel>,
    pub engine_force: f32,
    pub brake_force: f32,
    pub steering_angle: f32,
    /// Lateral friction applied at each wheel when grounded.
    pub lateral_friction: f32,
}

impl Vehicle {
    pub fn new(body: u32) -> Self {
        Self {
            body,
            wheels: Vec::new(),
            engine_force: 0.0,
            brake_force: 0.0,
            steering_angle: 0.0,
            lateral_friction: 6.0,
        }
    }

    pub fn add_wheel(&mut self, wheel: VehicleWheel) -> &mut Self {
        self.wheels.push(wheel);
        self
    }

    /// Advance the vehicle by `dt`. `ground_height(x, z)` gives the terrain.
    pub fn simulate<F>(&mut self, bodies: &mut [RigidBody], dt: f32, ground_height: F)
    where
        F: Fn(f32, f32) -> f32,
    {
        if self.body as usize >= bodies.len() {
            return;
        }
        let forward = {
            let ch = &bodies[self.body as usize];
            (ch.rot * Vec3::new(0.0, 0.0, -1.0)).normalize() // -Z forward
        };
        let steering = self.steering_angle;
        let side = {
            let ch = &bodies[self.body as usize];
            (ch.rot * Vec3::new(1.0, 0.0, 0.0)).normalize() // +X right
        };

        // Capture wheel geometry before mutably borrowing the chassis.
        let wheel_data: Vec<(Vec3, VehicleWheel)> = {
            let ch = &bodies[self.body as usize];
            self.wheels
                .iter()
                .map(|w| (ch.pos + ch.rot * w.local_position, *w))
                .collect()
        };

        for (ws, w) in &wheel_data {
            let ground = ground_height(ws.x, ws.z);
            let len = ws.y - ground;
            let compression = w.suspension_rest - len;
            if compression <= 0.0 {
                continue;
            }
            let ch = &mut bodies[self.body as usize];
            if ch.is_static() {
                continue;
            }
            let vel = ch.velocity_at_point(*ws);
            let damping = if compression > 0.0 { w.damping_compression } else { w.damping_relaxation };
            let force_mag = w.suspension_stiffness * compression - damping * vel.y;
            ch.apply_force_at(Vec3::new(0.0, force_mag, 0.0), *ws);

            let wheel_forward = if w.is_front { rotate_y(forward, steering) } else { forward };
            let drive = if w.is_driven { self.engine_force } else { 0.0 };
            if drive != 0.0 {
                ch.apply_force_at(wheel_forward * drive, *ws);
            }
            if self.brake_force > 0.0 {
                ch.apply_force_at(-vel * self.brake_force * 0.01, *ws);
            }
            if self.lateral_friction > 0.0 {
                let side_vel = side * vel.dot(&side);
                ch.apply_force_at(-side_vel * self.lateral_friction, *ws);
            }
        }
        let _ = dt;
    }
}

fn rotate_y(v: Vec3, angle: f32) -> Vec3 {
    let (s, c) = angle.sin_cos();
    Vec3::new(v.x * c + v.z * s, v.y, -v.x * s + v.z * c)
}
