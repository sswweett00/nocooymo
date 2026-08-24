//! Entity management with generational indices and entity metadata.

use serde::{Deserialize, Serialize};

use crate::component::Component;
use crate::data_structures::Handle;

/// A generational entity ID.  The generation distinguishes a recycled slot
/// from a previous (now-dead) entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Entity {
    pub id: u32,
    pub generation: u32,
}

impl Entity {
    pub const NULL: Self = Self {
        id: u32::MAX,
        generation: 0,
    };

    pub fn is_null(self) -> bool {
        self.id == u32::MAX
    }

    pub fn to_bits(self) -> u64 {
        ((self.generation as u64) << 32) | self.id as u64
    }

    pub fn from_bits(bits: u64) -> Self {
        Self {
            id: bits as u32,
            generation: (bits >> 32) as u32,
        }
    }
}

impl std::fmt::Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Entity({}v{})", self.id, self.generation)
    }
}

impl Entity {
    /// Numeric index used by the sparse-set storage.
    pub fn index(self) -> u32 {
        self.id
    }

    /// Invalid placeholder entity (same as [`Entity::NULL`]).
    pub const INVALID: Self = Self { id: u32::MAX, generation: 0 };

    /// Construct an entity from raw parts.
    pub fn from_parts(id: u32, generation: u32) -> Self {
        Self { id, generation }
    }
}

impl From<Handle> for Entity {
    fn from(h: Handle) -> Self {
        Self {
            id: h.index,
            generation: h.generation,
        }
    }
}

impl From<Entity> for Handle {
    fn from(e: Entity) -> Self {
        Self {
            index: e.id,
            generation: e.generation,
        }
    }
}

// ---------------------------------------------------------------------------
// Entity allocator
// ---------------------------------------------------------------------------

/// Allocates and recycles entity IDs with generation tracking.
pub struct EntityAllocator {
    /// Per-slot generation counter.
    generations: Vec<u32>,
    /// Whether the slot is currently alive.
    alive: Vec<bool>,
    /// Free list of recycled slots.
    free: Vec<u32>,
    /// Total number of alive entities.
    alive_count: usize,
}

impl EntityAllocator {
    pub fn new() -> Self {
        Self {
            generations: Vec::new(),
            alive: Vec::new(),
            free: Vec::new(),
            alive_count: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            generations: Vec::with_capacity(capacity),
            alive: Vec::with_capacity(capacity),
            free: Vec::new(),
            alive_count: 0,
        }
    }

    /// Allocate a new entity.
    pub fn allocate(&mut self) -> Entity {
        self.alive_count += 1;
        if let Some(id) = self.free.pop() {
            self.alive[id as usize] = true;
            Entity {
                id,
                generation: self.generations[id as usize],
            }
        } else {
            let id = self.generations.len() as u32;
            self.generations.push(0);
            self.alive.push(true);
            Entity {
                id,
                generation: 0,
            }
        }
    }

    /// Deallocate an entity, allowing its slot to be recycled.
    pub fn deallocate(&mut self, entity: Entity) -> bool {
        if self.is_alive(entity) {
            self.alive[entity.id as usize] = false;
            self.generations[entity.id as usize] =
                self.generations[entity.id as usize].wrapping_add(1);
            self.free.push(entity.id);
            self.alive_count -= 1;
            true
        } else {
            false
        }
    }

    /// Check if an entity is alive.
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.alive
            .get(entity.id as usize)
            .map(|&a| a && self.generations[entity.id as usize] == entity.generation)
            .unwrap_or(false)
    }

    /// Number of alive entities.
    pub fn alive_count(&self) -> usize {
        self.alive_count
    }

    /// Get the current generation for an entity slot.
    pub fn generation(&self, id: u32) -> u32 {
        self.generations.get(id as usize).copied().unwrap_or(0)
    }

    /// Total capacity (alive + recycled slots).
    pub fn capacity(&self) -> usize {
        self.generations.len()
    }

    /// Iterate over all alive entities.
    pub fn iter_alive(&self) -> impl Iterator<Item = Entity> + '_ {
        self.alive
            .iter()
            .enumerate()
            .filter_map(|(i, &a)| {
                if a {
                    Some(Entity {
                        id: i as u32,
                        generation: self.generations[i],
                    })
                } else {
                    None
                }
            })
    }

    /// Clear all entities.
    pub fn clear(&mut self) {
        self.generations.clear();
        self.alive.clear();
        self.free.clear();
        self.alive_count = 0;
    }
}

impl Default for EntityAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Entity builder
// ---------------------------------------------------------------------------

/// A helper for building entities with components.
pub struct EntityBuilder<'a> {
    world: &'a mut crate::world::World,
    entity: Entity,
}

impl<'a> EntityBuilder<'a> {
    pub fn new(world: &'a mut crate::world::World) -> Self {
        let entity = world.spawn();
        Self { world, entity }
    }

    /// Attach a component to the entity being built.
    pub fn with<T: Component + Clone>(mut self, component: T) -> Self {
        self.world.insert_component(self.entity, component);
        self
    }

    /// Finalise and return the entity.
    pub fn build(self) -> Entity {
        self.entity
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_and_deallocate() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.allocate();
        let e2 = alloc.allocate();
        assert!(alloc.is_alive(e1));
        assert!(alloc.is_alive(e2));
        assert_eq!(alloc.alive_count(), 2);

        alloc.deallocate(e1);
        assert!(!alloc.is_alive(e1));
        assert_eq!(alloc.alive_count(), 1);

        // Recycle
        let e3 = alloc.allocate();
        assert_eq!(e3.id, e1.id);
        assert_ne!(e3.generation, e1.generation);
        assert!(alloc.is_alive(e3));
        assert!(!alloc.is_alive(e1)); // old generation is dead
    }

    #[test]
    fn entity_bits_roundtrip() {
        let e = Entity {
            id: 42,
            generation: 7,
        };
        let bits = e.to_bits();
        let e2 = Entity::from_bits(bits);
        assert_eq!(e, e2);
    }

    #[test]
    fn iter_alive() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.allocate();
        let e2 = alloc.allocate();
        let e3 = alloc.allocate();
        alloc.deallocate(e2);
        let alive: Vec<_> = alloc.iter_alive().collect();
        assert_eq!(alive.len(), 2);
        assert!(alive.contains(&e1));
        assert!(alive.contains(&e3));
    }
}