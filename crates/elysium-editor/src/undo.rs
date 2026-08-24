use elysium_core::{Entity, Transform, World};
use std::collections::VecDeque;
use serde::{Serialize, Deserialize};

const DEFAULT_HISTORY_SIZE: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UndoAction {
    TransformChanged {
        entity: Entity,
        old_transform: Transform,
        new_transform: Transform,
    },
    EntityCreated {
        entity: Entity,
    },
    EntityDeleted {
        entity: Entity,
        components: EntitySnapshot,
    },
    ComponentAdded {
        entity: Entity,
        component_type: String,
        component_data: Vec<u8>,
    },
    ComponentRemoved {
        entity: Entity,
        component_type: String,
        component_data: Vec<u8>,
    },
    MultiAction {
        actions: Vec<UndoAction>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub entity: Entity,
    pub transform: Option<Transform>,
    pub components: Vec<(String, Vec<u8>)>, // (component_type, serialized_data)
}

impl EntitySnapshot {
    pub fn from_entity(world: &World, entity: Entity) -> Option<Self> {
        if !world.entity_exists(entity) {
            return None;
        }

        let mut snapshot = EntitySnapshot {
            entity,
            transform: None,
            components: Vec::new(),
        };

        // Capture transform if it exists
        if let Ok(transform) = world.get_component::<Transform>(entity) {
            snapshot.transform = Some(transform.clone());
        }

        // Capture other components (in a real implementation, we'd need a way to iterate over all components)
        // For now, we'll just store what we know about

        Some(snapshot)
    }
}

pub struct UndoStack {
    pub history: VecDeque<UndoAction>,
    pub redo_stack: VecDeque<UndoAction>,
    pub max_history_size: usize,
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            history: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_history_size: DEFAULT_HISTORY_SIZE,
        }
    }

    pub fn new_with_size(max_size: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(max_size),
            redo_stack: VecDeque::new(),
            max_history_size: max_size,
        }
    }

    pub fn push_action(&mut self, action: UndoAction) {
        self.history.push_back(action);

        // Limit history size
        if self.history.len() > self.max_history_size {
            self.history.pop_front();
        }

        // Clear redo stack when new action is added
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, world: &mut World) -> Result<(), UndoError> {
        if let Some(action) = self.history.pop_back() {
            let reverse_action = self.apply_undo_action(world, &action)?;
            self.redo_stack.push_front(action);
            
            // Add the reverse action to redo stack
            self.redo_stack.push_front(reverse_action);
            
            Ok(())
        } else {
            Err(UndoError::EmptyHistory)
        }
    }

    pub fn redo(&mut self, world: &mut World) -> Result<(), UndoError> {
        if let Some(action) = self.redo_stack.pop_front() {
            let reverse_action = self.apply_redo_action(world, &action)?;
            self.history.push_back(action);
            
            // Add the reverse action to undo stack
            self.history.push_back(reverse_action);
            
            Ok(())
        } else {
            Err(UndoError::EmptyRedoStack)
        }
    }

    fn apply_undo_action(&mut self, world: &mut World, action: &UndoAction) -> Result<UndoAction, UndoError> {
        match action {
            UndoAction::TransformChanged { entity, old_transform, new_transform } => {
                // Apply the old transform to revert the change
                if world.entity_exists(*entity) {
                    world.insert_one(*entity, old_transform.clone());
                    
                    // Return the reverse action
                    Ok(UndoAction::TransformChanged {
                        entity: *entity,
                        old_transform: new_transform.clone(),
                        new_transform: old_transform.clone(),
                    })
                } else {
                    Err(UndoError::EntityNotFound(*entity))
                }
            }
            UndoAction::EntityCreated { entity } => {
                // Remove the entity to undo creation
                if world.entity_exists(*entity) {
                    world.despawn(*entity);
                    
                    // Return the reverse action
                    Ok(UndoAction::EntityDeleted {
                        entity: *entity,
                        components: EntitySnapshot::from_entity(world, *entity)
                            .unwrap_or_else(|| EntitySnapshot {
                                entity: *entity,
                                transform: None,
                                components: Vec::new(),
                            }),
                    })
                } else {
                    Err(UndoError::EntityNotFound(*entity))
                }
            }
            UndoAction::EntityDeleted { entity, components } => {
                // Recreate the entity to undo deletion
                let new_entity = world.spawn();
                
                // Restore components
                if let Some(transform) = &components.transform {
                    world.insert_one(new_entity, transform.clone());
                }
                
                // Note: In a real implementation, we'd restore all components
                // For now, we just restore the transform
                
                // Return the reverse action
                Ok(UndoAction::EntityCreated {
                    entity: new_entity,
                })
            }
            UndoAction::ComponentAdded { entity, component_type, component_data } => {
                // Remove the component to undo addition
                if world.entity_exists(*entity) {
                    // In a real implementation, we'd need a way to remove the component
                    // For now, we'll just return the reverse action
                    Ok(UndoAction::ComponentRemoved {
                        entity: *entity,
                        component_type: component_type.clone(),
                        component_data: component_data.clone(),
                    })
                } else {
                    Err(UndoError::EntityNotFound(*entity))
                }
            }
            UndoAction::ComponentRemoved { entity, component_type, component_data } => {
                // Add back the component to undo removal
                if world.entity_exists(*entity) {
                    // In a real implementation, we'd need a way to add the component back
                    // For now, we'll just return the reverse action
                    Ok(UndoAction::ComponentAdded {
                        entity: *entity,
                        component_type: component_type.clone(),
                        component_data: component_data.clone(),
                    })
                } else {
                    Err(UndoError::EntityNotFound(*entity))
                }
            }
            UndoAction::MultiAction { actions } => {
                // Apply undo to each action in reverse order
                let mut reverse_actions = Vec::new();
                
                for action in actions.iter().rev() {
                    reverse_actions.push(self.apply_undo_action(world, action)?);
                }
                
                // Return the reverse multi-action
                Ok(UndoAction::MultiAction {
                    actions: reverse_actions,
                })
            }
        }
    }

    fn apply_redo_action(&mut self, world: &mut World, action: &UndoAction) -> Result<UndoAction, UndoError> {
        // Redo is essentially the same as the original action
        // But with swapped "old" and "new" values where applicable
        match action {
            UndoAction::TransformChanged { entity, old_transform, new_transform } => {
                if world.entity_exists(*entity) {
                    world.insert_one(*entity, new_transform.clone());
                    
                    // Return the reverse action for undo
                    Ok(UndoAction::TransformChanged {
                        entity: *entity,
                        old_transform: new_transform.clone(),
                        new_transform: old_transform.clone(),
                    })
                } else {
                    Err(UndoError::EntityNotFound(*entity))
                }
            }
            UndoAction::EntityCreated { entity } => {
                if !world.entity_exists(*entity) {
                    let new_entity = world.spawn();
                    world.insert_one(new_entity, Transform::default());
                    
                    // Return the reverse action for undo
                    Ok(UndoAction::EntityDeleted {
                        entity: new_entity,
                        components: EntitySnapshot::from_entity(world, new_entity)
                            .unwrap_or_else(|| EntitySnapshot {
                                entity: new_entity,
                                transform: Some(Transform::default()),
                                components: Vec::new(),
                            }),
                    })
                } else {
                    Err(UndoError::EntityAlreadyExists(*entity))
                }
            }
            UndoAction::EntityDeleted { entity, components } => {
                if !world.entity_exists(*entity) {
                    // Recreate the entity
                    let new_entity = world.spawn();
                    
                    if let Some(transform) = &components.transform {
                        world.insert_one(new_entity, transform.clone());
                    }
                    
                    // Return the reverse action for undo
                    Ok(UndoAction::EntityCreated {
                        entity: new_entity,
                    })
                } else {
                    Err(UndoError::EntityAlreadyExists(*entity))
                }
            }
            UndoAction::ComponentAdded { entity, component_type, component_data } => {
                if world.entity_exists(*entity) {
                    // Add the component
                    // Return the reverse action for undo
                    Ok(UndoAction::ComponentRemoved {
                        entity: *entity,
                        component_type: component_type.clone(),
                        component_data: component_data.clone(),
                    })
                } else {
                    Err(UndoError::EntityNotFound(*entity))
                }
            }
            UndoAction::ComponentRemoved { entity, component_type, component_data } => {
                if world.entity_exists(*entity) {
                    // Remove the component
                    // Return the reverse action for undo
                    Ok(UndoAction::ComponentAdded {
                        entity: *entity,
                        component_type: component_type.clone(),
                        component_data: component_data.clone(),
                    })
                } else {
                    Err(UndoError::EntityNotFound(*entity))
                }
            }
            UndoAction::MultiAction { actions } => {
                // Apply redo to each action in original order
                let mut reverse_actions = Vec::new();
                
                for action in actions {
                    reverse_actions.push(self.apply_redo_action(world, action)?);
                }
                
                // Return the reverse multi-action
                Ok(UndoAction::MultiAction {
                    actions: reverse_actions,
                })
            }
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.redo_stack.clear();
    }

    pub fn history_size(&self) -> usize {
        self.history.len()
    }

    pub fn redo_size(&self) -> usize {
        self.redo_stack.len()
    }
}

