//! CFP-009-BROADPHASE — the uniform-grid broadphase pair screen of the
//! boolean entry (packet `CFP-009-BROADPHASE`,
//! `docs/CONTACT_FAST_PATH_BUILD_SPEC.md` §4 packet row).
//!
//! `sweep_contact_events` used to test every cross-solid stratum pair with an
//! AABB `touches()` check in a flat O(n·m) double loop. This module replaces
//! the flat loop with a deterministic uniform grid keyed on a cell size
//! derived from the participating strata's MEDIAN box extent. The observable
//! contract is EXACTNESS: the grid returns exactly the pair set the flat loop
//! admits — same pairs, no additions, no omissions — and emits it in the flat
//! loop's canonical `(index_left, index_right)` order. The index is a pure
//! reorganization of the same inclusive `touches()` predicate over the same
//! boxes.
//!
//! Mechanics (deterministic; packet scope decisions 2, 3 and 6):
//! - the cell size is `max(median box extent, union span / 64)` over the
//!   FINITE boxes of both sides of one screen (fixed formula; every constant
//!   carries the H-3 tag on the same line as its literal);
//! - a box registers in EVERY cell it overlaps; a coordinate exactly on a cell
//!   boundary belongs to the cell the `floor((coord − origin) / cell)` rule
//!   assigns it, so two INCLUSIVE-touching boxes always share a cell (their
//!   closed intervals meet at a point lying inside both boxes' cell spans);
//! - candidate pairs are de-duplicated by `(index_left, index_right)` and
//!   sorted into the flat loop's canonical order;
//! - the same inclusive overlap predicate filters the candidates, so a
//!   same-cell-but-separated pair is never admitted;
//! - an empty AABB (the lift's "no usable cover" box) touches nothing and is
//!   never registered or queried — exactly as in the flat loop;
//! - pathological skew (every box in one cell) degrades to the flat loop over
//!   that cell's contents — still exact, never a heuristic.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use rustc_hash::FxHashMap as HashMap;
use truck_base::cgmath64::Point3;

/// The H-3 cell-size bound: the minimum grid resolution across the union
/// box's longest axis. A box never spans more than this many cells along any
/// axis, which bounds registration cost per box and keeps the cell-coordinate
/// arithmetic far inside `i64` range.
const MAX_UNION_AXIS_CELLS: f64 = 64.0; // H-3: cell-count bound across the union's longest axis

/// A lifted stratum whose 3-D screen box the index can key.
pub(super) trait HasBox {
    /// The inclusive lower corner.
    fn lo(&self) -> Point3;
    /// The inclusive upper corner.
    fn hi(&self) -> Point3;
}

/// A deterministic uniform grid over the finite members of one side's boxes.
#[derive(Debug, PartialEq)]
struct UniformGrid {
    /// The per-axis cell origin (the union box's lower corner).
    origin: Point3,
    /// The isotropic cell size (see the module docs for the formula).
    cell: f64,
    /// Every cell -> the left indices registered in it, in ascending index
    /// order (the order they were inserted, which is index order).
    cells: HashMap<(i64, i64, i64), Vec<usize>>,
}

impl UniformGrid {
    /// Builds the grid: every finite box registers in EVERY cell it overlaps.
    /// Boxes whose corners are not finite (the empty screen box) are skipped —
    /// an empty box touches nothing in the flat loop, so it contributes no
    /// candidates here either.
    fn over<B: HasBox>(boxes: &[B], origin: Point3, cell: f64) -> UniformGrid {
        let mut cells: HashMap<(i64, i64, i64), Vec<usize>> = HashMap::default();
        for (idx, b) in boxes.iter().enumerate() {
            let Some((lo, hi)) = finite_box(b) else {
                continue;
            };
            let (x0, y0, z0, x1, y1, z1) = span_to_cells(origin, cell, lo, hi);
            for ix in x0..=x1 {
                for iy in y0..=y1 {
                    for iz in z0..=z1 {
                        cells.entry((ix, iy, iz)).or_default().push(idx);
                    }
                }
            }
        }
        UniformGrid {
            origin,
            cell,
            cells,
        }
    }

    /// Collects every `(left_index, right_index)` pair whose boxes touch,
    /// de-duplicated and sorted into the flat loop's canonical order
    /// (left index ascending, then right index ascending).
    fn query<L: HasBox, R: HasBox>(&self, left: &[L], right: &[R]) -> Vec<(usize, usize)> {
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        for (ri, rb) in right.iter().enumerate() {
            let Some((rlo, rhi)) = finite_box(rb) else {
                continue;
            };
            let (x0, y0, z0, x1, y1, z1) = span_to_cells(self.origin, self.cell, rlo, rhi);
            for ix in x0..=x1 {
                for iy in y0..=y1 {
                    for iz in z0..=z1 {
                        let Some(entries) = self.cells.get(&(ix, iy, iz)) else {
                            continue;
                        };
                        for &li in entries {
                            let Some(lb) = left.get(li) else {
                                continue;
                            };
                            let Some((llo, lhi)) = finite_box(lb) else {
                                continue;
                            };
                            if boxes_touch(llo, lhi, rlo, rhi) {
                                pairs.push((li, ri));
                            }
                        }
                    }
                }
            }
        }
        pairs.sort_unstable();
        pairs.dedup();
        pairs
    }
}

