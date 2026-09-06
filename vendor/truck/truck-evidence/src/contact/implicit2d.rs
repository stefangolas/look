//! CFP-004-IMPLICIT-REDUCTION — the funnel's analytic×spline implicit-reduction
//! stage (Theorem 4, `docs/CONTACT_FAST_PATH_BUILD_SPEC.md` §3).
//!
//! For a **recognized analytic carrier** (a plane or a quadric — sphere,
//! cylinder, cone) against a **spline patch**, the contact set on the spline
//! chart is the zero set of ONE scalar function
//!
//! ```text
//! h(u, v) = g(S(u, v))
//! ```
//!
//! in exact Bernstein form: bidegree `(p, q)` preserved for planes
//! (coefficients = signed control-point distances) and `(2p, 2q)` for quadrics
//! (the exact Bernstein product). The torus is **explicitly excluded**: its
//! quartic implicit gives `(4p, 4q)` and its own economics (spec §7). A
//! critical point of `h` is `∇h = 0` — a 2×2 square system for the landed
//! Krawczyk directly (the `2×2` specialization of `num::krawczyk`), so the
//! funnel needs no F3 reduction, no continuation-axis choice, and no
//! `ConditioningBelowThreshold` for this class.
//!
//! # The elevation trap (packet-normative, F-C4)
//!
//! `∂h/∂u` and `∂h/∂v` have bidegrees `(p−1, q)` and `(p, q−1)`; their
//! coefficient arrays cannot be paired into 2-vectors until both are
//! degree-elevated to a COMMON bidegree — only then is
//! `∇h(x) = Σ B_I(x)(a_I, b_I)` a convex combination and the hull test valid.
//! The naive (non-elevated) pairing can emit a false loop-free certificate on a
//! cell that in fact contains a critical point (the F-C4 twin: the bilinear
//! saddle `h = u + v − 2uv`). Cheap and exact;
//! [`gradient_hull_elevated`] is the verdict the stage trusts.
//!
//! # Stage semantics
//!
//! The reduction over one spline-chart cell:
//!
//! 1. **Exclusion** — if the `h` coefficient hull (outward-rounded) excludes
//!    zero, the cell certifies empty (`0 ∉ conv{h coefficients}`: every point
//!    of the patch lies on one side of the analytic carrier).
//! 2. **Loop detection** — if the ELEVATED `∇h` coefficient hull excludes the
//!    origin, the cell is loop-free: `h` has no interior critical point (Jordan
//!    + extreme-value theorem; `h ≡ 0` on the cell fails the test honestly).
//! 3. **Critical-point certification** — otherwise a float Newton search (the
//!    search half of an SFC pair) hunts for `∇h = 0`; each candidate is
//!    certified by the landed 2×2 Krawczyk operator over a small enclosing box
//!    (the exact half). A certified critical point is the tangent/degenerate
//!    contact structure of the cell.
//!
//! # Consumer rule (spec §3a, DECIDED 2026-09-06)
//!
//! An `Unresolved` cell that propagates into the boundary rewrite is a typed
//! refusal to the caller of `boolean()` **failing the whole operation**, with
//! the stratum-pair identity carried so a client-layer partition strategy
//! remains available — no narrowing retry, no fallback, no partial-result
//! shape. The funnel-side localization vocabulary (the evidence mirror of the
//! CFP spine's decision-3 shapes) lives in `super` ([`super::LocalizedRefusal`],
//! [`super::StratumSide`], [`super::StratumPair`]); F1 forbids this crate
//! naming `truck-certified`, so the shapes are mirrored here exactly as the
//! instrument schema is mirrored (`contact::instrument`).
//!
//! House rules H-1..H-8 apply: no `unwrap`/`expect`/`panic!`, no out-of-range
//! indexing, SFC (search in floats, certify exactly), and a float is never
//! recorded as `Exact` (H-6).

use crate::enclosure::interval_at;
use crate::enclosure::Interval;
use crate::num::krawczyk::{krawczyk, KrawczykProof, KrawczykSystem};
use truck_base::cgmath64::Vector4;
use truck_base::evidence::Budget;
use truck_geometry::nurbs::BSplineSurface;
use truck_geometry::recognize::CanonicalSurface;
use truck_geometry::specifieds::{Cone, Cylinder, Plane, Sphere};

/// The relative outward pad of the hull sign predicates, as a multiple of
/// `EPSILON` (the `bspline.rs` `HULL_PAD` precedent; sized to absorb the `f64`
/// rounding of the quadric Bernstein products).
const HULL_PAD: f64 = 64.0 * f64::EPSILON;

/// The relative half-width of the certified Krawczyk box around a Newton
/// candidate (dimensionless local-chart units).
const CERT_RADIUS: f64 = 1.0e-6;

/// The Newton convergence tolerance (dimensionless local-chart gradient units).
const NEWTON_TOL: f64 = 1.0e-14;

/// The candidate-merging distance (dimensionless local-chart units).
const CLUSTER_TOL: f64 = 1.0e-9;

/// The Newton step clamp (dimensionless local-chart units).
const STEP_CLAMP: f64 = 0.5;

/// The fixed interior seed lattice of the float Newton search (deterministic;
/// never hash iteration — determinism house rule).
const SEEDS: [(f64, f64); 9] = [
    (0.25, 0.25),
    (0.25, 0.5),
    (0.25, 0.75),
    (0.5, 0.25),
    (0.5, 0.5),
    (0.5, 0.75),
    (0.75, 0.25),
    (0.75, 0.5),
    (0.75, 0.75),
];

/// A bivariate scalar Bernstein net over the local unit chart `[0, 1]²`.
///
/// Bidegree `(p, q)`, coefficients in row-major order (`c[i][j]`, the `u`
/// index `i` outer, the `v` index `j` inner; length `(p+1)·(q+1)`):
/// `h(u, v) = Σ_i Σ_j B^p_i(u) B^q_j(v) c[i][j]`.
///
/// All arithmetic is ordinary `f64`; the certified discipline is that the SIGN
/// decisions built on the nets use outward-rounded hull predicates
/// ([`ScalarNet2::hull_excludes_zero`]), never naked float comparisons.
#[derive(Debug, Clone, PartialEq)]
pub struct ScalarNet2 {
    /// The bidegree in the first (u) parameter.
    p: usize,
    /// The bidegree in the second (v) parameter.
    q: usize,
    /// The row-major coefficient grid, length `(p+1)·(q+1)`.
    c: Vec<f64>,
}

impl ScalarNet2 {
    /// Build a scalar net from a degree and a row-major coefficient grid.
    ///
    /// Returns `None` when the grid length is not `(p+1)·(q+1)` (a ragged or
    /// mis-declared net — never silently truncated) or contains a non-finite
    /// coefficient (a non-finite coefficient cannot take part in a sound hull
    /// predicate).
    pub fn try_new(p: usize, q: usize, c: Vec<f64>) -> Option<Self> {
        if c.len() != (p + 1) * (q + 1) || !c.iter().all(|x| x.is_finite()) {
            return None;
        }
        Some(Self { p, q, c })
    }

    /// The bidegree `(p, q)`, verbatim.
    pub fn degrees(&self) -> (usize, usize) {
        (self.p, self.q)
    }

