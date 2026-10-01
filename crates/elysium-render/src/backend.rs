//! # Graphics Backend Abstraction Layer
//!
//! Bu modül, elysium-render'ın farklı graphics API'leri (WGPU, Vulkan, DX12,
//! Metal, OpenGL, Software) arasında geçiş yapabilmesi için tam soyutlama
//! katmanı sağlar.
//!
//! ## Mimari
//! - `GraphicsBackend` trait: tüm backend'ler için ortak arayüz.
//! - `AnyBackend` enum: runtime'da backend değiştirmeyi sağlar.
//! - `BackendSelector`: en uygun backend'i otomatik olarak seçer.
//!
//! ## Örnek kullanım
//! ```no_run
//! use elysium_render::backend::{GraphicsBackend, BackendSelector, BackendConfig};
//!
//! let config = BackendConfig::new(BackendType::Wgpu);
//! let backend = BackendSelector::auto_select(config).await?;
//! backend.configure_surface(1920, 1080, PresentMode::AutoVsync)?;
//! ```

use std::fmt;
use std::sync::Arc;
use wgpu::*;

// ---------------------------------------------------------------------------
// Backend Türü
// ---------------------------------------------------------------------------

/// Desteklenen graphics backend API'leri.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BackendType {
    /// WebGPU üzerinden Vulkan/Metal/DX12/WebGPU erişimi.
    Wgpu,
    /// Vulkan graphics API.
    Vulkan,
    /// Microsoft DirectX 12.
    DirectX12,
    /// Apple Metal.
    Metal,
    /// OpenGL (legacy).
    OpenGL,
    /// CPU tabanlı yazılım renderer.
    #[default]
    Software,
}

impl fmt::Display for BackendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wgpu => write!(f, "WGPU"),
            Self::Vulkan => write!(f, "Vulkan"),
            Self::DirectX12 => write!(f, "DirectX 12"),
            Self::Metal => write!(f, "Metal"),
            Self::OpenGL => write!(f, "OpenGL"),
            Self::Software => write!(f, "Software"),
        }
    }
}

// ---------------------------------------------------------------------------
// Backend Yetenekleri
// ---------------------------------------------------------------------------

/// Backend'ın desteklediği özellikler, formatlar ve sınırlar.
#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    pub backend_type: BackendType,
    pub device_name: String,
    pub adapter_name: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub max_texture_size: u32,
    pub max_buffer_size: u64,
    pub max_bind_groups: u32,
    pub max_color_attachments: u32,
    pub supports_ray_tracing: bool,
    pub supports_mesh_shaders: bool,
    pub supports_bindless: bool,
    pub supports_compute: bool,
    pub supports_depth_clamp: bool,
    pub supports_independent_blending: bool,
    pub supported_present_modes: Vec<PresentMode>,
    pub supported_texture_formats: Vec<TextureFormat>,
    pub timestamp_supported: bool,
    pub pipeline_statistics_supported: bool,
}

impl Default for BackendCapabilities {
    fn default() -> Self {
        Self {
            backend_type: BackendType::Software,
            device_name: "Software Renderer".to_string(),
            adapter_name: "CPU".to_string(),
            vendor_id: 0,
            device_id: 0,
            max_texture_size: 8192,
            max_buffer_size: u64::MAX,
            max_bind_groups: 4,
            max_color_attachments: 4,
            supports_ray_tracing: false,
            supports_mesh_shaders: false,
            supports_bindless: false,
            supports_compute: false,
            supports_depth_clamp: false,
            supports_independent_blending: true,
            supported_present_modes: vec![PresentMode::AutoVsync],
            supported_texture_formats: vec![TextureFormat::Rgba8UnormSrgb],
            timestamp_supported: false,
            pipeline_statistics_supported: false,
        }
    }
}

impl BackendCapabilities {
    /// Yeni bir BackendCapabilities oluşturur.
    pub fn new(backend_type: BackendType, device_name: &str, adapter_name: &str) -> Self {
        Self {
            backend_type,
            device_name: device_name.to_string(),
            adapter_name: adapter_name.to_string(),
            ..Default::default()
        }
    }

    /// WGPU adapter'dan BackendCapabilities oluşturur.
    pub fn from_wgpu(adapter: &Adapter, device: &Device) -> Self {
        let info = adapter.get_info();
        let limits = device.limits();
        let features = device.features();

        Self {
            backend_type: BackendType::Wgpu,
            device_name: info.device.clone(),
            adapter_name: info.adapter.clone(),
            vendor_id: info.vendor,
            device_id: info.device,
            max_texture_size: limits.max_texture_dimension_2d,
            max_buffer_size: limits.max_buffer_size,
            max_bind_groups: limits.max_bind_groups,
            max_color_attachments: limits.max_color_attachments,
            supports_ray_tracing: features.contains(Features::EXPERIMENTAL_RAY_TRACING),
            supports_mesh_shaders: features.contains(Features::EXPERIMENTAL_MESH_SHADER),
            supports_bindless: limits.max_bind_groups >= 8,
            supports_compute: true,
            supports_depth_clamp: features.contains(Features::DEPTH_CLAMP),
            supports_independent_blending: features.contains(Features::INDEPENDENT_BLENDING),
            supported_present_modes: vec![
                PresentMode::AutoVsync,
                PresentMode::AutoNoVsync,
                PresentMode::Mailbox,
            ],
            supported_texture_formats: vec![
                TextureFormat::Rgba8UnormSrgb,
                TextureFormat::Bgra8UnormSrgb,
                TextureFormat::Rgba16Float,
                TextureFormat::Depth32Float,
                TextureFormat::Depth24Plus,
                TextureFormat::Rgba32Float,
            ],
            timestamp_supported: features.contains(Features::TIMESTAMP_QUERY),
            pipeline_statistics_supported: features.contains(Features::PIPELINE_STATISTICS_QUERY),
        }
    }

    /// Belirli bir formatın desteklenip desteklenmediğini kontrol eder.
    pub fn is_format_supported(&self, format: TextureFormat) -> bool {
        self.supported_texture_formats.contains(&format)
    }

    /// Belirli bir present modunun desteklenip desteklenmediğini kontrol eder.
    pub fn is_present_mode_supported(&self, mode: PresentMode) -> bool {
        self.supported_present_modes.contains(&mode)
    }
}

// ---------------------------------------------------------------------------
// Backend Yapılandırması
// ---------------------------------------------------------------------------

/// Backend oluşturma ve çalıştırma yapılandırması.
#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub backend_type: BackendType,
    pub power_preference: PowerPreference,
    pub required_features: Features,
    pub required_limits: Limits,
    pub vsync: bool,
    pub max_frames_in_flight: u32,
    pub sample_count: u32,
    pub enable_bindless: bool,
    pub enable_pipeline_cache: bool,
    pub force_fallback: bool,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            backend_type: BackendType::Wgpu,
            power_preference: PowerPreference::HighPerformance,
            required_features: Features::empty(),
            required_limits: Limits::default(),
            vsync: true,
            max_frames_in_flight: 2,
            sample_count: 4,
            enable_bindless: true,
            enable_pipeline_cache: true,
            force_fallback: false,
        }
    }
}

impl BackendConfig {
    /// Yeni bir BackendConfig oluşturur.
    pub fn new(backend_type: BackendType) -> Self {
        Self {
            backend_type,
            ..Default::default()
        }
    }

    /// Güç tercihini ayarlar.
    pub fn with_power_preference(mut self, preference: PowerPreference) -> Self {
        self.power_preference = preference;
        self
    }

    /// VSync ayarını değiştirir.
    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }

    /// Örnekleme sayısını ayarlar.
    pub fn with_samples(mut self, count: u32) -> Self {
        self.sample_count = count;
        self
    }

    /// Bindless desteğini ayarlar.
    pub fn with_bindless(mut self, enabled: bool) -> Self {
        self.enable_bindless = enabled;
        self
    }

    /// Pipeline önbelleğini ayarlar.
    pub fn with_pipeline_cache(mut self, enabled: bool) -> Self {
        self.enable_pipeline_cache = enabled;
        self
    }

    /// Fallback (düşük performanslı) adapter'ı zorlar.
    pub fn with_fallback(mut self, fallback: bool) -> Self {
        self.force_fallback = fallback;
        self
    }

    /// Yapılandırmaya göre uygun PresentMode döndürür.
    pub fn present_mode(&self) -> PresentMode {
        if self.vsync {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        }
    }
}

