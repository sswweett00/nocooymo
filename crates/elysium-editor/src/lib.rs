//! Elysium Editor (Mimari §7).

pub mod gizmo;
pub mod tools;
pub mod undo;

pub use gizmo::*;
pub use tools::*;
pub use undo::*;

use elysium_core::{World, System};
use gpui::*;

pub struct EditorState {
    pub selected_entity: Option<elysium_core::Entity>,
    pub tool_mode: ToolMode,
    pub gizmo_mode: GizmoMode,
    pub show_grid: bool,
    pub show_gizmos: bool,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            selected_entity: None,
            tool_mode: ToolMode::Select,
            gizmo_mode: GizmoMode::Translate,
            show_grid: true,
            show_gizmos: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ToolMode {
    Select,
    Translate,
    Rotate,
    Scale,
    Create,
    Delete,
}

#[derive(Clone, Copy, PartialEq)]
pub enum GizmoMode {
    Translate,
    Rotate,
    Scale,
}

pub struct EditorApp {
    pub editor_state: EditorState,
    pub ui_context: Option<WindowContext>,
}

impl EditorApp {
    pub fn new() -> Self {
        Self {
            editor_state: EditorState::new(),
            ui_context: None,
        }
    }

    pub fn register_systems(schedule: &mut elysium_core::scheduler::Schedule) {
        schedule.add_system(editor_update_system);
    }
}

fn editor_update_system(world: &mut World) {
    // This system would handle editor-specific updates
    // For now, it's a placeholder
}

pub fn init_editor_app() -> App {
    App::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_state() {
        let state = EditorState::new();
        assert!(state.selected_entity.is_none());
        assert_eq!(state.tool_mode, ToolMode::Select);
    }
}
