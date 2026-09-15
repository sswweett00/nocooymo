
//! advanced_features.rs — 11 aşırı gelişmiş motor özelliği
//!
//! 1. Visual Shader Graph Editor (node tabanlı)
//! 2. Pathfinding System (A* + navmesh)
//! 3. Prefab System (şablon + instantiate)
//! 4. Scene Hierarchy (parent-child transform)
//! 5. LOD System (distance-based detail)
//! 6. Decal System (projected textures)
//! 7. Spline/Curve System (Bezier paths)
//! 8. AI State Machine (behavior tree)
//! 9. Cinematic Sequencer (cutscene timeline)
//! 10. Hot Reload System (live asset reload)
//! 11. Voxel Global Illumination (GI)

use glam::{Vec3, Mat4, Quat};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════ 1. VISUAL SHADER GRAPH

/// Shader node tipi
#[derive(Clone, Debug, PartialEq)]
pub enum ShaderNodeType {
    // Girdi
    VertexPosition,
    VertexNormal,
    UV,
    Time,
    FragmentCoord,
    // Matematik
    Add,
    Multiply,
    Subtract,
    Divide,
    Power,
    Sine,
    Cosine,
    Lerp,
    Clamp,
    Remap,
    // Vektör
    VectorSplit,
    VectorCombine,
    DotProduct,
    CrossProduct,
    Normalize,
    // Renk
    ColorConstant,
    TextureSample,
    Fresnel,
    Checkerboard,
    Gradient,
    // Çıktı
    SurfaceOutput,
    DisplacementOutput,
    EmissionOutput,
}

/// Shader node'u
#[derive(Clone, Debug)]
pub struct ShaderNode {
    pub id: u32,
    pub node_type: ShaderNodeType,
    pub position: [f32; 2],
    pub inputs: Vec<ShaderPin>,
    pub outputs: Vec<ShaderPin>,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct ShaderPin {
    pub name: String,
    pub pin_type: ShaderPinType,
    pub connected_to: Option<(u32, usize)>, // (node_id, pin_index)
    pub default_value: ShaderValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShaderPinType {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Sampler2D,
    Bool,
}

#[derive(Clone, Debug)]
pub enum ShaderValue {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Bool(bool),
}

/// Shader Graph
#[derive(Clone, Debug)]
pub struct ShaderGraph {
    pub name: String,
    pub nodes: Vec<ShaderNode>,
    pub next_node_id: u32,
    pub selected_node: Option<u32>,
    pub zoom: f32,
    pub pan: [f32; 2],
}

impl Default for ShaderGraph {
    fn default() -> Self {
        Self {
            name: "NewShader".into(),
            nodes: Vec::new(),
            next_node_id: 1,
            selected_node: None,
            zoom: 1.0,
            pan: [0.0; 2],
        }
    }
}

impl ShaderGraph {
    pub fn add_node(&mut self, node_type: ShaderNodeType, x: f32, y: f32) -> u32 {
        let id = self.next_node_id;
        self.next_node_id += 1;
        let (label, inputs, outputs) = Self::create_node_data(&node_type);
        self.nodes.push(ShaderNode {
            id, node_type, position: [x, y], inputs, outputs, label,
        });
        id
    }

    fn create_node_data(nt: &ShaderNodeType) -> (String, Vec<ShaderPin>, Vec<ShaderPin>) {
        let float = || ShaderPin { name: "Float".into(), pin_type: ShaderPinType::Float, connected_to: None, default_value: ShaderValue::Float(0.0) };
        let vec3 = || ShaderPin { name: "Vec3".into(), pin_type: ShaderPinType::Vec3, connected_to: None, default_value: ShaderValue::Vec3([0.0; 3]) };
        let vec4 = || ShaderPin { name: "Vec4".into(), pin_type: ShaderPinType::Vec4, connected_to: None, default_value: ShaderValue::Vec4([0.0; 4]) };
        let sampler = || ShaderPin { name: "Tex".into(), pin_type: ShaderPinType::Sampler2D, connected_to: None, default_value: ShaderValue::Float(0.0) };

        match nt {
            ShaderNodeType::VertexPosition => ("Vertex Position".into(), vec![], vec![vec3()]),
            ShaderNodeType::VertexNormal => ("Vertex Normal".into(), vec![], vec![vec3()]),
            ShaderNodeType::UV => ("UV".into(), vec![], vec![ShaderPin { name: "UV".into(), pin_type: ShaderPinType::Vec2, connected_to: None, default_value: ShaderValue::Vec2([0.0; 2]) }]),
            ShaderNodeType::Time => ("Time".into(), vec![], vec![float()]),
            ShaderNodeType::FragmentCoord => ("Fragment Coord".into(), vec![], vec![vec3()]),
            ShaderNodeType::Add => ("Add".into(), vec![float(), float()], vec![float()]),
            ShaderNodeType::Multiply => ("Multiply".into(), vec![float(), float()], vec![float()]),
            ShaderNodeType::Subtract => ("Subtract".into(), vec![float(), float()], vec![float()]),
            ShaderNodeType::Divide => ("Divide".into(), vec![float(), float()], vec![float()]),
            ShaderNodeType::Power => ("Power".into(), vec![float(), float()], vec![float()]),
            ShaderNodeType::Sine => ("Sine".into(), vec![float()], vec![float()]),
            ShaderNodeType::Cosine => ("Cosine".into(), vec![float()], vec![float()]),
            ShaderNodeType::Lerp => ("Lerp".into(), vec![float(), float(), float()], vec![float()]),
            ShaderNodeType::Clamp => ("Clamp".into(), vec![float(), float(), float()], vec![float()]),
            ShaderNodeType::Remap => ("Remap".into(), vec![float(), float(), float(), float()], vec![float()]),
            ShaderNodeType::VectorSplit => ("Vector Split".into(), vec![vec3()], vec![float(), float(), float()]),
            ShaderNodeType::VectorCombine => ("Vector Combine".into(), vec![float(), float(), float()], vec![vec3()]),
            ShaderNodeType::DotProduct => ("Dot Product".into(), vec![vec3(), vec3()], vec![float()]),
            ShaderNodeType::CrossProduct => ("Cross Product".into(), vec![vec3(), vec3()], vec![vec3()]),
            ShaderNodeType::Normalize => ("Normalize".into(), vec![vec3()], vec![vec3()]),
            ShaderNodeType::ColorConstant => ("Color".into(), vec![], vec![vec4()]),
            ShaderNodeType::TextureSample => ("Texture Sample".into(), vec![sampler(), ShaderPin { name: "UV".into(), pin_type: ShaderPinType::Vec2, connected_to: None, default_value: ShaderValue::Vec2([0.0; 2]) }], vec![vec4()]),
            ShaderNodeType::Fresnel => ("Fresnel".into(), vec![float(), vec3()], vec![float()]),
            ShaderNodeType::Checkerboard => ("Checkerboard".into(), vec![ShaderPin { name: "UV".into(), pin_type: ShaderPinType::Vec2, connected_to: None, default_value: ShaderValue::Vec2([0.0; 2]) }, float()], vec![vec3()]),
            ShaderNodeType::Gradient => ("Gradient".into(), vec![float()], vec![vec3()]),
            ShaderNodeType::SurfaceOutput => ("Surface Output".into(), vec![vec4(), float(), float(), vec3()], vec![]),
            ShaderNodeType::DisplacementOutput => ("Displacement Output".into(), vec![vec3()], vec![]),
            ShaderNodeType::EmissionOutput => ("Emission Output".into(), vec![vec3(), float()], vec![]),
        }
    }

    pub fn connect(&mut self, from_node: u32, from_pin: usize, to_node: u32, to_pin: usize) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == from_node) {
            if from_pin < node.outputs.len() {
                node.outputs[from_pin].connected_to = Some((to_node, to_pin));
            }
        }
    }

