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

//! The Tier-2 critical-point start set (BG-KV2-304-S3B): the §7 R3
//! critical-point residual and the §9.2 subdivision start set with the
//! a-posteriori `k_a` direction-perturbation retry rule.
//!
//! **Theorem 9.2 / Corollary 9.3.** Let `B` be the compact lifted domain of
//! one leaf pair, `a ∈ R⁴`, and `Ψ_a(x) = (F(x), a·m(x)) : R⁴ → R⁴` the R3
//! minor form of §7. Then every connected component `C` of `Z = F⁻¹(0) ∩ B`
//! either meets `∂B` (the §9.3 R8 boundary seeds of [`crate::kernel::tier1`])
//! or contains a zero of `Ψ_a`. This module isolates the zeros of `Ψ_a` over
//! `B`: a square 4x4 Krawczyk subdivision with `a·m` exclusion first, exactly
//! the §9.2 evaluation rule (`0 ∉ □(a·m)` from the cached enclosure of `m`,
//! N7's two-stage rule).
//!
//! **The R3 residual.** [`PsiA`] implements the frozen [`SquareResidualEval`]
//! seam at arity 4 over a stored [`SquareSystem3`]: the value enclosure of
//! `F` is composed with the Theorem 6.4(iii) `a·m` enclosure — the maximal
//! minor enclosure of [`crate::kernel::minor_algebra`] composed with the
//! Jacobian enclosure, `a·m` via the landed `a_dot_m`. The Jacobian of the
//! fourth component (the gradient of `a·m`) is assembled from the Hessian
//! tensor of the stored system by the row-replacement expansion of the
//! derivative of each maximal minor, with Theorem 6.4's `(−1)^j` sign
//! pattern. Certification runs through the packet's additive arity-4 C1 entry
//! [`crate::kernel::engine::krawczyk_c1_n4`] (the N3CERT pattern, second
//! application); the engine stamps R1 and the start set rebuilds each
//! certificate with [`ResidualId::R3`], the critical-point residual's own id
//! (the documented one-line seam).
//!
//! **The §9.2 a-posteriori genericity rule.** `a` must be chosen so the
//! covector restricted to `Z` has isolated critical points on the smooth part;
//! this cannot be certified in advance, so it is verified a posteriori. If
//! square Krawczyk isolates every zero of `Ψ_a` and exclusion clears the
//! remainder, the start set is complete ([`TierTwoOutcome::Complete`]). If
//! subdivision stalls at [`crate::kernel::config::DEPTH_MAX`] without
//! isolation, the direction is perturbed and the search retried up to
//! [`crate::kernel::config::KA`] times ([`A_TABLE`], a fixed, deterministic,
//! unit-norm rational table — no RNG). A persistent positive-dimensional
//! `Ψ_a` zero set — isolation fails because the zero set is a CURVE, so every
//! cell of the shrinking sub-box family still carries zero and the bounded
//! search caps on the first attempt — is [`RefusalKind::TangentialCurve`] and
//! routes to §10.4, NOT [`RefusalKind::IncompleteStartSet`]. A caller
//! direction that stalls on a bounded, isolated-but-unresolved leaf set, whose
//! `KA` perturbations also cannot complete the start set, is
//! [`RefusalKind::IncompleteStartSet`].
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`. Where a `Result` must carry the
//! frozen `Refusal` (which holds `Option<PartialGraph>`), the large-`Err`
//! lint is allowed item-level only, exactly as the shim files do.
//!
//! **N4 / bit-reproducibility.** No transcendental call appears anywhere in
//! this module: no `sin`, `cos`, `atan2`, `exp`, `ln`, `log`, or `powf`, and
//! no `sqrt`. The arithmetic is deterministic `f64` / `CertifiedInterval`
//! sequences only, over the landed engine and minor-algebra hull kernels.
//!
//! **det3 discipline.** The one 3x3 determinant used by the `a·m` gradient
//! assembly mirrors the S2A cofactor expansion verbatim (the exact engine op
//! order, as [`crate::kernel::minor_algebra`]'s `det3_iv` mirror does); no new
//! interval linear algebra is forked.

use crate::kernel::certs::{IBox4, PointCert4};
use crate::kernel::config::{DEPTH_MAX, KA};
use crate::kernel::engine::{krawczyk_c1_n4, SquareResidualEval};
use crate::kernel::evidence::{ClaimVerdict, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::minor_algebra;
use crate::kernel::patch::CertifiedPositive;
use crate::kernel::residual::ResidualId;
use crate::kernel::Interval;
use crate::SquareSystem3;

/// The fixed a-posteriori perturbation table (§9.2): the `KA` alternative,
/// deterministic, unit-norm rational continuation directions tried in order
/// when the caller-supplied direction stalls. No RNG (the a-posteriori rule
/// must be reproducible bit for bit).
const A_TABLE: [[f64; 4]; KA as usize] = [
    [0.5, 0.5, 0.5, 0.5],
    [0.5, -0.5, 0.5, -0.5],
    [0.5, 0.5, -0.5, -0.5],
    [0.5, -0.5, -0.5, 0.5],
];

/// The vacuous enclosure of the full real line: the value the residual
/// components return when a box is not a compact subset of the stored chart
/// rectangle or a hull kernel refuses (the leaf.rs "vacuously true
/// enclosure" convention).
fn unbounded() -> Interval {
    Interval {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
    }
}

/// The certified-interval point of a float.
fn point(v: f64) -> Interval {
    Interval::point(v)
}

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}

// ---------------------------------------------------------------------------
// The R3 residual: Psi_a(x) = (F(x), a·m(x)), arity 4
// ---------------------------------------------------------------------------

/// The §7 R3 critical-point residual in its minor form (Theorem 6.4(iii)):
/// `Ψ_a(x) = (F(x), a·m(x)) : R⁴ → R⁴`, evaluated as the stored system's
/// value enclosure `F(x)` stacked over the certified `a·m(x)` enclosure of
/// the maximal-minor vector `m(x)` of the Jacobian of `F`.
///
/// This is the exclusion form of §9.2: `a·m = 0` isolates the points of `Z`
/// whose tangent is orthogonal to the constant covector `a` (the critical
/// points of `λ(x) = a·x` restricted to `Z`), and Theorem 9.2's proof uses
/// exactly the square determinant `det[DF; aᵀ] = a·m`.
#[derive(Debug, Clone, Copy)]
pub struct PsiA<'a> {
    /// The stored square system whose zero set `Z = F⁻¹(0)` is traced.
    pub sys: &'a SquareSystem3,
    /// The constant covector of the R3 residual.
    pub a: [f64; 4],
}

impl<'a> PsiA<'a> {
    /// Build the R3 minor-form residual over a stored system.
    pub fn new(sys: &'a SquareSystem3, a: [f64; 4]) -> PsiA<'a> {
        PsiA { sys, a }
    }

    /// The residual's chart box as the engine's `[(lo, hi); 4]` shape; `None`
    /// for a wrong-length, non-finite, or inverted interval list.
    fn chart_box(b: &[Interval]) -> Option<[(f64, f64); 4]> {
        if b.len() != 4 {
            return None;
        }
        if !b.iter().all(|iv| iv.is_finite() && iv.lo <= iv.hi) {
            return None;
        }
        let mut out = [(0.0f64, 0.0f64); 4];
        for i in 0..4 {
            out[i] = (b[i].lo, b[i].hi);
        }
        Some(out)
    }

    /// The certified `a·m` enclosure over a chart box, or `None` when the
    /// Jacobian enclosure could not be produced (the vacuous convention is
    /// the caller's to apply).
    fn am_over(&self, box_: &[(f64, f64); 4]) -> Option<Interval> {
        let jac = crate::kernel::engine::system_jacobian(self.sys, *box_).ok()?;
        let m = minor_algebra::minor_vector_encl(&jac);
        let a_iv = [
            point(self.a[0]),
            point(self.a[1]),
            point(self.a[2]),
            point(self.a[3]),
        ];
        Some(minor_algebra::a_dot_m(a_iv, &m))
    }
}

/// Determinant of a 3x3 interval matrix under directed rounding: the same
/// cofactor expansion as the engine's private `det3_iv` (and
/// [`crate::kernel::minor_algebra`]'s mirror), mirrored verbatim so no new
/// interval linear algebra appears.
fn det3_iv(m: &[[Interval; 3]; 3]) -> Interval {
    let a = m[0][0].mul(&m[1][1].mul(&m[2][2]).sub(&m[1][2].mul(&m[2][1])));
    let b = m[0][1].mul(&m[1][0].mul(&m[2][2]).sub(&m[1][2].mul(&m[2][0])));
    let c = m[0][2].mul(&m[1][0].mul(&m[2][1]).sub(&m[1][1].mul(&m[2][0])));
    a.sub(&b).add(&c)
}

/// The certified gradient row of the `a·m` component over a box: the
/// `x_l`-derivative of `Σ_j a_j m_j` by the row-replacement expansion of the
/// derivative of each maximal minor `m_j = (−1)^j det(DF without column j)`,
/// over the certified Jacobian enclosure `jac` and the certified Hessian
/// tensor `hes` of the stored system (both enclosing over the same box). The
/// `(−1)^j` sign pattern is Theorem 6.4's, exactly as
/// [`minor_algebra::minor_vector_encl`] spells it.
///
/// For each deleted column `j`, each row `r` of the 3x3 submatrix is replaced
/// by the `x_l`-partial of that row (`hes[r][l][col]`), and the three
/// cofactor determinants are summed; `∂(a·m)/∂x_l = Σ_j a_j (−1)^j Σ_r
/// det(M^{(j)} with row r replaced)`.
fn am_gradient_row(
    jac: &[[Interval; 4]; 3],
    hes: &[[[Interval; 4]; 4]; 3],
    a: [f64; 4],
) -> [Interval; 4] {
    let mut row = [Interval::point(0.0); 4];
    for l in 0..4 {
        let mut acc = Interval::point(0.0);
        for j in 0..4 {
            let sign = if j % 2 == 0 { 1.0 } else { -1.0 };
            let cols: [usize; 3] = {
                let mut c = [0usize; 3];
                let mut k = 0usize;
                for col in 0..4 {
                    if col != j {
                        c[k] = col;
                        k += 1;
                    }
                }
                c
            };
            for r in 0..3 {
                let mut m = [[Interval::point(0.0); 3]; 3];
                for (r2, mrow) in m.iter_mut().enumerate() {
                    for (k, &col) in cols.iter().enumerate() {
                        if r2 == r {
                            mrow[k] = hes[r][l][col];
                        } else {
                            mrow[k] = jac[r2][col];
                        }
                    }
                }
                let det = det3_iv(&m);
                acc = acc.add(&point(sign * a[j]).mul(&det));
            }
        }
        row[l] = acc;
    }
    row
}

