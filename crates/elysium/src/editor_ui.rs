//! editor_ui.rs — Anında mod (immediate-mode) 2D arayüz katmanı.
//! Software framebuffer üzerine düz dikdörtgenler ve fontdue ile
//! rasterlenmiş metinler çizer. Harici GUI crate'i gerektirmez.

use std::collections::HashMap;

use crate::renderer::SoftwareRenderer;

/// Sistem fontunu yükler (birkaç yaygın yol denenir).
pub struct UiFont {
    font: fontdue::Font,
    cache: HashMap<(char, u16), (fontdue::Metrics, Vec<u8>)>,
}

const FONT_PATHS: &[&str] = &[
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
];

impl UiFont {
    pub fn load() -> Option<Self> {
        for path in FONT_PATHS {
            if let Ok(data) = std::fs::read(path) {
                if let Ok(font) = fontdue::Font::from_bytes(
                    data,
                    fontdue::FontSettings { scale: 40.0, collection_index: 0, load_substitutions: false },
                ) {
                    return Some(Self { font, cache: HashMap::new() });
                }
            }
        }
        None
    }

    fn glyph(&mut self, ch: char, px: u16) -> (fontdue::Metrics, Vec<u8>) {
        if let Some(g) = self.cache.get(&(ch, px)) {
            return g.clone();
        }
        let g = self.font.rasterize(ch, px as f32);
        self.cache.insert((ch, px), g.clone());
        g
    }

    /// Metni çizer; genişliğini döndürür.
    pub fn draw_text(
        &mut self,
        fb: &mut SoftwareRenderer,
        x: i32,
        y: i32,
        px: u16,
        color: [u8; 4],
        text: &str,
    ) -> f32 {
        let mut cursor_x = x as f32;
        for ch in text.chars() {
            let (metrics, bitmap) = self.glyph(ch, px);
            fb.blit_glyph(
                cursor_x as i32 + metrics.xmin,
                y + metrics.ymin,
                metrics.width,
                metrics.height,
                &bitmap,
                color,
            );
            cursor_x += metrics.advance_width;
        }
        cursor_x - x as f32
    }

    /// Metnin piksel genişliği.
    pub fn measure(&mut self, px: u16, text: &str) -> f32 {
        let mut w = 0.0f32;
        for ch in text.chars() {
            let (metrics, _) = self.glyph(ch, px);
            w += metrics.advance_width;
        }
        w
    }
}

// ─────────────────────────────────────────────────────────── Palet

pub const COL_PANEL: [u8; 4] = [37, 37, 38, 255];
pub const COL_HEADER: [u8; 4] = [51, 51, 51, 255];
pub const COL_SELECT: [u8; 4] = [9, 71, 113, 255];
pub const COL_HOVER: [u8; 4] = [42, 45, 46, 255];
pub const COL_BUTTON: [u8; 4] = [55, 55, 58, 255];
pub const COL_BUTTON_PLAY: [u8; 4] = [36, 92, 55, 255];
pub const COL_BUTTON_PAUSE: [u8; 4] = [110, 82, 30, 255];
pub const COL_BORDER: [u8; 4] = [60, 60, 64, 255];
pub const COL_TEXT: [u8; 4] = [240, 240, 240, 255];
pub const COL_TEXT_DIM: [u8; 4] = [150, 150, 150, 255];
pub const COL_TEXT_GREEN: [u8; 4] = [120, 220, 140, 255];

/// Bir UI butonu (hit-test için saklanır).
#[derive(Clone)]
pub struct UiButton {
    pub id: String,
    pub rect: [i32; 4],
}

/// Inspector'daki sürükle-bir-değer hedefi.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EditComp {
    Position,
    Rotation,
    Scale,
}

#[derive(Clone, Copy)]
pub struct UiDragHandle {
    pub comp: EditComp,
    pub axis: usize,
    pub rect: [i32; 4],
}

/// Her frame yeniden inşa edilen düzen bilgisi.
#[derive(Default)]
pub struct UiLayout {
    pub buttons: Vec<UiButton>,
    pub drags: Vec<UiDragHandle>,
}

