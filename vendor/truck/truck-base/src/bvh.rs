//! BG-SOL-P0-BVH — the certified solver family's broad-phase substrate.
//!
//! The BVH and the Bézier-span cache (`truck-geometry/src/span.rs`) share one
//! abstraction, `BoundedPiece` (`bbox`, `derivative_bounds`, `subdivide`).
//! Analytic faces, Bézier surface spans, Bézier curve spans, trim segments and
//! intersection candidates all enter the same broad phase:
//! BREP → faces → carrier spans → BVH nodes → candidate span pairs →
//! certified solver (docs/SOLVER_FAMILY_PLAN.md §2, §4).
//!
//! This module is the **broad-phase box**: a plain `f64` axis-aligned box. It
//! deliberately does NOT use `truck-evidence`'s certified `Box3` — `truck-base`
//! has no `inari` dependency, and the broad phase culls candidates; it never
//! certifies. The certified stage later converts conservative enclosures into
//! this type (`.inf()`/`.sup()` endpoints) when it feeds the BVH.
//!
//! Implemented by packet BG-SOL-P0-BVH:
//!
//! - `BoundingBox<Point3>::intersects` — the one missing box primitive.
//! - `DerivativeBounds` — conservative first/second partial bounds (or none).
//! - `BoundedPiece` — the shared `bbox`/`derivative_bounds`/`subdivide` trait.
//! - `Bvh<P>` — a flat pre-order node array with contiguous leaves, built
//!   deterministically and queried by leaf-box overlap only.
//!
//! House rules H-1..H-8 apply.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::bounding_box::BoundingBox;
use crate::cgmath64::Point3;
use std::cmp::Ordering;
use std::marker::PhantomData;

impl BoundingBox<Point3> {
    /// Whether the two boxes overlap in all three axes (closed boxes; an
    /// empty box never intersects).
    pub fn intersects(&self, other: &Self) -> bool {
        !(self.max().x < other.min().x
            || other.max().x < self.min().x
            || self.max().y < other.min().y
            || other.max().y < self.min().y
            || self.max().z < other.min().z
            || other.max().z < self.min().z)
    }
}

/// Conservative bounds on a piece's first and second partials over its whole
/// domain. An EMPTY `first` box means "no certified derivative bound is
/// available" (e.g. a rational surface, whose derivative control points are
/// not a hull); consumers must not use an empty box for culling. The broad
/// phase only ever reads `bbox`; `derivative_bounds` exists so the solver
/// phases can use the same pieces without re-extraction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DerivativeBounds {
    /// Box containing every first partial of the piece.
    pub first: BoundingBox<Point3>,
    /// Box containing every second partial of the piece.
    pub second: BoundingBox<Point3>,
}

impl DerivativeBounds {
    /// Both bounds unknown (empty boxes).
    pub fn new() -> Self {
        Self {
            first: BoundingBox::new(),
            second: BoundingBox::new(),
        }
    }
    /// Whether a certified first-derivative bound is available.
    pub fn is_known(&self) -> bool {
        !self.first.is_empty()
    }
}

impl Default for DerivativeBounds {
    fn default() -> Self {
        Self::new()
    }
}

/// The shared broad-phase abstraction (plan §2): everything that enters the
/// BVH reports a conservative bounding box, optional derivative bounds, and a
/// subdivision into smaller pieces.
pub trait BoundedPiece {
    /// A conservative box containing the piece's image. MUST contain the whole
    /// piece (soundness); looseness is acceptable.
    fn bbox(&self) -> BoundingBox<Point3>;
    /// Conservative bounds on the piece's partials; empty boxes mean unknown.
    fn derivative_bounds(&self) -> DerivativeBounds;
    /// Subdivide into smaller pieces covering the same image; an empty vec is
    /// a valid answer meaning "cannot subdivide".
    fn subdivide(&self) -> Vec<Self>
    where
        Self: Sized;
}

/// The maximum number of primitives a leaf node may carry.
const LEAF_CAP: usize = 8;

/// A deterministic flat-array BVH over pieces sharing the `BoundedPiece`
/// abstraction. The broad phase produces candidate pairs cheaply and
/// deterministically; it certifies nothing. `primitives` holds indices into
/// the `&[P]` slice passed to `build`; each leaf owns a contiguous range of
/// it.
#[derive(Debug)]
pub struct Bvh<P: BoundedPiece> {
    nodes: Vec<BvhNode>,
    primitives: Vec<u32>,
    _marker: PhantomData<P>,
}