impl<'a> SquareResidualEval for PsiA<'a> {
    fn arity(&self) -> usize {
        4
    }

    fn eval(&self, b: &[Interval]) -> Vec<Interval> {
        let box_ = match Self::chart_box(b) {
            Some(box_) => box_,
            None => return vec![unbounded(); 4],
        };
        let vals = match crate::kernel::engine::system_values(self.sys, box_) {
            Ok(vals) => vals,
            Err(_) => return vec![unbounded(); 4],
        };
        let am = match self.am_over(&box_) {
            Some(am) => am,
            None => return vec![unbounded(); 4],
        };
        vec![vals[0], vals[1], vals[2], am]
    }

    fn jac_encl(&self, b: &[Interval]) -> Vec<Vec<Interval>> {
        let box_ = match Self::chart_box(b) {
            Some(box_) => box_,
            None => return vec![vec![unbounded(); 4]; 4],
        };
        let jac = match crate::kernel::engine::system_jacobian(self.sys, box_) {
            Ok(jac) => jac,
            Err(_) => return vec![vec![unbounded(); 4]; 4],
        };
        let hes = match crate::kernel::engine::system_hessian(self.sys, box_) {
            Ok(hes) => hes,
            Err(_) => return vec![vec![unbounded(); 4]; 4],
        };
        let row3 = am_gradient_row(&jac, &hes, self.a);
        let mut rows = Vec::with_capacity(4);
        for r in 0..3 {
            rows.push(vec![jac[r][0], jac[r][1], jac[r][2], jac[r][3]]);
        }
        rows.push(vec![row3[0], row3[1], row3[2], row3[3]]);
        rows
    }
}