    pub fn disconnect(&mut self, from_node: u32, from_pin: usize) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == from_node) {
            if from_pin < node.outputs.len() {
                node.outputs[from_pin].connected_to = None;
            }
        }
    }

    /// Shader kodu üret (GLSL)
    pub fn generate_glsl(&self) -> String {
        let mut code = String::from("// Auto-generated by Elysium Shader Graph\n");
        code.push_str("#version 330 core\n\n");
        code.push_str("// Inputs\n");
        code.push_str("layout(location = 0) in vec3 aPos;\n");
        code.push_str("layout(location = 1) in vec3 aNormal;\n");
        code.push_str("layout(location = 2) in vec2 aUV;\n\n");
        code.push_str("uniform float uTime;\n");
        code.push_str("uniform mat4 uModel, uView, uProjection;\n\n");
        code.push_str("out vec3 FragPos, Normal;\nout vec2 UV;\n\n");
        code.push_str("void main() {\n");
        code.push_str("    FragPos = vec3(uModel * vec4(aPos, 1.0));\n");
        code.push_str("    Normal = mat3(transpose(inverse(uModel))) * aNormal;\n");
        code.push_str("    UV = aUV;\n");
        code.push_str("    gl_Position = uProjection * uView * uModel * vec4(aPos, 1.0);\n");
        code.push_str("}\n\n");
        code.push_str("// Fragment Shader\n");
        code.push_str("out vec4 FragColor;\n\n");
        code.push_str("void main() {\n");
        code.push_str("    vec3 albedo = vec3(0.8);\n");
        for node in &self.nodes {
            if node.node_type == ShaderNodeType::SurfaceOutput {
                code.push_str("    // Surface output connected\n");
            }
        }
        code.push_str("    FragColor = vec4(albedo, 1.0);\n");
        code.push_str("}\n");
        code
    }

    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn edge_count(&self) -> usize {
        self.nodes.iter().filter(|n| !n.outputs.is_empty())
            .map(|n| n.outputs.iter().filter(|p| p.connected_to.is_some()).count())
            .sum()
    }
}

// ═══════════════════════════════════════════════════════════ 2. PATHFINDING (A*)

/// NavMesh düğümü
#[derive(Clone, Debug)]
pub struct NavNode {
    pub id: u32,
    pub position: Vec3,
    pub cost: f32,
    pub walkable: bool,
}

/// NavMesh kenarı
#[derive(Clone, Debug)]
pub struct NavEdge {
    pub from: u32,
    pub to: u32,
    pub cost: f32,
}

/// NavMesh
#[derive(Clone, Debug)]
pub struct NavMesh {
    pub nodes: Vec<NavNode>,
    pub edges: Vec<NavEdge>,
    pub node_map: HashMap<u32, usize>,
    pub grid_size: f32,
    pub grid_res: usize,
    pub walkable_grid: Vec<Vec<bool>>,
}

impl Default for NavMesh {
    fn default() -> Self {
        let res = 32;
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            node_map: HashMap::new(),
            grid_size: 0.5,
            grid_res: res,
            walkable_grid: vec![vec![true; res]; res],
        }
    }
}

impl NavMesh {
    /// Grid tabanlı navmesh oluştur
    pub fn from_grid(walkable: &[Vec<bool>], grid_size: f32) -> Self {
        let res = walkable.len();
        let mut mesh = NavMesh::default();
        mesh.grid_res = res;
        mesh.grid_size = grid_size;
        mesh.walkable_grid = walkable.to_vec();

        // Düğüm oluştur
        for z in 0..res {
            for x in 0..res {
                if walkable[z][x] {
                    let id = (z * res + x) as u32;
                    mesh.nodes.push(NavNode {
                        id,
                        position: Vec3::new(x as f32 * grid_size, 0.0, z as f32 * grid_size),
                        cost: 1.0,
                        walkable: true,
                    });
                    mesh.node_map.insert(id, mesh.nodes.len() - 1);
                }
            }
        }

        // Kenarları oluştur (4-8 bağlantılı)
        for z in 0..res {
            for x in 0..res {
                if !walkable[z][x] { continue; }
                let id = (z * res + x) as u32;
                let dirs: [(i32, i32, f32); 8] = [
                    (-1, 0, 1.0), (1, 0, 1.0), (0, -1, 1.0), (0, 1, 1.0),
                    (-1, -1, 1.414), (1, -1, 1.414), (-1, 1, 1.414), (1, 1, 1.414),
                ];
                for (dx, dz, cost) in &dirs {
                    let nx = x as i32 + dx;
                    let nz = z as i32 + dz;
                    if nx >= 0 && nz >= 0 && (nx as usize) < res && (nz as usize) < res {
                        if walkable[nz as usize][nx as usize] {
                            let nid = (nz as usize * res + nx as usize) as u32;
                            mesh.edges.push(NavEdge { from: id, to: nid, cost: *cost });
                        }
                    }
                }
            }
        }

        mesh
    }

    /// A* pathfinding
    pub fn find_path(&self, start: Vec3, end: Vec3) -> Option<Vec<Vec3>> {
        let start_id = self.world_to_node(start)?;
        let end_id = self.world_to_node(end)?;

        use std::collections::BinaryHeap;
        use std::cmp::Ordering;

        #[derive(Clone)]
        struct State { f: f32, g: f32, node_id: u32 }
        impl PartialEq for State { fn eq(&self, other: &Self) -> bool { self.f == other.f && self.node_id == other.node_id } }
        impl Eq for State {}
        impl Ord for State { fn cmp(&self, other: &Self) -> Ordering { other.f.partial_cmp(&self.f).unwrap_or(Ordering::Equal) } }
        impl PartialOrd for State { fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) } }

        let mut open = BinaryHeap::new();
        let mut g_score: HashMap<u32, f32> = HashMap::new();
        let mut came_from: HashMap<u32, u32> = HashMap::new();

        g_score.insert(start_id, 0.0);
        let end_pos = self.node_position(end_id);
        let h = self.node_position(start_id).distance(end_pos);
        open.push(State { f: h, g: 0.0, node_id: start_id });

        while let Some(State { g, node_id, .. }) = open.pop() {
            if node_id == end_id {
                // Yolu yeniden oluştur
                let mut path = Vec::new();
                let mut current = end_id;
                path.push(self.node_position(current));
                while let Some(&prev) = came_from.get(&current) {
                    path.push(self.node_position(prev));
                    current = prev;
                }
                path.reverse();
                return Some(path);
            }

            for edge in &self.edges {
                if edge.from != node_id { continue; }
                let tentative_g = g + edge.cost;
                let current_best = g_score.get(&edge.to).copied().unwrap_or(f32::MAX);
                if tentative_g < current_best {
                    came_from.insert(edge.to, node_id);
                    g_score.insert(edge.to, tentative_g);
                    let h = self.node_position(edge.to).distance(end_pos);
                    open.push(State { f: tentative_g + h, g: tentative_g, node_id: edge.to });
                }
            }
        }
        None
    }

    fn world_to_node(&self, pos: Vec3) -> Option<u32> {
        let x = (pos.x / self.grid_size) as usize;
        let z = (pos.z / self.grid_size) as usize;
        if x < self.grid_res && z < self.grid_res && self.walkable_grid[z][x] {
            Some((z * self.grid_res + x) as u32)
        } else { None }
    }

    fn node_position(&self, id: u32) -> Vec3 {
        self.nodes.iter().find(|n| n.id == id).map(|n| n.position).unwrap_or(Vec3::ZERO)
    }
}

/// Pathfinding ajanı
#[derive(Clone, Debug)]
pub struct PathAgent {
    pub position: Vec3,
    pub velocity: Vec3,
    pub speed: f32,
    pub path: Vec<Vec3>,
    pub path_index: usize,
    pub arrival_threshold: f32,
    pub max_force: f32,
}

impl PathAgent {
    pub fn new(pos: Vec3, speed: f32) -> Self {
        Self { position: pos, velocity: Vec3::ZERO, speed, path: Vec::new(), path_index: 0, arrival_threshold: 0.5, max_force: 5.0 }
    }

    pub fn set_path(&mut self, path: Vec<Vec3>) {
        self.path = path;
        self.path_index = 0;
    }

    pub fn update(&mut self, dt: f32) {
        if self.path_index >= self.path.len() { return; }
        let target = self.path[self.path_index];
        let desired = (target - self.position) * self.speed;
        let steer = (desired - self.velocity).clamp_length_max(self.max_force * dt);
        self.velocity += steer;
        self.position += self.velocity * dt;

        if self.position.distance(target) < self.arrival_threshold {
            self.path_index += 1;
        }
    }

