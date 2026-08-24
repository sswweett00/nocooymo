//! Layout management for UI

use crate::{UiRect, Bounds};
use gpui::{Pixels, Point, Size};
use std::collections::HashMap;

/// Layout direction for arranging widgets
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutDirection {
    Horizontal,
    Vertical,
}

impl Default for LayoutDirection {
    fn default() -> Self {
        LayoutDirection::Vertical
    }
}

/// Layout constraints for responsive design
#[derive(Debug, Clone, Copy)]
pub struct LayoutConstraints {
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_height: Option<f32>,
    pub margin: f32,
    pub padding: f32,
}

impl Default for LayoutConstraints {
    fn default() -> Self {
        Self {
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            margin: 0.0,
            padding: 0.0,
        }
    }
}

impl LayoutConstraints {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_margin(mut self, margin: f32) -> Self {
        self.margin = margin;
        self
    }

    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    pub fn with_min_width(mut self, min_width: f32) -> Self {
        self.min_width = Some(min_width);
        self
    }

    pub fn with_max_width(mut self, max_width: f32) -> Self {
        self.max_width = Some(max_width);
        self
    }

    pub fn with_min_height(mut self, min_height: f32) -> Self {
        self.min_height = Some(min_height);
        self
    }

    pub fn with_max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(max_height);
        self
    }
}