// ---------------------------------------------------------------------------
// The §9.2 subdivision start set
// ---------------------------------------------------------------------------

/// The outcome of one Tier-2 start-set attempt (§9.2 / Corollary 9.3).
#[derive(Debug, Clone)]
pub enum TierTwoOutcome {
    /// Every zero of `Ψ_a` in the domain is isolated and the remainder is
    /// excluded: the critical-point start set is complete.
    Complete {
        /// The isolated zeros of `Ψ_a`, each a certified [`PointCert4`]
        /// rebuilt with [`ResidualId::R3`].
        start_set: Vec<PointCert4>,
    },
    /// The start set could not be completed: a named [`Refusal`]
    /// (Inconclusive-backed [`RefusalKind::TangentialCurve`] routing to §10.4,
    /// or [`RefusalKind::IncompleteStartSet`] after the `KA` direction
    /// retries).
    Refused(Refusal),
}

/// Whether a box is a live candidate: its `a·m` enclosure contains `0`, so
/// the §9.2 exclusion (`0 ∉ □(a·m)`) does not clear it.
fn am_contains_zero(psi: &PsiA<'_>, b: &IBox4) -> bool {
    let box_ = [
        (b.lo[0], b.hi[0]),
        (b.lo[1], b.hi[1]),
        (b.lo[2], b.hi[2]),
        (b.lo[3], b.hi[3]),
    ];
    match psi.am_over(&box_) {
        Some(am) => am.contains(0.0),
        None => true,
    }
}

