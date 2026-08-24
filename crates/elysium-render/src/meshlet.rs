use elysium_core::math::{Vec3, Vec4};
use glam::Vec4Swizzles;

// Constants for meshlet configuration
pub const MAX_VERTICES_PER_MESHLET: usize = 64;
pub const MAX_TRIANGLES_PER_MESHLET: usize = 128;
pub const VERTEX_COUNT_BITMASK_SIZE: usize = (MAX_VERTICES_PER_MESHLET + 31) / 32;

#[derive(Debug, Clone)]
pub struct Meshlet {
    pub vertex_indices: [u32; MAX_VERTICES_PER_MESHLET],
    pub triangle_indices: [u8; MAX_TRIANGLES_PER_MESHLET * 3],
    pub vertex_count: u8,
    pub triangle_count: u8,
    pub bounds_center: Vec3,
    pub bounds_radius: f32,
    pub normal_cone_axis: Vec3,
    pub normal_cone_angle_sin: f32,
    pub normal_cone_angle_cos: f32,
}

impl Meshlet {
    pub fn new(vertex_indices: Vec<u32>, triangle_indices: Vec<u8>) -> Self {
        let mut meshlet = Self {
            vertex_indices: [0; MAX_VERTICES_PER_MESHLET],
            triangle_indices: [0; MAX_TRIANGLES_PER_MESHLET * 3],
            vertex_count: 0,
            triangle_count: 0,
            bounds_center: Vec3::ZERO,
            bounds_radius: 0.0,
            normal_cone_axis: Vec3::Y,
            normal_cone_angle_sin: 0.0,
            normal_cone_angle_cos: 1.0,
        };
        
        // Copy vertex indices
        let vertex_count = vertex_indices.len().min(MAX_VERTICES_PER_MESHLET);
        meshlet.vertex_indices[..vertex_count].copy_from_slice(&vertex_indices[..vertex_count]);
        meshlet.vertex_count = vertex_count as u8;
        
        // Copy triangle indices
        let triangle_count = triangle_indices.len().min(MAX_TRIANGLES_PER_MESHLET * 3);
        meshlet.triangle_indices[..triangle_count].copy_from_slice(&triangle_indices[..triangle_count]);
        meshlet.triangle_count = (triangle_count / 3) as u8;
        
        // Calculate bounds and normal cone
        meshlet.calculate_bounds();
        meshlet.calculate_normal_cone();
        
        meshlet
    }
    
    fn calculate_bounds(&mut self) {
        // This would typically be calculated from actual vertex positions
        // For now, using a placeholder
        self.bounds_center = Vec3::ZERO;
        self.bounds_radius = 1.0;
    }
    
    fn calculate_normal_cone(&mut self) {
        // This would typically be calculated from triangle normals
        // For now, using a placeholder
        self.normal_cone_axis = Vec3::Y;
        self.normal_cone_angle_sin = 0.0;
        self.normal_cone_angle_cos = 1.0;
    }
    
    pub fn get_triangle(&self, index: u8) -> Option<[u8; 3]> {
        if index >= self.triangle_count {
            None
        } else {
            let i = index as usize * 3;
            Some([
                self.triangle_indices[i],
                self.triangle_indices[i + 1],
                self.triangle_indices[i + 2]
            ])
        }
    }
}

#[derive(Debug, Clone)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

impl BoundingSphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }
    
    pub fn contains_point(&self, point: Vec3) -> bool {
        (point - self.center).length_squared() <= self.radius * self.radius
    }
    
    pub fn intersects_sphere(&self, other: &BoundingSphere) -> bool {
        let distance = (self.center - other.center).length();
        distance <= (self.radius + other.radius)
    }
}

#[derive(Debug, Clone)]
pub struct NormalCone {
    pub axis: Vec3,
    pub angle_sin: f32,
    pub angle_cos: f32,
}

impl NormalCone {
    pub fn new(axis: Vec3, angle: f32) -> Self {
        Self {
            axis: axis.normalize(),
            angle_sin: angle.sin(),
            angle_cos: angle.cos(),
        }
    }
    
    pub fn contains_direction(&self, direction: Vec3) -> bool {
        let normalized_dir = direction.normalize();
        normalized_dir.dot(self.axis) >= self.angle_cos
    }
}

pub struct MeshletBuilder {
    pub max_vertices_per_meshlet: usize,
    pub max_triangles_per_meshlet: usize,
}

