use std::collections::HashMap;
use std::time::Duration;
use std::sync::{Arc, RwLock};

/// Tuş kodları
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Alfabetik tuşlar
    KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ,
    KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR, KeyS, KeyT,
    KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,

    // Sayısal tuşlar
    Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,

    // Fonksiyon tuşları
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,

    // Kontrol tuşları
    Escape, Enter, Tab, Backspace, Delete, Insert, Home, End, PageUp, PageDown,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,

    // Modifier tuşları
    ShiftLeft, ShiftRight, ControlLeft, ControlRight, AltLeft, AltRight, MetaLeft, MetaRight,

    // Klavye dışı
    Space, CapsLock, NumLock, ScrollLock,

    // Nümerik tuş takımı
    Numpad0, Numpad1, Numpad2, Numpad3, Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
    NumpadAdd, NumpadSubtract, NumpadMultiply, NumpadDivide, NumpadDecimal, NumpadEnter,

    // Diğer
    Semicolon, Equal, Comma, Minus, Period, Slash, Backquote,
    BracketLeft, BracketRight, Backslash, Quote,
}

/// Fare butonları
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

/// Olay türü
#[derive(Debug, Clone, PartialEq)]
pub enum InputEventType {
    KeyPressed(KeyCode),
    KeyReleased(KeyCode),
    KeyRepeated(KeyCode),
    MouseButtonPressed(MouseButton),
    MouseButtonReleased(MouseButton),
    MouseMoved { x: f32, y: f32 },
    MouseScrolled { x: f32, y: f32 },
    GamepadButtonPressed(u32, GamepadButton),
    GamepadButtonReleased(u32, GamepadButton),
    GamepadAxisChanged(u32, GamepadAxis, f32),
}

/// Olay
#[derive(Debug, Clone, PartialEq)]
pub struct InputEvent {
    pub event_type: InputEventType,
    pub timestamp: std::time::Instant,
    pub consumed: bool,
}

impl InputEvent {
    pub fn new(event_type: InputEventType) -> Self {
        Self {
            event_type,
            timestamp: std::time::Instant::now(),
            consumed: false,
        }
    }
}

/// Olay işleyici
pub type InputEventHandler = Box<dyn Fn(&InputEvent) -> Result<(), Box<dyn std::error::Error>> + Send + Sync>;

/// Tuş durumu
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyState {
    Released,
    Pressed,
    Repeated,
}

/// Tuş girdisi
#[derive(Debug, Clone)]
pub struct KeyInput {
    pub state: KeyState,
    pub press_time: Option<std::time::Instant>,
    pub repeat_count: u32,
}

/// Fare girdisi
#[derive(Debug, Clone)]
pub struct MouseInput {
    pub position: (f32, f32),
    pub buttons: HashMap<MouseButton, KeyInput>,
    pub wheel_delta: (f32, f32),
    pub moved: bool,
}

/// Oyun kolu butonları
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    South,      // A (Xbox) / X (PS)
    East,       // B (Xbox) / Circle (PS)
    West,       // X (Xbox) / Square (PS)
    North,      // Y (Xbox) / Triangle (PS)
    LeftShoulder,
    RightShoulder,
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    LeftStick,
    RightStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

/// Oyun kolu eksenleri
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,
    RightTrigger,
}

/// Oyun kolu girdisi
#[derive(Debug, Clone)]
pub struct GamepadInput {
    pub id: u32,
    pub buttons: HashMap<GamepadButton, KeyInput>,
    pub axes: HashMap<GamepadAxis, f32>,
    pub connected: bool,
    pub name: String,
}

/// Girdi eylemi
#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    Jump,
    Crouch,
    Sprint,
    Shoot,
    Reload,
    Interact,
    Menu,
    Pause,
    Custom(String),
}

/// Girdi eşlemesi
#[derive(Debug, Clone)]
pub struct InputBinding {
    pub action: InputAction,
    pub key_bindings: Vec<KeyCode>,
    pub mouse_bindings: Vec<MouseButton>,
    pub gamepad_bindings: Vec<(GamepadButton, u32)>, // (button, gamepad_id)
}

/// Girdi modu
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    KeyboardMouse,
    Gamepad,
    Touch,
    Mixed,
}

/// Girdi sistemi yapılandırması
#[derive(Debug, Clone)]
pub struct InputSystemConfig {
    pub enable_gamepad: bool,
    pub enable_touch: bool,
    pub key_repeat_delay: Duration,
    pub key_repeat_rate: Duration,
    pub mouse_sensitivity: f32,
    pub gamepad_deadzone: f32,
    pub invert_mouse_y: bool,
}