/// The screened pair list of one cross-solid combination: every `(i, j)` in
/// `left[i].box.touches(right[j].box)`, in the flat loop's canonical order.
///
/// The index is EXACT: two inclusive-touching boxes always share a cell, so no
/// touching pair is missed, and the same inclusive overlap predicate filters
/// the candidates, so no separated pair is admitted.
pub(super) fn candidate_touching_pairs<L: HasBox, R: HasBox>(
    left: &[L],
    right: &[R],
) -> Vec<(usize, usize)> {
    if left.is_empty() || right.is_empty() {
        return Vec::new();
    }
    let Some((origin, cell)) = grid_params(left, right) else {
        return Vec::new();
    };
    let grid = UniformGrid::over(left, origin, cell);
    grid.query(left, right)
}

/// The deterministic grid parameters for one screen: the union box's lower
/// corner as origin and the cell size from the fixed median formula. `None`
/// when no finite box participates (nothing can touch).
fn grid_params<L: HasBox, R: HasBox>(left: &[L], right: &[R]) -> Option<(Point3, f64)> {
    let mut lo = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut hi = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut extents: Vec<f64> = Vec::new();
    let mut count = 0usize;
    accumulate(left, &mut lo, &mut hi, &mut extents, &mut count);
    accumulate(right, &mut lo, &mut hi, &mut extents, &mut count);
    if count == 0 {
        return None;
    }
    extents.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = extents.get(extents.len() / 2).copied().unwrap_or(0.0);
    let span = largest_extent(lo, hi);
    let min_cell = span / MAX_UNION_AXIS_CELLS;
    let mut cell = median.max(min_cell);
    if !cell.is_finite() || cell <= 0.0 {
        // The median formula collapsed (only zero-extent/coincident boxes):
        // keep a unit cell so coincident points still share one cell.
        cell = if span.is_finite() && span > 0.0 {
            min_cell
        } else {
            1.0 // H-3: unit fallback cell for an all-degenerate box set
        };
    }
    Some((lo, cell))
}

/// Accumulates the union bounds and the per-box largest extents of the finite
/// members of one side (a non-finite box contributes nothing to either).
fn accumulate<B: HasBox>(
    boxes: &[B],
    lo: &mut Point3,
    hi: &mut Point3,
    extents: &mut Vec<f64>,
    count: &mut usize,
) {
    for b in boxes {
        let Some((blo, bhi)) = finite_box(b) else {
            continue;
        };
        lo.x = lo.x.min(blo.x);
        lo.y = lo.y.min(blo.y);
        lo.z = lo.z.min(blo.z);
        hi.x = hi.x.max(bhi.x);
        hi.y = hi.y.max(bhi.y);
        hi.z = hi.z.max(bhi.z);
        extents.push(largest_extent(blo, bhi));
        *count += 1;
    }
}

/// The box as finite coordinate corners; `None` for a box with a non-finite
/// corner (the lift's empty screen box, which touches nothing).
fn finite_box<B: HasBox>(b: &B) -> Option<(Point3, Point3)> {
    let lo = b.lo();
    let hi = b.hi();
    let finite = lo.x.is_finite()
        && lo.y.is_finite()
        && lo.z.is_finite()
        && hi.x.is_finite()
        && hi.y.is_finite()
        && hi.z.is_finite();
    if finite {
        Some((lo, hi))
    } else {
        None
    }
}

/// The box's largest axis extent.
fn largest_extent(lo: Point3, hi: Point3) -> f64 {
    let dx = hi.x - lo.x;
    let dy = hi.y - lo.y;
    let dz = hi.z - lo.z;
    dx.max(dy).max(dz)
}

/// The inclusive cell spans `(x0, y0, z0, x1, y1, z1)` a box overlaps: every
/// cell from `floor((lo − origin) / cell)` to `floor((hi − origin) / cell)`
/// inclusive on each axis. A boundary coordinate belongs to the cell the
/// `floor` rule assigns it, so inclusive-touching boxes share a cell.
fn span_to_cells(
    origin: Point3,
    cell: f64,
    lo: Point3,
    hi: Point3,
) -> (i64, i64, i64, i64, i64, i64) {
    let x0 = cell_of(cell, lo.x - origin.x);
    let y0 = cell_of(cell, lo.y - origin.y);
    let z0 = cell_of(cell, lo.z - origin.z);
    let x1 = cell_of(cell, hi.x - origin.x);
    let y1 = cell_of(cell, hi.y - origin.y);
    let z1 = cell_of(cell, hi.z - origin.z);
    (x0, y0, z0, x1, y1, z1)
}

