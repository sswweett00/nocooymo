use std::any::Any;
use std::ptr::NonNull;

use crate::component::ComponentInfo;
use crate::entity::Entity;
use crate::CHUNK_SIZE;

/// Dense SoA storage page for one archetype (Mimari §1.1).
/// Stores component data and entity IDs for iteration.
///
/// ## Layout
/// - **Pod components** are stored inline in the contiguous `data` block
///   (SoA layout, cache-friendly).
/// - **Non-Pod (dynamic) components** are stored in `boxed_data`, one
///   `Vec<Option<Box<dyn Any + Send + Sync>>>` per component column.
///     Each inner vec has `capacity` entries, indexed by entity slot.
pub struct Chunk {
    pub data: NonNull<u8>,
    pub capacity: u16,
    pub len: u16,
    pub component_layout: Vec<ComponentLayout>,
    pub entity_ids: Vec<Entity>,
    /// Per-component column boxed storage for non-Pod components.
    /// Indexed by [component_layout_index][slot_index].
    pub boxed_data: Vec<Vec<Option<Box<dyn Any + Send + Sync>>>>,
}

#[derive(Clone, Debug)]
pub struct ComponentLayout {
    pub component_index: u16,
    pub offset: u32,
    pub size: u32,
    pub align: u32,
    /// `true` if this component column stores Pod data inline.
    /// `false` if it uses boxed storage for non-Pod types.
    pub is_pod: bool,
}

impl Chunk {
    pub fn new(data: NonNull<u8>, layouts: Vec<ComponentLayout>) -> Self {
        // Only Pod components contribute to the inline entity stride.
        let entity_stride = layouts
            .iter()
            .filter(|l| l.is_pod)
            .map(|l| l.offset + l.size)
            .max()
            .unwrap_or(0);
        let capacity = if entity_stride == 0 {
            u16::MAX
        } else {
            (CHUNK_SIZE as u32 / entity_stride).min(u16::MAX as u32) as u16
        };

        // Pre-allocate boxed storage columns for non-Pod layouts.
        let boxed_data: Vec<Vec<Option<Box<dyn Any + Send + Sync>>>> = layouts
            .iter()
            .map(|l| {
                if l.is_pod {
                    Vec::new()
                } else {
                    Vec::with_capacity(capacity as usize)
                }
            })
            .collect();

        Self {
            data,
            capacity,
            len: 0,
            component_layout: layouts,
            entity_ids: Vec::with_capacity(capacity as usize),
            boxed_data,
        }
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.len >= self.capacity
    }

    #[inline]
    pub fn push_slot(&mut self, entity: Entity) -> u16 {
        debug_assert!(!self.is_full());
        let slot = self.len;
        self.len += 1;
        self.entity_ids.push(entity);

        // Extend boxed columns with a None placeholder for the new slot.
        for col in &mut self.boxed_data {
            if col.len() <= slot as usize {
                col.push(None);
            }
        }

        slot
    }

    #[inline]
    pub fn remove_slot(&mut self, slot: u16) {
        debug_assert!(slot < self.len);
        if slot < self.len - 1 {
            self.swap_slots(slot, self.len - 1);
        }
        self.len -= 1;
        self.entity_ids.pop();

        // Pop the last element from each boxed column.
        for col in &mut self.boxed_data {
            if col.len() > self.len as usize {
                col.pop();
            }
        }
    }

    fn swap_slots(&mut self, a: u16, b: u16) {
        debug_assert!(a < self.len && b < self.len);
        for (layout_index, layout) in self.component_layout.iter().enumerate() {
            if layout.is_pod {
                // Inline pod swap (existing logic).
                let size = layout.size as usize;
                let base = self.data.as_ptr();
                let ptr_a = unsafe { base.add((layout.offset as usize) + (a as usize) * size) };
                let ptr_b = unsafe { base.add((layout.offset as usize) + (b as usize) * size) };
                unsafe {
                    std::ptr::swap(ptr_a as *mut u8, ptr_b as *mut u8);
                }
            } else if let Some(col) = self.boxed_data.get_mut(layout_index) {
                // Boxed storage swap.
                if (a as usize) < col.len() && (b as usize) < col.len() {
                    col.swap(a as usize, b as usize);
                }
            }
        }
        self.entity_ids.swap(a as usize, b as usize);
    }

