//! 3D Viewport — sahne görüntüleme ve kamera kontrolü.

/// Kamera modu
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraMode { Perspective, Orthographic }

/// Viewport modu
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewportMode { Solid, Wireframe, Textured, Lighting }

/// Kamera
pub struct Camera {
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub mode: CameraMode,
}

impl Default for Camera {
    fn default() -> Self { Self::new() }
}

impl Camera {
    pub fn new() -> Self {
        Self { position: [0.0, 0.0, 0.0], yaw: -45.0, pitch: 30.0, distance: 18.0,
               fov: 60.0, near: 0.1, far: 1000.0, mode: CameraMode::Perspective }
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw += dx;
        self.pitch = (self.pitch + dy).clamp(-89.0, 89.0);
    }

    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance - delta).clamp(2.0, 200.0);
    }

    pub fn reset(&mut self) { *self = Self::new(); }

    pub fn eye_position(&self) -> [f32; 3] {
        let yaw_r = self.yaw.to_radians();
        let pitch_r = self.pitch.to_radians();
        [
            self.position[0] + self.distance * yaw_r.cos() * pitch_r.cos(),
            self.position[1] + self.distance * pitch_r.sin(),
            self.position[2] + self.distance * yaw_r.sin() * pitch_r.cos(),
        ]
    }
}

/// Viewport
pub struct Viewport {
    pub camera: Camera,
    pub mode: ViewportMode,
    pub show_grid: bool,
    pub show_axes: bool,
    pub width: u32,
    pub height: u32,
}

impl Default for Viewport {
    fn default() -> Self { Self::new(800, 600) }
}

impl Viewport {
    pub fn new(width: u32, height: u32) -> Self {
        Self { camera: Camera::new(), mode: ViewportMode::Solid, show_grid: true, show_axes: true, width, height }
    }

    pub fn resize(&mut self, w: u32, h: u32) { self.width = w; self.height = h; }
    pub fn aspect_ratio(&self) -> f32 { self.width as f32 / self.height.max(1) as f32 }
    pub fn toggle_grid(&mut self) { self.show_grid = !self.show_grid; }
    pub fn cycle_mode(&mut self) {
        self.mode = match self.mode {
            ViewportMode::Solid => ViewportMode::Wireframe,
            ViewportMode::Wireframe => ViewportMode::Textured,
            ViewportMode::Textured => ViewportMode::Lighting,
            ViewportMode::Lighting => ViewportMode::Solid,
        };
    }
}

/// Performans istatistikleri
pub struct ViewportStats {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub draw_calls: u32,
    pub triangles: u32,
}

impl Default for ViewportStats {
    fn default() -> Self { Self::new() }
}

impl ViewportStats {
    pub fn new() -> Self { Self { fps: 0.0, frame_time_ms: 0.0, draw_calls: 0, triangles: 0 } }
    pub fn update(&mut self, dt: f32) {
        self.frame_time_ms = dt * 1000.0;
        self.fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
    }
}