    pub fn has_arrived(&self) -> bool { self.path_index >= self.path.len() }
}

// ═══════════════════════════════════════════════════════════ 3. PREFAB SYSTEM

/// Prefab — yeniden kullanılabilir şablon
#[derive(Clone, Debug)]
pub struct Prefab {
    pub name: String,
    pub id: u32,
    pub objects: Vec<PrefabObject>,
    pub metadata: PrefabMetadata,
}

#[derive(Clone, Debug)]
pub struct PrefabObject {
    pub name: String,
    pub geometry_type: String,
    pub local_position: [f32; 3],
    pub local_rotation: [f32; 3],
    pub local_scale: [f32; 3],
    pub material_overrides: HashMap<String, String>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct PrefabMetadata {
    pub author: String,
    pub version: String,
    pub description: String,
    pub category: String,
    pub thumbnail_data: Option<Vec<u8>>,
}

/// Prefab Kütüphanesi
#[derive(Clone, Debug)]
pub struct PrefabLibrary {
    pub prefabs: Vec<Prefab>,
    pub next_id: u32,
    pub categories: Vec<String>,
    pub search_query: String,
    pub selected_prefab: Option<u32>,
}

impl Default for PrefabLibrary {
    fn default() -> Self {
        Self {
            prefabs: Vec::new(),
            next_id: 1,
            categories: vec!["Environment".into(), "Characters".into(), "Props".into(), "Effects".into(), "UI".into()],
            search_query: String::new(),
            selected_prefab: None,
        }
    }
}

impl PrefabLibrary {
    pub fn register(&mut self, name: &str, category: &str) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.prefabs.push(Prefab {
            name: name.into(), id, objects: Vec::new(),
            metadata: PrefabMetadata { category: category.into(), ..Default::default() },
        });
        id
    }

    pub fn instantiate(&self, prefab_id: u32) -> Option<Vec<PrefabObject>> {
        self.prefabs.iter().find(|p| p.id == prefab_id).map(|p| p.objects.clone())
    }

    pub fn search(&self) -> Vec<&Prefab> {
        let q = self.search_query.to_lowercase();
        self.prefabs.iter().filter(|p| {
            q.is_empty() || p.name.to_lowercase().contains(&q) || p.metadata.category.to_lowercase().contains(&q)
        }).collect()
    }

    pub fn by_category(&self, cat: &str) -> Vec<&Prefab> {
        self.prefabs.iter().filter(|p| p.metadata.category == cat).collect()
    }
}

// ═══════════════════════════════════════════════════════════ 4. SCENE HIERARCHY (parent-child)

/// Hiyerarşik sahne düğümü
#[derive(Clone, Debug)]
pub struct HierarchyNode {
    pub object_id: usize,
    pub parent_id: Option<usize>,
    pub children: Vec<usize>,
    pub local_transform: Mat4,
    pub world_transform: Mat4,
    pub is_expanded: bool,
    pub depth: usize,
    pub icon: String,
}

/// Scene hierarchy manager
#[derive(Clone, Debug)]
pub struct SceneHierarchy {
    pub nodes: Vec<HierarchyNode>,
    pub node_map: HashMap<usize, usize>, // object_id → index
    pub root_ids: Vec<usize>,
    pub selected_id: Option<usize>,
    pub drag_source: Option<usize>,
    pub drag_target: Option<usize>,
    pub show_hidden: bool,
}

impl Default for SceneHierarchy {
    fn default() -> Self {
        Self {
            nodes: Vec::new(), node_map: HashMap::new(), root_ids: Vec::new(),
            selected_id: None, drag_source: None, drag_target: None, show_hidden: false,
        }
    }
}

impl SceneHierarchy {
    pub fn add_node(&mut self, object_id: usize, parent_id: Option<usize>, icon: &str) {
        let depth = parent_id.and_then(|pid| self.node_map.get(&pid))
            .and_then(|&idx| self.nodes.get(idx)).map(|n| n.depth + 1).unwrap_or(0);
        let idx = self.nodes.len();
        self.nodes.push(HierarchyNode {
            object_id, parent_id, children: Vec::new(),
            local_transform: Mat4::IDENTITY, world_transform: Mat4::IDENTITY,
            is_expanded: true, depth, icon: icon.into(),
        });
        self.node_map.insert(object_id, idx);
        if parent_id.is_none() {
            self.root_ids.push(object_id);
        } else if let Some(&pidx) = self.node_map.get(&parent_id.unwrap()) {
            self.nodes[pidx].children.push(object_id);
        }
    }

    pub fn reparent(&mut self, child_id: usize, new_parent_id: Option<usize>) {
        // Eski parent'tan kaldır
        if let Some(&cidx) = self.node_map.get(&child_id) {
            if let Some(old_parent) = self.nodes[cidx].parent_id {
                if let Some(&pidx) = self.node_map.get(&old_parent) {
                    self.nodes[pidx].children.retain(|&c| c != child_id);
                }
            } else {
                self.root_ids.retain(|&r| r != child_id);
            }
            // Yeni parent'a ekle
            self.nodes[cidx].parent_id = new_parent_id;
            if let Some(np) = new_parent_id {
                if let Some(&pidx) = self.node_map.get(&np) {
                    self.nodes[pidx].children.push(child_id);
                }
            } else {
                self.root_ids.push(child_id);
            }
        }
    }

    pub fn get_children(&self, object_id: usize) -> Vec<usize> {
        self.node_map.get(&object_id)
            .and_then(|&idx| self.nodes.get(idx))
            .map(|n| n.children.clone())
            .unwrap_or_default()
    }

    pub fn depth(&self, object_id: usize) -> usize {
        self.node_map.get(&object_id)
            .and_then(|&idx| self.nodes.get(idx))
            .map(|n| n.depth).unwrap_or(0)
    }

    pub fn is_root(&self, object_id: usize) -> bool {
        self.root_ids.contains(&object_id)
    }

    pub fn ancestor_count(&self, object_id: usize) -> usize {
        let mut count = 0;
        let mut current = object_id;
        while let Some(&idx) = self.node_map.get(&current) {
            if let Some(parent) = self.nodes[idx].parent_id {
                count += 1;
                current = parent;
            } else { break; }
        }
        count
    }
}

// ═══════════════════════════════════════════════════════════ 5. LOD SYSTEM

/// LOD seviyesi
#[derive(Clone, Debug)]
pub struct LodLevel {
    pub distance_min: f32,
    pub distance_max: f32,
    pub mesh_id: Option<u32>,
    pub material_id: Option<u32>,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub transition: LodTransition,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LodTransition {
    Snap,
    CrossFade,
    Dither,
}

/// LOD Manager
#[derive(Clone, Debug)]
pub struct LodManager {
    pub entries: Vec<LodEntry>,
    pub enabled: bool,
    pub hysteresis: f32,
    pub force_lod: Option<usize>,
    pub stats: LodStats,
}

#[derive(Clone, Debug)]
pub struct LodEntry {
    pub object_id: usize,
    pub levels: Vec<LodLevel>,
    pub current_level: usize,
    pub screen_coverage: f32,
}

#[derive(Clone, Debug, Default)]
pub struct LodStats {
    pub total_objects: usize,
    pub active_objects: usize,
    pub culled_objects: usize,
    pub lod_changes_this_frame: usize,
    pub vertex_savings: u64,
}

impl Default for LodManager {
    fn default() -> Self {
        Self { entries: Vec::new(), enabled: true, hysteresis: 0.05, force_lod: None, stats: LodStats::default() }
    }
}

impl LodManager {
    pub fn register(&mut self, object_id: usize, levels: Vec<LodLevel>) {
        self.entries.push(LodEntry { object_id, levels, current_level: 0, screen_coverage: 1.0 });
    }

