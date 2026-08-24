//! The World — central ECS registry combining entities, components, and archetypes.

use std::any::TypeId;
use std::collections::{HashMap, HashSet};

use crate::archetype::{ArchetypeGraph, ArchetypeId};
use crate::component::{Component, ComponentRegistry};
use crate::entity::{Entity, EntityAllocator};

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

/// The central ECS world.  Owns the entity allocator, component registry,
/// and archetype graph.
pub struct World {
    pub entities: EntityAllocator,
    pub components: ComponentRegistry,
    pub archetypes: ArchetypeGraph,
    /// Maps entity to its current archetype.
    entity_archetype: HashMap<u32, ArchetypeId>,
    /// Maps entity to its component type bits.
    entity_bits: HashMap<u32, u64>,
    /// Resource storage (singletons).
    resources: HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>,
    /// Entity tags (string -> set of entities).
    tags: HashMap<String, Vec<Entity>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: EntityAllocator::new(),
            components: ComponentRegistry::new(),
            archetypes: ArchetypeGraph::new(),
            entity_archetype: HashMap::new(),
            entity_bits: HashMap::new(),
            resources: HashMap::new(),
            tags: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entities: EntityAllocator::with_capacity(capacity),
            components: ComponentRegistry::new(),
            archetypes: ArchetypeGraph::new(),
            entity_archetype: HashMap::with_capacity(capacity),
            entity_bits: HashMap::with_capacity(capacity),
            resources: HashMap::new(),
            tags: HashMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Entity management
    // -----------------------------------------------------------------------

    /// Spawn a new empty entity.
    pub fn spawn(&mut self) -> Entity {
        let entity = self.entities.allocate();
        self.entity_archetype.insert(entity.id, ArchetypeId::EMPTY);
        self.entity_bits.insert(entity.id, 0);
        self.archetypes.add_entity(ArchetypeId::EMPTY, entity);
        entity
    }

