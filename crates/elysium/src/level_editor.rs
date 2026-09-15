
/// level_editor.rs — Elysium Level Editor
/// Tam teşekküllü sahne düzenleme editörü.
///
/// Özellikler:
/// - Entity palette: drag & drop ile nesne yerleştirme
/// - Tool modes: Select, Place, Move, Rotate, Scale, Paint, Erase
/// - Prefab sistemi: şablon kaydet/yükle
/// - Spawn points: başlangıç noktaları, checkpoint'ler
/// - Play Mode: düzenle→oyna geçişi, runtime test
/// - Save/Export: JSON + binary format, export to game
/// - Undo/Redo: unlimited history stack
/// - Grid snap: snap-to-grid toggle
/// - Layers: katman sistemi (visibility toggle)
/// - Multi-select: shift+click ile çoklu seçim
/// - Alignment tools: hizalama, dağılma, eşit aralık
/// - Search/Filtre: entity arama, tag bazlı filtre
/// - Keyboard shortcuts: tüm araçlar için kısayol

use crate::renderer::*;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque};
use glam::Vec3;

// ═══════════════════════════════════════════════════════════ Tool Modes

/// Editor tool modes
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolMode {
    /// Varsayılan seçim aracı
    Select,
    /// Sahneye yeni nesne yerleştir
    Place,
    /// Nesneyi taşı
    Move,
    /// Nesneyi döndür
    Rotate,
    /// Nesneyi ölçekle
    Scale,
    /// Boyama aracı (texture/material)
    Paint,
    /// Silme aracı
    Erase,
    /// Lineer clone (ctrl+drag)
    Duplicate,
    /// Measure aracı (mesafe ölçümü)
    Measure,
}

impl ToolMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Select => "Select", Self::Place => "Place", Self::Move => "Move",
            Self::Rotate => "Rotate", Self::Scale => "Scale", Self::Paint => "Paint",
            Self::Erase => "Erase", Self::Duplicate => "Duplicate", Self::Measure => "Measure",
        }
    }
    pub fn shortcut(self) -> char {
        match self {
            Self::Select => 'Q', Self::Place => 'P', Self::Move => 'G',
            Self::Rotate => 'R', Self::Scale => 'S', Self::Paint => 'B',
            Self::Erase => 'X', Self::Duplicate => 'D', Self::Measure => 'M',
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            Self::Select => "⬚", Self::Place => "+", Self::Move => "✥",
            Self::Rotate => "↻", Self::Scale => "⇔", Self::Paint => "🎨",
            Self::Erase => "✕", Self::Duplicate => "⧉", Self::Measure => "📏",
        }
    }
}

// ═══════════════════════════════════════════════════════════ Entity Palette

