use std::collections::HashMap;
use std::time::Duration;

// Tuş kodları enum'ı (platform bağımsız)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Harfler
    KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ,
    KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR, KeyS, KeyT,
    KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
    
    // Rakamlar
    Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
    
    // Fonksiyon tuşları
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    
    // Yön tuşları
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    
    // Kontrol tuşları
    Space, Enter, Escape, Tab, Backspace, Delete, Insert,
    Home, End, PageUp, PageDown,
    
    // Shift, Ctrl, Alt
    ShiftLeft, ShiftRight, ControlLeft, ControlRight, AltLeft, AltRight,
    
    // Diğeler
    CapsLock, NumLock, ScrollLock,
    PrintScreen, Pause, Menu,
    
    // Fare düğmeleri
    MouseButtonLeft, MouseButtonRight, MouseButtonMiddle,
    MouseButton4, MouseButton5,
    
    // Oyun kontrolleri
    GamepadA, GamepadB, GamepadX, GamepadY,
    GamepadUp, GamepadDown, GamepadLeft, GamepadRight,
    GamepadStart, GamepadSelect,
    GamepadLeftShoulder, GamepadRightShoulder,
    GamepadLeftTrigger, GamepadRightTrigger,
    GamepadLeftStick, GamepadRightStick,
}

// Tuş durumu enum'ı
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyState {
    Pressed,
    Released,
    Down,
    Up,
}

// Fare konumu
#[derive(Debug, Clone, Copy)]
pub struct MousePosition {
    pub x: f32,
    pub y: f32,
}

// Oyun kontrolleri için eksenler
#[derive(Debug, Clone, Copy)]
pub struct GamepadAxes {
    pub left_stick_x: f32,
    pub left_stick_y: f32,
    pub right_stick_x: f32,
    pub right_stick_y: f32,
    pub left_trigger: f32,  // 0.0 to 1.0
    pub right_trigger: f32, // 0.0 to 1.0
}

// Oyun kontrolleri için düğme durumları
#[derive(Debug, Clone, Copy)]
pub struct GamepadButtons {
    pub a: KeyState,
    pub b: KeyState,
    pub x: KeyState,
    pub y: KeyState,
    pub up: KeyState,
    pub down: KeyState,
    pub left: KeyState,
    pub right: KeyState,
    pub start: KeyState,
    pub select: KeyState,
    pub left_shoulder: KeyState,
    pub right_shoulder: KeyState,
    pub left_trigger: KeyState,
    pub right_trigger: KeyState,
    pub left_stick: KeyState,
    pub right_stick: KeyState,
}

impl Default for GamepadButtons {
    fn default() -> Self {
        Self {
            a: KeyState::Up,
            b: KeyState::Up,
            x: KeyState::Up,
            y: KeyState::Up,
            up: KeyState::Up,
            down: KeyState::Up,
            left: KeyState::Up,
            right: KeyState::Up,
            start: KeyState::Up,
            select: KeyState::Up,
            left_shoulder: KeyState::Up,
            right_shoulder: KeyState::Up,
            left_trigger: KeyState::Up,
            right_trigger: KeyState::Up,
            left_stick: KeyState::Up,
            right_stick: KeyState::Up,
        }
    }
}

// Girdi haritası girdisi
#[derive(Debug, Clone)]
pub struct InputAction {
    pub name: String,
    pub bindings: Vec<InputBinding>,
    pub axis_deadzone: f32,
}

// Girdi bağlaması
#[derive(Debug, Clone)]
pub enum InputBinding {
    Keyboard { key: KeyCode },
    Mouse { button: KeyCode, modifiers: Vec<KeyCode> },
    MouseMotion { axis: MouseAxis, sensitivity: f32 },
    GamepadButton { button: KeyCode },
    GamepadAxis { axis: GamepadAxis, deadzone: f32, invert: bool },
    Touch { gesture: TouchGesture },
}

#[derive(Debug, Clone)]
pub enum MouseAxis {
    X,
    Y,
    DeltaX,
    DeltaY,
}

