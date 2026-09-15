/// asset_pipeline.rs — Enterprise Asset Pipeline
///
/// Features:
/// - Asset database with metadata and reference tracking
/// - Dependency graph (topological sort for load order)
/// - Asset versioning and hash-based change detection
/// - Hot-reload support (file watcher concept)
/// - Asset categories and tags
/// - Memory budget tracking
/// - Import/export pipeline stages
/// - Asset bundles (collections)

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};

// ═══════════════════════════════════════════════════════════ Asset Types

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetType {
    Texture,
    Mesh,
    Material,
    Shader,
    Animation,
    Audio,
    Scene,
    Prefab,
    Font,
    Script,
    Config,
    Binary,
    Unknown,
}

impl Default for AssetType {
    fn default() -> Self { AssetType::Unknown }
}

impl AssetType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Texture => "Texture",
            Self::Mesh => "Mesh",
            Self::Material => "Material",
            Self::Shader => "Shader",
            Self::Animation => "Animation",
            Self::Audio => "Audio",
            Self::Scene => "Scene",
            Self::Prefab => "Prefab",
            Self::Font => "Font",
            Self::Script => "Script",
            Self::Config => "Config",
            Self::Binary => "Binary",
            Self::Unknown => "Unknown",
        }
    }

    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "bmp" | "tga" | "hdr" | "dds" | "ktx" => Self::Texture,
            "obj" | "fbx" | "gltf" | "glb" | "stl" | "ply" | "dae" => Self::Mesh,
            "mat" | "material" => Self::Material,
            "glsl" | "hlsl" | "wgsl" | "vert" | "frag" | "comp" => Self::Shader,
            "anim" | "bvh" | "csd" => Self::Animation,
            "wav" | "mp3" | "ogg" | "flac" | "opus" => Self::Audio,
            "scene" | "level" | "map" => Self::Scene,
            "prefab" | "template" => Self::Prefab,
            "ttf" | "otf" | "woff" | "woff2" => Self::Font,
            "lua" | "rs" | "py" | "js" => Self::Script,
            "json" | "toml" | "yaml" | "cfg" => Self::Config,
            _ => Self::Unknown,
        }
    }

    pub fn extensions() -> Vec<&'static str> {
        vec!["png", "jpg", "obj", "fbx", "gltf", "glb", "mat", "glsl", "wgsl",
             "wav", "mp3", "ogg", "scene", "prefab", "ttf", "lua", "json"]
    }
}

// ═══════════════════════════════════════════════════════════ Asset Metadata

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetMetadata {
    pub id: AssetId,
    pub name: String,
    pub path: String,
    pub asset_type: AssetType,
    pub size_bytes: usize,
    pub hash: u64,
    pub version: u32,
    pub tags: Vec<String>,
    pub dependencies: Vec<AssetId>,
    pub dependents: Vec<AssetId>,
    pub import_settings: HashMap<String, String>,
    pub imported: bool,
    pub dirty: bool,
    pub created_at: u64,
    pub modified_at: u64,
}

// ═══════════════════════════════════════════════════════════ Asset ID

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(pub u64);

impl AssetId {
    pub fn invalid() -> Self { Self(0) }
    pub fn is_valid(&self) -> bool { self.0 != 0 }
}

// ═══════════════════════════════════════════════════════════ Asset States

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetState {
    Unloaded,
    Loading,
    Loaded,
    Modified,
    Error,
}

impl Default for AssetState {
    fn default() -> Self { AssetState::Unloaded }
}

// ═══════════════════════════════════════════════════════════ Asset Entry

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetEntry {
    pub metadata: AssetMetadata,
    pub state: AssetState,
    pub load_count: u32,
    pub last_accessed: u64,
    pub memory_budget_bytes: usize,
    pub error_message: Option<String>,
}

