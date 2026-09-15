
// ═══════════════════════════════════════════════════════════ Minimal Math

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3(pub f32, pub f32, pub f32);

impl Vec3 {
    pub const ZERO: Self = Self(0.0, 0.0, 0.0);
    pub const ONE: Self = Self(1.0, 1.0, 1.0);
    pub const Y: Self = Self(0.0, 1.0, 0.0);

    pub fn new(x: f32, y: f32, z: f32) -> Self { Self(x, y, z) }
    pub fn splat(v: f32) -> Self { Self(v, v, v) }
    pub fn x(&self) -> f32 { self.0 }
    pub fn y(&self) -> f32 { self.1 }
    pub fn z(&self) -> f32 { self.2 }

    pub fn dot(self, o: Self) -> f32 { self.0*o.0 + self.1*o.1 + self.2*o.2 }
    pub fn cross(self, o: Self) -> Self {
        Self(self.1*o.2 - self.2*o.1, self.2*o.0 - self.0*o.2, self.0*o.1 - self.1*o.0)
    }
    pub fn length(self) -> f32 { self.dot(self).sqrt() }
    pub fn length_squared(self) -> f32 { self.dot(self) }
    pub fn normalize(self) -> Self {
        let l = self.length();
        if l > 1e-8 { self * (1.0 / l) } else { Self::ZERO }
    }
    pub fn normalize_or_zero(self) -> Self { self.normalize() }
    pub fn lerp(self, o: Self, t: f32) -> Self {
        Self(self.0 + (o.0 - self.0) * t, self.1 + (o.1 - self.1) * t, self.2 + (o.2 - self.2) * t)
    }
    pub fn min(self, o: Self) -> Self { Self(self.0.min(o.0), self.1.min(o.1), self.2.min(o.2)) }
    pub fn max(self, o: Self) -> Self { Self(self.0.max(o.0), self.1.max(o.1), self.2.max(o.2)) }
    pub fn to_array(self) -> [f32; 3] { [self.0, self.1, self.2] }
}

impl std::ops::Add for Vec3 { type Output = Self; fn add(self, o: Self) -> Self { Self(self.0+o.0, self.1+o.1, self.2+o.2) } }
impl std::ops::Sub for Vec3 { type Output = Self; fn sub(self, o: Self) -> Self { Self(self.0-o.0, self.1-o.1, self.2-o.2) } }
impl std::ops::Mul<f32> for Vec3 { type Output = Self; fn mul(self, s: f32) -> Self { Self(self.0*s, self.1*s, self.2*s) } }
impl std::ops::Mul<Vec3> for f32 { type Output = Vec3; fn mul(self, v: Vec3) -> Vec3 { Vec3(self*v.0, self*v.1, self*v.2) } }
impl std::ops::Mul<Vec3> for Vec3 { type Output = Self; fn mul(self, o: Self) -> Self { Self(self.0*o.0, self.1*o.1, self.2*o.2) } }
impl std::ops::Div<f32> for Vec3 { type Output = Self; fn div(self, s: f32) -> Self { Self(self.0/s, self.1/s, self.2/s) } }
impl std::ops::Neg for Vec3 { type Output = Self; fn neg(self) -> Self { Self(-self.0, -self.1, -self.2) } }
impl std::ops::AddAssign for Vec3 { fn add_assign(&mut self, o: Self) { self.0+=o.0; self.1+=o.1; self.2+=o.2; } }
impl std::ops::SubAssign for Vec3 { fn sub_assign(&mut self, o: Self) { self.0-=o.0; self.1-=o.1; self.2-=o.2; } }

/// 4x4 Matris (column-major, wgpu/glam compatible layout)
#[derive(Clone, Copy, Debug, Default)]
pub struct Mat4(pub [f32; 16]);

impl Mat4 {
    pub const IDENTITY: Self = Self([
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]);

    /// Column-major: m[col * 4 + row]
    #[inline]
    pub fn col_row(col: usize, row: usize) -> usize { col * 4 + row }

    pub fn from_translation(t: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.0[12] = t.0; m.0[13] = t.1; m.0[14] = t.2;
        m
    }

    pub fn from_scale(s: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.0[0] = s.0; m.0[5] = s.1; m.0[10] = s.2;
        m
    }

