//! # Elysium Editor (Mimari §7)
//!
//! Sahne düzenleyici: hierarchy, inspector, viewport, gizmos, undo/redo.

pub mod gizmo;
pub mod tools;
pub mod undo;

pub use gizmo::*;
pub use tools::*;
pub use undo::*;

/// Editör araç modu
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ToolMode {
    Select,
    Translate,
    Rotate,
    Scale,
    Create,
    Delete,
}

/// Gizmo modu
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GizmoMode {
    Translate,
    Rotate,
    Scale,
}

/// Editör durumu
pub struct EditorState {
    pub selected_entities: Vec<u32>,
    pub tool_mode: ToolMode,
    pub gizmo_mode: GizmoMode,
    pub show_grid: bool,
    pub show_gizmos: bool,
    pub show_bounding_boxes: bool,
    pub snap_to_grid: bool,
    pub snap_distance: f32,
    pub grid_spacing: f32,
    pub camera_speed: f32,
    pub gizmo_size: f32,
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            selected_entities: Vec::new(),
            tool_mode: ToolMode::Select,
            gizmo_mode: GizmoMode::Translate,
            show_grid: true,
            show_gizmos: true,
            show_bounding_boxes: false,
            snap_to_grid: false,
            snap_distance: 0.25,
            grid_spacing: 1.0,
            camera_speed: 5.0,
            gizmo_size: 1.0,
        }
    }

    /// Seçili varlık sayısını döndür
    pub fn selection_count(&self) -> usize {
        self.selected_entities.len()
    }

    /// Tek varlık seç
    pub fn select(&mut self, entity: u32) {
        self.selected_entities.clear();
        self.selected_entities.push(entity);
    }

    /// Varlık ekle seçime
    pub fn add_to_selection(&mut self, entity: u32) {
        if !self.selected_entities.contains(&entity) {
            self.selected_entities.push(entity);
        }
    }

    /// Seçimi temizle
    pub fn clear_selection(&mut self) {
        self.selected_entities.clear();
    }

    /// Seçili ilk varlığı al
    pub fn primary_selection(&self) -> Option<u32> {
        self.selected_entities.first().copied()
    }
}

/// EditorApp — ana editör uygulaması
pub struct EditorApp {
    pub editor_state: EditorState,
}

impl EditorApp {
    pub fn new() -> Self {
        Self {
            editor_state: EditorState::new(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        // Editör güncellemeleri burada yapılır
        let _ = dt;
    }
}

impl Default for EditorApp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_state() {
        let mut state = EditorState::new();
        assert!(state.selected_entities.is_empty());
        assert_eq!(state.tool_mode, ToolMode::Select);

        state.select(42);
        assert_eq!(state.selected_entities.len(), 1);
        assert_eq!(state.primary_selection(), Some(42));

        state.add_to_selection(100);
        assert_eq!(state.selection_count(), 2);

        state.clear_selection();
        assert!(state.selected_entities.is_empty());
    }

    #[test]
    fn test_editor_app() {
        let mut app = EditorApp::new();
        app.update(0.016);
        assert_eq!(app.editor_state.tool_mode, ToolMode::Select);
    }
}
