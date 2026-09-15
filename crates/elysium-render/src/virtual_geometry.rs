use elysium_core::math::{Vec2, Vec3, Vec4, Mat4};
use std::collections::HashMap;

/// Sanal geometri bloğu
#[derive(Debug, Clone)]
pub struct VirtualGeometryBlock {
    pub id: u32,
    pub position: Vec3,
    pub size: Vec3,
    pub lod_level: u8,
    pub is_loaded: bool,
    pub bounding_box: BoundingBox,
}

/// Sınırlayıcı kutu
#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y &&
        point.z >= self.min.z && point.z <= self.max.z
    }
}

/// Sanal geometri sistem yapılandırması
#[derive(Debug, Clone)]
pub struct VirtualGeometryConfig {
    pub block_size: f32,
    pub max_blocks: u32,
    pub lod_count: u8,
    pub streaming_distance: f32,
}

impl Default for VirtualGeometryConfig {
    fn default() -> Self {
        Self {
            block_size: 100.0,
            max_blocks: 1024,
            lod_count: 4,
            streaming_distance: 1000.0,
        }
    }
}

/// Sanal geometri sistemi
pub struct VirtualGeometrySystem {
    pub blocks: HashMap<u32, VirtualGeometryBlock>,
    pub config: VirtualGeometryConfig,
    pub active_blocks: Vec<u32>,
    pub loading_queue: Vec<u32>,
    pub camera_position: Vec3,
}

impl VirtualGeometrySystem {
    pub fn new(config: VirtualGeometryConfig) -> Self {
        Self {
            blocks: HashMap::new(),
            config,
            active_blocks: Vec::new(),
            loading_queue: Vec::new(),
            camera_position: Vec3::ZERO,
        }
    }

    /// Yeni bir geometri bloğu ekler
    pub fn add_block(&mut self, id: u32, position: Vec3, size: Vec3) -> &mut VirtualGeometryBlock {
        let bounding_box = BoundingBox::new(
            position - size * 0.5,
            position + size * 0.5,
        );

        let lod_level = self.calculate_lod_level(position);
        
        let block = VirtualGeometryBlock {
            id,
            position,
            size,
            lod_level,
            is_loaded: false,
            bounding_box,
        };

        self.blocks.insert(id, block);
        self.blocks.get_mut(&id).unwrap()
    }

    /// LOD seviyesini kameraya göre hesaplar
    fn calculate_lod_level(&self, block_position: Vec3) -> u8 {
        let distance = (block_position - self.camera_position).length();
        
        // Basit bir LOD hesaplama
        let lod_count = self.config.lod_count as f32;
        let max_distance = self.config.streaming_distance;
        
        let normalized_distance = (distance / max_distance).min(1.0);
        let lod = (normalized_distance * lod_count).floor() as u8;
        
        lod.min(self.config.lod_count - 1)
    }

    /// Kamera pozisyonunu günceller ve gerekli blokları yükleme/boşaltma kuyruğuna alır
    pub fn update_camera_position(&mut self, camera_pos: Vec3) {
        self.camera_position = camera_pos;
        
        // Aktif blokları yeniden hesapla
        self.active_blocks.clear();
        self.loading_queue.clear();
        
        for (id, block) in &mut self.blocks {
            let distance = (block.position - camera_pos).length();
            
            if distance <= self.config.streaming_distance {
                self.active_blocks.push(*id);
                
                // Eğer blok yüklenmemişse ve LOD seviyesi uygunsa yükleme kuyruğuna al
                if !block.is_loaded {
                    self.loading_queue.push(*id);
                }
            }
        }
    }

    /// Sanal geometri için frustum culling
    pub fn cull_by_frustum(&self, frustum_planes: &[Vec4; 6]) -> Vec<u32> {
        let mut visible_blocks = Vec::new();
        
        for block_id in &self.active_blocks {
            if let Some(block) = self.blocks.get(block_id) {
                if self.is_bounding_box_visible(&block.bounding_box, frustum_planes) {
                    visible_blocks.push(*block_id);
                }
            }
        }
        
        visible_blocks
    }

