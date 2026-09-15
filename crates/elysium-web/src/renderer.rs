/// renderer.rs — Elysium Web Software Rasterizer
/// CPU-based 3D renderer that outputs an RGBA framebuffer for WebGL upload.

use crate::scene::*;

// ═══════════════════════════════════════════════════════════ PBR BRDF

struct Pbr;

impl Pbr {
    #[inline]
    fn fresnel(cos_theta: f32, f0: Vec3) -> Vec3 {
        let x = (1.0 - cos_theta).clamp(0.0, 1.0);
        let x5 = x * x * x * x * x;
        f0 + (Vec3::ONE - f0) * x5
    }
    #[inline]
    fn d_ggx(nh: f32, r: f32) -> f32 {
        let a = r * r; let a2 = a * a;
        let d = nh * nh * (a2 - 1.0) + 1.0;
        a2 / (std::f32::consts::PI * d * d + 0.0001)
    }
    #[inline]
    fn g_schlick(nv: f32, r: f32) -> f32 {
        let k = (r + 1.0).powi(2) / 8.0;
        nv / (nv * (1.0 - k) + k + 0.0001)
    }
    #[inline]
    fn g_smith(nv: f32, nl: f32, r: f32) -> f32 {
        Self::g_schlick(nv, r) * Self::g_schlick(nl, r)
    }
    fn compute(n: Vec3, v: Vec3, l: Vec3, lc: Vec3, li: f32, albedo: Vec3, m: f32, r: f32) -> Vec3 {
        let nl = n.dot(l).max(0.0);
        if nl <= 0.0 { return Vec3::ZERO; }
        let h = (v + l).normalize();
        let nv = n.dot(v).max(0.001);
        let nh = n.dot(h).max(0.0);
        let vh = v.dot(h).max(0.0);
        let f0 = Vec3::splat(0.04).lerp(albedo, m);
        let d = Self::d_ggx(nh, r);
        let g = Self::g_smith(nv, nl, r);
        let f = Self::fresnel(vh, f0);
        let spec = f * ((d * g) / (4.0 * nv * nl + 0.0001));
        let kd = (Vec3::ONE - f) * (1.0 - m);
        let diff = kd * albedo / std::f32::consts::PI;
        (diff + spec) * lc * (li * nl)
    }
}

// ═══════════════════════════════════════════════════════════ Tonemap + Post

#[inline]
fn aces(c: Vec3) -> Vec3 {
    let (a,b,cc,d,e) = (2.51, 0.03, 2.43, 0.59, 0.14);
    Vec3::new(
        ((c.x()*(a*c.x()+b))/(c.x()*(cc*c.x()+d)+e)).clamp(0.0,1.0),
        ((c.y()*(a*c.y()+b))/(c.y()*(cc*c.y()+d)+e)).clamp(0.0,1.0),
        ((c.z()*(a*c.z()+b))/(c.z()*(cc*c.z()+d)+e)).clamp(0.0,1.0),
    )
}

// ═══════════════════════════════════════════════════════════ Software Renderer

pub struct WebRenderer {
    pub width: u32,
    pub height: u32,
    pub grid_enabled: bool,
    pub hdr: Vec<[f32; 3]>,
    pub color: Vec<u8>,
    pub depth: Vec<f32>,
    pub particles: Vec<WebParticle>,
    pub post_exposure: f32,
    pub post_vignette: f32,
}

#[derive(Clone)]
pub struct WebParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub color: Vec3,
    pub size: f32,
}

