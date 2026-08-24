//! GPU-accelerated immediate mode UI (Mimari §7.1).

pub mod context;
pub mod widgets;
pub mod draw;
pub mod event;
pub mod input;
pub mod layout;
pub mod theme;
pub mod command;

pub use context::*;
pub use widgets::*;
pub use draw::*;
pub use event::*;
pub use input::*;
pub use layout::*;
pub use theme::*;
pub use command::*;

use gpui::*;

/// Shared rectangle type used across all UI modules.
#[derive(Clone, Copy, Debug, Default)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Shared bounds type used across all UI modules.
#[derive(Debug, Clone, Copy)]
pub struct Bounds<T> {
    pub origin: gpui::Point<T>,
    pub size: gpui::Size<T>,
}

impl<T> Bounds<T> {
    pub fn new(origin: gpui::Point<T>, size: gpui::Size<T>) -> Self {
        Self { origin, size }
    }
    
    pub fn contains(&self, point: &gpui::Point<T>) -> bool 
    where
        T: PartialOrd,
    {
        point.x >= self.origin.x && 
        point.x <= (self.origin.x + self.size.width) &&
        point.y >= self.origin.y && 
        point.y <= (self.origin.y + self.size.height)
    }
    
    pub fn intersects(&self, other: &Self) -> bool 
    where
        T: PartialOrd,
    {
        self.origin.x < other.origin.x + other.size.width &&
        self.origin.x + self.size.width > other.origin.x &&
        self.origin.y < other.origin.y + other.size.height &&
        self.origin.y + self.size.height > other.origin.y
    }
}

pub struct UiSystem {
    pub context: Option<UiContext>,
    pub theme: Theme,
    pub focused_widget: Option<WidgetId>,
}

impl UiSystem {
    pub fn new() -> Self {
        Self {
            context: None,
            theme: Theme::default(),
            focused_widget: None,
        }
    }

    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn set_context(&mut self, context: UiContext) {
        self.context = Some(context);
    }

    pub fn get_context(&mut self) -> Option<&mut UiContext> {
        self.context.as_mut()
    }

    pub fn register_systems(schedule: &mut elysium_core::scheduler::Schedule) {
        schedule.add_system(ui_update_system);
    }
}

fn ui_update_system(world: &mut elysium_core::World) {
    // This system would handle UI updates
    // For now, it's a placeholder
}

pub fn init_ui_system() -> App {
    App::new()
}

impl UiRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn min(&self) -> glam::Vec2 {
        glam::Vec2::new(self.x, self.y)
    }

    pub fn max(&self) -> glam::Vec2 {
        glam::Vec2::new(self.x + self.width, self.y + self.height)
    }

    pub fn center(&self) -> glam::Vec2 {
        glam::Vec2::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn size(&self) -> glam::Vec2 {
        glam::Vec2::new(self.width, self.height)
    }

    pub fn offset(&self, dx: f32, dy: f32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            width: self.width,
            height: self.height,
        }
    }

    pub fn scale(&self, factor: f32) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
            width: self.width * factor,
            height: self.height * factor,
        }
    }

    pub fn inset(&self, padding: f32) -> Self {
        Self {
            x: self.x + padding,
            y: self.y + padding,
            width: self.width - 2.0 * padding,
            height: self.height - 2.0 * padding,
        }
    }

    pub fn padded(&self, padding: f32) -> Self {
        Self {
            x: self.x - padding,
            y: self.y - padding,
            width: self.width + 2.0 * padding,
            height: self.height + 2.0 * padding,
        }
    }

    pub fn contains_point(&self, point: glam::Vec2) -> bool {
        point.x >= self.x && 
        point.x < self.x + self.width &&
        point.y >= self.y && 
        point.y < self.y + self.height
    }

    pub fn expand_to_fit(&mut self, other: &UiRect) {
        let min_x = self.x.min(other.x);
        let min_y = self.y.min(other.y);
        let max_x = (self.x + self.width).max(other.x + other.width);
        let max_y = (self.y + self.height).max(other.y + other.height);
        
        self.x = min_x;
        self.y = min_y;
        self.width = max_x - min_x;
        self.height = max_y - min_y;
    }
}

impl From<UiRect> for Bounds<f32> {
    fn from(rect: UiRect) -> Self {
        Bounds::new(
            gpui::Point::new(rect.x, rect.y),
            gpui::Size::new(rect.width, rect.height),
        )
    }
}

// Yardımcı fonksiyonlar
impl UiSystem {
    /// UI sistemini başlatır ve varsayılan ayarlarla yapılandırır
    pub fn initialize_default() -> Self {
        Self::new()
            .with_theme(Theme::default())
    }

