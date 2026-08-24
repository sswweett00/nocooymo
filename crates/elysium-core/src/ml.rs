use std::collections::HashMap;
use nalgebra::{DVector, Matrix3, Vector3};
use serde::{Deserialize, Serialize};
use rand::prelude::SliceRandom;

// ML veri yapısı
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlData {
    pub inputs: Vec<DVector<f32>>,
    pub outputs: Vec<DVector<f32>>,
    pub metadata: MlMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlMetadata {
    pub input_size: usize,
    pub output_size: usize,
    pub sample_count: usize,
    pub created_at: String,
    pub model_type: MlModelType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MlModelType {
    NeuralNetwork,
    DecisionTree,
    RandomForest,
    SupportVectorMachine,
    KNearestNeighbors,
    Custom(String),
}

// ML modeli arayüzü
pub trait MlModel {
    fn train(&mut self, data: &MlData) -> Result<(), Box<dyn std::error::Error>>;
    fn predict(&self, input: &DVector<f32>) -> Result<DVector<f32>, Box<dyn std::error::Error>>;
    fn evaluate(&self, test_data: &MlData) -> Result<MlEvaluationResult, Box<dyn std::error::Error>>;
    fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn load(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>>;
}

// Basit bir yapay sinir ağı modeli
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleNeuralNetwork {
    pub layers: Vec<Layer>,
    pub learning_rate: f32,
    pub activation_function: ActivationFunction,
    pub loss_function: LossFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub weights: nalgebra::DMatrix<f32>,
    pub biases: DVector<f32>,
    pub activation: ActivationFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivationFunction {
    Sigmoid,
    Tanh,
    Relu,
    LeakyRelu(f32),
    Linear,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LossFunction {
    MeanSquaredError,
    CrossEntropy,
    BinaryCrossEntropy,
}

#[derive(Debug, Clone)]
pub struct MlEvaluationResult {
    pub accuracy: f32,
    pub precision: f32,
    pub recall: f32,
    pub f1_score: f32,
    pub loss: f32,
}

impl MlEvaluationResult {
    pub fn new() -> Self {
        Self {
            accuracy: 0.0,
            precision: 0.0,
            recall: 0.0,
            f1_score: 0.0,
            loss: 0.0,
        }
    }
}

impl SimpleNeuralNetwork {
    pub fn new(layer_sizes: &[usize], learning_rate: f32) -> Self {
        let mut layers = Vec::new();
        
        for i in 0..layer_sizes.len() - 1 {
            let input_size = layer_sizes[i];
            let output_size = layer_sizes[i + 1];
            
            let weights = nalgebra::DMatrix::from_element(output_size, input_size, 0.0)
                .map(|_| rand::random::<f32>() * 2.0 - 1.0); // -1 ile 1 arasında rastgele
            let biases = DVector::from_element(output_size, 0.0);
            
            layers.push(Layer {
                weights,
                biases,
                activation: ActivationFunction::Relu,
            });
        }
        
        Self {
            layers,
            learning_rate,
            activation_function: ActivationFunction::Relu,
            loss_function: LossFunction::MeanSquaredError,
        }
    }
    
    fn activate(&self, x: f32, activation: &ActivationFunction) -> f32 {
        match activation {
            ActivationFunction::Sigmoid => 1.0 / (1.0 + (-x).exp()),
            ActivationFunction::Tanh => x.tanh(),
            ActivationFunction::Relu => x.max(0.0),
            ActivationFunction::LeakyRelu(alpha) => if x > 0.0 { x } else { x * alpha },
            ActivationFunction::Linear => x,
        }
    }
    
    fn activate_derivative(&self, x: f32, activation: &ActivationFunction) -> f32 {
        match activation {
            ActivationFunction::Sigmoid => {
                let sigmoid_x = self.activate(x, activation);
                sigmoid_x * (1.0 - sigmoid_x)
            },
            ActivationFunction::Tanh => 1.0 - x.tanh().powi(2),
            ActivationFunction::Relu => if x > 0.0 { 1.0 } else { 0.0 },
            ActivationFunction::LeakyRelu(alpha) => if x > 0.0 { 1.0 } else { *alpha },
            ActivationFunction::Linear => 1.0,
        }
    }
    
    fn forward_pass(&self, input: &DVector<f32>) -> Vec<DVector<f32>> {
        let mut activations = vec![input.clone()];
        
        for layer in &self.layers {
            let prev_activation = activations.last().unwrap();
            let z = &layer.weights * prev_activation + &layer.biases;
            let activated = z.map(|x| self.activate(x, &layer.activation));
            activations.push(activated);
        }
        
        activations
    }
    
    fn calculate_loss(&self, predicted: &DVector<f32>, actual: &DVector<f32>) -> f32 {
        match self.loss_function {
            LossFunction::MeanSquaredError => {
                let diff = predicted - actual;
                (diff.component_mul(&diff)).sum() / predicted.len() as f32
            },
            LossFunction::CrossEntropy => {
                // Basit bir MSE uygulaması ile devam edelim
                let diff = predicted - actual;
                (diff.component_mul(&diff)).sum() / predicted.len() as f32
            },
            LossFunction::BinaryCrossEntropy => {
                // Basit bir MSE uygulaması ile devam edelim
                let diff = predicted - actual;
                (diff.component_mul(&diff)).sum() / predicted.len() as f32
            },
        }
    }
}

impl MlModel for SimpleNeuralNetwork {
    fn train(&mut self, data: &MlData) -> Result<(), Box<dyn std::error::Error>> {
        for epoch in 0..1000 { // Basit bir eğitim döngüsü
            let mut total_loss = 0.0;
            
            for (input, expected_output) in data.inputs.iter().zip(data.outputs.iter()) {
                // İleri yayılım
                let activations = self.forward_pass(input);
                
                // Geri yayılım
                let mut delta = activations.last().unwrap().clone() - expected_output;
                
                for i in (0..self.layers.len()).rev() {
                    let activation = &activations[i];
                    let next_activation = if i > 0 { &activations[i - 1] } else { input };
                    
                    // Gradyan hesaplamaları
                    let activation_derivatives = activation.map(|x| self.activate_derivative(x, &self.layers[i].activation));
                    delta = delta.component_mul(&activation_derivatives);
                    
                    // Ağırlık ve bias güncellemeleri
                    let weight_gradients = &delta * next_activation.transpose();
                    let bias_gradients = delta.clone();
                    
                    // Ağırlıkları ve bias'ları güncelle
                    self.layers[i].weights = &self.layers[i].weights - &(weight_gradients * self.learning_rate);
                    self.layers[i].biases = &self.layers[i].biases - &(bias_gradients * self.learning_rate);
                    
                    // Delta'yı önceki katmana yay
                    delta = self.layers[i].weights.transpose() * &delta;
                }
                
                let loss = self.calculate_loss(activations.last().unwrap(), expected_output);
                total_loss += loss;
            }
            
            // Her 100 epoch'ta bir ortalama hatayı yazdır
            if epoch % 100 == 0 {
                println!("Epoch {}: Average Loss = {}", epoch, total_loss / data.inputs.len() as f32);
            }
        }
        
        Ok(())
    }
    
    fn predict(&self, input: &DVector<f32>) -> Result<DVector<f32>, Box<dyn std::error::Error>> {
        let activations = self.forward_pass(input);
        Ok(activations.last().unwrap().clone())
    }
    
    fn evaluate(&self, test_data: &MlData) -> Result<MlEvaluationResult, Box<dyn std::error::Error>> {
        let mut correct_predictions = 0;
        let mut total_loss = 0.0;
        
        for (input, expected_output) in test_data.inputs.iter().zip(test_data.outputs.iter()) {
            let prediction = self.predict(input)?;
            let loss = self.calculate_loss(&prediction, expected_output);
            total_loss += loss;
            
            // Basit bir doğruluk hesabı (tahmini output ile beklenen output karşılaştırması)
            let diff = (&prediction - expected_output).map(|x| x.abs());
            if diff.max() < 0.1 { // Eşik değeri
                correct_predictions += 1;
            }
        }
        
        let accuracy = correct_predictions as f32 / test_data.inputs.len() as f32;
        
        Ok(MlEvaluationResult {
            accuracy,
            precision: 0.0, // Basit model için placeholder
            recall: 0.0,    // Basit model için placeholder
            f1_score: 0.0,  // Basit model için placeholder
            loss: total_loss / test_data.inputs.len() as f32,
        })
    }
    
    fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let serialized = serde_json::to_string(self)?;
        std::fs::write(path, serialized)?;
        Ok(())
    }
    
    fn load(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let loaded_model: SimpleNeuralNetwork = serde_json::from_str(&contents)?;
        *self = loaded_model;
        Ok(())
    }
}

// ML bileşeni
use crate::Component;

pub struct MlAgent {
    pub model: Box<dyn MlModel>,
    pub training_data: Option<MlData>,
    pub state: MlAgentState,
    pub learning_enabled: bool,
    pub prediction_interval: std::time::Duration,
    pub last_prediction: std::time::SystemTime,
    pub experience_buffer: Vec<MlExperience>,
    pub max_experience: usize,
    pub exploration_rate: f32,
    pub exploitation_rate: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MlAgentState {
    Idle,
    Training,
    Predicting,
    Learning,
}

#[derive(Debug, Clone)]
pub struct MlExperience {
    pub state: DVector<f32>,
    pub action: DVector<f32>,
    pub reward: f32,
    pub next_state: DVector<f32>,
    pub done: bool,
}

impl MlAgent {
    pub fn new(model: Box<dyn MlModel>) -> Self {
        Self {
            model,
            training_data: None,
            state: MlAgentState::Idle,
            learning_enabled: true,
            prediction_interval: std::time::Duration::from_millis(100),
            last_prediction: std::time::SystemTime::now(),
            experience_buffer: Vec::new(),
            max_experience: 10000,
            exploration_rate: 0.1,
            exploitation_rate: 0.9,
        }
    }
    
    pub fn set_training_data(&mut self, data: MlData) {
        self.training_data = Some(data);
    }
    
    pub fn train(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref data) = self.training_data {
            self.state = MlAgentState::Training;
            self.model.train(data)?;
            self.state = MlAgentState::Idle;
            Ok(())
        } else {
            Err("No training data available".into())
        }
    }
    
    pub fn predict(&self, input: &DVector<f32>) -> Result<DVector<f32>, Box<dyn std::error::Error>> {
        self.model.predict(input)
    }
    
    pub fn evaluate(&self, test_data: &MlData) -> Result<MlEvaluationResult, Box<dyn std::error::Error>> {
        self.model.evaluate(test_data)
    }
    
    pub fn add_experience(&mut self, experience: MlExperience) {
        self.experience_buffer.push(experience);
        
        // Buffer'ı sınırla
        if self.experience_buffer.len() > self.max_experience {
            self.experience_buffer.remove(0);
        }
    }
    
    pub fn should_predict(&self) -> bool {
        if let Ok(elapsed) = std::time::SystemTime::now().duration_since(self.last_prediction) {
            elapsed >= self.prediction_interval
        } else {
            true
        }
    }
    
    pub fn update_prediction_timer(&mut self) {
        self.last_prediction = std::time::SystemTime::now();
    }
    
    pub fn select_action(&self, state: &DVector<f32>) -> Result<DVector<f32>, Box<dyn std::error::Error>> {
        // Keşif-açgözlülük dengesi
        let explore = rand::random::<f32>() < self.exploration_rate;
        
        if explore {
            // Rastgele aksiyon (keşif)
            Ok(DVector::from_iterator(state.len(), (0..state.len()).map(|_| rand::random::<f32>() * 2.0 - 1.0)))
        } else {
            // Modelden tahmin (açgözlülük)
            self.predict(state)
        }
    }
    
    pub fn save_model(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Modeli kaydetmek için geçici bir değişken kullan
        // Bu sadece örnek; gerçek implementasyonda modelin klonlanması gerekir
        todo!("Modelin klonlanabilir olması gerek");
    }
    
    pub fn load_model(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.model.load(path)
    }
}

// RL (Reinforcement Learning) bileşeni
pub struct ReinforcementLearningAgent {
    pub q_table: HashMap<String, f32>, // Basit bir Q-learning tablosu
    pub learning_rate: f32,
    pub discount_factor: f32,
    pub epsilon: f32, // Keşif oranı
    pub state_space: Vec<String>, // Olası durumlar
    pub action_space: Vec<String>, // Olası aksiyonlar
}

impl ReinforcementLearningAgent {
    pub fn new(learning_rate: f32, discount_factor: f32, epsilon: f32) -> Self {
        Self {
            q_table: HashMap::new(),
            learning_rate,
            discount_factor,
            epsilon,
            state_space: Vec::new(),
            action_space: Vec::new(),
        }
    }
    
    pub fn get_q_value(&self, state: &str, action: &str) -> f32 {
        let key = format!("{}:{}", state, action);
        *self.q_table.get(&key).unwrap_or(&0.0)
    }
    
    pub fn update_q_value(&mut self, state: &str, action: &str, value: f32) {
        let key = format!("{}:{}", state, action);
        self.q_table.insert(key, value);
    }
    
    pub fn choose_action(&self, state: &str) -> Option<String> {
        if rand::random::<f32>() < self.epsilon {
            // Keşif: Rastgele aksiyon
            self.action_space.choose(&mut rand::thread_rng()).cloned()
        } else {
            // Açgözlülük: En iyi bilinen aksiyon
            self.action_space.iter()
                .max_by(|a, b| self.get_q_value(state, a).partial_cmp(&self.get_q_value(state, b)).unwrap())
                .cloned()
        }
    }
    
    pub fn learn(&mut self, state: &str, action: &str, reward: f32, next_state: &str) {
        let current_q = self.get_q_value(state, action);
        let next_max_q = self.action_space.iter()
            .map(|a| self.get_q_value(next_state, a))
            .fold(f32::NEG_INFINITY, f32::max);
        
        let new_q = current_q + self.learning_rate * (reward + self.discount_factor * next_max_q - current_q);
        self.update_q_value(state, action, new_q);
    }
    
    pub fn add_state(&mut self, state: String) {
        if !self.state_space.contains(&state) {
            self.state_space.push(state);
        }
    }
    
    pub fn add_action(&mut self, action: String) {
        if !self.action_space.contains(&action) {
            self.action_space.push(action);
        }
    }
}

// ML sistem bileşeni
pub struct MlSystem {
    pub agents: Vec<MlAgent>,
    pub models: HashMap<String, Box<dyn MlModel>>,
    pub datasets: HashMap<String, MlData>,
    pub is_learning_enabled: bool,
    pub learning_thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl MlSystem {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            models: HashMap::new(),
            datasets: HashMap::new(),
            is_learning_enabled: true,
            learning_thread_handle: None,
        }
    }
    
    pub fn register_agent(&mut self, agent: MlAgent) -> usize {
        let id = self.agents.len();
        self.agents.push(agent);
        id
    }
    
    pub fn register_model(&mut self, name: String, model: Box<dyn MlModel>) {
        self.models.insert(name, model);
    }
    
    pub fn register_dataset(&mut self, name: String, data: MlData) {
        self.datasets.insert(name, data);
    }
    
    pub fn get_agent(&self, id: usize) -> Option<&MlAgent> {
        self.agents.get(id)
    }
    
    pub fn get_agent_mut(&mut self, id: usize) -> Option<&mut MlAgent> {
        self.agents.get_mut(id)
    }
    
    pub fn get_model(&self, name: &str) -> Option<&Box<dyn MlModel>> {
        self.models.get(name)
    }
    
    pub fn get_model_mut(&mut self, name: &str) -> Option<&mut Box<dyn MlModel>> {
        self.models.get_mut(name)
    }
    
    pub fn train_agent(&mut self, agent_id: usize) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.train()
        } else {
            Err("Agent not found".into())
        }
    }
    
    pub fn predict_with_agent(&self, agent_id: usize, input: &DVector<f32>) -> Result<DVector<f32>, Box<dyn std::error::Error>> {
        if let Some(agent) = self.agents.get(agent_id) {
            agent.predict(input)
        } else {
            Err("Agent not found".into())
        }
    }
    
    pub fn update(&mut self, delta_time: f32) {
        if !self.is_learning_enabled {
            return;
        }
        
        // Aktif ajanları güncelle
        for agent in &mut self.agents {
            if agent.should_predict() && agent.state == MlAgentState::Idle {
                // Gerekirse tahmin yap
                agent.update_prediction_timer();
            }
        }
    }
    
    pub fn start_learning_thread(&mut self) {
        if self.learning_thread_handle.is_some() {
            return; // Zaten çalışıyor
        }
        
        let _agents_clone = self.agents.len(); // Gerçek implementasyonda Arc<Mutex<>> kullanılmalı
        
        self.learning_thread_handle = Some(std::thread::spawn(move || {
            // Öğrenme döngüsü
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
                
                // Burada arka planda öğrenme işlemleri yapılabilir
                // Bu sadece placeholder
            }
        }));
    }
    
    pub fn stop_learning_thread(&mut self) {
        if let Some(handle) = self.learning_thread_handle.take() {
            // Thread'i sonlandırmak için işaret gönder
            // Gerçek implementasyonda bu daha karmaşık olabilir
        }
    }
}