    /// Bir bounding box'in frustum içinde olup olmadığını kontrol eder
    fn is_bounding_box_visible(&self, bbox: &BoundingBox, frustum_planes: &[Vec4; 6]) -> bool {
        for plane in frustum_planes {
            // Her frustum düzlemine göre test et
            let mut out = 0;
            
            // Box'un 8 köşe noktasını test et
            let corners = [
                Vec3::new(bbox.min.x, bbox.min.y, bbox.min.z),
                Vec3::new(bbox.max.x, bbox.min.y, bbox.min.z),
                Vec3::new(bbox.min.x, bbox.max.y, bbox.min.z),
                Vec3::new(bbox.max.x, bbox.max.y, bbox.min.z),
                Vec3::new(bbox.min.x, bbox.min.y, bbox.max.z),
                Vec3::new(bbox.max.x, bbox.min.y, bbox.max.z),
                Vec3::new(bbox.min.x, bbox.max.y, bbox.max.z),
                Vec3::new(bbox.max.x, bbox.max.y, bbox.max.z),
            ];
            
            for corner in &corners {
                if plane.x * corner.x + plane.y * corner.y + plane.z * corner.z + plane.w > 0.0 {
                    out += 1;
                }
            }
            
            // Eğer tüm köşeler düzlemin dışında ise, bu box görünür değil
            if out == 8 {
                return false;
            }
        }
        
        true
    }

    /// Sanal geometri için occlusion culling
    pub fn cull_by_occlusion(&self, visible_blocks: &mut Vec<u32>, occlusion_data: &[bool]) {
        // Basit bir occlusion culling - daha gelişmiş hali için Z-buffer veya hiyerarşik Z buffer kullanılabilir
        visible_blocks.retain(|&id| {
            let index = id as usize % occlusion_data.len();
            occlusion_data[index]
        });
    }

    /// Sanal geometri bloklarını LOD seviyelerine göre gruplar
    pub fn group_by_lod(&self) -> HashMap<u8, Vec<u32>> {
        let mut lod_groups = HashMap::new();
        
        for block_id in &self.active_blocks {
            if let Some(block) = self.blocks.get(block_id) {
                lod_groups.entry(block.lod_level)
                    .or_insert_with(Vec::new)
                    .push(*block_id);
            }
        }
        
        lod_groups
    }
}

/// Sanal geometri için mesh veri yapısı
#[derive(Debug, Clone)]
pub struct VirtualMeshData {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub vertex_count: u32,
    pub index_count: u32,
    pub lod_levels: Vec<LodLevel>,
}

#[derive(Debug, Clone)]
pub struct LodLevel {
    pub vertex_start: u32,
    pub vertex_count: u32,
    pub index_start: u32,
    pub index_count: u32,
    pub screen_coverage: f32,
}

impl VirtualMeshData {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            vertex_count: 0,
            index_count: 0,
            lod_levels: Vec::new(),
        }
    }

    pub fn add_lod_level(&mut self, lod: LodLevel) {
        self.lod_levels.push(lod);
    }

    pub fn get_lod_for_screen_coverage(&self, coverage: f32) -> Option<&LodLevel> {
        // En yakın LOD seviyesini bul
        self.lod_levels.iter()
            .filter(|lod| lod.screen_coverage <= coverage)
            .min_by(|a, b| a.screen_coverage.partial_cmp(&b.screen_coverage).unwrap())
    }
}

/// Visibility Buffer - GPU-driven rendering için
#[derive(Debug, Clone)]
pub struct VisibilityBuffer {
    pub meshlet_ids: Vec<u32>,
    pub triangle_ids: Vec<u32>,
    pub width: u32,
    pub height: u32,
}

