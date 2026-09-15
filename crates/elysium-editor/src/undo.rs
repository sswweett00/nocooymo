//! Undo/Redo sistemi — delta tabanlı geçmiş yönetimi.

use std::collections::VecDeque;

const DEFAULT_HISTORY_SIZE: usize = 256;

/// Geri alınamayan eylem türü
#[derive(Debug, Clone)]
pub enum UndoAction {
    /// Transform değişikliği
    TransformChanged {
        entity_id: u32,
        old_position: [f32; 3],
        new_position: [f32; 3],
        old_rotation: [f32; 3],
        new_rotation: [f32; 3],
    },
    /// Varlık oluşturma
    EntityCreated { entity_id: u32 },
    /// Varlık silme (snapshot dahil)
    EntityDeleted { entity_id: u32, snapshot: EntitySnapshot },
    /// Komponent ekleme
    ComponentAdded { entity_id: u32, component_type: String },
    /// Komponent çıkarma
    ComponentRemoved { entity_id: u32, component_type: String, data: Vec<u8> },
    /// Çoklu eylem
    MultiAction { actions: Vec<UndoAction> },
}

/// Varlık anlık görüntüsü (undo için)
#[derive(Debug, Clone)]
pub struct EntitySnapshot {
    pub entity_id: u32,
    pub name: String,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub visible: bool,
    pub tags: Vec<String>,
}

/// Undo yığını
pub struct UndoStack {
    pub history: VecDeque<UndoAction>,
    pub redo_stack: VecDeque<UndoAction>,
    pub max_size: usize,
}

impl Default for UndoStack {
    fn default() -> Self { Self::new() }
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(DEFAULT_HISTORY_SIZE),
            redo_stack: VecDeque::new(),
            max_size: DEFAULT_HISTORY_SIZE,
        }
    }

    pub fn with_size(max_size: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(max_size),
            redo_stack: VecDeque::new(),
            max_size,
        }
    }

    pub fn push(&mut self, action: UndoAction) {
        self.history.push_back(action);
        if self.history.len() > self.max_size {
            self.history.pop_front();
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> Option<UndoAction> {
        let action = self.history.pop_back()?;
        // Ters eylemi oluştur ve redo'ya ekle
        let reverse = Self::reverse_action(&action);
        self.redo_stack.push_front(reverse);
        Some(action)
    }

    pub fn redo(&mut self) -> Option<UndoAction> {
        let action = self.redo_stack.pop_front()?;
        let reverse = Self::reverse_action(&action);
        self.history.push_back(reverse);
        Some(action)
    }

    fn reverse_action(action: &UndoAction) -> UndoAction {
        match action {
            UndoAction::TransformChanged { entity_id, old_position, new_position, old_rotation, new_rotation } => {
                UndoAction::TransformChanged {
                    entity_id: *entity_id,
                    old_position: *new_position,
                    new_position: *old_position,
                    old_rotation: *new_rotation,
                    new_rotation: *old_rotation,
                }
            }
            UndoAction::EntityCreated { entity_id } => {
                UndoAction::EntityDeleted {
                    entity_id: *entity_id,
                    snapshot: EntitySnapshot {
                        entity_id: *entity_id,
                        name: String::new(),
                        position: [0.0; 3],
                        rotation: [0.0; 3],
                        scale: [1.0; 3],
                        visible: true,
                        tags: Vec::new(),
                    },
                }
            }
            UndoAction::EntityDeleted { entity_id, snapshot: _ } => {
                UndoAction::EntityCreated { entity_id: *entity_id }
            }
            UndoAction::ComponentAdded { entity_id, component_type } => {
                UndoAction::ComponentRemoved {
                    entity_id: *entity_id,
                    component_type: component_type.clone(),
                    data: Vec::new(),
                }
            }
            UndoAction::ComponentRemoved { entity_id, component_type, data: _ } => {
                UndoAction::ComponentAdded {
                    entity_id: *entity_id,
                    component_type: component_type.clone(),
                }
            }
            UndoAction::MultiAction { actions } => {
                UndoAction::MultiAction {
                    actions: actions.iter().rev().map(|a| Self::reverse_action(a)).collect(),
                }
            }
        }
    }

    pub fn can_undo(&self) -> bool { !self.history.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo_stack.is_empty() }
    pub fn clear(&mut self) { self.history.clear(); self.redo_stack.clear(); }
    pub fn history_len(&self) -> usize { self.history.len() }
    pub fn redo_len(&self) -> usize { self.redo_stack.len() }
}

/// Undo sistemi — yüksek seviye API
pub struct UndoSystem {
    pub stack: UndoStack,
    pub recording: bool,
}

impl Default for UndoSystem {
    fn default() -> Self { Self::new() }
}

impl UndoSystem {
    pub fn new() -> Self { Self { stack: UndoStack::new(), recording: true } }

    pub fn record(&mut self, action: UndoAction) {
        if self.recording { self.stack.push(action); }
    }

    pub fn undo(&mut self) -> Option<UndoAction> { self.stack.undo() }
    pub fn redo(&mut self) -> Option<UndoAction> { self.stack.redo() }
    pub fn can_undo(&self) -> bool { self.stack.can_undo() }
    pub fn can_redo(&self) -> bool { self.stack.can_redo() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_redo() {
        let mut stack = UndoStack::new();
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());

        stack.push(UndoAction::EntityCreated { entity_id: 1 });
        assert!(stack.can_undo());
        assert!(!stack.can_redo());

        // Undo: returns original action, stores reverse in redo
        let action = stack.undo().unwrap();
        assert!(matches!(action, UndoAction::EntityCreated { entity_id: 1 }));
        assert!(!stack.can_undo());
        assert!(stack.can_redo());

        // Redo: returns reverse action, stores original back in history
        let action = stack.redo().unwrap();
        assert!(matches!(action, UndoAction::EntityDeleted { entity_id: 1, .. }));
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_transform_undo() {
        let mut stack = UndoStack::new();
        stack.push(UndoAction::TransformChanged {
            entity_id: 1,
            old_position: [0.0; 3],
            new_position: [1.0, 0.0, 0.0],
            old_rotation: [0.0; 3],
            new_rotation: [0.0; 3],
        });

        // Undo returns the original action as-is
        let action = stack.undo().unwrap();
        if let UndoAction::TransformChanged { old_position, new_position, .. } = action {
            assert_eq!(old_position, [0.0; 3]);
            assert_eq!(new_position, [1.0, 0.0, 0.0]);
        } else { panic!("Expected TransformChanged"); }
    }
}
