//! # Elysium UI — Immediate Mode GUI System
//!
//! Software framebuffer üzerinde çalışan, GPU hızlandırma gerektirmeyen
//! immediate-mode UI sistemi.

pub mod types;
pub mod context;
pub mod layout;
pub mod widgets;
pub mod theme;
pub mod event;
pub mod input;
pub mod draw;
pub mod font;
pub mod command;

// Paylaşılan türleri yeniden dışa aktar
pub use types::*;

use std::collections::HashMap;

/// UI bağlamı — tüm widget durumunu tutar
pub struct UiContext {
    pub widgets: HashMap<WidgetId, Widget>,
    pub events: Vec<UiEvent>,
    pub next_id: u64,
    pub hot_widget: Option<WidgetId>,
    pub active_widget: Option<WidgetId>,
    pub focused_widget: Option<WidgetId>,
    pub mouse_pos: [f32; 2],
    pub mouse_down: bool,
    pub input_text: String,
    pub frame_count: u64,
}

impl Default for UiContext {
    fn default() -> Self {
        Self::new()
    }
}

impl UiContext {
    pub fn new() -> Self {
        Self {
            widgets: HashMap::new(),
            events: Vec::new(),
            next_id: 1,
            hot_widget: None,
            active_widget: None,
            focused_widget: None,
            mouse_pos: [0.0; 2],
            mouse_down: false,
            input_text: String::new(),
            frame_count: 0,
        }
    }

    pub fn next_id(&mut self) -> WidgetId {
        let id = WidgetId(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn add_widget(&mut self, widget: Widget) -> WidgetId {
        let id = widget.id;
        self.widgets.insert(id, widget);
        id
    }

    pub fn get_widget(&self, id: WidgetId) -> Option<&Widget> {
        self.widgets.get(&id)
    }

    pub fn set_hovered(&mut self, id: WidgetId, hovered: bool) {
        if let Some(w) = self.widgets.get_mut(&id) {
            w.state.hovered = hovered;
        }
    }

    pub fn set_pressed(&mut self, id: WidgetId, pressed: bool) {
        if let Some(w) = self.widgets.get_mut(&id) {
            w.state.pressed = pressed;
        }
    }

    pub fn push_event(&mut self, event: UiEvent) {
        self.events.push(event);
    }

    pub fn drain_events(&mut self) -> Vec<UiEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn set_mouse_pos(&mut self, x: f32, y: f32) {
        self.mouse_pos = [x, y];
    }

    pub fn is_hovered(&self, id: WidgetId) -> bool {
        self.widgets.get(&id).map(|w| w.state.hovered).unwrap_or(false)
    }

    pub fn is_pressed(&self, id: WidgetId) -> bool {
        self.widgets.get(&id).map(|w| w.state.pressed).unwrap_or(false)
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<WidgetId> {
        for (id, widget) in &self.widgets {
            if !widget.visible || !widget.interactive { continue; }
            let [wx, wy, ww, wh] = widget.rect;
            if x >= wx as f32 && x < (wx + ww) as f32 &&
               y >= wy as f32 && y < (wy + wh) as f32 {
                return Some(*id);
            }
        }
        None
    }

    pub fn end_frame(&mut self) {
        self.frame_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_context() {
        let mut ctx = UiContext::new();
        let id = ctx.next_id();
        let btn = make_button(id, 0, 0, 100, 30);
        ctx.add_widget(btn);
        assert!(ctx.get_widget(id).is_some());
        assert_eq!(ctx.hit_test(50.0, 15.0), Some(id));
        assert_eq!(ctx.hit_test(200.0, 200.0), None);
    }

    #[test]
    fn test_events() {
        let mut ctx = UiContext::new();
        let id = ctx.next_id();
        ctx.push_event(UiEvent::Click { id });
        let events = ctx.drain_events();
        assert_eq!(events.len(), 1);
        assert!(ctx.events.is_empty());
    }
}