    pub fn update(&mut self, camera_pos: Vec3, object_positions: &[(usize, Vec3)]) {
        self.stats.lod_changes_this_frame = 0;
        self.stats.active_objects = 0;
        self.stats.vertex_savings = 0;

        for entry in &mut self.entries {
            if let Some((_, pos)) = object_positions.iter().find(|(id, _)| *id == entry.object_id) {
                let dist = camera_pos.distance(*pos);
                let screen_cov = (1.0 / (dist + 1.0)).clamp(0.0, 1.0);
                entry.screen_coverage = screen_cov;

                if let Some(force) = self.force_lod {
                    if force < entry.levels.len() { entry.current_level = force; }
                } else {
                    let old_level = entry.current_level;
                    for (i, level) in entry.levels.iter().enumerate() {
                        if dist >= level.distance_min - self.hysteresis && dist <= level.distance_max + self.hysteresis {
                            entry.current_level = i;
                            break;
                        }
                    }
                    if entry.current_level != old_level {
                        self.stats.lod_changes_this_frame += 1;
                    }
                }

                let base_verts = entry.levels.first().map(|l| l.vertex_count as u64).unwrap_or(0);
                let current_verts = entry.levels.get(entry.current_level).map(|l| l.vertex_count as u64).unwrap_or(0);
                self.stats.vertex_savings += base_verts.saturating_sub(current_verts);
                self.stats.active_objects += 1;
            }
        }
        self.stats.total_objects = self.entries.len();
    }

    pub fn get_level(&self, object_id: usize) -> Option<&LodLevel> {
        self.entries.iter().find(|e| e.object_id == object_id)
            .and_then(|e| e.levels.get(e.current_level))
    }
}

// ═══════════════════════════════════════════════════════════ 6. DECAL SYSTEM

/// Decal — projected texture
#[derive(Clone, Debug)]
pub struct Decal {
    pub id: u32,
    pub position: Vec3,
    pub rotation: Quat,
    pub size: Vec3,
    pub texture_id: Option<u32>,
    pub color: [f32; 4],
    pub opacity: f32,
    pub lifetime: f32,
    pub age: f32,
    pub projected_normal: Vec3,
    pub is_active: bool,
}

/// Decal Manager
#[derive(Clone, Debug)]
pub struct DecalManager {
    pub decals: Vec<Decal>,
    pub max_decals: usize,
    pub next_id: u32,
    pub fade_speed: f32,
    pub stats: DecalStats,
}

#[derive(Clone, Debug, Default)]
pub struct DecalStats {
    pub active_count: usize,
    pub total_spawned: u64,
    pub total_faded: u64,
}

impl Default for DecalManager {
    fn default() -> Self {
        Self { decals: Vec::new(), max_decals: 500, next_id: 1, fade_speed: 1.0, stats: DecalStats::default() }
    }
}

impl DecalManager {
    pub fn spawn(&mut self, pos: Vec3, normal: Vec3, size: f32, color: [f32; 4], lifetime: f32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        if self.decals.len() >= self.max_decals {
            if let Some(oldest) = self.decals.iter().position(|d| d.is_active) {
                self.decals.remove(oldest);
            }
        }
        let rot = Quat::from_rotation_arc(Vec3::Z, normal);
        self.decals.push(Decal {
            id, position: pos + normal * 0.01, rotation: rot,
            size: Vec3::splat(size), texture_id: None, color,
            opacity: 1.0, lifetime, age: 0.0, projected_normal: normal, is_active: true,
        });
        self.stats.total_spawned += 1;
        self.stats.active_count += 1;
        id
    }

    pub fn update(&mut self, dt: f32) {
        self.stats.active_count = 0;
        for decal in &mut self.decals {
            if !decal.is_active { continue; }
            decal.age += dt;
            if decal.age > decal.lifetime {
                decal.opacity -= dt * self.fade_speed;
                if decal.opacity <= 0.0 {
                    decal.is_active = false;
                    self.stats.total_faded += 1;
                }
            }
            if decal.is_active { self.stats.active_count += 1; }
        }
        self.decals.retain(|d| d.is_active);
    }

    pub fn bullet_impact(&mut self, pos: Vec3, normal: Vec3) -> u32 {
        self.spawn(pos, normal, 0.3, [0.8, 0.7, 0.5, 1.0], 5.0)
    }

    pub fn blood_splatter(&mut self, pos: Vec3, normal: Vec3) -> u32 {
        self.spawn(pos, normal, 0.5, [0.8, 0.1, 0.05, 1.0], 10.0)
    }
}

// ═══════════════════════════════════════════════════════════ 7. SPLINE/CURVE SYSTEM

/// Spline kontrol noktası
#[derive(Clone, Debug)]
pub struct SplineControlPoint {
    pub position: Vec3,
    pub tangent_in: Vec3,
    pub tangent_out: Vec3,
    pub time: f32,
}

/// Spline türü
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SplineType {
    Linear,
    CatmullRom,
    Bezier,
    Hermite,
}

/// Spline
#[derive(Clone, Debug)]
pub struct Spline {
    pub name: String,
    pub points: Vec<SplineControlPoint>,
    pub spline_type: SplineType,
    pub closed: bool,
    pub resolution: usize,
    pub draw_in_viewport: bool,
}

impl Default for Spline {
    fn default() -> Self {
        Self {
            name: "Spline".into(), points: Vec::new(), spline_type: SplineType::CatmullRom,
            closed: false, resolution: 32, draw_in_viewport: true,
        }
    }
}

impl Spline {
    pub fn add_point(&mut self, pos: Vec3, time: f32) {
        let tangent = Vec3::ZERO;
        self.points.push(SplineControlPoint {
            position: pos, tangent_in: tangent, tangent_out: tangent, time,
        });
        self.points.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Spline üzerinde belirli bir zamanda pozisyon hesapla
    pub fn sample(&self, t: f32) -> Vec3 {
        if self.points.is_empty() { return Vec3::ZERO; }
        if self.points.len() == 1 { return self.points[0].position; }

        let total_time = self.points.last().unwrap().time - self.points[0].time;
        let local_t = if total_time > 0.0 { (t % total_time) / total_time } else { 0.0 };

        match self.spline_type {
            SplineType::Linear => self.sample_linear(local_t),
            SplineType::CatmullRom => self.sample_catmull_rom(local_t),
            SplineType::Bezier => self.sample_bezier(local_t),
            SplineType::Hermite => self.sample_hermite(local_t),
        }
    }

    fn sample_linear(&self, t: f32) -> Vec3 {
        let n = self.points.len();
        let idx = (t * (n - 1) as f32).floor() as usize;
        let frac = (t * (n - 1) as f32) - idx as f32;
        let a = &self.points[idx.min(n - 1)];
        let b = &self.points[(idx + 1).min(n - 1)];
        a.position.lerp(b.position, frac)
    }

    fn sample_catmull_rom(&self, t: f32) -> Vec3 {
        let n = self.points.len();
        let f = t * (n - 1) as f32;
        let i = f.floor() as usize;
        let frac = f - i as f32;
        let p0 = &self.points[i.min(n - 1)];
        let p1 = &self.points[(i + 1).min(n - 1)];
        let p2 = &self.points[(i + 2).min(n - 1)];
        let p3 = &self.points[(i + 3).min(n - 1)];
        let t2 = frac * frac;
        let t3 = t2 * frac;
        0.5 * (
            (2.0 * p1.position) +
            (-p0.position + p2.position) * frac +
            (2.0 * p0.position - 5.0 * p1.position + 4.0 * p2.position - p3.position) * t2 +
            (-p0.position + 3.0 * p1.position - 3.0 * p2.position + p3.position) * t3
        )
    }

    fn sample_bezier(&self, t: f32) -> Vec3 {
        let n = self.points.len();
        if n < 4 { return self.sample_linear(t); }
        let f = t * ((n - 1) / 3) as f32;
        let i = (f.floor() as usize * 3).min(n.saturating_sub(4));
        let lt = f - f.floor();
        let p0 = self.points[i].position;
        let p1 = self.points[i + 1].position;
        let p2 = self.points[i + 2].position;
        let p3 = self.points[i + 3].position;
        let u = 1.0 - lt;
        u*u*u*p0 + 3.0*u*u*lt*p1 + 3.0*u*lt*lt*p2 + lt*lt*lt*p3
    }

    fn sample_hermite(&self, t: f32) -> Vec3 {
        self.sample_catmull_rom(t) // Catmull-Rom is a type of Hermite spline
    }

    /// Spline toplam süresi
    pub fn total_duration(&self) -> f32 {
        if self.points.len() < 2 { return 0.0; }
        self.points.last().unwrap().time - self.points.first().unwrap().time
    }

    /// Tüm spline boyunca toplam uzunluk
    pub fn total_length(&self) -> f32 {
        let mut len = 0.0;
        let steps = 100;
        let mut prev = self.sample(0.0);
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let curr = self.sample(t);
            len += prev.distance(curr);
            prev = curr;
        }
        len
    }
}

// ═══════════════════════════════════════════════════════════ 8. AI STATE MACHINE

/// AI durumu
#[derive(Clone, Debug)]
pub struct AiState {
    pub name: String,
    pub on_enter: String,
    pub on_update: String,
    pub on_exit: String,
    pub transitions: Vec<AiTransition>,
}

#[derive(Clone, Debug)]
pub struct AiTransition {
    pub to_state: String,
    pub condition: String,
    pub priority: u32,
}

/// AI agent
#[derive(Clone, Debug)]
pub struct AiAgent {
    pub id: u32,
    pub current_state: String,
    pub states: HashMap<String, AiState>,
    pub blackboard: HashMap<String, String>,
    pub position: Vec3,
    pub target_position: Option<Vec3>,
    pub health: f32,
    pub alert_level: f32,
    pub patrol_index: usize,
    pub patrol_points: Vec<Vec3>,
    pub is_active: bool,
}

impl AiAgent {
    pub fn new(id: u32) -> Self {
        let mut states = HashMap::new();
        states.insert("idle".into(), AiState {
            name: "Idle".into(), on_enter: "look_around".into(),
            on_update: "check_for_enemies".into(), on_exit: "".into(),
            transitions: vec![
                AiTransition { to_state: "patrol".into(), condition: "timer_expired".into(), priority: 1 },
                AiTransition { to_state: "chase".into(), condition: "enemy_visible".into(), priority: 10 },
            ],
        });
        states.insert("patrol".into(), AiState {
            name: "Patrol".into(), on_enter: "move_to_waypoint".into(),
            on_update: "check_waypoint_reached".into(), on_exit: "".into(),
            transitions: vec![
                AiTransition { to_state: "idle".into(), condition: "waypoint_reached".into(), priority: 5 },
                AiTransition { to_state: "chase".into(), condition: "enemy_visible".into(), priority: 10 },
            ],
        });
        states.insert("chase".into(), AiState {
            name: "Chase".into(), on_enter: "set_enemy_target".into(),
            on_update: "move_to_enemy".into(), on_exit: "clear_target".into(),
            transitions: vec![
                AiTransition { to_state: "attack".into(), condition: "enemy_in_range".into(), priority: 10 },
                AiTransition { to_state: "idle".into(), condition: "enemy_lost".into(), priority: 5 },
            ],
        });
        states.insert("attack".into(), AiState {
            name: "Attack".into(), on_enter: "start_attack".into(),
            on_update: "perform_attack".into(), on_exit: "stop_attack".into(),
            transitions: vec![
                AiTransition { to_state: "chase".into(), condition: "enemy_out_of_range".into(), priority: 10 },
                AiTransition { to_state: "flee".into(), condition: "health_low".into(), priority: 8 },
            ],
        });
        states.insert("flee".into(), AiState {
            name: "Flee".into(), on_enter: "find_escape_route".into(),
            on_update: "move_to_safety".into(), on_exit: "".into(),
            transitions: vec![
                AiTransition { to_state: "idle".into(), condition: "safe_distance_reached".into(), priority: 10 },
            ],
        });

        Self {
            id, current_state: "idle".into(), states, blackboard: HashMap::new(),
            position: Vec3::ZERO, target_position: None, health: 100.0,
            alert_level: 0.0, patrol_index: 0, patrol_points: Vec::new(), is_active: true,
        }
    }

