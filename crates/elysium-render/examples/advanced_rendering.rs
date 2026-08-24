//! Gelişmiş rendering sistemi kullanım örneği
//! Bu örnek, PBR rendering, CSM gölgeleri ve post-processing efektlerinin nasıl kullanılacağını gösterir

use elysium_render::*;

fn main() {
    println!("Elysium Render Sistemi - Gelişmiş Rendering Örneği");
    
    // Render pipeline yapılandırması
    let config = RenderPipelineConfig {
        enable_pbr: true,
        enable_shadows: true,
        enable_post_processing: true,
        shadow_config: CascadeConfig {
            split_lambda: 0.75,
            resolution: 2048, // Daha yüksek çözünürlük
            num_cascades: 4,
            depth_bias: 0.0001,
        },
    };
    
    // Rendering pipeline sistemi oluştur
    let mut render_pipeline = RenderPipeline::new(config);
    
    // Kamera parametrelerini ayarla
    render_pipeline.lighting_constants.camera_position = glam::Vec3::new(0.0, 5.0, 10.0);
    render_pipeline.lighting_constants.view_matrix = glam::Mat4::look_at_rh(
        glam::Vec3::new(0.0, 5.0, 10.0),  // Pozisyon
        glam::Vec3::ZERO,                   // Hedef
        glam::Vec3::Y                       // Yukarı vektörü
    );
    render_pipeline.lighting_constants.projection_matrix = glam::Mat4::perspective_rh_gl(
        60.0_f32.to_radians(),  // FOV
        16.0 / 9.0,             // En-boy oranı
        0.1,                    // Yakın düzlem
        1000.0                  // Uzak düzlem
    );
    
    // Işık parametrelerini ayarla
    render_pipeline.lighting_constants.directional_light = DirectionalLight {
        direction: glam::Vec3::new(-0.3, -1.0, -0.5).normalize(),
        color: glam::Vec3::new(1.0, 0.95, 0.8),
        intensity: 3.0,
    };
    
    // Ambient ışığı ayarla
    render_pipeline.lighting_constants.ambient_light = glam::Vec3::splat(0.1);
    
    // Post-process efektlerini yapılandır
    render_pipeline.post_process_pipeline.bloom_params = BloomParams {
        intensity: 1.2,
        threshold: 1.0,
        soft_knee: 0.5,
        radius: 7.0,
    };
    
    render_pipeline.post_process_pipeline.ssao_params = SsaoParams {
        sample_radius: 0.5,
        bias: 0.025,
        intensity: 1.5,
        power: 2.0,
        quality: 16,
    };
    
    println!("Render pipeline başarıyla yapılandırıldı!");
    println!("PBR rendering: {}", if render_pipeline.config.enable_pbr { "aktif" } else { "pasif" });
    println!("Gölgeler: {}", if render_pipeline.config.enable_shadows { "aktif" } else { "pasif" });
    println!("Post-processing: {}", if render_pipeline.config.enable_post_processing { "aktif" } else { "pasif" });
    
    // Pipeline'i güncelle
    render_pipeline.update();
    
    // PBR rendering örneği
    let normal = glam::Vec3::Y;
    let view_dir = (glam::Vec3::ZERO - render_pipeline.lighting_constants.camera_position).normalize();
    let light_dir = render_pipeline.lighting_constants.directional_light.direction;
    let albedo = glam::Vec3::new(0.8, 0.4, 0.2);
    let metallic = 0.1;
    let roughness = 0.4;
    let f0 = glam::Vec3::new(0.04, 0.04, 0.04);
    
    let lighting_result = render_pipeline.calculate_pbr_lighting(
        normal,
        view_dir,
        light_dir,
        albedo,
        metallic,
        roughness,
        f0
    );
    
    println!("PBR aydınlatma sonucu: {:?}", lighting_result);
    
    // Render sabitlerini al
    let (lighting_constants, shadow_constants) = render_pipeline.get_render_constants();
    println!("Render sabitleri başarıyla oluşturuldu");
    
    // Material hazırlama örneği
    let material = Material {
        base_color: glam::Vec4::new(0.8, 0.4, 0.2, 1.0),
        metallic: 0.1,
        roughness: 0.4,
        normal_scale: 1.0,
        emissive: glam::Vec3::ZERO,
        alpha_cutoff: 0.5,
    };
    
    let material_params = render_pipeline.prepare_material_params(&material);
    println!("Material parametreleri hazırlandı: {:?}", material_params);
    
    // Shader kaynaklarını al
    let shaders = render_pipeline.post_process_pipeline.process_effects();
    println!("Toplam {} post-processing shader'ı oluşturuldu", shaders.len());
    
    for (name, _) in &shaders {
        println!("  - {}", name);
    }
}