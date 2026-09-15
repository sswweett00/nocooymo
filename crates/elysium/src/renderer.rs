/// renderer.rs — Elysium Software Rasterizer
/// Gerçek 3D sahneyi bir RGBA pixel buffer'a render eder.
/// PBR ışıklandırma, skybox, sis efekti, çoklu ışık, post-processing içerir.

use core::fmt;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use glam::{Mat4, Vec2, Vec3, Vec4};
use serde::{Serialize, Deserialize};

// ═══════════════════════════════════════════════════════════ PBR Işıklandırma Sistemi

/// Işık türü
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum LightType {
    Directional,
    Point { radius: f32 },
    Spot { radius: f32, inner_angle: f32, outer_angle: f32 },
}

/// Sahne ışığı
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SceneLight {
    pub position: Vec3,
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub light_type: LightType,
    pub cast_shadows: bool,
    pub shadow_bias: f32,
    pub range: f32,
}

impl SceneLight {
    pub fn directional(direction: Vec3, color: Vec3, intensity: f32) -> Self {
        Self {
            position: Vec3::ZERO,
            direction: direction.normalize(),
            color,
            intensity,
            light_type: LightType::Directional,
            cast_shadows: true,
            shadow_bias: 0.005,
            range: f32::MAX,
        }
    }

    pub fn point(position: Vec3, color: Vec3, intensity: f32, radius: f32) -> Self {
        Self {
            position,
            direction: Vec3::ZERO,
            color,
            intensity,
            light_type: LightType::Point { radius },
            cast_shadows: false,
            shadow_bias: 0.005,
            range: radius,
        }
    }

    pub fn spot(position: Vec3, direction: Vec3, color: Vec3, intensity: f32, radius: f32, inner_angle: f32, outer_angle: f32) -> Self {
        Self {
            position,
            direction: direction.normalize(),
            color,
            intensity,
            light_type: LightType::Spot { radius, inner_angle, outer_angle },
            cast_shadows: false,
            shadow_bias: 0.005,
            range: radius,
        }
    }
}

/// PBR materyal parametreleri
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PbrMaterial {
    pub albedo: Vec3,
    pub metallic: f32,
    pub roughness: f32,
    pub ao: f32,
    pub emissive: Vec3,
    pub emissive_strength: f32,
    // Texture slotları (texture ID → Scene.textures)
    pub albedo_texture: Option<u32>,
    pub normal_texture: Option<u32>,
    pub roughness_texture: Option<u32>,
    pub metallic_texture: Option<u32>,
    pub ao_texture: Option<u32>,
    pub emissive_texture: Option<u32>,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            albedo: Vec3::ONE,
            metallic: 0.0,
            roughness: 0.5,
            ao: 1.0,
            emissive: Vec3::ZERO,
            emissive_strength: 0.0,
            albedo_texture: None,
            normal_texture: None,
            roughness_texture: None,
            metallic_texture: None,
            ao_texture: None,
            emissive_texture: None,
        }
    }
}

/// PBR Cook-Torrance BRDF hesaplayıcı
struct PbrCalc;

impl PbrCalc {
    #[inline]
    fn fresnel(cos_theta: f32, f0: Vec3) -> Vec3 {
        let x = (1.0 - cos_theta).clamp(0.0, 1.0);
        let x5 = x * x * x * x * x;
        f0 + (Vec3::ONE - f0) * x5
    }

    #[inline]
    fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
        let a = roughness * roughness;
        let a2 = a * a;
        let d = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
        a2 / (std::f32::consts::PI * d * d + 0.0001)
    }

    #[inline]
    fn geometry_schlick(n_dot_v: f32, roughness: f32) -> f32 {
        let k = (roughness + 1.0).powi(2) / 8.0;
        n_dot_v / (n_dot_v * (1.0 - k) + k + 0.0001)
    }

    #[inline]
    fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
        Self::geometry_schlick(n_dot_v, roughness) * Self::geometry_schlick(n_dot_l, roughness)
    }

    #[inline]
    fn compute(
        n: Vec3, v: Vec3, l: Vec3, light_color: Vec3, light_intensity: f32,
        albedo: Vec3, metallic: f32, roughness: f32,
    ) -> Vec3 {
        let n_dot_l = n.dot(l).max(0.0);
        if n_dot_l <= 0.0 { return Vec3::ZERO; }
        let h = (v + l).normalize();
        let n_dot_v = n.dot(v).max(0.001);
        let n_dot_h = n.dot(h).max(0.0);
        let v_dot_h = v.dot(h).max(0.0);
        let f0 = Vec3::splat(0.04).lerp(albedo, metallic);
        let d = Self::distribution_ggx(n_dot_h, roughness);
        let g = Self::geometry_smith(n_dot_v, n_dot_l, roughness);
        let f = Self::fresnel(v_dot_h, f0);
        let specular = (d * g * f) / (4.0 * n_dot_v * n_dot_l + 0.0001);
        let k_d = (Vec3::ONE - f) * (1.0 - metallic);
        let diffuse = k_d * albedo / std::f32::consts::PI;
        (diffuse + specular) * light_color * light_intensity * n_dot_l
    }
}

/// Sis (fog) parametreleri
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct FogParams {
    pub enabled: bool,
    pub color: Vec3,
    pub density: f32,
    pub start: f32,
    pub end: f32,
}

impl Default for FogParams {
    fn default() -> Self {
        Self {
            enabled: true,
            color: Vec3::new(0.05, 0.06, 0.1),
            density: 0.015,
            start: 30.0,
            end: 120.0,
        }
    }
}

/// Ton mapping modu
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TonemapMode {
    Reinhard,
    Aces,
    Uncharted2,
    Linear,
}

/// Post-processing parametreleri
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PostProcessParams {
    pub tonemap_mode: TonemapMode,
    pub exposure: f32,
    pub gamma: f32,
    pub fxaa_enabled: bool,
    pub bloom_threshold: f32,
    pub bloom_intensity: f32,
    pub vignette_strength: f32,
    pub chromatic_aberration: f32,
    pub contrast: f32,
    pub saturation: f32,
}

impl Default for PostProcessParams {
    fn default() -> Self {
        Self {
            tonemap_mode: TonemapMode::Aces,
            exposure: 1.2,
            gamma: 2.2,
            fxaa_enabled: true,
            bloom_threshold: 0.8,
            bloom_intensity: 0.15,
            vignette_strength: 0.3,
            chromatic_aberration: 0.002,
            contrast: 1.1,
            saturation: 1.05,
        }
    }
}

/// HDR framebuffer (post-processing için)
#[derive(Clone, Debug)]
struct HdrBuffer {
    color: Vec<[f32; 3]>,
    width: u32,
    height: u32,
}

impl HdrBuffer {
    fn new(width: u32, height: u32) -> Self {
        Self { color: vec![[0.0; 3]; (width * height) as usize], width, height }
    }
    fn resize(&mut self, w: u32, h: u32) {
        self.width = w; self.height = h;
        self.color.resize((w * h) as usize, [0.0; 3]);
    }
    #[inline]
    fn get(&self, x: u32, y: u32) -> [f32; 3] {
        if x < self.width && y < self.height { self.color[(y * self.width + x) as usize] } else { [0.0; 3] }
    }
    #[inline]
    fn set(&mut self, x: u32, y: u32, c: [f32; 3]) {
        if x < self.width && y < self.height { self.color[(y * self.width + x) as usize] = c; }
    }
    fn clear(&mut self) { self.color.iter_mut().for_each(|c| *c = [0.0; 3]); }
}