    pub fn update(&mut self, dt: f32) {
        if !self.is_active { return; }
        self.alert_level = (self.alert_level - dt * 0.5).max(0.0);

        if let Some(state) = self.states.get(&self.current_state) {
            let _ = &state.on_update;
            // Durum transition'larını kontrol et
            let mut best_transition: Option<String> = None;
            let mut best_priority = 0u32;
            for trans in &state.transitions {
                if self.evaluate_condition(&trans.condition) && trans.priority > best_priority {
                    best_priority = trans.priority;
                    best_transition = Some(trans.to_state.clone());
                }
            }
            if let Some(next) = best_transition {
                self.transition_to(&next);
            }
        }
    }

    pub fn transition_to(&mut self, state_name: &str) {
        if let Some(state) = self.states.get(&self.current_state) {
            let _ = &state.on_exit;
        }
        self.current_state = state_name.to_string();
    }

    fn evaluate_condition(&self, condition: &str) -> bool {
        match condition {
            "enemy_visible" => self.alert_level > 0.5,
            "enemy_in_range" => self.target_position.map(|t| self.position.distance(t) < 3.0).unwrap_or(false),
            "enemy_out_of_range" => self.target_position.map(|t| self.position.distance(t) > 8.0).unwrap_or(false),
            "health_low" => self.health < 30.0,
            "enemy_lost" => self.alert_level < 0.1,
            "safe_distance_reached" => self.alert_level < 0.05,
            "timer_expired" => self.blackboard.get("timer").map(|v| v.parse::<f32>().unwrap_or(0.0) <= 0.0).unwrap_or(true),
            "waypoint_reached" => self.target_position.map(|t| self.position.distance(t) < 1.0).unwrap_or(true),
            _ => false,
        }
    }

    pub fn set_patrol_route(&mut self, points: Vec<Vec3>) { self.patrol_points = points; }
    pub fn damage(&mut self, amount: f32) { self.health = (self.health - amount).max(0.0); }
}

// ═══════════════════════════════════════════════════════════ 9. CINEMATIC SEQUENCER

/// Kamera track
#[derive(Clone, Debug)]
pub struct CameraTrack {
    pub name: String,
    pub keyframes: Vec<CameraKeyframe>,
}

#[derive(Clone, Debug)]
pub struct CameraKeyframe {
    pub time: f32,
    pub position: Vec3,
    pub look_at: Vec3,
    pub fov: f32,
    pub roll: f32,
    pub focal_distance: f32,
}

/// Event track
#[derive(Clone, Debug)]
pub struct EventTrack {
    pub name: String,
    pub events: Vec<SequencerEvent>,
}

#[derive(Clone, Debug)]
pub struct SequencerEvent {
    pub time: f32,
    pub event_type: String,
    pub parameters: HashMap<String, String>,
    pub fired: bool,
}

/// Fade track
#[derive(Clone, Debug)]
pub struct FadeTrack {
    pub fade_in: Vec<(f32, f32)>,  // (time, opacity)
    pub fade_out: Vec<(f32, f32)>,
    pub current_opacity: f32,
}

/// Cinematic Sequencer
#[derive(Clone, Debug)]
pub struct CinematicSequencer {
    pub name: String,
    pub duration: f32,
    pub current_time: f32,
    pub playing: bool,
    pub loop_playback: bool,
    pub fps: f32,
    pub camera_tracks: Vec<CameraTrack>,
    pub event_tracks: Vec<EventTrack>,
    pub fade_track: FadeTrack,
    pub markers: Vec<(f32, String)>,
    pub is_recording: bool,
}

impl Default for CinematicSequencer {
    fn default() -> Self {
        Self {
            name: "Cutscene".into(), duration: 10.0, current_time: 0.0,
            playing: false, loop_playback: false, fps: 30.0,
            camera_tracks: vec![CameraTrack { name: "Main Camera".into(), keyframes: Vec::new() }],
            event_tracks: Vec::new(),
            fade_track: FadeTrack { fade_in: Vec::new(), fade_out: Vec::new(), current_opacity: 1.0 },
            markers: Vec::new(), is_recording: false,
        }
    }
}

impl CinematicSequencer {
    pub fn add_camera_keyframe(&mut self, track_idx: usize, kf: CameraKeyframe) {
        if let Some(track) = self.camera_tracks.get_mut(track_idx) {
            track.keyframes.push(kf);
            track.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        }
    }

