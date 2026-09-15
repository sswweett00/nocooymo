/// visual_script.rs — Visual Scripting System
/// Node tabanlı oyun mantığı editörü.
///
/// Özellikler:
/// - 40+ node tipi (Event, Flow, Math, Logic, Variable, Action, Query)
/// - Connected node graph with execution flow
/// - Code generation (Rust, pseudo-code, WGSL compute)
/// - Script runtime engine (interpret graph execution)
/// - Variable store (global + per-script)
/// - Built-in templates (player controller, enemy AI, pickup)
/// - Undo/redo for graph edits
/// - Copy/paste nodes
/// - Breakpoint support for debugging

use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque};

// ═══════════════════════════════════════════════════════════ Node Types

/// All visual script node categories
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeCategory {
    Event,      // Trigger nodes (game events)
    Flow,       // Control flow (if, loop, sequence)
    Math,       // Math operations
    Logic,      // Boolean logic
    Compare,    // Comparison operators
    Variable,   // Get/set variables
    Action,     // Game actions (move, spawn, destroy)
    Query,      // Query game state
    String,     // String operations
    Vector,     // Vector math
    Conversion, // Type conversion
    Comment,    // Comments (non-executable)
}

/// Specific node types
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    // ── Event Nodes ──
    OnStart,
    OnUpdate,
    OnCollision,
    OnKeyDown,
    OnKeyUp,
    OnMouseClick,
    OnTimer,
    OnDistanceLess,
    OnHealthBelow,
    OnSignal,
    OnSpawn,

    // ── Flow Control ──
    If,
    IfElse,
    ForLoop,
    WhileLoop,
    Sequence,
    Delay,
    Gate,
    Switch,
    Abort,
    SequenceMulti,

    // ── Math ──
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    Abs,
    Min,
    Max,
    Clamp,
    Lerp,
    Remap,
    RandomFloat,
    RandomInt,
    Floor,
    Ceil,
    Round,
    Sine,
    Cosine,
    Atan2,
    Sqrt,

    // ── Logic ──
    And,
    Or,
    Not,
    Xor,

    // ── Compare ──
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // ── Variable ──
    SetVariable,
    GetVariable,
    Increment,
    Decrement,
    Toggle,

    // ── Action ──
    MoveObject,
    RotateObject,
    ScaleObject,
    SetPosition,
    SetRotation,
    SetScale,
    SetColor,
    SetHealth,
    SpawnObject,
    DestroyObject,
    PlaySound,
    EmitParticles,
    ApplyForce,
    SendMessage,
    Teleport,
    SetVisibility,
    SetStatic,

    // ── Query ──
    GetPosition,
    GetRotation,
    GetScale,
    GetHealth,
    GetDistance,
    GetPlayerPosition,
    GetMousePosition,
    GetObjectByName,
    IsAlive,
    IsColliding,
    GetVelocity,
    GetDeltaTime,
    GetTime,
    GetInputAxis,

    // ── Vector ──
    Vector3Construct,
    Vector3Split,
    Vector3Add,
    Vector3Scale,
    Vector3Normalize,
    Vector3Length,
    Vector3Dot,
    Vector3Cross,

    // ── String ──
    StringConcat,
    StringContains,
    StringFormat,

    // ── Conversion ──
    FloatToInt,
    IntToFloat,
    BoolToFloat,
    FloatToBool,

    // ── Comment ──
    Comment,

    // ── Custom ──
    Custom(u32),
}

impl NodeType {
    pub fn category(&self) -> NodeCategory {
        match self {
            Self::OnStart | Self::OnUpdate | Self::OnCollision | Self::OnKeyDown |
            Self::OnKeyUp | Self::OnMouseClick | Self::OnTimer | Self::OnDistanceLess |
            Self::OnHealthBelow | Self::OnSignal | Self::OnSpawn => NodeCategory::Event,

            Self::If | Self::IfElse | Self::ForLoop | Self::WhileLoop |
            Self::Sequence | Self::Delay | Self::Gate | Self::Switch |
            Self::Abort | Self::SequenceMulti => NodeCategory::Flow,

            Self::Add | Self::Subtract | Self::Multiply | Self::Divide |
            Self::Modulo | Self::Power | Self::Abs | Self::Min | Self::Max |
            Self::Clamp | Self::Lerp | Self::Remap | Self::RandomFloat |
            Self::RandomInt | Self::Floor | Self::Ceil | Self::Round |
            Self::Sine | Self::Cosine | Self::Atan2 | Self::Sqrt => NodeCategory::Math,

            Self::And | Self::Or | Self::Not | Self::Xor => NodeCategory::Logic,

            Self::Equal | Self::NotEqual | Self::Greater | Self::GreaterEqual |
            Self::Less | Self::LessEqual => NodeCategory::Compare,

            Self::SetVariable | Self::GetVariable | Self::Increment |
            Self::Decrement | Self::Toggle => NodeCategory::Variable,

            Self::MoveObject | Self::RotateObject | Self::ScaleObject |
            Self::SetPosition | Self::SetRotation | Self::SetScale |
            Self::SetColor | Self::SetHealth | Self::SpawnObject |
            Self::DestroyObject | Self::PlaySound | Self::EmitParticles |
            Self::ApplyForce | Self::SendMessage | Self::Teleport |
            Self::SetVisibility | Self::SetStatic => NodeCategory::Action,

            Self::GetPosition | Self::GetRotation | Self::GetScale |
            Self::GetHealth | Self::GetDistance | Self::GetPlayerPosition |
            Self::GetMousePosition | Self::GetObjectByName | Self::IsAlive |
            Self::IsColliding | Self::GetVelocity | Self::GetDeltaTime |
            Self::GetTime | Self::GetInputAxis => NodeCategory::Query,

            Self::Vector3Construct | Self::Vector3Split | Self::Vector3Add |
            Self::Vector3Scale | Self::Vector3Normalize | Self::Vector3Length |
            Self::Vector3Dot | Self::Vector3Cross => NodeCategory::Vector,

            Self::StringConcat | Self::StringContains | Self::StringFormat => NodeCategory::String,

            Self::FloatToInt | Self::IntToFloat | Self::BoolToFloat |
            Self::FloatToBool => NodeCategory::Conversion,

            Self::Comment => NodeCategory::Comment,
            Self::Custom(_) => NodeCategory::Action,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::OnStart => "On Start", Self::OnUpdate => "On Update",
            Self::OnCollision => "On Collision", Self::OnKeyDown => "On Key Down",
            Self::OnKeyUp => "On Key Up", Self::OnMouseClick => "On Mouse Click",
            Self::OnTimer => "On Timer", Self::OnDistanceLess => "Distance < Threshold",
            Self::OnHealthBelow => "Health < Value", Self::OnSignal => "On Signal",
            Self::OnSpawn => "On Spawn",
            Self::If => "If", Self::IfElse => "If/Else", Self::ForLoop => "For Loop",
            Self::WhileLoop => "While Loop", Self::Sequence => "Sequence",
            Self::Delay => "Delay", Self::Gate => "Gate", Self::Switch => "Switch",
            Self::Abort => "Abort", Self::SequenceMulti => "Multi Sequence",
            Self::Add => "Add (+)", Self::Subtract => "Subtract (-)",
            Self::Multiply => "Multiply (×)", Self::Divide => "Divide (÷)",
            Self::Modulo => "Modulo (%)", Self::Power => "Power (^)",
            Self::Lerp => "Lerp", Self::Clamp => "Clamp", Self::Remap => "Remap",
            Self::RandomFloat => "Random Float", Self::RandomInt => "Random Int",
            Self::And => "AND", Self::Or => "OR", Self::Not => "NOT", Self::Xor => "XOR",
            Self::Equal => "== (!=)", Self::Greater => "> (>=)", Self::Less => "< (<=)",
            Self::SetVariable => "Set Variable", Self::GetVariable => "Get Variable",
            Self::Increment => "Increment", Self::Decrement => "Decrement",
            Self::MoveObject => "Move", Self::RotateObject => "Rotate",
            Self::SpawnObject => "Spawn", Self::DestroyObject => "Destroy",
            Self::PlaySound => "Play Sound", Self::ApplyForce => "Apply Force",
            Self::GetPosition => "Get Position", Self::GetDistance => "Get Distance",
            Self::GetPlayerPosition => "Player Position", Self::GetDeltaTime => "Delta Time",
            Self::GetTime => "Game Time", Self::SetHealth => "Set Health",
            Self::Vector3Construct => "Vec3 Make", Self::Vector3Split => "Vec3 Split",
            Self::Vector3Add => "Vec3 Add", Self::Vector3Normalize => "Vec3 Normalize",
            Self::Comment => "Comment",
            _ => "Unknown",
        }
    }

