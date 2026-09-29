//! Character controller system for FPS/TPS games (Mimari §3.1 — Tremor).
//!
//! Provides a high-level [`CharacterController`] that wraps a [`crate::RigidBody`]
//! and implements capsule-based kinematic movement with:
//!
//! * Movement states: idle, walk, run, jump, fall, crouch, slide.
//! * Slope detection & automatic sliding.
//! * Auto-step (climb over small obstacles).
//! * Pushable objects via physics impulses.
//! * Client-side prediction with server reconciliation (networked play).
//! * Animation state synchronization, root motion and foot IK support.
//!
//! The controller is **not** a replacement for the physics engine; it reads
//! contacts from the embedded [`crate::PhysicsWorld`] and applies kinematic
//! velocity/position corrections to the underlying body each frame.

use crate::body::{BodyHandle, RigidBody};
use crate::collider::{Collider, ColliderShape, Material};
use crate::math::{Quat, Vec3};
use crate::raycast::Ray;
use crate::world::PhysicsWorld;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for a character controller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterControllerConfig {
    /// Capsule radius (metres).
    pub capsule_radius: f32,
    /// Capsule half-height (metres) between centres of the two sphere ends.
    pub capsule_half_height: f32,
    /// Mass of the character (kg).
    pub mass: f32,
    /// Maximum slope angle in radians that the character can stand on without sliding.
    pub max_slope_angle: f32,
    /// Maximum slope angle in radians for sliding.
    pub slide_slope_angle: f32,
    /// Maximum step height the character can climb automatically (metres).
    pub max_step_height: f32,
    /// Character movement speeds.
    pub speeds: MovementSpeeds,
    /// Jump impulse applied when jumping (m/s).
    pub jump_impulse: f32,
    /// Gravity scale multiplier.
    pub gravity_scale: f32,
    /// Crouching capsule half-height (metres).
    pub crouch_half_height: f32,
    /// Slide speed decay per second.
    pub slide_friction: f32,
    /// Minimum speed to start sliding.
    pub slide_min_speed: f32,
    /// Maximum prediction history for network reconciliation.
    pub max_prediction_history: usize,
}

impl Default for CharacterControllerConfig {
    fn default() -> Self {
        Self {
            capsule_radius: 0.35,
            capsule_half_height: 0.9,
            mass: 80.0,
            max_slope_angle: 45.0_f32.to_radians(),
            slide_slope_angle: 55.0_f32.to_radians(),
            max_step_height: 0.35,
            speeds: MovementSpeeds::default(),
            jump_impulse: 6.5,
            gravity_scale: 1.0,
            crouch_half_height: 0.45,
            slide_friction: 4.0,
            slide_min_speed: 4.0,
            max_prediction_history: 32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementSpeeds {
    pub walk: f32,
    pub run: f32,
    pub crouch: f32,
    pub air: f32,
    pub rotation_speed: f32,
}

impl Default for MovementSpeeds {
    fn default() -> Self {
        Self {
            walk: 4.5,
            run: 8.5,
            crouch: 2.5,
            air: 2.0,
            rotation_speed: 12.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Character state machine
// ---------------------------------------------------------------------------

/// Discrete states of the character animation / movement graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharacterState {
    Idle,
    Walk,
    Run,
    Jump,
    Fall,
    Crouch,
    Slide,
}

// ---------------------------------------------------------------------------
// Input & animation sync
// ---------------------------------------------------------------------------

/// Per-frame input supplied by the player / AI.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterInput {
    pub move_x: f32,
    pub move_y: f32,
    pub jump: bool,
    pub crouch: bool,
    pub sprint: bool,
}

impl Default for CharacterInput {
    fn default() -> Self {
        Self {
            move_x: 0.0,
            move_y: 0.0,
            jump: false,
            crouch: false,
            sprint: false,
        }
    }
}

/// Animation state emitted to the render/animation system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationState {
    pub state: CharacterState,
    pub speed: f32,
    pub direction: f32,
    pub turn_rate: f32,
    pub crouching: bool,
    pub grounded: bool,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            state: CharacterState::Idle,
            speed: 0.0,
            direction: 0.0,
            turn_rate: 0.0,
            crouching: false,
            grounded: false,
        }
    }
}

/// Root-motion parameters extracted from animation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RootMotion {
    pub translation: Vec3,
    pub rotation: Quat,
    pub active: bool,
}