// ---------------------------------------------------------------------------
// Backend Hata Türü
// ---------------------------------------------------------------------------

/// Backend işlemleri sırasında oluşabilecek hatalar.
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("uygun graphics adapter bulunamadı")]
    AdapterNotFound,

    #[error("cihaz oluşturma başarısız: {0}")]
    DeviceCreationFailed(String),

    #[error("surface yapılandırma başarısız")]
    SurfaceConfigFailed,

    #[error("desteklenmeyen backend: {0}")]
    UnsupportedBackend(String),

    #[error("özellik desteklenmiyor: {0}")]
    FeatureNotSupported(String),

    #[error("swapchain hatası")]
    SwapchainError,

    #[error("frame alınamadı")]
    FrameAcquisitionFailed,

    #[error("present hatası: {0}")]
    PresentError(String),

    #[error("kaynak oluşturma başarısız: {0}")]
    ResourceCreationFailed(String),

    #[error("yazılım backend hatası: {0}")]
    SoftwareError(String),
}

// ---------------------------------------------------------------------------
// GraphicsBackend Trait
// ---------------------------------------------------------------------------

/// Tüm graphics backend'leri için ortak soyutlama arayüzü.
///
/// Bu trait, WGPU, Vulkan, DX12, Metal, OpenGL ve Software renderer'ları
/// arasında tutarlı bir API sağlar. Gerçek uygulamada her backend kendi
/// API'sini bu arayüzün altında sarmalar.
pub trait GraphicsBackend: Send + Sync {
    /// Backend türünü döndürür.
    fn backend_type(&self) -> BackendType;

    /// Backend yeteneklerini döndürür.
    fn capabilities(&self) -> BackendCapabilities;

    /// Kullanılan cihaz adını döndürür.
    fn device_name(&self) -> &str;

    /// Belirli bir özelliğin desteklenip desteklenmediğini kontrol eder.
    fn is_feature_supported(&self, feature: &str) -> bool;

    // -----------------------------------------------------------------------
    // Surface Yönetimi
    // -----------------------------------------------------------------------

    /// Surface'ı belirtilen boyut ve present moduna göre yapılandırır.
    fn configure_surface(
        &mut self,
        width: u32,
        height: u32,
        present_mode: PresentMode,
    ) -> Result<(), BackendError>;

    /// Geçerli frame için swapchain texture'ını alır.
    fn get_current_frame(&mut self) -> Result<SurfaceTexture, BackendError>;

    /// Frame'i sunucuya sunar.
    fn present(&mut self);

    // -----------------------------------------------------------------------
    // Kaynak Oluşturma
    // -----------------------------------------------------------------------

    /// GPU buffer oluşturur.
    fn create_buffer(&self, desc: &BufferDescriptor) -> Result<Buffer, BackendError>;

    /// GPU texture oluşturur.
    fn create_texture(&self, desc: &TextureDescriptor) -> Result<Texture, BackendError>;

    /// GPU sampler oluşturur.
    fn create_sampler(&self, desc: &SamplerDescriptor) -> Result<Sampler, BackendError>;

    /// Shader modülü oluşturur.
    fn create_shader_module(&self, desc: ShaderModuleDescriptor) -> Result<ShaderModule, BackendError>;

    /// Bind group layout oluşturur.
    fn create_bind_group_layout(
        &self,
        desc: &BindGroupLayoutDescriptor,
    ) -> Result<BindGroupLayout, BackendError>;

    /// Bind group oluşturur.
    fn create_bind_group(&self, desc: &BindGroupDescriptor) -> Result<BindGroup, BackendError>;

    /// Render pipeline oluşturur.
    fn create_render_pipeline(
        &self,
        desc: &RenderPipelineDescriptor,
    ) -> Result<RenderPipeline, BackendError>;

    /// Compute pipeline oluşturur.
    fn create_compute_pipeline(
        &self,
        desc: &ComputePipelineDescriptor,
    ) -> Result<ComputePipeline, BackendError>;

    /// Pipeline layout oluşturur.
    fn create_pipeline_layout(
        &self,
        desc: &PipelineLayoutDescriptor,
    ) -> Result<PipelineLayout, BackendError>;

    // -----------------------------------------------------------------------
    // Komut İşleme
    // -----------------------------------------------------------------------

    /// Komut buffer'ları GPU kuyruğuna gönderir.
    fn submit_commands(&self, command_buffers: Vec<CommandBuffer>);

    /// Buffer'dan buffer'a veri kopyalar.
    fn copy_buffer_to_buffer(
        &self,
        source: &Buffer,
        source_offset: u64,
        destination: &Buffer,
        dest_offset: u64,
        size: u64,
    );

    /// Buffer'dan texture'a veri kopyalar.
    fn copy_buffer_to_texture(
        &self,
        source: ImageCopyBuffer,
        destination: ImageCopyTexture,
        copy_size: Extent3d,
    );
}

// ---------------------------------------------------------------------------
// WGPU Backend — RenderBackend Entegrasyonu
// ---------------------------------------------------------------------------

// RenderBackend artık GraphicsBackend trait'ini implement eder.
// rhi.rs'deki RenderBackend struct'ına capabilities desteği eklenir.

impl GraphicsBackend for crate::rhi::RenderBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Wgpu
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::from_wgpu(&self.adapter, &self.device)
    }

    fn device_name(&self) -> &str {
        &self.adapter.get_info().device
    }

    fn is_feature_supported(&self, feature: &str) -> bool {
        let features = self.device.features();
        // Basit özellik eşleştirme — gerçek uygulamada daha kapsamlı olur.
        match feature {
            "ray_tracing" => features.contains(Features::EXPERIMENTAL_RAY_TRACING),
            "mesh_shader" => features.contains(Features::EXPERIMENTAL_MESH_SHADER),
            "depth_clamp" => features.contains(Features::DEPTH_CLAMP),
            "timestamp" => features.contains(Features::TIMESTAMP_QUERY),
            "pipeline_stats" => features.contains(Features::PIPELINE_STATISTICS_QUERY),
            "bindless" => self.device.limits().max_bind_groups >= 8,
            "compute" => true,
            _ => false,
        }
    }

    fn configure_surface(
        &mut self,
        width: u32,
        height: u32,
        present_mode: PresentMode,
    ) -> Result<(), BackendError> {
        if let Some(surface) = &self.surface {
            let caps = surface.get_capabilities(&self.adapter);
            let format = caps.formats[0];
            let config = SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format,
                width,
                height,
                present_mode,
                alpha_mode: CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&self.device, &config);
            self.surface_config = Some(config);
            Ok(())
        } else {
            Err(BackendError::SurfaceConfigFailed)
        }
    }

    fn get_current_frame(&mut self) -> Result<SurfaceTexture, BackendError> {
        if let Some(surface) = &self.surface {
            surface.get_current_texture().map_err(|_| BackendError::FrameAcquisitionFailed)
        } else {
            Err(BackendError::SurfaceConfigFailed)
        }
    }

    fn present(&mut self) {
        // WGPU'da present_frame artık get_current_texture sonrası texture.submit() ile yapılır.
        // Burada sadece placeholder bırakıyoruz — gerçek present işlemi çağıran tarafça yönetilir.
    }

    fn create_buffer(&self, desc: &BufferDescriptor) -> Result<Buffer, BackendError> {
        Ok(self.device.create_buffer(desc))
    }

    fn create_texture(&self, desc: &TextureDescriptor) -> Result<Texture, BackendError> {
        Ok(self.device.create_texture(desc))
    }

    fn create_sampler(&self, desc: &SamplerDescriptor) -> Result<Sampler, BackendError> {
        Ok(self.device.create_sampler(desc))
    }

    fn create_shader_module(&self, desc: ShaderModuleDescriptor) -> Result<ShaderModule, BackendError> {
        Ok(self.device.create_shader_module(desc))
    }

    fn create_bind_group_layout(
        &self,
        desc: &BindGroupLayoutDescriptor,
    ) -> Result<BindGroupLayout, BackendError> {
        Ok(self.device.create_bind_group_layout(desc))
    }

    fn create_bind_group(&self, desc: &BindGroupDescriptor) -> Result<BindGroup, BackendError> {
        Ok(self.device.create_bind_group(desc))
    }

    fn create_render_pipeline(
        &self,
        desc: &RenderPipelineDescriptor,
    ) -> Result<RenderPipeline, BackendError> {
        Ok(self.device.create_render_pipeline(desc))
    }

    fn create_compute_pipeline(
        &self,
        desc: &ComputePipelineDescriptor,
    ) -> Result<ComputePipeline, BackendError> {
        Ok(self.device.create_compute_pipeline(desc))
    }

    fn create_pipeline_layout(
        &self,
        desc: &PipelineLayoutDescriptor,
    ) -> Result<PipelineLayout, BackendError> {
        Ok(self.device.create_pipeline_layout(desc))
    }

    fn submit_commands(&self, command_buffers: Vec<CommandBuffer>) {
        self.queue.submit(command_buffers);
    }

    fn copy_buffer_to_buffer(
        &self,
        source: &Buffer,
        source_offset: u64,
        destination: &Buffer,
        dest_offset: u64,
        size: u64,
    ) {
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        encoder.copy_buffer_to_buffer(source, source_offset, destination, dest_offset, size);
        self.queue.submit(Some(encoder.finish()));
    }

    fn copy_buffer_to_texture(
        &self,
        source: ImageCopyBuffer,
        destination: ImageCopyTexture,
        copy_size: Extent3d,
    ) {
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        encoder.copy_buffer_to_texture(source, destination, copy_size);
        self.queue.submit(Some(encoder.finish()));
    }
}

