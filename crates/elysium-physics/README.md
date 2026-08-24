# elysium-physics — Tremor

A self-contained, enterprise-grade physics engine for the Elysium engine,
aligned with Mimari §3.1 (F11 Tremor, F12 broad/narrow phase, F13 particles,
F14 fluid).

## Design

* **Dependency-light core.** Only `nalgebra` — no mandatory ECS coupling. The
  engine is fully unit-tested and builds standalone.
* **Optional ECS bridge.** The `ecs-integration` feature exposes `TremorSystem`
  as an `elysium_core::System` adapter. It is gated because `elysium-core` is a
  separate (currently non-compiling) crate.

## Modules

| Module        | Responsibility                                                |
|---------------|---------------------------------------------------------------|
| `body`        | `RigidBody`, `BodyType`, mass/inertia, integration, sleep      |
| `collider`    | `Collider`, `ColliderShape`, `Material` (friction/restitution) |
| `broadphase`  | Uniform spatial-hash grid (F12 "hücre grid")                  |
| `narrowphase` | Sphere/box/capsule analytic contacts (SAT for boxes)          |
| `solver`      | `TremorSolver` — warm-started sequential impulses + correction |
| `joints`      | Distance and point-to-point constraints                        |
| `world`       | `PhysicsWorld` orchestrator, events, queries, raycasting       |
| `raycast`     | Ray vs sphere/box/capsule                                     |
| `fluid`       | `SPHFluid` — smoothed-particle hydrodynamics (F14)            |
| `vehicle`     | Ray-cast suspension wheeled vehicle model                      |

## Example

```rust
use elysium_physics::*;

let mut world = PhysicsWorld::new(PhysicsConfig::default());

// Ground (its top face is at y = 0).
world.add_body(RigidBody::static_body().with_collider(
    Collider::box_collider(Vec3::new(20.0, 0.5, 20.0))
        .with_offset(Vec3::new(0.0, -0.5, 0.0)),
));

// A bouncing ball.
let ball = world.add_body(
    RigidBody::dynamic(1.0)
        .with_collider(Collider::sphere(0.5))
        .with_position(Vec3::new(0.0, 5.0, 0.0)),
);

for _ in 0..240 {
    world.step(1.0 / 60.0);
}
assert!((world.body(ball).unwrap().pos.y - 0.5).abs() < 0.05);
```

## Jacob's status

* 18 unit + integration tests covering gravity, mass properties, narrow phase,
  raycasting, broad phase, fluid settling, joints and stable box stacking.
* `cargo clippy --package elysium-physics --all-targets` is clean.
