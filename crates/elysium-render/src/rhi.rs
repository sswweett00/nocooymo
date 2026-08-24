use wgpu::*;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Gelişmiş render hedefi
#[derive(Debug)]
pub struct RenderTarget {
    pub texture: Texture,
    pub view: TextureView,
    pub format: TextureFormat,
    pub size: (u32, u32),
}

impl RenderTarget {
    pub fn new(device: &Device, size: (u32, u32), format: TextureFormat, label: Option<&str>) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&TextureViewDescriptor::default());

        Self {
            texture,
            view,
            format,
            size,
        }
    }
}

///多重 Örnekleme (MSAA) Render Hedefi
#[derive(Debug)]
pub struct MultisampledRenderTarget {
    pub multisampled_texture: Texture,
    pub multisampled_view: TextureView,
    pub resolve_target: RenderTarget,
    pub sample_count: u32,
}

impl MultisampledRenderTarget {
    pub fn new(device: &Device, size: (u32, u32), format: TextureFormat, sample_count: u32) -> Self {
        let multisampled_texture = device.create_texture(&TextureDescriptor {
            label: Some("Multisampled Render Target"),
            size: Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let multisampled_view = multisampled_texture.create_view(&TextureViewDescriptor::default());
        let resolve_target = RenderTarget::new(device, size, format, Some("Resolve Target"));

        Self {
            multisampled_texture,
            multisampled_view,
            resolve_target,
            sample_count,
        }
    }
}

/// Compute işlemi için gerekli yapı
#[derive(Debug)]
pub struct ComputePipelineWrapper {
    pub pipeline: ComputePipeline,
    pub bind_group_layout: BindGroupLayout,
}

impl ComputePipelineWrapper {
    pub fn new(device: &Device, cs_module: &ShaderModule, entry_point: &str, label: Option<&str>) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[],
            label: Some(&format!("{}_bind_group_layout", entry_point)),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label,
            layout: Some(&pipeline_layout),
            module: cs_module,
            entry_point,
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }
}

/// RenderHardwareInterface: WGPU tabanlı soyutlama katmanı
pub struct RenderBackend {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub surface: Option<Surface<'static>>,
    pub swap_chain: Option<SurfaceTexture>,
    pub surface_config: Option<SurfaceConfiguration>,
    pub bindless_descriptor_set: Option<BindGroup>,
    pub pipeline_cache: PipelineCache,
    pub bindless_manager: Option<BindlessDescriptorManager>,
}

impl RenderBackend {
    pub async fn new(window: Option<&'static winit::window::Window>) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            dx12_shader_compiler: Default::default(),
            gles_minor_version: Default::default(),
            ..Default::default()
        });
        
        let surface = if let Some(window) = window {
            Some(unsafe { instance.create_surface(window)? })
        } else {
            None
        };
        
