//! Font atlas and text rasterization for UI rendering.

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default)]
pub struct GlyphInfo {
    pub u: u32,
    pub v: u32,
    pub width: u32,
    pub height: u32,
    pub advance: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
}

pub struct FontAtlas {
    pub texture_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub glyphs: HashMap<char, GlyphInfo>,
    pub font_size: f32,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
}

impl FontAtlas {
    pub fn new(font_size: f32) -> Self {
        Self {
            texture_data: Vec::new(),
            width: 2048,
            height: 2048,
            glyphs: HashMap::new(),
            font_size,
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
        }
    }

    pub fn rasterize_glyph(&mut self, font: &mut fontdue::Font, c: char) -> Option<GlyphInfo> {
        let (metrics, bitmap) = font.rasterize(c, self.font_size);

        let gw = metrics.width as u32;
        let gh = metrics.height as u32;

        if gw == 0 || gh == 0 {
            return Some(GlyphInfo {
                u: 0,
                v: 0,
                width: 0,
                height: 0,
                advance: metrics.advance_width,
                bearing_x: metrics.bounds.xmin,
                bearing_y: metrics.bounds.ymin,
            });
        }

        if self.cursor_x + gw > self.width {
            self.cursor_x = 0;
            self.cursor_y += self.row_height;
            self.row_height = 0;
        }

        if self.cursor_y + gh > self.height {
            return None;
        }

        let atlas_x = self.cursor_x;
        let atlas_y = self.cursor_y;

        let old_len = self.texture_data.len();
        let need = (self.height * self.width) as usize;
        if old_len < need {
            self.texture_data.resize(need, 0);
        }

        for dy in 0..gh {
            for dx in 0..gw {
                let src = (dy as usize * gw as usize + dx as usize) as usize;
                let dst = ((atlas_y + dy) * self.width + (atlas_x + dx)) as usize;
                if src < bitmap.len() && dst < self.texture_data.len() {
                    self.texture_data[dst] = bitmap[src];
                }
            }
        }

        self.cursor_x += gw;
        self.row_height = self.row_height.max(gh);

        let info = GlyphInfo {
            u: atlas_x,
            v: atlas_y,
            width: gw,
            height: gh,
            advance: metrics.advance_width,
            bearing_x: metrics.bounds.xmin,
            bearing_y: metrics.bounds.ymin,
        };
        self.glyphs.insert(c, info.clone());
        Some(info)
    }

    pub fn get_or_rasterize(&mut self, font: &mut fontdue::Font, c: char) -> GlyphInfo {
        if let Some(info) = self.glyphs.get(&c).cloned() {
            return info;
        }
        self.rasterize_glyph(font, c).unwrap_or_default()
    }

    pub fn uv_for(&self, glyph: &GlyphInfo) -> (f32, f32, f32, f32) {
        (
            glyph.u as f32 / self.width as f32,
            glyph.v as f32 / self.height as f32,
            (glyph.u + glyph.width) as f32 / self.width as f32,
            (glyph.v + glyph.height) as f32 / self.height as f32,
        )
    }
}

impl Default for FontAtlas {
    fn default() -> Self {
        Self::new(14.0)
    }
}