    /// Despawn an entity and all its components.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.entities.is_alive(entity) {
            return false;
        }
        // Remove from archetype
        if let Some(&arch_id) = self.entity_archetype.get(&entity.id) {
            self.archetypes.remove_entity(arch_id, entity);
        }
        // Remove all components
        self.components.remove_all(entity);
        // Remove from maps
        self.entity_archetype.remove(&entity.id);
        self.entity_bits.remove(&entity.id);
        // Deallocate
        self.entities.deallocate(entity);
        // Remove from tags
        for entities in self.tags.values_mut() {
            entities.retain(|e| e != &entity);
        }
        true
    }

    /// Check if an entity is alive.
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entities.is_alive(entity)
    }

    /// Number of alive entities.
    pub fn entity_count(&self) -> usize {
        self.entities.alive_count()
    }

    /// Iterate over all alive entities.
    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.entities.iter_alive()
    }

    // -----------------------------------------------------------------------
    // Component management
    // -----------------------------------------------------------------------

    /// Insert a component into an entity, updating its archetype.
    pub fn insert_component<T: Component + Clone>(&mut self, entity: Entity, component: T) {
        if !self.is_alive(entity) {
            return;
        }
        // Register the type and get its index
        let type_idx = self.components.register::<T>();
        if type_idx >= 64 {
            tracing::warn!("Component type index {} >=64 — archetype bits overflow, ignoring archetype tracking", type_idx);
            self.components.insert(entity, component);
            return;
        }
        // Insert into storage
        self.components.insert(entity, component);
        // Update entity's type bits
        let old_bits = self.entity_bits.get(&entity.id).copied().unwrap_or(0);
        let new_bits = old_bits | (1u64 << type_idx);
        // Update archetype
        let old_arch = self.entity_archetype.get(&entity.id).copied().unwrap_or(ArchetypeId::EMPTY);
        self.archetypes.remove_entity(old_arch, entity);

        // Compute new archetype
        let types: HashSet<TypeId> = self
            .components
            .type_indices()
            .iter()
            .filter(|(_, idx)| new_bits & (1u64 << **idx) != 0)
            .map(|(tid, _)| *tid)
            .collect();
        let new_arch = self.archetypes.get_or_create(types, new_bits);
        self.archetypes.add_entity(new_arch, entity);
        self.entity_archetype.insert(entity.id, new_arch);
        self.entity_bits.insert(entity.id, new_bits);
    }

    /// Remove a component from an entity.
    pub fn remove_component<T: Component + Clone>(&mut self, entity: Entity) -> Option<T> {
        if !self.is_alive(entity) {
            return None;
        }
        let type_idx = self.components.type_index::<T>()?;
        let removed = self.components.remove::<T>(entity);
        if type_idx >= 64 {
            return removed;
        }
        // Update bits
        let old_bits = self.entity_bits.get(&entity.id).copied().unwrap_or(0);
        let new_bits = old_bits & !(1u64 << type_idx);
        // Update archetype
        let old_arch = self.entity_archetype.get(&entity.id).copied().unwrap_or(ArchetypeId::EMPTY);
        self.archetypes.remove_entity(old_arch, entity);

        let types: HashSet<TypeId> = self
            .components
            .type_indices()
            .iter()
            .filter(|(_, idx)| new_bits & (1u64 << **idx) != 0)
            .map(|(tid, _)| *tid)
            .collect();
        let new_arch = self.archetypes.get_or_create(types, new_bits);
        self.archetypes.add_entity(new_arch, entity);
        self.entity_archetype.insert(entity.id, new_arch);
        self.entity_bits.insert(entity.id, new_bits);
        removed
    }

    /// Get a reference to a component.
    pub fn get_component<T: Component + Clone>(&self, entity: Entity) -> Option<&T> {
        self.components.get::<T>(entity)
    }

    /// Get a mutable reference to a component.
    pub fn get_component_mut<T: Component + Clone>(&mut self, entity: Entity) -> Option<&mut T> {
        self.components.get_mut::<T>(entity)
    }

    /// Check if an entity has a component.
    pub fn has_component<T: Component + Clone>(&self, entity: Entity) -> bool {
        self.components.has::<T>(entity)
    }

        // -----------------------------------------------------------------------
    // Querying
    // -----------------------------------------------------------------------

    /// Alias mirroring the command-buffer API (`insert_component_by_id`).
    pub fn insert_component_by_id<T: Component + Clone>(
        &mut self,
        entity: Entity,
        component: T,
    ) {
        self.insert_component(entity, component);
    }

    /// Advance the world by one frame.  Currently a no-op; reserved for
    /// per-frame resource updates driven by the engine.
    pub fn update(&mut self) {}

    /// Get all entities that have the given component type.
    pub fn query<T: Component + Clone>(&self) -> Vec<Entity> {
        let storage = match self.components.storage::<T>() {
            Some(s) => s,
            None => return Vec::new(),
        };
        storage
            .iter()
            .map(|(e, _)| Entity {
                id: e.id,
                generation: self.entities.generation(e.id),
            })
            .filter(|e| self.is_alive(*e))
            .collect()
    }

    /// Get all entities that have all of the given component types.
    pub fn query_two<A: Component + Clone, B: Component + Clone>(&self) -> Vec<Entity> {
        let storage_a = match self.components.storage::<A>() {
            Some(s) => s,
            None => return Vec::new(),
        };
        storage_a
            .iter()
            .map(|(e, _)| Entity {
                id: e.id,
                generation: self.entities.generation(e.id),
            })
            .filter(|e| self.is_alive(*e) && self.has_component::<B>(*e))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Resources
    // -----------------------------------------------------------------------

    /// Insert a resource (singleton).
    pub fn insert_resource<R: 'static + Send + Sync>(&mut self, resource: R) {
        self.resources
            .insert(TypeId::of::<R>(), Box::new(resource));
    }

    /// Get a reference to a resource.
    pub fn get_resource<R: 'static + Send + Sync>(&self) -> Option<&R> {
        self.resources
            .get(&TypeId::of::<R>())
            .and_then(|r| r.downcast_ref::<R>())
    }

    /// Get a mutable reference to a resource.
    pub fn get_resource_mut<R: 'static + Send + Sync>(&mut self) -> Option<&mut R> {
        self.resources
            .get_mut(&TypeId::of::<R>())
            .and_then(|r| r.downcast_mut::<R>())
    }

    /// Remove a resource.
    pub fn remove_resource<R: 'static + Send + Sync>(&mut self) -> Option<R> {
        self.resources
            .remove(&TypeId::of::<R>())
            .and_then(|r| r.downcast::<R>().ok())
            .map(|b| *b)
    }

    // -----------------------------------------------------------------------
    // Tags
    // -----------------------------------------------------------------------

    /// Tag an entity with a string label.
    pub fn tag(&mut self, entity: Entity, tag: &str) {
        let entry = self.tags.entry(tag.to_string()).or_default();
        if !entry.contains(&entity) {
            entry.push(entity);
        }
    }

    /// Get all entities with a given tag.
    pub fn get_tagged(&self, tag: &str) -> &[Entity] {
        self.tags.get(tag).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Remove a tag from an entity.
    pub fn untag(&mut self, entity: Entity, tag: &str) {
        if let Some(entities) = self.tags.get_mut(tag) {
            entities.retain(|e| e != &entity);
        }
    }

    // -----------------------------------------------------------------------
    // Utility
    // -----------------------------------------------------------------------

    /// Clear all entities and components.
    pub fn clear(&mut self) {
        self.entities.clear();
        self.components = ComponentRegistry::new();
        self.archetypes = ArchetypeGraph::new();
        self.entity_archetype.clear();
        self.entity_bits.clear();
        self.tags.clear();
    }

    /// Total number of components across all entities.
    pub fn component_count(&self) -> usize {
        self.components.total_components()
    }

    /// Number of unique archetypes.
    pub fn archetype_count(&self) -> usize {
        self.archetypes.len()
    }
}

