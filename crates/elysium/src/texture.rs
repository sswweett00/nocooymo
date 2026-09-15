//! texture.rs — Texture import ve sampling sistemi
//!
//! PNG/JPG/WEBP/BMP texture yükleme, UV sampling, bilinear filtreleme.
//! PBR material sistemiyle entegre çalışır.

use glam::Vec3;

// ═══════════════════════════════════════════════════════════ Texture Türleri

/// Texture pixel formatı
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextureFormat {
    RGBA8,
    RGB8,
    Grayscale8,
}

/// CPU'da tutulan texture verisi
#[derive(Clone, Debug)]
pub struct Texture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub pixels: Vec<u8>,
    pub mip_levels: u8,
}

impl Texture {
    /// Boş texture oluştur
    pub fn new(name: &str, width: u32, height: u32) -> Self {
        let pixels = vec![128u8; (width * height * 4) as usize];
        Self {
            name: name.to_string(),
            width,
            height,
            format: TextureFormat::RGBA8,
            pixels,
            mip_levels: 1,
        }
    }

    /// Tek renkli texture oluştur
    pub fn solid_color(name: &str, r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            name: name.to_string(),
            width: 1,
            height: 1,
            format: TextureFormat::RGBA8,
            pixels: vec![r, g, b, a],
            mip_levels: 1,
        }
    }

    /// Checkerboard texture oluştur (varsayılan/grid için)
    pub fn checkerboard(name: &str, size: u32, check_size: u32, c1: [u8; 3], c2: [u8; 3]) -> Self {
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        for y in 0..size {
            for x in 0..size {
                let check = ((x / check_size) + (y / check_size)) % 2 == 0;
                let c = if check { c1 } else { c2 };
                pixels.extend_from_slice(&c);
                pixels.push(255);
            }
        }
        Self {
            name: name.to_string(),
            width: size,
            height: size,
            format: TextureFormat::RGBA8,
            pixels,
            mip_levels: 1,
        }
    }

    /// Çim/doku texture'sı üret (prosedürel)
    pub fn grass(name: &str, size: u32) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        for y in 0..size {
            for x in 0..size {
                let mut h = DefaultHasher::new();
                (x, y).hash(&mut h);
                let noise = (h.finish() % 100) as f32 / 100.0;
                let green = (0.3 + noise * 0.4) as f32;
                let r = (10.0 + noise * 30.0) as u8;
                let g = (80.0 + green * 120.0) as u8;
                let b = (10.0 + noise * 20.0) as u8;
                pixels.extend_from_slice(&[r, g, b, 255]);
            }
        }
        Self {
            name: name.to_string(),
            width: size,
            height: size,
            format: TextureFormat::RGBA8,
            pixels,
            mip_levels: 1,
        }
    }

    /// Kaynak pixel sayısını döndür
    #[inline]
    fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }

    /// RGBA offset hesapla
    #[inline]
    fn rgba_offset(&self, x: u32, y: u32) -> usize {
        ((y * self.width + x) * 4) as usize
    }

    /// Tek piksel oku (RGBA)
    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x >= self.width || y >= self.height {
            return [128, 128, 128, 255];
        }
        let offset = self.rgba_offset(x, y);
        if offset + 3 < self.pixels.len() {
            [self.pixels[offset], self.pixels[offset + 1], self.pixels[offset + 2], self.pixels[offset + 3]]
        } else {
            [128, 128, 128, 255]
        }
    }

    /// Normalized UV ile texture örnekle (bilinear filtreli)
    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> [u8; 4] {
        let u = u.fract();
        let v = v.fract();
        let u = if u < 0.0 { u + 1.0 } else { u };
        let v = if v < 0.0 { v + 1.0 } else { v };

        let w = self.width as f32;
        let h = self.height as f32;

        let px = u * w;
        let py = v * h;

        let x0 = (px as u32) % self.width;
        let y0 = (py as u32) % self.height;
        let x1 = (x0 + 1) % self.width;
        let y1 = (y0 + 1) % self.height;

        let fx = px - px.floor();
        let fy = py - py.floor();

        let c00 = self.get_pixel(x0, y0);
        let c10 = self.get_pixel(x1, y0);
        let c01 = self.get_pixel(x0, y1);
        let c11 = self.get_pixel(x1, y1);

        let lerp = |a: u8, b: u8, t: f32| -> u8 {
            (a as f32 * (1.0 - t) + b as f32 * t) as u8
        };

        let r = lerp(lerp(c00[0], c10[0], fx), lerp(c01[0], c11[0], fx), fy);
        let g = lerp(lerp(c00[1], c10[1], fx), lerp(c01[1], c11[1], fx), fy);
        let b = lerp(lerp(c00[2], c10[2], fx), lerp(c01[2], c11[2], fx), fy);
        let a = lerp(lerp(c00[3], c10[3], fx), lerp(c01[3], c11[3], fx), fy);

        [r, g, b, a]
    }

    /// Normalized UV ile Vec3 (renk) örnekle [0.0, 1.0] aralığında
    #[inline]
    pub fn sample_color(&self, u: f32, v: f32) -> Vec3 {
        let [r, g, b, _] = self.sample(u, v);
        Vec3::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
    }

    /// Mip seviyesi oluştur (basit box filter)
    pub fn generate_mipmaps(&mut self) {
        let mut current_w = self.width;
        let mut current_h = self.height;
        let mut mip = 1u8;

        while current_w > 1 || current_h > 1 {
            let new_w = (current_w / 2).max(1);
            let new_h = (current_h / 2).max(1);

            let mut new_pixels = Vec::with_capacity((new_w * new_h * 4) as usize);
            for y in 0..new_h {
                for x in 0..new_w {
                    // 2x2 box filter
                    let sx = x * 2;
                    let sy = y * 2;
                    let c00 = self.get_pixel(sx, sy);
                    let c10 = self.get_pixel((sx + 1).min(current_w - 1), sy);
                    let c01 = self.get_pixel(sx, (sy + 1).min(current_h - 1));
                    let c11 = self.get_pixel((sx + 1).min(current_w - 1), (sy + 1).min(current_h - 1));

                    for ch in 0..4 {
                        let v = (c00[ch] as u16 + c10[ch] as u16 + c01[ch] as u16 + c11[ch] as u16) / 4;
                        new_pixels.push(v as u8);
                    }
                }
            }

            self.pixels = new_pixels;
            self.width = new_w;
            self.height = new_h;
            mip += 1;
            current_w = new_w;
            current_h = new_h;
        }
        self.mip_levels = mip;
    }
}