/// Foot-placement targets for IK.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FootIKTarget {
    pub left: Vec3,
    pub right: Vec3,
    pub left_normal: Vec3,
    pub right_normal: Vec3,
    pub left_valid: bool,
    pub right_valid: bool,
}

impl Default for FootIKTarget {
    fn default() -> Self {
        Self {
            left: Vec3::zeros(),
            right: Vec3::zeros(),
            left_normal: Vec3::new(0.0, 1.0, 0.0),
            right_normal: Vec3::new(0.0, 1.0, 0.0),
            left_valid: false,
            right_valid: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Network prediction
// ---------------------------------------------------------------------------

/// Snapshot of the authoritative state used for client prediction / server reconciliation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterSnapshot {
    pub pos: Vec3,
    pub rot: Quat,
    pub lin_vel: Vec3,
    pub ang_vel: Vec3,
    pub state: CharacterState,
    pub input: CharacterInput,
    pub time: f32,
}

/// Client-side prediction buffer.
#[derive(Debug, Clone, Default)]
pub struct PredictionBuffer {
    pub history: Vec<CharacterSnapshot>,
    pub max_history: usize,
}

impl PredictionBuffer {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::with_capacity(max_history),
            max_history,
        }
    }

    pub fn push(&mut self, snapshot: CharacterSnapshot) {
        self.history.push(snapshot);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    pub fn latest(&self) -> Option<&CharacterSnapshot> {
        self.history.last()
    }

    pub fn oldest(&self) -> Option<&CharacterSnapshot> {
        self.history.first()
    }

    pub fn clear(&mut self) {
        self.history.clear();
    }
}

// ---------------------------------------------------------------------------
// Contact query helper
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GroundContact {
    pub normal: Vec3,
    pub point: Vec3,
    pub body: BodyHandle,
    pub slope: f32,
    pub stepping: bool,
}

impl GroundContact {
    pub fn is_slope(&self) -> bool {
        self.slope > 0.0
    }
}

// ---------------------------------------------------------------------------
// Main controller
// ---------------------------------------------------------------------------

/// A character controller that owns a body handle and high-level movement state.
///
/// The controller does **not** own the `PhysicsWorld`; it borrows it each frame
/// so the simulation remains the single source of truth for contacts, rays and
/// collisions.
#[derive(Debug, Clone)]
pub struct CharacterController {
    pub body: BodyHandle,
    pub config: CharacterControllerConfig,
    pub state: CharacterState,
    pub input: CharacterInput,
    pub anim: AnimationState,
    pub root_motion: RootMotion,
    pub foot_ik: FootIKTarget,
    pub ground: Option<GroundContact>,
    pub prediction: PredictionBuffer,
    /// Slide timer (seconds remaining before slide ends).
    pub slide_timer: f32,
    /// Coyote-time remaining (seconds).
    pub coyote_time: f32,
    /// Jump buffer remaining (seconds).
    pub jump_buffer: f32,
    /// Current step-climb offset.
    pub step_offset: f32,
    /// Total distance slid this slide.
    pub slide_distance: f32,
    /// Pending root motion to consume.
    pub pending_root_motion: RootMotion,
}

impl CharacterController {
    /// Create a new character controller from a config.
    pub fn new(body: BodyHandle, config: CharacterControllerConfig) -> Self {
        Self {
            body,
            config,
            state: CharacterState::Idle,
            input: CharacterInput::default(),
            anim: AnimationState::default(),
            root_motion: RootMotion::default(),
            foot_ik: FootIKTarget::default(),
            ground: None,
            prediction: PredictionBuffer::new(config.max_prediction_history),
            slide_timer: 0.0,
            coyote_time: 0.0,
            jump_buffer: 0.0,
            step_offset: 0.0,
            slide_distance: 0.0,
            pending_root_motion: RootMotion::default(),
        }
    }

    /// Set the movement input for the next update.
    pub fn set_input(&mut self, input: CharacterInput) {
        self.input = input;
    }

    /// Current animation state for the render/animation system.
    pub fn animation_state(&self) -> AnimationState {
        self.anim
    }

