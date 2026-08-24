//! Inspector Panel for Elysium Editor
//! Displays and edits entity properties and components

use elysium_core::{Entity, World};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Color([f32; 4]),
    Enum(String, Vec<String>), // (current_value, options)
}

#[derive(Debug, Clone)]
pub struct Property {
    pub name: String,
    pub value: PropertyValue,
    pub read_only: bool,
    pub category: String,
}

impl Property {
    pub fn new(name: String, value: PropertyValue) -> Self {
        Self {
            name,
            value,
            read_only: false,
            category: "Default".to_string(),
        }
    }

    pub fn with_category(mut self, category: String) -> Self {
        self.category = category;
        self
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}

#[derive(Debug, Clone)]
pub struct ComponentSection {
    pub component_type: String,
    pub properties: Vec<Property>,
    pub is_enabled: bool,
    pub is_expanded: bool,
}

impl ComponentSection {
    pub fn new(component_type: String) -> Self {
        Self {
            component_type,
            properties: Vec::new(),
            is_enabled: true,
            is_expanded: true,
        }
    }

    pub fn add_property(&mut self, property: Property) {
        self.properties.push(property);
    }

    pub fn get_property(&mut self, name: &str) -> Option<&mut Property> {
        self.properties.iter_mut().find(|p| p.name == name)
    }
}

#[derive(Debug, Clone)]
pub struct InspectorPanel {
    pub selected_entity: Option<Entity>,
    pub sections: Vec<ComponentSection>,
    pub show_components: bool,
    pub show_add_component: bool,
    pub search_filter: String,
}

impl InspectorPanel {
    pub fn new() -> Self {
        Self {
            selected_entity: None,
            sections: Vec::new(),
            show_components: true,
            show_add_component: false,
            search_filter: String::new(),
        }
    }

    pub fn select_entity(&mut self, entity: Entity, world: &World) {
        self.selected_entity = Some(entity);
        self.refresh_sections(world);
    }

    pub fn deselect_entity(&mut self) {
        self.selected_entity = None;
        self.sections.clear();
    }

    pub fn refresh_sections(&mut self, world: &World) {
        self.sections.clear();
        
        if let Some(entity) = self.selected_entity {
            // Transform component
            let transform_section = self.create_transform_section(world, entity);
            self.sections.push(transform_section);
            
            // Mesh component (if exists)
            if self.has_component(world, entity, "Mesh") {
                let mesh_section = self.create_mesh_section(world, entity);
                self.sections.push(mesh_section);
            }
            
            // Material component (if exists)
            if self.has_component(world, entity, "Material") {
                let material_section = self.create_material_section(world, entity);
                self.sections.push(material_section);
            }
            
            // Physics component (if exists)
            if self.has_component(world, entity, "Physics") {
                let physics_section = self.create_physics_section(world, entity);
                self.sections.push(physics_section);
            }
        }
    }

    fn create_transform_section(&self, world: &World, entity: Entity) -> ComponentSection {
        let mut section = ComponentSection::new("Transform".to_string());
        section.category = "Core".to_string();
        
        // Position
        section.add_property(Property::new(
            "Position".to_string(),
            PropertyValue::Vec3([0.0, 0.0, 0.0]), // Would be fetched from actual component
        ).with_category("Transform".to_string()));
        
        // Rotation
        section.add_property(Property::new(
            "Rotation".to_string(),
            PropertyValue::Vec3([0.0, 0.0, 0.0]),
        ).with_category("Transform".to_string()));
        
        // Scale
        section.add_property(Property::new(
            "Scale".to_string(),
            PropertyValue::Vec3([1.0, 1.0, 1.0]),
        ).with_category("Transform".to_string()));
        
        section
    }

    fn create_mesh_section(&self, world: &World, entity: Entity) -> ComponentSection {
        let mut section = ComponentSection::new("Mesh".to_string());
        section.category = "Rendering".to_string();
        
        section.add_property(Property::new(
            "Mesh Asset".to_string(),
            PropertyValue::String("default_mesh".to_string()),
        ).with_category("Mesh".to_string()));
        
        section.add_property(Property::new(
            "Cast Shadows".to_string(),
            PropertyValue::Bool(true),
        ).with_category("Mesh".to_string()));
        
        section.add_property(Property::new(
            "Receive Shadows".to_string(),
            PropertyValue::Bool(true),
        ).with_category("Mesh".to_string()));
        
        section
    }

