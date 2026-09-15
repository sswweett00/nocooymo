/// Elysium Web — WebAssembly/WebGL Game Engine
/// Tam oyun motoru: PBR rendering, fizik, input, sahne yönetimi

mod scene;
mod renderer;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext, WebGlProgram, WebGlShader,
    WebGlBuffer, WebGlTexture,
    KeyboardEvent, MouseEvent, WheelEvent,
};
use std::cell::RefCell;
use std::rc::Rc;

use scene::*;
use renderer::WebRenderer;

// ═══════════════════════════════════════════════════════════ WebGL2 Helpers

fn compile_shader(gl: &WebGl2RenderingContext, shader_type: u32, source: &str) -> Result<WebGlShader, JsValue> {
    let shader = gl.create_shader(shader_type).ok_or_else(|| JsValue::from_str("create_shader failed"))?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    let ok = gl.get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool().unwrap_or(false);
    if ok { Ok(shader) } else {
        let log = gl.get_shader_info_log(&shader).unwrap_or_default();
        gl.delete_shader(Some(&shader));
        Err(JsValue::from_str(&log))
    }
}

fn link_program(gl: &WebGl2RenderingContext, vs: &WebGlShader, fs: &WebGlShader) -> Result<WebGlProgram, JsValue> {
    let prog = gl.create_program().ok_or_else(|| JsValue::from_str("create_program failed"))?;
    gl.attach_shader(&prog, vs);
    gl.attach_shader(&prog, fs);
    gl.link_program(&prog);
    let ok = gl.get_program_parameter(&prog, WebGl2RenderingContext::LINK_STATUS)
        .as_bool().unwrap_or(false);
    if ok { Ok(prog) } else {
        let log = gl.get_program_info_log(&prog).unwrap_or_default();
        gl.delete_program(Some(&prog));
        Err(JsValue::from_str(&log))
    }
}

