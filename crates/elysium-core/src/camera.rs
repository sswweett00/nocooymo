// Kamera türleri
use crate::math::{EulerRot, Mat4, Quat, Vec2, Vec3, Vec4, vec4};
use crate::Transform;

#[derive(Debug, Clone, PartialEq)]
pub enum CameraProjection {
    Perspective { fov: f32, aspect: f32, near: f32, far: f32 },
    Orthographic { left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32 },
}

// Kamera bileşeni
pub struct Camera {
    pub projection: CameraProjection,
    pub viewport: Option<(u32, u32, u32, u32)>, // x, y, width, height
    pub clear_color: [f32; 4],
    pub priority: i32,
    pub enabled: bool,
    pub render_target: Option<u32>, // Opsiyonel render hedefi
    pub use_dynamic_aspect: bool,   // Ekran boyutu değiştiğinde oranı otomatik güncelle
}

impl Camera {
    pub fn new_perspective(fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            projection: CameraProjection::Perspective { fov, aspect, near, far },
            viewport: None,
            clear_color: [0.1, 0.1, 0.1, 1.0],
            priority: 0,
            enabled: true,
            render_target: None,
            use_dynamic_aspect: true,
        }
    }
    
    pub fn new_orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        Self {
            projection: CameraProjection::Orthographic { left, right, bottom, top, near, far },
            viewport: None,
            clear_color: [0.1, 0.1, 0.1, 1.0],
            priority: 0,
            enabled: true,
            render_target: None,
            use_dynamic_aspect: false,
        }
    }
    
    pub fn perspective_with_defaults(aspect: f32) -> Self {
        Self::new_perspective(45.0_f32.to_radians(), aspect, 0.1, 1000.0)
    }
    
    pub fn orthographic_2d(width: f32, height: f32) -> Self {
        let half_width = width / 2.0;
        let half_height = height / 2.0;
        Self::new_orthographic(-half_width, half_width, -half_height, half_height, -1.0, 1.0)
    }
    
    pub fn set_perspective(&mut self, fov: f32, aspect: f32, near: f32, far: f32) {
        self.projection = CameraProjection::Perspective { fov, aspect, near, far };
    }
    
    pub fn set_orthographic(&mut self, left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) {
        self.projection = CameraProjection::Orthographic { left, right, bottom, top, near, far };
    }
    
    pub fn set_aspect_ratio(&mut self, aspect: f32) {
        match &mut self.projection {
            CameraProjection::Perspective { aspect: ref mut cam_aspect, .. } => {
                *cam_aspect = aspect;
            }
            CameraProjection::Orthographic { .. } => {
                // Ortografik projeksiyonda oran manuel olarak ayarlanmalı
            }
        }
    }
    
    pub fn get_projection_matrix(&self) -> Mat4 {
        match &self.projection {
            CameraProjection::Perspective { fov, aspect, near, far } => {
                Mat4::perspective_rh(*fov, *aspect, *near, *far)
            }
            CameraProjection::Orthographic { left, right, bottom, top, near, far } => {
                Mat4::orthographic_rh(*left, *right, *bottom, *top, *near, *far)
            }
        }
    }
    
    pub fn screen_to_world_ray(&self, screen_pos: Vec2, screen_size: Vec2, transform: &Transform) -> (Vec3, Vec3) {
        // Normalized device coordinates (NDC) hesapla
        let ndc_x = (2.0 * screen_pos.x / screen_size.x) - 1.0;
        let ndc_y = 1.0 - (2.0 * screen_pos.y / screen_size.y);
        
        // Projection matrisinin tersini al
        let proj_matrix = self.get_projection_matrix();
        let inv_proj_matrix = proj_matrix.inverse();
        
        // View matrisini hesapla ve tersini al
        let view_matrix = transform.compute_view_matrix();
        let inv_view_matrix = view_matrix.inverse();
        
        // NDC'den view space'e — z bileşenini de doğru şekilde böl
        let clip_coords = Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
        let view_coords = inv_proj_matrix * clip_coords;
        // Perspektif bölme sonrası view uzayı yönü
        let view_ray = Vec3::new(
            view_coords.x / view_coords.w,
            view_coords.y / view_coords.w,
            view_coords.z / view_coords.w,
        );
        
        // World space'e dönüştür
        let world_origin = transform.translation;
        let world_direction = inv_view_matrix.transform_vector3(view_ray).normalize();
        
        (world_origin, world_direction)
    }
    
    pub fn world_to_screen(&self, world_pos: Vec3, transform: &Transform, screen_size: Vec2) -> Option<Vec2> {
        let view_matrix = transform.compute_view_matrix();
        let proj_matrix = self.get_projection_matrix();
        let view_proj_matrix = proj_matrix * view_matrix;
        
        let clip_space_pos = view_proj_matrix * world_pos.extend(1.0);
        
        if clip_space_pos.w == 0.0 {
            return None;
        }
        
        let ndc_space_pos = Vec3::new(
            clip_space_pos.x / clip_space_pos.w,
            clip_space_pos.y / clip_space_pos.w,
            clip_space_pos.z / clip_space_pos.w,
        );
        
        // NDC'den screen space'e
        if ndc_space_pos.z < -1.0 || ndc_space_pos.z > 1.0 {
            return None; // Kamera ön-arka düzlemi dışında
        }
        
        let screen_x = ((ndc_space_pos.x + 1.0) / 2.0) * screen_size.x;
        let screen_y = ((1.0 - ndc_space_pos.y) / 2.0) * screen_size.y;
        
        Some(Vec2::new(screen_x, screen_y))
    }
    
    pub fn set_viewport(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.viewport = Some((x, y, width, height));
    }
    
    pub fn clear_viewport(&mut self) {
        self.viewport = None;
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        if self.use_dynamic_aspect {
            let aspect = width as f32 / height as f32;
            self.set_aspect_ratio(aspect);
        }
    }
}

