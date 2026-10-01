//! benchmark.rs — Performans / stres testi modu
//!
//! Belirlenen sayıda varlık üretir, çerçeve döngüsünü (fizik + update)
//! çalıştırır, ortalama/en düşük/en yüksek kare süresini ve FPS'i ölçer.
//! Test bitince üretilen varlıklar sahnedan kaldırılır (sahne aslına döner).

use std::time::Instant;

use crate::renderer::{EntityTeam, GeometryType, Scene};

/// Benchmark yapılandırması
#[derive(Clone, Copy, Debug)]
pub struct BenchmarkConfig {
    /// Üretilecek ek varlık sayısı
    pub entity_count: usize,
    /// Çalışacak çerçeve sayısı
    pub frame_count: u32,
    /// Kare başına simüle edilecek delta (saniye)
    pub dt: f32,
    /// Fizik adımı çalıştırılsın mı
    pub with_physics: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            entity_count: 200,
            frame_count: 120,
            dt: 1.0 / 60.0,
            with_physics: true,
        }
    }
}

/// Ölçüm sonucu
#[derive(Clone, Debug)]
pub struct BenchmarkResult {
    pub entity_count: usize,
    pub frames: u32,
    pub elapsed_secs: f32,
    pub avg_frame_ms: f32,
    pub min_frame_ms: f32,
    pub max_frame_ms: f32,
    pub fps: f32,
    /// Kare başına ortalama işlenen varlık sayısı (spawn + mevcut)
    pub objects_per_frame: usize,
    pub physics: bool,
}

impl BenchmarkResult {
    /// Konsola/üç panel yazısı için çok satırlık rapor
    pub fn report(&self) -> String {
        format!(
            "BENCHMARK {} varlık x {} kare ({}):\n  ort. kare: {:.2} ms | min: {:.2} ms | max: {:.2} ms\n  FPS: {:.1} | fizik: {}",
            self.entity_count,
            self.frames,
            if self.physics { "AÇIK" } else { "KAPALI" },
            self.avg_frame_ms,
            self.min_frame_ms,
            self.max_frame_ms,
            self.fps,
            if self.physics { "evet" } else { "hayır" },
        )
    }

    /// Tek satırlık özet (status bar için)
    pub fn summary(&self) -> String {
        format!(
            "Benchmark: {} varlık @ {:.1} FPS (ort. {:.2} ms)",
            self.entity_count, self.fps, self.avg_frame_ms
        )
    }
}

/// Benchmark'ı çalıştırır: sahneye test varlıkları ekler, döngüyü koşar,
/// sonra ek varlıkları siler. `&mut Scene` orijinal haliyle korunur.
pub fn run_benchmark(scene: &mut Scene, config: &BenchmarkConfig) -> BenchmarkResult {
    let base_count = scene.objects.len();
    let base_ids: Vec<usize> = scene.objects.iter().map(|o| o.id).collect();

    // ── Test varlıklarını üret
    for i in 0..config.entity_count {
        let id = scene.add_object(
            format!("Bench {}", i),
            if i % 3 == 0 { GeometryType::Cube } else { GeometryType::Sphere },
            EntityTeam::Neutral,
        );
        if let Some(obj) = scene.get_object_mut(id) {
            let a = i as f32 * 0.7;
            let r = 5.0 + (i % 8) as f32 * 2.0;
            obj.transform.position = glam::Vec3::new(a.cos() * r, 1.0 + (i % 4) as f32, a.sin() * r);
            if config.with_physics {
                obj.rigid_body = Some(crate::renderer::RigidBody {
                    body_type: crate::renderer::RigidBodyType::Dynamic,
                    mass: 1.0,
                    restitution: 0.4,
                    ..Default::default()
                });
            }
        }
    }
    let total = scene.objects.len();

    // ── Zamanlanan döngü
    let mut min_ms = f32::MAX;
    let mut max_ms = 0.0f32;
    let mut sum_ms = 0.0f32;
    let sim_start = Instant::now();

    for frame in 0..config.frame_count {
        let t0 = Instant::now();
        // Döngü: fizik + basit per-object update (rotasyon)
        if config.with_physics {
            scene.physics_step(config.dt);
        }
        for obj in &mut scene.objects {
            obj.transform.rotation.y += config.dt * 0.5;
        }
        let ms = t0.elapsed().as_secs_f32() * 1000.0;
        min_ms = min_ms.min(ms);
        max_ms = max_ms.max(ms);
        sum_ms += ms;
        let _ = frame;
    }

    let elapsed = sim_start.elapsed().as_secs_f32();

    // ── Test varlıklarını temizle (yalnızca yeni eklenenleri)
    scene.objects.retain(|o| base_ids.contains(&o.id));

    let avg = if config.frame_count > 0 { sum_ms / config.frame_count as f32 } else { 0.0 };
    BenchmarkResult {
        entity_count: config.entity_count,
        frames: config.frame_count,
        elapsed_secs: elapsed,
        avg_frame_ms: avg,
        min_frame_ms: if min_ms == f32::MAX { 0.0 } else { min_ms },
        max_frame_ms: max_ms,
        fps: if avg > 0.0 { 1000.0 / avg } else { 0.0 },
        objects_per_frame: total,
        physics: config.with_physics,
    }
}

// ═══════════════════════════════════════════════════════════ Testler

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_small() {
        let mut scene = Scene::new();
        let cfg = BenchmarkConfig { entity_count: 50, frame_count: 30, ..Default::default() };
        let before = scene.objects.len();
        let res = run_benchmark(&mut scene, &cfg);
        assert_eq!(res.entity_count, 50);
        assert_eq!(res.frames, 30);
        assert!(res.fps > 0.0, "FPS pozitif olmalı: {}", res.fps);
        assert!(res.avg_frame_ms > 0.0);
        assert!(res.min_frame_ms <= res.avg_frame_ms);
        assert!(res.avg_frame_ms <= res.max_frame_ms);
        // Sahne aslına dönmeli
        assert_eq!(scene.objects.len(), before, "test varlıkları temizlenmeli");
    }

    #[test]
    fn test_benchmark_no_physics() {
        let mut scene = Scene::new();
        let cfg = BenchmarkConfig { with_physics: false, ..Default::default() };
        let res = run_benchmark(&mut scene, &cfg);
        assert!(!res.physics);
        assert_eq!(scene.objects.len(), 0);
    }

    #[test]
    fn test_benchmark_report_format() {
        let res = BenchmarkResult {
            entity_count: 100,
            frames: 60,
            elapsed_secs: 1.0,
            avg_frame_ms: 16.0,
            min_frame_ms: 10.0,
            max_frame_ms: 25.0,
            fps: 62.5,
            objects_per_frame: 105,
            physics: true,
        };
        let report = res.report();
        assert!(report.contains("100 varlık"));
        assert!(report.contains("FPS"));
        let summary = res.summary();
        assert!(summary.contains("Benchmark"));
        assert!(summary.contains("62.5 FPS"));
    }

    #[test]
    fn test_benchmark_preserves_original_objects() {
        let mut scene = Scene::new();
        let id = scene.add_object("Keep".into(), GeometryType::Cube, EntityTeam::Player);
        let cfg = BenchmarkConfig { entity_count: 20, frame_count: 10, ..Default::default() };
        run_benchmark(&mut scene, &cfg);
        assert_eq!(scene.objects.len(), 1);
        assert_eq!(scene.get_object(id).unwrap().name, "Keep");
    }
}
