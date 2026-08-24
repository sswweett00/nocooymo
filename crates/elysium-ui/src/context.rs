//! UI Context - Main entry point for immediate mode UI

use crate::command::UiCommandList;
use crate::draw::DrawList;
use crate::event::UiMessage;
use crate::input::{winit_key_to_keycode, InputState, KeyCode, MouseButton};
use crate::theme::Theme;
use crate::widgets::{
    WidgetId, WidgetState as WidgetsWidgetState, 
    DragDropState, ResponsiveState, Breakpoint,
    ModalState, MenuState, MenuItem, SliderState, ProgressBarState
};
use crate::{UiRect, Bounds, layout::{ResponsiveLayoutManager, LayoutType, LayoutConfig}};
use glam::Vec2;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::Duration;
use gpui::{ElementId, LayoutId, Pixels, Point, Size, Style, WindowContext};

pub struct UiContext {
    pub window_context: WindowContext,
    pub theme: Theme,
    pub widgets: HashMap<ElementId, WidgetEntry>,
    pub focus_chain: VecDeque<ElementId>,
    pub event_queue: VecDeque<UiMessage>,
    pub layout_cache: HashMap<ElementId, LayoutInfo>,
    pub dirty_widgets: Vec<ElementId>,
    pub draw_list: DrawList,
    pub mouse_pos: Vec2,
    pub is_hovered: bool,
    pub drag_drop_state: DragDropState,
    pub responsive_state: ResponsiveState,
    pub modal_stack: Vec<ModalState>,
    pub menu_state: Option<MenuState>,
    pub clipboard: String,
    pub animation_controller: AnimationController,
    pub layout_manager: ResponsiveLayoutManager,
}

pub struct WidgetEntry {
    pub element_id: ElementId,
    pub layout_id: Option<LayoutId>,
    pub bounds: Option<Bounds<Pixels>>,
    pub visible: bool,
    pub enabled: bool,
    pub focused: bool,
    pub widget_state: WidgetsWidgetState,
}

pub struct LayoutInfo {
    pub size: Size<Pixels>,
    pub position: Point<Pixels>,
    pub style: Style,
}

pub struct AnimationController {
    pub global_time: Duration,
    pub animation_speed: f32,
}

impl AnimationController {
    pub fn new() -> Self {
        Self {
            global_time: Duration::from_secs(0),
            animation_speed: 1.0,
        }
    }

    pub fn update(&mut self, delta_time: Duration) {
        self.global_time += delta_time.mul_f32(self.animation_speed);
    }

    pub fn get_time(&self) -> Duration {
        self.global_time
    }
}

impl UiContext {
    pub fn new(window_context: WindowContext) -> Self {
        Self {
            window_context,
            theme: Theme::default(),
            widgets: HashMap::new(),
            focus_chain: VecDeque::new(),
            event_queue: VecDeque::new(),
            layout_cache: HashMap::new(),
            dirty_widgets: Vec::new(),
            draw_list: DrawList::new(),
            mouse_pos: Vec2::ZERO,
            is_hovered: false,
            drag_drop_state: DragDropState::new(),
            responsive_state: ResponsiveState::new(1920.0, 1080.0), // Varsayılan ekran boyutu
            modal_stack: Vec::new(),
            menu_state: None,
            clipboard: String::new(),
            animation_controller: AnimationController::new(),
            layout_manager: ResponsiveLayoutManager::new(),
        }
    }

    pub fn begin_frame(&mut self) {
        // Çerçeve başlangıcında gerekli temizlik işlemleri
        self.draw_list.clear();
        self.is_hovered = false;
        self.dirty_widgets.clear();
        self.draw_list.update_animations();
    }

    pub fn end_frame(&mut self) {
        // Çerçeve sonunda gerekli bitirme işlemleri
        // Animasyon kontrolörünü güncelle
        let delta_time = Duration::from_millis(16); // ~60 FPS
        self.animation_controller.update(delta_time);
    }

    pub fn register_widget(&mut self, element_id: ElementId) -> ElementId {
        let widget_entry = WidgetEntry {
            element_id,
            layout_id: None,
            bounds: None,
            visible: true,
            enabled: true,
            focused: false,
            widget_state: WidgetsWidgetState::new(),
        };

        self.widgets.insert(element_id, widget_entry);
        self.dirty_widgets.push(element_id);

        element_id
    }

