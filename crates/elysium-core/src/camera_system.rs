use crate::math::{Vec3, Vec2, Mat4, Quat};
use std::f32::consts::PI;

/// Kamera türü
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraType {
    Perspective,
    Orthographic,
    Custom,
}

/// Kamera modu
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraMode {
    FirstPerson,
    ThirdPerson,
    Free,
    Fixed,
    Orbit,
}

/// Kamera bileşeni
#[derive(Debug, Clone)]
pub struct Camera {
    pub id: String,
    pub camera_type: CameraType,
    pub mode: CameraMode,
    pub position: Vec3,
    pub target: Vec3,         // Bakış noktası (sadece bazı modlarda kullanılır)
    pub up: Vec3,
    pub fov: f32,             // Field of View (derece cinsinden)
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
    pub ortho_size: Vec2,     // Sadece orthographic modda kullanılır
    pub view_matrix: Mat4,
    pub projection_matrix: Mat4,
    pub view_projection_matrix: Mat4,
    pub dirty: bool,          // Matrixlerin yeniden hesaplanması gerekip gerekmediğini belirtir
    pub sensitivity: f32,
    pub movement_speed: f32,
    pub zoom_speed: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub current_zoom: f32,
    pub rotation: Quat,
    pub offset: Vec3,         // Third-person offset
}

impl Camera {
    pub fn new_perspective(id: String, fov: f32, aspect_ratio: f32, near: f32, far: f32) -> Self {
        let mut camera = Self {
            id,
            camera_type: CameraType::Perspective,
            mode: CameraMode::Free,
            position: Vec3::ZERO,
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov,
            aspect_ratio,
            near,
            far,
            ortho_size: Vec2::ONE,
            view_matrix: Mat4::IDENTITY,
            projection_matrix: Mat4::IDENTITY,
            view_projection_matrix: Mat4::IDENTITY,
            dirty: true,
            sensitivity: 0.1,
            movement_speed: 1.0,
            zoom_speed: 0.1,
            min_zoom: 0.1,
            max_zoom: 100.0,
            current_zoom: 1.0,
            rotation: Quat::IDENTITY,
            offset: Vec3::new(0.0, 0.0, 0.0),
        };
        
        camera.update_matrices();
        camera
    }

    pub fn new_orthographic(id: String, size: Vec2, near: f32, far: f32) -> Self {
        let mut camera = Self {
            id,
            camera_type: CameraType::Orthographic,
            mode: CameraMode::Fixed,
            position: Vec3::ZERO,
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: 45.0,
            aspect_ratio: size.x / size.y,
            near,
            far,
            ortho_size: size,
            view_matrix: Mat4::IDENTITY,
            projection_matrix: Mat4::IDENTITY,
            view_projection_matrix: Mat4::IDENTITY,
            dirty: true,
            sensitivity: 0.1,
            movement_speed: 1.0,
            zoom_speed: 0.1,
            min_zoom: 0.1,
            max_zoom: 100.0,
            current_zoom: 1.0,
            rotation: Quat::IDENTITY,
            offset: Vec3::new(0.0, 0.0, 0.0),
        };
        
        camera.update_matrices();
        camera
    }

    /// View matrisini hesapla
    pub fn update_view_matrix(&mut self) {
        match self.mode {
            CameraMode::FirstPerson | CameraMode::ThirdPerson | CameraMode::Free | CameraMode::Orbit => {
                let forward = self.rotation * Vec3::Z;
                let right = self.rotation * Vec3::X;
                let up = self.rotation * Vec3::Y;
                
                let target = self.position + forward * self.current_zoom;
                self.view_matrix = Mat4::look_at_rh(self.position, target, up);
            }
            CameraMode::Fixed => {
                // Sabit kamera için özel hesaplama
                self.view_matrix = Mat4::from_translation(-self.position);
            }
        }
    }