#[derive(Debug, Clone)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,
    RightTrigger,
}

#[derive(Debug, Clone)]
pub enum TouchGesture {
    Tap,
    DoubleTap,
    SwipeLeft,
    SwipeRight,
    SwipeUp,
    SwipeDown,
    PinchIn,
    PinchOut,
}

// Girdi durumu
#[derive(Debug, Clone)]
pub struct InputState {
    pub keyboard_keys: HashMap<KeyCode, KeyState>,
    pub mouse_position: MousePosition,
    pub mouse_delta: MousePosition, // Önceki frame'e göre fark
    pub mouse_wheel: f32,
    pub gamepads: Vec<(GamepadAxes, GamepadButtons)>,
    pub touch_positions: Vec<MousePosition>,
    pub text_input: String,
    pub actions: HashMap<String, f32>,
    pub last_frame_actions: HashMap<String, f32>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keyboard_keys: HashMap::new(),
            mouse_position: MousePosition { x: 0.0, y: 0.0 },
            mouse_delta: MousePosition { x: 0.0, y: 0.0 },
            mouse_wheel: 0.0,
            gamepads: Vec::new(),
            touch_positions: Vec::new(),
            text_input: String::new(),
            actions: HashMap::new(),
            last_frame_actions: HashMap::new(),
        }
    }
    
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        matches!(self.keyboard_keys.get(&key), Some(KeyState::Pressed))
    }
    
    pub fn is_key_down(&self, key: KeyCode) -> bool {
        matches!(self.keyboard_keys.get(&key), Some(KeyState::Down) | Some(KeyState::Pressed))
    }
    
    pub fn is_key_released(&self, key: KeyCode) -> bool {
        matches!(self.keyboard_keys.get(&key), Some(KeyState::Released))
    }
    
    pub fn was_key_down_last_frame(&self, key: KeyCode) -> bool {
        matches!(self.keyboard_keys.get(&key), Some(KeyState::Down) | Some(KeyState::Pressed))
    }
    
    pub fn is_action_pressed(&self, action_name: &str) -> bool {
        matches!(self.actions.get(action_name), Some(value) if *value > 0.5)
    }
    
    pub fn is_action_down(&self, action_name: &str) -> bool {
        matches!(self.actions.get(action_name), Some(value) if *value > 0.0)
    }
    
    pub fn get_action_value(&self, action_name: &str) -> f32 {
        *self.actions.get(action_name).unwrap_or(&0.0)
    }
    
    pub fn get_axis_value(&self, action_name: &str) -> f32 {
        self.get_action_value(action_name)
    }
    
    pub fn is_mouse_button_pressed(&self, button: KeyCode) -> bool {
        self.is_key_pressed(button)
    }
    
    pub fn is_mouse_button_down(&self, button: KeyCode) -> bool {
        self.is_key_down(button)
    }
    
    pub fn get_mouse_position(&self) -> MousePosition {
        self.mouse_position
    }
    
    pub fn get_mouse_delta(&self) -> MousePosition {
        self.mouse_delta
    }
    
    pub fn get_mouse_wheel(&self) -> f32 {
        self.mouse_wheel
    }
    
    pub fn is_gamepad_connected(&self, index: usize) -> bool {
        index < self.gamepads.len()
    }
    
    pub fn get_gamepad_axes(&self, index: usize) -> Option<GamepadAxes> {
        self.gamepads.get(index).map(|(axes, _)| *axes)
    }
    
    pub fn get_gamepad_buttons(&self, index: usize) -> Option<GamepadButtons> {
        self.gamepads.get(index).map(|(_, buttons)| *buttons)
    }
}