/// The cell coordinate of one offset from the origin.
fn cell_of(cell: f64, offset: f64) -> i64 {
    (offset / cell).floor() as i64
}

/// The INCLUSIVE overlap predicate — identical to the flat loop's `touches()`
/// (boundary touch counts on all three axes).
fn boxes_touch(alo: Point3, ahi: Point3, blo: Point3, bhi: Point3) -> bool {
    alo.x <= bhi.x
        && blo.x <= ahi.x
        && alo.y <= bhi.y
        && blo.y <= ahi.y
        && alo.z <= bhi.z
        && blo.z <= ahi.z
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. The index's own deterministic tests operate on
// hand-built boxes, and the unwraps/indexing below cannot fire for them.
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    /// A raw indexable box with `Point3` corners (the index's test probe).
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct ProbeBox {
        lo: Point3,
        hi: Point3,
    }

    impl HasBox for ProbeBox {
        fn lo(&self) -> Point3 {
            self.lo
        }
        fn hi(&self) -> Point3 {
            self.hi
        }
    }

    /// A probe box from corner arrays.
    fn probe(lo: [f64; 3], hi: [f64; 3]) -> ProbeBox {
        ProbeBox {
            lo: Point3::new(lo[0], lo[1], lo[2]),
            hi: Point3::new(hi[0], hi[1], hi[2]),
        }
    }

    /// The brute-force flat pair list over the same predicate (the oracle the
    /// index must reproduce).
    fn flat_candidates<L: HasBox, R: HasBox>(left: &[L], right: &[R]) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (i, l) in left.iter().enumerate() {
            let (llo, lhi) = finite_box(l).expect("the probe is finite");
            for (j, r) in right.iter().enumerate() {
                let (rlo, rhi) = finite_box(r).expect("the probe is finite");
                if boxes_touch(llo, lhi, rlo, rhi) {
                    out.push((i, j));
                }
            }
        }
        out
    }

    /// Required test 2: two builds on identical input produce identical
    /// structures (cell maps and grid parameters) and identical pair
    /// sequences, and the index reproduces the brute-force flat pair set.
    #[test]
    fn index_build_deterministic() {
        let left = vec![
            probe([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
            probe([0.9, 0.9, 0.9], [1.1, 1.1, 1.1]),
            probe([5.0, 5.0, 5.0], [6.0, 6.0, 6.0]),
            probe([3.0, 3.0, 3.0], [3.0, 3.0, 3.0]),
        ];
        let right = vec![
            probe([1.0, 1.0, 1.0], [2.0, 2.0, 2.0]),
            probe([0.2, 0.2, 0.2], [0.3, 0.3, 0.3]),
            probe([6.0, 6.0, 6.0], [6.0, 6.0, 6.0]),
            probe([3.0, 3.0, 3.0], [3.0, 3.0, 3.0]),
            probe([10.0, 10.0, 10.0], [11.0, 11.0, 11.0]),
        ];

        // Identical input -> identical grid parameters and cell maps.
        let (origin, cell) = grid_params(&left, &right).expect("finite probes yield params");
        let (origin2, cell2) = grid_params(&left, &right).expect("finite probes yield params");
        assert_eq!((origin, cell), (origin2, cell2));
        let grid_a = UniformGrid::over(&left, origin, cell);
        let grid_b = UniformGrid::over(&left, origin, cell);
        assert_eq!(
            grid_a, grid_b,
            "identical builds produce identical cell maps"
        );

        // Identical structures -> identical pair sequences.
        assert_eq!(
            grid_a.query(&left, &right),
            grid_b.query(&left, &right),
            "identical grids produce identical pair sequences"
        );

        // The whole candidate computation is a pure function of its input.
        let first = candidate_touching_pairs(&left, &right);
        let second = candidate_touching_pairs(&left, &right);
        assert_eq!(
            first, second,
            "identical input reproduces the pair sequence"
        );

        // Exactness against the brute-force flat oracle: touching corners,
        // touching edges, coincident point boxes, and same-cell-but-separated
        // boxes (the far box shares no cell; the overlap predicate drops any
        // co-cell separated pair).
        assert_eq!(first, flat_candidates(&left, &right));
        assert_eq!(
            first,
            vec![(0, 0), (0, 1), (1, 0), (2, 2), (3, 3)],
            "the exact pair list: boundary/point/corner touches only"
        );
    }
}
