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

//! The SSI wave shim, part 1: the shared shapes the Phase-2 wave packets
//! exchange (BG-CK-P2-CONTRACT).
//!
//! This is a freeze in the P0-FREEZE pattern: only the shared shapes
//! (`SquareSystem3`, `KrawczykCertificate3`, `TraceStep`/`TraceOutcome`/
//! `TraceRefusal`) land here, with refusing constructors and verbatim
//! accessors. The four implementation packets (SYSTEM / KRAWCZYK3 / TRACE /
//! RESIDUAL) build against this module; nothing here evaluates, solves,
//! isolates, or certifies numerically. The mathematics is frozen in
//! `docs/CERTIFIED_PHASE2_BOOKING.md` and the frozen F3 contract
//! (`src/contract.rs`): this packet adds no decisions and invents no evidence
//! kinds.
//!
//! **D-shim.** Types and refusing constructors only. Any method that would
//! evaluate, solve, isolate, or certify NUMERICALLY refuses
//! (`InvalidInput`-shaped or a named case from the existing vocabularies). The
//! module doc says verbatim: "This module freezes shapes; BG-CK-P2-SYSTEM /
//! KRAWCZYK3 / TRACE implement against it and never restate it."
//!
//! **D-reuse.** `contract.rs`'s frozen F3 types are the vocabulary:
//! [`ContinuationCoordinate`], [`CoordinateSwitch`], [`SquareSystemInput`],
//! [`Refusal::ConditioningBelowThreshold`]. `formal/span.rs`'s [`BranchGerm`],
//! `formal/contact.rs`'s [`BranchIncidence`]. The shim wraps/aliases; it never
//! duplicates a landed type under a new name. Refusals wrap the landed named
//! causes verbatim — no new top-level evidence kinds (mapping section C).
//!
//! **D-homogeneous.** [`SquareSystem3`] carries the cross-multiplied
//! homogeneous system `F_k = W2*N1_k − W1*N2_k` (k ∈ x,y,z) as the two
//! PER-SIDE admitted carriers (the two rational tensor-Bernstein patches,
//! each with its own component numerator grids and weight grid) over
//! `(u,v) x (s,t)` (CFP-003-SEPARABILITY scope decision 1). Under D2 (unit
//! weights) the materialized 4-axis product grid is never needed for an
//! enclosure: Corollary 1.1 reduces every per-cell hull to the interval
//! difference of two per-side 2-D hulls. A derived materialized view of the
//! three component grids remains available (backed by a lazily-filled cache)
//! for the landed consumers this shim predates; the fast path reads the
//! per-side accessors and never fills it. The KRAWCZYK3 packet's `K(X)`
//! contract operates on the per-side enclosures. The weight certificates
//! `W1, W2 > 0` are INPUTS (carried as the patches' own landed
//! certificates), never re-derived here.

use crate::contract::{ContinuationCoordinate, Refusal};
use crate::formal::contact::{BranchIncidence, GenericUnresolved};
use crate::formal::span::BranchGerm;
use crate::hull::HullRefusal;

