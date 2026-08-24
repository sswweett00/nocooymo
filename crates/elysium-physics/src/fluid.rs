//! Fluid simulation (Mimari §3.1 F14) — Smoothed-Particle Hydrodynamics.
//!
//! A compact SPH backend: density via the Poly6 kernel, pressure via the
//! Spiky kernel, viscosity plus gravity, integrated against a container.

use crate::math::{Aabb, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct FluidParticle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub density: f32,
    pub pressure: f32,
    pub mass: f32,
}

impl FluidParticle {
    pub fn new(pos: Vec3, mass: f32) -> Self {
        Self { pos, vel: Vec3::zeros(), density: 0.0, pressure: 0.0, mass }
    }
}

/// A container-bounded SPH fluid volume.
#[derive(Debug, Clone)]
pub struct SPHFluid {
    pub particles: Vec<FluidParticle>,
    pub kernel_radius: f32,
    pub rest_density: f32,
    pub gas_constant: f32,
    pub viscosity: f32,
    pub gravity: Vec3,
    /// CFL-style cap on particle velocity magnitude.
    pub max_speed: f32,
}

impl Default for SPHFluid {
    fn default() -> Self {
        Self::new()
    }
}

impl SPHFluid {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            kernel_radius: 0.2,
            rest_density: 1000.0,
            gas_constant: 300.0,
            viscosity: 0.1,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            max_speed: 30.0,
        }
    }

    pub fn spawn_grid(&mut self, origin: Vec3, spacing: f32, nx: usize, ny: usize, nz: usize, mass: f32) {
        self.particles.clear();
        for x in 0..nx {
            for y in 0..ny {
                for z in 0..nz {
                    let pos = origin + Vec3::new(x as f32 * spacing, y as f32 * spacing, z as f32 * spacing);
                    self.particles.push(FluidParticle::new(pos, mass));
                }
            }
        }
    }

    /// Poly6 kernel for density.
    fn density_kernel(&self, h: f32) -> f32 {
        315.0 / (64.0 * std::f32::consts::PI * h.powi(9))
    }

    /// Spiky gradient kernel for pressure force — q = distance
    fn pressure_gradient_mag(&self, h: f32, dist: f32) -> f32 {
        let h3 = h.powi(3);
        let coef = -45.0 / (std::f32::consts::PI * h3);
        coef * (h - dist) * (h - dist)
    }

    /// Advance the simulation; `bounds` is the containing volume.
    pub fn update(&mut self, dt: f32, bounds: &Aabb) {
        self.compute_density();
        self.compute_pressure();
        self.integrate(dt, bounds);
    }

    fn compute_density(&mut self) {
        let h = self.kernel_radius;
        let k = self.density_kernel(h);
        let h2 = h * h;
        for i in 0..self.particles.len() {
            let mut density = 0.0;
            let pi = self.particles[i].pos;
            for j in 0..self.particles.len() {
                // self contribution dahil — SPH yoğunluk için kritik
                let d2 = (self.particles[j].pos - pi).norm_squared();
                if d2 < h2 {
                    let t = h2 - d2;
                    density += self.particles[j].mass * k * t * t * t;
                }
            }
            self.particles[i].density = density;
        }
    }

    fn compute_pressure(&mut self) {
        for p in self.particles.iter_mut() {
            p.pressure = self.gas_constant * (p.density - self.rest_density).max(0.0);
        }
    }

    fn integrate(&mut self, dt: f32, bounds: &Aabb) {
        let h = self.kernel_radius;
        let h2 = h * h;
        let n = self.particles.len();
        let grads = self.compute_pressure_accel(h, h2);
        let pos: Vec<Vec3> = self.particles.iter().map(|p| p.pos).collect();
        let vel: Vec<Vec3> = self.particles.iter().map(|p| p.vel).collect();
        let dens: Vec<f32> = self.particles.iter().map(|p| p.density).collect();

        // Viscosity: SPH laplacian — toplam, ortalama değil
        let mut viscosity = vec![Vec3::zeros(); n];
        for i in 0..n {
            let mut acc = Vec3::zeros();
            for j in 0..n {
                if i == j {
                    continue;
                }
                let d = (pos[j] - pos[i]).norm();
                if d < h && d > 1e-9 {
                    let mj = self.particles[j].mass;
                    let dj = dens[j].max(1e-6);
                    // standard SPH viscosity term
                    acc += (vel[j] - vel[i]) * (self.viscosity * mj / dj) * (1.0 / d);
                }
            }
            viscosity[i] = acc;
        }

        let max_speed = self.max_speed;
        for (i, p) in self.particles.iter_mut().enumerate() {
            let accel = self.gravity + grads[i] + viscosity[i];
            p.vel += accel * dt;
            if p.vel.norm() > max_speed {
                p.vel = p.vel.normalize() * max_speed;
            }
            p.pos += p.vel * dt;
            Self::resolve_bounds(p, bounds);
        }
    }

    fn compute_pressure_accel(&self, h: f32, _h2: f32) -> Vec<Vec3> {
        let n = self.particles.len();
        let mut accel = vec![Vec3::zeros(); n];
        for (i, p_i) in self.particles.iter().enumerate() {
            let pi = p_i.pos;
            let di = p_i.density.max(1e-6);
            for j in 0..n {
                if i == j {
                    continue;
                }
                let d_vec = pi - self.particles[j].pos;
                let dist = d_vec.norm();
                if dist < h && dist > 1e-9 {
                    let grad = self.pressure_gradient_mag(h, dist);
                    let dir = d_vec / dist;
                    let dj = self.particles[j].density.max(1e-6);
                    let mj = self.particles[j].mass;
                    // doğru basınç: (pi/di² + pj/dj²) * grad * mj  ; grad zaten (h-dist)² içeriyor
                    accel[i] -= (p_i.pressure / (di * di) + self.particles[j].pressure / (dj * dj))
                        * grad
                        * dir
                        * mj;
                }
            }
        }
        accel
    }

