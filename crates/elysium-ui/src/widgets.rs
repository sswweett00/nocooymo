//! Widget davranışları — her widget türü için mantık.
//!
//! Bu modül temel, kapsayıcı, veri, gelişmiş widget'ları ve
//! layout + tema yardımcılarını içerir.

use crate::types::{WidgetId, UiEvent, Widget, WidgetType, WidgetState};
use crate::theme::{ThemeColor, DarkTheme, LightTheme, WidgetStyle, CornerStyle, BorderStyle};
use crate::layout::{LayoutBag, LayoutDirection, LayoutItem};
use crate::draw::{Color, DrawList};

// ---------------------------------------------------------------------------
// Temel davranış arayüzü (mevcut kod korunuyor)
// ---------------------------------------------------------------------------

/// Widget davranış arayüzü
pub trait WidgetBehavior {
    fn widget_type(&self) -> &str;
    fn update(&self, id: WidgetId, rect: [i32; 4], mouse_pos: [f32; 2]) -> WidgetUpdateResult;
    fn handle_click(&self, id: WidgetId) -> Option<UiEvent>;
}

/// Widget güncelleme sonucu
pub struct WidgetUpdateResult {
    pub hovered: bool,
    pub changed: bool,
}

// ---------------------------------------------------------------------------
// 1. Temel Widget'lar
// ---------------------------------------------------------------------------

// --- Button ---

/// Düğme türü
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ButtonType {
    #[default]
    Push,
    Toggle,
    Radio,
    Checkbox,
}

/// Buton durumu
#[derive(Clone, Debug, Default)]
pub struct ButtonState {
    pub button_type: ButtonType,
    pub pressed: bool,
    pub toggled: bool,
    pub radio_group: Option<u32>,
}

/// Düğme davranışı
pub struct ButtonBehavior {
    pub button_type: ButtonType,
}

impl Default for ButtonBehavior {
    fn default() -> Self { Self { button_type: ButtonType::Push } }
}

impl WidgetBehavior for ButtonBehavior {
    fn widget_type(&self) -> &str { "button" }