impl WebRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        let n = (width * height) as usize;
        Self {
            width, height, grid_enabled: true,
            hdr: vec![[0.0; 3]; n],
            color: vec![0u8; n * 4],
            depth: vec![f32::INFINITY; n],
            particles: Vec::new(),
            post_exposure: 1.2,
            post_vignette: 0.3,
        }
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if self.width == w && self.height == h { return; }
        let n = (w * h) as usize;
        self.width = w; self.height = h;
        self.hdr.resize(n, [0.0; 3]);
        self.color.resize(n * 4, 0);
        self.depth.resize(n, f32::INFINITY);
    }

    pub fn frame_buffer(&self) -> &[u8] { &self.color }

    pub fn set_pixel_hdr(&mut self, x: i32, y: i32, z: f32, c: [f32; 3]) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 { return; }
        let idx = (y as u32 * self.width + x as u32) as usize;
        if z < self.depth[idx] {
            self.depth[idx] = z;
            self.hdr[idx] = c;
        }
    }

    fn set_pixel(&mut self, x: i32, y: i32, z: f32, r: u8, g: u8, b: u8) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 { return; }
        let idx = (y as u32 * self.width + x as u32) as usize;
        if z < self.depth[idx] {
            self.depth[idx] = z;
            self.color[idx*4] = r; self.color[idx*4+1] = g; self.color[idx*4+2] = b; self.color[idx*4+3] = 255;
        }
    }

    fn project(&self, p: Vec3, mvp: &Mat4) -> Option<Vec3> {
        let clip = mvp.mul_vec4([p.x(), p.y(), p.z(), 1.0]);
        if clip[3].abs() < 1e-6 { return None; }
        let ndc = Vec3::new(clip[0]/clip[3], clip[1]/clip[3], clip[2]/clip[3]);
        if ndc.z() < -1.0 || ndc.z() > 1.0 { return None; }
        let sx = (ndc.x() * 0.5 + 0.5) * self.width as f32;
        let sy = (1.0 - (ndc.y() * 0.5 + 0.5)) * self.height as f32;
        Some(Vec3::new(sx, sy, clip[2] / clip[3]))
    }

    fn draw_triangle_hdr(&mut self, p0: Vec3, p1: Vec3, p2: Vec3, c0: [f32;3], c1: [f32;3], c2: [f32;3]) {
        let (w, h) = (self.width as i32, self.height as i32);
        let min_x = p0.x().min(p1.x()).min(p2.x()).max(0.0) as i32;
        let max_x = (p0.x().max(p1.x()).max(p2.x()) as i32 + 1).min(w - 1);
        let min_y = p0.y().min(p1.y()).min(p2.y()).max(0.0) as i32;
        let max_y = (p0.y().max(p1.y()).max(p2.y()) as i32 + 1).min(h - 1);
        let edge = |ax:f32,ay:f32,bx:f32,by:f32,cx:f32,cy:f32|->f32{(cx-ax)*(by-ay)-(cy-ay)*(bx-ax)};
        let area = edge(p0.x(),p0.y(),p1.x(),p1.y(),p2.x(),p2.y());
        if area.abs() < 0.5 { return; }
        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let px_f = px as f32 + 0.5; let py_f = py as f32 + 0.5;
                let w0 = edge(p1.x(),p1.y(),p2.x(),p2.y(),px_f,py_f);
                let w1 = edge(p2.x(),p2.y(),p0.x(),p0.y(),px_f,py_f);
                let w2 = edge(p0.x(),p0.y(),p1.x(),p1.y(),px_f,py_f);
                let inside = if area > 0.0 { w0>=0.0&&w1>=0.0&&w2>=0.0 } else { w0<=0.0&&w1<=0.0&&w2<=0.0 };
                if inside {
                    let denom = w0+w1+w2;
                    let (bw0,bw1,bw2) = (w0/denom, w1/denom, w2/denom);
                    let z = bw0*p0.z()+bw1*p1.z()+bw2*p2.z();
                    let c = [bw0*c0[0]+bw1*c1[0]+bw2*c2[0], bw0*c0[1]+bw1*c1[1]+bw2*c2[1], bw0*c0[2]+bw1*c1[2]+bw2*c2[2]];
                    self.set_pixel_hdr(px, py, z, c);
                }
            }
        }
    }

    fn draw_line(&mut self, mut x0:i32, mut y0:i32, z0:f32, x1:i32, y1:i32, z1:f32, r:u8, g:u8, b:u8) {
        let dx = (x1-x0).abs(); let dy = (y1-y0).abs();
        let sx = if x0<x1{1}else{-1}; let sy = if y0<y1{1}else{-1};
        let mut err = dx - dy;
        let total = dx.max(dy).max(1) as f32;
        let mut step = 0.0f32;
        loop {
            let t = step / total;
            let z = z0 + (z1-z0) * t;
            self.set_pixel(x0,y0,z,r,g,b);
            if x0==x1 && y0==y1 { break; }
            let e2 = 2*err;
            if e2 > -dy { err -= dy; x0 += sx; }
            if e2 < dx { err += dx; y0 += sy; }
            step += 1.0;
        }
    }

    fn draw_line_hdr(&mut self, mut x0:i32, mut y0:i32, z0:f32, x1:i32, y1:i32, z1:f32, c: [f32;3]) {
        let dx = (x1-x0).abs(); let dy = (y1-y0).abs();
        let sx = if x0<x1{1}else{-1}; let sy = if y0<y1{1}else{-1};
        let mut err = dx - dy;
        let total = dx.max(dy).max(1) as f32;
        let mut step = 0.0f32;
        loop {
            let t = step / total;
            let z = z0 + (z1-z0) * t;
            self.set_pixel_hdr(x0, y0, z, c);
            if x0==x1 && y0==y1 { break; }
            let e2 = 2*err;
            if e2 > -dy { err -= dy; x0 += sx; }
            if e2 < dx { err += dx; y0 += sy; }
            step += 1.0;
        }
    }

    fn draw_grid(&mut self, vp: &Mat4) {
        let size = 20.0f32; let divs = 20u32;
        let half = size * 0.5; let step = size / divs as f32;
        for i in 0..=divs {
            let t = -half + i as f32 * step;
            let c = if i == divs/2 { 60u8 } else if i%5==0 { 45u8 } else { 30u8 };
            if let (Some(a), Some(b)) = (self.project(Vec3::new(-half,0.0,t),vp), self.project(Vec3::new(half,0.0,t),vp)) {
                self.draw_line(a.x() as i32,a.y() as i32,a.z(), b.x() as i32,b.y() as i32,b.z(), c,c,c);
            }
            if let (Some(a), Some(b)) = (self.project(Vec3::new(t,0.0,-half),vp), self.project(Vec3::new(t,0.0,half),vp)) {
                self.draw_line(a.x() as i32,a.y() as i32,a.z(), b.x() as i32,b.y() as i32,b.z(), c,c,c);
            }
        }
    }

    fn draw_axes(&mut self, vp: &Mat4, len: f32) {
        if let (Some(o), Some(x)) = (self.project(Vec3::ZERO,vp), self.project(Vec3::new(len,0.0,0.0),vp)) {
            self.draw_line(o.x() as i32,o.y() as i32,o.z()-0.01, x.x() as i32,x.y() as i32,x.z()-0.01, 220,60,60);
        }
        if let (Some(o), Some(y)) = (self.project(Vec3::ZERO,vp), self.project(Vec3::new(0.0,len,0.0),vp)) {
            self.draw_line(o.x() as i32,o.y() as i32,o.z()-0.01, y.x() as i32,y.y() as i32,y.z()-0.01, 60,200,60);
        }
        if let (Some(o), Some(z)) = (self.project(Vec3::ZERO,vp), self.project(Vec3::new(0.0,0.0,len),vp)) {
            self.draw_line(o.x() as i32,o.y() as i32,o.z()-0.01, z.x() as i32,z.y() as i32,z.z()-0.01, 60,100,220);
        }
    }

    fn draw_health_bar(&mut self, vp: &Mat4, pos: Vec3, health: f32, max_health: f32) {
        if max_health <= 0.0 { return; }
        let ratio = (health / max_health).clamp(0.0, 1.0);
        let bar = Vec3::new(1.2, 0.0, 0.0);
        let top = Vec3::new(0.0, 1.8, 0.0);
        let Some(bs) = self.project(pos + top, vp) else { return };
        let Some(es) = self.project(pos + top + bar, vp) else { return };
        let (r,g,b) = if ratio > 0.6 {(40,200,80)} else if ratio > 0.3 {(240,180,40)} else {(220,60,40)};
        let x0 = bs.x() as i32; let y0 = (bs.y()-4.0).max(0.0) as i32;
        let y1 = (bs.y()+4.0).min(self.height as f32) as i32;
        let bar_w = (es.x() - bs.x()).ceil() as i32;
        for py in y0..y1 {
            for px in x0..(x0+bar_w) {
                let t = ((px-x0) as f32 / bar_w.max(1) as f32).clamp(0.0, 1.0);
                if t <= ratio { self.set_pixel(px,py,bs.z(),r,g,b); }
                else { self.set_pixel(px,py,bs.z(),40,40,40); }
            }
        }
    }

    fn draw_mesh_pbr(
        &mut self, mesh: &Mesh, model: &Mat4, vp: &Mat4,
        material: &PbrMaterial, team: EntityTeam, health: f32, max_health: f32,
        selected: bool, lights: &[SceneLight], cam_pos: Vec3,
        amb_color: Vec3, amb_int: f32,
    ) {
        let mvp = vp.mul_mat4(*model);
        let view_dir = (cam_pos - model.transform_point3(Vec3::ZERO)).normalize_or_zero();
        let mut base_albedo = material.albedo;
        match team {
            EntityTeam::Enemy => base_albedo = Vec3::new(0.85, 0.2, 0.2),
            EntityTeam::Player => base_albedo = Vec3::new(0.2, 0.5, 0.9),
            _ => {}
        }

        for tri in &mesh.triangles {
            let v0 = model.transform_point3(mesh.vertices[tri.0]);
            let v1 = model.transform_point3(mesh.vertices[tri.1]);
            let v2 = model.transform_point3(mesh.vertices[tri.2]);
            let normal = (v1 - v0).cross(v2 - v0).normalize_or_zero();
            let centroid = (v0 + v1 + v2) / 3.0;

            let Some(s0) = self.project(v0, &mvp) else { continue };
            let Some(s1) = self.project(v1, &mvp) else { continue };
            let Some(s2) = self.project(v2, &mvp) else { continue };

            let mut total_light = amb_color * amb_int;
            for light in lights {
                let contrib = match light.light_type {
                    LightType::Directional => Pbr::compute(normal, view_dir, -light.direction, light.color, light.intensity, base_albedo, material.metallic, material.roughness),
                    LightType::Point { radius } => {
                        let to_l = light.position - centroid;
                        let dist = to_l.length();
                        if dist > radius { continue; }
                        let atten = 1.0 / (1.0 + dist*dist / (radius*radius));
                        Pbr::compute(normal, view_dir, to_l.normalize_or_zero(), light.color, light.intensity * atten, base_albedo, material.metallic, material.roughness)
                    }
                };
                total_light = total_light + contrib;
            }
            total_light = total_light + material.emissive * material.emissive_strength;
            total_light = total_light * material.ao;

            if max_health > 0.0 {
                let tint = 0.6 + 0.4 * (health / max_health);
                total_light = total_light * tint;
            }

            // Highlight selected objects
            let c = if selected {
                Vec3::new((total_light.x()+0.1).min(1.0), (total_light.y()+0.15).min(1.0), (total_light.z()+0.25).min(1.0))
            } else { total_light };

            let c0 = [c.x(), c.y(), c.z()];
            self.draw_triangle_hdr(s0, s1, s2, c0, c0, c0);

            // Wireframe lines
            if selected {
                let wc = [(c.x()+0.4).min(1.0)*0.5, (c.y()+0.6).min(1.0)*0.5, (c.z()+0.8).min(1.0)*0.5];
                self.draw_line_hdr(s0.x() as i32,s0.y() as i32,s0.z(), s1.x() as i32,s1.y() as i32,s1.z(), wc);
                self.draw_line_hdr(s1.x() as i32,s1.y() as i32,s1.z(), s2.x() as i32,s2.y() as i32,s2.z(), wc);
                self.draw_line_hdr(s2.x() as i32,s2.y() as i32,s2.z(), s0.x() as i32,s0.y() as i32,s0.z(), wc);
            } else {
                let wc = [c.x()*0.1, c.y()*0.1, c.z()*0.1];
                self.draw_line_hdr(s0.x() as i32,s0.y() as i32,s0.z()-0.001, s1.x() as i32,s1.y() as i32,s1.z()-0.001, wc);
                self.draw_line_hdr(s1.x() as i32,s1.y() as i32,s1.z()-0.001, s2.x() as i32,s2.y() as i32,s2.z()-0.001, wc);
                self.draw_line_hdr(s2.x() as i32,s2.y() as i32,s2.z()-0.001, s0.x() as i32,s0.y() as i32,s0.z()-0.001, wc);
            }
        }
    }

    fn draw_object_gizmo(&mut self, vp: &Mat4, pos: Vec3, len: f32) {
        if let (Some(o), Some(x)) = (self.project(pos,vp), self.project(pos+Vec3::new(len,0.0,0.0),vp)) {
            self.draw_line(o.x() as i32,o.y() as i32,o.z()-0.005, x.x() as i32,x.y() as i32,x.z()-0.005, 255,80,80);
        }
        if let (Some(o), Some(y)) = (self.project(pos,vp), self.project(pos+Vec3::new(0.0,len,0.0),vp)) {
            self.draw_line(o.x() as i32,o.y() as i32,o.z()-0.005, y.x() as i32,y.y() as i32,y.z()-0.005, 80,255,80);
        }
        if let (Some(o), Some(z)) = (self.project(pos,vp), self.project(pos+Vec3::new(0.0,0.0,len),vp)) {
            self.draw_line(o.x() as i32,o.y() as i32,o.z()-0.005, z.x() as i32,z.y() as i32,z.z()-0.005, 80,130,255);
        }
    }

    fn draw_skybox(&mut self, _cam: &Camera) {
        let zenith = Vec3::new(0.05, 0.07, 0.2);
        let horizon = Vec3::new(0.4, 0.5, 0.7);
        let ground = Vec3::new(0.15, 0.12, 0.1);
        for y in 0..self.height {
            let t = y as f32 / self.height as f32;
            let c = if t < 0.5 {
                let st = t * 2.0;
                horizon.lerp(zenith, st)
            } else {
                let st = (t - 0.5) * 2.0;
                ground.lerp(horizon, 1.0 - st)
            };
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                self.hdr[idx] = [c.x(), c.y(), c.z()];
            }
        }
    }

    fn update_particles(&mut self, dt: f32, vp: &Mat4) {
        for p in &mut self.particles {
            p.position = p.position + p.velocity * dt;
            p.velocity.1 -= 4.0 * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
        for p in &self.particles.clone() {
            if let Some(sp) = self.project(p.position, vp) {
                let alpha = (p.life / p.max_life).clamp(0.0, 1.0);
                let r = (p.color.x() * alpha * 255.0) as u8;
                let g = (p.color.y() * alpha * 255.0) as u8;
                let b = (p.color.z() * alpha * 255.0) as u8;
                let sz = (p.size * 4.0) as i32;
                for dy in -sz..=sz {
                    for dx in -sz..=sz {
                        if dx*dx + dy*dy <= sz*sz {
                            self.set_pixel(sp.x() as i32+dx, sp.y() as i32+dy, sp.z(), r, g, b);
                        }
                    }
                }
            }
        }
    }

    pub fn spawn_particles(&mut self, pos: Vec3, count: u32, color: [f32; 3]) {
        for i in 0..count {
            // Simple deterministic pseudo-random using index
            let f = (i as f32 * 7.31 + 0.17).fract();
            let f2 = (i as f32 * 13.79 + 0.53).fract();
            let f3 = (i as f32 * 3.47 + 0.91).fract();
            self.particles.push(WebParticle {
                position: pos,
                velocity: Vec3::new(f * 4.0 - 2.0, f2 * 4.0 + 1.0, f3 * 4.0 - 2.0),
                life: 0.3 + f * 0.7,
                max_life: 1.0,
                color: Vec3::new(color[0], color[1], color[2]),
                size: 0.05 + f2 * 0.1,
            });
        }
    }

    /// Full render pass: skybox → grid → meshes → particles → tonemap → gamma
    pub fn render_scene(&mut self, scene: &Scene, selected_id: Option<usize>, dt: f32) {
        let (w, h) = (self.width, self.height);
        let aspect = w as f32 / h as f32;
        let view = scene.camera.view_matrix();
        let proj = scene.camera.projection_matrix(aspect);
        let vp = proj.mul_mat4(view);

        // Clear
        self.clear_hdr();
        self.depth.fill(f32::INFINITY);

        // Skybox
        self.draw_skybox(&scene.camera);

        // Grid
        if self.grid_enabled { self.draw_grid(&vp); }
        self.draw_axes(&vp, 3.0);

        // Sort opaque back-to-front (painter's)
        let cam_pos = scene.camera.position();
        let mut sorted: Vec<&SceneObject> = scene.objects.iter().filter(|o| o.visible && !o.is_light).collect();
        sorted.sort_by(|a, b| {
            let da = (a.transform.position - cam_pos).length_squared();
            let db = (b.transform.position - cam_pos).length_squared();
            db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Render meshes
        for obj in sorted {
            let mesh = build_mesh(obj.geometry);
            let model = obj.transform.to_matrix();
            let sel = selected_id.map_or(false, |s| s == obj.id);
            self.draw_mesh_pbr(
                &mesh, &model, &vp, &obj.material, obj.team,
                obj.health, obj.max_health, sel,
                &scene.lights, cam_pos, scene.ambient_color, scene.ambient_intensity,
            );
            if obj.health < obj.max_health {
                self.draw_health_bar(&vp, obj.transform.position, obj.health, obj.max_health);
            }
            if sel {
                self.draw_object_gizmo(&vp, obj.transform.position, 1.5);
            }
        }

        // Light indicators
        for light in &scene.lights {
            if let LightType::Point { .. } = light.light_type {
                if let Some(sp) = self.project(light.position, &vp) {
                    let r = 5;
                    for dy in -r..=r {
                        for dx in -r..=r {
                            if dx*dx+dy*dy <= r*r {
                                self.set_pixel(sp.x() as i32+dx, sp.y() as i32+dy, sp.z()-0.01,
                                    (light.color.x()*255.0) as u8, (light.color.y()*255.0) as u8, (light.color.z()*255.0) as u8);
                            }
                        }
                    }
                }
            }
        }

        // Particles
        self.update_particles(dt, &vp);

        // Tonemap + Gamma + Vignette → final color buffer
        self.tonemap_to_color();
    }

    fn clear_hdr(&mut self) {
        for c in self.hdr.iter_mut() { *c = [0.0; 3]; }
    }

    fn tonemap_to_color(&mut self) {
        let w = self.width; let h = self.height;
        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                let mut c = Vec3::new(self.hdr[idx][0], self.hdr[idx][1], self.hdr[idx][2]);
                c = c * self.post_exposure;
                c = aces(c);
                // Gamma
                c = Vec3::new(c.x().powf(1.0/2.2), c.y().powf(1.0/2.2), c.z().powf(1.0/2.2));
                // Vignette
                let nx = (x as f32 / w as f32 - 0.5) * 2.0;
                let ny = (y as f32 / h as f32 - 0.5) * 2.0;
                let vig = 1.0 - (nx*nx + ny*ny) * self.post_vignette * 0.5;
                c = c * vig.clamp(0.0, 1.0);
                // Contrast + Saturation
                let luma = 0.299*c.x() + 0.587*c.y() + 0.114*c.z();
                c = Vec3::new(luma + (c.x()-luma)*1.05, luma + (c.y()-luma)*1.05, luma + (c.z()-luma)*1.05);
                c = (c - Vec3::splat(0.5)) * 1.1 + Vec3::splat(0.5);

                let pi = self.color.as_mut_ptr();
                unsafe {
                    *pi.add(idx*4) = (c.x().clamp(0.0,1.0) * 255.0) as u8;
                    *pi.add(idx*4+1) = (c.y().clamp(0.0,1.0) * 255.0) as u8;
                    *pi.add(idx*4+2) = (c.z().clamp(0.0,1.0) * 255.0) as u8;
                    *pi.add(idx*4+3) = 255;
                }
            }
        }
    }
}