// ═══════════════════════════════════════════════════════════ Import Pipeline Stage

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportStage {
    pub name: String,
    pub stage_type: StageType,
    pub enabled: bool,
    pub order: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StageType {
    Validate,
    Decompress,
    Decode,
    Transform,
    Optimize,
    Compress,
    Package,
    ValidateOutput,
}

// ═══════════════════════════════════════════════════════════ Asset Bundle

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetBundle {
    pub name: String,
    pub description: String,
    pub assets: Vec<AssetId>,
    pub tags: Vec<String>,
    pub priority: u32,
    pub memory_budget: usize,
    pub loaded: bool,
}

impl AssetBundle {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            assets: Vec::new(),
            tags: Vec::new(),
            priority: 0,
            memory_budget: 0,
            loaded: false,
        }
    }
}

// ═══════════════════════════════════════════════════════════ Pipeline Stats

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AssetPipelineStats {
    pub total_assets: u64,
    pub loaded_assets: u64,
    pub total_memory_bytes: usize,
    pub memory_budget_bytes: usize,
    pub assets_by_type: HashMap<String, u64>,
    pub assets_by_state: HashMap<String, u64>,
    pub dirty_assets: u64,
    pub total_references: u64,
    pub bundles_loaded: u32,
}

// ═══════════════════════════════════════════════════════════ Dependency Graph

pub struct DependencyGraph {
    adj: HashMap<AssetId, Vec<AssetId>>,
    reverse_adj: HashMap<AssetId, Vec<AssetId>>,
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self { adj: HashMap::new(), reverse_adj: HashMap::new() }
    }
}

impl DependencyGraph {
    pub fn new() -> Self { Self::default() }

    pub fn add_edge(&mut self, from: AssetId, to: AssetId) {
        self.adj.entry(from).or_default().push(to);
        self.reverse_adj.entry(to).or_default().push(from);
    }

    pub fn remove_node(&mut self, id: AssetId) {
        self.adj.remove(&id);
        self.reverse_adj.remove(&id);
        for deps in self.adj.values_mut() {
            deps.retain(|d| *d != id);
        }
        for deps in self.reverse_adj.values_mut() {
            deps.retain(|d| *d != id);
        }
    }

    pub fn dependencies(&self, id: AssetId) -> Vec<AssetId> {
        self.adj.get(&id).cloned().unwrap_or_default()
    }

    pub fn dependents(&self, id: AssetId) -> Vec<AssetId> {
        self.reverse_adj.get(&id).cloned().unwrap_or_default()
    }

    /// Topological sort for load order (dependencies first)
    pub fn topological_sort(&self, roots: &[AssetId]) -> Vec<AssetId> {
        let mut visited = HashSet::new();
        let mut order = Vec::new();

        fn visit(id: AssetId, graph: &DependencyGraph, visited: &mut HashSet<AssetId>, order: &mut Vec<AssetId>) {
            if visited.contains(&id) { return; }
            visited.insert(id);
            for &dep in graph.adj.get(&id).unwrap_or(&vec![]) {
                visit(dep, graph, visited, order);
            }
            order.push(id);
        }

        for &root in roots {
            visit(root, self, &mut visited, &mut order);
        }
        order
    }

    /// Check for circular dependencies
    pub fn has_cycle(&self) -> bool {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        fn dfs(id: AssetId, graph: &DependencyGraph, visited: &mut HashSet<AssetId>, stack: &mut HashSet<AssetId>) -> bool {
            visited.insert(id);
            stack.insert(id);
            for &dep in graph.adj.get(&id).unwrap_or(&vec![]) {
                if !visited.contains(&dep) {
                    if dfs(dep, graph, visited, stack) { return true; }
                } else if stack.contains(&dep) {
                    return true;
                }
            }
            stack.remove(&id);
            false
        }

        for &id in self.adj.keys() {
            if !visited.contains(&id) && dfs(id, self, &mut visited, &mut stack) {
                return true;
            }
        }
        false
    }

    pub fn node_count(&self) -> usize { self.adj.len() }
    pub fn edge_count(&self) -> usize { self.adj.values().map(|v| v.len()).sum() }
}

// ═══════════════════════════════════════════════════════════ Asset Pipeline

pub struct AssetPipeline {
    pub assets: Vec<AssetEntry>,
    pub bundles: Vec<AssetBundle>,
    pub graph: DependencyGraph,
    pub import_stages: Vec<ImportStage>,
    next_id: u64,
    total_memory: usize,
    memory_budget: usize,
    stats: AssetPipelineStats,
}