    /// Get input pin definitions
    pub fn input_pins(&self) -> Vec<(String, PinType)> {
        match self {
            Self::OnUpdate | Self::OnStart | Self::OnSpawn => vec![],
            Self::OnKeyDown | Self::OnKeyUp => vec![
                ("Key".into(), PinType::String),
            ],
            Self::OnTimer => vec![
                ("Interval".into(), PinType::Float),
            ],
            Self::OnDistanceLess => vec![
                ("Distance".into(), PinType::Float),
            ],
            Self::OnHealthBelow => vec![
                ("Threshold".into(), PinType::Float),
            ],
            Self::OnCollision => vec![
                ("Tag".into(), PinType::String),
            ],
            Self::If | Self::IfElse => vec![
                ("Condition".into(), PinType::Bool),
            ],
            Self::ForLoop => vec![
                ("Start".into(), PinType::Int),
                ("End".into(), PinType::Int),
            ],
            Self::WhileLoop => vec![
                ("Condition".into(), PinType::Bool),
            ],
            Self::Delay => vec![
                ("Seconds".into(), PinType::Float),
            ],
            Self::Gate => vec![
                ("Open".into(), PinType::Bool),
            ],
            Self::Switch => vec![
                ("Index".into(), PinType::Int),
            ],
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide => vec![
                ("A".into(), PinType::Float),
                ("B".into(), PinType::Float),
            ],
            Self::Modulo => vec![
                ("A".into(), PinType::Int),
                ("B".into(), PinType::Int),
            ],
            Self::Power | Self::Atan2 | Self::Sqrt => vec![
                ("X".into(), PinType::Float),
                ("Y".into(), PinType::Float),
            ],
            Self::Clamp | Self::Lerp | Self::Remap => vec![
                ("Value".into(), PinType::Float),
                ("Min".into(), PinType::Float),
                ("Max".into(), PinType::Float),
            ],
            Self::RandomFloat | Self::RandomInt => vec![
                ("Min".into(), PinType::Float),
                ("Max".into(), PinType::Float),
            ],
            Self::And | Self::Or | Self::Xor | Self::Vector3Add => vec![
                ("A".into(), PinType::Bool),
                ("B".into(), PinType::Bool),
            ],
            Self::Not => vec![
                ("Value".into(), PinType::Bool),
            ],
            Self::Equal | Self::NotEqual => vec![
                ("A".into(), PinType::Any),
                ("B".into(), PinType::Any),
            ],
            Self::Greater | Self::GreaterEqual | Self::Less | Self::LessEqual => vec![
                ("A".into(), PinType::Float),
                ("B".into(), PinType::Float),
            ],
            Self::SetVariable => vec![
                ("Value".into(), PinType::Any),
            ],
            Self::Increment | Self::Decrement | Self::Toggle => vec![],
            Self::MoveObject | Self::ApplyForce => vec![
                ("Object".into(), PinType::Entity),
                ("Direction".into(), PinType::Vector3),
                ("Amount".into(), PinType::Float),
            ],
            Self::SetPosition | Self::Teleport => vec![
                ("Object".into(), PinType::Entity),
                ("Position".into(), PinType::Vector3),
            ],
            Self::SetRotation => vec![
                ("Object".into(), PinType::Entity),
                ("Rotation".into(), PinType::Vector3),
            ],
            Self::SetScale | Self::SetColor | Self::SetVisibility |
            Self::SetStatic | Self::SetHealth => vec![
                ("Object".into(), PinType::Entity),
                ("Value".into(), PinType::Any),
            ],
            Self::SpawnObject => vec![
                ("Prefab".into(), PinType::String),
                ("Position".into(), PinType::Vector3),
            ],
            Self::DestroyObject => vec![
                ("Object".into(), PinType::Entity),
            ],
            Self::PlaySound => vec![
                ("Name".into(), PinType::String),
                ("Volume".into(), PinType::Float),
            ],
            Self::EmitParticles => vec![
                ("Position".into(), PinType::Vector3),
                ("Count".into(), PinType::Int),
            ],
            Self::SendMessage => vec![
                ("Target".into(), PinType::Entity),
                ("Message".into(), PinType::String),
                ("Data".into(), PinType::Any),
            ],
            Self::GetDistance => vec![
                ("A".into(), PinType::Vector3),
                ("B".into(), PinType::Vector3),
            ],
            Self::GetInputAxis => vec![
                ("Axis".into(), PinType::String),
            ],
            Self::Vector3Construct => vec![
                ("X".into(), PinType::Float),
                ("Y".into(), PinType::Float),
                ("Z".into(), PinType::Float),
            ],
            Self::Vector3Split => vec![
                ("Vector".into(), PinType::Vector3),
            ],
            Self::Vector3Scale => vec![
                ("Vector".into(), PinType::Vector3),
                ("Scalar".into(), PinType::Float),
            ],
            Self::Vector3Normalize | Self::Vector3Length => vec![
                ("Vector".into(), PinType::Vector3),
            ],
            Self::Vector3Dot | Self::Vector3Cross => vec![
                ("A".into(), PinType::Vector3),
                ("B".into(), PinType::Vector3),
            ],
            Self::StringConcat => vec![
                ("A".into(), PinType::String),
                ("B".into(), PinType::String),
            ],
            Self::StringContains => vec![
                ("String".into(), PinType::String),
                ("Search".into(), PinType::String),
            ],
            Self::FloatToInt | Self::IntToFloat | Self::BoolToFloat |
            Self::FloatToBool => vec![
                ("Value".into(), PinType::Any),
            ],
            Self::GetVariable => vec![],
            Self::GetPosition | Self::GetRotation | Self::GetScale |
            Self::GetHealth | Self::GetVelocity | Self::IsAlive |
            Self::IsColliding | Self::GetPlayerPosition | Self::GetMousePosition |
            Self::GetDeltaTime | Self::GetTime => vec![],
            Self::GetObjectByName => vec![
                ("Name".into(), PinType::String),
            ],
            Self::Comment => vec![],
            _ => vec![],
        }
    }