    /// The coefficient at index `(i, j)` (`i` indexes `u`, `j` indexes `v`).
    ///
    /// In-range reads are guaranteed by construction; an out-of-range read
    /// returns `0.0` (the caller's index arithmetic is bounds-correct, so the
    /// fallback is unreachable).
    pub fn coeff(&self, i: usize, j: usize) -> f64 {
        let stride = self.q + 1;
        self.c.get(i * stride + j).copied().unwrap_or(0.0)
    }

    /// The coefficient grid, verbatim (row-major, length `(p+1)·(q+1)`).
    pub fn coeffs(&self) -> &[f64] {
        &self.c
    }

    /// The `(min, max)` of the coefficient grid (the axis hull of the net's
    /// convex-hull property).
    pub fn hull(&self) -> (f64, f64) {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for x in &self.c {
            lo = lo.min(*x);
            hi = hi.max(*x);
        }
        (lo, hi)
    }

    /// Whether the outward-rounded coefficient hull excludes zero.
    ///
    /// The hull endpoints are widened by a relative pad before the comparison,
    /// so a sign decision taken on a net whose coefficients carry `f64`
    /// rounding (e.g. the quadric products) is an outward-rounded predicate,
    /// never a naked float comparison (SFC). `true` certifies that `h` cannot
    /// vanish anywhere on the cell (convex-hull property).
    pub fn hull_excludes_zero(&self) -> bool {
        let (lo, hi) = self.hull();
        let pad = |x: f64| HULL_PAD * (1.0 + x.abs());
        let (lo, hi) = (lo - pad(lo), hi + pad(hi));
        lo > 0.0 || hi < 0.0
    }

    /// The Bernstein evaluation `h(u, v)` at a point, in `f64` (the search half
    /// of an SFC pair; never evidence on its own — H-6).
    pub fn eval(&self, u: f64, v: f64) -> f64 {
        // h = Σ_j B^q_j(v) · g_j(u) with g_j(u) = Σ_i B^p_i(u) c[i][j].
        let mut acc = 0.0;
        for j in 0..=self.q {
            let col: Vec<f64> = (0..=self.p).map(|i| self.coeff(i, j)).collect();
            let gj = bern_eval_1d(&col, u);
            acc += bern_basis(self.q, j, v) * gj;
        }
        acc
    }

    /// The net of the first partial derivative `∂h/∂u`, bidegree `(p−1, q)`.
    ///
    /// A `p = 0` net is constant in `u`; its derivative is the all-zero net of
    /// bidegree `(0, q)`.
    pub fn derivative_u(&self) -> Self {
        let stride = self.q + 1;
        if self.p == 0 {
            return Self {
                p: 0,
                q: self.q,
                c: vec![0.0; self.q + 1],
            };
        }
        let scale = self.p as f64;
        let mut out = Vec::with_capacity(self.p * stride);
        for i in 0..self.p {
            for j in 0..=self.q {
                let a = self.c.get(i * stride + j).copied().unwrap_or(0.0);
                let b = self.c.get((i + 1) * stride + j).copied().unwrap_or(0.0);
                out.push(scale * (b - a));
            }
        }
        Self {
            p: self.p - 1,
            q: self.q,
            c: out,
        }
    }

    /// The net of the first partial derivative `∂h/∂v`, bidegree `(p, q−1)`.
    ///
    /// A `q = 0` net is constant in `v`; its derivative is the all-zero net of
    /// bidegree `(p, 0)`.
    pub fn derivative_v(&self) -> Self {
        let stride = self.q + 1;
        if self.q == 0 {
            return Self {
                p: self.p,
                q: 0,
                c: vec![0.0; self.p + 1],
            };
        }
        let scale = self.q as f64;
        let mut out = Vec::with_capacity((self.p + 1) * self.q);
        for i in 0..=self.p {
            for j in 0..self.q {
                let a = self.c.get(i * stride + j).copied().unwrap_or(0.0);
                let b = self.c.get(i * stride + j + 1).copied().unwrap_or(0.0);
                out.push(scale * (b - a));
            }
        }
        Self {
            p: self.p,
            q: self.q - 1,
            c: out,
        }
    }

    /// Exact degree elevation in `u` to a bidegree with first degree `p`.
    ///
    /// Returns `None` when `p < self.p` (elevation only ever raises degree).
    pub fn elevate_u(&self, p: usize) -> Option<Self> {
        if p < self.p {
            return None;
        }
        let mut out = self.clone();
        while out.p < p {
            out = out.elevate_u_step();
        }
        Some(out)
    }

    /// Exact degree elevation in `v` to a bidegree with second degree `q`.
    ///
    /// Returns `None` when `q < self.q` (elevation only ever raises degree).
    pub fn elevate_v(&self, q: usize) -> Option<Self> {
        if q < self.q {
            return None;
        }
        let mut out = self.clone();
        while out.q < q {
            out = out.elevate_v_step();
        }
        Some(out)
    }

    /// Exact degree elevation to a common bidegree `(p, q)`.
    ///
    /// Returns `None` when `p < self.p` or `q < self.q`.
    pub fn elevate(&self, p: usize, q: usize) -> Option<Self> {
        let up = self.elevate_u(p)?;
        up.elevate_v(q)
    }

    /// The interval enclosure of `h(u, v)` over an interval box inside the
    /// local unit chart.
    ///
    /// Bernstein basis functions are non-negative on `[0, 1]`, so the interval
    /// evaluation with interval parameters inside `[0, 1]` is sound (BG-ENC-001).
    fn ival_eval(&self, u: Interval, v: Interval) -> Interval {
        let mut terms = Vec::with_capacity(self.q + 1);
        for j in 0..=self.q {
            let col: Vec<Interval> = (0..=self.p)
                .map(|i| interval_at(self.coeff(i, j)))
                .collect();
            let gj = bern_ival_eval_1d(&col, u);
            terms.push(bern_ival_basis(self.q, j, v) * gj);
        }
        terms
            .into_iter()
            .fold(interval_at(0.0), |acc, term| acc + term)
    }

    /// One exact degree-elevation step in `u` (degree `p` → `p + 1`).
    fn elevate_u_step(&self) -> Self {
        let n = self.p + 1;
        let stride = self.q + 1;
        let mut out = Vec::with_capacity((n + 1) * stride);
        for i in 0..=n {
            let t = i as f64 / n as f64;
            for j in 0..=self.q {
                let prev = if i == 0 {
                    0.0
                } else {
                    self.c.get((i - 1) * stride + j).copied().unwrap_or(0.0)
                };
                let cur = self.c.get(i * stride + j).copied().unwrap_or(0.0);
                out.push(t * prev + (1.0 - t) * cur);
            }
        }
        Self {
            p: n,
            q: self.q,
            c: out,
        }
    }

    /// One exact degree-elevation step in `v` (degree `q` → `q + 1`).
    fn elevate_v_step(&self) -> Self {
        let n = self.q + 1;
        let stride = self.q + 1;
        let mut out = Vec::with_capacity((self.p + 1) * (n + 1));
        for i in 0..=self.p {
            for j in 0..=n {
                let t = j as f64 / n as f64;
                let prev = if j == 0 {
                    0.0
                } else {
                    self.c.get(i * stride + j - 1).copied().unwrap_or(0.0)
                };
                let cur = self.c.get(i * stride + j).copied().unwrap_or(0.0);
                out.push(t * prev + (1.0 - t) * cur);
            }
        }
        Self {
            p: self.p,
            q: n,
            c: out,
        }
    }
}