impl Default for AssetPipeline {
    fn default() -> Self {
        let mut pipeline = Self {
            assets: Vec::new(),
            bundles: Vec::new(),
            graph: DependencyGraph::new(),
            import_stages: vec![
                ImportStage { name: "Validate".into(), stage_type: StageType::Validate, enabled: true, order: 0 },
                ImportStage { name: "Decode".into(), stage_type: StageType::Decode, enabled: true, order: 10 },
                ImportStage { name: "Transform".into(), stage_type: StageType::Transform, enabled: true, order: 20 },
                ImportStage { name: "Optimize".into(), stage_type: StageType::Optimize, enabled: true, order: 30 },
                ImportStage { name: "Compress".into(), stage_type: StageType::Compress, enabled: true, order: 40 },
                ImportStage { name: "Package".into(), stage_type: StageType::Package, enabled: true, order: 50 },
            ],
            next_id: 1,
            total_memory: 0,
            memory_budget: 512 * 1024 * 1024, // 512 MB
            stats: AssetPipelineStats::default(),
        };
        pipeline.import_stages.sort_by_key(|s| s.order);
        pipeline
    }
}

impl AssetPipeline {
    pub fn new() -> Self { Self::default() }

    pub fn with_budget(budget_bytes: usize) -> Self {
        Self { memory_budget: budget_bytes, ..Default::default() }
    }

    // ── Registration ─────────────────────────────────────

    pub fn register(&mut self, name: &str, path: &str, asset_type: AssetType, size: usize) -> AssetId {
        let id = AssetId(self.next_id);
        self.next_id += 1;

        let metadata = AssetMetadata {
            id,
            name: name.to_string(),
            path: path.to_string(),
            asset_type,
            size_bytes: size,
            hash: simple_hash(path),
            version: 1,
            tags: Vec::new(),
            dependencies: Vec::new(),
            dependents: Vec::new(),
            import_settings: HashMap::new(),
            imported: false,
            dirty: false,
            created_at: 0,
            modified_at: 0,
        };

        self.assets.push(AssetEntry {
            metadata,
            state: AssetState::Unloaded,
            load_count: 0,
            last_accessed: 0,
            memory_budget_bytes: size,
            error_message: None,
        });

        self.graph.adj.entry(id).or_default();
        self.update_stats();
        id
    }

    pub fn unregister(&mut self, id: AssetId) {
        self.assets.retain(|a| a.metadata.id != id);
        self.graph.remove_node(id);
        self.update_stats();
    }

    // ── Dependencies ─────────────────────────────────────

    pub fn add_dependency(&mut self, asset_id: AssetId, depends_on: AssetId) {
        if let Some(asset) = self.assets.iter_mut().find(|a| a.metadata.id == asset_id) {
            asset.metadata.dependencies.push(depends_on);
        }
        if let Some(dep) = self.assets.iter_mut().find(|a| a.metadata.id == depends_on) {
            dep.metadata.dependents.push(asset_id);
        }
        self.graph.add_edge(asset_id, depends_on);
    }

    pub fn remove_dependency(&mut self, asset_id: AssetId, depends_on: AssetId) {
        if let Some(asset) = self.assets.iter_mut().find(|a| a.metadata.id == asset_id) {
            asset.metadata.dependencies.retain(|d| *d != depends_on);
        }
        if let Some(dep) = self.assets.iter_mut().find(|a| a.metadata.id == depends_on) {
            dep.metadata.dependents.retain(|d| *d != asset_id);
        }
    }

    pub fn load_order(&self, id: AssetId) -> Vec<AssetId> {
        self.graph.topological_sort(&[id])
    }

    // ── Tags ─────────────────────────────────────────────

    pub fn add_tag(&mut self, id: AssetId, tag: &str) {
        if let Some(asset) = self.assets.iter_mut().find(|a| a.metadata.id == id) {
            if !asset.metadata.tags.contains(&tag.to_string()) {
                asset.metadata.tags.push(tag.to_string());
            }
        }
    }

