//! # GPU Compute Pipeline
//!
//! Parçacık simülasyonu ve GPU frustum culling için compute shader'lar.


// ═══════════════════════════════════════════════════════════ WGSL Shader Kaynakları

/// Parçacık simülasyonu compute shader'ı
pub const PARTICLE_COMPUTE_SHADER: &str = r#"
// Elysium Particle Simulation Compute Shader
// GPU'da parçacık lifecycle, gravity, collision hesaplaması

struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    color: vec3<f32>,
    life: f32,
    max_life: f32,
    size: f32,
    mass: f32,
    flags: u32, // bit 0: alive, bit 1: collision, bit 2: gravity
}

struct SimParams {
    delta_time: f32,
    gravity: f32,
    drag: f32,
    collision_radius: f32,
    ground_y: f32,
    bounce: f32,
    max_particles: u32,
    time: f32,
    wind_x: f32,
    wind_y: f32,
    wind_z: f32,
    turbulence: f32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: SimParams;

// Simple hash function for noise
fn hash(p: u32) -> f32 {
    var x = p;
    x = x ^ (x >> 16u);
    x = x * 0x45d9f3bu;
    x = x ^ (x >> 16u);
    x = x * 0x45d9f3bu;
    x = x ^ (x >> 16u);
    return f32(x) / 4294967295.0;
}

// 3D noise function
fn noise3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash(u32(i.x + i.y * 157.0 + i.z * 113.0));
    let b = hash(u32(i.x + 1.0 + i.y * 157.0 + i.z * 113.0));
    let c = hash(u32(i.x + i.y * 157.0 + i.z + 113.0));
    let d = hash(u32(i.x + 1.0 + i.y * 157.0 + i.z + 113.0));
    let e = hash(u32(i.x + i.y * 157.0 + (i.z + 1.0) * 113.0));
    let ff = hash(u32(i.x + 1.0 + i.y * 157.0 + (i.z + 1.0) * 113.0));
    let g = hash(u32(i.x + (i.y + 1.0) * 157.0 + i.z * 113.0));
    let h = hash(u32(i.x + 1.0 + (i.y + 1.0) * 157.0 + i.z * 113.0));

    return mix(
        mix(mix(a, b, u.x), mix(c, d, u.x), u.y),
        mix(mix(e, ff, u.x), mix(g, h, u.x), u.y),
        u.z
    );
}

@compute @workgroup_size(256)
fn simulate(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= params.max_particles) { return; }

    var p = particles[idx];

    // Ölü parçacıkları atla
    if (p.life <= 0.0) {
        p.flags = 0u;
        particles[idx] = p;
        return;
    }

    let dt = params.delta_time;

    // Gravity
    if ((p.flags & 4u) != 0u) {
        p.velocity.y -= params.gravity * p.mass * dt;
    }

    // Wind + turbulence
    let wind = vec3<f32>(params.wind_x, params.wind_y, params.wind_z);
    let noise_val = noise3d(p.position * 0.1 + vec3<f32>(params.time * 0.5));
    let turbulence = (noise_val - 0.5) * params.turbulence;
    p.velocity += (wind + vec3<f32>(turbulence, turbulence * 0.5, turbulence)) * dt;

    // Drag
    p.velocity *= (1.0 - params.drag * dt);

    // Position update
    p.position += p.velocity * dt;

    // Ground collision
    if (p.position.y < params.ground_y) {
        p.position.y = params.ground_y;
        p.velocity.y = abs(p.velocity.y) * params.bounce;
        p.velocity.x *= 0.8;
        p.velocity.z *= 0.8;
        p.flags = p.flags | 2u; // collision flag
    } else {
        p.flags = p.flags & ~2u;
    }

    // Life decay
    p.life -= dt;

    // Size fade
    p.size = mix(0.0, p.size, p.life / p.max_life);

    particles[idx] = p;
}

