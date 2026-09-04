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

//! The CC fixture kit (CC-000-CONTRACT, spine §6): machine-checked ground
//! truths and refusal-path inputs for the wave packets that type against the
//! construct seams.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **TEST SUPPORT ONLY.** This module is `#[doc(hidden)] pub`, excluded from
//! the certified API surface, but reachable by wave packets' integration tests
//! through the crate's public path (the `kernel/fixtures.rs` pattern).
//!
//! Each fixture is a `pub fn` returning constructed data plus a doc-stated
//! NUMERIC ground truth; the `tests/construct_contract.rs` target
//! machine-checks it by direct evaluation — never by solving. Fixtures are
//! data + builders only; no solver is exercised to build them.
//!
//! 1. `banded_cubic_uniform(n)`: the order-`(n+1)` uniform cubic B-spline
//!    interpolation collocation matrix (tridiagonal Toeplitz, diagonal 4,
//!    unit off-diagonals) under uniform stations over `n + 1` sections; ground
//!    truth: strictly diagonally dominant with positive diagonal, hence
//!    positive definite — the exact integer determinant `det_exact` (positive)
//!    is stored and recomputed by the tridiagonal recurrence in the test.
//! 2. `banded_pivot_spans_zero()`: a `2 x 2` row-major band whose first
//!    diagonal pivot strictly contains `0`; ground truth: the CC-001 no-pivot
//!    banded GE refusal path.
//! 3. `argmin_separated()`: interval enclosures with a unique strict
//!    `sup < inf` argmin at index 0; ground truth: the S5 success path.
//! 4. `argmin_overlapping()`: overlapping interval enclosures; ground truth:
//!    no index separates — the S5 `AmbiguousEventOrdering` refusal path.
//! 5. `flat_patch()`: `sigma > 0` with `L = 0` (a planar patch); ground
//!    truth: `expected_delta = 2 sigma_lo / L = +infinity`.
//! 6. `curved_patch()`: `sigma > 0` with a known positive `L`; ground truth:
//!    `expected_delta = 2 sigma_lo / L` exactly (dyadic data).
//! 7. `degenerate_patch()`: a `sigma` enclosure strictly containing `0`;
//!    ground truth: the CC-002 injectivity-radius refusal path.
//! 8. `genuine_star()`: two S6-shaped disk pieces whose determinant lower
//!    bounds are all strictly positive, seams glued, boundaries simple;
//!    ground truth: graph-disk admission data (CC-005 success path).
//! 9. `folded_corner()`: a constructed fold — the two disk pieces carry
//!    opposite-signed determinant lower bounds and the fold piece's boundary
//!    is not simple; ground truth: the CC-005
//!    `StarNotEmbedded`/`NoAdmissibleProjection` refusal path.

use crate::construct::refusal::ConstructRefusal;
use crate::construct::Interval;

/// The `banded_cubic_uniform` fixture (spine §6, CC-001 success path).
#[derive(Debug, Clone)]
pub struct BandedCubicFixture {
    /// The requested interval count `n`; sections = `n + 1`.
    pub n: usize,
    /// The matrix order `n + 1`.
    pub size: usize,
    /// The uniform stations over `0..=n`, spacing exactly `1.0`.
    pub stations: Vec<f64>,
    /// The row-major tridiagonal collocation coefficients (order `size`).
    pub bands: Vec<Interval>,
    /// The exact integer determinant of the collocation matrix (positive).
    pub det_exact: i64,
}

/// Fixture 1 (CC-001 success): the uniform cubic collocation matrix.
///
/// Ground truth: `bands` is the order-`(n+1)` tridiagonal Toeplitz matrix
/// with diagonal `4` and unit off-diagonals — the uniform-station cubic
/// B-spline interpolation matrix over `n + 1` sections. It is strictly
/// diagonally dominant with a strictly positive diagonal, so it is positive
/// definite (Sylvester): the determinant is positive for every `n`, stored
/// exactly in `det_exact` via the tridiagonal recurrence
/// `D_0 = 1, D_1 = 4, D_k = 4 D_{k-1} - D_{k-2}`. Sizes whose determinant
/// overflows `i64` refuse `InvalidInput`.
pub fn banded_cubic_uniform(n: usize) -> Result<BandedCubicFixture, ConstructRefusal> {
    let size = n.checked_add(1).ok_or(ConstructRefusal::InvalidInput)?;
    let det_exact = exact_tridiagonal_det(size).ok_or(ConstructRefusal::InvalidInput)?;
    let stations: Vec<f64> = (0..=n).map(|i| i as f64).collect();
    let mut bands = Vec::with_capacity(size * size);
    for row in 0..size {
        for col in 0..size {
            let value = if row == col {
                4.0
            } else if (row as isize - col as isize).abs() == 1 {
                1.0
            } else {
                0.0
            };
            bands.push(Interval::point(value));
        }
    }
    Ok(BandedCubicFixture {
        n,
        size,
        stations,
        bands,
        det_exact,
    })
}