// Fullscreen quad shaders
const QUAD_VS: &str = r#"#version 300 es
in vec2 a_pos;
in vec2 a_uv;
out vec2 v_uv;
void main() {
    v_uv = a_uv;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

const QUAD_FS: &str = r#"#version 300 es
precision mediump float;
in vec2 v_uv;
out vec4 fragColor;
uniform sampler2D u_tex;
void main() {
    fragColor = texture(u_tex, v_uv);
}
"#;

// ═══════════════════════════════════════════════════════════ Engine State

struct EngineState {
    renderer: WebRenderer,
    scene: Scene,
    selected: Option<usize>,
    playing: bool,
    keys: Vec<u32>,
    mouse_pos: (f64, f64),
    mouse_down: bool,
    orbiting: bool,
    last_frame: f64,
    frame_count: u64,
    fps: f32,
    // WebGL resources
    gl: Option<WebGl2RenderingContext>,
    program: Option<WebGlProgram>,
    tex: Option<WebGlTexture>,
    vbo: Option<WebGlBuffer>,
    canvas_w: u32,
    canvas_h: u32,
    // UI
    status_msg: String,
    fps_frames: u32,
    fps_time: f64,
}

impl EngineState {
    fn new() -> Self {
        Self {
            renderer: WebRenderer::new(800, 600),
            scene: Scene::create_demo_scene(),
            selected: Some(1),
            playing: true,
            keys: Vec::new(),
            mouse_pos: (0.0, 0.0),
            mouse_down: false,
            orbiting: false,
            last_frame: 0.0,
            frame_count: 0,
            fps: 60.0,
            gl: None,
            program: None,
            tex: None,
            vbo: None,
            canvas_w: 800,
            canvas_h: 600,
            status_msg: String::new(),
            fps_frames: 0,
            fps_time: 0.0,
        }
    }
}

// ═══════════════════════════════════════════════════════════ WebGL Init

fn init_webgl(canvas: &HtmlCanvasElement) -> Result<(WebGl2RenderingContext, WebGlProgram, WebGlTexture, WebGlBuffer), JsValue> {
    let gl: WebGl2RenderingContext = canvas.get_context("webgl2")
        .map_err(|e| JsValue::from_str(&format!("get_context: {:?}", e)))?
        .ok_or_else(|| JsValue::from_str("WebGL2 not supported"))?
        .dyn_into()?;

    gl.enable(WebGl2RenderingContext::DEPTH_TEST);
    gl.depth_func(WebGl2RenderingContext::LEQUAL);
    gl.enable(WebGl2RenderingContext::CULL_FACE);
    gl.cull_face(WebGl2RenderingContext::BACK);

    let vs = compile_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, QUAD_VS)?;
    let fs = compile_shader(&gl, WebGl2RenderingContext::FRAGMENT_SHADER, QUAD_FS)?;
    let program = link_program(&gl, &vs, &fs)?;
    gl.delete_shader(Some(&vs));
    gl.delete_shader(Some(&fs));
    gl.use_program(Some(&program));

    // Fullscreen quad VBO (pos.xy + uv.xy)
    #[rustfmt::skip]
    let quad: [f32; 24] = [
        -1.0, -1.0,  0.0, 1.0,
         1.0, -1.0,  1.0, 1.0,
        -1.0,  1.0,  0.0, 0.0,
        -1.0,  1.0,  0.0, 0.0,
         1.0, -1.0,  1.0, 1.0,
         1.0,  1.0,  1.0, 0.0,
    ];
    let vbo = gl.create_buffer().ok_or_else(|| JsValue::from_str("create_buffer"))?;
    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&vbo));
    unsafe {
        let arr = js_sys::Float32Array::view(&quad);
        gl.buffer_data_with_array_buffer_view(WebGl2RenderingContext::ARRAY_BUFFER, &arr, WebGl2RenderingContext::STATIC_DRAW);
    }
    let a_pos = gl.get_attrib_location(&program, "a_pos") as u32;
    let a_uv  = gl.get_attrib_location(&program, "a_uv") as u32;
    gl.enable_vertex_attrib_array(a_pos);
    gl.vertex_attrib_pointer_with_i32(a_pos, 2, WebGl2RenderingContext::FLOAT, false, 16, 0);
    gl.enable_vertex_attrib_array(a_uv);
    gl.vertex_attrib_pointer_with_i32(a_uv, 2, WebGl2RenderingContext::FLOAT, false, 16, 8);

    // Texture for framebuffer upload
    let tex = gl.create_texture().ok_or_else(|| JsValue::from_str("create_texture"))?;
    gl.active_texture(WebGl2RenderingContext::TEXTURE0);
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&tex));
    gl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MIN_FILTER, WebGl2RenderingContext::NEAREST as i32);
    gl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MAG_FILTER, WebGl2RenderingContext::NEAREST as i32);
    gl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_S, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_T, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);

    if let Some(loc) = gl.get_uniform_location(&program, "u_tex") {
        gl.uniform1i(Some(&loc), 0);
    }

    Ok((gl, program, tex, vbo))
}

// ═══════════════════════════════════════════════════════════ Game Logic

fn update_game(state: &mut EngineState, dt: f32) {
    if !state.playing { return; }

    // Camera movement with WASD
    let yaw_r = state.scene.camera.yaw.to_radians();
    let mut move_dir = Vec3::ZERO;
    if state.keys.contains(&87) { move_dir = move_dir + Vec3::new(yaw_r.sin(), 0.0, -yaw_r.cos()); } // W
    if state.keys.contains(&83) { move_dir = move_dir - Vec3::new(yaw_r.sin(), 0.0, -yaw_r.cos()); } // S
    if state.keys.contains(&65) { move_dir = move_dir - Vec3::new(yaw_r.cos(), 0.0, yaw_r.sin()); } // A
    if state.keys.contains(&68) { move_dir = move_dir + Vec3::new(yaw_r.cos(), 0.0, yaw_r.sin()); } // D
    if move_dir.length_squared() > 0.0 {
        let mv = move_dir.normalize() * 4.0 * dt;
        if let Some(p) = state.scene.get_object_mut(1) {
            p.transform.position = p.transform.position + mv;
        }
    }

    // Enemy AI
    let player_pos = state.scene.get_object(1).map(|o| o.transform.position).unwrap_or(Vec3::ZERO);
    for obj in &mut state.scene.objects {
        if obj.team != EntityTeam::Enemy || obj.health <= 0.0 { continue; }
        obj.transform.rotation.1 += dt * 40.0;
        let to_player = player_pos - obj.transform.position;
        let dist = to_player.length();
        if dist > 1.2 && dist < 50.0 {
            obj.transform.position = obj.transform.position + to_player.normalize() * dt * 2.5;
        } else if dist <= 1.2 {
            obj.health -= 25.0 * dt;
            if obj.health <= 0.0 {
                state.renderer.spawn_particles(obj.transform.position, 30, [1.0, 0.4, 0.15]);
                let ang: f32 = (js_sys::Math::random() as f32) * std::f32::consts::TAU;
                obj.transform.position = Vec3::new(ang.cos() * 14.0, 1.0, ang.sin() * 14.0);
                obj.health = obj.max_health;
            }
        }
    }

    // Camera follow player
    if let Some(p) = state.scene.get_object(1) {
        state.scene.camera.target = p.transform.position;
    }

    state.frame_count += 1;
}