// Kamera yardımcı fonksiyonları
impl Transform {
    pub fn compute_view_matrix(&self) -> Mat4 {
        // Look-at matrisi hesapla
        let eye = self.translation;
        let target = eye + self.forward_vector();
        let up = self.up_vector();
        
        Mat4::look_at_rh(eye, target, up)
    }
    
    pub fn forward_vector(&self) -> Vec3 {
        // Rotasyon quaternion'unu kullanarak ileri vektörünü hesapla
        self.rotation * -Vec3::Z
    }
    
    pub fn right_vector(&self) -> Vec3 {
        // Sağ vektörünü hesapla (ileri vektörüne dik)
        self.rotation * Vec3::X
    }
    
    pub fn up_vector(&self) -> Vec3 {
        // Yukarı vektörünü hesapla
        self.rotation * Vec3::Y
    }
}

// Kamera kontrolleri
pub struct FreeCameraController {
    pub move_speed: f32,
    pub look_speed: f32,
    pub enable_flight: bool,
    pub invert_y: bool,
    pub sensitivity: f32,
    pub current_move_direction: Vec3,
    pub current_look_direction: Vec2,
}

impl FreeCameraController {
    pub fn new() -> Self {
        Self {
            move_speed: 5.0,
            look_speed: 2.0,
            enable_flight: true,
            invert_y: false,
            sensitivity: 0.1,
            current_move_direction: Vec3::new(0.0, 0.0, 0.0),
            current_look_direction: Vec2::new(0.0, 0.0),
        }
    }
    
    pub fn set_move_speed(&mut self, speed: f32) {
        self.move_speed = speed;
    }
    
    pub fn set_look_speed(&mut self, speed: f32) {
        self.look_speed = speed;
    }
    
    pub fn update_transform(&self, transform: &mut Transform, delta_time: f32) {
        // Hareket yönüne göre konumu güncelle
        let forward = transform.forward_vector();
        let right = transform.right_vector();
        let up = if self.enable_flight { transform.up_vector() } else { Vec3::new(0.0, 1.0, 0.0) };
        
        let movement = 
            forward * self.current_move_direction.z * self.move_speed * delta_time +
            right * self.current_move_direction.x * self.move_speed * delta_time +
            up * self.current_move_direction.y * self.move_speed * delta_time;
        
        transform.translation += movement;
        
        // Bakış yönüne göre rotasyonu güncelle
        let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
        let new_yaw = yaw + self.current_look_direction.x * self.look_speed * delta_time;
        let new_pitch = if self.enable_flight {
            (pitch + self.current_look_direction.y * self.look_speed * delta_time)
                .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians())
        } else {
            pitch
        };
        
