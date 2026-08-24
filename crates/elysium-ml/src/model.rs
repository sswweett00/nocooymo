use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use ndarray::{Array, Array1, Array2, ArrayD, IxDyn};
use std::fs::File;
use std::io::{BufReader, BufWriter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralNetwork {
    pub layers: Vec<Layer>,
    pub activation_functions: Vec<ActivationFunction>,
    pub learning_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub weights: Array2<f32>,
    pub biases: Array1<f32>,
    pub layer_type: LayerType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerType {
    Dense { input_size: usize, output_size: usize },
    Conv2D { kernel_size: (usize, usize), channels: usize },
    Pooling { pool_size: (usize, usize) },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivationFunction {
    ReLU,
    Sigmoid,
    Tanh,
    Linear,
}

impl NeuralNetwork {
    pub fn new(layers_config: &[LayerConfig]) -> Result<Self, Box<dyn std::error::Error>> {
        let mut layers = Vec::new();
        let mut activation_functions = Vec::new();
        
        for config in layers_config {
            let layer = match config.layer_type {
                LayerType::Dense { input_size, output_size } => {
                    // Initialize weights with Xavier initialization
                    let weights = Array2::from_shape_fn((input_size, output_size), |_| {
                        let range = (6.0 / (input_size + output_size) as f32).sqrt();
                        (rand::random::<f32>() - 0.5) * 2.0 * range
                    });
                    
                    let biases = Array1::zeros(output_size);
                    
                    Layer {
                        weights,
                        biases,
                        layer_type: LayerType::Dense { input_size, output_size },
                    }
                },
                _ => return Err("Only Dense layers are currently supported".into()),
            };
            
            layers.push(layer);
            activation_functions.push(config.activation.clone());
        }
        
        Ok(NeuralNetwork {
            layers,
            activation_functions,
            learning_rate: 0.01,
        })
    }
    
    pub fn forward(&self, input: &Array2<f32>) -> Result<Array2<f32>, Box<dyn std::error::Error>> {
        let mut current_input = input.clone();
        
        for (i, layer) in self.layers.iter().enumerate() {
            let z = current_input.dot(&layer.weights) + &layer.biases;
            let a = self.apply_activation(&z, &self.activation_functions[i]);
            current_input = a;
        }
        
        Ok(current_input)
    }
    
    fn apply_activation(&self, input: &Array2<f32>, activation: &ActivationFunction) -> Array2<f32> {
        match activation {
            ActivationFunction::ReLU => input.mapv(|x| x.max(0.0)),
            ActivationFunction::Sigmoid => input.mapv(|x| 1.0 / (1.0 + (-x).exp())),
            ActivationFunction::Tanh => input.mapv(|x| x.tanh()),
            ActivationFunction::Linear => input.clone(),
        }
    }
    
    pub fn predict(&mut self, input: &[f32]) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let input_array = Array2::from_shape_vec((1, input.len()), input.to_vec())?;
        let output = self.forward(&input_array)?;
        let result = output.row(0).to_vec();
        Ok(result)
    }
    
    pub fn train_batch(&mut self, inputs: &[Vec<f32>], targets: &[Vec<f32>]) -> Result<(), Box<dyn std::error::Error>> {
        // This is a simplified training implementation
        // In a real implementation, you'd want to use backpropagation
        
        for (input, target) in inputs.iter().zip(targets.iter()) {
            let input_arr = Array2::from_shape_vec((1, input.len()), input.clone())?;
            let target_arr = Array2::from_shape_vec((1, target.len()), target.clone())?;
            
            // Forward pass
            let output = self.forward(&input_arr)?;
            
            // Simple gradient calculation (not real backprop)
            let error = &target_arr - &output;
            
            // Update weights (simplified)
            for layer in &mut self.layers {
                // This is a very simplified weight update
                // Real implementation would use proper backpropagation
                let weight_updates = input_arr.t().dot(&error) * self.learning_rate;
                layer.weights = &layer.weights + &weight_updates;
            }
        }
        
        Ok(())
    }
    
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, self)?;
        Ok(())
    }
    
    pub fn load(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let loaded: NeuralNetwork = bincode::deserialize_from(reader)?;
        
        // Update self with loaded data
        self.layers = loaded.layers;
        self.activation_functions = loaded.activation_functions;
        self.learning_rate = loaded.learning_rate;
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct LayerConfig {
    pub layer_type: LayerType,
    pub activation: ActivationFunction,
}

impl LayerConfig {
    pub fn dense(input_size: usize, output_size: usize, activation: ActivationFunction) -> Self {
        Self {
            layer_type: LayerType::Dense { input_size, output_size },
            activation,
        }
    }
}

pub struct ModelRegistry {
    pub models: HashMap<String, Box<dyn crate::Model>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, model: Box<dyn crate::Model>) {
        self.models.insert(name, model);
    }

    pub fn get(&self, name: &str) -> Option<&(dyn crate::Model + 'static)> {
        self.models.get(name).map(|b| b.as_ref() as &dyn crate::Model)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut (dyn crate::Model + 'static)> {
        self.models.get_mut(name).map(|b| b.as_mut() as &mut dyn crate::Model)
    }
}