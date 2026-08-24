//! Dock Layout System for Elysium Editor
//! Flexible docking system for panels and windows

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockArea {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub struct DockNode {
    pub id: String,
    pub node_type: DockNodeType,
    pub size: (f32, f32),
    pub position: (f32, f32),
    pub children: Vec<String>,
    pub split_direction: Option<DockDirection>,
    pub split_ratio: f32,
}

#[derive(Debug, Clone)]
pub enum DockNodeType {
    Leaf {
        panel_id: String,
        title: String,
        is_visible: bool,
        is_floating: bool,
    },
    Container {
        direction: DockDirection,
    },
}

impl DockNode {
    pub fn new_leaf(id: String, panel_id: String, title: String) -> Self {
        Self {
            id,
            node_type: DockNodeType::Leaf {
                panel_id,
                title,
                is_visible: true,
                is_floating: false,
            },
            size: (400.0, 300.0),
            position: (0.0, 0.0),
            children: Vec::new(),
            split_direction: None,
            split_ratio: 0.5,
        }
    }

    pub fn new_container(id: String, direction: DockDirection) -> Self {
        Self {
            id,
            node_type: DockNodeType::Container { direction },
            size: (800.0, 600.0),
            position: (0.0, 0.0),
            children: Vec::new(),
            split_direction: Some(direction),
            split_ratio: 0.5,
        }
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self.node_type, DockNodeType::Leaf { .. })
    }

    pub fn is_container(&self) -> bool {
        matches!(self.node_type, DockNodeType::Container { .. })
    }
}

pub struct DockLayout {
    pub nodes: HashMap<String, DockNode>,
    pub root_node: Option<String>,
    pub floating_panels: Vec<String>,
    pub active_panel: Option<String>,
}