#[derive(Debug)]
pub enum UndoError {
    EmptyHistory,
    EmptyRedoStack,
    EntityNotFound(Entity),
    EntityAlreadyExists(Entity),
    SerializationError(String),
}

impl std::fmt::Display for UndoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UndoError::EmptyHistory => write!(f, "Undo history is empty"),
            UndoError::EmptyRedoStack => write!(f, "Redo stack is empty"),
            UndoError::EntityNotFound(entity) => write!(f, "Entity {:?} not found", entity),
            UndoError::EntityAlreadyExists(entity) => write!(f, "Entity {:?} already exists", entity),
            UndoError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for UndoError {}

pub struct UndoSystem {
    pub stack: UndoStack,
    pub recording: bool,
}

impl UndoSystem {
    pub fn new() -> Self {
        Self {
            stack: UndoStack::new(),
            recording: true,
        }
    }

    pub fn new_with_size(max_size: usize) -> Self {
        Self {
            stack: UndoStack::new_with_size(max_size),
            recording: true,
        }
    }

    pub fn begin_record(&mut self) {
        self.recording = true;
    }

    pub fn end_record(&mut self) {
        self.recording = false;
    }

    pub fn record_action(&mut self, action: UndoAction) {
        if self.recording {
            self.stack.push_action(action);
        }
    }

