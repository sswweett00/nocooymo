//! Virtual Reality (VR) support for Elysium Engine.
//!
//! Provides abstractions for VR headset integration, hand tracking,
//! spatial audio, and VR-specific rendering features.
//!
//! Supported backends (via feature flags):
//! - `openxr` — OpenXR runtime (SteamVR, Oculus, etc.)
//! - `oculus` — Oculus PC SDK / Mobile SDK
//! - `steamvr` — SteamVR / Lighthouse tracking
//! - `openvr` — OpenVR (legacy)
//! - `mock` — Mock VR backend for testing/editor

use std::time::Duration;

/// Supported VR runtimes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VrRuntime {
    OpenXR,
    Oculus,
    SteamVR,
    OpenVR,
    Mock,
    Unknown,
}

/// VR headset display properties.
#[derive(Debug, Clone)]
pub struct VrDisplayProperties {
    pub resolution: (u32, u32),
    pub refresh_rate: f32,
    pub fov: (f32, f32, f32, f32), // left, right, up, down in radians
    pub ipd: f32, // inter-pupillary distance in meters
}

impl Default for VrDisplayProperties {
    fn default() -> Self {
        Self {
            resolution: (1832, 1920),
            refresh_rate: 90.0,
            fov: (1.230, 1.230, 1.105, 1.105),
            ipd: 0.063,
        }
    }
}

/// VR hand tracking data.
#[derive(Debug, Clone, Copy)]
pub struct VrHand {
    pub position: [f32; 3],
    pub orientation: [f32; 4], // quaternion
    pub is_tracked: bool,
    pub pinch_strength: f32,
    pub grip_strength: f32,
}

impl Default for VrHand {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            orientation: [0.0, 0.0, 0.0, 1.0],
            is_tracked: false,
            pinch_strength: 0.0,
            grip_strength: 0.0,
        }
    }
}

/// VR controller input state.
#[derive(Debug, Clone, Copy)]
pub struct VrController {
    pub position: [f32; 3],
    pub orientation: [f32; 4],
    pub is_tracked: bool,
    pub trigger: f32,
    pub grip: f32,
    pub thumbstick: [f32; 2],
    pub buttons: u32,
}

impl Default for VrController {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            orientation: [0.0, 0.0, 0.0, 1.0],
            is_tracked: false,
            trigger: 0.0,
            grip: 0.0,
            thumbstick: [0.0; 2],
            buttons: 0,
        }
    }
}

/// VR tracking space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VrTrackingSpace {
    Local,
    LocalFloor,
    Stage,
    Seated,
    Unbounded,
}

/// VR frame data from the headset.
#[derive(Debug, Clone)]
pub struct VrFrame {
    pub head_pose: [f32; 3],
    pub head_orientation: [f32; 4],
    pub left_hand: VrHand,
    pub right_hand: VrHand,
    pub left_controller: VrController,
    pub right_controller: VrController,
    pub eye_poses: [[[f32; 3]; 2]; 2], // left/right eye position and orientation
    pub frame_number: u64,
    pub predicted_display_time: Duration,
}

impl Default for VrFrame {
    fn default() -> Self {
        Self {
            head_pose: [0.0; 3],
            head_orientation: [0.0, 0.0, 0.0, 1.0],
            left_hand: VrHand::default(),
            right_hand: VrHand::default(),
            left_controller: VrController::default(),
            right_controller: VrController::default(),
            eye_poses: [[[0.0; 3]; 2]; 2],
            frame_number: 0,
            predicted_display_time: Duration::ZERO,
        }
    }
}

/// VR system configuration.
#[derive(Debug, Clone)]
pub struct VrConfig {
    pub enabled: bool,
    pub runtime: VrRuntime,
    pub tracking_space: VrTrackingSpace,
    pub render_scale: f32,
    pub enable_hand_tracking: bool,
    pub enable_face_tracking: bool,
    pub enable_eye_tracking: bool,
    pub enable_body_tracking: bool,
    pub motion_sickness_reduction: bool,
    pub snap_turn_angle: f32,
    pub smooth_turn_speed: f32,
    pub movement_speed: f32,
}

impl Default for VrConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            runtime: VrRuntime::Mock,
            tracking_space: VrTrackingSpace::LocalFloor,
            render_scale: 1.0,
            enable_hand_tracking: true,
            enable_face_tracking: false,
            enable_eye_tracking: false,
            enable_body_tracking: false,
            motion_sickness_reduction: true,
            snap_turn_angle: 30.0_f32.to_radians(),
            smooth_turn_speed: 2.0,
            movement_speed: 3.0,
        }
    }
}

/// VR system trait — implement for each supported runtime.
pub trait VrSystem: Send + Sync {
    /// Initialize the VR runtime.
    fn initialize(&mut self) -> Result<(), String>;

    /// Shutdown the VR runtime.
    fn shutdown(&mut self);

    /// Poll for new frame data.
    fn poll_frame(&mut self) -> Result<VrFrame, String>;

    /// Submit the rendered frame to the headset.
    fn submit_frame(&mut self, left_eye: &[u8], right_eye: &[u8]) -> Result<(), String>;

    /// Get display properties.
    fn display_properties(&self) -> VrDisplayProperties;

    /// Check if the headset is connected and tracking.
    fn is_connected(&self) -> bool;

    /// Recenter the tracking origin.
    fn recenter_tracking(&mut self) -> Result<(), String>;