// ═══════════════════════════════════════════════════════════ Texture Import

/// PNG dosyasından texture yükle
pub fn load_png(path: &str) -> Result<Texture, String> {
    let data = std::fs::read(path)
        .map_err(|e| format!("Dosya okunamadı '{}': {}", path, e))?;
    load_png_bytes(&data, path)
}

/// PNG byte dizisinden texture yükle
pub fn load_png_bytes(data: &[u8], name: &str) -> Result<Texture, String> {
    let img = image::load_from_memory(data)
        .map_err(|e| format!("PNG decode hatası: {}", e))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let pixels = rgba.into_raw();

    let mut tex = Texture {
        name: name.to_string(),
        width: w,
        height: h,
        format: TextureFormat::RGBA8,
        pixels,
        mip_levels: 1,
    };
    tex.generate_mipmaps();
    Ok(tex)
}

/// JPEG dosyasından texture yükle
pub fn load_jpg(path: &str) -> Result<Texture, String> {
    let data = std::fs::read(path)
        .map_err(|e| format!("Dosya okunamadı '{}': {}", path, e))?;
    load_jpg_bytes(&data, path)
}

/// JPEG byte dizisinden texture yükle
pub fn load_jpg_bytes(data: &[u8], name: &str) -> Result<Texture, String> {
    let img = image::load_from_memory(data)
        .map_err(|e| format!("JPEG decode hatası: {}", e))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let pixels = rgba.into_raw();

    let mut tex = Texture {
        name: name.to_string(),
        width: w,
        height: h,
        format: TextureFormat::RGBA8,
        pixels,
        mip_levels: 1,
    };
    tex.generate_mipmaps();
    Ok(tex)
}

/// Desteklenen formatları otomatik algıla ve yükle
pub fn load_texture(path: &str) -> Result<Texture, String> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "png" => load_png(path),
        "jpg" | "jpeg" => load_jpg(path),
        "bmp" => {
            let data = std::fs::read(path)
                .map_err(|e| format!("Dosya okunamadı '{}': {}", path, e))?;
            let img = image::load_from_memory(&data)
                .map_err(|e| format!("BMP decode hatası: {}", e))?;
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let mut tex = Texture {
                name: path.to_string(),
                width: w, height: h,
                format: TextureFormat::RGBA8,
                pixels: rgba.into_raw(),
                mip_levels: 1,
            };
            tex.generate_mipmaps();
            Ok(tex)
        }
        "webp" => {
            let data = std::fs::read(path)
                .map_err(|e| format!("Dosya okunamadı '{}': {}", path, e))?;
            let img = image::load_from_memory(&data)
                .map_err(|e| format!("WebP decode hatası: {}", e))?;
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let mut tex = Texture {
                name: path.to_string(),
                width: w, height: h,
                format: TextureFormat::RGBA8,
                pixels: rgba.into_raw(),
                mip_levels: 1,
            };
            tex.generate_mipmaps();
            Ok(tex)
        }
        _ => Err(format!("Desteklenmeyen texture formatı: '{}'", ext)),
    }
}

