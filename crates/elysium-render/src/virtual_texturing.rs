use elysium_core::math::{Vec2, Vec3};
use std::collections::HashMap;

/// Sanal dokuma atlası bloğu
#[derive(Debug, Clone, PartialEq)]
pub struct VirtualTextureBlock {
    pub page_id: u32,
    pub physical_coords: (u32, u32), // Fiziksel atlas koordinatları
    pub virtual_coords: (u32, u32), // Sanal koordinatlar
    pub level: u8,
    pub is_resident: bool,
    pub last_accessed_frame: u64,
    pub priority: f32,
}

/// Sanal dokuma atlası yapılandırması
#[derive(Debug, Clone)]
pub struct VirtualTextureAtlasConfig {
    pub virtual_size: u32,
    pub physical_size: u32,
    pub block_size: u32,
    pub max_pages: u32,
    pub mipmap_count: u8,
}

impl Default for VirtualTextureAtlasConfig {
    fn default() -> Self {
        Self {
            virtual_size: 16384, // 16K x 16K sanal atlas
            physical_size: 2048, // 2K x 2K fiziksel bellek
            block_size: 64,      // 64x64 blok boyutu
            max_pages: 1024,
            mipmap_count: 8,
        }
    }
}

/// Sanal dokuma atlası
pub struct VirtualTextureAtlas {
    pub config: VirtualTextureAtlasConfig,
    pub blocks: HashMap<(u32, u32, u8), VirtualTextureBlock>,
    pub physical_pages: Vec<Option<VirtualTextureBlock>>,
    pub resident_pages: Vec<u32>,
    pub evicted_pages: Vec<u32>,
    pub frame_counter: u64,
    pub texture_format: TextureFormat,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextureFormat {
    Rgba8Unorm,
    Rgba8Srgb,
    Rgba32Float,
    Bc1Unorm,
    Bc3Unorm,
    Bc7Unorm,
}

impl VirtualTextureAtlas {
    pub fn new(config: VirtualTextureAtlasConfig, format: TextureFormat) -> Self {
        let physical_pages = vec![None; config.max_pages as usize];
        
        Self {
            config,
            blocks: HashMap::new(),
            physical_pages,
            resident_pages: Vec::new(),
            evicted_pages: Vec::new(),
            frame_counter: 0,
            texture_format: format,
        }
    }

    /// Sanal koordinatları fiziksel koordinatlara çevirir
    pub fn virtual_to_physical(&self, virtual_coords: (u32, u32), level: u8) -> Option<(u32, u32)> {
        let key = (virtual_coords.0, virtual_coords.1, level);
        
        if let Some(block) = self.blocks.get(&key) {
            if block.is_resident {
                Some(block.physical_coords)
            } else {
                None // Blok fiziksel bellekte değil
            }
        } else {
            None // Blok hiç oluşturulmamış
        }
    }

    /// Yeni bir sanal blok talep eder
    pub fn request_page(&mut self, virtual_coords: (u32, u32), level: u8) -> RequestResult {
        let key = (virtual_coords.0, virtual_coords.1, level);
        
        // Blok zaten var mı kontrol et — borrow checker için önce klonla kontrol
        if self.blocks.contains_key(&key) {
            let (is_resident, page_id) = {
                let b = self.blocks.get(&key).unwrap();
                (b.is_resident, b.page_id)
            };
            if is_resident {
                // Zaten fiziksel bellekte, erişim zamanını güncelle
                let frame = self.frame_counter;
                let priority = self.calculate_priority_for_key(&key);
                if let Some(b) = self.blocks.get_mut(&key) {
                    b.last_accessed_frame = frame;
                    b.priority = priority;
                }
                return RequestResult::AlreadyResident;
            } else {
                let priority = self.calculate_priority_for_key(&key);
                if let Some(b) = self.blocks.get_mut(&key) {
                    b.priority = priority;
                }
                return RequestResult::NeedsLoading(page_id);
            }
        }
        
        // Yeni blok oluştur
        let page_id = self.allocate_new_page();
        let mut new_block = VirtualTextureBlock {
            page_id,
            physical_coords: (0, 0),
            virtual_coords,
            level,
            is_resident: false,
            last_accessed_frame: 0,
            priority: 0.0,
        };
        
        // Gerekli fiziksel alan tahsis et
        if let Some(physical_coords) = self.allocate_physical_space() {
            new_block.physical_coords = physical_coords;
            new_block.is_resident = true;
            new_block.last_accessed_frame = self.frame_counter;
            new_block.priority = 1.0;
            self.occupy_physical_slot(physical_coords, new_block.clone());
            self.blocks.insert(key, new_block.clone());
            self.resident_pages.push(page_id);
            RequestResult::Allocated(new_block)
        } else {
            // Yeterli fiziksel alan yok, yer açmak için başka sayfaları çıkar
            if let Some(evicted_page) = self.evict_lowest_priority_page() {
                self.evicted_pages.push(evicted_page);
                if let Some(physical_coords) = self.allocate_physical_space() {
                    new_block.physical_coords = physical_coords;
                    new_block.is_resident = true;
                    new_block.last_accessed_frame = self.frame_counter;
                    new_block.priority = 1.0;
                    self.occupy_physical_slot(physical_coords, new_block.clone());
                    self.blocks.insert(key, new_block.clone());
                    self.resident_pages.push(page_id);
                    RequestResult::AllocatedWithEviction(new_block, evicted_page)
                } else {
                    // Hala yeterli alan yok — ghost block ekleme, fail dön
                    RequestResult::Failed
                }
            } else {
                RequestResult::Failed
            }
        }
    }