impl MeshletBuilder {
    pub fn new() -> Self {
        Self {
            max_vertices_per_meshlet: MAX_VERTICES_PER_MESHLET,
            max_triangles_per_meshlet: MAX_TRIANGLES_PER_MESHLET,
        }
    }
    
    pub fn build_meshlets(
        &self,
        _vertices: &[f32],
        indices: &[u32],
    ) -> Vec<Meshlet> {
        let mut meshlets = Vec::new();
        let mut current_meshlet_vertices = Vec::new();
        let mut current_meshlet_triangles = Vec::new();
        
        for triangle_idx in (0..indices.len()).step_by(3) {
            let tri_indices = [
                indices[triangle_idx],
                indices[triangle_idx + 1],
                indices[triangle_idx + 2],
            ];
            
            // Check if adding this triangle would exceed limits
            let new_vertices_needed = self.count_new_vertices(&current_meshlet_vertices, &tri_indices);
            
            if current_meshlet_vertices.len() + new_vertices_needed > self.max_vertices_per_meshlet ||
               current_meshlet_triangles.len() / 3 >= self.max_triangles_per_meshlet {
                // Finalize current meshlet and start a new one
                if !current_meshlet_vertices.is_empty() && !current_meshlet_triangles.is_empty() {
                    let meshlet = self.create_meshlet(&current_meshlet_vertices, &current_meshlet_triangles);
                    meshlets.push(meshlet);
                }
                
                current_meshlet_vertices.clear();
                current_meshlet_triangles.clear();
            }
            
            // Add triangle to current meshlet
            let _base_vertex = current_meshlet_vertices.len() as u8;
            for &idx in &tri_indices {
                let local_idx = self.add_vertex_if_new(&mut current_meshlet_vertices, idx) as u8;
                current_meshlet_triangles.push(local_idx);
            }
        }
        
        // Add final meshlet if it has content
        if !current_meshlet_vertices.is_empty() && !current_meshlet_triangles.is_empty() {
            let meshlet = self.create_meshlet(&current_meshlet_vertices, &current_meshlet_triangles);
            meshlets.push(meshlet);
        }
        
        meshlets
    }
    
    fn add_vertex_if_new(&self, meshlet_vertices: &mut Vec<u32>, global_idx: u32) -> u32 {
        match meshlet_vertices.iter().position(|&v| v == global_idx) {
            Some(local_idx) => local_idx as u32,
            None => {
                meshlet_vertices.push(global_idx);
                (meshlet_vertices.len() - 1) as u32
            }
        }
    }
    
    fn count_new_vertices(&self, meshlet_vertices: &[u32], new_triangle: &[u32]) -> usize {
        new_triangle.iter().filter(|&&idx| !meshlet_vertices.contains(&idx)).count()
    }
    
    fn create_meshlet(&self, vertex_indices: &[u32], triangle_indices: &[u8]) -> Meshlet {
        Meshlet::new(vertex_indices.to_vec(), triangle_indices.to_vec())
    }
}

pub struct MeshletCulling {
    pub frustum_planes: [Vec4; 6],
    pub occlusion_culling_enabled: bool,
}

impl MeshletCulling {
    pub fn new() -> Self {
        Self {
            frustum_planes: [Vec4::ZERO; 6],
            occlusion_culling_enabled: false,
        }
    }
    
    pub fn set_frustum_planes(&mut self, planes: [Vec4; 6]) {
        self.frustum_planes = planes;
    }
    
    pub fn cull_meshlets(&self, meshlets: &[Meshlet], camera_position: Vec3) -> Vec<usize> {
        let mut visible_meshlets = Vec::new();
        
        for (idx, meshlet) in meshlets.iter().enumerate() {
            if self.is_meshlet_visible(meshlet, camera_position) {
                visible_meshlets.push(idx);
            }
        }
        
        visible_meshlets
    }
    
    fn is_meshlet_visible(&self, meshlet: &Meshlet, camera_position: Vec3) -> bool {
        // First, check if the bounding sphere is within view
        if !self.is_sphere_in_frustum(meshlet) {
            return false;
        }
        
        // Check if the meshlet is backfacing based on the normal cone
        if self.is_backfacing(meshlet, camera_position) {
            return false;
        }
        
        // Additional culling techniques could be added here:
        // - Distance-based culling
        // - Hierarchical culling using BVH
        // - Occlusion culling
        
        true
    }
    