impl VisibilityBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let pixel_count = (width * height) as usize;
        Self {
            meshlet_ids: vec![u32::MAX; pixel_count],
            triangle_ids: vec![u32::MAX; pixel_count],
            width,
            height,
        }
    }

    pub fn clear(&mut self) {
        self.meshlet_ids.fill(u32::MAX);
        self.triangle_ids.fill(u32::MAX);
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, meshlet_id: u32, triangle_id: u32) {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            self.meshlet_ids[index] = meshlet_id;
            self.triangle_ids[index] = triangle_id;
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Option<(u32, u32)> {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            if self.meshlet_ids[index] != u32::MAX {
                return Some((self.meshlet_ids[index], self.triangle_ids[index]));
            }
        }
        None
    }
}

/// Hierarchical Z-Buffer (Hi-Z) for occlusion culling
#[derive(Debug, Clone)]
pub struct HiZBuffer {
    pub levels: Vec<HiZLevel>,
    pub max_level: u8,
}

#[derive(Debug, Clone)]
pub struct HiZLevel {
    pub depth_values: Vec<f32>,
    pub width: u32,
    pub height: u32,
}

impl HiZBuffer {
    pub fn new(base_width: u32, base_height: u32, max_level: u8) -> Self {
        let mut levels = Vec::new();
        
        let mut current_width = base_width;
        let mut current_height = base_height;
        
        for _level in 0..=max_level {
            let pixel_count = (current_width * current_height) as usize;
            levels.push(HiZLevel {
                depth_values: vec![1.0; pixel_count], // Start with far plane
                width: current_width,
                height: current_height,
            });
            
            current_width = (current_width / 2).max(1);
            current_height = (current_height / 2).max(1);
        }
        
        Self { levels, max_level }
    }

    pub fn update_from_depth(&mut self, depth_buffer: &[f32], _width: u32, _height: u32) {
        // Base level (level 0) - copy from depth buffer
        if let Some(base_level) = self.levels.get_mut(0) {
            let copy_count = (base_level.width * base_level.height) as usize;
            let copy_count = copy_count.min(depth_buffer.len());
            base_level.depth_values[..copy_count].copy_from_slice(&depth_buffer[..copy_count]);
        }

        // Build mip levels - use cloning to avoid simultaneous immutable/mutable borrows
        for level in 1..=self.max_level as usize {
            if level < self.levels.len() {
                let prev_level = self.levels[level - 1].clone();
                if let Some(current_level) = self.levels.get_mut(level) {
                    Self::build_mip_level(&prev_level, current_level);
                }
            }
        }
    }

    fn build_mip_level(prev_level: &HiZLevel, current_level: &mut HiZLevel) {
        let prev_w = prev_level.width as usize;
        let prev_h = prev_level.height as usize;
        let curr_w = current_level.width as usize;
        let curr_h = current_level.height as usize;

        for y in 0..curr_h {
            for x in 0..curr_w {
                // Sample 2x2 region from previous level
                let prev_x = x * 2;
                let prev_y = y * 2;
                
                let mut max_depth: f32 = 0.0;
                let mut sample_count = 0;

                for dy in 0..2 {
                    for dx in 0..2 {
                        let px = prev_x + dx;
                        let py = prev_y + dy;
                        
                        if px < prev_w && py < prev_h {
                            let idx = py * prev_w + px;
                            max_depth = max_depth.max(prev_level.depth_values[idx]);
                            sample_count += 1;
                        }
                    }
                }

                if sample_count > 0 {
                    let idx = y * curr_w + x;
                    current_level.depth_values[idx] = max_depth;
                }
            }
        }
    }

