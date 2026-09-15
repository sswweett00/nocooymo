//! selection_gizmo.rs — Çoklu seçim ve transform gizmosu
//!
//! - Box selection: Fare sürükleme ile dikdörtgen seçim alanı
//! - Multi-select: Ctrl ile çoklu seçim, Shift ile toggle
//! - Transform gizmosu: Translate/Rotate/Scale eksen göstergeleri
//! - Toplu transform: Seçili nesnelere aynı anda uygula

use glam::Vec3;
use crate::renderer::*;

// ═══════════════════════════════════════════════════════════ Seçim Sistemi

/// Seçim modu
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SelectionMode {
    Single,
    Box,
}

/// Seçim durumu
#[derive(Clone, Debug)]
pub struct SelectionState {
    /// Seçili nesne ID'leri
    pub selected_ids: Vec<usize>,
    /// Tekil seçim (hızlı erişim)
    pub primary_selection: Option<usize>,
    /// Seçim modu
    pub mode: SelectionMode,
    /// Box selection durumu
    pub box_selection: Option<BoxSelection>,
    /// Gizmo modu
    pub gizmo_mode: GizmoMode,
    /// Gizmo activе mi?
    pub gizmo_active: bool,
    /// Seçim kutusu rengi
    pub selection_color: [u8; 4],
}

/// Box selection rect
#[derive(Clone, Copy, Debug)]
pub struct BoxSelection {
    pub start_x: f64,
    pub start_y: f64,
    pub end_x: f64,
    pub end_y: f64,
    pub is_active: bool,
}

impl BoxSelection {
    pub fn new(start_x: f64, start_y: f64) -> Self {
        Self {
            start_x, start_y,
            end_x: start_x,
            end_y: start_y,
            is_active: true,
        }
    }

    /// Normalized koordinatlarda rect döndür (her iki yön de pozitif)
    pub fn normalized_rect(&self) -> (f64, f64, f64, f64) {
        let x = self.start_x.min(self.end_x);
        let y = self.start_y.min(self.end_y);
        let w = (self.end_x - self.start_x).abs();
        let h = (self.end_y - self.start_y).abs();
        (x, y, w, h)
    }

    /// Rect'in alanını hesapla
    pub fn area(&self) -> f64 {
        let (_, _, w, h) = self.normalized_rect();
        w * h
    }
}

/// Gizmo modu
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GizmoMode {
    Translate,
    Rotate,
    Scale,
    None,
}

impl GizmoMode {
    pub fn label(&self) -> &str {
        match self {
            Self::Translate => "Move",
            Self::Rotate => "Rotate",
            Self::Scale => "Scale",
            Self::None => "None",
        }
    }

    pub fn shortcut_key() -> &'static str {
        "G/R/S"
    }

    /// Sıradaki moda geç
    pub fn next(&self) -> Self {
        match self {
            Self::Translate => Self::Rotate,
            Self::Rotate => Self::Scale,
            Self::Scale => Self::Translate,
            Self::None => Self::Translate,
        }
    }
}

/// Gizmo eksen türü
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GizmoAxis {
    X,
    Y,
    Z,
    XY,
    XZ,
    YZ,
    XYZ,
}

impl GizmoAxis {
    pub fn color(&self) -> [u8; 4] {
        match self {
            Self::X => [230, 80, 80, 255],   // Kırmızı
            Self::Y => [80, 200, 80, 255],   // Yeşil
            Self::Z => [80, 130, 230, 255],  // Mavi
            Self::XY => [230, 200, 80, 255], // Sarı
            Self::XZ => [200, 80, 230, 255], // Mor
            Self::YZ => [80, 230, 200, 255], // Cyan
            Self::XYZ => [200, 200, 200, 255], // Gri
        }
    }
}

impl Default for SelectionState {
    fn default() -> Self {
        Self {
            selected_ids: Vec::new(),
            primary_selection: None,
            mode: SelectionMode::Single,
            box_selection: None,
            gizmo_mode: GizmoMode::Translate,
            gizmo_active: false,
            selection_color: [9, 71, 113, 255],
        }
    }
}

