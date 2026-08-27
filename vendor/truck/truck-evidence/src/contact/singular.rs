//! BG-SOL-S7-SING-CLASSIFY — the singular-event stage.
//!
//! Post-CHART, `BranchCover::singular_boxes` holds UNSUBDIVIDED domains where
//! all three cross-gradient minors merely contain zero: chart-artifact boxes
//! are already recovered, but a singular domain may still contain (a) regular
//! crossings whose tangent directions vary too much for a domain-level chart,
//! (b) isolated tangency points, (c) gradient-parallel saddle points where the
//! contact locus crosses itself, (d) carrier-degenerate contact points (cone
//! apex on the other carrier), or any mix. The dispatcher used to refuse every
//! such pair with `ContactReductionDeferred`. This stage classifies the
//! singular cells: it refines each cell (recovering regular crossings inside
//! broad singular domains into the regular cover), then classifies every
//! resolution-floor residue leaf, then lets the dispatcher emit
//! `Point0`/`Tangency` records for PROVEN isolated tangencies and defer
//! everything else with named reasons.
//!
//! Every certificate step is interval-based (BG-ENC-001 soundness): the
//! degenerate pass (an exact on-surface degenerate point of one carrier lying
//! on the other carrier's zero set), the Lagrange critical-point system
//! `[f1, grad(f2) + lam·grad(f1)]` certified by the 4-D Krawczyk operator
//! (BG-NUM-003) over a sound multiplier envelope, and the restricted-Hessian
//! inertia test that separates isolated tangencies (definite inertia) from
//! gradient-parallel saddle crossings (indefinite inertia) and defers the rest.
//!
//! House rules H-1..H-8 apply.
//!
//! NOTE ON DECISION 3(b) (recorded in RESULT.json deviations): the packet says
//! to run `krawczyk::<4>` on `[leaf.x, leaf.y, leaf.z, lam_box]`. The
//! refinement bisects on a grid whose points include the packet's own dyadic
//! tangency witnesses, so the certified root lands exactly ON the residue
//! leaf's boundary (or on a collapsed zero-width axis of the certified AABB),
//! where the Krawczyk strict-interior rule cannot certify (measured:
//! `NumericallyUnresolved`, unsplittable, ~200 subdivisions). This module runs
//! the operator on the leaf widened by `tau` on each side, so the certified
//! root is strictly interior while the box stays tau-scale tight; the root is
//! still verified to satisfy the system, and spurious roots are caught by the
//! contact sanity check and the inertia. A non-exhaustion Krawczyk refusal
//! defers the leaf rather than propagating (the packet's "refusal: budget
//! exhaustion propagates" names exhaustion only).
//!
//! NOTE ON DECISION 3(c) (recorded in RESULT.json deviations): the packet's
//! decision-3 text chooses the tangent-basis axis index `a` as the index of
//! the LARGEST `|n_i|`; that choice is degenerate on the packet's own
//! witnesses (their normals are exactly axis-aligned, so `n × e_a = 0` and no
//! frame exists). This module uses the index of the SMALLEST `|n_i|` (ties
//! toward the lowest index), which produces the non-degenerate frames the
//! packet's decision 7 machine-checks (`diag(4, 2)` and `diag(-2, 2)`) and is
//! the standard robust orthonormal-frame construction. Everything else follows
//! the packet.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::enclosure::{interval_at, Box3};
use crate::num::krawczyk::{krawczyk, KrawczykProof, KrawczykSystem};
use inari::Interval;
use truck_base::cgmath64::{InnerSpace, Point3, Vector3};
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Outcome, PropMap, Refusal,
    UnresolvedWitness,
};

use super::gff::{self, BranchCover};
use super::implicit::ImplicitField;

/// The certified classification of a set of singular cells.
#[derive(Clone, Debug)]
pub struct SingularReport {
    /// Crossings certified inside chartable children of the singular cells,
    /// accumulated in discovery order (the regular cover's own lists). Points
    /// are certified cross-sections of one or more Arc1 components; sub-tau
    /// structure is the resolution contract's business, exactly as for regular
    /// cover points.
    pub regular: BranchCover,
    /// Certified isolated tangency points: unique Lagrange root with definite
    /// restricted Hessian; the contact set in a neighborhood of each point is
    /// exactly that point. The isolation claim is about a neighborhood of the
    /// certified point, not about the whole leaf.
    pub tangencies: Vec<Point3>,
    /// Certified gradient-parallel saddle points: unique Lagrange root with
    /// INdefinite restricted Hessian. The contact locus self-crosses there;
    /// NOT isolated; deferred with the point recorded.
    pub tangential_crossings: Vec<Point3>,
    /// Certified carrier-degenerate contact points (e.g. cone apex on the
    /// other carrier). Local branch topology unclassified; deferred.
    pub degenerate: Vec<Point3>,
    /// Resolution-floor leaves that certified nothing. Dimension unknown.
    pub residue: Vec<Box3>,
}

