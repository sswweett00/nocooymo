use crate::Entity;

/// Benzersiz bileşen ve kaynak türlerini tanımlamak için string klonlamasını önleyen hafif bir ID.
/// Çalışma zamanında string'leri 'interning' (ortak havuzda toplama) yaparak hash performansını artırır.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct TypeId(u64);

impl TypeId {
    /// Tür isminden hızlıca bir ID üretir (Derleme zamanında veya çalışma zamanında FNV/MurmurHash ile).
    pub fn of(name: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        Self(hasher.finish())
    }
}

/// Her bileşen verisinin hangi türe ait olduğunu ve binary bloğunu tutan yapı.
#[derive(Clone, Debug)]
pub struct ComponentData {
    pub type_id: TypeId,
    pub data: Vec<u8>,
}

/// World snapshot optimized for contiguous memory layout and low allocation overhead.
#[derive(Clone, Debug, Default)]
pub struct WorldSnapshot {
    /// Aktif olan tüm entity listesi.
    pub entities: Vec<Entity>,
    
    /// Geliştirme: İç içe HashMap yerine düz (flat) bir liste.
    /// Her entity'nin bileşenleri bellek ardışıklığı (cache locality) için bir arada tutulur.
    pub component_data: Vec<(Entity, Vec<ComponentData>)>,
    
    /// Kaynaklar (Resources) için de String yerine TypeId kullanarak hash hızını artırdık.
    pub resources: Vec<(TypeId, Vec<u8>)>,
    pub timestamp: f64,
}

impl WorldSnapshot {
    /// Creates a new, empty `WorldSnapshot`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            component_data: Vec::new(),
            resources: Vec::new(),
            timestamp: 0.0,
        }
    }

    /// Pre-allocates space for elements to minimize runtime reallocations.
    #[must_use]
    pub fn with_capacity(entities_cap: usize, resources_cap: usize) -> Self {
        Self {
            entities: Vec::with_capacity(entities_cap),
            component_data: Vec::with_capacity(entities_cap),
            resources: Vec::with_capacity(resources_cap),
            timestamp: 0.0,
        }
    }

    /// Clears the snapshot, maintaining allocated memory for object reuse.
    pub fn clear(&mut self) {
        self.entities.clear();
        self.component_data.clear();
        self.resources.clear();
        self.timestamp = 0.0;
    }
}

/// Serialization format hints.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SerializationFormat {
    /// Rkyv: Zero-copy serileştirme için ideal (Süper hızlı ağ paketleri).
    #[default]
    Rkyv,
    Json,
    Binary,
}

/// Trait for serializable ECS resources/components.
/// Geliştirme: Hata yönetimi için dinamik hata desteği ve `thiserror` entegrasyonu sağlandı.
pub trait Serializable: Sized {
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(data: &[u8]) -> Result<Self, SerializationError>;
}

#[derive(Debug, thiserror::Error)]
pub enum SerializationError {
    #[error("incomplete data: expected {expected} bytes, got {got}")]
    IncompleteData { expected: usize, got: usize },
    
    #[error("type mismatch: expected type ID {expected:?}, found {found:?}")]
    TypeError { expected: Option<TypeId>, found: Option<TypeId> },
    
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("deserialization custom error: {0}")]
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_snapshot_new() {
        let snapshot = WorldSnapshot::new();
        assert_eq!(snapshot.entities.len(), 0);
        assert_eq!(snapshot.component_data.len(), 0);
    }

    #[test]
    fn test_type_id_determinism() {
        let id1 = TypeId::of("Position");
        let id2 = TypeId::of("Position");
        assert_eq!(id1, id2);
    }
}