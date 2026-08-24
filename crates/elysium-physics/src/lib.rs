//! # Elysium Tremor — enterprise physics engine
//!
//! A self-contained rigid-body + fluid + vehicle physics engine conforming to
//! the Elysium architecture (Mimari §3.1):
//!
//! * **F11 — Tremor**: XPBD-flavoured sequential-impulse solver with
//!   restitution, Coulomb friction, warm starting and position correction.
//! * **F12 — Broad/Narrow phase**: uniform spatial-hash grid + analytic
//!   primitive contacts (sphere / box / capsule).
//! * **F13 — Fracture & particles**: see `SPHFluid` for the particle backend
//!   (fracture support lives on the render/asset side).
//! * **F14 — Fluid**: `SPHFluid` — a compact SPH solver.
//! * **Vehicles & joints**: ray-cast suspension `Vehicle` and `Joint`s.
//!
//! The core is dependency-light (`nalgebra` only) and fully unit-tested.
//! Optional ECS integration (feature `ecs-integration`) exposes `TremorSystem`.

pub mod body;
pub mod broadphase;
pub mod collider;
pub mod contact;
pub mod fluid;
pub mod joints;
pub mod math;
pub mod narrowphase;
pub mod raycast;
pub mod solver;
pub mod vehicle;
pub mod world;

// Re-export the public surface for ergonomic `use elysium_physics::*`.
pub use body::{BodyHandle, BodyType, RigidBody, static_plane};
pub use broadphase::UniformGrid;
pub use collider::{Collider, ColliderShape, Material};
pub use contact::{ContactManifold, ContactPoint};
pub use fluid::{FluidParticle, SPHFluid};
pub use joints::Joint;
pub use math::{Aabb, Mat3, Quat, Vec3};
pub use raycast::{Ray, RayHit};
pub use solver::{SolverSettings, TremorSolver};
pub use vehicle::{Vehicle, VehicleWheel};
pub use world::{ContactEvent, PhysicsConfig, PhysicsWorld};

/// Optional ECS bridge. When the `ecs-integration` feature is enabled,
/// `TremorSystem` adapts a [`PhysicsWorld`] into the elysium-core scheduler.
#[cfg(feature = "ecs-integration")]
pub use ecs::{TremorSystem, PhysicsWorldResource};

#[cfg(feature = "ecs-integration")]
mod ecs {
    //! Integration with the elysium-core ECS scheduler.
    //!
    //! Enabled via the `ecs-integration` feature. Because elysium-core is a
    //! separate breaking crate, this adapter is gated so the core engine
    //! always builds and tests on its own.

    use crate::world::PhysicsWorld;

    /// Resource key under which the [`PhysicsWorld`] is stored.
    #[derive(Debug, Clone)]
    pub struct PhysicsWorldResource;

    /// A scheduler `System` that advances the embedded [`PhysicsWorld`].
    #[derive(Debug)]
    pub struct TremorSystem {
        world: PhysicsWorld,
    }

    impl TremorSystem {
        pub fn new(world: PhysicsWorld) -> Self {
            Self { world }
        }
        pub fn world(&self) -> &PhysicsWorld {
            &self.world
        }
        pub fn world_mut(&mut self) -> &mut PhysicsWorld {
            &mut self.world
        }
    }

    impl elysium_core::System for TremorSystem {
        fn name(&self) -> &str {
            "TremorSystem"
        }
        fn update(&mut self, _world: &mut elysium_core::World, dt: f32) {
            self.world.step(dt);
        }
    }
}