    fn create_material_section(&self, world: &World, entity: Entity) -> ComponentSection {
        let mut section = ComponentSection::new("Material".to_string());
        section.category = "Rendering".to_string();
        
        section.add_property(Property::new(
            "Albedo".to_string(),
            PropertyValue::Color([1.0, 1.0, 1.0, 1.0]),
        ).with_category("Material".to_string()));
        
        section.add_property(Property::new(
            "Metallic".to_string(),
            PropertyValue::Float(0.0),
        ).with_category("Material".to_string()));
        
        section.add_property(Property::new(
            "Roughness".to_string(),
            PropertyValue::Float(0.5),
        ).with_category("Material".to_string()));
        
        section.add_property(Property::new(
            "Emissive".to_string(),
            PropertyValue::Color([0.0, 0.0, 0.0, 1.0]),
        ).with_category("Material".to_string()));
        
        section
    }

    fn create_physics_section(&self, world: &World, entity: Entity) -> ComponentSection {
        let mut section = ComponentSection::new("Physics".to_string());
        section.category = "Physics".to_string();
        
        section.add_property(Property::new(
            "Mass".to_string(),
            PropertyValue::Float(1.0),
        ).with_category("Physics".to_string()));
        
        section.add_property(Property::new(
            "Friction".to_string(),
            PropertyValue::Float(0.5),
        ).with_category("Physics".to_string()));
        
        section.add_property(Property::new(
            "Restitution".to_string(),
            PropertyValue::Float(0.0),
        ).with_category("Physics".to_string()));
        
        section.add_property(Property::new(
            "Is Kinematic".to_string(),
            PropertyValue::Bool(false),
        ).with_category("Physics".to_string()));
        
        section
    }

    fn has_component(&self, _world: &World, _entity: Entity, _component_type: &str) -> bool {
        // Basit implementasyon - gerçek implementasyonda ECS sorgusu gerekir
        false
    }

    pub fn update_property(&mut self, section_index: usize, property_index: usize, new_value: PropertyValue) {
        if let Some(section) = self.sections.get_mut(section_index) {
            if let Some(property) = section.properties.get_mut(property_index) {
                property.value = new_value;
            }
        }
    }

    pub fn add_component(&mut self, component_type: String) {
        let section = match component_type.as_str() {
            "Mesh" => self.create_mesh_section(&World::new(), Entity::from_bits(0)),
            "Material" => self.create_material_section(&World::new(), Entity::from_bits(0)),
            "Physics" => self.create_physics_section(&World::new(), Entity::from_bits(0)),
            _ => ComponentSection::new(component_type),
        };
        self.sections.push(section);
    }

    pub fn remove_component(&mut self, index: usize) {
        if index < self.sections.len() {
            self.sections.remove(index);
        }
    }

    pub fn toggle_component_enabled(&mut self, index: usize) {
        if let Some(section) = self.sections.get_mut(index) {
            section.is_enabled = !section.is_enabled;
        }
    }

    pub fn toggle_section_expanded(&mut self, index: usize) {
        if let Some(section) = self.sections.get_mut(index) {
            section.is_expanded = !section.is_expanded;
        }
    }

    pub fn set_search_filter(&mut self, filter: String) {
        self.search_filter = filter.to_lowercase();
    }

    pub fn get_filtered_sections(&self) -> Vec<&ComponentSection> {
        if self.search_filter.is_empty() {
            self.sections.iter().collect()
        } else {
            self.sections.iter()
                .filter(|section| {
                    section.component_type.to_lowercase().contains(&self.search_filter) ||
                    section.properties.iter().any(|p| p.name.to_lowercase().contains(&self.search_filter))
                })
                .collect()
        }
    }

    pub fn get_property_value(&self, section_index: usize, property_name: &str) -> Option<&PropertyValue> {
        self.sections.get(section_index)
            .and_then(|section| section.get_property(property_name))
            .map(|p| &p.value)
    }
}

impl Default for InspectorPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspector_creation() {
        let inspector = InspectorPanel::new();
        assert!(inspector.selected_entity.is_none());
        assert!(inspector.sections.is_empty());
    }

    #[test]
    fn test_property_creation() {
        let property = Property::new("test".to_string(), PropertyValue::Int(42));
        assert_eq!(property.name, "test");
        assert!(!property.read_only);
    }

    #[test]
    fn test_component_section() {
        let mut section = ComponentSection::new("TestComponent".to_string());
        section.add_property(Property::new("prop".to_string(), PropertyValue::Float(1.0)));
        assert_eq!(section.properties.len(), 1);
    }
}
