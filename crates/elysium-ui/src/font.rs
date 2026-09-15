//! Font sistemi — fontdue ile metin rasterleme.

use std::collections::HashMap;

/// Font yükleme ve glyph önbellekleme
pub struct FontSystem {
    font: Option<fontdue::Font>,
    cache: HashMap<(char, u16), (fontdue::Metrics, Vec<u8>)>,
    line_height: f32,
}

impl Default for FontSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl FontSystem {
    pub fn new() -> Self {
        let font = Self::load_default_font();
        Self {
            font,
            cache: HashMap::new(),
            line_height: 14.0,
        }
    }

    fn load_default_font() -> Option<fontdue::Font> {
        let paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
            "C:\\Windows\\Fonts\\arial.ttf",
        ];
        for path in &paths {
            if let Ok(data) = std::fs::read(path) {
                if let Ok(font) = fontdue::Font::from_bytes(
                    data,
                    fontdue::FontSettings { scale: 40.0, collection_index: 0, load_substitutions: false },
                ) {
                    return Some(font);
                }
            }
        }
        None
    }

    pub fn glyph(&mut self, ch: char, px: u16) -> Option<(&fontdue::Metrics, &[u8])> {
        if self.font.is_none() {
            return None;
        }
        if !self.cache.contains_key(&(ch, px)) {
            let font = self.font.as_ref().unwrap();
            let g = font.rasterize(ch, px as f32);
            self.cache.insert((ch, px), g);
        }
        self.cache.get(&(ch, px)).map(|(m, b)| (m, b.as_slice()))
    }

    pub fn measure(&mut self, px: u16, text: &str) -> f32 {
        let mut w = 0.0f32;
        for ch in text.chars() {
            if let Some((metrics, _)) = self.glyph(ch, px) {
                w += metrics.advance_width;
            }
        }
        w
    }

    pub fn line_height_px(&self, px: u16) -> f32 {
        self.line_height * px as f32 / 14.0
    }

    pub fn has_font(&self) -> bool {
        self.font.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_system_creation() {
        let fs = FontSystem::new();
        // Font bulunamayabilir, bu test sadece panic etmemeli
        let _ = fs.has_font();
    }
}