    pub fn from_rotation_x(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        let mut m = Self::IDENTITY;
        m.0[5] = c;  m.0[6] = s;
        m.0[9] = -s; m.0[10] = c;
        m
    }

    pub fn from_rotation_y(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        let mut m = Self::IDENTITY;
        m.0[0] = c;  m.0[2] = -s;
        m.0[8] = s;  m.0[10] = c;
        m
    }

    pub fn from_rotation_z(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        let mut m = Self::IDENTITY;
        m.0[0] = c;  m.0[1] = s;
        m.0[4] = -s; m.0[5] = c;
        m
    }

    pub fn perspective(fov_y_rad: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y_rad / 2.0).tan();
        let nf = 1.0 / (near - far);
        let mut m = [0.0f32; 16];
        m[0] = f / aspect;
        m[5] = f;
        m[10] = (far + near) * nf;
        m[11] = -1.0;
        m[14] = 2.0 * far * near * nf;
        Self(m)
    }

    pub fn look_at_rh(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = (target - eye).normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(f);
        let mut m = Self::IDENTITY;
        m.0[0] = s.0; m.0[4] = s.1; m.0[8]  = s.2; m.0[12] = -s.dot(eye);
        m.0[1] = u.0; m.0[5] = u.1; m.0[9]  = u.2; m.0[13] = -u.dot(eye);
        m.0[2] = -f.0; m.0[6] = -f.1; m.0[10] = -f.2; m.0[14] = f.dot(eye);
        m
    }

    pub fn mul_mat4(self, o: Self) -> Self {
        let mut r = [0.0f32; 16];
        for c in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.0[k * 4 + row] * o.0[c * 4 + k];
                }
                r[c * 4 + row] = sum;
            }
        }
        Self(r)
    }

    pub fn mul_vec4(self, v: [f32; 4]) -> [f32; 4] {
        [
            self.0[0]*v[0] + self.0[4]*v[1] + self.0[8]*v[2]  + self.0[12]*v[3],
            self.0[1]*v[0] + self.0[5]*v[1] + self.0[9]*v[2]  + self.0[13]*v[3],
            self.0[2]*v[0] + self.0[6]*v[1] + self.0[10]*v[2] + self.0[14]*v[3],
            self.0[3]*v[0] + self.0[7]*v[1] + self.0[11]*v[2] + self.0[15]*v[3],
        ]
    }

    pub fn transform_point3(self, p: Vec3) -> Vec3 {
        let r = self.mul_vec4([p.0, p.1, p.2, 1.0]);
        Vec3(r[0] / r[3], r[1] / r[3], r[2] / r[3])
    }

    pub fn to_cols_array(self) -> [f32; 16] { self.0 }
}

// ═══════════════════════════════════════════════════════════ Geometry

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GeometryType { Cube, Sphere, Cylinder, Capsule }

impl GeometryType {
    pub fn display_name(self) -> &'static str {
        match self { Self::Cube => "Cube", Self::Sphere => "Sphere", Self::Cylinder => "Cylinder", Self::Capsule => "Capsule" }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3, // Euler degrees
    pub scale: Vec3,
}