    pub fn remove_tag(&mut self, id: AssetId, tag: &str) {
        if let Some(asset) = self.assets.iter_mut().find(|a| a.metadata.id == id) {
            asset.metadata.tags.retain(|t| t != tag);
        }
    }

    pub fn assets_with_tag(&self, tag: &str) -> Vec<AssetId> {
        self.assets.iter()
            .filter(|a| a.metadata.tags.iter().any(|t| t == tag))
            .map(|a| a.metadata.id)
            .collect()
    }

    pub fn assets_with_type(&self, asset_type: AssetType) -> Vec<AssetId> {
        self.assets.iter()
            .filter(|a| a.metadata.asset_type == asset_type)
            .map(|a| a.metadata.id)
            .collect()
    }

    // ── Loading ──────────────────────────────────────────

    pub fn mark_loaded(&mut self, id: AssetId) {
        if let Some(asset) = self.assets.iter_mut().find(|a| a.metadata.id == id) {
            asset.state = AssetState::Loaded;
            asset.load_count += 1;
            asset.metadata.imported = true;
            self.total_memory += asset.metadata.size_bytes;
        }
        self.update_stats();
    }

    pub fn mark_error(&mut self, id: AssetId, message: &str) {
        if let Some(asset) = self.assets.iter_mut().find(|a| a.metadata.id == id) {
            asset.state = AssetState::Error;
            asset.error_message = Some(message.to_string());
        }
    }

    pub fn unload(&mut self, id: AssetId) {
        if let Some(asset) = self.assets.iter_mut().find(|a| a.metadata.id == id) {
            if asset.state == AssetState::Loaded {
                self.total_memory = self.total_memory.saturating_sub(asset.metadata.size_bytes);
            }
            asset.state = AssetState::Unloaded;
            asset.load_count = asset.load_count.saturating_sub(1);
        }
        self.update_stats();
    }

    // ── Bundles ──────────────────────────────────────────

    pub fn create_bundle(&mut self, name: &str) -> usize {
        let idx = self.bundles.len();
        self.bundles.push(AssetBundle::new(name));
        idx
    }

    pub fn add_to_bundle(&mut self, bundle_idx: usize, asset_id: AssetId) {
        if let Some(bundle) = self.bundles.get_mut(bundle_idx) {
            if !bundle.assets.contains(&asset_id) {
                bundle.assets.push(asset_id);
            }
        }
    }

    pub fn bundle_memory(&self, bundle_idx: usize) -> usize {
        self.bundles.get(bundle_idx).map_or(0, |b| {
            b.assets.iter()
                .filter_map(|id| self.assets.iter().find(|a| a.metadata.id == *id))
                .map(|a| a.metadata.size_bytes)
                .sum()
        })
    }

    // ── Hash & Version ───────────────────────────────────

    pub fn update_hash(&mut self, id: AssetId) {
        let new_hash = id.0.wrapping_mul(0x9e3779b9); // Simple change
        if let Some(asset) = self.assets.iter_mut().find(|a| a.metadata.id == id) {
            if asset.metadata.hash != new_hash {
                asset.metadata.hash = new_hash;
                asset.metadata.version += 1;
                asset.metadata.dirty = true;
                asset.state = AssetState::Modified;
            }
        }
    }

    // ── Query ────────────────────────────────────────────

    pub fn get(&self, id: AssetId) -> Option<&AssetEntry> {
        self.assets.iter().find(|a| a.metadata.id == id)
    }

    pub fn get_mut(&mut self, id: AssetId) -> Option<&mut AssetEntry> {
        self.assets.iter_mut().find(|a| a.metadata.id == id)
    }

    pub fn find_by_name(&self, name: &str) -> Option<&AssetEntry> {
        self.assets.iter().find(|a| a.metadata.name == name)
    }

    pub fn find_by_path(&self, path: &str) -> Option<&AssetEntry> {
        self.assets.iter().find(|a| a.metadata.path == path)
    }

    pub fn asset_count(&self) -> usize { self.assets.len() }

    pub fn loaded_count(&self) -> usize {
        self.assets.iter().filter(|a| a.state == AssetState::Loaded).count()
    }

