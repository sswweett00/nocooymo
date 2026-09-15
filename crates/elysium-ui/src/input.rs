//! Girdi sistemi — klavye ve fare girdisi UI için.

/// Klavye tuşu
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UiKey {
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    Space, Enter, Escape, Tab, Backspace, Delete,
    Up, Down, Left, Right,
    Home, End, PageUp, PageDown,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Unknown,
}

/// UI girdi durumu
pub struct UiInputState {
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_left: bool,
    pub mouse_right: bool,
    pub mouse_middle: bool,
    pub scroll_delta: f32,
    pub keys_pressed: Vec<UiKey>,
    pub text_input: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Default for UiInputState {
    fn default() -> Self {
        Self::new()
    }
}

impl UiInputState {
    pub fn new() -> Self {
        Self {
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_left: false,
            mouse_right: false,
            mouse_middle: false,
            scroll_delta: 0.0,
            keys_pressed: Vec::new(),
            text_input: String::new(),
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_x = x;
        self.mouse_y = y;
    }

    pub fn set_mouse_button(&mut self, button: super::event::MouseButton, pressed: bool) {
        match button {
            super::event::MouseButton::Left => self.mouse_left = pressed,
            super::event::MouseButton::Right => self.mouse_right = pressed,
            super::event::MouseButton::Middle => self.mouse_middle = pressed,
        }
    }

    pub fn set_scroll(&mut self, delta: f32) {
        self.scroll_delta = delta;
    }

    pub fn set_key(&mut self, key: UiKey, pressed: bool) {
        if pressed {
            if !self.keys_pressed.contains(&key) {
                self.keys_pressed.push(key);
            }
        } else {
            self.keys_pressed.retain(|k| *k != key);
        }

        // Modifier tuşları: Ctrl, Shift, Alt durumu çağrı tarafında yönetilir
    }

    pub fn is_key_pressed(&self, key: &UiKey) -> bool {
        self.keys_pressed.contains(key)
    }

    pub fn clear_frame(&mut self) {
        self.scroll_delta = 0.0;
        self.text_input.clear();
    }

    /// Copy shortcut kontrolü
    pub fn is_copy(&self) -> bool {
        self.ctrl && self.is_key_pressed(&UiKey::C)
    }

    /// Paste shortcut kontrolü
    pub fn is_paste(&self) -> bool {
        self.ctrl && self.is_key_pressed(&UiKey::V)
    }

    /// Undo shortcut kontrolü
    pub fn is_undo(&self) -> bool {
        self.ctrl && self.is_key_pressed(&UiKey::Z)
    }

    /// Redo shortcut kontrolü
    pub fn is_redo(&self) -> bool {
        (self.ctrl && self.shift && self.is_key_pressed(&UiKey::Z)) ||
        (self.ctrl && self.is_key_pressed(&UiKey::Y))
    }

    /// Select All shortcut kontrolü
    pub fn is_select_all(&self) -> bool {
        self.ctrl && self.is_key_pressed(&UiKey::A)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_state() {
        let mut state = UiInputState::new();
        state.set_mouse_position(100.0, 200.0);
        assert_eq!(state.mouse_x, 100.0);
        assert_eq!(state.mouse_y, 200.0);

        state.set_mouse_button(crate::event::MouseButton::Left, true);
        assert!(state.mouse_left);

        state.set_key(UiKey::Space, true);
        assert!(state.is_key_pressed(&UiKey::Space));

        state.set_key(UiKey::Space, false);
        assert!(!state.is_key_pressed(&UiKey::Space));
    }

    #[test]
    fn test_shortcuts() {
        let mut state = UiInputState::new();
        state.ctrl = true;
        state.set_key(UiKey::Z, true);
        assert!(state.is_undo());

        state.shift = true;
        assert!(state.is_redo());
    }
}