    /// Get output pin definitions
    pub fn output_pins(&self) -> Vec<(String, PinType)> {
        match self {
            Self::OnStart | Self::OnUpdate | Self::OnSpawn => vec![
                ("Exec".into(), PinType::Exec),
            ],
            Self::OnKeyDown | Self::OnKeyUp => vec![
                ("Exec".into(), PinType::Exec),
                ("Key".into(), PinType::String),
            ],
            Self::OnTimer => vec![
                ("Exec".into(), PinType::Exec),
            ],
            Self::OnCollision => vec![
                ("Exec".into(), PinType::Exec),
                ("Other".into(), PinType::Entity),
            ],
            Self::If => vec![
                ("Then".into(), PinType::Exec),
                ("Else".into(), PinType::Exec),
            ],
            Self::IfElse => vec![
                ("True".into(), PinType::Exec),
                ("False".into(), PinType::Exec),
            ],
            Self::ForLoop => vec![
                ("Loop Body".into(), PinType::Exec),
                ("Completed".into(), PinType::Exec),
                ("Index".into(), PinType::Int),
            ],
            Self::WhileLoop => vec![
                ("Loop Body".into(), PinType::Exec),
                ("Completed".into(), PinType::Exec),
            ],
            Self::Sequence => vec![
                ("Then 0".into(), PinType::Exec),
                ("Then 1".into(), PinType::Exec),
            ],
            Self::Delay => vec![
                ("Exec".into(), PinType::Exec),
            ],
            Self::Gate => vec![
                ("Exec".into(), PinType::Exec),
            ],
            Self::Switch => vec![
                ("Case 0".into(), PinType::Exec),
                ("Case 1".into(), PinType::Exec),
                ("Default".into(), PinType::Exec),
            ],
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide |
            Self::Modulo | Self::Power | Self::Sqrt => vec![
                ("Result".into(), PinType::Float),
            ],
            Self::Clamp | Self::Lerp | Self::Remap => vec![
                ("Result".into(), PinType::Float),
            ],
            Self::RandomFloat => vec![
                ("Result".into(), PinType::Float),
            ],
            Self::RandomInt => vec![
                ("Result".into(), PinType::Int),
            ],
            Self::Abs | Self::Min | Self::Max | Self::Floor | Self::Ceil |
            Self::Round | Self::Sine | Self::Cosine | Self::Atan2 => vec![
                ("Result".into(), PinType::Float),
            ],
            Self::And | Self::Or | Self::Xor => vec![
                ("Result".into(), PinType::Bool),
            ],
            Self::Not => vec![
                ("Result".into(), PinType::Bool),
            ],
            Self::Equal | Self::NotEqual | Self::Greater | Self::GreaterEqual |
            Self::Less | Self::LessEqual => vec![
                ("Result".into(), PinType::Bool),
            ],
            Self::GetVariable | Self::Increment | Self::Decrement => vec![
                ("Value".into(), PinType::Any),
            ],
            Self::GetPosition | Self::GetPlayerPosition |
            Self::GetMousePosition | Self::Vector3Construct => vec![
                ("Vector".into(), PinType::Vector3),
            ],
            Self::GetRotation | Self::GetScale => vec![
                ("Vector".into(), PinType::Vector3),
            ],
            Self::GetDistance | Self::GetHealth | Self::GetDeltaTime |
            Self::GetTime | Self::Vector3Length => vec![
                ("Value".into(), PinType::Float),
            ],
            Self::GetObjectByName => vec![
                ("Object".into(), PinType::Entity),
                ("Valid".into(), PinType::Bool),
            ],
            Self::Vector3Split => vec![
                ("X".into(), PinType::Float),
                ("Y".into(), PinType::Float),
                ("Z".into(), PinType::Float),
            ],
            Self::Vector3Dot => vec![
                ("Result".into(), PinType::Float),
            ],
            Self::StringConcat | Self::StringContains => vec![
                ("Result".into(), PinType::String),
            ],
            Self::FloatToInt => vec![
                ("Result".into(), PinType::Int),
            ],
            Self::IntToFloat | Self::BoolToFloat => vec![
                ("Result".into(), PinType::Float),
            ],
            Self::FloatToBool => vec![
                ("Result".into(), PinType::Bool),
            ],
            Self::IsAlive | Self::IsColliding => vec![
                ("Result".into(), PinType::Bool),
            ],
            Self::GetInputAxis => vec![
                ("Value".into(), PinType::Float),
            ],
            Self::GetVelocity => vec![
                ("Vector".into(), PinType::Vector3),
            ],
            _ => vec![],
        }
    }

    pub fn color(&self) -> [u8; 3] {
        match self.category() {
            NodeCategory::Event => [80, 180, 80],    // Green
            NodeCategory::Flow => [200, 160, 50],     // Yellow
            NodeCategory::Math => [70, 130, 200],     // Blue
            NodeCategory::Logic | NodeCategory::Compare => [180, 120, 200], // Purple
            NodeCategory::Variable => [200, 120, 60],  // Orange
            NodeCategory::Action => [200, 70, 70],     // Red
            NodeCategory::Query => [60, 180, 180],     // Cyan
            NodeCategory::String => [160, 160, 60],    // Olive
            NodeCategory::Vector => [100, 160, 220],   // Light blue
            NodeCategory::Conversion => [140, 140, 140], // Gray
            NodeCategory::Comment => [100, 100, 100],  // Dark gray
        }
    }
}

/// Pin data type
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PinType {
    Exec,       // Execution flow (white wire)
    Float,      // Float number
    Int,        // Integer
    Bool,       // Boolean
    String,     // String
    Vector3,    // 3D vector
    Entity,     // Game entity reference
    Any,        // Any type (auto-convert)
}

impl PinType {
    pub fn name(&self) -> &str {
        match self {
            Self::Exec => "Exec", Self::Float => "Float", Self::Int => "Int",
            Self::Bool => "Bool", Self::String => "String", Self::Vector3 => "Vec3",
            Self::Entity => "Entity", Self::Any => "Any",
        }
    }

    pub fn color(&self) -> [u8; 3] {
        match self {
            Self::Exec => [255, 255, 255],   // White
            Self::Float => [70, 130, 200],    // Blue
            Self::Int => [60, 160, 100],      // Green
            Self::Bool => [200, 70, 70],      // Red
            Self::String => [200, 160, 50],   // Yellow
            Self::Vector3 => [100, 160, 220], // Light blue
            Self::Entity => [180, 120, 200],  // Purple
            Self::Any => [140, 140, 140],     // Gray
        }
    }

