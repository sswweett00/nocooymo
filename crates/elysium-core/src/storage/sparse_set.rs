use crate::entity::Entity;

/// Sparse set for tag components and fast entity membership queries.
pub struct SparseSet {
    sparse: Vec<u32>,
    dense_entities: Vec<Entity>,
    // Geliştirme: dense_indices kaldırıldı. dense_entities içindeki entity'lerden 
    // index'e zaten anında erişebiliyoruz (entity.index()). Çift bellek kullanımını engelledik.
}

impl SparseSet {
    /// Creates a new, empty `SparseSet`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sparse: Vec::new(),
            dense_entities: Vec::new(),
        }
    }

    /// Creates a new `SparseSet` with pre-allocated capacity for dense and sparse arrays.
    #[must_use]
    pub fn with_capacity(dense_cap: usize, sparse_cap: usize) -> Self {
        Self {
            sparse: vec![u32::MAX; sparse_cap],
            dense_entities: Vec::with_capacity(dense_cap),
        }
    }

    /// Inserts an entity into the set. Returns `true` if it was newly inserted.
    pub fn insert(&mut self, entity: Entity) -> bool {
        let index = entity.index() as usize;
        self.ensure_sparse(index);
        
        // Güvenlik & Performans: Bound check'leri (sınır kontrollerini) optimize etmek için
        // get_mut ve unsafe unchecked erişim yerine Rust'ın optimizer'ına yardım ediyoruz.
        if self.sparse[index] != u32::MAX {
            return false;
        }

        let dense_index = self.dense_entities.len() as u32;
        self.sparse[index] = dense_index;
        self.dense_entities.push(entity);
        true
    }

    /// Removes an entity from the set. Returns `true` if the entity was present.
    pub fn remove(&mut self, entity: Entity) -> bool {
        let index = entity.index() as usize;
        if index >= self.sparse.len() {
            return false;
        }

        let dense_index = self.sparse[index];
        if dense_index == u32::MAX {
            return false;
        }
        
        let dense_index = dense_index as usize;
        let last_dense_idx = self.dense_entities.len() - 1;

        if dense_index != last_dense_idx {
            // Swap-and-pop mantığı
            let moved = self.dense_entities[last_dense_idx];
            self.dense_entities[dense_index] = moved;
            self.sparse[moved.index() as usize] = dense_index as u32;
        }

        self.dense_entities.pop();
        self.sparse[index] = u32::MAX;
        true
    }

    /// Returns `true` if the set contains the given entity.
    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        let index = entity.index() as usize;
        // Geliştirme: Sadece sparse array içinde var mı diye bakmak yetmez, 
        // neslin (generation) değişip değişmediğini doğrulamak için dense array kontrolü ekledik.
        if let Some(&dense_index) = self.sparse.get(index) {
            if dense_index != u32::MAX {
                if let Some(&dense_entity) = self.dense_entities.get(dense_index as usize) {
                    return dense_entity == entity;
                }
            }
        }
        false
    }

    /// Returns a slice containing all entities in the set.
    #[inline]
    pub fn entities(&self) -> &[Entity] {
        &self.dense_entities
    }

    /// Returns the number of entities in the set.
    #[inline]
    pub fn len(&self) -> usize {
        self.dense_entities.len()
    }

    /// Returns `true` if the set contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dense_entities.is_empty()
    }

    /// Clears the set, removing all entities. Keeps allocated memory.
    pub fn clear(&mut self) {
        // Tüm sparse array'i yeniden tahsis etmek yerine içini u32::MAX ile dolduruyoruz (O(n))
        // Eğer çok büyükse alternatif olarak sadece kayıtlı dense_entities'lerin 
        // karşılık gelen sparse slotlarını sıfırlayabilirsiniz (O(dense_len)).
        for entity in &self.dense_entities {
            self.sparse[entity.index() as usize] = u32::MAX;
        }
        self.dense_entities.clear();
    }

    fn ensure_sparse(&mut self, index: usize) {
        if self.sparse.len() <= index {
            // Performans: Sık sık resize tetiklenmesini önlemek için 
            // kapasiteyi mevcut boyutun 2 katına (veya ihtiyaç duyulan kadar) çıkarıyoruz.
            let new_len = std::cmp::max(self.sparse.len() * 2, index + 1);
            self.sparse.resize(new_len, u32::MAX);
        }
    }
}

impl Default for SparseSet {
    fn default() -> Self {
        Self::new()
    }
}