// ---------------------------------------------------------------------------
// Vulkan Backend (stub)
// ---------------------------------------------------------------------------

/// Vulkan graphics backend — şu an stub implementasyon.
///
/// Gerçek uygulamada `ash` veya `vulkanalia` crate'leri kullanılarak
/// tam Vulkan instance/device/swapchain oluşturma yapılır.
#[derive(Debug, Default)]
pub struct VulkanBackend {
    device_name: String,
    instance: Option<()>,
    device: Option<()>,
    swapchain: Option<()>,
    capabilities: BackendCapabilities,
}

impl VulkanBackend {
    /// Yeni bir VulkanBackend örneği oluşturur.
    pub fn new(device_name: &str) -> Self {
        Self {
            device_name: device_name.to_string(),
            ..Default::default()
        }
    }

    /// Vulkan instance oluşturur.
    pub fn create_instance(&mut self) -> Result<(), BackendError> {
        // Gerçek implementasyonda vk::Instance oluşturulur.
        self.instance = Some(());
        Ok(())
    }

    /// Fiziksel cihaz seçer.
    pub fn select_adapter(&self) -> Result<(), BackendError> {
        // Gerçek implementasyonda vkEnumeratePhysicalDevices ile uygun cihaz seçilir.
        Ok(())
    }

    /// Cihaz ve kuyruk oluşturur.
    pub fn create_device(&mut self) -> Result<(), BackendError> {
        // Gerçek implementasyonda vkCreateDevice ile cihaz oluşturulur.
        self.device = Some(());
        Ok(())
    }

    /// Swapchain yönetimi.
    pub fn create_swapchain(&mut self, width: u32, height: u32) -> Result<(), BackendError> {
        // Gerçek implementasyonda vkCreateSwapchainKHR ile swapchain oluşturulur.
        self.swapchain = Some(());
        Ok(())
    }

    /// Vulkan özelliklerini sorgular.
    pub fn query_features(&self) -> BackendCapabilities {
        BackendCapabilities {
            backend_type: BackendType::Vulkan,
            device_name: self.device_name.clone(),
            adapter_name: "Vulkan Physical Device".to_string(),
            vendor_id: 0,
            device_id: 0,
            max_texture_size: 16384,
            max_buffer_size: u64::MAX,
            max_bind_groups: 8,
            max_color_attachments: 8,
            supports_ray_tracing: false,
            supports_mesh_shaders: false,
            supports_bindless: true,
            supports_compute: true,
            supports_depth_clamp: true,
            supports_independent_blending: true,
            supported_present_modes: vec![
                PresentMode::AutoVsync,
                PresentMode::AutoNoVsync,
                PresentMode::Mailbox,
            ],
            supported_texture_formats: vec![
                TextureFormat::Rgba8UnormSrgb,
                TextureFormat::Bgra8UnormSrgb,
                TextureFormat::Rgba16Float,
                TextureFormat::Depth32Float,
            ],
            timestamp_supported: true,
            pipeline_statistics_supported: true,
        }
    }
}

impl GraphicsBackend for VulkanBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Vulkan
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.query_features()
    }

    fn device_name(&self) -> &str {
        &self.device_name
    }

    fn is_feature_supported(&self, feature: &str) -> bool {
        self.query_features().is_feature_supported(feature)
    }

    fn configure_surface(
        &mut self,
        _width: u32,
        _height: u32,
        _present_mode: PresentMode,
    ) -> Result<(), BackendError> {
        Err(BackendError::UnsupportedBackend(
            "VulkanBackend: configure_surface henüz implement edilmedi".to_string(),
        ))
    }

    fn get_current_frame(&mut self) -> Result<SurfaceTexture, BackendError> {
        Err(BackendError::UnsupportedBackend(
            "VulkanBackend: get_current_frame henüz implement edilmedi".to_string(),
        ))
    }

    fn present(&mut self) {
        // Stub
    }

    fn create_buffer(&self, _desc: &BufferDescriptor) -> Result<Buffer, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "VulkanBackend: buffer oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_texture(&self, _desc: &TextureDescriptor) -> Result<Texture, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "VulkanBackend: texture oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_sampler(&self, _desc: &SamplerDescriptor) -> Result<Sampler, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "VulkanBackend: sampler oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_shader_module(
        &self,
        _desc: ShaderModuleDescriptor,
    ) -> Result<ShaderModule, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "VulkanBackend: shader modülü oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_bind_group_layout(
        &self,
        _desc: &BindGroupLayoutDescriptor,
    ) -> Result<BindGroupLayout, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "VulkanBackend: bind group layout henüz implement edilmedi".to_string(),
        ))
    }

    fn create_bind_group(&self, _desc: &BindGroupDescriptor) -> Result<BindGroup, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "VulkanBackend: bind group henüz implement edilmedi".to_string(),
        ))
    }

    fn create_render_pipeline(
        &self,
        _desc: &RenderPipelineDescriptor,
    ) -> Result<RenderPipeline, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "VulkanBackend: render pipeline henüz implement edilmedi".to_string(),
        ))
    }

    fn create_compute_pipeline(
        &self,
        _desc: &ComputePipelineDescriptor,
    ) -> Result<ComputePipeline, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "VulkanBackend: compute pipeline henüz implement edilmedi".to_string(),
        ))
    }

    fn create_pipeline_layout(
        &self,
        _desc: &PipelineLayoutDescriptor,
    ) -> Result<PipelineLayout, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "VulkanBackend: pipeline layout henüz implement edilmedi".to_string(),
        ))
    }

    fn submit_commands(&self, _command_buffers: Vec<CommandBuffer>) {
        // Stub — gerçek uygulamada vkQueueSubmit ile komutlar gönderilir.
    }

    fn copy_buffer_to_buffer(
        &self,
        _source: &Buffer,
        _source_offset: u64,
        _destination: &Buffer,
        _dest_offset: u64,
        _size: u64,
    ) {
        // Stub
    }

    fn copy_buffer_to_texture(
        &self,
        _source: ImageCopyBuffer,
        _destination: ImageCopyTexture,
        _copy_size: Extent3d,
    ) {
        // Stub
    }
}

// ---------------------------------------------------------------------------
// DirectX 12 Backend (stub)
// ---------------------------------------------------------------------------

/// Microsoft DirectX 12 graphics backend — şu an stub implementasyon.
///
/// Gerçek uygulamada `windows` crate üzerinden D3D12 API'si kullanılır.
#[derive(Debug, Default)]
pub struct Dx12Backend {
    device_name: String,
    device: Option<()>,
    command_queue: Option<()>,
    swapchain: Option<()>,
    capabilities: BackendCapabilities,
}