    /// Projection matrisini hesapla
    pub fn update_projection_matrix(&mut self) {
        match self.camera_type {
            CameraType::Perspective => {
                self.projection_matrix = Mat4::perspective_rh_gl(
                    self.fov.to_radians(),
                    self.aspect_ratio,
                    self.near,
                    self.far
                );
            }
            CameraType::Orthographic => {
                let half_width = self.ortho_size.x * self.current_zoom * 0.5;
                let half_height = self.ortho_size.y * self.current_zoom * 0.5;
                
                self.projection_matrix = Mat4::orthographic_rh_gl(
                    -half_width, half_width,
                    -half_height, half_height,
                    self.near,
                    self.far
                );
            }
            CameraType::Custom => {
                // Özel projection matrisi için kullanıcı tanımlı matris kullanılabilir
                // Şimdilik perspective olarak ayarlıyoruz
                self.projection_matrix = Mat4::perspective_rh_gl(
                    self.fov.to_radians(),
                    self.aspect_ratio,
                    self.near,
                    self.far
                );
            }
        }
    }

    /// View ve projection matrislerini güncelle
    pub fn update_matrices(&mut self) {
        self.update_view_matrix();
        self.update_projection_matrix();
        self.view_projection_matrix = self.projection_matrix * self.view_matrix;
        self.dirty = false;
    }

    /// Kamerayı konumuna ve rotasyonuna göre güncelle
    pub fn look_at(&mut self, target: Vec3) {
        self.target = target;
        self.dirty = true;
    }

    /// Kamerayı döndür (mouse hareketi için)
    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        let yaw = Quat::from_rotation_y(yaw.to_radians());
        let pitch = Quat::from_rotation_x(pitch.to_radians());
        
        self.rotation = yaw * self.rotation * pitch;
        
        // Ters çevrilme engelleme
        let up = self.rotation * Vec3::Y;
        if up.y.abs() < 0.99 {
            self.rotation = yaw * self.rotation;
        }
        
        self.dirty = true;
    }

    /// Kamerayı hareket ettir
    pub fn move_camera(&mut self, direction: Vec3) {
        let forward = self.rotation * Vec3::Z;
        let right = self.rotation * Vec3::X;
        let up = Vec3::Y; // Dünya koordinatındaki yukarı
        
        let actual_direction = forward * direction.z + right * direction.x + up * direction.y;
        self.position += actual_direction * self.movement_speed;
        self.dirty = true;
    }

    /// Zoom yap
    pub fn zoom(&mut self, amount: f32) {
        self.current_zoom = (self.current_zoom - amount * self.zoom_speed)
            .max(self.min_zoom)
            .min(self.max_zoom);
        self.dirty = true;
    }

    /// Kameranın_forward vektörünü al
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::Z
    }

    /// Kameranın_right vektörünü al
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    /// Kameranın_up vektörünü al
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    /// Kamerayı ayarla
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
        self.dirty = true;
    }

    /// Kamerayı ayarla
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
        self.dirty = true;
    }

    /// Kamerayı ayarla
    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov;
        self.dirty = true;
    }

    /// Kamerayı ayarla
    pub fn set_mode(&mut self, mode: CameraMode) {
        self.mode = mode;
        self.dirty = true;
    }

    /// Kameranın pozisyonuna göre bir noktayı dünya koordinatlarından ekran koordinatlarına çevir
    pub fn world_to_screen(&self, world_pos: Vec3, viewport_size: Vec2) -> Option<Vec2> {
        let clip_space = self.view_projection_matrix.transform_point3(world_pos);
        
        if clip_space.w == 0.0 {
            return None;
        }
        
        let ndc = Vec3::new(clip_space.x / clip_space.w, clip_space.y / clip_space.w, clip_space.z / clip_space.w);
        
        // NDC'den ekran koordinatlarına geç
        let screen_x = (ndc.x + 1.0) * 0.5 * viewport_size.x;
        let screen_y = (1.0 - ndc.y) * 0.5 * viewport_size.y; // Y eksenini çevir
        
        Some(Vec2::new(screen_x, screen_y))
    }

    /// Ekran koordinatlarını dünya koordinatlarına çevir
    pub fn screen_to_world(&self, screen_pos: Vec2, viewport_size: Vec2, depth: f32) -> Vec3 {
        // Ekran koordinatlarını NDC'ye çevir
        let ndc_x = (screen_pos.x / viewport_size.x) * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_pos.y / viewport_size.y) * 2.0;
        let ndc_z = depth * 2.0 - 1.0;
        
        let ndc = Vec3::new(ndc_x, ndc_y, ndc_z);
        
        // Inverse view-projection matrisi ile world koordinatlarını bul
        let inv_vp = self.view_projection_matrix.inverse();
        let world = inv_vp.transform_point3(ndc);
        
        world
    }

    /// Kameranın frustum düzlemlerini al
    pub fn get_frustum_planes(&self) -> [Vec4; 6] {
        let m = self.view_projection_matrix;
        
        // Sağ düzlem
        let right = Vec4::new(
            m.x.w - m.x.x,
            m.y.w - m.y.x,
            m.z.w - m.z.x,
            m.w.w - m.w.x,
        ).normalize();
        
        // Sol düzlem
        let left = Vec4::new(
            m.x.w + m.x.x,
            m.y.w + m.y.x,
            m.z.w + m.z.x,
            m.w.w + m.w.x,
        ).normalize();
        
        // Üst düzlem
        let top = Vec4::new(
            m.x.w - m.x.y,
            m.y.w - m.y.y,
            m.z.w - m.z.y,
            m.w.w - m.w.y,
        ).normalize();
        
        // Alt düzlem
        let bottom = Vec4::new(
            m.x.w + m.x.y,
            m.y.w + m.y.y,
            m.z.w + m.z.y,
            m.w.w + m.w.y,
        ).normalize();
        
        // Yakın düzlem
        let near = Vec4::new(
            m.x.w + m.x.z,
            m.y.w + m.y.z,
            m.z.w + m.z.z,
            m.w.w + m.w.z,
        ).normalize();
        
        // Uzak düzlem
        let far = Vec4::new(
            m.x.w - m.x.z,
            m.y.w - m.y.z,
            m.z.w - m.z.z,
            m.w.w - m.w.z,
        ).normalize();
        
        [right, left, top, bottom, near, far]
    }
}

