/// shader_editor.rs — Real-time Shader Editor
/// GLSL/WGSL kod editörü, live preview, node-to-code conversion.
///
/// Özellikler:
/// - Shader code editing with line tracking
/// - Vertex + Fragment shader tabs
/// - Live compilation with error reporting
/// - Uniform management (float, vec2, vec3, vec4, sampler)
/// - Node-to-code: visual shader graph → GLSL/WGSL
/// - Built-in shader library (PBR, unlit, toon, dissolve, etc.)
/// - Shader presets and templates
/// - Preview rendering (sphere, cube, torus, plane)
/// - Hot reload support
/// - Export to .glsl / .wgsl files

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════ Shader Types

/// Shader language
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShaderLanguage {
    Glsl300Es,    // WebGL2 / OpenGL ES 3.0
    Glsl330,      // Desktop OpenGL 3.3
    Glsl450,      // OpenGL 4.5
    Wgsl,         // WebGPU
    Hlsl,         // DirectX (future)
}

impl ShaderLanguage {
    pub fn name(&self) -> &str {
        match self {
            Self::Glsl300Es => "GLSL ES 3.0", Self::Glsl330 => "GLSL 3.30",
            Self::Glsl450 => "GLSL 4.50", Self::Wgsl => "WGSL",
            Self::Hlsl => "HLSL",
        }
    }
    pub fn file_extension(&self) -> &str {
        match self {
            Self::Wgsl => "wgsl", _ => "glsl",
        }
    }
}

/// Shader stage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

impl ShaderStage {
    pub fn name(&self) -> &str {
        match self { Self::Vertex => "Vertex", Self::Fragment => "Fragment", Self::Compute => "Compute" }
    }
}

/// Shader compilation status
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompileStatus {
    NotCompiled,
    Compiling,
    Success,
    Error(ShaderError),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShaderError {
    pub line: u32,
    pub column: u32,
    pub message: String,
    pub severity: ErrorSeverity,
    pub source_line: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Error,
    Warning,
    Info,
}

// ═══════════════════════════════════════════════════════════ Uniforms

/// Shader uniform value
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UniformValue {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Int(i32),
    Mat4([[f32; 4]; 4]),
    Bool(bool),
    /// Texture slot index
    Texture(u32),
}

impl UniformValue {
    pub fn type_name(&self) -> &str {
        match self {
            Self::Float(_) => "float", Self::Vec2(_) => "vec2", Self::Vec3(_) => "vec3",
            Self::Vec4(_) => "vec4", Self::Int(_) => "int", Self::Mat4(_) => "mat4",
            Self::Bool(_) => "bool", Self::Texture(_) => "sampler2D",
        }
    }
    pub fn to_glsl(&self, name: &str) -> String {
        match self {
            Self::Float(v) => format!("uniform float {} = {:.4};", name, v),
            Self::Vec2(v) => format!("uniform vec2 {} = vec2({:.4}, {:.4});", name, v[0], v[1]),
            Self::Vec3(v) => format!("uniform vec3 {} = vec3({:.4}, {:.4}, {:.4});", name, v[0], v[1], v[2]),
            Self::Vec4(v) => format!("uniform vec4 {} = vec4({:.4}, {:.4}, {:.4}, {:.4});", name, v[0], v[1], v[2], v[3]),
            Self::Int(v) => format!("uniform int {} = {};", name, v),
            Self::Bool(v) => format!("uniform bool {} = {};", name, if *v { "true" } else { "false" }),
            Self::Texture(slot) => format!("uniform sampler2D {};", name),
            Self::Mat4(_) => format!("uniform mat4 {};", name),
        }
    }
    pub fn to_wgsl(&self, name: &str) -> String {
        match self {
            Self::Float(v) => format!("var<uniform> {}: f32 = {:.4};", name, v),
            Self::Vec2(v) => format!("var<uniform> {}: vec2<f32> = vec2<f32>({:.4}, {:.4});", name, v[0], v[1]),
            Self::Vec3(v) => format!("var<uniform> {}: vec3<f32> = vec3<f32>({:.4}, {:.4}, {:.4});", name, v[0], v[1], v[2]),
            Self::Vec4(v) => format!("var<uniform> {}: vec4<f32> = vec4<f32>({:.4}, {:.4}, {:.4}, {:.4});", name, v[0], v[1], v[2], v[3]),
            Self::Int(v) => format!("var<uniform> {}: i32 = {};", name, v),
            Self::Bool(v) => format!("var<uniform> {}: u32 = {};", name, if *v { 1u32 } else { 0u32 }),
            Self::Texture(_) => format!("var {}: texture_2d<f32>;", name),
            Self::Mat4(_) => format!("var<uniform> {}: mat4x4<f32>;", name),
        }
    }
}

/// Uniform declaration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Uniform {
    pub name: String,
    pub value: UniformValue,
    pub group: UniformGroup,
    pub visible: bool,
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub step: Option<f32>,
    pub tooltip: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UniformGroup {
    Transform,
    Material,
    Lighting,
    Time,
    Camera,
    Custom(String),
}

// ═══════════════════════════════════════════════════════════ Shader Code

/// Complete shader program
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShaderProgram {
    pub name: String,
    pub language: ShaderLanguage,
    pub vertex_source: String,
    pub fragment_source: String,
    pub uniforms: Vec<Uniform>,
    pub includes: Vec<String>,
    pub tags: Vec<String>,
    pub description: String,
    pub version: u32,
}

impl Default for ShaderProgram {
    fn default() -> Self {
        Self {
            name: "Untitled Shader".into(), language: ShaderLanguage::Glsl300Es,
            vertex_source: String::new(), fragment_source: String::new(),
            uniforms: Vec::new(), includes: Vec::new(), tags: Vec::new(),
            description: String::new(), version: 1,
        }
    }
}

impl ShaderProgram {
    /// Full vertex + fragment source combined
    pub fn full_source(&self) -> String {
        let mut src = String::new();
        // Version
        match self.language {
            ShaderLanguage::Glsl300Es => src += "#version 300 es\nprecision highp float;\n",
            ShaderLanguage::Glsl330 => src += "#version 330 core\n",
            ShaderLanguage::Glsl450 => src += "#version 450 core\n",
            ShaderLanguage::Wgsl => {},
            ShaderLanguage::Hlsl => {},
        }
        // Uniforms
        for u in &self.uniforms {
            src += &u.value.to_glsl(&u.name);
            src += "\n";
        }
        src += "\n// ── Vertex Shader ──\n";
        src += &self.vertex_source;
        src += "\n\n// ── Fragment Shader ──\n";
        src += &self.fragment_source;
        src
    }

    /// Validate shader source for common errors
    pub fn validate(&self) -> Vec<ShaderError> {
        let mut errors = Vec::new();

        // Check vertex shader
        self.validate_stage(&self.vertex_source, ShaderStage::Vertex, &mut errors);
        // Check fragment shader
        self.validate_stage(&self.fragment_source, ShaderStage::Fragment, &mut errors);

        errors
    }

    fn validate_stage(&self, source: &str, stage: ShaderStage, errors: &mut Vec<ShaderError>) {
        let stage_name = stage.name();
        for (i, line) in source.lines().enumerate() {
            let line_num = (i + 1) as u32;
            let trimmed = line.trim();

            // Check for common errors
            if trimmed.contains("texture(") && !trimmed.contains("sampler2D") && !trimmed.starts_with("//") {
                errors.push(ShaderError {
                    line: line_num, column: 0,
                    message: format!("[{}] texture() requires sampler2D uniform", stage_name),
                    severity: ErrorSeverity::Warning,
                    source_line: line.to_string(),
                });
            }

            // Check for missing semicolons (simple heuristic)
            if !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("#") &&
               !trimmed.ends_with('{') && !trimmed.ends_with('}') && !trimmed.ends_with(';') &&
               !trimmed.ends_with(',') && trimmed.len() > 3 {
                errors.push(ShaderError {
                    line: line_num, column: line.len() as u32,
                    message: format!("[{}] Possible missing semicolon", stage_name),
                    severity: ErrorSeverity::Info,
                    source_line: line.to_string(),
                });
            }

            // Check for undeclared variables (simple check)
            if trimmed.starts_with("gl_Position") && !source.contains("gl_Position") {
                // OK — it's being assigned
            }
        }
    }

    /// Compile with simulated results
    pub fn compile(&self) -> CompileStatus {
        let errors = self.validate();
        if errors.iter().any(|e| e.severity == ErrorSeverity::Error) {
            CompileStatus::Error(errors.into_iter().find(|e| e.severity == ErrorSeverity::Error).unwrap())
        } else if self.vertex_source.is_empty() || self.fragment_source.is_empty() {
            CompileStatus::Error(ShaderError {
                line: 0, column: 0,
                message: "Empty shader source".into(),
                severity: ErrorSeverity::Error,
                source_line: String::new(),
            })
        } else {
            CompileStatus::Success
        }
    }
}

// ═══════════════════════════════════════════════════════════ Node System

/// Node type in the visual shader graph
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShaderNodeType {
    // Input nodes
    VertexPosition,
    VertexNormal,
    VertexUV,
    VertexColor,
    Time,
    FragmentCoord,
    ViewDirection,
    Normal,

    // Math nodes
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
    Sine,
    Cosine,
    Tangent,
    Lerp,
    Clamp,
    Remap,
    Smoothstep,
    Abs,
    Floor,
    Ceil,
    Fract,
    Min,
    Max,
    Step,

    // Vector nodes
    VectorSplit,
    VectorCombine,
    VectorLength,
    DotProduct,
    CrossProduct,
    Normalize,
    Reflect,
    Refract,
    MixVec,

    // Color nodes
    ColorConstant,
    Gradient,
    Checkerboard,
    Noise,
    Fresnel,
    RimLight,

    // Texture nodes
    TextureSample,
    TextureTransform,

    // Output nodes
    SurfaceOutput,
    EmissionOutput,
    DisplacementOutput,
    AlphaOutput,
}

/// Pin data type
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShaderPinType {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Int,
    Bool,
    Sampler,
    Matrix,
}

/// A pin on a node (input or output)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShaderPin {
    pub name: String,
    pub pin_type: ShaderPinType,
    pub connected_to: Option<(u32, String)>, // (node_id, pin_name)
    pub default_value: ShaderValue,
}