@compute @workgroup_size(256)
fn emit(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= params.max_particles) { return; }

    var p = particles[idx];

    // Yeni parçacık üretimi (ölmüş parçacıkları yeniden kullan)
    if (p.life <= 0.0) {
        let seed = u32(idx) + u32(params.time * 1000.0);
        let r1 = hash(seed);
        let r2 = hash(seed + 1u);
        let r3 = hash(seed + 2u);

        p.position = vec3<f32>(
            (r1 - 0.5) * 2.0,
            r2 * 3.0 + 1.0,
            (r3 - 0.5) * 2.0
        );

        let speed = 1.0 + r1 * 3.0;
        let angle = r2 * 6.283185;
        p.velocity = vec3<f32>(
            cos(angle) * speed * 0.5,
            speed * 1.5,
            sin(angle) * speed * 0.5
        );

        p.color = vec3<f32>(0.8 + r1 * 0.2, 0.3 + r2 * 0.4, 0.1 + r3 * 0.2);
        p.life = 1.0 + r1 * 2.0;
        p.max_life = p.life;
        p.size = 0.5 + r2 * 1.5;
        p.mass = 0.5 + r3 * 1.0;
        p.flags = 5u; // alive + gravity
    }

    particles[idx] = p;
}
"#;

/// GPU Frustum Culling compute shader'ı
pub const CULLING_COMPUTE_SHADER: &str = &r#"
// Elysium GPU Frustum Culling
// Meshlet tabanlı visibility buffer üretimi

struct DrawCommand {
    vertex_offset: i32,
    index_offset: i32,
    index_count: u32,
    instance_count: u32,
    meshlet_id: u32,
    padding: u32,
}

struct FrustumPlane {
    normal: vec3<f32>,
    distance: f32,
}

struct CullingParams {
    frustum_planes: array<FrustumPlane, 6>,
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    max_draws: u32,
    active_draws: atomic<u32>,
    total_culled: u32,
    total_visible: u32,
    lod_factor: f32,
    padding: f32,
}

struct MeshletData {
    center: vec3<f32>,
    radius: f32,
    cone_axis: vec3<f32>,
    cone_cutoff: f32,
    vertex_offset: u32,
    triangle_offset: u32,
    vertex_count: u32,
    triangle_count: u32,
}

@group(0) @binding(0) var<storage, read> meshlets: array<MeshletData>;
@group(0) @binding(1) var<storage, read_write> draw_commands: array<DrawCommand>;
@group(0) @binding(2) var<uniform> culling_params: CullingParams;
@group(0) @binding(3) var<storage, read_write> visibility_buffer: array<u32>;
@group(0) @binding(4) var<storage, read_write> counter: atomic<u32>;

// Frustum test
fn is_in_frustum(center: vec3<f32>, radius: f32) -> bool {
    for (var i = 0u; i < 6u; i = i + 1u) {
        let plane = culling_params.frustum_planes[i];
        let dist = dot(plane.normal, center) + plane.distance;
        if (dist < -radius) {
            return false;
        }
    }
    return true;
}

// Backface culling (normal cone test)
fn is_front_facing(cone_axis: vec3<f32>, cone_cutoff: f32) -> bool {
    let view_dir = normalize(culling_params.camera_pos - cone_axis);
    return dot(cone_axis, view_dir) < cone_cutoff;
}

// LOD selection based on screen coverage
fn select_lod(center: vec3<f32>, radius: f32) -> u32 {
    let dist = length(center - culling_params.camera_pos);
    let screen_coverage = radius / max(dist * culling_params.lod_factor, 0.001);

    if (screen_coverage > 0.1) { return 0u; }
    if (screen_coverage > 0.03) { return 1u; }
    if (screen_coverage > 0.01) { return 2u; }
    return 3u;
}

@compute @workgroup_size(256)
fn cull_meshlets(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= culling_params.max_draws) { return; }

    let meshlet = meshlets[idx];

    // Frustum culling
    if (!is_in_frustum(meshlet.center, meshlet.radius)) {
        visibility_buffer[idx] = 0u;
        return;
    }

    // Backface culling
    if (!is_front_facing(meshlet.cone_axis, meshlet.cone_cutoff)) {
        visibility_buffer[idx] = 0u;
        return;
    }

    // LOD selection
    let lod = select_lod(meshlet.center, meshlet.radius);

    // Draw command oluştur
    let draw_idx = atomicAdd(&counter, 1u);
    if (draw_idx < culling_params.max_draws) {
        draw_commands[draw_idx] = DrawCommand(
            i32(meshlet.vertex_offset),
            i32(meshlet.triangle_offset),
            meshlet.triangle_count * 3u,
            1u,
            idx,
            0u
        );
        visibility_buffer[idx] = 1u;
    }
}
"#;