// ML yardımcı fonksiyonları
pub mod ml_utils {
    use super::*;
    
    pub fn normalize_vector(v: &DVector<f32>) -> DVector<f32> {
        let norm = v.norm();
        if norm > 0.0 {
            v / norm
        } else {
            v.clone()
        }
    }
    
    pub fn create_sample_data(input_size: usize, output_size: usize, sample_count: usize) -> MlData {
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        
        for _ in 0..sample_count {
            let input = DVector::from_iterator(input_size, (0..input_size).map(|_| rand::random::<f32>() * 2.0 - 1.0));
            let output = DVector::from_iterator(output_size, (0..output_size).map(|_| rand::random::<f32>() * 2.0 - 1.0));
            
            inputs.push(input);
            outputs.push(output);
        }
        
        MlData {
            inputs,
            outputs,
            metadata: MlMetadata {
                input_size,
                output_size,
                sample_count,
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs().to_string())
                    .unwrap_or_else(|_| "0".to_string()),
                model_type: MlModelType::NeuralNetwork,
            },
        }
    }
    
    pub fn split_data(data: &MlData, train_ratio: f32) -> (MlData, MlData) {
        let split_point = (data.metadata.sample_count as f32 * train_ratio) as usize;
        
        let train_inputs = data.inputs.iter().take(split_point).cloned().collect();
        let train_outputs = data.outputs.iter().take(split_point).cloned().collect();
        
        let test_inputs = data.inputs.iter().skip(split_point).cloned().collect();
        let test_outputs = data.outputs.iter().skip(split_point).cloned().collect();
        
        let train_data = MlData {
            inputs: train_inputs,
            outputs: train_outputs,
            metadata: MlMetadata {
                sample_count: split_point,
                ..data.metadata.clone()
            },
        };
        
        let test_data = MlData {
            inputs: test_inputs,
            outputs: test_outputs,
            metadata: MlMetadata {
                sample_count: data.metadata.sample_count - split_point,
                ..data.metadata.clone()
            },
        };
        
        (train_data, test_data)
    }
    
    pub fn calculate_accuracy(predicted: &DVector<f32>, actual: &DVector<f32>, tolerance: f32) -> f32 {
        let matches = predicted.iter()
            .zip(actual.iter())
            .filter(|(p, a)| (**p - **a).abs() < tolerance)
            .count();
            
        matches as f32 / predicted.len() as f32
    }
}