/// A `(p, q)` tensor control net of a spline patch on the local unit chart.
///
/// The points are model-space coordinates. Row-major like [`ScalarNet2`]:
/// `pts[i][j]` has `u`-index `i` and `v`-index `j`.
#[derive(Debug, Clone, PartialEq)]
pub struct PatchNet2 {
    /// The bidegree in the first parameter.
    p: usize,
    /// The bidegree in the second parameter.
    q: usize,
    /// The row-major control lattice, length `(p+1)·(q+1)`.
    pts: Vec<[f64; 3]>,
}

impl PatchNet2 {
    /// Build a control net from a degree and a row-major point lattice.
    ///
    /// Returns `None` on a length mismatch or a non-finite coordinate.
    pub fn try_new(p: usize, q: usize, pts: Vec<[f64; 3]>) -> Option<Self> {
        if pts.len() != (p + 1) * (q + 1) {
            return None;
        }
        if !pts.iter().all(|pt| pt.iter().all(|x| x.is_finite())) {
            return None;
        }
        Some(Self { p, q, pts })
    }

    /// The bidegree `(p, q)`, verbatim.
    pub fn degrees(&self) -> (usize, usize) {
        (self.p, self.q)
    }

    /// The control point at index `(i, j)`.
    ///
    /// In-range reads are guaranteed by construction; an out-of-range read
    /// returns the origin (the fallback is unreachable for correct callers).
    pub fn point(&self, i: usize, j: usize) -> [f64; 3] {
        self.pts
            .get(i * (self.q + 1) + j)
            .copied()
            .unwrap_or([0.0, 0.0, 0.0])
    }

    /// The control lattice, verbatim (row-major, `u` outer).
    pub fn points(&self) -> &[[f64; 3]] {
        &self.pts
    }

    /// The model-space point `S(u, v)` of the patch at a local parameter, in
    /// `f64` (a float diagnostic; never evidence on its own — H-6).
    fn eval(&self, u: f64, v: f64) -> [f64; 3] {
        let mut acc = [0.0, 0.0, 0.0];
        for i in 0..=self.p {
            for j in 0..=self.q {
                let w = bern_basis(self.p, i, u) * bern_basis(self.q, j, v);
                let pt = self.point(i, j);
                acc[0] += w * pt[0];
                acc[1] += w * pt[1];
                acc[2] += w * pt[2];
            }
        }
        acc
    }
}

/// Whether the origin lies in the axis-aligned hull of a set of 2-vectors.
///
/// This is the exclusion direction of the hull test: an axis hull that excludes
/// the origin proves the origin is not in the convex hull (the convex hull is a
/// subset of the axis hull). Used for loop detection — `0 ∉ conv{∇h
/// coefficients}` certifies no interior critical point of `h`.
pub fn origin_in_axis_hull(vectors: &[(f64, f64)]) -> bool {
    if vectors.is_empty() {
        return false;
    }
    let mut lo0 = f64::INFINITY;
    let mut hi0 = f64::NEG_INFINITY;
    let mut lo1 = f64::INFINITY;
    let mut hi1 = f64::NEG_INFINITY;
    for (a, b) in vectors {
        lo0 = lo0.min(*a);
        hi0 = hi0.max(*a);
        lo1 = lo1.min(*b);
        hi1 = hi1.max(*b);
    }
    lo0 <= 0.0 && 0.0 <= hi0 && lo1 <= 0.0 && 0.0 <= hi1
}

/// The naive (non-elevated) gradient pairing of `∂h/∂u` and `∂h/∂v`.
///
/// Pairs the vectors where BOTH derivative nets define a coefficient on the
/// shared index lattice (`i ≤ min(p_du, p_dv)`, `j ≤ min(q_du, q_dv)`). This is
/// the pairing the F-C4 elevation trap demonstrates is WRONG: on the bilinear
/// saddle it keeps only the corner 2-vector `(1, 1)` and falsely certifies
/// loop-free.
pub fn gradient_vectors_naive(du: &ScalarNet2, dv: &ScalarNet2) -> Vec<(f64, f64)> {
    let (pu, qu) = du.degrees();
    let (pv, qv) = dv.degrees();
    let pi = pu.min(pv);
    let qi = qu.min(qv);
    let mut out = Vec::with_capacity((pi + 1) * (qi + 1));
    for i in 0..=pi {
        for j in 0..=qi {
            out.push((du.coeff(i, j), dv.coeff(i, j)));
        }
    }
    out
}

/// The ELEVATED gradient pairing of `∂h/∂u` and `∂h/∂v`.
///
/// Both nets are degree-elevated to the common bidegree
/// `(max(p_du, p_dv), max(q_du, q_dv))` and paired index-wise, so
/// `∇h(x) = Σ B_I(x)(a_I, b_I)` is a convex combination and the hull test is
/// valid (the elevation-trap fix).
pub fn gradient_vectors_elevated(du: &ScalarNet2, dv: &ScalarNet2) -> Vec<(f64, f64)> {
    let (pu, qu) = du.degrees();
    let (pv, qv) = dv.degrees();
    let p = pu.max(pv);
    let q = qu.max(qv);
    let (du, dv) = match (du.elevate(p, q), dv.elevate(p, q)) {
        (Some(du), Some(dv)) => (du, dv),
        // Unreachable: elevation to `>= self` degree never fails for finite
        // degrees; kept total so the pairing is total on the public surface.
        _ => return Vec::new(),
    };
    let mut out = Vec::with_capacity((p + 1) * (q + 1));
    for i in 0..=p {
        for j in 0..=q {
            out.push((du.coeff(i, j), dv.coeff(i, j)));
        }
    }
    out
}

/// The naive hull verdict: whether the origin lies in the axis hull of the
/// NAIVE pairing (the F-C4 false-certificate side of the elevation trap).
pub fn gradient_hull_naive(du: &ScalarNet2, dv: &ScalarNet2) -> bool {
    origin_in_axis_hull(&gradient_vectors_naive(du, dv))
}

/// The ELEVATED hull verdict: whether the origin lies in the axis hull of the
/// ELEVATED pairing.
///
/// This is the verdict the stage trusts for loop detection: a cell whose
/// elevated hull CONTAINS the origin is not certified loop-free (a critical
/// point of `h` may lie on it).
pub fn gradient_hull_elevated(du: &ScalarNet2, dv: &ScalarNet2) -> bool {
    origin_in_axis_hull(&gradient_vectors_elevated(du, dv))
}