/// Whether the full residual enclosure over the box contains `0` in every
/// component (a box that could, on the interval evidence, hold a zero of
/// `Ψ_a`). A leaf whose residual excludes `0` in any component is
/// root-free even when the operator could not close on it.
fn residual_contains_zero(psi: &PsiA<'_>, b: &IBox4) -> bool {
    let ivs = [
        Interval {
            lo: b.lo[0],
            hi: b.hi[0],
        },
        Interval {
            lo: b.lo[1],
            hi: b.hi[1],
        },
        Interval {
            lo: b.lo[2],
            hi: b.hi[2],
        },
        Interval {
            lo: b.lo[3],
            hi: b.hi[3],
        },
    ];
    psi.eval(&ivs)
        .iter()
        .all(|component| component.contains(0.0))
}

/// Bisect the box along its widest axis (lowest-index tie-break, deterministic
/// order): returns the two closed half-boxes.
fn bisect4(b: &IBox4) -> Vec<IBox4> {
    let mut axis = 0usize;
    let mut width = b.hi[0] - b.lo[0];
    for i in 1..4 {
        let w = b.hi[i] - b.lo[i];
        if w > width {
            width = w;
            axis = i;
        }
    }
    if !(width.is_finite() && width > 0.0) {
        return Vec::new();
    }
    let mid = 0.5 * (b.lo[axis] + b.hi[axis]);
    let mut lo_child = *b;
    let mut hi_child = *b;
    lo_child.hi[axis] = mid;
    hi_child.lo[axis] = mid;
    vec![lo_child, hi_child]
}

/// One §9.2 subdivision attempt over a fixed direction. On each cell the
/// cheap `a·m` exclusion is tried first (`0 ∉ □(a·m)` clears the cell — the
/// N7 cheap form); a cell whose `F` enclosure excludes zero in any component
/// is likewise root-free and cleared (the same sound exclusion the Tier-1
/// boundary search applies, keeping the subdivision on the zero tube); only a
/// cell that survives both exclusions runs the arity-4 square Krawczyk on
/// `Ψ_a`. A `Proven` arm is collected (rebuilt with [`ResidualId::R3`] through
/// the documented one-line seam), a `Disproven` arm clears the cell, and an
/// `Inconclusive` arm subdivides to [`DEPTH_MAX`]; an inconclusive cell at
/// `DEPTH_MAX` is a stall leaf.
///
/// A positive-dimensional `Ψ_a` zero set keeps every descendant of a
/// zero-carrying cell alive, so a genuinely non-isolating search is bounded
/// by [`CELL_BUDGET`] processed cells; on the budget the attempt stops and the
/// remaining live leaves (after the same root-free sweep) are its stalls
/// (`capped = true`). Isolated zeros keep the frontier small, so a complete
/// search finishes inside the budget.
struct Attempt {
    /// The certificates the attempt isolated.
    certs: Vec<PointCert4>,
    /// The stall leaves the attempt could neither isolate nor clear.
    stalls: Vec<IBox4>,
    /// Whether the attempt terminated on the [`CELL_BUDGET`] rather than by
    /// exhausting the subdivision tree (the positive-dimensional signature).
    capped: bool,
}

/// The bounded work of one subdivision attempt (§9.2): how many cells a
/// single direction may process before a non-isolating (positive-dimensional)
/// search is declared rather than subdividing a curve to `DEPTH_MAX`.
const CELL_BUDGET: u64 = 1 << 10;