        let adapter = instance.request_adapter(
            &RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: surface.as_ref(),
                force_fallback_adapter: false,
            },
        ).await.ok_or("Failed to find an appropriate adapter")?;

        let (device, queue) = adapter.request_device(
            &DeviceDescriptor {
                label: Some("Main Device"),
                required_features: Features::empty(),
                required_limits: Limits::default(),
            },
            None,
        ).await?;
        
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            swap_chain: None,
            surface_config: None,
            bindless_descriptor_set: None,
            pipeline_cache: PipelineCache::new(),
            bindless_manager: None,
        })
    }
    
    pub fn configure_surface(&mut self, width: u32, height: u32, present_mode: PresentMode) -> Result<(), SurfaceError> {
        if let Some(surface) = &self.surface {
            let config = SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: surface.get_capabilities(&self.adapter).formats[0],
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
            Err(SurfaceError::Lost)
        }
    }
    
    pub fn get_current_frame(&mut self) -> Result<SurfaceTexture, SurfaceError> {
        if let Some(surface) = &self.surface {
            surface.get_current_texture()
        } else {
            panic!("Surface not configured");
        }
    }
    
    pub fn create_buffer(&self, desc: &BufferDescriptor) -> Buffer {
        self.device.create_buffer(desc)
    }
    
    pub fn create_texture(&self, desc: &TextureDescriptor) -> Texture {
        self.device.create_texture(desc)
    }
    
    pub fn create_sampler(&self, desc: &SamplerDescriptor) -> Sampler {
        self.device.create_sampler(desc)
    }
    
    pub fn create_bind_group_layout(&self, desc: &BindGroupLayoutDescriptor) -> BindGroupLayout {
        self.device.create_bind_group_layout(desc)
    }
    
    pub fn create_bind_group(&self, desc: &BindGroupDescriptor) -> BindGroup {
        self.device.create_bind_group(desc)
    }
    
    pub fn create_shader_module(&self, desc: ShaderModuleDescriptor) -> ShaderModule {
        self.device.create_shader_module(desc)
    }
    
    pub fn create_render_pipeline(&mut self, label: &str, desc: &RenderPipelineDescriptor) -> Arc<RenderPipeline> {
        if let Some(cached) = self.pipeline_cache.get_render(label) {
            return cached;
        }
        let pipeline = self.device.create_render_pipeline(desc);
        self.pipeline_cache.insert_render(label.to_string(), pipeline);
        self.pipeline_cache.get_render(label).unwrap()
    }
    
    pub fn create_compute_pipeline(&mut self, label: &str, desc: &ComputePipelineDescriptor) -> Arc<ComputePipeline> {
        if let Some(cached) = self.pipeline_cache.get_compute(label) {
            return cached;
        }
        let pipeline = self.device.create_compute_pipeline(desc);
        self.pipeline_cache.insert_compute(label.to_string(), pipeline);
        self.pipeline_cache.get_compute(label).unwrap()
    }
    
    pub fn create_pipeline_layout(&self, desc: &PipelineLayoutDescriptor) -> PipelineLayout {
        self.device.create_pipeline_layout(desc)
    }
    
    pub fn set_bindless_descriptors(&mut self, bind_group: BindGroup) {
        self.bindless_descriptor_set = Some(bind_group);
    }

    /// Bindless descriptor manager'ı başlat
    pub fn init_bindless_manager(&mut self, max_textures: u32, max_samplers: u32) {
        self.bindless_manager = Some(BindlessDescriptorManager::new(max_textures, max_samplers));
    }

    /// Texture'u bindless heap'e kaydet
    pub fn register_bindless_texture(&mut self, texture_view: TextureView, hash: u64) -> Result<u32, String> {
        if let Some(ref mut manager) = self.bindless_manager {
            manager.register_texture(&self.device, texture_view, hash)
        } else {
            Err("Bindless manager not initialized".to_string())
        }
    }

    /// Sampler'ı bindless heap'e kaydet
    pub fn register_bindless_sampler(&mut self, sampler: Sampler, hash: u64) -> Result<u32, String> {
        if let Some(ref mut manager) = self.bindless_manager {
            manager.register_sampler(&self.device, sampler, hash)
        } else {
            Err("Bindless manager not initialized".to_string())
        }
    }

    /// Bindless bind group'ı oluştur ve set et
    pub fn create_bindless_bind_group(&mut self) -> Result<(), String> {
        if let Some(ref mut manager) = self.bindless_manager {
            let bind_group = manager.create_bind_group(&self.device)?;
            self.bindless_descriptor_set = Some(bind_group);
            Ok(())
        } else {
            Err("Bindless manager not initialized".to_string())
        }
    }

    /// Texture indeksini hash ile al
    pub fn get_bindless_texture_index(&self, hash: u64) -> Option<u32> {
        self.bindless_manager.as_ref()?.get_texture_index(hash)
    }

    /// Sampler indeksini hash ile al
    pub fn get_bindless_sampler_index(&self, hash: u64) -> Option<u32> {
        self.bindless_manager.as_ref()?.get_sampler_index(hash)
    }
    
    pub fn submit_commands(&self, command_buffers: Vec<CommandBuffer>) {
        self.queue.submit(command_buffers);
    }
    
    pub fn copy_buffer_to_buffer(&self, source: &Buffer, source_offset: u64, destination: &Buffer, dest_offset: u64, size: u64) {
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        encoder.copy_buffer_to_buffer(source, source_offset, destination, dest_offset, size);
        self.queue.submit(Some(encoder.finish()));
    }
    
    pub fn copy_buffer_to_texture(&self, source: ImageCopyBuffer, destination: ImageCopyTexture, copy_size: Extent3d) {
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        encoder.copy_buffer_to_texture(source, destination, copy_size);
        self.queue.submit(Some(encoder.finish()));
    }
}