/// The exact Bernstein product of two scalar nets: bidegree
/// `(p_a + p_b, q_a + q_b)`, computed with the tensor-product convolution
/// weights `C(p_a,i₁)·C(p_b,k−i₁)/C(p_a+p_b,k)` per axis.
///
/// This is the quadric arm's "exact Bernstein product" (Theorem 4): composing a
/// quadratic implicit with a bidegree-`(p, q)` patch doubles the bidegree.
pub fn bern_product(a: &ScalarNet2, b: &ScalarNet2) -> ScalarNet2 {
    let (pa, qa) = a.degrees();
    let (pb, qb) = b.degrees();
    let p = pa + pb;
    let q = qa + qb;
    let mut c = vec![0.0; (p + 1) * (q + 1)];
    for i1 in 0..=pa {
        for i2 in 0..=pb {
            let ku = i1 + i2;
            let wu = binom(pa, i1) * binom(pb, i2) / binom(p, ku);
            for j1 in 0..=qa {
                for j2 in 0..=qb {
                    let kv = j1 + j2;
                    let wv = binom(qa, j1) * binom(qb, j2) / binom(q, kv);
                    let value = a.coeff(i1, j1) * b.coeff(i2, j2);
                    let idx = ku * (q + 1) + kv;
                    // The index is in range by construction (`ku ≤ p`,
                    // `kv ≤ q`); a defensive write keeps the loop total.
                    if let Some(slot) = c.get_mut(idx) {
                        *slot += wu * wv * value;
                    }
                }
            }
        }
    }
    // Coefficients of a product of finite nets are finite.
    ScalarNet2 { p, q, c }
}

/// A binomial coefficient `C(n, k)` in `f64` (exact for the admitted bidegrees).
fn binom(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut num = 1.0;
    let mut den = 1.0;
    for i in 0..k {
        num *= (n - i) as f64;
        den *= (i + 1) as f64;
    }
    num / den
}

/// The univariate Bernstein basis value `B^n_k(t)` in `f64`.
fn bern_basis(n: usize, k: usize, t: f64) -> f64 {
    if k > n {
        return 0.0;
    }
    let c = binom(n, k);
    c * t.powi(k as i32) * (1.0 - t).powi((n - k) as i32)
}

/// The univariate interval Bernstein basis value `B^n_k(t)` for `t` an interval
/// inside `[0, 1]`.
fn bern_ival_basis(n: usize, k: usize, t: Interval) -> Interval {
    if k > n {
        return Interval::EMPTY;
    }
    let one = interval_at(1.0);
    let t_pow = (0..k).fold(interval_at(1.0), |acc, _| acc * t);
    let one_minus_pow = (0..(n - k)).fold(interval_at(1.0), |acc, _| acc * (one - t));
    interval_at(binom(n, k)) * t_pow * one_minus_pow
}

/// The univariate Bernstein evaluation of degree-`n` coefficients at `t` (the
/// list length determines the degree).
fn bern_eval_1d(c: &[f64], t: f64) -> f64 {
    let n = c.len().saturating_sub(1);
    let mut acc = 0.0;
    for (k, ck) in c.iter().enumerate() {
        acc += bern_basis(n, k, t) * ck;
    }
    acc
}

/// The univariate interval Bernstein evaluation of degree-`n` coefficients at
/// an interval parameter inside `[0, 1]` (repeated convex combinations — sound
/// for parameters in the unit interval).
fn bern_ival_eval_1d(c: &[Interval], t: Interval) -> Interval {
    let mut level = c.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() - 1);
        for pair in level.windows(2) {
            let a = pair.first().copied().unwrap_or(Interval::EMPTY);
            let b = pair.get(1).copied().unwrap_or(Interval::EMPTY);
            next.push((interval_at(1.0) - t) * a + t * b);
        }
        level = next;
    }
    level.first().copied().unwrap_or(Interval::EMPTY)
}

/// The verdict of one reduction of an analytic × spline chart cell.
#[derive(Debug, Clone, PartialEq)]
pub enum ReductionVerdict {
    /// The `h` coefficient hull excludes zero: certified no contact on the cell.
    Empty,
    /// The elevated `∇h` hull excludes the origin: `h` has no interior critical
    /// point on the cell (loop-free). Any `h = 0` arcs meet the cell boundary;
    /// tracing them is downstream machinery.
    LoopFree,
    /// A certified isolated critical point of `h` on the cell (`∇h = 0`), found
    /// by the float Newton search and certified by the landed 2×2 Krawczyk
    /// operator.
    Critical {
        /// The certified 2-D cell (in the local unit chart) containing the
        /// unique critical point.
        cell: ((f64, f64), (f64, f64)),
        /// The local chart centre of the certified cell (float diagnostics).
        local: (f64, f64),
        /// The critical point in the spline carrier's `(u, v)` chart (float
        /// diagnostics; the certified cell is the certified statement — H-6).
        chart: (f64, f64),
        /// The model point `S(chart)` (float diagnostics; the cell is the
        /// certified statement — H-6).
        centre: [f64; 3],
        /// `|h|` at the certified centre (float diagnostics; `0` for a
        /// tangency that lies on the analytic carrier).
        residue: f64,
    },
    /// The cell resisted certification (a degenerate `h ≡ 0` cell, an exhausted
    /// search, a Krawczyk refusal). Per the consumer rule this propagates as a
    /// typed whole-operation refusal carrying the stratum-pair identity — no
    /// retry, no fallback.
    Resistant,
}

/// The output of one analytic × spline chart-cell reduction.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplicitReduction {
    /// The scalar contact net `h = g∘S` over the cell's local unit chart.
    pub h: ScalarNet2,
    /// The reduction verdict over the cell.
    pub verdict: ReductionVerdict,
}

/// Reduce one analytic × spline chart cell to its scalar contact net and
/// verdict (Theorem 4), over the unit-weight homogeneous funnel carrier.
///
/// Returns `None` when the stage does NOT apply and the caller must route the
/// pair through the old 4-D machinery:
///
/// - the analytic carrier is not a plane or a quadric (`Torus`, `Placed`, … —
///   the torus arm is a no-op route-through, spec §7);
/// - the spline carrier is not a **single-span clamped** surface over `[0, 1]²`
///   with **exact unit weights** (the D2 admission; a multi-span or rational
///   carrier stays on the old path);
/// - the chart cell is not a valid sub-box of `[0, 1]²`.
///
/// `box_uv` is the `(u, v)` window of the funnel cell on the spline chart (the
/// funnel's `(s, t)` analytic window plays no role in the scalar reduction —
/// `g` describes the whole canonical carrier).
pub fn reduce_analytic_spline(
    analytic: &CanonicalSurface,
    spline: &BSplineSurface<Vector4>,
    box_uv: ((f64, f64), (f64, f64)),
    budget: &mut Budget,
) -> Option<ImplicitReduction> {
    let net = unit_net_homogeneous(spline)?;
    let local = restrict_patch(&net, box_uv)?;
    let h = analytic_h(analytic, &local)?;
    let verdict = reduce_verdict(&h, &net, box_uv, budget);
    Some(ImplicitReduction { h, verdict })
}

/// The funnel pre-screen: whether the reduction certifies EMPTY on the cell.
///
/// Returns `None` when the stage does not apply (route through the old
/// machinery), `Some(true)` when the cell certifies no contact (a certified
/// prune), and `Some(false)` when the reduction does not close the cell as
/// empty (route through — the old machinery certifies the crossing).
///
/// This is the funnel's sound, non-spending use of the stage: exclusion needs
/// no budget, and a cell the exclusion cannot close is never downgraded.
pub fn screen_empty(
    analytic: &CanonicalSurface,
    spline: &BSplineSurface<Vector4>,
    box_uv: ((f64, f64), (f64, f64)),
) -> Option<bool> {
    let net = unit_net_homogeneous(spline)?;
    let local = restrict_patch(&net, box_uv)?;
    let h = analytic_h(analytic, &local)?;
    Some(h.hull_excludes_zero())
}

