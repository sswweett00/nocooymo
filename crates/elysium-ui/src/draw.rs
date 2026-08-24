//! Draw list for UI rendering

use crate::command::{UiCommandList, UiDrawLine, UiDrawQuad, UiDrawText};
use crate::theme::{Theme, Animations};
use crate::{UiRect, widgets::{AnimationType, Direction}};
use glam::Vec2;
use std::time::Duration;

#[derive(Clone, Debug)]
pub enum DrawCmd {
    Rect {
        rect: UiRect,
        color: [f32; 4],
        corner_radius: f32,
        z: f32,
    },
    Text {
        text: String,
        pos: Vec2,
        color: [f32; 4],
        size: f32,
        z: f32,
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: [f32; 4],
        thickness: f32,
        z: f32,
    },
    Quad {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
        color: [f32; 4],
        corner_radius: f32,
        texture_id: Option<u32>,
        z: f32,
    },
    Scissor {
        rect: UiRect,
        z: f32,
    },
    // Yeni animasyon komutları
    AnimatedRect {
        rect: UiRect,
        start_color: [f32; 4],
        end_color: [f32; 4],
        corner_radius: f32,
        z: f32,
        duration: Duration,
        elapsed: Duration,
        animation_type: AnimationType,
    },
    // Yeni efekt komutları
    DropShadow {
        rect: UiRect,
        blur: f32,
        spread: f32,
        color: [f32; 4],
        z: f32,
    },
    GradientRect {
        rect: UiRect,
        start_color: [f32; 4],
        end_color: [f32; 4],
        direction: Direction,
        z: f32,
    },
}

pub struct DrawList {
    commands: Vec<DrawCmd>,
    current_z: f32,
    animation_manager: AnimationManager,
}

pub struct AnimationManager {
    animations: Vec<AnimationState>,
}

pub struct AnimationState {
    pub id: u64,
    pub start_time: std::time::Instant,
    pub duration: Duration,
    pub progress: f32,
    pub completed: bool,
}

impl AnimationManager {
    pub fn new() -> Self {
        Self {
            animations: Vec::new(),
        }
    }

    pub fn start_animation(&mut self, duration: Duration) -> u64 {
        let id = rand::random::<u64>(); // Basit bir ID üretici
        
        self.animations.push(AnimationState {
            id,
            start_time: std::time::Instant::now(),
            duration,
            progress: 0.0,
            completed: false,
        });
        
        id
    }

    pub fn update_animations(&mut self) {
        for anim in self.animations.iter_mut() {
            if !anim.completed {
                let elapsed = anim.start_time.elapsed();
                anim.progress = (elapsed.as_secs_f32() / anim.duration.as_secs_f32()).min(1.0);
                
                if elapsed >= anim.duration {
                    anim.completed = true;
                    anim.progress = 1.0;
                }
            }
        }
        
        // Tamamlanan animasyonları temizle
        self.animations.retain(|anim| !anim.completed || anim.progress < 1.0);
    }