impl Dx12Backend {
    /// Yeni bir Dx12Backend örneği oluşturur.
    pub fn new(device_name: &str) -> Self {
        Self {
            device_name: device_name.to_string(),
            ..Default::default()
        }
    }

    /// D3D12 cihazı oluşturur.
    pub fn create_device(&mut self) -> Result<(), BackendError> {
        self.device = Some(());
        Ok(())
    }

    /// Komut kuyruğu oluşturur.
    pub fn create_command_queue(&mut self) -> Result<(), BackendError> {
        self.command_queue = Some(());
        Ok(())
    }

    /// Swapchain oluşturur.
    pub fn create_swapchain(&mut self, width: u32, height: u32) -> Result<(), BackendError> {
        self.swapchain = Some(());
        Ok(())
    }

    /// DX12 özelliklerini sorgular.
    pub fn query_features(&self) -> BackendCapabilities {
        BackendCapabilities {
            backend_type: BackendType::DirectX12,
            device_name: self.device_name.clone(),
            adapter_name: "D3D12 Adapter".to_string(),
            vendor_id: 0,
            device_id: 0,
            max_texture_size: 16384,
            max_buffer_size: u64::MAX,
            max_bind_groups: 8,
            max_color_attachments: 8,
            supports_ray_tracing: false,
            supports_mesh_shaders: false,
            supports_bindless: true,
            supports_compute: true,
            supports_depth_clamp: true,
            supports_independent_blending: true,
            supported_present_modes: vec![
                PresentMode::AutoVsync,
                PresentMode::AutoNoVsync,
                PresentMode::Mailbox,
            ],
            supported_texture_formats: vec![
                TextureFormat::Rgba8UnormSrgb,
                TextureFormat::Bgra8UnormSrgb,
                TextureFormat::Rgba16Float,
                TextureFormat::Depth32Float,
            ],
            timestamp_supported: true,
            pipeline_statistics_supported: true,
        }
    }
}

impl GraphicsBackend for Dx12Backend {
    fn backend_type(&self) -> BackendType {
        BackendType::DirectX12
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.query_features()
    }

    fn device_name(&self) -> &str {
        &self.device_name
    }

    fn is_feature_supported(&self, feature: &str) -> bool {
        self.query_features().is_feature_supported(feature)
    }

    fn configure_surface(
        &mut self,
        _width: u32,
        _height: u32,
        _present_mode: PresentMode,
    ) -> Result<(), BackendError> {
        Err(BackendError::UnsupportedBackend(
            "Dx12Backend: configure_surface henüz implement edilmedi".to_string(),
        ))
    }

    fn get_current_frame(&mut self) -> Result<SurfaceTexture, BackendError> {
        Err(BackendError::UnsupportedBackend(
            "Dx12Backend: get_current_frame henüz implement edilmedi".to_string(),
        ))
    }

    fn present(&mut self) {
        // Stub
    }

    fn create_buffer(&self, _desc: &BufferDescriptor) -> Result<Buffer, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "Dx12Backend: buffer oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_texture(&self, _desc: &TextureDescriptor) -> Result<Texture, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "Dx12Backend: texture oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_sampler(&self, _desc: &SamplerDescriptor) -> Result<Sampler, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "Dx12Backend: sampler oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_shader_module(
        &self,
        _desc: ShaderModuleDescriptor,
    ) -> Result<ShaderModule, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "Dx12Backend: shader modülü oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_bind_group_layout(
        &self,
        _desc: &BindGroupLayoutDescriptor,
    ) -> Result<BindGroupLayout, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "Dx12Backend: bind group layout henüz implement edilmedi".to_string(),
        ))
    }

    fn create_bind_group(&self, _desc: &BindGroupDescriptor) -> Result<BindGroup, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "Dx12Backend: bind group henüz implement edilmedi".to_string(),
        ))
    }

    fn create_render_pipeline(
        &self,
        _desc: &RenderPipelineDescriptor,
    ) -> Result<RenderPipeline, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "Dx12Backend: render pipeline henüz implement edilmedi".to_string(),
        ))
    }

    fn create_compute_pipeline(
        &self,
        _desc: &ComputePipelineDescriptor,
    ) -> Result<ComputePipeline, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "Dx12Backend: compute pipeline henüz implement edilmedi".to_string(),
        ))
    }

    fn create_pipeline_layout(
        &self,
        _desc: &PipelineLayoutDescriptor,
    ) -> Result<PipelineLayout, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "Dx12Backend: pipeline layout henüz implement edilmedi".to_string(),
        ))
    }

    fn submit_commands(&self, _command_buffers: Vec<CommandBuffer>) {
        // Stub — gerçek uygulamada ID3D12CommandQueue::ExecuteCommandLists ile gönderilir.
    }

    fn copy_buffer_to_buffer(
        &self,
        _source: &Buffer,
        _source_offset: u64,
        _destination: &Buffer,
        _dest_offset: u64,
        _size: u64,
    ) {
        // Stub
    }

    fn copy_buffer_to_texture(
        &self,
        _source: ImageCopyBuffer,
        _destination: ImageCopyTexture,
        _copy_size: Extent3d,
    ) {
        // Stub
    }
}

// ---------------------------------------------------------------------------
// Metal Backend (stub)
// ---------------------------------------------------------------------------

/// Apple Metal graphics backend — şu an stub implementasyon.
///
/// Gerçek uygulamada `metal` crate üzerinden Objective-C runtime
/// kullanılarak tam Metal cihaz ve komut kuyruğu oluşturma yapılır.
#[derive(Debug, Default)]
pub struct MetalBackend {
    device_name: String,
    device: Option<()>,
    command_queue: Option<()>,
    capabilities: BackendCapabilities,
}

impl MetalBackend {
    /// Yeni bir MetalBackend örneği oluşturur.
    pub fn new(device_name: &str) -> Self {
        Self {
            device_name: device_name.to_string(),
            ..Default::default()
        }
    }

    /// Metal cihazı oluşturur.
    pub fn create_device(&mut self) -> Result<(), BackendError> {
        self.device = Some(());
        Ok(())
    }

    /// Komut kuyruğu oluşturur.
    pub fn create_command_queue(&mut self) -> Result<(), BackendError> {
        self.command_queue = Some(());
        Ok(())
    }

    /// Metal özelliklerini sorgular.
    pub fn query_features(&self) -> BackendCapabilities {
        BackendCapabilities {
            backend_type: BackendType::Metal,
            device_name: self.device_name.clone(),
            adapter_name: "Apple GPU".to_string(),
            vendor_id: 0,
            device_id: 0,
            max_texture_size: 16384,
            max_buffer_size: u64::MAX,
            max_bind_groups: 8,
            max_color_attachments: 8,
            supports_ray_tracing: false,
            supports_mesh_shaders: false,
            supports_bindless: true,
            supports_compute: true,
            supports_depth_clamp: true,
            supports_independent_blending: true,
            supported_present_modes: vec![
                PresentMode::AutoVsync,
                PresentMode::AutoNoVsync,
            ],
            supported_texture_formats: vec![
                TextureFormat::Rgba8UnormSrgb,
                TextureFormat::Bgra8UnormSrgb,
                TextureFormat::Rgba16Float,
                TextureFormat::Depth32Float,
            ],
            timestamp_supported: true,
            pipeline_statistics_supported: false,
        }
    }
}

