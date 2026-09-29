//! Elysium Engine — gerçek zamanlı oyun motoru editörü.
//! winit + wgpu + yerleşik software rasterizer ve immediate-mode UI.

#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod renderer;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod editor_ui;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod editor_extensions;
mod history;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod advanced_features;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod critical_systems;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod console;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod engine;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod editor_features;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod glb_import;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod gpu_compute;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod selection_gizmo;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod serialization;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod skeletal;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod texture;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod multiplayer;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod level_editor;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod websocket_transport;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod voice_chat;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod shader_editor;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod visual_script;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod anim_state_machine;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod particle_editor;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod audio_3d;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod event_bus;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod plugin_system;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod profiler;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod config_system;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod structured_logger;
#[allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod asset_pipeline;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use glam::Vec3;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use renderer::{
    Camera, EntityTeam, GeometryType, Scene, SoftwareRenderer,
};
use editor_ui::{
    compute_layout, draw_ui, EditComp, UiDragHandle, UiFont, UiLayout,
};
use history::History;

// ─────────────────────────────────────────────────────────── Editor state

pub struct EditorShared {
    pub scene: Scene,
    pub selected: Option<usize>,
    pub playing: bool,
    pub show_grid: bool,
    pub frame_count: u64,
    pub fps: f32,
    pub can_undo: bool,
    pub can_redo: bool,
    /// Inspector sürükleme hedefi
    pub active_drag: Option<UiDragHandle>,
    pub console: console::Console,
    pub console_input: String,
    pub console_cursor: usize,
    // GPU Compute durumu
    pub particle_sim: gpu_compute::CpuParticleSimulator,
    pub frustum_culler: gpu_compute::CpuFrustumCuller,
    pub gpu_particles_active: usize,
    pub gpu_culled: u32,
    pub gpu_visible: u32,
    // Skeletal animasyon durumu
    pub skeleton: skeletal::Skeleton,
    pub animation_clips: std::collections::HashMap<String, skeletal::AnimClip>,
    pub blend_machine: skeletal::BlendStateMachine,
    pub anim_playing: bool,
    pub anim_time: f32,
    pub anim_speed: f32,
    pub skinned_mesh: Option<skeletal::SkinnedMesh>,
    // Çoklu seçim ve gizmo durumu
    pub selection: selection_gizmo::SelectionState,
    pub gizmo: selection_gizmo::TransformGizmo,
    // Yeni özellikler
    pub particle_editor: editor_features::ParticleSystem,
    pub terrain_editor: editor_features::TerrainEditor,
    pub material_editor: editor_features::MaterialEditor,
    pub scene_search: editor_features::SceneSearch,
    pub timeline: editor_features::AnimationTimeline,
    pub annotations: editor_features::AnnotationSystem,
    pub extensions: editor_extensions::EditorExtensions,
}

struct EditorApp {
    // wgpu
    window: Option<Arc<Window>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
    texture: Option<wgpu::Texture>,
    bind_group: Option<wgpu::BindGroup>,
    pipeline: Option<wgpu::RenderPipeline>,
    sampler: Option<wgpu::Sampler>,

    // editör durumu
    state: EditorShared,
    renderer: SoftwareRenderer,
    font: Option<UiFont>,
    ui_layout: UiLayout,

    // girdi
    keys: HashSet<KeyCode>,
    mouse_down: bool,
    last_mouse: Option<(f64, f64)>,
    orbiting: bool,

    // geçmiş / durum mesajı
    history: History,
    status_message: String,
    status_until: Instant,

    // zamanlama
    last_frame: Instant,
    fps_ema: f32,
}

impl EditorApp {
    fn new() -> Self {
        let scene = create_demo_scene();

        Self {
            window: None,
            device: None,
            queue: None,
            surface: None,
            config: None,
            texture: None,
            bind_group: None,
            pipeline: None,
            state: EditorShared {
                scene,
                selected: Some(1),
                playing: false,
                show_grid: true,
                frame_count: 0,
                fps: 60.0,
                can_undo: false,
                can_redo: false,
                active_drag: None,
                console: console::Console::new(),
                console_input: String::new(),
                console_cursor: 0,
                particle_sim: gpu_compute::CpuParticleSimulator::with_capacity(8192),
                frustum_culler: gpu_compute::CpuFrustumCuller::new(),
                gpu_particles_active: 0,
                gpu_culled: 0,
                gpu_visible: 0,
                // Skeletal animasyon başlat
                skeleton: skeletal::Skeleton::demo_humanoid(),
                animation_clips: {
                    let mut clips = std::collections::HashMap::new();
                    let idle = skeletal::create_idle_animation();
                    clips.insert(idle.name.clone(), idle);
                    let walk = skeletal::create_walk_animation();
                    clips.insert(walk.name.clone(), walk);
                    let run = skeletal::create_run_animation();
                    clips.insert(run.name.clone(), run);
                    clips
                },
                blend_machine: {
                    let mut bsm = skeletal::BlendStateMachine::new();
                    bsm.add_state(skeletal::AnimState::new("Idle"));
                    bsm.add_state(skeletal::AnimState::new("Walk"));
                    bsm.add_state(skeletal::AnimState::new("Run"));
                    bsm
                },
                anim_playing: false,
                anim_time: 0.0,
                anim_speed: 1.0,
                skinned_mesh: Some(skeletal::SkinnedMesh::demo_skinned_cube()),
                selection: selection_gizmo::SelectionState::new(),
                gizmo: selection_gizmo::TransformGizmo::new(),
                particle_editor: editor_features::ParticleSystem::fire(),
                terrain_editor: {
                    let mut te = editor_features::TerrainEditor::default();
                    te.generate_procedural(42, 4, 0.5);
                    te.enabled = false;
                    te
                },
                material_editor: editor_features::MaterialEditor::default(),
                scene_search: editor_features::SceneSearch::default(),
                timeline: editor_features::AnimationTimeline::default(),
                annotations: {
                    let mut ann = editor_features::AnnotationSystem::default();
                    ann.add_note("Player spawn point", Vec3::new(0.0, 1.5, 0.0), editor_features::NoteColor::Green);
                    ann.add_note("Boss arena boundary", Vec3::new(7.0, 0.0, 0.0), editor_features::NoteColor::Red);
                    ann.add_note("Look here first!", Vec3::new(0.0, 3.0, -5.0), editor_features::NoteColor::Yellow);
                    ann
                },
                extensions: editor_extensions::EditorExtensions::new(),
            },
            renderer: SoftwareRenderer::new(800, 600),
            sampler: None,
            font: UiFont::load(),
            ui_layout: UiLayout::default(),
            keys: HashSet::new(),
            mouse_down: false,
            last_mouse: None,
            orbiting: false,
            history: History::new(),
            status_message: String::new(),
            status_until: Instant::now(),
            last_frame: Instant::now(),
            fps_ema: 60.0,
        }
    }