impl SelectionState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Tek nesne seç
    pub fn select_single(&mut self, id: usize) {
        self.selected_ids.clear();
        self.selected_ids.push(id);
        self.primary_selection = Some(id);
    }

    /// Seçime ekle/kaldır (toggle)
    pub fn toggle_selection(&mut self, id: usize) {
        if let Some(pos) = self.selected_ids.iter().position(|&x| x == id) {
            self.selected_ids.remove(pos);
            if self.primary_selection == Some(id) {
                self.primary_selection = self.selected_ids.first().copied();
            }
        } else {
            self.selected_ids.push(id);
            self.primary_selection = Some(id);
        }
    }

    /// Seçime ekle (mevcut seçimi koruyarak)
    pub fn add_to_selection(&mut self, id: usize) {
        if !self.selected_ids.contains(&id) {
            self.selected_ids.push(id);
            self.primary_selection = Some(id);
        }
    }

    /// Tümünü seç
    pub fn select_all(&mut self, scene: &Scene) {
        self.selected_ids.clear();
        for obj in &scene.objects {
            self.selected_ids.push(obj.id);
        }
        self.primary_selection = self.selected_ids.first().copied();
    }

    /// Seçimi temizle
    pub fn clear_selection(&mut self) {
        self.selected_ids.clear();
        self.primary_selection = None;
    }

    /// Seçili nesne sayısını döndür
    pub fn selection_count(&self) -> usize {
        self.selected_ids.len()
    }

    /// Tek nesne seçili mi?
    pub fn is_single_selection(&self) -> bool {
        self.selected_ids.len() == 1
    }

    /// Birden fazla nesne seçili mi?
    pub fn is_multi_selection(&self) -> bool {
        self.selected_ids.len() > 1
    }

    /// Box selection başlat
    pub fn start_box_selection(&mut self, x: f64, y: f64) {
        self.box_selection = Some(BoxSelection::new(x, y));
        self.mode = SelectionMode::Box;
    }

    /// Box selection güncelle
    pub fn update_box_selection(&mut self, x: f64, y: f64) {
        if let Some(ref mut bs) = self.box_selection {
            bs.end_x = x;
            bs.end_y = y;
        }
    }

    /// Box selection bitir ve içine düşen nesneleri seç
    pub fn finish_box_selection(
        &mut self,
        scene: &Scene,
        camera: &Camera,
        viewport_w: f32,
        viewport_h: f32,
        additive: bool,
    ) -> Vec<usize> {
        let mut selected = Vec::new();

        if let Some(bs) = self.box_selection.take() {
            if bs.area() < 4.0 {
                // Çok küçük box → tek tıklama olarak işle
                self.mode = SelectionMode::Single;
                return selected;
            }

            let (rx, ry, rw, rh) = bs.normalized_rect();

            for obj in &scene.objects {
                if !obj.visible { continue; }

                // 3D → ekran projeksiyonu
                let screen_pos = project_to_screen(
                    camera, obj.transform.position, viewport_w, viewport_h,
                );

                if let Some((sx, sy)) = screen_pos {
                    // Screen space'de box kontrolü
                    if sx >= rx && sx <= rx + rw && sy >= ry && sy <= ry + rh {
                        selected.push(obj.id);
                    }
                }
            }

            if !additive {
                self.selected_ids.clear();
            }
            for id in &selected {
                if !self.selected_ids.contains(id) {
                    self.selected_ids.push(*id);
                }
            }
            self.primary_selection = self.selected_ids.first().copied();
        }

        self.mode = SelectionMode::Single;
        selected
    }

    /// 3D→2D projeksiyon (basit orthographic)
    fn project_to_screen_point(
        cam: &Camera,
        world_pos: Vec3,
        vw: f32,
        vh: f32,
    ) -> Option<(f64, f64)> {
        project_to_screen(cam, world_pos, vw, vh)
    }
}

/// 3D world pozisyonunu screen space'e projekte et
fn project_to_screen(
    cam: &Camera,
    world_pos: Vec3,
    viewport_w: f32,
    viewport_h: f32,
) -> Option<(f64, f64)> {
    let view = cam.view_matrix();
    let aspect = viewport_w / viewport_h;
    let proj = cam.projection_matrix(aspect);
    let vp = proj * view;

    let clip = vp * world_pos.extend(1.0);
    if clip.w.abs() < 0.001 { return None; }

    let ndc = Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);

    // NDC → Screen
    let sx = (ndc.x * 0.5 + 0.5) as f64 * viewport_w as f64;
    let sy = (1.0 - (ndc.y * 0.5 + 0.5)) as f64 * viewport_h as f64;

    Some((sx, sy))
}

