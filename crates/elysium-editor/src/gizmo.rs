//! Dönüşüm gizmosu — öteleme, döndürme, ölçekleme araçları.

/// Gizmo eksen türü
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GizmoAxis {
    X, Y, Z,
    XY, XZ, YZ,
    XYZ,
}

/// Gizmo türü
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GizmoType {
    Translate,
    Rotate,
    Scale,
}

/// Basit 3D gizmo
pub struct Gizmo {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub gizmo_type: GizmoType,
    pub visible: bool,
    pub highlighted_axis: Option<GizmoAxis>,
    pub size: f32,
}

impl Gizmo {
    pub fn new(gizmo_type: GizmoType) -> Self {
        Self {
            position: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
            gizmo_type,
            visible: true,
            highlighted_axis: None,
            size: 1.0,
        }
    }

    pub fn set_position(&mut self, x: f32, y: f32, z: f32) {
        self.position = [x, y, z];
    }

    /// Fare girdisine göre eksen seç
    pub fn hit_test(&self, mouse_x: f32, mouse_y: f32, viewport_width: f32, viewport_height: f32) -> Option<GizmoAxis> {
        // Basit: hangi eksen en yakınsa onu seç
        let screen_x = (mouse_x / viewport_width - 0.5) * 2.0;
        let screen_y = (mouse_y / viewport_height - 0.5) * -2.0;

        let dx = screen_x.abs();
        let dy = screen_y.abs();

        if dx > dy * 2.0 {
            if screen_x > 0.0 { Some(GizmoAxis::X) } else { Some(GizmoAxis::X) }
        } else if dy > dx * 2.0 {
            if screen_y > 0.0 { Some(GizmoAxis::Y) } else { Some(GizmoAxis::Y) }
        } else {
            Some(GizmoAxis::XY)
        }
    }

    /// Gizmo'yu uygula — transform'a delta ekle
    pub fn apply_delta(&self, axis: GizmoAxis, delta_x: f32, delta_y: f32) -> [f32; 3] {
        match self.gizmo_type {
            GizmoType::Translate => {
                match axis {
                    GizmoAxis::X => [delta_x, 0.0, 0.0],
                    GizmoAxis::Y => [0.0, delta_y, 0.0],
                    GizmoAxis::Z => [0.0, 0.0, delta_x],
                    GizmoAxis::XY => [delta_x, delta_y, 0.0],
                    GizmoAxis::XZ => [delta_x, 0.0, delta_y],
                    GizmoAxis::YZ => [0.0, delta_x, delta_y],
                    GizmoAxis::XYZ => [delta_x, delta_y, delta_x],
                }
            }
            GizmoType::Rotate => {
                match axis {
                    GizmoAxis::X => [delta_y * 45.0, 0.0, 0.0],
                    GizmoAxis::Y => [0.0, delta_x * 45.0, 0.0],
                    GizmoAxis::Z => [0.0, 0.0, delta_x * 45.0],
                    _ => [delta_x * 30.0, delta_y * 30.0, 0.0],
                }
            }
            GizmoType::Scale => {
                let s = 1.0 + delta_x * 0.1;
                match axis {
                    GizmoAxis::X => [s, 1.0, 1.0],
                    GizmoAxis::Y => [1.0, s, 1.0],
                    GizmoAxis::Z => [1.0, 1.0, s],
                    GizmoAxis::XYZ => [s, s, s],
                    _ => [s, s, s],
                }
            }
        }
    }
}

/// Gizmo sistemi
pub struct GizmoSystem {
    pub active_gizmo: Option<Gizmo>,
    pub gizmo_size: f32,
    pub snap_enabled: bool,
    pub snap_increment: f32,
}

impl Default for GizmoSystem {
    fn default() -> Self { Self::new() }
}

impl GizmoSystem {
    pub fn new() -> Self {
        Self { active_gizmo: None, gizmo_size: 1.0, snap_enabled: false, snap_increment: 0.25 }
    }

    pub fn set_type(&mut self, gizmo_type: GizmoType) {
        self.active_gizmo = Some(Gizmo::new(gizmo_type));
    }

    pub fn update_position(&mut self, x: f32, y: f32, z: f32) {
        if let Some(ref mut g) = self.active_gizmo {
            g.set_position(x, y, z);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gizmo_creation() {
        let g = Gizmo::new(GizmoType::Translate);
        assert_eq!(g.gizmo_type, GizmoType::Translate);
        assert_eq!(g.position, [0.0; 3]);
    }

    #[test]
    fn test_gizmo_apply_delta() {
        let g = Gizmo::new(GizmoType::Translate);
        let delta = g.apply_delta(GizmoAxis::X, 1.0, 0.0);
        assert_eq!(delta, [1.0, 0.0, 0.0]);
    }
}
