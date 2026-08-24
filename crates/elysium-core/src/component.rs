//! Component storage and query system.
//!
//! Uses sparse-set-based storage for each component type, with a type-erased
//! `ComponentStorage` trait for heterogeneous iteration.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::RwLock;

use crate::entity::Entity;

// ---------------------------------------------------------------------------
// Component trait
// ---------------------------------------------------------------------------

/// Marker trait for components.  Automatically implemented for any
/// `'static + Send + Sync` type.
pub trait Component: 'static + Send + Sync {
    /// Called when the component is attached to an entity (optional).
    fn on_attach(&mut self, _entity: Entity) {}
    /// Called when the component is detached (optional).
    fn on_detach(&mut self, _entity: Entity) {}
}

impl<T: 'static + Send + Sync> Component for T {
    //
}

/// Type/layout descriptor for a component family.
#[derive(Clone, Debug)]
pub struct ComponentInfo {
    pub name: String,
    pub size: usize,
    pub align: usize,
    pub is_pod: bool,
}

// ---------------------------------------------------------------------------
// Type-erased storage trait
// ---------------------------------------------------------------------------

/// A type-erased component storage.
pub trait ComponentStorage: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn remove(&mut self, entity: Entity);
    fn contains(&self, entity: Entity) -> bool;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn clear(&mut self);
    /// Clone the component for a given entity (if Clone is supported).
    fn clone_for(&self, _entity: Entity) -> Option<Box<dyn ComponentStorage>> {
        None
    }
}

/// Concrete storage for a specific component type.
pub struct TypedStorage<T: Component> {
    components: HashMap<u32, T>,
}

impl<T: Component> TypedStorage<T> {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    pub fn insert(&mut self, entity: Entity, value: T) {
        self.components.insert(entity.id, value);
    }

    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.components.get(&entity.id)
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.components.get_mut(&entity.id)
    }

    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        self.components.remove(&entity.id)
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.components.contains_key(&entity.id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.components.iter().map(|(id, v)| {
            (
                Entity {
                    id: *id,
                    generation: 0, // generation tracked by allocator
                },
                v,
            )
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.components.iter_mut().map(|(id, v)| {
            (
                Entity {
                    id: *id,
                    generation: 0,
                },
                v,
            )
        })
    }
}

impl<T: Component> Default for TypedStorage<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Component + Clone> ComponentStorage for TypedStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn remove(&mut self, entity: Entity) {
        self.components.remove(&entity.id);
    }

    fn contains(&self, entity: Entity) -> bool {
        self.components.contains_key(&entity.id)
    }

    fn len(&self) -> usize {
        self.components.len()
    }

    fn clear(&mut self) {
        self.components.clear();
    }

    fn clone_for(&self, entity: Entity) -> Option<Box<dyn ComponentStorage>> {
        self.get(entity).map(|v| {
            Box::new(TypedStorage {
                components: {
                    let mut s = HashMap::new();
                    s.insert(entity.id, v.clone());
                    s
                },
            }) as Box<dyn ComponentStorage>
        })
    }
}

// ---------------------------------------------------------------------------
// Component registry
// ---------------------------------------------------------------------------

