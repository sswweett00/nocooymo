/// renderer.rs — Elysium Software Rasterizer
/// Gerçek 3D sahneyi bir RGBA pixel buffer'a render eder.
/// GPUI'ın ImageSource ile gösterilmek üzere tasarlanmıştır.

use core::fmt;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use glam::{Mat4, Vec2, Vec3, Vec4};
use serde::{Serialize, Deserialize};

// ─────────────────────────────────────────────────────────── Geometry Types

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum GeometryType {
    Cube,
    Sphere,
    Plane,
    Cylinder,
    Capsule,
}

impl fmt::Display for GeometryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeometryType::Cube => write!(f, "Cube"),
            GeometryType::Sphere => write!(f, "Sphere"),
            GeometryType::Plane => write!(f, "Plane"),
            GeometryType::Cylinder => write!(f, "Cylinder"),
            GeometryType::Capsule => write!(f, "Capsule"),
        }
    }
}

// ─────────────────────────────────────────────────────────── Transform

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3, // Euler degrees
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    #[inline]
    pub fn to_matrix(&self) -> Mat4 {
        let t = Mat4::from_translation(self.position);
        let rx = Mat4::from_rotation_x(self.rotation.x.to_radians());
        let ry = Mat4::from_rotation_y(self.rotation.y.to_radians());
        let rz = Mat4::from_rotation_z(self.rotation.z.to_radians());
        let s = Mat4::from_scale(self.scale);
        t * ry * rx * rz * s
    }
}

// ─────────────────────────────────────────────────────────── Camera

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub target: Vec3,
    pub yaw: f32,   // degrees, horizontal orbit
    pub pitch: f32, // degrees, vertical orbit
    pub distance: f32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera::new()
    }
}

impl Camera {
    pub fn new() -> Self {
        Self {
            target: Vec3::ZERO,
            yaw: -45.0,
            pitch: 30.0,
            distance: 18.0,
            fov: 60.0,
            near: 0.1,
            far: 1000.0,
        }
    }

    pub fn position(&self) -> Vec3 {
        let yaw_r = self.yaw.to_radians();
        let pitch_r = self.pitch.to_radians();
        self.target
            + Vec3::new(
                self.distance * yaw_r.cos() * pitch_r.cos(),
                self.distance * pitch_r.sin(),
                self.distance * yaw_r.sin() * pitch_r.cos(),
            )
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.target, Vec3::Y)
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov.to_radians(), aspect, self.near, self.far)
    }

    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch).clamp(-89.0, 89.0);
    }

    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance - delta).clamp(2.0, 200.0);
    }
}

// ─────────────────────────────────────────────────────────── Mesh

#[derive(Clone, Copy)]
struct Tri(usize, usize, usize);

struct Mesh {
    vertices: Vec<Vec3>,
    triangles: Vec<Tri>,
}

// ── Mesh generators ───────────────────────────────────────

fn cube_mesh() -> Mesh {
    let v = vec![
        Vec3::new(-0.5, -0.5,  0.5),
        Vec3::new( 0.5, -0.5,  0.5),
        Vec3::new( 0.5,  0.5,  0.5),
        Vec3::new(-0.5,  0.5,  0.5),
        Vec3::new( 0.5, -0.5, -0.5),
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(-0.5,  0.5, -0.5),
        Vec3::new( 0.5,  0.5, -0.5),
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(-0.5, -0.5,  0.5),
        Vec3::new(-0.5,  0.5,  0.5),
        Vec3::new(-0.5,  0.5, -0.5),
        Vec3::new( 0.5, -0.5,  0.5),
        Vec3::new( 0.5, -0.5, -0.5),
        Vec3::new( 0.5,  0.5, -0.5),
        Vec3::new( 0.5,  0.5,  0.5),
        Vec3::new(-0.5,  0.5,  0.5),
        Vec3::new( 0.5,  0.5,  0.5),
        Vec3::new( 0.5,  0.5, -0.5),
        Vec3::new(-0.5,  0.5, -0.5),
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new( 0.5, -0.5, -0.5),
        Vec3::new( 0.5, -0.5,  0.5),
        Vec3::new(-0.5, -0.5,  0.5),
    ];
    let mut tris = Vec::new();
    for face in 0..6 {
        let b = face * 4;
        tris.push(Tri(b, b+1, b+2));
        tris.push(Tri(b, b+2, b+3));
    }
    Mesh { vertices: v, triangles: tris }
}