    /// Get a raw pointer to a Pod component's data at the given slot.
    #[inline]
    pub fn component_ptr(&self, component_index: u16, slot: u16) -> *mut u8 {
        let layout = &self.component_layout[component_index as usize];
        debug_assert!(layout.is_pod, "component_ptr called for non-Pod layout");
        debug_assert!(slot < self.len);
        unsafe {
            self.data
                .as_ptr()
                .add((layout.offset as usize) + (slot as usize) * (layout.size as usize))
        }
    }

    /// Write Pod component bytes into the inline storage at the given slot.
    #[inline]
    pub fn write_component(&mut self, component_index: u16, slot: u16, bytes: &[u8]) {
        let layout = &self.component_layout[component_index as usize];
        debug_assert!(
            layout.is_pod,
            "write_component bytes called for non-Pod layout; use write_boxed_component"
        );
        debug_assert_eq!(bytes.len(), layout.size as usize);
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.component_ptr(component_index, slot),
                bytes.len(),
            );
        }
    }

    /// Write a non-Pod (boxed) component value for the given slot.
    #[inline]
    pub fn write_boxed_component(
        &mut self,
        component_index: u16,
        slot: u16,
        value: Box<dyn Any + Send + Sync>,
    ) {
        let layout_index = component_index as usize;
        debug_assert!(
            !self.component_layout[layout_index].is_pod,
            "write_boxed_component called for Pod layout; use write_component"
        );
        if let Some(col) = self.boxed_data.get_mut(layout_index) {
            // Ensure the column is long enough for the given slot.
            // We cannot use `resize` because Box<dyn Any> does not implement Clone.
            while col.len() <= slot as usize {
                col.push(None);
            }
            col[slot as usize] = Some(value);
        }
    }

    /// Read a reference to a non-Pod (boxed) component value.
    #[inline]
    pub fn read_boxed_component(
        &self,
        component_index: u16,
        slot: u16,
    ) -> Option<&dyn Any> {
        let layout_index = component_index as usize;
        self.boxed_data
            .get(layout_index)
            .and_then(|col| col.get(slot as usize))
            .and_then(|opt| opt.as_deref().map(|b| b as &dyn Any))
    }

    /// Read a mutable reference to a non-Pod (boxed) component value.
    #[inline]
    pub fn read_boxed_component_mut(
        &mut self,
        component_index: u16,
        slot: u16,
    ) -> Option<&mut dyn Any> {
        let layout_index = component_index as usize;
        self.boxed_data
            .get_mut(layout_index)
            .and_then(|col| col.get_mut(slot as usize))
            .and_then(|opt| opt.as_deref_mut().map(|b| b as &mut dyn Any))
    }
}

pub fn build_layouts(infos: &[&ComponentInfo]) -> Vec<ComponentLayout> {
    let mut offset = 0u32;
    let mut layouts = Vec::with_capacity(infos.len());

    // Build layouts for all components (Pod and non-Pod).
    for (index, info) in infos.iter().enumerate() {
        if info.is_pod {
            let align = info.align as u32;
            offset = (offset + align - 1) & !(align - 1);
            layouts.push(ComponentLayout {
                component_index: index as u16,
                offset,
                size: info.size as u32,
                align,
                is_pod: true,
            });
            offset += info.size as u32;
        } else {
            // Non-Pod components get a zero-size, zero-offset placeholder.
            // They don't occupy inline SoA space.
            layouts.push(ComponentLayout {
                component_index: index as u16,
                offset: 0,
                size: info.size as u32,
                align: 1,
                is_pod: false,
            });
        }
    }

    layouts
}

/// Number of bytes consumed by a single entity's Pod data in the inline SoA region.
pub fn entity_stride(layouts: &[ComponentLayout]) -> usize {
    layouts
        .iter()
        .filter(|l| l.is_pod)
        .map(|l| (l.offset + l.size) as usize)
        .max()
        .unwrap_or(0)
}