/// Pipeline önbelleği — wgpu RenderPipeline Clone edilemez, Arc kullanıyoruz.
#[derive(Debug)]
pub struct PipelineCache {
    render_pipelines: HashMap<String, Arc<RenderPipeline>>,
    compute_pipelines: HashMap<String, Arc<ComputePipeline>>,
    pipeline_hashes: HashMap<u64, String>, // Hash to label mapping for fast lookup
}

impl PipelineCache {
    pub fn new() -> Self {
        Self {
            render_pipelines: HashMap::new(),
            compute_pipelines: HashMap::new(),
            pipeline_hashes: HashMap::new(),
        }
    }

    /// Verilen etiket ile saklanan render pipeline'ı döndürür.
    pub fn get_render(&self, label: &str) -> Option<Arc<RenderPipeline>> {
        self.render_pipelines.get(label).cloned()
    }

    /// Verilen etiket ile saklanan compute pipeline'ı döndürür.
    pub fn get_compute(&self, label: &str) -> Option<Arc<ComputePipeline>> {
        self.compute_pipelines.get(label).cloned()
    }

    /// Pipeline'ı önbellekle — anahtarı istediğin etiket/hash string.
    pub fn insert_render(&mut self, label: String, pipeline: RenderPipeline) {
        let hash = self.compute_pipeline_hash(&label);
        self.pipeline_hashes.insert(hash, label.clone());
        self.render_pipelines.insert(label, Arc::new(pipeline));
    }

    /// Compute pipeline'ı önbellekle.
    pub fn insert_compute(&mut self, label: String, pipeline: ComputePipeline) {
        let hash = self.compute_pipeline_hash(&label);
        self.pipeline_hashes.insert(hash, label.clone());
        self.compute_pipelines.insert(label, Arc::new(pipeline));
    }

    /// Hash ile pipeline ara (daha hızlı lookup).
    pub fn get_by_hash(&self, hash: u64) -> Option<Arc<RenderPipeline>> {
        if let Some(label) = self.pipeline_hashes.get(&hash) {
            self.get_render(label)
        } else {
            None
        }
    }

    /// Pipeline sayısı.
    pub fn len(&self) -> usize {
        self.render_pipelines.len() + self.compute_pipelines.len()
    }

    /// Önbelleği temizle.
    pub fn clear(&mut self) {
        self.render_pipelines.clear();
        self.compute_pipelines.clear();
        self.pipeline_hashes.clear();
    }

    /// Pipeline hash hesapla (basit implementasyon).
    fn compute_pipeline_hash(&self, label: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        label.hash(&mut hasher);
        hasher.finish()
    }

    /// Pipeline önbelleğini diske kaydet (persist).
    pub fn save_to_disk(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        // Gerçek implementasyonda pipeline binary'leri kaydedilir
        // Şimdilik sadece metadata kaydediyoruz
        let metadata = serde_json::json!({
            "render_pipelines": self.render_pipelines.keys().collect::<Vec<_>>(),
            "compute_pipelines": self.compute_pipelines.keys().collect::<Vec<_>>(),
            "count": self.len()
        });
        
        std::fs::write(path, metadata.to_string())?;
        Ok(())
    }

