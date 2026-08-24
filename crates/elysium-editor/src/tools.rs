use elysium_core::{Entity, Transform, World};
use glam::Vec3;

pub struct SelectionTool {
    pub selected_entities: Vec<Entity>,
    pub selection_mode: SelectionMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionMode {
    Single,
    Multiple,
    Rectangle,
    Lasso,
}

impl SelectionTool {
    pub fn new() -> Self {
        Self {
            selected_entities: Vec::new(),
            selection_mode: SelectionMode::Single,
        }
    }

    pub fn select_entity(&mut self, entity: Entity) {
        match self.selection_mode {
            SelectionMode::Single => {
                self.selected_entities.clear();
                self.selected_entities.push(entity);
            }
            SelectionMode::Multiple => {
                if !self.selected_entities.contains(&entity) {
                    self.selected_entities.push(entity);
                }
            }
            _ => {
                // Other selection modes would be implemented differently
                self.selected_entities.clear();
                self.selected_entities.push(entity);
            }
        }
    }

    pub fn deselect_entity(&mut self, entity: Entity) {
        if let Some(pos) = self.selected_entities.iter().position(|&e| e == entity) {
            self.selected_entities.remove(pos);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_entities.clear();
    }

    pub fn get_selected_entities(&self) -> &[Entity] {
        &self.selected_entities
    }
}

pub struct TransformTool {
    pub translation_speed: f32,
    pub rotation_speed: f32,
    pub scale_speed: f32,
    pub snap_enabled: bool,
    pub snap_distance: f32,
    pub snap_angle: f32,
}

impl TransformTool {
    pub fn new() -> Self {
        Self {
            translation_speed: 1.0,
            rotation_speed: 1.0,
            scale_speed: 1.0,
            snap_enabled: false,
            snap_distance: 0.5,
            snap_angle: 15.0, // degrees
        }
    }

    pub fn translate_entity(&mut self, transform: &mut Transform, delta: Vec3) {
        let mut translation = delta * self.translation_speed;
        
        if self.snap_enabled {
            translation.x = (translation.x / self.snap_distance).round() * self.snap_distance;
            translation.y = (translation.y / self.snap_distance).round() * self.snap_distance;
            translation.z = (translation.z / self.snap_distance).round() * self.snap_distance;
        }
        
        transform.position.x += translation.x;
        transform.position.y += translation.y;
        transform.position.z += translation.z;
    }

    pub fn rotate_entity(&mut self, transform: &mut Transform, delta: Vec3) {
        let mut rotation = delta * self.rotation_speed;
        
        if self.snap_enabled {
            rotation.x = (rotation.x.to_degrees() / self.snap_angle).round() * self.snap_angle;
            rotation.y = (rotation.y.to_degrees() / self.snap_angle).round() * self.snap_angle;
            rotation.z = (rotation.z.to_degrees() / self.snap_angle).round() * self.snap_angle;
        }
        
        transform.rotation.x += rotation.x.to_degrees();
        transform.rotation.y += rotation.y.to_degrees();
        transform.rotation.z += rotation.z.to_degrees();
    }

    pub fn scale_entity(&mut self, transform: &mut Transform, delta: Vec3) {
        let scale_delta = delta * self.scale_speed;
        
        transform.scale.x += scale_delta.x;
        transform.scale.y += scale_delta.y;
        transform.scale.z += scale_delta.z;
        
        // Prevent negative scaling
        transform.scale.x = transform.scale.x.max(0.01);
        transform.scale.y = transform.scale.y.max(0.01);
        transform.scale.z = transform.scale.z.max(0.01);
    }
}

pub struct CreationTool {
    pub prefab_mode: PrefabMode,
    pub current_prefab: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrefabMode {
    EmptyObject,
    Cube,
    Sphere,
    Plane,
    Custom(String),
}

impl CreationTool {
    pub fn new() -> Self {
        Self {
            prefab_mode: PrefabMode::EmptyObject,
            current_prefab: None,
        }
    }

