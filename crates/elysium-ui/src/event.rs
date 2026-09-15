//! Olay sistemi — UI olay türleri ve işleme.

use crate::types::{WidgetId, UiEvent};

/// Klavye olayı
#[derive(Clone, Debug)]
pub struct KeyEvent {
    pub key: String,
    pub pressed: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

/// Fare olayı
#[derive(Clone, Debug)]
pub struct MouseEvent {
    pub x: f32,
    pub y: f32,
    pub button: MouseButton,
    pub pressed: bool,
    pub scroll_delta: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Olay işleyici arayüzü
pub trait EventHandler {
    fn handle_event(&mut self, event: &UiEvent);
}

/// Olay bus'ı
pub struct EventBus {
    handlers: Vec<Box<dyn EventHandler>>,
}

impl EventBus {
    pub fn new() -> Self { Self { handlers: Vec::new() } }
    pub fn add_handler(&mut self, handler: Box<dyn EventHandler>) { self.handlers.push(handler); }
    pub fn dispatch(&mut self, event: &UiEvent) {
        for handler in &mut self.handlers { handler.handle_event(event); }
    }
    pub fn clear(&mut self) { self.handlers.clear(); }
    pub fn handler_count(&self) -> usize { self.handlers.len() }
}

/// Basit log olay işleyicisi
pub struct LogEventHandler;

impl EventHandler for LogEventHandler {
    fn handle_event(&mut self, event: &UiEvent) {
        match event {
            UiEvent::Click { id } => println!("[UI] Click on {:?}", id),
            UiEvent::Hover { id } => println!("[UI] Hover on {:?}", id),
            UiEvent::ValueChanged { id, .. } => println!("[UI] Value changed on {:?}", id),
            UiEvent::TextInput { id, text } => println!("[UI] Text on {:?}: {}", id, text),
            UiEvent::Scroll { id, delta } => println!("[UI] Scroll on {:?}: {}", id, delta),
            UiEvent::Drag { id, delta } => println!("[UI] Drag on {:?}: {:?}", id, delta),
            UiEvent::Resize { id, size } => println!("[UI] Resize on {:?}: {:?}", id, size),
            UiEvent::KeyPress { key, ctrl, shift, alt } => {
                println!("[UI] Key: {} (ctrl={}, shift={}, alt={})", key, ctrl, shift, alt);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::WidgetId;

    #[test]
    fn test_event_bus() {
        let mut bus = EventBus::new();
        bus.add_handler(Box::new(LogEventHandler));
        assert_eq!(bus.handler_count(), 1);
        bus.dispatch(&UiEvent::Click { id: WidgetId(1) });
        bus.clear();
        assert_eq!(bus.handler_count(), 0);
    }
}