/// Shader value for pins
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ShaderValue {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Int(i32),
    Bool(bool),
    Sampler(u32),
}

/// A node in the shader graph
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShaderNode {
    pub id: u32,
    pub node_type: ShaderNodeType,
    pub position: [f32; 2],
    pub inputs: Vec<ShaderPin>,
    pub outputs: Vec<ShaderPin>,
    pub enabled: bool,
    pub label: String,
}

/// Connection between two pins
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShaderConnection {
    pub from_node: u32,
    pub from_pin: String,
    pub to_node: u32,
    pub to_pin: String,
}

/// Complete shader graph
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShaderGraph {
    pub nodes: Vec<ShaderNode>,
    pub connections: Vec<ShaderConnection>,
    pub output_node: Option<u32>,
    pub name: String,
}

impl Default for ShaderGraph {
    fn default() -> Self {
        Self {
            nodes: Vec::new(), connections: Vec::new(),
            output_node: None, name: "Untitled Graph".into(),
        }
    }
}

impl ShaderGraph {
    pub fn new() -> Self { Self::default() }

    /// Add a node and return its ID
    pub fn add_node(&mut self, node_type: ShaderNodeType, position: [f32; 2]) -> u32 {
        let id = self.nodes.len() as u32;
        let (label, inputs, outputs) = Self::node_info(&node_type);

        self.nodes.push(ShaderNode {
            id, node_type, position, inputs, outputs, enabled: true, label,
        });
        id
    }

    /// Connect two nodes
    pub fn connect(&mut self, from_node: u32, from_pin: &str, to_node: u32, to_pin: &str) {
        // Remove existing connection to the target pin
        self.connections.retain(|c| !(c.to_node == to_node && c.to_pin == to_pin));
        self.connections.push(ShaderConnection {
            from_node, from_pin: from_pin.to_string(),
            to_node, to_pin: to_pin.to_string(),
        });
    }

    /// Disconnect a pin
    pub fn disconnect(&mut self, to_node: u32, to_pin: &str) {
        self.connections.retain(|c| !(c.to_node == to_node && c.to_pin == to_pin));
    }

    /// Get connected value for an input pin
    pub fn get_input_value(&self, node_id: u32, pin_name: &str) -> Option<ShaderValue> {
        let conn = self.connections.iter().find(|c| c.to_node == node_id && c.to_pin == pin_name)?;
        let from_node = self.nodes.iter().find(|n| n.id == conn.from_node)?;
        let output_pin = from_node.outputs.iter().find(|p| p.name == conn.from_pin)?;
        Some(output_pin.default_value.clone())
    }

    /// Remove a node and its connections
    pub fn remove_node(&mut self, id: u32) {
        self.nodes.retain(|n| n.id != id);
        self.connections.retain(|c| c.from_node != id && c.to_node != id);
    }

    /// Generate GLSL code from the graph
    pub fn generate_glsl(&self) -> String {
        let mut code = String::new();

        // Find the output node
        let output = match self.nodes.iter().find(|n| n.node_type == ShaderNodeType::SurfaceOutput) {
            Some(n) => n,
            None => return "// No output node\n".into(),
        };

        // Trace connections back to inputs and generate expressions
        let albedo = self.resolve_pin_glsl(output.id, "albedo");
        let emission = self.resolve_pin_glsl(output.id, "emission");
        let alpha = self.resolve_pin_glsl(output.id, "alpha");
        let normal = self.resolve_pin_glsl(output.id, "normal");

        code += &format!("// Generated by Elysium Shader Editor\n");
        code += "// ── Vertex Shader ──\n";
        code += "out vec3 vWorldPos;\n";
        code += "out vec3 vNormal;\n";
        code += "out vec2 vUV;\n\n";
        code += "void main() {\n";
        code += "    vWorldPos = (uModel * vec4(aPos, 1.0)).xyz;\n";
        code += "    vNormal = normalize(uNormalMatrix * aNormal);\n";
        code += "    vUV = aUV;\n";
        code += "    gl_Position = uViewProjection * vec4(vWorldPos, 1.0);\n";
        code += "}\n\n";

        code += "// ── Fragment Shader ──\n";
        code += "in vec3 vWorldPos;\n";
        code += "in vec3 vNormal;\n";
        code += "in vec2 vUV;\n\n";
        code += "void main() {\n";
        code += "    vec3 normal = normalize(vNormal);\n";
        code += &format!("    vec3 albedo = {};\n", albedo);
        if emission != "vec3(0.0)" {
            code += &format!("    vec3 emission = {};\n", emission);
        }
        if alpha != "1.0" {
            code += &format!("    float alpha = {};\n", alpha);
        }
        code += "\n    // Simple lighting\n";
        code += "    vec3 lightDir = normalize(vec3(0.5, 1.0, 0.3));\n";
        code += "    float diff = max(dot(normal, lightDir), 0.0);\n";
        code += "    vec3 color = albedo * (0.2 + diff * 0.8);\n";
        if emission != "vec3(0.0)" {
            code += "    color += emission;\n";
        }
        code += "    FragColor = vec4(color, 1.0);\n";
        code += "}\n";

        code
    }