    fn is_sphere_in_frustum(&self, meshlet: &Meshlet) -> bool {
        let sphere_center = meshlet.bounds_center;
        let sphere_radius = meshlet.bounds_radius;
        
        for plane in &self.frustum_planes {
            let distance = plane.xyz().dot(sphere_center) + plane.w;
            if distance < -sphere_radius {
                return false; // Sphere is completely behind this plane
            }
        }
        
        true // Sphere is in front of all planes
    }
    
    fn is_backfacing(&self, meshlet: &Meshlet, camera_position: Vec3) -> bool {
        // Calculate the direction from the meshlet to the camera
        let to_camera = (camera_position - meshlet.bounds_center).normalize();
        
        // Check if the camera is outside the normal cone
        to_camera.dot(meshlet.normal_cone_axis) < meshlet.normal_cone_angle_cos
    }
}

// Helper function to calculate bounding sphere for a meshlet
fn calculate_bounding_sphere(_vertex_indices: &[u32]) -> BoundingSphere {
    // This would typically be calculated from actual vertex positions
    // For now, returning a placeholder
    BoundingSphere::new(Vec3::ZERO, 1.0)
}

// Helper function to calculate normal cone for a meshlet
fn calculate_normal_cone(_triangle_indices: &[u8]) -> NormalCone {
    // This would typically be calculated from triangle normals
    // For now, returning a placeholder
    NormalCone::new(Vec3::Y, std::f32::consts::PI / 2.0)
}

// Function to calculate triangle normal
pub fn calculate_triangle_normal(v0: Vec3, v1: Vec3, v2: Vec3) -> Vec3 {
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    edge1.cross(edge2).normalize()
}

// Function to calculate meshlet normal cone from triangle normals
pub fn calculate_meshlet_normal_cone(triangle_normals: &[Vec3]) -> NormalCone {
    if triangle_normals.is_empty() {
        return NormalCone::new(Vec3::Y, std::f32::consts::PI / 2.0);
    }
    
    // Calculate average normal
    let mut average_normal = Vec3::ZERO;
    for normal in triangle_normals {
        average_normal += *normal;
    }
    average_normal = average_normal.normalize();
    
    // Calculate maximum angle deviation from average normal
    let mut max_angle: f32 = 0.0;
    for normal in triangle_normals {
        let dot = average_normal.dot(*normal).clamp(-1.0, 1.0);
        let angle = dot.acos();
        max_angle = max_angle.max(angle);
    }
    
    // Add a small margin for safety
    let angle_with_margin = (max_angle + 0.1).min(std::f32::consts::PI / 2.0);
    
    NormalCone::new(average_normal, angle_with_margin)
}

/// BVH (Bounding Volume Hierarchy) Node for meshlet culling
#[derive(Debug, Clone)]
pub struct BvhNode {
    pub bounds: BoundingSphere,
    pub left_child: Option<usize>,
    pub right_child: Option<usize>,
    pub meshlet_indices: Vec<usize>,
    pub is_leaf: bool,
    pub depth: u8,
}

impl BvhNode {
    pub fn new_leaf(meshlet_indices: Vec<usize>, bounds: BoundingSphere, depth: u8) -> Self {
        Self {
            bounds,
            left_child: None,
            right_child: None,
            meshlet_indices,
            is_leaf: true,
            depth,
        }
    }

    pub fn new_internal(left: usize, right: usize, bounds: BoundingSphere, depth: u8) -> Self {
        Self {
            bounds,
            left_child: Some(left),
            right_child: Some(right),
            meshlet_indices: Vec::new(),
            is_leaf: false,
            depth,
        }
    }
}

/// BVH Builder for constructing meshlet hierarchy
pub struct BvhBuilder {
    max_meshlets_per_leaf: usize,
    max_depth: u8,
}

impl BvhBuilder {
    pub fn new(max_meshlets_per_leaf: usize, max_depth: u8) -> Self {
        Self {
            max_meshlets_per_leaf,
            max_depth,
        }
    }

    pub fn build(&self, meshlets: &[Meshlet]) -> Vec<BvhNode> {
        if meshlets.is_empty() {
            return Vec::new();
        }

        let mut nodes = Vec::new();
        let meshlet_indices: Vec<usize> = (0..meshlets.len()).collect();
        
        self.build_recursive(&mut nodes, meshlets, &meshlet_indices, 0);
        nodes
    }