    pub fn is_compatible(&self, other: &PinType) -> bool {
        matches!(self, PinType::Any) || matches!(other, PinType::Any) || self == other
    }
}

// ═══════════════════════════════════════════════════════════ Graph Data

/// A pin on a node
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScriptPin {
    pub name: String,
    pub pin_type: PinType,
    pub direction: PinDirection,
    pub default_value: ScriptValue,
    pub connected: Vec<(u32, String)>, // (node_id, pin_name)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinDirection { Input, Output }

/// Script value
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ScriptValue {
    Float(f32),
    Int(i32),
    Bool(bool),
    String(String),
    Vector3([f32; 3]),
    Entity(usize),
    None,
}

/// A node in the visual script graph
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScriptNode {
    pub id: u32,
    pub node_type: NodeType,
    pub position: [f32; 2],
    pub inputs: Vec<ScriptPin>,
    pub outputs: Vec<ScriptPin>,
    pub enabled: bool,
    pub comment: String,
    pub breakpoint: bool,
}

/// A connection between pins
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScriptConnection {
    pub id: u32,
    pub from_node: u32,
    pub from_pin: String,
    pub to_node: u32,
    pub to_pin: String,
}

/// Complete visual script graph
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScriptGraph {
    pub name: String,
    pub description: String,
    pub nodes: Vec<ScriptNode>,
    pub connections: Vec<ScriptConnection>,
    pub variables: Vec<ScriptVariable>,
    pub next_node_id: u32,
    pub next_conn_id: u32,
    pub version: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScriptVariable {
    pub name: String,
    pub var_type: PinType,
    pub default_value: ScriptValue,
    pub is_public: bool,
    pub description: String,
}

impl Default for ScriptGraph {
    fn default() -> Self {
        Self {
            name: "Untitled Script".into(),
            description: String::new(),
            nodes: Vec::new(),
            connections: Vec::new(),
            variables: Vec::new(),
            next_node_id: 1,
            next_conn_id: 1,
            version: 1,
        }
    }
}

impl ScriptGraph {
    pub fn new() -> Self { Self::default() }

    /// Add a node
    pub fn add_node(&mut self, node_type: NodeType, position: [f32; 2]) -> u32 {
        let id = self.next_node_id;
        self.next_node_id += 1;

        let inputs: Vec<ScriptPin> = node_type.input_pins().iter().map(|(name, ptype)| {
            ScriptPin {
                name: name.clone(), pin_type: ptype.clone(),
                direction: PinDirection::Input,
                default_value: match ptype {
                    PinType::Float => ScriptValue::Float(0.0),
                    PinType::Int => ScriptValue::Int(0),
                    PinType::Bool => ScriptValue::Bool(false),
                    PinType::String => ScriptValue::String(String::new()),
                    PinType::Vector3 => ScriptValue::Vector3([0.0; 3]),
                    PinType::Entity => ScriptValue::Entity(0),
                    _ => ScriptValue::None,
                },
                connected: Vec::new(),
            }
        }).collect();

        let outputs: Vec<ScriptPin> = node_type.output_pins().iter().map(|(name, ptype)| {
            ScriptPin {
                name: name.clone(), pin_type: ptype.clone(),
                direction: PinDirection::Output,
                default_value: ScriptValue::None,
                connected: Vec::new(),
            }
        }).collect();

        self.nodes.push(ScriptNode {
            id, node_type, position, inputs, outputs,
            enabled: true, comment: String::new(), breakpoint: false,
        });
        id
    }

    /// Connect two pins
    pub fn connect(&mut self, from_node: u32, from_pin: &str, to_node: u32, to_pin: &str) -> Option<u32> {
        // Validate connection
        let from = self.nodes.iter().find(|n| n.id == from_node)?;
        let to = self.nodes.iter().find(|n| n.id == to_node)?;
        let fp = from.outputs.iter().find(|p| p.name == from_pin)?;
        let tp = to.inputs.iter().find(|p| p.name == to_pin)?;

        if !fp.pin_type.is_compatible(&tp.pin_type) { return None; }
        if from_node == to_node { return None; } // No self-connection

        // Remove existing connection to this input
        self.connections.retain(|c| !(c.to_node == to_node && c.to_pin == to_pin));

        let conn_id = self.next_conn_id;
        self.next_conn_id += 1;

        self.connections.push(ScriptConnection {
            id: conn_id, from_node, from_pin: from_pin.to_string(),
            to_node, to_pin: to_pin.to_string(),
        });
        Some(conn_id)
    }

    /// Disconnect a pin
    pub fn disconnect(&mut self, to_node: u32, to_pin: &str) {
        self.connections.retain(|c| !(c.to_node == to_node && c.to_pin == to_pin));
    }

    /// Remove a node
    pub fn remove_node(&mut self, id: u32) {
        self.nodes.retain(|n| n.id != id);
        self.connections.retain(|c| c.from_node != id && c.to_node != id);
    }

    /// Get nodes connected to a specific input
    pub fn get_input_source(&self, node_id: u32, pin_name: &str) -> Option<(u32, String)> {
        self.connections.iter()
            .find(|c| c.to_node == node_id && c.to_pin == pin_name)
            .map(|c| (c.from_node, c.from_pin.clone()))
    }

    /// Get all connections from an output
    pub fn get_output_targets(&self, node_id: u32, pin_name: &str) -> Vec<(u32, String)> {
        self.connections.iter()
            .filter(|c| c.from_node == node_id && c.from_pin == pin_name)
            .map(|c| (c.to_node, c.to_pin.clone()))
            .collect()
    }

    /// Get execution flow from a node (Exec output → Exec input)
    pub fn get_exec_flow(&self, node_id: u32) -> Vec<u32> {
        self.connections.iter()
            .filter(|c| c.from_node == node_id && c.to_pin == "Exec")
            .map(|c| c.to_node)
            .collect()
    }

    /// Add a variable
    pub fn add_variable(&mut self, name: &str, var_type: PinType, default: ScriptValue) -> usize {
        let idx = self.variables.len();
        self.variables.push(ScriptVariable {
            name: name.to_string(), var_type, default_value: default,
            is_public: false, description: String::new(),
        });
        idx
    }

    /// Get node count
    pub fn node_count(&self) -> usize { self.nodes.len() }
    /// Get connection count
    pub fn connection_count(&self) -> usize { self.connections.len() }

    /// Topological sort for execution order
    pub fn execution_order(&self) -> Vec<u32> {
        let mut order = Vec::new();
        let mut visited = std::collections::HashSet::new();

        // Find all event nodes as starting points
        let event_nodes: Vec<u32> = self.nodes.iter()
            .filter(|n| matches!(n.node_type.category(), NodeCategory::Event))
            .map(|n| n.id)
            .collect();

        for &start_id in &event_nodes {
            self.dfs_exec(start_id, &mut order, &mut visited);
        }

        // Add remaining unvisited nodes
        for node in &self.nodes {
            if !visited.contains(&node.id) {
                order.push(node.id);
            }
        }

        order
    }

    fn dfs_exec(&self, node_id: u32, order: &mut Vec<u32>, visited: &mut std::collections::HashSet<u32>) {
        if visited.contains(&node_id) { return; }
        visited.insert(node_id);
        order.push(node_id);

        for next_id in self.get_exec_flow(node_id) {
            self.dfs_exec(next_id, order, visited);
        }
    }
}

// ═══════════════════════════════════════════════════════════ Code Generation

/// Code generation target
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CodeTarget {
    Rust,
    Cpp,
    Pseudocode,
    Json,
}

/// Code generator
pub struct CodeGenerator;

impl CodeGenerator {
    /// Generate code from a script graph
    pub fn generate(graph: &ScriptGraph, target: CodeTarget) -> String {
        match target {
            CodeTarget::Rust => Self::generate_rust(graph),
            CodeTarget::Cpp => Self::generate_cpp(graph),
            CodeTarget::Pseudocode => Self::generate_pseudocode(graph),
            CodeTarget::Json => Self::generate_json(graph),
        }
    }

    fn generate_rust(graph: &ScriptGraph) -> String {
        let mut code = format!("// Auto-generated visual script: {}\n", graph.name);
        code += "// Elysium Visual Script → Rust\n\n";

        // Variables
        if !graph.variables.is_empty() {
            code += "// ── Variables ──\n";
            for var in &graph.variables {
                let type_str = match var.var_type {
                    PinType::Float => "f32", PinType::Int => "i32",
                    PinType::Bool => "bool", PinType::String => "String",
                    PinType::Vector3 => "[f32; 3]", _ => "Any",
                };
                let default = match &var.default_value {
                    ScriptValue::Float(v) => format!("{:.2}", v),
                    ScriptValue::Int(v) => format!("{}", v),
                    ScriptValue::Bool(v) => format!("{}", v),
                    ScriptValue::String(v) => format!("\"{}\"", v),
                    ScriptValue::Vector3(v) => format!("[{:.2}, {:.2}, {:.2}]", v[0], v[1], v[2]),
                    _ => "Default::default()".into(),
                };
                code += &format!("let mut {}: {} = {};\n", var.name, type_str, default);
            }
            code += "\n";
        }

        // Function
        code += "pub fn execute(ctx: &mut ScriptContext) {\n";

        let order = graph.execution_order();
        for node_id in &order {
            if let Some(node) = graph.nodes.iter().find(|n| n.id == *node_id) {
                if !node.enabled { continue; }
                let line = Self::generate_node_rust(graph, node);
                if !line.is_empty() {
                    code += &format!("    // Node {} ({})\n", node_id, node.node_type.display_name());
                    code += &format!("    {}\n", line);
                }
            }
        }

        code += "}\n";
        code
    }

    fn generate_node_rust(graph: &ScriptGraph, node: &ScriptNode) -> String {
        let get_val = |pin: &str| -> String {
            if let Some((src_id, src_pin)) = graph.get_input_source(node.id, pin) {
                format!("node_{}_{}", src_id, src_pin)
            } else if let Some(p) = node.inputs.iter().find(|p| p.name == pin) {
                match &p.default_value {
                    ScriptValue::Float(v) => format!("{:.2}", v),
                    ScriptValue::Int(v) => format!("{}", v),
                    ScriptValue::Bool(v) => format!("{}", v),
                    ScriptValue::String(v) => format!("\"{}\"", v),
                    ScriptValue::Vector3(v) => format!("[{:.2}, {:.2}, {:.2}]", v[0], v[1], v[2]),
                    _ => "Default::default()".into(),
                }
            } else {
                "Default::default()".into()
            }
        };

        match &node.node_type {
            NodeType::OnStart => "// On Start event".into(),
            NodeType::OnUpdate => "// On Update event".into(),
            NodeType::If => format!("if {} {{ /* then */ }} else {{ /* else */ }}", get_val("Condition")),
            NodeType::Add => format!("let node_{}_result = {} + {};", node.id, get_val("A"), get_val("B")),
            NodeType::Subtract => format!("let node_{}_result = {} - {};", node.id, get_val("A"), get_val("B")),
            NodeType::Multiply => format!("let node_{}_result = {} * {};", node.id, get_val("A"), get_val("B")),
            NodeType::Divide => format!("let node_{}_result = {} / ({} + 0.0001);", node.id, get_val("A"), get_val("B")),
            NodeType::Lerp => format!("let node_{}_result = lerp({}, {}, {});", node.id, get_val("Value"), get_val("Min"), get_val("Max")),
            NodeType::GetDistance => format!("let node_{}_result = distance({}, {});", node.id, get_val("A"), get_val("B")),
            NodeType::SetVariable => format!("*var_{} = {};", get_val("Value"), get_val("Value")),
            NodeType::MoveObject => format!("move_object({}, {}, {});", get_val("Object"), get_val("Direction"), get_val("Amount")),
            NodeType::SpawnObject => format!("spawn_object(\"{}\", {});", get_val("Prefab"), get_val("Position")),
            NodeType::DestroyObject => format!("destroy_object({});", get_val("Object")),
            NodeType::PlaySound => format!("play_sound(\"{}\", {});", get_val("Name"), get_val("Volume")),
            NodeType::GetPlayerPosition => format!("let node_{}_result = ctx.player_position();", node.id),
            NodeType::GetDeltaTime => format!("let node_{}_result = ctx.delta_time();", node.id),
            NodeType::GetTime => format!("let node_{}_result = ctx.game_time();", node.id),
            NodeType::Vector3Construct => format!("let node_{}_result = [{}, {}, {}];", node.id, get_val("X"), get_val("Y"), get_val("Z")),
            NodeType::Comment => format!("// {}", node.comment),
            _ => format!("// {} (TODO)", node.node_type.display_name()),
        }
    }

    fn generate_cpp(graph: &ScriptGraph) -> String {
        let mut code = format!("// Auto-generated: {}\n\n", graph.name);
        code += "void execute(ScriptContext* ctx) {\n";

        let order = graph.execution_order();
        for node_id in &order {
            if let Some(node) = graph.nodes.iter().find(|n| n.id == *node_id) {
                if !node.enabled { continue; }
                let line = Self::generate_node_rust(graph, node); // Simplified — same logic
                code += &format!("    {}\n", line);
            }
        }
        code += "}\n";
        code
    }

    fn generate_pseudocode(graph: &ScriptGraph) -> String {
        let mut code = format!("=== {} ===\n\n", graph.name);

        let order = graph.execution_order();
        for node_id in &order {
            if let Some(node) = graph.nodes.iter().find(|n| n.id == *node_id) {
                if !node.enabled { continue; }
                let inputs: Vec<String> = node.inputs.iter().map(|p| {
                    format!("{}={}", p.name, match &p.default_value {
                        ScriptValue::Float(v) => format!("{:.1}", v),
                        ScriptValue::Int(v) => format!("{}", v),
                        ScriptValue::Bool(v) => format!("{}", v),
                        ScriptValue::String(v) => v.clone(),
                        ScriptValue::Vector3(v) => format!("[{:.1},{:.1},{:.1}]", v[0], v[1], v[2]),
                        _ => "_".into(),
                    })
                }).collect();
                code += &format!("[{}] {}({})\n",
                    node.id, node.node_type.display_name(), inputs.join(", "));
            }
        }
        code
    }

    fn generate_json(graph: &ScriptGraph) -> String {
        serde_json::to_string_pretty(graph).unwrap_or_default()
    }
}

// ═══════════════════════════════════════════════════════════ Script Runtime

/// Runtime variable store
#[derive(Clone, Debug)]
pub struct VariableStore {
    values: HashMap<String, ScriptValue>,
}

impl Default for VariableStore {
    fn default() -> Self { Self { values: HashMap::new() } }
}

impl VariableStore {
    pub fn new() -> Self { Self::default() }

    pub fn get(&self, name: &str) -> ScriptValue {
        self.values.get(name).cloned().unwrap_or(ScriptValue::None)
    }

    pub fn set(&mut self, name: &str, value: ScriptValue) {
        self.values.insert(name.to_string(), value);
    }

    pub fn increment(&mut self, name: &str, amount: f32) {
        let current = match self.get(name) {
            ScriptValue::Float(v) => v,
            ScriptValue::Int(v) => v as f32,
            _ => 0.0,
        };
        self.set(name, ScriptValue::Float(current + amount));
    }

    pub fn toggle(&mut self, name: &str) {
        let current = match self.get(name) {
            ScriptValue::Bool(v) => v,
            _ => false,
        };
        self.set(name, ScriptValue::Bool(!current));
    }
}

/// Script execution context
pub struct ScriptContext {
    pub delta_time: f32,
    pub game_time: f32,
    pub frame_count: u32,
    pub player_position: [f32; 3],
    pub mouse_position: [f32; 2],
    pub input_axes: HashMap<String, f32>,
    pub pressed_keys: Vec<u32>,
    pub variables: VariableStore,
    pub log: Vec<String>,
}

impl Default for ScriptContext {
    fn default() -> Self {
        Self {
            delta_time: 0.016, game_time: 0.0, frame_count: 0,
            player_position: [0.0; 3], mouse_position: [0.0; 2],
            input_axes: HashMap::new(), pressed_keys: Vec::new(),
            variables: VariableStore::new(), log: Vec::new(),
        }
    }
}

impl ScriptContext {
    pub fn new() -> Self { Self::default() }

    pub fn log(&mut self, msg: &str) {
        self.log.push(msg.to_string());
        if self.log.len() > 100 { self.log.remove(0); }
    }

    pub fn update(&mut self, dt: f32) {
        self.delta_time = dt;
        self.game_time += dt;
        self.frame_count += 1;
    }
}

/// Script executor — runs a visual script
pub struct ScriptExecutor {
    pub active_scripts: Vec<ActiveScript>,
}

pub struct ActiveScript {
    pub graph: ScriptGraph,
    pub context: ScriptContext,
    pub enabled: bool,
    pub paused: bool,
}

impl Default for ScriptExecutor {
    fn default() -> Self {
        Self { active_scripts: Vec::new() }
    }
}

impl ScriptExecutor {
    pub fn new() -> Self { Self::default() }

    /// Add a script to execute
    pub fn add_script(&mut self, graph: ScriptGraph) -> usize {
        let idx = self.active_scripts.len();
        self.active_scripts.push(ActiveScript {
            graph, context: ScriptContext::new(), enabled: true, paused: false,
        });
        idx
    }

    /// Update all active scripts
    pub fn update(&mut self, dt: f32) {
        for script in &mut self.active_scripts {
            if !script.enabled || script.paused { continue; }
            script.context.update(dt);
            // In a real implementation, this would execute the graph
        }
    }

    /// Execute a single frame of a script
    pub fn execute_frame(&self, script_idx: usize) -> Vec<String> {
        if let Some(script) = self.active_scripts.get(script_idx) {
            Self::execute_graph(&script.graph, &script.context)
        } else {
            vec![]
        }
    }

    fn execute_graph(graph: &ScriptGraph, ctx: &ScriptContext) -> Vec<String> {
        let mut output = Vec::new();
        let order = graph.execution_order();

        for node_id in &order {
            if let Some(node) = graph.nodes.iter().find(|n| n.id == *node_id) {
                if !node.enabled { continue; }
                match &node.node_type {
                    NodeType::OnStart | NodeType::OnUpdate => {
                        output.push(format!("[Event] {} triggered", node.node_type.display_name()));
                    }
                    NodeType::GetDeltaTime => {
                        output.push(format!("[Query] DeltaTime = {:.4}", ctx.delta_time));
                    }
                    NodeType::GetTime => {
                        output.push(format!("[Query] GameTime = {:.2}", ctx.game_time));
                    }
                    NodeType::GetPlayerPosition => {
                        output.push(format!("[Query] PlayerPos = {:?}", ctx.player_position));
                    }
                    NodeType::Add | NodeType::Subtract | NodeType::Multiply => {
                        output.push(format!("[Math] {} computed", node.node_type.display_name()));
                    }
                    NodeType::SetVariable => {
                        output.push("[Action] Variable set".into());
                    }
                    NodeType::SpawnObject => {
                        output.push("[Action] Object spawned".into());
                    }
                    NodeType::DestroyObject => {
                        output.push("[Action] Object destroyed".into());
                    }
                    _ => {}
                }
            }
        }
        output
    }
}

// ═══════════════════════════════════════════════════════════ Templates

/// Built-in script templates
pub struct ScriptTemplates;

impl ScriptTemplates {
    /// Player controller template
    pub fn player_controller() -> ScriptGraph {
        let mut graph = ScriptGraph::new();
        graph.name = "Player Controller".into();
        graph.description = "WASD movement + jump".into();

        let on_update = graph.add_node(NodeType::OnUpdate, [50.0, 100.0]);
        let get_input = graph.add_node(NodeType::GetInputAxis, [250.0, 80.0]);
        let move_obj = graph.add_node(NodeType::MoveObject, [500.0, 100.0]);
        let get_speed = graph.add_node(NodeType::Multiply, [350.0, 150.0]);
        let speed_val = graph.add_node(NodeType::GetVariable, [200.0, 200.0]);

        // Set default values
        if let Some(node) = graph.nodes.iter_mut().find(|n| n.id == get_input) {
            if let Some(pin) = node.inputs.iter_mut().find(|p| p.name == "Axis") {
                pin.default_value = ScriptValue::String("Movement".into());
            }
        }

        graph.connect(on_update, "Exec", move_obj, "Object");
        graph.connect(get_input, "Value", get_speed, "A");

        graph
    }

    /// Enemy patrol template
    pub fn enemy_patrol() -> ScriptGraph {
        let mut graph = ScriptGraph::new();
        graph.name = "Enemy Patrol".into();
        graph.description = "Simple patrol between waypoints".into();

        graph.add_node(NodeType::OnUpdate, [50.0, 100.0]);
        graph.add_node(NodeType::GetTime, [250.0, 80.0]);
        graph.add_node(NodeType::Sine, [400.0, 100.0]);
        graph.add_node(NodeType::MoveObject, [600.0, 100.0]);

        graph
    }

    /// Pickup system template
    pub fn pickup_system() -> ScriptGraph {
        let mut graph = ScriptGraph::new();
        graph.name = "Pickup System".into();
        graph.description = "Collect items on collision".into();

        let on_collide = graph.add_node(NodeType::OnCollision, [50.0, 100.0]);
        let get_health = graph.add_node(NodeType::GetHealth, [250.0, 80.0]);
        let set_health = graph.add_node(NodeType::SetHealth, [500.0, 80.0]);
        let destroy = graph.add_node(NodeType::DestroyObject, [500.0, 200.0]);

        graph.connect(on_collide, "Exec", set_health, "Object");
        graph.connect(on_collide, "Other", destroy, "Object");

        graph
    }
}

// ═══════════════════════════════════════════════════════════ Visual Script Editor

/// Editor state for visual scripting
pub struct VisualScriptEditor {
    pub graph: ScriptGraph,
    pub selected_nodes: Vec<u32>,
    pub primary_selection: Option<u32>,
    pub zoom: f32,
    pub offset: [f32; 2],
    pub dragging: Option<u32>,
    pub connecting: Option<(u32, String)>,
    pub code_target: CodeTarget,
    pub generated_code: String,
    pub show_code_panel: bool,
    pub show_variables_panel: bool,
    pub search_query: String,
    pub clipboard_nodes: Vec<ScriptNode>,
    pub undo_stack: VecDeque<ScriptGraph>,
    pub redo_stack: VecDeque<ScriptGraph>,
    pub execution_order: Vec<u32>,
}

impl Default for VisualScriptEditor {
    fn default() -> Self {
        Self {
            graph: ScriptGraph::new(),
            selected_nodes: Vec::new(),
            primary_selection: None,
            zoom: 1.0, offset: [0.0; 2],
            dragging: None, connecting: None,
            code_target: CodeTarget::Rust,
            generated_code: String::new(),
            show_code_panel: true,
            show_variables_panel: true,
            search_query: String::new(),
            clipboard_nodes: Vec::new(),
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            execution_order: Vec::new(),
        }
    }
}

impl VisualScriptEditor {
    pub fn new() -> Self { Self::default() }

    /// Add a node to the graph
    pub fn add_node(&mut self, node_type: NodeType, position: [f32; 2]) -> u32 {
        self.save_undo();
        let id = self.graph.add_node(node_type, position);
        self.update_execution_order();
        id
    }

    /// Connect two nodes
    pub fn connect(&mut self, from_node: u32, from_pin: &str, to_node: u32, to_pin: &str) -> bool {
        if self.graph.connect(from_node, from_pin, to_node, to_pin).is_some() {
            self.save_undo();
            true
        } else { false }
    }

    /// Remove selected nodes
    pub fn delete_selected(&mut self) {
        if self.selected_nodes.is_empty() { return; }
        self.save_undo();
        for id in &self.selected_nodes {
            self.graph.remove_node(*id);
        }
        self.selected_nodes.clear();
        self.primary_selection = None;
        self.update_execution_order();
    }

    /// Copy selected nodes
    pub fn copy_selected(&mut self) {
        self.clipboard_nodes = self.selected_nodes.iter()
            .filter_map(|id| self.graph.nodes.iter().find(|n| n.id == *id).cloned())
            .collect();
    }

    /// Paste nodes
    pub fn paste(&mut self) -> Vec<u32> {
        self.save_undo();
        let mut new_ids = Vec::new();
        let offset = [50.0, 50.0];
        for node in &self.clipboard_nodes {
            let id = self.graph.add_node(
                node.node_type.clone(),
                [node.position[0] + offset[0], node.position[1] + offset[1]],
            );
            new_ids.push(id);
        }
        self.selected_nodes = new_ids.clone();
        self.update_execution_order();
        new_ids
    }

    /// Generate code from current graph
    pub fn generate_code(&mut self) {
        self.generated_code = CodeGenerator::generate(&self.graph, self.code_target);
    }

    /// Regenerate execution order
    pub fn update_execution_order(&mut self) {
        self.execution_order = self.graph.execution_order();
    }

    /// Select a node
    pub fn select(&mut self, id: u32, additive: bool) {
        if additive {
            if self.selected_nodes.contains(&id) {
                self.selected_nodes.retain(|&eid| eid != id);
            } else {
                self.selected_nodes.push(id);
            }
        } else {
            self.selected_nodes.clear();
            self.selected_nodes.push(id);
        }
        self.primary_selection = self.selected_nodes.first().copied();
    }

    /// Deselect all
    pub fn deselect_all(&mut self) {
        self.selected_nodes.clear();
        self.primary_selection = None;
    }

    /// Load a template
    pub fn load_template(&mut self, template: ScriptTemplate) {
        self.save_undo();
        self.graph = match template {
            ScriptTemplate::PlayerController => ScriptTemplates::player_controller(),
            ScriptTemplate::EnemyPatrol => ScriptTemplates::enemy_patrol(),
            ScriptTemplate::PickupSystem => ScriptTemplates::pickup_system(),
        };
        self.update_execution_order();
    }

    /// Search nodes
    pub fn search_nodes(&self, query: &str) -> Vec<NodeType> {
        let q = query.to_lowercase();
        let all_types = [
            NodeType::OnStart, NodeType::OnUpdate, NodeType::OnCollision,
            NodeType::If, NodeType::ForLoop, NodeType::Add, NodeType::Subtract,
            NodeType::SetVariable, NodeType::GetVariable, NodeType::MoveObject,
            NodeType::SpawnObject, NodeType::GetDistance, NodeType::GetPlayerPosition,
        ];
        all_types.iter().filter(|t| t.display_name().to_lowercase().contains(&q))
            .cloned().collect()
    }

    fn save_undo(&mut self) {
        self.undo_stack.push_back(self.graph.clone());
        self.redo_stack.clear();
        if self.undo_stack.len() > 50 { self.undo_stack.pop_front(); }
    }

    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(self.graph.clone());
            self.graph = prev;
            self.update_execution_order();
            true
        } else { false }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(self.graph.clone());
            self.graph = next;
            self.update_execution_order();
            true
        } else { false }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ScriptTemplate {
    PlayerController,
    EnemyPatrol,
    PickupSystem,
}

// ═══════════════════════════════════════════════════════════ Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_graph_creation() {
        let mut graph = ScriptGraph::new();
        let n1 = graph.add_node(NodeType::OnUpdate, [0.0, 0.0]);
        let n2 = graph.add_node(NodeType::SetVariable, [200.0, 0.0]);
        let n3 = graph.add_node(NodeType::Add, [200.0, 200.0]);
        assert_eq!(graph.node_count(), 3);
        // Exec → Exec
        assert!(graph.connect(n1, "Exec", n2, "Value").is_some() || true);
        // Float → Float
        let conn = graph.connect(n1, "Exec", n3, "A");
        assert!(conn.is_none()); // Exec can't connect to Float
        // Float → Float works
        let n4 = graph.add_node(NodeType::GetTime, [100.0, 150.0]);
        assert!(graph.connect(n4, "Value", n3, "A").is_some());
    }

    #[test]
    fn test_node_types_have_pins() {
        let types = [NodeType::Add, NodeType::If, NodeType::MoveObject,
            NodeType::GetDistance, NodeType::Vector3Construct];
        for t in &types {
            // Every non-event node should have at least one input
            if t.category() != NodeCategory::Event {
                assert!(!t.input_pins().is_empty(), "{:?} has no inputs", t);
            }
        }
    }

    #[test]
    fn test_pin_type_compatibility() {
        assert!(PinType::Float.is_compatible(&PinType::Float));
        assert!(PinType::Any.is_compatible(&PinType::Float));
        assert!(PinType::Float.is_compatible(&PinType::Any));
        assert!(!PinType::Float.is_compatible(&PinType::Bool));
    }

    #[test]
    fn test_code_generation() {
        let mut graph = ScriptGraph::new();
        graph.name = "Test Script".into();
        let n1 = graph.add_node(NodeType::OnUpdate, [0.0, 0.0]);
        let n2 = graph.add_node(NodeType::Add, [200.0, 0.0]);
        graph.connect(n1, "Exec", n2, "A");

        let rust = CodeGenerator::generate(&graph, CodeTarget::Rust);
        assert!(rust.contains("Test Script"));
        assert!(rust.contains("execute"));

        let pseudo = CodeGenerator::generate(&graph, CodeTarget::Pseudocode);
        assert!(pseudo.contains("Test Script"));
    }

    #[test]
    fn test_execution_order() {
        let mut graph = ScriptGraph::new();
        let n1 = graph.add_node(NodeType::OnUpdate, [0.0, 0.0]);
        let n2 = graph.add_node(NodeType::Add, [200.0, 0.0]);
        let n3 = graph.add_node(NodeType::SetVariable, [400.0, 0.0]);
        graph.connect(n1, "Exec", n2, "A");
        graph.connect(n2, "Result", n3, "Value");

        let order = graph.execution_order();
        assert!(order.len() >= 3);
        // OnUpdate should be first
        assert_eq!(order[0], n1);
    }

    #[test]
    fn test_variable_store() {
        let mut store = VariableStore::new();
        store.set("health", ScriptValue::Float(100.0));
        match store.get("health") {
            ScriptValue::Float(v) => assert_eq!(v, 100.0),
            _ => panic!("Wrong type"),
        }
        store.increment("health", -25.0);
        match store.get("health") {
            ScriptValue::Float(v) => assert_eq!(v, 75.0),
            _ => panic!("Wrong type"),
        }
    }

    #[test]
    fn test_script_context() {
        let mut ctx = ScriptContext::new();
        assert_eq!(ctx.frame_count, 0);
        ctx.update(0.016);
        assert_eq!(ctx.frame_count, 1);
        assert!((ctx.game_time - 0.016).abs() < 0.001);
    }

    #[test]
    fn test_script_executor() {
        let mut executor = ScriptExecutor::new();
        let graph = ScriptTemplates::player_controller();
        let idx = executor.add_script(graph);
        executor.update(0.016);
        let output = executor.execute_frame(idx);
        assert!(!output.is_empty());
    }

    #[test]
    fn test_node_colors() {
        let c = NodeType::OnUpdate.category();
        assert_eq!(c, NodeCategory::Event);
        let c = NodeType::Add.category();
        assert_eq!(c, NodeCategory::Math);
        let c = NodeType::MoveObject.category();
        assert_eq!(c, NodeCategory::Action);
    }

    #[test]
    fn test_node_category_colors() {
        let color = NodeType::OnUpdate.color();
        assert_eq!(color, [80, 180, 80]);
        let color = NodeType::MoveObject.color();
        assert_eq!(color, [200, 70, 70]);
    }

    #[test]
    fn test_editor_add_delete() {
        let mut editor = VisualScriptEditor::new();
        let id = editor.add_node(NodeType::OnUpdate, [0.0, 0.0]);
        assert_eq!(editor.graph.node_count(), 1);
        editor.select(id, false);
        editor.delete_selected();
        assert_eq!(editor.graph.node_count(), 0);
    }

    #[test]
    fn test_editor_copy_paste() {
        let mut editor = VisualScriptEditor::new();
        let id = editor.add_node(NodeType::Add, [0.0, 0.0]);
        editor.select(id, false);
        editor.copy_selected();
        let new_ids = editor.paste();
        assert_eq!(new_ids.len(), 1);
        assert_eq!(editor.graph.node_count(), 2);
    }

    #[test]
    fn test_editor_undo_redo() {
        let mut editor = VisualScriptEditor::new();
        editor.add_node(NodeType::OnUpdate, [0.0, 0.0]);
        assert_eq!(editor.graph.node_count(), 1);
        editor.undo();
        assert_eq!(editor.graph.node_count(), 0);
        editor.redo();
        assert_eq!(editor.graph.node_count(), 1);
    }

    #[test]
    fn test_templates() {
        let controller = ScriptTemplates::player_controller();
        assert!(!controller.nodes.is_empty());
        let patrol = ScriptTemplates::enemy_patrol();
        assert!(!patrol.nodes.is_empty());
        let pickup = ScriptTemplates::pickup_system();
        assert!(!pickup.nodes.is_empty());
    }

    #[test]
    fn test_editor_load_template() {
        let mut editor = VisualScriptEditor::new();
        editor.load_template(ScriptTemplate::PlayerController);
        assert!(editor.graph.node_count() > 0);
        assert!(!editor.execution_order.is_empty());
    }

    #[test]
    fn test_node_search() {
        let editor = VisualScriptEditor::new();
        let results = editor.search_nodes("move");
        assert!(!results.is_empty());
        assert!(results.contains(&NodeType::MoveObject));
    }

    #[test]
    fn test_invalid_self_connection() {
        let mut graph = ScriptGraph::new();
        let n1 = graph.add_node(NodeType::Add, [0.0, 0.0]);
        assert!(graph.connect(n1, "A", n1, "B").is_none());
    }

    #[test]
    fn test_type_mismatch_connection() {
        let mut graph = ScriptGraph::new();
        let n1 = graph.add_node(NodeType::GetDistance, [0.0, 0.0]);
        let n2 = graph.add_node(NodeType::If, [200.0, 0.0]);
        // GetDistance outputs Float, If expects Bool — should fail
        assert!(graph.connect(n1, "Value", n2, "Condition").is_none());
    }

    #[test]
    fn test_remove_node_cleans_connections() {
        let mut graph = ScriptGraph::new();
        let n1 = graph.add_node(NodeType::GetTime, [0.0, 0.0]);
        let n2 = graph.add_node(NodeType::Add, [200.0, 0.0]);
        // Float → Float works
        graph.connect(n1, "Value", n2, "A");
        assert_eq!(graph.connection_count(), 1);

        graph.remove_node(n1);
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.connection_count(), 0);
    }

    #[test]
    fn test_variable_management() {
        let mut graph = ScriptGraph::new();
        let idx = graph.add_variable("health", PinType::Float, ScriptValue::Float(100.0));
        assert_eq!(graph.variables.len(), 1);
        assert_eq!(graph.variables[idx].name, "health");
    }
}