/// The `h = g∘S` net of one recognized analytic carrier over a restricted
/// patch (the bidegree rule: `(p, q)` for planes, `(2p, 2q)` for quadrics).
fn analytic_h(analytic: &CanonicalSurface, local: &PatchNet2) -> Option<ScalarNet2> {
    match analytic {
        CanonicalSurface::Plane(plane) => Some(plane_h(plane, local)),
        CanonicalSurface::Sphere(sphere) => Some(sphere_h(sphere, local)),
        CanonicalSurface::Cylinder(cylinder) => Some(cylinder_h(cylinder, local)),
        CanonicalSurface::Cone(cone) => Some(cone_h(cone, local)),
        CanonicalSurface::Torus(_) | CanonicalSurface::Placed(_) => None,
    }
}

/// The `h` net of a plane × patch: coefficients = signed control-point
/// distances `n·(P − o)` (Theorem 4, bidegree `(p, q)` preserved exactly).
pub fn plane_h(plane: &Plane, net: &PatchNet2) -> ScalarNet2 {
    let n = plane.normal();
    let o = plane.origin();
    let (p, q) = net.degrees();
    let c = net
        .points()
        .iter()
        .map(|pt| {
            let [x, y, z] = *pt;
            n.x * (x - o.x) + n.y * (y - o.y) + n.z * (z - o.z)
        })
        .collect();
    // The signed distances of a finite lattice against an affine plane are
    // finite by construction.
    ScalarNet2 { p, q, c }
}

/// The `h` net of a sphere × patch: `h = |S − c|² − r²` at bidegree `(2p, 2q)`.
pub fn sphere_h(sphere: &Sphere, net: &PatchNet2) -> ScalarNet2 {
    let c = sphere.center();
    let r = sphere.radius();
    quadric_squares(net, [c.x, c.y, c.z], 1.0, -r * r)
}

/// The `h` net of a cylinder × patch: `h = (x−cx)² + (y−cy)² − r²`.
pub fn cylinder_h(cylinder: &Cylinder, net: &PatchNet2) -> ScalarNet2 {
    let c = cylinder.center();
    let r = cylinder.radius();
    quadric_squares(net, [c.x, c.y, c.z], 0.0, -r * r)
}

/// The `h` net of a cone × patch: `h = (x−a)² + (y−a)² − (t·(z−a))²` with
/// `t = tan(half_angle)`.
pub fn cone_h(cone: &Cone, net: &PatchNet2) -> ScalarNet2 {
    let a = cone.apex();
    let t = cone.half_angle().tan();
    quadric_squares(net, [a.x, a.y, a.z], -t * t, 0.0)
}

/// The shared quadric composition: `h = (x')² + (y')² + z_coeff·(z')² + const`
/// with `(x', y', z')` the centered coordinates — each square is the EXACT
/// Bernstein product (bidegree `(2p, 2q)`).
fn quadric_squares(net: &PatchNet2, center: [f64; 3], z_coeff: f64, constant: f64) -> ScalarNet2 {
    let (p, q) = net.degrees();
    let x = coord_net(net, 0).add_constant(-center[0]);
    let y = coord_net(net, 1).add_constant(-center[1]);
    let z = coord_net(net, 2).add_constant(-center[2]);
    let xx = bern_product(&x, &x);
    let yy = bern_product(&y, &y);
    let zz = if z_coeff == 0.0 {
        ScalarNet2 {
            p: 2 * p,
            q: 2 * q,
            c: vec![0.0; (2 * p + 1) * (2 * q + 1)],
        }
    } else {
        bern_product(&z, &z).scale(z_coeff)
    };
    // The three squares share the product bidegree `(2p, 2q)`, so the sum and
    // the constant lift are exact coefficient operations.
    let sum = sum_nets(&xx, &yy);
    let sum = sum_nets(&sum, &zz);
    sum.add_constant(constant)
}

/// The coefficient-wise sum of two equal-bidegree nets (the caller guarantees
/// equal bidegrees — the elevation-trap discipline).
fn sum_nets(a: &ScalarNet2, b: &ScalarNet2) -> ScalarNet2 {
    let c = a.c.iter().zip(b.c.iter()).map(|(x, y)| x + y).collect();
    ScalarNet2 { p: a.p, q: a.q, c }
}

/// The coordinate scalar net of a control lattice along one axis.
fn coord_net(net: &PatchNet2, axis: usize) -> ScalarNet2 {
    let (p, q) = net.degrees();
    let c = net.points().iter().map(|pt| coord(pt, axis)).collect();
    ScalarNet2 { p, q, c }
}

/// One coordinate of a model-space point (0, 1, or 2).
fn coord(pt: &[f64; 3], axis: usize) -> f64 {
    match axis {
        0 => pt[0],
        1 => pt[1],
        _ => pt[2],
    }
}

/// The verdict of one reduction over an `h` net on the local chart (stage
/// semantics steps 1–3).
fn reduce_verdict(
    h: &ScalarNet2,
    full_net: &PatchNet2,
    box_uv: ((f64, f64), (f64, f64)),
    budget: &mut Budget,
) -> ReductionVerdict {
    // Step 1: exclusion.
    if h.hull_excludes_zero() {
        return ReductionVerdict::Empty;
    }
    // Step 2: loop detection on the ELEVATED gradient hull.
    let du = h.derivative_u();
    let dv = h.derivative_v();
    if !gradient_hull_elevated(&du, &dv) {
        return ReductionVerdict::LoopFree;
    }
    // Step 3: critical-point certification via (float Newton, 2×2 Krawczyk).
    // The search half runs on a scratch budget so a failed search never burns
    // the caller's ledger (the caller routes the cell through the old machinery
    // instead — the refusal records the caller's intact budget).
    let mut scratch = *budget;
    let hess_uu = du.derivative_u();
    let hess_uv = du.derivative_v();
    let hess_vv = dv.derivative_v();
    let mut candidates: Vec<(f64, f64)> = Vec::new();
    for seed in SEEDS {
        let Some(c) = newton_critical(&du, &dv, &hess_uu, &hess_uv, &hess_vv, seed, &mut scratch)
        else {
            continue;
        };
        if !merge_candidate(&mut candidates, c) {
            continue;
        }
    }
    for (u, v) in candidates {
        let r = CERT_RADIUS;
        let u_lo = (u - r).max(0.0);
        let u_hi = (u + r).min(1.0);
        let v_lo = (v - r).max(0.0);
        let v_hi = (v + r).min(1.0);
        if u_lo >= u_hi || v_lo >= v_hi {
            continue;
        }
        let system = CriticalSystem2 {
            du: du.clone(),
            dv: dv.clone(),
            hess_uu: hess_uu.clone(),
            hess_uv: hess_uv.clone(),
            hess_vv: hess_vv.clone(),
        };
        let start = [
            Interval::try_from((u_lo, u_hi)).unwrap_or(Interval::EMPTY),
            Interval::try_from((v_lo, v_hi)).unwrap_or(Interval::EMPTY),
        ];
        match krawczyk(&system, &start, budget) {
            Ok(certified) if certified.value == KrawczykProof::Unique => {
                // Map the local centre back to the carrier chart and the model.
                let ((u0, u1), (v0, v1)) = box_uv;
                let chart = (u0 + (u1 - u0) * u, v0 + (v1 - v0) * v);
                let centre = full_net.eval(chart.0, chart.1);
                let residue = h.eval(u, v).abs();
                return ReductionVerdict::Critical {
                    cell: ((u_lo, u_hi), (v_lo, v_hi)),
                    local: (u, v),
                    chart,
                    centre,
                    residue,
                };
            }
            // NoRoot / a typed refusal: this candidate did not certify; the
            // cell is not closed by it.
            _ => continue,
        }
    }
    ReductionVerdict::Resistant
}