impl DockLayout {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root_node: None,
            floating_panels: Vec::new(),
            active_panel: None,
        }
    }

    pub fn create_root(&mut self, direction: DockDirection) -> String {
        let id = "root".to_string();
        let node = DockNode::new_container(id.clone(), direction);
        self.nodes.insert(id.clone(), node);
        self.root_node = Some(id.clone());
        id
    }

    pub fn add_panel(&mut self, panel_id: String, title: String, parent_id: Option<String>) -> String {
        let id = format!("panel_{}", panel_id);
        let node = DockNode::new_leaf(id.clone(), panel_id.clone(), title);
        
        if let Some(parent) = parent_id {
            if let Some(parent_node) = self.nodes.get_mut(&parent) {
                parent_node.children.push(id.clone());
            }
        } else if let Some(root) = &self.root_node {
            if let Some(root_node) = self.nodes.get_mut(root) {
                root_node.children.push(id.clone());
            }
        }
        
        self.nodes.insert(id.clone(), node);
        id
    }

    pub fn split_panel(&mut self, panel_id: &str, direction: DockDirection, ratio: f32) -> Result<String, String> {
        if let Some(panel_node) = self.nodes.get(panel_id) {
            if !panel_node.is_leaf() {
                return Err("Cannot split a container node".to_string());
            }

            let parent_id = self.find_parent(panel_id);
            
            // Create new container
            let container_id = format!("container_{}", panel_id);
            let mut container = DockNode::new_container(container_id.clone(), direction);
            container.split_ratio = ratio;
            container.children = vec![panel_id.to_string()];
            
            // Create new leaf panel
            let new_panel_id = format!("{}_split", panel_id);
            let new_panel = DockNode::new_leaf(
                new_panel_id.clone(),
                format!("{}_panel", panel_id),
                "New Panel".to_string(),
            );
            container.children.push(new_panel_id.clone());
            
            self.nodes.insert(new_panel_id.clone(), new_panel);
            self.nodes.insert(container_id.clone(), container);
            
            // Update parent
            if let Some(parent) = parent_id {
                if let Some(parent_node) = self.nodes.get_mut(&parent) {
                    if let Some(idx) = parent_node.children.iter().position(|id| id == panel_id) {
                        parent_node.children[idx] = container_id.clone();
                    }
                }
            } else if let Some(root) = &self.root_node {
                if *root == panel_id {
                    self.root_node = Some(container_id.clone());
                }
            }
            
            Ok(container_id)
        } else {
            Err("Panel not found".to_string())
        }
    }

    pub fn dock_panel(&mut self, panel_id: &str, target_id: &str, area: DockArea) -> Result<(), String> {
        match area {
            DockArea::Center => {
                // Replace target with panel
                if let Some(target_node) = self.nodes.get(target_id) {
                    if target_node.is_leaf() {
                        let parent_id = self.find_parent(target_id);
                        if let Some(parent) = parent_id {
                            if let Some(parent_node) = self.nodes.get_mut(&parent) {
                                if let Some(idx) = parent_node.children.iter().position(|id| id == target_id) {
                                    parent_node.children[idx] = panel_id.to_string();
                                }
                            }
                        }
                    }
                }
            }
            DockArea::Left | DockArea::Right | DockArea::Top | DockArea::Bottom => {
                // Split target and dock panel
                let direction = match area {
                    DockArea::Left | DockArea::Right => DockDirection::Horizontal,
                    DockArea::Top | DockArea::Bottom => DockDirection::Vertical,
                    _ => unreachable!(),
                };
                
                let ratio = match area {
                    DockArea::Left | DockArea::Top => 0.3,
                    DockArea::Right | DockArea::Bottom => 0.7,
                    _ => 0.5,
                };
                
                self.split_panel(target_id, direction, ratio)?;
            }
        }
        
        Ok(())
    }

    pub fn undock_panel(&mut self, panel_id: &str) -> Result<(), String> {
        if let Some(panel_node) = self.nodes.get_mut(panel_id) {
            if let DockNodeType::Leaf { is_floating, .. } = &mut panel_node.node_type {
                *is_floating = true;
                self.floating_panels.push(panel_id.to_string());
            }
        }
        Ok(())
    }

    pub fn close_panel(&mut self, panel_id: &str) -> Result<(), String> {
        if let Some(panel_node) = self.nodes.remove(panel_id) {
            if panel_node.is_leaf() {
                let parent_id = self.find_parent(panel_id);
                if let Some(parent) = parent_id {
                    if let Some(parent_node) = self.nodes.get_mut(&parent) {
                        parent_node.children.retain(|id| id != panel_id);
                        
                        // If parent has only one child, collapse it
                        if parent_node.children.len() == 1 {
                            let only_child = parent_node.children[0].clone();
                            if let Some(grandparent) = self.find_parent(&parent) {
                                if let Some(grandparent_node) = self.nodes.get_mut(&grandparent) {
                                    if let Some(idx) = grandparent_node.children.iter().position(|id| id == &parent) {
                                        grandparent_node.children[idx] = only_child;
                                    }
                                }
                            } else if let Some(root) = &self.root_node {
                                if *root == parent {
                                    self.root_node = Some(only_child);
                                }
                            }
                            self.nodes.remove(&parent);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn set_active_panel(&mut self, panel_id: &str) {
        self.active_panel = Some(panel_id.to_string());
    }

    pub fn get_panel(&self, panel_id: &str) -> Option<&DockNode> {
        self.nodes.get(panel_id)
    }

    pub fn get_active_panel(&self) -> Option<&DockNode> {
        self.active_panel.as_ref().and_then(|id| self.nodes.get(id))
    }

    fn find_parent(&self, node_id: &str) -> Option<String> {
        for (id, node) in &self.nodes {
            if node.children.contains(&node_id.to_string()) {
                return Some(id.clone());
            }
        }
        None
    }

    pub fn calculate_layout(&mut self, width: f32, height: f32) {
        if let Some(root_id) = &self.root_node {
            self.calculate_node_layout(root_id, (0.0, 0.0), (width, height));
        }
    }

    fn calculate_node_layout(&mut self, node_id: &str, position: (f32, f32), size: (f32, f32)) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.position = position;
            node.size = size;
            
            if let DockNodeType::Container { direction } = node.node_type {
                let child_count = node.children.len();
                if child_count > 0 {
                    let ratio = 1.0 / child_count as f32;
                    
                    for (i, child_id) in node.children.iter().enumerate() {
                        let child_size = match direction {
                            DockDirection::Horizontal => (size.0 * ratio, size.1),
                            DockDirection::Vertical => (size.0, size.1 * ratio),
                        };
                        
                        let child_position = match direction {
                            DockDirection::Horizontal => (position.0 + size.0 * ratio * i as f32, position.1),
                            DockDirection::Vertical => (position.0, position.1 + size.1 * ratio * i as f32),
                        };
                        
                        self.calculate_node_layout(child_id, child_position, child_size);
                    }
                }
            }
        }
    }
}

impl Default for DockLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dock_layout_creation() {
        let layout = DockLayout::new();
        assert!(layout.root_node.is_none());
    }

    #[test]
    fn test_create_root() {
        let mut layout = DockLayout::new();
        let root_id = layout.create_root(DockDirection::Horizontal);
        assert_eq!(layout.root_node, Some(root_id));
    }

    #[test]
    fn test_add_panel() {
        let mut layout = DockLayout::new();
        layout.create_root(DockDirection::Horizontal);
        let panel_id = layout.add_panel("test".to_string(), "Test Panel".to_string(), None);
        assert!(layout.nodes.contains_key(&panel_id));
    }
}