        transform.rotation = Quat::from_euler(EulerRot::YXZ, new_yaw, new_pitch, 0.0);
    }
    
    pub fn handle_input(&mut self, input_manager: &crate::input::InputManager) {
        // Hareket girdilerini işle
        let mut move_dir = Vec3::new(0.0, 0.0, 0.0);
        
        if input_manager.get_input_state().is_action_down("move_forward") {
            move_dir.z += 1.0;
        }
        if input_manager.get_input_state().is_action_down("move_backward") {
            move_dir.z -= 1.0;
        }
        if input_manager.get_input_state().is_action_down("move_left") {
            move_dir.x -= 1.0;
        }
        if input_manager.get_input_state().is_action_down("move_right") {
            move_dir.x += 1.0;
        }
        if self.enable_flight {
            if input_manager.get_input_state().is_action_down("move_up") {
                move_dir.y += 1.0;
            }
            if input_manager.get_input_state().is_action_down("move_down") {
                move_dir.y -= 1.0;
            }
        }
        
        self.current_move_direction = move_dir.normalize_or_zero();
        
        // Bakış girdilerini işle
        let mouse_delta = input_manager.get_input_state().get_mouse_delta();
        let look_dir = Vec2::new(
            mouse_delta.x * self.sensitivity,
            -mouse_delta.y * self.sensitivity * if self.invert_y { -1.0 } else { 1.0 }
        );
        
        self.current_look_direction = look_dir;
    }
}

// Takip kamerası bileşeni
pub struct FollowCamera {
    pub target: Option<u32>, // Takip edilen varlığın ID'si
    pub offset: Vec3,
    pub smooth_follow: bool,
    pub follow_smoothness: f32,
    pub look_at_target: bool,
    pub rotation_lag: f32,
    pub distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub zoom_speed: f32,
}

impl FollowCamera {
    pub fn new(target: Option<u32>) -> Self {
        Self {
            target,
            offset: Vec3::new(0.0, 2.0, -5.0),
            smooth_follow: true,
            follow_smoothness: 5.0,
            look_at_target: true,
            rotation_lag: 0.0,
            distance: 5.0,
            min_distance: 1.0,
            max_distance: 20.0,
            zoom_speed: 10.0,
        }
    }
    
    pub fn set_target(&mut self, target: Option<u32>) {
        self.target = target;
    }
    
    pub fn set_offset(&mut self, offset: Vec3) {
        self.offset = offset;
    }
    
    pub fn zoom(&mut self, amount: f32) {
        self.distance = (self.distance - amount * self.zoom_speed).clamp(self.min_distance, self.max_distance);
    }
    
    pub fn update_transform(&self, transform: &mut Transform, target_transform: &Transform, delta_time: f32) {
        let target_pos = target_transform.translation;
        // Offset yönünü koru, uzaklığı distance ile ölçekle — normalize magnitude'u kaybetmesin
        let desired_pos = if self.offset.length_squared() > 1e-9 {
            target_pos + self.offset.normalize() * self.distance
        } else {
            target_pos + Vec3::new(0.0, 2.0, -1.0).normalize() * self.distance
        };
        // Eğer offset zaten istenen mesafeyi içeriyorsa, doğrudan kullan
        // (offset'in uzunluğu distance'a yakınsa normalize etmeden ekle)
        let _ = desired_pos;
        
        if self.smooth_follow {
            // Yumuşak takip
            transform.translation = transform.translation.lerp(desired_pos, (self.follow_smoothness * delta_time).min(1.0));
        } else {
            transform.translation = desired_pos;
        }
        
        if self.look_at_target {
            // Takip edilen varlığa doğru bak
            let direction = (target_pos - transform.translation).normalize();
            transform.rotation = direction_to_rotation(direction);
        }
    }
}