// ═══════════════════════════════════════════════════════════ WebGL Render

fn render_webgl(state: &mut EngineState) {
    let gl = match &state.gl { Some(g) => g.clone(), None => return };
    let program = match &state.program { Some(p) => p.clone(), None => return };
    let tex = match &state.tex { Some(t) => t.clone(), None => return };

    // Resize framebuffer
    state.renderer.resize(state.canvas_w, state.canvas_h);

    // CPU render
    state.renderer.render_scene(&state.scene, state.selected, 0.016);

    // Upload framebuffer to WebGL texture
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&tex));
    let fb = state.renderer.frame_buffer();
    unsafe {
        let _arr = js_sys::Uint8Array::view(fb);
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            WebGl2RenderingContext::TEXTURE_2D, 0,
            WebGl2RenderingContext::RGBA as i32,
            state.canvas_w as i32, state.canvas_h as i32, 0,
            WebGl2RenderingContext::RGBA, WebGl2RenderingContext::UNSIGNED_BYTE, Some(fb),
        ).ok();
    }

    // Draw fullscreen quad
    gl.viewport(0, 0, state.canvas_w as i32, state.canvas_h as i32);
    gl.clear_color(0.0, 0.0, 0.0, 1.0);
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);
    gl.use_program(Some(&program));
    gl.active_texture(WebGl2RenderingContext::TEXTURE0);
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&tex));
    gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 6);
}

// ═══════════════════════════════════════════════════════════ WASM Entry Point

#[wasm_bindgen]
pub struct ElysiumWebApp {
    state: Rc<RefCell<EngineState>>,
}

#[wasm_bindgen]
impl ElysiumWebApp {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<ElysiumWebApp, JsValue> {
        let mut state = EngineState::new();

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let document = window.document().ok_or_else(|| JsValue::from_str("no document"))?;
        let canvas: HtmlCanvasElement = document.get_element_by_id("canvas")
            .ok_or_else(|| JsValue::from_str("no canvas"))?.dyn_into()?;

        // Size canvas
        let dpr = window.device_pixel_ratio();
        let w = (canvas.client_width() as f64 * dpr) as u32;
        let h = (canvas.client_height() as f64 * dpr) as u32;
        canvas.set_width(w);
        canvas.set_height(h);
        state.canvas_w = w;
        state.canvas_h = h;

        // Init WebGL
        let (gl, program, tex_obj, vbo) = init_webgl(&canvas)?;
        state.gl = Some(gl);
        state.program = Some(program);
        state.tex = Some(tex_obj);
        state.vbo = Some(vbo);

        let state = Rc::new(RefCell::new(state));

        // ── Keyboard ──
        {
            let s = state.clone();
            let c = Closure::wrap(Box::new(move |e: KeyboardEvent| {
                let code = e.key_code();
                let mut st = s.borrow_mut();
                if !st.keys.contains(&code) { st.keys.push(code); }
                match code {
                    32 => { st.playing = !st.playing; } // Space
                    82 if !e.ctrl_key() => { st.scene.camera = Camera::new(); } // R
                    70 => { // F
                        if let Some(id) = st.selected {
                            let cam_target = st.scene.get_object(id).map(|o| o.transform.position);
                    let obj_name = st.scene.get_object(id).map(|o| o.name.clone()).unwrap_or_default();
                    if let Some(target) = cam_target {
                                st.scene.camera.target = target;
                                st.status_msg = format!("Focus: {}", obj_name);
                            }
                        }
                    }
                    _ => {}
                }
            }) as Box<dyn FnMut(_)>);
            document.add_event_listener_with_callback("keydown", c.as_ref().unchecked_ref()).ok();
            c.forget();
        }
        {
            let s = state.clone();
            let c = Closure::wrap(Box::new(move |e: KeyboardEvent| {
                let code = e.key_code();
                s.borrow_mut().keys.retain(|&k| k != code);
            }) as Box<dyn FnMut(_)>);
            document.add_event_listener_with_callback("keyup", c.as_ref().unchecked_ref()).ok();
            c.forget();
        }