/// Kamera sistem durumu
#[derive(Debug, Clone)]
pub struct CameraSystem {
    pub cameras: std::collections::HashMap<String, Camera>,
    pub active_camera: Option<String>,
    pub default_camera: String,
}

impl Default for CameraSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraSystem {
    pub fn new() -> Self {
        let mut system = Self {
            cameras: std::collections::HashMap::new(),
            active_camera: None,
            default_camera: String::new(),
        };
        
        // Varsayılan kamera oluştur
        let default_cam = Camera::new_perspective(
            "default".to_string(),
            45.0,
            16.0 / 9.0,
            0.1,
            1000.0
        );
        
        system.add_camera(default_cam);
        system.set_active_camera("default");
        
        system
    }

    /// Kamera ekle
    pub fn add_camera(&mut self, camera: Camera) {
        let id = camera.id.clone();
        self.cameras.insert(id, camera);
        
        if self.active_camera.is_none() {
            self.active_camera = Some("default".to_string());
        }
    }

    /// Kamera al
    pub fn get_camera(&self, id: &str) -> Option<&Camera> {
        self.cameras.get(id)
    }

    /// Kamera al (mutable)
    pub fn get_camera_mut(&mut self, id: &str) -> Option<&mut Camera> {
        self.cameras.get_mut(id)
    }

    /// Aktif kamerayı al
    pub fn get_active_camera(&self) -> Option<&Camera> {
        if let Some(ref id) = self.active_camera {
            self.cameras.get(id)
        } else {
            None
        }
    }

    /// Aktif kamerayı al (mutable)
    pub fn get_active_camera_mut(&mut self) -> Option<&mut Camera> {
        if let Some(ref id) = self.active_camera {
            self.cameras.get_mut(id)
        } else {
            None
        }
    }

    /// Aktif kamerayı ayarla
    pub fn set_active_camera(&mut self, id: &str) -> bool {
        if self.cameras.contains_key(id) {
            self.active_camera = Some(id.to_string());
            true
        } else {
            false
        }
    }

