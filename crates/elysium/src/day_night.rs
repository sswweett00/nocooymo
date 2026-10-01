//! day_night.rs — Gün/Döngüsü (Day-Night Cycle) sistemi
//!
//! Gün saati (0–24) güneş konumunu, gökyüzü renklerini, sis rengini,
//! ışık şiddetlerini ve ambiyans ışığını sürer. Otomatik ilerleme
//! (time_scale) veya preset'ler (Şafak/Öğlen/Akşam/Gece) ile çalışır.

use glam::Vec3;

use crate::renderer::{FogParams, LightType, SceneLight, SkyboxConfig};

/// Gün saati — 0.0 (geçin) .. 24.0 (geçin)
pub type TimeOfDay = f32;

/// Gökyüzü fazı (UI için okunur etiket döndürür)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkyPhase {
    Night,
    Dawn,
    Day,
    Dusk,
}

impl SkyPhase {
    pub fn label(self) -> &'static str {
        match self {
            SkyPhase::Night => "Gece",
            SkyPhase::Dawn => "Şafak",
            SkyPhase::Day => "Gündüz",
            SkyPhase::Dusk => "Akşam",
        }
    }
}

/// Gün/Döngüsü durumu
#[derive(Clone, Debug)]
pub struct DayNightCycle {
    pub enabled: bool,
    /// Otomatik ilerlesin mi
    pub auto_advance: bool,
    /// Gerçek saniye başına ilerleyecek oyun saati
    pub time_scale: f32,
    time: TimeOfDay,
}

impl Default for DayNightCycle {
    fn default() -> Self {
        Self::new()
    }
}

impl DayNightCycle {
    pub fn new() -> Self {
        Self {
            enabled: true,
            auto_advance: false,
            time_scale: 0.5,
            time: 12.0,
        }
    }

    /// Güncel gün saatini döndürür (0..24)
    pub fn time(&self) -> TimeOfDay {
        self.time
    }

    /// Güneşin yukarı bakış açısı: -1 (yer altında) .. +1 (zirve)
    pub fn sun_elevation(&self) -> f32 {
        // 06:00 doğuş, 12:00 zirve, 18:00 batış
        ((self.time - 6.0) / 12.0 * std::f32::consts::PI).sin()
    }

    /// 0 = karanlık gece, 1 = tam gündüz
    pub fn daylight(&self) -> f32 {
        smoothstep(-0.12, 0.18, self.sun_elevation())
    }

    pub fn phase(&self) -> SkyPhase {
        let el = self.sun_elevation();
        let t = self.time;
        if el <= -0.05 {
            SkyPhase::Night
        } else if t < 9.0 {
            SkyPhase::Dawn
        } else if el >= 0.05 && t < 17.0 {
            SkyPhase::Day
        } else {
            SkyPhase::Dusk
        }
    }

    /// Zamanı doğrudan set et (sınıra taşır)
    pub fn set_time(&mut self, t: TimeOfDay) {
        self.time = t.rem_euclid(24.0);
    }

    /// Preset'ler
    pub fn set_dawn(&mut self) {
        self.set_time(6.5);
    }
    pub fn set_noon(&mut self) {
        self.set_time(12.0);
    }
    pub fn set_dusk(&mut self) {
        self.set_time(18.5);
    }
    pub fn set_night(&mut self) {
        self.set_time(23.0);
    }

    /// dt saniye ilerlet; aktif değilse etki etmez
    pub fn advance(&mut self, dt: f32) {
        if !self.enabled || !self.auto_advance {
            return;
        }
        self.time = (self.time + self.time_scale * dt).rem_euclid(24.0);
    }

    /// Güneş yönü (dünyadan güneşe bakan birim vektör)
    pub fn sun_direction(&self) -> Vec3 {
        let el = self.sun_elevation();
        // Güneş doğudan (x+) doğar, batıya (x-) batar; öğlen tepeye (y+) yükselir
        let x = ((self.time - 6.0) / 12.0 * std::f32::consts::PI).cos();
        let mut dir = Vec3::new(x, el, 0.25);
        if dir.length_squared() < 1e-6 {
            dir = Vec3::new(0.0, -1.0, 0.0);
        }
        dir.normalize()
    }