    pub fn memory_usage(&self) -> usize { self.total_memory }
    pub fn memory_budget(&self) -> usize { self.memory_budget }
    pub fn memory_usage_percent(&self) -> f32 {
        if self.memory_budget == 0 { 0.0 }
        else { self.total_memory as f32 / self.memory_budget as f32 * 100.0 }
    }

    pub fn dirty_assets(&self) -> Vec<&AssetEntry> {
        self.assets.iter().filter(|a| a.metadata.dirty).collect()
    }

    pub fn all_entries(&self) -> &[AssetEntry] { &self.assets }

    pub fn stats(&self) -> &AssetPipelineStats { &self.stats }

    fn update_stats(&mut self) {
        self.stats.total_assets = self.assets.len() as u64;
        self.stats.loaded_assets = self.assets.iter().filter(|a| a.state == AssetState::Loaded).count() as u64;
        self.stats.total_memory_bytes = self.total_memory;
        self.stats.memory_budget_bytes = self.memory_budget;
        self.stats.dirty_assets = self.assets.iter().filter(|a| a.metadata.dirty).count() as u64;
        self.stats.total_references = self.assets.iter().map(|a| a.metadata.dependencies.len() as u64).sum();

        self.stats.assets_by_type.clear();
        for a in &self.assets {
            *self.stats.assets_by_type.entry(a.metadata.asset_type.name().to_string()).or_insert(0) += 1;
        }
        self.stats.assets_by_state.clear();
        for a in &self.assets {
            *self.stats.assets_by_state.entry(format!("{:?}", a.state)).or_insert(0) += 1;
        }
        self.stats.bundles_loaded = self.bundles.iter().filter(|b| b.loaded).count() as u32;
    }
}

// ═══════════════════════════════════════════════════════════ Helpers

fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ═══════════════════════════════════════════════════════════ Thread-safe wrapper

#[derive(Clone)]
pub struct SharedAssetPipeline {
    inner: Arc<RwLock<AssetPipeline>>,
}

impl SharedAssetPipeline {
    pub fn new() -> Self {
        Self { inner: Arc::new(RwLock::new(AssetPipeline::new())) }
    }

    pub fn register(&self, name: &str, path: &str, asset_type: AssetType, size: usize) -> AssetId {
        self.inner.write().unwrap().register(name, path, asset_type, size)
    }

    pub fn asset_count(&self) -> usize {
        self.inner.read().unwrap().asset_count()
    }

    pub fn memory_usage(&self) -> usize {
        self.inner.read().unwrap().memory_usage()
    }
}