    pub fn update_widget_visibility(&mut self, element_id: ElementId, visible: bool) {
        if let Some(widget) = self.widgets.get_mut(&element_id) {
            widget.visible = visible;
            self.mark_dirty(element_id);
        }
    }

    pub fn update_widget_enabled(&mut self, element_id: ElementId, enabled: bool) {
        if let Some(widget) = self.widgets.get_mut(&element_id) {
            widget.enabled = enabled;
            self.mark_dirty(element_id);
        }
    }

    pub fn set_focus(&mut self, element_id: ElementId) {
        // Remove focus from previously focused widget
        for widget in self.widgets.values_mut() {
            if widget.focused {
                widget.focused = false;
            }
        }

        // Set focus to new widget
        if let Some(widget) = self.widgets.get_mut(&element_id) {
            widget.focused = true;
            self.focus_chain.push_back(element_id);
        }
    }

    pub fn get_focus(&self) -> Option<ElementId> {
        self.focus_chain.back().copied()
    }

    pub fn mark_dirty(&mut self, element_id: ElementId) {
        if !self.dirty_widgets.contains(&element_id) {
            self.dirty_widgets.push(element_id);
        }
    }

    pub fn is_dirty(&self, element_id: ElementId) -> bool {
        self.dirty_widgets.contains(&element_id)
    }

    pub fn update_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_pos = Vec2::new(x, y);
        