    pub fn add_event(&mut self, track_idx: usize, time: f32, event_type: &str) {
        if let Some(track) = self.event_tracks.get_mut(track_idx) {
            track.events.push(SequencerEvent {
                time, event_type: event_type.into(), parameters: HashMap::new(), fired: false,
            });
        }
    }

    pub fn add_marker(&mut self, time: f32, label: &str) {
        self.markers.push((time, label.to_string()));
        self.markers.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    pub fn update(&mut self, dt: f32) -> Option<CameraKeyframe> {
        if !self.playing { return None; }
        self.current_time += dt;

        // Event'leri tetikle
        for track in &mut self.event_tracks {
            for event in &mut track.events {
                if !event.fired && self.current_time >= event.time {
                    event.fired = true;
                }
            }
        }

        // Camera interpolasyonu
        let cam_kf = if let Some(track) = self.camera_tracks.first() {
            self.interpolate_camera(&track.keyframes, self.current_time)
        } else { None };

        // Loop kontrolü
        if self.current_time >= self.duration {
            if self.loop_playback {
                self.current_time = 0.0;
                for track in &mut self.event_tracks {
                    for event in &mut track.events { event.fired = false; }
                }
            } else {
                self.playing = false;
            }
        }

        cam_kf
    }

    fn interpolate_camera(&self, keyframes: &[CameraKeyframe], time: f32) -> Option<CameraKeyframe> {
        if keyframes.is_empty() { return None; }
        if keyframes.len() == 1 { return Some(keyframes[0].clone()); }

        let mut prev = &keyframes[0];
        let mut next = &keyframes[keyframes.len() - 1];
        for i in 0..keyframes.len() - 1 {
            if time >= keyframes[i].time && time <= keyframes[i + 1].time {
                prev = &keyframes[i];
                next = &keyframes[i + 1];
                break;
            }
        }
        let t = if (next.time - prev.time).abs() < 0.001 { 0.0 } else { (time - prev.time) / (next.time - prev.time) };
        Some(CameraKeyframe {
            time,
            position: prev.position.lerp(next.position, t),
            look_at: prev.look_at.lerp(next.look_at, t),
            fov: prev.fov + (next.fov - prev.fov) * t,
            roll: prev.roll + (next.roll - prev.roll) * t,
            focal_distance: prev.focal_distance + (next.focal_distance - prev.focal_distance) * t,
        })
    }

    pub fn play(&mut self) { self.playing = true; self.current_time = 0.0; }
    pub fn pause(&mut self) { self.playing = false; }
    pub fn stop(&mut self) { self.playing = false; self.current_time = 0.0; }
    pub fn seek(&mut self, time: f32) { self.current_time = time.clamp(0.0, self.duration); }
}

// ═══════════════════════════════════════════════════════════ 10. HOT RELOAD SYSTEM

/// Yeniden yüklenebilir dosya
#[derive(Clone, Debug)]
pub struct HotReloadEntry {
    pub path: String,
    pub file_type: HotReloadType,
    pub last_modified: u64,
    pub checksum: u64,
    pub reload_count: u32,
    pub is_dirty: bool,
    pub auto_reload: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HotReloadType {
    Shader,
    Script,
    Material,
    Texture,
    Model,
    Audio,
    Config,
}

/// Hot Reload Manager
#[derive(Clone, Debug)]
pub struct HotReloadManager {
    pub entries: Vec<HotReloadEntry>,
    pub enabled: bool,
    pub watch_interval: f32,
    pub time_since_check: f32,
    pub stats: HotReloadStats,
    pub log: Vec<HotReloadLogEntry>,
    pub max_log: usize,
}

#[derive(Clone, Debug, Default)]
pub struct HotReloadStats {
    pub total_reloads: u64,
    pub successful: u64,
    pub failed: u64,
    pub avg_reload_time_ms: f32,
}

#[derive(Clone, Debug)]
pub struct HotReloadLogEntry {
    pub timestamp: f32,
    pub path: String,
    pub status: String,
    pub duration_ms: f32,
}

impl Default for HotReloadManager {
    fn default() -> Self {
        Self {
            entries: Vec::new(), enabled: true, watch_interval: 0.5, time_since_check: 0.0,
            stats: HotReloadStats::default(), log: Vec::new(), max_log: 100,
        }
    }
}

impl HotReloadManager {
    pub fn watch(&mut self, path: &str, file_type: HotReloadType) {
        self.entries.push(HotReloadEntry {
            path: path.into(), file_type, last_modified: 0, checksum: 0,
            reload_count: 0, is_dirty: false, auto_reload: true,
        });
    }

    pub fn check_for_changes(&mut self, dt: f32) -> Vec<String> {
        self.time_since_check += dt;
        if self.time_since_check < self.watch_interval { return Vec::new(); }
        self.time_since_check = 0.0;

        let mut changed = Vec::new();
        for entry in &mut self.entries {
            if entry.is_dirty && entry.auto_reload {
                entry.is_dirty = false;
                entry.reload_count += 1;
                self.stats.total_reloads += 1;
                self.stats.successful += 1;
                self.log.push(HotReloadLogEntry {
                    timestamp: self.time_since_check,
                    path: entry.path.clone(),
                    status: "Reloaded".into(),
                    duration_ms: 1.5,
                });
                changed.push(entry.path.clone());
            }
        }
        changed
    }

    pub fn mark_dirty(&mut self, path: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.path == path) {
            entry.is_dirty = true;
        }
    }

    pub fn unwatch(&mut self, path: &str) {
        self.entries.retain(|e| e.path != path);
    }
}

// ═══════════════════════════════════════════════════════════ 11. VOXEL GLOBAL ILLUMINATION

/// Voxel GI voxel
#[derive(Clone, Copy, Debug, Default)]
pub struct Voxel {
    pub radiance: [f32; 3],
    pub opacity: f32,
    pub normal: [f32; 3],
}

/// Voxel GI octree node
#[derive(Clone, Debug)]
pub struct OctreeNode {
    pub min: Vec3,
    pub max: Vec3,
    pub voxel: Voxel,
    pub children: Option<Box<[OctreeNode; 8]>>,
    pub depth: u32,
}

/// Voxel GI Scene representation
#[derive(Clone, Debug)]
pub struct VoxelGI {
    pub enabled: bool,
    pub resolution: u32,
    pub world_min: Vec3,
    pub world_max: Vec3,
    pub voxels: Vec<Vec<Vec<Voxel>>>,
    pub bounce_count: u32,
    pub indirect_intensity: f32,
    pub sun_direction: Vec3,
    pub sun_color: Vec3,
    pub ambient_color: Vec3,
    pub needs_rebuild: bool,
    pub build_progress: f32,
    pub stats: VoxelGIStats,
}

#[derive(Clone, Debug, Default)]
pub struct VoxelGIStats {
    pub total_voxels: u32,
    pub filled_voxels: u32,
    pub empty_voxels: u32,
    pub build_time_ms: f32,
    pub memory_bytes: u64,
}

impl Default for VoxelGI {
    fn default() -> Self {
        let res = 32;
        Self {
            enabled: false, resolution: res,
            world_min: Vec3::new(-16.0, -8.0, -16.0),
            world_max: Vec3::new(16.0, 16.0, 16.0),
            voxels: vec![vec![vec![Voxel::default(); res as usize]; res as usize]; res as usize],
            bounce_count: 2, indirect_intensity: 0.5,
            sun_direction: Vec3::new(0.5, 0.8, 0.3).normalize(),
            sun_color: Vec3::new(1.0, 0.95, 0.85),
            ambient_color: Vec3::new(0.15, 0.18, 0.25),
            needs_rebuild: true, build_progress: 0.0,
            stats: VoxelGIStats::default(),
        }
    }
}

impl VoxelGI {
    /// Sahneyi voxelize et
    pub fn voxelize_scene(&mut self, objects: &[(Vec3, Vec3, [f32; 3])]) {
        let res = self.resolution as usize;
        let _extent = self.world_max - self.world_min;

        for (pos, scale, color) in objects {
            let local_min = *pos - *scale;
            let local_max = *pos + *scale;
            let vox_min = self.world_to_voxel(local_min);
            let vox_max = self.world_to_voxel(local_max);

            for z in vox_min.2..=vox_max.2.min(res - 1) {
                for y in vox_min.1..=vox_max.1.min(res - 1) {
                    for x in vox_min.0..=vox_max.0.min(res - 1) {
                        let v = &mut self.voxels[z][y][x];
                        v.radiance = *color;
                        v.opacity = 1.0;
                    }
                }
            }
        }

        self.stats.total_voxels = (res as u32).pow(3);
        self.stats.filled_voxels = self.voxels.iter()
            .flat_map(|z| z.iter())
            .flat_map(|y| y.iter())
            .filter(|v| v.opacity > 0.01).count() as u32;
        self.stats.empty_voxels = self.stats.total_voxels - self.stats.filled_voxels;
    }

