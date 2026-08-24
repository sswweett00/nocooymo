//! Enterprise system scheduler with dependency resolution, parallel execution,
//! and stage-based ordering.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::world::World;

// ---------------------------------------------------------------------------
// System trait
// ---------------------------------------------------------------------------

/// A system is a function that operates on the world.
pub trait System: Send + Sync {
    fn name(&self) -> &str;
    fn update(&mut self, world: &mut World, dt: f32);
}

/// Wrapper for a closure-based system.
pub struct FunctionSystem<F> {
    name: String,
    func: F,
}

impl<F> FunctionSystem<F>
where
    F: Fn(&mut World, f32) + Send + Sync,
{
    pub fn new(name: impl Into<String>, func: F) -> Self {
        Self {
            name: name.into(),
            func,
        }
    }
}

impl<F> System for FunctionSystem<F>
where
    F: Fn(&mut World, f32) + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn update(&mut self, world: &mut World, dt: f32) {
        (self.func)(world, dt);
    }
}

// ---------------------------------------------------------------------------
// Schedule stages
// ---------------------------------------------------------------------------

/// Execution stage.  Systems in the same stage may run in parallel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stage {
    /// First stage — systems that run once at startup / resource init.
    Startup,
    /// First per-frame stage — input, early logic.
    PreUpdate,
    /// Main update stage.
    Update,
    /// Late update — post-processing, camera follow.
    PostUpdate,
    /// Before rendering.
    PreRender,
    /// After rendering.
    PostRender,
}

impl Stage {
    pub fn order(self) -> u8 {
        match self {
            Self::Startup => 0,
            Self::PreUpdate => 1,
            Self::Update => 2,
            Self::PostUpdate => 3,
            Self::PreRender => 4,
            Self::PostRender => 5,
        }
    }
}

// ---------------------------------------------------------------------------
// System descriptor
// ---------------------------------------------------------------------------

/// Describes a system and its dependencies.
pub struct SystemDescriptor {
    pub name: String,
    pub stage: Stage,
    /// Systems that must run *before* this one.
    pub after: Vec<String>,
    /// Systems that must run *after* this one.
    pub before: Vec<String>,
    /// Component types this system reads (for parallel scheduling).
    pub reads: Vec<String>,
    /// Component types this system writes (for parallel scheduling).
    pub writes: Vec<String>,
    pub system: Box<dyn System>,
}

impl SystemDescriptor {
    pub fn new(name: impl Into<String>, system: Box<dyn System>) -> Self {
        Self {
            name: name.into(),
            stage: Stage::Update,
            after: Vec::new(),
            before: Vec::new(),
            reads: Vec::new(),
            writes: Vec::new(),
            system,
        }
    }

    pub fn in_stage(mut self, stage: Stage) -> Self {
        self.stage = stage;
        self
    }

    pub fn after(mut self, name: impl Into<String>) -> Self {
        self.after.push(name.into());
        self
    }

    pub fn before(mut self, name: impl Into<String>) -> Self {
        self.before.push(name.into());
        self
    }

    pub fn reads(mut self, component: impl Into<String>) -> Self {
        self.reads.push(component.into());
        self
    }