/// Byte dizisinden otomatik format algıla
pub fn load_texture_from_bytes(data: &[u8], name: &str) -> Result<Texture, String> {
    // Format magic bytes kontrolü
    if data.len() >= 8 {
        // PNG magic: 89 50 4E 47 0D 0A 1A 0A
        if data[0] == 0x89 && data[1] == 0x50 && data[2] == 0x4E && data[3] == 0x47 {
            return load_png_bytes(data, name);
        }
        // JPEG magic: FF D8 FF
        if data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
            return load_jpg_bytes(data, name);
        }
    }

    // image crate'e bırak (otomatik algılama)
    let img = image::load_from_memory(data)
        .map_err(|e| format!("Texture decode hatası: {}", e))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut tex = Texture {
        name: name.to_string(),
        width: w, height: h,
        format: TextureFormat::RGBA8,
        pixels: rgba.into_raw(),
        mip_levels: 1,
    };
    tex.generate_mipmaps();
    Ok(tex)
}

// ═══════════════════════════════════════════════════════════ Texture Set

/// Material'a texture atamaları
#[derive(Clone, Debug)]
pub struct TextureSet {
    pub albedo: Option<u32>,    // Texture ID
    pub normal: Option<u32>,
    pub roughness: Option<u32>,
    pub metallic: Option<u32>,
    pub ao: Option<u32>,
    pub emissive: Option<u32>,
}

impl Default for TextureSet {
    fn default() -> Self {
        Self {
            albedo: None,
            normal: None,
            roughness: None,
            metallic: None,
            ao: None,
            emissive: None,
        }
    }
}

impl TextureSet {
    /// Hiç texture atanmamış mı?
    pub fn is_empty(&self) -> bool {
        self.albedo.is_none() && self.normal.is_none() && self.roughness.is_none()
            && self.metallic.is_none() && self.ao.is_none() && self.emissive.is_none()
    }

    /// Aktif texture sayısını döndür
    pub fn active_count(&self) -> usize {
        let mut count = 0;
        if self.albedo.is_some() { count += 1; }
        if self.normal.is_some() { count += 1; }
        if self.roughness.is_some() { count += 1; }
        if self.metallic.is_some() { count += 1; }
        if self.ao.is_some() { count += 1; }
        if self.emissive.is_some() { count += 1; }
        count
    }
}

/// Texture slot tanımı (material'daki texture bağlantı noktası)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextureSlot {
    Albedo,
    Normal,
    Roughness,
    Metallic,
    Ao,
    Emissive,
}

impl TextureSlot {
    pub fn name(&self) -> &str {
        match self {
            Self::Albedo => "Albedo",
            Self::Normal => "Normal",
            Self::Roughness => "Roughness",
            Self::Metallic => "Metallic",
            Self::Ao => "AO",
            Self::Emissive => "Emissive",
        }
    }

    pub fn all() -> &'static [TextureSlot] {
        &[Self::Albedo, Self::Normal, Self::Roughness, Self::Metallic, Self::Ao, Self::Emissive]
    }
}

// ═══════════════════════════════════════════════════════════ Normal Map

/// Normal map'ten world-space normal oku
pub fn sample_normal_map(
    texture: &Texture,
    u: f32,
    v: f32,
    tangent: Vec3,
    bitangent: Vec3,
    normal: Vec3,
) -> Vec3 {
    let [r, g, b, _] = texture.sample(u, v);
    // Tangent space normal [0,1] → [-1,1]
    let t_normal = Vec3::new(
        (r as f32 / 255.0) * 2.0 - 1.0,
        (g as f32 / 255.0) * 2.0 - 1.0,
        (b as f32 / 255.0) * 2.0 - 1.0,
    );
    // TBN dönüşümü
    (tangent * t_normal.x + bitangent * t_normal.y + normal * t_normal.z).normalize()
}