fn sphere_mesh(stacks: u32, slices: u32) -> Mesh {
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    for i in 0..=stacks {
        let phi = std::f32::consts::PI * i as f32 / stacks as f32;
        for j in 0..=slices {
            let theta = 2.0 * std::f32::consts::PI * j as f32 / slices as f32;
            verts.push(Vec3::new(
                phi.sin() * theta.cos() * 0.5,
                phi.cos() * 0.5,
                phi.sin() * theta.sin() * 0.5,
            ));
        }
    }
    let w = slices + 1;
    for i in 0..stacks {
        for j in 0..slices {
            let a = i * w + j;
            let b = a + 1;
            let c = a + w;
            let d = c + 1;
            tris.push(Tri(a as usize, c as usize, b as usize));
            tris.push(Tri(b as usize, c as usize, d as usize));
        }
    }
    Mesh { vertices: verts, triangles: tris }
}

fn plane_mesh() -> Mesh {
    let v = vec![
        Vec3::new(-0.5, 0.0, -0.5),
        Vec3::new( 0.5, 0.0, -0.5),
        Vec3::new( 0.5, 0.0,  0.5),
        Vec3::new(-0.5, 0.0,  0.5),
    ];
    let tris = vec![Tri(0,2,1), Tri(0,3,2)];
    Mesh { vertices: v, triangles: tris }
}

fn cylinder_mesh(slices: u32) -> Mesh {
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    let top = Vec3::new(0.0, 0.5, 0.0);
    let bot = Vec3::new(0.0, -0.5, 0.0);
    verts.push(top);
    verts.push(bot);
    let base = 2usize;
    for i in 0..slices {
        let theta = 2.0 * std::f32::consts::PI * i as f32 / slices as f32;
        verts.push(Vec3::new(theta.cos() * 0.5, 0.5, theta.sin() * 0.5));
        verts.push(Vec3::new(theta.cos() * 0.5, -0.5, theta.sin() * 0.5));
    }
    for i in 0..slices as usize {
        let next = (i + 1) % slices as usize;
        let t0 = base + i * 2;
        let b0 = base + i * 2 + 1;
        let t1 = base + next * 2;
        let b1 = base + next * 2 + 1;
        tris.push(Tri(0, t0, t1));
        tris.push(Tri(1, b1, b0));
        tris.push(Tri(t0, b0, t1));
        tris.push(Tri(t1, b0, b1));
    }
    Mesh { vertices: verts, triangles: tris }
}

fn capsule_mesh(slices: u32, half_height: f32) -> Mesh {
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    let hh = half_height.max(0.1);
    let r = 0.5;

    verts.push(Vec3::new(0.0, hh + r, 0.0));
    verts.push(Vec3::new(0.0, hh, 0.0));

    let top_center_idx = 0usize;
    let top_base_idx = 2usize;
    for i in 0..slices {
        let theta = 2.0 * std::f32::consts::PI * i as f32 / slices as f32;
        verts.push(Vec3::new(theta.cos() * r, hh, theta.sin() * r));
    }

    for i in 0..slices as usize {
        let next = (i + 1) % slices as usize;
        let a = top_base_idx + i;
        let b = top_base_idx + next;
        tris.push(Tri(top_center_idx, a, b));
    }

    verts.push(Vec3::new(0.0, -hh, 0.0));

    for i in 0..slices as usize {
        let next = (i + 1) % slices as usize;
        let a = top_base_idx + i;
        let b = top_base_idx + next;
        let c = verts.len() - 1;
        tris.push(Tri(c, b, a));
    }

    verts.push(Vec3::new(0.0, -(hh + r), 0.0));
    let bot_center_idx = verts.len() - 1;
    let bot_start_idx = verts.len() - (slices as usize + 1);

    for i in 0..slices as usize {
        let a = bot_start_idx + i;
        let b = bot_start_idx + ((i + 1) % slices as usize);
        tris.push(Tri(bot_center_idx, b, a));
    }

    Mesh { vertices: verts, triangles: tris }
}