/// Flex layout system
#[derive(Debug, Clone)]
pub struct FlexLayout {
    pub direction: LayoutDirection,
    pub gap: f32,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub flex_grow: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JustifyContent {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlignItems {
    Start,
    Center,
    End,
    Stretch,
}

impl Default for FlexLayout {
    fn default() -> Self {
        Self {
            direction: LayoutDirection::Horizontal,
            gap: 0.0,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            flex_grow: false,
        }
    }
}

impl FlexLayout {
    pub fn new(direction: LayoutDirection) -> Self {
        Self {
            direction,
            ..Default::default()
        }
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn with_justify_content(mut self, justify: JustifyContent) -> Self {
        self.justify_content = justify;
        self
    }

    pub fn with_align_items(mut self, align: AlignItems) -> Self {
        self.align_items = align;
        self
    }

    pub fn with_flex_grow(mut self, grow: bool) -> Self {
        self.flex_grow = grow;
        self
    }

    /// Calculate positions for child elements based on flexbox rules
    pub fn calculate_positions(
        &self,
        container_rect: UiRect,
        children_count: usize,
        constraints: &[LayoutConstraints],
    ) -> Vec<UiRect> {
        let mut positions = Vec::with_capacity(children_count);
        
        if children_count == 0 {
            return positions;
        }

        let available_space = UiRect::new(
            container_rect.x + self.gap,
            container_rect.y + self.gap,
            container_rect.width - 2.0 * self.gap,
            container_rect.height - 2.0 * self.gap,
        );

        match self.direction {
            LayoutDirection::Horizontal => {
                self.calculate_horizontal_positions(available_space, children_count, constraints, &mut positions);
            }
            LayoutDirection::Vertical => {
                self.calculate_vertical_positions(available_space, children_count, constraints, &mut positions);
            }
        }

        positions
    }

    fn calculate_horizontal_positions(
        &self,
        container: UiRect,
        count: usize,
        constraints: &[LayoutConstraints],
        positions: &mut Vec<UiRect>,
    ) {
        let total_gap = self.gap * (count as f32 - 1.0).max(0.0);
        let available_width = container.width - total_gap;
        
        // Calculate individual widths based on constraints
        let mut widths: Vec<f32> = Vec::with_capacity(count);
        let mut remaining_width = available_width;
        
        for i in 0..count {
            let constraint = if i < constraints.len() { 
                &constraints[i] 
            } else { 
                &LayoutConstraints::default() 
            };
            
            let width = if let Some(max_w) = constraint.max_width {
                max_w.min(remaining_width)
            } else if let Some(min_w) = constraint.min_width {
                min_w.min(remaining_width)
            } else {
                remaining_width / (count - i) as f32
            };
            
            widths.push(width);
            remaining_width -= width;
        }
        
        // Calculate positions
        let mut current_x = container.x;
        
        for (i, &width) in widths.iter().enumerate() {
            let constraint = if i < constraints.len() { 
                &constraints[i] 
            } else { 
                &LayoutConstraints::default() 
            };
            
            let height = if let Some(max_h) = constraint.max_height {
                max_h.min(container.height)
            } else if let Some(min_h) = constraint.min_height {
                min_h.max(container.height)
            } else {
                container.height
            };
            
            let y = match self.align_items {
                AlignItems::Start => container.y,
                AlignItems::Center => container.y + (container.height - height) * 0.5,
                AlignItems::End => container.y + container.height - height,
                AlignItems::Stretch => container.y,
            };
            
            positions.push(UiRect::new(current_x, y, width, height));
            current_x += width + self.gap;
        }
    }

    fn calculate_vertical_positions(
        &self,
        container: UiRect,
        count: usize,
        constraints: &[LayoutConstraints],
        positions: &mut Vec<UiRect>,
    ) {
        let total_gap = self.gap * (count as f32 - 1.0).max(0.0);
        let available_height = container.height - total_gap;
        
        // Calculate individual heights based on constraints
        let mut heights: Vec<f32> = Vec::with_capacity(count);
        let mut remaining_height = available_height;
        
        for i in 0..count {
            let constraint = if i < constraints.len() { 
                &constraints[i] 
            } else { 
                &LayoutConstraints::default() 
            };
            
            let height = if let Some(max_h) = constraint.max_height {
                max_h.min(remaining_height)
            } else if let Some(min_h) = constraint.min_height {
                min_h.min(remaining_height)
            } else {
                remaining_height / (count - i) as f32
            };
            
            heights.push(height);
            remaining_height -= height;
        }
        
        // Calculate positions
        let mut current_y = container.y;
        
        for (i, &height) in heights.iter().enumerate() {
            let constraint = if i < constraints.len() { 
                &constraints[i] 
            } else { 
                &LayoutConstraints::default() 
            };
            
            let width = if let Some(max_w) = constraint.max_width {
                max_w.min(container.width)
            } else if let Some(min_w) = constraint.min_width {
                min_w.max(container.width)
            } else {
                container.width
            };
            
            let x = match self.align_items {
                AlignItems::Start => container.x,
                AlignItems::Center => container.x + (container.width - width) * 0.5,
                AlignItems::End => container.x + container.width - width,
                AlignItems::Stretch => container.x,
            };
            
            positions.push(UiRect::new(x, current_y, width, height));
            current_y += height + self.gap;
        }
    }
}

/// Grid layout system
#[derive(Debug, Clone)]
pub struct GridLayout {
    pub columns: usize,
    pub rows: Option<usize>, // None means auto-calculate based on children count
    pub gap: f32,
    pub cell_alignment: AlignItems,
}

impl GridLayout {
    pub fn new(columns: usize) -> Self {
        Self {
            columns,
            rows: None,
            gap: 0.0,
            cell_alignment: AlignItems::Center,
        }
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn with_cell_alignment(mut self, alignment: AlignItems) -> Self {
        self.cell_alignment = alignment;
        self
    }

    pub fn calculate_grid_positions(
        &self,
        container_rect: UiRect,
        children_count: usize,
    ) -> Vec<UiRect> {
        let rows = self.rows.unwrap_or((children_count as f32 / self.columns as f32).ceil() as usize);
        let total_cells = self.columns * rows;
        
        // Actually calculate based on actual children, not theoretical grid size
        let actual_rows = ((children_count as f32) / (self.columns as f32)).ceil() as usize;
        
        let cell_width = (container_rect.width - self.gap * (self.columns as f32 - 1).max(0.0)) / self.columns as f32;
        let cell_height = (container_rect.height - self.gap * (actual_rows as f32 - 1).max(0.0)) / actual_rows as f32;
        
        let mut positions = Vec::with_capacity(children_count);
        
        for i in 0..children_count {
            let col = i % self.columns;
            let row = i / self.columns;
            
            let x = container_rect.x + col as f32 * (cell_width + self.gap);
            let y = container_rect.y + row as f32 * (cell_height + self.gap);
            
            positions.push(UiRect::new(x, y, cell_width, cell_height));
        }
        
        positions
    }
}

/// Dock layout system (like in IDEs)
#[derive(Debug, Clone)]
pub struct DockLayout {
    pub areas: HashMap<DockArea, UiRect>,
    pub main_area: UiRect,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum DockArea {
    Top,
    Bottom,
    Left,
    Right,
    Center,
}

impl DockLayout {
    pub fn new(main_area: UiRect) -> Self {
        let mut areas = HashMap::new();
        areas.insert(DockArea::Center, main_area);
        
        Self {
            areas,
            main_area,
        }
    }

    pub fn with_docked_area(mut self, area: DockArea, size: f32) -> Self {
        let main = self.main_area;
        
        match area {
            DockArea::Top => {
                let docked = UiRect::new(main.x, main.y, main.width, size);
                let new_main = UiRect::new(main.x, main.y + size, main.width, main.height - size);
                
                self.areas.insert(DockArea::Top, docked);
                self.areas.insert(DockArea::Center, new_main);
                self.main_area = new_main;
            },
            DockArea::Bottom => {
                let docked = UiRect::new(main.x, main.y + main.height - size, main.width, size);
                let new_main = UiRect::new(main.x, main.y, main.width, main.height - size);
                
                self.areas.insert(DockArea::Bottom, docked);
                self.areas.insert(DockArea::Center, new_main);
                self.main_area = new_main;
            },
            DockArea::Left => {
                let docked = UiRect::new(main.x, main.y, size, main.height);
                let new_main = UiRect::new(main.x + size, main.y, main.width - size, main.height);
                
                self.areas.insert(DockArea::Left, docked);
                self.areas.insert(DockArea::Center, new_main);
                self.main_area = new_main;
            },
            DockArea::Right => {
                let docked = UiRect::new(main.x + main.width - size, main.y, size, main.height);
                let new_main = UiRect::new(main.x, main.y, main.width - size, main.height);
                
                self.areas.insert(DockArea::Right, docked);
                self.areas.insert(DockArea::Center, new_main);
                self.main_area = new_main;
            },
            _ => {} // Center doesn't create a separate docked area
        }
        
        self
    }

    pub fn get_area(&self, area: DockArea) -> Option<&UiRect> {
        self.areas.get(&area)
    }
}

/// Responsive layout manager
#[derive(Debug, Clone)]
pub struct ResponsiveLayoutManager {
    pub breakpoints: HashMap<crate::widgets::Breakpoint, LayoutConfig>,
    pub current_breakpoint: crate::widgets::Breakpoint,
}

#[derive(Debug, Clone)]
pub struct LayoutConfig {
    pub layout_type: LayoutType,
    pub constraints: LayoutConstraints,
}

#[derive(Debug, Clone)]
pub enum LayoutType {
    Flex(FlexLayout),
    Grid(GridLayout),
    Custom(Box<dyn Fn(UiRect, usize) -> Vec<UiRect>>),
}

impl ResponsiveLayoutManager {
    pub fn new() -> Self {
        Self {
            breakpoints: HashMap::new(),
            current_breakpoint: crate::widgets::Breakpoint::Desktop,
        }
    }

    pub fn add_breakpoint_config(
        mut self,
        breakpoint: crate::widgets::Breakpoint,
        config: LayoutConfig,
    ) -> Self {
        self.breakpoints.insert(breakpoint, config);
        self
    }

    pub fn calculate_layout_for_current_breakpoint(
        &self,
        container_rect: UiRect,
        children_count: usize,
    ) -> Vec<UiRect> {
        if let Some(config) = self.breakpoints.get(&self.current_breakpoint) {
            match &config.layout_type {
                LayoutType::Flex(flex_layout) => {
                    flex_layout.calculate_positions(
                        container_rect,
                        children_count,
                        &[]
                    )
                },
                LayoutType::Grid(grid_layout) => {
                    grid_layout.calculate_grid_positions(
                        container_rect,
                        children_count
                    )
                },
                LayoutType::Custom(custom_fn) => {
                    custom_fn(container_rect, children_count)
                }
            }
        } else {
            // Fallback to basic vertical layout
            let height_per_child = container_rect.height / children_count.max(1) as f32;
            let mut positions = Vec::with_capacity(children_count);
            
            for i in 0..children_count {
                positions.push(UiRect::new(
                    container_rect.x,
                    container_rect.y + i as f32 * height_per_child,
                    container_rect.width,
                    height_per_child,
                ));
            }
            
            positions
        }
    }
}

// Helper functions for common layouts
pub fn create_vertical_stack(rect: UiRect, count: usize, gap: f32) -> Vec<UiRect> {
    let height_per_item = (rect.height - gap * (count as f32 - 1.0).max(0.0)) / count.max(1) as f32;
    let mut positions = Vec::with_capacity(count);
    
    for i in 0..count {
        positions.push(UiRect::new(
            rect.x,
            rect.y + i as f32 * (height_per_item + gap),
            rect.width,
            height_per_item,
        ));
    }
    
    positions
}

pub fn create_horizontal_stack(rect: UiRect, count: usize, gap: f32) -> Vec<UiRect> {
    let width_per_item = (rect.width - gap * (count as f32 - 1.0).max(0.0)) / count.max(1) as f32;
    let mut positions = Vec::with_capacity(count);
    
    for i in 0..count {
        positions.push(UiRect::new(
            rect.x + i as f32 * (width_per_item + gap),
            rect.y,
            width_per_item,
            rect.height,
        ));
    }
    
    positions
}