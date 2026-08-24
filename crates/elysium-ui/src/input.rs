//! Input handling for UI

use glam::Vec2;
use std::collections::HashSet;

/// Mouse button identifiers
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u8),
}

/// Keyboard key codes
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Unknown,
    Escape,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    PrintScreen,
    ScrollLock,
    Pause,
    Backquote,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,
    Minus,
    Equal,
    Backspace,
    Tab,
    Q,
    W,
    E,
    R,
    T,
    Y,
    U,
    I,
    O,
    P,
    LeftBracket,
    RightBracket,
    Backslash,
    CapsLock,
    A,
    S,
    D,
    F,
    G,
    H,
    J,
    K,
    L,
    Semicolon,
    Quote,
    Enter,
    LeftShift,
    Z,
    X,
    C,
    V,
    B,
    N,
    M,
    Comma,
    Period,
    Slash,
    RightShift,
    LeftControl,
    LeftAlt,
    LeftMeta,
    Space,
    RightMeta,
    RightAlt,
    RightControl,
    Up,
    Down,
    Left,
    Right,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    NumLock,
    NumpadDivide,
    NumpadMultiply,
    NumpadSubtract,
    NumpadAdd,
    NumpadEnter,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    Numpad0,
    NumpadDecimal,
}

impl KeyCode {
    pub fn from_winit(key: winit::keyboard::KeyCode) -> Self {
        use winit::keyboard::KeyCode::*;
        match key {
            Escape => KeyCode::Escape,
            F1 => KeyCode::F1,
            F2 => KeyCode::F2,
            F3 => KeyCode::F3,
            F4 => KeyCode::F4,
            F5 => KeyCode::F5,
            F6 => KeyCode::F6,
            F7 => KeyCode::F7,
            F8 => KeyCode::F8,
            F9 => KeyCode::F9,
            F10 => KeyCode::F10,
            F11 => KeyCode::F11,
            F12 => KeyCode::F12,
            PrintScreen => KeyCode::PrintScreen,
            ScrollLock => KeyCode::ScrollLock,
            Pause => KeyCode::Pause,
            Backquote => KeyCode::Backquote,
            Digit1 => KeyCode::Key1,
            Digit2 => KeyCode::Key2,
            Digit3 => KeyCode::Key3,
            Digit4 => KeyCode::Key4,
            Digit5 => KeyCode::Key5,
            Digit6 => KeyCode::Key6,
            Digit7 => KeyCode::Key7,
            Digit8 => KeyCode::Key8,
            Digit9 => KeyCode::Key9,
            Digit0 => KeyCode::Key0,
            Minus => KeyCode::Minus,
            Equal => KeyCode::Equal,
            Backspace => KeyCode::Backspace,
            Tab => KeyCode::Tab,
            KeyQ => KeyCode::Q,
            KeyW => KeyCode::W,
            KeyE => KeyCode::E,
            KeyR => KeyCode::R,
            KeyT => KeyCode::T,
            KeyY => KeyCode::Y,
            KeyU => KeyCode::U,
            KeyI => KeyCode::I,
            KeyO => KeyCode::O,
            KeyP => KeyCode::P,
            BracketLeft => KeyCode::LeftBracket,
            BracketRight => KeyCode::RightBracket,
            Backslash => KeyCode::Backslash,
            CapsLock => KeyCode::CapsLock,
            KeyA => KeyCode::A,
            KeyS => KeyCode::S,
            KeyD => KeyCode::D,
            KeyF => KeyCode::F,
            KeyG => KeyCode::G,
            KeyH => KeyCode::H,
            KeyJ => KeyCode::J,
            KeyK => KeyCode::K,
            KeyL => KeyCode::L,
            Semicolon => KeyCode::Semicolon,
            Quote => KeyCode::Quote,
            Enter => KeyCode::Enter,
            ShiftLeft => KeyCode::LeftShift,
            KeyZ => KeyCode::Z,
            KeyX => KeyCode::X,
            KeyC => KeyCode::C,
            KeyV => KeyCode::V,
            KeyB => KeyCode::B,
            KeyN => KeyCode::N,
            KeyM => KeyCode::M,
            Comma => KeyCode::Comma,
            Period => KeyCode::Period,
            Slash => KeyCode::Slash,
            ShiftRight => KeyCode::RightShift,
            ControlLeft => KeyCode::LeftControl,
            AltLeft => KeyCode::LeftAlt,
            SuperLeft => KeyCode::LeftMeta,
            Space => KeyCode::Space,
            SuperRight => KeyCode::RightMeta,
            AltRight => KeyCode::RightAlt,
            ControlRight => KeyCode::RightControl,
            ArrowUp => KeyCode::Up,
            ArrowDown => KeyCode::Down,
            ArrowLeft => KeyCode::Left,
            ArrowRight => KeyCode::Right,
            Insert => KeyCode::Insert,
            Delete => KeyCode::Delete,
            Home => KeyCode::Home,
            End => KeyCode::End,
            PageUp => KeyCode::PageUp,
            PageDown => KeyCode::PageDown,
            NumLock => KeyCode::NumLock,
            NumpadDivide => KeyCode::NumpadDivide,
            NumpadMultiply => KeyCode::NumpadMultiply,
            NumpadSubtract => KeyCode::NumpadSubtract,
            NumpadAdd => KeyCode::NumpadAdd,
            NumpadEnter => KeyCode::NumpadEnter,
            Numpad1 => KeyCode::Numpad1,
            Numpad2 => KeyCode::Numpad2,
            Numpad3 => KeyCode::Numpad3,
            Numpad4 => KeyCode::Numpad4,
            Numpad5 => KeyCode::Numpad5,
            Numpad6 => KeyCode::Numpad6,
            Numpad7 => KeyCode::Numpad7,
            Numpad8 => KeyCode::Numpad8,
            Numpad9 => KeyCode::Numpad9,
            Numpad0 => KeyCode::Numpad0,
            NumpadDecimal => KeyCode::NumpadDecimal,
            _ => KeyCode::Unknown,
        }
    }
}