    /// Pipeline önbelleğini diskten yükle.
    pub fn load_from_disk(&mut self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        // Gerçek implementasyonda pipeline binary'leri yüklenir
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let _metadata: serde_json::Value = serde_json::from_str(&content)?;
            // Metadata işleme yapılabilir
        }
        Ok(())
    }
}

/// Render pipeline yapılandırması
#[derive(Debug, Clone)]
pub struct GraphicsPipelineConfig<'a> {
    pub vertex_module: &'a ShaderModule,
    pub fragment_module: &'a ShaderModule,
    pub vertex_entry_point: &'a str,
    pub fragment_entry_point: &'a str,
    pub layout: Option<&'a PipelineLayout>,
    pub primitive: PrimitiveState,
    pub depth_stencil: Option<DepthStencilState>,
    pub multisample: MultisampleState,
    pub color_targets: Vec<ColorTargetState>,
    pub vertex_buffers: Vec<VertexBufferLayout<'a>>,
    pub label: Option<&'a str>,
}

impl<'a> GraphicsPipelineConfig<'a> {
    pub fn create_pipeline(&self, device: &Device) -> RenderPipeline {
        // Handle pipeline layout with proper ownership and no dangling temporary.
        let mut owned_layout: Option<PipelineLayout> = None;
        let layout_ref: &PipelineLayout = if let Some(layout) = self.layout {
            layout
        } else {
            let label_owned = self.label.map(|l| format!("{}_layout", l));
            owned_layout = Some(device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: label_owned.as_deref(),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            }));
            owned_layout.as_ref().unwrap()
        };

        // Convert Vec<ColorTargetState> to Vec<Option<ColorTargetState>> as required by FragmentState.
        let color_targets: Vec<Option<ColorTargetState>> =
            self.color_targets.iter().cloned().map(Some).collect();

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: self.label,
            layout: Some(layout_ref),
            vertex: VertexState {
                module: self.vertex_module,
                entry_point: self.vertex_entry_point,
                buffers: &self.vertex_buffers,
            },
            fragment: Some(FragmentState {
                module: self.fragment_module,
                entry_point: self.fragment_entry_point,
                targets: &color_targets,
            }),
            primitive: self.primitive.clone(),
            depth_stencil: self.depth_stencil.clone(),
            multisample: self.multisample.clone(),
            multiview: None,
        })
    }
}

/// Render komutlarını kolaylaştıran yardımcı yapı
pub struct RenderCommandEncoder<'a> {
    encoder: CommandEncoder,
    device: &'a Device,
}

impl<'a> RenderCommandEncoder<'a> {
    pub fn new(device: &'a Device, label: Option<&str>) -> Self {
        let encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label,
        });

        Self { encoder, device }
    }

    pub fn begin_render_pass<'b>(&'b mut self, color_attachments: &'b [Option<RenderPassColorAttachment<'b>>], depth_stencil_attachment: Option<RenderPassDepthStencilAttachment<'b>>) -> RenderPass<'b> {
        self.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments,
            depth_stencil_attachment,
            occlusion_query_set: None,
            timestamp_writes: None,
        })
    }

    pub fn finish(self) -> CommandBuffer {
        self.encoder.finish()
    }
}

/// GPU zamanlayıcı
#[derive(Debug)]
pub struct GpuTimer {
    pub query_set: QuerySet,
    pub resolve_buffer: Buffer,
    pub timestamp_period: f32,
}

impl GpuTimer {
    pub fn new(device: &Device, label: Option<&str>) -> Self {
        let query_set = device.create_query_set(&QuerySetDescriptor {
            label,
            ty: QueryType::Timestamp,
            count: 2,
        });

        let resolve_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Timestamp Resolve Buffer"),
            size: 16, // 2 timestamps * 8 bytes each
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let timestamp_period = 1.0;

        Self {
            query_set,
            resolve_buffer,
            timestamp_period,
        }
    }

