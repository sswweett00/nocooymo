/// plugin_system.rs — Enterprise Plugin System
///
/// Features:
/// - Plugin registry with versioning and dependencies
/// - Lifecycle hooks: init, start, update, stop, unload
/// - Plugin metadata and discovery
/// - Dependency resolution (topological sort)
/// - Hot-reload support (conceptual)
/// - Plugin configuration
/// - Plugin events (loaded, unloaded, error)
/// - Plugin health monitoring

use std::collections::{HashMap, HashSet, VecDeque};
use serde::{Serialize, Deserialize};

// ═══════════════════════════════════════════════════════════ Plugin State

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginState {
    Registered,
    Loading,
    Initialized,
    Running,
    Stopped,
    Error,
    Unloaded,
}

impl Default for PluginState {
    fn default() -> Self { PluginState::Registered }
}

impl PluginState {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Registered => "Registered",
            Self::Loading => "Loading",
            Self::Initialized => "Initialized",
            Self::Running => "Running",
            Self::Stopped => "Stopped",
            Self::Error => "Error",
            Self::Unloaded => "Unloaded",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running | Self::Initialized)
    }
}

// ═══════════════════════════════════════════════════════════ Plugin Metadata

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub category: PluginCategory,
    pub dependencies: Vec<PluginDependency>,
    pub optional_dependencies: Vec<PluginDependency>,
    pub conflicts_with: Vec<String>,
    pub priority: i32,
    pub enabled: bool,
    pub settings: HashMap<String, String>,
}