    fn build_recursive(
        &self,
        nodes: &mut Vec<BvhNode>,
        meshlets: &[Meshlet],
        indices: &[usize],
        depth: u8,
    ) -> usize {
        // Calculate bounds for this node
        let bounds = self.calculate_bounds(meshlets, indices);
        
        // Check if we should create a leaf node
        if indices.len() <= self.max_meshlets_per_leaf || depth >= self.max_depth {
            let node = BvhNode::new_leaf(indices.to_vec(), bounds, depth);
            let node_index = nodes.len();
            nodes.push(node);
            return node_index;
        }

        // Split the meshlets along the longest axis
        let (left_indices, right_indices) = self.split_indices(meshlets, indices, &bounds);

        // Recursively build children
        let left_index = self.build_recursive(nodes, meshlets, &left_indices, depth + 1);
        let right_index = self.build_recursive(nodes, meshlets, &right_indices, depth + 1);

        // Calculate combined bounds for children
        let left_bounds = nodes[left_index].bounds.clone();
        let right_bounds = nodes[right_index].bounds.clone();
        let combined_bounds = self.combine_bounds(&left_bounds, &right_bounds);

        // Create internal node
        let node = BvhNode::new_internal(left_index, right_index, combined_bounds, depth);
        let node_index = nodes.len();
        nodes.push(node);
        node_index
    }

    fn calculate_bounds(&self, meshlets: &[Meshlet], indices: &[usize]) -> BoundingSphere {
        if indices.is_empty() {
            return BoundingSphere::new(Vec3::ZERO, 0.0);
        }

        let mut center = Vec3::ZERO;
        let mut max_radius: f32 = 0.0;

        for &idx in indices {
            let meshlet = &meshlets[idx];
            center += meshlet.bounds_center;
            max_radius = max_radius.max(meshlet.bounds_radius);
        }

        center /= indices.len() as f32;
        
        // Expand radius to encompass all meshlets
        for &idx in indices {
            let meshlet = &meshlets[idx];
            let distance = (meshlet.bounds_center - center).length() + meshlet.bounds_radius;
            max_radius = max_radius.max(distance);
        }

        BoundingSphere::new(center, max_radius)
    }