#[derive(Clone, Copy, Debug)]
struct BvhNode {
    bbox: BoundingBox<Point3>,
    left: u32, // u32::MAX when this is a leaf
    right: u32,
    start: u32, // leaf: half-open range [start, start + count) into `primitives`
    count: u32, // 0 for interior nodes
}

impl<P: BoundedPiece> Bvh<P> {
    /// Builds the BVH over `pieces`. Deterministic: identical input produces
    /// an identical tree and identical query answers.
    pub fn build(pieces: &[P]) -> Self {
        let bboxes: Vec<BoundingBox<Point3>> = pieces.iter().map(P::bbox).collect();
        let mut primitives: Vec<u32> = (0..pieces.len() as u32).collect();
        let mut nodes: Vec<BvhNode> = Vec::new();
        if !pieces.is_empty() {
            build_range(&mut nodes, &mut primitives, &bboxes, 0, pieces.len());
        }
        Bvh {
            nodes,
            primitives,
            _marker: PhantomData,
        }
    }

    /// Leaf-box-overlapping primitive pairs (i, j) where i indexes `pieces` of
    /// THIS tree and j indexes the OTHER tree's `pieces`. The two trees may be
    /// the same object's trees from two different spans; they are NOT required
    /// to be different structures. Returns pairs sorted by (i, j).
    pub fn candidate_pairs(&self, other: &Self) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();
        if self.nodes.is_empty() || other.nodes.is_empty() {
            return pairs;
        }
        self.collect_pairs(other, 0, 0, &mut pairs);
        pairs.sort();
        pairs.dedup();
        pairs
    }

    /// Self-intersection pairs: (i, j) with i < j whose leaf boxes overlap.
    /// Sorted by (i, j). Primitive pairs INSIDE one leaf are included (two
    /// distinct pieces in the same leaf can overlap).
    pub fn candidate_pairs_self(&self) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();
        if self.nodes.is_empty() {
            return pairs;
        }
        self.traverse_self(0, &mut pairs);
        for pair in pairs.iter_mut() {
            if pair.0 > pair.1 {
                *pair = (pair.1, pair.0);
            }
        }
        pairs.sort();
        pairs.dedup();
        pairs
    }

    /// Indices of pieces whose leaf box intersects `aabb`. Sorted.
    pub fn query(&self, aabb: &BoundingBox<Point3>) -> Vec<usize> {
        let mut out = Vec::new();
        if self.nodes.is_empty() {
            return out;
        }
        self.traverse_query(0, aabb, &mut out);
        out.sort();
        out
    }

    /// A certified lower bound on the minimum distance between THIS piece set
    /// and `other`'s piece set: the returned value is `<=` the true minimum
    /// over every piece pair (one piece from each tree). The bound is a pure
    /// box-level certificate — every piece lies inside its leaf box, so the
    /// minimum leaf-box distance over the two trees is never above the true
    /// minimum. `f64::INFINITY` is returned exactly when the two sets are
    /// provably disjoint at the root (their root boxes do not overlap) or when
    /// either set is empty: no piece pair can then meet within any finite
    /// distance, so the separation is certified unbounded.
    ///
    /// The dual traversal is deterministic (the fixed child order of
    /// [`Bvh::candidate_pairs`], never hash- or parallel-dependent). A node
    /// pair is pruned once its box distance already exceeds the best-so-far
    /// upper bound on the true minimum; that upper bound is refined from leaf
    /// box pairs (their farthest-corner distance dominates the true minimum),
    /// so the surviving leaf pairs always include the minimizer and the
    /// reported minimum is exact at the box level.
    pub fn distance_lower_bound(&self, other: &Self) -> f64 {
        let Some(a) = self.nodes.first() else {
            return f64::INFINITY;
        };
        let Some(b) = other.nodes.first() else {
            return f64::INFINITY;
        };
        if box_distance_lower(&a.bbox, &b.bbox) > 0.0 {
            return f64::INFINITY;
        }
        let mut best = f64::INFINITY;
        let mut upper = f64::INFINITY;
        dual_distance_min(&self.nodes, &other.nodes, 0, 0, &mut best, &mut upper);
        best
    }

    /// A certified lower bound on the minimum distance between two DISTINCT
    /// pieces of this same tree: `<=` the true minimum over every `i < j`
    /// piece pair. A leaf holding two or more pieces cannot certify a positive
    /// separation for its own pairs (the pieces are only known through the
    /// leaf's union box, so their pair distance may be `0`), so the bound is
    /// `0` whenever any leaf carries more than one primitive. Returns
    /// `f64::INFINITY` when fewer than two primitives exist (no pair at all).
    /// Deterministic: same fixed traversal order as
    /// [`Bvh::candidate_pairs_self`].
    pub fn distance_lower_bound_self(&self) -> f64 {
        if self.primitives.len() < 2 {
            return f64::INFINITY;
        }
        if self.nodes.is_empty() {
            return f64::INFINITY;
        }
        let mut best = f64::INFINITY;
        let mut upper = f64::INFINITY;
        self_distance_min(&self.nodes, 0, &mut best, &mut upper);
        best
    }

    /// The number of primitives this BVH was built over.
    pub fn len(&self) -> usize {
        self.primitives.len()
    }

    /// Whether the BVH has no primitives.
    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }

    /// Emits the leaf-box-overlapping primitive pairs across the subtrees
    /// rooted at node `na` of `self` and node `nb` of `other`.
    fn collect_pairs(&self, other: &Self, na: usize, nb: usize, pairs: &mut Vec<(usize, usize)>) {
        let Some(a) = self.nodes.get(na) else { return };
        let Some(b) = other.nodes.get(nb) else { return };
        if !a.bbox.intersects(&b.bbox) {
            return;
        }
        let a_leaf = a.left == u32::MAX;
        let b_leaf = b.left == u32::MAX;
        if a_leaf && b_leaf {
            let a_end = a.start as usize + a.count as usize;
            let b_end = b.start as usize + b.count as usize;
            if let (Some(a_slice), Some(b_slice)) = (
                self.primitives.get(a.start as usize..a_end),
                other.primitives.get(b.start as usize..b_end),
            ) {
                for &i in a_slice.iter() {
                    for &j in b_slice.iter() {
                        pairs.push((i as usize, j as usize));
                    }
                }
            }
            return;
        }
        if a_leaf {
            self.collect_pairs(other, na, b.left as usize, pairs);
            self.collect_pairs(other, na, b.right as usize, pairs);
        } else if b_leaf {
            self.collect_pairs(other, a.left as usize, nb, pairs);
            self.collect_pairs(other, a.right as usize, nb, pairs);
        } else {
            self.collect_pairs(other, a.left as usize, b.left as usize, pairs);
            self.collect_pairs(other, a.left as usize, b.right as usize, pairs);
            self.collect_pairs(other, a.right as usize, b.left as usize, pairs);
            self.collect_pairs(other, a.right as usize, b.right as usize, pairs);
        }
    }

    /// Emits the self-intersection pairs of the subtree rooted at `node`.
    fn traverse_self(&self, node: usize, pairs: &mut Vec<(usize, usize)>) {
        let Some(n) = self.nodes.get(node) else {
            return;
        };
        if n.left == u32::MAX {
            let end = n.start as usize + n.count as usize;
            if let Some(range) = self.primitives.get(n.start as usize..end) {
                for (k, &i) in range.iter().enumerate() {
                    for &j in range.get(k + 1..).unwrap_or(&[]) {
                        if i < j {
                            pairs.push((i as usize, j as usize));
                        } else {
                            pairs.push((j as usize, i as usize));
                        }
                    }
                }
            }
            return;
        }
        self.traverse_self(n.left as usize, pairs);
        self.traverse_self(n.right as usize, pairs);
        self.collect_pairs(self, n.left as usize, n.right as usize, pairs);
    }

    /// Emits the indices of pieces whose leaf box intersects `aabb`, walking
    /// the subtree rooted at `node`.
    fn traverse_query(&self, node: usize, aabb: &BoundingBox<Point3>, out: &mut Vec<usize>) {
        let Some(n) = self.nodes.get(node) else {
            return;
        };
        if !n.bbox.intersects(aabb) {
            return;
        }
        if n.left == u32::MAX {
            let end = n.start as usize + n.count as usize;
            if let Some(range) = self.primitives.get(n.start as usize..end) {
                out.extend(range.iter().map(|&i| i as usize));
            }
            return;
        }
        self.traverse_query(n.left as usize, aabb, out);
        self.traverse_query(n.right as usize, aabb, out);
    }
}