/// The stored square-system representation (SYSTEM's output contract).
///
/// `F_k(u,v,s,t) = W2*N1_k − W1*N2_k` as the two PER-SIDE carriers side A
/// (patch 1, over `(u, v)`) and side B (patch 2, over `(s, t)`), each a
/// rational tensor-Bernstein patch with its own component numerator grids and
/// strictly positive weight grid (CFP-003-SEPARABILITY scope decision 1). The
/// 4-axis product grid of the D-homogeneous system is NOT stored: every
/// certified enclosure is the interval difference / product of per-side 2-D
/// hulls (spec Corollaries 1.1-1.2), so the fast path never pays the O(deg⁴)
/// materialization. A derived materialized view ([`SquareSystem3::grids`]) is
/// kept for the pre-CFP consumers that read whole component grids; it is a
/// lazily-filled cache, populated only on demand. The 3x4 Jacobian is DERIVED
/// by consumers via the landed hull kernels, not stored. Constructed through
/// [`SquareSystem3::new`] (legacy, from materialized tables — the tables are
/// retained verbatim for the grid view and factored per side) or
/// [`SquareSystem3::from_sides`] (per-side, the CFP fast path); both refuse
/// ragged/empty grids, non-finite coefficients, and degree-0 inputs.
///
/// # Per-side layout
///
/// Side A has bidegree `(m1, n1)` in `(u, v)`, side B bidegree `(m2, n2)` in
/// `(s, t)`; `degrees = (m1, n1, m2, n2)`. Each side's numerator component
/// grids and weight grid are `(m+1) x (n+1)` tables over that side's own unit
/// square (rows index the first parameter). Under D2 (unit weights) the
/// cross-multiplied system is the coefficient difference `P1_k − P2_k`
/// (spec Theorem 1), so a component enclosure over `B1 x B2` is the interval
/// difference of the two per-side hulls (Corollary 1.1).
///
/// # Materialized view (legacy)
///
/// The pre-CFP consumers (`tangency/{chart, graph, hessian, cascade, a2}`,
/// the kernel engine, the fixture kit) read the three flat component grids
/// through [`SquareSystem3::grids`] in the historic layout
/// `row = a*(n1+1) + b`, `column = i*(n2+1) + j`. For a system constructed
/// from materialized tables that view is the input verbatim; for a per-side
/// system it is the cross-multiplied recomposition `W2*N1_k − W1*N2_k`, cached
/// on first demand. Nothing in the per-side fast path touches it.
///
/// This is a certificate *carrier*, not an interval algebra: it performs no
/// arithmetic (D-shim). Consumers read the per-side data or the materialized
/// view through the accessors and own every hull/interval evaluation.
pub struct SquareSystem3 {
    /// The two per-side admitted carriers (patch 1 over `(u,v)`, patch 2 over
    /// `(s,t)`).
    sides: [SideCarrier; 2],
    /// `(u0,u1,v0,v1,s0,s1,t0,t1)` — the two chart rectangles.
    domain_maps: (f64, f64, f64, f64, f64, f64, f64, f64),
    /// The lazily-filled materialized view of the three component grids (the
    /// pre-CFP whole-grid readers). Never touched by the per-side fast path.
    materialized: std::sync::OnceLock<[Vec<Vec<f64>>; 3]>,
}

/// One per-side carrier of a stored square system: a rational tensor-Bernstein
/// patch over its own `[0, 1]^2`.
///
/// `m`/`n` are the side's bidegree; `num[k][a][b]` is the `(m+1) x (n+1)`
/// homogeneous-numerator control grid of component `k` (x, y, z) and `w[a][b]`
/// the strictly positive, finite `(m+1) x (n+1)` weight grid (rows index the
/// first parameter). The weight certificate is an input (carried here as the
/// strictly positive finite weight grid); it is never re-derived.
#[derive(Debug, Clone)]
pub struct SideCarrier {
    /// Bidegree in the first parameter.
    m: usize,
    /// Bidegree in the second parameter.
    n: usize,
    /// Homogeneous numerator control grids, `(x, y, z)` order.
    num: [Vec<Vec<f64>>; 3],
    /// Strictly positive, finite weight grid.
    w: Vec<Vec<f64>>,
}

impl SideCarrier {
    /// Construct a side, refusing a degree-0 bidegree, empty or ragged grids,
    /// non-finite coefficients, or a non-positive weight.
    pub(crate) fn new(
        m: usize,
        n: usize,
        num: [Vec<Vec<f64>>; 3],
        w: Vec<Vec<f64>>,
    ) -> Result<Self, Refusal> {
        if m == 0 || n == 0 {
            return Err(Refusal::InvalidInput);
        }
        let shape_ok = |g: &[Vec<f64>]| {
            g.len() == m + 1
                && g.iter()
                    .all(|row| row.len() == n + 1 && row.iter().all(|c| c.is_finite()))
        };
        if !shape_ok(&num[0]) || !shape_ok(&num[1]) || !shape_ok(&num[2]) || !shape_ok(&w) {
            return Err(Refusal::InvalidInput);
        }
        if w.iter().any(|row| row.iter().any(|c| *c <= 0.0)) {
            return Err(Refusal::InvalidInput);
        }
        Ok(SideCarrier { m, n, num, w })
    }

