//! Widget davranışları — her widget türü için mantık.

use crate::types::{WidgetId, UiEvent, Widget};

/// Widget davranış arayüzü
pub trait WidgetBehavior {
    fn widget_type(&self) -> &str;
    fn update(&self, id: WidgetId, rect: [i32; 4], mouse_pos: [f32; 2]) -> WidgetUpdateResult;
    fn handle_click(&self, id: WidgetId) -> Option<UiEvent>;
}

/// Widget güncelleme sonucu
pub struct WidgetUpdateResult {
    pub hovered: bool,
    pub changed: bool,
}

/// Button davranışı
pub struct ButtonBehavior;

impl WidgetBehavior for ButtonBehavior {
    fn widget_type(&self) -> &str { "button" }
    fn update(&self, _id: WidgetId, rect: [i32; 4], mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        let hovered = mouse_pos[0] >= rect[0] as f32 && mouse_pos[0] < (rect[0] + rect[2]) as f32
            && mouse_pos[1] >= rect[1] as f32 && mouse_pos[1] < (rect[1] + rect[3]) as f32;
        WidgetUpdateResult { hovered, changed: false }
    }
    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

/// Checkbox davranışı
pub struct CheckboxBehavior;

impl WidgetBehavior for CheckboxBehavior {
    fn widget_type(&self) -> &str { "checkbox" }
    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }
    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

/// Slider davranışı
pub struct SliderBehavior;

impl WidgetBehavior for SliderBehavior {
    fn widget_type(&self) -> &str { "slider" }
    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }
    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

/// Tree düğümü
#[derive(Clone, Debug)]
pub struct TreeNode {
    pub id: WidgetId,
    pub label: String,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
    pub depth: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::WidgetId;

    #[test]
    fn test_button_behavior() {
        let b = ButtonBehavior;
        let id = WidgetId(1);
        let result = b.update(id, [0, 0, 100, 30], [50.0, 15.0]);
        assert!(result.hovered);
        let result = b.update(id, [0, 0, 100, 30], [200.0, 200.0]);
        assert!(!result.hovered);
    }
}