/// A registry of all component storages, indexed by `TypeId`.
pub struct ComponentRegistry {
    storages: HashMap<TypeId, Box<dyn ComponentStorage>>,
    /// Maps TypeId to a sequential index (for archetype bitset).
    type_indices: HashMap<TypeId, usize>,
    /// Next available type index.
    next_index: usize,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self {
            storages: HashMap::new(),
            type_indices: HashMap::new(),
            next_index: 0,
        }
    }

    /// Register a component type.  Idempotent.
    pub fn register<T: Component + Clone>(&mut self) -> usize {
        let type_id = TypeId::of::<T>();
        if let Some(&idx) = self.type_indices.get(&type_id) {
            return idx;
        }
        let idx = self.next_index;
        self.next_index += 1;
        self.type_indices.insert(type_id, idx);
        self.storages
            .insert(type_id, Box::new(TypedStorage::<T>::new()));
        idx
    }

    /// Get the type index for a component (must be registered first).
        pub fn type_index<T: Component + Clone>(&self) -> Option<usize> {
        self.type_indices.get(&TypeId::of::<T>()).copied()
    }

    /// Read-only access to the component type -> index map.
    pub fn type_indices(&self) -> &HashMap<TypeId, usize> {
        &self.type_indices
    }

    /// Get the typed storage for a component.
    pub fn storage<T: Component + Clone>(&self) -> Option<&TypedStorage<T>> {
        self.storages
            .get(&TypeId::of::<T>())
            .and_then(|s| s.as_any().downcast_ref::<TypedStorage<T>>())
    }

    /// Get a mutable reference to the typed storage.
    pub fn storage_mut<T: Component + Clone>(&mut self) -> Option<&mut TypedStorage<T>> {
        self.storages
            .get_mut(&TypeId::of::<T>())
            .and_then(|s| s.as_any_mut().downcast_mut::<TypedStorage<T>>())
    }

    /// Insert a component for an entity.
    pub fn insert<T: Component + Clone>(&mut self, entity: Entity, value: T) {
        self.register::<T>(); // ensure registered
        if let Some(storage) = self.storage_mut::<T>() {
            storage.insert(entity, value);
        }
    }

    /// Remove a component from an entity.
    pub fn remove<T: Component + Clone>(&mut self, entity: Entity) -> Option<T> {
        self.storage_mut::<T>().and_then(|s| s.remove(entity))
    }

    /// Remove all components for an entity across all storages.
    pub fn remove_all(&mut self, entity: Entity) {
        for storage in self.storages.values_mut() {
            storage.remove(entity);
        }
    }

    /// Get a component for an entity.
    pub fn get<T: Component + Clone>(&self, entity: Entity) -> Option<&T> {
        self.storage::<T>().and_then(|s| s.get(entity))
    }

    /// Get a mutable reference to a component.
    pub fn get_mut<T: Component + Clone>(&mut self, entity: Entity) -> Option<&mut T> {
        self.storage_mut::<T>().and_then(|s| s.get_mut(entity))
    }

    /// Check if an entity has a component.
    pub fn has<T: Component + Clone>(&self, entity: Entity) -> bool {
        self.storage::<T>().map(|s| s.contains(entity)).unwrap_or(false)
    }

    /// Number of registered component types.
    pub fn type_count(&self) -> usize {
        self.storages.len()
    }

    /// Total number of components across all storages.
    pub fn total_components(&self) -> usize {
        self.storages.values().map(|s| s.len()).sum()
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Query system
// ---------------------------------------------------------------------------

/// A query that matches entities having all specified component types.
pub struct Query<'a, F> {
    registry: &'a ComponentRegistry,
    filter: F,
}

impl<'a, F> Query<'a, F> {
    pub fn new(registry: &'a ComponentRegistry, filter: F) -> Self {
        Self { registry, filter }
    }
}

/// A query filter that matches entities with all of the given component types.
pub struct QueryFilter {
    pub required: Vec<TypeId>,
}

impl QueryFilter {
    pub fn new() -> Self {
        Self {
            required: Vec::new(),
        }
    }

    pub fn with<T: Component + Clone>(mut self) -> Self {
        self.required.push(TypeId::of::<T>());
        self
    }
}

impl Default for QueryFilter {
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

    #[derive(Clone, PartialEq, Debug)]
    struct Position {
        x: f32,
        y: f32,
        z: f32,
    }

    #[derive(Clone, PartialEq, Debug)]
    struct Velocity {
        x: f32,
        y: f32,
        z: f32,
    }

    #[test]
    fn storage_insert_get_remove() {
        let mut storage = TypedStorage::<Position>::new();
        let e = Entity {
            id: 0,
            generation: 0,
        };
        storage.insert(e, Position { x: 1.0, y: 2.0, z: 3.0 });
        assert!(storage.contains(e));
        assert_eq!(storage.get(e), Some(&Position { x: 1.0, y: 2.0, z: 3.0 }));
        storage.remove(e);
        assert!(!storage.contains(e));
    }

    #[test]
    fn registry_multiple_types() {
        let mut registry = ComponentRegistry::new();
        let e = Entity {
            id: 0,
            generation: 0,
        };
        registry.insert(e, Position { x: 1.0, y: 0.0, z: 0.0 });
        registry.insert(e, Velocity { x: 0.1, y: 0.0, z: 0.0 });
        assert!(registry.has::<Position>(e));
        assert!(registry.has::<Velocity>(e));
        assert_eq!(registry.type_count(), 2);
        assert_eq!(registry.total_components(), 2);
    }

    #[test]
    fn registry_remove_all() {
        let mut registry = ComponentRegistry::new();
        let e = Entity {
            id: 0,
            generation: 0,
        };
        registry.insert(e, Position { x: 1.0, y: 0.0, z: 0.0 });
        registry.insert(e, Velocity { x: 0.1, y: 0.0, z: 0.0 });
        registry.remove_all(e);
        assert!(!registry.has::<Position>(e));
        assert!(!registry.has::<Velocity>(e));
    }
}