//! Inference engine, NPC davranış tahmini, hareket tahmini, dudak senkronizasyonu.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use ndarray::{Array1, Array2};

use crate::Model;

// ─────────────────────────────────────────────────── Inference Engine

pub struct InferenceEngine {
    pub models: HashMap<String, Arc<Mutex<dyn Model>>>,
    pub gpu_accelerated: bool,
    pub batch_size: usize,
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            gpu_accelerated: false,
            batch_size: 32,
        }
    }

    pub fn register_model(&mut self, name: String, model: Arc<Mutex<dyn Model>>) {
        self.models.insert(name, model);
    }

    pub fn run_inference(
        &self,
        model_name: &str,
        input: &[f32],
    ) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let model = self
            .models
            .get(model_name)
            .ok_or_else(|| format!("Model '{}' bulunamadı", model_name))?;
        let mut guard = model.lock().unwrap();
        guard.predict(input)
    }

    pub fn run_batch_inference(
        &self,
        model_name: &str,
        inputs: &[Vec<f32>],
    ) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
        inputs
            .iter()
            .map(|input| self.run_inference(model_name, input))
            .collect()
    }

    pub fn enable_gpu(&mut self) {
        self.gpu_accelerated = true;
    }

    pub fn set_batch_size(&mut self, size: usize) {
        self.batch_size = size;
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────── Synapse System

pub struct SynapseSystem {
    pub inference_engine: InferenceEngine,
    pub behavior_predictors: HashMap<String, NpcBehaviorPredictor>,
    pub motion_predictors: HashMap<String, MotionPredictionNetwork>,
    pub lip_sync_predictors: HashMap<String, LipSyncPredictor>,
}

impl Default for SynapseSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl SynapseSystem {
    pub fn new() -> Self {
        Self {
            inference_engine: InferenceEngine::new(),
            behavior_predictors: HashMap::new(),
            motion_predictors: HashMap::new(),
            lip_sync_predictors: HashMap::new(),
        }
    }

    pub fn register_model(&mut self, name: String, model: Arc<Mutex<dyn Model>>) {
        self.inference_engine.register_model(name, model);
    }

    pub fn run_inference(
        &self,
        model_name: &str,
        input: &[f32],
    ) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        self.inference_engine.run_inference(model_name, input)
    }

    pub fn register_npc_behavior_predictor(&mut self, name: String, p: NpcBehaviorPredictor) {
        self.behavior_predictors.insert(name, p);
    }

    pub fn predict_npc_behavior(&self, name: &str, memory: &[f32], stimulus: &[f32]) -> Option<NpcAction> {
        self.behavior_predictors
            .get(name)
            .map(|p| p.predict_action(memory, stimulus))
    }

    pub fn register_motion_predictor(&mut self, name: String, p: MotionPredictionNetwork) {
        self.motion_predictors.insert(name, p);
    }

    pub fn predict_motion(
        &self,
        name: &str,
        pos: [f32; 3],
        vel: [f32; 3],
        steps: usize,
    ) -> Option<Vec<[f32; 3]>> {
        self.motion_predictors
            .get(name)
            .map(|p| p.predict_motion(pos, vel, steps))
    }

    pub fn register_lip_sync_predictor(&mut self, name: String, p: LipSyncPredictor) {
        self.lip_sync_predictors.insert(name, p);
    }

    pub fn predict_lip_sync(&self, name: &str, text: &str) -> Option<Vec<Vec<f32>>> {
        self.lip_sync_predictors
            .get(name)
            .map(|p| p.predict_mouth_shapes(text))
    }
}

// ─────────────────────────────────────────────────── NPC Behavior Predictor

pub struct NpcBehaviorPredictor {
    /// Kişilik ağırlıkları: [memory_slots × personality_traits]
    pub personality_weights: Array2<f32>,
    pub memory_decay: f32,
    pub decision_threshold: f32,
}

impl NpcBehaviorPredictor {
    pub fn new(personality_traits: usize, memory_slots: usize) -> Self {
        let personality_weights = Array2::from_shape_fn((memory_slots, personality_traits), |_| {
            (rand::random::<f32>() - 0.5) * 2.0
        });
        Self {
            personality_weights,
            memory_decay: 0.95,
            decision_threshold: 0.7,
        }
    }

    pub fn predict_action(&self, memory: &[f32], stimulus: &[f32]) -> NpcAction {
        // Birleşik girdi uzunluğu = memory uzunluğu (slots), stimulus eksikse 0 ile pad
        let n = memory.len();
        let combined: Vec<f32> = (0..n)
            .map(|i| memory[i] * self.memory_decay + stimulus.get(i).copied().unwrap_or(0.0))
            .collect();

        let input = Array1::from(combined);
        let weights_t = self.personality_weights.t();
        let scores = weights_t.dot(&input);

        let (max_idx, max_val) = scores
            .iter()
            .enumerate()
            .fold((0usize, f32::NEG_INFINITY), |(bi, bv), (i, &v)| {
                if v > bv { (i, v) } else { (bi, bv) }
            });

        if max_val > self.decision_threshold {
            match max_idx % 5 {
                0 => NpcAction::MoveToRandomLocation,
                1 => NpcAction::AttackNearestThreat,
                2 => NpcAction::FleeFromDanger,
                3 => NpcAction::InteractWithEnvironment,
                4 => NpcAction::Patrol,
                _ => NpcAction::Idle,
            }
        } else {
            NpcAction::Idle
        }
    }