/// Classify the singular cells of a validated FF cover.
///
/// Refinement (decision 2): a LIFO worklist of cells in the given order; each
/// pop runs `gff::cover_branch` (recovering the regular crossings hiding
/// inside broad singular domains into the regular cover), then every chartless
/// singular box is bisected on its widest axis (ties toward the lowest axis
/// index, convex-combination midpoint, one subdivision per bisection) down to
/// the resolution floor `tau`. Classification (decision 3), per
/// resolution-floor residue leaf in discovery order: (a) the degenerate pass,
/// (b) the Lagrange system under the sound multiplier envelope, (c) the
/// restricted-Hessian inertia at the certified root. A certified root that
/// lands exactly on a residue leaf's bisection-grid boundary is certified by
/// running the Krawczyk operator on the leaf widened by `tau` on each side
/// (a resolution-floor treatment: the certified root is still verified to
/// satisfy the system, and any spurious root is caught by the contact sanity
/// check and the inertia). The caller owns `budget`: it is captured once at
/// entry, every internal `cover_branch`/`krawczyk` refusal carries the total
/// spend (`initial − remaining`), and only genuine budget exhaustion
/// propagates as `NumericallyUnresolved` with
/// `UnresolvedWitness::KrawczykIndeterminate`; a non-exhaustion Krawczyk
/// refusal (an unsplittable leaf) defers the leaf to `residue`.
pub fn singular_events(
    f1: &impl ImplicitField,
    f2: &impl ImplicitField,
    cells: &[Box3],
    tau: f64,
    budget: &mut Budget,
) -> Outcome<SingularReport> {
    let initial = *budget;
    let d1: &dyn ImplicitField = f1;
    let d2: &dyn ImplicitField = f2;
    let mut report = SingularReport {
        regular: BranchCover::default(),
        tangencies: Vec::new(),
        tangential_crossings: Vec::new(),
        degenerate: Vec::new(),
        residue: Vec::new(),
    };
    let deg1 = d1.degenerate_points();
    let deg2 = d2.degenerate_points();
    let ctx = ClassifyCtx {
        f1: d1,
        f2: d2,
        deg1: &deg1,
        deg2: &deg2,
        initial,
    };
    // Refinement (decision 2). The stack is LIFO; cells are pushed in reverse
    // so they pop in the given order. The sound multiplier envelope is
    // computed once per singular CELL (decision 3(b) machine-checks use the
    // whole-cell envelope; a resolution-floor leaf's own envelope is so tight
    // that the certified multiplier sits a few ulps from the boundary and the
    // Krawczyk strict-interior rule cannot certify on the lam axis) and rides
    // down to every residue leaf of that cell.
    let mut stack: Vec<(Box3, Option<Interval>)> = cells.iter().rev().map(|b| (*b, None)).collect();
    let mut residue_candidates: Vec<(Box3, Option<Interval>)> = Vec::new();
    while let Some((b, inherited_lam)) = stack.pop() {
        let cover = gff::cover_branch(f1, f2, &b, tau, budget)?;
        report.regular.points.extend(cover.value.points);
        report
            .regular
            .unresolved_boxes
            .extend(cover.value.unresolved_boxes);
        for s in cover.value.singular_boxes {
            // A descendant box inherits its singular cell's envelope; a fresh
            // cell's envelope is computed over the cell itself.
            let lam_box = inherited_lam.or_else(|| multiplier_envelope(d1, d2, &s));
            if s.width() <= tau {
                residue_candidates.push((s, lam_box));
            } else {
                let Some((lo, hi)) = bisect_box(&s) else {
                    // A box wider than tau that cannot bisect in f64 is at its
                    // own resolution floor: classify it as-is.
                    residue_candidates.push((s, lam_box));
                    continue;
                };
                budget
                    .spend_subdiv(1)
                    .map_err(|_| Refusal::NumericallyUnresolved {
                        spent: spent(&initial, budget),
                        witness: UnresolvedWitness::KrawczykIndeterminate,
                    })?;
                stack.push((lo, lam_box));
                stack.push((hi, lam_box));
            }
        }
    }
    // Classification of the resolution-floor residue leaves (decision 3), in
    // discovery order.
    for (leaf, lam_box) in residue_candidates {
        let verdict = classify_leaf(&ctx, &leaf, tau, lam_box, budget, &mut report)?;
        match verdict {
            LeafVerdict::NoRoot => {
                report.regular.unresolved_boxes.push(leaf);
            }
            LeafVerdict::Certified => {}
            LeafVerdict::Inconclusive => {
                // Defer unless the leaf is sub-resolution content of a point
                // already certified by a neighbouring leaf.
                if !contains_certified(&leaf, &report) {
                    report.residue.push(leaf);
                }
            }
        }
    }
    Ok(Certified::new(report, certificate(budget)))
}

/// The verdict of one leaf's classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LeafVerdict {
    /// A contact event was certified (isolated tangency, tangential crossing,
    /// or carrier-degenerate contact) and recorded.
    Certified,
    /// The Lagrange system proved no tangency in the leaf and the degenerate
    /// pass found nothing: a resolution issue, not a singular point.
    NoRoot,
    /// Nothing could be certified or refuted (envelope missing, Krawczyk
    /// indeterminate on an unsplittable leaf, or the restricted-Hessian
    /// inertia was inconclusive): the leaf's locus dimension is unclassified.
    Inconclusive,
}

/// The shared classification context of one `singular_events` run: the two
/// implicit fields, their exact on-surface degenerate points, and the entry
/// budget used to report total spend.
struct ClassifyCtx<'a> {
    f1: &'a dyn ImplicitField,
    f2: &'a dyn ImplicitField,
    deg1: &'a [Point3],
    deg2: &'a [Point3],
    initial: Budget,
}

