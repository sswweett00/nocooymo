//! Paylaşılan UI türleri — tüm modüller tarafından kullanılır.

use serde::{Serialize, Deserialize};

/// Widget kimliği
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WidgetId(pub u64);

impl WidgetId {
    pub const INVALID: Self = Self(u64::MAX);
    pub fn new(id: u64) -> Self { Self(id) }
}

/// Widget türü
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WidgetType {
    Panel,
    Button,
    Label,
    TextInput,
    Checkbox,
    Slider,
    Dropdown,
    Separator,
    Spacer,
    Image,
    ProgressBar,
    Tooltip,
    Window,
    ScrollArea,
    Tab,
    Tree,
    ColorPicker,
}

/// Widget verisi
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Widget {
    pub id: WidgetId,
    pub widget_type: WidgetType,
    pub rect: [i32; 4],
    pub visible: bool,
    pub interactive: bool,
    pub tooltip: Option<String>,
    pub parent: Option<WidgetId>,
    pub children: Vec<WidgetId>,
    pub state: WidgetState,
}

/// Widget durumu
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WidgetState {
    pub hovered: bool,
    pub focused: bool,
    pub pressed: bool,
    pub value_changed: bool,
    pub text_value: Option<String>,
    pub float_value: Option<f32>,
    pub bool_value: Option<bool>,
    pub int_value: Option<i32>,
    pub selected_index: Option<usize>,
}

/// UI olayı
#[derive(Clone, Debug)]
pub enum UiEvent {
    Click { id: WidgetId },
    Hover { id: WidgetId },
    ValueChanged { id: WidgetId, value: WidgetState },
    TextInput { id: WidgetId, text: String },
    Scroll { id: WidgetId, delta: f32 },
    Drag { id: WidgetId, delta: [f32; 2] },
    Resize { id: WidgetId, size: [i32; 2] },
    KeyPress { key: String, ctrl: bool, shift: bool, alt: bool },
}

// Widget oluşturucu fonksiyonlar

pub fn make_button(id: WidgetId, x: i32, y: i32, w: i32, h: i32) -> Widget {
    Widget { id, widget_type: WidgetType::Button, rect: [x, y, w, h], visible: true, interactive: true, tooltip: None, parent: None, children: Vec::new(), state: WidgetState::default() }
}

pub fn make_label(id: WidgetId, x: i32, y: i32, text: &str) -> Widget {
    let mut w = Widget { id, widget_type: WidgetType::Label, rect: [x, y, text.len() as i32 * 8, 16], visible: true, interactive: false, tooltip: None, parent: None, children: Vec::new(), state: WidgetState::default() };
    w.state.text_value = Some(text.to_string());
    w
}

pub fn make_checkbox(id: WidgetId, x: i32, y: i32, checked: bool) -> Widget {
    let mut w = Widget { id, widget_type: WidgetType::Checkbox, rect: [x, y, 16, 16], visible: true, interactive: true, tooltip: None, parent: None, children: Vec::new(), state: WidgetState::default() };
    w.state.bool_value = Some(checked);
    w
}

pub fn make_slider(id: WidgetId, x: i32, y: i32, w: i32, value: f32, min: f32, max: f32) -> Widget {
    let mut widget = Widget { id, widget_type: WidgetType::Slider, rect: [x, y, w, 20], visible: true, interactive: true, tooltip: None, parent: None, children: Vec::new(), state: WidgetState::default() };
    widget.state.float_value = Some(if max > min { (value - min) / (max - min) } else { 0.0 });
    widget
}

pub fn make_progress_bar(id: WidgetId, x: i32, y: i32, w: i32, progress: f32) -> Widget {
    let mut widget = Widget { id, widget_type: WidgetType::ProgressBar, rect: [x, y, w, 12], visible: true, interactive: false, tooltip: None, parent: None, children: Vec::new(), state: WidgetState::default() };
    widget.state.float_value = Some(progress.clamp(0.0, 1.0));
    widget
}

pub fn make_panel(id: WidgetId, x: i32, y: i32, w: i32, h: i32) -> Widget {
    Widget { id, widget_type: WidgetType::Panel, rect: [x, y, w, h], visible: true, interactive: false, tooltip: None, parent: None, children: Vec::new(), state: WidgetState::default() }
}

pub fn make_separator(id: WidgetId, x: i32, y: i32, w: i32) -> Widget {
    Widget { id, widget_type: WidgetType::Separator, rect: [x, y, w, 2], visible: true, interactive: false, tooltip: None, parent: None, children: Vec::new(), state: WidgetState::default() }
}

pub fn make_window(id: WidgetId, x: i32, y: i32, w: i32, h: i32, title: &str) -> Widget {
    let mut widget = Widget { id, widget_type: WidgetType::Window, rect: [x, y, w, h], visible: true, interactive: true, tooltip: None, parent: None, children: Vec::new(), state: WidgetState::default() };
    widget.state.text_value = Some(title.to_string());
    widget
}

pub fn make_dropdown(id: WidgetId, x: i32, y: i32, w: i32, options: &[&str], selected: usize) -> Widget {
    let mut widget = Widget { id, widget_type: WidgetType::Dropdown, rect: [x, y, w, 24], visible: true, interactive: true, tooltip: None, parent: None, children: Vec::new(), state: WidgetState::default() };
    widget.state.text_value = Some(options.join("|"));
    widget.state.selected_index = Some(selected);
    widget
}

pub fn make_scroll_area(id: WidgetId, x: i32, y: i32, w: i32, h: i32) -> Widget {
    Widget { id, widget_type: WidgetType::ScrollArea, rect: [x, y, w, h], visible: true, interactive: true, tooltip: None, parent: None, children: Vec::new(), state: WidgetState::default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_creation() {
        let id = WidgetId(1);
        let btn = make_button(id, 10, 20, 100, 30);
        assert_eq!(btn.id, id);
        assert_eq!(btn.widget_type, WidgetType::Button);
        assert_eq!(btn.rect, [10, 20, 100, 30]);
        assert!(btn.interactive);
    }

    #[test]
    fn test_checkbox_creation() {
        let id = WidgetId(2);
        let cb = make_checkbox(id, 0, 0, true);
        assert_eq!(cb.state.bool_value, Some(true));
    }

    #[test]
    fn test_slider_creation() {
        let id = WidgetId(3);
        let sl = make_slider(id, 0, 0, 200, 0.5, 0.0, 1.0);
        assert_eq!(sl.state.float_value, Some(0.5));
    }
}
