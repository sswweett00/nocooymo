//! Çizim modu — UI için temel çizim primitifleri.

/// Renk
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const WHITE: Color = Color::new(255, 255, 255, 255);
    pub const BLACK: Color = Color::new(0, 0, 0, 255);
    pub const RED: Color = Color::new(255, 0, 0, 255);
    pub const GREEN: Color = Color::new(0, 255, 0, 255);
    pub const BLUE: Color = Color::new(0, 0, 255, 255);
    pub const TRANSPARENT: Color = Color::new(0, 0, 0, 0);

    pub fn lerp(&self, other: &Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color {
            r: (self.r as f32 + (other.r as f32 - self.r as f32) * t) as u8,
            g: (self.g as f32 + (other.g as f32 - self.g as f32) * t) as u8,
            b: (self.b as f32 + (other.b as f32 - self.b as f32) * t) as u8,
            a: (self.a as f32 + (other.a as f32 - self.a as f32) * t) as u8,
        }
    }
}

impl From<[u8; 4]> for Color {
    fn from(arr: [u8; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }
}

impl From<Color> for [u8; 4] {
    fn from(c: Color) -> Self {
        [c.r, c.g, c.b, c.a]
    }
}

/// Çizim komutu
#[derive(Clone, Debug)]
pub enum DrawCommand {
    Rect { x: i32, y: i32, w: i32, h: i32, color: Color },
    FilledRect { x: i32, y: i32, w: i32, h: i32, color: Color },
    Border { x: i32, y: i32, w: i32, h: i32, color: Color, thickness: i32 },
    Line { x0: i32, y0: i32, x1: i32, y1: i32, color: Color, thickness: i32 },
    Circle { cx: i32, cy: i32, radius: i32, color: Color, filled: bool },
    Text { x: i32, y: i32, text: String, color: Color, size: u16 },
    Image { x: i32, y: i32, w: i32, h: i32, data: Vec<u8> },
    Gradient { x: i32, y: i32, w: i32, h: i32, top_color: Color, bottom_color: Color },
    ClipRect { x: i32, y: i32, w: i32, h: i32 },
    PopClip,
}

/// Çizim listesi — frame başına biriken tüm komutlar
pub struct DrawList {
    pub commands: Vec<DrawCommand>,
    pub clip_stack: Vec<[i32; 4]>,
}

impl Default for DrawList {
    fn default() -> Self {
        Self::new()
    }
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            clip_stack: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.clip_stack.clear();
    }

    pub fn push_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color) {
        self.commands.push(DrawCommand::FilledRect { x, y, w, h, color });
    }

    pub fn push_border(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color) {
        self.commands.push(DrawCommand::Border { x, y, w, h, color, thickness: 1 });
    }

    pub fn push_text(&mut self, x: i32, y: i32, text: &str, color: Color, size: u16) {
        self.commands.push(DrawCommand::Text { x, y, text: text.to_string(), color, size });
    }

    pub fn push_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        self.commands.push(DrawCommand::Line { x0, y0, x1, y1, color, thickness: 1 });
    }

    pub fn push_circle(&mut self, cx: i32, cy: i32, radius: i32, color: Color, filled: bool) {
        self.commands.push(DrawCommand::Circle { cx, cy, radius, color, filled });
    }

    pub fn push_gradient(&mut self, x: i32, y: i32, w: i32, h: i32, top: Color, bottom: Color) {
        self.commands.push(DrawCommand::Gradient { x, y, w, h, top_color: top, bottom_color: bottom });
    }

    pub fn push_clip(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.clip_stack.push([x, y, w, h]);
        self.commands.push(DrawCommand::ClipRect { x, y, w, h });
    }

    pub fn pop_clip(&mut self) {
        self.clip_stack.pop();
        self.commands.push(DrawCommand::PopClip);
    }

    pub fn command_count(&self) -> usize {
        self.commands.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_list() {
        let mut list = DrawList::new();
        list.push_rect(0, 0, 100, 50, Color::rgb(255, 0, 0));
        list.push_border(0, 0, 100, 50, Color::rgb(0, 255, 0));
        list.push_text(10, 10, "Hello", Color::WHITE, 14);
        assert_eq!(list.command_count(), 3);
    }

    #[test]
    fn test_color_lerp() {
        let c1 = Color::rgb(0, 0, 0);
        let c2 = Color::rgb(255, 255, 255);
        let mid = c1.lerp(&c2, 0.5);
        assert_eq!(mid.r, 127);
    }
}