    /// Kamera kaldır
    pub fn remove_camera(&mut self, id: &str) -> bool {
        if self.active_camera.as_ref().map_or(false, |active| active == id) {
            // Aktif kamera kaldırılıyorsa, başka bir kamerayı aktif yap
            if let Some(first_key) = self.cameras.keys().next().cloned() {
                self.active_camera = Some(first_key);
            } else {
                self.active_camera = None;
            }
        }
        
        self.cameras.remove(id).is_some()
    }

    /// Sistemi güncelle
    pub fn update(&mut self) {
        // Kamera matrislerini güncelle
        for camera in self.cameras.values_mut() {
            if camera.dirty {
                camera.update_matrices();
            }
        }
    }

    /// Kamera konumunu ayarla
    pub fn set_camera_position(&mut self, id: &str, position: Vec3) -> bool {
        if let Some(camera) = self.cameras.get_mut(id) {
            camera.set_position(position);
            true
        } else {
            false
        }
    }

    /// Kamera rotasyonunu ayarla
    pub fn set_camera_rotation(&mut self, id: &str, rotation: Quat) -> bool {
        if let Some(camera) = self.cameras.get_mut(id) {
            camera.rotation = rotation;
            camera.dirty = true;
            true
        } else {
            false
        }
    }

    /// Kamera modunu ayarla
    pub fn set_camera_mode(&mut self, id: &str, mode: CameraMode) -> bool {
        if let Some(camera) = self.cameras.get_mut(id) {
            camera.set_mode(mode);
            true
        } else {
            false
        }
    }

    /// Kamera FOV'nu ayarla
    pub fn set_camera_fov(&mut self, id: &str, fov: f32) -> bool {
        if let Some(camera) = self.cameras.get_mut(id) {
            camera.set_fov(fov);
            true
        } else {
            false
        }
    }

    /// Kamera aspect ratio'sunu ayarla
    pub fn set_camera_aspect_ratio(&mut self, id: &str, aspect_ratio: f32) -> bool {
        if let Some(camera) = self.cameras.get_mut(id) {
            camera.set_aspect_ratio(aspect_ratio);
            true
        } else {
            false
        }
    }

    /// Kamerayı hareket ettir
    pub fn move_camera(&mut self, id: &str, direction: Vec3) -> bool {
        if let Some(camera) = self.cameras.get_mut(id) {
            camera.move_camera(direction);
            true
        } else {
            false
        }
    }

    /// Kamerayı döndür
    pub fn rotate_camera(&mut self, id: &str, yaw: f32, pitch: f32) -> bool {
        if let Some(camera) = self.cameras.get_mut(id) {
            camera.rotate(yaw, pitch);
            true
        } else {
            false
        }
    }

    /// Zoom yap
    pub fn zoom_camera(&mut self, id: &str, amount: f32) -> bool {
        if let Some(camera) = self.cameras.get_mut(id) {
            camera.zoom(amount);
            true
        } else {
            false
        }
    }

    /// World-to-screen dönüşümü yap
    pub fn world_to_screen(&self, id: &str, world_pos: Vec3, viewport_size: Vec2) -> Option<Vec2> {
        if let Some(camera) = self.cameras.get(id) {
            camera.world_to_screen(world_pos, viewport_size)
        } else {
            None
        }
    }

    /// Screen-to-world dönüşümü yap
    pub fn screen_to_world(&self, id: &str, screen_pos: Vec2, viewport_size: Vec2, depth: f32) -> Option<Vec3> {
        if let Some(camera) = self.cameras.get(id) {
            Some(camera.screen_to_world(screen_pos, viewport_size, depth))
        } else {
            None
        }
    }

    /// Kamera sayısını al
    pub fn camera_count(&self) -> usize {
        self.cameras.len()
    }

    /// Tüm kamera isimlerini al
    pub fn camera_names(&self) -> Vec<String> {
        self.cameras.keys().cloned().collect()
    }
}

/// Kamera yardımcı fonksiyonları
pub mod helpers {
    use super::*;

    /// LookAt matrisi oluştur
    pub fn look_at_matrix(position: Vec3, target: Vec3, up: Vec3) -> Mat4 {
        Mat4::look_at_rh(position, target, up)
    }

    /// Perspective projection matrisi oluştur
    pub fn perspective_matrix(fov: f32, aspect_ratio: f32, near: f32, far: f32) -> Mat4 {
        Mat4::perspective_rh_gl(fov.to_radians(), aspect_ratio, near, far)
    }

