//! ECS query system for safe component access (Mimari §1.1).

use std::any::Any;
use std::marker::PhantomData;

use rayon::prelude::*;

use crate::chunk::Chunk;
use crate::component::{Component, ComponentId};
use crate::entity::Entity;
use crate::world::World;

/// Trait for specifying component access patterns in queries.
pub trait ComponentAccess {}

/// Immutable query for component access.
pub struct Query<'w, T> {
    world: &'w World,
    _marker: PhantomData<T>,
}

impl<'w, T> Query<'w, T> {
    pub(crate) fn new(world: &'w World) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }

    pub fn iter(&self) -> QueryIter<'_, T> {
        QueryIter {
            world: self.world,
            archetype_iter: self.world.archetypes.iter(),
            current_entities: Vec::new(),
            current_offset: 0,
            _marker: PhantomData,
        }
    }
}

pub struct QueryIter<'w, T> {
    world: &'w World,
    archetype_iter: std::vec::IntoIter<&'w crate::archetype::Archetype>,
    current_entities: Vec<(Entity, &'w T)>,
    current_offset: usize,
    _marker: PhantomData<T>,
}

impl<'w, T: Component> Iterator for QueryIter<'w, T> {
    type Item = (Entity, &'w T);

    fn next(&mut self) -> Option<Self::Item> {
        while self.current_offset >= self.current_entities.len() {
            let archetype = self.archetype_iter.next()?;
            self.current_entities.clear();
            self.current_offset = 0;

            let comp_id = match self.world.registry.id_of::<T>() {
                Some(id) => id,
                None => continue,
            };

            let info = match self.world.registry.info(comp_id) {
                Some(info) => info,
                None => continue,
            };

            let comp_idx = match archetype.component_index(comp_id) {
                Some(idx) => idx as u16,
                None => continue,
            };

            for chunk in archetype.chunks.iter() {
                for (slot, &entity) in chunk.entity_ids.iter().enumerate() {
                    if info.is_pod {
                        let ptr = chunk.component_ptr(comp_idx, slot as u16) as *const T;
                        let component = unsafe { &*ptr };
                        self.current_entities.push((entity, component));
                    } else {
                        if let Some(any) = chunk.read_boxed_component(comp_idx, slot as u16) {
                            if let Some(component) = any.downcast_ref::<T>() {
                                self.current_entities.push((entity, component));
                            }
                        }
                    }
                }
            }
        }

        if self.current_offset < self.current_entities.len() {
            let (entity, component) = self.current_entities[self.current_offset];
            self.current_offset += 1;
            Some((entity, component))
        } else {
            None
        }
    }
}

/// Mutable query for component access.
pub struct QueryMut<'w, T> {
    world: &'w mut World,
    _marker: PhantomData<T>,
}

impl<'w, T> QueryMut<'w, T> {
    pub(crate) fn new(world: &'w mut World) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }

    pub fn iter_mut(&mut self) -> QueryMutIter<'_, T> {
        QueryMutIter {
            world: self.world,
            archetype_iter: self.world.archetypes.iter_mut(),
            current_entities: Vec::new(),
            current_offsets: Vec::new(),
            _marker: PhantomData,
        }
    }
}

/// Holds a mutable reference to a component. Since we yield `&'w mut T` which
/// cannot be stored in a Vec, we store raw pointers for Pod and use the
/// boxed storage's `&mut dyn Any` directly.
struct ComponentMut<'w, T> {
    entity: Entity,
    pod_ptr: Option<*mut T>,
    boxed_any: Option<&'w mut dyn Any>,
}

pub struct QueryMutIter<'w, T> {
    world: &'w mut World,
    archetype_iter: std::vec::IntoIter<&'w mut crate::archetype::Archetype>,
    current_entities: Vec<ComponentMut<'w, T>>,
    current_offsets: Vec<usize>,
    _marker: PhantomData<T>,
}

impl<'w, T: Component> Iterator for QueryMutIter<'w, T> {
    type Item = (Entity, &'w mut T);

    fn next(&mut self) -> Option<Self::Item> {
        while self.current_offsets.is_empty()
            || self.current_offsets.last().copied()? >= self.current_entities.len()
        {
            let archetype = match self.archetype_iter.next() {
                Some(a) => a,
                None => return None,
            };
            self.current_entities.clear();
            self.current_offsets.clear();
            self.current_offsets.push(0);

            let comp_id = match self.world.registry.id_of::<T>() {
                Some(id) => id,
                None => continue,
            };

            let info = match self.world.registry.info(comp_id) {
                Some(info) => info,
                None => continue,
            };

            let comp_idx = match archetype.component_index(comp_id) {
                Some(idx) => idx as u16,
                None => continue,
            };

            if info.is_pod {
                for chunk in archetype.chunks.iter_mut() {
                    for (slot, &entity) in chunk.entity_ids.iter().enumerate() {
                        let ptr = chunk.component_ptr(comp_idx, slot as u16) as *mut T;
                        self.current_entities.push(ComponentMut {
                            entity,
                            pod_ptr: Some(ptr),
                            boxed_any: None,
                        });
                    }
                }
            } else {
                for chunk in archetype.chunks.iter_mut() {
                    for (slot, &entity) in chunk.entity_ids.iter().enumerate() {
                        let boxed = chunk.read_boxed_component_mut(comp_idx, slot as u16);
                        self.current_entities.push(ComponentMut {
                            entity,
                            pod_ptr: None,
                            boxed_any: boxed,
                        });
                    }
                }
            }
        }

        if let Some(offset) = self.current_offsets.last_mut() {
            while *offset < self.current_entities.len() {
                let cm = &mut self.current_entities[*offset];
                *offset += 1;
                if let Some(ptr) = cm.pod_ptr {
                    let component = unsafe { &mut *ptr };
                    return Some((cm.entity, component));
                } else if let Some(any) = cm.boxed_any.as_deref_mut() {
                    if let Some(component) = any.downcast_mut::<T>() {
                        return Some((cm.entity, component));
                    }
                }
            }
        }
        None
    }
}

/// Parallel immutable query iterator over chunks.
pub struct ParIter<'w, T: Component> {
    world: &'w World,
    _marker: PhantomData<T>,
}

impl<'w, T: Component> ParIter<'w, T> {
    pub fn new(world: &'w World) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }

    pub fn for_each<F>(&self, f: F)
    where
        F: Fn(Entity, &T) + Send + Sync,
    {
        self.world.par_for_each_chunk(|archetype_id, chunk_index, _len| {
            let archetype = self.world.archetypes.get(archetype_id);
            let comp_id = match self.world.registry.id_of::<T>() {
                Some(id) => id,
                None => return,
            };
            let info = match self.world.registry.info(comp_id) {
                Some(info) => info,
                None => return,
            };
            let comp_idx = match archetype.component_index(comp_id) {
                Some(idx) => idx as u16,
                None => return,
            };
            let chunk = archetype.chunk(chunk_index);
            for slot in 0..chunk.len {
                let entity = chunk.entity_ids[slot as usize];
                if info.is_pod {
                    let ptr = chunk.component_ptr(comp_idx, slot) as *const T;
                    let component = unsafe { &*ptr };
                    f(entity, component);
                } else {
                    if let Some(any) = chunk.read_boxed_component(comp_idx, slot) {
                        if let Some(component) = any.downcast_ref::<T>() {
                            f(entity, component);
                        }
                    }
                }
            }
        });
    }
}
