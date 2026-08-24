use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::math::Vec2;

/// UI elementlerinin konum ve boyut bilgileri
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width &&
        y >= self.y && y <= self.y + self.height
    }
}

/// UI elementlerinin stilleri
#[derive(Debug, Clone)]
pub struct Style {
    pub background_color: Color,
    pub border_color: Color,
    pub text_color: Color,
    pub border_width: f32,
    pub border_radius: f32,
    pub padding: f32,
    pub margin: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background_color: Color::TRANSPARENT,
            border_color: Color::WHITE,
            text_color: Color::WHITE,
            border_width: 1.0,
            border_radius: 0.0,
            padding: 4.0,
            margin: 2.0,
        }
    }
}

/// Alias matching the engine's `UiStyle` naming.
pub type UiStyle = Style;

impl Style {
    /// Default button appearance.
    pub fn button_style() -> Self {
        Self {
            background_color: Color::new(0.2, 0.2, 0.2, 1.0),
            border_color: Color::WHITE,
            text_color: Color::WHITE,
            border_width: 1.0,
            border_radius: 4.0,
            padding: 6.0,
            margin: 2.0,
        }
    }

    /// Default panel appearance.
    pub fn panel_style() -> Self {
        Self {
            background_color: Color::new(0.1, 0.1, 0.1, 0.9),
            border_color: Color::WHITE,
            text_color: Color::WHITE,
            border_width: 1.0,
            border_radius: 0.0,
            padding: 8.0,
            margin: 4.0,
        }
    }
}