// Girdi yöneticisi
pub struct InputManager {
    pub input_state: InputState,
    pub action_map: HashMap<String, InputAction>,
    pub text_input_enabled: bool,
    pub mouse_locked: bool,
    pub mouse_visible: bool,
    pub key_repeat_delay: Duration,
    pub key_repeat_rate: Duration,
    pub last_key_times: HashMap<KeyCode, std::time::Instant>,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            input_state: InputState::new(),
            action_map: HashMap::new(),
            text_input_enabled: false,
            mouse_locked: false,
            mouse_visible: true,
            key_repeat_delay: Duration::from_millis(500),
            key_repeat_rate: Duration::from_millis(30),
            last_key_times: HashMap::new(),
        }
    }
    
    pub fn register_action(&mut self, action: InputAction) {
        self.action_map.insert(action.name.clone(), action);
    }
    
    pub fn register_action_simple(&mut self, name: &str, binding: InputBinding) {
        let action = InputAction {
            name: name.to_string(),
            bindings: vec![binding],
            axis_deadzone: 0.1,
        };
        self.register_action(action);
    }
    
    pub fn process_keyboard_event(&mut self, key: KeyCode, state: KeyState) {
        // Tuş durumunu güncelle
        self.input_state.keyboard_keys.insert(key, state);
        
        // Tuş tekrarını yönet
        if state == KeyState::Pressed {
            self.last_key_times.insert(key, std::time::Instant::now());
        }
    }
    
    pub fn process_mouse_motion(&mut self, x: f32, y: f32) {
        let old_pos = self.input_state.mouse_position;
        self.input_state.mouse_position = MousePosition { x, y };
        self.input_state.mouse_delta = MousePosition {
            x: x - old_pos.x,
            y: y - old_pos.y,
        };
    }
    
    pub fn process_mouse_wheel(&mut self, delta: f32) {
        self.input_state.mouse_wheel = delta;
    }
    
    pub fn process_mouse_button(&mut self, button: KeyCode, state: KeyState) {
        self.process_keyboard_event(button, state);
    }
    
    pub fn process_gamepad_event(&mut self, gamepad_index: usize, axes: GamepadAxes, buttons: GamepadButtons) {
        // Oyun kontrolleri sayısını gerekirse artır
        while self.input_state.gamepads.len() <= gamepad_index {
            self.input_state.gamepads.push((GamepadAxes {
                left_stick_x: 0.0,
                left_stick_y: 0.0,
                right_stick_x: 0.0,
                right_stick_y: 0.0,
                left_trigger: 0.0,
                right_trigger: 0.0,
            }, GamepadButtons::default()));
        }
        
        self.input_state.gamepads[gamepad_index] = (axes, buttons);
    }
    
    pub fn process_text_input(&mut self, text: &str) {
        if self.text_input_enabled {
            self.input_state.text_input.push_str(text);
        }
    }
    
    pub fn clear_text_input(&mut self) {
        self.input_state.text_input.clear();
    }
    
    pub fn update(&mut self) {
        // Önceki frame eylemlerini sakla
        self.input_state.last_frame_actions = self.input_state.actions.clone();
        self.input_state.actions.clear();
        
        // Eylemleri bağlamalardan hesapla
        for (action_name, action) in &self.action_map {
            let mut value = 0.0f32;
            
            for binding in &action.bindings {
                match binding {
                    InputBinding::Keyboard { key } => {
                        if self.input_state.is_key_down(*key) {
                            value = 1.0;
                        }
                    }
                    InputBinding::Mouse { button, modifiers: _ } => {
                        if self.input_state.is_mouse_button_down(*button) {
                            value = 1.0;
                        }
                    }
                    InputBinding::MouseMotion { axis, sensitivity } => {
                        match axis {
                            MouseAxis::DeltaX => value = self.input_state.mouse_delta.x * sensitivity,
                            MouseAxis::DeltaY => value = self.input_state.mouse_delta.y * sensitivity,
                            MouseAxis::X => value = self.input_state.mouse_position.x * sensitivity,
                            MouseAxis::Y => value = self.input_state.mouse_position.y * sensitivity,
                        }
                    }
                    InputBinding::GamepadButton { button } => {
                        for (_, buttons) in &self.input_state.gamepads {
                            let button_state = match button {
                                KeyCode::GamepadA => buttons.a,
                                KeyCode::GamepadB => buttons.b,
                                KeyCode::GamepadX => buttons.x,
                                KeyCode::GamepadY => buttons.y,
                                KeyCode::GamepadUp => buttons.up,
                                KeyCode::GamepadDown => buttons.down,
                                KeyCode::GamepadLeft => buttons.left,
                                KeyCode::GamepadRight => buttons.right,
                                KeyCode::GamepadStart => buttons.start,
                                KeyCode::GamepadSelect => buttons.select,
                                KeyCode::GamepadLeftShoulder => buttons.left_shoulder,
                                KeyCode::GamepadRightShoulder => buttons.right_shoulder,
                                KeyCode::GamepadLeftTrigger => buttons.left_trigger,
                                KeyCode::GamepadRightTrigger => buttons.right_trigger,
                                KeyCode::GamepadLeftStick => buttons.left_stick,
                                KeyCode::GamepadRightStick => buttons.right_stick,
                                _ => KeyState::Up,
                            };
                            
                            if matches!(button_state, KeyState::Down | KeyState::Pressed) {
                                value = 1.0;
                                break;
                            }
                        }
                    }
                    InputBinding::GamepadAxis { axis, deadzone, invert } => {
                        for (axes, _) in &self.input_state.gamepads {
                            let axis_value = match axis {
                                GamepadAxis::LeftStickX => axes.left_stick_x,
                                GamepadAxis::LeftStickY => axes.left_stick_y,
                                GamepadAxis::RightStickX => axes.right_stick_x,
                                GamepadAxis::RightStickY => axes.right_stick_y,
                                GamepadAxis::LeftTrigger => axes.left_trigger,
                                GamepadAxis::RightTrigger => axes.right_trigger,
                            };
                            
                            let final_value = if *invert { -axis_value } else { axis_value };
                            
                            if final_value.abs() > *deadzone {
                                value = final_value;
                                break;
                            }
                        }
                    }
                    InputBinding::Touch { gesture: _ } => {
                        // Dokunmatik girdiler için placeholder
                        // Gerçek implementasyon platforma bağlıdır
                    }
                }
                
                // Eğer bir bağlamadan değer geldiyse diğerlerine bakma
                if value.abs() > f32::EPSILON {
                    break;
                }
            }
            
            // Eylem değerini ekle
            self.input_state.actions.insert(action_name.clone(), value);
        }
        
        // Tuş durumlarını güncelle (pressed/released -> down)
        let keys_to_remove: Vec<_> = self.input_state.keyboard_keys
            .iter()
            .filter(|(_, state)| matches!(state, KeyState::Released))
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in keys_to_remove {
            self.input_state.keyboard_keys.remove(&key);
        }
        
        for state in self.input_state.keyboard_keys.values_mut() {
            if matches!(state, KeyState::Pressed) {
                *state = KeyState::Down;
            }
        }
        
        // Fare tekerleğini sıfırla
        self.input_state.mouse_wheel = 0.0;
        
        // Fare delta değerini sıfırla
        self.input_state.mouse_delta = MousePosition { x: 0.0, y: 0.0 };
    }
    
    pub fn get_input_state(&self) -> &InputState {
        &self.input_state
    }
    
    pub fn get_input_state_mut(&mut self) -> &mut InputState {
        &mut self.input_state
    }
    
    pub fn set_mouse_lock(&mut self, locked: bool) {
        self.mouse_locked = locked;
    }
    
    pub fn set_mouse_visibility(&mut self, visible: bool) {
        self.mouse_visible = visible;
    }
    
    pub fn set_text_input_enabled(&mut self, enabled: bool) {
        self.text_input_enabled = enabled;
    }
    
    pub fn is_mouse_locked(&self) -> bool {
        self.mouse_locked
    }
    
    pub fn is_mouse_visible(&self) -> bool {
        self.mouse_visible
    }
    
    pub fn is_text_input_enabled(&self) -> bool {
        self.text_input_enabled
    }
}