    /// Generate WGSL code from the graph
    pub fn generate_wgsl(&self) -> String {
        let output = match self.nodes.iter().find(|n| n.node_type == ShaderNodeType::SurfaceOutput) {
            Some(n) => n,
            None => return "// No output node\n".into(),
        };

        let albedo = self.resolve_pin_wgsl(output.id, "albedo");

        let mut code = String::new();
        code += "// Generated by Elysium Shader Editor (WGSL)\n\n";
        code += "struct VertexOutput {\n    @builtin(position) pos: vec4<f32>,\n";
        code += "    @location(0) worldPos: vec3<f32>,\n";
        code += "    @location(1) normal: vec3<f32>,\n";
        code += "    @location(2) uv: vec2<f32>,\n};\n\n";
        code += "@vertex fn vs(@location(0) pos: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>) -> VertexOutput {\n";
        code += "    var out: VertexOutput;\n";
        code += "    out.pos = uViewProjection * uModel * vec4<f32>(pos, 1.0);\n";
        code += "    out.worldPos = (uModel * vec4<f32>(pos, 1.0)).xyz;\n";
        code += "    out.normal = n;\n";
        code += "    out.uv = uv;\n";
        code += "    return out;\n}\n\n";
        code += "@fragment fn fs(in: VertexOutput) -> @location(0) vec4<f32> {\n";
        code += &format!("    let albedo = {};\n", albedo);
        code += "    let n = normalize(in.normal);\n";
        code += "    let lightDir = normalize(vec3<f32>(0.5, 1.0, 0.3));\n";
        code += "    let diff = max(dot(n, lightDir), 0.0);\n";
        code += "    let color = albedo * (0.2 + diff * 0.8);\n";
        code += "    return vec4<f32>(color, 1.0);\n}\n";

        code
    }

    /// Resolve a pin value to GLSL expression
    fn resolve_pin_glsl(&self, node_id: u32, pin_name: &str) -> String {
        let conn = self.connections.iter().find(|c| c.to_node == node_id && c.to_pin == pin_name);
        match conn {
            Some(c) => self.node_output_glsl(c.from_node, &c.from_pin),
            None => {
                // Get default value from input pin
                if let Some(node) = self.nodes.iter().find(|n| n.id == node_id) {
                    if let Some(pin) = node.inputs.iter().find(|p| p.name == pin_name) {
                        return Self::value_to_glsl(&pin.default_value);
                    }
                }
                "vec3(0.5)".into()
            }
        }
    }

    fn node_output_glsl(&self, node_id: u32, pin_name: &str) -> String {
        let node = match self.nodes.iter().find(|n| n.id == node_id) {
            Some(n) => n,
            None => return "vec3(0.0)".into(),
        };
        match node.node_type {
            ShaderNodeType::VertexPosition => "vWorldPos".into(),
            ShaderNodeType::VertexNormal | ShaderNodeType::Normal => "normal".into(),
            ShaderNodeType::VertexUV => "vUV".into(),
            ShaderNodeType::Time => "uTime".into(),
            ShaderNodeType::ViewDirection => "normalize(uCameraPos - vWorldPos)".into(),
            ShaderNodeType::ColorConstant => {
                let color = self.get_input_value(node_id, "color");
                match color {
                    Some(ShaderValue::Vec3(c)) => format!("vec3({:.2}, {:.2}, {:.2})", c[0], c[1], c[2]),
                    _ => "vec3(1.0)".into(),
                }
            }
            ShaderNodeType::Add => {
                let a = self.resolve_pin_glsl(node_id, "a");
                let b = self.resolve_pin_glsl(node_id, "b");
                format!("({} + {})", a, b)
            }
            ShaderNodeType::Multiply => {
                let a = self.resolve_pin_glsl(node_id, "a");
                let b = self.resolve_pin_glsl(node_id, "b");
                format!("({} * {})", a, b)
            }
            ShaderNodeType::Lerp => {
                let a = self.resolve_pin_glsl(node_id, "a");
                let b = self.resolve_pin_glsl(node_id, "b");
                let t = self.resolve_pin_glsl(node_id, "t");
                format!("mix({}, {}, {})", a, b, t)
            }
            ShaderNodeType::Sine => {
                let x = self.resolve_pin_glsl(node_id, "x");
                format!("sin({})", x)
            }
            ShaderNodeType::Fresnel => {
                "pow(1.0 - max(dot(normal, normalize(uCameraPos - vWorldPos)), 0.0), 3.0)".into()
            }
            ShaderNodeType::TextureSample => {
                let uv = self.resolve_pin_glsl(node_id, "uv");
                format!("texture(uTex0, {})", uv)
            }
            ShaderNodeType::Noise => {
                let uv = self.resolve_pin_glsl(node_id, "uv");
                format!("fract(sin(dot({}, vec2(12.9898, 78.233))) * 43758.5453)", uv)
            }
            _ => "vec3(0.5)".into(),
        }
    }

    fn resolve_pin_wgsl(&self, node_id: u32, pin_name: &str) -> String {
        let conn = self.connections.iter().find(|c| c.to_node == node_id && c.to_pin == pin_name);
        match conn {
            Some(c) => self.node_output_wgsl(c.from_node, &c.from_pin),
            None => {
                if let Some(node) = self.nodes.iter().find(|n| n.id == node_id) {
                    if let Some(pin) = node.inputs.iter().find(|p| p.name == pin_name) {
                        return Self::value_to_wgsl(&pin.default_value);
                    }
                }
                "vec3<f32>(0.5, 0.5, 0.5)".into()
            }
        }
    }