/// Merge a Newton candidate into the deduplicated candidate list.
///
/// Returns `true` when `c` is a NEW candidate (no recorded candidate lies
/// within [`CLUSTER_TOL`]), `false` when it merged into an existing one.
fn merge_candidate(candidates: &mut Vec<(f64, f64)>, c: (f64, f64)) -> bool {
    for (u, v) in candidates.iter() {
        if (u - c.0).abs() <= CLUSTER_TOL && (v - c.1).abs() <= CLUSTER_TOL {
            return false;
        }
    }
    candidates.push(c);
    true
}

/// The float Newton search half of the critical-point SFC pair.
///
/// Solves `∇h = 0` by damped Newton from a seed inside the local cell,
/// returning the converged candidate, or `None` when the iteration leaves the
/// cell, stalls at a singular Hessian, or exhausts the caller's Newton budget.
fn newton_critical(
    du: &ScalarNet2,
    dv: &ScalarNet2,
    h_uu: &ScalarNet2,
    h_uv: &ScalarNet2,
    h_vv: &ScalarNet2,
    seed: (f64, f64),
    budget: &mut Budget,
) -> Option<(f64, f64)> {
    let mut u = seed.0;
    let mut v = seed.1;
    let max_iters = budget.newton.min(64);
    for _ in 0..max_iters {
        let f0 = du.eval(u, v);
        let f1 = dv.eval(u, v);
        if f0.abs() <= NEWTON_TOL && f1.abs() <= NEWTON_TOL {
            return Some((u, v));
        }
        let a = h_uu.eval(u, v);
        let b = h_uv.eval(u, v);
        let c = h_uv.eval(u, v);
        let d = h_vv.eval(u, v);
        let det = a * d - b * c;
        if det.abs() <= f64::EPSILON {
            return None;
        }
        let su = ((-f0) * d + b * f1) / det;
        let sv = (c * f0 - a * f1) / det;
        u += su.clamp(-STEP_CLAMP, STEP_CLAMP);
        v += sv.clamp(-STEP_CLAMP, STEP_CLAMP);
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
            return None;
        }
        if budget.spend_newton(1).is_err() {
            return None;
        }
    }
    None
}

/// The 2×2 square system `∇h = 0` for the landed Krawczyk operator
/// (instantiated, never extended).
struct CriticalSystem2 {
    /// `∂h/∂u`.
    du: ScalarNet2,
    /// `∂h/∂v`.
    dv: ScalarNet2,
    /// `∂²h/∂u²`.
    hess_uu: ScalarNet2,
    /// `∂²h/∂u∂v`.
    hess_uv: ScalarNet2,
    /// `∂²h/∂v²`.
    hess_vv: ScalarNet2,
}

impl KrawczykSystem<2> for CriticalSystem2 {
    fn f_point(&self, x: &[f64; 2]) -> [Interval; 2] {
        [
            interval_at(self.du.eval(x[0], x[1])),
            interval_at(self.dv.eval(x[0], x[1])),
        ]
    }

    fn jacobian(&self, b: &[Interval; 2]) -> [[Interval; 2]; 2] {
        let h_uu = self.hess_uu.ival_eval(b[0], b[1]);
        let h_uv = self.hess_uv.ival_eval(b[0], b[1]);
        let h_vv = self.hess_vv.ival_eval(b[0], b[1]);
        [[h_uu, h_uv], [h_uv, h_vv]]
    }

    fn preconditioner(&self, x: &[f64; 2]) -> Option<[[f64; 2]; 2]> {
        let a = self.hess_uu.eval(x[0], x[1]);
        let b = self.hess_uv.eval(x[0], x[1]);
        let d = self.hess_vv.eval(x[0], x[1]);
        let det = a * d - b * b;
        if det.abs() <= f64::EPSILON {
            None
        } else {
            Some([[d / det, -b / det], [-b / det, a / det]])
        }
    }
}

/// Extract the exact unit-weight homogeneous control lattice of a single-span
/// clamped `[0, 1]²` `BSplineSurface<Vector4>` as a model-space `PatchNet2`.
///
/// Returns `None` for a multi-span carrier, a non-`[0, 1]` domain, or a
/// non-unit weight (the D2 admission gates).
fn unit_net_homogeneous(surface: &BSplineSurface<Vector4>) -> Option<PatchNet2> {
    let (p, q) = surface.degrees();
    if !is_unit_bezier_knot(surface.uknot_vec().as_slice(), p)
        || !is_unit_bezier_knot(surface.vknot_vec().as_slice(), q)
    {
        return None;
    }
    let mut pts = Vec::with_capacity((p + 1) * (q + 1));
    for i in 0..=p {
        for j in 0..=q {
            let pt = surface.control_point(i, j);
            if pt.w != 1.0 {
                return None;
            }
            pts.push([pt.x, pt.y, pt.z]);
        }
    }
    PatchNet2::try_new(p, q, pts)
}

/// Whether a knot vector is the `[0, 1]`-domain single-span clamped form of a
/// degree-`d` Bézier patch: `d + 1` copies of `0.0` then `d + 1` copies of
/// `1.0`.
fn is_unit_bezier_knot(knots: &[f64], degree: usize) -> bool {
    if knots.len() != 2 * (degree + 1) {
        return false;
    }
    for (k, x) in knots.iter().enumerate() {
        let expect = if k <= degree { 0.0 } else { 1.0 };
        if *x != expect {
            return false;
        }
    }
    true
}

/// Restrict a unit-chart `PatchNet2` to a sub-box
/// `([u_lo, u_hi], [v_lo, v_hi])` inside `[0, 1]²`, returning the sub-patch net
/// over the sub-box in its own normalized local chart.
///
/// Exact two-pass de Casteljau subdivision (the tensor-product restriction):
/// the returned control lattice defines the same surface on the sub-box, with
/// the local parameters affinely mapping onto `[u_lo, u_hi]` / `[v_lo, v_hi]`.
/// `None` when the box is degenerate or not inside `[0, 1]²`.
pub fn restrict_patch(net: &PatchNet2, box_uv: ((f64, f64), (f64, f64))) -> Option<PatchNet2> {
    let ((u_lo, u_hi), (v_lo, v_hi)) = box_uv;
    if !(0.0..=1.0).contains(&u_lo)
        || !(0.0..=1.0).contains(&u_hi)
        || !(0.0..=1.0).contains(&v_lo)
        || !(0.0..=1.0).contains(&v_hi)
        || u_lo >= u_hi
        || v_lo >= v_hi
    {
        return None;
    }
    if u_lo == 0.0 && u_hi == 1.0 && v_lo == 0.0 && v_hi == 1.0 {
        return Some(net.clone());
    }
    let (p, q) = net.degrees();
    let pts = tensor_restrict(net, u_lo, u_hi, v_lo, v_hi)?;
    PatchNet2::try_new(p, q, pts)
}