/// Post-processing fonksiyonları
struct PostProc;

impl PostProc {
    #[inline]
    fn aces(c: [f32; 3]) -> [f32; 3] {
        let (a,b,cc,d,e) = (2.51, 0.03, 2.43, 0.59, 0.14);
        [
            ((c[0]*(a*c[0]+b))/(c[0]*(cc*c[0]+d)+e)).clamp(0.0,1.0),
            ((c[1]*(a*c[1]+b))/(c[1]*(cc*c[1]+d)+e)).clamp(0.0,1.0),
            ((c[2]*(a*c[2]+b))/(c[2]*(cc*c[2]+d)+e)).clamp(0.0,1.0),
        ]
    }
    #[inline]
    fn reinhard(c: [f32; 3]) -> [f32; 3] {
        [c[0]/(1.0+c[0]), c[1]/(1.0+c[1]), c[2]/(1.0+c[2])]
    }
    #[inline]
    fn uncharted2_partial(x: f32) -> f32 {
        ((x*(0.15*x+0.10*0.50)+0.20*0.02)/(x*(0.15*x+0.50)+0.20*0.30))-0.02/0.30
    }
    fn tonemap(c: [f32; 3], mode: TonemapMode, exposure: f32) -> [f32; 3] {
        let e = [c[0]*exposure, c[1]*exposure, c[2]*exposure];
        match mode {
            TonemapMode::Reinhard => Self::reinhard(e),
            TonemapMode::Aces => Self::aces(e),
            TonemapMode::Uncharted2 => {
                let w = Self::uncharted2_partial(11.2);
                let c = [Self::uncharted2_partial(e[0]*2.0*exposure), Self::uncharted2_partial(e[1]*2.0*exposure), Self::uncharted2_partial(e[2]*2.0*exposure)];
                [c[0]/w, c[1]/w, c[2]/w]
            }
            TonemapMode::Linear => e.map(|v| v.clamp(0.0, 1.0)),
        }
    }
    #[inline]
    fn gamma(c: [f32; 3], g: f32) -> [f32; 3] {
        let ig = 1.0/g; [c[0].powf(ig), c[1].powf(ig), c[2].powf(ig)]
    }
    fn gaussian_3x3(src: &HdrBuffer, dst: &mut HdrBuffer) {
        let k = [[1.0/16.0,2.0/16.0,1.0/16.0],[2.0/16.0,4.0/16.0,2.0/16.0],[1.0/16.0,2.0/16.0,1.0/16.0]];
        for y in 0..src.height { for x in 0..src.width {
            let mut s = [0.0f32;3];
            for ky in 0..3i32 { for kx in 0..3i32 {
                let sx = (x as i32+kx-1).max(0).min(src.width as i32-1) as u32;
                let sy = (y as i32+ky-1).max(0).min(src.height as i32-1) as u32;
                let p = src.get(sx,sy); let w = k[ky as usize][kx as usize];
                s[0]+=p[0]*w; s[1]+=p[1]*w; s[2]+=p[2]*w;
            }} dst.set(x,y,s);
        }}
    }
    fn apply_fxaa(src: &HdrBuffer, dst: &mut HdrBuffer) {
        for y in 0..src.height { for x in 0..src.width {
            let c = src.get(x,y);
            if x==0||y==0||x>=src.width-1||y>=src.height-1 { dst.set(x,y,c); continue; }
            let nw=src.get(x-1,y-1); let ne=src.get(x+1,y-1); let sw=src.get(x-1,y+1); let se=src.get(x+1,y+1);
            let luma=|c:[f32;3]|->f32{0.299*c[0]+0.587*c[1]+0.114*c[2]};
            let lm=luma(c); let lmin=lm.min(luma(nw)).min(luma(ne)).min(luma(sw)).min(luma(se));
            let lmax=lm.max(luma(nw)).max(luma(ne)).max(luma(sw)).max(luma(se));
            let lr=lmax-lmin;
            if lr < 0.083f32.max(lmax*0.05) { dst.set(x,y,c); continue; }
            let avg=[(nw[0]+ne[0]+sw[0]+se[0]+c[0]*4.0)/8.0,(nw[1]+ne[1]+sw[1]+se[1]+c[1]*4.0)/8.0,(nw[2]+ne[2]+sw[2]+se[2]+c[2]*4.0)/8.0];
            let b=((lr-0.083)/(lmax*0.05-0.083)).clamp(0.0,0.5);
            dst.set(x,y,[c[0]+(avg[0]-c[0])*b, c[1]+(avg[1]-c[1])*b, c[2]+(avg[2]-c[2])*b]);
        }}
    }
}

/// Skybox renkleri (gradient tabanlı)
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SkyboxConfig {
    pub enabled: bool,
    pub zenith_color: Vec3,
    pub horizon_color: Vec3,
    pub ground_color: Vec3,
    pub sun_direction: Vec3,
    pub sun_color: Vec3,
    pub sun_intensity: f32,
    pub star_density: f32,
}

impl Default for SkyboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            zenith_color: Vec3::new(0.05, 0.07, 0.2),
            horizon_color: Vec3::new(0.4, 0.5, 0.7),
            ground_color: Vec3::new(0.15, 0.12, 0.1),
            sun_direction: Vec3::new(0.5, 0.8, 0.3).normalize(),
            sun_color: Vec3::new(1.0, 0.95, 0.8),
            sun_intensity: 2.0,
            star_density: 0.001,
        }
    }
}

