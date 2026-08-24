//! Scene hierarchy system: parent-child relationships and world matrix propagation.

use std::collections::HashMap;

use crate::entity::Entity;
use crate::math::{Mat4, Vec3};
use crate::transform::Transform;
use crate::world::World;

// ---------------------------------------------------------------------------
// Hierarchy component
// ---------------------------------------------------------------------------

/// Component storing parent and children references.
#[derive(Clone, Debug, Default)]
pub struct HierarchyNode {
    pub parent: Option<Entity>,
    pub children: Vec<Entity>,
}

// ---------------------------------------------------------------------------
// Hierarchy system
// ---------------------------------------------------------------------------

/// Manages parent-child relationships and world matrix propagation.
pub struct HierarchySystem {
    /// Flat list of entities in depth-first order (parents before children).
    sorted: Vec<Entity>,
    dirty: bool,
}

impl HierarchySystem {
    pub fn new() -> Self {
        Self {
            sorted: Vec::new(),
            dirty: true,
        }
    }

    /// Set a parent-child relationship.
    pub fn set_parent(&mut self, world: &mut World, child: Entity, parent: Option<Entity>) {
        // Remove from old parent - clone the old parent first
        let old_parent = {
            if let Some(node) = world.get_component::<HierarchyNode>(child) {
                node.parent
            } else {
                None
            }
        };
        
        if let Some(old_parent) = old_parent {
            if let Some(old_node) = world.get_component_mut::<HierarchyNode>(old_parent) {
                old_node.children.retain(|e| e != &child);
            }
        }
        
        // Set new parent
        if let Some(node) = world.get_component_mut::<HierarchyNode>(child) {
            node.parent = parent;
        } else {
            // Create the node if it doesn't exist
            world.insert_component(
                child,
                HierarchyNode {
                    parent,
                    children: Vec::new(),
                },
            );
        }

        // Add to new parent
        if let Some(parent) = parent {
            if world.get_component::<HierarchyNode>(parent).is_none() {
                world.insert_component(
                    parent,
                    HierarchyNode {
                        parent: None,
                        children: Vec::new(),
                    },
                );
            }
            if let Some(node) = world.get_component_mut::<HierarchyNode>(parent) {
                if !node.children.contains(&child) {
                    node.children.push(child);
                }
            }
        }

        self.dirty = true;
    }

    /// Get the parent of an entity.
    pub fn parent(&self, world: &World, entity: Entity) -> Option<Entity> {
        world
            .get_component::<HierarchyNode>(entity)
            .and_then(|n| n.parent)
    }

    /// Get the children of an entity.
    pub fn children<'a>(&self, world: &'a World, entity: Entity) -> &'a [Entity] {
        world
            .get_component::<HierarchyNode>(entity)
            .map(|n| n.children.as_slice())
            .unwrap_or(&[])
    }

    /// Traverse the hierarchy depth-first.
    pub fn traverse_depth_first(&self, world: &World, root: Entity) -> Vec<Entity> {
        let mut result = Vec::new();
        let mut stack = vec![root];
        while let Some(entity) = stack.pop() {
            result.push(entity);
            if let Some(node) = world.get_component::<HierarchyNode>(entity) {
                // Push children in reverse so they're processed in order
                for child in node.children.iter().rev() {
                    stack.push(*child);
                }
            }
        }
        result
    }

    /// Update world matrices for all entities with transforms.
    pub fn update(&mut self, world: &mut World) {
        // Collect all entities with HierarchyNode
        let roots: Vec<Entity> = world
            .entities()
            .filter(|e| {
                world
                    .get_component::<HierarchyNode>(*e)
                    .map(|n| n.parent.is_none())
                    .unwrap_or(true)
            })
            .collect();

        // For root entities (no parent or no hierarchy node), world = local
        for root in &roots {
            if let Some(transform) = world.get_component_mut::<Transform>(*root) {
                let local = transform.local_matrix();
                transform.set_world_matrix(local);
            }
        }

        // Propagate to children depth-first
        for root in roots {
            self.propagate(world, root);
        }
    }

    fn propagate(&self, world: &mut World, entity: Entity) {
        let mut visited = std::collections::HashSet::new();
        self.propagate_inner(world, entity, &mut visited);
    }

    fn propagate_inner(&self, world: &mut World, entity: Entity, visited: &mut std::collections::HashSet<crate::entity::Entity>) {
        if !visited.insert(entity) {
            return; // döngü tespit edildi
        }
        let children: Vec<Entity> = world
            .get_component::<HierarchyNode>(entity)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        let parent_world = world
            .get_component::<Transform>(entity)
            .map(|t| t.world_matrix())
            .unwrap_or(Mat4::IDENTITY);
        for child in children {
            if let Some(transform) = world.get_component_mut::<Transform>(child) {
                let local = transform.local_matrix();
                transform.set_world_matrix(parent_world * local);
            }
            self.propagate_inner(world, child, visited);
        }
    }

    /// Detach all children from an entity (used before despawning).
    pub fn detach_children(&self, world: &mut World, entity: Entity) {
        let children: Vec<Entity> = world
            .get_component::<HierarchyNode>(entity)
            .map(|n| n.children.clone())
            .unwrap_or_default();

        for child in children {
            if let Some(node) = world.get_component_mut::<HierarchyNode>(child) {
                node.parent = None;
            }
        }

        if let Some(node) = world.get_component_mut::<HierarchyNode>(entity) {
            node.children.clear();
        }
    }
}

impl Default for HierarchySystem {
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
    fn set_parent_child() {
        let mut world = World::new();
        let parent = world.spawn();
        let child = world.spawn();

        let mut hierarchy = HierarchySystem::new();
        hierarchy.set_parent(&mut world, child, Some(parent));

        let node = world.get_component::<HierarchyNode>(child).unwrap();
        assert_eq!(node.parent, Some(parent));

        let parent_node = world.get_component::<HierarchyNode>(parent).unwrap();
        assert!(parent_node.children.contains(&child));
    }

    #[test]
    fn world_matrix_propagation() {
        let mut world = World::new();
        let parent = world.spawn();
        let child = world.spawn();

        world.insert_component(
            parent,
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
        );
        world.insert_component(
            child,
            Transform::from_translation(Vec3::new(1.0, 0.0, 0.0)),
        );

        let mut hierarchy = HierarchySystem::new();
        hierarchy.set_parent(&mut world, child, Some(parent));

        let mut hsys = HierarchySystem::new();
        hsys.update(&mut world);

        let child_transform = world.get_component::<Transform>(child).unwrap();
        let world_pos = child_transform.world_matrix().w_axis.truncate();
        assert!((world_pos - Vec3::new(11.0, 0.0, 0.0)).length() < 0.01);
    }

    #[test]
    fn traverse_depth_first() {
        let mut world = World::new();
        let root = world.spawn();
        let c1 = world.spawn();
        let c2 = world.spawn();
        let gc1 = world.spawn();

        let mut hierarchy = HierarchySystem::new();
        hierarchy.set_parent(&mut world, c1, Some(root));
        hierarchy.set_parent(&mut world, c2, Some(root));
        hierarchy.set_parent(&mut world, gc1, Some(c1));

        let order = hierarchy.traverse_depth_first(&world, root);
        assert_eq!(order[0], root);
        // c1 should come before gc1
        let c1_idx = order.iter().position(|e| *e == c1).unwrap();
        let gc1_idx = order.iter().position(|e| *e == gc1).unwrap();
        assert!(c1_idx < gc1_idx);
    }
}