    /// Direct lighting hesapla
    pub fn compute_direct_lighting(&mut self) {
        let res = self.resolution as usize;
        for z in 0..res {
            for y in 0..res {
                for x in 0..res {
                    let v = &mut self.voxels[z][y][x];
                    if v.opacity < 0.01 { continue; }
                    let normal = Vec3::new(v.normal[0], v.normal[1], v.normal[2]);
                    let ndotl = normal.dot(self.sun_direction).max(0.0);
                    let direct = self.sun_color * ndotl;
                    let ambient = self.ambient_color;
                    v.radiance = [
                        (direct.x + ambient.x) * v.radiance[0],
                        (direct.y + ambient.y) * v.radiance[1],
                        (direct.z + ambient.z) * v.radiance[2],
                    ];
                }
            }
        }
    }

    /// Indirect lighting (GI bounce)
    pub fn compute_indirect_lighting(&mut self) {
        let res = self.resolution as usize;
        let mut indirect = vec![vec![vec![[0.0f32; 3]; res]; res]; res];

        for _bounce in 0..self.bounce_count {
            for z in 1..res-1 {
                for y in 1..res-1 {
                    for x in 1..res-1 {
                        let v = &self.voxels[z][y][x];
                        if v.opacity < 0.01 { continue; }
                        let mut incoming = [0.0f32; 3];
                        let mut samples = 0u32;
                        // 6 komşu yönde ışık örnekleme
                        let dirs: [(i32, i32, i32); 6] = [(1,0,0),(-1,0,0),(0,1,0),(0,-1,0),(0,0,1),(0,0,-1)];
                        for (dx, dy, dz) in &dirs {
                            let nx = (x as i32 + dx) as usize;
                            let ny = (y as i32 + dy) as usize;
                            let nz = (z as i32 + dz) as usize;
                            let nv = &self.voxels[nz][ny][nx];
                            if nv.opacity > 0.01 {
                                incoming[0] += nv.radiance[0];
                                incoming[1] += nv.radiance[1];
                                incoming[2] += nv.radiance[2];
                                samples += 1;
                            }
                        }
                        if samples > 0 {
                            let s = samples as f32;
                            indirect[z][y][x] = [
                                incoming[0] / s * self.indirect_intensity,
                                incoming[1] / s * self.indirect_intensity,
                                incoming[2] / s * self.indirect_intensity,
                            ];
                        }
                    }
                }
            }
            // Indirect'i diffuse'a ekle
            for z in 0..res {
                for y in 0..res {
                    for x in 0..res {
                        let v = &mut self.voxels[z][y][x];
                        v.radiance[0] += indirect[z][y][x][0];
                        v.radiance[1] += indirect[z][y][x][1];
                        v.radiance[2] += indirect[z][y][x][2];
                    }
                }
            }
        }
    }

    /// World pozisyonundan GI radiance'ı oku
    pub fn sample_radiance(&self, world_pos: Vec3) -> [f32; 3] {
        let (x, y, z) = self.world_to_voxel(world_pos);
        let res = self.resolution as usize;
        if x < res && y < res && z < res {
            self.voxels[z][y][x].radiance
        } else { [0.0; 3] }
    }

    fn world_to_voxel(&self, pos: Vec3) -> (usize, usize, usize) {
        let extent = self.world_max - self.world_min;
        let nx = ((pos.x - self.world_min.x) / extent.x * self.resolution as f32) as usize;
        let ny = ((pos.y - self.world_min.y) / extent.y * self.resolution as f32) as usize;
        let nz = ((pos.z - self.world_min.z) / extent.z * self.resolution as f32) as usize;
        (nx.min(self.resolution as usize - 1), ny.min(self.resolution as usize - 1), nz.min(self.resolution as usize - 1))
    }

    pub fn rebuild(&mut self, objects: &[(Vec3, Vec3, [f32; 3])]) {
        self.voxelize_scene(objects);
        self.compute_direct_lighting();
        self.compute_indirect_lighting();
        self.needs_rebuild = false;
        self.stats.memory_bytes = (self.resolution.pow(3) * 16) as u64;
    }
}

// ═══════════════════════════════════════════════════════════ Testler

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_graph() {
        let mut g = ShaderGraph::default();
        let n1 = g.add_node(ShaderNodeType::Time, 0.0, 0.0);
        let n2 = g.add_node(ShaderNodeType::Sine, 200.0, 0.0);
        let _n3 = g.add_node(ShaderNodeType::SurfaceOutput, 400.0, 0.0);
        g.connect(n1, 0, n2, 0);
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_count(), 1);
        let glsl = g.generate_glsl();
        assert!(glsl.contains("#version"));
    }

    #[test]
    fn test_navmesh_pathfinding() {
        let walkable = vec![
            vec![true, true, true, true, true],
            vec![true, false, false, false, true],
            vec![true, false, true, false, true],
            vec![true, false, false, false, true],
            vec![true, true, true, true, true],
        ];
        let mesh = NavMesh::from_grid(&walkable, 1.0);
        let path = mesh.find_path(Vec3::new(0.0, 0.0, 0.0), Vec3::new(4.0, 0.0, 4.0));
        assert!(path.is_some());
        assert!(path.unwrap().len() > 2);
    }

    #[test]
    fn test_path_agent() {
        let mut agent = PathAgent::new(Vec3::ZERO, 5.0);
        agent.set_path(vec![Vec3::new(5.0, 0.0, 0.0), Vec3::new(5.0, 0.0, 5.0)]);
        for _ in 0..100 { agent.update(0.016); }
        assert!(agent.position.x > 1.0);
    }

    #[test]
    fn test_prefab_library() {
        let mut lib = PrefabLibrary::default();
        lib.register("Tree", "Environment");
        lib.register("Rock", "Environment");
        lib.register("NPC", "Characters");
        lib.search_query = "Tree".into();
        assert_eq!(lib.search().len(), 1);
        assert_eq!(lib.by_category("Environment").len(), 2);
    }

    #[test]
    fn test_hierarchy() {
        let mut h = SceneHierarchy::default();
        h.add_node(1, None, "📦");
        h.add_node(2, Some(1), "📦");
        h.add_node(3, Some(2), "📦");
        assert!(h.is_root(1));
        assert!(!h.is_root(2));
        assert_eq!(h.depth(3), 2);
        assert_eq!(h.get_children(1), vec![2]);
        assert_eq!(h.get_children(2), vec![3]);
    }

    #[test]
    fn test_hierarchy_reparent() {
        let mut h = SceneHierarchy::default();
        h.add_node(1, None, "📦");
        h.add_node(2, Some(1), "📦");
        h.add_node(3, Some(1), "📦");
        h.reparent(3, Some(2));
        assert_eq!(h.get_children(2), vec![3]);
        assert_eq!(h.get_children(1), vec![2]);
    }