impl Default for World {
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

    #[derive(Clone, Debug, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
        z: f32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Velocity {
        x: f32,
        y: f32,
        z: f32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Name(String);

    #[test]
    fn spawn_despawn() {
        let mut world = World::new();
        let e = world.spawn();
        assert!(world.is_alive(e));
        assert_eq!(world.entity_count(), 1);
        world.despawn(e);
        assert!(!world.is_alive(e));
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn insert_get_component() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert_component(e, Position { x: 1.0, y: 2.0, z: 3.0 });
        assert!(world.has_component::<Position>(e));
        assert_eq!(
            world.get_component::<Position>(e),
            Some(&Position { x: 1.0, y: 2.0, z: 3.0 })
        );
    }

    #[test]
    fn remove_component() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert_component(e, Position { x: 1.0, y: 0.0, z: 0.0 });
        let removed = world.remove_component::<Position>(e);
        assert_eq!(removed, Some(Position { x: 1.0, y: 0.0, z: 0.0 }));
        assert!(!world.has_component::<Position>(e));
    }

    #[test]
    fn query_entities() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();
        world.insert_component(e1, Position { x: 1.0, y: 0.0, z: 0.0 });
        world.insert_component(e2, Position { x: 2.0, y: 0.0, z: 0.0 });
        world.insert_component(e3, Name("e3".into()));
        let with_pos = world.query::<Position>();
        assert_eq!(with_pos.len(), 2);
    }

    #[test]
    fn query_two_components() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();
        world.insert_component(e1, Position { x: 1.0, y: 0.0, z: 0.0 });
        world.insert_component(e1, Velocity { x: 0.1, y: 0.0, z: 0.0 });
        world.insert_component(e2, Position { x: 2.0, y: 0.0, z: 0.0 });
        world.insert_component(e3, Velocity { x: 0.2, y: 0.0, z: 0.0 });
        let result = world.query_two::<Position, Velocity>();
        assert_eq!(result.len(), 1);
        assert!(result.contains(&e1));
    }

    #[test]
    fn resources() {
        let mut world = World::new();
        world.insert_resource(42i32);
        assert_eq!(world.get_resource::<i32>(), Some(&42));
        world.insert_resource("hello".to_string());
        assert_eq!(world.get_resource::<String>(), Some(&"hello".to_string()));
    }

    #[test]
    fn tags() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        world.tag(e1, "player");
        world.tag(e2, "player");
        world.tag(e1, "controllable");
        let players = world.get_tagged("player");
        assert_eq!(players.len(), 2);
        let controllable = world.get_tagged("controllable");
        assert_eq!(controllable.len(), 1);
    }

    #[test]
    fn archetype_transitions() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert_component(e, Position { x: 0.0, y: 0.0, z: 0.0 });
        assert_eq!(world.archetype_count(), 2); // empty + position
        world.insert_component(e, Velocity { x: 0.0, y: 0.0, z: 0.0 });
        assert_eq!(world.archetype_count(), 3); // empty + position + position+velocity
        world.remove_component::<Position>(e);
        assert!(world.has_component::<Velocity>(e));
        assert!(!world.has_component::<Position>(e));
    }
}