        // ── Mouse ──
        {
            let s = state.clone();
            let c = Closure::wrap(Box::new(move |e: MouseEvent| {
                let mut st = s.borrow_mut();
                let x = e.offset_x() as f64;
                let y = e.offset_y() as f64;
                st.mouse_pos = (x, y);
                st.mouse_down = true;
                st.orbiting = true;

                // Ray cast
                let w = st.canvas_w as f32;
                let h = st.canvas_h as f32;
                let cam_pos = st.scene.camera.position();
                let cam_fwd = (st.scene.camera.target - cam_pos).normalize();
                let cam_right = cam_fwd.cross(Vec3::Y).normalize();
                let cam_up = cam_right.cross(cam_fwd);
                let aspect = w / h;
                let fov_rad = st.scene.camera.fov.to_radians();
                let nx = (e.offset_x() as f32 / w) * 2.0 - 1.0;
                let ny = 1.0 - (e.offset_y() as f32 / h) * 2.0;
                let ray_dir = cam_fwd + cam_right * (nx * fov_rad.tan() * aspect) + cam_up * (ny * fov_rad.tan());
                if let Some(id) = st.scene.ray_intersect(cam_pos, ray_dir.normalize()) {
                    let name = st.scene.get_object(id).map(|o| o.name.clone()).unwrap_or_default();
                    st.selected = Some(id);
                    st.status_msg = format!("Selected: {}", name);
                } else {
                    st.selected = None;
                    st.status_msg.clear();
                }
            }) as Box<dyn FnMut(_)>);
            canvas.add_event_listener_with_callback("mousedown", c.as_ref().unchecked_ref()).ok();
            c.forget();
        }
        {
            let s = state.clone();
            let c = Closure::wrap(Box::new(move |e: MouseEvent| {
                let x = e.offset_x() as f64;
                let y = e.offset_y() as f64;
                let mut st = s.borrow_mut();
                if st.mouse_down && st.orbiting {
                    let dx = (x - st.mouse_pos.0) as f32;
                    let dy = (y - st.mouse_pos.1) as f32;
                    st.scene.camera.orbit(dx * 0.35, dy * 0.35);
                }
                st.mouse_pos = (x, y);
            }) as Box<dyn FnMut(_)>);
            canvas.add_event_listener_with_callback("mousemove", c.as_ref().unchecked_ref()).ok();
            c.forget();
        }
        {
            let s = state.clone();
            let c = Closure::wrap(Box::new(move |_: MouseEvent| {
                let mut st = s.borrow_mut();
                st.mouse_down = false;
                st.orbiting = false;
            }) as Box<dyn FnMut(_)>);
            document.add_event_listener_with_callback("mouseup", c.as_ref().unchecked_ref()).ok();
            c.forget();
        }

        // ── Wheel zoom ──
        {
            let s = state.clone();
            let c = Closure::wrap(Box::new(move |e: WheelEvent| {
                e.prevent_default();
                s.borrow_mut().scene.camera.zoom(e.delta_y() as f32 * 0.05);
            }) as Box<dyn FnMut(_)>);
            canvas.add_event_listener_with_callback("wheel", c.as_ref().unchecked_ref()).ok();
            c.forget();
        }

        // ── Resize ──
        {
            let s = state.clone();
            let canvas2 = canvas.clone();
            let c = Closure::wrap(Box::new(move |_: web_sys::Event| {
                let window = web_sys::window().unwrap();
                let dpr = window.device_pixel_ratio();
                let w = (canvas2.client_width() as f64 * dpr) as u32;
                let h = (canvas2.client_height() as f64 * dpr) as u32;
                canvas2.set_width(w);
                canvas2.set_height(h);
                let mut st = s.borrow_mut();
                st.canvas_w = w;
                st.canvas_h = h;
            }) as Box<dyn FnMut(_)>);
            window.add_event_listener_with_callback("resize", c.as_ref().unchecked_ref()).ok();
            c.forget();
        }

