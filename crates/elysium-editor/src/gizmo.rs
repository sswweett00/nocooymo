use elysium_core::{Transform, Entity};
use glam::{Vec3, Quat, Mat4};

pub struct Gizmo {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub gizmo_type: GizmoType,
    pub visibility: bool,
    pub highlight_axis: Option<GizmoAxis>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GizmoType {
    Translate,
    Rotate,
    Scale,
    Transform, // Combined gizmo
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GizmoAxis {
    X,
    Y,
    Z,
    XY,
    XZ,
    YZ,
    XYZ,
}

impl Gizmo {
    pub fn new(gizmo_type: GizmoType) -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            gizmo_type,
            visibility: true,
            highlight_axis: None,
        }
    }

    pub fn update_position(&mut self, new_position: Vec3) {
        self.position = new_position;
    }

    pub fn update_rotation(&mut self, new_rotation: Quat) {
        self.rotation = new_rotation;
    }

    pub fn update_transform(&mut self, transform: &Transform) {
        self.position = Vec3::new(transform.position.x, transform.position.y, transform.position.z);
        // Note: Transform in elysium_core might not have rotation as quat
        // This is a simplification
        self.scale = Vec3::new(transform.scale.x, transform.scale.y, transform.scale.z);
    }

    pub fn get_world_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            self.scale,
            self.rotation,
            self.position
        )
    }

    pub fn raycast(&self, ray_origin: Vec3, ray_direction: Vec3) -> Option<(GizmoAxis, f32)> {
        match self.gizmo_type {
            GizmoType::Translate => self.raycast_translate_gizmo(ray_origin, ray_direction),
            GizmoType::Rotate => self.raycast_rotate_gizmo(ray_origin, ray_direction),
            GizmoType::Scale => self.raycast_scale_gizmo(ray_origin, ray_direction),
            GizmoType::Transform => self.raycast_combined_gizmo(ray_origin, ray_direction),
        }
    }

    fn raycast_translate_gizmo(&self, ray_origin: Vec3, ray_direction: Vec3) -> Option<(GizmoAxis, f32)> {
        // Simplified raycasting for translate gizmo
        // In a real implementation, this would cast rays against actual gizmo geometry
        
        // Check X axis (red line)
        if let Some(dist) = self.intersect_ray_with_axis(ray_origin, ray_direction, Vec3::X, 0.1) {
            return Some((GizmoAxis::X, dist));
        }
        
        // Check Y axis (green line)
        if let Some(dist) = self.intersect_ray_with_axis(ray_origin, ray_direction, Vec3::Y, 0.1) {
            return Some((GizmoAxis::Y, dist));
        }
        
        // Check Z axis (blue line)
        if let Some(dist) = self.intersect_ray_with_axis(ray_origin, ray_direction, Vec3::Z, 0.1) {
            return Some((GizmoAxis::Z, dist));
        }
        
        None
    }

    fn raycast_rotate_gizmo(&self, ray_origin: Vec3, ray_direction: Vec3) -> Option<(GizmoAxis, f32)> {
        // Simplified raycasting for rotate gizmo
        // In a real implementation, this would cast rays against circular arcs
        
        // For now, just check if ray hits the general area of each axis circle
        let dist_x = (ray_origin - self.position).distance(Vec3::X * 1.0);
        let dist_y = (ray_origin - self.position).distance(Vec3::Y * 1.0);
        let dist_z = (ray_origin - self.position).distance(Vec3::Z * 1.0);
        
        let tolerance = 0.2;
        
        if dist_x < tolerance {
            Some((GizmoAxis::X, dist_x))
        } else if dist_y < tolerance {
            Some((GizmoAxis::Y, dist_y))
        } else if dist_z < tolerance {
            Some((GizmoAxis::Z, dist_z))
        } else {
            None
        }
    }

    fn raycast_scale_gizmo(&self, ray_origin: Vec3, ray_direction: Vec3) -> Option<(GizmoAxis, f32)> {
        // Similar to translate gizmo but with different visual representation
        self.raycast_translate_gizmo(ray_origin, ray_direction)
    }

    fn raycast_combined_gizmo(&self, ray_origin: Vec3, ray_direction: Vec3) -> Option<(GizmoAxis, f32)> {
        // Try to hit any of the gizmo components
        if let hit = self.raycast_translate_gizmo(ray_origin, ray_direction) {
            return hit;
        }
        
        if let hit = self.raycast_rotate_gizmo(ray_origin, ray_direction) {
            return hit;
        }
        
        if let hit = self.raycast_scale_gizmo(ray_origin, ray_direction) {
            return hit;
        }
        
        None
    }

    fn intersect_ray_with_axis(&self, ray_origin: Vec3, ray_direction: Vec3, axis: Vec3, thickness: f32) -> Option<f32> {
        // Simple line-ray intersection test
        let axis_start = self.position;
        let axis_end = self.position + axis * 1.0; // 1.0 unit length
        
        // Vector from ray origin to axis start
        let w = ray_origin - axis_start;
        let axis_dir = axis_end - axis_start;
        
        // Calculate intersection using parametric form
        let ray_cross_axis = ray_direction.cross(axis_dir);
        let denominator = ray_cross_axis.length_squared();
        
        if denominator < 1e-6 {
            // Lines are parallel, check if they're close enough
            let p1 = ray_origin;
            let p2 = ray_origin + ray_direction * 1000.0; // Extend ray
            
            // Distance from point to line segment
            let dist = self.point_to_line_distance(p1, p2, axis_start, axis_end);
            if dist < thickness {
                // Return approximate distance along ray
                Some((axis_start - ray_origin).length())
            } else {
                None
            }
        } else {
            // Not parallel, calculate closest points
            let w_cross_axis = w.cross(axis_dir);
            let c1 = w_cross_axis.length_squared() / denominator;
            let c2 = ray_cross_axis.length_squared() / denominator;
            
            // Check if intersection point is on the axis line segment
            if c2 >= 0.0 && c2 <= 1.0 {
                Some(c1)
            } else {
                None
            }
        }
    }

    fn point_to_line_distance(&self, ray_start: Vec3, ray_end: Vec3, line_start: Vec3, line_end: Vec3) -> f32 {
        // Calculate minimum distance between two line segments
        let r = ray_end - ray_start;
        let s = line_end - line_start;
        let w = ray_start - line_start;
        
        let r_dot_r = r.dot(r);
        let s_dot_s = s.dot(s);
        let r_dot_s = r.dot(s);
        
        let denominator = r_dot_r * s_dot_s - r_dot_s * r_dot_s;
        
        if denominator < 1e-6 {
            // Lines are parallel
            let t = w.dot(s) / s_dot_s;
            let projection = line_start + s * t.clamp(0.0, 1.0);
            return (projection - ray_start).length();
        }
        
        let t = (w.dot(s) * r_dot_s - w.dot(r) * s_dot_s) / denominator;
        let u = (w.dot(r) * r_dot_s - w.dot(s) * r_dot_r) / denominator;
        
        let clamped_t = t.clamp(0.0, 1.0);
        let clamped_u = u.clamp(0.0, 1.0);
        
        let point1 = ray_start + r * clamped_t;
        let point2 = line_start + s * clamped_u;
        
        (point1 - point2).length()
    }

    pub fn apply_transform(&self, original_transform: &Transform, delta: Vec3) -> Transform {
        let mut new_transform = original_transform.clone();
        
        match self.highlight_axis {
            Some(GizmoAxis::X) => new_transform.position.x += delta.x,
            Some(GizmoAxis::Y) => new_transform.position.y += delta.y,
            Some(GizmoAxis::Z) => new_transform.position.z += delta.z,
            Some(GizmoAxis::XY) => {
                new_transform.position.x += delta.x;
                new_transform.position.y += delta.y;
            },
            Some(GizmoAxis::XZ) => {
                new_transform.position.x += delta.x;
                new_transform.position.z += delta.z;
            },
            Some(GizmoAxis::YZ) => {
                new_transform.position.y += delta.y;
                new_transform.position.z += delta.z;
            },
            Some(GizmoAxis::XYZ) => {
                new_transform.position += glam::Vec3::new(delta.x, delta.y, delta.z);
            },
            None => {}, // No axis selected
        }
        
        new_transform
    }
}