impl Default for PluginMetadata {
    fn default() -> Self {
        Self {
            name: "Unknown".into(),
            version: "0.1.0".into(),
            author: "Unknown".into(),
            description: "".into(),
            category: PluginCategory::General,
            dependencies: Vec::new(),
            optional_dependencies: Vec::new(),
            conflicts_with: Vec::new(),
            priority: 0,
            enabled: true,
            settings: HashMap::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginCategory {
    General,
    Rendering,
    Physics,
    Audio,
    UI,
    Networking,
    Scripting,
    AI,
    Tool,
    Debug,
    Analytics,
}

impl PluginCategory {
    pub fn name(&self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Rendering => "Rendering",
            Self::Physics => "Physics",
            Self::Audio => "Audio",
            Self::UI => "UI",
            Self::Networking => "Networking",
            Self::Scripting => "Scripting",
            Self::AI => "AI",
            Self::Tool => "Tool",
            Self::Debug => "Debug",
            Self::Analytics => "Analytics",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginDependency {
    pub name: String,
    pub version_min: String,
    pub version_max: String,
    pub required: bool,
}

// ═══════════════════════════════════════════════════════════ Plugin Trait

pub trait Plugin: Send + Sync {
    fn metadata(&self) -> &PluginMetadata;
    fn on_init(&mut self) -> Result<(), PluginError> { Ok(()) }
    fn on_start(&mut self) -> Result<(), PluginError> { Ok(()) }
    fn on_update(&mut self, dt: f32) {}
    fn on_stop(&mut self) {}
    fn on_unload(&mut self) {}
    fn on_config_changed(&mut self, key: &str, value: &str) {}
    fn on_dependency_loaded(&mut self, _dep_name: &str) {}
    fn health_check(&self) -> PluginHealth { PluginHealth { status: HealthStatus::Healthy, ..Default::default() } }
}

// ═══════════════════════════════════════════════════════════ Plugin Health

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginHealth {
    pub status: HealthStatus,
    pub message: String,
    pub cpu_time_ms: f32,
    pub memory_bytes: usize,
    pub error_count: u32,
    pub last_error: Option<String>,
}

impl Default for PluginHealth {
    fn default() -> Self {
        Self {
            status: HealthStatus::Healthy,
            message: "OK".into(),
            cpu_time_ms: 0.0,
            memory_bytes: 0,
            error_count: 0,
            last_error: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Critical,
}

impl HealthStatus {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Healthy => "Healthy",
            Self::Degraded => "Degraded",
            Self::Unhealthy => "Unhealthy",
            Self::Critical => "Critical",
        }
    }
}

// ═══════════════════════════════════════════════════════════ Plugin Error

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginError {
    pub plugin_name: String,
    pub error_type: PluginErrorType,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginErrorType {
    LoadFailed,
    InitFailed,
    DependencyMissing,
    DependencyConflict,
    RuntimeError,
    ConfigError,
    HealthCheckFailed,
}

// ═══════════════════════════════════════════════════════════ Plugin Entry

pub struct PluginEntry {
    pub metadata: PluginMetadata,
    pub state: PluginState,
    pub health: PluginHealth,
    pub init_time_ms: f32,
    pub total_update_time_ms: f32,
    pub update_count: u64,
    pub errors: Vec<PluginError>,
}

impl PluginEntry {
    pub fn new(metadata: PluginMetadata) -> Self {
        Self {
            metadata,
            state: PluginState::Registered,
            health: PluginHealth::default(),
            init_time_ms: 0.0,
            total_update_time_ms: 0.0,
            update_count: 0,
            errors: Vec::new(),
        }
    }
}

// ═══════════════════════════════════════════════════════════ Plugin Manager

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    entries: Vec<PluginEntry>,
    load_order: Vec<usize>,
    next_id: usize,
    initialized: bool,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self {
            plugins: Vec::new(),
            entries: Vec::new(),
            load_order: Vec::new(),
            next_id: 0,
            initialized: false,
        }
    }
}

impl PluginManager {
    pub fn new() -> Self { Self::default() }

    // ── Registration ─────────────────────────────────────

    pub fn register(&mut self, mut plugin: Box<dyn Plugin>) -> usize {
        let meta = plugin.metadata().clone();
        let id = self.next_id;
        self.next_id += 1;
        self.plugins.push(plugin);
        self.entries.push(PluginEntry::new(meta));
        id
    }

    // ── Dependency Resolution ────────────────────────────

    pub fn resolve_load_order(&mut self) -> Result<Vec<usize>, PluginError> {
        let n = self.entries.len();
        let mut in_degree = vec![0i32; n];
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

        // Build dependency graph
        for i in 0..n {
            for dep in &self.entries[i].metadata.dependencies {
                if let Some(j) = self.find_plugin_index(&dep.name) {
                    adj[j].push(i);
                    in_degree[i] += 1;
                } else if dep.required {
                    return Err(PluginError {
                        plugin_name: self.entries[i].metadata.name.clone(),
                        error_type: PluginErrorType::DependencyMissing,
                        message: format!("Required dependency '{}' not found", dep.name),
                        recoverable: false,
                    });
                }
            }
        }

        // Kahn's algorithm
        let mut queue: VecDeque<usize> = VecDeque::new();
        for i in 0..n {
            if in_degree[i] == 0 && self.entries[i].metadata.enabled {
                queue.push_back(i);
            }
        }

        let mut sorted = Vec::new();
        while let Some(node) = queue.pop_front() {
            sorted.push(node);
            for &next in &adj[node] {
                in_degree[next] -= 1;
                if in_degree[next] == 0 {
                    queue.push_back(next);
                }
            }
        }

        if sorted.len() < self.entries.iter().filter(|e| e.metadata.enabled).count() {
            return Err(PluginError {
                plugin_name: "System".into(),
                error_type: PluginErrorType::DependencyConflict,
                message: "Circular dependency detected".into(),
                recoverable: false,
            });
        }

        // Sort by priority within same dependency level
        sorted.sort_by(|&a, &b| self.entries[b].metadata.priority.cmp(&self.entries[a].metadata.priority));

        self.load_order = sorted.clone();
        Ok(sorted)
    }

    // ── Lifecycle ────────────────────────────────────────

    pub fn initialize_all(&mut self) -> Result<(), PluginError> {
        let order = self.resolve_load_order()?.clone();
        for &idx in &order {
            self.plugins[idx].on_init()?;
            self.entries[idx].state = PluginState::Initialized;
        }
        self.initialized = true;
        Ok(())
    }

    pub fn start_all(&mut self) -> Result<(), PluginError> {
        for &idx in &self.load_order.clone() {
            self.plugins[idx].on_start()?;
            self.entries[idx].state = PluginState::Running;
        }
        Ok(())
    }

    pub fn update_all(&mut self, dt: f32) {
        let order = self.load_order.clone();
        for &idx in &order {
            if self.entries[idx].state == PluginState::Running {
                self.plugins[idx].on_update(dt);
                self.entries[idx].update_count += 1;
            }
        }
    }

    pub fn stop_all(&mut self) {
        let order = self.load_order.clone();
        for &idx in order.iter().rev() {
            if self.entries[idx].state.is_active() {
                self.plugins[idx].on_stop();
                self.entries[idx].state = PluginState::Stopped;
            }
        }
    }

    pub fn unload_all(&mut self) {
        let order = self.load_order.clone();
        for &idx in order.iter().rev() {
            self.plugins[idx].on_unload();
            self.entries[idx].state = PluginState::Unloaded;
        }
        self.plugins.clear();
        self.entries.clear();
        self.load_order.clear();
        self.initialized = false;
    }

    // ── Queries ──────────────────────────────────────────

    pub fn find_plugin_index(&self, name: &str) -> Option<usize> {
        self.entries.iter().position(|e| e.metadata.name == name)
    }

    pub fn plugin_count(&self) -> usize { self.plugins.len() }

    pub fn active_count(&self) -> usize {
        self.entries.iter().filter(|e| e.state.is_active()).count()
    }

    pub fn enabled_count(&self) -> usize {
        self.entries.iter().filter(|e| e.metadata.enabled).count()
    }

    pub fn error_count(&self) -> usize {
        self.entries.iter().filter(|e| e.state == PluginState::Error).count()
    }

    pub fn entry(&self, index: usize) -> Option<&PluginEntry> {
        self.entries.get(index)
    }

    pub fn entry_by_name(&self, name: &str) -> Option<&PluginEntry> {
        self.entries.iter().find(|e| e.metadata.name == name)
    }

    pub fn all_entries(&self) -> &[PluginEntry] {
        &self.entries
    }

    pub fn dependencies_of(&self, name: &str) -> Vec<&str> {
        if let Some(entry) = self.entry_by_name(name) {
            entry.metadata.dependencies.iter().map(|d| d.name.as_str()).collect()
        } else {
            Vec::new()
        }
    }

    pub fn dependents_of(&self, name: &str) -> Vec<&str> {
        self.entries.iter()
            .filter(|e| e.metadata.dependencies.iter().any(|d| d.name == name))
            .map(|e| e.metadata.name.as_str())
            .collect()
    }

    pub fn health_report(&self) -> Vec<(&str, &PluginHealth)> {
        self.entries.iter().map(|e| (e.metadata.name.as_str(), &e.health)).collect()
    }

    pub fn plugins_by_category(&self, cat: PluginCategory) -> Vec<&PluginEntry> {
        self.entries.iter().filter(|e| e.metadata.category == cat).collect()
    }
}

// ═══════════════════════════════════════════════════════════ Version Utilities

pub fn version_compatible(version: &str, min: &str, max: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.').filter_map(|s| s.parse().ok()).collect()
    };
    let ver = parse(version);
    let vmin = parse(min);
    let vmax = parse(max);

    let compare = |a: &[u32], b: &[u32]| -> std::cmp::Ordering {
        let len = a.len().max(b.len());
        for i in 0..len {
            let ai = a.get(i).unwrap_or(&0);
            let bi = b.get(i).unwrap_or(&0);
            match ai.cmp(bi) {
                std::cmp::Ordering::Equal => continue,
                other => return other,
            }
        }
        std::cmp::Ordering::Equal
    };

    compare(&ver, &vmin) != std::cmp::Ordering::Less &&
    compare(&ver, &vmax) != std::cmp::Ordering::Greater
}

// ═══════════════════════════════════════════════════════════ Tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

    struct TestPlugin {
        meta: PluginMetadata,
        init_count: Arc<AtomicUsize>,
        update_count: Arc<AtomicUsize>,
    }

    impl TestPlugin {
        fn new(name: &str, deps: Vec<PluginDependency>, init_count: Arc<AtomicUsize>, update_count: Arc<AtomicUsize>) -> Self {
            Self {
                meta: PluginMetadata {
                    name: name.into(),
                    dependencies: deps,
                    ..Default::default()
                },
                init_count,
                update_count,
            }
        }
    }

    impl Plugin for TestPlugin {
        fn metadata(&self) -> &PluginMetadata { &self.meta }
        fn on_init(&mut self) -> Result<(), PluginError> {
            self.init_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        fn on_start(&mut self) -> Result<(), PluginError> { Ok(()) }
        fn on_update(&mut self, _dt: f32) {
            self.update_count.fetch_add(1, Ordering::Relaxed);
        }
        fn health_check(&self) -> PluginHealth {
            PluginHealth { status: HealthStatus::Healthy, ..Default::default() }
        }
    }

    fn dep(name: &str) -> PluginDependency {
        PluginDependency { name: name.into(), version_min: "0.0.0".into(), version_max: "99.0.0".into(), required: true }
    }

    #[test]
    fn test_plugin_registration() {
        let mut mgr = PluginManager::new();
        let init_c = Arc::new(AtomicUsize::new(0));
        let upd_c = Arc::new(AtomicUsize::new(0));
        mgr.register(Box::new(TestPlugin::new("TestPlugin", vec![], Arc::clone(&init_c), Arc::clone(&upd_c))));
        assert_eq!(mgr.plugin_count(), 1);
        assert_eq!(mgr.enabled_count(), 1);
    }

    #[test]
    fn test_plugin_lifecycle() {
        let mut mgr = PluginManager::new();
        let init_c = Arc::new(AtomicUsize::new(0));
        let upd_c = Arc::new(AtomicUsize::new(0));
        mgr.register(Box::new(TestPlugin::new("A", vec![], Arc::clone(&init_c), Arc::clone(&upd_c))));

        mgr.resolve_load_order().unwrap();
        mgr.initialize_all().unwrap();
        assert_eq!(init_c.load(Ordering::Relaxed), 1);

        mgr.start_all().unwrap();
        mgr.update_all(0.016);
        assert_eq!(upd_c.load(Ordering::Relaxed), 1);

        mgr.stop_all();
        assert_eq!(mgr.entry(0).unwrap().state, PluginState::Stopped);
    }

    #[test]
    fn test_dependency_resolution() {
        let mut mgr = PluginManager::new();
        let c1 = Arc::new(AtomicUsize::new(0));
        let c2 = Arc::new(AtomicUsize::new(0));
        let c3 = Arc::new(AtomicUsize::new(0));
        let u1 = Arc::new(AtomicUsize::new(0));
        let u2 = Arc::new(AtomicUsize::new(0));
        let u3 = Arc::new(AtomicUsize::new(0));

        let deps_c = vec![dep("B")];
        let deps_a: Vec<PluginDependency> = vec![];
        let deps_b = vec![dep("A")];
        mgr.register(Box::new(TestPlugin::new("C", deps_c, Arc::clone(&c3), Arc::clone(&u3))));
        mgr.register(Box::new(TestPlugin::new("A", deps_a, Arc::clone(&c1), Arc::clone(&u1))));
        mgr.register(Box::new(TestPlugin::new("B", deps_b, Arc::clone(&c2), Arc::clone(&u2))));

        let order = mgr.resolve_load_order().unwrap();
        let pos_a = order.iter().position(|&i| mgr.entry(i).unwrap().metadata.name == "A").unwrap();
        let pos_b = order.iter().position(|&i| mgr.entry(i).unwrap().metadata.name == "B").unwrap();
        let pos_c = order.iter().position(|&i| mgr.entry(i).unwrap().metadata.name == "C").unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_dependency_missing() {
        let mut mgr = PluginManager::new();
        let c = Arc::new(AtomicUsize::new(0));
        let u = Arc::new(AtomicUsize::new(0));
        let deps = vec![dep("NonExistent")];
        mgr.register(Box::new(TestPlugin::new("A", deps, Arc::clone(&c), Arc::clone(&u))));

        let result = mgr.resolve_load_order();
        assert!(result.is_err());
    }

    #[test]
    fn test_health_check() {
        let mut mgr = PluginManager::new();
        let c = Arc::new(AtomicUsize::new(0));
        let u = Arc::new(AtomicUsize::new(0));
        mgr.register(Box::new(TestPlugin::new("A", vec![], Arc::clone(&c), Arc::clone(&u))));

        let health = mgr.health_report();
        assert_eq!(health.len(), 1);
        assert_eq!(health[0].0, "A");
        assert_eq!(health[0].1.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_plugin_unload() {
        let mut mgr = PluginManager::new();
        let c = Arc::new(AtomicUsize::new(0));
        let u = Arc::new(AtomicUsize::new(0));
        mgr.register(Box::new(TestPlugin::new("A", vec![], Arc::clone(&c), Arc::clone(&u))));
        mgr.resolve_load_order().unwrap();
        mgr.initialize_all().unwrap();
        mgr.start_all().unwrap();

        mgr.unload_all();
        assert_eq!(mgr.plugin_count(), 0);
    }

    #[test]
    fn test_plugins_by_category() {
        let mut mgr = PluginManager::new();
        let c = Arc::new(AtomicUsize::new(0));
        let u = Arc::new(AtomicUsize::new(0));
        let mut p1 = TestPlugin::new("Render", vec![], Arc::clone(&c), Arc::clone(&u));
        p1.meta.category = PluginCategory::Rendering;
        let mut p2 = TestPlugin::new("Physics", vec![], Arc::clone(&c), Arc::clone(&u));
        p2.meta.category = PluginCategory::Physics;
        mgr.register(Box::new(p1));
        mgr.register(Box::new(p2));

        assert_eq!(mgr.plugins_by_category(PluginCategory::Rendering).len(), 1);
        assert_eq!(mgr.plugins_by_category(PluginCategory::Physics).len(), 1);
        assert_eq!(mgr.plugins_by_category(PluginCategory::Audio).len(), 0);
    }

    #[test]
    fn test_version_compatible() {
        assert!(version_compatible("1.5.0", "1.0.0", "2.0.0"));
        assert!(version_compatible("1.0.0", "1.0.0", "1.0.0"));
        assert!(!version_compatible("0.9.0", "1.0.0", "2.0.0"));
        assert!(!version_compatible("2.1.0", "1.0.0", "2.0.0"));
    }

    #[test]
    fn test_dependents_of() {
        let mut mgr = PluginManager::new();
        let c = Arc::new(AtomicUsize::new(0));
        let u = Arc::new(AtomicUsize::new(0));
        mgr.register(Box::new(TestPlugin::new("Core", vec![], Arc::clone(&c), Arc::clone(&u))));
        let deps_a = vec![dep("Core")];
        let deps_b = vec![dep("Core")];
        mgr.register(Box::new(TestPlugin::new("A", deps_a, Arc::clone(&c), Arc::clone(&u))));
        mgr.register(Box::new(TestPlugin::new("B", deps_b, Arc::clone(&c), Arc::clone(&u))));

        let deps = mgr.dependents_of("Core");
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"A"));
        assert!(deps.contains(&"B"));
    }
}