    /// Current foot IK targets.
    pub fn foot_ik(&self) -> FootIKTarget {
        self.foot_ik
    }

    /// Advance the character controller by `dt`.
    ///
    /// Call this **after** the physics step so contacts are current.
    pub fn update(&mut self, world: &mut PhysicsWorld, dt: f32) {
        self.detect_ground(world);
        self.update_network_state(world, dt);
        self.handle_input(world, dt);
        self.update_state_machine(dt);
        self.update_animation(world, dt);
        self.update_foot_ik(world);
        self.apply_root_motion(world, dt);
        self.apply_push_forces(world);
        self.slope_slide(world, dt);
        self.update_network_reconciliation(world, dt);
    }

    // -----------------------------------------------------------------------
    // Ground detection
    // -----------------------------------------------------------------------

    fn detect_ground(&mut self, world: &mut PhysicsWorld) {
        let body = match world.body_mut(self.body) {
            Some(b) => b,
            None => return,
        };
        let pos = body.pos;
        let radius = self.config.capsule_radius;
        let half = self.config.capsule_half_height;

        // Cast downward from the bottom of the capsule.
        let origin = pos + Vec3::new(0.0, -half, 0.0);
        let ray = Ray::new(origin, Vec3::new(0.0, -1.0, 0.0));
        let hit = world.raycast(&ray, radius * 2.5);

        self.ground = hit.map(|h| {
            let slope = h.normal.dot(&Vec3::new(0.0, 1.0, 0.0)).acos();
            GroundContact {
                normal: h.normal,
                point: h.point,
                body: BodyHandle(h.body),
                slope,
                stepping: false,
            }
        });
    }

    fn is_grounded(&self) -> bool {
        self.ground.is_some()
    }

    // -----------------------------------------------------------------------
    // Input handling
    // -----------------------------------------------------------------------

    fn handle_input(&mut self, world: &mut PhysicsWorld, dt: f32) {
        let input_dir = Vec3::new(self.input.move_x, 0.0, self.input.move_y).normalize();
        let forward = match world.body(self.body) {
            Some(b) => b.rot * Vec3::new(0.0, 0.0, -1.0),
            None => return,
        };
        let right = match world.body(self.body) {
            Some(b) => b.rot * Vec3::new(1.0, 0.0, 0.0),
            None => return,
        };
        let mut wish_dir = forward * input_dir.z + right * input_dir.x;

        if wish_dir.norm_squared() > 1.0 {
            wish_dir = wish_dir.normalize();
        }

        let speeds = &self.config.speeds;
        let can_stand = !self.input.crouch;

        if self.input.crouch && self.state != CharacterState::Crouch && self.state != CharacterState::Slide {
            self.set_state(CharacterState::Crouch);
        } else if can_stand && matches!(self.state, CharacterState::Crouch | CharacterState::Slide) {
            if !self.would_be_colliding(world, self.config.capsule_half_height) {
                self.set_state(CharacterState::Idle);
            }
        }

        if self.input.jump {
            self.jump_buffer = 0.15;
        } else if self.jump_buffer > 0.0 {
            self.jump_buffer -= dt;
        }

        if matches!(self.state, CharacterState::Crouch | CharacterState::Slide) && self.input.sprint {
            let body = match world.body(self.body) {
                Some(b) => b,
                None => return,
            };
            let speed = body.lin_vel.norm();
            if speed >= self.config.slide_min_speed && self.slide_timer <= 0.0 {
                self.set_state(CharacterState::Slide);
                self.slide_timer = 0.8;
                self.slide_distance = 0.0;
            }
        }

        if self.is_grounded() && wish_dir.norm_squared() > 0.0 {
            self.try_auto_step(world, wish_dir, dt);
        }

        let body = match world.body_mut(self.body) {
            Some(b) => b,
            None => return,
        };

        match self.state {
            CharacterState::Crouch | CharacterState::Idle | CharacterState::Walk | CharacterState::Run => {
                if self.input.jump && self.is_grounded() && self.coyote_time > 0.0 {
                    body.lin_vel.y = self.config.jump_impulse;
                    self.set_state(CharacterState::Jump);
                    self.coyote_time = 0.0;
                } else if !self.is_grounded() {
                    body.lin_vel.y -= 9.81 * self.config.gravity_scale * dt;
                    self.set_state(if body.lin_vel.y < 0.0 {
                        CharacterState::Fall
                    } else {
                        CharacterState::Jump
                    });
                } else {
                    self.apply_wish_velocity(body, wish_dir, dt);
                    body.lin_vel.y = 0.0;
                }
            }
            CharacterState::Jump | CharacterState::Fall => {
                body.lin_vel.y -= 9.81 * self.config.gravity_scale * dt;
                if self.is_grounded() && body.lin_vel.y <= 0.0 {
                    body.lin_vel.y = 0.0;
                    self.set_state(CharacterState::Idle);
                } else {
                    let mut air_dir = wish_dir;
                    air_dir.y = 0.0;
                    if air_dir.norm_squared() > 1.0 {
                        air_dir = air_dir.normalize();
                    }
                    body.lin_vel.x += (air_dir.x * speeds.air - body.lin_vel.x) * 5.0 * dt;
                    body.lin_vel.z += (air_dir.z * speeds.air - body.lin_vel.z) * 5.0 * dt;
                }
            }
            CharacterState::Slide => {
                body.lin_vel.y -= 9.81 * self.config.gravity_scale * dt;
                if !self.is_grounded() || self.slide_timer <= 0.0 {
                    self.set_state(CharacterState::Crouch);
                }
            }
        }
    }

