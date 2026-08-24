//! Archetype-based entity grouping for cache-friendly iteration.
//!
//! An archetype groups all entities that share the exact same set of component
//! types.  This enables tight, contiguous storage and fast query iteration.

use std::any::TypeId;
use std::collections::{HashMap, HashSet};

use crate::component::ComponentRegistry;
use crate::entity::Entity;

// ---------------------------------------------------------------------------
// ArchetypeId
// ---------------------------------------------------------------------------

/// A unique identifier for an archetype (a unique component combination).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ArchetypeId(pub u32);

impl ArchetypeId {
    pub const EMPTY: Self = Self(0);
}

// ---------------------------------------------------------------------------
// Archetype
// ---------------------------------------------------------------------------

/// A group of entities sharing the same component type set.
pub struct Archetype {
    pub id: ArchetypeId,
    /// Set of component TypeIds in this archetype.
    pub component_types: HashSet<TypeId>,
    /// Bitset representation of component type indices.
    pub type_bits: u64,
    /// Entities in this archetype (ordered).
    pub entities: Vec<Entity>,
    /// Maps entity to row index in `entities`.
    pub entity_to_row: HashMap<u32, usize>,
}

impl Archetype {
    pub fn new(id: ArchetypeId, component_types: HashSet<TypeId>, type_bits: u64) -> Self {
        Self {
            id,
            component_types,
            type_bits,
            entities: Vec::new(),
            entity_to_row: HashMap::new(),
        }
    }

    pub fn add_entity(&mut self, entity: Entity) -> usize {
        let row = self.entities.len();
        self.entities.push(entity);
        self.entity_to_row.insert(entity.id, row);
        row
    }

    pub fn remove_entity(&mut self, entity: Entity) -> Option<usize> {
        let row = self.entity_to_row.remove(&entity.id)?;
        let last = self.entities.len() - 1;
        if row != last {
            // Swap-remove
            self.entities.swap(row, last);
            let swapped = self.entities[row].id;
            self.entity_to_row.insert(swapped, row);
        }
        self.entities.pop();
        Some(row)
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.entity_to_row.contains_key(&entity.id)
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Check if this archetype's component set is a superset of the given bits.
    pub fn matches_mask(&self, mask: u64) -> bool {
        (self.type_bits & mask) == mask
    }
}

// ---------------------------------------------------------------------------
// ArchetypeGraph — edges for component add/remove transitions
// ---------------------------------------------------------------------------

/// An edge in the archetype graph representing adding or removing a component.
#[derive(Clone, Copy, Debug)]
pub struct ArchetypeEdge {
    pub target: ArchetypeId,
    pub added: bool,
    pub type_id: TypeId,
}

/// A node in the archetype graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ArchetypeEdgeKey {
    type_id: TypeId,
    added: bool,
}

/// Manages archetypes and their transition graph.
pub struct ArchetypeGraph {
    archetypes: Vec<Archetype>,
    /// Maps a type_bits value to archetype id.
    bits_to_id: HashMap<u64, ArchetypeId>,
    /// Adjacency list: archetype -> edges
    edges: HashMap<ArchetypeId, Vec<ArchetypeEdge>>,
}

impl ArchetypeGraph {
    pub fn new() -> Self {
        let mut graph = Self {
            archetypes: Vec::new(),
            bits_to_id: HashMap::new(),
            edges: HashMap::new(),
        };
        // Register the empty archetype
        let _ = graph.get_or_create(HashSet::new(), 0);
        graph
    }

    /// Get or create an archetype for the given component type set.
    pub fn get_or_create(
        &mut self,
        types: HashSet<TypeId>,
        type_bits: u64,
    ) -> ArchetypeId {
        if let Some(&id) = self.bits_to_id.get(&type_bits) {
            return id;
        }
        let id = ArchetypeId(self.archetypes.len() as u32);
        self.archetypes
            .push(Archetype::new(id, types.clone(), type_bits));
        self.bits_to_id.insert(type_bits, id);
        self.edges.insert(id, Vec::new());
        id
    }

    /// Get the archetype for a given ID.
    pub fn get(&self, id: ArchetypeId) -> Option<&Archetype> {
        self.archetypes.get(id.0 as usize)
    }

    /// Get a mutable reference to an archetype.
    pub fn get_mut(&mut self, id: ArchetypeId) -> Option<&mut Archetype> {
        self.archetypes.get_mut(id.0 as usize)
    }

    /// Add an entity to an archetype.
    pub fn add_entity(&mut self, archetype: ArchetypeId, entity: Entity) {
        if let Some(arch) = self.archetypes.get_mut(archetype.0 as usize) {
            arch.add_entity(entity);
        }
    }

    /// Remove an entity from an archetype.
    pub fn remove_entity(&mut self, archetype: ArchetypeId, entity: Entity) {
        if let Some(arch) = self.archetypes.get_mut(archetype.0 as usize) {
            arch.remove_entity(entity);
        }
    }

    /// Find archetypes that match the given component mask.
    pub fn query_archetypes(&self, mask: u64) -> Vec<ArchetypeId> {
        self.archetypes
            .iter()
            .filter(|a| a.matches_mask(mask))
            .map(|a| a.id)
            .collect()
    }

    /// Number of archetypes.
    pub fn len(&self) -> usize {
        self.archetypes.len()
    }

    /// Iterate over all archetypes.
    pub fn iter(&self) -> impl Iterator<Item = &Archetype> {
        self.archetypes.iter()
    }

    /// Compute the type_bits for a set of component type indices.
    pub fn compute_bits(indices: &[usize]) -> u64 {
        let mut bits = 0u64;
        for &idx in indices {
            if idx < 64 {
                bits |= 1u64 << idx;
            }
        }
        bits
    }
}

impl Default for ArchetypeGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_archetype_exists() {
        let graph = ArchetypeGraph::new();
        assert_eq!(graph.len(), 1);
        assert!(graph.get(ArchetypeId::EMPTY).is_some());
    }

    #[test]
    fn add_remove_entity() {
        let mut graph = ArchetypeGraph::new();
        let e = Entity {
            id: 0,
            generation: 0,
        };
        graph.add_entity(ArchetypeId::EMPTY, e);
        assert_eq!(graph.get(ArchetypeId::EMPTY).unwrap().len(), 1);
        graph.remove_entity(ArchetypeId::EMPTY, e);
        assert_eq!(graph.get(ArchetypeId::EMPTY).unwrap().len(), 0);
    }

    #[test]
    fn query_archetypes_by_mask() {
        let mut graph = ArchetypeGraph::new();
        let types_a = HashSet::from([TypeId::of::<u32>()]);
        let id_a = graph.get_or_create(types_a, 0b01);
        let types_b = HashSet::from([TypeId::of::<u32>(), TypeId::of::<f32>()]);
        let id_b = graph.get_or_create(types_b, 0b11);
        // Query for entities with component 0 (bit 0)
        let result = graph.query_archetypes(0b01);
        assert!(result.contains(&id_a));
        assert!(result.contains(&id_b));
        // Query for entities with both components
        let result = graph.query_archetypes(0b11);
        assert!(result.contains(&id_b));
        assert!(!result.contains(&id_a));
    }
}