    pub fn writes(mut self, component: impl Into<String>) -> Self {
        self.writes.push(component.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Scheduler
// ---------------------------------------------------------------------------

/// The system scheduler.  Resolves dependencies and runs systems in order.
pub struct Scheduler {
    /// All registered systems, indexed by name.
    systems: HashMap<String, SystemDescriptor>,
    /// Ordered list of (stage, system_name) after topological sort.
    schedule: Vec<(Stage, String)>,
    /// Whether the schedule needs to be recomputed.
    dirty: bool,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            systems: HashMap::new(),
            schedule: Vec::new(),
            dirty: true,
        }
    }

    /// Add a system to the scheduler.
    pub fn add(&mut self, descriptor: SystemDescriptor) -> &mut Self {
        let name = descriptor.name.clone();
        self.systems.insert(name, descriptor);
        self.dirty = true;
        self
    }

    /// Remove a system.
    pub fn remove(&mut self, name: &str) -> Option<Box<dyn System>> {
        let desc = self.systems.remove(name)?;
        self.dirty = true;
        Some(desc.system)
    }

    /// Recompute the execution schedule using topological sort.
    pub fn build(&mut self) -> Result<(), String> {
        let mut graph: HashMap<String, HashSet<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // Initialize
        for name in self.systems.keys() {
            graph.entry(name.clone()).or_default();
            in_degree.entry(name.clone()).or_insert(0);
        }

        // Build edges from "after" constraints
        for (name, desc) in &self.systems {
            for after in &desc.after {
                if self.systems.contains_key(after) {
                    graph.entry(after.clone()).or_default().insert(name.clone());
                    *in_degree.entry(name.clone()).or_insert(0) += 1;
                }
            }
            for before in &desc.before {
                if self.systems.contains_key(before) {
                    graph.entry(name.clone()).or_default().insert(before.clone());
                    *in_degree.entry(before.clone()).or_insert(0) += 1;
                }
            }
        }

        // Kahn's algorithm, grouped by stage
        let mut schedule = Vec::new();
        let mut remaining: HashSet<String> = self.systems.keys().cloned().collect();

                for stage in [
            Stage::Startup,
            Stage::PreUpdate,
            Stage::Update,
            Stage::PostUpdate,
            Stage::PreRender,
            Stage::PostRender,
        ] {
            let stage_systems: Vec<String> = self
                .systems
                .iter()
                .filter(|(_, d)| d.stage == stage)
                .map(|(n, _)| n.clone())
                .collect();

            if stage_systems.is_empty() {
                continue;
            }

            // Topological sort within this stage — yalnızca stage-içi kenarları say
            let mut stage_in_degree: HashMap<String, usize> = HashMap::new();
            for name in &stage_systems {
                let mut deg = 0usize;
                // after/before içindeki aynı stage'deki öncülleri say
                if let Some(desc) = self.systems.get(name) {
                    for dep in &desc.after {
                        if stage_systems.contains(dep) {
                            deg += 1;
                        }
                    }
                    // before kenarları ters yöndür — bu sisteme bağımlı olanlar değil, bağımlı olduklarımızı sayıyoruz
                    // before için global graph'tan gelen indegree'i stage-içi filtrele
                }
                // Ayrıca 'before' nedeniyle oluşan kenarlar: bu sistemden başka sisteme giden kenar
                // in_degree'i etkilemez, sadece hedef etkilenir. O yüzden sadece 'after' yeterli.
                // Ancak global graph'ta before kenarları da var — onları da stage-içi say
                // Bunun için graph tersine bak: bu node'a gelen kenarlar
                for (src, neighbors) in &graph {
                    if neighbors.contains(name) && stage_systems.contains(src) {
                        // Bu sistem src'e bağımlı değil, tersine src bu sisteme bağımlı?
                        // after zaten kapsıyor, bu döngü çift saymayı önlemek için sadece after'a güven
                        // bu yüzden burada işlem yok
                    }
                }
                // En doğru: global in_degree'den sadece stage-içi öncülleri çıkar
                let global_deg = in_degree.get(name).copied().unwrap_or(0);
                // stage-içi olmayan öncülleri düş
                let mut intra = 0;
                for (pred, neighbors) in &graph {
                    if neighbors.contains(name) && stage_systems.contains(pred) {
                        intra += 1;
                    }
                }
                let _ = global_deg;
                deg = intra;
                stage_in_degree.insert(name.clone(), deg);
            }

            let mut queue: Vec<String> = stage_systems
                .iter()
                .filter(|n| *stage_in_degree.get(*n).unwrap_or(&0) == 0)
                .cloned()
                .collect();
            queue.sort(); // deterministic order

            // FIFO + deterministik: her iterasyonda en küçük elemanı al
            queue.sort();
            while !queue.is_empty() {
                let name = queue.remove(0);
                if !remaining.remove(&name) {
                    continue;
                }
                schedule.push((stage, name.clone()));
                if let Some(neighbors) = graph.get(&name) {
                    for neighbor in neighbors {
                        if let Some(deg) = stage_in_degree.get_mut(neighbor) {
                            *deg = deg.saturating_sub(1);
                            if *deg == 0 && remaining.contains(neighbor) {
                                queue.push(neighbor.clone());
                            }
                        }
                    }
                }
                queue.sort();
            }
        }

        // Check for cycles
        if !remaining.is_empty() {
            return Err(format!(
                "Cycle detected in system dependencies. Unresolved: {:?}",
                remaining
            ));
        }

        self.schedule = schedule;
        self.dirty = false;
        Ok(())
    }

    /// Run all systems in schedule order.
    pub fn run(&mut self, world: &mut World, dt: f32) -> Result<(), String> {
        if self.dirty {
            self.build()?;
        }

        for (stage, name) in &self.schedule.clone() {
            if let Some(desc) = self.systems.get_mut(name) {
                let _ = stage; // stage is for future parallel grouping
                desc.system.update(world, dt);
            }
        }
        Ok(())
    }

    /// Get the current schedule (for debugging/visualisation).
    pub fn schedule(&self) -> &[(Stage, String)] {
        &self.schedule
    }

    /// Number of registered systems.
    pub fn system_count(&self) -> usize {
        self.systems.len()
    }

    /// Clear all systems.
    pub fn clear(&mut self) {
        self.systems.clear();
        self.schedule.clear();
        self.dirty = true;
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Fixed timestep scheduler for physics
// ---------------------------------------------------------------------------

/// Runs systems at a fixed timestep, accumulating time.
pub struct FixedTimestep {
    accumulator: f32,
    timestep: f32,
    max_substeps: u32,
}

impl FixedTimestep {
    pub fn new(timestep: f32) -> Self {
        Self {
            accumulator: 0.0,
            timestep,
            max_substeps: 10,
        }
    }

    pub fn with_max_substeps(mut self, max: u32) -> Self {
        self.max_substeps = max;
        self
    }

    /// Advance the simulation.  Returns the number of substeps that should run.
    pub fn advance(&mut self, dt: f32) -> u32 {
        self.accumulator += dt;
        // Spiral-of-death öncesi clamp
        if self.accumulator > self.timestep * self.max_substeps as f32 {
            self.accumulator = self.timestep * self.max_substeps as f32;
        }
        let mut substeps = 0;
        while self.accumulator >= self.timestep && substeps < self.max_substeps {
            self.accumulator -= self.timestep;
            substeps += 1;
        }
        substeps
    }

    /// The fixed timestep duration.
    pub fn timestep(&self) -> f32 {
        self.timestep
    }

    /// The interpolation alpha (how far between previous and current step).
    pub fn alpha(&self) -> f32 {
        if self.timestep > 0.0 {
            self.accumulator / self.timestep
        } else {
            0.0
        }
    }
}

// ---------------------------------------------------------------------------
// Convenience aliases & TimeSystem (used by the engine binary)
// ---------------------------------------------------------------------------

/// Alias for [`Scheduler`] — the per-frame schedule driving systems.
pub type Schedule = Scheduler;
/// Alias for [`Stage`] matching the engine binary's naming.
pub type ScheduleStage = Stage;

/// Tracks elapsed game time.  Stored as a resource alongside the world and
/// exposed as `TimeSystem` by the engine.
pub struct TimeSystem {
    pub elapsed: f32,
}

impl TimeSystem {
    pub fn new() -> Self {
        Self { elapsed: 0.0 }
    }
}

impl Default for TimeSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for TimeSystem {
    fn name(&self) -> &str {
        "TimeSystem"
    }

    fn update(&mut self, _world: &mut World, dt: f32) {
        self.elapsed += dt;
    }
}

impl Scheduler {
    /// Register a boxed system in a specific [`Stage`].
    pub fn add_system_to_stage(&mut self, stage: Stage, system: Box<dyn System>) -> &mut Self {
        let name = system.name().to_string();
        self.add(SystemDescriptor::new(name, system).in_stage(stage));
        self
    }

    /// Run all systems with a fixed timestep of 1/60s.
    pub fn run_update(&mut self, world: &mut World) {
        let _ = self.run(world, 1.0 / 60.0);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_execution_order() {
        let mut scheduler = Scheduler::new();
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));

        let o1 = order.clone();
        scheduler.add(
            SystemDescriptor::new(
                "a",
                Box::new(FunctionSystem::new("a", move |_, _| {
                    o1.lock().unwrap().push("a");
                })),
            )
            .after("b"),
        );

        let o2 = order.clone();
        scheduler.add(SystemDescriptor::new(
            "b",
            Box::new(FunctionSystem::new("b", move |_, _| {
                o2.lock().unwrap().push("b");
            })),
        ));

        scheduler.build().unwrap();
        let mut world = World::new();
        scheduler.run(&mut world, 0.016).unwrap();

        let result = order.lock().unwrap();
        assert_eq!(*result, vec!["b", "a"]);
    }

    #[test]
    fn cycle_detection() {
        let mut scheduler = Scheduler::new();
        scheduler.add(
            SystemDescriptor::new(
                "a",
                Box::new(FunctionSystem::new("a", |_, _| {})),
            )
            .after("b"),
        );
        scheduler.add(
            SystemDescriptor::new(
                "b",
                Box::new(FunctionSystem::new("b", |_, _| {})),
            )
            .after("a"),
        );
        let result = scheduler.build();
        assert!(result.is_err());
    }

    #[test]
    fn fixed_timestep() {
        let mut ft = FixedTimestep::new(1.0 / 60.0);
        let substeps = ft.advance(1.0 / 60.0 * 3.0);
        assert_eq!(substeps, 3);
    }

    #[test]
    fn fixed_timestep_alpha() {
        let mut ft = FixedTimestep::new(0.1);
        ft.advance(0.15);
        assert!((ft.alpha() - 0.5).abs() < 0.01);
    }
}