impl GraphicsBackend for MetalBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Metal
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.query_features()
    }

    fn device_name(&self) -> &str {
        &self.device_name
    }

    fn is_feature_supported(&self, feature: &str) -> bool {
        self.query_features().is_feature_supported(feature)
    }

    fn configure_surface(
        &mut self,
        _width: u32,
        _height: u32,
        _present_mode: PresentMode,
    ) -> Result<(), BackendError> {
        Err(BackendError::UnsupportedBackend(
            "MetalBackend: configure_surface henüz implement edilmedi".to_string(),
        ))
    }

    fn get_current_frame(&mut self) -> Result<SurfaceTexture, BackendError> {
        Err(BackendError::UnsupportedBackend(
            "MetalBackend: get_current_frame henüz implement edilmedi".to_string(),
        ))
    }

    fn present(&mut self) {
        // Stub
    }

    fn create_buffer(&self, _desc: &BufferDescriptor) -> Result<Buffer, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "MetalBackend: buffer oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_texture(&self, _desc: &TextureDescriptor) -> Result<Texture, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "MetalBackend: texture oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_sampler(&self, _desc: &SamplerDescriptor) -> Result<Sampler, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "MetalBackend: sampler oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_shader_module(
        &self,
        _desc: ShaderModuleDescriptor,
    ) -> Result<ShaderModule, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "MetalBackend: shader modülü oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_bind_group_layout(
        &self,
        _desc: &BindGroupLayoutDescriptor,
    ) -> Result<BindGroupLayout, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "MetalBackend: bind group layout henüz implement edilmedi".to_string(),
        ))
    }

    fn create_bind_group(&self, _desc: &BindGroupDescriptor) -> Result<BindGroup, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "MetalBackend: bind group henüz implement edilmedi".to_string(),
        ))
    }

    fn create_render_pipeline(
        &self,
        _desc: &RenderPipelineDescriptor,
    ) -> Result<RenderPipeline, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "MetalBackend: render pipeline henüz implement edilmedi".to_string(),
        ))
    }

    fn create_compute_pipeline(
        &self,
        _desc: &ComputePipelineDescriptor,
    ) -> Result<ComputePipeline, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "MetalBackend: compute pipeline henüz implement edilmedi".to_string(),
        ))
    }

    fn create_pipeline_layout(
        &self,
        _desc: &PipelineLayoutDescriptor,
    ) -> Result<PipelineLayout, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "MetalBackend: pipeline layout henüz implement edilmedi".to_string(),
        ))
    }

    fn submit_commands(&self, _command_buffers: Vec<CommandBuffer>) {
        // Stub — gerçek uygulamada MTLCommandBuffer commit ile gönderilir.
    }

    fn copy_buffer_to_buffer(
        &self,
        _source: &Buffer,
        _source_offset: u64,
        _destination: &Buffer,
        _dest_offset: u64,
        _size: u64,
    ) {
        // Stub
    }

    fn copy_buffer_to_texture(
        &self,
        _source: ImageCopyBuffer,
        _destination: ImageCopyTexture,
        _copy_size: Extent3d,
    ) {
        // Stub
    }
}

// ---------------------------------------------------------------------------
// OpenGL Backend (stub)
// ---------------------------------------------------------------------------

/// OpenGL graphics backend — şu an stub implementasyon.
///
/// Gerçek uygulamada `gl` crate üzerinden OpenGL bağlamı oluşturulur
/// ve tüm GL işlemleri bu arayüzün altında sarmalanır.
#[derive(Debug, Default)]
pub struct OpenGLBackend {
    device_name: String,
    context: Option<()>,
    capabilities: BackendCapabilities,
}

impl OpenGLBackend {
    /// Yeni bir OpenGLBackend örneği oluşturur.
    pub fn new(device_name: &str) -> Self {
        Self {
            device_name: device_name.to_string(),
            ..Default::default()
        }
    }

    /// OpenGL bağlamı oluşturur.
    pub fn create_context(&mut self) -> Result<(), BackendError> {
        self.context = Some(());
        Ok(())
    }

    /// OpenGL uzantılarını yükler.
    pub fn load_extensions(&mut self) -> Result<(), BackendError> {
        // Gerçek implementasyonda gl::load_with ile tüm GL fonksiyonları yüklenir.
        Ok(())
    }

    /// OpenGL özelliklerini sorgular.
    pub fn query_features(&self) -> BackendCapabilities {
        BackendCapabilities {
            backend_type: BackendType::OpenGL,
            device_name: self.device_name.clone(),
            adapter_name: "OpenGL Renderer".to_string(),
            vendor_id: 0,
            device_id: 0,
            max_texture_size: 16384,
            max_buffer_size: u64::MAX,
            max_bind_groups: 4,
            max_color_attachments: 8,
            supports_ray_tracing: false,
            supports_mesh_shaders: false,
            supports_bindless: false,
            supports_compute: true,
            supports_depth_clamp: true,
            supports_independent_blending: true,
            supported_present_modes: vec![
                PresentMode::AutoVsync,
                PresentMode::AutoNoVsync,
            ],
            supported_texture_formats: vec![
                TextureFormat::Rgba8UnormSrgb,
                TextureFormat::Rgba16Float,
                TextureFormat::Depth32Float,
            ],
            timestamp_supported: true,
            pipeline_statistics_supported: false,
        }
    }
}

impl GraphicsBackend for OpenGLBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::OpenGL
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.query_features()
    }

    fn device_name(&self) -> &str {
        &self.device_name
    }

    fn is_feature_supported(&self, feature: &str) -> bool {
        self.query_features().is_feature_supported(feature)
    }

    fn configure_surface(
        &mut self,
        _width: u32,
        _height: u32,
        _present_mode: PresentMode,
    ) -> Result<(), BackendError> {
        Err(BackendError::UnsupportedBackend(
            "OpenGLBackend: configure_surface henüz implement edilmedi".to_string(),
        ))
    }

    fn get_current_frame(&mut self) -> Result<SurfaceTexture, BackendError> {
        Err(BackendError::UnsupportedBackend(
            "OpenGLBackend: get_current_frame henüz implement edilmedi".to_string(),
        ))
    }

    fn present(&mut self) {
        // Stub — gerçek uygulamada glSwapBuffers ile sunucuya iletilir.
    }

    fn create_buffer(&self, _desc: &BufferDescriptor) -> Result<Buffer, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "OpenGLBackend: buffer oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_texture(&self, _desc: &TextureDescriptor) -> Result<Texture, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "OpenGLBackend: texture oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_sampler(&self, _desc: &SamplerDescriptor) -> Result<Sampler, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "OpenGLBackend: sampler oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_shader_module(
        &self,
        _desc: ShaderModuleDescriptor,
    ) -> Result<ShaderModule, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "OpenGLBackend: shader modülü oluşturma henüz implement edilmedi".to_string(),
        ))
    }

    fn create_bind_group_layout(
        &self,
        _desc: &BindGroupLayoutDescriptor,
    ) -> Result<BindGroupLayout, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "OpenGLBackend: bind group layout henüz implement edilmedi".to_string(),
        ))
    }

    fn create_bind_group(&self, _desc: &BindGroupDescriptor) -> Result<BindGroup, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "OpenGLBackend: bind group henüz implement edilmedi".to_string(),
        ))
    }

    fn create_render_pipeline(
        &self,
        _desc: &RenderPipelineDescriptor,
    ) -> Result<RenderPipeline, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "OpenGLBackend: render pipeline henüz implement edilmedi".to_string(),
        ))
    }

    fn create_compute_pipeline(
        &self,
        _desc: &ComputePipelineDescriptor,
    ) -> Result<ComputePipeline, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "OpenGLBackend: compute pipeline henüz implement edilmedi".to_string(),
        ))
    }

    fn create_pipeline_layout(
        &self,
        _desc: &PipelineLayoutDescriptor,
    ) -> Result<PipelineLayout, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "OpenGLBackend: pipeline layout henüz implement edilmedi".to_string(),
        ))
    }

    fn submit_commands(&self, _command_buffers: Vec<CommandBuffer>) {
        // Stub — gerçek uygulamada OpenGL komutları çağrılır.
    }

    fn copy_buffer_to_buffer(
        &self,
        _source: &Buffer,
        _source_offset: u64,
        _destination: &Buffer,
        _dest_offset: u64,
        _size: u64,
    ) {
        // Stub
    }

    fn copy_buffer_to_texture(
        &self,
        _source: ImageCopyBuffer,
        _destination: ImageCopyTexture,
        _copy_size: Extent3d,
    ) {
        // Stub
    }
}

// ---------------------------------------------------------------------------
// Software Backend
// ---------------------------------------------------------------------------

/// CPU tabanlı yazılım renderer backend.
///
/// Bu yapı, `SoftwareRenderer` kavramını sarmalar ve sadece CPU'da
/// çalışan bir rendering yoludur. GPU olmayan sistemlerde veya
/// debug/fallback amaçlı kullanılır.
///
/// Desteklenen özellikler:
/// - Basit rasterization (üçgen çizimi)
/// - Z-buffer derinlik testi
/// - Ham RGBA framebuffer erişimi
/// - UI dikdörtgen ve glyph blit
#[derive(Debug, Clone)]
pub struct SoftwareBackend {
    width: u32,
    height: u32,
    viewport_offset: (i32, i32),
    viewport_size: (u32, u32),
    grid_enabled: bool,
    color: Vec<u8>,
    depth: Vec<f32>,
}