    fn node_output_wgsl(&self, node_id: u32, _pin_name: &str) -> String {
        let node = match self.nodes.iter().find(|n| n.id == node_id) {
            Some(n) => n,
            None => return "vec3<f32>(0.0, 0.0, 0.0)".into(),
        };
        match node.node_type {
            ShaderNodeType::VertexPosition => "in.worldPos".into(),
            ShaderNodeType::VertexNormal | ShaderNodeType::Normal => "in.normal".into(),
            ShaderNodeType::VertexUV => "in.uv".into(),
            ShaderNodeType::Time => "uTime".into(),
            ShaderNodeType::ColorConstant => {
                let color = self.get_input_value(node_id, "color");
                match color {
                    Some(ShaderValue::Vec3(c)) => format!("vec3<f32>({:.2}, {:.2}, {:.2})", c[0], c[1], c[2]),
                    _ => "vec3<f32>(1.0, 1.0, 1.0)".into(),
                }
            }
            ShaderNodeType::Add => {
                let a = self.resolve_pin_wgsl(node_id, "a");
                let b = self.resolve_pin_wgsl(node_id, "b");
                format!("({} + {})", a, b)
            }
            ShaderNodeType::Multiply => {
                let a = self.resolve_pin_wgsl(node_id, "a");
                let b = self.resolve_pin_wgsl(node_id, "b");
                format!("({} * {})", a, b)
            }
            ShaderNodeType::Fresnel => {
                "pow(1.0 - max(dot(in.normal, normalize(uCameraPos - in.worldPos)), 0.0), 3.0)".into()
            }
            _ => "vec3<f32>(0.5, 0.5, 0.5)".into(),
        }
    }

    fn value_to_glsl(v: &ShaderValue) -> String {
        match v {
            ShaderValue::Float(f) => format!("{:.4}", f),
            ShaderValue::Vec2(v) => format!("vec2({:.4}, {:.4})", v[0], v[1]),
            ShaderValue::Vec3(v) => format!("vec3({:.4}, {:.4}, {:.4})", v[0], v[1], v[2]),
            ShaderValue::Vec4(v) => format!("vec4({:.4}, {:.4}, {:.4}, {:.4})", v[0], v[1], v[2], v[3]),
            ShaderValue::Int(i) => format!("{}", i),
            ShaderValue::Bool(b) => if *b { "true".into() } else { "false".into() },
            ShaderValue::Sampler(_) => "vec3(0.5)".into(),
        }
    }

    fn value_to_wgsl(v: &ShaderValue) -> String {
        match v {
            ShaderValue::Float(f) => format!("{:.4}", f),
            ShaderValue::Vec2(v) => format!("vec2<f32>({:.4}, {:.4})", v[0], v[1]),
            ShaderValue::Vec3(v) => format!("vec3<f32>({:.4}, {:.4}, {:.4})", v[0], v[1], v[2]),
            ShaderValue::Vec4(v) => format!("vec4<f32>({:.4}, {:.4}, {:.4}, {:.4})", v[0], v[1], v[2], v[3]),
            ShaderValue::Int(i) => format!("{}", i),
            ShaderValue::Bool(b) => if *b { "1u".into() } else { "0u".into() },
            ShaderValue::Sampler(_) => "vec3<f32>(0.5, 0.5, 0.5)".into(),
        }
    }

    /// Get input/output pins for a node type
    fn node_info(t: &ShaderNodeType) -> (String, Vec<ShaderPin>, Vec<ShaderPin>) {
        let f = |name: &str| ShaderPin {
            name: name.into(), pin_type: ShaderPinType::Float,
            connected_to: None, default_value: ShaderValue::Float(0.0),
        };
        let v3 = |name: &str, def: [f32; 3]| ShaderPin {
            name: name.into(), pin_type: ShaderPinType::Vec3,
            connected_to: None, default_value: ShaderValue::Vec3(def),
        };
        let v2 = |name: &str| ShaderPin {
            name: name.into(), pin_type: ShaderPinType::Vec2,
            connected_to: None, default_value: ShaderValue::Vec2([0.0; 2]),
        };
        let out_v3 = |name: &str| ShaderPin {
            name: name.into(), pin_type: ShaderPinType::Vec3,
            connected_to: None, default_value: ShaderValue::Vec3([0.0; 3]),
        };
        let out_f = |name: &str| ShaderPin {
            name: name.into(), pin_type: ShaderPinType::Float,
            connected_to: None, default_value: ShaderValue::Float(0.0),
        };

        match t {
            ShaderNodeType::VertexPosition => ("Vertex Position".into(), vec![], vec![out_v3("position")]),
            ShaderNodeType::VertexNormal | ShaderNodeType::Normal => ("Normal".into(), vec![], vec![out_v3("normal")]),
            ShaderNodeType::VertexUV => ("UV".into(), vec![], vec![v2("uv")]),
            ShaderNodeType::Time => ("Time".into(), vec![], vec![out_f("time")]),
            ShaderNodeType::Add => ("Add".into(), vec![v3("a", [0.0; 3]), v3("b", [0.0; 3])], vec![out_v3("result")]),
            ShaderNodeType::Subtract => ("Subtract".into(), vec![v3("a", [0.0; 3]), v3("b", [0.0; 3])], vec![out_v3("result")]),
            ShaderNodeType::Multiply => ("Multiply".into(), vec![v3("a", [1.0; 3]), v3("b", [1.0; 3])], vec![out_v3("result")]),
            ShaderNodeType::Lerp => ("Lerp".into(), vec![v3("a", [0.0; 3]), v3("b", [1.0; 3]), f("t")], vec![out_v3("result")]),
            ShaderNodeType::Sine => ("Sine".into(), vec![f("x")], vec![out_f("result")]),
            ShaderNodeType::Cosine => ("Cosine".into(), vec![f("x")], vec![out_f("result")]),
            ShaderNodeType::Clamp => ("Clamp".into(), vec![f("value"), f("min"), f("max")], vec![out_f("result")]),
            ShaderNodeType::Fresnel => ("Fresnel".into(), vec![], vec![out_f("result")]),
            ShaderNodeType::ColorConstant => ("Color".into(), vec![v3("color", [1.0; 3])], vec![out_v3("color")]),
            ShaderNodeType::TextureSample => ("Texture Sample".into(), vec![v2("uv")], vec![out_v3("color")]),
            ShaderNodeType::Noise => ("Noise".into(), vec![v2("uv")], vec![out_f("noise")]),
            ShaderNodeType::SurfaceOutput => ("Surface Output".into(),
                vec![v3("albedo", [0.8; 3]), v3("emission", [0.0; 3]), f("alpha"), v3("normal", [0.0, 1.0, 0.0])],
                vec![]),
            _ => (format!("{:?}", t), vec![], vec![]),
        }
    }
}

// ═══════════════════════════════════════════════════════════ Shader Library