    fn update(&self, _id: WidgetId, rect: [i32; 4], mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        let hovered = mouse_pos[0] >= rect[0] as f32
            && mouse_pos[0] < (rect[0] + rect[2]) as f32
            && mouse_pos[1] >= rect[1] as f32
            && mouse_pos[1] < (rect[1] + rect[3]) as f32;
        WidgetUpdateResult { hovered, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl ButtonBehavior {
    pub fn push() -> Self { Self { button_type: ButtonType::Push } }
    pub fn toggle() -> Self { Self { button_type: ButtonType::Toggle } }
    pub fn radio(group: u32) -> Self { Self { button_type: ButtonType::Radio, } }
    pub fn checkbox() -> Self { Self { button_type: ButtonType::Checkbox } }
}

// --- Slider ---

/// Slider yönü
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SliderOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Slider türü
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SliderType {
    #[default]
    Single,
    Range,
}

/// Slider durumu
#[derive(Clone, Debug, Default)]
pub struct SliderState {
    pub min: f32,
    pub max: f32,
    pub value: f32,
    pub value2: f32, // range için ikinci değer
    pub orientation: SliderOrientation,
    pub slider_type: SliderType,
}

/// Slider davranışı
pub struct SliderBehavior {
    pub state: SliderState,
}

impl Default for SliderBehavior {
    fn default() -> Self { Self { state: SliderState::default() } }
}

impl WidgetBehavior for SliderBehavior {
    fn widget_type(&self) -> &str { "slider" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl SliderBehavior {
    pub fn new(min: f32, max: f32, value: f32) -> Self {
        Self {
            state: SliderState {
                min, max, value,
                ..Default::default()
            },
        }
    }

    pub fn range(min: f32, max: f32, v1: f32, v2: f32) -> Self {
        Self {
            state: SliderState {
                min, max, value: v1, value2: v2,
                slider_type: SliderType::Range,
                ..Default::default()
            },
        }
    }

    pub fn vertical(min: f32, max: f32, value: f32) -> Self {
        Self {
            state: SliderState {
                min, max, value,
                orientation: SliderOrientation::Vertical,
                ..Default::default()
            },
        }
    }
}

// --- Checkbox (detaylı) ---

/// Checkbox davranışı
pub struct CheckboxBehavior;

impl WidgetBehavior for CheckboxBehavior {
    fn widget_type(&self) -> &str { "checkbox" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

// --- Text Input ---

/// Metin girdisi türü
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextInputType {
    #[default]
    SingleLine,
    MultiLine,
    Password,
    Numeric,
}

/// Metin girdisi durumu
#[derive(Clone, Debug, Default)]
pub struct TextInputState {
    pub input_type: TextInputType,
    pub text: String,
    pub cursor_pos: usize,
    pub selection_start: Option<usize>,
    pub placeholder: String,
    pub max_length: Option<usize>,
    pub multiline_scroll: f32,
}

/// Metin girdisi davranışı
pub struct TextInputBehavior {
    pub state: TextInputState,
}

impl Default for TextInputBehavior {
    fn default() -> Self { Self { state: TextInputState::default() } }
}

impl WidgetBehavior for TextInputBehavior {
    fn widget_type(&self) -> &str { "text_input" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl TextInputBehavior {
    pub fn new(input_type: TextInputType) -> Self {
        Self {
            state: TextInputState {
                input_type,
                ..Default::default()
            },
        }
    }

    pub fn single_line() -> Self { Self::new(TextInputType::SingleLine) }
    pub fn multi_line() -> Self { Self::new(TextInputType::MultiLine) }
    pub fn password() -> Self { Self::new(TextInputType::Password) }
    pub fn numeric() -> Self { Self::new(TextInputType::Numeric) }

    pub fn with_placeholder(mut self, placeholder: &str) -> Self {
        self.state.placeholder = placeholder.to_string();
        self
    }

    pub fn with_max_length(mut self, max: usize) -> Self {
        self.state.max_length = Some(max);
        self
    }

    pub fn text(&self) -> &str { &self.state.text }
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.state.text = text.into();
    }
}

// --- Dropdown / ComboBox ---

/// Dropdown durumu
#[derive(Clone, Debug, Default)]
pub struct DropdownState {
    pub options: Vec<String>,
    pub selected_index: usize,
    pub is_open: bool,
    pub hovered_index: Option<usize>,
}

/// Dropdown davranışı
pub struct DropdownBehavior {
    pub state: DropdownState,
}

impl Default for DropdownBehavior {
    fn default() -> Self { Self { state: DropdownState::default() } }
}

impl WidgetBehavior for DropdownBehavior {
    fn widget_type(&self) -> &str { "dropdown" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl DropdownBehavior {
    pub fn new(options: Vec<&str>, selected: usize) -> Self {
        let opts: Vec<String> = options.iter().map(|s| s.to_string()).collect();
        Self {
            state: DropdownState {
                options: opts,
                selected_index: selected,
                ..Default::default()
            },
        }
    }

    pub fn add_option(&mut self, option: &str) {
        self.state.options.push(option.to_string());
    }

    pub fn selected(&self) -> Option<&str> {
        self.state.options.get(self.state.selected_index).map(|s| s.as_str())
    }

    pub fn select(&mut self, index: usize) {
        if index < self.state.options.len() {
            self.state.selected_index = index;
        }
    }
}

// --- ListBox ---

/// Liste öğesi
#[derive(Clone, Debug)]
pub struct ListItem {
    pub id: WidgetId,
    pub label: String,
    pub icon: Option<String>,
    pub data: Option<String>,
}

/// ListBox durumu
#[derive(Clone, Debug, Default)]
pub struct ListBoxState {
    pub items: Vec<ListItem>,
    pub selected_index: Option<usize>,
    pub hovered_index: Option<usize>,
    pub scroll_offset: f32,
    pub multi_select: bool,
    pub selected_indices: Vec<usize>,
}

/// ListBox davranışı
pub struct ListBoxBehavior {
    pub state: ListBoxState,
}

impl Default for ListBoxBehavior {
    fn default() -> Self { Self { state: ListBoxState::default() } }
}

impl WidgetBehavior for ListBoxBehavior {
    fn widget_type(&self) -> &str { "listbox" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl ListBoxBehavior {
    pub fn new(items: Vec<&str>) -> Self {
        let list_items: Vec<ListItem> = items.iter().enumerate().map(|(i, label)| {
            ListItem {
                id: WidgetId(i as u64 + 1000),
                label: label.to_string(),
                icon: None,
                data: None,
            }
        }).collect();
        Self {
            state: ListBoxState {
                items: list_items,
                ..Default::default()
            },
        }
    }

    pub fn multi_select(mut self) -> Self {
        self.state.multi_select = true;
        self
    }

    pub fn selected(&self) -> Option<&str> {
        self.state.selected_index.and_then(|i| {
            self.state.items.get(i).map(|item| item.label.as_str())
        })
    }

    pub fn add_item(&mut self, label: &str) {
        let id = WidgetId(self.state.items.len() as u64 + 1000);
        self.state.items.push(ListItem {
            id,
            label: label.to_string(),
            icon: None,
            data: None,
        });
    }
}

// --- TreeView ---

/// Ağaç düğümü (detaylı)
#[derive(Clone, Debug)]
pub struct TreeItem {
    pub id: WidgetId,
    pub label: String,
    pub icon: Option<String>,
    pub children: Vec<TreeItem>,
    pub expanded: bool,
    pub depth: usize,
    pub user_data: Option<String>,
}

/// Ağaç görünümü durumu
#[derive(Clone, Debug, Default)]
pub struct TreeViewState {
    pub root_items: Vec<TreeItem>,
    pub selected_id: Option<WidgetId>,
    pub hovered_id: Option<WidgetId>,
    pub scroll_offset: f32,
}

/// Ağaç görünümü davranışı
pub struct TreeViewBehavior {
    pub state: TreeViewState,
}

impl Default for TreeViewBehavior {
    fn default() -> Self { Self { state: TreeViewState::default() } }
}

impl WidgetBehavior for TreeViewBehavior {
    fn widget_type(&self) -> &str { "treeview" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl TreeViewBehavior {
    pub fn new(root_items: Vec<TreeItem>) -> Self {
        Self {
            state: TreeViewState {
                root_items,
                ..Default::default()
            },
        }
    }

    pub fn toggle_expand(&mut self, id: WidgetId) {
        Self::toggle_expand_in_list(&mut self.state.root_items, id);
    }

    fn toggle_expand_in_list(items: &mut Vec<TreeItem>, id: WidgetId) {
        for item in items.iter_mut() {
            if item.id == id {
                item.expanded = !item.expanded;
                return;
            }
            Self::toggle_expand_in_list(&mut item.children, id);
        }
    }

    pub fn select(&mut self, id: WidgetId) {
        self.state.selected_id = Some(id);
    }
}

// --- TabControl ---

/// Sekme durumu
#[derive(Clone, Debug, Default)]
pub struct TabItem {
    pub id: WidgetId,
    pub label: String,
    pub closable: bool,
    pub icon: Option<String>,
}

/// Sekme denetimi durumu
#[derive(Clone, Debug, Default)]
pub struct TabControlState {
    pub tabs: Vec<TabItem>,
    pub active_index: usize,
    pub scroll_offset: f32,
}

/// Sekme denetimi davranışı
pub struct TabControlBehavior {
    pub state: TabControlState,
}

impl Default for TabControlBehavior {
    fn default() -> Self { Self { state: TabControlState::default() } }
}

impl WidgetBehavior for TabControlBehavior {
    fn widget_type(&self) -> &str { "tabcontrol" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl TabControlBehavior {
    pub fn new(tabs: Vec<&str>) -> Self {
        let tab_items: Vec<TabItem> = tabs.iter().enumerate().map(|(i, label)| {
            TabItem {
                id: WidgetId(i as u64 + 2000),
                label: label.to_string(),
                closable: false,
                icon: None,
            }
        }).collect();
        Self {
            state: TabControlState {
                tabs: tab_items,
                active_index: 0,
                ..Default::default()
            },
        }
    }

    pub fn add_tab(&mut self, label: &str) -> WidgetId {
        let id = WidgetId(self.state.tabs.len() as u64 + 2000);
        self.state.tabs.push(TabItem {
            id,
            label: label.to_string(),
            closable: false,
            icon: None,
        });
        id
    }

    pub fn active_tab(&self) -> Option<&str> {
        self.state.tabs.get(self.state.active_index).map(|t| t.label.as_str())
    }

    pub fn set_active(&mut self, index: usize) {
        if index < self.state.tabs.len() {
            self.state.active_index = index;
        }
    }

    pub fn close_tab(&mut self, index: usize) {
        if index < self.state.tabs.len() {
            self.state.tabs.remove(index);
            if self.state.active_index >= self.state.tabs.len() {
                self.state.active_index = self.state.tabs.len().saturating_sub(1);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Kapsayıcı Widget'lar
// ---------------------------------------------------------------------------

// --- Panel ---

/// Panel durumu
#[derive(Clone, Debug, Default)]
pub struct PanelState {
    pub title: String,
    pub closable: bool,
    pub collapsed: bool,
    pub show_title_bar: bool,
}

/// Panel davranışı
pub struct PanelBehavior {
    pub state: PanelState,
}

impl Default for PanelBehavior {
    fn default() -> Self { Self { state: PanelState::default() } }
}

impl WidgetBehavior for PanelBehavior {
    fn widget_type(&self) -> &str { "panel" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl PanelBehavior {
    pub fn new(title: &str) -> Self {
        Self {
            state: PanelState {
                title: title.to_string(),
                ..Default::default()
            },
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.state.title = title.to_string();
        self
    }

    pub fn closable(mut self) -> Self {
        self.state.closable = true;
        self
    }

    pub fn collapsible(mut self) -> Self {
        self.state.collapsed = false;
        self
    }

    pub fn no_title_bar(mut self) -> Self {
        self.state.show_title_bar = false;
        self
    }
}

// --- ScrollPanel ---

/// Kaydırma yönü
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ScrollDirection {
    #[default]
    Vertical,
    Horizontal,
    Both,
}

/// Kaydırma paneli durumu
#[derive(Clone, Debug, Default)]
pub struct ScrollPanelState {
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub scroll_direction: ScrollDirection,
    pub content_size: [i32; 2],
    pub viewport_size: [i32; 2],
    pub auto_hide_bars: bool,
}

/// Kaydırma paneli davranışı
pub struct ScrollPanelBehavior {
    pub state: ScrollPanelState,
}

impl Default for ScrollPanelBehavior {
    fn default() -> Self { Self { state: ScrollPanelState::default() } }
}

impl WidgetBehavior for ScrollPanelBehavior {
    fn widget_type(&self) -> &str { "scrollpanel" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl ScrollPanelBehavior {
    pub fn new(direction: ScrollDirection) -> Self {
        Self {
            state: ScrollPanelState {
                scroll_direction: direction,
                ..Default::default()
            },
        }
    }

    pub fn vertical() -> Self { Self::new(ScrollDirection::Vertical) }
    pub fn horizontal() -> Self { Self::new(ScrollDirection::Horizontal) }
    pub fn both() -> Self { Self::new(ScrollDirection::Both) }

    pub fn with_content_size(mut self, width: i32, height: i32) -> Self {
        self.state.content_size = [width, height];
        self
    }

    pub fn scroll_to(&mut self, x: f32, y: f32) {
        self.state.scroll_x = x;
        self.state.scroll_y = y;
    }
}

// --- SplitPanel ---

/// Bölme yönü
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SplitDirection {
    #[default]
    Horizontal,
    Vertical,
}

/// Bölme paneli durumu
#[derive(Clone, Debug, Default)]
pub struct SplitPanelState {
    pub split_ratio: f32,      // 0.0 - 1.0
    pub split_position: i32,
    pub dragging: bool,
    pub min_size: i32,
    pub direction: SplitDirection,
    pub draggable: bool,
}

/// Bölme paneli davranışı
pub struct SplitPanelBehavior {
    pub state: SplitPanelState,
}

impl Default for SplitPanelBehavior {
    fn default() -> Self { Self { state: SplitPanelState::default() } }
}

impl WidgetBehavior for SplitPanelBehavior {
    fn widget_type(&self) -> &str { "splitpanel" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl SplitPanelBehavior {
    pub fn new(direction: SplitDirection, ratio: f32) -> Self {
        Self {
            state: SplitPanelState {
                split_ratio: ratio.clamp(0.0, 1.0),
                direction,
                draggable: true,
                ..Default::default()
            },
        }
    }

    pub fn horizontal(ratio: f32) -> Self { Self::new(SplitDirection::Horizontal, ratio) }
    pub fn vertical(ratio: f32) -> Self { Self::new(SplitDirection::Vertical, ratio) }

    pub fn with_min_size(mut self, min: i32) -> Self {
        self.state.min_size = min;
        self
    }

    pub fn non_draggable(mut self) -> Self {
        self.state.draggable = false;
        self
    }
}

// --- TabPanel ---

/// Sekme paneli durumu
#[derive(Clone, Debug, Default)]
pub struct TabPanelState {
    pub tabs: Vec<TabItem>,
    pub active_index: usize,
    pub tab_position: TabPosition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TabPosition {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

/// Sekme paneli davranışı
pub struct TabPanelBehavior {
    pub state: TabPanelState,
}

impl Default for TabPanelBehavior {
    fn default() -> Self { Self { state: TabPanelState::default() } }
}

impl WidgetBehavior for TabPanelBehavior {
    fn widget_type(&self) -> &str { "tabpanel" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl TabPanelBehavior {
    pub fn new(tabs: Vec<&str>) -> Self {
        let tab_items: Vec<TabItem> = tabs.iter().enumerate().map(|(i, label)| {
            TabItem {
                id: WidgetId(i as u64 + 3000),
                label: label.to_string(),
                ..Default::default()
            }
        }).collect();
        Self {
            state: TabPanelState {
                tabs: tab_items,
                active_index: 0,
                ..Default::default()
            },
        }
    }

    pub fn with_position(mut self, pos: TabPosition) -> Self {
        self.state.tab_position = pos;
        self
    }
}

// --- Accordion ---

/// Akordeon öğesi
#[derive(Clone, Debug)]
pub struct AccordionItem {
    pub id: WidgetId,
    pub title: String,
    pub content: Vec<WidgetId>,
    pub expanded: bool,
}

/// Akordeon durumu
#[derive(Clone, Debug, Default)]
pub struct AccordionState {
    pub items: Vec<AccordionItem>,
    pub allow_multiple: bool,
}

/// Akordeon davranışı
pub struct AccordionBehavior {
    pub state: AccordionState,
}

impl Default for AccordionBehavior {
    fn default() -> Self { Self { state: AccordionState::default() } }
}

impl WidgetBehavior for AccordionBehavior {
    fn widget_type(&self) -> &str { "accordion" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl AccordionBehavior {
    pub fn new(items: Vec<(&str, Vec<WidgetId>)>) -> Self {
        let accordion_items: Vec<AccordionItem> = items.iter().enumerate().map(|(i, (title, children))| {
            AccordionItem {
                id: WidgetId(i as u64 + 4000),
                title: title.to_string(),
                content: children.clone(),
                expanded: false,
            }
        }).collect();
        Self {
            state: AccordionState {
                items: accordion_items,
                ..Default::default()
            },
        }
    }

    pub fn allow_multiple(mut self) -> Self {
        self.state.allow_multiple = true;
        self
    }

    pub fn toggle(&mut self, id: WidgetId) {
        for item in &mut self.state.items {
            if item.id == id {
                if !self.state.allow_multiple {
                    for other in &mut self.state.items {
                        if other.id != id {
                            other.expanded = false;
                        }
                    }
                }
                item.expanded = !item.expanded;
                return;
            }
        }
    }
}

// --- GroupBox ---

/// Grup kutusu durumu
#[derive(Clone, Debug, Default)]
pub struct GroupBoxState {
    pub title: String,
    pub collapsible: bool,
    pub collapsed: bool,
}

/// Grup kutusu davranışı
pub struct GroupBoxBehavior {
    pub state: GroupBoxState,
}

impl Default for GroupBoxBehavior {
    fn default() -> Self { Self { state: GroupBoxState::default() } }
}

impl WidgetBehavior for GroupBoxBehavior {
    fn widget_type(&self) -> &str { "groupbox" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl GroupBoxBehavior {
    pub fn new(title: &str) -> Self {
        Self {
            state: GroupBoxState {
                title: title.to_string(),
                ..Default::default()
            },
        }
    }

    pub fn collapsible(mut self) -> Self {
        self.state.collapsible = true;
        self
    }
}

// ---------------------------------------------------------------------------
// 3. Veri Widget'ları
// ---------------------------------------------------------------------------

// --- PropertyGrid ---

/// Özellik türü
#[derive(Clone, Debug, PartialEq)]
pub enum PropertyType {
    Bool,
    Int { min: i32, max: i32 },
    Float { min: f32, max: f32, step: f32 },
    String,
    Color,
    Enum { options: Vec<String> },
    Vector2,
    Vector3,
    Vector4,
    Quaternion,
    AssetRef { extensions: Vec<String> },
}

/// Özellik editörü
#[derive(Clone, Debug)]
pub struct PropertyEditor {
    pub property_type: PropertyType,
    pub editor: Option<WidgetId>,
}

/// Özellik öğesi
#[derive(Clone, Debug)]
pub struct PropertyItem {
    pub name: String,
    pub category: String,
    pub property_type: PropertyType,
    pub value: PropertyValue,
    pub editor: Option<WidgetId>,
}

/// Özellik değeri
#[derive(Clone, Debug)]
pub enum PropertyValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
    Color(ThemeColor),
    Enum(usize),
    Vector2([f32; 2]),
    Vector3([f32; 3]),
    Vector4([f32; 4]),
    Quaternion([f32; 4]), // x, y, z, w
    AssetRef(String),
    None,
}

impl Default for PropertyValue {
    fn default() -> Self { Self::None }
}

/// Özellık ızgarası durumu
#[derive(Clone, Debug, Default)]
pub struct PropertyGridState {
    pub properties: Vec<PropertyItem>,
    pub expanded_categories: Vec<String>,
    pub selected_property: Option<usize>,
    pub filter_text: String,
}

/// Özellık ızgarası davranışı
pub struct PropertyGridBehavior {
    pub state: PropertyGridState,
}

impl Default for PropertyGridBehavior {
    fn default() -> Self { Self { state: PropertyGridState::default() } }
}

impl WidgetBehavior for PropertyGridBehavior {
    fn widget_type(&self) -> &str { "propertygrid" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl PropertyGridBehavior {
    pub fn new() -> Self { Self::default() }

    pub fn add_property(&mut self, name: &str, category: &str, property_type: PropertyType, value: PropertyValue) {
        self.state.properties.push(PropertyItem {
            name: name.to_string(),
            category: category.to_string(),
            property_type,
            value,
            editor: None,
        });
    }

    pub fn bool(&mut self, name: &str, category: &str, value: bool) {
        self.add_property(name, category, PropertyType::Bool, PropertyValue::Bool(value));
    }

    pub fn int(&mut self, name: &str, category: &str, value: i32, min: i32, max: i32) {
        self.add_property(name, category, PropertyType::Int { min, max }, PropertyValue::Int(value));
    }

    pub fn float(&mut self, name: &str, category: &str, value: f32, min: f32, max: f32, step: f32) {
        self.add_property(name, category, PropertyType::Float { min, max, step }, PropertyValue::Float(value));
    }

    pub fn text(&mut self, name: &str, category: &str, value: &str) {
        self.add_property(name, category, PropertyType::String, PropertyValue::String(value.to_string()));
    }

    pub fn color(&mut self, name: &str, category: &str, color: ThemeColor) {
        self.add_property(name, category, PropertyType::Color, PropertyValue::Color(color));
    }

    pub fn vec2(&mut self, name: &str, category: &str, value: [f32; 2]) {
        self.add_property(name, category, PropertyType::Vector2, PropertyValue::Vector2(value));
    }

    pub fn vec3(&mut self, name: &str, category: &str, value: [f32; 3]) {
        self.add_property(name, category, PropertyType::Vector3, PropertyValue::Vector3(value));
    }

    pub fn vec4(&mut self, name: &str, category: &str, value: [f32; 4]) {
        self.add_property(name, category, PropertyType::Vector4, PropertyValue::Vector4(value));
    }

    pub fn quat(&mut self, name: &str, category: &str, value: [f32; 4]) {
        self.add_property(name, category, PropertyType::Quaternion, PropertyValue::Quaternion(value));
    }
}

// --- Curve Editor ---

/// Eğri noktası türü
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CurvePointType {
    Linear,
    Bezier,
    Step,
}

/// Eğri noktası
#[derive(Clone, Debug)]
pub struct CurvePoint {
    pub time: f32,
    pub value: f32,
    pub point_type: CurvePointType,
    pub tangent_in: Option<[f32; 2]>,
    pub tangent_out: Option<[f32; 2]>,
}

/// Animasyon eğrisi
#[derive(Clone, Debug, Default)]
pub struct AnimationCurve {
    pub points: Vec<CurvePoint>,
    pub pre_infinity: CurveInfinity,
    pub post_infinity: CurveInfinity,
    pub color: ThemeColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CurveInfinity {
    #[default]
    Constant,
    Linear,
    Cycle,
    CycleOffset,
    Oscillate,
}

/// Eğri editörü durumu
#[derive(Clone, Debug, Default)]
pub struct CurveEditorState {
    pub curves: Vec<AnimationCurve>,
    pub selected_curve: Option<usize>,
    pub selected_point: Option<(usize, usize)>,
    pub zoom: f32,
    pub pan: [f32; 2],
    pub grid_visible: bool,
    pub snap_to_grid: bool,
}

/// Eğri editörü davranışı
pub struct CurveEditorBehavior {
    pub state: CurveEditorState,
}

impl Default for CurveEditorBehavior {
    fn default() -> Self { Self { state: CurveEditorState::default() } }
}

impl WidgetBehavior for CurveEditorBehavior {
    fn widget_type(&self) -> &str { "curveeditor" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl CurveEditorBehavior {
    pub fn new() -> Self { Self::default() }

    pub fn add_curve(&mut self, color: ThemeColor) -> usize {
        let idx = self.state.curves.len();
        self.state.curves.push(AnimationCurve {
            points: Vec::new(),
            pre_infinity: CurveInfinity::Constant,
            post_infinity: CurveInfinity::Constant,
            color,
        });
        idx
    }

    pub fn add_point(&mut self, curve_idx: usize, point: CurvePoint) {
        if let Some(curve) = self.state.curves.get_mut(curve_idx) {
            curve.points.push(point);
            curve.points.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
        }
    }
}

// --- Color Picker ---

/// Renk modeli
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorModel {
    #[default]
    RGB,
    HSL,
    HSV,
}

/// Renk seçici durumu
#[derive(Clone, Debug, Default)]
pub struct ColorPickerState {
    pub color: ThemeColor,
    pub color_model: ColorModel,
    pub show_alpha: bool,
    pub show_palette: bool,
    pub palette: Vec<ThemeColor>,
    pub recent_colors: Vec<ThemeColor>,
}

/// Renk seçici davranışı
pub struct ColorPickerBehavior {
    pub state: ColorPickerState,
}

impl Default for ColorPickerBehavior {
    fn default() -> Self { Self { state: ColorPickerState::default() } }
}

impl WidgetBehavior for ColorPickerBehavior {
    fn widget_type(&self) -> &str { "colorpicker" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl ColorPickerBehavior {
    pub fn new(color: ThemeColor) -> Self {
        Self {
            state: ColorPickerState {
                color,
                ..Default::default()
            },
        }
    }

    pub fn with_model(mut self, model: ColorModel) -> Self {
        self.state.color_model = model;
        self
    }

    pub fn with_alpha(mut self) -> Self {
        self.state.show_alpha = true;
        self
    }

    pub fn with_palette(mut self, palette: Vec<ThemeColor>) -> Self {
        self.state.palette = palette;
        self.state.show_palette = true;
        self
    }
}

// --- Vector Editor ---

/// Vektör editörü durumu
#[derive(Clone, Debug, Default)]
pub struct VectorEditorState {
    pub value: Vec<f32>, // 2, 3 veya 4 eleman
    pub min: Vec<f32>,
    pub max: Vec<f32>,
    pub step: Vec<f32>,
    pub labels: Vec<String>,
}

/// Vektör editörü davranışı
pub struct VectorEditorBehavior {
    pub state: VectorEditorState,
}

impl Default for VectorEditorBehavior {
    fn default() -> Self { Self { state: VectorEditorState::default() } }
}

impl WidgetBehavior for VectorEditorBehavior {
    fn widget_type(&self) -> &str { "vectoreditor" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl VectorEditorBehavior {
    pub fn vec2(x: f32, y: f32) -> Self {
        Self {
            state: VectorEditorState {
                value: vec![x, y],
                labels: vec!["X".to_string(), "Y".to_string()],
                ..Default::default()
            },
        }
    }

    pub fn vec3(x: f32, y: f32, z: f32) -> Self {
        Self {
            state: VectorEditorState {
                value: vec![x, y, z],
                labels: vec!["X".to_string(), "Y".to_string(), "Z".to_string()],
                ..Default::default()
            },
        }
    }

    pub fn vec4(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self {
            state: VectorEditorState {
                value: vec![x, y, z, w],
                labels: vec!["X".to_string(), "Y".to_string(), "Z".to_string(), "W".to_string()],
                ..Default::default()
            },
        }
    }
}

// --- Quaternion Editor ---

/// Kuaterniyon editörü durumu
#[derive(Clone, Debug, Default)]
pub struct QuaternionEditorState {
    pub value: [f32; 4], // x, y, z, w
    pub euler_mode: bool,
    pub euler_order: EulerOrder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EulerOrder {
    #[default]
    XYZ,
    XZY,
    YXZ,
    YZX,
    ZXY,
    ZYX,
}

/// Kuaterniyon editörü davranışı
pub struct QuaternionEditorBehavior {
    pub state: QuaternionEditorState,
}

impl Default for QuaternionEditorBehavior {
    fn default() -> Self { Self { state: QuaternionEditorState::default() } }
}

impl WidgetBehavior for QuaternionEditorBehavior {
    fn widget_type(&self) -> &str { "quaternioneditor" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl QuaternionEditorBehavior {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self {
            state: QuaternionEditorState {
                value: [x, y, z, w],
                euler_mode: true,
                ..Default::default()
            },
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Gelişmiş Widget'lar
// ---------------------------------------------------------------------------

// --- Dock System ---

/// Dock konumu
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DockArea {
    #[default]
    Center,
    Left,
    Right,
    Top,
    Bottom,
    Float,
    AutoHideLeft,
    AutoHideRight,
    AutoHideTop,
    AutoHideBottom,
}

/// Dock düğümü
#[derive(Clone, Debug)]
pub struct DockNode {
    pub id: WidgetId,
    pub area: DockArea,
    pub children: Vec<DockNode>,
    pub split_ratio: f32,
    pub split_direction: SplitDirection,
    pub visible: bool,
    pub floating_rect: Option<[i32; 4]>,
    pub auto_hide_size: i32,
    pub title: String,
}

/// Dock sistemi durumu
#[derive(Clone, Debug, Default)]
pub struct DockSystemState {
    pub nodes: Vec<DockNode>,
    pub root: Option<DockNode>,
    pub dragged_tab: Option<WidgetId>,
    pub drop_target: Option<DockArea>,
    pub floating_windows: Vec<DockNode>,
    pub active_dock_area: DockArea,
}

/// Dock sistemi davranışı
pub struct DockSystemBehavior {
    pub state: DockSystemState,
}

impl Default for DockSystemBehavior {
    fn default() -> Self { Self { state: DockSystemState::default() } }
}

impl WidgetBehavior for DockSystemBehavior {
    fn widget_type(&self) -> &str { "docksystem" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl DockSystemBehavior {
    pub fn new() -> Self { Self::default() }

    pub fn dock_widget(&mut self, widget_id: WidgetId, area: DockArea) {
        let node = DockNode {
            id: widget_id,
            area,
            children: Vec::new(),
            split_ratio: 0.5,
            split_direction: SplitDirection::Horizontal,
            visible: true,
            floating_rect: None,
            auto_hide_size: 24,
            title: String::new(),
        };
        self.state.nodes.push(node);
    }

    pub fn float_widget(&mut self, widget_id: WidgetId, rect: [i32; 4], title: &str) {
        let node = DockNode {
            id: widget_id,
            area: DockArea::Float,
            children: Vec::new(),
            split_ratio: 0.5,
            split_direction: SplitDirection::Horizontal,
            visible: true,
            floating_rect: Some(rect),
            auto_hide_size: 24,
            title: title.to_string(),
        };
        self.state.floating_windows.push(node);
    }
}

// --- Menu Bar ---

/// Menü öğesi
#[derive(Clone, Debug)]
pub struct MenuItem {
    pub id: WidgetId,
    pub label: String,
    pub shortcut: Option<String>,
    pub enabled: bool,
    pub checked: bool,
    pub children: Vec<MenuItem>,
    pub action: Option<String>,
}

/// Menü çubuğu durumu
#[derive(Clone, Debug, Default)]
pub struct MenuBarState {
    pub menus: Vec<MenuItem>,
    pub open_menu: Option<WidgetId>,
    pub active_item: Option<WidgetId>,
    pub menu_bar_height: i32,
}

/// Menü çubuğu davranışı
pub struct MenuBarBehavior {
    pub state: MenuBarState,
}

impl Default for MenuBarBehavior {
    fn default() -> Self { Self { state: MenuBarState::default() } }
}

impl WidgetBehavior for MenuBarBehavior {
    fn widget_type(&self) -> &str { "menubar" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl MenuBarBehavior {
    pub fn new() -> Self { Self::default() }

    pub fn add_menu(&mut self, label: &str) -> WidgetId {
        let id = WidgetId(self.state.menus.len() as u64 + 5000);
        self.state.menus.push(MenuItem {
            id,
            label: label.to_string(),
            shortcut: None,
            enabled: true,
            checked: false,
            children: Vec::new(),
            action: None,
        });
        id
    }

    pub fn add_menu_item(&mut self, menu_id: WidgetId, label: &str, shortcut: Option<&str>) -> WidgetId {
        if let Some(menu) = self.state.menus.iter_mut().find(|m| m.id == menu_id) {
            let item_id = WidgetId(menu.children.len() as u64 + 5100);
            menu.children.push(MenuItem {
                id: item_id,
                label: label.to_string(),
                shortcut: shortcut.map(|s| s.to_string()),
                enabled: true,
                checked: false,
                children: Vec::new(),
                action: Some(label.to_string()),
            });
            item_id
        } else {
            WidgetId::INVALID
        }
    }
}

// --- Toolbar ---

/// Araç çubuğu öğesi
#[derive(Clone, Debug)]
pub struct ToolbarItem {
    pub id: WidgetId,
    pub label: String,
    pub icon: Option<String>,
    pub tooltip: Option<String>,
    pub enabled: bool,
    pub toggled: bool,
    pub separator_before: bool,
    pub separator_after: bool,
    pub action: Option<String>,
}

/// Araç çubuğu durumu
#[derive(Clone, Debug, Default)]
pub struct ToolbarState {
    pub items: Vec<ToolbarItem>,
    pub orientation: LayoutDirection,
    pub button_size: i32,
    pub wrap: bool,
}

/// Araç çubuğu davranışı
pub struct ToolbarBehavior {
    pub state: ToolbarState,
}

impl Default for ToolbarBehavior {
    fn default() -> Self { Self { state: ToolbarState::default() } }
}

impl WidgetBehavior for ToolbarBehavior {
    fn widget_type(&self) -> &str { "toolbar" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl ToolbarBehavior {
    pub fn new() -> Self { Self::default() }

    pub fn horizontal() -> Self {
        Self {
            state: ToolbarState {
                orientation: LayoutDirection::Horizontal,
                ..Default::default()
            },
        }
    }

    pub fn vertical() -> Self {
        Self {
            state: ToolbarState {
                orientation: LayoutDirection::Vertical,
                ..Default::default()
            },
        }
    }

    pub fn add_button(&mut self, label: &str, tooltip: Option<&str>) -> WidgetId {
        let id = WidgetId(self.state.items.len() as u64 + 6000);
        self.state.items.push(ToolbarItem {
            id,
            label: label.to_string(),
            icon: None,
            tooltip: tooltip.map(|s| s.to_string()),
            enabled: true,
            toggled: false,
            separator_before: false,
            separator_after: false,
            action: Some(label.to_string()),
        });
        id
    }

    pub fn add_separator(&mut self) {
        if let Some(last) = self.state.items.last_mut() {
            last.separator_after = true;
        }
        self.state.items.push(ToolbarItem {
            id: WidgetId::INVALID,
            label: String::new(),
            icon: None,
            tooltip: None,
            enabled: false,
            toggled: false,
            separator_before: true,
            separator_after: false,
            action: None,
        });
    }
}

// --- Status Bar ---

/// Durum çubuğu öğesi
#[derive(Clone, Debug)]
pub struct StatusBarItem {
    pub id: WidgetId,
    pub text: String,
    pub width: i32,
    pub alignment: StatusAlignment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StatusAlignment {
    #[default]
    Left,
    Center,
    Right,
}

/// Durum çubuğu durumu
#[derive(Clone, Debug, Default)]
pub struct StatusBarState {
    pub items: Vec<StatusBarItem>,
    pub height: i32,
}

/// Durum çubuğu davranışı
pub struct StatusBarBehavior {
    pub state: StatusBarState,
}

impl Default for StatusBarBehavior {
    fn default() -> Self { Self { state: StatusBarState::default() } }
}

impl WidgetBehavior for StatusBarBehavior {
    fn widget_type(&self) -> &str { "statusbar" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl StatusBarBehavior {
    pub fn new(height: i32) -> Self {
        Self {
            state: StatusBarState { height, ..Default::default() },
        }
    }

    pub fn add_item(&mut self, text: &str, width: i32) -> WidgetId {
        let id = WidgetId(self.state.items.len() as u64 + 7000);
        self.state.items.push(StatusBarItem {
            id,
            text: text.to_string(),
            width,
            alignment: StatusAlignment::Left,
        });
        id
    }

    pub fn set_item_text(&mut self, id: WidgetId, text: &str) {
        if let Some(item) = self.state.items.iter_mut().find(|i| i.id == id) {
            item.text = text.to_string();
        }
    }
}

// --- Progress Bar ---

/// İlerleme çubuğu durumu
#[derive(Clone, Debug, Default)]
pub struct ProgressBarState {
    pub progress: f32,
    pub min: f32,
    pub max: f32,
    pub indeterminate: bool,
    pub indeterminate_offset: f32,
    pub show_text: bool,
    pub text_format: Option<String>,
}

/// İlerleme çubuğu davranışı
pub struct ProgressBarBehavior {
    pub state: ProgressBarState,
}

impl Default for ProgressBarBehavior {
    fn default() -> Self { Self { state: ProgressBarState::default() } }
}

impl WidgetBehavior for ProgressBarBehavior {
    fn widget_type(&self) -> &str { "progressbar" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl ProgressBarBehavior {
    pub fn new(progress: f32, min: f32, max: f32) -> Self {
        Self {
            state: ProgressBarState {
                progress: ((progress - min) / (max - min)).clamp(0.0, 1.0),
                min, max,
                ..Default::default()
            },
        }
    }

    pub fn indeterminate() -> Self {
        Self {
            state: ProgressBarState {
                indeterminate: true,
                ..Default::default()
            },
        }
    }

    pub fn with_text(mut self, format: &str) -> Self {
        self.state.show_text = true;
        self.state.text_format = Some(format.to_string());
        self
    }

    pub fn set_progress(&mut self, progress: f32) {
        self.state.progress = ((progress - self.state.min) / (self.state.max - self.state.min)).clamp(0.0, 1.0);
    }
}

// --- Spinner ---

/// Yükleme göstergesi durumu
#[derive(Clone, Debug, Default)]
pub struct SpinnerState {
    pub angle: f32,
    pub speed: f32,
    pub size: i32,
    pub color: ThemeColor,
    pub visible: bool,
}

/// Yükleme göstergesi davranışı
pub struct SpinnerBehavior {
    pub state: SpinnerState,
}

impl Default for SpinnerBehavior {
    fn default() -> Self { Self { state: SpinnerState::default() } }
}

impl WidgetBehavior for SpinnerBehavior {
    fn widget_type(&self) -> &str { "spinner" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl SpinnerBehavior {
    pub fn new(size: i32) -> Self {
        Self {
            state: SpinnerState {
                size,
                color: DarkTheme::TEXT_ACCENT,
                visible: true,
                ..Default::default()
            },
        }
    }

    pub fn with_color(mut self, color: ThemeColor) -> Self {
        self.state.color = color;
        self
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.state.speed = speed;
        self
    }
}

// --- Tooltip ---

/// İpucu durumu
#[derive(Clone, Debug, Default)]
pub struct TooltipState {
    pub text: String,
    pub title: Option<String>,
    pub visible: bool,
    pub delay_ms: u32,
    pub max_width: Option<i32>,
    pub position: TooltipPosition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TooltipPosition {
    #[default]
    Auto,
    Top,
    Bottom,
    Left,
    Right,
}

/// İpucu davranışı
pub struct TooltipBehavior {
    pub state: TooltipState,
}

impl Default for TooltipBehavior {
    fn default() -> Self { Self { state: TooltipState::default() } }
}

impl WidgetBehavior for TooltipBehavior {
    fn widget_type(&self) -> &str { "tooltip" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl TooltipBehavior {
    pub fn new(text: &str) -> Self {
        Self {
            state: TooltipState {
                text: text.to_string(),
                visible: false,
                ..Default::default()
            },
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.state.title = Some(title.to_string());
        self
    }

    pub fn with_delay(mut self, ms: u32) -> Self {
        self.state.delay_ms = ms;
        self
    }
}

// --- Context Menu ---

/// Bağlam menüsü öğesi
#[derive(Clone, Debug)]
pub struct ContextMenuItem {
    pub id: WidgetId,
    pub label: String,
    pub shortcut: Option<String>,
    pub enabled: bool,
    pub checked: bool,
    pub separator: bool,
    pub submenu: Vec<ContextMenuItem>,
    pub action: Option<String>,
}

/// Bağlam menüsü durumu
#[derive(Clone, Debug, Default)]
pub struct ContextMenuState {
    pub items: Vec<ContextMenuItem>,
    pub position: [i32; 2],
    pub visible: bool,
    pub active_item: Option<WidgetId>,
}

/// Bağlam menüsü davranışı
pub struct ContextMenuBehavior {
    pub state: ContextMenuState,
}

impl Default for ContextMenuBehavior {
    fn default() -> Self { Self { state: ContextMenuState::default() } }
}

impl WidgetBehavior for ContextMenuBehavior {
    fn widget_type(&self) -> &str { "contextmenu" }

    fn update(&self, _id: WidgetId, _rect: [i32; 4], _mouse_pos: [f32; 2]) -> WidgetUpdateResult {
        WidgetUpdateResult { hovered: false, changed: false }
    }

    fn handle_click(&self, id: WidgetId) -> Option<UiEvent> {
        Some(UiEvent::Click { id })
    }
}

impl ContextMenuBehavior {
    pub fn new() -> Self { Self::default() }

    pub fn at(mut self, x: i32, y: i32) -> Self {
        self.state.position = [x, y];
        self.state.visible = true;
        self
    }

    pub fn add_item(&mut self, label: &str) -> WidgetId {
        let id = WidgetId(self.state.items.len() as u64 + 8000);
        self.state.items.push(ContextMenuItem {
            id,
            label: label.to_string(),
            shortcut: None,
            enabled: true,
            checked: false,
            separator: false,
            submenu: Vec::new(),
            action: Some(label.to_string()),
        });
        id
    }

    pub fn add_separator(&mut self) {
        self.state.items.push(ContextMenuItem {
            id: WidgetId::INVALID,
            label: String::new(),
            shortcut: None,
            enabled: false,
            checked: false,
            separator: true,
            submenu: Vec::new(),
            action: None,
        });
    }

    pub fn show(&mut self, x: i32, y: i32) {
        self.state.position = [x, y];
        self.state.visible = true;
    }

    pub fn hide(&mut self) {
        self.state.visible = false;
    }
}

// ---------------------------------------------------------------------------
// 5. Layout Yardımcıları
// ---------------------------------------------------------------------------

// Flex, Grid, Stack, Canvas, Dock layout'ları için
// yüksek seviyeli builder API'ları.

// --- Flex Layout ---

/// Flex yönü
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

/// Flex hizlama
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FlexAlign {
    #[default]
    Start,
    Center,
    End,
    Stretch,
    SpaceBetween,
    SpaceAround,
}

/// Flex öğesi
#[derive(Clone, Debug)]
pub struct FlexItem {
    pub widget_id: WidgetId,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Option<i32>,
    pub align_self: Option<FlexAlign>,
    pub margin: [i32; 4],
}

/// Flex layout tanımı
#[derive(Clone, Debug, Default)]
pub struct FlexLayout {
    pub direction: FlexDirection,
    pub wrap: bool,
    pub gap: i32,
    pub justify_content: FlexAlign,
    pub align_items: FlexAlign,
    pub padding: [i32; 4],
    pub items: Vec<FlexItem>,
}

impl FlexLayout {
    pub fn new(direction: FlexDirection) -> Self {
        Self {
            direction,
            ..Default::default()
        }
    }

    pub fn row() -> Self { Self::new(FlexDirection::Row) }
    pub fn column() -> Self { Self::new(FlexDirection::Column) }

    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    pub fn gap(mut self, gap: i32) -> Self {
        self.gap = gap;
        self
    }

    pub fn justify(mut self, justify: FlexAlign) -> Self {
        self.justify_content = justify;
        self
    }

    pub fn align(mut self, align: FlexAlign) -> Self {
        self.align_items = align;
        self
    }

    pub fn padding(mut self, padding: [i32; 4]) -> Self {
        self.padding = padding;
        self
    }

    pub fn item(mut self, widget_id: WidgetId) -> Self {
        self.items.push(FlexItem {
            widget_id,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: None,
            align_self: None,
            margin: [0; 4],
        });
        self
    }

    pub fn item_flex(mut self, widget_id: WidgetId, grow: f32) -> Self {
        self.items.push(FlexItem {
            widget_id,
            flex_grow: grow,
            flex_shrink: 1.0,
            flex_basis: None,
            align_self: None,
            margin: [0; 4],
        });
        self
    }
}

// --- Grid Layout ---

/// Izgara öğesi
#[derive(Clone, Debug)]
pub struct GridItem {
    pub widget_id: WidgetId,
    pub row: usize,
    pub col: usize,
    pub row_span: usize,
    pub col_span: usize,
    pub align: FlexAlign,
}

/// Izgara layout tanımı
#[derive(Clone, Debug, Default)]
pub struct GridLayout {
    pub columns: usize,
    pub rows: usize,
    pub gap: i32,
    pub padding: [i32; 4],
    pub items: Vec<GridItem>,
    pub column_widths: Vec<i32>,
    pub row_heights: Vec<i32>,
}

impl GridLayout {
    pub fn new(columns: usize, rows: usize) -> Self {
        Self {
            columns,
            rows,
            ..Default::default()
        }
    }

    pub fn gap(mut self, gap: i32) -> Self {
        self.gap = gap;
        self
    }

    pub fn padding(mut self, padding: [i32; 4]) -> Self {
        self.padding = padding;
        self
    }

    pub fn at(mut self, widget_id: WidgetId, row: usize, col: usize) -> Self {
        self.items.push(GridItem {
            widget_id,
            row,
            col,
            row_span: 1,
            col_span: 1,
            align: FlexAlign::Stretch,
        });
        self
    }

    pub fn span(mut self, widget_id: WidgetId, row: usize, col: usize, row_span: usize, col_span: usize) -> Self {
        self.items.push(GridItem {
            widget_id,
            row,
            col,
            row_span,
            col_span,
            align: FlexAlign::Stretch,
        });
        self
    }
}

// --- Stack Layout ---

/// Yığın layout tanımı
#[derive(Clone, Debug, Default)]
pub struct StackLayout {
    pub items: Vec<StackItem>,
}

#[derive(Clone, Debug)]
pub struct StackItem {
    pub widget_id: WidgetId,
    pub alignment: FlexAlign,
    pub margin: [i32; 4],
}

impl StackLayout {
    pub fn new() -> Self { Self::default() }

    pub fn add(mut self, widget_id: WidgetId) -> Self {
        self.items.push(StackItem {
            widget_id,
            alignment: FlexAlign::Start,
            margin: [0; 4],
        });
        self
    }

    pub fn centered(mut self, widget_id: WidgetId) -> Self {
        self.items.push(StackItem {
            widget_id,
            alignment: FlexAlign::Center,
            margin: [0; 4],
        });
        self
    }
}

// --- Canvas Layout ---

/// Canvas pozisyonu
#[derive(Clone, Copy, Debug, Default)]
pub struct CanvasPosition {
    pub x: i32,
    pub y: i32,
    pub anchor: CanvasAnchor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CanvasAnchor {
    #[default]
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// Canvas layout tanımı
#[derive(Clone, Debug, Default)]
pub struct CanvasLayout {
    pub items: Vec<CanvasItem>,
}

#[derive(Clone, Debug)]
pub struct CanvasItem {
    pub widget_id: WidgetId,
    pub position: CanvasPosition,
    pub size: Option<[i32; 2]>,
    pub z_index: i32,
}

impl CanvasLayout {
    pub fn new() -> Self { Self::default() }

    pub fn add(mut self, widget_id: WidgetId, x: i32, y: i32) -> Self {
        self.items.push(CanvasItem {
            widget_id,
            position: CanvasPosition { x, y, ..Default::default() },
            size: None,
            z_index: 0,
        });
        self
    }

    pub fn sized(mut self, widget_id: WidgetId, x: i32, y: i32, w: i32, h: i32) -> Self {
        self.items.push(CanvasItem {
            widget_id,
            position: CanvasPosition { x, y, ..Default::default() },
            size: Some([w, h]),
            z_index: 0,
        });
        self
    }
}

// --- Dock Layout ---

/// Dock kenarı
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DockEdge {
    #[default]
    Left,
    Right,
    Top,
    Bottom,
    Center,
    Fill,
}

/// Dock layout tanımı
#[derive(Clone, Debug, Default)]
pub struct DockLayout {
    pub items: Vec<DockItem>,
}

#[derive(Clone, Debug)]
pub struct DockItem {
    pub widget_id: WidgetId,
    pub edge: DockEdge,
    pub size: i32,
    pub priority: i32,
}

impl DockLayout {
    pub fn new() -> Self { Self::default() }

    pub fn dock(mut self, widget_id: WidgetId, edge: DockEdge, size: i32) -> Self {
        self.items.push(DockItem {
            widget_id,
            edge,
            size,
            priority: 0,
        });
        self
    }

    pub fn dock_left(mut self, widget_id: WidgetId, size: i32) -> Self {
        self.dock(widget_id, DockEdge::Left, size)
    }

    pub fn dock_right(mut self, widget_id: WidgetId, size: i32) -> Self {
        self.dock(widget_id, DockEdge::Right, size)
    }

    pub fn dock_top(mut self, widget_id: WidgetId, size: i32) -> Self {
        self.dock(widget_id, DockEdge::Top, size)
    }

    pub fn dock_bottom(mut self, widget_id: WidgetId, size: i32) -> Self {
        self.dock(widget_id, DockEdge::Bottom, size)
    }

    pub fn center(mut self, widget_id: WidgetId) -> Self {
        self.dock(widget_id, DockEdge::Center, 0)
    }

    pub fn fill(mut self, widget_id: WidgetId) -> Self {
        self.dock(widget_id, DockEdge::Fill, 0)
    }
}

// ---------------------------------------------------------------------------
// 6. Tema Yardımcıları
// ---------------------------------------------------------------------------

/// CSS-benzeri seçici
#[derive(Clone, Debug, Default)]
pub struct StyleSelector {
    pub widget_type: Option<String>,
    pub class: Option<String>,
    pub id: Option<WidgetId>,
    pub parent_class: Option<String>,
}

/// CSS-benzeri kural
#[derive(Clone, Debug, Default)]
pub struct StyleRule {
    pub selector: StyleSelector,
    pub style: WidgetStyle,
    pub priority: u32,
}

/// Tema sınıfı
#[derive(Clone, Debug, Default)]
pub struct ThemeClass {
    pub name: String,
    pub style: WidgetStyle,
    pub inherits: Vec<String>,
}

/// Tema kaydı
#[derive(Clone, Debug, Default)]
pub struct ThemeRecord {
    pub name: String,
    pub is_dark: bool,
    pub classes: Vec<ThemeClass>,
    pub rules: Vec<StyleRule>,
}

impl ThemeRecord {
    pub fn dark(name: &str) -> Self {
        Self {
            name: name.to_string(),
            is_dark: true,
            ..Default::default()
        }
    }

    pub fn light(name: &str) -> Self {
        Self {
            name: name.to_string(),
            is_dark: false,
            ..Default::default()
        }
    }

    pub fn add_class(mut self, class: ThemeClass) -> Self {
        self.classes.push(class);
        self
    }

    pub fn add_rule(mut self, rule: StyleRule) -> Self {
        self.rules.push(rule);
        self
    }
}

// ---------------------------------------------------------------------------
// Widget Oluşturucu Yardımcıları
// ---------------------------------------------------------------------------

/// Panel widget'ı oluştur
pub fn make_panel_widget(id: WidgetId, x: i32, y: i32, w: i32, h: i32, title: &str) -> Widget {
    let mut widget = crate::types::make_panel(id, x, y, w, h);
    widget.state.text_value = Some(title.to_string());
    widget.widget_type = WidgetType::Panel;
    widget
}

/// Sekme denetimi widget'ları oluştur
pub fn make_tabs_widget(id: WidgetId, x: i32, y: i32, w: i32, h: i32, tabs: &[&str]) -> Vec<Widget> {
    let mut widgets = Vec::new();
    let tab_height = 24;
    let tab_w = w / tabs.len().max(1) as i32;

    for (i, &label) in tabs.iter().enumerate() {
        let tab_id = WidgetId(id.0 + i as u64);
        let tab_type = WidgetType::Tab;
        let tab = Widget {
            id: tab_id,
            widget_type: tab_type,
            rect: [x + i as i32 * tab_w, y, tab_w, tab_height],
            visible: true,
            interactive: true,
            tooltip: None,
            parent: Some(id),
            children: Vec::new(),
            state: WidgetState {
                text_value: Some(label.to_string()),
                ..Default::default()
            },
        };
        widgets.push(tab);
    }
    widgets
}

// ---------------------------------------------------------------------------
// Ağaç Düğümü Yardımcıları (mevcut TreeNode korunuyor)
// ---------------------------------------------------------------------------

/// Ağaç düğümü oluştur
pub fn make_tree_node(id: WidgetId, label: &str) -> TreeNode {
    TreeNode {
        id,
        label: label.to_string(),
        children: Vec::new(),
        expanded: false,
        depth: 0,
    }
}

impl TreeNode {
    pub fn add_child(&mut self, child: TreeNode) {
        let mut child = child;
        child.depth = self.depth + 1;
        self.children.push(child);
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    pub fn with_children(mut self, children: Vec<TreeNode>) -> Self {
        self.children = children;
        self
    }

    pub fn expanded(mut self) -> Self {
        self.expanded = true;
        self
    }
}

// ---------------------------------------------------------------------------
// Widget Çizim Yardımcıları
// ---------------------------------------------------------------------------

/// Basit widget çizim komutları üret
pub fn draw_widget_commands(
    widget: &Widget,
    style: &WidgetStyle,
    draw_list: &mut DrawList,
) {
    let [x, y, w, h] = widget.rect;

    // Arka plan
    if style.bg_color.a > 0 {
        draw_list.push_rect(x, y, w, h, color_from_theme(style.bg_color));
    }

    // Köşe yuvarlaklığı (basit dikdörtgen şimdilik)
    if style.border_style != BorderStyle::None {
        let border_color = color_from_theme(style.border_color);
        let thickness = match style.border_style {
            BorderStyle::Thin => 1,
            BorderStyle::Thick => 2,
            BorderStyle::Dashed => 1,
            BorderStyle::None => 0,
        };
        if thickness > 0 {
            draw_list.push_border(x, y, w, h, border_color);
        }
    }

    // Metin
    if let Some(ref text) = widget.state.text_value {
        if !text.is_empty() {
            draw_list.push_text(
                x + style.padding[3],
                y + style.padding[0],
                text,
                color_from_theme(style.text_color),
                13,
            );
        }
    }
}

/// ThemeColor'dan draw Color'a çevir
pub fn color_from_theme(theme: ThemeColor) -> Color {
    Color::new(theme.r, theme.g, theme.b, theme.a)
}

/// ThemeColor'dan [u8; 4] dönüştür
pub fn theme_color_to_rgba(theme: ThemeColor) -> [u8; 4] {
    [theme.r, theme.g, theme.b, theme.a]
}

// ---------------------------------------------------------------------------
// Testler
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::WidgetId;

    #[test]
    fn test_button_behavior() {
        let b = ButtonBehavior::default();
        let id = WidgetId(1);
        let result = b.update(id, [0, 0, 100, 30], [50.0, 15.0]);
        assert!(result.hovered);
        let result = b.update(id, [0, 0, 100, 30], [200.0, 200.0]);
        assert!(!result.hovered);
    }

    #[test]
    fn test_checkbox_behavior() {
        let b = CheckboxBehavior;
        let result = b.handle_click(WidgetId(1));
        assert!(result.is_some());
    }

    #[test]
    fn test_slider_creation() {
        let slider = SliderBehavior::new(0.0, 1.0, 0.5);
        assert_eq!(slider.state.min, 0.0);
        assert_eq!(slider.state.max, 1.0);
        assert_eq!(slider.state.value, 0.5);
    }

    #[test]
    fn test_text_input() {
        let mut input = TextInputBehavior::single_line();
        input.set_text("Hello");
        assert_eq!(input.text(), "Hello");
    }

    #[test]
    fn test_dropdown() {
        let mut dd = DropdownBehavior::new(vec!["A", "B", "C"], 0);
        assert_eq!(dd.selected(), Some("A"));
        dd.select(1);
        assert_eq!(dd.selected(), Some("B"));
    }

    #[test]
    fn test_listbox() {
        let lb = ListBoxBehavior::new(vec!["Item1", "Item2", "Item3"]);
        assert_eq!(lb.state.items.len(), 3);
    }

    #[test]
    fn test_tab_control() {
        let mut tabs = TabControlBehavior::new(vec!["Tab1", "Tab2"]);
        assert_eq!(tabs.active_tab(), Some("Tab1"));
        tabs.set_active(1);
        assert_eq!(tabs.active_tab(), Some("Tab2"));
    }

    #[test]
    fn test_property_grid() {
        let mut grid = PropertyGridBehavior::new();
        grid.bool("Visible", "General", true);
        grid.float("Speed", "Physics", 1.0, 0.0, 10.0, 0.1);
        assert_eq!(grid.state.properties.len(), 2);
    }

    #[test]
    fn test_split_panel() {
        let split = SplitPanelBehavior::horizontal(0.5);
        assert!(split.state.draggable);
        let non_draggable = SplitPanelBehavior::horizontal(0.5).non_draggable();
        assert!(!non_draggable.state.draggable);
    }

    #[test]
    fn test_color_picker() {
        let picker = ColorPickerBehavior::new(ThemeColor::rgb(255, 0, 0));
        assert_eq!(picker.state.color.r, 255);
        assert_eq!(picker.state.color.g, 0);
    }

    #[test]
    fn test_flex_layout() {
        let layout = FlexLayout::row().gap(8).item(WidgetId(1)).item_flex(WidgetId(2), 1.0);
        assert_eq!(layout.direction, FlexDirection::Row);
        assert_eq!(layout.gap, 8);
        assert_eq!(layout.items.len(), 2);
        assert_eq!(layout.items[1].flex_grow, 1.0);
    }

    #[test]
    fn test_grid_layout() {
        let layout = GridLayout::new(3, 2).gap(4).at(WidgetId(1), 0, 0).at(WidgetId(2), 0, 1);
        assert_eq!(layout.columns, 3);
        assert_eq!(layout.items.len(), 2);
    }

    #[test]
    fn test_dock_system() {
        let mut dock = DockSystemBehavior::new();
        dock.dock_widget(WidgetId(1), DockArea::Left);
        assert_eq!(dock.state.nodes.len(), 1);
        dock.float_widget(WidgetId(2), [100, 100, 300, 200], "Floating");
        assert_eq!(dock.state.floating_windows.len(), 1);
    }

    #[test]
    fn test_curve_editor() {
        let mut editor = CurveEditorBehavior::new();
        let curve_idx = editor.add_curve(ThemeColor::rgb(255, 255, 255));
        editor.add_point(curve_idx, CurvePoint {
            time: 0.0,
            value: 0.0,
            point_type: CurvePointType::Linear,
            tangent_in: None,
            tangent_out: None,
        });
        assert_eq!(editor.state.curves.len(), 1);
        assert_eq!(editor.state.curves[0].points.len(), 1);
    }

    #[test]
    fn test_vector_editor() {
        let vec3 = VectorEditorBehavior::vec3(1.0, 2.0, 3.0);
        assert_eq!(vec3.state.value, vec![1.0, 2.0, 3.0]);
        assert_eq!(vec3.state.labels, vec!["X", "Y", "Z"]);
    }

    #[test]
    fn test_menu_bar() {
        let mut menu = MenuBarBehavior::new();
        let file_id = menu.add_menu("File");
        menu.add_menu_item(file_id, "Open", Some("Ctrl+O"));
        assert_eq!(menu.state.menus.len(), 1);
        assert_eq!(menu.state.menus[0].children.len(), 1);
    }

    #[test]
    fn test_theme_record() {
        let theme = ThemeRecord::dark("MyTheme")
            .add_class(ThemeClass {
                name: "button".to_string(),
                style: WidgetStyle::button(),
                inherits: vec![],
            });
        assert!(theme.is_dark);
        assert_eq!(theme.classes.len(), 1);
    }

    #[test]
    fn test_spinner() {
        let spinner = SpinnerBehavior::new(32);
        assert_eq!(spinner.state.size, 32);
    }

    #[test]
    fn test_tree_view_toggle() {
        let mut tree = TreeViewBehavior::new(vec![
            TreeItem {
                id: WidgetId(1),
                label: "Root".to_string(),
                children: vec![
                    TreeItem {
                        id: WidgetId(2),
                        label: "Child".to_string(),
                        children: Vec::new(),
                        expanded: false,
                        depth: 1,
                        user_data: None,
                    }
                ],
                expanded: false,
                depth: 0,
                user_data: None,
            }
        ]);
        tree.toggle_expand(WidgetId(1));
        assert!(tree.state.root_items[0].expanded);
    }
}
