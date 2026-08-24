//! Elysium Engine — gerçek zamanlı oyun motoru editörü.
//! winit + wgpu + yerleşik software rasterizer ve immediate-mode UI.

mod renderer;
mod editor_ui;
mod history;

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
        let mut scene = Scene::new();
        let player = scene.add_object("Player".into(), GeometryType::Capsule, EntityTeam::Player);
        scene.get_object_mut(player).unwrap().transform.position = Vec3::new(0.0, 1.0, 0.0);
        for i in 0..3 {
            let e = scene.add_object(format!("Enemy{}", i + 1), GeometryType::Cube, EntityTeam::Enemy);
            let ang = i as f32 * 2.1;
            scene.get_object_mut(e).unwrap().transform.position =
                Vec3::new(ang.cos() * 6.0, 1.0, ang.sin() * 6.0);
        }
        scene.add_object("Ground".into(), GeometryType::Plane, EntityTeam::Neutral)
            .then_set_scale(&mut scene);

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
        }

        // Kamera takibi (her zaman)
        if let Some(p) = st.scene.get_object(player_id) {
            st.scene.camera.target = p.transform.position;
        }
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
            draw_ui(
                &mut self.renderer,
                font,
                win_w,
                win_h,
                st,
                &layout,
                hover.as_deref(),
                status.as_ref().map(|(m, ok)| (m.as_str(), *ok)),
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

        // Buton hit-test
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
            self.orbiting = true;
            // Seçim ray'i
            let nx = ((x - vx as f64) / vw as f64) * 2.0 - 1.0;
            let ny = 1.0 - ((y - vy as f64) / vh as f64) * 2.0;
            if let Some((origin, dir)) =
                screen_ray(&self.state.scene.camera, nx as f32, ny as f32, vw as f32 / vh as f32)
            {
                if let Some(id) = self.state.scene.ray_intersect(origin, dir) {
                    self.state.selected = Some(id);
                } else {
                    self.state.selected = None;
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
            "save", "load",
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
            "save" => match self.state.scene.save_to_file("scene.json") {
                Ok(()) => self.set_status("Sahne kaydedildi: scene.json"),
                Err(e) => self.set_status(format!("Kaydetme hatası: {e}")),
            },
            "load" => match Scene::load_from_file("scene.json") {
                Ok(scene) => {
                    let n = scene.objects.len();
                    self.state.scene = scene;
                    self.state.selected = None;
                    self.set_status(format!("Sahne yüklendi ({n} varlık)"));
                }
                Err(e) => self.set_status(format!("Yükleme hatası: {e}")),
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
            other => {
                if let Some(ent_id) = other.strip_prefix("ent_").and_then(|s| s.parse::<usize>().ok()) {
                    self.state.selected = Some(ent_id);
                }
            }
        }
    }

    fn on_mouse_move(&mut self, x: f64, y: f64) {
        let dx = self.last_mouse.map(|(lx, _)| x - lx).unwrap_or(0.0);
        let dy = self.last_mouse.map(|(_, ly)| y - ly).unwrap_or(0.0);
        self.last_mouse = Some((x, y));

        // Inspector drag-edit
        if self.mouse_down {
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
        self.mouse_down = false;
        self.orbiting = false;
        self.state.active_drag = None;
    }

    fn on_scroll(&mut self, delta_y: f32) {
        self.state.scene.camera.zoom(delta_y * 0.8);
    }

    fn on_key(&mut self, code: KeyCode, pressed: bool, event_loop: &ActiveEventLoop) {
        match code {
            KeyCode::Space if pressed => self.state.playing = !self.state.playing,
            KeyCode::KeyR if pressed => self.state.scene.camera = Camera::new(),
            KeyCode::Delete if pressed => {
                if let Some(id) = self.state.selected.take() {
                    self.state.scene.objects.retain(|o| o.id != id);
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
            _ => {}
        }
        if pressed {
            self.keys.insert(code);
        } else {
            self.keys.remove(&code);
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