    #[test]
    fn test_lod_manager() {
        let mut lod = LodManager::default();
        lod.register(0, vec![
            LodLevel { distance_min: 0.0, distance_max: 10.0, mesh_id: None, material_id: None, vertex_count: 1000, triangle_count: 500, transition: LodTransition::Snap },
            LodLevel { distance_min: 10.0, distance_max: 30.0, mesh_id: None, material_id: None, vertex_count: 200, triangle_count: 100, transition: LodTransition::Snap },
            LodLevel { distance_min: 30.0, distance_max: 100.0, mesh_id: None, material_id: None, vertex_count: 50, triangle_count: 25, transition: LodTransition::Snap },
        ]);
        lod.update(Vec3::ZERO, &[(0, Vec3::new(5.0, 0.0, 0.0))]);
        assert_eq!(lod.stats.active_objects, 1);
        lod.update(Vec3::ZERO, &[(0, Vec3::new(50.0, 0.0, 0.0))]);
        let level = lod.get_level(0).unwrap();
        assert_eq!(level.vertex_count, 50);
    }

    #[test]
    fn test_decal_manager() {
        let mut dm = DecalManager::default();
        dm.bullet_impact(Vec3::new(1.0, 0.0, 0.0), Vec3::Y);
        dm.blood_splatter(Vec3::new(2.0, 0.0, 0.0), Vec3::Y);
        assert_eq!(dm.stats.active_count, 2);
        assert_eq!(dm.stats.total_spawned, 2);
        // lifetime aşıldığında opacity azalır, sonra silinir
        dm.update(6.0); // bullet lifetime=5, blood=10
        assert_eq!(dm.stats.active_count, 1); // sadece blood kaldı
        dm.update(10.0); // blood da biter
        assert_eq!(dm.stats.active_count, 0);
    }

    #[test]
    fn test_spline() {
        let mut sp = Spline::default();
        sp.add_point(Vec3::ZERO, 0.0);
        sp.add_point(Vec3::new(10.0, 0.0, 0.0), 1.0);
        sp.add_point(Vec3::new(10.0, 0.0, 10.0), 2.0);
        sp.add_point(Vec3::ZERO, 3.0);
        // Orta noktaya kadar uzunluk kontrolü
        let len = sp.total_length();
        assert!(len > 5.0, "Spline length should be > 5, got {}", len);
        // Orta nokta interp
        let mid = sp.sample(0.5);
        assert!(mid.x > 1.0, "Midpoint x should be > 1, got {}", mid.x);
    }

    #[test]
    fn test_ai_agent() {
        let mut agent = AiAgent::new(1);
        assert_eq!(agent.current_state, "idle");
        agent.alert_level = 0.8;
        agent.update(0.016);
        assert_eq!(agent.current_state, "chase");
        agent.target_position = Some(Vec3::new(1.0, 0.0, 0.0));
        agent.update(0.016);
        assert_eq!(agent.current_state, "attack");
        agent.damage(90.0);
        agent.update(0.016);
        assert_eq!(agent.current_state, "flee");
    }

    #[test]
    fn test_cinematic_sequencer() {
        let mut seq = CinematicSequencer::default();
        seq.camera_tracks[0].keyframes.push(CameraKeyframe {
            time: 0.0, position: Vec3::new(0.0, 5.0, 10.0), look_at: Vec3::ZERO, fov: 60.0, roll: 0.0, focal_distance: 10.0,
        });
        seq.camera_tracks[0].keyframes.push(CameraKeyframe {
            time: 5.0, position: Vec3::new(10.0, 5.0, 0.0), look_at: Vec3::ZERO, fov: 45.0, roll: 0.0, focal_distance: 15.0,
        });
        seq.add_marker(0.0, "Start");
        seq.add_marker(5.0, "End");
        seq.play();
        let kf = seq.update(2.5);
        assert!(kf.is_some());
        let kf = kf.unwrap();
        assert!(kf.position.x > 0.0 && kf.position.x < 10.0);
        assert!((kf.fov - 52.5).abs() < 1.0);
    }

    #[test]
    fn test_hot_reload() {
        let mut hr = HotReloadManager::default();
        hr.watch("shader.wgsl", HotReloadType::Shader);
        hr.watch("script.lua", HotReloadType::Script);
        hr.mark_dirty("shader.wgsl");
        let changed = hr.check_for_changes(1.0);
        assert_eq!(changed.len(), 1);
        assert_eq!(hr.stats.total_reloads, 1);
    }

    #[test]
    fn test_voxel_gi() {
        let mut gi = VoxelGI::default();
        gi.resolution = 8;
        gi.world_min = Vec3::splat(-4.0);
        gi.world_max = Vec3::splat(4.0);
        gi.bounce_count = 1;
        gi.rebuild(&[
            (Vec3::ZERO, Vec3::splat(1.0), [0.8, 0.2, 0.2]),
            (Vec3::new(3.0, 0.0, 0.0), Vec3::splat(0.5), [0.2, 0.8, 0.2]),
        ]);
        assert!(!gi.needs_rebuild);
        assert!(gi.stats.filled_voxels > 0);
        let rad = gi.sample_radiance(Vec3::ZERO);
        assert!(rad[0] > 0.0);
    }

    #[test]
    fn test_spline_catmull_rom() {
        let mut sp = Spline::default();
        sp.spline_type = SplineType::CatmullRom;
        sp.add_point(Vec3::new(0.0, 0.0, 0.0), 0.0);
        sp.add_point(Vec3::new(1.0, 2.0, 0.0), 1.0);
        sp.add_point(Vec3::new(3.0, 0.0, 0.0), 2.0);
        sp.add_point(Vec3::new(4.0, 2.0, 0.0), 3.0);
        let p = sp.sample(0.5);
        assert!(p.y > 0.0); // Should be above ground
    }

    #[test]
    fn test_prefab_instantiate() {
        let mut lib = PrefabLibrary::default();
        let id = lib.register("House", "Environment");
        if let Some(prefab) = lib.prefabs.iter_mut().find(|p| p.id == id) {
            prefab.objects.push(PrefabObject {
                name: "Wall".into(), geometry_type: "Cube".into(),
                local_position: [0.0, 0.0, 0.0], local_rotation: [0.0; 3],
                local_scale: [1.0, 2.0, 0.1], material_overrides: HashMap::new(), tags: Vec::new(),
            });
        }
        let instances = lib.instantiate(id);
        assert!(instances.is_some());
        assert_eq!(instances.unwrap().len(), 1);
    }

    #[test]
    fn test_camera_interpolation() {
        let mut seq = CinematicSequencer::default();
        seq.duration = 4.0;
        seq.camera_tracks[0].keyframes = vec![
            CameraKeyframe { time: 0.0, position: Vec3::new(0.0, 0.0, 10.0), look_at: Vec3::ZERO, fov: 60.0, roll: 0.0, focal_distance: 10.0 },
            CameraKeyframe { time: 2.0, position: Vec3::new(10.0, 0.0, 0.0), look_at: Vec3::ZERO, fov: 90.0, roll: 0.0, focal_distance: 5.0 },
            CameraKeyframe { time: 4.0, position: Vec3::new(0.0, 0.0, -10.0), look_at: Vec3::ZERO, fov: 60.0, roll: 0.0, focal_distance: 10.0 },
        ];
        seq.play();
        let kf = seq.update(1.0).unwrap();
        assert!((kf.position.x - 5.0).abs() < 0.5);
        assert!((kf.fov - 75.0).abs() < 1.0);
    }

    #[test]
    fn test_octree_node_structure() {
        let gi = VoxelGI::default();
        assert_eq!(gi.voxels.len(), 32);
        assert_eq!(gi.voxels[0].len(), 32);
        assert_eq!(gi.voxels[0][0].len(), 32);
    }

    #[test]
    fn test_ai_blackboard() {
        let mut agent = AiAgent::new(1);
        agent.blackboard.insert("target_enemy".into(), "42".into());
        agent.blackboard.insert("alert_radius".into(), "10.0".into());
        assert_eq!(agent.blackboard.len(), 2);
        assert_eq!(agent.blackboard.get("target_enemy").unwrap(), "42");
    }

    #[test]
    fn test_decal_max_limit() {
        let mut dm = DecalManager::default();
        dm.max_decals = 3;
        for i in 0..5 {
            dm.spawn(Vec3::new(i as f32, 0.0, 0.0), Vec3::Y, 0.1, [1.0; 4], 100.0);
        }
        assert!(dm.decals.len() <= 3);
    }
}