/// Classify one leaf (decision 3): (a) the degenerate pass, (b) the Lagrange
/// system under the sound multiplier envelope, (c) the restricted-Hessian
/// inertia at the certified root. Records the certified event into `report`
/// and returns the verdict. A non-exhaustion Krawczyk refusal (the root sits
/// on the leaf's boundary, so the strict-interior rule cannot certify) is
/// `Inconclusive`, not a propagation; only genuine budget exhaustion (no
/// subdivisions remain) propagates. `lam_box` is the certified cell's
/// multiplier envelope (sound over every residue leaf of that cell).
fn classify_leaf(
    ctx: &ClassifyCtx<'_>,
    leaf: &Box3,
    tau: f64,
    lam_box: Option<Interval>,
    budget: &mut Budget,
    report: &mut SingularReport,
) -> Result<LeafVerdict, Refusal> {
    let d1 = ctx.f1;
    let d2 = ctx.f2;
    // (a) Degenerate pass.
    if let Some(q) = degenerate_contact(d1, d2, leaf, ctx.deg1, ctx.deg2) {
        dedup_push(&mut report.degenerate, q);
        return Ok(LeafVerdict::Certified);
    }
    // (b) Lagrange system. `delta == 0` over the cell means the multiplier
    // envelope does not exist and the leaf's locus dimension stays
    // unclassified (it may hold an f1 degenerate locus that (a) did not
    // enumerate exactly).
    let Some(lam_box) = lam_box else {
        return Ok(LeafVerdict::Inconclusive);
    };
    let sys = LagrangeSystem { f1: d1, f2: d2 };
    let start = krawczyk_box(leaf, lam_box, tau);
    match krawczyk::<4>(&sys, &start, budget) {
        Ok(Certified {
            value: KrawczykProof::Unique,
            ..
        }) => {
            // Extract the certified root: a few Newton steps from the 4-box
            // midpoint using the same f64 Jacobian inverse (mirrors
            // `gff::refine_point`), with a robust multiplier seed.
            let m = [
                0.5 * (start[0].inf() + start[0].sup()),
                0.5 * (start[1].inf() + start[1].sup()),
                0.5 * (start[2].inf() + start[2].sup()),
                0.5 * (start[3].inf() + start[3].sup()),
            ];
            let (p, lam_star) = refine_root(
                &sys,
                Point3::new(m[0], m[1], m[2]),
                m[3],
                lam_box.sup(),
                &start,
            );
            match classify_restricted(d1, d2, p, lam_star) {
                Inertia::Tangency(p) => {
                    dedup_push(&mut report.tangencies, p);
                    Ok(LeafVerdict::Certified)
                }
                Inertia::Crossing(p) => {
                    dedup_push(&mut report.tangential_crossings, p);
                    Ok(LeafVerdict::Certified)
                }
                Inertia::Indeterminate => Ok(LeafVerdict::Inconclusive),
            }
        }
        Ok(Certified {
            value: KrawczykProof::NoRoot,
            ..
        }) => {
            // No tangency in the leaf and no degenerate contact: whatever kept
            // it from charting is a resolution issue, not a singular point.
            Ok(LeafVerdict::NoRoot)
        }
        Err(Refusal::NumericallyUnresolved { .. }) => {
            if budget.subdiv == 0 {
                // Genuine budget exhaustion: propagate with the total spend.
                Err(Refusal::NumericallyUnresolved {
                    spent: spent(&ctx.initial, budget),
                    witness: UnresolvedWitness::KrawczykIndeterminate,
                })
            } else {
                // An unsplittable leaf whose certified root sits on its own
                // boundary: defer, never refuse.
                Ok(LeafVerdict::Inconclusive)
            }
        }
        // An empty or non-finite start component cannot certify anything.
        Err(_) => Ok(LeafVerdict::Inconclusive),
    }
}

/// Whether a leaf is sub-resolution content of an already-certified contact
/// point (a tangency, a tangential crossing, or a carrier-degenerate contact).
fn contains_certified(leaf: &Box3, report: &SingularReport) -> bool {
    report
        .tangencies
        .iter()
        .chain(report.tangential_crossings.iter())
        .chain(report.degenerate.iter())
        .any(|p| leaf.contains(*p))
}

/// The Krawczyk start box for a leaf: `[leaf.x, leaf.y, leaf.z, lam_box]`,
/// with every finite coordinate widened by `tau` on each side. The refinement
/// bisects on the grid whose points include the packet's dyadic tangency
/// witnesses, so a certified root lands exactly ON a residue leaf's boundary
/// (or on a collapsed zero-width axis of the certified AABB), where the
/// Krawczyk strict-interior rule cannot certify. Widening by the resolution
/// floor makes the certified root strictly interior while the box stays
/// tau-scale tight; the certified root is still verified to satisfy the
/// system, and any spurious root is caught by the contact sanity check and
/// the restricted-Hessian inertia.
fn krawczyk_box(leaf: &Box3, lam_box: Interval, tau: f64) -> [Interval; 4] {
    let widen = |iv: Interval| {
        if iv.inf().is_finite() && iv.sup().is_finite() {
            Interval::try_from((iv.inf() - tau, iv.sup() + tau)).unwrap_or(iv)
        } else {
            iv
        }
    };
    [widen(leaf.x), widen(leaf.y), widen(leaf.z), lam_box]
}

/// The degenerate pass (decision 3(a)): the first exact on-surface degenerate
/// point `q` of either carrier that lies inside `leaf` and whose OTHER
/// carrier's point enclosure contains zero. `None` when no such contact is
/// certified.
fn degenerate_contact(
    d1: &dyn ImplicitField,
    d2: &dyn ImplicitField,
    leaf: &Box3,
    deg1: &[Point3],
    deg2: &[Point3],
) -> Option<Point3> {
    for q in deg1 {
        if leaf.contains(*q) && d2.implicit(&Box3::point(*q)).contains(0.0) {
            return Some(*q);
        }
    }
    for q in deg2 {
        if leaf.contains(*q) && d1.implicit(&Box3::point(*q)).contains(0.0) {
            return Some(*q);
        }
    }
    None
}

/// The sound multiplier envelope (decision 3(b)): every tangency `t` in the
/// leaf with `grad(f1)(t) != 0` satisfies `|lam(t)| <= B2/delta` with
/// `delta = max_k inf_leaf |df1/dx_k|` and
/// `B2 = sqrt(sum_k sup_leaf |df2/dx_k|²)`. `None` when the envelope does not
/// exist (`delta == 0`, or a non-finite bound).
fn multiplier_envelope(
    d1: &dyn ImplicitField,
    d2: &dyn ImplicitField,
    leaf: &Box3,
) -> Option<Interval> {
    let g1 = d1.grad(leaf);
    let g2 = d2.grad(leaf);
    let delta = g1.iter().fold(0.0f64, |acc, iv| acc.max(iv.mig()));
    if !delta.is_finite() || delta <= 0.0 {
        return None;
    }
    let b2 = g2
        .iter()
        .fold(0.0f64, |acc, iv| {
            acc + iv.inf().abs().max(iv.sup().abs()).powi(2)
        })
        .sqrt();
    if !b2.is_finite() {
        return None;
    }
    let bound = b2 / delta;
    if !bound.is_finite() {
        return None;
    }
    Interval::try_from((-bound, bound)).ok()
}

/// The certified Lagrange critical-point system (decision 3(b)).
///
/// Unknowns `(x, y, z, lam)`, constraint `f1`, objective `f2`:
/// `F = [f1, grad(f2) + lam·grad(f1)]`. `f_point` evaluates at the point
/// (degenerate intervals); `jacobian` is the interval 4×4 over the 4-box,
/// row-major, columns `(x, y, z, lam)`, row 0 = `[df1/dx, df1/dy, df1/dz, 0]`
/// and row `1+i` = `[hess(f2)[i] + lam·hess(f1)[i], grad(f1)_i]`;
/// `preconditioner` builds the same Jacobian at the f64 point and inverts it
/// with a private Gauss-Jordan with partial pivoting.
struct LagrangeSystem<'a> {
    f1: &'a dyn ImplicitField,
    f2: &'a dyn ImplicitField,
}

