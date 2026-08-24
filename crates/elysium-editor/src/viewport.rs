//! 3D Viewport for Elysium Editor
//! Handles 3D scene rendering and camera controls

use elysium_core::math::{Vec3, Mat4, Quat};
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraMode {
    Perspective,
    Orthographic,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewportMode {
    Solid,
    Wireframe,
    Textured,
    Lighting,
}

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec3,
    pub rotation: Quat,
    pub fov: f32,
    pub near_plane: f32,
    pub far_plane: f32,
    pub mode: CameraMode,
    pub zoom: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: Vec3::new(5.0, 5.0, 5.0),
            rotation: Quat::from_euler(elysium_core::math::EulerRot::XYZ, -0.785, 0.0, 0.785),
            fov: 60.0,
            near_plane: 0.1,
            far_plane: 1000.0,
            mode: CameraMode::Perspective,
            zoom: 1.0,
        }
    }

    pub fn look_at(&mut self, target: Vec3) {
        let direction = (target - self.position).normalize();
        let up = Vec3::Y;
        let right = direction.cross(up).normalize();
        let up = right.cross(direction).normalize();
        
        // Create rotation from basis vectors
        let forward = -direction;
        let rotation_matrix = Mat4::from_cols(
            right.extend(0.0),
            up.extend(0.0),
            forward.extend(0.0),
            Vec3::ZERO.extend(1.0),
        );
        
        self.rotation = Quat::from_mat4(&rotation_matrix);
    }

    pub fn get_view_matrix(&self) -> Mat4 {
        let translation = Mat4::from_translation(-self.position);
        let rotation = Mat4::from_quat(self.rotation);
        rotation * translation
    }

    pub fn get_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        match self.mode {
            CameraMode::Perspective => {
                let fov_rad = self.fov * PI / 180.0;
                Mat4::perspective_infinite_reverse_lh(fov_rad, aspect_ratio, self.near_plane)
            }
            CameraMode::Orthographic => {
                let height = 10.0 / self.zoom;
                let width = height * aspect_ratio;
                Mat4::orthographic_lh(-width, width, -height, height, self.near_plane, self.far_plane)
            }
        }
    }

    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        let sensitivity = 0.01;
        
        // Rotate around Y axis (yaw)
        let yaw_rot = Quat::from_axis_angle(Vec3::Y, delta_x * sensitivity);
        self.rotation = yaw_rot * self.rotation;
        
        // Rotate around X axis (pitch)
        let pitch_rot = Quat::from_axis_angle(Vec3::X, delta_y * sensitivity);
        self.rotation = self.rotation * pitch_rot;
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let sensitivity = 0.01;
        let right = self.rotation * Vec3::X;
        let up = self.rotation * Vec3::Y;
        
        self.position += right * delta_x * sensitivity;
        self.position += up * delta_y * sensitivity;
    }

    pub fn zoom(&mut self, delta: f32) {
        let sensitivity = 0.1;
        let forward = self.rotation * Vec3::NEG_Z;
        self.position += forward * delta * sensitivity;
        self.zoom = (self.zoom - delta * sensitivity * 0.1).max(0.1);
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Viewport {
    pub camera: Camera,
    pub mode: ViewportMode,
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_stats: bool,
    pub width: u32,
    pub height: u32,
    pub is_focused: bool,
}

impl Viewport {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            camera: Camera::new(),
            mode: ViewportMode::Solid,
            show_grid: true,
            show_axes: true,
            show_stats: false,
            width,
            height,
            is_focused: false,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    pub fn get_aspect_ratio(&self) -> f32 {
        if self.height > 0 {
            self.width as f32 / self.height as f32
        } else {
            1.0
        }
    }

    pub fn get_view_projection_matrix(&self) -> Mat4 {
        let view = self.camera.get_view_matrix();
        let projection = self.camera.get_projection_matrix(self.get_aspect_ratio());
        projection * view
    }

    pub fn toggle_grid(&mut self) {
        self.show_grid = !self.show_grid;
    }

    pub fn toggle_axes(&mut self) {
        self.show_axes = !self.show_axes;
    }

    pub fn toggle_stats(&mut self) {
        self.show_stats = !self.show_stats;
    }

    pub fn set_mode(&mut self, mode: ViewportMode) {
        self.mode = mode;
    }

    pub fn cycle_mode(&mut self) {
        self.mode = match self.mode {
            ViewportMode::Solid => ViewportMode::Wireframe,
            ViewportMode::Wireframe => ViewportMode::Textured,
            ViewportMode::Textured => ViewportMode::Lighting,
            ViewportMode::Lighting => ViewportMode::Solid,
        };
    }
}

#[derive(Debug, Clone)]
pub struct ViewportStats {
    pub fps: f32,
    pub frame_time: f32,
    pub draw_calls: u32,
    pub triangle_count: u32,
    pub vertex_count: u32,
}

impl ViewportStats {
    pub fn new() -> Self {
        Self {
            fps: 0.0,
            frame_time: 0.0,
            draw_calls: 0,
            triangle_count: 0,
            vertex_count: 0,
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        self.frame_time = delta_time;
        self.fps = if delta_time > 0.0 {
            1.0 / delta_time
        } else {
            0.0
        };
    }
}

impl Default for ViewportStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_creation() {
        let camera = Camera::new();
        assert_eq!(camera.fov, 60.0);
        assert_eq!(camera.mode, CameraMode::Perspective);
    }

    #[test]
    fn test_viewport_creation() {
        let viewport = Viewport::new(800, 600);
        assert_eq!(viewport.width, 800);
        assert_eq!(viewport.height, 600);
        assert!(viewport.show_grid);
    }

    #[test]
    fn test_aspect_ratio() {
        let viewport = Viewport::new(1920, 1080);
        let ratio = viewport.get_aspect_ratio();
        assert!((ratio - 1.777).abs() < 0.01);
    }
}