/// The per-axis gap between two boxes, clamped at `0`: `max(lo_b - hi_a,
/// lo_a - hi_b)` is non-zero only on a strictly separating axis. Euclidean
/// combination of the three axis gaps gives a lower bound on the point-set
/// distance between the two boxes (port of the six-line `box_distance` formula
/// of `truck-evidence`'s feature-size substrate, reimplemented here because
/// `truck-base` cannot depend on `truck-evidence`).
fn box_distance_lower(a: &BoundingBox<Point3>, b: &BoundingBox<Point3>) -> f64 {
    let a_min = a.min();
    let a_max = a.max();
    let b_min = b.min();
    let b_max = b.max();
    let gap = |lo_a: f64, hi_a: f64, lo_b: f64, hi_b: f64| (lo_b - hi_a).max(lo_a - hi_b).max(0.0);
    let dx = gap(a_min.x, a_max.x, b_min.x, b_max.x);
    let dy = gap(a_min.y, a_max.y, b_min.y, b_max.y);
    let dz = gap(a_min.z, a_max.z, b_min.z, b_max.z);
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// The farthest-corner distance between two boxes: an upper bound on the
/// distance between ANY point pair, one point in each box. Because every piece
/// lies inside its box, this dominates the true minimum piece-set distance, so
/// it is a sound "best-so-far" pruning target for the dual distance walk.
fn box_distance_upper(a: &BoundingBox<Point3>, b: &BoundingBox<Point3>) -> f64 {
    let a_min = a.min();
    let a_max = a.max();
    let b_min = b.min();
    let b_max = b.max();
    let farthest =
        |lo_a: f64, hi_a: f64, lo_b: f64, hi_b: f64| (lo_a - hi_b).abs().max((hi_a - lo_b).abs());
    let dx = farthest(a_min.x, a_max.x, b_min.x, b_max.x);
    let dy = farthest(a_min.y, a_max.y, b_min.y, b_max.y);
    let dz = farthest(a_min.z, a_max.z, b_min.z, b_max.z);
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Walks the node pairs of the subtrees rooted at `na` (of `a_nodes`) and `nb`
/// (of `b_nodes`) in the fixed order of [`Bvh::candidate_pairs`], recording in
/// `best` the minimum leaf-box distance seen. A node pair whose box distance
/// already exceeds `upper` (or whose interior cannot beat a recorded `0`) is
/// pruned; `upper` is the best-so-far bound on the true minimum, tightened
/// from the farthest-corner distances of visited leaf pairs, so pruning never
/// discards the minimizing leaf pair.
fn dual_distance_min(
    a_nodes: &[BvhNode],
    b_nodes: &[BvhNode],
    na: usize,
    nb: usize,
    best: &mut f64,
    upper: &mut f64,
) {
    if *best == 0.0 {
        return;
    }
    let Some(a) = a_nodes.get(na) else {
        return;
    };
    let Some(b) = b_nodes.get(nb) else {
        return;
    };
    let d = box_distance_lower(&a.bbox, &b.bbox);
    if d > *upper {
        return;
    }
    let a_leaf = a.left == u32::MAX;
    let b_leaf = b.left == u32::MAX;
    if a_leaf && b_leaf {
        if d < *best {
            *best = d;
        }
        if *best == 0.0 {
            *upper = 0.0;
            return;
        }
        let candidate = box_distance_upper(&a.bbox, &b.bbox);
        if candidate < *upper {
            *upper = candidate;
        }
        return;
    }
    if a_leaf {
        dual_distance_min(a_nodes, b_nodes, na, b.left as usize, best, upper);
        dual_distance_min(a_nodes, b_nodes, na, b.right as usize, best, upper);
    } else if b_leaf {
        dual_distance_min(a_nodes, b_nodes, a.left as usize, nb, best, upper);
        dual_distance_min(a_nodes, b_nodes, a.right as usize, nb, best, upper);
    } else {
        dual_distance_min(
            a_nodes,
            b_nodes,
            a.left as usize,
            b.left as usize,
            best,
            upper,
        );
        dual_distance_min(
            a_nodes,
            b_nodes,
            a.left as usize,
            b.right as usize,
            best,
            upper,
        );
        dual_distance_min(
            a_nodes,
            b_nodes,
            a.right as usize,
            b.left as usize,
            best,
            upper,
        );
        dual_distance_min(
            a_nodes,
            b_nodes,
            a.right as usize,
            b.right as usize,
            best,
            upper,
        );
    }
}

/// The self-distance walk over the subtree rooted at `node`: within-leaf piece
/// pairs (two or more primitives in one leaf) certify `0`, and distinct
/// subtrees are compared by the dual walk. Mirrors [`Bvh::candidate_pairs_self`].
fn self_distance_min(nodes: &[BvhNode], node: usize, best: &mut f64, upper: &mut f64) {
    if *best == 0.0 {
        return;
    }
    let Some(n) = nodes.get(node) else {
        return;
    };
    if n.left == u32::MAX {
        if n.count >= 2 {
            *best = 0.0;
            *upper = 0.0;
        }
        return;
    }
    self_distance_min(nodes, n.left as usize, best, upper);
    self_distance_min(nodes, n.right as usize, best, upper);
    dual_distance_min(nodes, nodes, n.left as usize, n.right as usize, best, upper);
}

/// Recursively builds the subtree over `primitives[lo..hi)` pre-order (parent
/// pushed before children), permuting `primitives` so each leaf owns a
/// contiguous range. Returns the subtree's root index in `nodes`.
fn build_range(
    nodes: &mut Vec<BvhNode>,
    primitives: &mut Vec<u32>,
    bboxes: &[BoundingBox<Point3>],
    lo: usize,
    hi: usize,
) -> u32 {
    let mut union = BoundingBox::new();
    if let Some(range) = primitives.get(lo..hi) {
        for &idx in range.iter() {
            if let Some(b) = bboxes.get(idx as usize) {
                union += *b;
            }
        }
    }
    let count = hi - lo;
    if count <= LEAF_CAP {
        let node_idx = nodes.len() as u32;
        nodes.push(BvhNode {
            bbox: union,
            left: u32::MAX,
            right: u32::MAX,
            start: lo as u32,
            count: count as u32,
        });
        return node_idx;
    }
    let diagonal = union.diagonal();
    let mut axis = 0;
    if diagonal.y > diagonal.x {
        axis = 1;
    }
    let second_axis = if axis == 0 { diagonal.x } else { diagonal.y };
    if diagonal.z > second_axis {
        axis = 2;
    }
    if let Some(range) = primitives.get_mut(lo..hi) {
        range.sort_by(|&a, &b| {
            let ca = bboxes.get(a as usize).map(|bbox| bbox.center());
            let cb = bboxes.get(b as usize).map(|bbox| bbox.center());
            match (ca, cb) {
                (Some(ca), Some(cb)) => {
                    let va = match axis {
                        0 => ca.x,
                        1 => ca.y,
                        _ => ca.z,
                    };
                    let vb = match axis {
                        0 => cb.x,
                        1 => cb.y,
                        _ => cb.z,
                    };
                    va.partial_cmp(&vb).unwrap_or(Ordering::Equal)
                }
                _ => Ordering::Equal,
            }
        });
    }
    let mid = lo + count / 2;
    let node_idx = nodes.len() as u32;
    nodes.push(BvhNode {
        bbox: union,
        left: u32::MAX,
        right: u32::MAX,
        start: lo as u32,
        count: 0,
    });
    let left = build_range(nodes, primitives, bboxes, lo, mid);
    let right = build_range(nodes, primitives, bboxes, mid, hi);
    if let Some(node) = nodes.get_mut(node_idx as usize) {
        node.left = left;
        node.right = right;
    }
    node_idx
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Piece {
        bbox: BoundingBox<Point3>,
    }

    impl BoundedPiece for Piece {
        fn bbox(&self) -> BoundingBox<Point3> {
            self.bbox
        }
        fn derivative_bounds(&self) -> DerivativeBounds {
            DerivativeBounds::new()
        }
        fn subdivide(&self) -> Vec<Self> {
            Vec::new()
        }
    }

    /// Deterministic LCG so a failure is reproducible.
    fn lcg_next(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state
    }

    /// A dyadic box centered at an LCG point with an LCG half-width.
    fn lcg_box(state: &mut u64) -> BoundingBox<Point3> {
        let x = (lcg_next(state) % 64) as f64;
        let y = (lcg_next(state) % 64) as f64;
        let z = (lcg_next(state) % 64) as f64;
        let half = 1.0 + ((lcg_next(state) % 12) as f64);
        let mut b = BoundingBox::new();
        b.push(Point3::new(x - half, y - half, z - half));
        b.push(Point3::new(x + half, y + half, z + half));
        b
    }

    /// `groups` groups of eight identical pieces. Equal centroids sort
    /// contiguously (stable sort), so the midpoint build never mixes groups;
    /// every leaf is exactly one group and leaf boxes coincide with piece
    /// boxes, making the broad phase exact on this data.
    fn lcg_pieces(state: &mut u64, groups: usize) -> Vec<Piece> {
        let mut pieces = Vec::new();
        for _ in 0..groups {
            let b = lcg_box(state);
            for _ in 0..8 {
                pieces.push(Piece { bbox: b });
            }
        }
        pieces
    }

    fn brute_pairs(a: &[Piece], b: &[Piece]) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (i, pa) in a.iter().enumerate() {
            for (j, pb) in b.iter().enumerate() {
                if pa.bbox().intersects(&pb.bbox()) {
                    out.push((i, j));
                }
            }
        }
        out
    }

    fn brute_self(pieces: &[Piece]) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (i, pa) in pieces.iter().enumerate() {
            for (j, pb) in pieces.iter().enumerate().skip(i + 1) {
                if pa.bbox().intersects(&pb.bbox()) {
                    out.push((i, j));
                }
            }
        }
        out
    }

    #[test]
    fn bvh_candidate_pairs_matches_brute_force() {
        let mut state = 0x9E37_79B9_7F4A_7C15;
        let a = lcg_pieces(&mut state, 4);
        let b = lcg_pieces(&mut state, 4);
        for piece in a.iter().chain(b.iter()) {
            assert_eq!(piece.bbox(), piece.bbox);
        }
        let bvh_a = Bvh::build(&a);
        let bvh_b = Bvh::build(&b);
        let got = bvh_a.candidate_pairs(&bvh_b);
        let want = brute_pairs(&a, &b);
        assert_eq!(got, want);
        for &(i, j) in got.iter() {
            let pa = a.get(i).unwrap();
            let pb = b.get(j).unwrap();
            assert!(pa.bbox().intersects(&pb.bbox()));
        }
    }

    #[test]
    fn bvh_self_pairs_are_ordered_and_complete() {
        let mut state = 0x0123_4567_89AB_CDEF;
        let pieces = lcg_pieces(&mut state, 4);
        let bvh = Bvh::build(&pieces);
        let got = bvh.candidate_pairs_self();
        let want = brute_self(&pieces);
        assert_eq!(got, want);
        for &(i, j) in got.iter() {
            assert!(i < j);
            let pa = pieces.get(i).unwrap();
            let pb = pieces.get(j).unwrap();
            assert!(pa.bbox().intersects(&pb.bbox()));
        }
    }

    #[test]
    fn bvh_query_returns_intersecting_pieces() {
        let mut state = 0xDEAD_BEEF_CAFE_F00D;
        let pieces = lcg_pieces(&mut state, 4);
        let bvh = Bvh::build(&pieces);
        let mut query = BoundingBox::new();
        for piece in pieces.iter().take(10) {
            query += piece.bbox();
        }
        let got = bvh.query(&query);
        let want: Vec<usize> = pieces
            .iter()
            .enumerate()
            .filter(|(_, piece)| piece.bbox().intersects(&query))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn bvh_build_is_deterministic() {
        let mut state = 0x1234_5678_9ABC_DEF0;
        let pieces = lcg_pieces(&mut state, 4);
        let first = Bvh::build(&pieces);
        let second = Bvh::build(&pieces);
        assert_eq!(
            first.candidate_pairs(&second),
            second.candidate_pairs(&first)
        );
        assert_eq!(first.candidate_pairs_self(), second.candidate_pairs_self());
        let mut query = BoundingBox::new();
        for piece in pieces.iter().take(6) {
            query += piece.bbox();
        }
        assert_eq!(first.query(&query), second.query(&query));
    }

    #[test]
    fn empty_bvh_has_no_candidate_pairs() {
        let bvh: Bvh<Piece> = Bvh::build(&[]);
        assert!(bvh.is_empty());
        assert_eq!(bvh.len(), 0);
        assert!(bvh.candidate_pairs(&bvh).is_empty());
        assert!(bvh.candidate_pairs_self().is_empty());
        assert!(bvh.query(&BoundingBox::new()).is_empty());
    }
}