/// The two-pass tensor restriction: restrict each fixed-`v` column along `u`
/// (pass 1), then each resulting fixed-`u` row along `v` (pass 2).
fn tensor_restrict(
    net: &PatchNet2,
    u_lo: f64,
    u_hi: f64,
    v_lo: f64,
    v_hi: f64,
) -> Option<Vec<[f64; 3]>> {
    let (p, q) = net.degrees();
    // Pass 1: restrict each column (fixed v index) along u.
    let mut cols: Vec<Vec<[f64; 3]>> = Vec::with_capacity(q + 1);
    for j in 0..=q {
        let column: Vec<[f64; 3]> = (0..=p).map(|i| net.point(i, j)).collect();
        cols.push(bezier_sub_curve(&column, u_lo, u_hi)?);
    }
    // Re-arrange the u-restricted columns into rows (fixed u index), then
    // restrict each row along v.
    let mut rows: Vec<Vec<[f64; 3]>> = vec![Vec::with_capacity(q + 1); p + 1];
    for col in &cols {
        for (i, pt) in col.iter().enumerate() {
            if let Some(row) = rows.get_mut(i) {
                row.push(*pt);
            }
        }
    }
    let mut out = Vec::with_capacity((p + 1) * (q + 1));
    for row in &rows {
        let sub = bezier_sub_curve(row, v_lo, v_hi)?;
        out.extend(sub.iter().copied());
    }
    Some(out)
}

/// The exact sub-curve control polygon of a univariate Bézier over `[a, b]`
/// within `[0, 1]`.
///
/// `points` is the degree-`n` control polygon over `[0, 1]`; the result is the
/// control polygon of the same curve restricted to `[a, b]`, expressed with the
/// local parameter `s` where `t = a + (b − a)·s`. `None` when `[a, b]` is
/// degenerate.
fn bezier_sub_curve(points: &[[f64; 3]], a: f64, b: f64) -> Option<Vec<[f64; 3]>> {
    if !(0.0..=1.0).contains(&a) || !(0.0..=1.0).contains(&b) || a >= b {
        return None;
    }
    if a == 0.0 && b == 1.0 {
        return Some(points.to_vec());
    }
    // Split at a, keep the right segment (covers [a, 1] in its own unit chart),
    // then split that segment at the relative position of b.
    let (_, right) = split_bezier(points, a);
    let denom = 1.0 - a;
    let rel = if denom.abs() < f64::EPSILON {
        1.0
    } else {
        (b - a) / denom
    };
    let (left, _) = split_bezier(&right, rel.clamp(0.0, 1.0));
    Some(left)
}

/// Split a degree-`n` Bézier polygon at parameter `t` (de Casteljau), returning
/// the left and right sub-polygons (each normalized to the unit chart of its
/// own sub-segment).
fn split_bezier(points: &[[f64; 3]], t: f64) -> (Vec<[f64; 3]>, Vec<[f64; 3]>) {
    let mut level = points.to_vec();
    let mut left = Vec::with_capacity(points.len());
    let mut right = Vec::with_capacity(points.len());
    loop {
        if let Some(first) = level.first().copied() {
            left.push(first);
        }
        if let Some(last) = level.last().copied() {
            right.push(last);
        }
        if level.len() == 1 {
            break;
        }
        let mut next = Vec::with_capacity(level.len() - 1);
        for pair in level.windows(2) {
            let a = pair.first().copied().unwrap_or([0.0, 0.0, 0.0]);
            let b = pair.get(1).copied().unwrap_or([0.0, 0.0, 0.0]);
            next.push(lerp3(a, b, t));
        }
        level = next;
    }
    right.reverse();
    (left, right)
}

/// The convex combination `(1−t)·a + t·b` of two model-space points.
fn lerp3(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [
        (1.0 - t) * a[0] + t * b[0],
        (1.0 - t) * a[1] + t * b[1],
        (1.0 - t) * a[2] + t * b[2],
    ]
}

impl ScalarNet2 {
    /// Adds a constant to the net (`h + k`): the constant rides every
    /// coefficient, since `Σ B_I = 1` on the unit chart.
    fn add_constant(&self, k: f64) -> Self {
        let c = self.c.iter().map(|x| x + k).collect();
        Self {
            p: self.p,
            q: self.q,
            c,
        }
    }

    /// Scales every coefficient by `k` (`k·h`).
    fn scale(&self, k: f64) -> Self {
        let c = self.c.iter().map(|x| x * k).collect();
        Self {
            p: self.p,
            q: self.q,
            c,
        }
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. Unit-test assertions on hand-built dyadic witnesses are
// not such a path; the unwraps below cannot fire for the values constructed.
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use truck_base::cgmath64::Point3;
    use truck_geometry::nurbs::KnotVec;
    use truck_geometry::specifieds::Plane;

    /// The F-C4 plane × bicubic surface `z = 12[(u − ½)² + (v − ½)²]` as a unit
    /// homogeneous `BSplineSurface<Vector4>` (the integer `z` grid is the
    /// signed control-point distances of the plane `z = 0`).
    fn fc4_bicubic() -> BSplineSurface<Vector4> {
        // The graph net P_ij = (i, j, h_ij) with h_ij = a_i + a_j and
        // a = [3, -1, -1, 3] (F-C4 plane × bicubic): integer, plane z = 0.
        let net = [
            [[0, 0, 6], [0, 1, 2], [0, 2, 2], [0, 3, 6]],
            [[1, 0, 2], [1, 1, -2], [1, 2, -2], [1, 3, 2]],
            [[2, 0, 2], [2, 1, -2], [2, 2, -2], [2, 3, 2]],
            [[3, 0, 6], [3, 1, 2], [3, 2, 2], [3, 3, 6]],
        ];
        let knot = KnotVec::bezier_knot(3);
        let ctrl = net
            .map(|row| {
                row.map(|[x, y, z]| Vector4::new(x as f64, y as f64, z as f64, 1.0))
                    .to_vec()
            })
            .to_vec();
        BSplineSurface::new((knot.clone(), knot), ctrl)
    }

    /// The F-C4 elevation-trap saddle surface `z = u + v − 2uv` as a unit
    /// homogeneous `BSplineSurface<Vector4>` (corner values `[[0, 1], [1, 0]]`).
    fn trap_saddle() -> BSplineSurface<Vector4> {
        let knot = KnotVec::bezier_knot(1);
        let ctrl = vec![
            vec![
                Vector4::new(0.0, 0.0, 0.0, 1.0),
                Vector4::new(0.0, 1.0, 1.0, 1.0),
            ],
            vec![
                Vector4::new(1.0, 0.0, 1.0, 1.0),
                Vector4::new(1.0, 1.0, 0.0, 1.0),
            ],
        ];
        BSplineSurface::new((knot.clone(), knot), ctrl)
    }

    /// The plane `z = 0` over the unit window.
    fn z_plane() -> CanonicalSurface {
        CanonicalSurface::Plane(Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ))
    }

