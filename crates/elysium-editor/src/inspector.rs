//! Inspector Panel — varlık özelliklerini görüntüler ve düzenler.

use std::collections::HashMap;

/// Özellik değeri
#[derive(Debug, Clone)]
pub enum PropertyValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Color([f32; 4]),
}

/// Özellik
#[derive(Debug, Clone)]
pub struct Property {
    pub name: String,
    pub value: PropertyValue,
    pub read_only: bool,
    pub category: String,
}

impl Property {
    pub fn new(name: &str, value: PropertyValue) -> Self {
        Self { name: name.to_string(), value, read_only: false, category: "Default".to_string() }
    }

    pub fn read_only(mut self) -> Self { self.read_only = true; self }
    pub fn with_category(mut self, cat: &str) -> Self { self.category = cat.to_string(); self }
}

/// Bileşen bölümü
#[derive(Debug, Clone)]
pub struct ComponentSection {
    pub component_type: String,
    pub properties: Vec<Property>,
    pub enabled: bool,
    pub expanded: bool,
}

impl ComponentSection {
    pub fn new(component_type: &str) -> Self {
        Self { component_type: component_type.to_string(), properties: Vec::new(), enabled: true, expanded: true }
    }

    pub fn add_property(&mut self, prop: Property) { self.properties.push(prop); }
}

/// Inspector paneli
pub struct InspectorPanel {
    pub selected_entity: Option<u32>,
    pub sections: Vec<ComponentSection>,
    pub search_filter: String,
}

impl Default for InspectorPanel {
    fn default() -> Self { Self::new() }
}

impl InspectorPanel {
    pub fn new() -> Self {
        Self { selected_entity: None, sections: Vec::new(), search_filter: String::new() }
    }

    pub fn select(&mut self, entity_id: u32) {
        self.selected_entity = Some(entity_id);
        self.sections.clear();

        // Transform bölümü
        let mut transform = ComponentSection::new("Transform");
        transform.add_property(Property::new("Position", PropertyValue::Vec3([0.0, 0.0, 0.0])).with_category("Transform"));
        transform.add_property(Property::new("Rotation", PropertyValue::Vec3([0.0, 0.0, 0.0])).with_category("Transform"));
        transform.add_property(Property::new("Scale", PropertyValue::Vec3([1.0, 1.0, 1.0])).with_category("Transform"));
        self.sections.push(transform);

        // PBR Materyal bölümü
        let mut material = ComponentSection::new("PBR Material");
        material.add_property(Property::new("Albedo", PropertyValue::Color([1.0, 1.0, 1.0, 1.0])).with_category("Material"));
        material.add_property(Property::new("Metallic", PropertyValue::Float(0.0)).with_category("Material"));
        material.add_property(Property::new("Roughness", PropertyValue::Float(0.5)).with_category("Material"));
        material.add_property(Property::new("AO", PropertyValue::Float(1.0)).with_category("Material"));
        material.add_property(Property::new("Emissive", PropertyValue::Color([0.0, 0.0, 0.0, 0.0])).with_category("Material"));
        self.sections.push(material);

        // Physics bölümü
        let mut physics = ComponentSection::new("Physics");
        physics.add_property(Property::new("Body Type", PropertyValue::String("Dynamic".to_string())).with_category("Physics"));
        physics.add_property(Property::new("Mass", PropertyValue::Float(1.0)).with_category("Physics"));
        physics.add_property(Property::new("Friction", PropertyValue::Float(0.5)).with_category("Physics"));
        physics.add_property(Property::new("Restitution", PropertyValue::Float(0.3)).with_category("Physics"));
        self.sections.push(physics);
    }

    pub fn deselect(&mut self) {
        self.selected_entity = None;
        self.sections.clear();
    }

    pub fn set_search(&mut self, filter: &str) {
        self.search_filter = filter.to_lowercase();
    }

    pub fn get_filtered(&self) -> Vec<&ComponentSection> {
        if self.search_filter.is_empty() {
            self.sections.iter().collect()
        } else {
            self.sections.iter()
                .filter(|s| s.component_type.to_lowercase().contains(&self.search_filter))
                .collect()
        }
    }
}
