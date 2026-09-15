//! Layout motoru — widget yerleşimi için flex-box benzeri sistem.

/// Yerleşim yönü
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LayoutDirection {
    Horizontal,
    Vertical,
}

/// Yerleşim çantası
#[derive(Clone, Debug)]
pub struct LayoutBag {
    pub direction: LayoutDirection,
    pub spacing: i32,
    pub padding: [i32; 4], // top, right, bottom, left
    pub items: Vec<LayoutItem>,
}

/// Yerleşim öğesi
#[derive(Clone, Debug)]
pub struct LayoutItem {
    pub id: super::WidgetId,
    pub size: [i32; 2],  // width, height
    pub flex: f32,        // esneklik oranı
    pub margin: [i32; 4],
}

impl LayoutBag {
    pub fn new(direction: LayoutDirection) -> Self {
        Self {
            direction,
            spacing: 4,
            padding: [0; 4],
            items: Vec::new(),
        }
    }

    pub fn with_spacing(mut self, spacing: i32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn with_padding(mut self, padding: [i32; 4]) -> Self {
        self.padding = padding;
        self
    }

    pub fn add_item(&mut self, item: LayoutItem) {
        self.items.push(item);
    }

    /// Yerleşimi hesapla ve her öğe için x,y koordinatlarını döndür
    pub fn compute(&self, start_x: i32, start_y: i32) -> Vec<(super::WidgetId, [i32; 2])> {
        let mut result = Vec::new();
        let mut cursor = match self.direction {
            LayoutDirection::Horizontal => start_x + self.padding[3],
            LayoutDirection::Vertical => start_y + self.padding[0],
        };

        for item in &self.items {
            let pos = match self.direction {
                LayoutDirection::Horizontal => {
                    let x = cursor;
                    let y = start_y + self.padding[0] + item.margin[0];
                    cursor += item.size[0] + self.spacing + item.margin[3] + item.margin[1];
                    [x, y]
                }
                LayoutDirection::Vertical => {
                    let x = start_x + self.padding[3] + item.margin[3];
                    let y = cursor;
                    cursor += item.size[1] + self.spacing + item.margin[0] + item.margin[2];
                    [x, y]
                }
            };
            result.push((item.id, pos));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WidgetId;

    #[test]
    fn test_layout_bag_horizontal() {
        let mut bag = LayoutBag::new(LayoutDirection::Horizontal).with_spacing(4);
        bag.add_item(LayoutItem { id: WidgetId(1), size: [100, 30], flex: 1.0, margin: [0; 4] });
        bag.add_item(LayoutItem { id: WidgetId(2), size: [80, 30], flex: 1.0, margin: [0; 4] });
        let positions = bag.compute(0, 0);
        assert_eq!(positions[0].1, [0, 0]);
        assert_eq!(positions[1].1, [104, 0]);
    }

    #[test]
    fn test_layout_bag_vertical() {
        let mut bag = LayoutBag::new(LayoutDirection::Vertical).with_spacing(8);
        bag.add_item(LayoutItem { id: WidgetId(1), size: [200, 30], flex: 1.0, margin: [0; 4] });
        bag.add_item(LayoutItem { id: WidgetId(2), size: [200, 30], flex: 1.0, margin: [0; 4] });
        let positions = bag.compute(10, 10);
        assert_eq!(positions[0].1, [10, 10]);
        assert_eq!(positions[1].1, [10, 48]);
    }
}