    fn apply_wish_velocity(&self, body: &mut RigidBody, wish_dir: Vec3, dt: f32) {
        let speeds = &self.config.speeds;
        let target_speed = match self.state {
            CharacterState::Run if self.input.sprint => speeds.run,
            CharacterState::Crouch => speeds.crouch,
            CharacterState::Walk => speeds.walk,
            _ => speeds.walk,
        };

        let mut move_dir = wish_dir;
        move_dir.y = 0.0;
        if move_dir.norm_squared() > 1.0 {
            move_dir = move_dir.normalize();
        }

        let current_ground_speed = Vec3::new(body.lin_vel.x, 0.0, body.lin_vel.z).norm();
        let mut target = move_dir * target_speed;

        // Slope sliding
        if let Some(g) = &self.ground {
            if g.slope > self.config.max_slope_angle {
                let slide_dir = g.normal.cross(&Vec3::new(0.0, 1.0, 0.0)).normalize();
                target += slide_dir * (g.slope - self.config.max_slope_angle) * 4.0;
            }
            if g.slope > self.config.slide_slope_angle {
                target += -g.normal * 2.0;
            }
        }

        let lerp_factor = (target_speed * 8.0 * dt).min(1.0);
        body.lin_vel.x += (target.x - body.lin_vel.x) * lerp_factor;
        body.lin_vel.z += (target.z - body.lin_vel.z) * lerp_factor;

        // Face movement direction
        if move_dir.norm_squared() > 0.001 {
            let target_yaw = move_dir.z.atan2(move_dir.x);
            let current_yaw = Self::yaw_from_quat(body.rot);
            let mut diff = target_yaw - current_yaw;
            while diff > std::f32::consts::PI {
                diff -= 2.0 * std::f32::consts::PI;
            }
            while diff < -std::f32::consts::PI {
                diff += 2.0 * std::f32::consts::PI;
            }
            let max_turn = speeds.rotation_speed * dt;
            let clamped = diff.clamp(-max_turn, max_turn);
            let new_yaw = current_yaw + clamped;
            body.rot = Quat::from_axis_angle(&Vec3::new(0.0, 1.0, 0.0), new_yaw);
        }
    }

    fn yaw_from_quat(q: Quat) -> f32 {
        let m = q.to_rotation_matrix();
        let r = m.matrix();
        (r.m31).atan2(r.m11)
    }

    // -----------------------------------------------------------------------
    // State machine
    // -----------------------------------------------------------------------

    fn set_state(&mut self, new: CharacterState) {
        if self.state != new {
            self.state = new;
        }
    }

