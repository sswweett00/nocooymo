use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use crate::input::{
    GamepadAxes, GamepadAxis as LegacyGamepadAxis, GamepadButtons, InputAction, InputBinding,
    InputManager, InputState, KeyCode, KeyState, MouseAxis, MousePosition, TouchGesture,
};

// ---------------------------------------------------------------------------
// Binding priority: lower value = higher priority
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BindingSource {
    Gamepad = 0,
    Keyboard = 1,
    Mouse = 2,
    Touch = 3,
}

impl BindingSource {
    pub fn from_binding(binding: &InputBinding) -> Self {
        match binding {
            InputBinding::Keyboard { .. } => Self::Keyboard,
            InputBinding::Mouse { .. } | InputBinding::MouseMotion { .. } => Self::Mouse,
            InputBinding::GamepadButton { .. } | InputBinding::GamepadAxis { .. } => Self::Gamepad,
            InputBinding::Touch { .. } => Self::Touch,
        }
    }
}

// ---------------------------------------------------------------------------
// Rebinding result
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum RebindResult {
    Success { action_name: String, old_binding: Option<InputBinding>, new_binding: InputBinding },
    Conflict { existing_action: String, existing_binding: InputBinding, new_binding: InputBinding },
    InvalidBinding(String),
    ActionNotFound(String),
}

// ---------------------------------------------------------------------------
// Validation error
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum ValidationError {
    UnsupportedKey(KeyCode),
    ReservedBinding(KeyCode),
    PlatformIncompatible(KeyCode),
}

// ---------------------------------------------------------------------------
// Input presets
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputPreset {
    Default,
    Accessibility,
    LeftHanded,
    OneHanded,
    PlatformDefault,
}

impl InputPreset {
    pub fn all() -> &'static [Self] {
        &[Self::Default, Self::Accessibility, Self::LeftHanded, Self::OneHanded, Self::PlatformDefault]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Accessibility => "Accessibility",
            Self::LeftHanded => "Left-Handed",
            Self::OneHanded => "One-Handed",
            Self::PlatformDefault => "Platform Default",
        }
    }
}

// ---------------------------------------------------------------------------
// Rumble / vibration definition
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
pub struct RumbleEffect {
    pub weak_magnitude: f32,
    pub strong_magnitude: f32,
    pub duration: Duration,
}

impl RumbleEffect {
    pub fn new(weak: f32, strong: f32, duration_ms: u64) -> Self {
        Self {
            weak_magnitude: weak.clamp(0.0, 1.0),
            strong_magnitude: strong.clamp(0.0, 1.0),
            duration: Duration::from_millis(duration_ms),
        }
    }

    pub fn light_pulse() -> Self { Self::new(0.1, 0.1, 100) }
    pub fn heavy_pulse() -> Self { Self::new(0.6, 0.8, 200) }
    pub fn none() -> Self { Self::new(0.0, 0.0, 0) }
}

// ---------------------------------------------------------------------------
// Mouse modulation settings
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
pub struct MouseSettings {
    pub sensitivity: f32,
    pub acceleration: f32,
    pub smoothing_factor: f32,
    pub invert_y: bool,
    pub raw_input: bool,
}

impl Default for MouseSettings {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
            acceleration: 0.0,
            smoothing_factor: 0.0,
            invert_y: false,
            raw_input: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Gamepad modulation settings
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
pub struct GamepadSettings {
    pub left_deadzone: f32,
    pub right_deadzone: f32,
    pub trigger_deadzone: f32,
    pub vibration_enabled: bool,
    pub rumble_decay: f32,
}

impl Default for GamepadSettings {
    fn default() -> Self {
        Self {
            left_deadzone: 0.1,
            right_deadzone: 0.1,
            trigger_deadzone: 0.05,
            vibration_enabled: true,
            rumble_decay: 0.9,
        }
    }
}

// ---------------------------------------------------------------------------
// Accessibility settings
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
pub enum ButtonBehavior {
    Hold,
    Toggle,
}

#[derive(Debug, Clone, Copy)]
pub struct AccessibilitySettings {
    pub button_behavior: ButtonBehavior,
    pub auto_repeat_enabled: bool,
    pub auto_repeat_delay: Duration,
    pub auto_repeat_rate: Duration,
    pub one_handed_mode: bool,
    pub one_handed_layout: OneHandedLayout,
    pub sticky_keys: bool,
    pub slow_keys: bool,
    pub slow_keys_delay: Duration,
    pub bounce_keys: bool,
    pub bounce_keys_delay: Duration,
}

impl Default for AccessibilitySettings {
    fn default() -> Self {
        Self {
            button_behavior: ButtonBehavior::Hold,
            auto_repeat_enabled: false,
            auto_repeat_delay: Duration::from_millis(500),
            auto_repeat_rate: Duration::from_millis(30),
            one_handed_mode: false,
            one_handed_layout: OneHandedLayout::Left,
            sticky_keys: false,
            slow_keys: false,
            slow_keys_delay: Duration::from_millis(500),
            bounce_keys: false,
            bounce_keys_delay: Duration::from_millis(300),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OneHandedLayout {
    Left,
    Right,
}

// ---------------------------------------------------------------------------
// Controller type detection
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControllerType {
    XBox,
    PlayStation,
    NintendoSwitch,
    Generic,
    Unknown,
}

// ---------------------------------------------------------------------------
// Platform input profiles
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
    iOS,
    Android,
    Unknown,
}

impl Platform {
    pub fn detect() -> Self {
        #[cfg(target_os = "windows")]
        return Self::Windows;
        #[cfg(target_os = "macos")]
        return Self::MacOS;
        #[cfg(target_os = "linux")]
        return Self::Linux;
        #[cfg(target_os = "ios")]
        return Self::iOS;
        #[cfg(target_os = "android")]
        return Self::Android;
        #[cfg(not(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "linux",
            target_os = "ios",
            target_os = "android"
        )))]
        Self::Unknown
    }
}