/// Parçacık veri yapısı (CPU tarafı)
#[derive(Clone, Debug)]
pub struct GpuParticle {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub color: [f32; 3],
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub mass: f32,
    pub flags: u32,
}

impl GpuParticle {
    pub fn new() -> Self {
        Self {
            position: [0.0; 3],
            velocity: [0.0; 3],
            color: [1.0; 3],
            life: 0.0,
            max_life: 1.0,
            size: 1.0,
            mass: 1.0,
            flags: 0,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.life > 0.0
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for v in &self.position { bytes.extend_from_slice(&v.to_ne_bytes()); }
        for v in &self.velocity { bytes.extend_from_slice(&v.to_ne_bytes()); }
        for v in &self.color { bytes.extend_from_slice(&v.to_ne_bytes()); }
        bytes.extend_from_slice(&self.life.to_ne_bytes());
        bytes.extend_from_slice(&self.max_life.to_ne_bytes());
        bytes.extend_from_slice(&self.size.to_ne_bytes());
        bytes.extend_from_slice(&self.mass.to_ne_bytes());
        bytes.extend_from_slice(&self.flags.to_ne_bytes());
        bytes
    }
}

/// Simülasyon parametreleri
#[derive(Clone, Debug)]
pub struct SimParams {
    pub delta_time: f32,
    pub gravity: f32,
    pub drag: f32,
    pub collision_radius: f32,
    pub ground_y: f32,
    pub bounce: f32,
    pub max_particles: u32,
    pub time: f32,
    pub wind: [f32; 3],
    pub turbulence: f32,
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            delta_time: 0.016,
            gravity: 9.81,
            drag: 0.1,
            collision_radius: 0.1,
            ground_y: 0.0,
            bounce: 0.4,
            max_particles: 65536,
            time: 0.0,
            wind: [0.0; 3],
            turbulence: 0.5,
        }
    }
}

impl SimParams {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.delta_time.to_ne_bytes());
        bytes.extend_from_slice(&self.gravity.to_ne_bytes());
        bytes.extend_from_slice(&self.drag.to_ne_bytes());
        bytes.extend_from_slice(&self.collision_radius.to_ne_bytes());
        bytes.extend_from_slice(&self.ground_y.to_ne_bytes());
        bytes.extend_from_slice(&self.bounce.to_ne_bytes());
        bytes.extend_from_slice(&self.max_particles.to_ne_bytes());
        bytes.extend_from_slice(&self.time.to_ne_bytes());
        bytes.extend_from_slice(&self.wind[0].to_ne_bytes());
        bytes.extend_from_slice(&self.wind[1].to_ne_bytes());
        bytes.extend_from_slice(&self.wind[2].to_ne_bytes());
        bytes.extend_from_slice(&self.turbulence.to_ne_bytes());
        bytes
    }
}

/// Draw command (GPU → CPU)
#[derive(Clone, Debug)]
pub struct DrawCommand {
    pub vertex_offset: i32,
    pub index_offset: i32,
    pub index_count: u32,
    pub instance_count: u32,
    pub meshlet_id: u32,
}

impl Default for MeshletData {
    fn default() -> Self {
        Self {
            center: [0.0; 3],
            radius: 1.0,
            cone_axis: [0.0, 0.0, -1.0],
            cone_cutoff: 0.0,
            vertex_offset: 0,
            triangle_offset: 0,
            vertex_count: 0,
            triangle_count: 0,
        }
    }
}

/// Meshlet verisi (CPU tarafı)
#[derive(Clone, Debug)]
pub struct MeshletData {
    pub center: [f32; 3],
    pub radius: f32,
    pub cone_axis: [f32; 3],
    pub cone_cutoff: f32,
    pub vertex_offset: u32,
    pub triangle_offset: u32,
    pub vertex_count: u32,
    pub triangle_count: u32,
}

/// Culling parametreleri
#[derive(Clone, Debug)]
pub struct CullingParams {
    pub frustum_planes: [[f32; 4]; 6],
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub max_draws: u32,
    pub lod_factor: f32,
}

impl Default for CullingParams {
    fn default() -> Self {
        Self {
            frustum_planes: [[0.0; 4]; 6],
            view_proj: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
            camera_pos: [0.0, 0.0, 5.0],
            max_draws: 65536,
            lod_factor: 1.0,
        }
    }
}