    /// UI sistemine tema uygular
    pub fn apply_theme(&mut self, theme: Theme) -> &mut Self {
        self.theme = theme;
        self
    }

    /// Yeni bir UI bağlamı oluşturur
    pub fn create_context(&self, window_context: WindowContext) -> UiContext {
        let mut context = UiContext::new(window_context);
        
        // Responsive layout manager'ı ayarla
        let layout_manager = ResponsiveLayoutManager::new()
            .add_breakpoint_config(
                Breakpoint::Mobile,
                LayoutConfig {
                    layout_type: LayoutType::Flex(
                        FlexLayout::new(LayoutDirection::Vertical)
                            .with_gap(8.0)
                    ),
                    constraints: LayoutConstraints::new()
                        .with_padding(8.0)
                        .with_margin(4.0),
                }
            )
            .add_breakpoint_config(
                Breakpoint::Desktop,
                LayoutConfig {
                    layout_type: LayoutType::Flex(
                        FlexLayout::new(LayoutDirection::Horizontal)
                            .with_gap(12.0)
                    ),
                    constraints: LayoutConstraints::new()
                        .with_padding(12.0)
                        .with_margin(6.0),
                }
            );
        
        context.set_responsive_layout_manager(layout_manager);
        context
    }

    /// Widget ID'si oluşturur
    pub fn create_widget_id(&self, label: &str, parent_id: u64) -> WidgetId {
        WidgetId::new(label, parent_id)
    }

    /// Geçerli odaklanılmış widget'ı döndürür
    pub fn get_focused_widget(&self) -> Option<WidgetId> {
        self.focused_widget
    }

    /// Widget'ı odaklar
    pub fn focus_widget(&mut self, widget_id: WidgetId) -> &mut Self {
        self.focused_widget = Some(widget_id);
        self
    }

    /// Odaklama durumunu sıfırlar
    pub fn clear_focus(&mut self) -> &mut Self {
        self.focused_widget = None;
        self
    }

    /// Animasyon kontrolleri
    pub fn animate_widget<F>(&self, widget_id: WidgetId, animation_func: F) -> bool
    where
        F: FnOnce(&AnimationType) -> (),
    {
        // Burada animasyon kontrolü yapılacak
        // Gerçek implementasyon UI context ile entegre edilecek
        true
    }

    /// Drag & drop işlemleri
    pub fn start_drag_operation(&self, source_id: WidgetId, data: std::sync::Arc<dyn std::any::Any + Send + Sync>) {
        // Drag işlemini başlat
        // Gerçek implementasyon UI context ile entegre edilecek
    }

    /// Responsive tasarım işlemleri
    pub fn update_responsive_ui(&mut self, width: f32, height: f32) -> &mut Self {
        if let Some(ref mut context) = self.context {
            context.update_responsive_state(width, height);
        }
        self
    }

    /// Modal işlemleri
    pub fn show_modal(&mut self, modal: ModalState) -> &mut Self {
        if let Some(ref mut context) = self.context {
            context.show_modal(modal);
        }
        self
    }

    /// Menü işlemleri
    pub fn show_menu(&mut self, menu: MenuState) -> &mut Self {
        if let Some(ref mut context) = self.context {
            context.show_menu(menu);
        }
        self
    }
}

// UI Builder pattern
pub struct UiBuilder<'a> {
    context: &'a mut UiContext,
}

impl<'a> UiBuilder<'a> {
    pub fn new(context: &'a mut UiContext) -> Self {
        Self { context }
    }

    pub fn add_button(&mut self, text: &str, rect: UiRect) -> WidgetId {
        let widget_id = WidgetId::new(text, 0);
        // Buton ekleme işlemleri
        widget_id
    }

    pub fn add_slider(&mut self, rect: UiRect, min: f32, max: f32, initial: f32) -> WidgetId {
        let slider_state = SliderState::new(min, max, initial);
        let widget_id = WidgetId::new("slider", 0);
        // Slider ekleme işlemleri
        widget_id
    }

    pub fn add_progress_bar(&mut self, rect: UiRect, min: f32, max: f32) -> WidgetId {
        let progress_state = ProgressBarState::new(min, max);
        let widget_id = WidgetId::new("progress", 0);
        // Progress bar ekleme işlemleri
        widget_id
    }

    pub fn add_text(&mut self, text: &str, pos: glam::Vec2) -> WidgetId {
        let widget_id = WidgetId::new(text, 0);
        // Text ekleme işlemleri
        widget_id
    }
}