/// The `banded_pivot_spans_zero` fixture (CC-001 refusal path).
#[derive(Debug, Clone)]
pub struct BandedPivotFixture {
    /// The row-major `2 x 2` collocation bands; `bands[0]` is the first
    /// diagonal pivot.
    pub bands: Vec<Interval>,
    /// The matrix order (`2`).
    pub size: usize,
}

/// Fixture 2 (CC-001 refusal): a banded system whose first pivot contains 0.
///
/// Ground truth: the first diagonal pivot `bands[0]` strictly contains `0`
/// (`lo < 0 < hi`), so a no-pivot banded GE must refuse
/// `SingularInterpolationSystem` on it.
pub fn banded_pivot_spans_zero() -> Result<BandedPivotFixture, ConstructRefusal> {
    let bands = vec![
        Interval {
            lo: -0.25,
            hi: 0.25,
        },
        Interval::point(1.0),
        Interval::point(0.0),
        Interval::point(1.0),
    ];
    Ok(BandedPivotFixture { bands, size: 2 })
}

/// The `argmin_separated` fixture (S5 success path).
#[derive(Debug, Clone)]
pub struct ArgminSeparatedFixture {
    /// The interval enclosures, in index order.
    pub enclosures: Vec<Interval>,
    /// The unique strict argmin index.
    pub argmin: usize,
}

/// Fixture 3 (CC-003 success): enclosures with a strict separated argmin.
///
/// Ground truth: `enclosures = [[0,1], [2,3], [4,5]]` with `argmin = 0`; the
/// argmin's supremum is strictly below every other enclosure's infimum, so
/// `argmin_margin` succeeds with `i* = 0`.
pub fn argmin_separated() -> Result<ArgminSeparatedFixture, ConstructRefusal> {
    Ok(ArgminSeparatedFixture {
        enclosures: vec![
            Interval { lo: 0.0, hi: 1.0 },
            Interval { lo: 2.0, hi: 3.0 },
            Interval { lo: 4.0, hi: 5.0 },
        ],
        argmin: 0,
    })
}

/// The `argmin_overlapping` fixture (S5 refusal path).
#[derive(Debug, Clone)]
pub struct ArgminOverlappingFixture {
    /// The interval enclosures, in index order.
    pub enclosures: Vec<Interval>,
}

/// Fixture 4 (CC-003 refusal): overlapping enclosures, no strict argmin.
///
/// Ground truth: `enclosures = [[0,3], [2,5], [4,7]]` pairwise overlap, so no
/// index `i*` satisfies `sup[i*] < inf[j]` for every `j != i*` —
/// `argmin_margin` refuses `AmbiguousEventOrdering`.
pub fn argmin_overlapping() -> Result<ArgminOverlappingFixture, ConstructRefusal> {
    Ok(ArgminOverlappingFixture {
        enclosures: vec![
            Interval { lo: 0.0, hi: 3.0 },
            Interval { lo: 2.0, hi: 5.0 },
            Interval { lo: 4.0, hi: 7.0 },
        ],
    })
}

/// The `flat_patch` / `curved_patch` / `degenerate_patch` fixtures (CC-002).
#[derive(Debug, Clone, Copy)]
pub struct PatchMarginFixture {
    /// The `sigma` margin enclosure `(lo, hi)` (the rank-margin style bound on
    /// `|S_u x S_v|` over the patch).
    pub sigma: (f64, f64),
    /// The curvature upper bound `L` of the patch.
    pub curvature_l: f64,
    /// The expected injectivity-radius lower bound `2 sigma_lo / L`.
    pub expected_delta: f64,
}

/// Fixture 5 (CC-002 success, planar): `sigma > 0`, `L = 0`.
///
/// Ground truth: a planar patch has `L = 0`, so
/// `expected_delta = 2 sigma_lo / L = +infinity`.
pub fn flat_patch() -> Result<PatchMarginFixture, ConstructRefusal> {
    Ok(PatchMarginFixture {
        sigma: (2.0, 3.0),
        curvature_l: 0.0,
        expected_delta: f64::INFINITY,
    })
}