    fn split_indices(&self, meshlets: &[Meshlet], indices: &[usize], bounds: &BoundingSphere) -> (Vec<usize>, Vec<usize>) {
        if indices.len() <= 1 {
            return (indices.to_vec(), Vec::new());
        }

        // Find the longest axis of the bounding sphere
        let axis = self.find_longest_axis(meshlets, indices, bounds);

        // Sort indices along the axis
        let mut sorted_indices = indices.to_vec();
        sorted_indices.sort_by(|&a, &b| {
            let pos_a = meshlets[a].bounds_center;
            let pos_b = meshlets[b].bounds_center;
            
            let coord_a = match axis {
                0 => pos_a.x,
                1 => pos_a.y,
                _ => pos_a.z,
            };
            
            let coord_b = match axis {
                0 => pos_b.x,
                1 => pos_b.y,
                _ => pos_b.z,
            };
            
            coord_a.partial_cmp(&coord_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Split at median
        let mid = sorted_indices.len() / 2;
        let left_indices = sorted_indices[..mid].to_vec();
        let right_indices = sorted_indices[mid..].to_vec();

        (left_indices, right_indices)
    }

    fn find_longest_axis(&self, meshlets: &[Meshlet], indices: &[usize], _bounds: &BoundingSphere) -> usize {
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;

        for &idx in indices {
            let pos = meshlets[idx].bounds_center;
            min_x = min_x.min(pos.x);
            max_x = max_x.max(pos.x);
            min_y = min_y.min(pos.y);
            max_y = max_y.max(pos.y);
            min_z = min_z.min(pos.z);
            max_z = max_z.max(pos.z);
        }

        let extent_x = max_x - min_x;
        let extent_y = max_y - min_y;
        let extent_z = max_z - min_z;

        if extent_x >= extent_y && extent_x >= extent_z {
            0 // X axis
        } else if extent_y >= extent_z {
            1 // Y axis
        } else {
            2 // Z axis
        }
    }

    fn combine_bounds(&self, left: &BoundingSphere, right: &BoundingSphere) -> BoundingSphere {
        let center = (left.center + right.center) * 0.5;
        let radius = (left.center - center).length().max(left.radius)
            .max((right.center - center).length().max(right.radius));
        BoundingSphere::new(center, radius)
    }
}

/// BVH traversal for culling
pub struct BvhTraverser {
    nodes: Vec<BvhNode>,
}

impl BvhTraverser {
    pub fn new(nodes: Vec<BvhNode>) -> Self {
        Self { nodes }
    }

    /// Traverse BVH and return visible meshlet indices
    pub fn cull(&self, frustum_planes: &[Vec4; 6], camera_position: Vec3) -> Vec<usize> {
        let mut visible_meshlets = Vec::new();
        
        if self.nodes.is_empty() {
            return visible_meshlets;
        }

        self.traverse_recursive(0, frustum_planes, camera_position, &mut visible_meshlets);
        visible_meshlets
    }

    fn traverse_recursive(
        &self,
        node_index: usize,
        frustum_planes: &[Vec4; 6],
        camera_position: Vec3,
        visible_meshlets: &mut Vec<usize>,
    ) {
        let node = &self.nodes[node_index];

        // Frustum culling
        if !self.is_sphere_in_frustum(&node.bounds, frustum_planes) {
            return;
        }

        if node.is_leaf {
            // Add all meshlets in this leaf
            visible_meshlets.extend_from_slice(&node.meshlet_indices);
        } else {
            // Traverse children
            if let Some(left) = node.left_child {
                self.traverse_recursive(left, frustum_planes, camera_position, visible_meshlets);
            }
            if let Some(right) = node.right_child {
                self.traverse_recursive(right, frustum_planes, camera_position, visible_meshlets);
            }
        }
    }

    fn is_sphere_in_frustum(&self, sphere: &BoundingSphere, frustum_planes: &[Vec4; 6]) -> bool {
        for plane in frustum_planes {
            let distance = plane.xyz().dot(sphere.center) + plane.w;
            if distance < -sphere.radius {
                return false;
            }
        }
        true
    }

    /// LOD-aware traversal - returns meshlets grouped by LOD level
    pub fn cull_with_lod(
        &self,
        frustum_planes: &[Vec4; 6],
        camera_position: Vec3,
        meshlets: &[Meshlet],
        lod_thresholds: &[f32],
    ) -> Vec<(usize, u8)> {
        let mut visible_with_lod = Vec::new();
        
        if self.nodes.is_empty() {
            return visible_with_lod;
        }

        self.traverse_with_lod_recursive(
            0,
            frustum_planes,
            camera_position,
            meshlets,
            lod_thresholds,
            &mut visible_with_lod,
        );
        visible_with_lod
    }

    fn traverse_with_lod_recursive(
        &self,
        node_index: usize,
        frustum_planes: &[Vec4; 6],
        camera_position: Vec3,
        meshlets: &[Meshlet],
        lod_thresholds: &[f32],
        visible_with_lod: &mut Vec<(usize, u8)>,
    ) {
        let node = &self.nodes[node_index];

        // Frustum culling
        if !self.is_sphere_in_frustum(&node.bounds, frustum_planes) {
            return;
        }

        if node.is_leaf {
            // Calculate LOD for each meshlet
            for &meshlet_idx in &node.meshlet_indices {
                let meshlet = &meshlets[meshlet_idx];
                let distance = (meshlet.bounds_center - camera_position).length();
                let lod_level = self.calculate_lod_level(distance, lod_thresholds);
                visible_with_lod.push((meshlet_idx, lod_level));
            }
        } else {
            // Traverse children
            if let Some(left) = node.left_child {
                self.traverse_with_lod_recursive(
                    left,
                    frustum_planes,
                    camera_position,
                    meshlets,
                    lod_thresholds,
                    visible_with_lod,
                );
            }
            if let Some(right) = node.right_child {
                self.traverse_with_lod_recursive(
                    right,
                    frustum_planes,
                    camera_position,
                    meshlets,
                    lod_thresholds,
                    visible_with_lod,
                );
            }
        }
    }

    fn calculate_lod_level(&self, distance: f32, thresholds: &[f32]) -> u8 {
        for (i, &threshold) in thresholds.iter().enumerate() {
            if distance < threshold {
                return i as u8;
            }
        }
        thresholds.len() as u8
    }
}