    pub fn create_entity(&self, world: &mut World) -> Entity {
        let entity = world.spawn();
        
        // Add default transform component
        world.insert_one(entity, Transform::default());
        
        // Add other components based on prefab type
        match &self.prefab_mode {
            PrefabMode::EmptyObject => {
                // Just the transform
            }
            PrefabMode::Cube => {
                // Add cube mesh component
                // Add material component
            }
            PrefabMode::Sphere => {
                // Add sphere mesh component
                // Add material component
            }
            PrefabMode::Plane => {
                // Add plane mesh component
                // Add material component
            }
            PrefabMode::Custom(prefab_name) => {
                // Load custom prefab
                // Add components based on prefab definition
            }
        }
        
        entity
    }

    pub fn set_prefab(&mut self, prefab: PrefabMode) {
        self.prefab_mode = prefab;
        if let PrefabMode::Custom(name) = &prefab {
            self.current_prefab = Some(name.clone());
        } else {
            self.current_prefab = None;
        }
    }
}

pub struct DeletionTool {
    pub confirm_required: bool,
    pub last_deleted: Vec<Entity>,
}

impl DeletionTool {
    pub fn new() -> Self {
        Self {
            confirm_required: true,
            last_deleted: Vec::new(),
        }
    }

    pub fn delete_entity(&mut self, world: &mut World, entity: Entity) -> bool {
        if self.confirm_required {
            // In a real implementation, this would show a confirmation dialog
            // For now, we'll just proceed
        }
        
        // Store for potential undo
        self.last_deleted.push(entity);
        
        // Actually remove the entity
        world.despawn(entity)
    }

    pub fn bulk_delete(&mut self, world: &mut World, entities: &[Entity]) -> usize {
        let mut deleted_count = 0;
        
        for &entity in entities {
            if self.delete_entity(world, entity) {
                deleted_count += 1;
            }
        }
        
        deleted_count
    }
}

pub struct PaintingTool {
    pub brush_size: f32,
    pub brush_strength: f32,
    pub painting_property: PaintingProperty,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaintingProperty {
    Weight(f32), // For skinning weights
    Color([f32; 4]), // For vertex colors
    UVOffset([f32; 2]), // For texture coordinate adjustments
}

impl PaintingTool {
    pub fn new() -> Self {
        Self {
            brush_size: 1.0,
            brush_strength: 1.0,
            painting_property: PaintingProperty::Weight(1.0),
        }
    }

    pub fn set_property(&mut self, property: PaintingProperty) {
        self.painting_property = property;
    }
}

pub struct NavigationTool {
    pub navigation_mode: NavigationMode,
    pub movement_speed: f32,
    pub look_sensitivity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavigationMode {
    Orbit,
    Fly,
    Walk,
    Pan,
}

impl NavigationTool {
    pub fn new() -> Self {
        Self {
            navigation_mode: NavigationMode::Orbit,
            movement_speed: 1.0,
            look_sensitivity: 1.0,
        }
    }