    /// Trigger haptic feedback on a hand.
    fn trigger_haptic(&mut self, hand: VrHandedness, duration: Duration, frequency: f32, amplitude: f32);

    /// Check if hand tracking is supported.
    fn supports_hand_tracking(&self) -> bool;

    /// Check if eye tracking is supported.
    fn supports_eye_tracking(&self) -> bool;
}

/// VR handedness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VrHandedness {
    Left,
    Right,
    Any,
}

/// Mock VR system for testing and editor.
#[derive(Debug, Default)]
pub struct MockVrSystem {
    pub frame_count: u64,
    pub connected: bool,
}

impl MockVrSystem {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            connected: true,
        }
    }
}

impl VrSystem for MockVrSystem {
    fn initialize(&mut self) -> Result<(), String> {
        self.connected = true;
        Ok(())
    }

    fn shutdown(&mut self) {
        self.connected = false;
    }

    fn poll_frame(&mut self) -> Result<VrFrame, String> {
        self.frame_count += 1;
        let t = self.frame_count as f32 * 0.016;
        let mut frame = VrFrame::default();
        frame.head_pose = [0.0, 1.7, 0.0];
        frame.head_orientation = [0.0, 0.0, 0.0, 1.0];
        frame.left_hand.position = [0.3 + t.sin() * 0.1, 1.0, 0.3];
        frame.right_hand.position = [-0.3 - t.sin() * 0.1, 1.0, 0.3];
        frame.left_hand.is_tracked = true;
        frame.right_hand.is_tracked = true;
        frame.left_controller.is_tracked = true;
        frame.right_controller.is_tracked = true;
        frame.frame_number = self.frame_count;
        frame.predicted_display_time = Duration::from_millis(11);
        frame
    }

    fn submit_frame(&mut self, _left_eye: &[u8], _right_eye: &[u8]) -> Result<(), String> {
        Ok(())
    }

    fn display_properties(&self) -> VrDisplayProperties {
        VrDisplayProperties::default()
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn recenter_tracking(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn trigger_haptic(&mut self, _hand: VrHandedness, _duration: Duration, _frequency: f32, _amplitude: f32) {
        // No-op for mock
    }

    fn supports_hand_tracking(&self) -> bool {
        true
    }

    fn supports_eye_tracking(&self) -> bool {
        false
    }
}

/// VR manager — owns the active runtime and provides frame lifecycle.
#[derive(Debug)]
pub struct VrManager {
    config: VrConfig,
    system: Option<Box<dyn VrSystem>>,
    current_frame: VrFrame,
}

impl VrManager {
    /// Create a new VR manager with the given configuration.
    pub fn new(config: VrConfig) -> Self {
        Self {
            config,
            system: None,
            current_frame: VrFrame::default(),
        }
    }

    /// Initialize the VR system. Returns the runtime in use.
    pub fn initialize(&mut self) -> Result<VrRuntime, String> {
        if !self.config.enabled {
            return Ok(VrRuntime::Mock);
        }

        match self.config.runtime {
            VrRuntime::Mock => {
                let mut mock = MockVrSystem::new();
                mock.initialize()?;
                self.system = Some(Box::new(mock));
                Ok(VrRuntime::Mock)
            }
            _ => {
                // Real runtimes would be initialized here behind feature flags.
                // For now, fall back to mock.
                let mut mock = MockVrSystem::new();
                mock.initialize()?;
                self.system = Some(Box::new(mock));
                Ok(VrRuntime::Mock)
            }
        }
    }

    /// Shutdown the VR system.
    pub fn shutdown(&mut self) {
        if let Some(mut sys) = self.system.take() {
            sys.shutdown();
        }
    }

    /// Poll for the next VR frame.
    pub fn poll_frame(&mut self) -> Result<&VrFrame, String> {
        if let Some(sys) = &mut self.system {
            self.current_frame = sys.poll_frame()?;
        }
        Ok(&self.current_frame)
    }

    /// Submit rendered frame to the headset.
    pub fn submit_frame(&mut self, left: &[u8], right: &[u8]) -> Result<(), String> {
        if let Some(sys) = &mut self.system {
            sys.submit_frame(left, right)
        } else {
            Ok(())
        }
    }

    /// Get current VR frame data.
    pub fn current_frame(&self) -> &VrFrame {
        &self.current_frame
    }

    /// Get VR configuration.
    pub fn config(&self) -> &VrConfig {
        &self.config
    }

    /// Check if VR is active.
    pub fn is_active(&self) -> bool {
        self.config.enabled && self.system.is_some()
    }

    /// Trigger haptic feedback on a hand.
    pub fn trigger_haptic(&mut self, hand: VrHandedness, duration: Duration, frequency: f32, amplitude: f32) {
        if let Some(sys) = &mut self.system {
            sys.trigger_haptic(hand, duration, frequency, amplitude);
        }
    }

    /// Recenter tracking origin.
    pub fn recenter_tracking(&mut self) -> Result<(), String> {
        if let Some(sys) = &mut self.system {
            sys.recenter_tracking()
        } else {
            Ok(())
        }
    }

    /// Get display properties.
    pub fn display_properties(&self) -> VrDisplayProperties {
        self.system
            .as_ref()
            .map(|sys| sys.display_properties())
            .unwrap_or_default()
    }

    /// Check if the headset is connected.
    pub fn is_connected(&self) -> bool {
        self.system.as_ref().map(|sys| sys.is_connected()).unwrap_or(false)
    }
}

impl Drop for VrManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}