// Kamera modları
#[derive(Debug, Clone, PartialEq)]
pub enum CameraMode {
    FirstPerson,
    ThirdPerson { offset: Vec3, distance: f32 },
    TopDown { height: f32 },
    SideScrolling,
}

// Kamera modu bileşeni
pub struct CameraModeController {
    pub mode: CameraMode,
    pub transition_speed: f32,
    pub current_transition: Option<CameraTransition>,
}

#[derive(Debug, Clone)]
struct CameraTransition {
    start_transform: Transform,
    target_transform: Transform,
    progress: f32,
    duration: f32,
}

impl CameraModeController {
    pub fn new(mode: CameraMode) -> Self {
        Self {
            mode,
            transition_speed: 2.0,
            current_transition: None,
        }
    }
    
    pub fn switch_mode(&mut self, new_mode: CameraMode, current_transform: Transform, target_transform: Transform) {
        self.current_transition = Some(CameraTransition {
            start_transform: current_transform,
            target_transform,
            progress: 0.0,
            duration: 1.0 / self.transition_speed,
        });
        self.mode = new_mode;
    }
    
    pub fn update(&mut self, transform: &mut Transform, delta_time: f32) {
        if let Some(ref mut transition) = self.current_transition {
            transition.progress += delta_time / transition.duration;
            
            if transition.progress >= 1.0 {
                // Geçiş tamamlandı
                *transform = transition.target_transform.clone();
                self.current_transition = None;
            } else {
                // Aradeğerleme
                transform.translation = transition.start_transform.translation
                    .lerp(transition.target_transform.translation, transition.progress);
                transform.rotation = transition.start_transform.rotation
                    .slerp(transition.target_transform.rotation, transition.progress);
                transform.scale = transition.start_transform.scale
                    .lerp(transition.target_transform.scale, transition.progress);
            }
        }
    }
}

// Yardımcı fonksiyonlar
fn direction_to_rotation(direction: Vec3) -> Quat {
    let pitch = (-direction.y).asin();
    let yaw = direction.x.atan2(direction.z);
    Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0)
}

// Kamera frustum — math::Frustum ile aynı mantık, transform için yeniden ihraç
#[derive(Debug, Clone)]
pub struct Frustum {
    pub planes: [Vec4; 6],
}

impl Frustum {
    pub fn from_view_projection(view_proj_matrix: &Mat4) -> Self {
        let m = view_proj_matrix.to_cols_array_2d();
        let row0 = Vec4::new(m[0][0], m[1][0], m[2][0], m[3][0]);
        let row1 = Vec4::new(m[0][1], m[1][1], m[2][1], m[3][1]);
        let row2 = Vec4::new(m[0][2], m[1][2], m[2][2], m[3][2]);
        let row3 = Vec4::new(m[0][3], m[1][3], m[2][3], m[3][3]);
        Self { 
            planes: [
                (row3 + row0).normalize(),
                (row3 - row0).normalize(),
                (row3 + row1).normalize(),
                (row3 - row1).normalize(),
                (row3 + row2).normalize(),
                (row3 - row2).normalize(),
            ]
        }
    }
    
    pub fn contains_point(&self, point: Vec3) -> bool {
        for plane in &self.planes {
            let dot_product = plane.x * point.x + plane.y * point.y + plane.z * point.z + plane.w;
            if dot_product < 0.0 {
                return false; // Nokta düzlemin dışındaysa frustumun dışında
            }
        }
        true
    }
    
    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            let distance = plane.x * center.x + plane.y * center.y + plane.z * center.z + plane.w;
            if distance < -radius {
                return false; // Küre tamamen düzlemin dışındaysa frustumun dışında
            }
        }
        true
    }
}

// Kamera sistem bileşeni
pub struct CameraSystem {
    pub cameras: Vec<(u32, Camera)>, // (varlık id, kamera)
    pub active_camera: Option<u32>,
    pub render_order: Vec<u32>,
}