impl LagrangeSystem<'_> {
    /// The point Jacobian at `(x, y, z, lam)`: entries extracted from the
    /// degenerate point-box enclosures (`.inf()` of a degenerate interval).
    fn point_jacobian(&self, x: &[f64; 4]) -> [[f64; 4]; 4] {
        let [x0, y0, z0, lam0] = *x;
        let p = Box3::point(Point3::new(x0, y0, z0));
        let g1 = self.f1.grad(&p);
        let h1 = self.f1.hess(&p);
        let h2 = self.f2.hess(&p);
        let e = |iv: Interval| iv.inf();
        [
            [e(g1[0]), e(g1[1]), e(g1[2]), 0.0],
            [
                e(h2[0][0]) + lam0 * e(h1[0][0]),
                e(h2[0][1]) + lam0 * e(h1[0][1]),
                e(h2[0][2]) + lam0 * e(h1[0][2]),
                e(g1[0]),
            ],
            [
                e(h2[1][0]) + lam0 * e(h1[1][0]),
                e(h2[1][1]) + lam0 * e(h1[1][1]),
                e(h2[1][2]) + lam0 * e(h1[1][2]),
                e(g1[1]),
            ],
            [
                e(h2[2][0]) + lam0 * e(h1[2][0]),
                e(h2[2][1]) + lam0 * e(h1[2][1]),
                e(h2[2][2]) + lam0 * e(h1[2][2]),
                e(g1[2]),
            ],
        ]
    }
}

impl KrawczykSystem<4> for LagrangeSystem<'_> {
    fn f_point(&self, x: &[f64; 4]) -> [Interval; 4] {
        let [x0, y0, z0, lam0] = *x;
        let p = Box3::point(Point3::new(x0, y0, z0));
        let l = interval_at(lam0);
        let g1 = self.f1.grad(&p);
        let g2 = self.f2.grad(&p);
        [
            self.f1.implicit(&p),
            g2[0] + l * g1[0],
            g2[1] + l * g1[1],
            g2[2] + l * g1[2],
        ]
    }

    fn jacobian(&self, b: &[Interval; 4]) -> [[Interval; 4]; 4] {
        let boxed = Box3 {
            x: b[0],
            y: b[1],
            z: b[2],
        };
        let lam = b[3];
        let g1 = self.f1.grad(&boxed);
        let h1 = self.f1.hess(&boxed);
        let h2 = self.f2.hess(&boxed);
        let zero = interval_at(0.0);
        [
            [g1[0], g1[1], g1[2], zero],
            [
                h2[0][0] + lam * h1[0][0],
                h2[0][1] + lam * h1[0][1],
                h2[0][2] + lam * h1[0][2],
                g1[0],
            ],
            [
                h2[1][0] + lam * h1[1][0],
                h2[1][1] + lam * h1[1][1],
                h2[1][2] + lam * h1[1][2],
                g1[1],
            ],
            [
                h2[2][0] + lam * h1[2][0],
                h2[2][1] + lam * h1[2][1],
                h2[2][2] + lam * h1[2][2],
                g1[2],
            ],
        ]
    }

    fn preconditioner(&self, x: &[f64; 4]) -> Option<[[f64; 4]; 4]> {
        invert4(&self.point_jacobian(x))
    }
}

/// The private 4×4 Gauss-Jordan inverse with partial pivoting: `None` when the
/// best pivot is zero or non-finite. No indexing (H-1): row/column access goes
/// through `.get()`/`.get_mut()`.
fn invert4(m: &[[f64; 4]; 4]) -> Option<[[f64; 4]; 4]> {
    // Augment M with the 4×4 identity.
    let mut a = [[0.0f64; 8]; 4];
    for (r, row) in a.iter_mut().enumerate() {
        for (c, v) in m.get(r)?.iter().enumerate() {
            *row.get_mut(c)? = *v;
        }
        *row.get_mut(4 + r)? = 1.0;
    }
    for p in 0..4 {
        // Partial pivoting: the row in `p..4` with the largest |a[r][p]|, ties
        // toward the lowest row.
        let mut best = p;
        let mut best_abs = a
            .get(p)
            .and_then(|row| row.get(p))
            .copied()
            .unwrap_or(0.0)
            .abs();
        for r in (p + 1)..4 {
            let v = a
                .get(r)
                .and_then(|row| row.get(p))
                .copied()
                .unwrap_or(0.0)
                .abs();
            if v > best_abs {
                best = r;
                best_abs = v;
            }
        }
        let pivot = a
            .get(best)
            .and_then(|row| row.get(p))
            .copied()
            .unwrap_or(0.0);
        if !pivot.is_finite() || pivot == 0.0 {
            return None;
        }
        if best != p {
            a.swap(p, best);
        }
        // Normalize the pivot row.
        let row_p = a.get_mut(p)?;
        for c in 0..8 {
            let v = row_p.get(c).copied().unwrap_or(0.0);
            *row_p.get_mut(c)? = v / pivot;
        }
        // Eliminate column p from every other row.
        for r in 0..4 {
            if r == p {
                continue;
            }
            let factor = a.get(r).and_then(|row| row.get(p)).copied().unwrap_or(0.0);
            let pivot_row = a.get(p).copied().unwrap_or([0.0; 8]);
            let target = a.get_mut(r)?;
            for c in 0..8 {
                let pv = pivot_row.get(c).copied().unwrap_or(0.0);
                let cur = target.get(c).copied().unwrap_or(0.0);
                *target.get_mut(c)? = cur - factor * pv;
            }
        }
    }
    let mut inv = [[0.0f64; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            let v = a
                .get(r)
                .and_then(|row| row.get(4 + c))
                .copied()
                .unwrap_or(0.0);
            *inv.get_mut(r)?.get_mut(c)? = v;
        }
    }
    Some(inv)
}