// ═══════════════════════════════════════════════════════════ Testler

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_solid_color() {
        let tex = Texture::solid_color("red", 255, 0, 0, 255);
        assert_eq!(tex.width, 1);
        assert_eq!(tex.height, 1);
        let px = tex.get_pixel(0, 0);
        assert_eq!(px, [255, 0, 0, 255]);
    }

    #[test]
    fn test_texture_checkerboard() {
        let tex = Texture::checkerboard("checker", 8, 4, [255, 255, 255], [0, 0, 0]);
        assert_eq!(tex.width, 8);
        assert_eq!(tex.height, 8);
        // (0,0) beyaz olmalı
        let px = tex.get_pixel(0, 0);
        assert_eq!(px[0], 255);
        // (4,0) siyah olmalı
        let px2 = tex.get_pixel(4, 0);
        assert_eq!(px2[0], 0);
    }

    #[test]
    fn test_texture_sampling() {
        // 8x8 texture: sol üst kırmızı blok, sağ üst yeşil blok
        let mut tex = Texture::new("test", 8, 8);
        for y in 0..4 {
            for x in 0..4 {
                let off = (y * 8 + x) as usize * 4;
                tex.pixels[off] = 255; tex.pixels[off+1] = 0; tex.pixels[off+2] = 0; tex.pixels[off+3] = 255;
            }
        }
        for y in 0..4 {
            for x in 4..8 {
                let off = (y * 8 + x) as usize * 4;
                tex.pixels[off] = 0; tex.pixels[off+1] = 255; tex.pixels[off+2] = 0; tex.pixels[off+3] = 255;
            }
        }

        // Kırmızı bloğun merkezinde sample → saf kırmızı
        let c = tex.sample(0.25, 0.25);
        assert_eq!(c[0], 255);
        assert_eq!(c[1], 0);

        // Yeşil bloğun merkezinde sample → saf yeşil
        let c = tex.sample(0.75, 0.25);
        assert_eq!(c[0], 0);
        assert_eq!(c[1], 255);
    }

    #[test]
    fn test_texture_sample_color() {
        let tex = Texture::solid_color("blue", 0, 0, 255, 255);
        let c = tex.sample_color(0.5, 0.5);
        assert!((c.z - 1.0).abs() < 0.01);
        assert!((c.x).abs() < 0.01);
    }

    #[test]
    fn test_texture_out_of_bounds() {
        let tex = Texture::new("test", 4, 4);
        let px = tex.get_pixel(100, 100);
        assert_eq!(px, [128, 128, 128, 255]); // default
    }

    #[test]
    fn test_texture_uv_wrapping() {
        let tex = Texture::new("test", 4, 4);
        // UV wrap-around
        let c1 = tex.sample(1.5, 0.5);
        let c2 = tex.sample(0.5, 0.5);
        // İkisi de aynı pikseli örneklemeli (mod 1.0 → 0.5)
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_texture_mipmaps() {
        let mut tex = Texture::checkerboard("mip", 64, 8, [200, 200, 200], [50, 50, 50]);
        tex.generate_mipmaps();
        assert!(tex.mip_levels > 1);
        // Son mip 1x1 olmalı
        assert_eq!(tex.width, 1);
        assert_eq!(tex.height, 1);
    }

    #[test]
    fn test_texture_set() {
        let mut ts = TextureSet::default();
        assert!(ts.is_empty());
        assert_eq!(ts.active_count(), 0);

        ts.albedo = Some(1);
        assert!(!ts.is_empty());
        assert_eq!(ts.active_count(), 1);

        ts.normal = Some(2);
        ts.roughness = Some(3);
        assert_eq!(ts.active_count(), 3);
    }

    #[test]
    fn test_texture_slot_names() {
        assert_eq!(TextureSlot::Albedo.name(), "Albedo");
        assert_eq!(TextureSlot::Normal.name(), "Normal");
        assert_eq!(TextureSlot::Roughness.name(), "Roughness");
        assert_eq!(TextureSlot::Metallic.name(), "Metallic");
        assert_eq!(TextureSlot::Ao.name(), "AO");
        assert_eq!(TextureSlot::Emissive.name(), "Emissive");
        assert_eq!(TextureSlot::all().len(), 6);
    }

    #[test]
    fn test_grass_texture() {
        let tex = Texture::grass("lawn", 16);
        assert_eq!(tex.width, 16);
        assert_eq!(tex.height, 16);
        let px = tex.get_pixel(0, 0);
        // Grass should be green-ish
        assert!(px[1] > px[0]); // green > red
        assert!(px[1] > px[2]); // green > blue
    }

    #[test]
    fn test_normal_map_sampling() {
        let mut tex = Texture::new("nrm", 2, 2);
        // Flat normal: (0,0,1) → encoded as (128, 128, 255)
        tex.pixels[0] = 128; tex.pixels[1] = 128; tex.pixels[2] = 255; tex.pixels[3] = 255;

        let tangent = Vec3::X;
        let bitangent = Vec3::Y;
        let normal = Vec3::Z;

        let result = sample_normal_map(&tex, 0.1, 0.1, tangent, bitangent, normal);
        // Flat normal should be close to Z
        assert!(result.z > 0.9);
    }
}