fn resolve_bounds(p: &mut FluidParticle, bounds: &Aabb) {
        let eps = 0.001;
        if p.pos.x < bounds.min.x + eps {
            p.pos.x = bounds.min.x + eps;
            p.vel.x = p.vel.x.abs() * 0.3;
        }
        if p.pos.x > bounds.max.x - eps {
            p.pos.x = bounds.max.x - eps;
            p.vel.x = -p.vel.x.abs() * 0.3;
        }
        if p.pos.y < bounds.min.y + eps {
            p.pos.y = bounds.min.y + eps;
            p.vel.y = p.vel.y.abs() * 0.3;
        }
        if p.pos.y > bounds.max.y - eps {
            p.pos.y = bounds.max.y - eps;
            p.vel.y = -p.vel.y.abs() * 0.3;
        }
        if p.pos.z < bounds.min.z + eps {
            p.pos.z = bounds.min.z + eps;
            p.vel.z = p.vel.z.abs() * 0.3;
        }
        if p.pos.z > bounds.max.z - eps {
            p.pos.z = bounds.max.z - eps;
            p.vel.z = -p.vel.z.abs() * 0.3;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fluid_falls_and_settles() {
        let mut fluid = SPHFluid::new();
        fluid.kernel_radius = 0.5;
        fluid.spawn_grid(Vec3::new(0.0, 1.0, 0.0), 0.4, 3, 3, 1, 1.0);
        let bounds = Aabb::new(Vec3::new(-5.0, -1.0, -5.0), Vec3::new(5.0, 5.0, 5.0));
        for _ in 0..60 {
            fluid.update(1.0 / 60.0, &bounds);
        }
        // Particles must have fallen and be stable (finite, bounded speed).
        for p in &fluid.particles {
            assert!(p.pos.y < 0.8, "particle did not settle: {}", p.pos.y);
            assert!(p.pos.x.is_finite() && p.pos.y.is_finite() && p.pos.z.is_finite());
            assert!(p.vel.norm() < 40.0, "unstable velocity: {}", p.vel);
        }
    }
}
