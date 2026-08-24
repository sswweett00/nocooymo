//! Elysium ML altyapısı — inference engine, NPC davranışı, hareket tahmini (Mimari §9).

pub mod model;
pub mod inference;

pub use model::*;
pub use inference::*;

use elysium_core::{World, System};

// ─────────────────────────────────────────────────── Model trait (tek tanım)

/// Tüm ML modellerinin uygulaması gereken temel trait.
pub trait Model: Send + Sync {
    fn predict(&mut self, input: &[f32]) -> Result<Vec<f32>, Box<dyn std::error::Error>>;
    fn train(
        &mut self,
        data: &[Vec<f32>],
        labels: &[Vec<f32>],
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn load(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>>;
}

// ─────────────────────────────────────────────────── ML Update System

pub struct MLUpdateSystem;

impl System for MLUpdateSystem {
    fn update(&mut self, _world: &mut World, _dt: f32) {
        // İlerideki fazlarda: NPC inference, motion prediction tick'i buraya.
    }

    fn name(&self) -> &str {
        "MLUpdateSystem"
    }
}

pub fn build_ml_schedule(schedule: &mut elysium_core::scheduler::Schedule) {
    let sys = elysium_core::scheduler::FunctionSystem::new("ml_update_fn", |_world: &mut World, _dt: f32| {});
    schedule.add_system_to_stage(elysium_core::scheduler::Stage::Update, Box::new(sys));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct MockModel {
        weights: Vec<f32>,
    }

    impl Model for MockModel {
        fn predict(&mut self, input: &[f32]) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
            let out = input
                .iter()
                .zip(self.weights.iter())
                .map(|(x, w)| x * w)
                .collect();
            Ok(out)
        }
        fn train(&mut self, _data: &[Vec<f32>], _labels: &[Vec<f32>]) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }
        fn save(&self, _path: &str) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }
        fn load(&mut self, _path: &str) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }
    }

    #[test]
    fn test_mock_model_predict() {
        let mut m = MockModel { weights: vec![1.0, 2.0, 3.0] };
        let out = m.predict(&[1.0, 1.0, 1.0]).unwrap();
        assert_eq!(out, vec![1.0, 2.0, 3.0]);
    }
}