    /// Yeni bir sayfa için ID tahsis eder
    fn allocate_new_page(&self) -> u32 {
        // Basit bir sayfa tahsisi - daha gelişmişi için sayfa havuzu kullanılabilir
        self.blocks.len() as u32
    }

    /// Fiziksel alan tahsis eder
    fn allocate_physical_space(&mut self) -> Option<(u32, u32)> {
        // Boş bir fiziksel sayfa bul ve işaretle
        for (index, page) in self.physical_pages.iter().enumerate() {
            if page.is_none() {
                let x = (index as u32 % (self.config.physical_size / self.config.block_size)) * self.config.block_size;
                let y = (index as u32 / (self.config.physical_size / self.config.block_size)) * self.config.block_size;
                // Rezerv et — burası çağrılan yerde gerçek blok ile doldurulacak
                // şimdilik placeholder koyarak aynı indeksin tekrar verilmesini engelle
                self.physical_pages[index] = Some(VirtualTextureBlock {
                    page_id: u32::MAX,
                    physical_coords: (x, y),
                    virtual_coords: (0, 0),
                    level: 0,
                    is_resident: false,
                    last_accessed_frame: 0,
                    priority: 0.0,
                });
                return Some((x, y));
            }
        }
        
        None
    }

    /// Fiziksel alanı gerçek blok ile güncelle
    fn occupy_physical_slot(&mut self, coords: (u32, u32), block: VirtualTextureBlock) {
        let cols = self.config.physical_size / self.config.block_size;
        let idx = ((coords.1 / self.config.block_size) * cols + (coords.0 / self.config.block_size)) as usize;
        if idx < self.physical_pages.len() {
            self.physical_pages[idx] = Some(block);
        }
    }

    /// En düşük öncelikli sayfayı çıkarır
    fn evict_lowest_priority_page(&mut self) -> Option<u32> {
        if self.resident_pages.is_empty() {
            return None;
        }
        
        let mut lowest_priority_idx = 0;
        let mut lowest_priority = f32::MAX;
        
        for (idx, &page_id) in self.resident_pages.iter().enumerate() {
            if let Some(block) = self.blocks.values().find(|b| b.page_id == page_id) {
                if block.priority < lowest_priority {
                    lowest_priority = block.priority;
                    lowest_priority_idx = idx;
                }
            }
        }
        
        let evicted_page = self.resident_pages.remove(lowest_priority_idx);
        
        // Fiziksel sayfayı ve block'u temizle
        let evicted_coords = self.blocks.values().find(|b| b.page_id == evicted_page).map(|b| b.physical_coords);
        if let Some(block) = self.blocks.values_mut().find(|b| b.page_id == evicted_page) {
            block.is_resident = false;
            block.physical_coords = (0, 0);
        }
        if let Some(coords) = evicted_coords {
            let cols = self.config.physical_size / self.config.block_size;
            let idx = ((coords.1 / self.config.block_size) * cols + (coords.0 / self.config.block_size)) as usize;
            if idx < self.physical_pages.len() {
                self.physical_pages[idx] = None;
            }
        }
        
        Some(evicted_page)
    }

