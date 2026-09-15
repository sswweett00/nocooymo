//! Dock Layout Sistemi — panel yerleştirme ve kenetleme.

use std::collections::HashMap;

/// Kenetleme yönü
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DockDirection {
    Horizontal,
    Vertical,
}

/// Kenetleme alanı
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DockArea {
    Left, Right, Top, Bottom, Center,
}

/// Dock düğümü
#[derive(Debug, Clone)]
pub struct DockNode {
    pub id: String,
    pub children: Vec<String>,
    pub direction: DockDirection,
    pub split_ratio: f32,
    pub panel_id: Option<String>,
    pub title: String,
    pub visible: bool,
}

impl DockNode {
    pub fn leaf(id: &str, panel_id: &str, title: &str) -> Self {
        Self { id: id.to_string(), children: Vec::new(), direction: DockDirection::Horizontal,
               split_ratio: 0.5, panel_id: Some(panel_id.to_string()), title: title.to_string(), visible: true }
    }

    pub fn container(id: &str, direction: DockDirection) -> Self {
        Self { id: id.to_string(), children: Vec::new(), direction, split_ratio: 0.5,
               panel_id: None, title: String::new(), visible: true }
    }
}

/// Dock yerleşim sistemi
pub struct DockLayout {
    pub nodes: HashMap<String, DockNode>,
    pub root: Option<String>,
    pub active_panel: Option<String>,
}

impl Default for DockLayout {
    fn default() -> Self { Self::new() }
}

impl DockLayout {
    pub fn new() -> Self {
        Self { nodes: HashMap::new(), root: None, active_panel: None }
    }

    pub fn create_root(&mut self, direction: DockDirection) -> &str {
        let node = DockNode::container("root", direction);
        self.root = Some("root".to_string());
        self.nodes.insert("root".to_string(), node);
        "root"
    }

    pub fn add_panel(&mut self, id: &str, panel_id: &str, title: &str) {
        let node = DockNode::leaf(id, panel_id, title);
        self.nodes.insert(id.to_string(), node);
        if let Some(root) = &self.root {
            if let Some(root_node) = self.nodes.get_mut(root) {
                root_node.children.push(id.to_string());
            }
        }
    }

    pub fn get_panel_rect(&self, id: &str, total_width: f32, total_height: f32) -> Option<[f32; 4]> {
        // Basit: split ratio'ya göre dikdörtgen hesapla
        self.nodes.get(id).map(|_| {
            let w = total_width * 0.25;
            let h = total_height * 0.6;
            [0.0, 0.0, w, h]
        })
    }

    pub fn set_active(&mut self, id: &str) { self.active_panel = Some(id.to_string()); }
    pub fn node_count(&self) -> usize { self.nodes.len() }
}
