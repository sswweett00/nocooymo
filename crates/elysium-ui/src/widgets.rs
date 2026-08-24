//! Widget system for UI

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

/// Unique identifier for widgets
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u64);

impl WidgetId {
    pub fn new(label: &str, parent_id: u64) -> Self {
        let mut hasher = DefaultHasher::new();
        parent_id.hash(&mut hasher);
        label.hash(&mut hasher);
        WidgetId(hasher.finish())
    }

    pub fn from_u64(id: u64) -> Self {
        WidgetId(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

use std::sync::Arc;

/// Widget state storage
#[derive(Clone, Debug, Default)]
pub struct WidgetState {
    pub open: bool,
    pub hovered: bool,
    pub active: bool,
    pub focused: bool,
    pub value_f32: f32,
    pub value_string: String,
    pub selected_index: Option<usize>,
    pub scroll_offset: f32,
    pub custom_data: Option<Arc<dyn std::any::Any + Send + Sync>>,
    pub animation_progress: f32,
    pub drag_offset: Option<(f32, f32)>,
    pub last_click_time: Option<std::time::Instant>,
}

impl WidgetState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.hovered = false;
        self.active = false;
        self.selected_index = None;
        self.drag_offset = None;
    }

    pub fn set_value_f32(&mut self, value: f32) {
        self.value_f32 = value.clamp(0.0, 1.0); // Normalized değer aralığı
    }

    pub fn set_string(&mut self, s: String) {
        self.value_string = s;
    }

    pub fn toggle_open(&mut self) {
        self.open = !self.open;
    }

    pub fn set_selected_index(&mut self, index: usize) {
        self.selected_index = Some(index);
    }

    pub fn start_drag(&mut self, offset_x: f32, offset_y: f32) {
        self.drag_offset = Some((offset_x, offset_y));
    }

    pub fn stop_drag(&mut self) {
        self.drag_offset = None;
    }

    pub fn update_animation(&mut self, delta_time: Duration) {
        let progress_increment = (delta_time.as_secs_f32() * 2.0).min(1.0); // 0.5s animasyon süresi
        self.animation_progress = (self.animation_progress + progress_increment).min(1.0);
    }

    pub fn is_double_clicked(&mut self, threshold: Duration) -> bool {
        if let Some(last_click) = self.last_click_time {
            if std::time::Instant::now().duration_since(last_click) < threshold {
                // Çift tıklama algılandı
                self.last_click_time = None; // Yeni tıklama için sıfırla
                return true;
            }
        }
        self.last_click_time = Some(std::time::Instant::now());
        false
    }
}

/// Animation types for widgets
#[derive(Clone, Debug)]
pub enum AnimationType {
    FadeIn,
    FadeOut,
    SlideIn(Direction),
    SlideOut(Direction),
    Scale(f32),
    Bounce,
    Pulse,
}

#[derive(Clone, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Tree node state for hierarchical widgets
#[derive(Clone, Debug, Default)]
pub struct TreeNodeState {
    pub expanded: bool,
    pub selected: bool,
    pub depth: u32,
}

impl TreeNodeState {
    pub fn new(depth: u32) -> Self {
        Self {
            expanded: false,
            selected: false,
            depth,
        }
    }

    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }
}

/// Tab bar state
#[derive(Clone, Debug, Default)]
pub struct TabBarState {
    pub active_tab: Option<usize>,
    pub hovered_tab: Option<usize>,
}

impl TabBarState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_active_tab(&mut self, index: usize) {
        self.active_tab = Some(index);
    }

    pub fn next_tab(&mut self, count: usize) {
        if let Some(current) = self.active_tab {
            if count > 0 {
                self.active_tab = Some((current + 1) % count);
            }
        } else if count > 0 {
            self.active_tab = Some(0);
        }
    }

    pub fn prev_tab(&mut self, count: usize) {
        if let Some(current) = self.active_tab {
            if count > 0 {
                self.active_tab = Some(if current == 0 { count - 1 } else { current - 1 });
            }
        } else if count > 0 {
            self.active_tab = Some(0);
        }
    }
}

/// Tooltip state
#[derive(Clone, Debug, Default)]
pub struct TooltipState {
    pub text: String,
    pub position: Option<(f32, f32)>,
    pub visible: bool,
    pub delay: Duration,
}

impl TooltipState {
    pub fn new() -> Self {
        Self {
            delay: Duration::from_millis(500), // 0.5 saniye gecikme
            ..Default::default()
        }
    }

    pub fn show(&mut self, text: impl Into<String>, x: f32, y: f32) {
        self.text = text.into();
        self.position = Some((x, y));
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn update_position(&mut self, x: f32, y: f32) {
        if self.visible {
            self.position = Some((x, y));
        }
    }
}

/// Slider state
#[derive(Clone, Debug, Default)]
pub struct SliderState {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: Option<f32>,
}

impl SliderState {
    pub fn new(min: f32, max: f32, initial: f32) -> Self {
        Self {
            value: initial.clamp(min, max),
            min,
            max,
            step: None,
        }
    }