// ML ile ilgili yardımcı veri yapıları
#[derive(Debug, Clone)]
pub struct MlFeature {
    pub name: String,
    pub value: f32,
    pub min_value: f32,
    pub max_value: f32,
    pub importance: f32,
}

#[derive(Debug, Clone)]
pub struct MlFeatureVector {
    pub features: Vec<MlFeature>,
}

impl MlFeatureVector {
    pub fn new() -> Self {
        Self {
            features: Vec::new(),
        }
    }
    
    pub fn add_feature(&mut self, name: String, value: f32, min_val: f32, max_val: f32) {
        let importance = 1.0; // Placeholder
        self.features.push(MlFeature {
            name,
            value,
            min_value: min_val,
            max_value: max_val,
            importance,
        });
    }
    
    pub fn to_dvector(&self) -> DVector<f32> {
        DVector::from_iterator(
            self.features.len(),
            self.features.iter().map(|f| f.value)
        )
    }
    
    pub fn normalize(&mut self) {
        for feature in &mut self.features {
            if feature.max_value != feature.min_value {
                feature.value = (feature.value - feature.min_value) / (feature.max_value - feature.min_value);
            }
        }
    }
}

// ML ile oyun nesnesi kontrolü
pub struct MlControlledObject {
    pub control_type: MlControlType,
    pub neural_network: Option<SimpleNeuralNetwork>,
    pub feature_weights: Vec<f32>,
    pub last_action: Option<DVector<f32>>,
    pub reward: f32,
    pub cumulative_reward: f32,
}

#[derive(Debug, Clone)]
pub enum MlControlType {
    Autonomous,         // Tam otomatik kontrol
    Assisted,           // Yardımcı kontrol
    Predictive,         // Tahmini kontrol
    Reactive,           // Tepkisel kontrol
}

impl MlControlledObject {
    pub fn new(control_type: MlControlType) -> Self {
        Self {
            control_type,
            neural_network: None,
            feature_weights: vec![],
            last_action: None,
            reward: 0.0,
            cumulative_reward: 0.0,
        }
    }
    
    pub fn set_neural_network(&mut self, nn: SimpleNeuralNetwork) {
        self.neural_network = Some(nn);
    }
    
    pub fn get_action(&self, state: &DVector<f32>) -> Result<DVector<f32>, Box<dyn std::error::Error>> {
        if let Some(ref nn) = self.neural_network {
            nn.predict(state)
        } else {
            Err("No neural network assigned".into())
        }
    }
    
    pub fn update_reward(&mut self, reward: f32) {
        self.reward = reward;
        self.cumulative_reward += reward;
    }
    
    pub fn reset_cumulative_reward(&mut self) {
        self.cumulative_reward = 0.0;
    }
}