    // ── wgpu kurulumu ──────────────────────────────────────────
    fn init_gpu(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).expect("surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .or_else(|| {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: true,
            }))
        })
        .expect("GPU adaptörü bulunamadı");
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor::default(),
            None,
        ))
        .expect("device");
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        self.window = Some(window);
        self.device = Some(device);
        self.queue = Some(queue);
        self.surface = Some(surface);
        self.config = Some(config);
        self.create_texture(size.width.max(1), size.height.max(1));
        let device = self.device.as_ref().unwrap();
        let _queue = self.queue.as_ref().unwrap();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                r#"
                @group(0) @binding(0) var tex: texture_2d<f32>;
                @group(0) @binding(1) var samp: sampler;
                struct VSOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, };
                @vertex fn vs(@builtin(vertex_index) i: u32) -> VSOut {
                    var pos = array<vec2<f32>,6>(vec2(-1,-1),vec2(1,-1),vec2(-1,1),vec2(-1,1),vec2(1,-1),vec2(1,1));
                    var uv  = array<vec2<f32>,6>(vec2(0,1),vec2(1,1),vec2(0,0),vec2(0,0),vec2(1,1),vec2(1,0));
                    var o: VSOut; o.pos = vec4(pos[i],0.0,1.0); o.uv = uv[i]; return o;
                }
                @fragment fn fs(in: VSOut) -> @location(0) vec4<f32> {
                    return textureSample(tex, samp, in.uv);
                }
                "#
                .into(),
            ),
        });
        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: None,
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&tex_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: "vs", buffers: &[] },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        self.pipeline = Some(pipeline);
        self.sampler = Some(sampler);
        self.rebuild_bind_group();
    }

    fn create_texture(&mut self, w: u32, h: u32) {
        let device = self.device.as_ref().unwrap();
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("editor_fb"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.texture = Some(tex);
    }

    fn rebuild_bind_group(&mut self) {
        let device = self.device.as_ref().unwrap();
        let view = self.texture.as_ref().unwrap().create_view(&Default::default());
        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: None,
        });
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &tex_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(self.sampler.as_ref().unwrap()) },
            ],
            label: None,
        });
        self.bind_group = Some(bg);
    }

    fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 { return; }
        if let (Some(surface), Some(device), Some(config)) =
            (&self.surface, &self.device, &mut self.config)
        {
            config.width = w;
            config.height = h;
            surface.configure(device, config);
            self.renderer.resize(w, h);
            self.create_texture(w, h);
            self.rebuild_bind_group();
        }
    }

    // ── Oyun mantığı ───────────────────────────────────────────
    fn update_game(&mut self, dt: f32) {
        let st = &mut self.state;

        // Klavye: WASD oyuncu hareketi (kamera göreli)
        let player_id = 1usize;
        let yaw_r = st.scene.camera.yaw.to_radians();
        let mut move_dir = Vec3::ZERO;
        if self.keys.contains(&KeyCode::KeyW) { move_dir += Vec3::new(yaw_r.sin(), 0.0, -yaw_r.cos()); }
        if self.keys.contains(&KeyCode::KeyS) { move_dir -= Vec3::new(yaw_r.sin(), 0.0, -yaw_r.cos()); }
        if self.keys.contains(&KeyCode::KeyA) { move_dir -= Vec3::new(yaw_r.cos(), 0.0, yaw_r.sin()); }
        if self.keys.contains(&KeyCode::KeyD) { move_dir += Vec3::new(yaw_r.cos(), 0.0, yaw_r.sin()); }
        if move_dir.length_squared() > 0.0 {
            move_dir = move_dir.normalize() * 4.0 * dt;
            if let Some(p) = st.scene.get_object_mut(player_id) {
                p.transform.position += move_dir;
            }
        }

        if st.playing {
            st.frame_count += 1;

            // Düşman yapay zekası: oyuncuya yönel + temas hasarı
            let player_pos = st
                .scene
                .get_object(player_id)
                .map(|o| o.transform.position)
                .unwrap_or(Vec3::ZERO);

            for obj in &mut st.scene.objects {
                if obj.team != EntityTeam::Enemy || obj.health <= 0.0 { continue; }
                obj.transform.rotation.y += dt * 40.0;
                let to_player = player_pos - obj.transform.position;
                let dist = to_player.length();
                if dist > 1.2 && dist < 50.0 {
                    obj.transform.position += to_player.normalize() * dt * 2.5;
                } else if dist <= 1.2 {
                    obj.health = (obj.health - 25.0 * dt).max(0.0);
                    if obj.health <= 0.0 {
                        // Patlama efekti + uzakta respawn
                        self.renderer.spawn_particles(obj.transform.position, 30, [1.0, 0.4, 0.15]);
                        let ang = rand::random::<f32>() * std::f32::consts::TAU;
                        obj.transform.position = Vec3::new(ang.cos() * 14.0, 1.0, ang.sin() * 14.0);
                        obj.health = obj.max_health;
                    }
                }
            }

            // Fizik simülasyonu
            st.scene.physics_step(dt);

            // Skeletal animasyon güncelle
            if st.anim_playing {
                st.anim_time += dt * st.anim_speed;
                st.blend_machine.update(dt);
                st.blend_machine.apply_to_skeleton(&st.animation_clips, &mut st.skeleton);
            }

            // GPU parçacık simülasyonu (CPU fallback)
            st.particle_sim.params.time += dt;
            st.particle_sim.simulate(dt);
            st.gpu_particles_active = st.particle_sim.alive_count();
        }

        // Frustum culling her kare çalışır
        {
            let aspect = self.renderer.viewport_size.0 as f32 / self.renderer.viewport_size.1 as f32;
            let view = st.scene.camera.view_matrix();
            let proj = st.scene.camera.projection_matrix(aspect);
            let vp = proj * view;
            let cols = vp.to_cols_array();
            let mut view_proj = [[0.0f32; 4]; 4];
            for r in 0..4 { for c in 0..4 { view_proj[r][c] = cols[r * 4 + c]; } }
            st.frustum_culler.extract_frustum_planes(&view_proj);
            // Basit meshlet verileri oluştur (her obje = 1 meshlet)
            let meshlets: Vec<gpu_compute::MeshletData> = st.scene.objects.iter()
                .filter(|o| o.visible)
                .map(|o| gpu_compute::MeshletData {
                    center: o.transform.position.to_array(),
                    radius: 1.5,
                    ..Default::default()
                }).collect();
            st.frustum_culler.cull_meshlets(&meshlets);
            let (vis, culled, _) = st.frustum_culler.stats();
            st.gpu_visible = vis;
            st.gpu_culled = culled;
        }

        // Kamera takibi (her zaman)
        if let Some(p) = st.scene.get_object(player_id) {
            st.scene.camera.target = p.transform.position;
        }

        // Uzantı panellerini güncelle
        self.state.extensions.update(dt);
    }

    // ── Çizim ──────────────────────────────────────────────────
    fn redraw(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        if dt > 0.0001 {
            self.fps_ema = self.fps_ema * 0.9 + (1.0 / dt) * 0.1;
        }
        self.update_game(dt);
        self.state.can_undo = self.history.can_undo();
        self.state.can_redo = self.history.can_redo();

        let win_w = self.renderer.width as i32;
        let win_h = self.renderer.height as i32;
        let layout = compute_layout(win_w, win_h);

        // 3D viewport render (viewport boyutu/offset'i projeksiyona iletilir,
        // framebuffer tam pencere boyutunda kalır)
        self.renderer.viewport_offset = (layout.viewport[0], layout.viewport[1]);
        self.renderer.viewport_size = (layout.viewport[2].max(1) as u32, layout.viewport[3].max(1) as u32);
        {
            let selected = self.state.selected;
            let scene = &self.state.scene;
            self.renderer.set_grid_enabled(self.state.show_grid);
            self.renderer.render_scene(scene, selected, dt);
        }

        // UI overlay
        let hover = self.last_mouse.map(|(x, y)| self.ui_layout.button_at(x, y)).flatten();
        let status: Option<(String, bool)> = if Instant::now() < self.status_until {
            Some((self.status_message.clone(), !self.status_message.contains("hatası")))
        } else {
            None
        };
        if let Some(font) = &mut self.font {
            let st = &self.state;
            self.ui_layout = draw_ui(
                &mut self.renderer,
                font,
                win_w,
                win_h,
                st,
                &layout,
                hover.as_deref(),
                status.as_ref().map(|(m, ok)| (m.as_str(), *ok)),
            );
            self.state.extensions.draw_extensions_panels(
                &mut self.renderer,
                font,
                layout.right_panel,
                hover.as_deref(),
                &mut self.ui_layout,
            );
        }

        self.present();
        if let Some(w) = &self.window { w.request_redraw(); }
    }


    fn present(&mut self) {
        let (device, queue, surface, config) = match (
            &self.device, &self.queue, &self.surface, &self.config,
        ) {
            (Some(d), Some(q), Some(s), Some(c)) => (d, q, s, c),
            _ => return,
        };
        let frame = match surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost) => {
                surface.configure(device, config);
                return;
            }
            Err(_) => return,
        };
        let view = frame.texture.create_view(&Default::default());

        let w = self.renderer.width;
        let h = self.renderer.height;
        // Not: ara texture Rgba8 — swapchain dönüşümünü donanım yapar,
        // manuel BGRA swap gerekmez (çift swap renkleri bozar).
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: self.texture.as_ref().unwrap(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            self.renderer.frame_buffer(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );

        let mut enc = device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(self.pipeline.as_ref().unwrap());
            pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);
            pass.draw(0..6, 0..1);
        }
        queue.submit(Some(enc.finish()));
        frame.present();
    }

    // ── Etkileşim ──────────────────────────────────────────────
    fn on_mouse_down(&mut self, x: f64, y: f64) {
        let layout = compute_layout(self.renderer.width as i32, self.renderer.height as i32);

        // Konsol açıkken, konsol alanı tıklamalarını yakala
        if self.state.console.visible {
            let console_h = 180;
            let console_y = self.renderer.height as i32 - 26 - console_h;
            if y >= console_y as f64 && y < (console_y + console_h) as f64 {
                // Konsol input satırına tıklandı → odaklan
                let input_y = console_y + console_h - 22;
                if y >= input_y as f64 {
                    self.state.console_input.clear();
                    self.state.console_cursor = 0;
                }
                self.mouse_down = true;
                return;
            }
        }

        // Buton hit-test (tüm UI panelleri)
        if let Some(btn) = self.ui_layout.button_at(x, y) {
            self.handle_button(&btn);
            return;
        }
        // Inspector sürükleme başlat
        if let Some(drag) = self.ui_layout.drag_at(x, y) {
            self.history.push(&self.state.scene);
            self.state.active_drag = Some(drag);
            self.mouse_down = true;
            return;
        }
        // Viewport: tıklayarak seç / sürükleyerek orbit
        let (vx, vy, vw, vh) = (
            layout.viewport[0], layout.viewport[1],
            layout.viewport[2], layout.viewport[3],
        );
        if x >= vx as f64 && x < (vx + vw) as f64 && y >= vy as f64 && y < (vy + vh) as f64 {
            let ctrl = self.keys.contains(&KeyCode::ControlLeft) || self.keys.contains(&KeyCode::ControlRight);

            // Gizmo hit test
            if let Some(axis) = self.state.gizmo.hit_test(
                x, y, &self.state.scene.camera, vw as f32, vh as f32,
            ) {
                self.state.gizmo.start_drag(axis, x, y);
                self.history.push(&self.state.scene);
                self.mouse_down = true;
                return;
            }

            // Ctrl+drag = box selection başlat
            if ctrl {
                self.state.selection.start_box_selection(x, y);
                self.mouse_down = true;
                return;
            }

            self.orbiting = true;

            // Seçim ray'i
            let nx = ((x - vx as f64) / vw as f64) * 2.0 - 1.0;
            let ny = 1.0 - ((y - vy as f64) / vh as f64) * 2.0;
            if let Some((origin, dir)) =
                screen_ray(&self.state.scene.camera, nx as f32, ny as f32, vw as f32 / vh as f32)
            {
                if let Some(id) = self.state.scene.ray_intersect(origin, dir) {
                    if ctrl {
                        // Ctrl+tıkla → toggle selection
                        self.state.selection.toggle_selection(id);
                        self.state.selected = self.state.selection.primary_selection;
                    } else {
                        self.state.selection.select_single(id);
                        self.state.selected = Some(id);
                    }
                } else {
                    if !ctrl {
                        self.state.selection.clear_selection();
                        self.state.selected = None;
                    }
                }
            }
        }
        self.mouse_down = true;
    }


    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        self.status_until = Instant::now() + std::time::Duration::from_secs(2);
    }

    fn handle_button(&mut self, id: &str) {
        // Sahne değiştiren işlemlerden önce anlık görüntü al
        let snapshot_ops = [
            "add_cube", "add_sphere", "add_cylinder", "add_capsule", "del",
            "save", "load", "add_dynamic", "add_light", "gen_terrain", "import_obj", "import_glb", "import_texture",
        ];
        if snapshot_ops.contains(&id) {
            self.history.push(&self.state.scene);
        }
        match id {
            "play" => self.state.playing = true,
            "pause" => self.state.playing = false,
            "add_cube" => {
                let id = self.state.scene.add_object("Cube".into(), GeometryType::Cube, EntityTeam::Neutral);
                self.state.selected = Some(id);
            }
            "add_sphere" => {
                let id = self.state.scene.add_object("Sphere".into(), GeometryType::Sphere, EntityTeam::Neutral);
                self.state.selected = Some(id);
            }
            "add_cylinder" => {
                let id = self.state.scene.add_object("Cylinder".into(), GeometryType::Cylinder, EntityTeam::Neutral);
                self.state.selected = Some(id);
            }
            "add_capsule" => {
                let id = self.state.scene.add_object("Capsule".into(), GeometryType::Capsule, EntityTeam::Neutral);
                self.state.selected = Some(id);
            }
            "del" => {
                if let Some(id) = self.state.selected {
                    let pos = self.state.scene.get_object(id).map(|o| o.transform.position);
                    if let Some(pos) = pos {
                        self.renderer.spawn_particles(pos, 40, [1.0, 0.6, 0.2]);
                    }
                    self.state.scene.objects.retain(|o| o.id != id);
                    self.state.selected = None;
                    self.set_status("Varlık silindi");
                }
            }
            "reset_cam" => {
                self.state.scene.camera = Camera::new();
                self.set_status("Kamera sıfırlandı");
            }
            "grid" => self.state.show_grid = !self.state.show_grid,
            "anim_play" => {
                self.state.anim_playing = !self.state.anim_playing;
                if self.state.anim_playing {
                    self.state.blend_machine.transition_to_by_name("Idle");
                    self.set_status("Animasyon oynatılıyor");
                } else {
                    self.set_status("Animasyon duraklatıldı");
                }
            }
            "anim_idle" => {
                self.state.blend_machine.transition_to_by_name("Idle");
                self.set_status("Idle animasyonuna geçildi");
            }
            "anim_walk" => {
                self.state.blend_machine.transition_to_by_name("Walk");
                self.set_status("Walk animasyonuna geçildi");
            }
            "anim_run" => {
                self.state.blend_machine.transition_to_by_name("Run");
                self.set_status("Run animasyonuna geçildi");
            },
            "save" => {
                // Her iki format'ta da kaydet
                let json_result = crate::serialization::save_scene(&self.state.scene, "scene.json");
                let bin_result = crate::serialization::save_scene(&self.state.scene, "scene.bin");
                let (json_size, bin_size) = crate::serialization::compare_formats(&self.state.scene);
                match (json_result, bin_result) {
                    (Ok(()), Ok(())) => {
                        self.set_status(format!("Kaydedildi: scene.json ({}, JSON) + scene.bin ({}, Binary)",
                            format_bytes(json_size), format_bytes(bin_size)));
                    }
                    (Ok(()), Err(e)) => {
                        self.set_status(format!("JSON OK, Binary hata: {} — scene.json kaydedildi", e));
                    }
                    (Err(e), _) => self.set_status(format!("Kaydetme hatası: {e}")),
                }
            }
            "load" => {
                // JSON'ı dene, olmazsa binary'yi dene
                match crate::serialization::load_scene("scene.json") {
                    Ok(scene) => {
                        let n = scene.objects.len();
                        self.state.scene = scene;
                        self.state.selected = None;
                        self.set_status(format!("Sahne yüklendi: scene.json ({n} varlık)"));
                    }
                    Err(_) => {
                        match crate::serialization::load_scene("scene.bin") {
                            Ok(scene) => {
                                let n = scene.objects.len();
                                self.state.scene = scene;
                                self.state.selected = None;
                                self.set_status(format!("Sahne yüklendi: scene.bin ({n} varlık)"));
                            }
                            Err(e) => self.set_status(format!("Yükleme hatası: {e}")),
                        }
                    }
                }
            },
            "undo" => {
                if self.history.undo(&mut self.state.scene) {
                    self.set_status("Geri alındı");
                } else {
                    self.set_status("Geri alınacak işlem yok");
                }
            }
            "redo" => {
                if self.history.redo(&mut self.state.scene) {
                    self.set_status("Yinelendi");
                } else {
                    self.set_status("Yinelenecek işlem yok");
                }
            }
            "add_dynamic" => {
                let id = self.state.scene.add_object("Dynamic Sphere".into(), GeometryType::Sphere, EntityTeam::Neutral);
                if let Some(obj) = self.state.scene.get_object_mut(id) {
                    obj.transform.position = Vec3::new(0.0, 8.0, 0.0);
                    obj.rigid_body = Some(crate::renderer::RigidBody {
                        body_type: crate::renderer::RigidBodyType::Dynamic,
                        mass: 2.0,
                        restitution: 0.6,
                        ..Default::default()
                    });
                    obj.material.metallic = 0.8;
                    obj.material.roughness = 0.2;
                }
                self.state.selected = Some(id);
                self.set_status("Dinamik küre eklendi (fizik aktif)");
            }
            "add_light" => {
                let id = self.state.scene.add_object("Point Light".into(), GeometryType::Sphere, EntityTeam::Neutral);
                if let Some(obj) = self.state.scene.get_object_mut(id) {
                    obj.transform.position = Vec3::new(0.0, 5.0, 0.0);
                    obj.transform.scale = Vec3::splat(0.2);
                    obj.material.emissive = Vec3::new(1.0, 0.9, 0.7);
                    obj.material.emissive_strength = 5.0;
                    obj.is_light = true;
                    let light_pos = obj.transform.position;
                    self.state.scene.lights.push(crate::renderer::SceneLight::point(
                        light_pos, Vec3::new(1.0, 0.9, 0.7), 3.0, 15.0,
                    ));
                }
                self.state.selected = Some(id);
                self.set_status("Nokta ışığı eklendi");
            }
            "gen_terrain" => {
                let (verts, tris) = crate::renderer::Scene::generate_terrain_mesh(40.0, 32, 3.0, 42);
                // Terrain'i ground objesine uygula
                if let Some(ground) = self.state.scene.objects.iter_mut().find(|o| o.name == "Ground") {
                    ground.name = "Terrain".into();
                }
                self.set_status(format!("Terrain üretildi: {} vertex, {} üçgen", verts.len(), tris.len()));
            }
            "toggle_physics" => {
                self.state.scene.physics_enabled = !self.state.scene.physics_enabled;
                let s = if self.state.scene.physics_enabled { "AÇIK" } else { "KAPALI" };
                self.set_status(format!("Fizik: {}", s));
            }
            "toggle_console" => {
                self.state.console.toggle();
            }
            "console_close" => {
                self.state.console.visible = false;
            }
            "import_obj" => {
                // Demo: baseline bir OBJ verisi oluştur ve import et
                let demo_obj = r#"# Demo OBJ cube
o DemoCube
v -0.5 -0.5 0.5
v 0.5 -0.5 0.5
v 0.5 0.5 0.5
v -0.5 0.5 0.5
v -0.5 -0.5 -0.5
v 0.5 -0.5 -0.5
v 0.5 0.5 -0.5
v -0.5 0.5 -0.5
f 1 2 3 4
f 5 8 7 6
f 1 5 6 2
f 2 6 7 3
f 3 7 8 4
f 4 8 5 1
"#;
                match self.state.scene.import_obj(demo_obj) {
                    Ok(mesh_id) => {
                        let name = format!("Imported OBJ {}", mesh_id);
                        let id = self.state.scene.add_custom_object(
                            name, mesh_id, crate::renderer::EntityTeam::Neutral,
                        );
                        self.state.selected = Some(id);
                        self.set_status(format!("OBJ import edildi (mesh_id={})", mesh_id));
                    }
                    Err(e) => self.set_status(format!("Import hatası: {}", e)),
                }
            }
            "import_glb" => {
                // Demo: basit bir GLB cube oluştur ve import et
                match create_demo_glb() {
                    Ok(glb_data) => {
                        match self.state.scene.import_glb(&glb_data) {
                            Ok(mesh_id) => {
                                let name = format!("Imported GLB {}", mesh_id);
                                let id = self.state.scene.add_custom_object(
                                    name, mesh_id, crate::renderer::EntityTeam::Neutral,
                                );
                                self.state.selected = Some(id);
                                self.set_status(format!("GLB import edildi (mesh_id={})", mesh_id));
                            }
                            Err(e) => self.set_status(format!("GLB import hatası: {}", e)),
                        }
                    }
                    Err(e) => self.set_status(format!("GLB oluşturma hatası: {}", e)),
                }
            }
            "import_texture" => {
                // Demo: procedural texture oluştur ve ata
                let tex = crate::texture::Texture::checkerboard(
                    "demo_checker", 32, 8,
                    [200, 180, 140], [80, 60, 40],
                );
                let tex_id = self.state.scene.next_texture_id;
                self.state.scene.next_texture_id += 1;
                self.state.scene.texture_names.insert(tex_id, "demo_checker".into());
                self.state.scene.textures.insert(tex_id, tex);

                // Seçili nesneye ata
                if let Some(obj_id) = self.state.selected {
                    if let Ok(()) = self.state.scene.assign_texture_to_object(obj_id, tex_id, "albedo") {
                        self.set_status(format!("Texture atandı (tex_id={})", tex_id));
                    } else {
                        self.set_status(format!("Texture oluşturuldu (id={}), nesne seçili değil", tex_id));
                    }
                } else {
                    self.set_status(format!("Texture oluşturuldu (id={}), nesne seçili değil", tex_id));
                }
            }
            other => {
                if let Some(ent_id) = other.strip_prefix("ent_").and_then(|s| s.parse::<usize>().ok()) {
                    self.state.selected = Some(ent_id);
                } else {
                    crate::editor_extensions::handle_extension_button(&mut self.state.extensions, other);
                }
            }
        }
    }

    fn on_mouse_move(&mut self, x: f64, y: f64) {
        let dx = self.last_mouse.map(|(lx, _)| x - lx).unwrap_or(0.0);
        let dy = self.last_mouse.map(|(_, ly)| y - ly).unwrap_or(0.0);
        self.last_mouse = Some((x, y));

        if self.mouse_down {
            // Gizmo sürükleme
            if self.state.gizmo.is_dragging {
                if let Some(delta) = self.state.gizmo.drag_delta(x, y) {
                    let layout = compute_layout(self.renderer.width as i32, self.renderer.height as i32);
                    let vw = layout.viewport[2] as f32;
                    let vh = layout.viewport[3] as f32;
                    let (dpos, drot, dscl) = self.state.gizmo.screen_delta_to_transform(
                        delta.0, delta.1, &self.state.scene.camera, vw, vh,
                    );
                    // Seçili tüm nesnelere uygula
                    selection_gizmo::apply_multi_transform(
                        &mut self.state.scene,
                        &self.state.selection.selected_ids,
                        dpos, drot, dscl,
                    );
                    self.state.gizmo.drag_start = Some((x, y));
                }
                return;
            }

            // Box selection güncelle
            if self.state.selection.box_selection.is_some() {
                self.state.selection.update_box_selection(x, y);
                return;
            }

            // Inspector drag-edit
            if let Some(drag) = self.state.active_drag {
                let sens = match drag.comp {
                    EditComp::Rotation => 0.5,
                    _ => 0.02,
                };
                if let Some(sel) = self.state.selected {
                    if let Some(obj) = self.state.scene.get_object_mut(sel) {
                        let v = match drag.comp {
                            EditComp::Position => &mut obj.transform.position,
                            EditComp::Rotation => &mut obj.transform.rotation,
                            EditComp::Scale => &mut obj.transform.scale,
                        };
                        v[drag.axis] += (dx as f32) * sens;
                    }
                }
                return;
            }
        }
        // Orbit
        if self.orbiting && self.mouse_down {
            self.state.scene.camera.orbit(dx as f32 * 0.35, dy as f32 * 0.35);
        }
    }

    fn on_mouse_up(&mut self) {
        // Box selection bitir
        if self.state.selection.box_selection.is_some() {
            let layout = compute_layout(self.renderer.width as i32, self.renderer.height as i32);
            let vw = layout.viewport[2] as f32;
            let vh = layout.viewport[3] as f32;
            let ctrl = self.keys.contains(&KeyCode::ControlLeft) || self.keys.contains(&KeyCode::ControlRight);
            self.state.selection.finish_box_selection(
                &self.state.scene, &self.state.scene.camera.clone(), vw, vh, ctrl,
            );
            self.state.selected = self.state.selection.primary_selection;
        }

        // Gizmo drag bitir
        if self.state.gizmo.is_dragging {
            self.state.gizmo.end_drag();
        }

        self.mouse_down = false;
        self.orbiting = false;
        self.state.active_drag = None;
    }

    fn on_scroll(&mut self, delta_y: f32) {
        self.state.scene.camera.zoom(delta_y * 0.8);
    }

    fn on_key(&mut self, code: KeyCode, pressed: bool, event_loop: &ActiveEventLoop) {
        // Konsolvisible ise, tuşları konsola yönlendir
        if self.state.console.visible && pressed {
            match code {
                KeyCode::Enter => {
                    let input = self.state.console_input.clone();
                    self.state.console.execute(&input);
                    self.state.console_input.clear();
                    self.state.console_cursor = 0;
                    return;
                }
                KeyCode::Backspace => {
                    if self.state.console_cursor > 0 {
                        self.state.console_cursor -= 1;
                        self.state.console_input.remove(self.state.console_cursor);
                    }
                    return;
                }
                KeyCode::ArrowUp => {
                    if let Some(cmd) = self.state.console.history_up() {
                        self.state.console_input = cmd;
                        self.state.console_cursor = self.state.console_input.len();
                    }
                    return;
                }
                KeyCode::ArrowDown => {
                    if let Some(cmd) = self.state.console.history_down() {
                        self.state.console_input = cmd;
                        self.state.console_cursor = self.state.console_input.len();
                    }
                    return;
                }
                KeyCode::Escape => {
                    self.state.console.visible = false;
                    return;
                }
                KeyCode::Tab => {
                    let partial = self.state.console_input.clone();
                    let completions = self.state.console.autocomplete(&partial);
                    if completions.len() == 1 {
                        self.state.console_input = completions[0].clone();
                        self.state.console_cursor = self.state.console_input.len();
                    } else if completions.len() > 1 {
                        self.state.console.push_line(
                            console::ConsoleLine::Info(completions.join("  "))
                        );
                    }
                    return;
                }
                _ => { return; } // Konsol açıkken diğer tüm tuşları yut
            }
        }

        match code {
            KeyCode::Space if pressed => self.state.playing = !self.state.playing,
            KeyCode::Backquote if pressed => { self.state.console.visible = !self.state.console.visible; }
            KeyCode::KeyR if pressed => self.state.scene.camera = Camera::new(),
            KeyCode::Delete if pressed => {
                // Çoklu silme desteği
                let ids: Vec<usize> = self.state.selection.selected_ids.clone();
                if !ids.is_empty() {
                    selection_gizmo::delete_selected(&mut self.state.scene, &ids);
                    self.state.selection.clear_selection();
                    self.state.selected = None;
                    self.set_status(format!("{} nesne silindi", ids.len()));
                }
            }
            // Gizmo modu kısayolları
            KeyCode::KeyG if pressed && !self.keys.contains(&KeyCode::ControlLeft) => {
                self.state.gizmo.mode = selection_gizmo::GizmoMode::Translate;
                self.set_status("Gizmo: Move (G)");
            }
            KeyCode::KeyR if pressed && !self.keys.contains(&KeyCode::ControlLeft) => {
                // Ctrl+R kamera reset, R gizmo rotate
                if self.keys.contains(&KeyCode::ControlLeft) || self.keys.contains(&KeyCode::ControlRight) {
                    self.state.scene.camera = Camera::new();
                } else {
                    self.state.gizmo.mode = selection_gizmo::GizmoMode::Rotate;
                    self.set_status("Gizmo: Rotate (R)");
                }
            }
            KeyCode::KeyS if pressed && !self.keys.contains(&KeyCode::ControlLeft) => {
                self.state.gizmo.mode = selection_gizmo::GizmoMode::Scale;
                self.set_status("Gizmo: Scale (S)");
            }
            // Seçili nesneye odaklan
            KeyCode::KeyF if pressed => {
                if let Some(id) = self.state.selected {
                    let focus_info = self.state.scene.get_object(id)
                        .map(|o| (o.transform.position, o.name.clone()));
                    if let Some((pos, name)) = focus_info {
                        self.state.scene.camera.target = pos;
                        self.set_status(format!("Odaklandı: {}", name));
                    }
                }
            }
            // Tümünü seç (Ctrl+A)
            KeyCode::KeyA if pressed => {
                if self.keys.contains(&KeyCode::ControlLeft) || self.keys.contains(&KeyCode::ControlRight) {
                    self.state.selection.select_all(&self.state.scene);
                    self.state.selected = self.state.selection.primary_selection;
                    self.set_status(format!("{} nesne seçildi", self.state.selection.selection_count()));
                }
            }
            KeyCode::Escape if pressed => event_loop.exit(),
            KeyCode::KeyZ if pressed => {
                if self.keys.contains(&KeyCode::ControlLeft) || self.keys.contains(&KeyCode::ControlRight) {
                    if self.keys.contains(&KeyCode::ShiftLeft) || self.keys.contains(&KeyCode::ShiftRight) {
                        if self.history.redo(&mut self.state.scene) { self.set_status("Yinelendi"); }
                    } else if self.history.undo(&mut self.state.scene) {
                        self.set_status("Geri alındı");
                    }
                }
            }
            KeyCode::KeyY if pressed => {
                if self.history.redo(&mut self.state.scene) { self.set_status("Yinelendi"); }
            }
            // Uzantı paneli kısayolları (F1-F10)
            KeyCode::F1 if pressed => toggle_panel(&mut self.state.extensions, crate::editor_extensions::PanelId::AudioMixer),
            KeyCode::F2 if pressed => toggle_panel(&mut self.state.extensions, crate::editor_extensions::PanelId::SaveManager),
            KeyCode::F3 if pressed => toggle_panel(&mut self.state.extensions, crate::editor_extensions::PanelId::InputRebind),
            KeyCode::F4 if pressed => toggle_panel(&mut self.state.extensions, crate::editor_extensions::PanelId::Localization),
            KeyCode::F5 if pressed => toggle_panel(&mut self.state.extensions, crate::editor_extensions::PanelId::VRSettings),
            KeyCode::F6 if pressed => toggle_panel(&mut self.state.extensions, crate::editor_extensions::PanelId::ModManager),
            KeyCode::F7 if pressed => toggle_panel(&mut self.state.extensions, crate::editor_extensions::PanelId::Debug),
            KeyCode::F8 if pressed => toggle_panel(&mut self.state.extensions, crate::editor_extensions::PanelId::AINav),
            KeyCode::F9 if pressed => toggle_panel(&mut self.state.extensions, crate::editor_extensions::PanelId::Settings),
            KeyCode::F10 if pressed => toggle_panel(&mut self.state.extensions, crate::editor_extensions::PanelId::Profiler),
            _ => {}
        }
        if pressed {
            self.keys.insert(code);
        } else {
            self.keys.remove(&code);
        }
    }

    /// Konsol klavye girdisini işle (winit window_event'ten çağrılır)
    fn on_char_input(&mut self, ch: char) {
        if self.state.console.visible && !ch.is_control() {
            self.state.console_input.insert(self.state.console_cursor, ch);
            self.state.console_cursor += 1;
        }
    }
}