/// Rectangular spacing around a UI element.
#[derive(Debug, Clone, Copy, Default)]
pub struct UiMargin {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UiPadding {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct UiBorder {
    pub thickness: f32,
    pub color: [f32; 4],
}

impl Default for UiBorder {
    fn default() -> Self {
        Self { thickness: 0.0, color: [0.0, 0.0, 0.0, 1.0] }
    }
}

/// Renk tanımı
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const RED: Self = Self { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN: Self = Self { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const BLUE: Self = Self { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

/// UI element türleri
#[derive(Debug, Clone, PartialEq)]
pub enum UiElementType {
    Button,
    Label,
    TextBox,
    Image,
    Panel,
    Slider,
    ProgressBar,
    CheckBox,
    RadioButton,
}

/// UI elementi
#[derive(Debug, Clone)]
pub struct UiElement {
    pub id: String,
    pub element_type: UiElementType,
    pub position: Vec2,
    pub size: Vec2,
    pub pivot: Vec2, // Normalize edilmiş merkez noktası (0-1 arası)
    pub anchor_min: Vec2, // Minimum çapa noktası (0-1 arası)
    pub anchor_max: Vec2, // Maksimum çapa noktası (0-1 arası)
    pub offset_min: Vec2, // Anchor'ların minimum köşesine göre ofset
    pub offset_max: Vec2, // Anchor'ların maksimum köşesine göre ofset
    pub style: UiStyle,
    pub text: String,
    pub visible: bool,
    pub enabled: bool,
    pub children: Vec<String>,
    pub parent: Option<String>,
    pub z_index: i32,
    pub margin: UiMargin,
    pub padding: UiPadding,
    pub border: UiBorder,
}

impl UiElement {
    pub fn new(id: String, element_type: UiElementType, position: Vec2, size: Vec2) -> Self {
        Self {
            id,
            element_type,
            position,
            size,
            pivot: Vec2::new(0.5, 0.5),
            anchor_min: Vec2::new(0.0, 0.0),
            anchor_max: Vec2::new(0.0, 0.0),
            offset_min: Vec2::new(0.0, 0.0),
            offset_max: Vec2::new(0.0, 0.0),
            style: UiStyle::default(),
            text: String::new(),
            visible: true,
            enabled: true,
            children: Vec::new(),
            parent: None,
            z_index: 0,
            margin: UiMargin { left: 0.0, right: 0.0, top: 0.0, bottom: 0.0 },
            padding: UiPadding { left: 0.0, right: 0.0, top: 0.0, bottom: 0.0 },
            border: UiBorder { thickness: 0.0, color: [0.0, 0.0, 0.0, 1.0] },
        }
    }
    
    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.position = Vec2::new(x, y);
        self
    }
    
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.size = Vec2::new(width, height);
        self
    }
    
    pub fn with_text<T: Into<String>>(mut self, text: T) -> Self {
        self.text = text.into();
        self
    }
    
    pub fn with_style(mut self, style: UiStyle) -> Self {
        self.style = style;
        self
    }
    
    pub fn with_margin(mut self, left: f32, right: f32, top: f32, bottom: f32) -> Self {
        self.margin = UiMargin { left, right, top, bottom };
        self
    }
    
    pub fn with_padding(mut self, left: f32, right: f32, top: f32, bottom: f32) -> Self {
        self.padding = UiPadding { left, right, top, bottom };
        self
    }
    
    pub fn with_border(mut self, thickness: f32, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.border = UiBorder { thickness, color: [r, g, b, a] };
        self
    }
    
    pub fn set_visibility(&mut self, visible: bool) {
        self.visible = visible;
    }
    
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    pub fn calculate_absolute_position(&self, parent_size: Option<Vec2>) -> Vec2 {
        let parent_size = parent_size.unwrap_or(Vec2::new(1.0, 1.0));
        
        // Anchor'ları kullanarak pozisyonu hesapla
        let anchored_x = self.anchor_min.x * parent_size.x + self.offset_min.x;
        let anchored_y = self.anchor_min.y * parent_size.y + self.offset_min.y;
        
        // Pivot etrafında konumlandırma
        let pivot_offset_x = self.pivot.x * self.size.x;
        let pivot_offset_y = self.pivot.y * self.size.y;
        
        Vec2::new(anchored_x - pivot_offset_x, anchored_y - pivot_offset_y)
    }
    
    pub fn contains_point(&self, point: Vec2, absolute_pos: Vec2) -> bool {
        point.x >= absolute_pos.x && 
        point.x <= absolute_pos.x + self.size.x &&
        point.y >= absolute_pos.y && 
        point.y <= absolute_pos.y + self.size.y
    }
}

/// UI olayları
#[derive(Debug, Clone)]
pub enum UiEvent {
    Click { element_id: String },
    Hover { element_id: String },
    Unhover { element_id: String },
    Focus { element_id: String },
    Blur { element_id: String },
    TextChanged { element_id: String, text: String },
    ValueChanged { element_id: String, value: f32 },
    DragStart { element_id: String, start_pos: Vec2 },
    DragEnd { element_id: String, end_pos: Vec2 },
    MouseDown { element_id: String, x: f32, y: f32 },
    MouseUp { element_id: String, x: f32, y: f32 },
    TextInput { element_id: String, text: String },
}

/// UI sistem durumu
pub struct UiSystem {
    pub elements: HashMap<String, UiElement>,
    pub event_queue: Vec<UiEvent>,
    pub focused_element: Option<String>,
    pub hovered_element: Option<String>,
    pub root_elements: Vec<String>,
    pub screen_size: Vec2,
    pub styles: HashMap<String, UiStyle>,
    pub dirty_elements: Vec<String>, // Değişiklik yapılan elemanlar
}

impl Default for UiSystem {
    fn default() -> Self {
        Self::new(1920.0, 1080.0)
    }
}

impl UiSystem {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let mut styles = HashMap::new();
        styles.insert("default".to_string(), UiStyle::default());
        styles.insert("button".to_string(), UiStyle::button_style());
        styles.insert("panel".to_string(), UiStyle::panel_style());
        
        Self {
            elements: HashMap::new(),
            event_queue: Vec::new(),
            focused_element: None,
            hovered_element: None,
            root_elements: Vec::new(),
            screen_size: Vec2::new(screen_width, screen_height),
            styles,
            dirty_elements: Vec::new(),
        }
    }

    /// Yeni bir UI elementi ekler
    pub fn add_element(&mut self, element: UiElement) -> &mut UiElement {
        let id = element.id.clone();
        let parent = element.parent.clone();
        
        self.elements.insert(id.clone(), element);
        self.dirty_elements.push(id.clone());
        
        if let Some(parent_id) = parent {
            if let Some(parent_element) = self.elements.get_mut(&parent_id) {
                parent_element.children.push(id.clone());
            }
        } else {
            self.root_elements.push(id.clone());
        }
        
        self.elements.get_mut(&id).unwrap()
    }

    /// Elementi ID ile alır
    pub fn get_element(&mut self, id: &str) -> Option<&mut UiElement> {
        self.elements.get_mut(id)
    }

    /// Elementi siler
    pub fn remove_element(&mut self, id: &str) -> Option<UiElement> {
        let element = self.elements.remove(id)?;
        
        // Parent'ı olan elementleri güncelle
        if let Some(parent_id) = &element.parent {
            if let Some(parent) = self.elements.get_mut(parent_id) {
                parent.children.retain(|child_id| child_id != id);
            }
        }
        
        // Child elementleri de sil
        for child_id in &element.children {
            self.remove_element(child_id);
        }
        
        // Root element listesinden kaldır
        self.root_elements.retain(|root_id| root_id != id);
        
        Some(element)
    }

    /// Mouse pozisyonunu günceller
    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        let mouse_pos = Vec2::new(x, y);
        let mut new_hovered_element = None;
        
        // Elemanları z-index'e göre sırala (yüksek olan önde)
        let mut sorted_elements: Vec<(&String, &UiElement)> = self.elements.iter().collect();
        sorted_elements.sort_by(|a, b| b.1.z_index.cmp(&a.1.z_index));
        
        for (id, element) in sorted_elements {
            if !element.visible || !element.enabled {
                continue;
            }
            
            // Mutlak pozisyonu hesapla
            let absolute_pos = self.calculate_absolute_position(id, None);
            
            if element.contains_point(mouse_pos, absolute_pos) {
                new_hovered_element = Some(id.clone());
                
                // Yeni elemanın üzerine gelindi
                if self.hovered_element.as_ref() != Some(id) {
                    self.event_queue.push(UiEvent::Hover { element_id: id.clone() });
                }
                
                break; // Sadece en üstteki elemana tıklama
            }
        }
        
        // Önceki hover edilen element için Unhover olayı gönder
        if let Some(old_hovered) = &self.hovered_element {
            if old_hovered != new_hovered_element.as_ref().unwrap_or(&String::new()) {
                self.event_queue.push(UiEvent::Unhover { element_id: old_hovered.clone() });
            }
        }
        
        self.hovered_element = new_hovered_element;
    }

