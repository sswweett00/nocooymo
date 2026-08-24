use elysium_core::math::{Vec3, Vec4};

/// Farklı doku türlerini temsil eder
#[derive(Debug, Clone, PartialEq)]
pub enum TextureType {
    BaseColor,
    Normal,
    Metallic,
    Roughness,
    Emissive,
    Occlusion,
    AlphaMask,
}

/// Doku referansı
#[derive(Debug, Clone)]
pub struct Texture {
    pub handle: u32,  // GPU handle
    pub texture_type: TextureType,
    pub sampler_params: SamplerParams,
}

#[derive(Debug, Clone)]
pub struct SamplerParams {
    pub mag_filter: FilterMode,
    pub min_filter: FilterMode,
    pub mipmap_filter: FilterMode,
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
    pub address_mode_w: AddressMode,
    pub anisotropy_clamp: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterMode {
    Nearest,
    Linear,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AddressMode {
    Repeat,
    Mirror,
    ClampToEdge,
    ClampToBorder,
}

/// PBR malzeme tanımı
#[derive(Debug, Clone)]
pub struct Material {
    pub base_color: Vec4,
    pub base_color_texture: Option<Texture>,
    pub metallic: f32,
    pub metallic_texture: Option<Texture>,
    pub roughness: f32,
    pub roughness_texture: Option<Texture>,
    pub normal_texture: Option<Texture>,
    pub normal_scale: f32,
    pub emissive: Vec3,
    pub emissive_texture: Option<Texture>,
    pub occlusion_texture: Option<Texture>,
    pub alpha_cutoff: f32,
    pub alpha_mode: AlphaMode,
    pub double_sided: bool,
    pub cull_mode: CullMode,
    pub unlit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlphaMode {
    Opaque,
    Mask(f32),      // Alpha cutoff değeri
    Blend,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CullMode {
    Back,
    Front,
    None,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            base_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            base_color_texture: None,
            metallic: 0.0,
            metallic_texture: None,
            roughness: 0.5,
            roughness_texture: None,
            normal_texture: None,
            normal_scale: 1.0,
            emissive: Vec3::ZERO,
            emissive_texture: None,
            occlusion_texture: None,
            alpha_cutoff: 0.5,
            alpha_mode: AlphaMode::Opaque,
            double_sided: false,
            cull_mode: CullMode::Back,
            unlit: false,
        }
    }
}

impl Material {
    /// Malzemenin herhangi bir doku içerip içermediğini kontrol eder
    pub fn has_textures(&self) -> bool {
        self.base_color_texture.is_some() ||
        self.metallic_texture.is_some() ||
        self.roughness_texture.is_some() ||
        self.normal_texture.is_some() ||
        self.emissive_texture.is_some() ||
        self.occlusion_texture.is_some()
    }

    /// Malzemenin transparan olup olmadığını kontrol eder
    pub fn is_transparent(&self) -> bool {
        matches!(self.alpha_mode, AlphaMode::Blend) || self.base_color.w < 1.0
    }

    /// Malzemenin alpha testing gerektirip gerektirmediğini kontrol eder
    pub fn requires_alpha_test(&self) -> bool {
        matches!(self.alpha_mode, AlphaMode::Mask(_))
    }

    /// Malzeme için gerekli olan tüm doku türlerini döndürür
    pub fn get_required_textures(&self) -> Vec<TextureType> {
        let mut textures = Vec::new();
        
        if self.base_color_texture.is_some() {
            textures.push(TextureType::BaseColor);
        }
        if self.metallic_texture.is_some() {
            textures.push(TextureType::Metallic);
        }
        if self.roughness_texture.is_some() {
            textures.push(TextureType::Roughness);
        }
        if self.normal_texture.is_some() {
            textures.push(TextureType::Normal);
        }
        if self.emissive_texture.is_some() {
            textures.push(TextureType::Emissive);
        }
        if self.occlusion_texture.is_some() {
            textures.push(TextureType::Occlusion);
        }
        
        textures
    }
}

/// Malzeme varyantlarını tanımlar (farklı rendering modları için)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaterialVariant {
    Default,
    Transparent,
    AlphaTest,
    Unlit,
    Shadow,
    DepthPrepass,
}

impl MaterialVariant {
    /// Malzeme varyantına göre uygun shader kombinasyonunu döndürür
    pub fn get_shader_permutation(&self) -> &'static str {
        match self {
            MaterialVariant::Default => "PBR_DEFAULT",
            MaterialVariant::Transparent => "PBR_TRANSPARENT",
            MaterialVariant::AlphaTest => "PBR_ALPHA_TEST",
            MaterialVariant::Unlit => "UNLIT",
            MaterialVariant::Shadow => "SHADOW_MAP",
            MaterialVariant::DepthPrepass => "DEPTH_PREPASS",
        }
    }
}