        // Yeni fare konumuyla ilgili işlemleri yap
        for (element_id, widget) in &mut self.widgets {
            if let Some(bounds) = widget.bounds {
                let point = Point::new(Pixels(self.mouse_pos.x), Pixels(self.mouse_pos.y));
                let is_over = bounds.contains(&point);
                
                // Hover durumunu güncelle
                if is_over && !widget.widget_state.hovered {
                    widget.widget_state.hovered = true;
                    self.queue_message(UiMessage::MouseEnter(*element_id));
                } else if !is_over && widget.widget_state.hovered {
                    widget.widget_state.hovered = false;
                    self.queue_message(UiMessage::MouseLeave(*element_id));
                }
            }
        }
    }

    pub fn is_point_over_widget(&self, element_id: ElementId) -> bool {
        if let Some(widget) = self.widgets.get(&element_id) {
            if let Some(bounds) = widget.bounds {
                let point = Point::new(Pixels(self.mouse_pos.x), Pixels(self.mouse_pos.y));
                return bounds.contains(&point);
            }
        }
        false
    }

    pub fn get_draw_list(&mut self) -> &mut DrawList {
        &mut self.draw_list
    }

    pub fn queue_message(&mut self, message: UiMessage) {
        self.event_queue.push_back(message);
    }

    pub fn process_events(&mut self) {
        while let Some(message) = self.event_queue.pop_front() {
            match message {
                UiMessage::MouseMove(pos) => {
                    self.update_mouse_position(pos.x.0, pos.y.0);
                },
                UiMessage::MouseDown(button, pos) => {
                    // Tıklanan widget'ı bul
                    for (element_id, widget) in &mut self.widgets {
                        if widget.visible && widget.enabled {
                            if let Some(bounds) = widget.bounds {
                                if bounds.contains(&pos) {
                                    widget.widget_state.active = true;
                                    self.set_focus(*element_id);
                                    
                                    // Çift tıklama kontrolü
                                    if widget.widget_state.is_double_clicked(Duration::from_millis(300)) {
                                        // Çift tıklama olayı
                                    }
                                    
                                    break;
                                }
                            }
                        }
                    }
                },
                UiMessage::MouseUp(button, pos) => {
                    // Aktif widget'ı pasif yap
                    for widget in self.widgets.values_mut() {
                        if widget.widget_state.active {
                            widget.widget_state.active = false;
                            break;
                        }
                    }
                },
                _ => {}
            }
        }
    }

    pub fn update_layout_cache(&mut self, element_id: ElementId, layout_info: LayoutInfo) {
        self.layout_cache.insert(element_id, layout_info);
    }

    pub fn get_cached_layout(&self, element_id: ElementId) -> Option<&LayoutInfo> {
        self.layout_cache.get(&element_id)
    }

    // Drag & drop işlemleri
    pub fn start_drag(&mut self, source_id: WidgetId, data: std::sync::Arc<dyn std::any::Any + Send + Sync>) {
        self.drag_drop_state.start_drag(source_id, data);
    }

    pub fn end_drag(&mut self) {
        self.drag_drop_state.end_drag();
    }

    pub fn update_drag_position(&mut self, x: f32, y: f32) {
        if self.drag_drop_state.is_dragging {
            // Hedef widget'ı belirle
            for (element_id, widget) in &self.widgets {
                if widget.visible && widget.enabled {
                    if let Some(bounds) = widget.bounds {
                        let pos = Point::new(Pixels(x), Pixels(y));
                        if bounds.contains(&pos) {
                            self.drag_drop_state.hover_target = Some(WidgetId::from_u64(element_id.0));
                            break;
                        }
                    }
                }
            }
        }
    }

    // Responsive tasarım işlemleri
    pub fn update_responsive_state(&mut self, width: f32, height: f32) {
        self.responsive_state = ResponsiveState::new(width, height);
        
        // Temayı breakpoint'e göre güncelle
        match self.responsive_state.current_breakpoint {
            Breakpoint::Mobile => {
                self.theme = Theme::default(); // Mobil için özel tema olabilir
            },
            Breakpoint::Tablet => {
                // Tablet için özel ayarlar
            },
            _ => {} // Diğer durumlar için değişiklik yok
        }
    }

    // Modal işlemleri
    pub fn show_modal(&mut self, mut modal: ModalState) {
        modal.visible = true;
        self.modal_stack.push(modal);
    }

    pub fn hide_top_modal(&mut self) {
        if let Some(mut modal) = self.modal_stack.pop() {
            modal.visible = false;
        }
    }

    pub fn is_any_modal_visible(&self) -> bool {
        self.modal_stack.iter().any(|modal| modal.visible)
    }

    // Menü işlemleri
    pub fn show_menu(&mut self, menu: MenuState) {
        self.menu_state = Some(menu);
    }

    pub fn hide_menu(&mut self) {
        self.menu_state = None;
    }

    pub fn select_menu_next(&mut self) {
        if let Some(ref mut menu) = self.menu_state {
            menu.select_next();
        }
    }

    pub fn select_menu_prev(&mut self) {
        if let Some(ref mut menu) = self.menu_state {
            menu.select_prev();
        }
    }

    // Clipboard işlemleri
    pub fn set_clipboard_text(&mut self, text: String) {
        self.clipboard = text;
    }

    pub fn get_clipboard_text(&self) -> &str {
        &self.clipboard
    }

    // Animasyon işlemleri
    pub fn start_widget_animation(&mut self, element_id: ElementId, animation_type: crate::widgets::AnimationType) {
        if let Some(widget) = self.widgets.get_mut(&element_id) {
            // Animasyon türüne göre özel işlemler yapılabilir
            match animation_type {
                crate::widgets::AnimationType::FadeIn => {
                    // Fade in animasyonu başlat
                },
                crate::widgets::AnimationType::SlideIn(_) => {
                    // Slide in animasyonu başlat
                },
                _ => {}
            }
        }
    }

    // Layout yönetimi
    pub fn set_responsive_layout_manager(&mut self, manager: ResponsiveLayoutManager) {
        self.layout_manager = manager;
    }

    pub fn calculate_responsive_layout(&self, container_rect: UiRect, children_count: usize) -> Vec<UiRect> {
        self.layout_manager.calculate_layout_for_current_breakpoint(container_rect, children_count)
    }

    // Slider işlemleri
    pub fn update_slider(&mut self, element_id: ElementId, value: f32) -> bool {
        if let Some(widget) = self.widgets.get_mut(&element_id) {
            if let Some(ref mut slider_state) = widget.widget_state.custom_data {
                // Slider state'i güncelle
                // Not: Gerçek implementasyonda bu tür dönüşümü güvenli olacak şekilde yapılmalıdır
                return true;
            }
        }
        false
    }

    // Progress bar işlemleri
    pub fn update_progress_bar(&mut self, element_id: ElementId, value: f32) -> bool {
        if let Some(widget) = self.widgets.get_mut(&element_id) {
            if let Some(ref mut progress_state) = widget.widget_state.custom_data {
                // Progress bar state'i güncelle
                // Not: Gerçek implementasyonda bu tür dönüşümü güvenli olacak şekilde yapılmalıdır
                return true;
            }
        }
        false
    }
}

impl Drop for UiContext {
    fn drop(&mut self) {
        // Gerekli temizlik işlemleri
    }
}