impl UiLayout {
    pub fn button_at(&self, x: f64, y: f64) -> Option<String> {
        let (x, y) = (x as i32, y as i32);
        self.buttons
            .iter()
            .find(|b| x >= b.rect[0] && x < b.rect[0] + b.rect[2] && y >= b.rect[1] && y < b.rect[1] + b.rect[3])
            .map(|b| b.id.clone())
    }

    pub fn drag_at(&self, x: f64, y: f64) -> Option<UiDragHandle> {
        let (x, y) = (x as i32, y as i32);
        self.drags
            .iter()
            .copied()
            .find(|d| x >= d.rect[0] && x < d.rect[0] + d.rect[2] && y >= d.rect[1] && y < d.rect[1] + d.rect[3])
    }
}

// ─────────────────────────────────────────────────────────── Layout

pub struct Layout {
    pub top_bar: [i32; 4],
    pub bottom_bar: [i32; 4],
    pub left_panel: [i32; 4],
    pub right_panel: [i32; 4],
    pub viewport: [i32; 4],
}

pub const TOP_BAR_H: i32 = 34;
pub const BOTTOM_BAR_H: i32 = 26;
pub const LEFT_W: i32 = 260;
pub const RIGHT_W: i32 = 230;

pub fn compute_layout(w: i32, h: i32) -> Layout {
    let vw = w - LEFT_W - RIGHT_W;
    let vh = h - TOP_BAR_H - BOTTOM_BAR_H;
    Layout {
        top_bar: [0, 0, w, TOP_BAR_H],
        bottom_bar: [0, h - BOTTOM_BAR_H, w, BOTTOM_BAR_H],
        left_panel: [0, TOP_BAR_H, LEFT_W, vh],
        right_panel: [w - RIGHT_W, TOP_BAR_H, RIGHT_W, vh],
        viewport: [LEFT_W, TOP_BAR_H, vw.max(1), vh.max(1)],
    }
}

fn rect(fb: &mut SoftwareRenderer, r: [i32; 4], c: [u8; 4]) {
    fb.ui_rect(r[0], r[1], r[2], r[3], c);
}

fn border(fb: &mut SoftwareRenderer, r: [i32; 4], c: [u8; 4]) {
    fb.ui_rect(r[0], r[1], r[2], 1, c);
    fb.ui_rect(r[0], r[1] + r[3] - 1, r[2], 1, c);
    fb.ui_rect(r[0], r[1], 1, r[3], c);
    fb.ui_rect(r[0] + r[2] - 1, r[1], 1, r[3], c);
}