    /// Test if a bounding box is visible using Hi-Z
    pub fn test_visibility(&self, bbox: &BoundingBox, view_proj: &Mat4) -> bool {
        // Transform bbox corners to screen space and test against Hi-Z
        let corners = self.get_bbox_corners(bbox);
        
        for corner in corners {
            let clip_pos = *view_proj * Vec4::new(corner.x, corner.y, corner.z, 1.0);
            
            // Perspective divide
            if clip_pos.w <= 0.0 {
                continue; // Behind camera
            }
            
            let ndc = Vec3::new(
                clip_pos.x / clip_pos.w,
                clip_pos.y / clip_pos.w,
                clip_pos.z / clip_pos.w,
            );
            
            // Convert to screen coordinates
            let screen_x = ((ndc.x + 1.0) * 0.5 * self.levels[0].width as f32) as u32;
            let screen_y = ((1.0 - ndc.y) * 0.5 * self.levels[0].height as f32) as u32;
            
            // Test against appropriate Hi-Z level based on size
            let lod_level = self.calculate_lod_for_bbox(bbox);
            if let Some(hiz_level) = self.levels.get(lod_level as usize) {
                let hiz_x = screen_x / (1 << lod_level);
                let hiz_y = screen_y / (1 << lod_level);
                
                if hiz_x < hiz_level.width && hiz_y < hiz_level.height {
                    let idx = (hiz_y * hiz_level.width + hiz_x) as usize;
                    let hiz_depth = hiz_level.depth_values[idx];
                    
                    // If our depth is closer than Hi-Z, we're visible
                    if ndc.z < hiz_depth {
                        return true;
                    }
                }
            }
        }
        
        false
    }

    fn get_bbox_corners(&self, bbox: &BoundingBox) -> Vec<Vec3> {
        vec![
            Vec3::new(bbox.min.x, bbox.min.y, bbox.min.z),
            Vec3::new(bbox.max.x, bbox.min.y, bbox.min.z),
            Vec3::new(bbox.min.x, bbox.max.y, bbox.min.z),
            Vec3::new(bbox.max.x, bbox.max.y, bbox.min.z),
            Vec3::new(bbox.min.x, bbox.min.y, bbox.max.z),
            Vec3::new(bbox.max.x, bbox.min.y, bbox.max.z),
            Vec3::new(bbox.min.x, bbox.max.y, bbox.max.z),
            Vec3::new(bbox.max.x, bbox.max.y, bbox.max.z),
        ]
    }

    fn calculate_lod_for_bbox(&self, bbox: &BoundingBox) -> u8 {
        let size = (bbox.max - bbox.min).length();
        let screen_size = size * 100.0; // Simplified calculation
        
        if screen_size > 100.0 {
            0
        } else if screen_size > 50.0 {
            1
        } else if screen_size > 25.0 {
            2
        } else {
            self.max_level.min(3)
        }
    }

    pub fn get_level(&self, level: u8) -> Option<&HiZLevel> {
        self.levels.get(level as usize)
    }
}

/// Weyra Pipeline - GPU-driven visibility system
pub struct WeyraPipeline {
    pub visibility_buffer: VisibilityBuffer,
    pub hiz_buffer: HiZBuffer,
    pub screen_size: (u32, u32),
    pub hiz_max_level: u8,
}