// ─────────────────────────────────────────────────────────── Geometry Types

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeometryType {
    Cube,
    Sphere,
    Plane,
    Cylinder,
    Capsule,
    Custom(#[serde(skip)] u32), // Custom mesh ID
}

impl GeometryType {
    pub fn is_custom(&self) -> bool {
        matches!(self, GeometryType::Custom(_))
    }
}

// ═══════════════════════════════════════════════════════════ OBJ Parser

/// Yüklü OBJ mesh verisi
#[derive(Clone, Debug)]
pub struct ObjMesh {
    pub name: String,
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<[f32; 2]>,
    pub faces: Vec<ObjFace>,
    pub material_name: Option<String>,
    pub triangle_count: usize,
    pub vertex_count: usize,
}

#[derive(Clone, Debug)]
pub struct ObjFace {
    pub vertices: Vec<(usize, Option<usize>, Option<usize>)>, // (v_idx, vt_idx, vn_idx)
}

impl ObjFace {
    pub fn triangulate(&self) -> Vec<[usize; 3]> {
        let mut tris = Vec::new();
        if self.vertices.len() < 3 { return tris; }
        // Fan triangulation
        for i in 1..self.vertices.len() - 1 {
            tris.push([self.vertices[0].0, self.vertices[i].0, self.vertices[i + 1].0]);
        }
        tris
    }
}

/// OBJ dosyasını parse et
pub fn parse_obj(data: &str) -> ObjMesh {
    let mut name = String::from("Unnamed");
    let mut positions: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut faces: Vec<ObjFace> = Vec::new();
    let mut material_name: Option<String> = None;
    let mut current_object = String::new();

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts[0] {
            "o" => {
                if parts.len() > 1 {
                    current_object = parts[1..].join(" ");
                    if name == "Unnamed" { name = current_object.clone(); }
                }
            }
            "v" => {
                if parts.len() >= 4 {
                    if let (Ok(x), Ok(y), Ok(z)) = (parts[1].parse::<f32>(), parts[2].parse::<f32>(), parts[3].parse::<f32>()) {
                        positions.push(Vec3::new(x, y, z));
                    }
                }
            }
            "vn" => {
                if parts.len() >= 4 {
                    if let (Ok(x), Ok(y), Ok(z)) = (parts[1].parse::<f32>(), parts[2].parse::<f32>(), parts[3].parse::<f32>()) {
                        normals.push(Vec3::new(x, y, z));
                    }
                }
            }
            "vt" => {
                if parts.len() >= 3 {
                    if let (Ok(u), Ok(v)) = (parts[1].parse::<f32>(), parts[2].parse::<f32>()) {
                        uvs.push([u, v]);
                    }
                }
            }
            "f" => {
                if parts.len() >= 4 {
                    let mut face_verts = Vec::new();
                    for part in &parts[1..] {
                        let indices: Vec<Option<usize>> = part.split('/')
                            .map(|s| s.parse::<usize>().ok().map(|i| i.wrapping_sub(1)))
                            .collect();
                        let v_idx = indices.get(0).and_then(|&x| x).unwrap_or(0);
                        let vt_idx = indices.get(1).and_then(|&x| x);
                        let vn_idx = indices.get(2).and_then(|&x| x);
                        face_verts.push((v_idx, vt_idx, vn_idx));
                    }
                    faces.push(ObjFace { vertices: face_verts });
                }
            }
            "usemtl" => {
                if parts.len() > 1 { material_name = Some(parts[1].to_string()); }
            }
            _ => {}
        }
    }

    let vertex_count = positions.len();
    let triangle_count: usize = faces.iter().map(|f| f.triangulate().len()).sum();

    ObjMesh {
        name,
        vertices: positions,
        normals,
        uvs,
        faces,
        material_name,
        triangle_count,
        vertex_count,
    }
}

/// OBJ mesh'ini SoftwareRenderer Mesh'e dönüştür
pub fn obj_to_mesh(obj: &ObjMesh) -> Mesh {
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();

    // Face'leri triangle'lara çevir ve vertex'leri yeniden indeksle
    let mut vertex_map: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut next_idx = 0usize;

    for face in &obj.faces {
        let tris = face.triangulate();
        for tri in &tris {
            let mut new_indices = [0usize; 3];
            for (k, &v_idx) in tri.iter().enumerate() {
                if let Some(&mapped) = vertex_map.get(&v_idx) {
                    new_indices[k] = mapped;
                } else {
                    let pos = obj.vertices.get(v_idx).copied().unwrap_or(Vec3::ZERO);
                    vertices.push(pos);
                    vertex_map.insert(v_idx, next_idx);
                    new_indices[k] = next_idx;
                    next_idx += 1;
                }
            }
            triangles.push(Tri(new_indices[0], new_indices[1], new_indices[2]));
        }
    }

    Mesh { vertices, triangles }
}

impl fmt::Display for GeometryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeometryType::Cube => write!(f, "Cube"),
            GeometryType::Sphere => write!(f, "Sphere"),
            GeometryType::Plane => write!(f, "Plane"),
            GeometryType::Cylinder => write!(f, "Cylinder"),
            GeometryType::Capsule => write!(f, "Capsule"),
            GeometryType::Custom(id) => write!(f, "Custom({})", id),
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