impl SoftwareBackend {
    /// Yeni bir SoftwareBackend oluşturur.
    pub fn new(width: u32, height: u32) -> Self {
        let n = (width * height) as usize;
        Self {
            width,
            height,
            viewport_offset: (0, 0),
            viewport_size: (width, height),
            grid_enabled: true,
            color: vec![0u8; n * 4],
            depth: vec![f32::INFINITY; n],
        }
    }

    /// Boyutu değiştirir.
    pub fn resize(&mut self, width: u32, height: u32) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            let n = (width * height) as usize;
            self.color = vec![0u8; n * 4];
            self.depth = vec![f32::INFINITY; n];
        }
    }

    /// Ham RGBA framebuffer erişimi.
    pub fn frame_buffer(&self) -> &[u8] {
        &self.color
    }

    /// Ham RGBA framebuffer erişimi (yazılabilir).
    pub fn frame_buffer_mut(&mut self) -> &mut [u8] {
        &mut self.color
    }

    /// Grid çizimini aç/kapat.
    pub fn set_grid_enabled(&mut self, enabled: bool) {
        self.grid_enabled = enabled;
    }

    /// Ekranı temizler.
    pub fn clear(&mut self, r: u8, g: u8, b: u8) {
        let n = (self.width * self.height) as usize;
        for i in 0..n {
            self.color[i * 4] = r;
            self.color[i * 4 + 1] = g;
            self.color[i * 4 + 2] = b;
            self.color[i * 4 + 3] = 255;
            self.depth[i] = f32::INFINITY;
        }
    }

    /// Tek bir pikseli ayarlar.
    pub fn set_pixel(&mut self, x: i32, y: i32, z: f32, r: u8, g: u8, b: u8) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = (y as u32 * self.width + x as u32) as usize;
        if z < self.depth[idx] {
            self.depth[idx] = z;
            self.color[idx * 4] = r;
            self.color[idx * 4 + 1] = g;
            self.color[idx * 4 + 2] = b;
            self.color[idx * 4 + 3] = 255;
        }
    }

    /// Üçgen rasterization.
    pub fn draw_triangle(&mut self, p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], r: u8, g: u8, b: u8) {
        let (w, h) = (self.width as i32, self.height as i32);
        let min_x = p0[0].min(p1[0]).min(p2[0]).max(0.0) as i32;
        let max_x = (p0[0].max(p1[0]).max(p2[0]) as i32 + 1).min(w - 1);
        let min_y = p0[1].min(p1[1]).min(p2[1]).max(0.0) as i32;
        let max_y = (p0[1].max(p1[1]).max(p2[1]) as i32 + 1).min(h - 1);

        let edge = |a: [f32; 2], b: [f32; 2], c: [f32; 2]| -> f32 {
            (c[0] - a[0]) * (b[1] - a[1]) - (c[1] - a[1]) * (b[0] - a[0])
        };

        let a2 = [p0[0], p0[1]];
        let b2 = [p1[0], p1[1]];
        let c2 = [p2[0], p2[1]];
        let area = edge(a2, b2, c2);
        if area.abs() < 0.5 {
            return;
        }

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let p = [px as f32 + 0.5, py as f32 + 0.5];
                let w0 = edge(b2, c2, p);
                let w1 = edge(c2, a2, p);
                let w2 = edge(a2, b2, p);
                let inside = if area > 0.0 {
                    w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0
                } else {
                    w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0
                };
                if inside {
                    let denom = w0 + w1 + w2;
                    let bary = [w0 / denom, w1 / denom, w2 / denom];
                    let z = bary[0] * p0[2] + bary[1] * p1[2] + bary[2] * p2[2];
                    self.set_pixel(px, py, z, r, g, b);
                }
            }
        }
    }

    /// Çizgi çizer.
    pub fn draw_line(
        &mut self,
        mut x0: i32,
        mut y0: i32,
        z0: f32,
        x1: i32,
        y1: i32,
        z1: f32,
        r: u8,
        g: u8,
        b: u8,
    ) {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        let steps_total = dx.max(dy).max(1) as f32;
        let mut step = 0.0f32;
        loop {
            let t = step / steps_total;
            let z = z0 + (z1 - z0) * t;
            self.set_pixel(x0, y0, z, r, g, b);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x0 += sx;
            }
            if e2 < dx {
                err -= dx;
                y0 += sy;
            }
            step += 1.0;
        }
    }

    /// UI için alpha-blend dikdörtgen çizer.
    pub fn ui_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 4]) {
        if w <= 0 || h <= 0 {
            return;
        }
        let (fw, fh) = (self.width as i32, self.height as i32);
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(fw);
        let y1 = (y + h).min(fh);
        let a = c[3] as f32 / 255.0;
        for py in y0..y1 {
            for px in x0..x1 {
                let idx = (py as u32 * self.width + px as u32) as usize;
                let inv = 1.0 - a;
                self.color[idx * 4] = (c[0] as f32 * a + self.color[idx * 4] as f32 * inv) as u8;
                self.color[idx * 4 + 1] =
                    (c[1] as f32 * a + self.color[idx * 4 + 1] as f32 * inv) as u8;
                self.color[idx * 4 + 2] =
                    (c[2] as f32 * a + self.color[idx * 4 + 2] as f32 * inv) as u8;
                self.color[idx * 4 + 3] = 255;
            }
        }
    }

    /// Genişlik ve yükseklik döndürür.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl GraphicsBackend for SoftwareBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Software
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            backend_type: BackendType::Software,
            device_name: format!("Software Renderer {}x{}", self.width, self.height),
            adapter_name: "CPU".to_string(),
            vendor_id: 0,
            device_id: 0,
            max_texture_size: 8192,
            max_buffer_size: u64::MAX,
            max_bind_groups: 0,
            max_color_attachments: 1,
            supports_ray_tracing: false,
            supports_mesh_shaders: false,
            supports_bindless: false,
            supports_compute: false,
            supports_depth_clamp: false,
            supports_independent_blending: true,
            supported_present_modes: vec![PresentMode::AutoVsync],
            supported_texture_formats: vec![TextureFormat::Rgba8UnormSrgb],
            timestamp_supported: false,
            pipeline_statistics_supported: false,
        }
    }

    fn device_name(&self) -> &str {
        "Software Renderer"
    }

    fn is_feature_supported(&self, _feature: &str) -> bool {
        false
    }

    fn configure_surface(
        &mut self,
        width: u32,
        height: u32,
        _present_mode: PresentMode,
    ) -> Result<(), BackendError> {
        self.resize(width, height);
        Ok(())
    }

    fn get_current_frame(&mut self) -> Result<SurfaceTexture, BackendError> {
        Err(BackendError::UnsupportedBackend(
            "SoftwareBackend: get_current_frame desteklenmiyor".to_string(),
        ))
    }

    fn present(&mut self) {
        // Yazılım renderer'da present işlemi yok — framebuffer doğrudan okunur.
    }

    fn create_buffer(&self, _desc: &BufferDescriptor) -> Result<Buffer, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "SoftwareBackend: GPU buffer oluşturma desteklenmiyor".to_string(),
        ))
    }

    fn create_texture(&self, _desc: &TextureDescriptor) -> Result<Texture, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "SoftwareBackend: GPU texture oluşturma desteklenmiyor".to_string(),
        ))
    }

    fn create_sampler(&self, _desc: &SamplerDescriptor) -> Result<Sampler, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "SoftwareBackend: GPU sampler oluşturma desteklenmiyor".to_string(),
        ))
    }

    fn create_shader_module(
        &self,
        _desc: ShaderModuleDescriptor,
    ) -> Result<ShaderModule, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "SoftwareBackend: shader modülü desteklenmiyor".to_string(),
        ))
    }

    fn create_bind_group_layout(
        &self,
        _desc: &BindGroupLayoutDescriptor,
    ) -> Result<BindGroupLayout, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "SoftwareBackend: bind group layout desteklenmiyor".to_string(),
        ))
    }

    fn create_bind_group(&self, _desc: &BindGroupDescriptor) -> Result<BindGroup, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "SoftwareBackend: bind group desteklenmiyor".to_string(),
        ))
    }

    fn create_render_pipeline(
        &self,
        _desc: &RenderPipelineDescriptor,
    ) -> Result<RenderPipeline, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "SoftwareBackend: render pipeline desteklenmiyor".to_string(),
        ))
    }

    fn create_compute_pipeline(
        &self,
        _desc: &ComputePipelineDescriptor,
    ) -> Result<ComputePipeline, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "SoftwareBackend: compute pipeline desteklenmiyor".to_string(),
        ))
    }

    fn create_pipeline_layout(
        &self,
        _desc: &PipelineLayoutDescriptor,
    ) -> Result<PipelineLayout, BackendError> {
        Err(BackendError::ResourceCreationFailed(
            "SoftwareBackend: pipeline layout desteklenmiyor".to_string(),
        ))
    }

    fn submit_commands(&self, _command_buffers: Vec<CommandBuffer>) {
        // Stub
    }

    fn copy_buffer_to_buffer(
        &self,
        _source: &Buffer,
        _source_offset: u64,
        _destination: &Buffer,
        _dest_offset: u64,
        _size: u64,
    ) {
        // Stub
    }

    fn copy_buffer_to_texture(
        &self,
        _source: ImageCopyBuffer,
        _destination: ImageCopyTexture,
        _copy_size: Extent3d,
    ) {
        // Stub
    }
}