// ═══════════════════════════════════════════════════════════ Transform Gizmosu

/// 3D gizmo — eksen okları, döndürme halkaları, ölçek küpleri
pub struct TransformGizmo {
    pub mode: GizmoMode,
    pub highlighted_axis: Option<GizmoAxis>,
    pub position: Vec3,
    pub scale: f32,
    pub is_dragging: bool,
    pub drag_start: Option<(f64, f64)>,
    pub drag_axis: Option<GizmoAxis>,
}

impl TransformGizmo {
    pub fn new() -> Self {
        Self {
            mode: GizmoMode::Translate,
            highlighted_axis: None,
            position: Vec3::ZERO,
            scale: 1.0,
            is_dragging: false,
            drag_start: None,
            drag_axis: None,
        }
    }

    /// Gizmo pozisyonunu güncelle
    pub fn set_position(&mut self, pos: Vec3) {
        self.position = pos;
    }

    /// Fare pozisyonuna göre eksen algılama (2D projeksiyon ile)
    pub fn hit_test(
        &self,
        mouse_x: f64,
        mouse_y: f64,
        camera: &Camera,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<GizmoAxis> {
        let gizmo_screen = project_to_screen(
            camera, self.position, viewport_w, viewport_h,
        )?;
        let (gx, gy) = gizmo_screen;

        let dx = mouse_x - gx;
        let dy = mouse_y - gy;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist > 120.0 * self.scale as f64 {
            return None; // Çok uzak
        }

        // Eksen algılama — hangi eksene en yakınsa
        let threshold = 25.0 * self.scale as f64;

        // Her eksen için tahmini screen-space yönünü hesapla
        let axis_screen_dirs: [(GizmoAxis, f64, f64); 3] = [
            // X ekseni → genellikle sağa
            (GizmoAxis::X, 1.0, 0.0),
            // Y ekseni → genellikle yukarı
            (GizmoAxis::Y, 0.0, -1.0),
            // Z ekseni → derinlik (perspektife bağlı)
            (GizmoAxis::Z, 0.3, -0.3),
        ];

        // Her eksen için tıklama mesafesini hesapla
        let mut best_axis = GizmoAxis::XYZ;
        let mut best_dist = f64::MAX;

        for (axis, adx, ady) in &axis_screen_dirs {
            // Eksen üzerindeki en yakın noktayı bul
            let len = (adx * adx + ady * ady).sqrt();
            if len < 0.001 { continue; }
            let anx = adx / len;
            let any = ady / len;

            // Mouse vektörü
            let mx = dx;
            let my = dy;

            // Eksen üzerindeki projeksiyon
            let proj = mx * anx + my * any;
            let perp_x = mx - proj * anx;
            let perp_y = my - proj * any;
            let perp_dist = (perp_x * perp_x + perp_y * perp_y).sqrt();

            // Sadece eksen yönünde ve belirli mesafede
            if proj > 0.0 && proj < 80.0 * self.scale as f64 && perp_dist < threshold {
                if perp_dist < best_dist {
                    best_dist = perp_dist;
                    best_axis = *axis;
                }
            }
        }

        if best_dist < threshold {
            Some(best_axis)
        } else {
            None
        }
    }

    /// Drag başlat
    pub fn start_drag(&mut self, axis: GizmoAxis, mouse_x: f64, mouse_y: f64) {
        self.is_dragging = true;
        self.drag_start = Some((mouse_x, mouse_y));
        self.drag_axis = Some(axis);
    }

    /// Drag delta hesapla
    pub fn drag_delta(&self, mouse_x: f64, mouse_y: f64) -> Option<(f64, f64)> {
        self.drag_start.map(|(sx, sy)| (mouse_x - sx, mouse_y - sy))
    }

    /// Drag bitir
    pub fn end_drag(&mut self) {
        self.is_dragging = false;
        self.drag_start = None;
        self.drag_axis = None;
    }

    /// Screen delta'yı world-space transform'a çevir
    pub fn screen_delta_to_transform(
        &self,
        delta_x: f64,
        delta_y: f64,
        _camera: &Camera,
        _viewport_w: f32,
        _viewport_h: f32,
    ) -> (Vec3, Vec3, Vec3) {
        let sensitivity = 0.02 * self.scale as f64;

        match self.mode {
            GizmoMode::Translate => {
                let axis = self.drag_axis.unwrap_or(GizmoAxis::XYZ);
                let delta = match axis {
                    GizmoAxis::X => Vec3::new(delta_x as f32 * sensitivity as f32, 0.0, 0.0),
                    GizmoAxis::Y => Vec3::new(0.0, -delta_y as f32 * sensitivity as f32, 0.0),
                    GizmoAxis::Z => Vec3::new(0.0, 0.0, delta_x as f32 * sensitivity as f32),
                    GizmoAxis::XY => Vec3::new(
                        delta_x as f32 * sensitivity as f32,
                        -delta_y as f32 * sensitivity as f32, 0.0),
                    GizmoAxis::XZ => Vec3::new(
                        delta_x as f32 * sensitivity as f32,
                        0.0, delta_y as f32 * sensitivity as f32),
                    GizmoAxis::YZ => Vec3::new(
                        0.0, -delta_y as f32 * sensitivity as f32,
                        delta_x as f32 * sensitivity as f32),
                    GizmoAxis::XYZ => Vec3::new(
                        delta_x as f32 * sensitivity as f32,
                        -delta_y as f32 * sensitivity as f32,
                        delta_x as f32 * sensitivity as f32 * 0.5),
                };
                (delta, Vec3::ZERO, Vec3::ZERO)
            }
            GizmoMode::Rotate => {
                let rot_sensitivity = sensitivity * 30.0;
                let axis = self.drag_axis.unwrap_or(GizmoAxis::Y);
                let angle = (delta_x * rot_sensitivity) as f32;
                let delta_rot = match axis {
                    GizmoAxis::X => Vec3::new(angle, 0.0, 0.0),
                    GizmoAxis::Y => Vec3::new(0.0, angle, 0.0),
                    GizmoAxis::Z => Vec3::new(0.0, 0.0, angle),
                    _ => Vec3::new(angle, -delta_y as f32 * rot_sensitivity as f32, 0.0),
                };
                (Vec3::ZERO, delta_rot, Vec3::ZERO)
            }
            GizmoMode::Scale => {
                let scale_sensitivity = sensitivity * 2.0;
                let axis = self.drag_axis.unwrap_or(GizmoAxis::XYZ);
                let factor = 1.0 + (delta_y as f32 * scale_sensitivity as f32);
                let delta_scale = match axis {
                    GizmoAxis::X => Vec3::new(factor - 1.0, 0.0, 0.0),
                    GizmoAxis::Y => Vec3::new(0.0, factor - 1.0, 0.0),
                    GizmoAxis::Z => Vec3::new(0.0, 0.0, factor - 1.0),
                    _ => Vec3::splat(factor - 1.0),
                };
                (Vec3::ZERO, Vec3::ZERO, delta_scale)
            }
            GizmoMode::None => (Vec3::ZERO, Vec3::ZERO, Vec3::ZERO),
        }
    }
}

// ═══════════════════════════════════════════════════════════ Toplu Transform

/// Seçili nesnelere toplu transform uygula
pub fn apply_multi_transform(
    scene: &mut Scene,
    selected_ids: &[usize],
    delta_position: Vec3,
    delta_rotation: Vec3,
    delta_scale: Vec3,
) {
    for id in selected_ids {
        if let Some(obj) = scene.get_object_mut(*id) {
            obj.transform.position += delta_position;
            obj.transform.rotation += delta_rotation;

            // Ölçek: çarpan olarak uygula
            if delta_scale != Vec3::ZERO {
                obj.transform.scale = Vec3::new(
                    obj.transform.scale.x * (1.0 + delta_scale.x),
                    obj.transform.scale.y * (1.0 + delta_scale.y),
                    obj.transform.scale.z * (1.0 + delta_scale.z),
                );
            }
        }
    }
}

/// Seçili nesneleri sil
pub fn delete_selected(scene: &mut Scene, selected_ids: &[usize]) {
    scene.objects.retain(|o| !selected_ids.contains(&o.id));
}

/// Seçili nesnelerin merkez pozisyonunu hesapla
pub fn selection_center(scene: &Scene, selected_ids: &[usize]) -> Option<Vec3> {
    if selected_ids.is_empty() { return None; }

    let mut center = Vec3::ZERO;
    let mut count = 0;
    for id in selected_ids {
        if let Some(obj) = scene.get_object(*id) {
            center += obj.transform.position;
            count += 1;
        }
    }

    if count > 0 {
        Some(center / count as f32)
    } else {
        None
    }
}

/// Seçili nesnelerin bounding box'ını hesapla
pub fn selection_bounds(
    scene: &Scene,
    selected_ids: &[usize],
) -> Option<(Vec3, Vec3)> {
    if selected_ids.is_empty() { return None; }

    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);