    /// Belirtilen pozisyonda hangi element olduğunu bulur
    fn calculate_absolute_position(&self, id: &str, _parent_id: Option<&str>) -> Vec2 {
        if let Some(element) = self.elements.get(id) {
            // Basit bir uygulama - daha gelişmiş bir sistem için hiyerarşik pozisyonlama yapılmalı
            Vec2::new(element.position.x, element.position.y)
        } else {
            Vec2::new(0.0, 0.0)
        }
    }

    /// Mouse click olayını işler
    pub fn handle_click(&mut self, x: f32, y: f32) {
        let mouse_pos = Vec2::new(x, y);
        
        // Elemanları z-index'e göre sırala (yüksek olan önde)
        let mut sorted_elements: Vec<(&String, &UiElement)> = self.elements.iter().collect();
        sorted_elements.sort_by(|a, b| b.1.z_index.cmp(&a.1.z_index));
        
        for (id, element) in sorted_elements.iter() {
            if !element.visible || !element.enabled {
                continue;
            }
            
            // Mutlak pozisyonu hesapla
            let absolute_pos = self.calculate_absolute_position(id, None);
            
            if element.contains_point(mouse_pos, absolute_pos) {
                // Olayı tetikle
                self.event_queue.push(UiEvent::Click { element_id: (*id).clone() });
                
                // Focus değiştir
                if self.focused_element.as_ref() != Some(id) {
                    if let Some(old_focus) = self.focused_element.take() {
                        self.event_queue.push(UiEvent::Blur { element_id: old_focus });
                    }
                    
                    self.focused_element = Some((*id).clone());
                    self.event_queue.push(UiEvent::Focus { element_id: (*id).clone() });
                }
                
                break; // Sadece en üstteki elemana tıklama
            }
        }
    }