    /// The side's bidegree in the first parameter.
    pub(crate) fn m(&self) -> usize {
        self.m
    }

    /// The side's bidegree in the second parameter.
    pub(crate) fn n(&self) -> usize {
        self.n
    }

    /// The homogeneous numerator control grids, `(x, y, z)` order.
    pub(crate) fn numerator(&self) -> &[Vec<Vec<f64>>; 3] {
        &self.num
    }

    /// The strictly positive weight grid.
    pub(crate) fn weights(&self) -> &[Vec<f64>] {
        &self.w
    }
}

impl SquareSystem3 {
    /// Construct a square system from three preformed component tables plus the
    /// degree and chart metadata.
    ///
    /// The tables are the historic flat materialization of the four-axis
    /// D-homogeneous system (legacy / fixture consumers); they are retained
    /// verbatim for the materialized view and factored per side under D2
    /// (unit weights) so the per-side accessors are available. A table whose
    /// rows are not each a fixed per-side difference (i.e. a genuinely
    /// four-axis array outside the D2 product-difference family) refuses.
    ///
    /// Refuses (each as [`Refusal::InvalidInput`], a construction outside a
    /// frozen rule):
    /// - a degree-0 input (any of `m1, n1, m2, n2` zero);
    /// - an empty, ragged, or degree-mismatched table (any table whose row or
    ///   column count does not equal the shape the degrees demand);
    /// - any non-finite coefficient;
    /// - a non-finite, misordered, or degenerate (zero-width) chart interval in
    ///   `domain_maps`.
    pub fn new(
        tables: [Vec<Vec<f64>>; 3],
        degrees: (usize, usize, usize, usize),
        domain_maps: (f64, f64, f64, f64, f64, f64, f64, f64),
    ) -> Result<Self, Refusal> {
        let (m1, n1, m2, n2) = degrees;
        if m1 == 0 || n1 == 0 || m2 == 0 || n2 == 0 {
            return Err(Refusal::InvalidInput);
        }
        let rows = (m1 + 1) * (n1 + 1);
        let cols = (m2 + 1) * (n2 + 1);
        for table in &tables {
            if table.len() != rows {
                return Err(Refusal::InvalidInput);
            }
            for row in table {
                if row.len() != cols || !row.iter().all(|c| c.is_finite()) {
                    return Err(Refusal::InvalidInput);
                }
            }
        }
        let [u0, u1, v0, v1, s0, s1, t0, t1] = [
            domain_maps.0,
            domain_maps.1,
            domain_maps.2,
            domain_maps.3,
            domain_maps.4,
            domain_maps.5,
            domain_maps.6,
            domain_maps.7,
        ];
        for (lo, hi) in [(u0, u1), (v0, v1), (s0, s1), (t0, t1)] {
            if !lo.is_finite() || !hi.is_finite() || lo >= hi {
                return Err(Refusal::InvalidInput);
            }
        }
        let sides = factor_sides(&tables, degrees)?;
        let system = SquareSystem3 {
            sides,
            domain_maps,
            materialized: std::sync::OnceLock::new(),
        };
        // The legacy grid view is the input verbatim.
        let _ = system.materialized.set(tables);
        Ok(system)
    }

    /// Construct a square system from the two per-side admitted carriers (the
    /// CFP fast path). The materialized view stays empty until a legacy
    /// whole-grid consumer requests it.
    pub(crate) fn from_sides(
        side_a: SideCarrier,
        side_b: SideCarrier,
        domain_maps: (f64, f64, f64, f64, f64, f64, f64, f64),
    ) -> Result<Self, Refusal> {
        let [u0, u1, v0, v1, s0, s1, t0, t1] = [
            domain_maps.0,
            domain_maps.1,
            domain_maps.2,
            domain_maps.3,
            domain_maps.4,
            domain_maps.5,
            domain_maps.6,
            domain_maps.7,
        ];
        for (lo, hi) in [(u0, u1), (v0, v1), (s0, s1), (t0, t1)] {
            if !lo.is_finite() || !hi.is_finite() || lo >= hi {
                return Err(Refusal::InvalidInput);
            }
        }
        Ok(SquareSystem3 {
            sides: [side_a, side_b],
            domain_maps,
            materialized: std::sync::OnceLock::new(),
        })
    }