    for id in selected_ids {
        if let Some(obj) = scene.get_object(*id) {
            let pos = obj.transform.position;
            let scale = obj.transform.scale;
            min = Vec3::new(
                min.x.min(pos.x - scale.x),
                min.y.min(pos.y - scale.y),
                min.z.min(pos.z - scale.z),
            );
            max = Vec3::new(
                max.x.max(pos.x + scale.x),
                max.y.max(pos.y + scale.y),
                max.z.max(pos.z + scale.z),
            );
        }
    }

    Some((min, max))
}

// ═══════════════════════════════════════════════════════════ Box Selection Çizimi

/// Box selection dikdörtgenini çiz (2D overlay)
pub fn draw_box_selection(
    fb: &mut crate::renderer::SoftwareRenderer,
    bs: &BoxSelection,
    color: [u8; 4],
) {
    let (x, y, w, h) = bs.normalized_rect();
    let x = x as i32;
    let y = y as i32;
    let w = w as i32;
    let h = h as i32;

    if w < 2 || h < 2 { return; }

    // Doldurma (yarı saydam)
    let fill_color = [color[0], color[1], color[2], 40];
    fb.ui_rect(x, y, w, h, fill_color);

    // Kenar çizgileri (1px)
    fb.ui_rect(x, y, w, 1, color);
    fb.ui_rect(x, y + h - 1, w, 1, color);
    fb.ui_rect(x, y, 1, h, color);
    fb.ui_rect(x + w - 1, y, 1, h, color);
}