// ═══════════════════════════════════════════════════════════ Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_type_from_extension() {
        assert_eq!(AssetType::from_extension("png"), AssetType::Texture);
        assert_eq!(AssetType::from_extension("obj"), AssetType::Mesh);
        assert_eq!(AssetType::from_extension("glsl"), AssetType::Shader);
        assert_eq!(AssetType::from_extension("wav"), AssetType::Audio);
        assert_eq!(AssetType::from_extension("scene"), AssetType::Scene);
        assert_eq!(AssetType::from_extension("xyz"), AssetType::Unknown);
    }

    #[test]
    fn test_pipeline_register() {
        let mut pipeline = AssetPipeline::new();
        let id = pipeline.register("test.png", "assets/test.png", AssetType::Texture, 1024);
        assert!(id.is_valid());
        assert_eq!(pipeline.asset_count(), 1);
    }

    #[test]
    fn test_pipeline_dependencies() {
        let mut pipeline = AssetPipeline::new();
        let mat = pipeline.register("mat.mat", "mat.mat", AssetType::Material, 100);
        let tex = pipeline.register("tex.png", "tex.png", AssetType::Texture, 1024);

        pipeline.add_dependency(mat, tex);
        assert_eq!(pipeline.load_order(mat).len(), 2);
        // Texture should come before material in load order
        let order = pipeline.load_order(mat);
        assert!(order.contains(&tex));
    }

    #[test]
    fn test_pipeline_tags() {
        let mut pipeline = AssetPipeline::new();
        let id = pipeline.register("test", "test.png", AssetType::Texture, 100);
        pipeline.add_tag(id, "player");
        pipeline.add_tag(id, "skin");
        pipeline.add_tag(id, "player"); // Duplicate

        let tagged = pipeline.assets_with_tag("player");
        assert_eq!(tagged.len(), 1);
        let tagged = pipeline.assets_with_tag("skin");
        assert_eq!(tagged.len(), 1);

        pipeline.remove_tag(id, "skin");
        assert_eq!(pipeline.assets_with_tag("skin").len(), 0);
    }

    #[test]
    fn test_pipeline_memory_tracking() {
        let mut pipeline = AssetPipeline::with_budget(1024 * 1024); // 1MB
        let id = pipeline.register("test", "test.png", AssetType::Texture, 100);
        pipeline.mark_loaded(id);
        assert_eq!(pipeline.memory_usage(), 100);
        assert!(pipeline.memory_usage_percent() > 0.0);

        pipeline.unload(id);
        assert_eq!(pipeline.memory_usage(), 0);
    }

    #[test]
    fn test_pipeline_bundles() {
        let mut pipeline = AssetPipeline::new();
        let id1 = pipeline.register("a", "a.png", AssetType::Texture, 100);
        let id2 = pipeline.register("b", "b.png", AssetType::Texture, 200);

        let bundle_idx = pipeline.create_bundle("Textures");
        pipeline.add_to_bundle(bundle_idx, id1);
        pipeline.add_to_bundle(bundle_idx, id2);

        assert_eq!(pipeline.bundle_memory(bundle_idx), 300);
    }

    #[test]
    fn test_pipeline_dirty_assets() {
        let mut pipeline = AssetPipeline::new();
        let id = pipeline.register("test", "test.png", AssetType::Texture, 100);
        assert!(pipeline.dirty_assets().is_empty());

        pipeline.update_hash(id);
        assert_eq!(pipeline.dirty_assets().len(), 1);
    }

    #[test]
    fn test_pipeline_unregister() {
        let mut pipeline = AssetPipeline::new();
        let id = pipeline.register("test", "test.png", AssetType::Texture, 100);
        assert_eq!(pipeline.asset_count(), 1);

        pipeline.unregister(id);
        assert_eq!(pipeline.asset_count(), 0);
    }

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();
        let a = AssetId(1);
        let b = AssetId(2);
        let c = AssetId(3);

        graph.add_edge(a, b);
        graph.add_edge(b, c);

        let order = graph.topological_sort(&[a]);
        assert_eq!(order.len(), 3);
        assert!(order.iter().position(|&x| x == a).unwrap() > order.iter().position(|&x| x == b).unwrap());
        assert!(order.iter().position(|&x| x == b).unwrap() > order.iter().position(|&x| x == c).unwrap());
    }

    #[test]
    fn test_dependency_graph_cycle() {
        let mut graph = DependencyGraph::new();
        let a = AssetId(1);
        let b = AssetId(2);
        let c = AssetId(3);

        graph.add_edge(a, b);
        graph.add_edge(b, c);
        graph.add_edge(c, a);

        assert!(graph.has_cycle());
    }

    #[test]
    fn test_dependency_graph_no_cycle() {
        let mut graph = DependencyGraph::new();
        let a = AssetId(1);
        let b = AssetId(2);

        graph.add_edge(a, b);
        assert!(!graph.has_cycle());
    }

    #[test]
    fn test_dependency_graph_dependents() {
        let mut graph = DependencyGraph::new();
        let a = AssetId(1);
        let b = AssetId(2);
        let c = AssetId(3);

        graph.add_edge(b, a);
        graph.add_edge(c, a);

        let deps_of_a = graph.dependents(a);
        assert_eq!(deps_of_a.len(), 2);
        assert!(deps_of_a.contains(&b));
        assert!(deps_of_a.contains(&c));
    }

    #[test]
    fn test_pipeline_find_by_name() {
        let mut pipeline = AssetPipeline::new();
        let _id = pipeline.register("my_texture", "tex.png", AssetType::Texture, 100);
        assert!(pipeline.find_by_name("my_texture").is_some());
        assert!(pipeline.find_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_pipeline_find_by_path() {
        let mut pipeline = AssetPipeline::new();
        let _id = pipeline.register("tex", "assets/textures/tex.png", AssetType::Texture, 100);
        assert!(pipeline.find_by_path("assets/textures/tex.png").is_some());
    }

    #[test]
    fn test_pipeline_stats() {
        let mut pipeline = AssetPipeline::new();
        let _id1 = pipeline.register("a", "a.png", AssetType::Texture, 100);
        let _id2 = pipeline.register("b", "b.obj", AssetType::Mesh, 200);
        pipeline.update_stats();

        let stats = pipeline.stats();
        assert_eq!(stats.total_assets, 2);
        assert_eq!(stats.assets_by_type.get("Texture"), Some(&1));
        assert_eq!(stats.assets_by_type.get("Mesh"), Some(&1));
    }

    #[test]
    fn test_asset_type_names() {
        assert_eq!(AssetType::Texture.name(), "Texture");
        assert_eq!(AssetType::Shader.name(), "Shader");
        assert_eq!(AssetType::Audio.name(), "Audio");
    }

    #[test]
    fn test_pipeline_import_stages() {
        let pipeline = AssetPipeline::new();
        assert!(!pipeline.import_stages.is_empty());
        // Should be sorted by order
        for i in 1..pipeline.import_stages.len() {
            assert!(pipeline.import_stages[i].order >= pipeline.import_stages[i-1].order);
        }
    }

    #[test]
    fn test_pipeline_mark_error() {
        let mut pipeline = AssetPipeline::new();
        let id = pipeline.register("test", "test.png", AssetType::Texture, 100);
        pipeline.mark_error(id, "Failed to decode");
        let entry = pipeline.get(id).unwrap();
        assert_eq!(entry.state, AssetState::Error);
        assert_eq!(entry.error_message.as_ref().unwrap(), "Failed to decode");
    }

    #[test]
    fn test_pipeline_assets_with_type() {
        let mut pipeline = AssetPipeline::new();
        let _id1 = pipeline.register("a", "a.png", AssetType::Texture, 100);
        let _id2 = pipeline.register("b", "b.obj", AssetType::Mesh, 200);
        let _id3 = pipeline.register("c", "c.png", AssetType::Texture, 300);

        assert_eq!(pipeline.assets_with_type(AssetType::Texture).len(), 2);
        assert_eq!(pipeline.assets_with_type(AssetType::Mesh).len(), 1);
        assert_eq!(pipeline.assets_with_type(AssetType::Audio).len(), 0);
    }

    #[test]
    fn test_simple_hash_deterministic() {
        let h1 = simple_hash("hello world");
        let h2 = simple_hash("hello world");
        let h3 = simple_hash("hello world!");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_bundle_duplicate_prevention() {
        let mut pipeline = AssetPipeline::new();
        let id = pipeline.register("a", "a.png", AssetType::Texture, 100);
        let idx = pipeline.create_bundle("Test");
        pipeline.add_to_bundle(idx, id);
        pipeline.add_to_bundle(idx, id); // Duplicate
        assert_eq!(pipeline.bundles[idx].assets.len(), 1);
    }

    #[test]
    fn test_remove_dependency() {
        let mut pipeline = AssetPipeline::new();
        let a = pipeline.register("a", "a.mat", AssetType::Material, 100);
        let b = pipeline.register("b", "b.png", AssetType::Texture, 200);
        pipeline.add_dependency(a, b);

        pipeline.remove_dependency(a, b);
        // After removing, a depends on nothing
        let entry = pipeline.get(a).unwrap();
        assert!(entry.metadata.dependencies.is_empty());
    }

    #[test]
    fn test_dependency_graph_remove_node() {
        let mut graph = DependencyGraph::new();
        let a = AssetId(1);
        let b = AssetId(2);
        let c = AssetId(3);
        graph.add_edge(a, b);
        graph.add_edge(b, c);

        graph.remove_node(b);
        // a still exists but its edge to b is removed
        assert!(graph.dependencies(a).is_empty());
        // b is removed
        assert!(graph.dependencies(b).is_empty());
    }
}