fn build_mesh(geometry: GeometryType) -> &'static Mesh {
    static CACHE: OnceLock<Mutex<HashMap<u8, &'static Mesh>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = match geometry {
        GeometryType::Cube => 0,
        GeometryType::Sphere => 1,
        GeometryType::Plane => 2,
        GeometryType::Cylinder => 3,
        GeometryType::Capsule => 4,
    };
    let mut map = cache.lock().unwrap();
    *map.entry(key).or_insert_with(move || {
        let mesh = match geometry {
            GeometryType::Cube => cube_mesh(),
            GeometryType::Sphere => sphere_mesh(16, 24),
            GeometryType::Plane => plane_mesh(),
            GeometryType::Cylinder => cylinder_mesh(20),
            GeometryType::Capsule => capsule_mesh(20, 0.8),
        };
        Box::leak(Box::new(mesh))
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityTeam {
    Neutral = 0,
    Player = 1,
    Enemy = 2,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneObject {
    pub id: usize,
    pub name: String,
    pub geometry: GeometryType,
    pub transform: Transform,
    pub color: [f32; 3],
    pub visible: bool,
    pub team: EntityTeam,
    pub health: f32,
    pub max_health: f32,
    pub damage: f32,
}

impl SceneObject {
    pub fn is_enemy(&self) -> bool {
        self.team == EntityTeam::Enemy
    }

    pub fn is_player(&self) -> bool {
        self.team == EntityTeam::Player
    }
}

#[derive(Clone, Debug)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub color: [f32; 3],
    pub size: f32,
}

#[derive(Clone, Debug)]
pub struct SoftwareRenderer {
    pub width: u32,
    pub height: u32,
    /// 3D sahnenin framebuffer içindeki sol-üst ofseti (UI panelleri için)
    pub viewport_offset: (i32, i32),
    /// Projeksiyon ölçeklemesi için viewport boyutu (framebuffer'dan bağımsız)
    pub viewport_size: (u32, u32),
    pub grid_enabled: bool,
    color: Vec<u8>,
    depth: Vec<f32>,
    particles: Vec<Particle>,
}

impl SoftwareRenderer {
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
            particles: Vec::new(),
        }
    }

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

    /// Grid çizimini aç/kapat.
    pub fn set_grid_enabled(&mut self, enabled: bool) {
        self.grid_enabled = enabled;
    }

    fn clear(&mut self, bg: [u8; 3]) {
        let n = (self.width * self.height) as usize;
        for i in 0..n {
            self.color[i*4  ] = bg[0];
            self.color[i*4+1] = bg[1];
            self.color[i*4+2] = bg[2];
            self.color[i*4+3] = 255;
            self.depth[i] = f32::INFINITY;
        }
    }

    #[inline]
    fn set_pixel(&mut self, x: i32, y: i32, z: f32, r: u8, g: u8, b: u8) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 { return; }
        let idx = (y as u32 * self.width + x as u32) as usize;
        if z < self.depth[idx] {
            self.depth[idx] = z;
            self.color[idx*4  ] = r;
            self.color[idx*4+1] = g;
            self.color[idx*4+2] = b;
            self.color[idx*4+3] = 255;
        }
    }

    fn draw_triangle(
        &mut self,
        p0: Vec3, p1: Vec3, p2: Vec3,
        r: u8, g: u8, b: u8,
    ) {
        let (w, h) = (self.width as i32, self.height as i32);
        let min_x = p0.x.min(p1.x).min(p2.x).max(0.0) as i32;
        let max_x = (p0.x.max(p1.x).max(p2.x) as i32 + 1).min(w - 1);
        let min_y = p0.y.min(p1.y).min(p2.y).max(0.0) as i32;
        let max_y = (p0.y.max(p1.y).max(p2.y) as i32 + 1).min(h - 1);

        let edge = |a: Vec2, b: Vec2, c: Vec2| -> f32 {
            (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x)
        };

        let a2 = Vec2::new(p0.x, p0.y);
        let b2 = Vec2::new(p1.x, p1.y);
        let c2 = Vec2::new(p2.x, p2.y);
        let area = edge(a2, b2, c2);
        if area.abs() < 0.5 { return; }

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let p = Vec2::new(px as f32 + 0.5, py as f32 + 0.5);
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
                    let bary = Vec3::new(w0 / denom, w1 / denom, w2 / denom);
                    let z = bary.x * p0.z + bary.y * p1.z + bary.z * p2.z;
                    self.set_pixel(px, py, z, r, g, b);
                }
            }
        }
    }

    fn draw_line(
        &mut self,
        mut x0: i32, mut y0: i32, z0: f32,
        x1: i32, y1: i32, z1: f32,
        r: u8, g: u8, b: u8,
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
            if x0 == x1 && y0 == y1 { break; }
            let e2 = 2 * err;
            if e2 > -dy { err -= dy; x0 += sx; }
            if e2 <  dx { err += dx; y0 += sy; }
            step += 1.0;
        }
    }

    fn project(&self, p: Vec3, mvp: &Mat4) -> Option<Vec3> {
        let clip = mvp.mul_vec4(Vec4::new(p.x, p.y, p.z, 1.0));
        if clip.w.abs() < 1e-6 { return None; }
        let ndc = Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);
        if ndc.z < -1.0 || ndc.z > 1.0 { return None; }
        let sx = (ndc.x * 0.5 + 0.5) * self.viewport_size.0 as f32 + self.viewport_offset.0 as f32;
        let sy = (1.0 - (ndc.y * 0.5 + 0.5)) * self.viewport_size.1 as f32 + self.viewport_offset.1 as f32;
        Some(Vec3::new(sx, sy, clip.z / clip.w))
    }

    /// UI için depth'siz alpha-blend dikdörtgen.
    pub fn ui_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 4]) {
        if w <= 0 || h <= 0 { return; }
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
                self.color[idx*4  ] = (c[0] as f32 * a + self.color[idx*4  ] as f32 * inv) as u8;
                self.color[idx*4+1] = (c[1] as f32 * a + self.color[idx*4+1] as f32 * inv) as u8;
                self.color[idx*4+2] = (c[2] as f32 * a + self.color[idx*4+2] as f32 * inv) as u8;
                self.color[idx*4+3] = 255;
            }
        }
    }

    /// Fontdue glyph bitmap'ini alpha ile blit eder.
    pub fn blit_glyph(
        &mut self,
        x: i32,
        y: i32,
        w: usize,
        h: usize,
        coverage: &[u8],
        c: [u8; 4],
    ) {
        for gy in 0..h {
            for gx in 0..w {
                let cov = coverage[gy * w + gx];
                if cov == 0 { continue; }
                let px = x + gx as i32;
                let py = y + gy as i32;
                if px < 0 || py < 0 || px >= self.width as i32 || py >= self.height as i32 { continue; }
                let idx = (py as u32 * self.width + px as u32) as usize;
                let a = (cov as f32 / 255.0) * (c[3] as f32 / 255.0);
                let inv = 1.0 - a;
                self.color[idx*4  ] = (c[0] as f32 * a + self.color[idx*4  ] as f32 * inv) as u8;
                self.color[idx*4+1] = (c[1] as f32 * a + self.color[idx*4+1] as f32 * inv) as u8;
                self.color[idx*4+2] = (c[2] as f32 * a + self.color[idx*4+2] as f32 * inv) as u8;
                self.color[idx*4+3] = 255;
            }
        }
    }

    fn draw_mesh(
        &mut self,
        mesh: &Mesh,
        model: &Mat4,
        vp: &Mat4,
        base_color: [f32; 3],
        selected: bool,
        wireframe_only: bool,
        team: EntityTeam,
        health: f32,
        max_health: f32,
    ) {
        let mvp = *vp * *model;
        let light_dir = Vec3::new(0.6, 1.0, 0.4).normalize();
        let ambient = 0.25f32;

        let mut color = base_color;
        if team == EntityTeam::Enemy {
            color = [0.85, 0.2, 0.2];
        } else if team == EntityTeam::Player {
            color = [0.2, 0.5, 0.9];
        }

        for tri in &mesh.triangles {
            let v0_w = model.transform_point3(mesh.vertices[tri.0]);
            let v1_w = model.transform_point3(mesh.vertices[tri.1]);
            let v2_w = model.transform_point3(mesh.vertices[tri.2]);

            let edge1 = v1_w - v0_w;
            let edge2 = v2_w - v0_w;
            let normal = edge1.cross(edge2).normalize_or_zero();

            let Some(s0) = self.project(v0_w, &mvp) else { continue };
            let Some(s1) = self.project(v1_w, &mvp) else { continue };
            let Some(s2) = self.project(v2_w, &mvp) else { continue };

            let diffuse = normal.dot(light_dir).max(0.0);
            let light = ambient + (1.0 - ambient) * diffuse;

            if selected {
                let sr = ((color[0] * light * 1.1).min(1.0) * 255.0) as u8;
                let sg = ((color[1] * light * 1.1).min(1.0) * 255.0) as u8;
                let sb = ((color[2] * light * 1.1).min(1.0) * 255.0) as u8;
                if !wireframe_only {
                    self.draw_triangle(s0, s1, s2, sr, sg, sb);
                }
                self.draw_line(s0.x as i32, s0.y as i32, s0.z, s1.x as i32, s1.y as i32, s1.z, 120, 200, 255);
                self.draw_line(s1.x as i32, s1.y as i32, s1.z, s2.x as i32, s2.y as i32, s2.z, 120, 200, 255);
                self.draw_line(s2.x as i32, s2.y as i32, s2.z, s0.x as i32, s0.y as i32, s0.z, 120, 200, 255);
            } else {
                let health_tint = if max_health > 0.0 {
                    0.6 + 0.4 * (health / max_health)
                } else {
                    1.0
                };
                let cr = (color[0] * light * health_tint * 255.0) as u8;
                let cg = (color[1] * light * health_tint * 255.0) as u8;
                let cb = (color[2] * light * health_tint * 255.0) as u8;
                if !wireframe_only {
                    self.draw_triangle(s0, s1, s2, cr, cg, cb);
                }
                let wr = (color[0] * light * 0.25 * 255.0) as u8;
                let wg = (color[1] * light * 0.25 * 255.0) as u8;
                let wb = (color[2] * light * 0.25 * 255.0) as u8;
                self.draw_line(s0.x as i32, s0.y as i32, s0.z - 0.001,
                               s1.x as i32, s1.y as i32, s1.z - 0.001, wr, wg, wb);
                self.draw_line(s1.x as i32, s1.y as i32, s1.z - 0.001,
                               s2.x as i32, s2.y as i32, s2.z - 0.001, wr, wg, wb);
                self.draw_line(s2.x as i32, s2.y as i32, s2.z - 0.001,
                               s0.x as i32, s0.y as i32, s0.z - 0.001, wr, wg, wb);
            }
        }
    }

    fn draw_grid(&mut self, vp: &Mat4, size: f32, divisions: u32) {
        let half = size * 0.5;
        let step = size / divisions as f32;

        for i in 0..=divisions {
            let t = -half + i as f32 * step;
            let is_major = i % 5 == 0;
            let c = if i == divisions / 2 {
                60u8
            } else if is_major {
                45u8
            } else {
                30u8
            };

            let a = Vec3::new(-half, 0.0, t);
            let b = Vec3::new( half, 0.0, t);
            if let (Some(sa), Some(sb)) = (self.project(a, vp), self.project(b, vp)) {
                self.draw_line(sa.x as i32, sa.y as i32, sa.z,
                               sb.x as i32, sb.y as i32, sb.z, c, c, c);
            }
            let a = Vec3::new(t, 0.0, -half);
            let b = Vec3::new(t, 0.0,  half);
            if let (Some(sa), Some(sb)) = (self.project(a, vp), self.project(b, vp)) {
                self.draw_line(sa.x as i32, sa.y as i32, sa.z,
                               sb.x as i32, sb.y as i32, sb.z, c, c, c);
            }
        }
    }

    fn draw_axes(&mut self, vp: &Mat4, len: f32) {
        let o = Vec3::ZERO;
        let ox = self.project(o, vp);
        let xx = self.project(Vec3::new(len, 0.0, 0.0), vp);
        let yy = self.project(Vec3::new(0.0, len, 0.0), vp);
        let zz = self.project(Vec3::new(0.0, 0.0, len), vp);

        if let (Some(so), Some(sx)) = (ox, xx) {
            self.draw_line(so.x as i32, so.y as i32, so.z - 0.01,
                           sx.x as i32, sx.y as i32, sx.z - 0.01, 220, 60, 60);
        }
        if let (Some(so), Some(sy)) = (ox, yy) {
            self.draw_line(so.x as i32, so.y as i32, so.z - 0.01,
                           sy.x as i32, sy.y as i32, sy.z - 0.01, 60, 200, 60);
        }
        if let (Some(so), Some(sz)) = (ox, zz) {
            self.draw_line(so.x as i32, so.y as i32, so.z - 0.01,
                           sz.x as i32, sz.y as i32, sz.z - 0.01, 60, 100, 220);
        }
    }

    fn draw_object_gizmo(&mut self, vp: &Mat4, pos: Vec3, len: f32) {
        let so = self.project(pos, vp);
        let sx = self.project(pos + Vec3::new(len, 0.0, 0.0), vp);
        let sy = self.project(pos + Vec3::new(0.0, len, 0.0), vp);
        let sz = self.project(pos + Vec3::new(0.0, 0.0, len), vp);

        if let (Some(o), Some(x)) = (so, sx) {
            self.draw_line(o.x as i32, o.y as i32, o.z - 0.005,
                           x.x as i32, x.y as i32, x.z - 0.005, 255, 80, 80);
        }
        if let (Some(o), Some(y)) = (so, sy) {
            self.draw_line(o.x as i32, o.y as i32, o.z - 0.005,
                           y.x as i32, y.y as i32, y.z - 0.005, 80, 255, 80);
        }
        if let (Some(o), Some(z)) = (so, sz) {
            self.draw_line(o.x as i32, o.y as i32, o.z - 0.005,
                           z.x as i32, z.y as i32, z.z - 0.005, 80, 130, 255);
        }
    }

    fn draw_circle(&mut self, cx: i32, cy: i32, z: f32, radius: i32, r: u8, g: u8, b: u8) {
        let mut x = radius;
        let mut y = 0i32;
        let mut err = 0i32;
        while x >= y {
            self.set_pixel(cx + x, cy + y, z, r, g, b);
            self.set_pixel(cx + y, cy + x, z, r, g, b);
            self.set_pixel(cx - y, cy + x, z, r, g, b);
            self.set_pixel(cx - x, cy + y, z, r, g, b);
            self.set_pixel(cx - x, cy - y, z, r, g, b);
            self.set_pixel(cx - y, cy - x, z, r, g, b);
            self.set_pixel(cx + y, cy - x, z, r, g, b);
            self.set_pixel(cx + x, cy - y, z, r, g, b);
            y += 1;
            if err <= 0 {
                err += 2 * y + 1;
            }
            if err > 0 {
                x -= 1;
                err -= 2 * x + 1;
            }
        }
    }

    fn draw_health_bar(&mut self, vp: &Mat4, pos: Vec3, health: f32, max_health: f32) {
        if max_health <= 0.0 { return; }
        let ratio = (health / max_health).clamp(0.0, 1.0);
        let bar_world = Vec3::new(1.2, 0.0, 0.0);
        let top_world = Vec3::new(0.0, 1.8, 0.0);
        let Some(base_screen) = self.project(pos + top_world, vp) else { return };
        let Some(end_screen) = self.project(pos + top_world + bar_world, vp) else { return };

        let x0 = base_screen.x as i32;
        let y0 = (base_screen.y - 4.0).max(0.0) as i32;
        let _x1 = (base_screen.x + (end_screen.x - base_screen.x) * ratio).max(base_screen.x) as i32;
        let y1 = (base_screen.y + 4.0).min(self.height as f32) as i32;
        let _bar_h = (y1 - y0).max(1);

        let (r, g, b) = if ratio > 0.6 {
            (40, 200, 80)
        } else if ratio > 0.3 {
            (240, 180, 40)
        } else {
            (220, 60, 40)
        };
        let bg_r = 40u8;
        let bg_g = 40u8;
        let bg_b = 40u8;

        for py in y0..y1 {
            for px in x0..(base_screen.x as i32 + (end_screen.x - base_screen.x).ceil() as i32) {
                let t = ((px - x0) as f32 / ((end_screen.x - base_screen.x).max(1.0))).clamp(0.0, 1.0);
                if t <= ratio {
                    self.set_pixel(px, py, base_screen.z, r, g, b);
                } else {
                    self.set_pixel(px, py, base_screen.z, bg_r, bg_g, bg_b);
                }
            }
        }
    }

    fn draw_particles(&mut self, vp: &Mat4) {
        let particles = self.particles.clone();
        for p in &particles {
            if p.life <= 0.0 { continue; }
            let Some(screen) = self.project(p.position, vp) else { continue };
            let radius = (p.size * 4.0).max(1.0) as i32;
            let alpha = (p.life / p.max_life).clamp(0.0, 1.0);
            let r = (p.color[0] * 255.0 * alpha) as u8;
            let g = (p.color[1] * 255.0 * alpha) as u8;
            let b = (p.color[2] * 255.0 * alpha) as u8;
            self.draw_circle(screen.x as i32, screen.y as i32, screen.z, radius, r, g, b);
        }
    }

    fn update_particles(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.position += p.velocity * dt;
            p.life -= dt;
            p.velocity.y -= 2.0 * dt;
        }
        self.particles.retain(|p| p.life > 0.0);
        if self.particles.len() > 2048 {
            self.particles.drain(0..self.particles.len() - 2048);
        }
    }

    pub fn spawn_particles(&mut self, position: Vec3, count: usize, color: [f32; 3]) {
        for _ in 0..count {
            let angle = rand::random::<f32>() * std::f32::consts::TAU;
            let speed = 1.0 + rand::random::<f32>() * 3.0;
            let vel = Vec3::new(
                angle.cos() * speed,
                1.0 + rand::random::<f32>() * 2.0,
                angle.sin() * speed,
            );
            self.particles.push(Particle {
                position,
                velocity: vel,
                life: 0.5 + rand::random::<f32>() * 1.5,
                max_life: 2.0,
                color,
                size: 1.0 + rand::random::<f32>() * 2.0,
            });
        }
        if self.particles.len() > 2048 {
            self.particles.drain(0..self.particles.len() - 2048);
        }
    }

    pub fn render_scene(
        &mut self,
        scene: &Scene,
        selected_id: Option<usize>,
        dt: f32,
    ) -> &[u8] {
        self.update_particles(dt);
        self.clear([10, 12, 18]);

        let aspect = self.viewport_size.0 as f32 / self.viewport_size.1 as f32;

        let view = scene.camera.view_matrix();
        let proj = scene.camera.projection_matrix(aspect);
        let vp = proj * view;

        if self.grid_enabled {
            self.draw_grid(&vp, scene.grid_size, scene.grid_divisions);
        }
        self.draw_axes(&vp, 2.0);

        for obj in &scene.objects {
            if !obj.visible { continue; }

            let model = obj.transform.to_matrix();
            let mesh = build_mesh(obj.geometry);
            let selected = selected_id == Some(obj.id);
            self.draw_mesh(mesh, &model, &vp, obj.color, selected, false, obj.team, obj.health, obj.max_health);

            if selected {
                self.draw_object_gizmo(&vp, obj.transform.position, 1.5);
            }

            if obj.team == EntityTeam::Enemy && obj.health > 0.0 {
                self.draw_health_bar(&vp, obj.transform.position, obj.health, obj.max_health);
            }
        }

        self.draw_particles(&vp);

        &self.color
    }

}