pub struct GizmoSystem {
    pub active_gizmo: Option<Gizmo>,
    pub gizmo_size: f32,
}

impl GizmoSystem {
    pub fn new() -> Self {
        Self {
            active_gizmo: None,
            gizmo_size: 1.0,
        }
    }

    pub fn set_active_gizmo(&mut self, gizmo_type: GizmoType) {
        self.active_gizmo = Some(Gizmo::new(gizmo_type));
    }

    pub fn update_gizmo_for_entity(&mut self, entity_transform: &Transform) {
        if let Some(ref mut gizmo) = self.active_gizmo {
            gizmo.update_transform(entity_transform);
        }
    }

    pub fn handle_mouse_input(&mut self, mouse_pos: (f32, f32), viewport_size: (f32, f32)) -> Option<Vec3> {
        // This would handle actual mouse input for gizmo manipulation
        // For now, returning None
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gizmo_creation() {
        let gizmo = Gizmo::new(GizmoType::Translate);
        assert_eq!(gizmo.gizmo_type, GizmoType::Translate);
        assert_eq!(gizmo.position, Vec3::ZERO);
    }

    #[test]
    fn test_gizmo_axis_enum() {
        let axis = GizmoAxis::X;
        assert!(matches!(axis, GizmoAxis::X));
    }
}