        // ── UI overlay updates ──
        {
            let s = state.clone();
            let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
            let f2 = f.clone();
            *f2.borrow_mut() = Some(Closure::wrap(Box::new(move || {
                {
                    let st = s.borrow();
                    if let Some(el) = web_sys::window().unwrap().document().unwrap().get_element_by_id("ui-overlay") {
                        let fps = st.fps;
                        let play_state = if st.playing { "▶ PLAYING" } else { "⏸ PAUSED" };
                        let sel = st.selected.and_then(|id| st.scene.get_object(id))
                            .map(|o| format!("Selected: {}", o.name)).unwrap_or_default();
                        let cam = st.scene.camera.position();
                        let objs = st.scene.objects.len();
                        let lights = st.scene.lights.len();
                        let html = format!(
                            r#"<div class="fps">{:.0} FPS</div>
<div class="status">{}</div>
<div class="scene-info">Objects: {} | Lights: {}</div>
<div class="selected">{}</div>
<div class="cam-info">Cam: ({:.1}, {:.1}, {:.1})</div>
<div class="controls">
<b>WASD</b> Move &nbsp; <b>Mouse</b> Orbit &nbsp; <b>Scroll</b> Zoom &nbsp;
<b>Click</b> Select &nbsp; <b>Space</b> Play/Pause &nbsp; <b>F</b> Focus &nbsp; <b>R</b> Reset
</div>"#,
                            fps, play_state, objs, lights,
                            if sel.is_empty() { "Click to select".to_string() } else { sel },
                            cam.x(), cam.y(), cam.z()
                        );
                        el.set_inner_html(&html);
                    }
                }
                if let Some(c) = f.borrow().as_ref() {
                    web_sys::window().unwrap().request_animation_frame(c.as_ref().unchecked_ref()).ok();
                }
            }) as Box<dyn FnMut()>));
            web_sys::window().unwrap().request_animation_frame(f2.borrow().as_ref().unwrap().as_ref().unchecked_ref()).ok();
        }

        // ── Main game loop ──
        {
            let s = state.clone();
            let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
            let f2 = f.clone();
            *f2.borrow_mut() = Some(Closure::wrap(Box::new(move || {
                let perf = web_sys::window().unwrap().performance().unwrap();
                let now = perf.now();
                {
                    let mut st = s.borrow_mut();
                    let dt = if st.last_frame > 0.0 { ((now - st.last_frame) / 1000.0) as f32 } else { 0.016 };
                    let dt = dt.min(0.1);
                    st.fps_frames += 1;
                    st.fps_time += dt as f64;
                    if st.fps_time >= 0.5 {
                        st.fps = st.fps_frames as f32 / st.fps_time as f32;
                        st.fps_frames = 0;
                        st.fps_time = 0.0;
                    }
                    st.last_frame = now;
                    update_game(&mut st, dt);
                    render_webgl(&mut st);
                }
                if let Some(c) = f.borrow().as_ref() {
                    web_sys::window().unwrap().request_animation_frame(c.as_ref().unchecked_ref()).ok();
                }
            }) as Box<dyn FnMut()>));
            web_sys::window().unwrap().request_animation_frame(f2.borrow().as_ref().unwrap().as_ref().unchecked_ref()).ok();
        }

        Ok(ElysiumWebApp { state })
    }

    pub fn get_fps(&self) -> f32 { self.state.borrow().fps }
    pub fn get_object_count(&self) -> usize { self.state.borrow().scene.objects.len() }
    pub fn is_playing(&self) -> bool { self.state.borrow().playing }
    pub fn toggle_play(&self) { self.state.borrow_mut().playing = !self.state.borrow().playing; }
    pub fn delete_selected(&self) {
        let mut st = self.state.borrow_mut();
        if let Some(id) = st.selected {
            let pos = st.scene.get_object(id).map(|o| o.transform.position);
            if let Some(pos) = pos {
                st.renderer.spawn_particles(pos, 40, [1.0, 0.6, 0.2]);
            }
            st.scene.objects.retain(|o| o.id != id);
            st.selected = None;
        }
    }
    pub fn get_scene_json(&self) -> String {
        let st = self.state.borrow();
        let objs: Vec<serde_json::Value> = st.scene.objects.iter().map(|o| {
            serde_json::json!({
                "id": o.id, "name": o.name, "geometry": o.geometry.display_name(),
                "position": [o.transform.position.x(), o.transform.position.y(), o.transform.position.z()],
                "team": format!("{:?}", o.team),
                "health": o.health,
            })
        }).collect();
        serde_json::to_string_pretty(&objs).unwrap_or_default()
    }
}

// Suppress unused import warnings
use scene::Camera;