impl WeyraPipeline {
    pub fn new(width: u32, height: u32, hiz_max_level: u8) -> Self {
        Self {
            visibility_buffer: VisibilityBuffer::new(width, height),
            hiz_buffer: HiZBuffer::new(width, height, hiz_max_level),
            screen_size: (width, height),
            hiz_max_level,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.visibility_buffer = VisibilityBuffer::new(width, height);
        self.hiz_buffer = HiZBuffer::new(width, height, self.hiz_max_level);
        self.screen_size = (width, height);
    }

    /// Update Hi-Z from depth buffer
    pub fn update_hiz(&mut self, depth_buffer: &[f32]) {
        self.hiz_buffer.update_from_depth(depth_buffer, self.screen_size.0, self.screen_size.1);
    }

    /// Test meshlet visibility
    pub fn test_meshlet_visibility(&self, meshlet: &VirtualGeometryBlock, view_proj: &Mat4) -> bool {
        self.hiz_buffer.test_visibility(&meshlet.bounding_box, view_proj)
    }

    /// Clear visibility buffer for new frame
    pub fn clear_visibility_buffer(&mut self) {
        self.visibility_buffer.clear();
    }

    /// Write meshlet to visibility buffer (software rasterization fallback)
    pub fn rasterize_meshlet(&mut self, meshlet_id: u32, vertices: &[Vec3], indices: &[u32], view_proj: &Mat4) {
        for tri_idx in (0..indices.len()).step_by(3) {
            if tri_idx + 2 >= indices.len() {
                break;
            }
            
            let v0 = vertices[indices[tri_idx] as usize];
            let v1 = vertices[indices[tri_idx + 1] as usize];
            let v2 = vertices[indices[tri_idx + 2] as usize];
            
            self.rasterize_triangle(meshlet_id, tri_idx as u32 / 3, v0, v1, v2, view_proj);
        }
    }

    fn rasterize_triangle(&mut self, meshlet_id: u32, triangle_id: u32, v0: Vec3, v1: Vec3, v2: Vec3, view_proj: &Mat4) {
        // Transform vertices to screen space
        let p0 = self.transform_to_screen(v0, view_proj);
        let p1 = self.transform_to_screen(v1, view_proj);
        let p2 = self.transform_to_screen(v2, view_proj);

        // Bounding box of triangle in screen space
        let min_x = p0.x.min(p1.x).min(p2.x).max(0.0) as u32;
        let max_x = p0.x.max(p1.x).max(p2.x).min(self.screen_size.0 as f32) as u32;
        let min_y = p0.y.min(p1.y).min(p2.y).max(0.0) as u32;
        let max_y = p0.y.max(p1.y).max(p2.y).min(self.screen_size.1 as f32) as u32;

        // Rasterize triangle using barycentric coordinates
        for y in min_y..max_y {
            for x in min_x..max_x {
                if self.point_in_triangle(x as f32, y as f32, p0, p1, p2) {
                    self.visibility_buffer.set_pixel(x, y, meshlet_id, triangle_id);
                }
            }
        }
    }

    fn transform_to_screen(&self, pos: Vec3, view_proj: &Mat4) -> Vec3 {
        let clip = *view_proj * Vec4::new(pos.x, pos.y, pos.z, 1.0);
        
        if clip.w <= 0.0 {
            return Vec3::new(0.0, 0.0, 1.0);
        }
        
        let ndc = Vec3::new(
            clip.x / clip.w,
            clip.y / clip.w,
            clip.z / clip.w,
        );
        
        Vec3::new(
            (ndc.x + 1.0) * 0.5 * self.screen_size.0 as f32,
            (1.0 - ndc.y) * 0.5 * self.screen_size.1 as f32,
            ndc.z,
        )
    }

    fn point_in_triangle(&self, px: f32, py: f32, p0: Vec3, p1: Vec3, p2: Vec3) -> bool {
        let p = Vec2::new(px, py);
        let v0 = Vec2::new(p0.x, p0.y);
        let v1 = Vec2::new(p1.x, p1.y);
        let v2 = Vec2::new(p2.x, p2.y);

        let area = self.triangle_area(v0, v1, v2);
        let a1 = self.triangle_area(p, v1, v2) / area;
        let a2 = self.triangle_area(v0, p, v2) / area;
        let a3 = self.triangle_area(v0, v1, p) / area;

        a1 >= 0.0 && a2 >= 0.0 && a3 >= 0.0 && (a1 + a2 + a3) <= 1.0
    }

    fn triangle_area(&self, v0: Vec2, v1: Vec2, v2: Vec2) -> f32 {
        ((v1.x - v0.x) * (v2.y - v0.y) - (v2.x - v0.x) * (v1.y - v0.y)).abs()
    }

    /// Get visible meshlets from visibility buffer
    pub fn get_visible_meshlets(&self) -> Vec<u32> {
        let mut visible_meshlets = std::collections::HashSet::new();
        
        for &meshlet_id in &self.visibility_buffer.meshlet_ids {
            if meshlet_id != u32::MAX {
                visible_meshlets.insert(meshlet_id);
            }
        }
        
        visible_meshlets.into_iter().collect()
    }
}