/// A float Newton refinement of the certified Lagrange root, mirroring
/// `gff::refine_point`: the Krawczyk proof is the certificate; this only
/// sharpens the recorded location toward the proven root.
///
/// The multiplier is seeded at the 4-box midpoint first (the packet's
/// pattern). The midpoint `lam = 0` of the symmetric envelope can sit on an
/// order-dependent near-singular Jacobian configuration (measured: for the
/// reversed field order the z-row of the system degenerates at `lam = 0` and
/// plain Newton diverges to a point whose `f2` is far from zero), so the
/// envelope's endpoints are tried in turn. Only a seed whose Newton iterate
/// converged (small final correction) to a point inside the certified 4-box
/// is accepted; the packet's midpoint seed remains the fallback.
fn refine_root(
    sys: &LagrangeSystem<'_>,
    c: Point3,
    lam_mid: f64,
    lam_bound: f64,
    box4: &[Interval; 4],
) -> (Point3, f64) {
    for seed in [lam_mid, lam_bound, -lam_bound] {
        let (p, l) = newton_from(sys, c, seed);
        if in_box4(p, l, box4) {
            // A converged iterate has a small final correction; a divergent
            // one lands far from the certified root.
            let f = sys.f_point(&[p.x, p.y, p.z, l]);
            let residual = f.iter().fold(0.0f64, |acc, iv| acc.max(iv.mid().abs()));
            if residual <= NEWTON_TOL {
                return (p, l);
            }
        }
    }
    // Fallback: the packet's midpoint-seed result (the classification's
    // contact sanity check defers it if it is not contact).
    newton_from(sys, c, lam_mid)
}

/// One Newton descent from `(c, seed)` using the f64 Jacobian inverse,
/// mirroring `gff::refine_point`'s iterate pattern.
fn newton_from(sys: &LagrangeSystem<'_>, c: Point3, seed: f64) -> (Point3, f64) {
    let mut p = c;
    let mut l = seed;
    for _ in 0..MAX_NEWTON_STEPS {
        let Some(y) = sys.preconditioner(&[p.x, p.y, p.z, l]) else {
            break;
        };
        let f = sys.f_point(&[p.x, p.y, p.z, l]);
        let [f0, f1, f2, f3] = f;
        let [[y00, y01, y02, y03], [y10, y11, y12, y13], [y20, y21, y22, y23], [y30, y31, y32, y33]] =
            y;
        let dx = y00 * f0.mid() + y01 * f1.mid() + y02 * f2.mid() + y03 * f3.mid();
        let dy = y10 * f0.mid() + y11 * f1.mid() + y12 * f2.mid() + y13 * f3.mid();
        let dz = y20 * f0.mid() + y21 * f1.mid() + y22 * f2.mid() + y23 * f3.mid();
        let dl = y30 * f0.mid() + y31 * f1.mid() + y32 * f2.mid() + y33 * f3.mid();
        let nx = p.x - dx;
        let ny = p.y - dy;
        let nz = p.z - dz;
        let nl = l - dl;
        let correction = (dx * dx + dy * dy + dz * dz + dl * dl).sqrt();
        if !correction.is_finite() || correction <= NEWTON_TOL {
            return (Point3::new(nx, ny, nz), nl);
        }
        p = Point3::new(nx, ny, nz);
        l = nl;
    }
    (p, l)
}

/// Whether a point (with multiplier) lies inside the certified 4-box (the
/// widened Krawczyk box of the leaf). A Newton iterate that left the box did
/// not converge to the certified root.
fn in_box4(p: Point3, l: f64, box4: &[Interval; 4]) -> bool {
    box4[0].contains(p.x) && box4[1].contains(p.y) && box4[2].contains(p.z) && box4[3].contains(l)
}

/// How many Newton steps refine a certified Lagrange root (mirrors
/// `gff::MAX_NEWTON_STEPS`). The Krawczyk contraction makes this a fixed small
/// budget, not a geometry-dependent loop.
const MAX_NEWTON_STEPS: usize = 8;

/// The Newton correction floor below which the iterate is taken as the root.
/// H-3: a dimensionless convergence floor on a float Newton iterate, not a
/// model-space length.
const NEWTON_TOL: f64 = 1.0e-10; // H-3: dimensionless Newton convergence floor, not a length

/// The restricted-Hessian inertia verdict at a certified Lagrange root.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Inertia {
    /// Definite positive or negative: an isolated strict local extremum of
    /// `f2` restricted to `f1`'s surface at value 0.
    Tangency(Point3),
    /// Indefinite: the zero set of `f2|f1=0` self-crosses at the saddle.
    Crossing(Point3),
    /// The inertia could not be decided, or the certified critical point is
    /// not a contact point.
    Indeterminate,
}

/// The contact sanity-check tolerance: a certified Lagrange root whose
/// `f2` point-box lands within `CONTACT_TOL` of zero is a contact point. The
/// float Newton refinement converges to within an ulp of the exact root, so
/// the exact `contains(0.0)` check rejects genuine tangencies whose refined
/// location rounds one ulp off the surface (measured: `f2 ≈ 4e-16` for the
/// packet's own witnesses); a non-contact critical point sits at a
/// unit-scale distance (e.g. `f2 = −3.75` for the broad domain's candidates)
/// and is still rejected.
/// H-3: a unit-scale contact-tolerance window on the certified root's f2
/// value, not a length.
const CONTACT_TOL: f64 = 1.0e-9; // H-3: unit-scale f2 contact tolerance, not a length