// Girdi yardımcı fonksiyonları
pub mod input_utils {
    use super::*;
    
    pub fn create_movement_action(forward: KeyCode, backward: KeyCode, left: KeyCode, right: KeyCode) -> InputAction {
        InputAction {
            name: "move".to_string(),
            bindings: vec![
                InputBinding::Keyboard { key: forward },
                InputBinding::Keyboard { key: backward },
                InputBinding::Keyboard { key: left },
                InputBinding::Keyboard { key: right },
            ],
            axis_deadzone: 0.1,
        }
    }
    
    pub fn create_look_action(horizontal: MouseAxis, vertical: MouseAxis) -> InputAction {
        InputAction {
            name: "look".to_string(),
            bindings: vec![
                InputBinding::MouseMotion { axis: horizontal, sensitivity: 0.1 },
                InputBinding::MouseMotion { axis: vertical, sensitivity: 0.1 },
            ],
            axis_deadzone: 0.05,
        }
    }
    
    pub fn create_jump_action(jump_key: KeyCode) -> InputAction {
        InputAction {
            name: "jump".to_string(),
            bindings: vec![
                InputBinding::Keyboard { key: jump_key },
            ],
            axis_deadzone: 0.1,
        }
    }
    
    pub fn create_shoot_action(shoot_key: KeyCode) -> InputAction {
        InputAction {
            name: "shoot".to_string(),
            bindings: vec![
                InputBinding::Mouse { button: shoot_key, modifiers: vec![] },
            ],
            axis_deadzone: 0.1,
        }
    }
}