    /// Whether the scalar is within an absolute slack of `want` (both are
    /// unit-scale chart or coefficient values; the slack is dimensionless).
    fn near(a: f64, want: f64) -> bool {
        let slack = 1.0e-9; // H-3: dimensionless coefficient/chart slack, not a length
        (a - want).abs() <= slack
    }

    #[test]
    fn implicit_stage_reduces_plane_spline_theorem4() {
        // F-C4's plane × bicubic ground truth: the stage produces the known h
        // (coefficients = signed control-point distances) and finds the known
        // critical point through the landed Krawczyk; the result matches the
        // recorded ground truth.
        let spline = fc4_bicubic();
        let plane = z_plane();
        let recorded_h = [
            [6.0, 2.0, 2.0, 6.0],
            [2.0, -2.0, -2.0, 2.0],
            [2.0, -2.0, -2.0, 2.0],
            [6.0, 2.0, 2.0, 6.0],
        ];
        let mut budget = Budget::new(4096, 4096, 64);
        let reduction =
            reduce_analytic_spline(&plane, &spline, ((0.0, 1.0), (0.0, 1.0)), &mut budget)
                .expect("a plane × single-span bicubic reduces");
        let h = &reduction.h;
        assert_eq!(
            h.degrees(),
            (3, 3),
            "the plane preserves the patch bidegree"
        );
        for (i, row) in recorded_h.iter().enumerate() {
            for (j, want) in row.iter().enumerate() {
                assert!(
                    near(h.coeff(i, j), *want),
                    "h[{i}][{j}] = {} != recorded {}",
                    h.coeff(i, j),
                    want
                );
            }
        }
        // The known critical point (½, ½) is found through the landed Krawczyk.
        let ReductionVerdict::Critical {
            cell,
            local,
            chart,
            residue,
            ..
        } = reduction.verdict
        else {
            panic!(
                "the F-C4 bowl certifies a critical point, got {:?}",
                reduction.verdict
            );
        };
        assert!(
            near(local.0, 0.5) && near(local.1, 0.5),
            "local centre {local:?}"
        );
        assert!(
            near(chart.0, 0.5) && near(chart.1, 0.5),
            "chart centre {chart:?}"
        );
        let ((u0, u1), (v0, v1)) = cell;
        assert!(
            u0 <= 0.5 && 0.5 <= u1 && v0 <= 0.5 && 0.5 <= v1,
            "the certified cell {cell:?} contains the recorded critical point (½, ½)"
        );
        assert!(
            near(residue, 0.0),
            "the critical point lies on the plane (|h| = {residue})"
        );
        assert!(
            h.hull().0 < 0.0 && h.hull().1 > 0.0,
            "the bowl's h hull straddles zero (a genuine contact scenario)"
        );
    }

    #[test]
    fn elevation_trap_twin_disagrees_fc4() {
        // The F-C4 elevation-trap twin: the non-elevated ∇h hull pairing
        // DISAGREES with the elevated one on the constructed saddle; the
        // elevated verdict (origin in hull → a critical point lies on the cell)
        // is the correct one.
        let spline = trap_saddle();
        let plane = z_plane();
        let mut budget = Budget::new(4096, 4096, 64);
        let reduction =
            reduce_analytic_spline(&plane, &spline, ((0.0, 1.0), (0.0, 1.0)), &mut budget)
                .expect("a plane × single-span bilinear reduces");
        let h = &reduction.h;
        assert_eq!(h.degrees(), (1, 1), "the bilinear trap keeps its bidegree");
        // The derived h matches the twin's integer corner values.
        assert!(near(h.coeff(0, 0), 0.0), "h(0,0)");
        assert!(near(h.coeff(1, 0), 1.0), "h(1,0)");
        assert!(near(h.coeff(0, 1), 1.0), "h(0,1)");
        assert!(near(h.coeff(1, 1), 0.0), "h(1,1)");

        let du = h.derivative_u();
        let dv = h.derivative_v();
        assert_eq!(du.degrees(), (0, 1), "∂h/∂u is bidegree (p−1, q)");
        assert_eq!(dv.degrees(), (1, 0), "∂h/∂v is bidegree (p, q−1)");
        // The recorded derivative arrays (1, −1).
        assert!(
            near(du.coeff(0, 0), 1.0) && near(du.coeff(0, 1), -1.0),
            "du = (1, −1)"
        );
        assert!(
            near(dv.coeff(0, 0), 1.0) && near(dv.coeff(1, 0), -1.0),
            "dv = (1, −1)"
        );

        // The naive pairing keeps only the overlap 2-vector (1, 1): hull
        // excludes the origin — a FALSE loop-free certificate.
        assert!(
            !gradient_hull_naive(&du, &dv),
            "the naive pairing falsely certifies loop-free on the saddle"
        );
        // The elevated pairing spans [−1, 1]²: origin contained — the correct
        // verdict (∇h(½, ½) = 0).
        assert!(
            gradient_hull_elevated(&du, &dv),
            "the elevated pairing contains the origin (critical point on cell)"
        );
    }

    #[test]
    fn plane_reduction_exclusion_prunes_sign_definite_cells() {
        // A plane strictly above the patch: the scalar hull excludes zero and
        // the reduction certifies empty without any solver call.
        let spline = fc4_bicubic();
        // Plane z = 10: every control point has z ≤ 6 < 10.
        let high_plane = CanonicalSurface::Plane(Plane::new(
            Point3::new(0.0, 0.0, 10.0),
            Point3::new(1.0, 0.0, 10.0),
            Point3::new(0.0, 1.0, 10.0),
        ));
        let mut budget = Budget::new(4096, 4096, 64);
        let reduction =
            reduce_analytic_spline(&high_plane, &spline, ((0.0, 1.0), (0.0, 1.0)), &mut budget)
                .expect("a plane × single-span bicubic reduces");
        assert!(
            matches!(reduction.verdict, ReductionVerdict::Empty),
            "a sign-definite cell certifies empty, got {:?}",
            reduction.verdict
        );
    }

    #[test]
    fn restriction_preserves_the_patch_on_a_sub_box() {
        // The de Casteljau restriction is exact: the sub-patch net evaluates to
        // the same model points as the full patch at the mapped parameters.
        let spline = fc4_bicubic();
        let net = unit_net_homogeneous(&spline).expect("the F-C4 bicubic is reducible");
        let box_uv = ((0.25, 0.75), (0.2, 0.8));
        let sub = restrict_patch(&net, box_uv).expect("a valid sub-box restricts");
        let ((u0, u1), (v0, v1)) = box_uv;
        for s in [0.0, 0.25, 0.5, 0.75, 1.0] {
            for t in [0.0, 0.5, 1.0] {
                let (u, v) = (u0 + (u1 - u0) * s, v0 + (v1 - v0) * t);
                let full = net.eval(u, v);
                let part = sub.eval(s, t);
                let [fx, fy, fz] = full;
                let [px, py, pz] = part;
                for (label, f, p) in [("x", fx, px), ("y", fy, py), ("z", fz, pz)] {
                    assert!(
                        (f - p).abs() <= 1.0e-9,
                        "axis {label} at (s,t)=({s},{t}): full {full:?} vs restricted {part:?}"
                    );
                }
            }
        }
    }
}