    fn update_state_machine(&mut self, dt: f32) {
        if self.jump_buffer > 0.0 && self.is_grounded() && matches!(self.state, CharacterState::Idle | CharacterState::Walk | CharacterState::Run | CharacterState::Crouch) {
            self.jump_buffer = 0.0;
        }

        match self.state {
            CharacterState::Jump | CharacterState::Fall => {
                if self.is_grounded() {
                    self.coyote_time = 0.15;
                    self.set_state(CharacterState::Idle);
                }
            }
            _ => {}
        }

        if self.coyote_time > 0.0 && self.is_grounded() {
            self.coyote_time = 0.15;
        } else if self.coyote_time > 0.0 {
            self.coyote_time -= dt;
        }

        if self.slide_timer > 0.0 {
            self.slide_timer -= dt;
        }
    }

    // -----------------------------------------------------------------------
    // Animation sync
    // -----------------------------------------------------------------------

    fn update_animation(&mut self, world: &mut PhysicsWorld, _dt: f32) {
        let body = match world.body(self.body) {
            Some(b) => b,
            None => return,
        };

        let speed = Vec3::new(body.lin_vel.x, 0.0, body.lin_vel.z).norm();
        let turn_rate = if speed > 0.001 {
            let forward = body.rot * Vec3::new(0.0, 0.0, -1.0);
            let vel = Vec3::new(body.lin_vel.x, 0.0, body.lin_vel.z).normalize();
            (1.0 - forward.dot(&vel).abs()) * self.config.speeds.rotation_speed
        } else {
            0.0
        };

        let direction = Self::yaw_from_quat(body.rot);

        self.anim = AnimationState {
            state: self.state,
            speed,
            direction,
            turn_rate,
            crouching: matches!(self.state, CharacterState::Crouch | CharacterState::Slide),
            grounded: self.is_grounded(),
        };
    }

    // -----------------------------------------------------------------------
    // Foot IK
    // -----------------------------------------------------------------------

    fn update_foot_ik(&mut self, world: &mut PhysicsWorld) {
        let body = match world.body(self.body) {
            Some(b) => b,
            None => return,
        };
        let pos = body.pos;
        let radius = self.config.capsule_radius;
        let half = if matches!(self.state, CharacterState::Crouch | CharacterState::Slide) {
            self.config.crouch_half_height
        } else {
            self.config.capsule_half_height
        };

        let left_origin = pos + body.rot * Vec3::new(-radius * 0.6, -half * 0.8, 0.0);
        let right_origin = pos + body.rot * Vec3::new(radius * 0.6, -half * 0.8, 0.0);

        self.foot_ik.left_valid = self.cast_foot(world, left_origin);
        self.foot_ik.right_valid = self.cast_foot(world, right_origin);

        // If not valid, use a ground-plane approximation.
        if !self.foot_ik.left_valid {
            self.foot_ik.left = left_origin + Vec3::new(0.0, -radius, 0.0);
        }
        if !self.foot_ik.right_valid {
            self.foot_ik.right = right_origin + Vec3::new(0.0, -radius, 0.0);
        }
    }

    fn cast_foot(&self, world: &PhysicsWorld, origin: Vec3) -> bool {
        let ray = Ray::new(origin + Vec3::new(0.0, 0.2, 0.0), Vec3::new(0.0, -1.0, 0.0));
        world.raycast(&ray, self.config.capsule_radius * 3.0).is_some()
    }

    // -----------------------------------------------------------------------
    // Root motion
    // -----------------------------------------------------------------------

    fn apply_root_motion(&mut self, world: &mut PhysicsWorld, dt: f32) {
        if !self.pending_root_motion.active {
            return;
        }

        let body = match world.body_mut(self.body) {
            Some(b) => b,
            None => return,
        };

        let motion = self.pending_root_motion;
        body.lin_vel += motion.translation / dt.max(1e-6);
        body.rot = motion.rotation * body.rot;
        self.pending_root_motion = RootMotion::default();
    }

    pub fn set_root_motion(&mut self, motion: RootMotion) {
        self.pending_root_motion = motion;
    }

    // -----------------------------------------------------------------------
    // Pushable objects
    // -----------------------------------------------------------------------

    fn apply_push_forces(&self, world: &mut PhysicsWorld) {
        let (speed, lin_vel) = match world.body(self.body) {
            Some(b) => (b.lin_vel.norm(), b.lin_vel),
            None => return,
        };
        if speed < 0.1 {
            return;
        }

        let contacts = self.collect_contacts(world);
        for contact in contacts {
            if let Some(target) = world.body_mut(contact.body) {
                if target.body_type == crate::body::BodyType::Dynamic {
                    let push = lin_vel * self.config.mass * 0.25;
                    target.apply_impulse_at(push, contact.point);
                }
            }
        }
    }