    /// Dkyüzü yapılandırmasını gün saatine göre günceller
    pub fn apply_skybox(&self, sky: &mut SkyboxConfig) {
        let day = self.daylight();
        let dawn_dusk = dawn_dusk_factor(self.sun_elevation());

        // Gece → Gündüz renk geçişi
        let zenith_night = Vec3::new(0.01, 0.02, 0.06);
        let zenith_day = Vec3::new(0.12, 0.35, 0.75);
        let horizon_night = Vec3::new(0.03, 0.04, 0.09);
        let horizon_day = Vec3::new(0.55, 0.7, 0.85);
        let ground_night = Vec3::new(0.04, 0.04, 0.05);
        let ground_day = Vec3::new(0.25, 0.22, 0.18);

        let mut zenith = zenith_night.lerp(zenith_day, day);
        let mut horizon = horizon_night.lerp(horizon_day, day);

        // Şafak/akşam turunculuğu
        let warm = Vec3::new(0.9, 0.45, 0.2);
        zenith = zenith + warm * (dawn_dusk * 0.15 * (1.0 - day.abs()));
        horizon = horizon.lerp(warm, dawn_dusk * 0.55);

        sky.zenith_color = zenith;
        sky.horizon_color = horizon;
        sky.ground_color = ground_night.lerp(ground_day, day);
        sky.sun_direction = self.sun_direction();
        // Günbatımında güneş renkli ve alçak; gecede söner
        let sun_col = Vec3::new(1.0, 0.95, 0.8).lerp(Vec3::new(1.0, 0.45, 0.2), dawn_dusk * 0.8);
        sky.sun_color = sun_col;
        sky.sun_intensity = 0.1 + 2.2 * day;
        // Yıldızlar yalnızca karanlıkta
        sky.star_density = 0.004 * (1.0 - day) * (1.0 - dawn_dusk * 0.5);
    }

    /// Sahne ışıklarını gün saatine göre ayarlar.
    /// İlk Directional ışık "güneş" kabul edilir; gerisi dolgu/ışıltı olarak ölçeklenir.
    pub fn apply_lights(&self, lights: &mut [SceneLight]) {
        let day = self.daylight();
        let warm = dawn_dusk_factor(self.sun_elevation());

        let mut sun_seen = false;
        for light in lights.iter_mut() {
            match light.light_type {
                LightType::Directional if !sun_seen => {
                    sun_seen = true;
                    light.direction = self.sun_direction();
                    let sun_col = Vec3::new(1.0, 0.95, 0.85)
                        .lerp(Vec3::new(1.0, 0.5, 0.25), warm * 0.7);
                    // Gece düşük soğuk dolgu, gündüz tam güneş
                    night_side_light(&mut light.color, sun_col, day);
                    light.intensity = 0.12 + 1.7 * day;
                }
                LightType::Directional => {
                    // Dolgu ışığı: gecede biraz daha belirgin (soğuk)
                    light.intensity = 0.15 + 0.45 * day;
                    light.color = light.color.lerp(Vec3::new(0.25, 0.35, 0.55), (1.0 - day) * 0.6);
                }
                LightType::Point { .. } | LightType::Spot { .. } => {
                    // Dekoratif ışıklar gecede daha parlak hissettirsin
                    light.intensity = light.intensity.max(0.2);
                }
            }
        }
    }

    /// Ambiyans rengi + şiddeti
    pub fn apply_ambient(&self, color: &mut Vec3, intensity: &mut f32) {
        let day = self.daylight();
        let mut amb = Vec3::new(0.05, 0.06, 0.12).lerp(Vec3::new(0.35, 0.38, 0.4), day);
        let warm = dawn_dusk_factor(self.sun_elevation());
        amb = amb.lerp(Vec3::new(0.3, 0.2, 0.15), warm * 0.4);
        *color = amb;
        *intensity = 0.15 + 0.45 * day;
    }