/// Convert winit KeyCode to our KeyCode
pub fn winit_key_to_keycode(key: winit::keyboard::KeyCode) -> KeyCode {
    KeyCode::from_winit(key)
}

/// Input state for the UI
pub struct InputState {
    mouse_pressed: HashSet<MouseButton>,
    mouse_down: HashSet<MouseButton>,
    mouse_released: HashSet<MouseButton>,
    #[allow(dead_code)]
    mouse_pos: Vec2,
    mouse_delta: Vec2,
    keys_pressed: HashSet<KeyCode>,
    keys_down: HashSet<KeyCode>,
    keys_released: HashSet<KeyCode>,
    chars: Vec<char>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            mouse_pressed: HashSet::new(),
            mouse_down: HashSet::new(),
            mouse_released: HashSet::new(),
            mouse_pos: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            keys_pressed: HashSet::new(),
            keys_down: HashSet::new(),
            keys_released: HashSet::new(),
            chars: Vec::new(),
        }
    }

    pub fn set_mouse_button(&mut self, button: MouseButton, pressed: bool) {
        if pressed {
            self.mouse_pressed.insert(button);
            self.mouse_down.insert(button);
            self.mouse_released.remove(&button);
        } else {
            self.mouse_down.remove(&button);
            self.mouse_released.insert(button);
            self.mouse_pressed.remove(&button);
        }
    }

    pub fn set_key(&mut self, key: KeyCode, pressed: bool) {
        if pressed {
            self.keys_pressed.insert(key);
            self.keys_down.insert(key);
            self.keys_released.remove(&key);
        } else {
            self.keys_down.remove(&key);
            self.keys_released.insert(key);
            self.keys_pressed.remove(&key);
        }
    }

    pub fn add_char(&mut self, c: char) {
        self.chars.push(c);
    }

    pub fn drain_chars(&mut self) -> Vec<char> {
        self.chars.drain(..).collect()
    }

    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_pressed.contains(&button)
    }

    pub fn mouse_down(&self, button: MouseButton) -> bool {
        self.mouse_down.contains(&button)
    }

    pub fn mouse_released(&self, button: MouseButton) -> bool {
        self.mouse_released.contains(&button)
    }

    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn key_released(&self, key: KeyCode) -> bool {
        self.keys_released.contains(&key)
    }

    pub fn end_frame(&mut self) {
        self.mouse_pressed.clear();
        self.mouse_released.clear();
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.chars.clear();
        self.mouse_delta = Vec2::ZERO;
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}