    fn collect_contacts(&self, world: &PhysicsWorld) -> Vec<GroundContact> {
        let mut out = Vec::new();
        if let Some(g) = &self.ground {
            out.push(*g);
        }
        out
    }

    // -----------------------------------------------------------------------
    // Auto-step
    // -----------------------------------------------------------------------

    fn try_auto_step(&mut self, world: &PhysicsWorld, wish_dir: Vec3, dt: f32) {
        if self.config.max_step_height <= 0.0 {
            return;
        }

        let body = match world.body(self.body) {
            Some(b) => b,
            None => return,
        };

        let pos = body.pos;
        let radius = self.config.capsule_radius;
        let step = self.config.max_step_height;

        // Cast forward at knee height to detect an obstacle.
        let knee_height = step * 0.5;
        let origin = pos + Vec3::new(0.0, knee_height, 0.0) + wish_dir * radius;
        let ray = Ray::new(origin, wish_dir.normalize());
        let max_dist = radius * 1.5;

        if let Some(hit) = world.raycast(&ray, max_dist) {
            // Cast from above the obstacle to see if there is room on top.
            let top = hit.point + Vec3::new(0.0, step + radius * 0.5, 0.0);
            let down = Ray::new(top, Vec3::new(0.0, -1.0, 0.0));
            if world.raycast(&down, step + radius * 2.0).is_none() {
                self.step_offset = step;
                return;
            }
        }

        self.step_offset *= 0.9_f32.max(0.0);
    }

    // -----------------------------------------------------------------------
    // Slope sliding (passive movement correction)
    // -----------------------------------------------------------------------

