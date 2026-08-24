//! Kapsamlı rendering sistemi kullanım örneği
//! Bu örnek, tüm gelişmiş rendering sistemlerinin nasıl bir arada kullanılacağını gösterir

use elysium_render::*;

fn main() {
    println!("Elysium Render Sistemi - Kapsamlı Rendering Örneği");
    
    // Ana rendering pipeline oluştur
    let mut render_pipeline = RenderPipeline::new(RenderPipelineConfig::default());
    
    // Gelişmiş rendering sistemlerini etkinleştir
    let mut advanced_systems = AdvancedRenderingSystems::new();
    advanced_systems.enable_deferred_shading((1920, 1080));
    advanced_systems.enable_ssr();
    advanced_systems.enable_taa((1920, 1080));
    advanced_systems.enable_volumetric_fog();
    
    // Sanal geometri sistemini oluştur
    let mut virtual_geometry = VirtualGeometrySystem::new(VirtualGeometryConfig::default());
    
    // Sanal dokuma sistemini oluştur
    let mut virtual_texturing = VirtualTextureSystem::new();
    virtual_texturing.create_atlas(
        "main_atlas".to_string(),
        VirtualTextureAtlasConfig::default(),
        TextureFormat::Rgba8Srgb
    );
    virtual_texturing.activate_atlas("main_atlas");
    
    // İskelet animasyon sistemini oluştur
    let mut skeleton = Skeleton::new();
    let root_bone = skeleton.add_bone(
        "root".to_string(),
        None,
        elysium_core::math::Mat4::IDENTITY,
        elysium_core::math::Mat4::IDENTITY
    );
    
    // Kamera ve ışık ayarları
    render_pipeline.lighting_constants.camera_position = elysium_core::math::Vec3::new(0.0, 5.0, 10.0);
    render_pipeline.lighting_constants.view_matrix = elysium_core::math::Mat4::look_at_rh(
        elysium_core::math::Vec3::new(0.0, 5.0, 10.0),
        elysium_core::math::Vec3::ZERO,
        elysium_core::math::Vec3::Y
    );
    render_pipeline.lighting_constants.projection_matrix = elysium_core::math::Mat4::perspective_rh_gl(
        60.0_f32.to_radians(),
        16.0 / 9.0,
        0.1,
        1000.0
    );
    
    // Ek ışıklar ekle
    if let Some(ref mut light_culling) = advanced_systems.deferred_shading.as_mut().map(|ds| &mut ds.light_culling) {
        light_culling.add_light(Light::Point(PointLight {
            position: elysium_core::math::Vec3::new(5.0, 5.0, 5.0),
            color: elysium_core::math::Vec3::new(1.0, 0.8, 0.6),
            intensity: 100.0,
            range: 20.0,
        }));
        
        light_culling.add_light(Light::Spot(SpotLight {
            position: elysium_core::math::Vec3::new(-5.0, 8.0, 3.0),
            direction: elysium_core::math::Vec3::new(0.0, -1.0, 0.0),
            color: elysium_core::math::Vec3::new(0.6, 0.8, 1.0),
            intensity: 50.0,
            range: 15.0,
            inner_cone_angle: 20.0_f32.to_radians(),
            outer_cone_angle: 40.0_f32.to_radians(),
        }));
    }
    
    // Sanal geometri blokları ekle
    virtual_geometry.add_block(1, elysium_core::math::Vec3::new(0.0, 0.0, 0.0), elysium_core::math::Vec3::splat(100.0));
    virtual_geometry.add_block(2, elysium_core::math::Vec3::new(200.0, 0.0, 0.0), elysium_core::math::Vec3::splat(100.0));
    virtual_geometry.add_block(3, elysium_core::math::Vec3::new(-200.0, 0.0, 0.0), elysium_core::math::Vec3::splat(100.0));
    
    // Kamera pozisyonunu güncelle
    virtual_geometry.update_camera_position(render_pipeline.lighting_constants.camera_position);
    
    // Sanal dokuma talepleri
    if let Some(result) = virtual_texturing.request_tile((0, 0), 0) {
        println!("Sanal dokuma talebi sonucu: {:?}", result);
    }
    
    // Sanal dokuma atlaslarını güncelle
    virtual_texturing.update_all_atlases();
    
    // Gelişmiş sistemleri güncelle
    advanced_systems.update();
    
    // Pipeline'i güncelle
    render_pipeline.update();
    
    // PBR rendering örneği
    let normal = elysium_core::math::Vec3::Y;
    let view_dir = (elysium_core::math::Vec3::ZERO - render_pipeline.lighting_constants.camera_position).normalize();
    let light_dir = render_pipeline.lighting_constants.directional_light.direction;
    let albedo = elysium_core::math::Vec3::new(0.8, 0.4, 0.2);
    let metallic = 0.1;
    let roughness = 0.4;
    let f0 = elysium_core::math::Vec3::new(0.04, 0.04, 0.04);
    
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
    
    // LOD gruplarını yazdır
    let lod_groups = virtual_geometry.group_by_lod();
    println!("LOD grupları:");
    for (lod, blocks) in &lod_groups {
        println!("  LOD {}: {} blok", lod, blocks.len());
    }
    
    // Görünürlük testi için sahte frustum düzlemleri
    let frustum_planes = [
        elysium_core::math::Vec4::new(1.0, 0.0, 0.0, 1.0), // Sağ
        elysium_core::math::Vec4::new(-1.0, 0.0, 0.0, 1.0), // Sol
        elysium_core::math::Vec4::new(0.0, 1.0, 0.0, 1.0), // Üst
        elysium_core::math::Vec4::new(0.0, -1.0, 0.0, 1.0), // Alt
        elysium_core::math::Vec4::new(0.0, 0.0, 1.0, 1.0), // Yakın
        elysium_core::math::Vec4::new(0.0, 0.0, -1.0, 1.0), // Uzak
    ];
    
    let visible_blocks = virtual_geometry.cull_by_frustum(&frustum_planes);
    println!("Frustum içinde {} blok görünür", visible_blocks.len());
    
    // Render sabitlerini al
    let (lighting_constants, shadow_constants) = render_pipeline.get_render_constants();
    println!("Render sabitleri oluşturuldu");
    
    // Gelişmiş rendering sistemlerinin durumu
    println!("Deferred shading: {}", if advanced_systems.deferred_shading.is_some() { "aktif" } else { "pasif" });
    println!("SSR: {}", if advanced_systems.ssr.is_some() { "aktif" } else { "pasif" });
    println!("TAA: {}", if advanced_systems.taa.is_some() { "aktif" } else { "pasif" });
    println!("Volumetric fog: {}", if advanced_systems.volumetric_fog.is_some() { "aktif" } else { "pasif" });
    
    println!("\nTüm gelişmiş rendering sistemleri başarıyla başlatıldı ve entegre edildi!");
    println!("Bu sistemler sayesinde:");
    println!("- PBR rendering ile fiziksel olarak doğru malzeme ve ışık modellemesi");
    println!("- CSM ile kaliteli gölgeler");
    println!("- Deferred shading ile çoklu ışık kaynakları");
    println!("- SSR ile ekran uzayında yansıma efektleri");
    println!("- TAA ile zaman bazlı anti-aliasing");
    println!("- Volumetric fog ile hacimsel sis efektleri");
    println!("- Sanal geometri ile büyük dünyalar");
    println!("- Sanal dokuma ile verimli doku kullanımı");
    println!("- Skeletal animasyon ile karakter animasyonları");
    println!("...gibi modern rendering teknikleri kullanılabilir.");
}