    pub fn move_camera(&self, transform: &mut Transform, direction: Vec3) {
        let movement = direction * self.movement_speed;
        
        match self.navigation_mode {
            NavigationMode::Orbit => {
                // In orbit mode, movement is relative to look direction
                transform.position.x += movement.x;
                transform.position.y += movement.y;
                transform.position.z += movement.z;
            }
            NavigationMode::Fly => {
                // In fly mode, movement is in world space
                transform.position.x += movement.x;
                transform.position.y += movement.y;
                transform.position.z += movement.z;
            }
            NavigationMode::Walk => {
                // In walk mode, only X/Z movement
                transform.position.x += movement.x;
                transform.position.z += movement.z;
            }
            NavigationMode::Pan => {
                // In pan mode, movement is perpendicular to look direction
                transform.position.x += movement.x;
                transform.position.y += movement.y;
            }
        }
    }
}

pub struct ToolSystem {
    pub selection_tool: SelectionTool,
    pub transform_tool: TransformTool,
    pub creation_tool: CreationTool,
    pub deletion_tool: DeletionTool,
    pub painting_tool: PaintingTool,
    pub navigation_tool: NavigationTool,
    pub active_tool: ActiveTool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveTool {
    Selection,
    Translate,
    Rotate,
    Scale,
    Create,
    Delete,
    Paint,
    Navigate,
}

impl ToolSystem {
    pub fn new() -> Self {
        Self {
            selection_tool: SelectionTool::new(),
            transform_tool: TransformTool::new(),
            creation_tool: CreationTool::new(),
            deletion_tool: DeletionTool::new(),
            painting_tool: PaintingTool::new(),
            navigation_tool: NavigationTool::new(),
            active_tool: ActiveTool::Selection,
        }
    }

    pub fn set_active_tool(&mut self, tool: ActiveTool) {
        self.active_tool = tool;
    }

    pub fn get_active_tool(&self) -> ActiveTool {
        self.active_tool
    }

    pub fn handle_input(&mut self, input_event: InputEvent) -> bool {
        match self.active_tool {
            ActiveTool::Selection => {
                match input_event {
                    InputEvent::MouseClick(pos, button) => {
                        // Handle selection click
                        true
                    }
                    _ => false,
                }
            }
            ActiveTool::Translate | ActiveTool::Rotate | ActiveTool::Scale => {
                match input_event {
                    InputEvent::MouseMove(delta) => {
                        // Handle transformation
                        true
                    }
                    InputEvent::MouseDrag(start, current) => {
                        // Handle dragging transformation
                        true
                    }
                    _ => false,
                }
            }
            ActiveTool::Create => {
                match input_event {
                    InputEvent::MouseClick(pos, button) => {
                        // Create new object at position
                        true
                    }
                    _ => false,
                }
            }
            ActiveTool::Delete => {
                match input_event {
                    InputEvent::KeyPress(key) => {
                        // Handle deletion
                        true
                    }
                    _ => false,
                }
            }
            ActiveTool::Paint => {
                match input_event {
                    InputEvent::MouseDrag(start, current) => {
                        // Handle painting
                        true
                    }
                    _ => false,
                }
            }
            ActiveTool::Navigate => {
                match input_event {
                    InputEvent::MouseMove(delta) => {
                        // Handle camera navigation
                        true
                    }
                    InputEvent::KeyboardInput(keys) => {
                        // Handle keyboard navigation
                        true
                    }
                    _ => false,
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum InputEvent {
    MouseClick((f32, f32), MouseButton), // (x, y), button
    MouseMove((f32, f32)), // (delta_x, delta_y)
    MouseDrag((f32, f32), (f32, f32)), // (start_x, start_y), (current_x, current_y)
    MouseScroll(f32), // delta
    KeyPress(char), // key
    KeyboardInput(Vec<KeyCode>), // pressed keys
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy)]
pub enum KeyCode {
    W, A, S, D, // Movement
    Q, E, // Up/down
    Shift, Ctrl, Alt, // Modifiers
    Space, Enter, Backspace, Delete, // Actions
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight, // Navigation
    F1, F2, F3, F4, F5, // Functions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_tool() {
        let mut tool = SelectionTool::new();
        let entity = Entity::new(1, 0); // Assuming Entity has a new method
        
        tool.select_entity(entity);
        assert_eq!(tool.selected_entities.len(), 1);
        assert_eq!(tool.selected_entities[0], entity);
    }

    #[test]
    fn test_transform_tool() {
        let mut tool = TransformTool::new();
        let mut transform = Transform::default();
        
        let delta = Vec3::new(1.0, 0.0, 0.0);
        tool.translate_entity(&mut transform, delta);
        
        assert_eq!(transform.position.x, 1.0);
    }
}