#[derive(Clone, Copy, Debug)]
pub struct Tri(pub usize, pub usize, pub usize);

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<Tri>,
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
        GeometryType::Custom(_) => return Box::leak(Box::new(Mesh { vertices: Vec::new(), triangles: Vec::new() })),
    };
    let mut map = cache.lock().unwrap();
    *map.entry(key).or_insert_with(move || {
        let mesh = match geometry {
            GeometryType::Cube => cube_mesh(),
            GeometryType::Sphere => sphere_mesh(16, 24),
            GeometryType::Plane => plane_mesh(),
            GeometryType::Cylinder => cylinder_mesh(20),
            GeometryType::Capsule => capsule_mesh(20, 0.8),
            GeometryType::Custom(_) => Mesh { vertices: Vec::new(), triangles: Vec::new() },
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

/// Fizik gövdesi türü
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum RigidBodyType {
    Static,
    Dynamic,
    Kinematic,
}

/// Fizik gövdesi bileşeni
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RigidBody {
    pub body_type: RigidBodyType,
    pub mass: f32,
    pub restitution: f32,
    pub friction: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub is_gravity_enabled: bool,
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            body_type: RigidBodyType::Static,
            mass: 1.0,
            restitution: 0.3,
            friction: 0.5,
            linear_damping: 0.01,
            angular_damping: 0.05,
            velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            is_gravity_enabled: true,
        }
    }
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
    pub material: PbrMaterial,
    pub is_light: bool,
    pub light_id: Option<usize>,
    pub rigid_body: Option<RigidBody>,
    pub collider_radius: f32,
    pub tags: Vec<String>,
    ///骨骼动画ID — Some ise bu nesne bir skinned mesh'tir
    pub skeleton_id: Option<usize>,
}

impl SceneObject {
    pub fn with_pbr(mut self, metallic: f32, roughness: f32) -> Self {
        self.material.metallic = metallic;
        self.material.roughness = roughness;
        self
    }

    pub fn with_emissive(mut self, color: Vec3, strength: f32) -> Self {
        self.material.emissive = color;
        self.material.emissive_strength = strength;
        self
    }
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
    pub skybox: SkyboxConfig,
    pub fog: FogParams,
    pub post_process: PostProcessParams,
    color: Vec<u8>,
    depth: Vec<f32>,
    hdr: HdrBuffer,
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
            skybox: SkyboxConfig::default(),
            fog: FogParams::default(),
            post_process: PostProcessParams::default(),
            color: vec![0u8; n * 4],
            depth: vec![f32::INFINITY; n],
            hdr: HdrBuffer::new(width, height),
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
            self.hdr.resize(width, height);
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

    fn draw_mesh_pbr(
        &mut self,
        mesh: &Mesh,
        model: &Mat4,
        vp: &Mat4,
        material: &PbrMaterial,
        team: EntityTeam,
        health: f32,
        max_health: f32,
        selected: bool,
        wireframe_only: bool,
        lights: &[SceneLight],
        camera_pos: Vec3,
        ambient_color: Vec3,
        ambient_intensity: f32,
        albedo_tex: Option<&crate::texture::Texture>,
    ) {
        let mvp = *vp * *model;
        let view_dir = (camera_pos - model.transform_point3(Vec3::ZERO)).normalize_or_zero();

        let mut base_albedo = material.albedo;
        if team == EntityTeam::Enemy { base_albedo = Vec3::new(0.85, 0.2, 0.2); }
        else if team == EntityTeam::Player { base_albedo = Vec3::new(0.2, 0.5, 0.9); }

        // Albedo texture varsa centroid UV ile örnekleyerek base_albedo'yu çarp
        let has_albedo_tex = albedo_tex.is_some();

        for tri in &mesh.triangles {
            let v0_w = model.transform_point3(mesh.vertices[tri.0]);
            let v1_w = model.transform_point3(mesh.vertices[tri.1]);
            let v2_w = model.transform_point3(mesh.vertices[tri.2]);

            let edge1 = v1_w - v0_w;
            let edge2 = v2_w - v0_w;
            let normal = edge1.cross(edge2).normalize_or_zero();

            // Centroid UV hesapla (basit texture mapping)
            let mut tri_albedo = base_albedo;
            if has_albedo_tex {
                if let Some(tex) = albedo_tex {
                    // Centroid pozisyonuna göre basit planar UV
                    let centroid = (v0_w + v1_w + v2_w) / 3.0;
                    let uv_u = (centroid.x * 0.5 + 0.5).fract();
                    let uv_v = (centroid.z * 0.5 + 0.5).fract();
                    let tex_color = tex.sample_color(uv_u, uv_v);
                    tri_albedo = base_albedo * tex_color;
                }
            }

            let Some(s0) = self.project(v0_w, &mvp) else { continue };
            let Some(s1) = self.project(v1_w, &mvp) else { continue };
            let Some(s2) = self.project(v2_w, &mvp) else { continue };

            // PBR: Tüm ışık kaynaklarından toplam aydınlatma
            let mut total_light = ambient_color * ambient_intensity;
            for light in lights {
                let light_contrib = match light.light_type {
                    LightType::Directional => {
                        PbrCalc::compute(normal, view_dir, -light.direction, light.color, light.intensity,
                            tri_albedo, material.metallic, material.roughness)
                    }
                    LightType::Point { radius } => {
                        let to_light = light.position - (v0_w + v1_w + v2_w) / 3.0;
                        let dist = to_light.length();
                        if dist > radius { continue; }
                        let attenuation = 1.0 / (1.0 + dist * dist / (radius * radius));
                        PbrCalc::compute(normal, view_dir, to_light.normalize_or_zero(), light.color,
                            light.intensity * attenuation, tri_albedo, material.metallic, material.roughness)
                    }
                    LightType::Spot { radius, inner_angle, outer_angle } => {
                        let to_light = light.position - (v0_w + v1_w + v2_w) / 3.0;
                        let dist = to_light.length();
                        if dist > radius { continue; }
                        let spot_dir = to_light.normalize_or_zero();
                        let theta = spot_dir.dot(-light.direction);
                        let cos_inner = inner_angle.cos();
                        let cos_outer = outer_angle.cos();
                        if theta < cos_outer { continue; }
                        let spot_factor = ((theta - cos_outer) / (cos_inner - cos_outer)).clamp(0.0, 1.0);
                        let attenuation = spot_factor / (1.0 + dist * dist / (radius * radius));
                        PbrCalc::compute(normal, view_dir, spot_dir, light.color,
                            light.intensity * attenuation, tri_albedo, material.metallic, material.roughness)
                    }
                };
                total_light += light_contrib;
            }

            // Emissive
            total_light += material.emissive * material.emissive_strength;

            // AO
            total_light *= material.ao;

            // Health tint
            if max_health > 0.0 {
                let tint = 0.6 + 0.4 * (health / max_health);
                total_light *= tint;
            }

            let sr = (total_light.x.min(1.0) * 255.0) as u8;
            let sg = (total_light.y.min(1.0) * 255.0) as u8;
            let sb = (total_light.z.min(1.0) * 255.0) as u8;

            if selected {
                if !wireframe_only {
                    self.draw_triangle(s0, s1, s2, (sr as u32 + 20).min(255) as u8, (sg as u32 + 30).min(255) as u8, (sb as u32 + 50).min(255) as u8);
                }
                self.draw_line(s0.x as i32, s0.y as i32, s0.z, s1.x as i32, s1.y as i32, s1.z, 120, 200, 255);
                self.draw_line(s1.x as i32, s1.y as i32, s1.z, s2.x as i32, s2.y as i32, s2.z, 120, 200, 255);
                self.draw_line(s2.x as i32, s2.y as i32, s2.z, s0.x as i32, s0.y as i32, s0.z, 120, 200, 255);
            } else {
                if !wireframe_only {
                    self.draw_triangle(s0, s1, s2, sr, sg, sb);
                }
                let wr = (total_light.x * 0.2 * 255.0) as u8;
                let wg = (total_light.y * 0.2 * 255.0) as u8;
                let wb = (total_light.z * 0.2 * 255.0) as u8;
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

    fn draw_skybox(&mut self, cam: &Camera, skybox: &SkyboxConfig) {
        if !skybox.enabled { return; }
        let (w, h) = (self.width, self.height);
        let _cam_pos = cam.position();
        let view = cam.view_matrix();
        let aspect = self.viewport_size.0 as f32 / self.viewport_size.1 as f32;
        let proj = cam.projection_matrix(aspect);
        let inv_vp = (proj * view).inverse();

        for y in 0..h {
            for x in 0..w {
                let ndc_x = (x as f32 / w as f32) * 2.0 - 1.0;
                let ndc_y = 1.0 - (y as f32 / h as f32) * 2.0;
                let near = inv_vp * Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
                let far = inv_vp * Vec4::new(ndc_x, ndc_y, 1.0, 1.0);
                if near.w.abs() < 1e-6 || far.w.abs() < 1e-6 { continue; }
                let dir = (far.truncate() / far.w - near.truncate() / near.w).normalize();

                // Sky gradient
                let t = (dir.y * 0.5 + 0.5).clamp(0.0, 1.0);
                let sky_color = if dir.y > 0.0 {
                    skybox.zenith_color.lerp(skybox.horizon_color, 1.0 - t)
                } else {
                    skybox.horizon_color.lerp(skybox.ground_color, (-dir.y).clamp(0.0, 1.0))
                };

                // Sun glow
                let sun_dot = dir.dot(skybox.sun_direction).max(0.0);
                let sun_glow = sun_dot.powi(32) * skybox.sun_intensity;
                let sun_disc = if sun_dot > 0.999 { skybox.sun_intensity * 2.0 } else { 0.0 };
                let mut color = sky_color + skybox.sun_color * (sun_glow + sun_disc);

                // Stars (deterministic noise)
                if dir.y > 0.1 && skybox.star_density > 0.0 {
                    let hash = ((dir.x * 1000.0) as i32).wrapping_mul(374761393) ^ ((dir.z * 1000.0) as i32).wrapping_mul(668265263);
                    let hash_f = (hash as f32).abs() / i32::MAX as f32;
                    if hash_f < skybox.star_density {
                        let twinkle = (hash_f * 100.0).sin() * 0.5 + 0.5;
                        color += Vec3::splat(twinkle * 0.8);
                    }
                }

                let idx = ((y * w + x) * 4) as usize;
                let c = color * 255.0;
                self.color[idx] = c.x.min(255.0) as u8;
                self.color[idx + 1] = c.y.min(255.0) as u8;
                self.color[idx + 2] = c.z.min(255.0) as u8;
                self.color[idx + 3] = 255;
                self.depth[(y * w + x) as usize] = f32::INFINITY;
            }
        }
    }

    fn apply_fog_to_pixel(&self, color: [f32; 3], depth: f32, fog: &FogParams, _cam_pos: Vec3) -> [f32; 3] {
        if !fog.enabled || depth <= 0.0 { return color; }
        let dist = (depth * 100.0).abs(); // Derinliği yaklaşık mesafeye çevir
        let fog_factor = if fog.end > fog.start {
            ((dist - fog.start) / (fog.end - fog.start)).clamp(0.0, 1.0)
        } else {
            (1.0 - (-fog.density * dist).exp()).clamp(0.0, 1.0)
        };
        [
            color[0] + (fog.color.x - color[0]) * fog_factor,
            color[1] + (fog.color.y - color[1]) * fog_factor,
            color[2] + (fog.color.z - color[2]) * fog_factor,
        ]
    }

    pub fn render_scene(
        &mut self,
        scene: &Scene,
        selected_id: Option<usize>,
        dt: f32,
    ) -> &[u8] {
        self.update_particles(dt);

        // Skybox çiz (HDR benzeri)
        let skybox_copy = self.skybox;
        self.draw_skybox(&scene.camera, &skybox_copy);
        // derinliği skybox'tan sonra sıfırla (skybox zaten derinlik yazdı)
        // Clear only depth buffer; skybox renkleri korunur
        let n = (self.width * self.height) as usize;
        for i in 0..n {
            self.depth[i] = f32::INFINITY;
        }
        self.hdr.clear();

        let aspect = self.viewport_size.0 as f32 / self.viewport_size.1 as f32;
        let view = scene.camera.view_matrix();
        let proj = scene.camera.projection_matrix(aspect);
        let vp = proj * view;
        let cam_pos = scene.camera.position();

        // Grid ve eksenler
        if self.grid_enabled {
            self.draw_grid(&vp, scene.grid_size, scene.grid_divisions);
        }
        self.draw_axes(&vp, 2.0);

        // Sahne nesnelerini PBR ile çiz
        for obj in &scene.objects {
            if !obj.visible { continue; }
            let model = obj.transform.to_matrix();
            let selected = selected_id == Some(obj.id);

            // Custom mesh mi yoksa built-in mi?
            // Albedo texture'ı bul
            let albedo_tex = obj.material.albedo_texture
                .and_then(|id| scene.textures.get(&id));

            if let GeometryType::Custom(mesh_id) = obj.geometry {
                if let Some(mesh) = scene.custom_meshes.get(&mesh_id) {
                    self.draw_mesh_pbr(
                        mesh, &model, &vp, &obj.material, obj.team, obj.health, obj.max_health,
                        selected, false, &scene.lights, cam_pos, scene.ambient_color, scene.ambient_intensity,
                        albedo_tex,
                    );
                }
            } else {
                let mesh = build_mesh(obj.geometry);
                self.draw_mesh_pbr(
                    mesh, &model, &vp, &obj.material, obj.team, obj.health, obj.max_health,
                    selected, false, &scene.lights, cam_pos, scene.ambient_color, scene.ambient_intensity,
                    albedo_tex,
                );
            }

            if selected {
                self.draw_object_gizmo(&vp, obj.transform.position, 1.5);
            }
            if obj.team == EntityTeam::Enemy && obj.health > 0.0 {
                self.draw_health_bar(&vp, obj.transform.position, obj.health, obj.max_health);
            }
        }

        self.draw_particles(&vp);

        // ─── Post-processing: HDR → LDR pipeline ───
        // 1) Mevcut framebuffer'ı HDR buffer'a kopyala
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                let c = [self.color[idx*4] as f32/255.0, self.color[idx*4+1] as f32/255.0, self.color[idx*4+2] as f32/255.0];
                self.hdr.set(x, y, c);
            }
        }

        // 2) Bloom: parlak pikselleri bulanıklaştır ve karıştır
        if self.post_process.bloom_intensity > 0.0 {
            let mut bloom_buf = HdrBuffer::new(self.width, self.height);
            // Brightness extraction
            for y in 0..self.height {
                for x in 0..self.width {
                    let c = self.hdr.get(x, y);
                    let brightness = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
                    if brightness > self.post_process.bloom_threshold {
                        let factor = (brightness - self.post_process.bloom_threshold).min(1.0);
                        bloom_buf.set(x, y, [c[0]*factor, c[1]*factor, c[2]*factor]);
                    }
                }
            }
            // Gaussian blur (2 passes)
            let mut bloom_blurred = HdrBuffer::new(self.width, self.height);
            PostProc::gaussian_3x3(&bloom_buf, &mut bloom_blurred);
            PostProc::gaussian_3x3(&bloom_blurred, &mut bloom_buf);
            // Merge
            for y in 0..self.height {
                for x in 0..self.width {
                    let c = self.hdr.get(x, y);
                    let b = bloom_buf.get(x, y);
                    self.hdr.set(x, y, [
                        c[0] + b[0] * self.post_process.bloom_intensity,
                        c[1] + b[1] * self.post_process.bloom_intensity,
                        c[2] + b[2] * self.post_process.bloom_intensity,
                    ]);
                }
            }
        }

        // 3) Sis uygula
        if self.fog.enabled {
            for y in 0..self.height {
                for x in 0..self.width {
                    let idx = (y * self.width + x) as usize;
                    let d = self.depth[idx];
                    let c = self.hdr.get(x, y);
                    self.hdr.set(x, y, self.apply_fog_to_pixel(c, d, &self.fog, cam_pos));
                }
            }
        }

        // 4) FXAA
        if self.post_process.fxaa_enabled {
            let mut aa_buf = HdrBuffer::new(self.width, self.height);
            PostProc::apply_fxaa(&self.hdr, &mut aa_buf);
            self.hdr.color = aa_buf.color;
        }

        // 5) Ton mapping + Gamma + Vignette → LDR framebuffer'a yaz
        for y in 0..self.height {
            for x in 0..self.width {
                let c = self.hdr.get(x, y);
                let mut c = PostProc::tonemap(c, self.post_process.tonemap_mode, self.post_process.exposure);

                // Contrast
                c = [((c[0]-0.5)*self.post_process.contrast+0.5).clamp(0.0,1.0),
                     ((c[1]-0.5)*self.post_process.contrast+0.5).clamp(0.0,1.0),
                     ((c[2]-0.5)*self.post_process.contrast+0.5).clamp(0.0,1.0)];

                // Saturation
                let luma = 0.2126*c[0] + 0.7152*c[1] + 0.0722*c[2];
                let s = self.post_process.saturation;
                c = [luma + (c[0]-luma)*s, luma + (c[1]-luma)*s, luma + (c[2]-luma)*s];

                // Chromatic Aberration (basit: R ve B kanalını kaydır)
                let ca = self.post_process.chromatic_aberration;
                if ca > 0.0 {
                    let vx = (x as f32 / self.width as f32 - 0.5) * 2.0;
                    let _vy = (y as f32 / self.height as f32 - 0.5) * 2.0;
                    let offset_r = (ca * vx * self.width as f32) as i32;
                    let offset_b = -(ca * vx * self.width as f32) as i32;
                    let rx = (x as i32 + offset_r).max(0).min(self.width as i32 - 1) as u32;
                    let bx = (x as i32 + offset_b).max(0).min(self.width as i32 - 1) as u32;
                    let rc = self.hdr.get(rx, y);
                    let bc = self.hdr.get(bx, y);
                    let rt = PostProc::tonemap(rc, self.post_process.tonemap_mode, self.post_process.exposure);
                    let bt = PostProc::tonemap(bc, self.post_process.tonemap_mode, self.post_process.exposure);
                    c[0] = rt[0]; // Red from offset
                    c[2] = bt[2]; // Blue from offset
                }

                // Vignette
                let vx = (x as f32 / self.width as f32 - 0.5) * 2.0;
                let vy = (y as f32 / self.height as f32 - 0.5) * 2.0;
                let vignette = 1.0 - (vx*vx + vy*vy) * self.post_process.vignette_strength;
                c = [c[0]*vignette, c[1]*vignette, c[2]*vignette];

                // CRT Scanline (yatay çizgiler)
                let scanline = if y % 3 == 0 { 0.92 } else { 1.0 };
                c = [c[0]*scanline, c[1]*scanline, c[2]*scanline];

                // Gamma correction
                c = PostProc::gamma(c, self.post_process.gamma);

                let idx = ((y * self.width + x) * 4) as usize;
                self.color[idx] = (c[0] * 255.0).min(255.0).max(0.0) as u8;
                self.color[idx+1] = (c[1] * 255.0).min(255.0).max(0.0) as u8;
                self.color[idx+2] = (c[2] * 255.0).min(255.0).max(0.0) as u8;
                self.color[idx+3] = 255;
            }
        }

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
    pub lights: Vec<SceneLight>,
    pub ambient_color: Vec3,
    pub ambient_intensity: f32,
    pub environment_map: Option<EnvironmentMap>,
    pub gravity: Vec3,
    pub physics_enabled: bool,
    pub ground_y: f32,
    pub custom_meshes: std::collections::HashMap<u32, Mesh>,
    pub next_mesh_id: u32,
    pub imported_obj_meshes: Vec<ObjMesh>,
    // Texture storage
    pub textures: std::collections::HashMap<u32, crate::texture::Texture>,
    pub next_texture_id: u32,
    pub texture_names: std::collections::HashMap<u32, String>,
    // Skeleton storage
    pub skeletons: std::collections::HashMap<usize, crate::skeletal::Skeleton>,
    pub next_skeleton_id: usize,
    pub animation_clips: std::collections::HashMap<String, crate::skeletal::AnimClip>,
    pub blend_machines: std::collections::HashMap<usize, crate::skeletal::BlendStateMachine>,
}

/// Ortam haritası (IBL için basit küresel harmonik)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvironmentMap {
    pub irradiance_coefficients: [[f32; 3]; 9], // SH katsayıları (L2)
    pub prefiltered_roughness: Vec<Vec<[f32; 3]>>, // Mipmap seviyeleri
    pub brdf_lut: Vec<[f32; 2]>, // BRDF look-up tablosu
}

impl Default for Scene {
    fn default() -> Self {
        let mut lights = Vec::new();
        // Ana güneş ışığı
        lights.push(SceneLight::directional(
            Vec3::new(0.5, 1.0, 0.3).normalize(),
            Vec3::new(1.0, 0.95, 0.85),
            1.5,
        ));
        // Dolgu ışığı (mavi tonlu)
        lights.push(SceneLight::directional(
            Vec3::new(-0.3, 0.5, -0.7).normalize(),
            Vec3::new(0.3, 0.4, 0.6),
            0.4,
        ));
        Self {
            objects: Vec::new(),
            camera: Camera::new(),
            next_id: 1,
            grid_size: 24.0,
            grid_divisions: 24,
            lights,
            ambient_color: Vec3::new(0.1, 0.12, 0.15),
            ambient_intensity: 0.3,
            environment_map: None,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            physics_enabled: true,
            ground_y: 0.0,
            custom_meshes: std::collections::HashMap::new(),
            next_mesh_id: 100,
            imported_obj_meshes: Vec::new(),
            textures: std::collections::HashMap::new(),
            next_texture_id: 1,
            texture_names: std::collections::HashMap::new(),
            skeletons: std::collections::HashMap::new(),
            next_skeleton_id: 1,
            animation_clips: std::collections::HashMap::new(),
            blend_machines: std::collections::HashMap::new(),
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
            material: PbrMaterial::default(),
            is_light: false,
            light_id: None,
            rigid_body: None,
            collider_radius: 0.5,
            tags: Vec::new(),
            skeleton_id: None,
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

    /// OBJ dosyasından mesh import et
    pub fn import_obj(&mut self, obj_data: &str) -> Result<u32, String> {
        let obj_mesh = parse_obj(obj_data);
        if obj_mesh.vertices.is_empty() {
            return Err("OBJ dosyasında vertex bulunamadı".to_string());
        }
        let mesh = obj_to_mesh(&obj_mesh);
        let mesh_id = self.next_mesh_id;
        self.next_mesh_id += 1;
        self.custom_meshes.insert(mesh_id, mesh);
        self.imported_obj_meshes.push(obj_mesh);
        Ok(mesh_id)
    }

    /// OBJ dosyasını dosya yolundan import et
    pub fn import_obj_from_file(&mut self, path: &str) -> Result<u32, String> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| format!("Dosya okunamadı: {}", e))?;
        self.import_obj(&data)
    }

    /// GLB binary verisinden mesh import et
    pub fn import_glb(&mut self, glb_data: &[u8]) -> Result<u32, String> {
        let glb_mesh = crate::glb_import::parse_glb(glb_data)?;
        if glb_mesh.vertices.is_empty() {
            return Err("GLB dosyasında vertex bulunamadı".into());
        }

        // GLB mesh'ini Scene Mesh formatına dönüştür
        let vertices = glb_mesh.vertices;
        let mut triangles = Vec::new();

        // Indices varsa onları kullan
        if glb_mesh.indices.len() >= 3 {
            for chunk in glb_mesh.indices.chunks(3) {
                if chunk.len() == 3 {
                    triangles.push(crate::renderer::Tri(
                        chunk[0] as usize,
                        chunk[1] as usize,
                        chunk[2] as usize,
                    ));
                }
            }
        } else {
            // Indices yoksa sıralı üçgenler oluştur
            for i in (0..vertices.len()).step_by(3) {
                if i + 2 < vertices.len() {
                    triangles.push(crate::renderer::Tri(i, i + 1, i + 2));
                }
            }
        }

        let mesh = crate::renderer::Mesh { vertices, triangles };
        let mesh_id = self.next_mesh_id;
        self.next_mesh_id += 1;
        self.custom_meshes.insert(mesh_id, mesh);

        // Material'ları kaydet (ilk material'ı kullan)
        let material_info = if let Some(mat) = glb_mesh.materials.first() {
            format!(" (mat: {}, metal:{:.2}, rough:{:.2})",
                mat.name, mat.metallic, mat.roughness)
        } else {
            String::new()
        };

        println!("GLB import: {} vertices, {} triangles{}",
            glb_mesh.vertex_count, glb_mesh.triangle_count, material_info);

        Ok(mesh_id)
    }

    /// GLB dosyasından import et
    pub fn import_glb_from_file(&mut self, path: &str) -> Result<u32, String> {
        let data = std::fs::read(path)
            .map_err(|e| format!("Dosya okunamadı '{}': {}", path, e))?;
        self.import_glb(&data)
    }

    /// GLB mesh'inden sahne nesnesi oluştur
    pub fn add_glb_object(&mut self, name: String, glb_data: &[u8], team: EntityTeam) -> Result<usize, String> {
        let mesh_id = self.import_glb(glb_data)?;
        let id = self.add_custom_object(name, mesh_id, team);
        Ok(id)
    }

    /// GLB dosyasından sahne nesnesi oluştur
    pub fn add_glb_object_from_file(&mut self, name: String, path: &str, team: EntityTeam) -> Result<usize, String> {
        let mesh_id = self.import_glb_from_file(path)?;
        let id = self.add_custom_object(name, mesh_id, team);
        Ok(id)
    }

    // ═══════════════════════════════════════════════════════════ Texture Import

    /// Texture byte dizisinden import et
    pub fn import_texture(&mut self, data: &[u8], name: &str) -> Result<u32, String> {
        let tex = crate::texture::load_texture_from_bytes(data, name)?;
        let tex_id = self.next_texture_id;
        self.next_texture_id += 1;
        self.texture_names.insert(tex_id, name.to_string());
        self.textures.insert(tex_id, tex);
        println!("Texture import: '{}' ({}x{}, mip:{})", name,
            self.textures[&tex_id].width, self.textures[&tex_id].height, self.textures[&tex_id].mip_levels);
        Ok(tex_id)
    }

    /// Dosyadan texture import et
    pub fn import_texture_from_file(&mut self, path: &str) -> Result<u32, String> {
        let tex = crate::texture::load_texture(path)?;
        let name = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("texture")
            .to_string();
        let tex_id = self.next_texture_id;
        self.next_texture_id += 1;
        self.texture_names.insert(tex_id, name.clone());
        self.textures.insert(tex_id, tex);
        Ok(tex_id)
    }

    /// Texture'ı scene objesine ata
    pub fn assign_texture_to_object(
        &mut self, object_id: usize, tex_id: u32, slot: &str,
    ) -> Result<(), String> {
        if let Some(obj) = self.get_object_mut(object_id) {
            match slot {
                "albedo" => obj.material.albedo_texture = Some(tex_id),
                "normal" => obj.material.normal_texture = Some(tex_id),
                "roughness" => obj.material.roughness_texture = Some(tex_id),
                "metallic" => obj.material.metallic_texture = Some(tex_id),
                "ao" => obj.material.ao_texture = Some(tex_id),
                "emissive" => obj.material.emissive_texture = Some(tex_id),
                _ => return Err(format!("Bilinmeyen texture slotı: {}", slot)),
            }
            Ok(())
        } else {
            Err(format!("Nesne bulunamadı: {}", object_id))
        }
    }

    /// Texture listesini göster
    pub fn list_textures(&self) -> Vec<(u32, &str, u32, u32)> {
        self.texture_names.iter()
            .filter_map(|(&id, name)| {
                self.textures.get(&id).map(|t| (id, name.as_str(), t.width, t.height))
            })
            .collect()
    }

    /// Texture'ı sil
    pub fn remove_texture(&mut self, tex_id:u32) -> bool {
        self.textures.remove(&tex_id);
        self.texture_names.remove(&tex_id);
        // Atamaları temizle
        for obj in &mut self.objects {
            if obj.material.albedo_texture == Some(tex_id) { obj.material.albedo_texture = None; }
            if obj.material.normal_texture == Some(tex_id) { obj.material.normal_texture = None; }
            if obj.material.roughness_texture == Some(tex_id) { obj.material.roughness_texture = None; }
            if obj.material.metallic_texture == Some(tex_id) { obj.material.metallic_texture = None; }
            if obj.material.ao_texture == Some(tex_id) { obj.material.ao_texture = None; }
            if obj.material.emissive_texture == Some(tex_id) { obj.material.emissive_texture = None; }
        }
        true
    }

    /// Varsayılan texture'ları oluştur (starter kit)
    pub fn create_default_textures(&mut self) {
        // Checkerboard texture
        let checker = crate::texture::Texture::checkerboard(
            "checker", 64, 8, [200, 200, 200], [60, 60, 60]
        );
        let id = self.next_texture_id;
        self.next_texture_id += 1;
        self.texture_names.insert(id, "checker".into());
        self.textures.insert(id, checker);

        // Grass texture
        let grass = crate::texture::Texture::grass("grass", 64);
        let id2 = self.next_texture_id;
        self.next_texture_id += 1;
        self.texture_names.insert(id2, "grass".into());
        self.textures.insert(id2, grass);

        // Normal map (flat)
        let flat_nrm = crate::texture::Texture::solid_color("flat_normal", 128, 128, 255, 255);
        let id3 = self.next_texture_id;
        self.next_texture_id += 1;
        self.texture_names.insert(id3, "flat_normal".into());
        self.textures.insert(id3, flat_nrm);
    }

    // ═══════════════════════════════════════════════════════════ Skeleton API

    /// Yeni skeleton oluştur ve kaydet
    pub fn create_skeleton(&mut self, skeleton: crate::skeletal::Skeleton) -> usize {
        let id = self.next_skeleton_id;
        self.next_skeleton_id += 1;
        self.skeletons.insert(id, skeleton);
        id
    }

    /// Demo humanoid skeleton oluştur
    pub fn create_demo_skeleton(&mut self) -> usize {
        let skel = crate::skeletal::Skeleton::demo_humanoid();
        self.create_skeleton(skel)
    }

    /// Skeleton'ı scene objesine ata
    pub fn assign_skeleton_to_object(&mut self, object_id: usize, skeleton_id: usize) -> Result<(), String> {
        if let Some(obj) = self.get_object_mut(object_id) {
            obj.skeleton_id = Some(skeleton_id);
            Ok(())
        } else {
            Err(format!("Nesne bulunamadı: {}", object_id))
        }
    }

    /// Skeleton'ı ve timeline'ı güncelle (her kare çağrılır)
    pub fn update_skeletons(&mut self, dt: f32) {
        let skeleton_ids: Vec<usize> = self.skeletons.keys().copied().collect();
        for skel_id in skeleton_ids {
            // Blend machine güncelle
            if let Some(bsm) = self.blend_machines.get_mut(&skel_id) {
                bsm.update(dt);
                if let Some(skeleton) = self.skeletons.get_mut(&skel_id) {
                    bsm.apply_to_skeleton(&self.animation_clips.clone(), skeleton);
                }
            }
        }
    }

    /// Skeleton listesi
    pub fn list_skeletons(&self) -> Vec<(usize, &str, usize)> {
        self.skeletons.iter()
            .map(|(&id, skel)| {
                let root_name = skel.root_bones.first()
                    .and_then(|&rid| skel.bones.get(rid as usize))
                    .map(|b| b.name.as_str())
                    .unwrap_or("?");
                (id, root_name, skel.bones.len())
            })
            .collect()
    }

    /// Nesnenin skeleton bilgisini al
    pub fn get_object_skeleton(&self, object_id: usize) -> Option<&crate::skeletal::Skeleton> {
        self.get_object(object_id)
            .and_then(|o| o.skeleton_id)
            .and_then(|sid| self.skeletons.get(&sid))
    }

    /// Seçili nesne için skinned pozisyonları hesapla
    pub fn compute_skinned_positions(&self, object_id: usize) -> Option<(Vec<Vec3>, Vec<Vec3>)> {
        let obj = self.get_object(object_id)?;
        let skel_id = obj.skeleton_id?;
        let skeleton = self.skeletons.get(&skel_id)?;
        let bone_matrices = skeleton.get_bone_matrices();
        // Demo skinned cube kullan
        let mesh = crate::skeletal::SkinnedMesh::demo_skinned_cube();
        Some(mesh.compute_skinned_positions(&bone_matrices))
    }

    /// Import edilmiş mesh'leri listele
    pub fn list_imported_meshes(&self) -> Vec<(u32, &str, usize, usize)> {
        self.custom_meshes.keys()
            .zip(self.imported_obj_meshes.iter())
            .map(|(&id, obj)| (id, obj.name.as_str(), obj.vertex_count, obj.triangle_count))
            .collect()
    }

    /// Custom mesh ile yeni nesne oluştur
    pub fn add_custom_object(&mut self, name: String, mesh_id: u32, team: EntityTeam) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.objects.push(SceneObject {
            id,
            name,
            geometry: GeometryType::Custom(mesh_id),
            transform: Transform::default(),
            color: [0.8, 0.8, 0.8],
            visible: true,
            team,
            health: 100.0,
            max_health: 100.0,
            damage: 10.0,
            material: PbrMaterial::default(),
            is_light: false,
            light_id: None,
            rigid_body: None,
            collider_radius: 1.0,
            tags: Vec::new(),
            skeleton_id: None,
        });
        id
    }

    /// Fizik adımı: gravity, çarpışma, pozisyon entegrasyonu
    pub fn physics_step(&mut self, dt: f32) {
        if !self.physics_enabled { return; }
        let substeps = 4;
        let sub_dt = dt / substeps as f32;

        for _ in 0..substeps {
            // Gravity + integration
            for obj in &mut self.objects {
                if let Some(ref mut rb) = obj.rigid_body {
                    if rb.body_type == RigidBodyType::Static { continue; }
                    if rb.is_gravity_enabled {
                        rb.velocity += self.gravity * sub_dt;
                    }
                    rb.velocity *= (1.0 - rb.linear_damping * sub_dt).max(0.0);
                    rb.angular_velocity *= (1.0 - rb.angular_damping * sub_dt).max(0.0);
                    obj.transform.position += rb.velocity * sub_dt;
                    obj.transform.rotation += rb.angular_velocity * sub_dt;
                }
            }

            // Collision detection & resolution (sphere-sphere)
            let pairs: Vec<(usize, usize)> = {
                let objs = &self.objects;
                let mut p = Vec::new();
                for i in 0..objs.len() {
                    for j in (i+1)..objs.len() {
                        if objs[i].rigid_body.is_some() && objs[j].rigid_body.is_some() {
                            let d = objs[i].transform.position - objs[j].transform.position;
                            let min_dist = objs[i].collider_radius + objs[j].collider_radius;
                            if d.length_squared() < min_dist * min_dist {
                                p.push((i, j));
                            }
                        }
                    }
                }
                p
            };

            for (i, j) in pairs {
                let (dist_vec, dist, min_dist) = {
                    let d = self.objects[i].transform.position - self.objects[j].transform.position;
                    let dist = d.length();
                    let min_dist = self.objects[i].collider_radius + self.objects[j].collider_radius;
                    (d, dist, min_dist)
                };
                if dist < 0.001 { continue; }
                let normal = dist_vec / dist;
                let penetration = min_dist - dist;
                let correction = normal * penetration * 0.5;

                let (body_a_type, body_b_type) = {
                    (self.objects[i].rigid_body.as_ref().unwrap().body_type,
                     self.objects[j].rigid_body.as_ref().unwrap().body_type)
                };

                if body_a_type != RigidBodyType::Static {
                    self.objects[i].transform.position += correction;
                }
                if body_b_type != RigidBodyType::Static {
                    self.objects[j].transform.position -= correction;
                }

                // Velocity resolution
                if body_a_type != RigidBodyType::Static && body_b_type != RigidBodyType::Static {
                    let rel_vel = self.objects[i].rigid_body.as_ref().unwrap().velocity
                                - self.objects[j].rigid_body.as_ref().unwrap().velocity;
                    let vel_along_normal = rel_vel.dot(normal);
                    if vel_along_normal > 0.0 { continue; }
                    let restitution = {
                        let r_a = self.objects[i].rigid_body.as_ref().unwrap().restitution;
                        let r_b = self.objects[j].rigid_body.as_ref().unwrap().restitution;
                        r_a.min(r_b)
                    };
                    let mass_a = self.objects[i].rigid_body.as_ref().unwrap().mass;
                    let mass_b = self.objects[j].rigid_body.as_ref().unwrap().mass;
                    let inv_mass = if mass_a > 0.0 && mass_b > 0.0 {
                        1.0 / mass_a + 1.0 / mass_b
                    } else if mass_a > 0.0 { 1.0 / mass_a }
                    else if mass_b > 0.0 { 1.0 / mass_b }
                    else { 0.0 };
                    if inv_mass <= 0.0 { continue; }
                    let impulse_magnitude = -(1.0 + restitution) * vel_along_normal / inv_mass;
                    let impulse = normal * impulse_magnitude;
                    if body_a_type != RigidBodyType::Static {
                        self.objects[i].rigid_body.as_mut().unwrap().velocity += impulse * (1.0 / mass_a);
                    }
                    if body_b_type != RigidBodyType::Static {
                        self.objects[j].rigid_body.as_mut().unwrap().velocity -= impulse * (1.0 / mass_b);
                    }
                }
            }

            // Ground collision
            for obj in &mut self.objects {
                if let Some(ref mut rb) = obj.rigid_body {
                    if rb.body_type == RigidBodyType::Static { continue; }
                    let half_height = match obj.geometry {
                        GeometryType::Cube | GeometryType::Plane => 0.5 * obj.transform.scale.y,
                        GeometryType::Sphere => 0.5 * obj.transform.scale.y.max(obj.transform.scale.x),
                        _ => 0.8,
                    };
                    let ground = self.ground_y + half_height;
                    if obj.transform.position.y < ground {
                        obj.transform.position.y = ground;
                        if rb.velocity.y < 0.0 {
                            rb.velocity.y = -rb.velocity.y * rb.restitution;
                            if rb.velocity.y.abs() < 0.1 { rb.velocity.y = 0.0; }
                        }
                    }
                }
            }
        }
    }

    /// Prosedürel terrain mesh üretir (Perlin-like noise)
    pub fn generate_terrain_mesh(size: f32, resolution: u32, height_scale: f32, seed: u32) -> (Vec<Vec3>, Vec<(usize, usize, usize)>) {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        let half = size * 0.5;
        let step = size / resolution as f32;

        // Basit noise fonksiyonu
        let noise2d = |x: f32, z: f32, s: u32| -> f32 {
            let ix = (x * 10.0 + s as f32 * 100.0) as i32;
            let iz = (z * 10.0 + s as f32 * 77.0) as i32;
            let h = (ix.wrapping_mul(374761393) ^ iz.wrapping_mul(668265263)) as f32;
            (h / i32::MAX as f32).abs()
        };

        let sample_height = |x: f32, z: f32| -> f32 {
            let mut h = 0.0f32;
            // Octave 1
            h += noise2d(x, z, seed) * height_scale;
            // Octave 2
            h += noise2d(x * 2.0, z * 2.0, seed + 1) * height_scale * 0.5;
            // Octave 3
            h += noise2d(x * 4.0, z * 4.0, seed + 2) * height_scale * 0.25;
            h / 1.75 // normalize
        };

        for iz in 0..=resolution {
            for ix in 0..=resolution {
                let x = -half + ix as f32 * step;
                let z = -half + iz as f32 * step;
                let y = sample_height(x, z);
                vertices.push(Vec3::new(x, y, z));
            }
        }

        for iz in 0..resolution {
            for ix in 0..resolution {
                let a = (iz * (resolution + 1) + ix) as usize;
                let b = a + 1;
                let c = a + (resolution + 1) as usize;
                let d = c + 1;
                triangles.push((a, c, b));
                triangles.push((b, c, d));
            }
        }

        (vertices, triangles)
    }
}
