use std::collections::{HashMap, HashSet};
use std::cmp::Ordering;
use serde::{Deserialize, Serialize};
use elysium_core::Entity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalTimestamp {
    pub vector: HashMap<u64, u64>, // actor_id -> timestamp
    pub actor_id: u64,
}

impl CausalTimestamp {
    pub fn new(actor_id: u64) -> Self {
        Self {
            vector: HashMap::new(),
            actor_id,
        }
    }

    pub fn increment(&mut self) -> u64 {
        let new_time = self.vector.get(&self.actor_id).unwrap_or(&0) + 1;
        self.vector.insert(self.actor_id, new_time);
        new_time
    }

    pub fn merge(&mut self, other: &CausalTimestamp) {
        for (actor, ts) in &other.vector {
            let current_ts = self.vector.entry(*actor).or_insert(0);
            *current_ts = (*current_ts).max(*ts);
        }
    }

    pub fn compare(&self, other: &CausalTimestamp) -> Ordering {
        let mut greater_equal = true;
        let mut less_equal = true;

        for (actor, ts) in &self.vector {
            let other_ts = other.vector.get(actor).unwrap_or(&0);
            if ts > other_ts {
                less_equal = false;
            } else if ts < other_ts {
                greater_equal = false;
            }
        }

        for (actor, ts) in &other.vector {
            let self_ts = self.vector.get(actor).unwrap_or(&0);
            if ts > self_ts {
                greater_equal = false;
            } else if ts < self_ts {
                less_equal = false;
            }
        }

        if greater_equal && !less_equal {
            Ordering::Greater
        } else if !greater_equal && less_equal {
            Ordering::Less
        } else if greater_equal && less_equal {
            Ordering::Equal
        } else {
            Ordering::Less // concurrent events
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtValue<T> {
    pub value: T,
    pub timestamp: CausalTimestamp,
}

impl<T> CrdtValue<T> {
    pub fn new(value: T, timestamp: CausalTimestamp) -> Self {
        Self { value, timestamp }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtMap<T> {
    pub values: HashMap<String, CrdtValue<T>>,
    pub tombstones: HashSet<String>, // deleted keys
}

impl<T: Clone + Serialize + for<'de> Deserialize<'de>> CrdtMap<T> {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            tombstones: HashSet::new(),
        }
    }

    pub fn insert(&mut self, key: String, value: T, mut timestamp: CausalTimestamp) -> bool {
        if self.tombstones.contains(&key) {
            return false; // Key has been deleted permanently
        }

        timestamp.increment();
        let crdt_value = CrdtValue {
            value,
            timestamp,
        };

        if let Some(existing_value) = self.values.get(&key) {
            if existing_value.timestamp.compare(&crdt_value.timestamp) == Ordering::Greater {
                return false; // Incoming value is older
            }
        }

        self.values.insert(key, crdt_value);
        true
    }

    pub fn get(&self, key: &str) -> Option<&T> {
        if self.tombstones.contains(key) {
            return None;
        }
        self.values.get(key).map(|v| &v.value)
    }

    pub fn remove(&mut self, key: &str, _timestamp: CausalTimestamp) {
        self.tombstones.insert(key.to_string());
        self.values.remove(key);
    }

    pub fn merge(&mut self, other: &CrdtMap<T>) {
        // Merge values
        for (key, other_value) in &other.values {
            if self.tombstones.contains(key) || other.tombstones.contains(key) {
                continue; // Skip if either side has deleted the key
            }

            if let Some(self_value) = self.values.get(key) {
                // If timestamps are concurrent, pick one deterministically (by comparing values)
                if self_value.timestamp.compare(&other_value.timestamp) == Ordering::Less {
                    self.values.insert(key.clone(), other_value.clone());
                }
            } else {
                self.values.insert(key.clone(), other_value.clone());
            }
        }

        // Merge tombstones
        for key in &other.tombstones {
            self.tombstones.insert(key.clone());
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrdtType {
    LWW(ElementLww),      // Last-Writer-Wins Element
    PNCounter(PnCounter), // Positive-Negative Counter
    ORMap(OrMap),         // Observed-Remove Map
    YType(YType),         // Yjs-like shared type
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementLww {
    pub value: Vec<u8>, // Serialized value
    pub timestamp: CausalTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnCounter {
    pub positive: HashMap<u64, u32>, // actor_id -> increment_value
    pub negative: HashMap<u64, u32>, // actor_id -> decrement_value
}

impl PnCounter {
    pub fn new() -> Self {
        Self {
            positive: HashMap::new(),
            negative: HashMap::new(),
        }
    }

    pub fn increment(&mut self, actor_id: u64, value: u32) {
        *self.positive.entry(actor_id).or_insert(0) += value;
    }

    pub fn decrement(&mut self, actor_id: u64, value: u32) {
        *self.negative.entry(actor_id).or_insert(0) += value;
    }

    pub fn value(&self) -> i32 {
        let pos_sum: u32 = self.positive.values().sum();
        let neg_sum: u32 = self.negative.values().sum();
        pos_sum as i32 - neg_sum as i32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrMap {
    pub values: HashMap<String, CrdtValue<Vec<u8>>>, // key -> value with timestamp
    pub tombstones: HashSet<String>,
}

impl OrMap {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            tombstones: HashSet::new(),
        }
    }

    pub fn insert(&mut self, key: String, value: Vec<u8>, timestamp: CausalTimestamp) {
        self.values.insert(key, CrdtValue::new(value, timestamp));
    }

    pub fn remove(&mut self, key: &str, timestamp: CausalTimestamp) {
        self.tombstones.insert(key.to_string());
        self.values.remove(key);
    }

    pub fn get(&self, key: &str) -> Option<&CrdtValue<Vec<u8>>> {
        if self.tombstones.contains(key) {
            None
        } else {
            self.values.get(key)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YType {
    pub items: Vec<YItem>,
    pub clock: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YItem {
    pub id: u32,
    pub content: Vec<u8>,
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtManager {
    pub entities: HashMap<Entity, CrdtMap<elysium_core::Transform>>,
    pub timestamp: CausalTimestamp,
}

impl CrdtManager {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            timestamp: CausalTimestamp::new(rand::random()),
        }
    }

    pub fn update_entity_transform(&mut self, entity: Entity, transform: elysium_core::Transform) {
        let entity_map = self.entities.entry(entity).or_insert_with(|| CrdtMap::new());
        entity_map.insert(
            "transform".to_string(),
            transform,
            self.timestamp.clone(),
        );
    }

    pub fn get_entity_transform(&self, entity: Entity) -> Option<elysium_core::Transform> {
        self.entities
            .get(&entity)
            .and_then(|entity_map| entity_map.get("transform").cloned())
    }

    pub fn merge_entity(&mut self, entity: Entity, other_entity_map: &CrdtMap<elysium_core::Transform>) {
        // Fixed: avoid multiple mutable borrows
        let timestamp_clone = self.timestamp.clone();
        if let Some(entity_map) = self.entities.get_mut(&entity) {
            entity_map.merge(other_entity_map);
        } else {
            self.entities.insert(entity, other_entity_map.clone());
        }
    }

    pub fn remove_entity(&mut self, entity: Entity) {
        // Fixed: avoid multiple mutable borrows
        let _timestamp_clone = self.timestamp.clone();
        if let Some(entity_map) = self.entities.get_mut(&entity) {
            entity_map.remove("transform", _timestamp_clone);
        }
    }

    pub fn get_timestamp(&mut self) -> CausalTimestamp {
        self.timestamp.increment();
        self.timestamp.clone()
    }

    pub fn merge(&mut self, other: &CrdtManager) {
        for (entity, other_entity_map) in &other.entities {
            self.merge_entity(*entity, other_entity_map);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pn_counter() {
        let mut counter = PnCounter::new();
        counter.increment(1, 5);
        counter.increment(1, 3);
        counter.decrement(1, 2);
        assert_eq!(counter.value(), 6); // 5+3-2=6
    }

    #[test]
    fn test_crdt_map_insert_get() {
        let mut map: CrdtMap<String> = CrdtMap::new();
        let ts = CausalTimestamp::new(1);
        assert!(map.insert("key".to_string(), "value".to_string(), ts));
        assert_eq!(map.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_crdt_manager_transform() {
        let mut manager = CrdtManager::new();
        let entity = elysium_core::Entity::from_parts(1, 0);
        let transform = elysium_core::Transform::default();
        manager.update_entity_transform(entity, transform.clone());
        let retrieved = manager.get_entity_transform(entity);
        assert!(retrieved.is_some());
    }
}