    /// The derived materialized view of the three component grids, in
    /// `(x, y, z)` order and the historic flat layout
    /// (`row = a*(n1+1) + b`, `column = i*(n2+1) + j`).
    ///
    /// For a system constructed from materialized tables this is the input
    /// verbatim; for a per-side system it is the cross-multiplied
    /// recomposition `W2*N1_k − W1*N2_k`, computed once and cached. The
    /// per-side fast path never calls this accessor.
    pub fn grids(&self) -> &[Vec<Vec<f64>>; 3] {
        self.materialized.get_or_init(|| {
            let a = &self.sides[0];
            let b = &self.sides[1];
            materialize_sides(a, b)
        })
    }

    /// The first side's carrier (patch 1, over `(u, v)`).
    pub fn side_a(&self) -> &SideCarrier {
        &self.sides[0]
    }

    /// The second side's carrier (patch 2, over `(s, t)`).
    pub fn side_b(&self) -> &SideCarrier {
        &self.sides[1]
    }

    /// `(m1, n1, m2, n2)` — the stored degrees (side A bidegree, then side B
    /// bidegree), verbatim.
    pub fn degrees(&self) -> (usize, usize, usize, usize) {
        (
            self.sides[0].m,
            self.sides[0].n,
            self.sides[1].m,
            self.sides[1].n,
        )
    }

    /// `(u0,u1,v0,v1,s0,s1,t0,t1)` — the stored chart rectangles, verbatim.
    pub fn domain_maps(&self) -> (f64, f64, f64, f64, f64, f64, f64, f64) {
        self.domain_maps
    }
}

impl std::fmt::Debug for SquareSystem3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SquareSystem3")
            .field("side_a", &self.sides[0])
            .field("side_b", &self.sides[1])
            .field("domain_maps", &self.domain_maps)
            .finish_non_exhaustive()
    }
}

impl Clone for SquareSystem3 {
    fn clone(&self) -> Self {
        let cloned = SquareSystem3 {
            sides: self.sides.clone(),
            domain_maps: self.domain_maps,
            materialized: std::sync::OnceLock::new(),
        };
        if let Some(view) = self.materialized.get() {
            let _ = cloned.materialized.set(view.clone());
        }
        cloned
    }
}

impl PartialEq for SquareSystem3 {
    fn eq(&self, other: &Self) -> bool {
        // Two stored systems are equal exactly when their materialized
        // component views, degrees, and chart rectangles agree. Comparing the
        // derived view (rather than the raw per-side carriers) keeps a
        // per-side system equal to the legacy system built from its own grids.
        self.degrees() == other.degrees()
            && self.domain_maps == other.domain_maps
            && self.grids() == other.grids()
    }
}