/// Tüm editör arayüzünü çizer ve hit-test düzenini üretir.
pub fn draw_ui(
    fb: &mut SoftwareRenderer,
    font: &mut UiFont,
    win_w: i32,
    win_h: i32,
    st: &crate::EditorShared,
    layout: &Layout,
    hover_btn: Option<&str>,
    status: Option<(&str, bool)>, // (mesaj, başarılı mı)
) -> UiLayout {
    let mut ui = UiLayout::default();

    // Not: viewport arka planı renderer.clear() tarafından boyanır;
    // burada tekrar doldurmak 3D sahnenin silinmesine yol açar.

    // ── Üst bar
    rect(fb, layout.top_bar, COL_HEADER);
    border(fb, layout.top_bar, COL_BORDER);
    font.draw_text(fb, 12, 10, 17, COL_TEXT, "ELYASIUM ENGINE");
    let mode_text = if st.playing { "● PLAYING" } else { "■ EDIT MODE" };
    let status_col = if st.playing { COL_TEXT_GREEN } else { COL_TEXT_DIM };
    font.draw_text(fb, 190, 11, 14, status_col, mode_text);
    let fps_text = format!("FPS {:5.1}   FRAME {}", st.fps, st.frame_count);
    let fw = font.measure(14, &fps_text) as i32;
    font.draw_text(fb, win_w - RIGHT_W - fw - 20, 11, 14, COL_TEXT_DIM, &fps_text);

    // ── Sol panel: Hiyerarşi
    rect(fb, layout.left_panel, COL_PANEL);
    let hdr_h = 28;
    rect(
        fb,
        [layout.left_panel[0], layout.left_panel[1], layout.left_panel[2], hdr_h],
        COL_HEADER,
    );
    font.draw_text(fb, layout.left_panel[0] + 10, layout.left_panel[1] + 7, 15, COL_TEXT, "HIERARCHY");

    let icons = ["📦", "🔮", "🌍", "🛢️", "💊"];
    let geom_icon = |g: crate::renderer::GeometryType| match g {
        crate::renderer::GeometryType::Cube => "[CUBE]",
        crate::renderer::GeometryType::Sphere => "[SPHR]",
        crate::renderer::GeometryType::Plane => "[PLNE]",
        crate::renderer::GeometryType::Cylinder => "[CYLD]",
        crate::renderer::GeometryType::Capsule => "[CAPS]",
        crate::renderer::GeometryType::Custom(_) => "[OBJ]",
    };
    let _ = icons;

    let mut iy = layout.left_panel[1] + hdr_h;
    for obj in &st.scene.objects {
        let row_h = 24;
        let selected = st.selected == Some(obj.id);
        let hovered = hover_btn == Some(format!("ent_{}", obj.id).as_str());
        if selected {
            rect(fb, [layout.left_panel[0], iy, layout.left_panel[2], row_h], COL_SELECT);
        } else if hovered {
            rect(fb, [layout.left_panel[0], iy, layout.left_panel[2], row_h], COL_HOVER);
        }
        let team_dot = match obj.team {
            crate::renderer::EntityTeam::Player => [90, 160, 255, 255],
            crate::renderer::EntityTeam::Enemy => [235, 80, 80, 255],
            crate::renderer::EntityTeam::Neutral => [120, 200, 120, 255],
        };
        rect(fb, [layout.left_panel[0] + 8, iy + 8, 8, 8], team_dot);
        font.draw_text(fb, layout.left_panel[0] + 24, iy + 5, 14, COL_TEXT, &obj.name);
        let tag = geom_icon(obj.geometry);
        font.draw_text(
            fb,
            layout.left_panel[0] + layout.left_panel[2] - 62,
            iy + 6,
            11,
            COL_TEXT_DIM,
            tag,
        );
        ui.buttons.push(UiButton {
            id: format!("ent_{}", obj.id),
            rect: [layout.left_panel[0], iy, layout.left_panel[2], row_h],
        });
        iy += row_h;
    }

    // ── Sol panel alt: Inspector
    let insp_y = iy + 8;
    rect(
        fb,
        [layout.left_panel[0], insp_y, layout.left_panel[2], hdr_h],
        COL_HEADER,
    );
    font.draw_text(fb, layout.left_panel[0] + 10, insp_y + 7, 15, COL_TEXT, "INSPECTOR");

    if let Some(obj) = st.selected.and_then(|id| st.scene.get_object(id)) {
        let name_line = format!("{}  (id {})", obj.name, obj.id);
        font.draw_text(fb, layout.left_panel[0] + 10, insp_y + 34, 13, COL_TEXT_DIM, &name_line);

        let hp = format!("HP {:.0}/{:.0}", obj.health, obj.max_health);
        font.draw_text(fb, layout.left_panel[0] + 10, insp_y + 54, 13, COL_TEXT_DIM, &hp);

        let sections = [
            ("Position", EditComp::Position, obj.transform.position),
            ("Rotation", EditComp::Rotation, obj.transform.rotation),
            ("Scale", EditComp::Scale, obj.transform.scale),
        ];
        let mut sy = insp_y + 76;
        for (label, comp, val) in sections {
            font.draw_text(fb, layout.left_panel[0] + 10, sy, 13, COL_TEXT, label);
            sy += 20;
            for (axis, v) in ["x", "y", "z"].iter().zip([val.x, val.y, val.z]) {
                let axis_col = match *axis {
                    "x" => [235, 110, 100, 255],
                    "y" => [130, 210, 120, 255],
                    _ => [110, 150, 240, 255],
                };
                font.draw_text(fb, layout.left_panel[0] + 22, sy, 13, axis_col, axis);
                let text = format!("{:+.2}", v);
                let dr = [
                    layout.left_panel[0] + 40,
                    sy - 3,
                    170,
                    19,
                ];
                let dragging = st
                    .active_drag
                    .map(|d| d.comp == comp && d.axis == axis_idx(axis))
                    .unwrap_or(false);
                if dragging {
                    rect(fb, dr, COL_SELECT);
                } else {
                    rect(fb, dr, [45, 45, 48, 255]);
                }
                font.draw_text(fb, dr[0] + 6, sy, 13, COL_TEXT, &text);
                ui.drags.push(UiDragHandle { comp, axis: axis_idx(axis), rect: dr });
                sy += 21;
            }
            sy += 6;
        }
        // Materyal bilgisi
        sy += 20;
        font.draw_text(fb, layout.left_panel[0] + 10, sy, 13, COL_TEXT, "Material");
        sy += 18;
        let mat_info = format!("Metallic: {:.2}  Rough: {:.2}", obj.material.metallic, obj.material.roughness);
        font.draw_text(fb, layout.left_panel[0] + 22, sy, 12, COL_TEXT_DIM, &mat_info);
        sy += 16;
        let ao_info = format!("AO: {:.2}  Emissive: {:.1}", obj.material.ao, obj.material.emissive_strength);
        font.draw_text(fb, layout.left_panel[0] + 22, sy, 12, COL_TEXT_DIM, &ao_info);
        sy += 16;
        // Texture bilgisi
        let tex_count = [
            obj.material.albedo_texture,
            obj.material.normal_texture,
            obj.material.roughness_texture,
            obj.material.metallic_texture,
            obj.material.ao_texture,
            obj.material.emissive_texture,
        ].iter().filter(|t| t.is_some()).count();
        if tex_count > 0 {
            font.draw_text(fb, layout.left_panel[0] + 10, sy, 13, COL_TEXT, "Textures");
            sy += 18;
            if let Some(tid) = obj.material.albedo_texture {
                let tn = st.scene.texture_names.get(&tid).map(|s| s.as_str()).unwrap_or("?");
                font.draw_text(fb, layout.left_panel[0] + 22, sy, 12, [180, 220, 140, 255], &format!("Albedo: {}", tn));
                sy += 14;
            }
            if let Some(tid) = obj.material.normal_texture {
                let tn = st.scene.texture_names.get(&tid).map(|s| s.as_str()).unwrap_or("?");
                font.draw_text(fb, layout.left_panel[0] + 22, sy, 12, [140, 180, 220, 255], &format!("Normal: {}", tn));
                sy += 14;
            }
            if let Some(tid) = obj.material.roughness_texture {
                let tn = st.scene.texture_names.get(&tid).map(|s| s.as_str()).unwrap_or("?");
                font.draw_text(fb, layout.left_panel[0] + 22, sy, 12, [180, 180, 180, 255], &format!("Rough: {}", tn));
                sy += 14;
            }
            let total_tex = st.scene.textures.len();
            font.draw_text(fb, layout.left_panel[0] + 22, sy, 11, COL_TEXT_DIM, &format!("(toplam {} texture)", total_tex));
            sy += 14;
        }
        // Fizik bilgisi
        if let Some(ref rb) = obj.rigid_body {
            font.draw_text(fb, layout.left_panel[0] + 10, sy, 13, COL_TEXT, "Physics");
            sy += 18;
            let rb_type = format!("Type: {:?}  Mass: {:.1}", rb.body_type, rb.mass);
            font.draw_text(fb, layout.left_panel[0] + 22, sy, 12, COL_TEXT_DIM, &rb_type);
            sy += 16;
            let vel = rb.velocity;
            let vel_info = format!("Vel: ({:.1}, {:.1}, {:.1})", vel.x, vel.y, vel.z);
            font.draw_text(fb, layout.left_panel[0] + 22, sy, 12, COL_TEXT_DIM, &vel_info);
            sy += 16;
            let rest_info = format!("Restitution: {:.2}  Friction: {:.2}", rb.restitution, rb.friction);
            font.draw_text(fb, layout.left_panel[0] + 22, sy, 12, COL_TEXT_DIM, &rest_info);
            sy += 16;
        }
        // Işık bilgisi
        if obj.is_light {
            font.draw_text(fb, layout.left_panel[0] + 10, sy, 13, COL_TEXT, "Light Source");
            sy += 18;
            let em_info = format!("Emissive: ({:.1}, {:.1}, {:.1})", obj.material.emissive.x, obj.material.emissive.y, obj.material.emissive.z);
            font.draw_text(fb, layout.left_panel[0] + 22, sy, 12, COL_TEXT_DIM, &em_info);
            sy += 16;
        }
        // Etiketler
        if !obj.tags.is_empty() {
            font.draw_text(fb, layout.left_panel[0] + 10, sy, 13, COL_TEXT, "Tags");
            sy += 18;
            let tags_str = obj.tags.join(", ");
            font.draw_text(fb, layout.left_panel[0] + 22, sy, 12, COL_TEXT_DIM, &tags_str);
            sy += 16;
        }
        // Collider bilgisi
        sy += 6;
        let collider_info = format!("Collider radius: {:.2}", obj.collider_radius);
        font.draw_text(fb, layout.left_panel[0] + 10, sy, 12, COL_TEXT_DIM, &collider_info);
        sy += 16;
        font.draw_text(
            fb,
            layout.left_panel[0] + 10,
            sy + 4,
            12,
            COL_TEXT_DIM,
            "← sürükle: değeri değiştir",
        );
    } else {
        font.draw_text(fb, layout.left_panel[0] + 10, insp_y + 34, 13, COL_TEXT_DIM, "varlık seçilmedi");
    }

    // ── Sağ panel: Araçlar
    rect(fb, layout.right_panel, COL_PANEL);
    rect(
        fb,
        [layout.right_panel[0], layout.right_panel[1], layout.right_panel[2], hdr_h],
        COL_HEADER,
    );
    font.draw_text(fb, layout.right_panel[0] + 10, layout.right_panel[1] + 7, 15, COL_TEXT, "TOOLS");

    let bx = layout.right_panel[0] + 12;
    let bw = layout.right_panel[2] - 24;
    let bh = 32;
    let mut by = layout.right_panel[1] + hdr_h + 12;

    #[allow(unused_assignments)]
    macro_rules! tool_button {
        ($id:expr, $label:expr, $color:expr) => {{
            let hovered = hover_btn == Some($id);
            let c = if hovered { lighten($color) } else { $color };
            rect(fb, [bx, by, bw, bh], c);
            border(fb, [bx, by, bw, bh], COL_BORDER);
            let tw = font.measure(14, $label);
            font.draw_text(fb, bx + ((bw as f32 - tw) / 2.0) as i32, by + 9, 14, COL_TEXT, $label);
            ui.buttons.push(UiButton { id: $id.to_string(), rect: [bx, by, bw, bh] });
            by += bh + 8;
        }};
    }

    if st.playing {
        tool_button!("pause", "⏸  PAUSE", COL_BUTTON_PAUSE);
    } else {
        tool_button!("play", "▶  PLAY", COL_BUTTON_PLAY);
    }
    by += 6;
    font.draw_text(fb, bx, by, 12, COL_TEXT_DIM, "— SPAWN —");
    by += 18;
    tool_button!("add_cube", "+ Cube", COL_BUTTON);
    tool_button!("add_sphere", "+ Sphere", COL_BUTTON);
    tool_button!("add_cylinder", "+ Cylinder", COL_BUTTON);
    tool_button!("add_capsule", "+ Capsule", COL_BUTTON);
    tool_button!("del", "🗑  Delete Selected", COL_BUTTON);
    by += 6;
    font.draw_text(fb, bx, by, 12, COL_TEXT_DIM, "— PHYSICS —");
    by += 18;
    tool_button!("import_obj", "📥 Import OBJ", COL_BUTTON);
    tool_button!("import_glb", "📥 Import GLB", COL_BUTTON);
    tool_button!("import_texture", "🖼  Import Texture", COL_BUTTON);
    tool_button!("add_dynamic", "⚡ Dynamic Sphere", COL_BUTTON);
    tool_button!("add_light", "💡 Point Light", COL_BUTTON);
    tool_button!("toggle_physics", if st.scene.physics_enabled { "⚙  Physics: ON" } else { "⚙  Physics: OFF" }, COL_BUTTON);
    by += 6;
    font.draw_text(fb, bx, by, 12, COL_TEXT_DIM, "— PROCEDURAL —");
    by += 18;
    tool_button!("gen_terrain", "🏔  Generate Terrain", COL_BUTTON);
    by += 6;
    font.draw_text(fb, bx, by, 12, COL_TEXT_DIM, "— SCENE —");
    by += 18;
    tool_button!("save", "💾  Save Scene", COL_BUTTON);
    tool_button!("load", "📂  Load Scene", COL_BUTTON);
    tool_button!("undo", if st.can_undo { "↶  Undo" } else { "↶  Undo (yok)" }, COL_BUTTON);
    tool_button!("redo", if st.can_redo { "↷  Redo" } else { "↷  Redo (yok)" }, COL_BUTTON);
    by += 6;
    font.draw_text(fb, bx, by, 12, COL_TEXT_DIM, "— VIEW —");
    by += 18;
    tool_button!("reset_cam", "⟲  Reset Camera", COL_BUTTON);
    tool_button!(
        "grid",
        if st.show_grid { "▦  Grid: ON" } else { "▢  Grid: OFF" },
        COL_BUTTON
    );
    by += 6;
    font.draw_text(fb, bx, by, 12, COL_TEXT_DIM, "— ANIMATION —");
    by += 18;
    tool_button!("anim_play", if st.anim_playing { "⏸  Anim Pause" } else { "▶  Anim Play" }, COL_BUTTON);
    tool_button!("anim_idle", "🧍 Idle", COL_BUTTON);
    tool_button!("anim_walk", "🚶 Walk", COL_BUTTON);
    tool_button!("anim_run", "🏃 Run", COL_BUTTON);

    // Animasyon durumu gösterimi
    let anim_text = format!("Anim: {:.1}s speed:{:.1}x", st.anim_time, st.anim_speed);
    font.draw_text(fb, bx, by, 11, COL_TEXT_DIM, &anim_text);
    by += 14;
    let skel_text = format!("Bones: {}", st.skeleton.bones.len());
    font.draw_text(fb, bx, by, 11, COL_TEXT_DIM, &skel_text);
    by += 14;
    let clip_count = st.animation_clips.len();
    let blend_text = format!("Clips: {} | Blend: {}", clip_count, st.blend_machine.current_state().map(|s| s.clip_name.as_str()).unwrap_or("none"));
    font.draw_text(fb, bx, by, 11, COL_TEXT_DIM, &blend_text);
    by += 16;
    // Bone hierarchy gösterimi
    font.draw_text(fb, bx, by, 12, COL_TEXT, "Bone Hierarchy");
    by += 15;
    let root_bones = st.skeleton.root_bones.clone();
    fn draw_bone_tree(
        fb: &mut SoftwareRenderer, font: &mut UiFont,
        skeleton: &crate::skeletal::Skeleton,
        bone_id: u32, x: i32, y: &mut i32, depth: usize,
    ) {
        if let Some(bone) = skeleton.bones.get(bone_id as usize) {
            let indent = (depth as i32) * 10 + x;
            let marker = if bone.parent_id.is_some() { "├─" } else { "● " };
            let bone_text = format!("{} {}", marker, bone.name);
            let col = if depth == 0 { COL_TEXT } else { COL_TEXT_DIM };
            font.draw_text(fb, indent, *y, 11, col, &bone_text);
            *y += 13;
            // Children
            let children: Vec<u32> = skeleton.bones.iter()
                .filter(|b| b.parent_id == Some(bone_id))
                .map(|b| b.id)
                .collect();
            for child_id in children {
                draw_bone_tree(fb, font, skeleton, child_id, x, y, depth + 1);
            }
        }
    }
    // En fazla 20 bone göster (yer tasarrufu)
    let max_bones = 20.min(st.skeleton.bones.len());
    let mut bone_y = by;
    let mut bone_count = 0;
    for &root_id in &root_bones {
        if bone_count >= max_bones { break; }
        draw_bone_tree(fb, font, &st.skeleton, root_id, bx, &mut bone_y, 0);
        bone_count += 1;
    }
    if st.skeleton.bones.len() > max_bones {
        font.draw_text(fb, bx, bone_y, 10, COL_TEXT_DIM,
            &format!("... +{} more bones", st.skeleton.bones.len() - max_bones));
    }

    // ── Konsol paneli (açılabilir)
    if st.console.visible {
        let console_h = 180;
        let console_y = win_h - BOTTOM_BAR_H - console_h;
        let console_rect = [0, console_y, win_w, console_h];
        rect(fb, console_rect, [25, 25, 28, 255]);
        border(fb, console_rect, COL_BORDER);

        // Konsol başlığı
        rect(fb, [0, console_y, win_w, 22], COL_HEADER);
        font.draw_text(fb, 10, console_y + 4, 13, COL_TEXT, "CONSOLE");
        let close_btn = [win_w - 24, console_y + 2, 22, 18];
        rect(fb, close_btn, COL_BUTTON);
        font.draw_text(fb, close_btn[0] + 6, close_btn[1] + 3, 12, COL_TEXT, "X");
        ui.buttons.push(UiButton { id: "console_close".to_string(), rect: close_btn });

        // Konsol satırları
        let line_h = 16;
        let max_visible = (console_h - 44) / line_h;
        let visible_lines = st.console.visible_lines(max_visible as usize);
        let mut ly = console_y + 26;
        for line in visible_lines {
            font.draw_text(fb, 10, ly, 12, line.color(), line.text());
            ly += line_h;
        }

        // Input satırı
        let input_y = console_y + console_h - 22;
        rect(fb, [0, input_y, win_w, 22], [35, 35, 38, 255]);
        font.draw_text(fb, 8, input_y + 4, 13, COL_TEXT_GREEN, "> ");
        font.draw_text(fb, 24, input_y + 4, 13, COL_TEXT, &st.console_input);
        // Cursor çizgisi
        let cursor_x = 24 + font.measure(13, &st.console_input) as i32;
        rect(fb, [cursor_x, input_y + 2, 2, 18], COL_TEXT);
    }

    // ── Alt bar
    rect(fb, layout.bottom_bar, COL_HEADER);
    border(fb, layout.bottom_bar, COL_BORDER);
    // Konsol toggle butonu
    let console_btn = [win_w - 150, win_h - BOTTOM_BAR_H + 4, 60, 18];
    rect(fb, console_btn, COL_BUTTON);
    let console_label = if st.console.visible { "Console ▲" } else { "Console ▼" };
    font.draw_text(fb, console_btn[0] + 4, console_btn[1] + 3, 11, COL_TEXT, console_label);
    ui.buttons.push(UiButton { id: "toggle_console".to_string(), rect: console_btn });

    match status {
        Some((msg, ok)) => {
            font.draw_text(
                fb,
                12,
                win_h - BOTTOM_BAR_H + 7,
                13,
                if ok { COL_TEXT_GREEN } else { [240, 140, 100, 255] },
                msg,
            );
        }
        None => {
            let sel_count = st.selection.selection_count();
            let gizmo_label = st.gizmo.mode.label();
            let sel_info = if sel_count > 1 {
                format!("{} selected | {} | G R S gizmo · Ctrl+drag box", sel_count, gizmo_label)
            } else {
                format!("Gizmo: {} | G R S switch · F focus · Ctrl+A all", gizmo_label)
            };
            font.draw_text(
                fb,
                12,
                win_h - BOTTOM_BAR_H + 7,
                12,
                COL_TEXT_DIM,
                &sel_info,
            );
        }
    }
    let sel_text = if st.selection.is_multi_selection() {
        format!("sel: {} | entities: {}", st.selection.selection_count(), st.scene.objects.len())
    } else {
        format!("entities: {}  tex: {}", st.scene.objects.len(), st.scene.textures.len())
    };
    font.draw_text(fb, win_w - 350, win_h - BOTTOM_BAR_H + 7, 13, COL_TEXT_DIM, &sel_text);

    // GPU Compute durumu
    let gpu_text = format!("GPU: {} particles | {}/{} visible/culled",
        st.gpu_particles_active, st.gpu_visible, st.gpu_visible + st.gpu_culled);
    font.draw_text(fb, win_w - 180, win_h - BOTTOM_BAR_H + 7, 13, [100, 200, 255, 255], &gpu_text);

    // Uzantı panellerini çiz
    st.extensions.draw_extensions_panels(
        fb,
        font,
        layout.right_panel,
        hover_btn,
        &mut ui,
    );

    ui
}

fn axis_idx(a: &str) -> usize {
    match a {
        "x" => 0,
        "y" => 1,
        _ => 2,
    }
}

fn lighten(c: [u8; 4]) -> [u8; 4] {
    [
        c[0].saturating_add(28),
        c[1].saturating_add(28),
        c[2].saturating_add(28),
        255,
    ]
}
