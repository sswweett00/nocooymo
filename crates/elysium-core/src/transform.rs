//! Transform component: translation, rotation, scale, and world matrix computation.

use serde::{Deserialize, Serialize};

use crate::math::{Mat3, Mat4, Quat, Vec3};

/// A transform representing translation, rotation, and scale.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    /// Cached local matrix.
    local_matrix: Mat4,
    /// Cached world matrix.
    world_matrix: Mat4,
    /// Whether the local matrix needs recomputation.
    dirty: bool,
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            local_matrix: Mat4::IDENTITY,
            world_matrix: Mat4::IDENTITY,
            dirty: false,
        }
    }

    pub fn from_translation(translation: Vec3) -> Self {
        let mut t = Self {
            translation,
            ..Self::identity()
        };
        t.dirty = true;
        t
    }

    pub fn from_rotation(rotation: Quat) -> Self {
        let mut t = Self {
            rotation,
            ..Self::identity()
        };
        t.dirty = true;
        t
    }

    pub fn from_scale(scale: Vec3) -> Self {
        let mut t = Self {
            scale,
            ..Self::identity()
        };
        t.dirty = true;
        t
    }

    pub fn from_trs(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        let mut t = Self {
            translation,
            rotation,
            scale,
            ..Self::identity()
        };
        t.update_local_matrix();
        t
    }

    // -----------------------------------------------------------------------
    // Mutators (mark dirty)
    // -----------------------------------------------------------------------

    pub fn set_translation(&mut self, translation: Vec3) -> &mut Self {
        self.translation = translation;
        self.dirty = true;
        self
    }

    pub fn set_rotation(&mut self, rotation: Quat) -> &mut Self {
        self.rotation = rotation;
        self.dirty = true;
        self
    }

    pub fn set_scale(&mut self, scale: Vec3) -> &mut Self {
        self.scale = scale;
        self.dirty = true;
        self
    }

    pub fn translate(&mut self, delta: Vec3) -> &mut Self {
        self.translation += delta;
        self.dirty = true;
        self
    }

    pub fn rotate(&mut self, rotation: Quat) -> &mut Self {
        self.rotation = rotation * self.rotation;
        self.dirty = true;
        self
    }

    pub fn rotate_axis_angle(&mut self, axis: Vec3, angle: f32) -> &mut Self {
        let q = Quat::from_axis_angle(axis.normalize_or_zero(), angle);
        self.rotate(q)
    }

    pub fn scale_by(&mut self, factor: Vec3) -> &mut Self {
        self.scale *= factor;
        self.dirty = true;
        self
    }

    // -----------------------------------------------------------------------
    // Matrix computation
    // -----------------------------------------------------------------------

    /// Recompute the local matrix from TRS.
    pub fn update_local_matrix(&mut self) {
        self.local_matrix = Mat4::from_translation(self.translation)
            * Mat4::from_quat(self.rotation)
            * Mat4::from_scale(self.scale);
        self.dirty = false;
    }

    /// Get the local matrix (recomputes if dirty).
    pub fn local_matrix(&mut self) -> Mat4 {
        if self.dirty {
            self.update_local_matrix();
        }
        self.local_matrix
    }

    /// Set the world matrix (called by the hierarchy system).
    pub fn set_world_matrix(&mut self, matrix: Mat4) {
        self.world_matrix = matrix;
    }

    /// Get the cached world matrix.
    pub fn world_matrix(&self) -> Mat4 {
        self.world_matrix
    }

    /// Get the world matrix without mutation (assumes it's up to date).
    pub fn world_matrix_ref(&self) -> &Mat4 {
        &self.world_matrix
    }

    /// Is the local matrix dirty?
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    // -----------------------------------------------------------------------
    // Utility
    // -----------------------------------------------------------------------

    /// Get the forward direction (-Z) in local space.
    pub fn forward(&self) -> Vec3 {
        self.rotation * -Vec3::Z
    }

    /// Get the right direction (+X) in local space.
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    /// Get the up direction (+Y) in local space.
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    /// Transform a point from local to world space.
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        self.world_matrix.transform_point3(point)
    }

    /// Transform a direction from local to world space.
    pub fn transform_direction(&self, dir: Vec3) -> Vec3 {
        self.world_matrix.transform_vector3(dir)
    }

    /// Lerp between two transforms.
    pub fn lerp(&self, other: &Transform, t: f32) -> Transform {
        Transform {
            translation: self.translation.lerp(other.translation, t),
            rotation: self.rotation.slerp(other.rotation, t),
            scale: self.scale.lerp(other.scale, t),
            local_matrix: Mat4::IDENTITY,
            world_matrix: Mat4::IDENTITY,
            dirty: true,
        }
    }

    /// Decompose a matrix into TRS.
    pub fn from_matrix(matrix: Mat4) -> Self {
        let translation = matrix.w_axis.truncate();
        let scale = Vec3::new(
            matrix.x_axis.truncate().length(),
            matrix.y_axis.truncate().length(),
            matrix.z_axis.truncate().length(),
        );
        let rotation = Quat::from_mat3(
            &(Mat3::from_mat4(matrix) * Mat3::from_diagonal(
                Vec3::new(
                    1.0 / scale.x.max(f32::EPSILON),
                    1.0 / scale.y.max(f32::EPSILON),
                    1.0 / scale.z.max(f32::EPSILON),
                ),
            )),
        );
        Self::from_trs(translation, rotation, scale)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_transform() {
        let t = Transform::identity();
        assert_eq!(t.translation, Vec3::ZERO);
        assert_eq!(t.rotation, Quat::IDENTITY);
        assert_eq!(t.scale, Vec3::ONE);
    }

    #[test]
    fn local_matrix_trs() {
        let mut t = Transform::from_trs(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::from_rotation_y(1.5708), // ~90°
            Vec3::new(2.0, 2.0, 2.0),
        );
        let m = t.local_matrix();
        // Check translation
        assert_eq!(m.w_axis.truncate(), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn forward_direction() {
        let t = Transform::from_rotation(Quat::from_rotation_y(0.0));
        let f = t.forward();
        assert!((f + Vec3::Z).length() < 0.01);
    }

    #[test]
    fn rotate_90_y() {
        let mut t = Transform::identity();
        t.rotate(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        let f = t.forward();
        // After 90° Y rotation, forward (-Z) should point in -X direction (right-handed)
        assert!((f + Vec3::X).length() < 0.01);
    }

    #[test]
    fn lerp_transforms() {
        let t1 = Transform::from_translation(Vec3::ZERO);
        let t2 = Transform::from_translation(Vec3::new(10.0, 0.0, 0.0));
        let mid = t1.lerp(&t2, 0.5);
        assert!((mid.translation - Vec3::new(5.0, 0.0, 0.0)).length() < 0.01);
    }
}