    /// Sis rengini gökyüzü ufkuyla hizalar
    pub fn apply_fog(&self, fog: &mut FogParams) {
        let day = self.daylight();
        let warm = dawn_dusk_factor(self.sun_elevation());
        let night = Vec3::new(0.02, 0.03, 0.06);
        let daytime = Vec3::new(0.6, 0.7, 0.82);
        let mut c = night.lerp(daytime, day);
        c = c.lerp(Vec3::new(0.75, 0.4, 0.25), warm * 0.5);
        fog.color = c;
        // Gecede sis biraz yoğun
        fog.density = 0.015 + 0.008 * (1.0 - day);
    }
}

/// Şafak/akşam faktörü: güneş ufka yakınken 1, tepe/derinde 0
fn dawn_dusk_factor(elevation: f32) -> f32 {
    (1.0 - elevation.abs()).clamp(0.0, 1.0).powi(2)
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Gece tarafında soğuk maviye kaydır
fn night_side_light(current: &mut Vec3, day_color: Vec3, day: f32) {
    *current = day_color.lerp(Vec3::new(0.3, 0.4, 0.7), (1.0 - day) * 0.7);
}

// ═══════════════════════════════════════════════════════════ Testler

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daylight_range() {
        let mut dn = DayNightCycle::new();
        dn.set_noon();
        assert!(dn.daylight() > 0.95, "öğlen %100 güneş olmalı: {}", dn.daylight());
        dn.set_night();
        assert!(dn.daylight() < 0.05, "gece karanlık olmalı: {}", dn.daylight());
    }

    #[test]
    fn test_time_wraps() {
        let mut dn = DayNightCycle::new();
        dn.set_time(30.0);
        assert!((dn.time() - 6.0).abs() < 1e-5);
        dn.set_time(-2.0);
        assert!((dn.time() - 22.0).abs() < 1e-5);
    }

    #[test]
    fn test_advance_wraps_24h() {
        let mut dn = DayNightCycle::new();
        dn.auto_advance = true;
        dn.time_scale = 1.0;
        dn.set_time(23.5);
        dn.advance(1.0);
        assert!((dn.time() - 0.5).abs() < 1e-4, "24 saatte sarılmalı: {}", dn.time());
    }

    #[test]
    fn test_skybox_day_night_colors() {
        let mut dn = DayNightCycle::new();
        let mut sky_day = SkyboxConfig::default();
        let mut sky_night = SkyboxConfig::default();
        dn.set_noon();
        dn.apply_skybox(&mut sky_day);
        dn.set_night();
        dn.apply_skybox(&mut sky_night);
        // Gündüz ufku geceden belirgin şekilde parlak
        assert!(sky_day.horizon_color.length() > sky_night.horizon_color.length());
        // Gündüz güneş şiddeti yüksek
        assert!(sky_day.sun_intensity > sky_night.sun_intensity);
        // Gece yıldız yoğunluğu yüksek
        assert!(sky_night.star_density > sky_day.star_density);
    }

    #[test]
    fn test_lights_day_night() {
        let mut dn = DayNightCycle::new();
        let mut lights = vec![
            SceneLight::directional(Vec3::Y, Vec3::ONE, 1.0),
            SceneLight::point(Vec3::ZERO, Vec3::ONE, 1.0, 10.0),
        ];
        dn.set_noon();
        dn.apply_lights(&mut lights);
        let noon_sun = lights[0].intensity;
        dn.set_night();
        dn.apply_lights(&mut lights);
        assert!(noon_sun > lights[0].intensity, "öğlen güneşi geceden parlak");
        // Güneş yönü öğlen yukarı bakmalı
        let dir = dn.sun_direction();
        assert!(dir.y > 0.5);
    }

    #[test]
    fn test_fog_follows_phase() {
        let mut dn = DayNightCycle::new();
        let mut fog = FogParams::default();
        let base = fog.density;
        dn.set_night();
        dn.apply_fog(&mut fog);
        assert!(fog.density >= base, "gece sis yoğunlaşmalı");
        dn.set_noon();
        let mut fog_day = FogParams::default();
        dn.apply_fog(&mut fog_day);
        assert!(fog_day.color.y > fog.color.y, "gündüz sis daha parlak");
    }
}