// ═══════════════════════════════════════════════════════════ CPU Fallback Simülasyonu

/// CPU fallback parçacık simülasyonu (GPU olmadığında kullanılır)
pub struct CpuParticleSimulator {
    pub particles: Vec<GpuParticle>,
    pub params: SimParams,
    pub active_count: usize,
}

impl Default for CpuParticleSimulator {
    fn default() -> Self { Self::new() }
}

impl CpuParticleSimulator {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            params: SimParams::default(),
            active_count: 0,
        }
    }

    pub fn with_capacity(max: usize) -> Self {
        Self {
            particles: vec![GpuParticle::new(); max],
            params: SimParams::default(),
            active_count: 0,
        }
    }

    pub fn emit_burst(&mut self, position: [f32; 3], count: usize, color: [f32; 3]) {
        for i in 0..count {
            let slot = self.find_dead_slot();
            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let speed = 1.0 + rand::random::<f32>() * 3.0;
            let up_speed = 1.0 + rand::random::<f32>() * 2.0;

            self.particles[slot] = GpuParticle {
                position,
                velocity: [angle.cos() * speed * 0.5, up_speed, angle.sin() * speed * 0.5],
                color,
                life: 0.5 + rand::random::<f32>() * 1.5,
                max_life: 2.0,
                size: 0.5 + rand::random::<f32>() * 1.5,
                mass: 0.5 + rand::random::<f32>() * 1.0,
                flags: 5, // alive + gravity
            };
        }
    }

    fn find_dead_slot(&mut self) -> usize {
        for (i, p) in self.particles.iter().enumerate() {
            if p.life <= 0.0 { return i; }
        }
        self.particles.push(GpuParticle::new());
        self.particles.len() - 1
    }

    pub fn simulate(&mut self, dt: f32) {
        self.params.delta_time = dt;
        self.params.time += dt;
        self.active_count = 0;

        for p in &mut self.particles {
            if p.life <= 0.0 { continue; }
            self.active_count += 1;

            // Gravity
            if p.flags & 4 != 0 {
                p.velocity[1] -= self.params.gravity * p.mass * dt;
            }

            // Wind
            p.velocity[0] += self.params.wind[0] * dt;
            p.velocity[1] += self.params.wind[1] * dt;
            p.velocity[2] += self.params.wind[2] * dt;

            // Drag
            let drag_factor = 1.0 - self.params.drag * dt;
            p.velocity[0] *= drag_factor;
            p.velocity[1] *= drag_factor;
            p.velocity[2] *= drag_factor;

            // Position
            p.position[0] += p.velocity[0] * dt;
            p.position[1] += p.velocity[1] * dt;
            p.position[2] += p.velocity[2] * dt;

            // Ground collision
            if p.position[1] < self.params.ground_y {
                p.position[1] = self.params.ground_y;
                p.velocity[1] = p.velocity[1].abs() * self.params.bounce;
                p.velocity[0] *= 0.8;
                p.velocity[2] *= 0.8;
            }

            // Life decay
            p.life -= dt;

            // Size fade
            if p.max_life > 0.0 {
                p.size *= (p.life / p.max_life).max(0.0);
            }
        }
    }

    pub fn alive_count(&self) -> usize {
        self.particles.iter().filter(|p| p.life > 0.0).count()
    }
}

// ═══════════════════════════════════════════════════════════ GPU Culling (CPU fallback)

/// CPU frustum culling (GPU olmadığında)
pub struct CpuFrustumCuller {
    pub view_frustum: Vec<[f32; 4]>,
    pub visible_meshlets: Vec<u32>,
    pub culled_count: u32,
    pub visible_count: u32,
}

impl Default for CpuFrustumCuller {
    fn default() -> Self { Self::new() }
}

impl CpuFrustumCuller {
    pub fn new() -> Self {
        Self { view_frustum: Vec::new(), visible_meshlets: Vec::new(), culled_count: 0, visible_count: 0 }
    }

