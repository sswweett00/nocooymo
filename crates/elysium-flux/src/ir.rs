use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrNode {
    Constant(ConstantNode),
    VariableAccess(VariableNode),
    FunctionCall(FunctionCallNode),
    BinaryOperation(BinaryOpNode),
    UnaryOperation(UnaryOpNode),
    Branch(BranchNode),
    Loop(LoopNode),
    Return(ReturnNode),
    Assignment(AssignmentNode),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantNode {
    pub value: IrValue,
    pub id: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableNode {
    pub name: String,
    pub var_type: IrType,
    pub id: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallNode {
    pub function_name: String,
    pub arguments: Vec<NodeId>,
    pub return_type: IrType,
    pub id: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryOpNode {
    pub op: BinaryOp,
    pub left: NodeId,
    pub right: NodeId,
    pub result_type: IrType,
    pub id: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnaryOpNode {
    pub op: UnaryOp,
    pub operand: NodeId,
    pub result_type: IrType,
    pub id: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchNode {
    pub condition: NodeId,
    pub then_block: Vec<NodeId>,
    pub else_block: Vec<NodeId>,
    pub id: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopNode {
    pub condition: NodeId,
    pub body: Vec<NodeId>,
    pub loop_type: LoopType,
    pub id: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnNode {
    pub value: Option<NodeId>,
    pub id: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentNode {
    pub target: NodeId,
    pub source: NodeId,
    pub id: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
    AddressOf,
    Dereference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopType {
    While,
    For,
    ForEach,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrType {
    Void,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    Pointer(Box<IrType>),
    Array(Box<IrType>, usize),
    Struct(StructDefinition),
    Function(Box<FunctionSignature>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructDefinition {
    pub name: String,
    pub fields: Vec<(String, IrType)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub return_type: IrType,
    pub parameters: Vec<IrType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrValue {
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
    String(String),
    Array(Vec<IrValue>),
    Struct(HashMap<String, IrValue>),
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeId(pub u32);

impl NodeId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrFunction {
    pub name: String,
    pub parameters: Vec<(String, IrType)>,
    pub return_type: IrType,
    pub body: Vec<NodeId>,
    pub nodes: HashMap<NodeId, IrNode>,
    pub ssa_form: bool, // SSA formunda olup olmadığı
    pub dominator_tree: Option<DominatorTree>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DominatorTree {
    pub idom: HashMap<NodeId, Option<NodeId>>, // Immediate dominator
    pub dom_frontier: HashMap<NodeId, Vec<NodeId>>, // Dominance frontier
    pub dom_tree: HashMap<NodeId, Vec<NodeId>>, // Dominator tree children
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrModule {
    pub functions: Vec<IrFunction>,
    pub globals: Vec<(String, IrType)>,
    pub structs: Vec<StructDefinition>,
    pub constants: Vec<(String, IrValue)>,
}

impl IrFunction {
    pub fn new(name: String, parameters: Vec<(String, IrType)>, return_type: IrType) -> Self {
        Self {
            name,
            parameters,
            return_type,
            body: Vec::new(),
            nodes: HashMap::new(),
            ssa_form: false,
            dominator_tree: None,
        }
    }

    pub fn add_node(&mut self, node_id: NodeId, node: IrNode) {
        self.nodes.insert(node_id, node);
    }

    pub fn get_node(&self, node_id: NodeId) -> Option<&IrNode> {
        self.nodes.get(&node_id)
    }

    /// SSA formuna dönüştür
    pub fn convert_to_ssa(&mut self) {
        if self.ssa_form {
            return; // Zaten SSA formunda
        }

        // Dominator tree hesapla
        self.dominator_tree = Some(self.compute_dominator_tree());
        
        // Phi node'ları ekle
        self.insert_phi_nodes();
        
        // Variable'ları rename et
        self.rename_variables();
        
        self.ssa_form = true;
    }

    /// Dominator tree hesapla (basit implementasyon)
    fn compute_dominator_tree(&self) -> DominatorTree {
        let mut idom = HashMap::new();
        let dom_frontier = HashMap::new();
        let mut dom_tree = HashMap::new();
        
        // Entry node (ilk node) tüm node'ları dominater
        if let Some(&first_node) = self.body.first() {
            for &node_id in &self.body {
                if node_id != first_node {
                    idom.insert(node_id, Some(first_node));
                } else {
                    idom.insert(node_id, None); // Entry node'ın dominator'ü yok
                }
            }
            
            // Dominator tree children
            for (&node, &dom) in &idom {
                if let Some(dominator) = dom {
                    dom_tree.entry(dominator).or_insert_with(Vec::new).push(node);
                }
            }
        }
        
        DominatorTree {
            idom,
            dom_frontier,
            dom_tree,
        }
    }

    /// Phi node'larını ekle
    fn insert_phi_nodes(&mut self) {
        // Basit implementasyon - her branch point için phi node ekle
        let mut phi_nodes = Vec::new();
        
        for (&_node_id, node) in &self.nodes {
            if let IrNode::Branch(branch) = node {
                // Her branch için phi node'lar ekle
                for &pred in &branch.then_block {
                    // Then branch için phi node
                    let phi_id = self.next_phi_id();
                    let phi_node = IrNode::FunctionCall(FunctionCallNode {
                        function_name: "phi".to_string(),
                        arguments: vec![pred, branch.condition],
                        return_type: IrType::Float32, // Basit varsayım
                        id: phi_id,
                    });
                    phi_nodes.push((phi_id, phi_node));
                }
            }
        }
        
        // Phi node'ları ekle
        for (id, node) in phi_nodes {
            self.nodes.insert(id, node);
        }
    }

    /// Variable'ları SSA formuna göre rename et
    fn rename_variables(&mut self) {
        // Basit implementasyon - her assignment için yeni variable name
        let mut var_counter = HashMap::new();
        
        for node_id in self.body.clone() {
            let target_opt = {
                if let Some(node) = self.nodes.get(&node_id) {
                    if let IrNode::Assignment(assignment) = node {
                        Some(assignment.target)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if let Some(target) = target_opt {
                if let Some(IrNode::VariableAccess(var_node)) = self.nodes.get(&target) {
                    let var_name = var_node.name.clone();
                    let counter = var_counter.entry(var_name.clone()).or_insert(0);
                    let new_name = format!("{}_{}", var_name, counter);
                    *counter += 1;
                    if let Some(IrNode::VariableAccess(v)) = self.nodes.get_mut(&target) {
                        v.name = new_name;
                    }
                }
            }
        }
    }

    fn next_phi_id(&self) -> NodeId {
        let max_id = self.nodes.keys().map(|id| id.0).max().unwrap_or(0);
        NodeId(max_id + 1)
    }

    /// Type checking
    pub fn type_check(&self) -> Result<Vec<TypeError>, String> {
        let mut errors = Vec::new();
        
        for (&node_id, node) in &self.nodes {
            match node {
                IrNode::BinaryOperation(bin_op) => {
                    let left_type = self.get_node_type(bin_op.left);
                    let right_type = self.get_node_type(bin_op.right);
                    
                    if left_type != right_type {
                        errors.push(TypeError {
                            node_id,
                            message: format!("Type mismatch: {:?} vs {:?}", left_type, right_type),
                        });
                    }
                    
                    if !self.is_operation_valid(bin_op.op.clone(), left_type.clone(), right_type.clone()) {
                        errors.push(TypeError {
                            node_id,
                            message: format!("Invalid operation {:?} for types {:?} {:?}", bin_op.op, left_type, right_type),
                        });
                    }
                }
                IrNode::FunctionCall(func_call) => {
                    // Function signature kontrolü
                    for (i, &arg) in func_call.arguments.iter().enumerate() {
                        let arg_type = self.get_node_type(arg);
                        // Basit kontrol - gerçek implementasyonda function signature kontrolü gerekir
                        if arg_type == IrType::Void {
                            errors.push(TypeError {
                                node_id,
                                message: format!("Argument {} has void type", i),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        
        Ok(errors)
    }

    fn get_node_type(&self, node_id: NodeId) -> IrType {
        if let Some(node) = self.nodes.get(&node_id) {
            match node {
                IrNode::Constant(const_node) => const_node.value.get_type(),
                IrNode::VariableAccess(var_node) => var_node.var_type.clone(),
                IrNode::BinaryOperation(bin_op) => bin_op.result_type.clone(),
                IrNode::UnaryOperation(unary_op) => unary_op.result_type.clone(),
                IrNode::FunctionCall(func_call) => func_call.return_type.clone(),
                _ => IrType::Void,
            }
        } else {
            IrType::Void
        }
    }

    fn is_operation_valid(&self, op: BinaryOp, left: IrType, right: IrType) -> bool {
        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                matches!(left, IrType::Int32 | IrType::Float32 | IrType::Float64) &&
                matches!(right, IrType::Int32 | IrType::Float32 | IrType::Float64)
            }
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                left == right
            }
            BinaryOp::And | BinaryOp::Or => {
                matches!(left, IrType::Bool) && matches!(right, IrType::Bool)
            }
            _ => true, // Diğer operasyonlar için basit varsayım
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeError {
    pub node_id: NodeId,
    pub message: String,
}

impl IrValue {
    pub fn get_type(&self) -> IrType {
        match self {
            IrValue::Bool(_) => IrType::Bool,
            IrValue::Int8(_) => IrType::Int8,
            IrValue::Int16(_) => IrType::Int16,
            IrValue::Int32(_) => IrType::Int32,
            IrValue::Int64(_) => IrType::Int64,
            IrValue::UInt8(_) => IrType::UInt8,
            IrValue::UInt16(_) => IrType::UInt16,
            IrValue::UInt32(_) => IrType::UInt32,
            IrValue::UInt64(_) => IrType::UInt64,
            IrValue::Float32(_) => IrType::Float32,
            IrValue::Float64(_) => IrType::Float64,
            IrValue::String(_) => IrType::Array(Box::new(IrType::UInt8), 0), // Basit varsayım
            IrValue::Array(_) => IrType::Array(Box::new(IrType::UInt8), 0), // Basit varsayım
            IrValue::Struct(_) => IrType::Struct(StructDefinition {
                name: "Unknown".to_string(),
                fields: Vec::new(),
            }),
        }
    }
}

impl IrModule {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            globals: Vec::new(),
            structs: Vec::new(),
            constants: Vec::new(),
        }
    }

    pub fn add_function(&mut self, function: IrFunction) {
        self.functions.push(function);
    }

    pub fn add_constant(&mut self, name: String, value: IrValue) {
        self.constants.push((name, value));
    }
}

pub struct IrBuilder {
    pub current_function: Option<String>,
    pub nodes: HashMap<NodeId, IrNode>,
    pub next_id: u32,
}

impl IrBuilder {
    pub fn new() -> Self {
        Self {
            current_function: None,
            nodes: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn with_function(mut self, name: String) -> Self {
        self.current_function = Some(name);
        self
    }

    fn next_id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn constant(&mut self, value: IrValue) -> NodeId {
        let id = self.next_id();
        let node = IrNode::Constant(ConstantNode { value, id });
        self.nodes.insert(id, node);
        id
    }

    pub fn variable(&mut self, name: String, var_type: IrType) -> NodeId {
        let id = self.next_id();
        let node = IrNode::VariableAccess(VariableNode { name, var_type, id });
        self.nodes.insert(id, node);
        id
    }

    pub fn binary_op(&mut self, op: BinaryOp, left: NodeId, right: NodeId, result_type: IrType) -> NodeId {
        let id = self.next_id();
        let node = IrNode::BinaryOperation(BinaryOpNode { op, left, right, result_type, id });
        self.nodes.insert(id, node);
        id
    }

    pub fn unary_op(&mut self, op: UnaryOp, operand: NodeId, result_type: IrType) -> NodeId {
        let id = self.next_id();
        let node = IrNode::UnaryOperation(UnaryOpNode { op, operand, result_type, id });
        self.nodes.insert(id, node);
        id
    }

    pub fn function_call(&mut self, function_name: String, arguments: Vec<NodeId>, return_type: IrType) -> NodeId {
        let id = self.next_id();
        let node = IrNode::FunctionCall(FunctionCallNode { function_name, arguments, return_type, id });
        self.nodes.insert(id, node);
        id
    }

    pub fn branch(&mut self, condition: NodeId, then_block: Vec<NodeId>, else_block: Vec<NodeId>) -> NodeId {
        let id = self.next_id();
        let node = IrNode::Branch(BranchNode { condition, then_block, else_block, id });
        self.nodes.insert(id, node);
        id
    }

    pub fn loop_node(&mut self, condition: NodeId, body: Vec<NodeId>, loop_type: LoopType) -> NodeId {
        let id = self.next_id();
        let node = IrNode::Loop(LoopNode { condition, body, loop_type, id });
        self.nodes.insert(id, node);
        id
    }

    pub fn assignment(&mut self, target: NodeId, source: NodeId) -> NodeId {
        let id = self.next_id();
        let node = IrNode::Assignment(AssignmentNode { target, source, id });
        self.nodes.insert(id, node);
        id
    }

    pub fn return_node(&mut self, value: Option<NodeId>) -> NodeId {
        let id = self.next_id();
        let node = IrNode::Return(ReturnNode { value, id });
        self.nodes.insert(id, node);
        id
    }

    pub fn build_function(self, name: String, parameters: Vec<(String, IrType)>, return_type: IrType) -> IrFunction {
        let body: Vec<NodeId> = self.nodes.keys().cloned().collect();
        IrFunction {
            name,
            parameters,
            return_type,
            body,
            nodes: self.nodes,
            ssa_form: false,
            dominator_tree: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ir_builder() {
        let mut builder = IrBuilder::new();
        
        // Create some constants
        let const1 = builder.constant(IrValue::Int32(5));
        let const2 = builder.constant(IrValue::Int32(10));
        
        // Create an addition operation
        let add_result = builder.binary_op(
            BinaryOp::Add,
            const1,
            const2,
            IrType::Int32
        );
        
        // Create a return statement
        let return_node = builder.return_node(Some(add_result));
        
        // Build the function
        let function = builder.build_function(
            "add_five_and_ten".to_string(),
            vec![],
            IrType::Int32
        );
        
        assert_eq!(function.name, "add_five_and_ten");
        assert_eq!(function.parameters.len(), 0);
        assert_eq!(function.return_type, IrType::Int32);
        assert!(function.nodes.contains_key(&return_node));
    }
}