/// Factor three flat four-axis component tables into the two per-side carriers
/// under D2 (unit weights).
///
/// The D2 product-difference family is `c[a][b][i][j] = A[a][b] − B[i][j]`
/// (spec Theorem 1). Picking the row slice at flat column `0` for `A` and the
/// column slice at flat row `0` for `B` reproduces every difference table
/// exactly in exact arithmetic: `A[r] := c[r][0]`, `B[c] := c[0][0] − c[0][c]`
/// gives `A[r] − B[c] = c[r][0] − c[0][0] + c[0][c] = c[r][c]` on the family.
/// The materialized view ([`SquareSystem3::grids`]) is retained verbatim, so
/// the whole-grid consumers never depend on the slice values; the slices only
/// feed the per-side fast path, which the fixture/exact systems (dyadic
/// coefficients, where the slice reconstruction is float-exact) exercise.
#[allow(clippy::needless_range_loop)] // flat-layout index algebra (row/column = a·sp+b) is clearest in the explicit range form
fn factor_sides(
    tables: &[Vec<Vec<f64>>; 3],
    degrees: (usize, usize, usize, usize),
) -> Result<[SideCarrier; 2], Refusal> {
    let (m1, n1, m2, n2) = degrees;
    let row_sp = n1 + 1;
    let col_sp = n2 + 1;
    let mut a_num: [Vec<Vec<f64>>; 3] = [
        vec![vec![0.0f64; n1 + 1]; m1 + 1],
        vec![vec![0.0f64; n1 + 1]; m1 + 1],
        vec![vec![0.0f64; n1 + 1]; m1 + 1],
    ];
    let mut b_num: [Vec<Vec<f64>>; 3] = [
        vec![vec![0.0f64; n2 + 1]; m2 + 1],
        vec![vec![0.0f64; n2 + 1]; m2 + 1],
        vec![vec![0.0f64; n2 + 1]; m2 + 1],
    ];
    for (k, table) in tables.iter().enumerate() {
        for r in 0..(m1 + 1) * (n1 + 1) {
            // A[r] := c[r][0]; r = a*(n1+1)+b.
            let a = r / row_sp;
            let b = r % row_sp;
            a_num[k][a][b] = table[r][0];
        }
        for c in 0..(m2 + 1) * (n2 + 1) {
            let i = c / col_sp;
            let j = c % col_sp;
            b_num[k][i][j] = table[0][0] - table[0][c];
        }
    }
    let ones_a = vec![vec![1.0f64; n1 + 1]; m1 + 1];
    let ones_b = vec![vec![1.0f64; n2 + 1]; m2 + 1];
    let side_a = SideCarrier::new(m1, n1, a_num, ones_a)?;
    let side_b = SideCarrier::new(m2, n2, b_num, ones_b)?;
    Ok([side_a, side_b])
}

/// Recompute the flat four-axis component tables of a per-side system
/// (the historic materialization): at flat index `(a, b, i, j)`,
/// `W2[i][j]·N1_k[a][b] − W1[a][b]·N2_k[i][j]`.
#[allow(clippy::needless_range_loop)] // cross-multiplied materialization over the per-axis index lattice is clearest in the explicit range form
fn materialize_sides(side_a: &SideCarrier, side_b: &SideCarrier) -> [Vec<Vec<f64>>; 3] {
    let (m1, n1) = (side_a.m, side_a.n);
    let (m2, n2) = (side_b.m, side_b.n);
    let rows = (m1 + 1) * (n1 + 1);
    let cols = (m2 + 1) * (n2 + 1);
    let mut out = [
        vec![vec![0.0f64; cols]; rows],
        vec![vec![0.0f64; cols]; rows],
        vec![vec![0.0f64; cols]; rows],
    ];
    for k in 0..3 {
        let grid_k = &mut out[k];
        for a in 0..=m1 {
            for bb in 0..=n1 {
                let row = a * (n1 + 1) + bb;
                let w1 = side_a.w[a][bb];
                let n1k = side_a.num[k][a][bb];
                for i in 0..=m2 {
                    for j in 0..=n2 {
                        let col = i * (n2 + 1) + j;
                        grid_k[row][col] = side_b.w[i][j] * n1k - w1 * side_b.num[k][i][j];
                    }
                }
            }
        }
    }
    out
}