impl Transform {
    pub fn to_matrix(self) -> Mat4 {
        let t = Mat4::from_translation(self.position);
        let rx = Mat4::from_rotation_x(self.rotation.0.to_radians());
        let ry = Mat4::from_rotation_y(self.rotation.1.to_radians());
        let rz = Mat4::from_rotation_z(self.rotation.2.to_radians());
        let s = Mat4::from_scale(self.scale);
        t.mul_mat4(ry).mul_mat4(rx).mul_mat4(rz).mul_mat4(s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EntityTeam { Neutral, Player, Enemy }

#[derive(Clone, Copy, Debug)]
pub struct PbrMaterial {
    pub albedo: Vec3,
    pub metallic: f32,
    pub roughness: f32,
    pub ao: f32,
    pub emissive: Vec3,
    pub emissive_strength: f32,
}

impl Default for PbrMaterial {
    fn default() -> Self { Self { albedo: Vec3::ONE, metallic: 0.0, roughness: 0.5, ao: 1.0, emissive: Vec3::ZERO, emissive_strength: 0.0 } }
}

#[derive(Clone, Debug)]
pub struct SceneObject {
    pub id: usize,
    pub name: String,
    pub geometry: GeometryType,
    pub transform: Transform,
    pub visible: bool,
    pub team: EntityTeam,
    pub health: f32,
    pub max_health: f32,
    pub material: PbrMaterial,
    pub is_light: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum LightType { Directional, Point { radius: f32 } }

#[derive(Clone, Copy, Debug)]
pub struct SceneLight {
    pub position: Vec3,
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub light_type: LightType,
}

impl SceneLight {
    pub fn directional(dir: Vec3, color: Vec3, intensity: f32) -> Self {
        Self { position: Vec3::ZERO, direction: dir.normalize(), color, intensity, light_type: LightType::Directional }
    }
    pub fn point(pos: Vec3, color: Vec3, intensity: f32, radius: f32) -> Self {
        Self { position: pos, direction: Vec3::ZERO, color, intensity, light_type: LightType::Point { radius } }
    }
}

// ═══════════════════════════════════════════════════════════ Camera

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self { Self::new() }
}

impl Camera {
    pub fn new() -> Self {
        Self { target: Vec3::ZERO, yaw: -45.0, pitch: 30.0, distance: 18.0, fov: 60.0, near: 0.1, far: 1000.0 }
    }
    pub fn position(&self) -> Vec3 {
        let yr = self.yaw.to_radians();
        let pr = self.pitch.to_radians();
        self.target + Vec3::new(self.distance * yr.cos() * pr.cos(), self.distance * pr.sin(), self.distance * yr.sin() * pr.cos())
    }
    pub fn view_matrix(&self) -> Mat4 { Mat4::look_at_rh(self.position(), self.target, Vec3::Y) }
    pub fn projection_matrix(&self, aspect: f32) -> Mat4 { Mat4::perspective(self.fov.to_radians(), aspect, self.near, self.far) }
    pub fn orbit(&mut self, dy: f32, dp: f32) { self.yaw += dy; self.pitch = (self.pitch + dp).clamp(-89.0, 89.0); }
    pub fn zoom(&mut self, d: f32) { self.distance = (self.distance - d).clamp(2.0, 200.0); }
}

// ═══════════════════════════════════════════════════════════ Mesh Data

#[derive(Clone, Copy, Debug)]
pub struct Tri(pub usize, pub usize, pub usize);

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<Tri>,
}

pub fn cube_mesh() -> Mesh {
    let v = vec![
        Vec3::new(-0.5,-0.5, 0.5), Vec3::new( 0.5,-0.5, 0.5), Vec3::new( 0.5, 0.5, 0.5), Vec3::new(-0.5, 0.5, 0.5),
        Vec3::new( 0.5,-0.5,-0.5), Vec3::new(-0.5,-0.5,-0.5), Vec3::new(-0.5, 0.5,-0.5), Vec3::new( 0.5, 0.5,-0.5),
        Vec3::new(-0.5,-0.5,-0.5), Vec3::new(-0.5,-0.5, 0.5), Vec3::new(-0.5, 0.5, 0.5), Vec3::new(-0.5, 0.5,-0.5),
        Vec3::new( 0.5,-0.5, 0.5), Vec3::new( 0.5,-0.5,-0.5), Vec3::new( 0.5, 0.5,-0.5), Vec3::new( 0.5, 0.5, 0.5),
        Vec3::new(-0.5, 0.5, 0.5), Vec3::new( 0.5, 0.5, 0.5), Vec3::new( 0.5, 0.5,-0.5), Vec3::new(-0.5, 0.5,-0.5),
        Vec3::new(-0.5,-0.5,-0.5), Vec3::new( 0.5,-0.5,-0.5), Vec3::new( 0.5,-0.5, 0.5), Vec3::new(-0.5,-0.5, 0.5),
    ];
    let mut tris = Vec::new();
    for face in 0..6u32 {
        let b = face as usize * 4;
        tris.push(Tri(b, b+1, b+2)); tris.push(Tri(b, b+2, b+3));
    }
    Mesh { vertices: v, triangles: tris }
}

pub fn sphere_mesh(stacks: u32, slices: u32) -> Mesh {
    let mut verts = Vec::new();
    for i in 0..=stacks {
        let phi = std::f32::consts::PI * i as f32 / stacks as f32;
        for j in 0..=slices {
            let theta = 2.0 * std::f32::consts::PI * j as f32 / slices as f32;
            verts.push(Vec3::new(phi.sin() * theta.cos() * 0.5, phi.cos() * 0.5, phi.sin() * theta.sin() * 0.5));
        }
    }
    let w = slices + 1;
    let mut tris = Vec::new();
    for i in 0..stacks {
        for j in 0..slices {
            let a = (i * w + j) as usize;
            let b = a + 1;
            let c = a + w as usize;
            let d = c + 1;
            tris.push(Tri(a, c, b)); tris.push(Tri(b, c, d));
        }
    }
    Mesh { vertices: verts, triangles: tris }
}

pub fn cylinder_mesh(slices: u32) -> Mesh {
    let mut verts = vec![Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.0, -0.5, 0.0)];
    for i in 0..slices {
        let th = 2.0 * std::f32::consts::PI * i as f32 / slices as f32;
        verts.push(Vec3::new(th.cos() * 0.5, 0.5, th.sin() * 0.5));
        verts.push(Vec3::new(th.cos() * 0.5, -0.5, th.sin() * 0.5));
    }
    let mut tris = Vec::new();
    for i in 0..slices as usize {
        let next = (i + 1) % slices as usize;
        let (t0, b0) = (2 + i * 2, 3 + i * 2);
        let (t1, b1) = (2 + next * 2, 3 + next * 2);
        tris.push(Tri(0, t0, t1)); tris.push(Tri(1, b1, b0));
        tris.push(Tri(t0, b0, t1)); tris.push(Tri(t1, b0, b1));
    }
    Mesh { vertices: verts, triangles: tris }
}

pub fn capsule_mesh(slices: u32, hh: f32) -> Mesh {
    let mut verts = vec![Vec3::new(0.0, hh + 0.5, 0.0), Vec3::new(0.0, hh, 0.0)];
    let base = 2usize;
    for i in 0..slices {
        let th = 2.0 * std::f32::consts::PI * i as f32 / slices as f32;
        verts.push(Vec3::new(th.cos() * 0.5, hh, th.sin() * 0.5));
    }
    let mut tris = Vec::new();
    for i in 0..slices as usize {
        let next = (i + 1) % slices as usize;
        tris.push(Tri(0, base + i, base + next));
    }
    verts.push(Vec3::new(0.0, -hh, 0.0));
    let mid = verts.len() - 1;
    for i in 0..slices as usize {
        let next = (i + 1) % slices as usize;
        tris.push(Tri(mid, base + next, base + i));
    }
    verts.push(Vec3::new(0.0, -(hh + 0.5), 0.0));
    let bot = verts.len() - 1;
    let bot_start = verts.len() - slices as usize - 1;
    for i in 0..slices as usize {
        let next = (i + 1) % slices as usize;
        tris.push(Tri(bot, bot_start + next, bot_start + i));
    }
    Mesh { vertices: verts, triangles: tris }
}

pub fn build_mesh(g: GeometryType) -> Mesh {
    match g {
        GeometryType::Cube => cube_mesh(),
        GeometryType::Sphere => sphere_mesh(16, 24),
        GeometryType::Cylinder => cylinder_mesh(20),
        GeometryType::Capsule => capsule_mesh(20, 0.8),
    }
}

// ═══════════════════════════════════════════════════════════ Scene

pub struct Scene {
    pub objects: Vec<SceneObject>,
    pub lights: Vec<SceneLight>,
    pub camera: Camera,
    pub next_id: usize,
    pub ambient_color: Vec3,
    pub ambient_intensity: f32,
}

impl Default for Scene {
    fn default() -> Self { Self::new() }
}

impl Scene {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(), lights: Vec::new(), camera: Camera::new(),
            next_id: 0, ambient_color: Vec3::new(0.15, 0.15, 0.2), ambient_intensity: 0.5,
        }
    }

    pub fn add_object(&mut self, name: &str, geom: GeometryType, team: EntityTeam) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.objects.push(SceneObject {
            id, name: name.to_string(), geometry: geom, transform: Transform::default(),
            visible: true, team, health: 100.0, max_health: 100.0, material: PbrMaterial::default(),
            is_light: false,
        });
        id
    }

    pub fn add_object_at(&mut self, name: &str, geom: GeometryType, team: EntityTeam, pos: Vec3) -> usize {
        let id = self.add_object(name, geom, team);
        if let Some(obj) = self.objects.iter_mut().find(|o| o.id == id) { obj.transform.position = pos; }
        id
    }

    pub fn get_object(&self, id: usize) -> Option<&SceneObject> { self.objects.iter().find(|o| o.id == id) }
    pub fn get_object_mut(&mut self, id: usize) -> Option<&mut SceneObject> { self.objects.iter_mut().find(|o| o.id == id) }

    pub fn ray_intersect(&self, origin: Vec3, dir: Vec3) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for obj in &self.objects {
            if !obj.visible || obj.is_light { continue; }
            let center = obj.transform.position;
            let to_center = center - origin;
            let t_along = to_center.dot(dir);
            if t_along < 0.0 { continue; }
            let proj = origin + dir * t_along;
            let r = obj.transform.scale.0.max(obj.transform.scale.1).max(obj.transform.scale.2) * 0.6;
            let dist = (proj - center).length();
            if dist < r {
                if best.map_or(true, |(bt, _)| t_along < bt) {
                    best = Some((t_along, obj.id));
                }
            }
        }
        best.map(|(_, id)| id)
    }

    pub fn create_demo_scene() -> Self {
        let mut scene = Scene::new();
        use std::f32::consts::PI;

        // Player
        let p = scene.add_object("Player", GeometryType::Capsule, EntityTeam::Player);
        if let Some(o) = scene.get_object_mut(p) { o.transform.position = Vec3::new(0.0, 1.5, 0.0); o.material.metallic = 0.3; o.material.roughness = 0.4; }

        // Enemies
        for i in 0..5 {
            let e = scene.add_object(&format!("Enemy {}", i+1), GeometryType::Cube, EntityTeam::Enemy);
            let a = i as f32 * (2.0 * PI / 5.0);
            let r = 7.0 + i as f32 * 1.5;
            if let Some(o) = scene.get_object_mut(e) {
                o.transform.position = Vec3::new(a.cos() * r, 1.0, a.sin() * r);
                o.material.metallic = 0.1; o.material.roughness = 0.8;
                o.health = 50.0; o.max_health = 50.0;
            }
        }

        // Pillars
        for i in 0..3 {
            let c = scene.add_object(&format!("Pillar {}", i+1), GeometryType::Cylinder, EntityTeam::Neutral);
            let a = i as f32 * (2.0 * PI / 3.0);
            if let Some(o) = scene.get_object_mut(c) {
                o.transform.position = Vec3::new(a.cos() * 10.0, 1.5, a.sin() * 10.0);
                o.transform.scale = Vec3::new(0.8, 3.0, 0.8);
                o.material.metallic = 0.9; o.material.roughness = 0.2;
                o.material.emissive = Vec3::new(0.1, 0.2, 0.8); o.material.emissive_strength = 2.0;
            }
        }

        // Orb
        let orb = scene.add_object("Orb", GeometryType::Sphere, EntityTeam::Neutral);
        if let Some(o) = scene.get_object_mut(orb) {
            o.transform.position = Vec3::new(3.0, 2.5, 3.0);
            o.transform.scale = Vec3::splat(1.5);
            o.material.metallic = 1.0; o.material.roughness = 0.05;
            o.material.albedo = Vec3::new(1.0, 0.85, 0.2);
        }

        // Ground
        let g = scene.add_object("Ground", GeometryType::Cube, EntityTeam::Neutral);
        if let Some(o) = scene.get_object_mut(g) {
            o.transform.scale = Vec3::new(20.0, 0.1, 20.0);
            o.material.albedo = Vec3::new(0.3, 0.35, 0.25);
            o.material.roughness = 0.9;
        }

        // Lights
        scene.lights.push(SceneLight::directional(Vec3::new(0.5, 0.8, 0.3).normalize(), Vec3::new(1.0, 0.95, 0.8), 2.0));
        scene.lights.push(SceneLight::directional(Vec3::new(-0.3, 0.5, -0.5).normalize(), Vec3::new(0.3, 0.4, 0.6), 0.8));
        scene.lights.push(SceneLight::point(Vec3::new(3.0, 3.0, 3.0), Vec3::new(1.0, 0.8, 0.4), 3.0, 15.0));
        scene.lights.push(SceneLight::point(Vec3::new(-5.0, 2.0, 5.0), Vec3::new(0.2, 0.5, 1.0), 2.0, 12.0));

        scene
    }
}