/// Fixture 6 (CC-002 success, curved): `sigma > 0` with a known positive `L`.
///
/// Ground truth: `sigma = (4, 5)`, `L = 2`, so
/// `expected_delta = 2 * 4 / 2 = 4` exactly (dyadic data).
pub fn curved_patch() -> Result<PatchMarginFixture, ConstructRefusal> {
    Ok(PatchMarginFixture {
        sigma: (4.0, 5.0),
        curvature_l: 2.0,
        expected_delta: 4.0,
    })
}

/// Fixture 7 (CC-002 refusal): a `sigma` enclosure strictly containing `0`.
///
/// Ground truth: `sigma = (-1, 1)` contains `0`, so no injectivity radius is
/// admissible; the expected delta of the raw formula is non-positive
/// (`expected_delta = -2`), which CC-002 refuses as a degenerate map.
pub fn degenerate_patch() -> Result<PatchMarginFixture, ConstructRefusal> {
    Ok(PatchMarginFixture {
        sigma: (-1.0, 1.0),
        curvature_l: 1.0,
        expected_delta: -2.0,
    })
}

/// One S6-shaped disk-piece record (spine §6, graph-disk fixtures).
#[derive(Debug, Clone, Copy)]
pub struct DiskPieceRecord {
    /// The piece's determinant lower-bound enclosure.
    pub det_lower: Interval,
    /// Whether the piece's seams are glued to its neighbours.
    pub seam_glued: bool,
    /// Whether the piece's boundary is simple.
    pub boundary_simple: bool,
}

/// The `genuine_star` / `folded_corner` graph-disk fixtures (CC-005).
#[derive(Debug, Clone)]
pub struct GraphDiskFixture {
    /// The per-piece records, in seam order.
    pub pieces: Vec<DiskPieceRecord>,
    /// Ground truth: whether the determinant lower bounds change sign across
    /// the piece set.
    pub sign_change: bool,
}

/// Fixture 8 (CC-005 success): a two-plane wedge star, known embedded.
///
/// Ground truth: every piece's determinant lower bound is strictly positive,
/// every seam is glued, and every boundary is simple — graph-disk admission
/// data (`sign_change = false`).
pub fn genuine_star() -> Result<GraphDiskFixture, ConstructRefusal> {
    Ok(GraphDiskFixture {
        pieces: vec![
            DiskPieceRecord {
                det_lower: Interval { lo: 1.0, hi: 2.0 },
                seam_glued: true,
                boundary_simple: true,
            },
            DiskPieceRecord {
                det_lower: Interval { lo: 0.5, hi: 1.0 },
                seam_glued: true,
                boundary_simple: true,
            },
        ],
        sign_change: false,
    })
}

/// Fixture 9 (CC-005 refusal): a constructed fold.
///
/// Ground truth: the two pieces carry opposite-signed determinant lower
/// bounds (a sign change across the fold, `sign_change = true`) and the fold
/// piece's boundary is not simple — graph-disk certification must refuse
/// (`StarNotEmbedded` / `NoAdmissibleProjection`).
pub fn folded_corner() -> Result<GraphDiskFixture, ConstructRefusal> {
    Ok(GraphDiskFixture {
        pieces: vec![
            DiskPieceRecord {
                det_lower: Interval { lo: 0.5, hi: 1.0 },
                seam_glued: true,
                boundary_simple: true,
            },
            DiskPieceRecord {
                det_lower: Interval { lo: -1.0, hi: -0.5 },
                seam_glued: true,
                boundary_simple: false,
            },
        ],
        sign_change: true,
    })
}

/// The exact integer determinant of the order-`k` tridiagonal Toeplitz matrix
/// with diagonal `4` and unit off-diagonals, via the recurrence
/// `D_0 = 1, D_1 = 4, D_k = 4 D_{k-1} - D_{k-2}`. `None` on overflow.
fn exact_tridiagonal_det(k: usize) -> Option<i64> {
    let mut d_km2: i64 = 1; // D_0
    let mut d_km1: i64 = 4; // D_1 (order >= 1 here)
    if k == 1 {
        return Some(d_km1);
    }
    for _ in 2..=k {
        let d = d_km1.checked_mul(4)?.checked_sub(d_km2)?;
        d_km2 = d_km1;
        d_km1 = d;
    }
    Some(d_km1)
}
