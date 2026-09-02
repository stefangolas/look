#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//! The SSI wave shim, part 2: the synthetic fixture kit (BG-CK-P2-CONTRACT).
//!
//! **This module is `#[doc(hidden)] pub`: TEST SUPPORT ONLY, explicitly
//! excluded from the certified API surface (a one-line mapping-table note, not
//! a row: no new evidence kind).** Wave workers' integration tests consume it
//! through the crate's public path; `#[cfg(test)]`-only items would be
//! invisible to them.
//!
//! The kit realizes mathematically valid states with known ground truth. Each
//! fixture carries a doc-stated ground truth that the contract tests verify by
//! direct evaluation — never by solving. Construction only; no solver, no
//! certified enclosure work: the point-evaluation and coefficient-derivative
//! helpers below are plain `f64` direct evaluation, used to check the stated
//! ground truths; it is not hull kernel work and not certified-interval
//! algebra.
//!
//! # Fixture geometry model
//!
//! Every fixture is built from two rational Bézier patches with the constant
//! positive weight certificate `w = 1` (the homogeneous numerator equals the
//! coordinate polynomial). The stored square system is the cross-multiplied
//! grid `F_k = W2*P1_k − W1*P2_k` (D-homogeneous), which for unit weights is
//! the coefficient-wise difference of the two patches' component grids. The
//! surfaces are graphs `(x, y) = (u, v)` over the shared horizontal chart with
//! `z = h(u, v)` (patch 1 over `(u, v)`, patch 2 over `(s, t)`); the zero set
//! of `F` is then the *diagonal lift* `{s = u, t = v, h1(u,v) = h2(u,v)}` of
//! the plane curve `h1 − h2 = 0`. Chart rectangles are the unit square for
//! both patches, so every domain map is `(0,1,0,1,0,1,0,1)`.
//!
//! The square-system reduction convention the kit documents (and its tests
//! machine-check) is the identity pairing: for a candidate continuation axis
//! `j`, the reduced 3×3 square system's unknowns are the other three chart axes
//! in ascending order and its equations are the `F` components in order, so the
//! coordinate-`i` diagonal derivative `∂H_i/∂t_i` is the partial of `F_i` along
//! the `i`-th smallest retained axis. Germ-class facts are stated as exact jets
//! of the reduced branch profile; classification (reading those jets into a
//! [`BranchGerm`]) is the consumer's job.

use crate::contract::{ContinuationCoordinate, Refusal};
use crate::formal::contact::BranchIncidence;
use crate::formal::curve2d::{
    CurveOccurrenceProvenance, SourceEdgeId, SourceEntityId, SourceFaceId,
};
use crate::formal::intersection::{ParameterEnclosure, ParameterLocation};
use crate::formal::quotient::{CanonicalBranchSide, CertifiedDeckLabel, DeckContext};
use crate::formal::span::{BranchGerm, SpanId};
use crate::source_evidence::{BoundId, EdgeUseId, SourceVertexKey};
use crate::ssi_types::{SquareSystem3, TraceStep};
use truck_geometry::prelude::Point2;

/// The identity chart rectangle used by every fixture pair.
const IDENTITY_DOMAIN_MAPS: (f64, f64, f64, f64, f64, f64, f64, f64) =
    (0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0);

/// A rational Bézier patch: bidegree `(m, n)` over its own `[0, 1]^2` with the
/// constant unit weight certificate. `coords[k]` is the `(m+1) x (n+1)`
/// coefficient grid of the homogeneous numerator of component `k` (rows index
/// the first parameter, columns the second).
struct Patch {
    /// Bidegree in the first parameter.
    m: usize,
    /// Bidegree in the second parameter.
    n: usize,
    /// Component coefficient grids, in `(x, y, z)` order.
    coords: [Vec<Vec<f64>>; 3],
}

impl Patch {
    /// Assemble a patch from precomputed component grids.
    fn new(m: usize, n: usize, coords: [Vec<Vec<f64>>; 3]) -> Self {
        Self { m, n, coords }
    }

    /// The graph patch `(x, y, z) = (u, v, h(u, v))` over the identity chart.
    fn graph(m: usize, n: usize, h: Vec<Vec<f64>>) -> Self {
        Self::new(
            m,
            n,
            [coordinate_grid(m, n, 0), coordinate_grid(m, n, 1), h],
        )
    }