    pub fn set_value(&mut self, value: f32) {
        self.value = value.clamp(self.min, self.max);
        if let Some(step) = self.step {
            self.value = (self.value / step).round() * step;
        }
    }

    pub fn normalized_value(&self) -> f32 {
        if self.max == self.min {
            0.0
        } else {
            (self.value - self.min) / (self.max - self.min)
        }
    }

    pub fn set_normalized_value(&mut self, normalized: f32) {
        let raw_value = self.min + normalized * (self.max - self.min);
        self.set_value(raw_value);
    }
}

/// Progress bar state
#[derive(Clone, Debug, Default)]
pub struct ProgressBarState {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub indeterminate: bool,
}

impl ProgressBarState {
    pub fn new(min: f32, max: f32) -> Self {
        Self {
            min,
            max,
            value: min,
            indeterminate: false,
        }
    }

    pub fn set_value(&mut self, value: f32) {
        self.value = value.clamp(self.min, self.max);
    }

    pub fn progress(&self) -> f32 {
        if self.max == self.min {
            0.0
        } else {
            (self.value - self.min) / (self.max - self.min)
        }
    }
}

/// Drag and drop state
#[derive(Clone, Debug, Default)]
pub struct DragDropState {
    pub is_dragging: bool,
    pub drag_source: Option<WidgetId>,
    pub drag_data: Option<Arc<dyn std::any::Any + Send + Sync>>,
    pub hover_target: Option<WidgetId>,
}

impl DragDropState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_drag(&mut self, source: WidgetId, data: Arc<dyn std::any::Any + Send + Sync>) {
        self.is_dragging = true;
        self.drag_source = Some(source);
        self.drag_data = Some(data);
    }

    pub fn end_drag(&mut self) {
        self.is_dragging = false;
        self.drag_source = None;
        self.drag_data = None;
    }

    pub fn is_over_target(&self, target: WidgetId) -> bool {
        self.hover_target == Some(target) && self.is_dragging
    }
}

/// Responsive layout state
#[derive(Clone, Debug, Default)]
pub struct ResponsiveState {
    pub screen_size: (f32, f32),
    pub current_breakpoint: Breakpoint,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Breakpoint {
    Mobile,
    Tablet,
    Desktop,
    LargeDesktop,
}

impl Default for Breakpoint {
    fn default() -> Self {
        Breakpoint::Desktop
    }
}

impl ResponsiveState {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let breakpoint = if screen_width < 768.0 {
            Breakpoint::Mobile
        } else if screen_width < 1024.0 {
            Breakpoint::Tablet
        } else if screen_width < 1440.0 {
            Breakpoint::Desktop
        } else {
            Breakpoint::LargeDesktop
        };

        Self {
            screen_size: (screen_width, screen_height),
            current_breakpoint: breakpoint,
        }
    }

    pub fn should_use_compact_ui(&self) -> bool {
        matches!(self.current_breakpoint, Breakpoint::Mobile | Breakpoint::Tablet)
    }
}

/// Modal state
#[derive(Clone, Debug, Default)]
pub struct ModalState {
    pub visible: bool,
    pub title: String,
    pub content_size: (f32, f32),
}

impl ModalState {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            visible: false,
            title: title.into(),
            content_size: (400.0, 300.0),
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }
}

/// Menu state
#[derive(Clone, Debug, Default)]
pub struct MenuState {
    pub visible: bool,
    pub items: Vec<MenuItem>,
    pub selected_index: Option<usize>,
    pub position: (f32, f32),
}

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub id: WidgetId,
    pub text: String,
    pub enabled: bool,
    pub shortcut: Option<String>,
}

impl MenuItem {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            id: WidgetId::new(&text.as_ref().to_lowercase(), 0),
            text: text.into(),
            enabled: true,
            shortcut: None,
        }
    }
}

impl MenuState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_item(&mut self, item: MenuItem) {
        self.items.push(item);
    }

    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        
        match self.selected_index {
            Some(index) => {
                if index + 1 < self.items.len() {
                    self.selected_index = Some(index + 1);
                } else {
                    self.selected_index = Some(0); // Döngüsel seçim
                }
            }
            None => self.selected_index = Some(0),
        }
    }

    pub fn select_prev(&mut self) {
        if self.items.is_empty() {
            return;
        }
        
        match self.selected_index {
            Some(0) => self.selected_index = Some(self.items.len() - 1),
            Some(index) => self.selected_index = Some(index - 1),
            None => self.selected_index = Some(0),
        }
    }

    pub fn activate_selected(&mut self) -> Option<WidgetId> {
        if let Some(index) = self.selected_index {
            if index < self.items.len() {
                return Some(self.items[index].id);
            }
        }
        None
    }
}