/// Palette item for entity placement
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaletteItem {
    pub id: String,
    pub name: String,
    pub category: PaletteCategory,
    pub geometry: GeometryType,
    pub default_scale: [f32; 3],
    pub default_material: PaletteMaterial,
    pub tags: Vec<String>,
    pub icon: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaletteCategory {
    Primitives,
    Environment,
    Characters,
    Props,
    Triggers,
    Lights,
    SpawnPoints,
    Vehicles,
    Effects,
    Custom(String),
}

impl PaletteCategory {
    pub fn name(&self) -> &str {
        match self {
            Self::Primitives => "Primitives",
            Self::Environment => "Environment",
            Self::Characters => "Characters",
            Self::Props => "Props",
            Self::Triggers => "Triggers",
            Self::Lights => "Lights",
            Self::SpawnPoints => "Spawn Points",
            Self::Vehicles => "Vehicles",
            Self::Effects => "Effects",
            Self::Custom(n) => n,
        }
    }
    pub fn icon(&self) -> &str {
        match self {
            Self::Primitives => "◻", Self::Environment => "🌿", Self::Characters => "🧑",
            Self::Props => "🪑", Self::Triggers => "⚡", Self::Lights => "💡",
            Self::SpawnPoints => "🚩", Self::Vehicles => "🚗", Self::Effects => "✨",
            Self::Custom(_) => "📦",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaletteMaterial {
    pub albedo: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub emissive_strength: f32,
}

impl Default for PaletteMaterial {
    fn default() -> Self {
        Self { albedo: [1.0; 3], metallic: 0.0, roughness: 0.5, emissive: [0.0; 3], emissive_strength: 0.0 }
    }
}

/// Palette data: available entities for placement
pub struct EntityPalette {
    pub items: Vec<PaletteItem>,
    pub active_category: PaletteCategory,
    pub selected_item: Option<usize>,
    pub search_query: String,
    pub favorites: Vec<String>,
}

impl Default for EntityPalette {
    fn default() -> Self {
        let mut palette = Self {
            items: Vec::new(),
            active_category: PaletteCategory::Primitives,
            selected_item: None,
            search_query: String::new(),
            favorites: Vec::new(),
        };
        palette.load_defaults();
        palette
    }
}

impl EntityPalette {
    fn load_defaults(&mut self) {
        // Primitives
        self.add_item("cube", "Cube", PaletteCategory::Primitives, GeometryType::Cube, [1.0; 3],
            PaletteMaterial { albedo: [0.8, 0.8, 0.8], ..Default::default() }, vec!["basic", "geometry"]);
        self.add_item("sphere", "Sphere", PaletteCategory::Primitives, GeometryType::Sphere, [1.0; 3],
            PaletteMaterial { albedo: [0.7, 0.85, 0.95], ..Default::default() }, vec!["basic", "geometry"]);
        self.add_item("cylinder", "Cylinder", PaletteCategory::Primitives, GeometryType::Cylinder, [1.0, 2.0, 1.0],
            PaletteMaterial { albedo: [0.9, 0.85, 0.7], ..Default::default() }, vec!["basic", "geometry"]);
        self.add_item("capsule", "Capsule", PaletteCategory::Primitives, GeometryType::Capsule, [1.0, 2.0, 1.0],
            PaletteMaterial { albedo: [0.6, 0.8, 1.0], ..Default::default() }, vec!["basic", "character"]);

        // Environment
        self.add_item("ground", "Ground Plane", PaletteCategory::Environment, GeometryType::Cube, [40.0, 0.1, 40.0],
            PaletteMaterial { albedo: [0.3, 0.4, 0.2], roughness: 0.9, ..Default::default() }, vec!["terrain", "large"]);
        self.add_item("wall", "Wall", PaletteCategory::Environment, GeometryType::Cube, [8.0, 3.0, 0.3],
            PaletteMaterial { albedo: [0.6, 0.6, 0.6], roughness: 0.8, ..Default::default() }, vec!["architecture"]);
        self.add_item("pillar", "Pillar", PaletteCategory::Environment, GeometryType::Cylinder, [0.8, 4.0, 0.8],
            PaletteMaterial { albedo: [0.9, 0.9, 0.9], metallic: 0.8, roughness: 0.2, ..Default::default() }, vec!["architecture", "decorative"]);
        self.add_item("floor_tile", "Floor Tile", PaletteCategory::Environment, GeometryType::Cube, [2.0, 0.05, 2.0],
            PaletteMaterial { albedo: [0.5, 0.5, 0.55], roughness: 0.6, ..Default::default() }, vec!["terrain"]);

        // Characters
        self.add_item("npc_humanoid", "NPC Humanoid", PaletteCategory::Characters, GeometryType::Capsule, [1.0, 2.0, 1.0],
            PaletteMaterial { albedo: [0.8, 0.6, 0.4], ..Default::default() }, vec!["character", "npc"]);
        self.add_item("enemy_basic", "Basic Enemy", PaletteCategory::Characters, GeometryType::Cube, [1.0, 1.0, 1.0],
            PaletteMaterial { albedo: [0.9, 0.2, 0.2], ..Default::default() }, vec!["character", "enemy"]);
        self.add_item("enemy_tank", "Tank Enemy", PaletteCategory::Characters, GeometryType::Cube, [2.0, 2.0, 2.0],
            PaletteMaterial { albedo: [0.6, 0.2, 0.2], metallic: 0.5, roughness: 0.3, ..Default::default() }, vec!["character", "enemy", "heavy"]);

        // Props
        self.add_item("crate", "Wooden Crate", PaletteCategory::Props, GeometryType::Cube, [1.0, 1.0, 1.0],
            PaletteMaterial { albedo: [0.6, 0.4, 0.2], roughness: 0.9, ..Default::default() }, vec!["prop", "breakable"]);
        self.add_item("barrel", "Barrel", PaletteCategory::Props, GeometryType::Cylinder, [0.6, 1.0, 0.6],
            PaletteMaterial { albedo: [0.3, 0.3, 0.3], metallic: 0.8, roughness: 0.3, ..Default::default() }, vec!["prop", "breakable"]);
        self.add_item("crate_large", "Large Crate", PaletteCategory::Props, GeometryType::Cube, [2.0, 2.0, 2.0],
            PaletteMaterial { albedo: [0.5, 0.5, 0.5], ..Default::default() }, vec!["prop", "cover"]);
        self.add_item("bench", "Bench", PaletteCategory::Props, GeometryType::Cube, [2.0, 0.5, 0.8],
            PaletteMaterial { albedo: [0.4, 0.3, 0.2], roughness: 0.9, ..Default::default() }, vec!["prop", "furniture"]);
        self.add_item("orb_glow", "Glowing Orb", PaletteCategory::Props, GeometryType::Sphere, [0.5, 0.5, 0.5],
            PaletteMaterial { albedo: [1.0, 0.9, 0.3], emissive: [1.0, 0.8, 0.2], emissive_strength: 5.0, ..Default::default() }, vec!["prop", "pickup"]);

        // Lights
        self.add_item("light_point", "Point Light", PaletteCategory::Lights, GeometryType::Sphere, [0.2, 0.2, 0.2],
            PaletteMaterial { emissive: [1.0, 0.9, 0.7], emissive_strength: 8.0, ..Default::default() }, vec!["light", "point"]);
        self.add_item("light_spot", "Spot Light", PaletteCategory::Lights, GeometryType::Cylinder, [0.3, 0.5, 0.3],
            PaletteMaterial { emissive: [1.0, 1.0, 0.8], emissive_strength: 6.0, ..Default::default() }, vec!["light", "spot"]);
        self.add_item("light_blue", "Blue Light", PaletteCategory::Lights, GeometryType::Sphere, [0.2, 0.2, 0.2],
            PaletteMaterial { emissive: [0.2, 0.5, 1.0], emissive_strength: 8.0, ..Default::default() }, vec!["light", "decorative"]);

        // Spawn Points
        self.add_item("spawn_player", "Player Spawn", PaletteCategory::SpawnPoints, GeometryType::Capsule, [1.0, 2.0, 1.0],
            PaletteMaterial { albedo: [0.2, 0.8, 0.2], emissive: [0.0, 0.5, 0.0], emissive_strength: 2.0, ..Default::default() }, vec!["spawn", "player"]);
        self.add_item("spawn_enemy", "Enemy Spawn", PaletteCategory::SpawnPoints, GeometryType::Cube, [1.0, 1.0, 1.0],
            PaletteMaterial { albedo: [0.8, 0.2, 0.2], emissive: [0.5, 0.0, 0.0], emissive_strength: 2.0, ..Default::default() }, vec!["spawn", "enemy"]);
        self.add_item("spawn_item", "Item Spawn", PaletteCategory::SpawnPoints, GeometryType::Sphere, [0.5, 0.5, 0.5],
            PaletteMaterial { albedo: [0.9, 0.8, 0.2], emissive: [0.5, 0.4, 0.0], emissive_strength: 3.0, ..Default::default() }, vec!["spawn", "item"]);
        self.add_item("checkpoint", "Checkpoint", PaletteCategory::SpawnPoints, GeometryType::Cylinder, [1.0, 0.1, 1.0],
            PaletteMaterial { albedo: [0.2, 0.6, 1.0], emissive: [0.0, 0.3, 0.8], emissive_strength: 4.0, ..Default::default() }, vec!["spawn", "checkpoint"]);

        // Triggers
        self.add_item("trigger_zone", "Trigger Zone", PaletteCategory::Triggers, GeometryType::Cube, [4.0, 2.0, 4.0],
            PaletteMaterial { albedo: [0.8, 0.8, 0.2], emissive: [0.3, 0.3, 0.0], emissive_strength: 1.0, ..Default::default() }, vec!["trigger", "zone"]);
        self.add_item("trigger_damage", "Damage Zone", PaletteCategory::Triggers, GeometryType::Cube, [3.0, 0.5, 3.0],
            PaletteMaterial { albedo: [0.9, 0.1, 0.1], emissive: [0.5, 0.0, 0.0], emissive_strength: 2.0, ..Default::default() }, vec!["trigger", "damage"]);
        self.add_item("trigger_teleport", "Teleporter", PaletteCategory::Triggers, GeometryType::Cylinder, [1.5, 0.1, 1.5],
            PaletteMaterial { albedo: [0.6, 0.2, 0.9], emissive: [0.4, 0.1, 0.8], emissive_strength: 5.0, ..Default::default() }, vec!["trigger", "teleport"]);
    }

    fn add_item(&mut self, id: &str, name: &str, category: PaletteCategory, geometry: GeometryType,
                default_scale: [f32; 3], material: PaletteMaterial, tags: Vec<&str>) {
        self.items.push(PaletteItem {
            id: id.to_string(),
            name: name.to_string(),
            category,
            geometry,
            default_scale,
            default_material: material,
            tags: tags.into_iter().map(String::from).collect(),
            icon: String::new(),
            description: String::new(),
        });
    }

    /// Kategorideki item'ları filtrele
    pub fn items_in_category(&self, cat: &PaletteCategory) -> Vec<&PaletteItem> {
        self.items.iter().filter(|i| &i.category == cat).collect()
    }

    /// Arama sorgusuna göre filtrele
    pub fn search(&self, query: &str) -> Vec<&PaletteItem> {
        let q = query.to_lowercase();
        self.items.iter().filter(|i| {
            i.name.to_lowercase().contains(&q) ||
            i.tags.iter().any(|t| t.to_lowercase().contains(&q)) ||
            i.id.to_lowercase().contains(&q)
        }).collect()
    }

    /// Favorilere ekle/çıkar
    pub fn toggle_favorite(&mut self, id: &str) {
        if let Some(pos) = self.favorites.iter().position(|f| f == id) {
            self.favorites.remove(pos);
        } else {
            self.favorites.push(id.to_string());
        }
    }

    /// Favori item'ları al
    pub fn favorite_items(&self) -> Vec<&PaletteItem> {
        self.items.iter().filter(|i| self.favorites.contains(&i.id)).collect()
    }
}

// ═══════════════════════════════════════════════════════════ Level Layer

/// Layer sistemi — entity'leri gruplandır
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LevelLayer {
    pub id: u32,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub color: [u8; 3],
    pub entity_ids: Vec<usize>,
}

impl LevelLayer {
    pub fn new(id: u32, name: &str, color: [u8; 3]) -> Self {
        Self { id, name: name.to_string(), visible: true, locked: false, color, entity_ids: Vec::new() }
    }
}

// ═══════════════════════════════════════════════════════════ Prefab System

/// Prefab: kaydedilmiş entity şablonu
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prefab {
    pub id: String,
    pub name: String,
    pub category: String,
    pub entity_template: EntityTemplate,
    pub thumbnail_color: [f32; 3],
    pub created_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityTemplate {
    pub name: String,
    pub geometry: String, // GeometryType display name
    pub scale: [f32; 3],
    pub material: PaletteMaterial,
    pub rigid_body: bool,
    pub tags: Vec<String>,
    pub custom_properties: HashMap<String, String>,
}

pub struct PrefabLibrary {
    pub prefabs: Vec<Prefab>,
}

impl Default for PrefabLibrary {
    fn default() -> Self {
        let mut lib = Self { prefabs: Vec::new() };
        lib.load_defaults();
        lib
    }
}

impl PrefabLibrary {
    fn load_defaults(&mut self) {
        self.prefabs.push(Prefab {
            id: "prefab_cover_small".into(), name: "Small Cover".into(), category: "Combat".into(),
            entity_template: EntityTemplate {
                name: "Small Cover".into(), geometry: "Cube".into(), scale: [1.0, 1.0, 1.0],
                material: PaletteMaterial { albedo: [0.5, 0.5, 0.5], roughness: 0.8, ..Default::default() },
                rigid_body: true, tags: vec!["cover".into()], custom_properties: HashMap::new(),
            },
            thumbnail_color: [0.5, 0.5, 0.5], created_at: 0,
        });
        self.prefabs.push(Prefab {
            id: "prefab_cover_large".into(), name: "Large Cover".into(), category: "Combat".into(),
            entity_template: EntityTemplate {
                name: "Large Cover".into(), geometry: "Cube".into(), scale: [2.0, 2.0, 0.5],
                material: PaletteMaterial { albedo: [0.4, 0.4, 0.4], roughness: 0.9, ..Default::default() },
                rigid_body: true, tags: vec!["cover".into(), "wall".into()], custom_properties: HashMap::new(),
            },
            thumbnail_color: [0.4, 0.4, 0.4], created_at: 0,
        });
        self.prefabs.push(Prefab {
            id: "prefab_arena_center".into(), name: "Arena Center".into(), category: "Level".into(),
            entity_template: EntityTemplate {
                name: "Arena Center".into(), geometry: "Cylinder".into(), scale: [5.0, 0.5, 5.0],
                material: PaletteMaterial { albedo: [0.6, 0.5, 0.3], roughness: 0.7, ..Default::default() },
                rigid_body: true, tags: vec!["terrain".into(), "arena".into()], custom_properties: HashMap::new(),
            },
            thumbnail_color: [0.6, 0.5, 0.3], created_at: 0,
        });
    }

    pub fn search(&self, query: &str) -> Vec<&Prefab> {
        let q = query.to_lowercase();
        self.prefabs.iter().filter(|p| p.name.to_lowercase().contains(&q) || p.category.to_lowercase().contains(&q)).collect()
    }
}

// ═══════════════════════════════════════════════════════════ Spawn Point Manager

/// Spawn noktaları yöneticisi
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpawnPoint {
    pub id: usize,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub spawn_type: SpawnType,
    pub team: u8,
    pub priority: u32,
    pub enabled: bool,
    pub properties: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpawnType {
    PlayerStart,
    EnemySpawn,
    ItemPickup,
    Checkpoint,
    RespawnPoint,
    TriggerSpawn,
}

impl SpawnType {
    pub fn name(&self) -> &str {
        match self {
            Self::PlayerStart => "Player Start", Self::EnemySpawn => "Enemy Spawn",
            Self::ItemPickup => "Item Pickup", Self::Checkpoint => "Checkpoint",
            Self::RespawnPoint => "Respawn", Self::TriggerSpawn => "Trigger Spawn",
        }
    }
}

pub struct SpawnManager {
    pub spawns: Vec<SpawnPoint>,
    pub next_id: usize,
}

impl Default for SpawnManager {
    fn default() -> Self {
        let mut mgr = Self { spawns: Vec::new(), next_id: 1 };
        // Varsayılan player spawn
        mgr.spawns.push(SpawnPoint {
            id: mgr.next_id, position: [0.0, 1.0, 0.0], rotation: [0.0, 0.0, 0.0],
            spawn_type: SpawnType::PlayerStart, team: 0, priority: 100, enabled: true,
            properties: HashMap::new(),
        });
        mgr.next_id += 1;
        mgr
    }
}

impl SpawnManager {
    pub fn add_spawn(&mut self, pos: [f32; 3], spawn_type: SpawnType) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.spawns.push(SpawnPoint {
            id, position: pos, rotation: [0.0; 3], spawn_type, team: 0,
            priority: 50, enabled: true, properties: HashMap::new(),
        });
        id
    }

    pub fn remove_spawn(&mut self, id: usize) {
        self.spawns.retain(|s| s.id != id);
    }

    pub fn get_spawn(&self, id: usize) -> Option<&SpawnPoint> {
        self.spawns.iter().find(|s| s.id == id)
    }

    pub fn spawns_by_type(&self, spawn_type: &SpawnType) -> Vec<&SpawnPoint> {
        self.spawns.iter().filter(|s| &s.spawn_type == spawn_type && s.enabled).collect()
    }

    pub fn get_player_start(&self) -> Option<&SpawnPoint> {
        self.spawns.iter().find(|s| s.spawn_type == SpawnType::PlayerStart && s.enabled)
    }
}

// ═══════════════════════════════════════════════════════════ Undo/Redo

/// Undo/Redo sistemi
#[derive(Clone, Debug)]
pub enum EditAction {
    PlaceEntity { id: usize, data: SceneObject },
    DeleteEntity { id: usize, data: SceneObject },
    MoveEntity { id: usize, old_pos: [f32; 3], new_pos: [f32; 3] },
    RotateEntity { id: usize, old_rot: [f32; 3], new_rot: [f32; 3] },
    ScaleEntity { id: usize, old_scale: [f32; 3], new_scale: [f32; 3] },
    ChangeMaterial { id: usize, old_mat: PbrMaterial, new_mat: PbrMaterial },
    Batch(Vec<EditAction>),
}

pub struct EditHistory {
    pub undo_stack: VecDeque<EditAction>,
    pub redo_stack: VecDeque<EditAction>,
    pub max_size: usize,
}

impl Default for EditHistory {
    fn default() -> Self {
        Self { undo_stack: VecDeque::new(), redo_stack: VecDeque::new(), max_size: 256 }
    }
}

impl EditHistory {
    pub fn push(&mut self, action: EditAction) {
        self.undo_stack.push_back(action);
        self.redo_stack.clear();
        if self.undo_stack.len() > self.max_size {
            self.undo_stack.pop_front();
        }
    }

    pub fn undo(&mut self) -> Option<EditAction> {
        if let Some(action) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(action.clone());
            Some(action)
        } else { None }
    }

    pub fn redo(&mut self) -> Option<EditAction> {
        if let Some(action) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(action.clone());
            Some(action)
        } else { None }
    }

    pub fn can_undo(&self) -> bool { !self.undo_stack.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo_stack.is_empty() }

    pub fn undo_count(&self) -> usize { self.undo_stack.len() }
    pub fn redo_count(&self) -> usize { self.redo_stack.len() }

    pub fn clear(&mut self) { self.undo_stack.clear(); self.redo_stack.clear(); }
}

// ═══════════════════════════════════════════════════════════ Alignment Tools

/// Hizalama araçları
pub struct AlignmentTools;

impl AlignmentTools {
    /// Seçili nesneleri sola hizala
    pub fn align_left(scene: &mut Scene, ids: &[usize]) {
        if ids.is_empty() { return; }
        let min_x = ids.iter().filter_map(|id| scene.get_object(*id))
            .map(|o| o.transform.position.x).fold(f32::INFINITY, f32::min);
        for id in ids {
            if let Some(obj) = scene.get_object_mut(*id) { obj.transform.position.x = min_x; }
        }
    }

    /// Seçili nesneleri sağa hizala
    pub fn align_right(scene: &mut Scene, ids: &[usize]) {
        if ids.is_empty() { return; }
        let max_x = ids.iter().filter_map(|id| scene.get_object(*id))
            .map(|o| o.transform.position.x).fold(f32::NEG_INFINITY, f32::max);
        for id in ids {
            if let Some(obj) = scene.get_object_mut(*id) { obj.transform.position.x = max_x; }
        }
    }

    /// Seçili nesneleri ortala (yatay)
    pub fn align_center_x(scene: &mut Scene, ids: &[usize]) {
        if ids.is_empty() { return; }
        let sum: f32 = ids.iter().filter_map(|id| scene.get_object(*id))
            .map(|o| o.transform.position.x).sum();
        let center = sum / ids.len() as f32;
        for id in ids {
            if let Some(obj) = scene.get_object_mut(*id) { obj.transform.position.x = center; }
        }
    }

    /// Seçili nesneleri dikey ortala
    pub fn align_center_y(scene: &mut Scene, ids: &[usize]) {
        if ids.is_empty() { return; }
        let sum: f32 = ids.iter().filter_map(|id| scene.get_object(*id))
            .map(|o| o.transform.position.y).sum();
        let center = sum / ids.len() as f32;
        for id in ids {
            if let Some(obj) = scene.get_object_mut(*id) { obj.transform.position.y = center; }
        }
    }

    /// Eşit aralıkla dağıt (yatay)
    pub fn distribute_x(scene: &mut Scene, ids: &[usize]) {
        if ids.len() < 3 { return; }
        let mut positions: Vec<(usize, f32)> = ids.iter().filter_map(|id| {
            scene.get_object(*id).map(|o| (*id, o.transform.position.x))
        }).collect();
        positions.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let min_x = positions.first().unwrap().1;
        let max_x = positions.last().unwrap().1;
        let spacing = (max_x - min_x) / (positions.len() - 1) as f32;
        for (i, (id, _)) in positions.iter().enumerate() {
            if let Some(obj) = scene.get_object_mut(*id) {
                obj.transform.position.x = min_x + i as f32 * spacing;
            }
        }
    }

    /// Eşit aralıkla dağıt (dikey)
    pub fn distribute_y(scene: &mut Scene, ids: &[usize]) {
        if ids.len() < 3 { return; }
        let mut positions: Vec<(usize, f32)> = ids.iter().filter_map(|id| {
            scene.get_object(*id).map(|o| (*id, o.transform.position.y))
        }).collect();
        positions.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let min_y = positions.first().unwrap().1;
        let max_y = positions.last().unwrap().1;
        let spacing = (max_y - min_y) / (positions.len() - 1) as f32;
        for (i, (id, _)) in positions.iter().enumerate() {
            if let Some(obj) = scene.get_object_mut(*id) {
                obj.transform.position.y = min_y + i as f32 * spacing;
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════ Level Save/Export

/// Level metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LevelMetadata {
    pub name: String,
    pub author: String,
    pub description: String,
    pub version: u32,
    pub created_at: u64,
    pub modified_at: u64,
    pub engine_version: String,
    pub tags: Vec<String>,
    pub thumbnail_data: Option<String>,
}

impl Default for LevelMetadata {
    fn default() -> Self {
        Self {
            name: "Untitled Level".into(), author: "Unknown".into(), description: String::new(),
            version: 1, created_at: 0, modified_at: 0, engine_version: "Elysium 0.2.0".into(),
            tags: Vec::new(), thumbnail_data: None,
        }
    }
}

/// Save data format
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LevelSaveData {
    pub metadata: LevelMetadata,
    pub format_version: u32,
    pub camera: CameraSnapshot,
    pub objects: Vec<EntitySaveData>,
    pub lights: Vec<LightSaveData>,
    pub spawn_points: Vec<SpawnPoint>,
    pub layers: Vec<LevelLayer>,
    pub grid_settings: GridSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraSnapshot {
    pub target: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntitySaveData {
    pub id: usize,
    pub name: String,
    pub geometry: String,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub material: MaterialSaveData,
    pub visible: bool,
    pub team: String,
    pub health: f32,
    pub is_light: bool,
    pub layer_id: u32,
    pub tags: Vec<String>,
    pub custom_properties: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterialSaveData {
    pub albedo: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    pub ao: f32,
    pub emissive: [f32; 3],
    pub emissive_strength: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LightSaveData {
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub light_type: String,
    pub range: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GridSettings {
    pub enabled: bool,
    pub size: f32,
    pub divisions: u32,
    pub snap_to_grid: bool,
    pub snap_size: f32,
}

impl Default for GridSettings {
    fn default() -> Self {
        Self { enabled: true, size: 20.0, divisions: 20, snap_to_grid: true, snap_size: 0.5 }
    }
}

/// Export format
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExportFormat {
    /// Tam sahne (tüm verilerle)
    FullJson,
    /// Compact binary
    FullBinary,
    /// Sadece entity listesi (hafif format)
    EntityListJson,
    /// Unity-compatible JSON
    UnityJson,
    /// Godot-compatible TSCN
    GodotTscn,
}

impl ExportFormat {
    pub fn extension(&self) -> &str {
        match self {
            Self::FullJson => "level.json",
            Self::FullBinary => "level.bin",
            Self::EntityListJson => "entities.json",
            Self::UnityJson => "level_unity.json",
            Self::GodotTscn => "level.tscn",
        }
    }
}

// ═══════════════════════════════════════════════════════════ Play Mode

/// Play mode state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayMode {
    /// Düzenleme modu
    Editing,
    /// Oyun modu (runtime test)
    Playing,
    /// Duraklatılmış
    Paused,
}

impl PlayMode {
    pub fn name(&self) -> &str {
        match self { Self::Editing => "Edit Mode", Self::Playing => "Play Mode", Self::Paused => "Paused" }
    }
    pub fn icon(&self) -> &str {
        match self { Self::Editing => "✏️", Self::Playing => "▶", Self::Paused => "⏸" }
    }
}

/// Play mode state data (editöre geri dönerken restore edilir)
pub struct PlayModeSnapshot {
    pub saved_camera: Camera,
    pub saved_selected: Option<usize>,
    pub saved_playing: bool,
    pub runtime_entities: Vec<usize>,
}

// ═══════════════════════════════════════════════════════════ Search/Filter

/// Entity arama ve filtreleme
pub struct EditorSearch {
    pub query: String,
    pub filter_by: SearchFilter,
    pub results: Vec<usize>,
    pub result_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SearchFilter {
    All,
    ByName,
    ByType,
    ByTag,
    ByTeam,
    ByMaterial,
}

impl Default for EditorSearch {
    fn default() -> Self {
        Self { query: String::new(), filter_by: SearchFilter::All, results: Vec::new(), result_index: 0 }
    }
}

impl EditorSearch {
    pub fn search(&mut self, scene: &Scene, query: &str) {
        self.query = query.to_string();
        let q = query.to_lowercase();
        self.results.clear();

        for obj in &scene.objects {
            let matches = match self.filter_by {
                SearchFilter::All => obj.name.to_lowercase().contains(&q)
                    || format!("{:?}", obj.geometry).to_lowercase().contains(&q)
                    || obj.tags.iter().any(|t| t.to_lowercase().contains(&q)),
                SearchFilter::ByName => obj.name.to_lowercase().contains(&q),
                SearchFilter::ByType => format!("{:?}", obj.geometry).to_lowercase().contains(&q),
                SearchFilter::ByTag => obj.tags.iter().any(|t| t.to_lowercase().contains(&q)),
                SearchFilter::ByTeam => format!("{:?}", obj.team).to_lowercase().contains(&q),
                SearchFilter::ByMaterial => false, // Material araması için farklı yaklaşım
            };
            if matches { self.results.push(obj.id); }
        }
        self.result_index = 0;
    }

    pub fn next_result(&mut self) -> Option<usize> {
        if self.results.is_empty() { return None; }
        let id = self.results[self.result_index];
        self.result_index = (self.result_index + 1) % self.results.len();
        Some(id)
    }

    pub fn prev_result(&mut self) -> Option<usize> {
        if self.results.is_empty() { return None; }
        if self.result_index == 0 { self.result_index = self.results.len() - 1; }
        else { self.result_index -= 1; }
        Some(self.results[self.result_index])
    }

    pub fn result_count(&self) -> usize { self.results.len() }
}

// ═══════════════════════════════════════════════════════════ Level Editor Main

/// Ana level editör yapısı
pub struct LevelEditor {
    // Durum
    pub tool: ToolMode,
    pub play_mode: PlayMode,
    pub play_snapshot: Option<PlayModeSnapshot>,

    // Seçim
    pub selected_entities: Vec<usize>,
    pub primary_selection: Option<usize>,
    pub hover_entity: Option<usize>,

    // Palette
    pub palette: EntityPalette,
    pub drag_item: Option<String>,
    pub place_preview: Option<[f32; 3]>,

    // Layers
    pub layers: Vec<LevelLayer>,
    pub active_layer: u32,
    pub next_layer_id: u32,

    // Prefabs
    pub prefab_library: PrefabLibrary,

    // Spawn points
    pub spawn_manager: SpawnManager,

    // Undo/Redo
    pub history: EditHistory,

    // Grid
    pub grid: GridSettings,

    // Search
    pub search: EditorSearch,

    // Multi-select box
    pub box_select: Option<BoxSelect>,

    // Alignment
    pub align_mode: Option<AlignMode>,

    // Level metadata
    pub metadata: LevelMetadata,

    // Statistics
    pub stats: EditorStats,

    // Clipboard (copy/paste)
    pub clipboard: Vec<EntitySaveData>,

    // File path
    pub current_file: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BoxSelect {
    pub start: (f64, f64),
    pub current: (f64, f64),
    pub additive: bool, // shift held
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AlignMode {
    Left, Right, CenterX, CenterY, DistributeX, DistributeY,
}

#[derive(Clone, Debug, Default)]
pub struct EditorStats {
    pub entities_placed: u32,
    pub entities_deleted: u32,
    pub undo_operations: u32,
    pub total_edits: u32,
    pub session_duration_secs: f64,
}

impl Default for LevelEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl LevelEditor {
    pub fn new() -> Self {
        let mut layers = Vec::new();
        layers.push(LevelLayer::new(0, "Default", [200, 200, 200]));
        layers.push(LevelLayer::new(1, "Environment", [100, 200, 100]));
        layers.push(LevelLayer::new(2, "Props", [200, 150, 100]));
        layers.push(LevelLayer::new(3, "Entities", [100, 150, 250]));
        layers.push(LevelLayer::new(4, "Triggers", [250, 200, 50]));
        layers.push(LevelLayer::new(5, "Lights", [250, 250, 100]));
        layers.push(LevelLayer::new(6, "Spawn Points", [100, 250, 150]));

        Self {
            tool: ToolMode::Select,
            play_mode: PlayMode::Editing,
            play_snapshot: None,
            selected_entities: Vec::new(),
            primary_selection: None,
            hover_entity: None,
            palette: EntityPalette::default(),
            drag_item: None,
            place_preview: None,
            layers,
            active_layer: 0,
            next_layer_id: 7,
            prefab_library: PrefabLibrary::default(),
            spawn_manager: SpawnManager::default(),
            history: EditHistory::default(),
            grid: GridSettings::default(),
            search: EditorSearch::default(),
            box_select: None,
            align_mode: None,
            metadata: LevelMetadata::default(),
            stats: EditorStats::default(),
            clipboard: Vec::new(),
            current_file: None,
        }
    }

    // ── Entity Management ──

    /// Palette'ten sahneye entity yerleştir
    pub fn place_entity(&mut self, scene: &mut Scene, item_id: &str, position: [f32; 3]) -> Option<usize> {
        let item = self.palette.items.iter().find(|i| i.id == item_id)?;
        let team = match item.category {
            PaletteCategory::Characters => EntityTeam::Enemy,
            PaletteCategory::SpawnPoints if item.tags.contains(&"player".into()) => EntityTeam::Player,
            _ => EntityTeam::Neutral,
        };

        let id = scene.add_object(item.name.clone(), item.geometry, team);
        if let Some(obj) = scene.get_object_mut(id) {
            obj.transform.position = Vec3::new(position[0], position[1], position[2]);
            obj.transform.scale = Vec3::new(item.default_scale[0], item.default_scale[1], item.default_scale[2]);
            obj.material.albedo = Vec3::new(item.default_material.albedo[0], item.default_material.albedo[1], item.default_material.albedo[2]);
            obj.material.metallic = item.default_material.metallic;
            obj.material.roughness = item.default_material.roughness;
            obj.material.emissive = Vec3::new(item.default_material.emissive[0], item.default_material.emissive[1], item.default_material.emissive[2]);
            obj.material.emissive_strength = item.default_material.emissive_strength;
            obj.tags = item.tags.clone();
        }

        // Layer'a ekle
        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == self.active_layer) {
            layer.entity_ids.push(id);
        }

        self.history.push(EditAction::PlaceEntity {
            id,
            data: scene.get_object(id).cloned().unwrap(),
        });
        self.stats.entities_placed += 1;
        self.stats.total_edits += 1;

        Some(id)
    }

    /// Entity sil
    pub fn delete_entity(&mut self, scene: &mut Scene, id: usize) -> bool {
        if let Some(obj) = scene.get_object(id) {
            let data = obj.clone();
            scene.objects.retain(|o| o.id != id);

            // Layer'dan çıkar
            for layer in &mut self.layers {
                layer.entity_ids.retain(|&eid| eid != id);
            }

            self.history.push(EditAction::DeleteEntity { id, data });
            self.selected_entities.retain(|&eid| eid != id);
            if self.primary_selection == Some(id) {
                self.primary_selection = self.selected_entities.first().copied();
            }
            self.stats.entities_deleted += 1;
            self.stats.total_edits += 1;
            return true;
        }
        false
    }

    /// Seçili entity'leri sil
    pub fn delete_selected(&mut self, scene: &mut Scene) -> u32 {
        let ids: Vec<usize> = self.selected_entities.clone();
        let count = ids.len() as u32;
        for id in ids {
            self.delete_entity(scene, id);
        }
        count
    }

    /// Entity'yi taşı (snap-to-grid desteği ile)
    pub fn move_entity(&mut self, scene: &mut Scene, id: usize, delta: [f32; 3]) -> bool {
        let old_pos;
        if let Some(obj) = scene.get_object(id) {
            old_pos = [obj.transform.position.x, obj.transform.position.y, obj.transform.position.z];
        } else {
            return false;
        }

        let mut new_pos = [
            old_pos[0] + delta[0],
            old_pos[1] + delta[1],
            old_pos[2] + delta[2],
        ];

        // Snap to grid
        if self.grid.snap_to_grid {
            let s = self.grid.snap_size;
            new_pos[0] = (new_pos[0] / s).round() * s;
            new_pos[1] = (new_pos[1] / s).round() * s;
            new_pos[2] = (new_pos[2] / s).round() * s;
        }

        if let Some(obj) = scene.get_object_mut(id) {
            obj.transform.position = Vec3::new(new_pos[0], new_pos[1], new_pos[2]);
        }

        self.history.push(EditAction::MoveEntity { id, old_pos, new_pos });
        self.stats.total_edits += 1;
        true
    }

    /// Entity'yi döndür
    pub fn rotate_entity(&mut self, scene: &mut Scene, id: usize, delta: [f32; 3]) -> bool {
        let old_rot;
        if let Some(obj) = scene.get_object(id) {
            old_rot = [obj.transform.rotation.x, obj.transform.rotation.y, obj.transform.rotation.z];
        } else {
            return false;
        }
        let new_rot = [old_rot[0] + delta[0], old_rot[1] + delta[1], old_rot[2] + delta[2]];
        if let Some(obj) = scene.get_object_mut(id) {
            obj.transform.rotation = Vec3::new(new_rot[0], new_rot[1], new_rot[2]);
        }
        self.history.push(EditAction::RotateEntity { id, old_rot, new_rot });
        self.stats.total_edits += 1;
        true
    }

    /// Entity'yi ölçekle
    pub fn scale_entity(&mut self, scene: &mut Scene, id: usize, delta: [f32; 3]) -> bool {
        let old_scale;
        if let Some(obj) = scene.get_object(id) {
            old_scale = [obj.transform.scale.x, obj.transform.scale.y, obj.transform.scale.z];
        } else {
            return false;
        }
        let new_scale = [
            (old_scale[0] + delta[0]).max(0.01),
            (old_scale[1] + delta[1]).max(0.01),
            (old_scale[2] + delta[2]).max(0.01),
        ];
        if let Some(obj) = scene.get_object_mut(id) {
            obj.transform.scale = Vec3::new(new_scale[0], new_scale[1], new_scale[2]);
        }
        self.history.push(EditAction::ScaleEntity { id, old_scale, new_scale });
        self.stats.total_edits += 1;
        true
    }

    /// Entity'yi duplicate et
    pub fn duplicate_entity(&mut self, scene: &mut Scene, id: usize) -> Option<usize> {
        let data = scene.get_object(id)?.clone();
        let new_id = scene.next_id;
        scene.next_id += 1;
        let mut new_obj = data.clone();
        new_obj.id = new_id;
        new_obj.name = format!("{} (Copy)", data.name);
        new_obj.transform.position.y += 0.5; // Yukarı taşı
        scene.objects.push(new_obj);
        self.selected_entities.push(new_id);
        self.primary_selection = Some(new_id);
        self.stats.entities_placed += 1;
        Some(new_id)
    }

    // ── Undo/Redo ──

    pub fn undo(&mut self, scene: &mut Scene) -> bool {
        if let Some(action) = self.history.undo() {
            self.apply_action_reverse(scene, action);
            self.stats.undo_operations += 1;
            true
        } else { false }
    }

    pub fn redo(&mut self, scene: &mut Scene) -> bool {
        if let Some(action) = self.history.redo() {
            self.apply_action_forward(scene, action);
            true
        } else { false }
    }

    fn apply_action_reverse(&mut self, scene: &mut Scene, action: EditAction) {
        match action {
            EditAction::PlaceEntity { id, data: _ } => {
                scene.objects.retain(|o| o.id != id);
            }
            EditAction::DeleteEntity { id: _, data } => {
                scene.objects.push(data);
            }
            EditAction::MoveEntity { id, old_pos, .. } => {
                if let Some(obj) = scene.get_object_mut(id) {
                    obj.transform.position = Vec3::new(old_pos[0], old_pos[1], old_pos[2]);
                }
            }
            EditAction::RotateEntity { id, old_rot, .. } => {
                if let Some(obj) = scene.get_object_mut(id) {
                    obj.transform.rotation = Vec3::new(old_rot[0], old_rot[1], old_rot[2]);
                }
            }
            EditAction::ScaleEntity { id, old_scale, .. } => {
                if let Some(obj) = scene.get_object_mut(id) {
                    obj.transform.scale = Vec3::new(old_scale[0], old_scale[1], old_scale[2]);
                }
            }
            EditAction::ChangeMaterial { id, old_mat, .. } => {
                if let Some(obj) = scene.get_object_mut(id) {
                    obj.material = old_mat;
                }
            }
            EditAction::Batch(actions) => {
                for action in actions.into_iter().rev() {
                    self.apply_action_reverse(scene, action);
                }
            }
        }
    }

    fn apply_action_forward(&mut self, scene: &mut Scene, action: EditAction) {
        match action {
            EditAction::PlaceEntity { id: _, data } => {
                scene.objects.push(data);
            }
            EditAction::DeleteEntity { id, .. } => {
                scene.objects.retain(|o| o.id != id);
            }
            EditAction::MoveEntity { id, new_pos, .. } => {
                if let Some(obj) = scene.get_object_mut(id) {
                    obj.transform.position = Vec3::new(new_pos[0], new_pos[1], new_pos[2]);
                }
            }
            EditAction::RotateEntity { id, new_rot, .. } => {
                if let Some(obj) = scene.get_object_mut(id) {
                    obj.transform.rotation = Vec3::new(new_rot[0], new_rot[1], new_rot[2]);
                }
            }
            EditAction::ScaleEntity { id, new_scale, .. } => {
                if let Some(obj) = scene.get_object_mut(id) {
                    obj.transform.scale = Vec3::new(new_scale[0], new_scale[1], new_scale[2]);
                }
            }
            EditAction::ChangeMaterial { id, new_mat, .. } => {
                if let Some(obj) = scene.get_object_mut(id) {
                    obj.material = new_mat;
                }
            }
            EditAction::Batch(actions) => {
                for action in actions {
                    self.apply_action_forward(scene, action);
                }
            }
        }
    }

    // ── Play Mode ──

    /// Play mode'a geç
    pub fn enter_play_mode(&mut self, scene: &Scene, camera: Camera, selected: Option<usize>) {
        self.play_snapshot = Some(PlayModeSnapshot {
            saved_camera: camera,
            saved_selected: selected,
            saved_playing: false,
            runtime_entities: scene.objects.iter().map(|o| o.id).collect(),
        });
        self.play_mode = PlayMode::Playing;
        self.selected_entities.clear();
        self.primary_selection = None;
    }

    /// Edit mode'a geri dön
    pub fn exit_play_mode(&mut self) -> Option<(Camera, Option<usize>)> {
        if let Some(snapshot) = self.play_snapshot.take() {
            self.play_mode = PlayMode::Editing;
            Some((snapshot.saved_camera, snapshot.saved_selected))
        } else {
            None
        }
    }

    // ── Save/Export ──

    /// Level'ı JSON olarak kaydet
    pub fn save_level(&self, scene: &Scene, camera: &Camera) -> Result<String, serde_json::Error> {
        let data = self.create_save_data(scene, camera);
        serde_json::to_string_pretty(&data)
    }

    /// Level'ı compact JSON olarak kaydet
    pub fn save_level_compact(&self, scene: &Scene, camera: &Camera) -> Result<String, serde_json::Error> {
        let data = self.create_save_data(scene, camera);
        serde_json::to_string(&data)
    }

    /// Export (farklı formatlarda)
    pub fn export_level(&self, scene: &Scene, camera: &Camera, format: ExportFormat) -> Result<String, String> {
        match format {
            ExportFormat::FullJson | ExportFormat::FullBinary => {
                self.save_level(scene, camera).map_err(|e| e.to_string())
            }
            ExportFormat::EntityListJson => {
                let entities: Vec<serde_json::Value> = scene.objects.iter().map(|o| {
                    serde_json::json!({
                        "name": o.name,
                        "type": o.geometry.to_string(),
                        "position": [o.transform.position.x, o.transform.position.y, o.transform.position.z],
                        "rotation": [o.transform.rotation.x, o.transform.rotation.y, o.transform.rotation.z],
                        "scale": [o.transform.scale.x, o.transform.scale.y, o.transform.scale.z],
                    })
                }).collect();
                serde_json::to_string_pretty(&entities).map_err(|e| e.to_string())
            }
            ExportFormat::UnityJson => {
                let entities: Vec<serde_json::Value> = scene.objects.iter().map(|o| {
                    serde_json::json!({
                        "m_Name": o.name,
                        "m_IsActive": o.visible,
                        "Transform": {
                            "m_LocalPosition": [o.transform.position.x, o.transform.position.y, o.transform.position.z],
                            "m_LocalRotation": [o.transform.rotation.x, o.transform.rotation.y, o.transform.rotation.z, 0.0],
                            "m_LocalScale": [o.transform.scale.x, o.transform.scale.y, o.transform.scale.z],
                        },
                        "MeshFilter": { "m_Mesh": o.geometry.to_string() },
                    })
                }).collect();
                let result = serde_json::json!({
                    "gameObjects": entities,
                    "version": "1.0",
                    "engine": "Elysium",
                });
                serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
            }
            ExportFormat::GodotTscn => {
                let mut tscn = String::from("[gd_scene load_steps=2 format=3]\n\n");
                for (_i, obj) in scene.objects.iter().enumerate() {
                    let node_type = match obj.geometry {
                        GeometryType::Cube | GeometryType::Sphere | GeometryType::Cylinder
                        | GeometryType::Capsule | GeometryType::Plane => "MeshInstance3D",
                        GeometryType::Custom(_) => "MeshInstance3D",
                    };
                    tscn += &format!(
                        "[node name=\"{}\" type=\"{}\" parent=\".\"]\n\
                         transform = Transform3D(1, 0, 0, 0, 1, 0, 0, 0, 1, {:.3}, {:.3}, {:.3})\n\n",
                        obj.name, node_type,
                        obj.transform.position.x, obj.transform.position.y, obj.transform.position.z,
                    );
                }
                Ok(tscn)
            }
        }
    }

    fn create_save_data(&self, scene: &Scene, camera: &Camera) -> LevelSaveData {
        let objects: Vec<EntitySaveData> = scene.objects.iter().map(|o| {
            let layer_id = self.layers.iter()
                .find(|l| l.entity_ids.contains(&o.id))
                .map(|l| l.id).unwrap_or(0);

            EntitySaveData {
                id: o.id, name: o.name.clone(), geometry: o.geometry.to_string(),
                position: o.transform.position.to_array(),
                rotation: [o.transform.rotation.x, o.transform.rotation.y, o.transform.rotation.z],
                scale: o.transform.scale.to_array(),
                material: MaterialSaveData {
                    albedo: o.material.albedo.to_array(),
                    metallic: o.material.metallic, roughness: o.material.roughness,
                    ao: o.material.ao,
                    emissive: o.material.emissive.to_array(),
                    emissive_strength: o.material.emissive_strength,
                },
                visible: o.visible, team: format!("{:?}", o.team),
                health: o.health, is_light: o.is_light, layer_id,
                tags: o.tags.clone(), custom_properties: HashMap::new(),
            }
        }).collect();

        let lights: Vec<LightSaveData> = scene.lights.iter().map(|l| {
            LightSaveData {
                position: l.position.to_array(),
                direction: l.direction.to_array(),
                color: l.color.to_array(),
                intensity: l.intensity,
                light_type: match l.light_type {
                    LightType::Directional => "directional".into(),
                    LightType::Point { .. } => "point".into(),
                    LightType::Spot { .. } => "spot".into(),
                },
                range: match l.light_type { LightType::Point { radius } => radius, _ => 1000.0 },
            }
        }).collect();

        LevelSaveData {
            metadata: self.metadata.clone(),
            format_version: 2,
            camera: CameraSnapshot {
                target: camera.target.to_array(),
                yaw: camera.yaw, pitch: camera.pitch, distance: camera.distance,
            },
            objects, lights,
            spawn_points: self.spawn_manager.spawns.clone(),
            layers: self.layers.clone(),
            grid_settings: self.grid.clone(),
        }
    }

    /// Level yükle
    pub fn load_level(&mut self, scene: &mut Scene, data: &LevelSaveData, camera: &mut Camera) {
        scene.objects.clear();
        scene.lights.clear();
        scene.next_id = 0;

        // Camera restore
        camera.target = Vec3::new(data.camera.target[0], data.camera.target[1], data.camera.target[2]);
        camera.yaw = data.camera.yaw;
        camera.pitch = data.camera.pitch;
        camera.distance = data.camera.distance;

        // Spawn points
        self.spawn_manager.spawns = data.spawn_points.clone();
        self.spawn_manager.next_id = data.spawn_points.iter().map(|s| s.id).max().unwrap_or(0) + 1;

        // Layers
        self.layers = data.layers.clone();

        // Grid
        self.grid = data.grid_settings.clone();

        // Metadata
        self.metadata = data.metadata.clone();

        // Entities
        for ed in &data.objects {
            let geom = match ed.geometry.as_str() {
                "Cube" => GeometryType::Cube, "Sphere" => GeometryType::Sphere,
                "Cylinder" => GeometryType::Cylinder, "Capsule" => GeometryType::Capsule,
                _ => GeometryType::Cube,
            };
            let team = match ed.team.as_str() {
                "Player" => EntityTeam::Player, "Enemy" => EntityTeam::Enemy,
                _ => EntityTeam::Neutral,
            };
            let id = scene.add_object(ed.name.clone(), geom, team);
            if let Some(obj) = scene.get_object_mut(id) {
                obj.id = ed.id;
                obj.transform.position = Vec3::new(ed.position[0], ed.position[1], ed.position[2]);
                obj.transform.rotation = Vec3::new(ed.rotation[0], ed.rotation[1], ed.rotation[2]);
                obj.transform.scale = Vec3::new(ed.scale[0], ed.scale[1], ed.scale[2]);
                obj.visible = ed.visible;
                obj.health = ed.health;
                obj.tags = ed.tags.clone();
                obj.material.albedo = Vec3::new(ed.material.albedo[0], ed.material.albedo[1], ed.material.albedo[2]);
                obj.material.metallic = ed.material.metallic;
                obj.material.roughness = ed.material.roughness;
                obj.material.ao = ed.material.ao;
                obj.material.emissive = Vec3::new(ed.material.emissive[0], ed.material.emissive[1], ed.material.albedo[2]);
                obj.material.emissive_strength = ed.material.emissive_strength;
            }
            if ed.id >= scene.next_id { scene.next_id = ed.id + 1; }
        }

        // Lights
        for ld in &data.lights {
            let pos = Vec3::new(ld.position[0], ld.position[1], ld.position[2]);
            let dir = Vec3::new(ld.direction[0], ld.direction[1], ld.direction[2]);
            let col = Vec3::new(ld.color[0], ld.color[1], ld.color[2]);
            scene.lights.push(match ld.light_type.as_str() {
                "directional" => SceneLight::directional(dir, col, ld.intensity),
                "point" => SceneLight::point(pos, col, ld.intensity, ld.range),
                _ => SceneLight::directional(dir, col, ld.intensity),
            });
        }
    }

    // ── Layer Management ──

    pub fn add_layer(&mut self, name: &str) -> u32 {
        let id = self.next_layer_id;
        self.next_layer_id += 1;
        let color = [(id * 50 % 255) as u8, (id * 80 % 255) as u8, (id * 130 % 255) as u8];
        self.layers.push(LevelLayer::new(id, name, color));
        id
    }

    pub fn toggle_layer_visibility(&mut self, layer_id: u32) {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == layer_id) {
            layer.visible = !layer.visible;
        }
    }

    pub fn toggle_layer_lock(&mut self, layer_id: u32) {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == layer_id) {
            layer.locked = !layer.locked;
        }
    }

    pub fn is_entity_locked(&self, entity_id: usize) -> bool {
        self.layers.iter().any(|l| l.locked && l.entity_ids.contains(&entity_id))
    }

    // ── Selection ──

    pub fn select(&mut self, id: usize, additive: bool) {
        if additive {
            if self.selected_entities.contains(&id) {
                self.selected_entities.retain(|&eid| eid != id);
                if self.primary_selection == Some(id) {
                    self.primary_selection = self.selected_entities.first().copied();
                }
            } else {
                self.selected_entities.push(id);
                self.primary_selection = Some(id);
            }
        } else {
            self.selected_entities.clear();
            self.selected_entities.push(id);
            self.primary_selection = Some(id);
        }
    }

    pub fn select_all(&mut self, scene: &Scene) {
        self.selected_entities = scene.objects.iter().map(|o| o.id).collect();
        self.primary_selection = self.selected_entities.first().copied();
    }

    pub fn deselect_all(&mut self) {
        self.selected_entities.clear();
        self.primary_selection = None;
    }

    pub fn selection_count(&self) -> usize { self.selected_entities.len() }

    // ── Clipboard ──

    pub fn copy_selected(&mut self, scene: &Scene) {
        self.clipboard.clear();
        for id in &self.selected_entities {
            if let Some(obj) = scene.get_object(*id) {
                self.clipboard.push(EntitySaveData {
                    id: obj.id, name: obj.name.clone(), geometry: obj.geometry.to_string(),
                    position: obj.transform.position.to_array(),
                    rotation: [obj.transform.rotation.x, obj.transform.rotation.y, obj.transform.rotation.z],
                    scale: obj.transform.scale.to_array(),
                    material: MaterialSaveData {
                        albedo: obj.material.albedo.to_array(),
                        metallic: obj.material.metallic, roughness: obj.material.roughness,
                        ao: obj.material.ao,
                        emissive: obj.material.emissive.to_array(),
                        emissive_strength: obj.material.emissive_strength,
                    },
                    visible: obj.visible, team: format!("{:?}", obj.team),
                    health: obj.health, is_light: obj.is_light, layer_id: self.active_layer,
                    tags: obj.tags.clone(), custom_properties: HashMap::new(),
                });
            }
        }
    }

    pub fn paste(&mut self, scene: &mut Scene, offset: [f32; 3]) -> Vec<usize> {
        let mut new_ids = Vec::new();
        for data in &self.clipboard {
            let geom = match data.geometry.as_str() {
                "Cube" => GeometryType::Cube, "Sphere" => GeometryType::Sphere,
                "Cylinder" => GeometryType::Cylinder, "Capsule" => GeometryType::Capsule,
                _ => GeometryType::Cube,
            };
            let team = match data.team.as_str() {
                "Player" => EntityTeam::Player, "Enemy" => EntityTeam::Enemy,
                _ => EntityTeam::Neutral,
            };
            let id = scene.add_object(data.name.clone(), geom, team);
            if let Some(obj) = scene.get_object_mut(id) {
                obj.transform.position = Vec3::new(
                    data.position[0] + offset[0],
                    data.position[1] + offset[1],
                    data.position[2] + offset[2],
                );
                obj.transform.rotation = Vec3::new(data.rotation[0], data.rotation[1], data.rotation[2]);
                obj.transform.scale = Vec3::new(data.scale[0], data.scale[1], data.scale[2]);
                obj.visible = data.visible;
                obj.health = data.health;
                obj.tags = data.tags.clone();
                obj.material.albedo = Vec3::new(data.material.albedo[0], data.material.albedo[1], data.material.albedo[2]);
                obj.material.metallic = data.material.metallic;
                obj.material.roughness = data.material.roughness;
                obj.material.emissive = Vec3::new(data.material.emissive[0], data.material.emissive[1], data.material.emissive[2]);
                obj.material.emissive_strength = data.material.emissive_strength;
            }
            new_ids.push(id);
        }
        self.selected_entities = new_ids.clone();
        self.primary_selection = new_ids.first().copied();
        new_ids
    }

    // ── Tool switching ──

    pub fn set_tool(&mut self, tool: ToolMode) {
        self.tool = tool;
    }

    pub fn tool_from_shortcut(key: char) -> Option<ToolMode> {
        match key {
            'Q' => Some(ToolMode::Select), 'P' => Some(ToolMode::Place),
            'G' => Some(ToolMode::Move), 'R' => Some(ToolMode::Rotate),
            'S' => Some(ToolMode::Scale), 'B' => Some(ToolMode::Paint),
            'X' => Some(ToolMode::Erase), 'D' => Some(ToolMode::Duplicate),
            'M' => Some(ToolMode::Measure),
            _ => None,
        }
    }

    // ── Statistics ──

    pub fn level_stats(&self, scene: &Scene) -> LevelStats {
        let mut stats = LevelStats::default();
        stats.total_entities = scene.objects.len();
        stats.total_lights = scene.lights.len();
        stats.total_triangles = scene.objects.iter().map(|o| match o.geometry {
            GeometryType::Cube => 12, GeometryType::Sphere => 16*24*2,
            GeometryType::Cylinder => 20*4, GeometryType::Capsule => 20*3,
            GeometryType::Plane => 2, GeometryType::Custom(_) => 0,
        }).sum();
        for obj in &scene.objects {
            match obj.team {
                EntityTeam::Player => stats.player_count += 1,
                EntityTeam::Enemy => stats.enemy_count += 1,
                EntityTeam::Neutral => stats.prop_count += 1,
            }
        }
        stats.layer_count = self.layers.len();
        stats.spawn_count = self.spawn_manager.spawns.len();
        stats.prefab_count = self.prefab_library.prefabs.len();
        stats
    }
}

#[derive(Clone, Debug, Default)]
pub struct LevelStats {
    pub total_entities: usize,
    pub total_lights: usize,
    pub total_triangles: usize,
    pub player_count: usize,
    pub enemy_count: usize,
    pub prop_count: usize,
    pub layer_count: usize,
    pub spawn_count: usize,
    pub prefab_count: usize,
}

// ═══════════════════════════════════════════════════════════ Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn test_scene() -> Scene {
        let mut scene = Scene::new();
        scene.add_object("Cube1".into(), GeometryType::Cube, EntityTeam::Neutral);
        scene.add_object("Sphere1".into(), GeometryType::Sphere, EntityTeam::Player);
        scene.add_object("Cylinder1".into(), GeometryType::Cylinder, EntityTeam::Enemy);
        scene
    }

    #[test]
    fn test_editor_creation() {
        let editor = LevelEditor::new();
        assert_eq!(editor.tool, ToolMode::Select);
        assert_eq!(editor.play_mode, PlayMode::Editing);
        assert_eq!(editor.layers.len(), 7);
        assert_eq!(editor.palette.items.len() > 0, true);
    }

    #[test]
    fn test_place_entity() {
        let mut editor = LevelEditor::new();
        let mut scene = test_scene();
        let id = editor.place_entity(&mut scene, "cube", [1.0, 2.0, 3.0]);
        assert!(id.is_some());
        let id = id.unwrap();
        let obj = scene.get_object(id).unwrap();
        assert_eq!(obj.transform.position.x, 1.0);
        assert_eq!(obj.transform.position.y, 2.0);
        assert_eq!(obj.transform.position.z, 3.0);
        assert_eq!(editor.stats.entities_placed, 1);
    }

    #[test]
    fn test_delete_entity() {
        let mut editor = LevelEditor::new();
        let mut scene = test_scene();
        let id = scene.add_object("ToDel".into(), GeometryType::Cube, EntityTeam::Neutral);
        let count_before = scene.objects.len();
        assert!(editor.delete_entity(&mut scene, id));
        assert_eq!(scene.objects.len(), count_before - 1);
        assert_eq!(editor.stats.entities_deleted, 1);
    }

    #[test]
    fn test_undo_redo() {
        let mut editor = LevelEditor::new();
        let mut scene = test_scene();
        let id = editor.place_entity(&mut scene, "cube", [0.0, 0.0, 0.0]).unwrap();
        assert_eq!(scene.objects.len(), 4); // 3 from test + 1 placed
        assert!(editor.undo(&mut scene));
        assert_eq!(scene.objects.len(), 3);
        assert!(editor.redo(&mut scene));
        assert_eq!(scene.objects.len(), 4);
    }

    #[test]
    fn test_move_entity() {
        let mut editor = LevelEditor::new();
        let mut scene = test_scene();
        let id = 1;
        assert!(editor.move_entity(&mut scene, id, [1.0, 0.0, 0.0]));
        let obj = scene.get_object(id).unwrap();
        assert!((obj.transform.position.x - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_rotate_entity() {
        let mut editor = LevelEditor::new();
        let mut scene = test_scene();
        assert!(editor.rotate_entity(&mut scene, 1, [0.0, 45.0, 0.0]));
        let obj = scene.get_object(1).unwrap();
        assert!((obj.transform.rotation.y - 45.0).abs() < 0.01);
    }

    #[test]
    fn test_scale_entity() {
        let mut editor = LevelEditor::new();
        let mut scene = test_scene();
        assert!(editor.scale_entity(&mut scene, 1, [1.0, 1.0, 1.0]));
        let obj = scene.get_object(1).unwrap();
        assert!((obj.transform.scale.x - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_duplicate_entity() {
        let mut editor = LevelEditor::new();
        let mut scene = test_scene();
        let new_id = editor.duplicate_entity(&mut scene, 1);
        assert!(new_id.is_some());
        assert_eq!(scene.objects.len(), 4);
    }

    #[test]
    fn test_selection() {
        let mut editor = LevelEditor::new();
        editor.select(1, false);
        assert_eq!(editor.selection_count(), 1);
        assert_eq!(editor.primary_selection, Some(1));
        editor.select(2, true);
        assert_eq!(editor.selection_count(), 2);
        editor.select(1, true); // toggle off
        assert_eq!(editor.selection_count(), 1);
        editor.deselect_all();
        assert_eq!(editor.selection_count(), 0);
    }

    #[test]
    fn test_layer_management() {
        let mut editor = LevelEditor::new();
        let id = editor.add_layer("Custom Layer");
        assert!(editor.layers.iter().any(|l| l.id == id));
        editor.toggle_layer_visibility(id);
        assert!(!editor.layers.iter().find(|l| l.id == id).unwrap().visible);
        editor.toggle_layer_lock(id);
        assert!(editor.layers.iter().find(|l| l.id == id).unwrap().locked);
    }

    #[test]
    fn test_palette_search() {
        let palette = EntityPalette::default();
        let results = palette.search("light");
        assert!(results.len() >= 3); // point light, spot light, blue light
    }

    #[test]
    fn test_save_load_level() {
        let mut editor = LevelEditor::new();
        let mut scene = test_scene();
        editor.place_entity(&mut scene, "cube", [5.0, 0.0, 5.0]);
        let camera = Camera::new();
        let json = editor.save_level(&scene, &camera).unwrap();
        assert!(!json.is_empty());

        let data: LevelSaveData = serde_json::from_str(&json).unwrap();
        assert_eq!(data.objects.len(), 4);
        assert_eq!(data.format_version, 2);
    }

    #[test]
    fn test_export_unity() {
        let editor = LevelEditor::new();
        let scene = test_scene();
        let camera = Camera::new();
        let result = editor.export_level(&scene, &camera, ExportFormat::UnityJson);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("gameObjects"));
    }

    #[test]
    fn test_export_godot() {
        let editor = LevelEditor::new();
        let scene = test_scene();
        let camera = Camera::new();
        let result = editor.export_level(&scene, &camera, ExportFormat::GodotTscn);
        assert!(result.is_ok());
        let tscn = result.unwrap();
        assert!(tscn.contains("[gd_scene"));
        assert!(tscn.contains("Cube1"));
    }

    #[test]
    fn test_play_mode() {
        let mut editor = LevelEditor::new();
        let scene = test_scene();
        let camera = Camera::new();
        editor.enter_play_mode(&scene, camera, Some(1));
        assert_eq!(editor.play_mode, PlayMode::Playing);
        let result = editor.exit_play_mode();
        assert!(result.is_some());
        assert_eq!(editor.play_mode, PlayMode::Editing);
    }

    #[test]
    fn test_alignment_tools() {
        let mut scene = Scene::new();
        let a = scene.add_object("A".into(), GeometryType::Cube, EntityTeam::Neutral);
        scene.get_object_mut(a).unwrap().transform.position = Vec3::new(0.0, 0.0, 0.0);
        let b = scene.add_object("B".into(), GeometryType::Cube, EntityTeam::Neutral);
        scene.get_object_mut(b).unwrap().transform.position = Vec3::new(10.0, 0.0, 0.0);
        let c = scene.add_object("C".into(), GeometryType::Cube, EntityTeam::Neutral);
        scene.get_object_mut(c).unwrap().transform.position = Vec3::new(20.0, 0.0, 0.0);
        let ids = vec![a, b, c];
        // Test align_left: all should move to min x = 0.0
        AlignmentTools::align_left(&mut scene, &ids);
        let pos_b = scene.get_object(b).unwrap().transform.position.x;
        assert!((pos_b - 0.0).abs() < 0.01, "align_left failed: {}", pos_b);
        // Reset positions for distribute test
        scene.get_object_mut(a).unwrap().transform.position = Vec3::new(0.0, 0.0, 0.0);
        scene.get_object_mut(b).unwrap().transform.position = Vec3::new(10.0, 0.0, 0.0);
        scene.get_object_mut(c).unwrap().transform.position = Vec3::new(20.0, 0.0, 0.0);
        // Test distribute_x: should be 0.0, 10.0, 20.0
        AlignmentTools::distribute_x(&mut scene, &ids);
        let pos_b2 = scene.get_object(b).unwrap().transform.position.x;
        assert!((pos_b2 - 10.0).abs() < 0.01, "distribute_x failed: {}", pos_b2);
    }

    #[test]
    fn test_search_filter() {
        let mut search = EditorSearch::default();
        let scene = test_scene();
        search.search(&scene, "Cube");
        assert_eq!(search.result_count(), 1);
        search.search(&scene, "");
        assert_eq!(search.result_count(), 3); // all entities
    }

    #[test]
    fn test_spawn_manager() {
        let mut mgr = SpawnManager::default();
        assert_eq!(mgr.spawns.len(), 1); // default player start
        let id = mgr.add_spawn([10.0, 0.0, 10.0], SpawnType::Checkpoint);
        assert!(mgr.get_spawn(id).is_some());
        mgr.remove_spawn(id);
        assert!(mgr.get_spawn(id).is_none());
    }

    #[test]
    fn test_copy_paste() {
        let mut editor = LevelEditor::new();
        let mut scene = test_scene();
        editor.select(1, false);
        editor.copy_selected(&scene);
        assert_eq!(editor.clipboard.len(), 1);
        let new_ids = editor.paste(&mut scene, [1.0, 0.0, 0.0]);
        assert_eq!(new_ids.len(), 1);
        assert_eq!(scene.objects.len(), 4);
        let obj = scene.get_object(new_ids[0]).unwrap();
        assert!((obj.transform.position.x - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_tool_shortcuts() {
        assert_eq!(LevelEditor::tool_from_shortcut('Q'), Some(ToolMode::Select));
        assert_eq!(LevelEditor::tool_from_shortcut('G'), Some(ToolMode::Move));
        assert_eq!(LevelEditor::tool_from_shortcut('R'), Some(ToolMode::Rotate));
        assert_eq!(LevelEditor::tool_from_shortcut('S'), Some(ToolMode::Scale));
        assert_eq!(LevelEditor::tool_from_shortcut('Z'), None);
    }

    #[test]
    fn test_level_stats() {
        let editor = LevelEditor::new();
        let scene = test_scene();
        let stats = editor.level_stats(&scene);
        assert_eq!(stats.total_entities, 3);
        assert_eq!(stats.player_count, 1);
        assert_eq!(stats.enemy_count, 1);
        assert_eq!(stats.prop_count, 1);
    }

    #[test]
    fn test_prefab_search() {
        let lib = PrefabLibrary::default();
        let results = lib.search("cover");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_history_size_limit() {
        let mut history = EditHistory::default();
        history.max_size = 5;
        for i in 0..10 {
            history.push(EditAction::MoveEntity {
                id: 0, old_pos: [i as f32, 0.0, 0.0], new_pos: [i as f32 + 1.0, 0.0, 0.0],
            });
        }
        assert!(history.undo_stack.len() <= 5);
    }

    #[test]
    fn test_grid_snap() {
        let mut editor = LevelEditor::new();
        editor.grid.snap_to_grid = true;
        editor.grid.snap_size = 0.5;
        let mut scene = test_scene();
        editor.move_entity(&mut scene, 1, [0.3, 0.0, 0.0]);
        let obj = scene.get_object(1).unwrap();
        assert!((obj.transform.position.x % 0.5).abs() < 0.01);
    }
}
