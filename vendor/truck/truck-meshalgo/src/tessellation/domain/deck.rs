//! Integer Potential Union-Find Solver for Periodic Lattice Constraints.
//!
//! Tracks deck equivalence potentials across boundary cycles and enforces exact
//! periodic deck displacements (m, n) ∈ ℤ² without ad-hoc seam shift heuristics.

use std::collections::HashMap;

/// Integer lattice displacement vector (m, n) ∈ ℤ² in deck cover space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LatticePotential(pub i64, pub i64);

impl LatticePotential {
    /// Zero potential (0, 0).
    pub fn zero() -> Self {
        Self(0, 0)
    }

    /// Rank 1 potential (k, 0).
    pub fn rank1(k: i64) -> Self {
        Self(k, 0)
    }
}

/// Union-find data structure carrying integer lattice offsets to parent node.
#[derive(Debug, Default)]
pub struct DeckPotentialUnionFind {
    parent: HashMap<usize, usize>,
    offset_to_parent: HashMap<usize, LatticePotential>,
}

impl DeckPotentialUnionFind {
    /// Creates a new empty potential union-find instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initializes a singleton set for `node`.
    pub fn make_set(&mut self, node: usize) {
        self.parent.entry(node).or_insert(node);
        self.offset_to_parent
            .entry(node)
            .or_insert(LatticePotential::zero());
    }

    /// Finds the representative root and accumulated lattice potential offset for `node`.
    pub fn find(&mut self, node: usize) -> (usize, LatticePotential) {
        self.make_set(node);
        let p = self.parent[&node];
        if p == node {
            (node, LatticePotential::zero())
        } else {
            let (root, parent_offset) = self.find(p);
            let node_offset = self.offset_to_parent[&node];
            let combined = LatticePotential(
                node_offset.0 + parent_offset.0,
                node_offset.1 + parent_offset.1,
            );
            self.parent.insert(node, root);
            self.offset_to_parent.insert(node, combined);
            (root, combined)
        }
    }

    /// Connects node `a` to node `b` with exact deck offset `a_to_b = (da, db)`.
    pub fn union(&mut self, a: usize, b: usize, offset: LatticePotential) -> bool {
        let (root_a, pot_a) = self.find(a);
        let (root_b, pot_b) = self.find(b);
        if root_a == root_b {
            let expected_b = LatticePotential(pot_a.0 + offset.0, pot_a.1 + offset.1);
            expected_b == pot_b
        } else {
            self.parent.insert(root_a, root_b);
            let rel = LatticePotential(pot_b.0 - pot_a.0 - offset.0, pot_b.1 - pot_a.1 - offset.1);
            self.offset_to_parent.insert(root_a, rel);
            true
        }
    }
}