impl CameraSystem {
    pub fn new() -> Self {
        Self {
            cameras: Vec::new(),
            active_camera: None,
            render_order: Vec::new(),
        }
    }
    
    pub fn add_camera(&mut self, entity_id: u32, camera: Camera) {
        self.cameras.push((entity_id, camera));
        self.sort_cameras();
    }
    
    pub fn remove_camera(&mut self, entity_id: u32) {
        self.cameras.retain(|(id, _)| *id != entity_id);
        if self.active_camera == Some(entity_id) {
            self.active_camera = None;
        }
    }
    
    pub fn set_active_camera(&mut self, entity_id: u32) {
        if self.cameras.iter().any(|(id, _)| *id == entity_id) {
            self.active_camera = Some(entity_id);
        }
    }
    
    pub fn get_active_camera(&self) -> Option<&Camera> {
        if let Some(active_id) = self.active_camera {
            self.cameras.iter()
                .find(|(id, _)| *id == active_id)
                .map(|(_, camera)| camera)
        } else {
            None
        }
    }
    
    pub fn get_active_camera_mut(&mut self) -> Option<&mut Camera> {
        if let Some(active_id) = self.active_camera {
            self.cameras.iter_mut()
                .find(|(id, _)| *id == active_id)
                .map(|(_, camera)| camera)
        } else {
            None
        }
    }
    
    pub fn sort_cameras(&mut self) {
        self.cameras.sort_by(|a, b| b.1.priority.cmp(&a.1.priority));
        self.render_order = self.cameras.iter().map(|(id, _)| *id).collect();
    }
    
    pub fn update_camera_aspect(&mut self, width: u32, height: u32) {
        for (_, camera) in &mut self.cameras {
            camera.resize(width, height);
        }
    }
    
    pub fn update(&mut self, _delta_time: f32) {
        // Kamera güncellemeleri burada yapılabilir
        // Örneğin: frustum hesaplamaları, viewport güncellemeleri
    }
}

// Kamera efektleri
pub struct CameraEffects {
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub motion_blur_enabled: bool,
    pub motion_blur_strength: f32,
    pub depth_of_field_enabled: bool,
    pub dof_focus_distance: f32,
    pub dof_range: f32,
    pub chromatic_aberration_enabled: bool,
    pub chromatic_aberration_amount: f32,
    pub vignette_enabled: bool,
    pub vignette_strength: f32,
    pub film_grain_enabled: bool,
    pub film_grain_strength: f32,
}

impl CameraEffects {
    pub fn new() -> Self {
        Self {
            bloom_enabled: false,
            bloom_intensity: 1.0,
            motion_blur_enabled: false,
            motion_blur_strength: 1.0,
            depth_of_field_enabled: false,
            dof_focus_distance: 10.0,
            dof_range: 5.0,
            chromatic_aberration_enabled: false,
            chromatic_aberration_amount: 0.05,
            vignette_enabled: false,
            vignette_strength: 0.5,
            film_grain_enabled: false,
            film_grain_strength: 0.1,
        }
    }
    
    pub fn enable_bloom(&mut self, intensity: f32) {
        self.bloom_enabled = true;
        self.bloom_intensity = intensity;
    }
    
    pub fn enable_motion_blur(&mut self, strength: f32) {
        self.motion_blur_enabled = true;
        self.motion_blur_strength = strength;
    }
    
    pub fn enable_depth_of_field(&mut self, focus_distance: f32, range: f32) {
        self.depth_of_field_enabled = true;
        self.dof_focus_distance = focus_distance;
        self.dof_range = range;
    }
    
    pub fn enable_chromatic_aberration(&mut self, amount: f32) {
        self.chromatic_aberration_enabled = true;
        self.chromatic_aberration_amount = amount;
    }
    
    pub fn enable_vignette(&mut self, strength: f32) {
        self.vignette_enabled = true;
        self.vignette_strength = strength;
    }
    
    pub fn enable_film_grain(&mut self, strength: f32) {
        self.film_grain_enabled = true;
        self.film_grain_strength = strength;
    }
}