// ---------------------------------------------------------------------------
// AnyBackend — Runtime Backend Değişimi
// ---------------------------------------------------------------------------

/// Runtime'da herhangi bir backend tutmak için enum.
///
/// Bu enum, farklı backend türlerini tek bir trait object altında
/// birleştirir ve backend değiştirmeyi sağlar.
pub enum AnyBackend {
    Wgpu(crate::rhi::RenderBackend),
    Vulkan(VulkanBackend),
    Dx12(Dx12Backend),
    Metal(MetalBackend),
    OpenGL(OpenGLBackend),
    Software(SoftwareBackend),
}

impl fmt::Debug for AnyBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wgpu(_) => write!(f, "AnyBackend::Wgpu"),
            Self::Vulkan(_) => write!(f, "AnyBackend::Vulkan"),
            Self::Dx12(_) => write!(f, "AnyBackend::Dx12"),
            Self::Metal(_) => write!(f, "AnyBackend::Metal"),
            Self::OpenGL(_) => write!(f, "AnyBackend::OpenGL"),
            Self::Software(_) => write!(f, "AnyBackend::Software"),
        }
    }
}

impl GraphicsBackend for AnyBackend {
    fn backend_type(&self) -> BackendType {
        match self {
            Self::Wgpu(b) => b.backend_type(),
            Self::Vulkan(b) => b.backend_type(),
            Self::Dx12(b) => b.backend_type(),
            Self::Metal(b) => b.backend_type(),
            Self::OpenGL(b) => b.backend_type(),
            Self::Software(b) => b.backend_type(),
        }
    }

    fn capabilities(&self) -> BackendCapabilities {
        match self {
            Self::Wgpu(b) => b.capabilities(),
            Self::Vulkan(b) => b.capabilities(),
            Self::Dx12(b) => b.capabilities(),
            Self::Metal(b) => b.capabilities(),
            Self::OpenGL(b) => b.capabilities(),
            Self::Software(b) => b.capabilities(),
        }
    }

    fn device_name(&self) -> &str {
        match self {
            Self::Wgpu(b) => b.device_name(),
            Self::Vulkan(b) => b.device_name(),
            Self::Dx12(b) => b.device_name(),
            Self::Metal(b) => b.device_name(),
            Self::OpenGL(b) => b.device_name(),
            Self::Software(b) => b.device_name(),
        }
    }

    fn is_feature_supported(&self, feature: &str) -> bool {
        match self {
            Self::Wgpu(b) => b.is_feature_supported(feature),
            Self::Vulkan(b) => b.is_feature_supported(feature),
            Self::Dx12(b) => b.is_feature_supported(feature),
            Self::Metal(b) => b.is_feature_supported(feature),
            Self::OpenGL(b) => b.is_feature_supported(feature),
            Self::Software(b) => b.is_feature_supported(feature),
        }
    }

    fn configure_surface(
        &mut self,
        width: u32,
        height: u32,
        present_mode: PresentMode,
    ) -> Result<(), BackendError> {
        match self {
            Self::Wgpu(b) => b.configure_surface(width, height, present_mode),
            Self::Vulkan(b) => b.configure_surface(width, height, present_mode),
            Self::Dx12(b) => b.configure_surface(width, height, present_mode),
            Self::Metal(b) => b.configure_surface(width, height, present_mode),
            Self::OpenGL(b) => b.configure_surface(width, height, present_mode),
            Self::Software(b) => b.configure_surface(width, height, present_mode),
        }
    }

    fn get_current_frame(&mut self) -> Result<SurfaceTexture, BackendError> {
        match self {
            Self::Wgpu(b) => b.get_current_frame(),
            Self::Vulkan(b) => b.get_current_frame(),
            Self::Dx12(b) => b.get_current_frame(),
            Self::Metal(b) => b.get_current_frame(),
            Self::OpenGL(b) => b.get_current_frame(),
            Self::Software(b) => b.get_current_frame(),
        }
    }

    fn present(&mut self) {
        match self {
            Self::Wgpu(b) => b.present(),
            Self::Vulkan(b) => b.present(),
            Self::Dx12(b) => b.present(),
            Self::Metal(b) => b.present(),
            Self::OpenGL(b) => b.present(),
            Self::Software(b) => b.present(),
        }
    }

    fn create_buffer(&self, desc: &BufferDescriptor) -> Result<Buffer, BackendError> {
        match self {
            Self::Wgpu(b) => b.create_buffer(desc),
            Self::Vulkan(b) => b.create_buffer(desc),
            Self::Dx12(b) => b.create_buffer(desc),
            Self::Metal(b) => b.create_buffer(desc),
            Self::OpenGL(b) => b.create_buffer(desc),
            Self::Software(b) => b.create_buffer(desc),
        }
    }

    fn create_texture(&self, desc: &TextureDescriptor) -> Result<Texture, BackendError> {
        match self {
            Self::Wgpu(b) => b.create_texture(desc),
            Self::Vulkan(b) => b.create_texture(desc),
            Self::Dx12(b) => b.create_texture(desc),
            Self::Metal(b) => b.create_texture(desc),
            Self::OpenGL(b) => b.create_texture(desc),
            Self::Software(b) => b.create_texture(desc),
        }
    }

    fn create_sampler(&self, desc: &SamplerDescriptor) -> Result<Sampler, BackendError> {
        match self {
            Self::Wgpu(b) => b.create_sampler(desc),
            Self::Vulkan(b) => b.create_sampler(desc),
            Self::Dx12(b) => b.create_sampler(desc),
            Self::Metal(b) => b.create_sampler(desc),
            Self::OpenGL(b) => b.create_sampler(desc),
            Self::Software(b) => b.create_sampler(desc),
        }
    }

    fn create_shader_module(
        &self,
        desc: ShaderModuleDescriptor,
    ) -> Result<ShaderModule, BackendError> {
        match self {
            Self::Wgpu(b) => b.create_shader_module(desc),
            Self::Vulkan(b) => b.create_shader_module(desc),
            Self::Dx12(b) => b.create_shader_module(desc),
            Self::Metal(b) => b.create_shader_module(desc),
            Self::OpenGL(b) => b.create_shader_module(desc),
            Self::Software(b) => b.create_shader_module(desc),
        }
    }

    fn create_bind_group_layout(
        &self,
        desc: &BindGroupLayoutDescriptor,
    ) -> Result<BindGroupLayout, BackendError> {
        match self {
            Self::Wgpu(b) => b.create_bind_group_layout(desc),
            Self::Vulkan(b) => b.create_bind_group_layout(desc),
            Self::Dx12(b) => b.create_bind_group_layout(desc),
            Self::Metal(b) => b.create_bind_group_layout(desc),
            Self::OpenGL(b) => b.create_bind_group_layout(desc),
            Self::Software(b) => b.create_bind_group_layout(desc),
        }
    }