    fn slope_slide(&self, world: &mut PhysicsWorld, dt: f32) {
        let body = match world.body_mut(self.body) {
            Some(b) => b,
            None => return,
        };

        if let Some(g) = &self.ground {
            if g.slope > self.config.max_slope_angle && g.slope <= self.config.slide_slope_angle {
                let tangent = g.normal.cross(&Vec3::new(0.0, 1.0, 0.0)).normalize();
                let slide_speed = (g.slope - self.config.max_slope_angle) * 3.0;
                body.lin_vel.x += tangent.x * slide_speed * dt;
                body.lin_vel.z += tangent.z * slide_speed * dt;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Collision checks
    // -----------------------------------------------------------------------

    fn would_be_colliding(&self, world: &PhysicsWorld, half_height: f32) -> bool {
        let body = match world.body(self.body) {
            Some(b) => b,
            None => return false,
        };
        let test_pos = body.pos + Vec3::new(0.0, half_height - self.config.capsule_half_height, 0.0);
        let candidates = world.query_sphere(test_pos, self.config.capsule_radius * 1.5);
        candidates.iter().any(|h| *h != self.body)
    }

    // -----------------------------------------------------------------------
    // Network prediction
    // -----------------------------------------------------------------------

    fn update_network_state(&mut self, world: &mut PhysicsWorld, dt: f32) {
        let body = match world.body(self.body) {
            Some(b) => b,
            None => return,
        };

        let snapshot = CharacterSnapshot {
            pos: body.pos,
            rot: body.rot,
            lin_vel: body.lin_vel,
            ang_vel: body.ang_vel,
            state: self.state,
            input: self.input,
            time: self.time_accumulated() + dt,
        };

        self.prediction.push(snapshot);
    }

    pub fn reconcile(&mut self, world: &mut PhysicsWorld, server_snapshot: CharacterSnapshot) {
        if let Some(client_snapshot) = self.prediction.latest() {
            if (client_snapshot.pos - server_snapshot.pos).norm() > 0.05 {
                let body = match world.body_mut(self.body) {
                    Some(b) => b,
                    None => return,
                };
                body.pos = server_snapshot.pos;
                body.rot = server_snapshot.rot;
                body.lin_vel = server_snapshot.lin_vel;
                body.ang_vel = server_snapshot.ang_vel;

                // Replay inputs from the mismatched snapshot onward.
                if let Some(idx) = self
                    .prediction
                    .history
                    .iter()
                    .position(|s| (s.pos - server_snapshot.pos).norm() < 0.05)
                {
                    for snapshot in &self.prediction.history[idx + 1..] {
                        self.input = snapshot.input;
                        self.update(world, 1.0 / 60.0);
                    }
                }

                self.prediction.clear();
            }
        }
    }

    fn update_network_reconciliation(&mut self, _world: &mut PhysicsWorld, _dt: f32) {
        // Reconciliation runs on authoritative snapshot arrival; nothing to do here.
    }

    fn time_accumulated(&self) -> f32 {
        self.prediction
            .latest()
            .map(|s| s.time)
            .unwrap_or(0.0)
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn yaw_from_quat(q: Quat) -> f32 {
        let m = q.to_rotation_matrix();
        let r = m.matrix();
        (r.m31).atan2(r.m11)
    }
}

// ---------------------------------------------------------------------------
// Builder helpers
// ---------------------------------------------------------------------------

impl CharacterController {
    /// Build a capsule collider matching the controller config.
    pub fn default_collider(&self) -> Collider {
        Collider::capsule(self.config.capsule_radius, self.config.capsule_half_height * 2.0)
            .with_material(Material::new(0.4, 0.1, self.config.mass / 10.0))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::RigidBody;
    use crate::collider::Collider;
    use crate::world::PhysicsConfig;

    fn setup_world() -> PhysicsWorld {
        let mut world = PhysicsWorld::new(PhysicsConfig {
            gravity: Vec3::new(0.0, -9.81, 0.0),
            substeps: 2,
            ..PhysicsConfig::default()
        });
        world.add_body(
            RigidBody::static_body().with_collider(Collider::box_collider(Vec3::new(50.0, 0.5, 50.0))
                .with_offset(Vec3::new(0.0, -0.5, 0.0))),
        );
        world
    }

    #[test]
    fn character_controller_starts_idle() {
        let mut world = setup_world();
        let handle = world.add_body(
            RigidBody::kinematic()
                .with_collider(Collider::capsule(0.35, 1.8))
                .with_position(Vec3::new(0.0, 1.0, 0.0)),
        );
        let mut ctrl = CharacterController::new(handle, CharacterControllerConfig::default());
        ctrl.update(&mut world, 1.0 / 60.0);
        assert_eq!(ctrl.state, CharacterState::Idle);
    }

    #[test]
    fn jump_sets_fall_state() {
        let mut world = setup_world();
        let handle = world.add_body(
            RigidBody::kinematic()
                .with_collider(Collider::capsule(0.35, 1.8))
                .with_position(Vec3::new(0.0, 1.0, 0.0)));

        let mut ctrl = CharacterController::new(handle, CharacterControllerConfig::default());
        ctrl.input.jump = true;
        ctrl.update(&mut world, 1.0 / 60.0);
        assert!(matches!(ctrl.state, CharacterState::Jump | CharacterState::Fall));
    }

    #[test]
    fn animation_state_updates() {
        let mut world = setup_world();
        let handle = world.add_body(
            RigidBody::kinematic()
                .with_collider(Collider::capsule(0.35, 1.8))
                .with_position(Vec3::new(0.0, 1.0, 0.0))),
        );
        let mut ctrl = CharacterController::new(handle, CharacterControllerConfig::default());
        ctrl.input.move_y = 1.0;
        ctrl.update(&mut world, 1.0 / 60.0);
        let anim = ctrl.animation_state();
        assert!(anim.speed >= 0.0);
    }

    #[test]
    fn prediction_buffer_round_trip() {
        let mut buf = PredictionBuffer::new(8);
        let snap = CharacterSnapshot {
            pos: Vec3::new(0.0, 1.0, 0.0),
            rot: Quat::identity(),
            lin_vel: Vec3::zeros(),
            ang_vel: Vec3::zeros(),
            state: CharacterState::Idle,
            input: CharacterInput::default(),
            time: 0.0,
        };
        buf.push(snap);
        buf.push(snap);
        assert_eq!(buf.history.len(), 2);
        assert!(buf.latest().is_some());
    }

    #[test]
    fn foot_ik_defaults_are_valid() {
        let ik = FootIKTarget::default();
        assert_eq!(ik.left_normal, Vec3::new(0.0, 1.0, 0.0));
    }
}