    /// Orthographic projection matrisi oluştur
    pub fn orthographic_matrix(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4 {
        Mat4::orthographic_rh_gl(left, right, bottom, top, near, far)
    }

    /// Kamera frustum culling
    pub fn is_sphere_in_frustum(center: Vec3, radius: f32, frustum_planes: &[Vec4; 6]) -> bool {
        for plane in frustum_planes {
            let distance = plane.x * center.x + plane.y * center.y + plane.z * center.z + plane.w;
            if distance < -radius {
                return false; // Tamamen frustum dışında
            }
        }
        true // Frustum içinde veya kesişiyor
    }

    /// Kamera frustum culling (dikdörtgen için)
    pub fn is_box_in_frustum(min: Vec3, max: Vec3, frustum_planes: &[Vec4; 6]) -> bool {
        for plane in frustum_planes {
            let mut out = 0;
            
            // 8 köşe noktayı test et
            let corners = [
                Vec3::new(min.x, min.y, min.z),
                Vec3::new(max.x, min.y, min.z),
                Vec3::new(min.x, max.y, min.z),
                Vec3::new(max.x, max.y, min.z),
                Vec3::new(min.x, min.y, max.z),
                Vec3::new(max.x, min.y, max.z),
                Vec3::new(min.x, max.y, max.z),
                Vec3::new(max.x, max.y, max.z),
            ];
            
            for corner in &corners {
                if plane.x * corner.x + plane.y * corner.y + plane.z * corner.z + plane.w > 0.0 {
                    out += 1;
                }
            }
            
            // Eğer tüm köşeler düzlemin dışında ise, kutu frustum içinde değil
            if out == 0 {
                return false;
            }
        }
        
        true
    }
}

/// Kamera sistemleri için önceden tanımlanmış ayarlar
pub mod presets {
    use super::*;

    /// First-person kamera oluştur
    pub fn first_person_camera(id: String, aspect_ratio: f32) -> Camera {
        let mut cam = Camera::new_perspective(id, 75.0, aspect_ratio, 0.1, 1000.0);
        cam.mode = CameraMode::FirstPerson;
        cam.movement_speed = 3.0;
        cam.sensitivity = 0.1;
        cam
    }

    /// Third-person kamera oluştur
    pub fn third_person_camera(id: String, aspect_ratio: f32) -> Camera {
        let mut cam = Camera::new_perspective(id, 45.0, aspect_ratio, 0.1, 1000.0);
        cam.mode = CameraMode::ThirdPerson;
        cam.offset = Vec3::new(0.0, 2.0, 5.0);
        cam.movement_speed = 2.0;
        cam.sensitivity = 0.1;
        cam
    }

    /// Ortografik kamera oluştur
    pub fn orthographic_camera(id: String, size: Vec2) -> Camera {
        let mut cam = Camera::new_orthographic(id, size, 0.1, 1000.0);
        cam.mode = CameraMode::Fixed;
        cam
    }

    /// Orbit kamera oluştur
    pub fn orbit_camera(id: String, aspect_ratio: f32) -> Camera {
        let mut cam = Camera::new_perspective(id, 45.0, aspect_ratio, 0.1, 1000.0);
        cam.mode = CameraMode::Orbit;
        cam.movement_speed = 1.0;
        cam.zoom_speed = 0.5;
        cam.min_zoom = 1.0;
        cam.max_zoom = 100.0;
        cam
    }
}

/// Kamera sistemini başlatan yardımcı fonksiyon
pub fn initialize_camera_system() -> CameraSystem {
    let mut system = CameraSystem::new();
    
    // Bazı öntanımlı kameralar ekle
    system.add_camera(presets::first_person_camera("fp_camera".to_string(), 16.0/9.0));
    system.add_camera(presets::third_person_camera("tp_camera".to_string(), 16.0/9.0));
    system.add_camera(presets::orthographic_camera("ortho_camera".to_string(), Vec2::new(20.0, 15.0)));
    system.add_camera(presets::orbit_camera("orbit_camera".to_string(), 16.0/9.0));
    
    system
}