    fn create_bind_group(&self, desc: &BindGroupDescriptor) -> Result<BindGroup, BackendError> {
        match self {
            Self::Wgpu(b) => b.create_bind_group(desc),
            Self::Vulkan(b) => b.create_bind_group(desc),
            Self::Dx12(b) => b.create_bind_group(desc),
            Self::Metal(b) => b.create_bind_group(desc),
            Self::OpenGL(b) => b.create_bind_group(desc),
            Self::Software(b) => b.create_bind_group(desc),
        }
    }

    fn create_render_pipeline(
        &self,
        desc: &RenderPipelineDescriptor,
    ) -> Result<RenderPipeline, BackendError> {
        match self {
            Self::Wgpu(b) => b.create_render_pipeline(desc),
            Self::Vulkan(b) => b.create_render_pipeline(desc),
            Self::Dx12(b) => b.create_render_pipeline(desc),
            Self::Metal(b) => b.create_render_pipeline(desc),
            Self::OpenGL(b) => b.create_render_pipeline(desc),
            Self::Software(b) => b.create_render_pipeline(desc),
        }
    }

    fn create_compute_pipeline(
        &self,
        desc: &ComputePipelineDescriptor,
    ) -> Result<ComputePipeline, BackendError> {
        match self {
            Self::Wgpu(b) => b.create_compute_pipeline(desc),
            Self::Vulkan(b) => b.create_compute_pipeline(desc),
            Self::Dx12(b) => b.create_compute_pipeline(desc),
            Self::Metal(b) => b.create_compute_pipeline(desc),
            Self::OpenGL(b) => b.create_compute_pipeline(desc),
            Self::Software(b) => b.create_compute_pipeline(desc),
        }
    }

    fn create_pipeline_layout(
        &self,
        desc: &PipelineLayoutDescriptor,
    ) -> Result<PipelineLayout, BackendError> {
        match self {
            Self::Wgpu(b) => b.create_pipeline_layout(desc),
            Self::Vulkan(b) => b.create_pipeline_layout(desc),
            Self::Dx12(b) => b.create_pipeline_layout(desc),
            Self::Metal(b) => b.create_pipeline_layout(desc),
            Self::OpenGL(b) => b.create_pipeline_layout(desc),
            Self::Software(b) => b.create_pipeline_layout(desc),
        }
    }

    fn submit_commands(&self, command_buffers: Vec<CommandBuffer>) {
        match self {
            Self::Wgpu(b) => b.submit_commands(command_buffers),
            Self::Vulkan(b) => b.submit_commands(command_buffers),
            Self::Dx12(b) => b.submit_commands(command_buffers),
            Self::Metal(b) => b.submit_commands(command_buffers),
            Self::OpenGL(b) => b.submit_commands(command_buffers),
            Self::Software(b) => b.submit_commands(command_buffers),
        }
    }

    fn copy_buffer_to_buffer(
        &self,
        source: &Buffer,
        source_offset: u64,
        destination: &Buffer,
        dest_offset: u64,
        size: u64,
    ) {
        match self {
            Self::Wgpu(b) => b.copy_buffer_to_buffer(source, source_offset, destination, dest_offset, size),
            Self::Vulkan(b) => b.copy_buffer_to_buffer(source, source_offset, destination, dest_offset, size),
            Self::Dx12(b) => b.copy_buffer_to_buffer(source, source_offset, destination, dest_offset, size),
            Self::Metal(b) => b.copy_buffer_to_buffer(source, source_offset, destination, dest_offset, size),
            Self::OpenGL(b) => b.copy_buffer_to_buffer(source, source_offset, destination, dest_offset, size),
            Self::Software(b) => b.copy_buffer_to_buffer(source, source_offset, destination, dest_offset, size),
        }
    }

    fn copy_buffer_to_texture(
        &self,
        source: ImageCopyBuffer,
        destination: ImageCopyTexture,
        copy_size: Extent3d,
    ) {
        match self {
            Self::Wgpu(b) => b.copy_buffer_to_texture(source, destination, copy_size),
            Self::Vulkan(b) => b.copy_buffer_to_texture(source, destination, copy_size),
            Self::Dx12(b) => b.copy_buffer_to_texture(source, destination, copy_size),
            Self::Metal(b) => b.copy_buffer_to_texture(source, destination, copy_size),
            Self::OpenGL(b) => b.copy_buffer_to_texture(source, destination, copy_size),
            Self::Software(b) => b.copy_buffer_to_texture(source, destination, copy_size),
        }
    }
}

// ---------------------------------------------------------------------------
// BackendSelector — Otomatik Backend Seçimi
// ---------------------------------------------------------------------------

/// Backend seçimi ve fallback zinciri yönetimi.
///
/// Kullanılabilir backend'leri öncelik sırasına göre dener ve
/// ilk başarılı olanı döndürür. Tüm GPU backends başarısız olursa
/// Software backend'e düşer.
#[derive(Debug)]
pub struct BackendSelector {
    fallback_chain: Vec<BackendType>,
}

impl Default for BackendSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl BackendSelector {
    /// Varsayılan fallback zinciriyle yeni bir selector oluşturur.
    ///
    /// Zincir: WGPU → Vulkan → DX12 → Metal → OpenGL → Software
    pub fn new() -> Self {
        Self {
            fallback_chain: vec![
                BackendType::Wgpu,
                BackendType::Vulkan,
                BackendType::DirectX12,
                BackendType::Metal,
                BackendType::OpenGL,
                BackendType::Software,
            ],
        }
    }

    /// Özel fallback zinciriyle selector oluşturur.
    pub fn with_chain(chain: Vec<BackendType>) -> Self {
        Self {
            fallback_chain: chain,
        }
    }

    /// Zinciri önceliğe göre dener ve ilk başarılı backend'i döndürür.
    pub async fn select(&self, config: BackendConfig) -> Result<AnyBackend, BackendError> {
        for backend_type in &self.fallback_chain {
            match self.try_backend(*backend_type, &config).await {
                Ok(backend) => return Ok(backend),
                Err(_) => continue,
            }
        }
        Err(BackendError::AdapterNotFound)
    }

    /// Tek bir backend türünü dener.
    async fn try_backend(
        &self,
        backend_type: BackendType,
        config: &BackendConfig,
    ) -> Result<AnyBackend, BackendError> {
        match backend_type {
            BackendType::Wgpu => {
                let backend = crate::rhi::RenderBackend::new(None)
                    .await
                    .map_err(|_| BackendError::AdapterNotFound)?;
                Ok(AnyBackend::Wgpu(backend))
            }
            BackendType::Vulkan => {
                let mut backend = VulkanBackend::new("Vulkan Device");
                backend.create_instance()?;
                backend.select_adapter()?;
                backend.create_device()?;
                Ok(AnyBackend::Vulkan(backend))
            }
            BackendType::DirectX12 => {
                let mut backend = Dx12Backend::new("D3D12 Device");
                backend.create_device()?;
                backend.create_command_queue()?;
                Ok(AnyBackend::Dx12(backend))
            }
            BackendType::Metal => {
                let mut backend = MetalBackend::new("Apple GPU");
                backend.create_device()?;
                backend.create_command_queue()?;
                Ok(AnyBackend::Metal(backend))
            }
            BackendType::OpenGL => {
                let mut backend = OpenGLBackend::new("OpenGL Renderer");
                backend.create_context()?;
                backend.load_extensions()?;
                Ok(AnyBackend::OpenGL(backend))
            }
            BackendType::Software => {
                let backend = SoftwareBackend::new(
                    config
                        .required_limits
                        .max_texture_dimension_2d
                        .min(1920),
                    1080,
                );
                Ok(AnyBackend::Software(backend))
            }
        }
    }

    /// Otomatik olarak en uygun backend'i seçer.
    ///
    /// Bu metod, varsayılan yapılandırma ile `select` çağrısının
    /// kısaltılmış halidir.
    pub async fn auto_select(config: BackendConfig) -> Result<AnyBackend, BackendError> {
        Self::new().select(config).await
    }

    /// Yalnızca belirli bir backend türünü dener.
    pub async fn select_only(
        &self,
        backend_type: BackendType,
        config: BackendConfig,
    ) -> Result<AnyBackend, BackendError> {
        self.try_backend(backend_type, &config).await
    }
}