/// Restricted-Hessian inertia at the certified root (decision 3(c)).
///
/// `H = hess(f2) + lam*·hess(f1)` over the root's point-box; `n = grad(f1)`
/// at the same point-box extracted to f64 (`delta > 0` guarantees it is
/// nonzero); a deterministic orthonormal tangent basis `{u, v}`; the
/// restricted 2×2 `R[i][j] = basis_iᵀ·H·basis_j` with interval dot products.
/// Definite inertia (either sign) certifies an isolated strict local extremum
/// of `f2` restricted to `f1`'s surface at value 0 — an isolated tangency;
/// indefinite inertia certifies a saddle whose zero set self-crosses — a
/// tangential crossing. A final sanity check requires the root's point-box
/// enclosure of `f2` to be within `CONTACT_TOL` of zero (a critical point of
/// `f2|f1` need not be a contact point).
fn classify_restricted(
    d1: &dyn ImplicitField,
    d2: &dyn ImplicitField,
    p: Point3,
    lam_star: f64,
) -> Inertia {
    let pbox = Box3::point(p);
    // Final sanity check: the certified critical point is contact only if
    // `f2` vanishes (up to the rounding scale of the float refinement) there.
    let f2_box = d2.implicit(&pbox);
    if f2_box.inf() > CONTACT_TOL || f2_box.sup() < -CONTACT_TOL {
        return Inertia::Indeterminate;
    }
    let lam = interval_at(lam_star);
    let h1 = d1.hess(&pbox);
    let h2 = d2.hess(&pbox);
    let h = [
        [
            h2[0][0] + lam * h1[0][0],
            h2[0][1] + lam * h1[0][1],
            h2[0][2] + lam * h1[0][2],
        ],
        [
            h2[1][0] + lam * h1[1][0],
            h2[1][1] + lam * h1[1][1],
            h2[1][2] + lam * h1[1][2],
        ],
        [
            h2[2][0] + lam * h1[2][0],
            h2[2][1] + lam * h1[2][1],
            h2[2][2] + lam * h1[2][2],
        ],
    ];
    let g1 = d1.grad(&pbox);
    let n = Vector3::new(g1[0].inf(), g1[1].inf(), g1[2].inf());
    let Some((u, v)) = tangent_basis(n) else {
        return Inertia::Indeterminate;
    };
    let r00 = restricted_form(u, u, h);
    let r01 = restricted_form(u, v, h);
    let r11 = restricted_form(v, v, h);
    let det = r00 * r11 - r01 * r01;
    if det.inf() > 0.0 && r00.inf() > 0.0 {
        // Definite positive: isolated strict local extremum of f2|f1=0.
        Inertia::Tangency(p)
    } else if det.inf() > 0.0 && r11.sup() < 0.0 {
        // Definite negative: isolated strict local extremum of f2|f1=0.
        Inertia::Tangency(p)
    } else if det.sup() < 0.0 {
        // Indefinite: the zero set of f2|f1=0 self-crosses at the saddle.
        Inertia::Crossing(p)
    } else {
        Inertia::Indeterminate
    }
}

/// The deterministic orthonormal tangent basis of the surface `f1 = 0` at a
/// root: `u = normalize(n × e_a)`, `v = normalize(n × u)`, with `a` the index
/// of the SMALLEST `|n_i|` (ties toward the lowest index). See the module
/// note: the packet's decision-3 "largest" choice is degenerate on the
/// packet's own axis-aligned witnesses, so the standard non-degenerate
/// smallest-component frame is used. `None` only when `n` is zero or
/// non-finite (impossible here: `delta > 0` guarantees a nonzero gradient).
fn tangent_basis(n: Vector3) -> Option<(Vector3, Vector3)> {
    let ax = n.x.abs();
    let ay = n.y.abs();
    let az = n.z.abs();
    let a = if ax <= ay && ax <= az {
        0
    } else if ay <= az {
        1
    } else {
        2
    };
    let e = match a {
        0 => Vector3::unit_x(),
        1 => Vector3::unit_y(),
        _ => Vector3::unit_z(),
    };
    let u0 = n.cross(e);
    let un = u0.magnitude();
    if !un.is_finite() || un <= 0.0 {
        return None;
    }
    let u = u0 / un;
    let v0 = n.cross(u);
    let vn = v0.magnitude();
    if !vn.is_finite() || vn <= 0.0 {
        return None;
    }
    let v = v0 / vn;
    Some((u, v))
}

/// The interval quadratic form `aᵀ·H·b` (the restricted-Hessian entry for the
/// two tangent basis vectors), with interval dot products over the root's
/// point-box.
fn restricted_form(a: Vector3, b: Vector3, h: [[Interval; 3]; 3]) -> Interval {
    let [[h00, h01, h02], [h10, h11, h12], [h20, h21, h22]] = h;
    let w0 = h00 * interval_at(b.x) + h01 * interval_at(b.y) + h02 * interval_at(b.z);
    let w1 = h10 * interval_at(b.x) + h11 * interval_at(b.y) + h12 * interval_at(b.z);
    let w2 = h20 * interval_at(b.x) + h21 * interval_at(b.y) + h22 * interval_at(b.z);
    interval_at(a.x) * w0 + interval_at(a.y) * w1 + interval_at(a.z) * w2
}

/// The unit-scale event-identity residual: two certified points whose
/// componentwise max-norm distance is `<= EVENT_RESIDUAL` are one event, first
/// in discovery order wins.
/// H-3: unit-scale event-identity residual, not a length.
const EVENT_RESIDUAL: f64 = 1.0e-6; // H-3: unit-scale event-identity residual, not a length

/// Push `p` into `list` unless an existing entry is the same event (dedup
/// within one classified list; decision 5).
fn dedup_push(list: &mut Vec<Point3>, p: Point3) {
    let is_new = list.iter().all(|q| max_norm(*q, p) > EVENT_RESIDUAL);
    if is_new {
        list.push(p);
    }
}

/// The componentwise max-norm distance between two points.
fn max_norm(a: Point3, b: Point3) -> f64 {
    let dx = (a.x - b.x).abs();
    let dy = (a.y - b.y).abs();
    let dz = (a.z - b.z).abs();
    dx.max(dy).max(dz)
}

/// Bisect a box on its widest axis (ties toward the lowest axis index) at the
/// convex-combination midpoint, exactly as `krawczyk::push_children` does.
/// `None` when the box cannot bisect in f64 (zero width or the midpoint
/// rounds onto an edge).
fn bisect_box(b: &Box3) -> Option<(Box3, Box3)> {
    let wx = b.x.sup() - b.x.inf();
    let wy = b.y.sup() - b.y.inf();
    let wz = b.z.sup() - b.z.inf();
    let (axis, a, s) = if wx >= wy && wx >= wz {
        (0, b.x.inf(), b.x.sup())
    } else if wy >= wz {
        (1, b.y.inf(), b.y.sup())
    } else {
        (2, b.z.inf(), b.z.sup())
    };
    let mid = 0.5 * a + 0.5 * s;
    if mid == a || mid == s {
        return None;
    }
    let lo_iv = Interval::try_from((a, mid)).ok()?;
    let hi_iv = Interval::try_from((mid, s)).ok()?;
    match axis {
        0 => Some((Box3 { x: lo_iv, ..*b }, Box3 { x: hi_iv, ..*b })),
        1 => Some((Box3 { y: lo_iv, ..*b }, Box3 { y: hi_iv, ..*b })),
        _ => Some((Box3 { z: lo_iv, ..*b }, Box3 { z: hi_iv, ..*b })),
    }
}