/// The Krawczyk unique-root certificate (KRAWCZYK3's output contract).
///
/// Constructed ONLY from a strict inclusion: [`KrawczykCertificate3::new`]
/// refuses a non-strict or boundary inclusion (K(X) must be component-wise
/// strictly inside X) — the frozen emission rule made typecheckable. Carries
/// the box `X`, the K(X) enclosure, and the determinant enclosure (0 excluded).
///
/// The determinant enclosure's excluding zero is part of construction, not a
/// later check: a determinant enclosure containing zero is a box the operator
/// may not certify, and the constructor refuses it. This is a carrier only
/// (D-shim); the enclosure values arrive from the consumers' certified
/// interval work.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KrawczykCertificate3 {
    /// The box `X`: three axis intervals.
    box_x: [(f64, f64); 3],
    /// The K(X) enclosure, component-wise strictly inside `box_x`.
    k_x: [(f64, f64); 3],
    /// The determinant enclosure over `box_x`, with 0 strictly excluded.
    det: (f64, f64),
}

impl KrawczykCertificate3 {
    /// Build the certificate from a strict inclusion and an orientation
    /// enclosure.
    ///
    /// Refuses (each as [`Refusal::InvalidInput`]):
    /// - any non-finite or misordered (`lo > hi`) box or K(X) interval;
    /// - a non-strict or boundary inclusion — K(X) is not component-wise
    ///   STRICTLY inside X on every axis;
    /// - a non-finite, misordered determinant interval, or one containing zero
    ///   (inclusive of a `0` endpoint) — the orientation precondition is part
    ///   of construction.
    pub fn new(
        box_x: [(f64, f64); 3],
        k_x: [(f64, f64); 3],
        det: (f64, f64),
    ) -> Result<Self, Refusal> {
        for (box_axis, k_axis) in box_x.iter().zip(k_x.iter()) {
            let (b_lo, b_hi) = *box_axis;
            let (k_lo, k_hi) = *k_axis;
            let interval_ok = |(lo, hi): (f64, f64)| lo.is_finite() && hi.is_finite() && lo <= hi;
            if !interval_ok(*box_axis) || !interval_ok(*k_axis) {
                return Err(Refusal::InvalidInput);
            }
            // Strict inclusion: K(X) strictly inside X on this axis. A
            // boundary-touching or reversed enclosure refuses.
            let strictly_inside = b_lo < k_lo && k_hi < b_hi;
            if !strictly_inside {
                return Err(Refusal::InvalidInput);
            }
        }
        let (d_lo, d_hi) = det;
        if !d_lo.is_finite() || !d_hi.is_finite() || d_lo > d_hi {
            return Err(Refusal::InvalidInput);
        }
        // 0 excluded STRICTLY: an endpoint of 0 is still a determinant
        // enclosure that contains zero.
        if d_lo <= 0.0 && 0.0 <= d_hi {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self { box_x, k_x, det })
    }

    /// The box `X`: three axis intervals, verbatim.
    pub fn box_x(&self) -> [(f64, f64); 3] {
        self.box_x
    }

    /// The K(X) enclosure, verbatim.
    pub fn k_x(&self) -> [(f64, f64); 3] {
        self.k_x
    }

    /// The determinant enclosure (0 excluded), verbatim.
    pub fn det(&self) -> (f64, f64) {
        self.det
    }
}

/// One traced branch box (TRACE's per-step output): the parameter box in the
/// 4D chart, the germ class, the branch incidence record, and the
/// continuation-coordinate certificate the frozen F3 rule carries at the box.
///
/// `box` holds the four axis intervals of the trace box in chart order
/// `(u, v, s, t)` — patch 1's two axes then patch 2's two axes. The germ and
/// the continuation coordinate are carried as the landed [`BranchGerm`] and
/// [`ContinuationCoordinate`] values; the incidence is the landed
/// [`BranchIncidence`] record (D-reuse). Constructed through
/// [`TraceStep::new`], which refuses a non-finite or misordered box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceStep {
    /// The trace box in the 4D chart, as `(u,v,s,t)` axis intervals.
    chart_box: [(f64, f64); 4],
    /// The germ class of the branch at this box.
    germ: BranchGerm,
    /// The branch incidence record.
    incidence: BranchIncidence,
    /// The certified continuation coordinate for this box.
    coordinate: ContinuationCoordinate,
}