    /// Text input olayını işler
    pub fn handle_text_input(&mut self, text: String) {
        if let Some(ref element_id) = self.focused_element {
            self.event_queue.push(UiEvent::TextInput { element_id: element_id.clone(), text });
        }
    }

    /// UI sistemini günceller
    pub fn update(&mut self) {
        // Burada layout calculation, animation updates, vs. olabilir
        // Şimdilik sadece olay kuyruğunu temizle
        self.event_queue.clear();
    }

    /// UI render için elementleri sıraya koyar
    pub fn get_render_order(&self) -> Vec<String> {
        let mut elements: Vec<(&String, &UiElement)> = self.elements.iter().collect();
        elements.sort_by(|a, b| a.1.z_index.cmp(&b.1.z_index));
        
        elements.into_iter().map(|(id, _)| id.clone()).collect()
    }
    
    /// Ekran boyutunu günceller
    pub fn resize(&mut self, width: f32, height: f32) {
        self.screen_size = Vec2::new(width, height);
        
        // Gerekirse layout'u yeniden hesapla
        // Burada layout sistemi daha gelişmiş olabilir
    }
    
    /// Element stilini alır
    pub fn get_style(&self, style_name: &str) -> Option<&UiStyle> {
        self.styles.get(style_name)
    }
    
    /// Element stilini ayarlar
    pub fn set_style(&mut self, style_name: String, style: UiStyle) {
        self.styles.insert(style_name, style);
    }

    /// Elementleri ID'ye göre arar
    pub fn find_elements_by_type(&self, element_type: &UiElementType) -> Vec<String> {
        self.elements
            .iter()
            .filter(|(_, element)| element.element_type == *element_type)
            .map(|(id, _)| id.clone())
            .collect()
    }
}

/// UI builder için yardımcı trait
pub trait UiBuilder {
    fn build_ui(&mut self) -> UiSystem;
}

/// UI stil tanımları için yardımcı yapılar
pub struct Theme {
    pub button_style: Style,
    pub label_style: Style,
    pub panel_style: Style,
    pub text_color: Color,
    pub background_color: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            button_style: Style {
                background_color: Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 },
                border_color: Color { r: 0.5, g: 0.5, b: 0.5, a: 1.0 },
                text_color: Color::WHITE,
                border_width: 1.0,
                border_radius: 4.0,
                padding: 8.0,
                margin: 4.0,
            },
            label_style: Style {
                background_color: Color::TRANSPARENT,
                border_color: Color::TRANSPARENT,
                text_color: Color::WHITE,
                border_width: 0.0,
                border_radius: 0.0,
                padding: 2.0,
                margin: 2.0,
            },
            panel_style: Style {
                background_color: Color { r: 0.1, g: 0.1, b: 0.1, a: 0.8 },
                border_color: Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 },
                text_color: Color::WHITE,
                border_width: 1.0,
                border_radius: 0.0,
                padding: 8.0,
                margin: 4.0,
            },
            text_color: Color::WHITE,
            background_color: Color { r: 0.05, g: 0.05, b: 0.05, a: 1.0 },
        }
    }
}