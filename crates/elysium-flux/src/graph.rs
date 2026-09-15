use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use elysium_core::{World, Entity, Transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Input(InputNode),
    Output(OutputNode),
    Function(FunctionNode),
    Variable(VariableNode),
    Conditional(ConditionalNode),
    Loop(LoopNode),
    Event(EventNode),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputNode {
    pub name: String,
    pub data_type: DataType,
    pub default_value: Option<DataValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputNode {
    pub name: String,
    pub data_type: DataType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionNode {
    pub function_name: String,
    pub inputs: Vec<Pin>,
    pub outputs: Vec<Pin>,
    pub properties: HashMap<String, DataValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableNode {
    pub variable_name: String,
    pub data_type: DataType,
    pub is_global: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalNode {
    pub condition: String, // Expression to evaluate
    pub true_outputs: Vec<Pin>,
    pub false_outputs: Vec<Pin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopNode {
    pub loop_type: LoopType,
    pub collection_pin: Pin,
    pub body_pins: Vec<Pin>,
    pub outputs: Vec<Pin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventNode {
    pub event_type: EventType,
    pub outputs: Vec<Pin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopType {
    For,
    ForEach,
    While,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    OnStart,
    OnUpdate,
    OnCollision,
    OnTriggerEnter,
    OnClick,
    OnKeyDown,
    OnCustom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub name: String,
    pub data_type: DataType,
    pub direction: PinDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PinDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    Boolean,
    Integer,
    Float,
    String,
    Vector2,
    Vector3,
    Entity,
    Transform,
    Object,
    Array(Box<DataType>),
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataValue {
    Boolean(bool),
    Integer(i32),
    Float(f32),
    String(String),
    Vector2([f32; 2]),
    Vector3([f32; 3]),
    Entity(Entity),
    Transform(Transform),
    Array(Vec<DataValue>),
    Custom(String, Vec<u8>), // (type_name, serialized_data)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub source_node: NodeId,
    pub source_pin: String,
    pub target_node: NodeId,
    pub target_pin: String,
    pub data_type: DataType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionGraph {
    pub nodes: HashMap<NodeId, NodeType>,
    pub edges: HashMap<EdgeId, Edge>,
    pub start_nodes: Vec<NodeId>,
    pub properties: GraphProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphProperties {
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub auto_execute: bool,
    pub execution_order: u32,
}

impl ExecutionGraph {
    pub fn new(name: String) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            start_nodes: Vec::new(),
            properties: GraphProperties {
                name,
                description: String::new(),
                author: String::new(),
                version: "1.0".to_string(),
                auto_execute: true,
                execution_order: 0,
            },
        }
    }

    pub fn add_node(&mut self, node_type: NodeType) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.insert(id, node_type);
        
        // If this is an event node, add it to start nodes
        if matches!(self.nodes[&id], NodeType::Event(_)) {
            self.start_nodes.push(id);
        }
        
        id
    }

    pub fn add_edge(&mut self, source_node: NodeId, source_pin: String, target_node: NodeId, target_pin: String, data_type: DataType) -> Result<EdgeId, String> {
        // Validate that the nodes and pins exist
        let source_node_exists = self.nodes.contains_key(&source_node);
        let target_node_exists = self.nodes.contains_key(&target_node);
        
        if !source_node_exists {
            return Err(format!("Source node {:?} does not exist", source_node));
        }
        
        if !target_node_exists {
            return Err(format!("Target node {:?} does not exist", target_node));
        }
        
        // Create edge
        let edge_id = EdgeId(self.edges.len() as u32);
        let edge = Edge {
            id: edge_id,
            source_node,
            source_pin,
            target_node,
            target_pin,
            data_type,
        };
        
        self.edges.insert(edge_id, edge);
        Ok(edge_id)
    }

    pub fn execute(&mut self, world: &mut World) -> Result<(), Box<dyn std::error::Error>> {
        // Execute all start nodes - clone to avoid borrow conflict
        let starts = self.start_nodes.clone();
        for start_node_id in starts {
            self.execute_node(start_node_id, world)?;
        }
        
        Ok(())
    }

    fn execute_node(&mut self, node_id: NodeId, world: &mut World) -> Result<(), Box<dyn std::error::Error>> {
        let node_type = match self.nodes.get(&node_id).cloned() {
            Some(nt) => nt,
            None => return Err(format!("Node {:?} not found", node_id).into()),
        };
        match node_type {
            NodeType::Event(event_node) => {
                self.execute_event_node(&event_node, node_id, world)?;
            }
            NodeType::Function(function_node) => {
                self.execute_function_node(&function_node, node_id, world)?;
            }
            NodeType::Conditional(conditional_node) => {
                self.execute_conditional_node(&conditional_node, node_id, world)?;
            }
            NodeType::Variable(variable_node) => {
                self.execute_variable_node(&variable_node, node_id, world)?;
            }
            NodeType::Input(_input_node) => {
            }
            NodeType::Output(_output_node) => {
            }
            NodeType::Loop(loop_node) => {
                self.execute_loop_node(&loop_node, node_id, world)?;
            }
        }
        self.execute_connected_nodes(node_id, world)?;
        Ok(())
    }

    fn execute_event_node(&mut self, _event_node: &EventNode, _node_id: NodeId, _world: &mut World) -> Result<(), Box<dyn std::error::Error>> {
        // Events trigger execution of connected nodes
        // For now, we just continue execution
        Ok(())
    }

    fn execute_function_node(&mut self, function_node: &FunctionNode, node_id: NodeId, world: &mut World) -> Result<(), Box<dyn std::error::Error>> {
        // Execute the function based on its name
        match function_node.function_name.as_str() {
            "Log" => {
                // Get input value
                if let Some(value) = self.get_input_value(&function_node.inputs[0], node_id, world)? {
                    println!("Script Log: {:?}", value);
                }
            }
            "SetPosition" => {
                // Set entity position
                if function_node.inputs.len() >= 2 {
                    if let Some(entity_value) = self.get_input_value(&function_node.inputs[0], node_id, world)? {
                        if let Some(pos_value) = self.get_input_value(&function_node.inputs[1], node_id, world)? {
                            if let DataValue::Entity(entity) = entity_value {
                                if let DataValue::Vector3(pos) = pos_value {
                                    if let Some(transform) = world.get_component_mut::<Transform>(entity) {
                                        transform.translation.x = pos[0];
                                        transform.translation.y = pos[1];
                                        transform.translation.z = pos[2];
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "GetPosition" => {
                // Get entity position
                if let Some(entity_value) = self.get_input_value(&function_node.inputs[0], node_id, world)? {
                    if let DataValue::Entity(entity) = entity_value {
                        if let Some(transform) = world.get_component::<Transform>(entity) {
                            let pos = [transform.translation.x, transform.translation.y, transform.translation.z];
                            
                            // Set output value (in a real implementation, we'd store this for connected nodes)
                            let _ = pos;
                        }
                    }
                }
            }
            _ => {
                // Unknown function
                return Err(format!("Unknown function: {}", function_node.function_name).into());
            }
        }
        
        Ok(())
    }

    fn execute_conditional_node(&mut self, conditional_node: &ConditionalNode, node_id: NodeId, world: &mut World) -> Result<(), Box<dyn std::error::Error>> {
        let true_outputs = conditional_node.true_outputs.clone();
        for output_pin in &true_outputs {
            self.execute_connected_nodes_for_pin(node_id, &output_pin.name, world)?;
        }
        
        Ok(())
    }

    fn execute_variable_node(&mut self, variable_node: &VariableNode, node_id: NodeId, world: &mut World) -> Result<(), Box<dyn std::error::Error>> {
        // Handle variable access
        // For now, this is just a placeholder
        let _ = (variable_node, node_id, world);
        Ok(())
    }

    fn execute_loop_node(&mut self, loop_node: &LoopNode, node_id: NodeId, world: &mut World) -> Result<(), Box<dyn std::error::Error>> {
        let _ = (loop_node, node_id, world);
        Ok(())
    }

    fn get_input_value(&self, pin: &Pin, current_node: NodeId, _world: &mut World) -> Result<Option<DataValue>, Box<dyn std::error::Error>> {
        // Find incoming edges to this pin
        let incoming_edge = self.edges.values()
            .find(|edge| edge.target_node == current_node && edge.target_pin == pin.name);
        
        if let Some(_edge) = incoming_edge {
            // Find the output value from the source node
            // This is a simplified implementation - in reality, you'd need to execute the source node first
            // and store its output value somewhere
            
            // For now, we'll just return a default value based on type
            Ok(match &pin.data_type {
                DataType::Boolean => Some(DataValue::Boolean(false)),
                DataType::Integer => Some(DataValue::Integer(0)),
                DataType::Float => Some(DataValue::Float(0.0)),
                DataType::String => Some(DataValue::String(String::new())),
                DataType::Vector2 => Some(DataValue::Vector2([0.0, 0.0])),
                DataType::Vector3 => Some(DataValue::Vector3([0.0, 0.0, 0.0])),
                DataType::Entity => Some(DataValue::Entity(Entity::from_parts(0, 0))),
                DataType::Transform => Some(DataValue::Transform(Transform::default())),
                DataType::Array(_inner_type) => Some(DataValue::Array(Vec::new())),
                DataType::Object => None, // Objects are complex, return None
                DataType::Custom(_) => None, // Custom types need special handling
            })
        } else {
            // No incoming connection, return default or error
            Ok(None)
        }
    }

    fn execute_connected_nodes(&mut self, current_node: NodeId, world: &mut World) -> Result<(), Box<dyn std::error::Error>> {
        // Find all edges originating from this node - clone to avoid borrow issues
        let outgoing: Vec<NodeId> = self.edges.values()
            .filter(|edge| edge.source_node == current_node)
            .map(|edge| edge.target_node)
            .collect();
        
        for target in outgoing {
            self.execute_node(target, world)?;
        }
        
        Ok(())
    }

    fn execute_connected_nodes_for_pin(&mut self, current_node: NodeId, pin_name: &str, world: &mut World) -> Result<(), Box<dyn std::error::Error>> {
        let outgoing: Vec<NodeId> = self.edges.values()
            .filter(|edge| edge.source_node == current_node && edge.source_pin == *pin_name)
            .map(|edge| edge.target_node)
            .collect();
        
        for target in outgoing {
            self.execute_node(target, world)?;
        }
        
        Ok(())
    }
}

pub struct GraphCompiler {
    pub optimization_level: u32,
}

impl GraphCompiler {
    pub fn new() -> Self {
        Self {
            optimization_level: 2,
        }
    }

    pub fn compile(&self, _graph: &ExecutionGraph) -> Result<CompiledGraph, Box<dyn std::error::Error>> {
        // In a real implementation, this would convert the graph to optimized bytecode or machine code
        // For now, we'll just return a placeholder
        Ok(CompiledGraph {
            bytecode: vec![],
            entry_points: vec![],
        })
    }
}

#[derive(Debug)]
pub struct CompiledGraph {
    pub bytecode: Vec<u8>,
    pub entry_points: Vec<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_graph_creation() {
        let mut graph = ExecutionGraph::new("TestGraph".to_string());
        assert_eq!(graph.properties.name, "TestGraph");
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = ExecutionGraph::new("TestGraph".to_string());
        
        let input_node = NodeType::Input(InputNode {
            name: "Input1".to_string(),
            data_type: DataType::Integer,
            default_value: Some(DataValue::Integer(0)),
        });
        
        let node_id = graph.add_node(input_node);
        assert!(graph.nodes.contains_key(&node_id));
    }
}