// ---------------------------------------------------------------------------
// Smoothed input value (exponential moving average)
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
pub struct SmoothedValue {
    pub current: f32,
    pub target: f32,
    pub smoothing: f32,
}

impl SmoothedValue {
    pub fn new(initial: f32, smoothing: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            smoothing: smoothing.clamp(0.0, 1.0),
        }
    }

    pub fn update(&mut self, new_target: f32) {
        self.target = new_target;
        self.current = self.current + (self.target - self.current) * self.smoothing;
    }

    pub fn is_settled(&self) -> bool {
        (self.target - self.current).abs() < 0.001
    }
}

impl Default for SmoothedValue {
    fn default() -> Self { Self::new(0.0, 0.0) }
}

// ---------------------------------------------------------------------------
// Extended rebinding manager
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct InputRebindManager {
    action_map: HashMap<String, InputAction>,
    bindings_by_key: HashMap<InputBinding, String>,
    bindings_by_source: HashMap<BindingSource, HashMap<InputBinding, String>>,
    presets: HashMap<InputPreset, HashMap<String, Vec<InputBinding>>>,
    active_preset: InputPreset,
    mouse_settings: MouseSettings,
    gamepad_settings: GamepadSettings,
    accessibility_settings: AccessibilitySettings,
    smoothed_values: HashMap<String, SmoothedValue>,
    pending_rebind: Option<(String, RebindTarget)>,
    rebind_listeners: Vec<Box<dyn Fn(&RebindResult)>>,
    rumble_active: HashMap<usize, (RumbleEffect, Instant)>,
    last_key_times: HashMap<KeyCode, (Instant, KeyState)>,
    key_repeat_state: HashMap<KeyCode, bool>,
    toggle_states: HashMap<KeyCode, bool>,
    key_repeat_delay: Duration,
    key_repeat_rate: Duration,
    one_handed_remap: HashMap<KeyCode, KeyCode>,
    platform: Platform,
    input_state: InputState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RebindTarget {
    Keyboard(KeyCode),
    Mouse(KeyCode),
    GamepadButton(KeyCode),
    GamepadAxis(LegacyGamepadAxis),
    Any,
}

impl RebindTarget {
    pub fn matches_binding(&self, binding: &InputBinding) -> bool {
        match (self, binding) {
            (Self::Keyboard(k), InputBinding::Keyboard { key }) => k == key,
            (Self::Mouse(m), InputBinding::Mouse { button, .. }) => m == button,
            (Self::GamepadButton(g), InputBinding::GamepadButton { button }) => g == button,
            (Self::GamepadAxis(a), InputBinding::GamepadAxis { axis, .. }) => {
                *a == LegacyGamepadAxis::from(*axis)
            }
            (Self::Any, _) => true,
            _ => false,
        }
    }

    pub fn source(&self) -> BindingSource {
        match self {
            Self::Keyboard(_) => BindingSource::Keyboard,
            Self::Mouse(_) => BindingSource::Mouse,
            Self::GamepadButton(_) | Self::GamepadAxis(_) => BindingSource::Gamepad,
            Self::Any => BindingSource::Keyboard,
        }
    }
}

// Convert legacy GamepadAxis enum back-and-forth
impl From<GamepadAxis> for LegacyGamepadAxis {
    fn from(value: GamepadAxis) -> Self {
        match value {
            GamepadAxis::LeftStickX => LegacyGamepadAxis::LeftStickX,
            GamepadAxis::LeftStickY => LegacyGamepadAxis::LeftStickY,
            GamepadAxis::RightStickX => LegacyGamepadAxis::RightStickX,
            GamepadAxis::RightStickY => LegacyGamepadAxis::RightStickY,
            GamepadAxis::LeftTrigger => LegacyGamepadAxis::LeftTrigger,
            GamepadAxis::RightTrigger => LegacyGamepadAxis::RightTrigger,
        }
    }
}

impl From<LegacyGamepadAxis> for GamepadAxis {
    fn from(value: LegacyGamepadAxis) -> Self {
        match value {
            LegacyGamepadAxis::LeftStickX => GamepadAxis::LeftStickX,
            LegacyGamepadAxis::LeftStickY => GamepadAxis::LeftStickY,
            LegacyGamepadAxis::RightStickX => GamepadAxis::RightStickX,
            LegacyGamepadAxis::RightStickY => GamepadAxis::RightStickY,
            LegacyGamepadAxis::LeftTrigger => GamepadAxis::LeftTrigger,
            LegacyGamepadAxis::RightTrigger => GamepadAxis::RightTrigger,
        }
    }
}

impl InputRebindManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            action_map: HashMap::new(),
            bindings_by_key: HashMap::new(),
            bindings_by_source: HashMap::new(),
            presets: HashMap::new(),
            active_preset: InputPreset::Default,
            mouse_settings: MouseSettings::default(),
            gamepad_settings: GamepadSettings::default(),
            accessibility_settings: AccessibilitySettings::default(),
            smoothed_values: HashMap::new(),
            pending_rebind: None,
            rebind_listeners: Vec::new(),
            rumble_active: HashMap::new(),
            last_key_times: HashMap::new(),
            key_repeat_state: HashMap::new(),
            toggle_states: HashMap::new(),
            key_repeat_delay: Duration::from_millis(500),
            key_repeat_rate: Duration::from_millis(30),
            one_handed_remap: HashMap::new(),
            platform: Platform::detect(),
            input_state: InputState::new(),
        };

        for src in [BindingSource::Keyboard, BindingSource::Mouse, BindingSource::Gamepad, BindingSource::Touch] {
            mgr.bindings_by_source.insert(src, HashMap::new());
        }

        mgr.build_default_presets();
        mgr
    }

    fn build_default_presets(&mut self) {
        let mut default: HashMap<String, Vec<InputBinding>> = HashMap::new();
        default.insert("jump".to_string(), vec![
            InputBinding::Keyboard { key: KeyCode::Space },
            InputBinding::GamepadButton { button: KeyCode::GamepadA },
        ]);
        default.insert("shoot".to_string(), vec![
            InputBinding::Mouse { button: KeyCode::MouseButtonLeft, modifiers: vec![] },
            InputBinding::GamepadButton { button: KeyCode::GamepadRightTrigger },
        ]);
        default.insert("menu".to_string(), vec![
            InputBinding::Keyboard { key: KeyCode::Escape },
            InputBinding::GamepadButton { button: KeyCode::GamepadStart },
        ]);
        default.insert("move_forward".to_string(), vec![
            InputBinding::Keyboard { key: KeyCode::KeyW },
            InputBinding::GamepadAxis { axis: GamepadAxis::LeftStickY, deadzone: 0.1, invert: false },
        ]);
        default.insert("move_right".to_string(), vec![
            InputBinding::Keyboard { key: KeyCode::KeyD },
            InputBinding::GamepadAxis { axis: GamepadAxis::LeftStickX, deadzone: 0.1, invert: false },
        ]);
        self.presets.insert(InputPreset::Default, default);

        let mut accessibility: HashMap<String, Vec<InputBinding>> = HashMap::new();
        accessibility.insert("jump".to_string(), vec![
            InputBinding::Keyboard { key: KeyCode::Space },
            InputBinding::Keyboard { key: KeyCode::KeyJ },
            InputBinding::GamepadButton { button: KeyCode::GamepadA },
        ]);
        accessibility.insert("shoot".to_string(), vec![
            InputBinding::Mouse { button: KeyCode::MouseButtonLeft, modifiers: vec![] },
            InputBinding::Keyboard { key: KeyCode::KeyF },
        ]);
        accessibility.insert("menu".to_string(), vec![
            InputBinding::Keyboard { key: KeyCode::Escape },
            InputBinding::Keyboard { key: KeyCode::KeyM },
        ]);
        self.presets.insert(InputPreset::Accessibility, accessibility);

        let mut left_handed: HashMap<String, Vec<InputBinding>> = HashMap::new();
        left_handed.insert("jump".to_string(), vec![
            InputBinding::Keyboard { key: KeyCode::KeyR },
            InputBinding::GamepadButton { button: KeyCode::GamepadA },
        ]);
        left_handed.insert("shoot".to_string(), vec![
            InputBinding::Mouse { button: KeyCode::MouseButtonRight, modifiers: vec![] },
            InputBinding::GamepadButton { button: KeyCode::GamepadRightTrigger },
        ]);
        left_handed.insert("menu".to_string(), vec![
            InputBinding::Keyboard { key: KeyCode::Escape },
            InputBinding::GamepadButton { button: KeyCode::GamepadStart },
        ]);
        self.presets.insert(InputPreset::LeftHanded, left_handed);

        let mut one_handed: HashMap<String, Vec<InputBinding>> = HashMap::new();
        one_handed.insert("jump".to_string(), vec![
            InputBinding::Keyboard { key: KeyCode::KeyJ },
            InputBinding::GamepadButton { button: KeyCode::GamepadA },
        ]);
        one_handed.insert("shoot".to_string(), vec![
            InputBinding::Keyboard { key: KeyCode::KeyK },
            InputBinding::GamepadButton { button: KeyCode::GamepadRightTrigger },
        ]);
        one_handed.insert("move_forward".to_string(), vec![
            InputBinding::Keyboard { key: KeyCode::KeyW },
            InputBinding::GamepadAxis { axis: GamepadAxis::LeftStickY, deadzone: 0.1, invert: false },
        ]);
        one_handed.insert("move_right".to_string(), vec![
            InputBinding::Keyboard { key: KeyCode::KeyD },
            InputBinding::GamepadAxis { axis: GamepadAxis::LeftStickX, deadzone: 0.1, invert: false },
        ]);
        self.presets.insert(InputPreset::OneHanded, one_handed);
    }

    // -----------------------------------------------------------------------
    // Action map operations
    // -----------------------------------------------------------------------
    pub fn register_action(&mut self, action: InputAction) {
        for binding in &action.bindings {
            self.bindings_by_key.insert(binding.clone(), action.name.clone());
            if let Some(entry) = self.bindings_by_source.get_mut(&BindingSource::from_binding(binding)) {
                entry.insert(binding.clone(), action.name.clone());
            }
        }
        self.action_map.insert(action.name.clone(), action);
    }

    pub fn unregister_action(&mut self, name: &str) {
        if let Some(action) = self.action_map.remove(name) {
            for binding in &action.bindings {
                self.bindings_by_key.remove(binding);
                if let Some(entry) = self.bindings_by_source.get_mut(&BindingSource::from_binding(binding)) {
                    entry.remove(binding);
                }
            }
        }
    }

    pub fn get_action(&self, name: &str) -> Option<&InputAction> {
        self.action_map.get(name)
    }

    pub fn get_action_mut(&mut self, name: &str) -> Option<&mut InputAction> {
        self.action_map.get_mut(name)
    }

    pub fn actions(&self) -> impl Iterator<Item = (&String, &InputAction)> {
        self.action_map.iter()
    }

    // -----------------------------------------------------------------------
    // Rebinding
    // -----------------------------------------------------------------------
    pub fn start_rebind(&mut self, action_name: &str, target: RebindTarget) -> RebindResult {
        if !self.action_map.contains_key(action_name) {
            return RebindResult::ActionNotFound(action_name.to_string());
        }
        self.pending_rebind = Some((action_name.to_string(), target));
        RebindResult::Success {
            action_name: action_name.to_string(),
            old_binding: None,
            new_binding: InputBinding::Keyboard { key: KeyCode::Space },
        }
    }

    pub fn commit_rebind(&mut self, action_name: &str, new_binding: InputBinding) -> RebindResult {
        if !self.action_map.contains_key(action_name) {
            return RebindResult::ActionNotFound(action_name.to_string());
        }

        if let Err(err) = self.validate_binding(&new_binding) {
            return RebindResult::InvalidBinding(format!("{:?}", err));
        }

        if let Some(existing_action) = self.bindings_by_key.get(&new_binding) {
            if existing_action != action_name {
                return RebindResult::Conflict {
                    existing_action: existing_action.clone(),
                    existing_binding: new_binding.clone(),
                    new_binding,
                };
            }
        }

        let mut old_binding: Option<InputBinding> = None;
        if let Some(action) = self.action_map.get_mut(action_name) {
            for (i, b) in action.bindings.iter().enumerate() {
                if BindingSource::from_binding(b) == BindingSource::from_binding(&new_binding)
                    && self.pending_rebind.map(|t| t.1.source()) == Some(BindingSource::from_binding(b))
                {
                    old_binding = Some(action.bindings[i].clone());
                    break;
                }
            }

            if let Some(ref old) = old_binding {
                self.bindings_by_key.remove(old);
                if let Some(entry) = self.bindings_by_source.get_mut(&BindingSource::from_binding(old)) {
                    entry.remove(old);
                }
            }

            if let Some(idx) = old_binding
                .as_ref()
                .and_then(|old| action.bindings.iter().position(|b| b == old))
            {
                action.bindings[idx] = new_binding.clone();
            } else {
                action.bindings.push(new_binding.clone());
            }
        }

        self.bindings_by_key.insert(new_binding.clone(), action_name.to_string());
        if let Some(entry) = self.bindings_by_source.get_mut(&BindingSource::from_binding(&new_binding)) {
            entry.insert(new_binding.clone(), action_name.to_string());
        }

        let result = RebindResult::Success {
            action_name: action_name.to_string(),
            old_binding,
            new_binding,
        };

        self.notify_listeners(&result);
        result
    }

    pub fn cancel_rebind(&mut self) {
        self.pending_rebind = None;
    }

    pub fn pending_rebind(&self) -> Option<(&str, RebindTarget)> {
        self.pending_rebind.as_ref().map(|(a, t)| (a.as_str(), *t))
    }

    pub fn on_rebind<F: Fn(&RebindResult) + 'static>(&mut self, listener: F) {
        self.rebind_listeners.push(Box::new(listener));
    }

    fn notify_listeners(&self, result: &RebindResult) {
        for listener in &self.rebind_listeners {
            listener(result);
        }
    }

    // -----------------------------------------------------------------------
    // Conflict detection
    // -----------------------------------------------------------------------
    pub fn detect_conflicts(&self) -> Vec<(InputBinding, String, String)> {
        let mut conflicts = Vec::new();
        let mut seen: HashMap<InputBinding, &str> = HashMap::new();

        for (action_name, action) in &self.action_map {
            for binding in &action.bindings {
                if let Some(existing) = seen.get(binding) {
                    if *existing != action_name.as_str() {
                        conflicts.push((binding.clone(), existing.to_string(), action_name.clone()));
                    }
                } else {
                    seen.insert(binding.clone(), action_name.as_str());
                }
            }
        }

        conflicts
    }

    pub fn conflicts_for_action(&self, action_name: &str) -> Vec<(InputBinding, String)> {
        let mut result = Vec::new();
        let Some(action) = self.action_map.get(action_name) else { return result };

        let mut other_bindings = HashSet::new();
        for (name, act) in &self.action_map {
            if name != action_name {
                for b in &act.bindings {
                    other_bindings.insert(b.clone());
                }
            }
        }

        for binding in &action.bindings {
            if other_bindings.contains(binding) {
                if let Some(other_action) = self.bindings_by_key.get(binding) {
                    result.push((binding.clone(), other_action.clone()));
                }
            }
        }

        result
    }

    pub fn resolve_conflicts(&mut self, keep_action: &str) -> Vec<RebindResult> {
        let mut results = Vec::new();
        let conflicts = self.detect_conflicts();

        for (binding, _existing, conflicting) in conflicts {
            if conflicting == keep_action {
                if let Some(action) = self.action_map.get_mut(&conflicting) {
                    if let Some(pos) = action.bindings.iter().position(|b| b == &binding) {
                        action.bindings.remove(pos);
                        results.push(RebindResult::Success {
                            action_name: conflicting.clone(),
                            old_binding: Some(binding.clone()),
                            new_binding: binding.clone(),
                        });
                    }
                }
            }
        }

        results
    }

    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------
    pub fn validate_binding(&self, binding: &InputBinding) -> Result<(), ValidationError> {
        match binding {
            InputBinding::Keyboard { key } => self.validate_key(*key),
            InputBinding::Mouse { button, .. } => self.validate_key(*button),
            InputBinding::MouseMotion { .. } => Ok(()),
            InputBinding::GamepadButton { button } => self.validate_key(*button),
            InputBinding::GamepadAxis { .. } => Ok(()),
            InputBinding::Touch { .. } => Ok(()),
        }
    }

    fn validate_key(&self, key: KeyCode) -> Result<(), ValidationError> {
        match key {
            KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
            | KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::CapsLock
            | KeyCode::NumLock
            | KeyCode::ScrollLock => Err(ValidationError::ReservedBinding(key)),
            KeyCode::PrintScreen | KeyCode::Pause => Err(ValidationError::ReservedBinding(key)),
            _ => Ok(()),
        }
    }

    // -----------------------------------------------------------------------
    // Preset management
    // -----------------------------------------------------------------------
    pub fn apply_preset(&mut self, preset: InputPreset) -> RebindResult {
        if let Some(preset_bindings) = self.presets.get(&preset).cloned() {
            self.action_map.clear();
            self.bindings_by_key.clear();
            for entry in self.bindings_by_source.values_mut() {
                entry.clear();
            }

            for (name, bindings) in preset_bindings {
                let action = InputAction {
                    name: name.clone(),
                    bindings: bindings.clone(),
                    axis_deadzone: self.gamepad_settings.left_deadzone,
                };
                self.register_action(action);
            }

            self.active_preset = preset;
            let result = RebindResult::Success {
                action_name: "system".to_string(),
                old_binding: None,
                new_binding: InputBinding::Keyboard { key: KeyCode::Space },
            };
            self.notify_listeners(&result);
            result
        } else {
            RebindResult::ActionNotFound(preset.name().to_string())
        }
    }

    pub fn save_preset(&mut self, preset: InputPreset) {
        let mut bindings: HashMap<String, Vec<InputBinding>> = HashMap::new();
        for (name, action) in &self.action_map {
            bindings.insert(name.clone(), action.bindings.clone());
        }
        self.presets.insert(preset, bindings);
    }

    pub fn current_preset(&self) -> InputPreset {
        self.active_preset
    }

    pub fn available_presets(&self) -> &'static [InputPreset] {
        InputPreset::all()
    }

    pub fn restore_defaults(&mut self) -> RebindResult {
        self.apply_preset(InputPreset::Default)
    }

    // -----------------------------------------------------------------------
    // Mouse modulation
    // -----------------------------------------------------------------------
    pub fn set_mouse_settings(&mut self, settings: MouseSettings) {
        self.mouse_settings = settings;
        self.smoothed_values.retain(|_, v| v.smoothing > 0.0);
    }

    pub fn mouse_settings(&self) -> MouseSettings {
        self.mouse_settings
    }

    pub fn modulate_mouse_motion(&mut self, raw_x: f32, raw_y: f32, dt: f32) -> MousePosition {
        let s = self.mouse_settings;
        let accel = if s.acceleration > 0.0 {
            let speed = (raw_x * raw_x + raw_y * raw_y).sqrt();
            if speed > 0.0 {
                1.0 + s.acceleration * (speed / 100.0).min(5.0)
            } else {
                1.0
            }
        } else {
            1.0
        };

        let mut out_x = raw_x * s.sensitivity * accel;
        let mut out_y = raw_y * s.sensitivity * accel;

        if s.invert_y {
            out_y = -out_y;
        }

        if s.smoothing_factor > 0.0 {
            let key = format!("mouse_x");
            let val = self.smoothed_values.entry(key).or_insert_with(|| SmoothedValue::new(out_x, s.smoothing_factor));
            val.smoothing = s.smoothing_factor;
            val.update(out_x);
            out_x = val.current;

            let key_y = format!("mouse_y");
            let val_y = self.smoothed_values.entry(key_y).or_insert_with(|| SmoothedValue::new(out_y, s.smoothing_factor));
            val_y.smoothing = s.smoothing_factor;
            val_y.update(out_y);
            out_y = val_y.current;
        }

        MousePosition { x: out_x, y: out_y }
    }

    // -----------------------------------------------------------------------
    // Gamepad modulation
    // -----------------------------------------------------------------------
    pub fn set_gamepad_settings(&mut self, settings: GamepadSettings) {
        self.gamepad_settings = settings;
    }

    pub fn gamepad_settings(&self) -> GamepadSettings {
        self.gamepad_settings
    }

    pub fn apply_gamepad_deadzone(&self, axis: LegacyGamepadAxis, raw: f32) -> f32 {
        let deadzone = match axis {
            LegacyGamepadAxis::LeftStickX | LegacyGamepadAxis::LeftStickY => self.gamepad_settings.left_deadzone,
            LegacyGamepadAxis::RightStickX | LegacyGamepadAxis::RightStickY => self.gamepad_settings.right_deadzone,
            LegacyGamepadAxis::LeftTrigger | LegacyGamepadAxis::RightTrigger => self.gamepad_settings.trigger_deadzone,
        };

        if raw.abs() < deadzone {
            0.0
        } else {
            let sign = raw.signum();
            sign * ((raw.abs() - deadzone) / (1.0 - deadzone)).clamp(0.0, 1.0)
        }
    }

    pub fn start_rumble(&mut self, gamepad_index: usize, effect: RumbleEffect) {
        if self.gamepad_settings.vibration_enabled && effect.duration > Duration::ZERO {
            self.rumble_active.insert(gamepad_index, (effect, Instant::now()));
        }
    }

    pub fn stop_rumble(&mut self, gamepad_index: usize) {
        self.rumble_active.remove(&gamepad_index);
    }

    pub fn rumble_active_for(&self, gamepad_index: usize) -> Option<RumbleEffect> {
        self.rumble_active.get(&gamepad_index).map(|(e, _)| *e)
    }

    pub fn update_rumble(&mut self) {
        let now = Instant::now();
        self.rumble_active.retain(|_, (effect, start)| {
            let elapsed = now.duration_since(*start);
            elapsed < effect.duration
        });
    }

    // -----------------------------------------------------------------------
    // Accessibility
    // -----------------------------------------------------------------------
    pub fn set_accessibility_settings(&mut self, settings: AccessibilitySettings) {
        self.accessibility_settings = settings;
        if settings.one_handed_mode {
            self.apply_one_handed_remap();
        }
    }

    pub fn accessibility_settings(&self) -> AccessibilitySettings {
        self.accessibility_settings
    }

    pub fn process_key_with_accessibility(&mut self, key: KeyCode, state: KeyState) -> KeyState {
        let settings = self.accessibility_settings;

        if settings.one_handed_mode {
            if let Some(remapped) = self.one_handed_remap.get(&key) {
                return self.process_key_behavior(*remapped, state, settings);
            }
        }

        self.process_key_behavior(key, state, settings)
    }

    fn process_key_behavior(&mut self, key: KeyCode, state: KeyState, settings: AccessibilitySettings) -> KeyState {
        match settings.button_behavior {
            ButtonBehavior::Toggle if state == KeyState::Pressed => {
                let current = self.toggle_states.entry(key).or_insert(false);
                *current = !*current;
                if *current { KeyState::Pressed } else { KeyState::Released }
            }
            ButtonBehavior::Hold => state,
            ButtonBehavior::Toggle => state,
        }
    }

    pub fn should_auto_repeat(&mut self, key: KeyCode, now: Instant) -> bool {
        if !self.accessibility_settings.auto_repeat_enabled {
            return false;
        }

        match self.last_key_times.get(&key) {
            Some((time, _)) => {
                let elapsed = now.duration_since(*time);
                if elapsed >= self.accessibility_settings.auto_repeat_delay {
                    let since_last = now
                        .duration_since(now - std::time::Duration::from_millis(0))
                        .as_secs_f32();
                    true
                } else {
                    false
                }
            }
            None => false,
        }
    }

    pub fn apply_one_handed_remap(&mut self) {
        self.one_handed_remap.clear();
        match self.accessibility_settings.one_handed_layout {
            OneHandedLayout::Left => {
                self.one_handed_remap.insert(KeyCode::KeyE, KeyCode::KeyQ);
                self.one_handed_remap.insert(KeyCode::KeyR, KeyCode::KeyF);
            }
            OneHandedLayout::Right => {
                self.one_handed_remap.insert(KeyCode::KeyQ, KeyCode::KeyE);
                self.one_handed_remap.insert(KeyCode::KeyU, KeyCode::KeyI);
            }
        }
    }

    pub fn is_one_handed_mode(&self) -> bool {
        self.accessibility_settings.one_handed_mode
    }

    // -----------------------------------------------------------------------
    // Input smoothing
    // -----------------------------------------------------------------------
    pub fn update_smoothed_values(&mut self, action_name: &str, raw_value: f32) -> f32 {
        let smoothing = self.mouse_settings.smoothing_factor;
        if smoothing <= 0.0 {
            return raw_value;
        }

        let val = self.smoothed_values
            .entry(action_name.to_string())
            .or_insert_with(|| SmoothedValue::new(raw_value, smoothing));
        val.smoothing = smoothing;
        val.update(raw_value);
        val.current
    }

    // -----------------------------------------------------------------------
    // Integration with existing InputManager
    // -----------------------------------------------------------------------
    pub fn integrate_with_input_manager(&mut self, manager: &mut InputManager) {
        for (name, action) in &self.action_map {
            if manager.action_map.get(name).is_none() {
                manager.action_map.insert(name.clone(), action.clone());
            }
        }
    }

    /// Update the legacy InputManager with modulated values.
    pub fn update_legacy_manager(&self, manager: &mut InputManager, dt: f32) {
        manager.update();

        if self.accessibility_settings.one_handed_mode {
            for (from, to) in &self.one_handed_remap {
                if let Some(state) = manager.input_state.keyboard_keys.get(from).copied() {
                    manager.input_state.keyboard_keys.insert(*to, state);
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Platform
    // -----------------------------------------------------------------------
    pub fn platform(&self) -> Platform {
        self.platform
    }

    pub fn controller_type(&self) -> ControllerType {
        if self.rebind_manager_input_state().gamepads.is_empty() {
            return ControllerType::Unknown;
        }

        ControllerType::Generic
    }

    pub fn rebind_manager_input_state(&self) -> &InputState {
        &self.input_state
    }

    pub fn rebind_manager_input_state_mut(&mut self) -> &mut InputState {
        &mut self.input_state
    }

    // -----------------------------------------------------------------------
    // Smoothed values introspection
    // -----------------------------------------------------------------------
    pub fn smoothed_values(&self) -> &HashMap<String, SmoothedValue> {
        &self.smoothed_values
    }

    pub fn reset_smoothed_values(&mut self) {
        self.smoothed_values.clear();
    }

    // -----------------------------------------------------------------------
    // Raw access for external systems
    // -----------------------------------------------------------------------
    pub fn get_action_value(&self, name: &str) -> f32 {
        self.action_map.get(name).map(|a| a.axis_deadzone).unwrap_or(0.0)
    }

    pub fn action_names(&self) -> Vec<String> {
        self.action_map.keys().cloned().collect()
    }

    pub fn is_action_bound(&self, name: &str) -> bool {
        self.action_map.get(name).map(|a| !a.bindings.is_empty()).unwrap_or(false)
    }

    pub fn bindings_for_action(&self, name: &str) -> Option<&Vec<InputBinding>> {
        self.action_map.get(name).map(|a| &a.bindings)
    }
}

// ---------------------------------------------------------------------------
// Builder helpers
// ---------------------------------------------------------------------------
pub struct InputRebindManagerBuilder {
    manager: InputRebindManager,
}

impl InputRebindManagerBuilder {
    pub fn new() -> Self {
        Self { manager: InputRebindManager::new() }
    }

    pub fn with_preset(mut self, preset: InputPreset) -> Self {
        self.manager.apply_preset(preset);
        self
    }

    pub fn with_mouse_sensitivity(mut self, sensitivity: f32) -> Self {
        self.manager.mouse_settings.sensitivity = sensitivity;
        self
    }

    pub fn with_deadzone(mut self, left: f32, right: f32, trigger: f32) -> Self {
        self.manager.gamepad_settings.left_deadzone = left;
        self.manager.gamepad_settings.right_deadzone = right;
        self.manager.gamepad_settings.trigger_deadzone = trigger;
        self
    }

    pub fn with_accessibility(mut self, settings: AccessibilitySettings) -> Self {
        self.manager.accessibility_settings = settings;
        self
    }

    pub fn with_one_handed(mut self, layout: OneHandedLayout) -> Self {
        self.manager.accessibility_settings.one_handed_mode = true;
        self.manager.accessibility_settings.one_handed_layout = layout;
        self.manager.apply_one_handed_remap();
        self
    }

    pub fn with_vibration(mut self, enabled: bool) -> Self {
        self.manager.gamepad_settings.vibration_enabled = enabled;
        self
    }

    pub fn build(self) -> InputRebindManager {
        self.manager
    }
}

impl Default for InputRebindManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Preset description for UI
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct PresetDescription {
    pub preset: InputPreset,
    pub name: &'static str,
    pub description: &'static str,
}

impl PresetDescription {
    pub fn all() -> Vec<Self> {
        vec![
            Self {
                preset: InputPreset::Default,
                name: "Default",
                description: "Standard controls for all players.",
            },
            Self {
                preset: InputPreset::Accessibility,
                name: "Accessibility",
                description: "Additional bindings and relaxed input requirements.",
            },
            Self {
                preset: InputPreset::LeftHanded,
                name: "Left-Handed",
                description: "Optimized for left-handed mouse usage.",
            },
            Self {
                preset: InputPreset::OneHanded,
                name: "One-Handed",
                description: "All essential actions on one side of the keyboard.",
            },
            Self {
                preset: InputPreset::PlatformDefault,
                name: "Platform Default",
                description: "Controls matched to the current operating system.",
            },
        ]
    }
}