    /// A general bilinear patch assembled from explicit component grids.
    fn bilinear(x: Vec<Vec<f64>>, y: Vec<Vec<f64>>, z: Vec<Vec<f64>>) -> Self {
        Self::new(1, 1, [x, y, z])
    }
}

/// The `x`-coordinate grid `u` of the identity chart at bidegree `(m, n)`.
fn coordinate_grid(m: usize, n: usize, which: usize) -> Vec<Vec<f64>> {
    let mut grid = Vec::with_capacity(m + 1);
    for a in 0..=m {
        let mut row = Vec::with_capacity(n + 1);
        for b in 0..=n {
            let v = if which == 0 {
                a as f64 / m as f64
            } else {
                b as f64 / n as f64
            };
            row.push(v);
        }
        grid.push(row);
    }
    grid
}

/// A constant grid.
fn constant_grid(m: usize, n: usize, value: f64) -> Vec<Vec<f64>> {
    vec![vec![value; n + 1]; m + 1]
}

/// Add `coeff * u^pu * v^pv` (in the monomial basis) onto a Bernstein grid of
/// bidegree `(m, n)`. The conversion is exact in `f64` for the small integer
/// degrees this kit uses.
fn add_monomial(grid: &mut [Vec<f64>], m: usize, n: usize, pu: usize, pv: usize, coeff: f64) {
    let row_factors: Vec<f64> = (pu..=m).map(|a| binom(a, pu) / binom(m, pu)).collect();
    let col_factors: Vec<f64> = (pv..=n).map(|b| binom(b, pv) / binom(n, pv)).collect();
    for (a, fa) in (pu..=m).zip(row_factors.iter()) {
        for (b, fb) in (pv..=n).zip(col_factors.iter()) {
            grid[a][b] += coeff * fa * fb;
        }
    }
}

/// A zero Bernstein grid of bidegree `(m, n)`.
fn zero_grid(m: usize, n: usize) -> Vec<Vec<f64>> {
    vec![vec![0.0; n + 1]; m + 1]
}

/// Build a Bernstein grid of bidegree `(m, n)` from monomial terms
/// `(pu, pv, coeff)`.
fn monomial_grid(m: usize, n: usize, terms: &[(usize, usize, f64)]) -> Vec<Vec<f64>> {
    let mut grid = zero_grid(m, n);
    for &(pu, pv, coeff) in terms {
        add_monomial(&mut grid, m, n, pu, pv, coeff);
    }
    grid
}

/// Small exact binomial coefficient.
fn binom(n: usize, k: usize) -> f64 {
    let mut numerator = 1u64;
    let mut denominator = 1u64;
    for i in 0..k {
        numerator *= (n - i) as u64;
        denominator *= (i + 1) as u64;
    }
    numerator as f64 / denominator as f64
}

/// Build the cross-multiplied `SquareSystem3` for a unit-weight patch pair.
///
/// The stored grids are `F_k[a][b][i][j] = P1_k[a][b] − P2_k[i][j]` in the flat
/// layout rows `a*(n1+1)+b`, columns `i*(n2+1)+j` (D-homogeneous with the two
/// unit weight certificates).
fn cross_system(a: &Patch, b: &Patch) -> Result<SquareSystem3, Refusal> {
    let (ma, na) = (a.m, a.n);
    let (mb, nb) = (b.m, b.n);
    let rows = (ma + 1) * (na + 1);
    let cols = (mb + 1) * (nb + 1);
    let mut grids = [
        vec![vec![0.0; cols]; rows],
        vec![vec![0.0; cols]; rows],
        vec![vec![0.0; cols]; rows],
    ];
    for (k, grid_k) in grids.iter_mut().enumerate() {
        for a_i in 0..=ma {
            for b_i in 0..=na {
                let row = a_i * (na + 1) + b_i;
                for i in 0..=mb {
                    for j in 0..=nb {
                        grid_k[row][i * (nb + 1) + j] = a.coords[k][a_i][b_i] - b.coords[k][i][j];
                    }
                }
            }
        }
    }
    SquareSystem3::new(grids, (ma, na, mb, nb), IDENTITY_DOMAIN_MAPS)
}

// ---------------------------------------------------------------------------
// Direct-evaluation test support (plain f64; not certified, not a solver)
// ---------------------------------------------------------------------------

