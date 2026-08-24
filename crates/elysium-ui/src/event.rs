use gpui::{ElementId, Point, Pixels, Size};
use winit::event::WindowEvent;
use glam::Vec2;
use crate::{input::MouseButton, input::KeyCode, widgets::WidgetId};

#[derive(Debug, Clone)]
pub enum UiMessage {
    Click(ElementId),
    Hover(ElementId),
    Focus(ElementId),
    Blur(ElementId),
    KeyDown(KeyCode),
    KeyUp(KeyCode),
    MouseDown(MouseButton, Point<Pixels>),
    MouseUp(MouseButton, Point<Pixels>),
    MouseMove(Point<Pixels>),
    MouseEnter(ElementId),
    MouseLeave(ElementId),
    TextInput(String),
    Scroll(f32),
    Resize(Size<Pixels>),
    Close,
}

/// High-level UI events emitted by the immediate-mode system.
#[derive(Debug, Clone)]
pub enum UiEvent {
    /// Raw window event passed through for external observers.
    RawWindowEvent(WindowEvent),
    /// Mouse moved to a new position.
    MouseMove { pos: Vec2 },
    /// Mouse button press/release.
    MouseButton { button: MouseButton, pressed: bool, pos: Vec2 },
    /// Keyboard key press/release.
    Key { code: KeyCode, pressed: bool },
    /// Text input from IME or direct character typing.
    TextInput(String),
    /// A button widget was activated (clicked).
    ButtonClicked { id: WidgetId, label: String },
    Click { element_id: String },
    Hover { element_id: String },
    Unhover { element_id: String },
    Focus { element_id: String },
    Blur { element_id: String },
    TextChanged { element_id: String, text: String },
    ValueChanged { element_id: String, value: f32 },
    DragStart { element_id: String, start_pos: Vec2 },
    DragEnd { element_id: String, end_pos: Vec2 },
    MouseDownEvent { element_id: String, x: f32, y: f32 },
    MouseUp { element_id: String, x: f32, y: f32 },
}

// Re-export for easy access from the crate root.
// re-export removed