    pub fn update_personality(&mut self, trait_importance: &[f32]) {
        // trait_importance uzunluğu = personality_traits (sütun sayısı)
        for (col_idx, &importance) in trait_importance.iter().enumerate() {
            if col_idx < self.personality_weights.ncols() {
                let mut col = self.personality_weights.column_mut(col_idx);
                col.mapv_inplace(|w| w * importance);
            }
        }
    }
}

// ─────────────────────────────────────────────────── NPC Actions

#[derive(Debug, Clone, PartialEq)]
pub enum NpcAction {
    MoveToRandomLocation,
    AttackNearestThreat,
    FleeFromDanger,
    InteractWithEnvironment,
    Idle,
    Patrol,
    Investigate,
    DefendArea,
}

// ─────────────────────────────────────────────────── Motion Prediction

pub struct MotionPredictionNetwork {
    pub prediction_horizon: usize,
    pub time_step: f32,
}

impl MotionPredictionNetwork {
    pub fn new(prediction_horizon: usize) -> Self {
        Self {
            prediction_horizon,
            time_step: 0.1,
        }
    }

    /// Basit fizik entegrasyonu ile ileriye dönük konum tahminleri üretir.
    pub fn predict_motion(
        &self,
        current_position: [f32; 3],
        velocity: [f32; 3],
        time_steps: usize,
    ) -> Vec<[f32; 3]> {
        let mut predictions = Vec::with_capacity(time_steps);
        let mut pos = current_position;
        let mut vel = velocity;
        let drag = 0.98f32;

        for _ in 0..time_steps.min(self.prediction_horizon) {
            pos[0] += vel[0] * self.time_step;
            pos[1] += vel[1] * self.time_step;
            pos[2] += vel[2] * self.time_step;
            vel[0] *= drag;
            vel[1] *= drag;
            vel[2] *= drag;
            predictions.push(pos);
        }
        predictions
    }
}

// ─────────────────────────────────────────────────── Lip Sync Predictor

pub struct LipSyncPredictor {
    /// Fonem → ağız şekli katsayıları (3-boyutlu blendshape).
    pub phoneme_map: HashMap<char, Vec<f32>>,
}

impl Default for LipSyncPredictor {
    fn default() -> Self {
        Self::new()
    }
}

impl LipSyncPredictor {
    pub fn new() -> Self {
        let mut phoneme_map = HashMap::new();
        // Temel fonem → blendshape haritası
        phoneme_map.insert('A', vec![1.0, 0.0, 0.0]);
        phoneme_map.insert('E', vec![0.7, 0.3, 0.0]);
        phoneme_map.insert('I', vec![0.4, 0.6, 0.0]);
        phoneme_map.insert('O', vec![0.2, 0.8, 0.0]);
        phoneme_map.insert('U', vec![0.1, 0.9, 0.0]);
        phoneme_map.insert(' ', vec![0.0, 1.0, 0.0]);
        Self { phoneme_map }
    }

    pub fn predict_mouth_shapes(&self, text: &str) -> Vec<Vec<f32>> {
        let closed = vec![0.0, 1.0, 0.0];
        text.chars()
            .map(|c| {
                self.phoneme_map
                    .get(&c.to_ascii_uppercase())
                    .cloned()
                    .unwrap_or_else(|| closed.clone())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_predictor() {
        let predictor = NpcBehaviorPredictor::new(5, 10);
        let memory = vec![0.5f32; 10];
        let stimulus = vec![0.3f32; 5];
        let _action = predictor.predict_action(&memory, &stimulus);
    }

    #[test]
    fn test_motion_prediction() {
        let predictor = MotionPredictionNetwork::new(10);
        let pos = [0.0f32; 3];
        let vel = [1.0, 0.0, 0.0];
        let preds = predictor.predict_motion(pos, vel, 5);
        assert_eq!(preds.len(), 5);
        assert!(preds[0][0] > pos[0]);
    }

    #[test]
    fn test_lip_sync() {
        let predictor = LipSyncPredictor::new();
        let shapes = predictor.predict_mouth_shapes("HELLO");
        assert_eq!(shapes.len(), 5);
        assert_eq!(shapes[0].len(), 3);
    }

    #[test]
    fn test_inference_engine_empty() {
        let engine = InferenceEngine::new();
        assert!(engine.models.is_empty());
        assert!(engine.run_inference("nonexistent", &[1.0]).is_err());
    }
}
