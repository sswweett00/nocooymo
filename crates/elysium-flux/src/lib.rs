pub mod ir;
pub mod graph;
pub mod codegen;

pub use ir::*;
pub use graph::*;
pub use codegen::*;

use std::collections::HashMap;
use elysium_core::World;

pub struct KineticSystem {
    pub graphs: HashMap<String, ExecutionGraph>,
    pub compiler: Compiler,
}

impl KineticSystem {
    pub fn new() -> Self {
        Self {
            graphs: HashMap::new(),
            compiler: Compiler::new(),
        }
    }

    pub fn register_graph(&mut self, name: String, graph: ExecutionGraph) {
        self.graphs.insert(name, graph);
    }

    pub fn execute_graph(&mut self, name: &str, world: &mut World) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(graph) = self.graphs.get_mut(name) {
            graph.execute(world)
        } else {
            Err(format!("Graph {} not found", name).into())
        }
    }

    pub fn register_systems(schedule: &mut elysium_core::scheduler::Schedule) {
        let sys = elysium_core::scheduler::FunctionSystem::new("kinetic_update_system", |_world: &mut World, _dt: f32| {});
        schedule.add_system_to_stage(elysium_core::scheduler::Stage::Update, Box::new(sys));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kinetic_system() {
        let mut system = KineticSystem::new();
        assert_eq!(system.graphs.len(), 0);
    }
}