// ─────────────────────────────────────────────────────────── Scene

#[derive(Clone, Debug)]
pub struct Scene {
    pub objects: Vec<SceneObject>,
    pub camera: Camera,
    pub next_id: usize,
    pub grid_size: f32,
    pub grid_divisions: u32,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            objects: Vec::new(),
            camera: Camera::new(),
            next_id: 1,
            grid_size: 24.0,
            grid_divisions: 24,
        }
    }
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_object(&mut self, name: String, geometry: GeometryType, team: EntityTeam) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.objects.push(SceneObject {
            id,
            name,
            geometry,
            transform: Transform::default(),
            color: [0.8, 0.8, 0.8],
            visible: true,
            team,
            health: 100.0,
            max_health: 100.0,
            damage: 10.0,
        });
        id
    }

    pub fn get_object(&self, id: usize) -> Option<&SceneObject> {
        self.objects.iter().find(|o| o.id == id)
    }

    pub fn get_object_mut(&mut self, id: usize) -> Option<&mut SceneObject> {
        self.objects.iter_mut().find(|o| o.id == id)
    }

    #[allow(dead_code)]
    pub fn ray_intersect(&self, ray_origin: Vec3, ray_dir: Vec3) -> Option<usize> {
        let mut closest_id = None;
        let mut closest_dist = f32::MAX;
        for obj in &self.objects {
            if !obj.visible { continue; }
            let to_obj = obj.transform.position - ray_origin;
            let dist = to_obj.dot(ray_dir);
            if dist > 0.0 && dist < closest_dist {
                let perp = (to_obj - ray_dir * dist).length();
                if perp < 1.0 { closest_dist = dist; closest_id = Some(obj.id); }
            }
        }
        closest_id
    }
    
    pub fn serialize(&self) -> Result<String, serde_json::Error> {
        #[derive(Serialize)]
        struct SceneFile {
            objects: Vec<SceneObject>,
            camera_target: [f32; 3],
            camera_yaw: f32,
            camera_pitch: f32,
            camera_distance: f32,
        }
        let file = SceneFile {
            objects: self.objects.clone(),
            camera_target: self.camera.target.to_array(),
            camera_yaw: self.camera.yaw,
            camera_pitch: self.camera.pitch,
            camera_distance: self.camera.distance,
        };
        serde_json::to_string_pretty(&file)
    }

    pub fn deserialize(json: &str) -> Result<Self, serde_json::Error> {
        #[derive(Deserialize)]
        struct SceneFile {
            objects: Vec<SceneObject>,
            camera_target: Option<[f32; 3]>,
            camera_yaw: Option<f32>,
            camera_pitch: Option<f32>,
            camera_distance: Option<f32>,
        }
        let file: SceneFile = serde_json::from_str(json)?;
        let mut scene = Scene::default();
        scene.objects = file.objects;
        scene.next_id = scene.objects.iter().map(|o| o.id).max().map(|m| m + 1).unwrap_or(1);
        if let Some(t) = file.camera_target { scene.camera.target = Vec3::from_array(t); }
        if let Some(y) = file.camera_yaw { scene.camera.yaw = y; }
        if let Some(p) = file.camera_pitch { scene.camera.pitch = p; }
        if let Some(d) = file.camera_distance { scene.camera.distance = d; }
        Ok(scene)
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = self.serialize()?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let scene = Self::deserialize(&json)?;
        Ok(scene)
    }
}