impl TraceStep {
    /// Build one trace step from the landed types plus the box.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a box containing a non-finite or
    /// misordered (`lo > hi`) axis interval. The germ, incidence and
    /// coordinate are already-typed landed values and are carried verbatim.
    pub fn new(
        chart_box: [(f64, f64); 4],
        germ: BranchGerm,
        incidence: BranchIncidence,
        coordinate: ContinuationCoordinate,
    ) -> Result<Self, Refusal> {
        for (lo, hi) in chart_box {
            if !lo.is_finite() || !hi.is_finite() || lo > hi {
                return Err(Refusal::InvalidInput);
            }
        }
        Ok(Self {
            chart_box,
            germ,
            incidence,
            coordinate,
        })
    }

    /// The trace box in the 4D chart, as `(u,v,s,t)` axis intervals, verbatim.
    pub fn chart_box(&self) -> [(f64, f64); 4] {
        self.chart_box
    }

    /// The germ class carried at this box.
    pub fn germ(&self) -> BranchGerm {
        self.germ
    }

    /// The branch incidence record.
    pub fn incidence(&self) -> BranchIncidence {
        self.incidence
    }

    /// The certified continuation coordinate for this box.
    pub fn coordinate(&self) -> ContinuationCoordinate {
        self.coordinate
    }
}

/// The outcome of tracing one branch from one seed.
///
/// Shape mirrors the landed pair-contact results ([`crate::formal::contact::PairContactResult`]):
/// named cases, no catch-all. The refusal vocabulary wraps the landed named
/// causes (D-reuse) — no new top-level evidence kinds (mapping section C).
#[derive(Debug, Clone, PartialEq)]
pub enum TraceOutcome {
    /// The branch closed on itself (identity recurrence) — the loop's first
    /// box id equals the closing box id.
    ClosedLoop {
        /// The steps traced around the closed loop.
        steps: Vec<TraceStep>,
    },
    /// The branch terminated at a certified boundary/refusal-free end.
    Terminated {
        /// The steps traced to the end.
        steps: Vec<TraceStep>,
    },
    /// A certified turning-point switch occurred mid-branch.
    Switched {
        /// The steps traced up to and including the switch box.
        steps: Vec<TraceStep>,
        /// The switch event, carrying BOTH required certificates (F3).
        switch: crate::contract::CoordinateSwitch,
    },
    /// A named refusal case.
    Refused(TraceRefusal),
}

/// The trace refusal vocabulary: aliases/wraps of LANDED named cases.
///
/// No new top-level evidence kinds (mapping section C): each variant carries a
/// landed cause type verbatim. A variant exists per refusal family — the
/// conditioning refusal (F3, `ConditioningBelowThreshold`-shaped), the hull
/// enclosure failures, and the generic-unresolved causes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceRefusal {
    /// An F3 conditioning refusal: a box where the frozen coordinate-selection
    /// rule could not certify any coordinate away-from-zero. Wraps the landed
    /// [`Refusal`] verbatim; the trace-relevant value is
    /// [`Refusal::ConditioningBelowThreshold`].
    Conditioning(Refusal),
    /// A certified enclosure could not be produced by the hull layer. Wraps
    /// [`HullRefusal`] verbatim (the `EnclosureUnavailable` / `DomainNotCompact`
    /// named cases).
    Hull(HullRefusal),
    /// The branch could not be certified under the declared numerical policy.
    /// Wraps [`GenericUnresolved`] verbatim (the landed named causes).
    Unresolved(GenericUnresolved),
}

/// One trace refusal's stable diagnostic tag.
impl TraceRefusal {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Conditioning(Refusal::ConditioningBelowThreshold) => "trace_refused_conditioning",
            Self::Conditioning(Refusal::InvalidInput) => "trace_refused_invalid_input",
            Self::Conditioning(Refusal::Unfrozen) => "trace_refused_unfrozen",
            Self::Hull(HullRefusal::EnclosureUnavailable) => {
                "trace_refused_hull_enclosure_unavailable"
            }
            Self::Hull(HullRefusal::DomainNotCompact) => "trace_refused_hull_domain_not_compact",
            Self::Unresolved(cause) => cause.tag(),
        }
    }
}