    /// Sayfa önceliğini hesaplar
    fn calculate_priority(&self, block: &VirtualTextureBlock) -> f32 {
        let time_factor = (self.frame_counter - block.last_accessed_frame) as f32;
        let level_factor = (self.config.mipmap_count - block.level) as f32;
        1.0 / (time_factor + 1.0) * level_factor
    }

    fn calculate_priority_for_key(&self, key: &(u32, u32, u8)) -> f32 {
        if let Some(block) = self.blocks.get(key) {
            self.calculate_priority(block)
        } else {
            0.0
        }
    }

    /// Atlası günceller (her frame çağrılır)
    pub fn update(&mut self) {
        self.frame_counter += 1;
        // Borrow checker için key'leri kopyala, sonra güncelle
        let keys: Vec<(u32, u32, u8)> = self.blocks.keys().cloned().collect();
        for key in keys {
            let should_update = self.blocks.get(&key).map(|b| b.is_resident).unwrap_or(false);
            if should_update {
                let prio = self.calculate_priority_for_key(&key);
                if let Some(b) = self.blocks.get_mut(&key) {
                    b.priority = prio;
                }
            }
        }
    }

    /// Sanal dokumanın UV koordinatlarını fiziksel koordinatlara dönüştürür
    pub fn convert_uv_coordinates(&self, virtual_uv: Vec2, virtual_coords: (u32, u32), level: u8) -> Option<Vec2> {
        if let Some(physical_coords) = self.virtual_to_physical(virtual_coords, level) {
            let physical_atlas_size = self.config.physical_size as f32;
            let block_size = self.config.block_size as f32;
            let physical_block_start_x = physical_coords.0 as f32 / physical_atlas_size;
            let physical_block_start_y = physical_coords.1 as f32 / physical_atlas_size;
            // virtual_uv 0..1 aralığında blok içi koordinat — fiziksel atlas ölçeğine çevir
            let local_u = virtual_uv.x * (block_size / physical_atlas_size);
            let local_v = virtual_uv.y * (block_size / physical_atlas_size);
            let physical_u = physical_block_start_x + local_u;
            let physical_v = physical_block_start_y + local_v;
            Some(Vec2::new(physical_u, physical_v))
        } else {
            None
        }
    }
}

/// Sayfa talebi sonuçları
#[derive(Debug, Clone)]
pub enum RequestResult {
    Allocated(VirtualTextureBlock),
    AllocatedWithEviction(VirtualTextureBlock, u32), // (yeni_blok, çıkarılan_sayfa_id)
    NeedsLoading(u32), // page_id
    AlreadyResident,
    Failed,
}

/// Sanal dokuma sistemi
pub struct VirtualTextureSystem {
    pub atlases: HashMap<String, VirtualTextureAtlas>,
    pub active_atlas: Option<String>,
    pub feedback_buffer: FeedbackBuffer,
    pub streaming_queue: StreamingQueue,
    pub mipmap_streaming_enabled: bool,
}

impl VirtualTextureSystem {
    pub fn new() -> Self {
        Self {
            atlases: HashMap::new(),
            active_atlas: None,
            feedback_buffer: FeedbackBuffer::new(2048, 2048),
            streaming_queue: StreamingQueue::new(512),
            mipmap_streaming_enabled: true,
        }
    }

    pub fn create_atlas(&mut self, name: String, config: VirtualTextureAtlasConfig, format: TextureFormat) {
        let atlas = VirtualTextureAtlas::new(config, format);
        self.atlases.insert(name, atlas);
    }

    pub fn activate_atlas(&mut self, name: &str) -> bool {
        if self.atlases.contains_key(name) {
            self.active_atlas = Some(name.to_string());
            true
        } else {
            false
        }
    }