/// 1-D de Casteljau evaluation of a Bernstein coefficient list at `x`.
fn bernstein_eval(coeffs: &[f64], x: f64) -> f64 {
    let mut level = coeffs.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() - 1);
        for pair in level.windows(2) {
            next.push(pair[0] + x * (pair[1] - pair[0]));
        }
        level = next;
    }
    level[0]
}

/// Evaluate a bivariate Bernstein polynomial laid out flat as `(m+1)` rows of
/// `(n+1)` coefficients (rows index the first parameter).
fn bernstein_eval_flat(flat: &[f64], m: usize, n: usize, p: f64, q: f64) -> f64 {
    let mut first = Vec::with_capacity(m + 1);
    for i in 0..=m {
        let row = &flat[i * (n + 1)..(i + 1) * (n + 1)];
        first.push(bernstein_eval(row, q));
    }
    bernstein_eval(&first, p)
}

/// Whether a stored grid has the shape the degrees demand.
fn grid_shape_matches(grid: &[Vec<f64>], degrees: (usize, usize, usize, usize)) -> bool {
    let (m1, n1, m2, n2) = degrees;
    let rows = (m1 + 1) * (n1 + 1);
    let cols = (m2 + 1) * (n2 + 1);
    grid.len() == rows && grid.iter().all(|row| row.len() == cols)
}

/// Directly evaluate one stored `SquareSystem3` component grid at a chart
/// point `(u, v, s, t)`. `None` when the grid does not match `degrees`.
///
/// Plain `f64` direct evaluation only — not a certified enclosure and not a
/// solver call.
pub fn eval_grid4(
    grid: &[Vec<f64>],
    degrees: (usize, usize, usize, usize),
    uvst: (f64, f64, f64, f64),
) -> Option<f64> {
    if !grid_shape_matches(grid, degrees) {
        return None;
    }
    let (m1, n1, m2, n2) = degrees;
    let rows = (m1 + 1) * (n1 + 1);
    let (u, v, s, t) = uvst;
    let mut inner = Vec::with_capacity(rows);
    for row in grid.iter().take(rows) {
        inner.push(bernstein_eval_flat(row, m2, n2, s, t));
    }
    Some(bernstein_eval_flat(&inner, m1, n1, u, v))
}

/// Directly evaluate all three stored components of a square system at a chart
/// point. `None` when any grid is malformed.
pub fn eval_system(system: &SquareSystem3, uvst: (f64, f64, f64, f64)) -> Option<[f64; 3]> {
    let degrees = system.degrees();
    let mut out = [0.0; 3];
    for (k, grid) in system.grids().iter().enumerate() {
        out[k] = eval_grid4(grid, degrees, uvst)?;
    }
    Some(out)
}

/// A coefficient grid after one axis-wise Bernstein differentiation.
struct DerivedGrid {
    /// The derived coefficients, in the flat layout.
    grid: Vec<Vec<f64>>,
    /// The derived degrees.
    degrees: (usize, usize, usize, usize),
}