    pub fn get_animation_progress(&self, id: u64) -> Option<f32> {
        self.animations.iter()
            .find(|anim| anim.id == id)
            .map(|anim| anim.progress)
    }
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            current_z: 0.0,
            animation_manager: AnimationManager::new(),
        }
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.current_z = 0.0;
        self.animation_manager.update_animations();
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Set the Z value for subsequent draw commands.
    pub fn set_z(&mut self, z: f32) {
        self.current_z = z;
    }

    /// Add a scissor rectangle with current Z.
    pub fn add_scissor(&mut self, rect: UiRect) {
        self.commands.push(DrawCmd::Scissor { rect, z: self.current_z });
    }

    pub fn add_rect(&mut self, rect: UiRect, color: [f32; 4], corner_radius: f32) {
        self.commands.push(DrawCmd::Rect {
            rect,
            color,
            corner_radius,
            z: self.current_z,
        });
    }

    pub fn add_rect_with_theme_color(&mut self, rect: UiRect, theme_color_getter: fn(&Theme) -> gpui::Hsla, corner_radius: f32) {
        // gpui::Hsla'dan [f32; 4]'e dönüşüm
        // Bu sadece placeholder - gerçek implementasyon gpui ile uyumlu olacak şekilde yapılmalı
        let color = [0.5, 0.5, 0.5, 1.0]; // placeholder
        self.add_rect(rect, color, corner_radius);
    }

    pub fn add_text(&mut self, text: String, pos: Vec2, color: [f32; 4], size: f32) {
        self.commands.push(DrawCmd::Text {
            text,
            pos,
            color,
            size,
            z: self.current_z,
        });
    }

    pub fn add_line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: [f32; 4],
        thickness: f32,
    ) {
        self.commands.push(DrawCmd::Line {
            x1,
            y1,
            x2,
            y2,
            color,
            thickness,
            z: self.current_z,
        });
    }

    pub fn add_quad(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
        color: [f32; 4],
        corner_radius: f32,
        texture_id: Option<u32>,
    ) {
        self.commands.push(DrawCmd::Quad {
            x,
            y,
            width,
            height,
            u0,
            v0,
            u1,
            v1,
            color,
            corner_radius,
            texture_id,
            z: self.current_z,
        });
    }

    pub fn add_filled_rect(&mut self, rect: UiRect, color: [f32; 4]) {
        self.add_rect(rect, color, 0.0);
    }

    pub fn add_border_rect(&mut self, rect: UiRect, color: [f32; 4], thickness: f32) {
        // Sol kenar
        self.add_filled_rect(
            UiRect::new(rect.x, rect.y, thickness, rect.height),
            color,
        );
        // Sağ kenar
        self.add_filled_rect(
            UiRect::new(
                rect.x + rect.width - thickness,
                rect.y,
                thickness,
                rect.height,
            ),
            color,
        );
        // Üst kenar
        self.add_filled_rect(
            UiRect::new(rect.x, rect.y, rect.width, thickness),
            color,
        );
        // Alt kenar
        self.add_filled_rect(
            UiRect::new(
                rect.x,
                rect.y + rect.height - thickness,
                rect.width,
                thickness,
            ),
            color,
        );
    }

    // Yeni animasyonlu çizim fonksiyonları
    pub fn add_animated_rect(
        &mut self,
        rect: UiRect,
        start_color: [f32; 4],
        end_color: [f32; 4],
        corner_radius: f32,
        duration: Duration,
        animation_type: AnimationType,
    ) {
        self.commands.push(DrawCmd::AnimatedRect {
            rect,
            start_color,
            end_color,
            corner_radius,
            z: self.current_z,
            duration,
            elapsed: Duration::from_secs(0),
            animation_type,
        });
    }

    pub fn add_drop_shadow(&mut self, rect: UiRect, blur: f32, spread: f32, color: [f32; 4]) {
        self.commands.push(DrawCmd::DropShadow {
            rect,
            blur,
            spread,
            color,
            z: self.current_z - 0.1, // Gölge daha arkada olmalı
        });
    }

    pub fn add_gradient_rect(&mut self, rect: UiRect, start_color: [f32; 4], end_color: [f32; 4], direction: Direction) {
        self.commands.push(DrawCmd::GradientRect {
            rect,
            start_color,
            end_color,
            direction,
            z: self.current_z,
        });
    }

    pub fn commands(&self) -> &[DrawCmd] {
        &self.commands
    }

    pub fn into_commands(self) -> Vec<DrawCmd> {
        self.commands
    }

    // Animasyon yönetimi
    pub fn start_animation(&mut self, duration: Duration) -> u64 {
        self.animation_manager.start_animation(duration)
    }

    pub fn update_animations(&mut self) {
        self.animation_manager.update_animations();
    }

    pub fn get_animation_progress(&self, id: u64) -> Option<f32> {
        self.animation_manager.get_animation_progress(id)
    }
}

impl Default for DrawList {
    fn default() -> Self {
        Self::new()
    }
}

// Renk yardımcı fonksiyonları
pub fn lerp_color(start: [f32; 4], end: [f32; 4], t: f32) -> [f32; 4] {
    [
        start[0] + (end[0] - start[0]) * t,
        start[1] + (end[1] - start[1]) * t,
        start[2] + (end[2] - start[2]) * t,
        start[3] + (end[3] - start[3]) * t,
    ]
}

// Animasyon yardımcı fonksiyonları
pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let f = 2.0 * t - 2.0;
        0.5 * f * f * f + 1.0
    }
}

pub fn ease_bounce(t: f32) -> f32 {
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;
    
    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t = t - 1.5 / D1;
        N1 * t * t + 0.75
    } else if t < 2.5 / D1 {
        let t = t - 2.25 / D1;
        N1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / D1;
        N1 * t * t + 0.984375
    }
}

// Geometrik yardımcı fonksiyonlar
pub fn rotate_rect(rect: UiRect, center: Vec2, angle_radians: f32) -> UiRect {
    // Basit döndürme işlemi - köşe noktalarını döndürüp yeni sınırları hesaplar
    let cos = angle_radians.cos();
    let sin = angle_radians.sin();
    
    // Köşe noktalarını merkeze göre döndür
    let corners = [
        Vec2::new(rect.x, rect.y),
        Vec2::new(rect.x + rect.width, rect.y),
        Vec2::new(rect.x, rect.y + rect.height),
        Vec2::new(rect.x + rect.width, rect.y + rect.height),
    ];
    
    let rotated_corners: Vec<Vec2> = corners.iter()
        .map(|&corner| {
            let translated = corner - center;
            Vec2::new(
                translated.x * cos - translated.y * sin,
                translated.x * sin + translated.y * cos,
            ) + center
        })
        .collect();
    
    // Yeni sınırları bul
    let min_x = rotated_corners.iter().map(|v| v.x).fold(f32::INFINITY, |a, b| a.min(b));
    let max_x = rotated_corners.iter().map(|v| v.x).fold(f32::NEG_INFINITY, |a, b| a.max(b));
    let min_y = rotated_corners.iter().map(|v| v.y).fold(f32::INFINITY, |a, b| a.min(b));
    let max_y = rotated_corners.iter().map(|v| v.y).fold(f32::NEG_INFINITY, |a, b| a.max(b));
    
    UiRect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}