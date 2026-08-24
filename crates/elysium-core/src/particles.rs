// No unused imports

/// Particle spawn parameters for one shot or continuous emission.
#[derive(Clone, Debug, Default)]
pub struct ParticleSpawner {
    pub rate: f32,
    pub burst: u32,
    pub lifetime_min: f32,
    pub lifetime_max: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub size_min: [f32; 2],
    pub size_max: [f32; 2],
    pub color: [f32; 4],
    pub direction: [f32; 3],
    pub spread: f32,
}

/// Per-particle runtime data. Stored in a dense buffer for GPU simulation.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Particle {
    pub position: [f32; 3],
    pub age: f32,
    pub velocity: [f32; 3],
    pub life: f32,
    pub size: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Clone, Debug, Default)]
pub struct ParticleSystemDescriptor {
    pub max_particles: u32,
    pub spawner: ParticleSpawner,
    pub gravity: [f32; 3],
    pub wind: [f32; 3],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ParticleSystem {
    pub descriptor_id: u64,
    pub active: bool,
}

/// Bounding box for particle system culling.
#[derive(Clone, Copy, Debug, Default)]
pub struct ParticleBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of_val;

    #[test]
    fn test_particle_alignment() {
        let p = Particle::default();
        assert_eq!(size_of_val(&p) % 16, 0);
    }
}