// yardımcı: Ground scale ayarı için küçük extension
trait ThenSetScale {
    fn then_set_scale(self, scene: &mut Scene) -> usize;
}
impl ThenSetScale for usize {
    fn then_set_scale(self, scene: &mut Scene) -> usize {
        if let Some(g) = scene.get_object_mut(self) {
            g.transform.position = Vec3::new(0.0, 0.0, 0.0);
            g.transform.scale = Vec3::new(12.0, 1.0, 12.0);
        }
        self
    }
}

/// Zengin demo sahne oluştur — başlatıldığında tüm motor özelliklerini sergiler
fn create_demo_scene() -> Scene {
    use std::f32::consts::PI;
    let mut scene = Scene::new();

    // ── Oyuncu (mavi kapsül)
    let player = scene.add_object("Player".into(), GeometryType::Capsule, EntityTeam::Player);
    if let Some(obj) = scene.get_object_mut(player) {
        obj.transform.position = Vec3::new(0.0, 1.5, 0.0);
        obj.material.metallic = 0.3;
        obj.material.roughness = 0.4;
    }

    // ── Düşmanlar (kırmızı küpler, farklı boyut ve pozisyonlarda)
    for i in 0..5 {
        let e = scene.add_object(format!("Enemy {}", i + 1), GeometryType::Cube, EntityTeam::Enemy);
        let angle = i as f32 * (2.0 * PI / 5.0);
        let radius = 7.0 + i as f32 * 1.5;
        if let Some(obj) = scene.get_object_mut(e) {
            obj.transform.position = Vec3::new(angle.cos() * radius, 1.0, angle.sin() * radius);
            obj.transform.scale = Vec3::splat(0.8 + i as f32 * 0.3);
            obj.health = 50.0 + i as f32 * 50.0;
            obj.max_health = 50.0 + i as f32 * 50.0;
            obj.rigid_body = Some(crate::renderer::RigidBody {
                body_type: crate::renderer::RigidBodyType::Dynamic,
                mass: 1.0 + i as f32,
                restitution: 0.3 + i as f32 * 0.1,
                ..Default::default()
            });
        }
    }

    // ── Objeler — geometri vitrini
    // Sol tarafta silindirler
    for i in 0..3 {
        let c = scene.add_object(format!("Pillar {}", i + 1), GeometryType::Cylinder, EntityTeam::Neutral);
        if let Some(obj) = scene.get_object_mut(c) {
            obj.transform.position = Vec3::new(-5.0, 1.5, -3.0 + i as f32 * 3.0);
            obj.transform.scale = Vec3::new(0.4, 2.0, 0.4);
            obj.material.metallic = 0.9;
            obj.material.roughness = 0.1;
            obj.material.emissive = Vec3::new(0.1, 0.3, 0.8);
            obj.material.emissive_strength = 0.5;
        }
    }

    // Sağ tarafta kapsüller
    for i in 0..3 {
        let c = scene.add_object(format!("Pod {}", i + 1), GeometryType::Capsule, EntityTeam::Neutral);
        if let Some(obj) = scene.get_object_mut(c) {
            obj.transform.position = Vec3::new(5.0, 1.0, -3.0 + i as f32 * 3.0);
            obj.transform.scale = Vec3::new(0.5, 1.0, 0.5);
            obj.material.albedo = Vec3::new(0.2, 0.8, 0.3);
            obj.material.metallic = 0.0;
            obj.material.roughness = 0.8;
        }
    }

    // Ortada büyük küre (metal)
    let sphere = scene.add_object("Orb".into(), GeometryType::Sphere, EntityTeam::Neutral);
    if let Some(obj) = scene.get_object_mut(sphere) {
        obj.transform.position = Vec3::new(0.0, 3.0, -5.0);
        obj.transform.scale = Vec3::splat(1.5);
        obj.material.metallic = 1.0;
        obj.material.roughness = 0.05;
        obj.material.albedo = Vec3::new(0.95, 0.85, 0.4);
    }

    // ── Işıklar
    // Ana güneş
    scene.lights.push(crate::renderer::SceneLight::directional(
        Vec3::new(0.4, 0.8, 0.3).normalize(),
        Vec3::new(1.0, 0.95, 0.85),
        1.8,
    ));
    // Dolgu ışığı
    scene.lights.push(crate::renderer::SceneLight::directional(
        Vec3::new(-0.3, 0.5, -0.6).normalize(),
        Vec3::new(0.3, 0.4, 0.6),
        0.5,
    ));
    // Nokta ışığı — orb yakınında sıcak ışık
    scene.lights.push(crate::renderer::SceneLight::point(
        Vec3::new(0.0, 5.0, -5.0),
        Vec3::new(1.0, 0.8, 0.4),
        4.0, 20.0,
    ));
    // Mavi dekoratif ışık
    scene.lights.push(crate::renderer::SceneLight::point(
        Vec3::new(-5.0, 3.0, 0.0),
        Vec3::new(0.2, 0.4, 1.0),
        3.0, 15.0,
    ));
    // Yeşil dekoratif ışık
    scene.lights.push(crate::renderer::SceneLight::point(
        Vec3::new(5.0, 3.0, 0.0),
        Vec3::new(0.2, 1.0, 0.4),
        3.0, 15.0,
    ));

    // ── Zemin
    scene.add_object("Ground".into(), GeometryType::Plane, EntityTeam::Neutral)
        .then_set_scale(&mut scene);

    // ── Texture'lar oluştur
    scene.create_default_textures();

    // ── Skeleton — humanoid demo
    let skel_id = scene.create_demo_skeleton();
    // Skeleton'u Player'a ata
    scene.assign_skeleton_to_object(player, skel_id).ok();
    // Walk animasyonunu başlat
    if let Some(bsm) = scene.blend_machines.get_mut(&skel_id) {
        bsm.transition_to_by_name("Walk");
    }

    // Sahne kamera ayarı
    scene.camera.distance = 20.0;
    scene.camera.yaw = -30.0;
    scene.camera.pitch = 25.0;
    scene.camera.target = Vec3::new(0.0, 1.5, 0.0);

    scene
}