/// Built-in shader library
pub struct ShaderLibrary {
    pub shaders: Vec<ShaderPreset>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShaderPreset {
    pub name: String,
    pub category: ShaderCategory,
    pub description: String,
    pub vertex: String,
    pub fragment: String,
    pub uniforms: Vec<Uniform>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShaderCategory {
    PBR, Unlit, Toon, Dissolve, Water, Glass, Emissive, PostProcess, Sky, Custom,
}

impl ShaderCategory {
    pub fn name(&self) -> &str {
        match self {
            Self::PBR => "PBR", Self::Unlit => "Unlit", Self::Toon => "Toon",
            Self::Dissolve => "Dissolve", Self::Water => "Water", Self::Glass => "Glass",
            Self::Emissive => "Emissive", Self::PostProcess => "Post-Process",
            Self::Sky => "Sky", Self::Custom => "Custom",
        }
    }
}

impl Default for ShaderLibrary {
    fn default() -> Self {
        let mut lib = Self { shaders: Vec::new() };
        lib.load_defaults();
        lib
    }
}

impl ShaderLibrary {
    fn load_defaults(&mut self) {
        // Unlit shader
        self.shaders.push(ShaderPreset {
            name: "Unlit".into(), category: ShaderCategory::Unlit,
            description: "Basic unlit shader with texture support".into(),
            vertex: "out vec2 vUV;\nvoid main() {\n    vUV = aUV;\n    gl_Position = uViewProjection * uModel * vec4(aPos, 1.0);\n}\n".into(),
            fragment: "uniform vec3 uColor = vec3(1.0);\nin vec2 vUV;\nout vec4 FragColor;\nvoid main() {\n    FragColor = vec4(uColor, 1.0);\n}\n".into(),
            uniforms: vec![Uniform {
                name: "uColor".into(), value: UniformValue::Vec3([1.0; 3]),
                group: UniformGroup::Material, visible: true, min: None, max: None,
                step: None, tooltip: "Base color".into(),
            }],
        });

        // Toon shader
        self.shaders.push(ShaderPreset {
            name: "Toon".into(), category: ShaderCategory::Toon,
            description: "Cel-shaded toon shader".into(),
            vertex: "out vec3 vNormal;\nout vec3 vWorldPos;\nvoid main() {\n    vWorldPos = (uModel * vec4(aPos, 1.0)).xyz;\n    vNormal = normalize(uNormalMatrix * aNormal);\n    gl_Position = uViewProjection * vec4(vWorldPos, 1.0);\n}\n".into(),
            fragment: "uniform vec3 uColor = vec3(0.8, 0.2, 0.2);\nuniform float uSteps = 4.0;\nin vec3 vNormal;\nin vec3 vWorldPos;\nout vec4 FragColor;\nvoid main() {\n    vec3 n = normalize(vNormal);\n    vec3 l = normalize(vec3(0.5, 1.0, 0.3));\n    float diff = max(dot(n, l), 0.0);\n    diff = floor(diff * uSteps) / uSteps;\n    FragColor = vec4(uColor * (0.2 + diff * 0.8), 1.0);\n}\n".into(),
            uniforms: vec![
                Uniform { name: "uColor".into(), value: UniformValue::Vec3([0.8, 0.2, 0.2]),
                    group: UniformGroup::Material, visible: true, min: None, max: None, step: None, tooltip: "Base color".into() },
                Uniform { name: "uSteps".into(), value: UniformValue::Float(4.0),
                    group: UniformGroup::Material, visible: true, min: Some(2.0), max: Some(16.0), step: Some(1.0), tooltip: "Number of shading steps".into() },
            ],
        });

        // Dissolve shader
        self.shaders.push(ShaderPreset {
            name: "Dissolve".into(), category: ShaderCategory::Dissolve,
            description: "Dissolve effect with edge glow".into(),
            vertex: "out vec2 vUV;\nout vec3 vWorldPos;\nvoid main() {\n    vUV = aUV;\n    vWorldPos = (uModel * vec4(aPos, 1.0)).xyz;\n    gl_Position = uViewProjection * vec4(vWorldPos, 1.0);\n}\n".into(),
            fragment: "uniform vec3 uColor = vec3(1.0);\nuniform float uDissolve = 0.5;\nuniform vec3 uEdgeColor = vec3(1.0, 0.5, 0.0);\nuniform float uEdgeWidth = 0.05;\nin vec2 vUV;\nin vec3 vWorldPos;\nout vec4 FragColor;\nfloat hash(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }\nvoid main() {\n    float n = hash(vUV * 10.0);\n    if (n < uDissolve) discard;\n    float edge = smoothstep(uDissolve, uDissolve + uEdgeWidth, n);\n    vec3 color = mix(uEdgeColor, uColor, edge);\n    FragColor = vec4(color, 1.0);\n}\n".into(),
            uniforms: vec![
                Uniform { name: "uDissolve".into(), value: UniformValue::Float(0.5),
                    group: UniformGroup::Material, visible: true, min: Some(0.0), max: Some(1.0), step: Some(0.01), tooltip: "Dissolve amount".into() },
                Uniform { name: "uEdgeColor".into(), value: UniformValue::Vec3([1.0, 0.5, 0.0]),
                    group: UniformGroup::Material, visible: true, min: None, max: None, step: None, tooltip: "Edge glow color".into() },
            ],
        });

        // Water shader
        self.shaders.push(ShaderPreset {
            name: "Water".into(), category: ShaderCategory::Water,
            description: "Animated water surface".into(),
            vertex: "uniform float uTime = 0.0;\nuniform float uWaveHeight = 0.1;\nout vec2 vUV;\nout vec3 vNormal;\nout vec3 vWorldPos;\nvoid main() {\n    vec3 pos = aPos;\n    pos.y += sin(pos.x * 3.0 + uTime * 2.0) * uWaveHeight;\n    pos.y += cos(pos.z * 2.5 + uTime * 1.5) * uWaveHeight * 0.5;\n    vUV = aUV;\n    vWorldPos = (uModel * vec4(pos, 1.0)).xyz;\n    vNormal = normalize(uNormalMatrix * aNormal);\n    gl_Position = uViewProjection * vec4(vWorldPos, 1.0);\n}\n".into(),
            fragment: "uniform vec3 uWaterColor = vec3(0.1, 0.3, 0.6);\nuniform float uTime = 0.0;\nuniform vec3 uFresnelColor = vec3(0.5, 0.8, 1.0);\nin vec2 vUV;\nin vec3 vNormal;\nin vec3 vWorldPos;\nout vec4 FragColor;\nvoid main() {\n    vec3 n = normalize(vNormal);\n    vec3 v = normalize(uCameraPos - vWorldPos);\n    float fresnel = pow(1.0 - max(dot(n, v), 0.0), 3.0);\n    vec3 color = mix(uWaterColor, uFresnelColor, fresnel);\n    float shimmer = sin(vUV.x * 20.0 + uTime * 3.0) * 0.1;\n    color += shimmer;\n    FragColor = vec4(color, 0.85);\n}\n".into(),
            uniforms: vec![
                Uniform { name: "uWaterColor".into(), value: UniformValue::Vec3([0.1, 0.3, 0.6]),
                    group: UniformGroup::Material, visible: true, min: None, max: None, step: None, tooltip: "Water base color".into() },
                Uniform { name: "uWaveHeight".into(), value: UniformValue::Float(0.1),
                    group: UniformGroup::Material, visible: true, min: Some(0.0), max: Some(0.5), step: Some(0.01), tooltip: "Wave amplitude".into() },
            ],
        });

        // Emissive glow shader
        self.shaders.push(ShaderPreset {
            name: "Emissive Glow".into(), category: ShaderCategory::Emissive,
            description: "Emissive material with pulse animation".into(),
            vertex: "out vec3 vNormal;\nout vec3 vWorldPos;\nout vec2 vUV;\nvoid main() {\n    vWorldPos = (uModel * vec4(aPos, 1.0)).xyz;\n    vNormal = normalize(uNormalMatrix * aNormal);\n    vUV = aUV;\n    gl_Position = uViewProjection * vec4(vWorldPos, 1.0);\n}\n".into(),
            fragment: "uniform vec3 uBaseColor = vec3(0.1);\nuniform vec3 uEmissiveColor = vec3(0.0, 0.5, 1.0);\nuniform float uEmissiveStrength = 2.0;\nuniform float uPulseSpeed = 1.0;\nuniform float uTime = 0.0;\nin vec3 vNormal;\nin vec3 vWorldPos;\nin vec2 vUV;\nout vec4 FragColor;\nvoid main() {\n    vec3 n = normalize(vNormal);\n    vec3 l = normalize(vec3(0.5, 1.0, 0.3));\n    float diff = max(dot(n, l), 0.0);\n    float pulse = 0.5 + 0.5 * sin(uTime * uPulseSpeed);\n    vec3 color = uBaseColor * (0.2 + diff * 0.8);\n    color += uEmissiveColor * uEmissiveStrength * pulse;\n    FragColor = vec4(color, 1.0);\n}\n".into(),
            uniforms: vec![
                Uniform { name: "uEmissiveColor".into(), value: UniformValue::Vec3([0.0, 0.5, 1.0]),
                    group: UniformGroup::Material, visible: true, min: None, max: None, step: None, tooltip: "Emission color".into() },
                Uniform { name: "uEmissiveStrength".into(), value: UniformValue::Float(2.0),
                    group: UniformGroup::Material, visible: true, min: Some(0.0), max: Some(10.0), step: Some(0.1), tooltip: "Emission intensity".into() },
            ],
        });
    }

    pub fn search(&self, query: &str) -> Vec<&ShaderPreset> {
        let q = query.to_lowercase();
        self.shaders.iter().filter(|s|
            s.name.to_lowercase().contains(&q) ||
            s.category.name().to_lowercase().contains(&q) ||
            s.description.to_lowercase().contains(&q)
        ).collect()
    }

    pub fn by_category(&self, cat: &ShaderCategory) -> Vec<&ShaderPreset> {
        self.shaders.iter().filter(|s| &s.category == cat).collect()
    }
}

// ═══════════════════════════════════════════════════════════ Preview Mesh

/// Preview mesh for shader preview
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PreviewMesh {
    Sphere,
    Cube,
    Plane,
    Torus,
    Cylinder,
    Custom,
}

impl PreviewMesh {
    pub fn name(&self) -> &str {
        match self {
            Self::Sphere => "Sphere", Self::Cube => "Cube", Self::Plane => "Plane",
            Self::Torus => "Torus", Self::Cylinder => "Cylinder", Self::Custom => "Custom",
        }
    }
}

// ═══════════════════════════════════════════════════════════ Editor State

/// Shader editor state
pub struct ShaderEditorState {
    pub active_language: ShaderLanguage,
    pub active_stage: ShaderStage,

    /// Current shader
    pub current_shader: ShaderProgram,

    /// Shader graph (for node mode)
    pub graph: ShaderGraph,

    /// Shader library
    pub library: ShaderLibrary,

    /// Preview
    pub preview_mesh: PreviewMesh,
    pub preview_auto_rotate: bool,
    pub preview_rotation: f32,
    pub preview_zoom: f32,

    /// Editor state
    pub cursor_line: u32,
    pub cursor_column: u32,
    pub scroll_offset: u32,
    pub selected_line: u32,
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    pub tab_size: u32,

    /// Compile result
    pub compile_status: CompileStatus,
    pub last_compile_time: u64,

    /// History (undo for code editing)
    pub code_history: Vec<String>,
    pub history_index: usize,

    /// Syntax highlighting cache
    pub highlighted_lines: Vec<HighlightedLine>,

    /// File management
    pub is_modified: bool,
    pub file_path: Option<String>,

    /// Node editor state
    pub node_mode: bool,
    pub graph_zoom: f32,
    pub graph_offset: [f32; 2],
    pub dragging_node: Option<u32>,
    pub connecting_from: Option<(u32, String)>,
}

#[derive(Clone, Debug)]
pub struct HighlightedLine {
    pub line_number: u32,
    pub segments: Vec<SyntaxSegment>,
}

#[derive(Clone, Debug)]
pub struct SyntaxSegment {
    pub text: String,
    pub color: SyntaxColor,
    pub bold: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyntaxColor {
    Keyword,      // Blue
    Type,         // Cyan
    Function,     // Yellow
    Number,       // Orange
    String,       // Green
    Comment,      // Gray
    Operator,     // White
    Uniform,      // Purple
    Builtin,      // Red
    Normal,       // Default
}

impl Default for ShaderEditorState {
    fn default() -> Self {
        let mut shader = ShaderProgram::default();
        shader.language = ShaderLanguage::Glsl300Es;
        shader.vertex_source = "out vec3 vNormal;\nout vec3 vWorldPos;\n\nvoid main() {\n    vWorldPos = (uModel * vec4(aPos, 1.0)).xyz;\n    vNormal = normalize(uNormalMatrix * aNormal);\n    gl_Position = uViewProjection * vec4(vWorldPos, 1.0);\n}\n".into();
        shader.fragment_source = "uniform vec3 uColor = vec3(0.8, 0.2, 0.2);\n\nin vec3 vNormal;\nin vec3 vWorldPos;\nout vec4 FragColor;\n\nvoid main() {\n    vec3 n = normalize(vNormal);\n    vec3 l = normalize(vec3(0.5, 1.0, 0.3));\n    float diff = max(dot(n, l), 0.0);\n    vec3 color = uColor * (0.2 + diff * 0.8);\n    FragColor = vec4(color, 1.0);\n}\n".into();
        shader.uniforms = vec![
            Uniform { name: "uColor".into(), value: UniformValue::Vec3([0.8, 0.2, 0.2]),
                group: UniformGroup::Material, visible: true, min: None, max: None,
                step: None, tooltip: "Object color".into() },
        ];

        Self {
            active_language: ShaderLanguage::Glsl300Es,
            active_stage: ShaderStage::Fragment,
            current_shader: shader,
            graph: ShaderGraph::new(),
            library: ShaderLibrary::default(),
            preview_mesh: PreviewMesh::Sphere,
            preview_auto_rotate: true,
            preview_rotation: 0.0,
            preview_zoom: 2.0,
            cursor_line: 1, cursor_column: 0, scroll_offset: 0, selected_line: 0,
            show_line_numbers: true, word_wrap: false, tab_size: 4,
            compile_status: CompileStatus::NotCompiled,
            last_compile_time: 0,
            code_history: Vec::new(), history_index: 0,
            highlighted_lines: Vec::new(),
            is_modified: false, file_path: None,
            node_mode: false, graph_zoom: 1.0, graph_offset: [0.0; 2],
            dragging_node: None, connecting_from: None,
        }
    }
}

impl ShaderEditorState {
    pub fn new() -> Self { Self::default() }

    /// Set vertex source
    pub fn set_vertex_source(&mut self, source: &str) {
        self.current_shader.vertex_source = source.to_string();
        self.is_modified = true;
        self.highlight_vertex();
    }

    /// Set fragment source
    pub fn set_fragment_source(&mut self, source: &str) {
        self.current_shader.fragment_source = source.to_string();
        self.is_modified = true;
        self.highlight_fragment();
    }

    /// Get active source based on current stage
    pub fn active_source(&self) -> &str {
        match self.active_stage {
            ShaderStage::Vertex => &self.current_shader.vertex_source,
            ShaderStage::Fragment => &self.current_shader.fragment_source,
            _ => &self.current_shader.fragment_source,
        }
    }

    /// Set active source
    pub fn set_active_source(&mut self, source: &str) {
        match self.active_stage {
            ShaderStage::Vertex => self.set_vertex_source(source),
            ShaderStage::Fragment => self.set_fragment_source(source),
            _ => {}
        }
    }

    /// Compile the current shader
    pub fn compile(&mut self) {
        self.compile_status = CompileStatus::Compiling;
        self.compile_status = self.current_shader.compile();
    }

    /// Load a preset from the library
    pub fn load_preset(&mut self, index: usize) {
        if let Some(preset) = self.library.shaders.get(index) {
            self.current_shader.vertex_source = preset.vertex.clone();
            self.current_shader.fragment_source = preset.fragment.clone();
            self.current_shader.uniforms = preset.uniforms.clone();
            self.current_shader.name = preset.name.clone();
            self.is_modified = true;
            self.compile();
        }
    }

    /// Syntax highlight a line
    pub fn highlight_line(&self, line: &str) -> Vec<SyntaxSegment> {
        let mut segments = Vec::new();
        let keywords = ["uniform", "varying", "in", "out", "void", "main", "if", "else",
            "for", "while", "return", "break", "continue", "struct", "const", "precision",
            "var", "fn", "let", "true", "false"];
        let types = ["float", "vec2", "vec3", "vec4", "mat3", "mat4", "int", "sampler2D",
            "bool", "f32", "i32", "u32", "vec2f", "vec3f", "vec4f"];
        let builtins = ["gl_Position", "gl_FragColor", "gl_FragCoord", "sin", "cos", "tan",
            "pow", "sqrt", "abs", "floor", "ceil", "fract", "mix", "clamp", "step", "smoothstep",
            "min", "max", "dot", "cross", "normalize", "length", "reflect", "texture", "discard"];

        let trimmed = line.trim();

        // Comment
        if trimmed.starts_with("//") {
            segments.push(SyntaxSegment { text: line.to_string(), color: SyntaxColor::Comment, bold: false });
            return segments;
        }

        // Tokenize
        let mut i = 0;
        let chars: Vec<char> = line.chars().collect();
        while i < chars.len() {
            // Skip whitespace
            if chars[i].is_whitespace() {
                let start = i;
                while i < chars.len() && chars[i].is_whitespace() { i += 1; }
                segments.push(SyntaxSegment {
                    text: chars[start..i].iter().collect(), color: SyntaxColor::Normal, bold: false,
                });
                continue;
            }

            // Number
            if chars[i].is_digit(10) || (chars[i] == '.' && i + 1 < chars.len() && chars[i+1].is_digit(10)) {
                let start = i;
                while i < chars.len() && (chars[i].is_digit(10) || chars[i] == '.' || chars[i] == 'x') { i += 1; }
                segments.push(SyntaxSegment {
                    text: chars[start..i].iter().collect(), color: SyntaxColor::Number, bold: false,
                });
                continue;
            }

            // Word (keyword, type, builtin, or identifier)
            if chars[i].is_alphabetic() || chars[i] == '_' || chars[i] == 'u' {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') { i += 1; }
                let word: String = chars[start..i].iter().collect();

                let color = if keywords.contains(&word.as_str()) { SyntaxColor::Keyword }
                    else if types.contains(&word.as_str()) { SyntaxColor::Type }
                    else if builtins.contains(&word.as_str()) { SyntaxColor::Builtin }
                    else if word.starts_with("u") && word.len() > 1 && chars[start+1].is_uppercase() { SyntaxColor::Uniform }
                    else if word == "main" { SyntaxColor::Function }
                    else { SyntaxColor::Normal };

                segments.push(SyntaxSegment { text: word, color, bold: color == SyntaxColor::Keyword });
                continue;
            }

            // String
            if chars[i] == '"' || chars[i] == '\'' {
                let quote = chars[i];
                let start = i;
                i += 1;
                while i < chars.len() && chars[i] != quote { i += 1; }
                if i < chars.len() { i += 1; } // Skip closing quote
                segments.push(SyntaxSegment {
                    text: chars[start..i].iter().collect(), color: SyntaxColor::String, bold: false,
                });
                continue;
            }

            // Operator
            segments.push(SyntaxSegment {
                text: chars[i].to_string(), color: SyntaxColor::Operator, bold: false,
            });
            i += 1;
        }

        segments
    }

    /// Highlight vertex shader
    pub fn highlight_vertex(&mut self) {
        let source = self.current_shader.vertex_source.clone();
        self.highlighted_lines = source.lines().enumerate().map(|(i, line)| {
            HighlightedLine {
                line_number: (i + 1) as u32,
                segments: self.highlight_line(line),
            }
        }).collect();
    }

    /// Highlight fragment shader
    pub fn highlight_fragment(&mut self) {
        let source = self.current_shader.fragment_source.clone();
        self.highlighted_lines = source.lines().enumerate().map(|(i, line)| {
            HighlightedLine {
                line_number: (i + 1) as u32,
                segments: self.highlight_line(line),
            }
        }).collect();
    }

    /// Refresh syntax highlighting for active stage
    pub fn refresh_highlighting(&mut self) {
        match self.active_stage {
            ShaderStage::Vertex => self.highlight_vertex(),
            ShaderStage::Fragment => self.highlight_fragment(),
            _ => {}
        }
    }

    /// Export shader to string
    pub fn export_shader(&self, format: ShaderLanguage) -> String {
        let mut result = String::new();
        match format {
            ShaderLanguage::Glsl300Es => {
                result += "#version 300 es\nprecision highp float;\n\n";
            }
            ShaderLanguage::Glsl330 => {
                result += "#version 330 core\n\n";
            }
            ShaderLanguage::Wgsl => {
                return self.current_shader.fragment_source.clone(); // Simplified
            }
            _ => {}
        }
        result += "// ── Uniforms ──\n";
        for u in &self.current_shader.uniforms {
            result += &u.value.to_glsl(&u.name);
            result += "\n";
        }
        result += "\n// ── Vertex Shader ──\n";
        result += &self.current_shader.vertex_source;
        result += "\n// ── Fragment Shader ──\n";
        result += &self.current_shader.fragment_source;
        result
    }

    /// Add uniform
    pub fn add_uniform(&mut self, name: &str, value: UniformValue) {
        self.current_shader.uniforms.push(Uniform {
            name: name.to_string(), value, group: UniformGroup::Custom("Custom".into()),
            visible: true, min: None, max: None, step: None, tooltip: String::new(),
        });
    }

    /// Remove uniform
    pub fn remove_uniform(&mut self, name: &str) {
        self.current_shader.uniforms.retain(|u| u.name != name);
    }

    pub fn line_count(&self) -> usize {
        self.active_source().lines().count()
    }
}

// ═══════════════════════════════════════════════════════════ Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_program_creation() {
        let mut prog = ShaderProgram::default();
        prog.vertex_source = "void main() { gl_Position = vec4(0.0); }".into();
        prog.fragment_source = "out vec4 FragColor; void main() { FragColor = vec4(1.0); }".into();
        assert!(!prog.full_source().is_empty());
    }

    #[test]
    fn test_shader_validation_empty() {
        let prog = ShaderProgram::default();
        let status = prog.compile();
        assert!(matches!(status, CompileStatus::Error(_)));
    }

    #[test]
    fn test_shader_validation_success() {
        let mut prog = ShaderProgram::default();
        prog.vertex_source = "void main() { gl_Position = vec4(0.0, 0.0, 0.0, 1.0); }".into();
        prog.fragment_source = "out vec4 FragColor; void main() { FragColor = vec4(1.0, 0.0, 0.0, 1.0); }".into();
        let status = prog.compile();
        assert!(matches!(status, CompileStatus::Success));
    }

    #[test]
    fn test_uniform_glsl_generation() {
        let u = UniformValue::Vec3([0.5, 0.3, 0.1]);
        let glsl = u.to_glsl("uColor");
        assert!(glsl.contains("uniform vec3 uColor"));
        assert!(glsl.contains("0.5"));
    }

    #[test]
    fn test_uniform_wgsl_generation() {
        let u = UniformValue::Vec3([0.5, 0.3, 0.1]);
        let wgsl = u.to_wgsl("u_color");
        assert!(wgsl.contains("vec3<f32>"));
    }

    #[test]
    fn test_shader_graph_creation() {
        let mut graph = ShaderGraph::new();
        let n1 = graph.add_node(ShaderNodeType::VertexPosition, [0.0, 0.0]);
        let n2 = graph.add_node(ShaderNodeType::ColorConstant, [200.0, 0.0]);
        let n3 = graph.add_node(ShaderNodeType::SurfaceOutput, [400.0, 0.0]);
        assert_eq!(graph.nodes.len(), 3);

        graph.connect(n2, "color", n3, "albedo");
        assert_eq!(graph.connections.len(), 1);
    }

    #[test]
    fn test_shader_graph_code_gen() {
        let mut graph = ShaderGraph::new();
        let color = graph.add_node(ShaderNodeType::ColorConstant, [0.0, 0.0]);
        let output = graph.add_node(ShaderNodeType::SurfaceOutput, [200.0, 0.0]);
        graph.connect(color, "color", output, "albedo");

        let glsl = graph.generate_glsl();
        assert!(glsl.contains("Vertex Shader"));
        assert!(glsl.contains("Fragment Shader"));
    }

    #[test]
    fn test_node_connections() {
        let mut graph = ShaderGraph::new();
        let n1 = graph.add_node(ShaderNodeType::Add, [0.0, 0.0]);
        let n2 = graph.add_node(ShaderNodeType::SurfaceOutput, [200.0, 0.0]);
        graph.connect(n1, "result", n2, "albedo");
        assert_eq!(graph.connections.len(), 1);

        graph.disconnect(n2, "albedo");
        assert_eq!(graph.connections.len(), 0);
    }

    #[test]
    fn test_node_removal() {
        let mut graph = ShaderGraph::new();
        let n1 = graph.add_node(ShaderNodeType::Add, [0.0, 0.0]);
        let n2 = graph.add_node(ShaderNodeType::SurfaceOutput, [200.0, 0.0]);
        graph.connect(n1, "result", n2, "albedo");

        graph.remove_node(n1);
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.connections.is_empty());
    }

