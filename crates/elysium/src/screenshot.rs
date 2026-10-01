//! screenshot.rs — Ekran görüntüsü yakalama (framebuffer → PNG)
//!
//! Software renderer'ın RGBA framebuffer'ını alır ve disk'e PNG olarak
//! kaydeder. Zaman damgalı dosya adı üretir; yalnızca viewport bölgesi
//! veya tam ekran yakalanabilir.

use std::path::Path;

use crate::renderer::SoftwareRenderer;

/// Yakalanan görüntü (ham RGBA)
#[derive(Clone, Debug)]
pub struct CapturedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Kayıt sonucu
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureInfo {
    pub width: u32,
    pub height: u32,
    /// Dosyaya yazılan byte sayısı
    pub bytes: usize,
}

impl CaptureInfo {
    pub fn label(&self) -> String {
        format!("{}x{} ({} KB)", self.width, self.height, self.bytes / 1024)
    }
}

/// Yalnızca viewport (3D) bölgesi mi, yoksa tam ekran mı
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureRegion {
    Viewport,
    FullScreen,
}

/// Zaman damgalı dosya adı üretir (saniye hassasiyeti)
pub fn screenshot_filename(prefix: &str) -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{prefix}_{:010}.png", secs)
}

/// Renderer'ın belirtilen bölgesini RGBA vektörü olarak kopyalar.
/// Veri [r,g,b,a] sıralı, satır satır soldan sağa dizilidir.
pub fn capture(renderer: &SoftwareRenderer, region: CaptureRegion) -> Result<CapturedImage, String> {
    let (ox, oy) = match region {
        CaptureRegion::FullScreen => (0i32, 0i32),
        CaptureRegion::Viewport => renderer.viewport_offset,
    };
    let (vw, vh) = match region {
        CaptureRegion::FullScreen => (renderer.width, renderer.height),
        CaptureRegion::Viewport => renderer.viewport_size,
    };
    if vw == 0 || vh == 0 {
        return Err("boş viewport".into());
    }
    let x0 = ox.max(0) as u32;
    let y0 = oy.max(0) as u32;
    let x1 = (ox + vw as i32).max(0).min(renderer.width as i32) as u32;
    let y1 = (oy + vh as i32).max(0).min(renderer.height as i32) as u32;
    let w = x1.saturating_sub(x0);
    let h = y1.saturating_sub(y0);
    if w == 0 || h == 0 {
        return Err("viewport ekran dışına taşmış".into());
    }

    let fb = renderer.frame_buffer();
    let src_row = renderer.width as usize;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in y0..y1 {
        let base = (y as usize * src_row + x0 as usize) * 4;
        rgba.extend_from_slice(&fb[base..base + (w as usize * 4)]);
    }
    Ok(CapturedImage { width: w, height: h, rgba })
}

/// RGBA buffer'ı PNG olarak diske yazar; boyutu döndürür.
pub fn save_png(path: impl AsRef<Path>, image: &CapturedImage) -> Result<CaptureInfo, String> {
    let img = image::RgbaImage::from_raw(image.width, image.height, image.rgba.clone())
        .ok_or_else(|| "geçersiz RGBA boyut".to_string())?;
    img.save(path.as_ref())
        .map_err(|e| format!("PNG yazılamadı: {e}"))?;
    let bytes = std::fs::metadata(path.as_ref()).map(|m| m.len() as usize).unwrap_or(0);
    Ok(CaptureInfo { width: image.width, height: image.height, bytes })
}

/// Yakala + kaydet (tek çağrı). Zaman damgalı dosya adı üretir ve döner.
pub fn capture_and_save(
    renderer: &SoftwareRenderer,
    region: CaptureRegion,
    prefix: &str,
) -> Result<(String, CaptureInfo), String> {
    let img = capture(renderer, region)?;
    let name = screenshot_filename(prefix);
    let info = save_png(&name, &img)?;
    Ok((name, info))
}

// ═══════════════════════════════════════════════════════════ Testler

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filename_has_timestamp() {
        let a = screenshot_filename("shot");
        assert!(a.ends_with(".png"));
        assert!(a.starts_with("shot_"));
        // "shot_" + 10 hane + ".png" = 19 karakter
        assert_eq!(a.len(), 5 + 10 + 4);
    }

    #[test]
    fn test_capture_fullscreen() {
        let r = SoftwareRenderer::new(32, 24);
        let img = capture(&r, CaptureRegion::FullScreen).unwrap();
        assert_eq!(img.width, 32);
        assert_eq!(img.height, 24);
        assert_eq!(img.rgba.len(), 32 * 24 * 4);
    }

    #[test]
    fn test_capture_viewport_clamped() {
        let mut r = SoftwareRenderer::new(100, 100);
        r.viewport_offset = (10, 20);
        r.viewport_size = (50, 40);
        let img = capture(&r, CaptureRegion::Viewport).unwrap();
        assert_eq!(img.width, 50);
        assert_eq!(img.height, 40);
    }

    #[test]
    fn test_capture_viewport_out_of_bounds() {
        let mut r = SoftwareRenderer::new(100, 100);
        r.viewport_offset = (-50, -50);
        r.viewport_size = (10, 10);
        assert!(capture(&r, CaptureRegion::Viewport).is_err());
    }

    #[test]
    fn test_capture_viewport_partial_overlap() {
        let mut r = SoftwareRenderer::new(100, 100);
        r.viewport_offset = (90, 90);
        r.viewport_size = (30, 30); // ekranın 10x10'u görünür
        let img = capture(&r, CaptureRegion::Viewport).unwrap();
        assert_eq!(img.width, 10);
        assert_eq!(img.height, 10);
    }

    #[test]
    fn test_save_png_roundtrip() {
        let w = 8u32;
        let h = 4u32;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for i in 0..(w * h) as usize {
            rgba[i * 4] = (i % 256) as u8;
            rgba[i * 4 + 1] = 128;
            rgba[i * 4 + 2] = 0;
            rgba[i * 4 + 3] = 255;
        }
        let img = CapturedImage { width: w, height: h, rgba };
        let path = std::env::temp_dir().join("elysium_screenshot_test.png");
        let info = save_png(&path, &img).unwrap();
        assert_eq!(info.width, w);
        assert_eq!(info.height, h);
        assert!(info.bytes > 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_capture_and_save_viewport() {
        let mut r = SoftwareRenderer::new(64, 48);
        r.viewport_offset = (8, 8);
        r.viewport_size = (32, 32);
        let (name, info) = capture_and_save(&r, CaptureRegion::Viewport, "test").unwrap();
        assert!(name.starts_with("test_"));
        assert_eq!(info.width, 32);
        assert_eq!(info.height, 32);
        assert!(info.bytes > 0);
        let _ = std::fs::remove_file(&name);
    }
}