/// First-derivative coefficient grids along one chart axis of a stored grid,
/// in the flat `rows x cols` layout.
fn differentiate_axis(
    grid: &[Vec<f64>],
    degrees: (usize, usize, usize, usize),
    axis: usize,
) -> Option<DerivedGrid> {
    if !grid_shape_matches(grid, degrees) {
        return None;
    }
    let (m1, n1, m2, n2) = degrees;
    let cols = (m2 + 1) * (n2 + 1);
    match axis {
        0 => {
            if m1 == 0 {
                return None;
            }
            let nm1 = m1 - 1;
            let nrows = (nm1 + 1) * (n1 + 1);
            let mut out = vec![vec![0.0; cols]; nrows];
            for a in 0..=nm1 {
                for b in 0..=n1 {
                    let dst = a * (n1 + 1) + b;
                    let lo = a * (n1 + 1) + b;
                    let hi = (a + 1) * (n1 + 1) + b;
                    for c in 0..cols {
                        out[dst][c] = (m1 as f64) * (grid[hi][c] - grid[lo][c]);
                    }
                }
            }
            Some(DerivedGrid {
                grid: out,
                degrees: (nm1, n1, m2, n2),
            })
        }
        1 => {
            if n1 == 0 {
                return None;
            }
            let nn1 = n1 - 1;
            let nrows = (m1 + 1) * (nn1 + 1);
            let mut out = vec![vec![0.0; cols]; nrows];
            for a in 0..=m1 {
                for b in 0..=nn1 {
                    let dst = a * (nn1 + 1) + b;
                    let lo = a * (n1 + 1) + b;
                    let hi = a * (n1 + 1) + (b + 1);
                    for c in 0..cols {
                        out[dst][c] = (n1 as f64) * (grid[hi][c] - grid[lo][c]);
                    }
                }
            }
            Some(DerivedGrid {
                grid: out,
                degrees: (m1, nn1, m2, n2),
            })
        }
        2 => {
            if m2 == 0 {
                return None;
            }
            let nm2 = m2 - 1;
            let ncols = (nm2 + 1) * (n2 + 1);
            let rows = grid.len();
            let mut out = vec![vec![0.0; ncols]; rows];
            for r in 0..rows {
                for i in 0..=nm2 {
                    for j in 0..=n2 {
                        let dst = i * (n2 + 1) + j;
                        let lo = i * (n2 + 1) + j;
                        let hi = (i + 1) * (n2 + 1) + j;
                        out[r][dst] = (m2 as f64) * (grid[r][hi] - grid[r][lo]);
                    }
                }
            }
            Some(DerivedGrid {
                grid: out,
                degrees: (m1, n1, nm2, n2),
            })
        }
        _ => {
            if n2 == 0 {
                return None;
            }
            let nn2 = n2 - 1;
            let ncols = (m2 + 1) * (nn2 + 1);
            let rows = grid.len();
            let mut out = vec![vec![0.0; ncols]; rows];
            for r in 0..rows {
                for i in 0..=m2 {
                    for j in 0..=nn2 {
                        let dst = i * (nn2 + 1) + j;
                        let lo = i * (n2 + 1) + j;
                        let hi = i * (n2 + 1) + (j + 1);
                        out[r][dst] = (n2 as f64) * (grid[r][hi] - grid[r][lo]);
                    }
                }
            }
            Some(DerivedGrid {
                grid: out,
                degrees: (m1, n1, m2, nn2),
            })
        }
    }
}

/// Directly evaluate the first partial of a stored grid along a chart axis at a
/// point. `None` on a malformed grid or a degree-0 axis.
pub fn partial_grid4_axis(
    grid: &[Vec<f64>],
    degrees: (usize, usize, usize, usize),
    axis: usize,
    uvst: (f64, f64, f64, f64),
) -> Option<f64> {
    if axis > 3 {
        return None;
    }
    let derived = differentiate_axis(grid, degrees, axis)?;
    eval_grid4(&derived.grid, derived.degrees, uvst)
}

/// Directly evaluate the second partial of a stored grid along a chart axis at
/// a point. `None` on a malformed grid or an axis that cannot be
/// differentiated twice.
pub fn second_partial_grid4_axis(
    grid: &[Vec<f64>],
    degrees: (usize, usize, usize, usize),
    axis: usize,
    uvst: (f64, f64, f64, f64),
) -> Option<f64> {
    if axis > 3 {
        return None;
    }
    let first = differentiate_axis(grid, degrees, axis)?;
    let second = differentiate_axis(&first.grid, first.degrees, axis)?;
    eval_grid4(&second.grid, second.degrees, uvst)
}

/// The determinant of a 3x3 matrix.
fn det3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// The reduced square-system Jacobian determinant under the kit's documented
/// continuation convention (identity pairing, retained axes ascending), for a
/// candidate continuation axis `j in 0..=3`. `None` on a malformed system.
pub fn reduced_square_determinant(
    system: &SquareSystem3,
    continuation_axis: usize,
    uvst: (f64, f64, f64, f64),
) -> Option<f64> {
    if continuation_axis > 3 {
        return None;
    }
    let degrees = system.degrees();
    let grids = system.grids();
    let mut matrix = [[0.0; 3]; 3];
    let mut column = 0usize;
    for axis in 0..4 {
        if axis == continuation_axis {
            continue;
        }
        for row in 0..3 {
            matrix[row][column] = partial_grid4_axis(&grids[row], degrees, axis, uvst)?;
        }
        column += 1;
    }
    Some(det3(matrix))
}