    #[test]
    fn test_shader_library() {
        let lib = ShaderLibrary::default();
        assert!(lib.shaders.len() >= 4);
        let toon: Vec<&ShaderPreset> = lib.by_category(&ShaderCategory::Toon);
        assert_eq!(toon.len(), 1);
    }

    #[test]
    fn test_library_search() {
        let lib = ShaderLibrary::default();
        let results = lib.search("water");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Water");
    }

    #[test]
    fn test_syntax_highlighting() {
        let mut editor = ShaderEditorState::new();
        let segments = editor.highlight_line("uniform vec3 uColor = vec3(1.0);");
        assert!(!segments.is_empty());
        // First segment should be keyword "uniform"
        assert_eq!(segments[0].text, "uniform");
        assert!(matches!(segments[0].color, SyntaxColor::Keyword));
    }

    #[test]
    fn test_syntax_highlight_comment() {
        let editor = ShaderEditorState::new();
        let segments = editor.highlight_line("// This is a comment");
        assert_eq!(segments.len(), 1);
        assert!(matches!(segments[0].color, SyntaxColor::Comment));
    }

    #[test]
    fn test_editor_state() {
        let mut editor = ShaderEditorState::new();
        assert_eq!(editor.active_stage, ShaderStage::Fragment);
        editor.set_active_source("void main() {}");
        assert_eq!(editor.active_source(), "void main() {}");
        assert!(editor.is_modified);
    }