// ═══════════════════════════════════════════════════════════ Testler

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_state_single() {
        let mut sel = SelectionState::new();
        sel.select_single(42);
        assert_eq!(sel.selection_count(), 1);
        assert_eq!(sel.primary_selection, Some(42));
        assert!(sel.is_single_selection());
    }

    #[test]
    fn test_selection_toggle() {
        let mut sel = SelectionState::new();
        sel.select_single(1);
        sel.toggle_selection(2);
        assert_eq!(sel.selection_count(), 2);
        sel.toggle_selection(1);
        assert_eq!(sel.selection_count(), 1);
        assert!(!sel.selected_ids.contains(&1));
    }

    #[test]
    fn test_selection_add() {
        let mut sel = SelectionState::new();
        sel.add_to_selection(1);
        sel.add_to_selection(2);
        sel.add_to_selection(3);
        assert_eq!(sel.selection_count(), 3);
        assert!(sel.is_multi_selection());
    }

    #[test]
    fn test_selection_clear() {
        let mut sel = SelectionState::new();
        sel.select_single(1);
        sel.clear_selection();
        assert_eq!(sel.selection_count(), 0);
        assert_eq!(sel.primary_selection, None);
    }

    #[test]
    fn test_box_selection_rect() {
        let mut bs = BoxSelection::new(100.0, 100.0);
        bs.end_x = 200.0;
        bs.end_y = 200.0;
        let (x, y, w, h) = bs.normalized_rect();
        assert_eq!(x, 100.0);
        assert_eq!(y, 100.0);
        assert_eq!(w, 100.0);
        assert_eq!(h, 100.0);
    }

    #[test]
    fn test_box_selection_negative() {
        let mut bs = BoxSelection::new(200.0, 200.0);
        bs.end_x = 100.0;
        bs.end_y = 100.0;
        let (x, y, w, h) = bs.normalized_rect();
        assert_eq!(x, 100.0);
        assert_eq!(w, 100.0);
    }

    #[test]
    fn test_gizmo_mode_cycle() {
        assert_eq!(GizmoMode::Translate.next(), GizmoMode::Rotate);
        assert_eq!(GizmoMode::Rotate.next(), GizmoMode::Scale);
        assert_eq!(GizmoMode::Scale.next(), GizmoMode::Translate);
    }

    #[test]
    fn test_gizmo_axis_colors() {
        // Her eksen farklı renk olmalı
        assert_ne!(GizmoAxis::X.color(), GizmoAxis::Y.color());
        assert_ne!(GizmoAxis::Y.color(), GizmoAxis::Z.color());
        assert_ne!(GizmoAxis::X.color(), GizmoAxis::Z.color());
    }

    #[test]
    fn test_gizmo_drag() {
        let mut gizmo = TransformGizmo::new();
        gizmo.start_drag(GizmoAxis::X, 100.0, 100.0);
        assert!(gizmo.is_dragging);
        assert_eq!(gizmo.drag_axis, Some(GizmoAxis::X));

        let delta = gizmo.drag_delta(150.0, 100.0);
        assert_eq!(delta, Some((50.0, 0.0)));

        gizmo.end_drag();
        assert!(!gizmo.is_dragging);
    }

    #[test]
    fn test_screen_delta_to_translate() {
        let gizmo = TransformGizmo {
            mode: GizmoMode::Translate,
            drag_axis: Some(GizmoAxis::X),
            ..TransformGizmo::new()
        };
        let cam = Camera::new();
        let (pos, _, _) = gizmo.screen_delta_to_transform(100.0, 0.0, &cam, 800.0, 600.0);
        assert!(pos.x > 0.0); // Pozitif X yönünde hareket
        assert_eq!(pos.y, 0.0);
    }

    #[test]
    fn test_screen_delta_to_rotate() {
        let gizmo = TransformGizmo {
            mode: GizmoMode::Rotate,
            drag_axis: Some(GizmoAxis::Y),
            ..TransformGizmo::new()
        };
        let cam = Camera::new();
        let (_, rot, _) = gizmo.screen_delta_to_transform(100.0, 0.0, &cam, 800.0, 600.0);
        assert!(rot.y != 0.0); // Y rotasyonu değişmeli
    }

    #[test]
    fn test_multi_transform() {
        let mut scene = Scene::default();
        let id1 = scene.add_object("A".into(), GeometryType::Cube, EntityTeam::Neutral);
        let id2 = scene.add_object("B".into(), GeometryType::Cube, EntityTeam::Neutral);

        scene.get_object_mut(id1).unwrap().transform.position = Vec3::ZERO;
        scene.get_object_mut(id2).unwrap().transform.position = Vec3::new(5.0, 0.0, 0.0);

        apply_multi_transform(&mut scene, &[id1, id2], Vec3::Y, Vec3::ZERO, Vec3::ZERO);

        let p1 = scene.get_object(id1).unwrap().transform.position;
        let p2 = scene.get_object(id2).unwrap().transform.position;
        assert!((p1.y - 1.0).abs() < 0.01);
        assert!((p2.y - 1.0).abs() < 0.01);
        assert!((p2.x - 5.0).abs() < 0.01); // X korundu
    }

    #[test]
    fn test_selection_center() {
        let mut scene = Scene::default();
        let id1 = scene.add_object("A".into(), GeometryType::Cube, EntityTeam::Neutral);
        let id2 = scene.add_object("B".into(), GeometryType::Cube, EntityTeam::Neutral);

        scene.get_object_mut(id1).unwrap().transform.position = Vec3::new(2.0, 0.0, 0.0);
        scene.get_object_mut(id2).unwrap().transform.position = Vec3::new(6.0, 0.0, 0.0);

        let center = selection_center(&scene, &[id1, id2]);
        assert!(center.is_some());
        let c = center.unwrap();
        assert!((c.x - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_selection_bounds() {
        let mut scene = Scene::default();
        let id1 = scene.add_object("A".into(), GeometryType::Cube, EntityTeam::Neutral);
        let id2 = scene.add_object("B".into(), GeometryType::Cube, EntityTeam::Neutral);

        scene.get_object_mut(id1).unwrap().transform.position = Vec3::new(-2.0, -1.0, 0.0);
        scene.get_object_mut(id2).unwrap().transform.position = Vec3::new(2.0, 1.0, 0.0);

        let bounds = selection_bounds(&scene, &[id1, id2]);
        assert!(bounds.is_some());
        let (min, max) = bounds.unwrap();
        assert!(min.x < max.x);
        assert!(min.y < max.y);
    }

    #[test]
    fn test_delete_selected() {
        let mut scene = Scene::default();
        let id1 = scene.add_object("A".into(), GeometryType::Cube, EntityTeam::Neutral);
        let _id2 = scene.add_object("B".into(), GeometryType::Cube, EntityTeam::Neutral);

        delete_selected(&mut scene, &[id1]);
        assert_eq!(scene.objects.len(), 1);
        assert_eq!(scene.objects[0].name, "B");
    }
}