fn attempt(psi: &PsiA<'_>, w: &[CertifiedPositive], domain: IBox4) -> Attempt {
    let mut certs: Vec<PointCert4> = Vec::new();
    let mut stalls: Vec<IBox4> = Vec::new();
    let mut stack: Vec<(IBox4, u32)> = vec![(domain, 0u32)];
    let mut processed: u64 = 0;
    let mut capped = false;
    while let Some((b, depth)) = stack.pop() {
        processed += 1;
        if processed > CELL_BUDGET {
            capped = true;
            break;
        }
        if !am_contains_zero(psi, &b) {
            continue;
        }
        // The full-residual root-free sweep: a cell whose F enclosure excludes
        // zero in any component contains no zero of Psi_a.
        let box_ivs = [
            Interval {
                lo: b.lo[0],
                hi: b.hi[0],
            },
            Interval {
                lo: b.lo[1],
                hi: b.hi[1],
            },
            Interval {
                lo: b.lo[2],
                hi: b.hi[2],
            },
            Interval {
                lo: b.lo[3],
                hi: b.hi[3],
            },
        ];
        let eval = psi.eval(&box_ivs);
        if eval.len() == 4 && eval.iter().any(|component| !component.contains(0.0)) {
            continue;
        }
        match krawczyk_c1_n4(psi, b, w) {
            ClaimVerdict::Proven(cert) => {
                if let Ok(cert) = PointCert4::try_new(ResidualId::R3, cert.box_, cert.rho) {
                    certs.push(cert);
                }
            }
            ClaimVerdict::Disproven(_) => {}
            ClaimVerdict::Inconclusive(_) => {
                if depth < DEPTH_MAX {
                    for child in bisect4(&b) {
                        stack.push((child, depth + 1));
                    }
                } else {
                    stalls.push(b);
                }
            }
        }
    }
    if capped {
        // The budget stopped the search mid-tree: clear the live leaves the
        // interval evidence shows are root-free, and keep the rest as stalls.
        for (live, _) in stack.into_iter() {
            if residual_contains_zero(psi, &live) {
                stalls.push(live);
            }
        }
    }
    Attempt {
        certs,
        stalls,
        capped,
    }
}

/// The §9.2 Tier-2 start set over a stored [`SquareSystem3`]: subdivide the
/// domain (depth capped at [`DEPTH_MAX`], work capped at [`CELL_BUDGET`])
/// isolating every zero of `Ψ_a(x) = (F(x), a·m(x))` — exclusion first
/// (`0 ∉ □(a·m)`, N7), else the additive arity-4 Krawczyk on `Ψ_a`. On a
/// stall the direction is perturbed and retried up to [`KA`] times with the
/// fixed deterministic table [`A_TABLE`].
///
/// * `Complete { start_set }`: every zero isolated, remainder excluded.
/// * `Refused(TangentialCurve)`: the caller's own direction runs into a
///   positive-dimensional `Ψ_a` zero set (a tangential curve): the bounded
///   search caps because every cell of the shrinking sub-box family still
///   carries zero and nothing isolates — the §10.4 routing, NOT
///   `IncompleteStartSet`.
/// * `Refused(IncompleteStartSet)`: the caller's direction stalls on a bounded
///   set of isolated-but-unresolved leaves, and the `KA` deterministic
///   perturbations cannot complete the start set either.
pub fn tier2_start_set(sys: &SquareSystem3, a: [f64; 4], domain: IBox4) -> TierTwoOutcome {
    let w = match CertifiedPositive::try_new(1.0) {
        Ok(w) => vec![w],
        Err(_) => {
            return TierTwoOutcome::Refused(refusal(
                RefusalKind::NonFinite,
                "tier2_weight_unavailable",
                "the unit §7.1 weight value could not be constructed".to_string(),
            ))
        }
    };
    // The caller's direction first, then the KA deterministic perturbations.
    let mut psi = PsiA::new(sys, a);
    let mut result = attempt(&psi, &w, domain);
    if result.stalls.is_empty() {
        return TierTwoOutcome::Complete {
            start_set: result.certs,
        };
    }
    let first_capped = result.capped;
    for &a_i in &A_TABLE {
        psi = PsiA::new(sys, a_i);
        result = attempt(&psi, &w, domain);
        if result.stalls.is_empty() {
            return TierTwoOutcome::Complete {
                start_set: result.certs,
            };
        }
    }
    // All KA + 1 attempts stalled. A caller direction whose Psi_a zero set is
    // intrinsically positive-dimensional (a tangential curve) caps the bounded
    // search on the first attempt and persists under every perturbation —
    // that routes to §10.4, NOT IncompleteStartSet. A caller direction that
    // stalls on a bounded, isolated-but-unresolved leaf set, and whose KA
    // perturbations cannot complete the start set, is IncompleteStartSet.
    if first_capped {
        TierTwoOutcome::Refused(refusal(
            RefusalKind::TangentialCurve,
            "tier2_persistent_positive_dimensional",
            format!(
                "the Psi_a zero set is positive-dimensional: every cell of the shrinking sub-box family carries zero and no direction of the {} tried isolates it (a tangential curve, routing to §10.4, not IncompleteStartSet)",
                KA + 1
            ),
        ))
    } else {
        TierTwoOutcome::Refused(refusal(
            RefusalKind::IncompleteStartSet,
            "tier2_stall_mixed_after_ka",
            format!(
                "subdivision stalled on {} leaves and the ka={KA} deterministic direction perturbations could not isolate every zero: incomplete start set",
                result.stalls.len()
            ),
        ))
    }
}