/// The diagonal entries of the reduced square-system Jacobian under the kit's
/// documented continuation convention: entry `i` is the partial of component
/// `i` along the `i`-th smallest retained axis. `None` on a malformed system.
pub fn reduced_diagonal_entries(
    system: &SquareSystem3,
    continuation_axis: usize,
    uvst: (f64, f64, f64, f64),
) -> Option<[f64; 3]> {
    if continuation_axis > 3 {
        return None;
    }
    let degrees = system.degrees();
    let grids = system.grids();
    let retained: Vec<usize> = (0..4).filter(|a| *a != continuation_axis).collect();
    let mut diagonal = [0.0; 3];
    for (i, &axis) in retained.iter().enumerate() {
        diagonal[i] = partial_grid4_axis(&grids[i], degrees, axis, uvst)?;
    }
    Some(diagonal)
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The ground truth carrier of the two plane-pair root fixtures.
#[derive(Debug)]
pub struct WellConditionedRoot {
    /// The cross-multiplied system of the patch pair.
    pub system: SquareSystem3,
    /// The root quadruple on the branch, interior to the unit 4D chart.
    pub root: (f64, f64, f64, f64),
    /// The documented continuation chart axis (`2`, the `s` axis).
    pub continuation_axis: usize,
    /// The documented reduced determinant at the root (`+1` well conditioned,
    /// `-1` for the parameter-flipped orientation).
    pub reduced_determinant: f64,
}

/// A fixture whose cross-multiplied system's zero set is the single transverse
/// branch `{s = u, t = v, v = 1/4 + s/2}` through the interior root
/// `(1/2, 1/2, 1/2, 1/2)`.
///
/// Built from two small rational Bézier planes (unit weights): patch 1 is
/// `(u, v, v)` (the plane `z = y`), patch 2 is `(s, t, 1/4 + s/2)` (the plane
/// `z = 1/4 + x/2`). The surfaces cross transversely along the line; the
/// reduced square system at the continuation slice `s = 1/2` (unknowns
/// `(u, v, t)` ascending, identity pairing) has exactly one root in the
/// interior of the chart, transverse with determinant `+1`. Ground truth:
/// `F(root) = 0`; crossing each reduced unknown through the root flips the
/// corresponding `F` component's sign; `reduced_determinant = +1`.
pub fn well_conditioned_root() -> Result<WellConditionedRoot, Refusal> {
    let a = Patch::graph(1, 1, coordinate_grid(1, 1, 1));
    let b = Patch::bilinear(
        coordinate_grid(1, 1, 0),
        coordinate_grid(1, 1, 1),
        monomial_grid(1, 1, &[(0, 0, 0.25), (1, 0, 0.5)]),
    );
    let system = cross_system(&a, &b)?;
    Ok(WellConditionedRoot {
        system,
        root: (0.5, 0.5, 0.5, 0.5),
        continuation_axis: 2,
        reduced_determinant: 1.0,
    })
}

/// The same pair with patch 1's parameter order flipped: `(u, v, v)` becomes
/// `(v, u, u)` (the same plane `z = y`, re-oriented). The geometric branch is
/// unchanged but the reduced determinant sign flips — the orientation
/// certificate's other branch. Ground truth: the same root quadruple, with
/// `reduced_determinant = -1` under the documented continuation convention.
pub fn negative_orientation_root() -> Result<WellConditionedRoot, Refusal> {
    let a = Patch::new(
        1,
        1,
        [
            coordinate_grid(1, 1, 1),
            coordinate_grid(1, 1, 0),
            coordinate_grid(1, 1, 0),
        ],
    );
    let b = Patch::bilinear(
        coordinate_grid(1, 1, 0),
        coordinate_grid(1, 1, 1),
        monomial_grid(1, 1, &[(0, 0, 0.25), (1, 0, 0.5)]),
    );
    let system = cross_system(&a, &b)?;
    Ok(WellConditionedRoot {
        system,
        root: (0.5, 0.5, 0.5, 0.5),
        continuation_axis: 2,
        reduced_determinant: -1.0,
    })
}

/// The carrier of the determinant-spans-zero fixture.
#[derive(Debug)]
pub struct DeterminantSpanningZero {
    /// The cross-multiplied system of the pair.
    pub system: SquareSystem3,
    /// The documented fixture box in the 4D chart.
    pub box_: [(f64, f64); 4],
    /// An interior diagonal witness point where every reduced determinant is
    /// exactly zero.
    pub witness: (f64, f64, f64, f64),
}

/// A system whose Jacobian determinant enclosure contains zero over the
/// fixture box, so no Krawczyk certificate may be emitted for it.
///
/// Both patches are the same rational Bézier graph `(x, y, z) = (u, v, u^2)`
/// (the parabolic cylinder `z = x^2`, unit weight), so the surfaces coincide:
/// the zero set of `F = (u - s, v - t, u^2 - s^2)` is the 2D diagonal, and the
/// Jacobian has rank 2 there — every reduced 3x3 determinant is exactly zero at
/// every diagonal point. Ground truth: `F(witness) = 0`; every
/// `reduced_square_determinant` at the witness (any continuation axis) is `0`;
/// away from the diagonal the determinant is nonzero (a genuinely non-flat
/// system), so a sound enclosure over a box straddling the diagonal contains
/// zero and `KrawczykCertificate3::new` must refuse.
pub fn determinant_spans_zero() -> Result<DeterminantSpanningZero, Refusal> {
    let z = monomial_grid(2, 1, &[(2, 0, 1.0)]);
    let patch = Patch::graph(2, 1, z);
    let system = cross_system(&patch, &patch)?;
    Ok(DeterminantSpanningZero {
        system,
        box_: [(0.3, 0.7), (0.3, 0.7), (0.3, 0.7), (0.3, 0.7)],
        witness: (0.5, 0.5, 0.5, 0.5),
    })
}

/// The carrier of the conditioning-below-threshold fixture.
#[derive(Debug)]
pub struct ConditioningBelowThreshold {
    /// The cross-multiplied system of the pair.
    pub system: SquareSystem3,
    /// The documented fixture box in the 4D chart.
    pub box_: [(f64, f64); 4],
    /// An interior root on the branch (`(1/2, 1/2, 1/2, 1/2)`).
    pub root: (f64, f64, f64, f64),
}

/// A system whose every coordinate margin fails the frozen relative-margin
/// rule at the fixture box: the trace must refuse `ConditioningBelowThreshold`
/// here even though a genuine branch (the transverse intersection line of two
/// planes) passes through the box.
///
/// The patches are two planes, patch 1 `(0, u, u+v)` (`x = 0`) and patch 2
/// `(s+t-1, t, 1)` (`z = 1`); `F = (1 - s - t, u - t, u + v - 1)`. Each of the
/// four candidate continuation axes produces a reduced square system whose
/// identity-paired diagonal derivatives are identically zero over the box
/// (`F_x` is `u`- and `v`-independent, `F_y` is `v`- and `s`-independent, `F_z`
/// is `s`- and `t`-independent), so no coordinate can certify away-from-zero
/// and the frozen rule refuses. Ground truth: `F(root) = 0` and every
/// `reduced_diagonal_entries` entry is `0` at interior box points for every
/// continuation axis.
pub fn conditioning_below_threshold() -> Result<ConditioningBelowThreshold, Refusal> {
    let a = Patch::bilinear(
        constant_grid(1, 1, 0.0),
        coordinate_grid(1, 1, 0),
        monomial_grid(1, 1, &[(1, 0, 1.0), (0, 1, 1.0)]),
    );
    let b = Patch::bilinear(
        monomial_grid(1, 1, &[(1, 0, 1.0), (0, 1, 1.0), (0, 0, -1.0)]),
        coordinate_grid(1, 1, 1),
        constant_grid(1, 1, 1.0),
    );
    let system = cross_system(&a, &b)?;
    Ok(ConditioningBelowThreshold {
        system,
        box_: [(0.0, 1.0), (0.0, 1.0), (0.0, 1.0), (0.0, 1.0)],
        root: (0.5, 0.5, 0.5, 0.5),
    })
}

/// One branch-germ fixture: a system whose diagonal branch realizes the
/// documented germ class by construction over the documented box.
#[derive(Debug)]
pub struct GermFixture {
    /// The cross-multiplied system of the graph pair.
    pub system: SquareSystem3,
    /// The documented trace box in the 4D chart.
    pub chart_box: [(f64, f64); 4],
    /// The documented event quadruple (on the branch).
    pub event: (f64, f64, f64, f64),
    /// The documented germ class the branch realizes at the event.
    pub germ: BranchGerm,
}

impl GermFixture {
    /// Whether the documented event lies strictly inside the documented chart
    /// box on every axis.
    pub fn event_is_interior(&self) -> bool {
        let coords = [self.event.0, self.event.1, self.event.2, self.event.3];
        coords
            .iter()
            .zip(self.chart_box.iter())
            .all(|(coord, (lo, hi))| lo < coord && coord < hi)
    }
}

/// The germ ladder: one fixture per `BranchGerm` variant.
///
/// Every entry is a graph patch `(u, v, h(u, v))` against the plane `z = 0`
/// (unit weights), so the branch is the diagonal lift of the plane curve
/// `h = 0`. The classes are guaranteed by construction through the exact
/// profile jets stated below; reading those jets into a `BranchGerm` is the
/// consumer's (TRACE's) job. Machine-checked facts per fixture:
///
/// - `Regular`: `h = v - 0.5u - 0.15`; the branch through the interior event
///   `(1/2, 2/5)` has nonzero slope (`∂h/∂u = -1/2 ≠ 0`).
/// - `StationaryRegular{2}`: `h = v - 0.4 - (u - 1/2)^2`; at the interior event
///   `(1/2, 2/5)` the branch's ordinate has a second-order stationary point
///   (`∂h/∂u = 0`, `∂h/∂v ≠ 0`, `∂²h/∂u² ≠ 0`).
/// - `CuspCandidate`: `h = (v - 1/2)^2 - (u - 1/4)^3`; the cuspidal SSI curve
///   at the interior event `(1/4, 1/2)` has `∇h = 0` and two half-branches
///   meeting with a collapsed tangent.
/// - `Singular`: both patches are the plane `z = 0` (coincident surfaces); the
///   zero set is the 2D diagonal, not a 1D branch — a collapsed stratum whose
///   local topology is not that of a regular branch.
/// - `Unresolved`: a regular line branch whose event lies exactly on the
///   documented box's lower-`u` face; classifying the germ at a box-boundary
///   event requires an endpoint certificate the declared policy does not
///   implement, so the classification is `Unresolved`.
pub fn germ_ladder() -> Result<Vec<GermFixture>, Refusal> {
    let plane = |h: Vec<Vec<f64>>| Patch::graph(1, 1, h);
    let zero_plane = Patch::bilinear(
        coordinate_grid(1, 1, 0),
        coordinate_grid(1, 1, 1),
        constant_grid(1, 1, 0.0),
    );
    let interior_box =
        |u0: f64, u1: f64, v0: f64, v1: f64| [(u0, u1), (v0, v1), (u0, u1), (v0, v1)];
    let build = |a: Patch| -> Result<GermFixture, Refusal> {
        let system = cross_system(&a, &zero_plane)?;
        Ok(GermFixture {
            system,
            chart_box: interior_box(0.0, 1.0, 0.0, 1.0),
            event: (0.0, 0.0, 0.0, 0.0),
            germ: BranchGerm::Regular,
        })
    };
    let mut ladder = Vec::with_capacity(5);

    let regular = build(plane(monomial_grid(
        1,
        1,
        &[(0, 1, 1.0), (1, 0, -0.5), (0, 0, -0.15)],
    )))?;
    ladder.push(GermFixture {
        chart_box: interior_box(0.3, 0.7, 0.25, 0.55),
        event: (0.5, 0.4, 0.5, 0.4),
        ..regular
    });

    let stationary = build(Patch::graph(
        2,
        1,
        monomial_grid(
            2,
            1,
            &[(2, 0, -1.0), (1, 0, 1.0), (0, 1, 1.0), (0, 0, -0.65)],
        ),
    ))?;
    ladder.push(GermFixture {
        germ: BranchGerm::StationaryRegular {
            first_nonzero_order: 2,
        },
        chart_box: interior_box(0.3, 0.7, 0.3, 0.55),
        event: (0.5, 0.4, 0.5, 0.4),
        ..stationary
    });

    let cusp = build(Patch::graph(
        3,
        2,
        monomial_grid(
            3,
            2,
            &[
                (3, 0, -1.0),
                (2, 0, 0.75),
                (1, 0, -0.1875),
                (0, 2, 1.0),
                (0, 1, -1.0),
                (0, 0, 0.265625),
            ],
        ),
    ))?;
    ladder.push(GermFixture {
        germ: BranchGerm::CuspCandidate,
        chart_box: interior_box(0.2, 0.6, 0.3, 0.7),
        event: (0.25, 0.5, 0.25, 0.5),
        ..cusp
    });

    let singular = build(plane(constant_grid(1, 1, 0.0)))?;
    ladder.push(GermFixture {
        germ: BranchGerm::Singular,
        chart_box: interior_box(0.2, 0.8, 0.2, 0.8),
        event: (0.5, 0.5, 0.5, 0.5),
        ..singular
    });

    let unresolved = build(plane(monomial_grid(
        1,
        1,
        &[(0, 1, 1.0), (1, 0, -0.5), (0, 0, -0.25)],
    )))?;
    ladder.push(GermFixture {
        germ: BranchGerm::Unresolved,
        chart_box: interior_box(0.3, 0.7, 0.3, 0.6),
        event: (0.3, 0.4, 0.3, 0.4),
        ..unresolved
    });

    Ok(ladder)
}

/// The carrier of the closed-loop fixture.
#[derive(Debug)]
pub struct ClosedLoopPair {
    /// The cross-multiplied system of the graph pair.
    pub system: SquareSystem3,
    /// The loop center in the shared chart `(u, v)`.
    pub center: (f64, f64),
    /// The loop radius.
    pub radius: f64,
    /// A first seed on the loop.
    pub first_seed: (f64, f64, f64, f64),
    /// A second seed on the same loop.
    pub second_seed: (f64, f64, f64, f64),
}

/// Two seeds on one closed SSI branch (identity-recurrence ground truth).
///
/// Patch 1 is the bowl `(u, v, (u - 1/2)^2 + (v - 1/2)^2 - r^2)` against the
/// plane `z = 0`, so the zero set is the diagonal lift of the circle of radius
/// `r = 3/10` about `(1/2, 1/2)` — one closed branch lying fully in the chart
/// interior, transverse to the plane everywhere on it. A trace seeded anywhere
/// on the loop closes on itself (the loop's first box id equals the closing box
/// id). The two seeds, at the top and right of the circle, share that single
/// branch. Ground truth: `F = 0` at both seeds and at every sampled point of
/// the parametrized loop.
pub fn closed_loop_pair() -> Result<ClosedLoopPair, Refusal> {
    let h = monomial_grid(
        2,
        2,
        &[
            (2, 0, 1.0),
            (1, 0, -1.0),
            (0, 2, 1.0),
            (0, 1, -1.0),
            (0, 0, 0.41),
        ],
    );
    let a = Patch::graph(2, 2, h);
    let b = Patch::bilinear(
        coordinate_grid(1, 1, 0),
        coordinate_grid(1, 1, 1),
        constant_grid(1, 1, 0.0),
    );
    let system = cross_system(&a, &b)?;
    Ok(ClosedLoopPair {
        system,
        center: (0.5, 0.5),
        radius: 0.3,
        first_seed: (0.5, 0.8, 0.5, 0.8),
        second_seed: (0.8, 0.5, 0.8, 0.5),
    })
}

/// A synthetic [`BranchIncidence`] over the landed types, for trace-step
/// fixture tests. Regular germ, interior location, rank-0 deck, synthetic
/// provenance — a record shape, not a claim about any real contact.
pub fn sample_trace_incidence() -> BranchIncidence {
    let provenance = CurveOccurrenceProvenance {
        source_face_id: Some(SourceFaceId(7)),
        bound_id: BoundId(0),
        edge_use_id: EdgeUseId::new(BoundId(0), 3),
        source_edge_id: SourceEdgeId(11),
        start_vertex_id: SourceVertexKey::ShellVertex(1),
        end_vertex_id: SourceVertexKey::ShellVertex(2),
        source_curve_entity_id: Some(SourceEntityId(99)),
    };
    BranchIncidence {
        span_id: SpanId::from_occurrence(&provenance),
        provenance,
        parameter: ParameterEnclosure::from_pair((0.25, 0.35)),
        location: ParameterLocation::PieceInterior,
        germ: BranchGerm::Regular,
        side: CanonicalBranchSide::First,
        deck: CertifiedDeckLabel::zero(DeckContext::rank0()),
        representative: Point2::new(0.0, 0.0),
    }
}

/// A synthetic [`TraceStep`] over the landed types, for the trace-shape
/// fixture tests. Box `(u,v,s,t) = [0.2,0.6] x [0.3,0.5] x [0.2,0.6] x [0.3,0.5]`
/// with the regular synthetic incidence and an `s` continuation certificate.
pub fn sample_trace_step() -> Result<TraceStep, Refusal> {
    let coordinate = ContinuationCoordinate {
        index: 2,
        relative_margin: crate::contract::IntervalEnclosure::new(0.5, 1.0)?,
    };
    TraceStep::new(
        [(0.2, 0.6), (0.3, 0.5), (0.2, 0.6), (0.3, 0.5)],
        BranchGerm::Regular,
        sample_trace_incidence(),
        coordinate,
    )
}
