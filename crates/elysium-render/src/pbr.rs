use elysium_core::math::{Vec3, Vec4};

/// PBR rendering için gerekli olan BRDF (Bidirectional Reflectance Distribution Function) hesaplamaları
pub struct PbrLighting;

impl PbrLighting {
    /// Cook-Torrance BRDF modelinin albedo bileşenini hesaplar
    pub fn calculate_diffuse(factor: f32) -> f32 {
        (1.0 - factor) / std::f32::consts::PI
    }

    /// Fresnel yansıma oranı hesaplaması (Schlick yaklaşımı) — power 5
    pub fn fresnel_schlick(f0: Vec3, f90: Vec3, cos_theta: f32) -> Vec3 {
        let x = 1.0 - cos_theta;
        let x2 = x * x;
        let x5 = x2 * x2 * x;
        f0 * (1.0 - x5) + f90 * x5
    }

    /// Microfacet dağılım fonksiyonu (GGX/Trowbridge-Reitz)
    pub fn distribution_ggx(alpha: f32, cos_theta: f32) -> f32 {
        let alpha_sq = alpha * alpha;
        let denom = cos_theta * cos_theta * (alpha_sq - 1.0) + 1.0;
        alpha_sq / (std::f32::consts::PI * denom * denom)
    }

    /// Geometrik atenuasyon faktörü (Smith yöntemi) — alpha burada roughness'tir
    pub fn geometry_smith(alpha: f32, cos_theta: f32) -> f32 {
        // alpha = roughness, roughness'tan türetilen k = (roughness+1)^2/8
        //distribution'da alpha = roughness^2 kullanılır ama Smith'te orijinal alpha gerekir
        let k = (alpha + 1.0) * (alpha + 1.0) / 8.0;
        cos_theta / (cos_theta * (1.0 - k) + k)
    }

    /// PBR rendering için kompleks aydınlatma hesaplaması
    pub fn calculate_lighting(
        normal: Vec3,
        view_dir: Vec3,
        light_dir: Vec3,
        albedo: Vec3,
        metallic: f32,
        roughness: f32,
        f0: Vec3,
    ) -> Vec3 {
        let alpha = roughness * roughness;
        let h = (view_dir + light_dir).normalize();

        let cos_theta_n_l = normal.dot(light_dir).max(0.0);
        let cos_theta_n_v = normal.dot(view_dir).max(0.0);
        let cos_theta_n_h = normal.dot(h).max(0.0);
        let cos_theta_v_h = view_dir.dot(h).max(0.0);

        // Fresnel — f90 = 1.0 enerji korunumu için
        let f = Self::fresnel_schlick(f0 * (1.0 - metallic), Vec3::splat(1.0), cos_theta_v_h);

        // Distribution — alpha = roughness^2
        let d = Self::distribution_ggx(alpha, cos_theta_n_h);

        // Geometry — Smith için orijinal roughness gerekir, alpha değil
        let g = Self::geometry_smith(roughness, cos_theta_n_v) * Self::geometry_smith(roughness, cos_theta_n_l);

        let specular = (f * d * g) / (4.0 * cos_theta_n_l * cos_theta_n_v + 0.001);

        let k_s = f;
        let k_d = Vec3::splat(1.0) - k_s;
        let k_d = k_d * (1.0 - metallic);

        let irradiance = albedo / std::f32::consts::PI;
        let diffuse = k_d * irradiance * cos_theta_n_l;

        diffuse + specular * cos_theta_n_l
    }
}