    pub fn write_timestamp(&self, encoder: &mut CommandEncoder, pass: &mut RenderPass, index: u32) {
        pass.write_timestamp(&self.query_set, index);
    }
}

/// Bindless descriptor set yöneticisi
#[derive(Debug)]
pub struct BindlessDescriptorManager {
    texture_heap: Vec<TextureView>,
    sampler_heap: Vec<Sampler>,
    texture_index_map: HashMap<u64, u32>, // Texture hash to index mapping
    sampler_index_map: HashMap<u64, u32>, // Sampler hash to index mapping
    max_textures: u32,
    max_samplers: u32,
    next_texture_index: u32,
    next_sampler_index: u32,
    bind_group_layout: Option<BindGroupLayout>,
    bind_group: Option<BindGroup>,
}

impl BindlessDescriptorManager {
    pub fn new(max_textures: u32, max_samplers: u32) -> Self {
        Self {
            texture_heap: Vec::with_capacity(max_textures as usize),
            sampler_heap: Vec::with_capacity(max_samplers as usize),
            texture_index_map: HashMap::new(),
            sampler_index_map: HashMap::new(),
            max_textures,
            max_samplers,
            next_texture_index: 0,
            next_sampler_index: 0,
            bind_group_layout: None,
            bind_group: None,
        }
    }

    /// Texture'u bindless heap'e ekler ve indeks döndürür
    pub fn register_texture(&mut self, device: &Device, texture_view: TextureView, hash: u64) -> Result<u32, String> {
        // Eğer texture zaten kayıtlıysa, mevcut indeksi döndür
        if let Some(&index) = self.texture_index_map.get(&hash) {
            return Ok(index);
        }

        // Kapasite kontrolü
        if self.next_texture_index >= self.max_textures {
            return Err("Texture heap is full".to_string());
        }

        let index = self.next_texture_index;
        self.texture_heap.push(texture_view);
        self.texture_index_map.insert(hash, index);
        self.next_texture_index += 1;

        Ok(index)
    }

    /// Sampler'ı bindless heap'e ekler ve indeks döndürür
    pub fn register_sampler(&mut self, device: &Device, sampler: Sampler, hash: u64) -> Result<u32, String> {
        // Eğer sampler zaten kayıtlıysa, mevcut indeksi döndür
        if let Some(&index) = self.sampler_index_map.get(&hash) {
            return Ok(index);
        }

        // Kapasite kontrolü
        if self.next_sampler_index >= self.max_samplers {
            return Err("Sampler heap is full".to_string());
        }

        let index = self.next_sampler_index;
        self.sampler_heap.push(sampler);
        self.sampler_index_map.insert(hash, index);
        self.next_sampler_index += 1;

        Ok(index)
    }