/// Spend since entry: the initial budget minus what remains (mirrored from
/// `gff`/`krawczyk`). Never the REMAINING budget as `spent` — that hides
/// exhaustion.
fn spent(initial: &Budget, budget: &Budget) -> Budget {
    Budget {
        subdiv: initial.subdiv - budget.subdiv,
        newton: initial.newton - budget.newton,
        depth: initial.depth - budget.depth,
    }
}

/// The successful certificate of a classification: interval method, empty
/// props, actual remaining budget, unbounded margin/modulus — exactly
/// `gff`'s `certificate(budget)` shape (decision 6).
fn certificate(budget: &Budget) -> Certificate {
    Certificate {
        props: PropMap::new(),
        method: Method::Interval,
        budget_left: *budget,
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. Unit-test assertions on hand-built dyadic witnesses are
// not such a path; the unwraps below cannot fire for the values constructed.
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use truck_base::cgmath64::EuclideanSpace;
    use truck_geometry::specifieds::{Cone, Cylinder, Sphere};

    /// Build a test interval, degrading to EMPTY (and failing its assertion)
    /// rather than panicking on a malformed bound.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// The validated UNIT z-cylinder at the origin, matching the `Outcome`
    /// constructor's shape.
    fn unit_cylinder() -> Cylinder {
        Cylinder::new(Point3::origin(), 1.0)
            .expect("a positive finite radius is always a valid cylinder")
            .value
    }

    /// The unit-scale residual on a certified singular-event location.
    /// H-3: unit-scale event-location residual, not a length.
    const SINGULAR_RESIDUAL: f64 = 1.0e-6; // H-3: unit-scale event-location residual, not a length

    /// The healthy subdivision budget the singular witnesses classify under
    /// (mirrors the `gff` test budgets).
    /// H-3: a subdivision budget counter, not a length.
    const SINGULAR_BUDGET: u32 = 8192; // H-3: a subdivision budget counter, not a length

    /// The resolution floor of a test box: its widest axis over 128.
    fn tau_of(b: &Box3) -> f64 {
        b.width() / 128.0
    }

    /// Every point of `a` has a match in `b` within `residual`, and vice
    /// versa (order-insensitive set comparison).
    fn points_match(a: &[Point3], b: &[Point3], residual: f64, label: &str) {
        assert_eq!(a.len(), b.len(), "{label}: same event count");
        for p in a {
            assert!(
                b.iter().any(|q| (*p - *q).magnitude() <= residual),
                "{label}: point {p:?} has no match"
            );
        }
        for q in b {
            assert!(
                a.iter().any(|p| (*p - *q).magnitude() <= residual),
                "{label}: point {q:?} has no match"
            );
        }
    }

    #[test]
    fn singular_refines_regular_cover_from_broad_singular_domain() {
        // Witness 4: the unit cylinder vs the sphere center (0.5,0,0) radius 2
        // over the broad box x∈[-1.5,1.5], y∈[-1.5,1.5], z∈[-2,2]. All three
        // cross-gradient minors contain zero (no domain chart), the box holds
        // no tangency (candidates (±1,0,0) give f2 = −3.75, −1.75), and the
        // crossing (1, 0, √3.75) lies inside. The stage refines the broad
        // singular domain into chartable children that certify the regular
        // crossings, and classifies nothing singular.
        let cyl = unit_cylinder();
        let sph = Sphere::new(Point3::new(0.5, 0.0, 0.0), 2.0);
        let box3 = Box3 {
            x: iv(-1.5, 1.5),
            y: iv(-1.5, 1.5),
            z: iv(-2.0, 2.0),
        };
        let mut budget = Budget::new(SINGULAR_BUDGET, 0, 0);
        let out = singular_events(&cyl, &sph, &[box3], tau_of(&box3), &mut budget)
            .expect("a healthy budget classifies the broad singular domain");
        assert!(
            !out.value.regular.points.is_empty(),
            "the refined broad domain certifies regular crossings"
        );
        assert!(out.value.tangencies.is_empty());
        assert!(out.value.tangential_crossings.is_empty());
        assert!(out.value.degenerate.is_empty());
        assert!(out.value.residue.is_empty());
    }

    #[test]
    fn singular_certifies_isolated_external_tangency() {
        // Witness 1: the unit cylinder is externally tangent to the sphere
        // center (2,0,0) radius 1 at exactly (1,0,0). The box
        // x∈[0.9,1.1], y∈[-0.1,0.1], z∈[-0.1,0.1] is chartless (minors
        // (4yz, -4xz, 8y) all contain zero); the Lagrange system certifies the
        // unique root (1,0,0, lam* = 1) and the restricted Hessian diag(4,2)
        // is definite positive: exactly one isolated tangency.
        let cyl = unit_cylinder();
        let sph = Sphere::new(Point3::new(2.0, 0.0, 0.0), 1.0);
        let box3 = Box3 {
            x: iv(0.9, 1.1),
            y: iv(-0.1, 0.1),
            z: iv(-0.1, 0.1),
        };
        let mut budget = Budget::new(SINGULAR_BUDGET, 0, 0);
        let out = singular_events(&cyl, &sph, &[box3], tau_of(&box3), &mut budget)
            .expect("a healthy budget certifies the isolated external tangency");
        assert_eq!(
            out.value.tangencies.len(),
            1,
            "exactly one isolated tangency"
        );
        assert!(
            (*out.value.tangencies.first().expect("one tangency") - Point3::new(1.0, 0.0, 0.0))
                .magnitude()
                <= SINGULAR_RESIDUAL,
            "the tangency is at (1,0,0)"
        );
        assert!(out.value.tangential_crossings.is_empty());
        assert!(out.value.degenerate.is_empty());
        assert!(out.value.residue.is_empty());
    }

    #[test]
    fn singular_classifies_internal_tangency_as_crossing() {
        // Witness 2: the unit cylinder and the sphere center (1,0,0) radius 2
        // are internally tangent at (-1,0,0); the contact locus self-crosses
        // there (the exit curve pinches through itself). The Lagrange system
        // certifies the unique root (-1,0,0, lam* = -2) and the restricted
        // Hessian diag(-2, 2) is indefinite, so the event is a tangential
        // crossing, never an isolated tangency. The crossing branches
        // chart-certify around the pinch, so the regular cover is non-empty.
        let cyl = unit_cylinder();
        let sph = Sphere::new(Point3::new(1.0, 0.0, 0.0), 2.0);
        let box3 = Box3 {
            x: iv(-1.1, -0.9),
            y: iv(-0.1, 0.1),
            z: iv(-0.1, 0.1),
        };
        let mut budget = Budget::new(SINGULAR_BUDGET, 0, 0);
        let out = singular_events(&cyl, &sph, &[box3], tau_of(&box3), &mut budget)
            .expect("a healthy budget classifies the internal tangency");
        assert_eq!(
            out.value.tangential_crossings.len(),
            1,
            "exactly one tangential crossing"
        );
        assert!(
            (*out
                .value
                .tangential_crossings
                .first()
                .expect("one crossing")
                - Point3::new(-1.0, 0.0, 0.0))
            .magnitude()
                <= SINGULAR_RESIDUAL,
            "the crossing is at (-1,0,0)"
        );
        assert!(out.value.tangencies.is_empty());
        assert!(out.value.degenerate.is_empty());
        assert!(
            !out.value.regular.points.is_empty(),
            "the crossing branches chart-certify around the pinch"
        );
    }

    #[test]
    fn singular_certifies_degenerate_apex_contact() {
        // Witness 3: the cone apex (1,0,0), half angle atan(3/4), sits exactly
        // on the unit cylinder wall (f_cyl(apex) = 0) and two contact branches
        // cross through it. The degenerate pass certifies the apex contact
        // without running the Lagrange step; the surrounding chartable
        // children certify the regular crossings of the two branches.
        let cone = Cone::new(Point3::new(1.0, 0.0, 0.0), (3.0 / 4.0f64).atan())
            .expect("a dyadic cone is a valid carrier")
            .value;
        let cyl = unit_cylinder();
        // The two contact branches leave the apex along x < 1 with y,z ~ √(1−x);
        // the box is wide enough that chartable children away from the yz≈0
        // chartless spine certify the branches' regular crossings.
        let box3 = Box3 {
            x: iv(0.85, 1.15),
            y: iv(-0.3, 0.3),
            z: iv(-0.4, 0.4),
        };
        let mut budget = Budget::new(SINGULAR_BUDGET, 0, 0);
        let out = singular_events(&cone, &cyl, &[box3], tau_of(&box3), &mut budget)
            .expect("a healthy budget certifies the degenerate apex contact");
        assert_eq!(out.value.degenerate.len(), 1, "exactly one apex contact");
        assert!(
            (*out
                .value
                .degenerate
                .first()
                .expect("one degenerate contact")
                - Point3::new(1.0, 0.0, 0.0))
            .magnitude()
                <= SINGULAR_RESIDUAL,
            "the degenerate contact is the apex (1,0,0)"
        );
        assert!(out.value.tangencies.is_empty());
        assert!(out.value.tangential_crossings.is_empty());
        assert!(
            !out.value.regular.points.is_empty(),
            "the two contact branches chart-certify around the apex"
        );
    }

    #[test]
    fn singular_events_are_order_insensitive() {
        // Witnesses 1 and 2 with the two field orders swapped: the classified
        // list KINDS are the same and the certified points match
        // order-insensitively within a named residual.
        let cyl = unit_cylinder();
        let sph_ext = Sphere::new(Point3::new(2.0, 0.0, 0.0), 1.0);
        let ext_box = Box3 {
            x: iv(0.9, 1.1),
            y: iv(-0.1, 0.1),
            z: iv(-0.1, 0.1),
        };
        let mut budget = Budget::new(SINGULAR_BUDGET, 0, 0);
        let fwd_ext = singular_events(&cyl, &sph_ext, &[ext_box], tau_of(&ext_box), &mut budget)
            .expect("the forward order certifies the external tangency");
        let mut budget = Budget::new(SINGULAR_BUDGET, 0, 0);
        let rev_ext = singular_events(&sph_ext, &cyl, &[ext_box], tau_of(&ext_box), &mut budget)
            .expect("the reversed order certifies the external tangency");
        assert!(
            !fwd_ext.value.tangencies.is_empty() && !rev_ext.value.tangencies.is_empty(),
            "both orders classify the external tangency"
        );
        points_match(
            &fwd_ext.value.tangencies,
            &rev_ext.value.tangencies,
            SINGULAR_RESIDUAL,
            "external tangency",
        );

        let sph_int = Sphere::new(Point3::new(1.0, 0.0, 0.0), 2.0);
        let int_box = Box3 {
            x: iv(-1.1, -0.9),
            y: iv(-0.1, 0.1),
            z: iv(-0.1, 0.1),
        };
        let mut budget = Budget::new(SINGULAR_BUDGET, 0, 0);
        let fwd_int = singular_events(&cyl, &sph_int, &[int_box], tau_of(&int_box), &mut budget)
            .expect("the forward order classifies the internal tangency");
        let mut budget = Budget::new(SINGULAR_BUDGET, 0, 0);
        let rev_int = singular_events(&sph_int, &cyl, &[int_box], tau_of(&int_box), &mut budget)
            .expect("the reversed order classifies the internal tangency");
        assert!(
            !fwd_int.value.tangential_crossings.is_empty()
                && !rev_int.value.tangential_crossings.is_empty(),
            "both orders classify the internal tangency as a crossing"
        );
        assert!(
            fwd_int.value.tangencies.is_empty() && rev_int.value.tangencies.is_empty(),
            "neither order calls the saddle isolated"
        );
        points_match(
            &fwd_int.value.tangential_crossings,
            &rev_int.value.tangential_crossings,
            SINGULAR_RESIDUAL,
            "internal tangency",
        );
    }
}