    pub fn record_transform_change(&mut self, entity: Entity, old_transform: Transform, new_transform: Transform) {
        if self.recording {
            self.stack.push_action(UndoAction::TransformChanged {
                entity,
                old_transform,
                new_transform,
            });
        }
    }

    pub fn record_entity_created(&mut self, entity: Entity) {
        if self.recording {
            self.stack.push_action(UndoAction::EntityCreated { entity });
        }
    }

    pub fn record_entity_deleted(&mut self, world: &World, entity: Entity) {
        if self.recording {
            if let Some(components) = EntitySnapshot::from_entity(world, entity) {
                self.stack.push_action(UndoAction::EntityDeleted {
                    entity,
                    components,
                });
            }
        }
    }

    pub fn undo(&mut self, world: &mut World) -> Result<(), UndoError> {
        self.stack.undo(world)
    }

    pub fn redo(&mut self, world: &mut World) -> Result<(), UndoError> {
        self.stack.redo(world)
    }

    pub fn can_undo(&self) -> bool {
        self.stack.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.stack.can_redo()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elysium_core::World;

    #[test]
    fn test_undo_stack_creation() {
        let stack = UndoStack::new();
        assert_eq!(stack.history.len(), 0);
        assert_eq!(stack.redo_stack.len(), 0);
        assert_eq!(stack.max_history_size, DEFAULT_HISTORY_SIZE);
    }

    #[test]
    fn test_undo_system() {
        let mut system = UndoSystem::new();
        assert!(system.can_undo() == false);
        assert!(system.can_redo() == false);
        
        // Add a dummy action
        system.record_action(UndoAction::EntityCreated { entity: Entity::new(1, 0) });
        assert!(system.can_undo() == true);
    }
}