    /// Bindless bind group layout'ı oluşturur
    pub fn create_bind_group_layout(&mut self, device: &Device) -> &BindGroupLayout {
        if self.bind_group_layout.is_some() {
            return self.bind_group_layout.as_ref().unwrap();
        }

        // Capture max values before mutable borrow side-effects to follow restructuring guideline
        let max_textures = self.max_textures;
        let max_samplers = self.max_samplers;

        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Bindless Descriptor Layout"),
            entries: &[
                // Texture array binding
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT | ShaderStages::VERTEX,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2Array,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: std::num::NonZeroU32::new(max_textures),
                },
                // Sampler array binding
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT | ShaderStages::VERTEX,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: std::num::NonZeroU32::new(max_samplers),
                },
            ],
        });

        self.bind_group_layout = Some(layout);
        self.bind_group_layout.as_ref().unwrap()
    }

    /// Bindless bind group'ı oluşturur
    pub fn create_bind_group(&mut self, device: &Device) -> Result<BindGroup, String> {
        // Capture max values before any borrow that would conflict with layout creation
        let max_textures = self.max_textures;
        let max_samplers = self.max_samplers;

        // Ensure layout is cached but create a local owned layout for bind group creation
        // to avoid holding a &mut borrow across heap borrows (fix E0502/E0503).
        // We do not hold a reference to self.bind_group_layout while borrowing heaps.
        if self.bind_group_layout.is_none() {
            let cached = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Bindless Descriptor Layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT | ShaderStages::VERTEX,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2Array,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: std::num::NonZeroU32::new(max_textures),
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT | ShaderStages::VERTEX,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: std::num::NonZeroU32::new(max_samplers),
                    },
                ],
            });
            self.bind_group_layout = Some(cached);
        }

        // Create a local owned layout for the bind group; this does not borrow self,
        // so heap borrows below can coexist without conflict.
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Bindless Descriptor Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT | ShaderStages::VERTEX,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2Array,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: std::num::NonZeroU32::new(max_textures),
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT | ShaderStages::VERTEX,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: std::num::NonZeroU32::new(max_samplers),
                },
            ],
        });

        // Placeholder textures ve samplers ile doldur
        let placeholder_texture = device.create_texture(&TextureDescriptor {
            label: Some("Placeholder Texture"),
            size: Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let placeholder_view = placeholder_texture.create_view(&TextureViewDescriptor::default());

        let placeholder_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Placeholder Sampler"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        // Texture heap'i doldur - referanslar üzerinden
        // Use captured max values to avoid borrowing self.max_* while heap is borrowed
        let mut texture_view_refs: Vec<&TextureView> = self.texture_heap.iter().collect();
        while texture_view_refs.len() < max_textures as usize {
            texture_view_refs.push(&placeholder_view);
        }

        // Sampler heap'i doldur
        let mut sampler_refs: Vec<&Sampler> = self.sampler_heap.iter().collect();
        while sampler_refs.len() < max_samplers as usize {
            sampler_refs.push(&placeholder_sampler);
        }

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Bindless Descriptor Set"),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureViewArray(&texture_view_refs),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::SamplerArray(&sampler_refs),
                },
            ],
        });

        // Return owned BindGroup directly to avoid Clone (BindGroup is not Clone)
        Ok(bind_group)
    }

    /// Texture indeksini hash ile al
    pub fn get_texture_index(&self, hash: u64) -> Option<u32> {
        self.texture_index_map.get(&hash).copied()
    }

    /// Sampler indeksini hash ile al
    pub fn get_sampler_index(&self, hash: u64) -> Option<u32> {
        self.sampler_index_map.get(&hash).copied()
    }

    /// Kayıtlı texture sayısı
    pub fn texture_count(&self) -> u32 {
        self.next_texture_index
    }

    /// Kayıtlı sampler sayısı
    pub fn sampler_count(&self) -> u32 {
        self.next_sampler_index
    }

    /// Heap'i temizle
    pub fn clear(&mut self) {
        self.texture_heap.clear();
        self.sampler_heap.clear();
        self.texture_index_map.clear();
        self.sampler_index_map.clear();
        self.next_texture_index = 0;
        self.next_sampler_index = 0;
        self.bind_group = None;
    }
}

/// GPU bellek yönetimi
pub struct GpuAllocator {
    // Bu, GPU bellek tahsisatçısı için basit bir soyutlama olurdu
    // Gerçek bir uygulamada gpu-allocator crate'i kullanılabilir
}

impl GpuAllocator {
    pub fn new() -> Self {
        Self {}
    }

    pub fn allocate_buffer(&self, device: &Device, size: u64, usage: BufferUsages) -> Buffer {
        device.create_buffer(&BufferDescriptor {
            label: None,
            size,
            usage,
            mapped_at_creation: false,
        })
    }

    pub fn allocate_texture(&self, device: &Device, descriptor: &TextureDescriptor) -> Texture {
        device.create_texture(descriptor)
    }
}