// Oyuncu girdi bileşeni
use crate::Component;

pub struct PlayerInput {
    pub movement_speed: f32,
    pub look_sensitivity: f32,
    pub jump_force: f32,
    pub controller_index: Option<usize>,
    pub input_mapping: HashMap<String, String>, // Eylem adı -> oynatıcı eylemi
}

impl PlayerInput {
    pub fn new() -> Self {
        Self {
            movement_speed: 5.0,
            look_sensitivity: 2.0,
            jump_force: 10.0,
            controller_index: None,
            input_mapping: HashMap::new(),
        }
    }
    
    pub fn map_action(&mut self, player_action: &str, input_action: &str) {
        self.input_mapping.insert(player_action.to_string(), input_action.to_string());
    }
    
    pub fn get_mapped_action_value(&self, input_manager: &InputManager, player_action: &str) -> f32 {
        if let Some(input_action) = self.input_mapping.get(player_action) {
            input_manager.get_input_state().get_action_value(input_action)
        } else {
            0.0
        }
    }
    
    pub fn is_mapped_action_pressed(&self, input_manager: &InputManager, player_action: &str) -> bool {
        if let Some(input_action) = self.input_mapping.get(player_action) {
            input_manager.get_input_state().is_action_pressed(input_action)
        } else {
            false
        }
    }
    
    pub fn is_mapped_action_down(&self, input_manager: &InputManager, player_action: &str) -> bool {
        if let Some(input_action) = self.input_mapping.get(player_action) {
            input_manager.get_input_state().is_action_down(input_action)
        } else {
            false
        }
    }
}

// Girdi sistem bileşeni
pub struct InputSystem {
    pub manager: InputManager,
}

impl InputSystem {
    pub fn new() -> Self {
        Self {
            manager: InputManager::new(),
        }
    }
    
    pub fn register_default_actions(&mut self) {
        // Hareket eylemleri
        self.manager.register_action(input_utils::create_movement_action(
            KeyCode::KeyW, KeyCode::KeyS, KeyCode::KeyA, KeyCode::KeyD
        ));
        
        // Bakış eylemi
        self.manager.register_action(input_utils::create_look_action(
            MouseAxis::DeltaX, MouseAxis::DeltaY
        ));
        
        // Zıplama eylemi
        self.manager.register_action(input_utils::create_jump_action(KeyCode::Space));
        
        // Ateş etme eylemi
        self.manager.register_action(input_utils::create_shoot_action(KeyCode::MouseButtonLeft));
    }
    
    pub fn update(&mut self) {
        self.manager.update();
    }
    
    pub fn get_input_manager(&self) -> &InputManager {
        &self.manager
    }
    
    pub fn get_input_manager_mut(&mut self) -> &mut InputManager {
        &mut self.manager
    }
}