//! Editör araçları — seçim, dönüştürme, oluşturma, silme araçları.

/// Seçim modu
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionMode {
    Single,
    Multiple,
    Rectangle,
}

/// Seçim aracı
pub struct SelectionTool {
    pub selected: Vec<u32>,
    pub mode: SelectionMode,
}

impl Default for SelectionTool {
    fn default() -> Self { Self::new() }
}

impl SelectionTool {
    pub fn new() -> Self { Self { selected: Vec::new(), mode: SelectionMode::Single } }

    pub fn select(&mut self, entity: u32) {
        match self.mode {
            SelectionMode::Single => { self.selected.clear(); self.selected.push(entity); }
            SelectionMode::Multiple => { if !self.selected.contains(&entity) { self.selected.push(entity); } }
            _ => { self.selected.clear(); self.selected.push(entity); }
        }
    }

    pub fn deselect(&mut self, entity: u32) {
        self.selected.retain(|&e| e != entity);
    }

    pub fn clear(&mut self) { self.selected.clear(); }
    pub fn is_selected(&self, entity: u32) -> bool { self.selected.contains(&entity) }
    pub fn count(&self) -> usize { self.selected.len() }
}

/// Dönüşüm aracı
pub struct TransformTool {
    pub snap_enabled: bool,
    pub snap_distance: f32,
    pub snap_angle: f32,
    pub translation_speed: f32,
    pub rotation_speed: f32,
}

impl Default for TransformTool {
    fn default() -> Self { Self::new() }
}

impl TransformTool {
    pub fn new() -> Self {
        Self { snap_enabled: false, snap_distance: 0.25, snap_angle: 15.0, translation_speed: 1.0, rotation_speed: 1.0 }
    }

    pub fn snap_value(&self, value: f32) -> f32 {
        if self.snap_enabled { (value / self.snap_distance).round() * self.snap_distance } else { value }
    }

    pub fn snap_angle(&self, angle: f32) -> f32 {
        if self.snap_enabled { (angle / self.snap_angle).round() * self.snap_angle } else { angle }
    }
}

/// Navigasyon aracı
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavigationMode {
    Orbit,
    Fly,
    Walk,
}

pub struct NavigationTool {
    pub mode: NavigationMode,
    pub speed: f32,
    pub sensitivity: f32,
}

impl Default for NavigationTool {
    fn default() -> Self { Self::new() }
}

impl NavigationTool {
    pub fn new() -> Self { Self { mode: NavigationMode::Orbit, speed: 1.0, sensitivity: 1.0 } }
}

/// Aktif araç
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveTool {
    Selection,
    Translate,
    Rotate,
    Scale,
    Create,
    Delete,
    Navigate,
}

/// Araç sistemi
pub struct ToolSystem {
    pub selection: SelectionTool,
    pub transform: TransformTool,
    pub navigation: NavigationTool,
    pub active: ActiveTool,
}

impl Default for ToolSystem {
    fn default() -> Self { Self::new() }
}

impl ToolSystem {
    pub fn new() -> Self {
        Self {
            selection: SelectionTool::new(),
            transform: TransformTool::new(),
            navigation: NavigationTool::new(),
            active: ActiveTool::Selection,
        }
    }

    pub fn set_active(&mut self, tool: ActiveTool) { self.active = tool; }
    pub fn get_active(&self) -> ActiveTool { self.active }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_tool() {
        let mut tool = SelectionTool::new();
        tool.select(1);
        assert_eq!(tool.count(), 1);
        assert!(tool.is_selected(1));
        tool.select(2);
        assert_eq!(tool.count(), 1); // Single mode
    }

    #[test]
    fn test_transform_snap() {
        let tool = TransformTool { snap_enabled: true, snap_distance: 0.5, ..TransformTool::new() };
        assert_eq!(tool.snap_value(0.3), 0.5);
        assert_eq!(tool.snap_value(0.7), 0.5);
        assert_eq!(tool.snap_value(1.2), 1.0);
    }
}