    pub fn request_tile(&mut self, virtual_coords: (u32, u32), level: u8) -> Option<RequestResult> {
        if let Some(atlas_name) = &self.active_atlas {
            if let Some(atlas) = self.atlases.get_mut(atlas_name) {
                Some(atlas.request_page(virtual_coords, level))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn update_all_atlases(&mut self) {
        for atlas in self.atlases.values_mut() {
            atlas.update();
        }
    }

    /// Feedback buffer'dan sayfa taleplerini işle
    pub fn process_feedback(&mut self) -> Vec<PageRequest> {
        let mut requests = Vec::new();
        
        // Feedback buffer'dan talepleri al
        let feedback_requests = self.feedback_buffer.get_page_requests();
        
        for request in feedback_requests {
            // Streaming queue'ya ekle
            if self.streaming_queue.enqueue(request.clone()) {
                requests.push(request);
            }
        }
        
        requests
    }

    /// Streaming queue'dan sayfaları yükle
    pub fn process_streaming_queue(&mut self) -> Vec<StreamingOperation> {
        let mut operations = Vec::new();
        
        while let Some(request) = self.streaming_queue.dequeue() {
            let operation = self.load_page(request);
            operations.push(operation);
        }
        
        operations
    }

    /// Sayfa yükleme işlemi
    fn load_page(&mut self, request: PageRequest) -> StreamingOperation {
        if let Some(atlas_name) = &self.active_atlas {
            if let Some(atlas) = self.atlases.get_mut(atlas_name) {
                match atlas.request_page((request.virtual_x, request.virtual_y), request.mipmap_level) {
                    RequestResult::Allocated(block) => {
                        StreamingOperation::LoadSuccess(request, block)
                    }
                    RequestResult::AllocatedWithEviction(block, evicted) => {
                        StreamingOperation::LoadWithEviction(request, block, evicted)
                    }
                    RequestResult::NeedsLoading(page_id) => {
                        StreamingOperation::NeedsLoading(request, page_id)
                    }
                    RequestResult::AlreadyResident => {
                        StreamingOperation::AlreadyResident(request)
                    }
                    RequestResult::Failed => {
                        StreamingOperation::LoadFailed(request)
                    }
                }
            } else {
                StreamingOperation::LoadFailed(request)
            }
        } else {
            StreamingOperation::LoadFailed(request)
        }
    }

    /// Mipmap streaming'i etkinleştir/devre dışı bırak
    pub fn set_mipmap_streaming(&mut self, enabled: bool) {
        self.mipmap_streaming_enabled = enabled;
    }

    /// Feedback buffer'ı güncelle
    pub fn update_feedback_buffer(&mut self, screen_uv: Vec2, virtual_coords: (u32, u32), mipmap_level: u8) {
        self.feedback_buffer.record_access(screen_uv, virtual_coords, mipmap_level);
    }

    /// Streaming queue durumunu al
    pub fn get_queue_status(&self) -> QueueStatus {
        QueueStatus {
            pending_count: self.streaming_queue.pending_count(),
            loading_count: self.streaming_queue.loading_count(),
            total_capacity: self.streaming_queue.capacity(),
        }
    }
}

/// Feedback buffer - GPU'dan gelen sayfa taleplerini toplar
#[derive(Debug, Clone)]
pub struct FeedbackBuffer {
    pub width: u32,
    pub height: u32,
    pub page_requests: Vec<PageRequest>,
    pub access_pattern: AccessPattern,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageRequest {
    pub virtual_x: u32,
    pub virtual_y: u32,
    pub mipmap_level: u8,
    pub priority: f32,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub enum AccessPattern {
    Sequential,
    Random,
    Clustered,
}

impl FeedbackBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            page_requests: Vec::new(),
            access_pattern: AccessPattern::Random,
        }
    }

    /// GPU'dan gelen erişimi kaydet
    pub fn record_access(&mut self, screen_uv: Vec2, virtual_coords: (u32, u32), mipmap_level: u8) {
        let request = PageRequest {
            virtual_x: virtual_coords.0,
            virtual_y: virtual_coords.1,
            mipmap_level,
            priority: self.calculate_priority(screen_uv),
            timestamp: self.get_timestamp(),
        };
        
        self.page_requests.push(request);
        
        // Erişim desenini analiz et
        self.analyze_access_pattern();
    }

    /// Sayfa taleplerini al ve temizle
    pub fn get_page_requests(&mut self) -> Vec<PageRequest> {
        let mut requests = Vec::new();
        
        // Öncelik sırasına göre sırala
        self.page_requests.sort_by(|a, b| {
            b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        std::mem::swap(&mut requests, &mut self.page_requests);
        requests
    }

    fn calculate_priority(&self, screen_uv: Vec2) -> f32 {
        // Ekran merkezine yakın talepler daha yüksek öncelik
        let center = Vec2::new(0.5, 0.5);
        let distance = (screen_uv - center).length();
        
        1.0 - distance.min(1.0)
    }

    fn analyze_access_pattern(&mut self) {
        if self.page_requests.len() < 10 {
            return;
        }
        
        let recent = &self.page_requests[self.page_requests.len().saturating_sub(10)..];
        
        // Basit erişim deseni analizi
        let mut sequential_count = 0;
        let mut clustered_count = 0;
        
        for i in 1..recent.len() {
            let prev = &recent[i - 1];
            let curr = &recent[i];
            
            // Sequential kontrolü
            if curr.virtual_x == prev.virtual_x + 1 || curr.virtual_y == prev.virtual_y + 1 {
                sequential_count += 1;
            }
            
            // Clustered kontrolü
            let dx = (curr.virtual_x as i32 - prev.virtual_x as i32).abs();
            let dy = (curr.virtual_y as i32 - prev.virtual_y as i32).abs();
            if dx <= 2 && dy <= 2 {
                clustered_count += 1;
            }
        }
        
        let total = recent.len() - 1;
        if sequential_count as f32 / total as f32 > 0.7 {
            self.access_pattern = AccessPattern::Sequential;
        } else if clustered_count as f32 / total as f32 > 0.7 {
            self.access_pattern = AccessPattern::Clustered;
        } else {
            self.access_pattern = AccessPattern::Random;
        }
    }

    fn get_timestamp(&self) -> u64 {
        // Basit timestamp - gerçek implementasyonda frame counter kullanılabilir
        0
    }
}

/// Streaming queue - sayfa yükleme kuyruğu
#[derive(Debug, Clone)]
pub struct StreamingQueue {
    pub queue: Vec<PageRequest>,
    pub loading: Vec<PageRequest>,
    pub capacity: usize,
}

impl StreamingQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: Vec::with_capacity(capacity),
            loading: Vec::new(),
            capacity,
        }
    }

    pub fn enqueue(&mut self, request: PageRequest) -> bool {
        if self.queue.len() < self.capacity {
            self.queue.push(request);
            true
        } else {
            false
        }
    }

    pub fn dequeue(&mut self) -> Option<PageRequest> {
        if self.queue.is_empty() {
            None
        } else {
            // En yüksek öncelikli talebi al
            let max_idx = self.queue.iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| {
                    a.priority.partial_cmp(&b.priority).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(idx, _)| idx);
            
            if let Some(idx) = max_idx {
                let request = self.queue.remove(idx);
                self.loading.push(request.clone());
                Some(request)
            } else {
                None
            }
        }
    }

    pub fn mark_complete(&mut self, request: &PageRequest) {
        self.loading.retain(|r| r != request);
    }

    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }

    pub fn loading_count(&self) -> usize {
        self.loading.len()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

/// Streaming işlemi sonuçları
#[derive(Debug, Clone)]
pub enum StreamingOperation {
    LoadSuccess(PageRequest, VirtualTextureBlock),
    LoadWithEviction(PageRequest, VirtualTextureBlock, u32),
    NeedsLoading(PageRequest, u32),
    AlreadyResident(PageRequest),
    LoadFailed(PageRequest),
}

/// Queue durumu
#[derive(Debug, Clone)]
pub struct QueueStatus {
    pub pending_count: usize,
    pub loading_count: usize,
    pub total_capacity: usize,
}

/// Mipmap streaming optimizasyonu
pub struct MipmapStreaming {
    pub enabled: bool,
    pub prefetch_distance: u8,
    pub min_lod_bias: i8,
    pub max_lod_bias: i8,
}

impl MipmapStreaming {
    pub fn new() -> Self {
        Self {
            enabled: true,
            prefetch_distance: 2,
            min_lod_bias: -1,
            max_lod_bias: 1,
        }
    }

    /// Optimize edilmiş mipmap seviyesini hesapla
    pub fn calculate_optimal_lod(&self, base_lod: u8, distance: f32) -> u8 {
        if !self.enabled {
            return base_lod;
        }
        let lod_adjustment = (distance / 1000.0) as i8;
        let adjusted_lod = base_lod as i8 + lod_adjustment;
        // LOD bias uygula, 0..mipmap_count aralığında tut
        let biased = adjusted_lod + self.min_lod_bias;
        biased.clamp(0, self.max_lod_bias.max(0)) as u8
    }

    /// Prefetch için mipmap seviyelerini hesapla
    pub fn calculate_prefetch_levels(&self, current_lod: u8) -> Vec<u8> {
        if !self.enabled {
            return vec![current_lod];
        }

        let mut levels = Vec::new();
        
        // Mevcut seviye ve daha düşük detay seviyelerini prefetch et
        for i in 0..=self.prefetch_distance {
            let lod = current_lod.saturating_add(i);
            levels.push(lod);
        }
        
        levels
    }

    /// Mipmap streaming'i etkinleştir/devre dışı bırak
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}