    pub fn extract_frustum_planes(&mut self, view_proj: &[[f32; 4]; 4]) {
        self.view_frustum.clear();

        // Left
        self.view_frustum.push([
            view_proj[0][3] + view_proj[0][0],
            view_proj[1][3] + view_proj[1][0],
            view_proj[2][3] + view_proj[2][0],
            view_proj[3][3] + view_proj[3][0],
        ]);
        // Right
        self.view_frustum.push([
            view_proj[0][3] - view_proj[0][0],
            view_proj[1][3] - view_proj[1][0],
            view_proj[2][3] - view_proj[2][0],
            view_proj[3][3] - view_proj[3][0],
        ]);
        // Bottom
        self.view_frustum.push([
            view_proj[0][3] + view_proj[0][1],
            view_proj[1][3] + view_proj[1][1],
            view_proj[2][3] + view_proj[2][1],
            view_proj[3][3] + view_proj[3][1],
        ]);
        // Top
        self.view_frustum.push([
            view_proj[0][3] - view_proj[0][1],
            view_proj[1][3] - view_proj[1][1],
            view_proj[2][3] - view_proj[2][1],
            view_proj[3][3] - view_proj[3][1],
        ]);
        // Near
        self.view_frustum.push([
            view_proj[0][3] + view_proj[0][2],
            view_proj[1][3] + view_proj[1][2],
            view_proj[2][3] + view_proj[2][2],
            view_proj[3][3] + view_proj[3][2],
        ]);
        // Far
        self.view_frustum.push([
            view_proj[0][3] - view_proj[0][2],
            view_proj[1][3] - view_proj[1][2],
            view_proj[2][3] - view_proj[2][2],
            view_proj[3][3] - view_proj[3][2],
        ]);

        // Normalize
        for plane in &mut self.view_frustum {
            let len = (plane[0] * plane[0] + plane[1] * plane[1] + plane[2] * plane[2]).sqrt();
            if len > 0.0 {
                plane[0] /= len;
                plane[1] /= len;
                plane[2] /= len;
                plane[3] /= len;
            }
        }
    }

    pub fn cull_meshlets(&mut self, meshlets: &[MeshletData]) {
        self.visible_meshlets.clear();
        self.culled_count = 0;
        self.visible_count = 0;

        for (i, meshlet) in meshlets.iter().enumerate() {
            if self.is_visible(meshlet) {
                self.visible_meshlets.push(i as u32);
                self.visible_count += 1;
            } else {
                self.culled_count += 1;
            }
        }
    }

    fn is_visible(&self, meshlet: &MeshletData) -> bool {
        let center = meshlet.center;
        let radius = meshlet.radius;

        for plane in &self.view_frustum {
            let dist = plane[0] * center[0] + plane[1] * center[1] + plane[2] * center[2] + plane[3];
            if dist < -radius {
                return false;
            }
        }
        true
    }

    pub fn stats(&self) -> (u32, u32, f32) {
        let total = self.visible_count + self.culled_count;
        let cull_rate = if total > 0 { self.culled_count as f32 / total as f32 } else { 0.0 };
        (self.visible_count, self.culled_count, cull_rate)
    }
}

// ═══════════════════════════════════════════════════════════ Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_particle() {
        let mut p = GpuParticle::new();
        assert!(!p.is_alive());
        p.life = 1.0;
        assert!(p.is_alive());
        let bytes = p.to_bytes();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_sim_params() {
        let params = SimParams::default();
        let bytes = params.to_bytes();
        assert_eq!(bytes.len(), 12 * 4); // 12 f32 values
    }

    #[test]
    fn test_cpu_particle_simulator() {
        let mut sim = CpuParticleSimulator::new();
        sim.emit_burst([0.0, 1.0, 0.0], 10, [1.0, 0.5, 0.0]);
        assert!(sim.alive_count() > 0);

        sim.simulate(0.016);
        assert!(sim.alive_count() > 0);
    }

    #[test]
    fn test_cpu_frustum_culling() {
        let mut culler = CpuFrustumCuller::new();
        let identity = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
        culler.extract_frustum_planes(&identity);

        let meshlets = vec![
            MeshletData { center: [0.0, 0.0, 0.0], radius: 1.0, ..Default::default() },
            MeshletData { center: [100.0, 100.0, 100.0], radius: 1.0, ..Default::default() },
        ];

        culler.cull_meshlets(&meshlets);
        let (visible, _culled, _) = culler.stats();
        assert!(visible > 0);
    }
}
