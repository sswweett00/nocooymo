//! Broad phase — uniform spatial-hash grid (Mimari §3.1 F12 "hücre grid").
//!
//! Bodies are binned into 3D cells by their (fattened) AABB. Candidate pairs
//! are emitted when two bodies share at least one cell, avoiding the O(n^2)
//! all-pairs scan.

use std::collections::HashMap;

use crate::math::{Aabb, Vec3};

/// A grid cell coordinate.
pub type CellCoord = (i32, i32, i32);

/// Uniform spatial hash grid used to prune collision pairs.
#[derive(Debug, Clone)]
pub struct UniformGrid {
    cell_size: f32,
    margin: f32,
    cells: HashMap<CellCoord, Vec<u32>>,
}

impl Default for UniformGrid {
    fn default() -> Self {
        Self::new(2.0, 0.1)
    }
}

impl UniformGrid {
    pub fn new(cell_size: f32, margin: f32) -> Self {
        Self {
            cell_size: cell_size.max(0.1),
            margin: margin.max(0.0),
            cells: HashMap::new(),
        }
    }

    fn coord_of(&self, p: Vec3) -> CellCoord {
        (
            (p.x / self.cell_size).floor() as i32,
            (p.y / self.cell_size).floor() as i32,
            (p.z / self.cell_size).floor() as i32,
        )
    }

    /// Insert all bodies by binning their fattened AABBs.
    pub fn rebuild(&mut self, aabbs: &[Aabb]) {
        self.cells.clear();
        for (idx, aabb) in aabbs.iter().enumerate() {
            let fat = aabb.fatten(self.margin);
            let (lo, hi) = fat.cells(self.cell_size);
            let lo = self.coord_of(lo * self.cell_size);
            let hi = self.coord_of(hi * self.cell_size);
            for x in lo.0..=hi.0 {
                for y in lo.1..=hi.1 {
                    for z in lo.2..=hi.2 {
                        self.cells.entry((x, y, z)).or_default().push(idx as u32);
                    }
                }
            }
        }
    }

    /// Emit unique candidate pairs whose fattened AABBs intersect.
    pub fn pairs(&self, aabbs: &[Aabb]) -> Vec<(u32, u32)> {
        let mut out: Vec<(u32, u32)> = Vec::new();
        let mut seen: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
        for (idx, aabb) in aabbs.iter().enumerate() {
            let fat = aabb.fatten(self.margin);
            let (lo, hi) = fat.cells(self.cell_size);
            let lo = self.coord_of(lo * self.cell_size);
            let hi = self.coord_of(hi * self.cell_size);
            for x in lo.0..=hi.0 {
                for y in lo.1..=hi.1 {
                    for z in lo.2..=hi.2 {
                        if let Some(neighbors) = self.cells.get(&(x, y, z)) {
                            for &nb in neighbors {
                                let (i, j) = (idx as u32, nb);
                                let (a, b) = if i <= j { (i, j) } else { (j, i) };
                                if a == b {
                                    continue;
                                }
                                // Require AABB overlap before emitting.
                                if aabbs[a as usize].intersects(&aabbs[b as usize])
                                    && seen.insert((a, b))
                                {
                                    out.push((a, b));
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;

    #[test]
    fn finds_overlapping_pairs_but_not_far_pairs() {
        let a = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let b = Aabb::new(Vec3::new(0.5, 0.5, 0.5), Vec3::new(1.5, 1.5, 1.5));
        let c = Aabb::new(Vec3::new(50.0, 50.0, 50.0), Vec3::new(51.0, 51.0, 51.0));
        let aabbs = vec![a, b, c];
        let mut grid = UniformGrid::new(2.0, 0.05);
        grid.rebuild(&aabbs);
        let pairs = grid.pairs(&aabbs);
        assert_eq!(pairs, vec![(0, 1)]);
    }
}