/// Byte boyutunu okunabilir formata çevir
fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Demo GLB cube oluştur (test/demo için)
fn create_demo_glb() -> Result<Vec<u8>, String> {
    // Basit bir küp GLB oluştur
    let _cube_verts: [[f32; 3]; 8] = [
        [-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [-0.5, 0.5, -0.5],
        [-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5],
    ];
    let _cube_normals: [[f32; 3]; 6] = [
        [0.0, 0.0, -1.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0],
        [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [1.0, 0.0, 0.0],
    ];
    // 12 üçgen (36 vertex, her yüz için 4 unique)
    let positions: Vec<f32> = vec![
        // Front (z=-0.5)
        -0.5, -0.5, -0.5,  0.5, -0.5, -0.5,  0.5, 0.5, -0.5, -0.5, 0.5, -0.5,
        // Back (z=0.5)
        0.5, -0.5, 0.5, -0.5, -0.5, 0.5, -0.5, 0.5, 0.5, 0.5, 0.5, 0.5,
        // Bottom (y=-0.5)
        -0.5, -0.5, -0.5, -0.5, -0.5, 0.5, 0.5, -0.5, 0.5, 0.5, -0.5, -0.5,
        // Top (y=0.5)
        -0.5, 0.5, 0.5, -0.5, 0.5, -0.5, 0.5, 0.5, -0.5, 0.5, 0.5, 0.5,
        // Left (x=-0.5)
        -0.5, -0.5, -0.5, -0.5, 0.5, -0.5, -0.5, 0.5, 0.5, -0.5, -0.5, 0.5,
        // Right (x=0.5)
        0.5, -0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, -0.5, 0.5, -0.5, -0.5,
    ];
    let normals: Vec<f32> = vec![
        0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0,
        0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
        0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0,
        0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
        1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
    ];
    let indices: Vec<u32> = vec![
        0,1,2, 0,2,3, 4,5,6, 4,6,7,
        8,9,10, 8,10,11, 12,13,14, 12,14,15,
        16,17,18, 16,18,19, 20,21,22, 20,22,23,
    ];

    // GLB JSON oluştur
    let bin_size = positions.len() * 4 + normals.len() * 4 + indices.len() * 4;
    let pos_offset = 0;
    let norm_offset = positions.len() * 4;
    let idx_offset = norm_offset + normals.len() * 4;

    let json = format!(r#"{{"buffers":[{{"byteLength":{bin}}}],"bufferViews":[{{"buffer":0,"byteOffset":{po},"byteLength":{pl},"target":34962}},{{"buffer":0,"byteOffset":{no},"byteLength":{nl},"target":34962}},{{"buffer":0,"byteOffset":{io},"byteLength":{il},"target":34963}}],"accessors":[{{"bufferView":0,"byteOffset":0,"componentType":5126,"count":24,"type":"VEC3","max":[0.5,0.5,0.5],"min":[-0.5,-0.5,-0.5]}},{{"bufferView":1,"byteOffset":0,"componentType":5126,"count":24,"type":"VEC3","max":[0.0,1.0,0.0],"min":[-1.0,0.0,0.0]}},{{"bufferView":2,"byteOffset":0,"componentType":5125,"count":36,"type":"SCALAR"}}],"meshes":[{{"name":"DemoGLBCube","primitives":[{{"attributes":{{"POSITION":0,"NORMAL":1}},"indices":2,"mode":4}}]}}],"materials":[{{"name":"BluePlastic","pbrMetallicRoughness":{{"baseColorFactor":[0.2,0.4,0.8,1.0],"metallicFactor":0.1,"roughnessFactor":0.4}},"emissiveFactor":[0.0,0.0,0.0]}}],"scene":0,"scenes":[{{"nodes":[]}}]}}"#,
        bin = bin_size, po = pos_offset, pl = positions.len() * 4,
        no = norm_offset, nl = normals.len() * 4,
        io = idx_offset, il = indices.len() * 4
    );

    // Binary chunk oluştur
    let mut bin_data = Vec::new();
    for v in &positions { bin_data.extend_from_slice(&v.to_le_bytes()); }
    for v in &normals { bin_data.extend_from_slice(&v.to_le_bytes()); }
    for v in &indices { bin_data.extend_from_slice(&v.to_le_bytes()); }

    // GLB dosyası oluştur
    let mut data = Vec::new();
    // Header
    data.extend_from_slice(&0x46546C67u32.to_le_bytes()); // magic "glTF"
    data.extend_from_slice(&2u32.to_le_bytes()); // version 2
    let total = 12 + 8 + json.len() + 8 + bin_data.len();
    data.extend_from_slice(&(total as u32).to_le_bytes());
    // JSON chunk
    data.extend_from_slice(&(json.len() as u32).to_le_bytes());
    data.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // "JSON"
    data.extend_from_slice(json.as_bytes());
    // Binary chunk
    data.extend_from_slice(&(bin_data.len() as u32).to_le_bytes());
    data.extend_from_slice(&0x004E4942u32.to_le_bytes()); // "BIN\0"
    data.extend_from_slice(&bin_data);

    Ok(data)
}

/// Ekran koordinatından dünya ray'i üretir (picking).
fn screen_ray(cam: &Camera, ndc_x: f32, ndc_y: f32, aspect: f32) -> Option<(Vec3, Vec3)> {
    use glam::Vec4;
    let proj = cam.projection_matrix(aspect);
    let view = cam.view_matrix();
    let inv_vp = (proj * view).inverse();

    let near = inv_vp * Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
    let far = inv_vp * Vec4::new(ndc_x, ndc_y, 1.0, 1.0);
    if near.w.abs() < 1e-6 || far.w.abs() < 1e-6 { return None; }
    let near = near.truncate() / near.w;
    let far = far.truncate() / far.w;
    let dir = (far - near).normalize();
    Some((cam.position(), dir))
}

// ─────────────────────────────────────────────────────────── winit handler

impl ApplicationHandler for EditorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("Elysium Engine")
                .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));
            let window = Arc::new(event_loop.create_window(attrs).expect("window"));
            self.init_gpu(window);
            let size = self.window.as_ref().unwrap().inner_size();
            self.resize(size.width.max(1), size.height.max(1));
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(s) => self.resize(s.width, s.height),
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => match state {
                ElementState::Pressed => {
                    if let Some((x, y)) = self.last_mouse { self.on_mouse_down(x, y); }
                }
                ElementState::Released => self.on_mouse_up(),
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.last_mouse = Some((position.x, position.y));
                self.on_mouse_move(position.x, position.y);
            }
            WindowEvent::MouseWheel { delta: winit::event::MouseScrollDelta::LineDelta(_, y), .. } => {
                self.on_scroll(y);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // Konsolvisible ise ve text varsa, karakter girdisi olarak işle
                if self.state.console.visible {
                    if let Some(ref text) = event.text {
                        for ch in text.chars() {
                            if !ch.is_control() {
                                self.on_char_input(ch);
                            }
                        }
                    }
                }
                if let PhysicalKey::Code(code) = event.physical_key {
                    self.on_key(code, event.state == ElementState::Pressed, event_loop);
                }
            }
            _ => {}
        }
    }
}

fn main() {
    println!("Elysium Engine başlatılıyor...");
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = EditorApp::new();
    event_loop.run_app(&mut app).expect("run");
}