impl Default for InputSystemConfig {
    fn default() -> Self {
        Self {
            enable_gamepad: true,
            enable_touch: false,
            key_repeat_delay: Duration::from_millis(500),
            key_repeat_rate: Duration::from_millis(30),
            mouse_sensitivity: 1.0,
            gamepad_deadzone: 0.1,
            invert_mouse_y: false,
        }
    }
}

/// Girdi sistemi
pub struct InputSystem {
    pub keys: HashMap<KeyCode, KeyInput>,
    pub mouse: MouseInput,
    pub gamepads: HashMap<u32, GamepadInput>,
    pub bindings: Vec<InputBinding>,
    pub actions: HashMap<InputAction, bool>,
    pub events: Vec<InputEvent>,
    pub event_handlers: HashMap<String, Vec<InputEventHandler>>,
    pub config: InputSystemConfig,
    pub mode: InputMode,
    pub last_mouse_position: (f32, f32),
    pub text_input: String,
    pub clipboard: String,
    pub focused: bool,
    pub key_repeat_timers: HashMap<KeyCode, std::time::Instant>,
}

impl Default for InputSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl InputSystem {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            mouse: MouseInput {
                position: (0.0, 0.0),
                buttons: HashMap::new(),
                wheel_delta: (0.0, 0.0),
                moved: false,
            },
            gamepads: HashMap::new(),
            bindings: Vec::new(),
            actions: HashMap::new(),
            events: Vec::new(),
            event_handlers: HashMap::new(),
            config: InputSystemConfig::default(),
            mode: InputMode::KeyboardMouse,
            last_mouse_position: (0.0, 0.0),
            text_input: String::new(),
            clipboard: String::new(),
            focused: true,
            key_repeat_timers: HashMap::new(),
        }
    }

    /// Tuş durumu al
    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys.get(&key).map_or(false, |input| {
            matches!(input.state, KeyState::Pressed | KeyState::Repeated)
        })
    }

    /// Tuş basıldığında
    pub fn key_pressed(&mut self, key: KeyCode) {
        let now = std::time::Instant::now();
        
        let key_input = self.keys.entry(key).or_insert_with(|| KeyInput {
            state: KeyState::Released,
            press_time: None,
            repeat_count: 0,
        });

        key_input.state = KeyState::Pressed;
        key_input.press_time = Some(now);
        key_input.repeat_count = 0;

        // Olay oluştur
        let event = InputEvent::new(InputEventType::KeyPressed(key));
        self.events.push(event);

        // Tekrar zamanlayıcıyı başlat
        self.key_repeat_timers.insert(key, now);
    }

    /// Tuş bırakıldığında
    pub fn key_released(&mut self, key: KeyCode) {
        if let Some(key_input) = self.keys.get_mut(&key) {
            key_input.state = KeyState::Released;
            key_input.press_time = None;

            // Olay oluştur
            let event = InputEvent::new(InputEventType::KeyReleased(key));
            self.events.push(event);

            // Tekrar zamanlayıcıyı kaldır
            self.key_repeat_timers.remove(&key);
        }
    }

    /// Fare butonu basıldığında
    pub fn mouse_button_pressed(&mut self, button: MouseButton) {
        let now = std::time::Instant::now();
        
        let mouse_input = self.mouse.buttons.entry(button).or_insert_with(|| KeyInput {
            state: KeyState::Released,
            press_time: None,
            repeat_count: 0,
        });

        mouse_input.state = KeyState::Pressed;
        mouse_input.press_time = Some(now);

        let event = InputEvent::new(InputEventType::MouseButtonPressed(button));
        self.events.push(event);
    }

    /// Fare butonu bırakıldığında
    pub fn mouse_button_released(&mut self, button: MouseButton) {
        if let Some(mouse_input) = self.mouse.buttons.get_mut(&button) {
            mouse_input.state = KeyState::Released;
            mouse_input.press_time = None;

            let event = InputEvent::new(InputEventType::MouseButtonReleased(button));
            self.events.push(event);
        }
    }

    /// Fare hareketi
    pub fn mouse_moved(&mut self, x: f32, y: f32) {
        self.last_mouse_position = self.mouse.position;
        self.mouse.position = (x, y);
        self.mouse.moved = true;

        let event = InputEvent::new(InputEventType::MouseMoved { x, y });
        self.events.push(event);
    }

    /// Fare kaydırma
    pub fn mouse_scrolled(&mut self, x: f32, y: f32) {
        self.mouse.wheel_delta = (x, y);

        let event = InputEvent::new(InputEventType::MouseScrolled { x, y });
        self.events.push(event);
    }

    /// Oyun kolu butonu basıldığında
    pub fn gamepad_button_pressed(&mut self, gamepad_id: u32, button: GamepadButton) {
        let gamepad = self.gamepads.entry(gamepad_id).or_insert_with(|| GamepadInput {
            id: gamepad_id,
            buttons: HashMap::new(),
            axes: HashMap::new(),
            connected: true,
            name: format!("Gamepad {}", gamepad_id),
        });

        let button_input = gamepad.buttons.entry(button).or_insert_with(|| KeyInput {
            state: KeyState::Released,
            press_time: None,
            repeat_count: 0,
        });

        button_input.state = KeyState::Pressed;
        button_input.press_time = Some(std::time::Instant::now());

        let event = InputEvent::new(InputEventType::GamepadButtonPressed(gamepad_id, button));
        self.events.push(event);
    }

    /// Oyun kolu ekseni değiştiğinde
    pub fn gamepad_axis_changed(&mut self, gamepad_id: u32, axis: GamepadAxis, value: f32) {
        let gamepad = self.gamepads.entry(gamepad_id).or_insert_with(|| GamepadInput {
            id: gamepad_id,
            buttons: HashMap::new(),
            axes: HashMap::new(),
            connected: true,
            name: format!("Gamepad {}", gamepad_id),
        });

        gamepad.axes.insert(axis, value);

        let event = InputEvent::new(InputEventType::GamepadAxisChanged(gamepad_id, axis, value));
        self.events.push(event);
    }

    /// Girdi eylemini eşleştir
    pub fn bind_action(&mut self, action: InputAction, binding: InputBinding) {
        self.bindings.push(binding);
        self.actions.insert(action, false);
    }

    /// Eylem durumu al
    pub fn is_action_pressed(&self, action: &InputAction) -> bool {
        if let Some(&pressed) = self.actions.get(action) {
            pressed
        } else {
            false
        }
    }

    /// Eylem için tuş durumu al
    pub fn is_action_just_pressed(&self, action: &InputAction) -> bool {
        for binding in &self.bindings {
            if &binding.action == action {
                for key in &binding.key_bindings {
                    if self.is_key_down(*key) {
                        return true;
                    }
                }
                for button in &binding.mouse_bindings {
                    if self.mouse.buttons.get(button).map_or(false, |input| {
                        matches!(input.state, KeyState::Pressed)
                    }) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Metin girdisi ekle
    pub fn add_text_input(&mut self, text: &str) {
        self.text_input.push_str(text);
    }

    /// Metin girdisini temizle
    pub fn clear_text_input(&mut self) {
        self.text_input.clear();
    }

    /// Panoya kopyala
    pub fn copy_to_clipboard(&mut self, text: &str) {
        self.clipboard = text.to_string();
    }

    /// Panodan yapıştır
    pub fn paste_from_clipboard(&self) -> String {
        self.clipboard.clone()
    }

    /// Sistemi güncelle
    pub fn update(&mut self, delta_time: f32) {
        let now = std::time::Instant::now();

        // Tuş tekrarlarını işle
        for (key, &last_press_time) in self.key_repeat_timers.iter() {
            if now.duration_since(last_press_time) >= self.config.key_repeat_delay {
                // Tuş hala basılı mı kontrol et
                if self.is_key_down(*key) {
                    if let Some(key_input) = self.keys.get_mut(key) {
                        key_input.state = KeyState::Repeated;
                        key_input.repeat_count += 1;

                        let event = InputEvent::new(InputEventType::KeyRepeated(*key));
                        self.events.push(event);
                    }
                }
            }
        }

        // Girdi eylemlerini güncelle
        for binding in &self.bindings {
            let mut action_pressed = false;

            // Tuş kontrolleri
            for key in &binding.key_bindings {
                if self.is_key_down(*key) {
                    action_pressed = true;
                    break;
                }
            }

            // Fare kontrolleri
            if !action_pressed {
                for button in &binding.mouse_bindings {
                    if self.mouse.buttons.get(button).map_or(false, |input| {
                        matches!(input.state, KeyState::Pressed)
                    }) {
                        action_pressed = true;
                        break;
                    }
                }
            }

            // Oyun kolu kontrolleri
            if !action_pressed {
                for (button, gamepad_id) in &binding.gamepad_bindings {
                    if let Some(gamepad) = self.gamepads.get(gamepad_id) {
                        if gamepad.buttons.get(button).map_or(false, |input| {
                            matches!(input.state, KeyState::Pressed)
                        }) {
                            action_pressed = true;
                            break;
                        }
                    }
                }
            }

            self.actions.insert(binding.action.clone(), action_pressed);
        }

        // Fare hareketini sıfırla
        self.mouse.moved = false;
    }

    /// Olay işleyici ekle
    pub fn add_event_handler(&mut self, name: String, handler: InputEventHandler) {
        self.event_handlers.entry(name).or_default().push(handler);
    }

    /// Olayları işleyiciye gönder
    pub fn process_events(&mut self) {
        for event in &mut self.events {
            for handlers in self.event_handlers.values() {
                for handler in handlers {
                    if let Err(e) = handler(event) {
                        eprintln!("Input event handler error: {}", e);
                    }
                }
            }
        }

        // İşlenmiş olayları temizle
        self.events.clear();
    }

    /// Girdi modunu değiştir
    pub fn set_input_mode(&mut self, mode: InputMode) {
        self.mode = mode;
    }

    /// Fare konumunu al
    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse.position
    }

    /// Fare hareket farkını al
    pub fn mouse_delta(&self) -> (f32, f32) {
        (
            self.mouse.position.0 - self.last_mouse_position.0,
            self.mouse.position.1 - self.last_mouse_position.1,
        )
    }

    /// Fare tekerleği konumunu al
    pub fn mouse_wheel(&self) -> (f32, f32) {
        self.mouse.wheel_delta
    }

    /// Oyun kolu bağla
    pub fn connect_gamepad(&mut self, id: u32, name: String) {
        self.gamepads.insert(id, GamepadInput {
            id,
            buttons: HashMap::new(),
            axes: HashMap::new(),
            connected: true,
            name,
        });
    }

    /// Oyun kolu çıkar
    pub fn disconnect_gamepad(&mut self, id: u32) -> bool {
        self.gamepads.remove(&id).is_some()
    }

    /// Oyun kolu bağlı mı kontrol et
    pub fn is_gamepad_connected(&self, id: u32) -> bool {
        self.gamepads.get(&id).map_or(false, |gp| gp.connected)
    }

    /// Oyun kolu ekseni değerini al
    pub fn get_gamepad_axis(&self, gamepad_id: u32, axis: GamepadAxis) -> f32 {
        self.gamepads.get(&gamepad_id)
            .and_then(|gp| gp.axes.get(&axis))
            .copied()
            .unwrap_or(0.0)
    }

    /// Oyun kolu butonu basılı mı
    pub fn is_gamepad_button_down(&self, gamepad_id: u32, button: GamepadButton) -> bool {
        self.gamepads.get(&gamepad_id)
            .and_then(|gp| gp.buttons.get(&button))
            .map_or(false, |input| {
                matches!(input.state, KeyState::Pressed | KeyState::Repeated)
            })
    }
}

/// Girdi yardımcı fonksiyonları
pub mod helpers {
    use super::*;

    /// Tuş ismini al
    pub fn key_name(key: KeyCode) -> String {
        match key {
            KeyCode::KeyA => "A".to_string(),
            KeyCode::KeyB => "B".to_string(),
            KeyCode::KeyC => "C".to_string(),
            KeyCode::KeyD => "D".to_string(),
            KeyCode::KeyE => "E".to_string(),
            KeyCode::KeyF => "F".to_string(),
            KeyCode::KeyG => "G".to_string(),
            KeyCode::KeyH => "H".to_string(),
            KeyCode::KeyI => "I".to_string(),
            KeyCode::KeyJ => "J".to_string(),
            KeyCode::KeyK => "K".to_string(),
            KeyCode::KeyL => "L".to_string(),
            KeyCode::KeyM => "M".to_string(),
            KeyCode::KeyN => "N".to_string(),
            KeyCode::KeyO => "O".to_string(),
            KeyCode::KeyP => "P".to_string(),
            KeyCode::KeyQ => "Q".to_string(),
            KeyCode::KeyR => "R".to_string(),
            KeyCode::KeyS => "S".to_string(),
            KeyCode::KeyT => "T".to_string(),
            KeyCode::KeyU => "U".to_string(),
            KeyCode::KeyV => "V".to_string(),
            KeyCode::KeyW => "W".to_string(),
            KeyCode::KeyX => "X".to_string(),
            KeyCode::KeyY => "Y".to_string(),
            KeyCode::KeyZ => "Z".to_string(),
            KeyCode::Digit0 => "0".to_string(),
            KeyCode::Digit1 => "1".to_string(),
            KeyCode::Digit2 => "2".to_string(),
            KeyCode::Digit3 => "3".to_string(),
            KeyCode::Digit4 => "4".to_string(),
            KeyCode::Digit5 => "5".to_string(),
            KeyCode::Digit6 => "6".to_string(),
            KeyCode::Digit7 => "7".to_string(),
            KeyCode::Digit8 => "8".to_string(),
            KeyCode::Digit9 => "9".to_string(),
            _ => format!("{:?}", key),
        }
    }

    /// Fare buton ismini al
    pub fn mouse_button_name(button: MouseButton) -> String {
        match button {
            MouseButton::Left => "Left".to_string(),
            MouseButton::Right => "Right".to_string(),
            MouseButton::Middle => "Middle".to_string(),
            MouseButton::Back => "Back".to_string(),
            MouseButton::Forward => "Forward".to_string(),
        }
    }

    /// Oyun kolu buton ismini al
    pub fn gamepad_button_name(button: GamepadButton) -> String {
        match button {
            GamepadButton::South => "South".to_string(),
            GamepadButton::East => "East".to_string(),
            GamepadButton::West => "West".to_string(),
            GamepadButton::North => "North".to_string(),
            _ => format!("{:?}", button),
        }
    }
}

/// Girdi sistemleri için önceden tanımlanmış ayarlar
pub mod presets {
    use super::*;

    /// Oyun kontrolleri
    pub fn game_controls() -> Vec<InputBinding> {
        vec![
            InputBinding {
                action: InputAction::MoveForward,
                key_bindings: vec![KeyCode::KeyW, KeyCode::ArrowUp],
                mouse_bindings: vec![],
                gamepad_bindings: vec![(GamepadButton::DPadUp, 0)],
            },
            InputBinding {
                action: InputAction::MoveBackward,
                key_bindings: vec![KeyCode::KeyS, KeyCode::ArrowDown],
                mouse_bindings: vec![],
                gamepad_bindings: vec![(GamepadButton::DPadDown, 0)],
            },
            InputBinding {
                action: InputAction::MoveLeft,
                key_bindings: vec![KeyCode::KeyA, KeyCode::ArrowLeft],
                mouse_bindings: vec![],
                gamepad_bindings: vec![(GamepadButton::DPadLeft, 0)],
            },
            InputBinding {
                action: InputAction::MoveRight,
                key_bindings: vec![KeyCode::KeyD, KeyCode::ArrowRight],
                mouse_bindings: vec![],
                gamepad_bindings: vec![(GamepadButton::DPadRight, 0)],
            },
            InputBinding {
                action: InputAction::Jump,
                key_bindings: vec![KeyCode::Space],
                mouse_bindings: vec![],
                gamepad_bindings: vec![(GamepadButton::South, 0)],
            },
            InputBinding {
                action: InputAction::Shoot,
                key_bindings: vec![KeyCode::MouseLeft],
                mouse_bindings: vec![MouseButton::Left],
                gamepad_bindings: vec![(GamepadButton::RightTrigger, 0)],
            },
        ]
    }

    /// Menü kontrolleri
    pub fn menu_controls() -> Vec<InputBinding> {
        vec![
            InputBinding {
                action: InputAction::Menu,
                key_bindings: vec![KeyCode::Escape],
                mouse_bindings: vec![],
                gamepad_bindings: vec![(GamepadButton::Start, 0)],
            },
            InputBinding {
                action: InputAction::Pause,
                key_bindings: vec![KeyCode::Escape],
                mouse_bindings: vec![],
                gamepad_bindings: vec![(GamepadButton::Start, 0)],
            },
        ]
    }
}

/// Girdi sistemini başlatan yardımcı fonksiyon
pub fn initialize_input_system() -> InputSystem {
    let mut system = InputSystem::new();
    
    // Varsayılan kontrolleri ekle
    for binding in presets::game_controls() {
        system.bind_action(binding.action.clone(), binding);
    }
    
    for binding in presets::menu_controls() {
        system.bind_action(binding.action.clone(), binding);
    }
    
    system
}