    #[test]
    fn test_export_shader() {
        let mut editor = ShaderEditorState::new();
        editor.current_shader.vertex_source = "void main() {}".into();
        editor.current_shader.fragment_source = "void main() {}".into();
        let exported = editor.export_shader(ShaderLanguage::Glsl300Es);
        assert!(exported.contains("#version 300 es"));
    }

    #[test]
    fn test_add_remove_uniform() {
        let mut editor = ShaderEditorState::new();
        editor.add_uniform("uCustom", UniformValue::Float(1.0));
        assert_eq!(editor.current_shader.uniforms.len(), 2);
        editor.remove_uniform("uCustom");
        assert_eq!(editor.current_shader.uniforms.len(), 1);
    }

    #[test]
    fn test_preview_mesh_names() {
        assert_eq!(PreviewMesh::Sphere.name(), "Sphere");
        assert_eq!(PreviewMesh::Torus.name(), "Torus");
    }

    #[test]
    fn test_shader_category_names() {
        assert_eq!(ShaderCategory::PBR.name(), "PBR");
        assert_eq!(ShaderCategory::Dissolve.name(), "Dissolve");
    }

    #[test]
    fn test_graph_wgsl_generation() {
        let mut graph = ShaderGraph::new();
        let color = graph.add_node(ShaderNodeType::ColorConstant, [0.0, 0.0]);
        let output = graph.add_node(ShaderNodeType::SurfaceOutput, [200.0, 0.0]);
        graph.connect(color, "color", output, "albedo");

        let wgsl = graph.generate_wgsl();
        assert!(wgsl.contains("WGSL"));
        assert!(wgsl.contains("@vertex"));
        assert!(wgsl.contains("@fragment"));
    }

    #[test]
    fn test_full_source_includes_uniforms() {
        let mut prog = ShaderProgram::default();
        prog.vertex_source = "void main() {}".into();
        prog.fragment_source = "void main() {}".into();
        prog.uniforms.push(Uniform {
            name: "uTest".into(), value: UniformValue::Float(0.5),
            group: UniformGroup::Custom("Test".into()), visible: true,
            min: None, max: None, step: None, tooltip: String::new(),
        });
        let src = prog.full_source();
        assert!(src.contains("uniform float uTest"));
    }
}
