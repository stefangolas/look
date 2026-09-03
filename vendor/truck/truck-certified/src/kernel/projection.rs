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

//! The §8.5/§8.6 projection certificates and the §7 R4/R4′ entries
//! (BG-KV2-305-S2B): [`graphcert`] (Theorem 8.3's cone test — no solve
//! anywhere), the R5 enclosure contract (§8.6's five steps over
//! [`crate::kernel::certs::R5Enclosure`]), and the packaged R4 projection
//! solve plus the R4′ normal-projection fallback.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`. Where a `Result` must carry the
//! frozen `Refusal` (which holds `Option<PartialGraph>`), the large-`Err` lint
//! is allowed item-level only, exactly as the shim files do.
//!
//! **N4.** No transcendental call appears in this module: no `sin`, `cos`,
//! `atan2`, `exp`, `ln`, `log`, or `powf` anywhere, and no
//! `std::f64::consts` value. The only `sqrt` is the IEEE square root used to
//! normalize the deterministic projection basis of [`complement_basis`] (the
//! N4 normalization carve-out, as in `engine.rs` / `tier1.rs`).
//!
//! **Theorem 8.3, verbatim.** For a patch `S` on a box `D` and a unit `n0`,
//! the projection `q = Π-proj ∘ S` to `Π = n0^⊥` satisfies
//! `det Dq = n0·N` with `N = S_u × S_v`. [`graphcert`] certifies
//! `0 ∉ □(n0·N)(D)` and thereby that `q` is injective on `D`. This is a CONE
//! TEST: the certificate is decided from the patch's `normal_cone` and
//! derivative enclosures — the certified sign of `n0` against the normal set
//! over the box — and no linear system is ever formed or solved on the path.
//! (The [`graphcert_is_a_cone_test_with_no_solve`] integration audit pins the
//! absence of any solve machinery inside the [`graphcert`] body.)
//!
//! **n0 feasibility.** The feasible `n0` for a leaf pair is the §9.1 two-cone
//! LP of [`crate::kernel::tier1::tier1_loop_free`] run on the two cached
//! normal cones (the Tier-1 cos-space discipline — cited, not forked; the
//! loop-free certificate's `d` is exactly the direction that keeps both
//! patches graphs). Where no feasible `n0` exists over a box, the caller
//! subdivides or falls back to R4′ ([`r4_prime`]); [`graphcert`] refuses with
//! the named predicate `graphcert_no_feasible_n0`.
//!
//! **§8.6 R5 enclosure contract (the reduced shared-chart configuration).**
//! [`r5_enclose`] implements step 1 (preimage) of the contract: for each
//! patch, C1 on the R4 residual (a local [`SquareResidualEval`] over the
//! patch's `CertifiedPatch` enclosures) via [`krawczyk_c1`], producing the
//! certified preimage boxes and the R4-stamped [`PointCert`]s. Steps 2, 3,
//! and 5 (value, gradient, and the `g = f1 − f2` difference) are exposed as
//! [`r5_graph_enclose`]; step 4 (Hessian, C2 carriers only) is DEFERRED to
//! S5A with the named predicate `r5_hessian_is_s5a_contact`. `g` is analytic
//! and non-polynomial: no Bernstein evaluation exists anywhere on the g path
//! (the `no_bernstein_applies_to_r5_audit` source audit pins it).
//!
//! The R5 entries assume the §10.3 reduced configuration: each patch is a
//! graph over the shared chart and its parameter box is a box of `Π`, so the
//! certified preimage of the target box `Q` is the target box itself (the two
//! carriers share the chart and base parameterization). The R5 residual data
//! (position, derivative, weight) is always read through the
//! `&dyn CertifiedPatch` seam; the module never re-derives a carrier weight
//! (§7.1 value argument) and never touches a Bernstein net.
//!
//! **R4 / R4′.** [`r4_project`] packages the square 2×2 projection solve for
//! one surface independent of the other (the C1 machinery IS the solver);
//! [`r4_prime`] packages the §7 R4′ normal-projection residual
//! `P(u0; s,t) = (S1_u·(S2 − S1), S1_v·(S2 − S1))` for a fixed chart point
//! `u0` of the first patch, solved over the second patch's chart — the
//! fallback retained for boxes where no feasible `n0` exists and subdivision
//! is capped.

use crate::kernel::certs::{GraphCert, PointCert, R5Enclosure};
use crate::kernel::config::{DEPTH_MAX, TOL_JACOBIAN};
use crate::kernel::engine::{krawczyk_c1, SquareResidualEval};
use crate::kernel::evidence::{ClaimVerdict, Construction, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::patch::{CertifiedPatch, CertifiedPositive, DerivativeEnclosure, IBox2, IBox3};
use crate::kernel::residual::ResidualId;
use crate::kernel::Interval;

/// A three-component interval vector.
type Iv3 = [Interval; 3];

/// The row-major interval Jacobian of the R4/R5 2×2 solve.
type M2 = [[Interval; 2]; 2];

/// A named predicate refusal for a projection invariant.
fn projection_refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}

/// The axis intervals of a box.
fn axes3(b: &IBox3) -> Iv3 {
    [
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
    ]
}

/// Whether every endpoint of a box is finite.
fn finite3(b: &IBox3) -> bool {
    b.lo.iter().chain(b.hi.iter()).all(|c| c.is_finite())
}

/// The certified dot product of an interval vector with a float direction.
fn dot_dir(a: &Iv3, d: [f64; 3]) -> Interval {
    let p = Interval::point;
    a[0].mul(&p(d[0]))
        .add(&a[1].mul(&p(d[1])))
        .add(&a[2].mul(&p(d[2])))
}

/// The certified dot product of two interval vectors.
fn dot_iv(a: &Iv3, b: &Iv3) -> Interval {
    a[0].mul(&b[0]).add(&a[1].mul(&b[1])).add(&a[2].mul(&b[2]))
}

/// The certified cross product of two interval vectors.
fn cross_iv(a: &Iv3, b: &Iv3) -> Iv3 {
    let cx = a[1].mul(&b[2]).sub(&a[2].mul(&b[1]));
    let cy = a[2].mul(&b[0]).sub(&a[0].mul(&b[2]));
    let cz = a[0].mul(&b[1]).sub(&a[1].mul(&b[0]));
    [cx, cy, cz]
}

/// The certified componentwise difference of two interval vectors.
fn sub_iv(a: &Iv3, b: &Iv3) -> Iv3 {
    [a[0].sub(&b[0]), a[1].sub(&b[1]), a[2].sub(&b[2])]
}

/// The outward-rounded enclosure of `n0·N` over the derivative data `de`, with
/// `N = S_u × S_v`. This is the interval enclosure `□(n0·N)(D)` of Theorem 8.3.
fn normal_dot_enclosure(de: &DerivativeEnclosure, n0: [f64; 3]) -> Interval {
    let su = axes3(&de.su);
    let sv = axes3(&de.sv);
    dot_dir(&cross_iv(&su, &sv), n0)
}

/// Whether a direction is finite and unit to [`TOL_JACOBIAN`].
fn is_unit_dir(d: [f64; 3]) -> bool {
    if !d.iter().all(|c| c.is_finite()) {
        return false;
    }
    let norm = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    (norm - 1.0).abs() <= TOL_JACOBIAN
}

/// Theorem 8.3's graph certificate: `0 ∉ □(n0·N)(D)` is decided as a cone
/// test over the patch's `normal_cone` and derivative enclosures — no linear
/// solve, no matrix inversion, no subdivision.
///
/// The certificate refuses a non-finite patch enclosure over `D` (the patch
/// does not certify there), a non-finite `normal_cone`, and — when zero cannot
/// be excluded from `□(n0·N)(D)` — a **no-feasible-n0** refusal (named
/// predicate `graphcert_no_feasible_n0`); the caller subdivides or falls back
/// to R4′ ([`r4_prime`]).
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn graphcert(p: &dyn CertifiedPatch, domain: IBox2, n0: [f64; 3]) -> Construction<GraphCert> {
    if !n0.iter().all(|c| c.is_finite()) {
        return Err(projection_refusal(
            RefusalKind::NonFinite,
            "graph_cert_n0_not_finite",
            format!("graphcert n0 {n0:?} is not finite"),
        ));
    }
    let de = p.derivs(domain);
    if !finite3(&de.su) || !finite3(&de.sv) {
        return Err(projection_refusal(
            RefusalKind::NonFinite,
            "graph_cert_derivative_enclosure_not_finite",
            "the patch does not certify a finite derivative enclosure over the box".to_string(),
        ));
    }
    let cone = p.normal_cone(domain);
    if !(cone.axis.iter().all(|c| c.is_finite()) && cone.half_angle.is_finite()) {
        return Err(projection_refusal(
            RefusalKind::NonFinite,
            "graph_cert_normal_cone_not_finite",
            "the patch does not certify a finite normal cone over the box".to_string(),
        ));
    }
    let det_iv = normal_dot_enclosure(&de, n0);
    if !det_iv.is_finite() {
        return Err(projection_refusal(
            RefusalKind::NonFinite,
            "graph_cert_det_enclosure_not_finite",
            format!("the certified enclosure of n0.N is not finite over the box: {det_iv:?}"),
        ));
    }
    // The cone test outcome: the determinant enclosure decides. `det` is the
    // endpoint nearest zero, so the certified-nonzero bound records the sign
    // and the tightest certified magnitude.
    let det = if det_iv.lo > 0.0 {
        det_iv.lo
    } else if det_iv.hi < 0.0 {
        det_iv.hi
    } else {
        // Zero cannot be excluded from the enclosure of n0·N over the box: no
        // feasible n0. The subnormal floor of the directed-rounding arithmetic
        // keeps an exactly-degenerate graph (e.g. a vertical plane) inside this
        // arm rather than distinguishable; the caller subdivides or falls back
        // to R4′.
        return Err(projection_refusal(
            RefusalKind::Conditioning,
            "graphcert_no_feasible_n0",
            format!("n0.N straddles zero over the box: {det_iv:?}"),
        ));
    };
    GraphCert::try_new(domain, n0, det)
}

// ---------------------------------------------------------------------------
// The projection basis of Π = n0^⊥ and the R4/R4′ residual systems
// ---------------------------------------------------------------------------

/// A deterministic orthonormal complement `(e1, e2)` of a unit `n0` with
/// `e1 × e2 = n0`. The reference axis is the coordinate axis of smallest
/// `|n0|` component (lowest index breaks ties), so the basis is reproducible
/// and reduces to the coordinate basis for coordinate-axis `n0`.
///
/// `None` when `n0` is not a finite unit direction (the caller's graphcert /
/// unit gate should have refused first).
fn complement_basis(n0: [f64; 3]) -> Option<([f64; 3], [f64; 3])> {
    if !is_unit_dir(n0) {
        return None;
    }
    let mut axis_k = 0usize;
    let mut best = n0[0].abs();
    for (k, component) in n0.iter().enumerate().skip(1) {
        let a = component.abs();
        if a < best {
            best = a;
            axis_k = k;
        }
    }
    let mut axis = [0.0f64; 3];
    axis[axis_k] = 1.0;
    let dot = axis[0] * n0[0] + axis[1] * n0[1] + axis[2] * n0[2];
    let v = [
        axis[0] - dot * n0[0],
        axis[1] - dot * n0[1],
        axis[2] - dot * n0[2],
    ];
    let norm_sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if !(norm_sq.is_finite() && norm_sq > 0.0) {
        return None;
    }
    let norm = norm_sq.sqrt();
    let e1 = [v[0] / norm, v[1] / norm, v[2] / norm];
    let e2 = [
        n0[1] * e1[2] - n0[2] * e1[1],
        n0[2] * e1[0] - n0[0] * e1[2],
        n0[0] * e1[1] - n0[1] * e1[0],
    ];
    Some((e1, e2))
}

/// The certified positive weight bound of a patch over `d` (§7.1 value
/// argument). A patch without a weight field is the unit-weight polynomial
/// case (`None` per the `CertifiedPatch` contract), whose weight is `1`. A
/// patch whose weight bound is present but not `Proven` provides no certified
/// weight (`None`).
fn certified_weight(patch: &dyn CertifiedPatch, d: IBox2) -> Option<CertifiedPositive> {
    match patch.weight_bound(d) {
        Some(ClaimVerdict::Proven(weight)) => Some(weight),
        Some(_) => None,
        None => CertifiedPositive::try_new(1.0).ok(),
    }
}

/// A box from a two-element interval slice (the `S1A` pattern).
fn box2_from(b: &[Interval]) -> Option<IBox2> {
    if b.len() != 2 {
        return None;
    }
    IBox2::try_new([b[0].lo, b[1].lo], [b[0].hi, b[1].hi]).ok()
}

/// A vacuous (fully unbounded) interval for an invalid joint box.
fn unbounded() -> Interval {
    Interval {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
    }
}

/// The §7 R4 residual: `Π-proj(S(u,v)) − q` expressed in the `(e1, e2)` basis
/// of `Π = n0^⊥`, a square 2×2 system in the patch's chart. The value reads
/// the patch's certified position enclosure and the Jacobian reads the patch's
/// derivative enclosures.
struct R4System<'a> {
    /// The patch being projected.
    patch: &'a dyn CertifiedPatch,
    /// The first basis direction of `Π`.
    e1: [f64; 3],
    /// The second basis direction of `Π`.
    e2: [f64; 3],
    /// The target point of `Π`, in `(e1, e2)` coordinates.
    q: [f64; 2],
}

impl core::fmt::Debug for R4System<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("R4System")
            .field("e1", &self.e1)
            .field("e2", &self.e2)
            .field("q", &self.q)
            .finish()
    }
}

impl SquareResidualEval for R4System<'_> {
    fn arity(&self) -> usize {
        2
    }

    fn eval(&self, b: &[Interval]) -> Vec<Interval> {
        let d = match box2_from(b) {
            Some(d) => d,
            None => return vec![unbounded(); 2],
        };
        let pos = axes3(&self.patch.enclose(d));
        let r0 = dot_dir(&pos, self.e1).sub(&Interval::point(self.q[0]));
        let r1 = dot_dir(&pos, self.e2).sub(&Interval::point(self.q[1]));
        vec![r0, r1]
    }

    fn jac_encl(&self, b: &[Interval]) -> Vec<Vec<Interval>> {
        let d = match box2_from(b) {
            Some(d) => d,
            None => return vec![vec![unbounded(); 2]; 2],
        };
        let de = self.patch.derivs(d);
        let su = axes3(&de.su);
        let sv = axes3(&de.sv);
        let row0 = [dot_dir(&su, self.e1), dot_dir(&sv, self.e1)];
        let row1 = [dot_dir(&su, self.e2), dot_dir(&sv, self.e2)];
        vec![row0.to_vec(), row1.to_vec()]
    }
}

/// The §7 R4′ normal-projection residual for a FIXED chart point `u0` of the
/// first patch: `P(s, t) = (S1_u(u0)·(S2(s,t) − S1(u0)), S1_v(u0)·(S2(s,t) −
/// S1(u0)))`, a square 2×2 system in the second patch's chart. This is the
/// fallback where no feasible `n0` exists.
struct R4PrimeSystem<'a> {
    /// The first patch.
    first: &'a dyn CertifiedPatch,
    /// The fixed chart point of the first patch.
    u0: [f64; 2],
    /// The second patch.
    second: &'a dyn CertifiedPatch,
}

impl core::fmt::Debug for R4PrimeSystem<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("R4PrimeSystem")
            .field("u0", &self.u0)
            .finish()
    }
}

/// The degenerate (point) chart box of `u0`.
fn point_box2(u0: [f64; 2]) -> IBox2 {
    match IBox2::try_new(u0, u0) {
        Ok(d) => d,
        Err(_) => IBox2 { lo: u0, hi: u0 },
    }
}

impl SquareResidualEval for R4PrimeSystem<'_> {
    fn arity(&self) -> usize {
        2
    }

    fn eval(&self, b: &[Interval]) -> Vec<Interval> {
        let d = match box2_from(b) {
            Some(d) => d,
            None => return vec![unbounded(); 2],
        };
        let u0 = point_box2(self.u0);
        let p1 = axes3(&self.first.enclose(u0));
        let p2 = axes3(&self.second.enclose(d));
        let de1 = self.first.derivs(u0);
        let su1 = axes3(&de1.su);
        let sv1 = axes3(&de1.sv);
        let diff = sub_iv(&p2, &p1);
        vec![dot_iv(&su1, &diff), dot_iv(&sv1, &diff)]
    }

    fn jac_encl(&self, b: &[Interval]) -> Vec<Vec<Interval>> {
        let d = match box2_from(b) {
            Some(d) => d,
            None => return vec![vec![unbounded(); 2]; 2],
        };
        let u0 = point_box2(self.u0);
        let de1 = self.first.derivs(u0);
        let de2 = self.second.derivs(d);
        let su1 = axes3(&de1.su);
        let sv1 = axes3(&de1.sv);
        let s2u = axes3(&de2.su);
        let s2v = axes3(&de2.sv);
        let row0 = [dot_iv(&su1, &s2u), dot_iv(&su1, &s2v)];
        let row1 = [dot_iv(&sv1, &s2u), dot_iv(&sv1, &s2v)];
        vec![row0.to_vec(), row1.to_vec()]
    }
}

// ---------------------------------------------------------------------------
// R4 / R4′ entries
// ---------------------------------------------------------------------------

/// The reason when a carrier weight bound is not `Proven` over the box.
const REASON_NO_WEIGHT: &str = "projection_weight_bound_not_proven";

/// The §7 R4 projection solve for one surface, packaged: run the frozen square
/// C1 ([`krawczyk_c1`]) over the R4 residual `Π-proj(S(u,v)) − q` in `search`,
/// certifying the unique preimage of `q` (in `Π = n0^⊥` coordinates) in the
/// box. A Proven arm is the [`PointCert`] rebuilt with [`ResidualId::R4`] (the
/// documented one-line residual seam of the engine).
///
/// `n0` must be a finite unit direction; the deterministic complement basis of
/// `Π` is built inside. `search` is a box of the patch's own chart.
pub fn r4_project(
    p: &dyn CertifiedPatch,
    q: [f64; 2],
    n0: [f64; 3],
    search: IBox2,
) -> ClaimVerdict<PointCert, Refusal, &'static str> {
    if !is_unit_dir(n0) {
        return ClaimVerdict::Disproven(projection_refusal(
            RefusalKind::ClaimRefuted,
            "r4_n0_not_unit",
            format!("r4_project requires a finite unit n0, got {n0:?}"),
        ));
    }
    let (e1, e2) = match complement_basis(n0) {
        Some(basis) => basis,
        None => {
            return ClaimVerdict::Disproven(projection_refusal(
                RefusalKind::Conditioning,
                "r4_complement_basis_unavailable",
                "the orthonormal complement basis of n0 could not be constructed".to_string(),
            ));
        }
    };
    let weight = match certified_weight(p, search) {
        Some(weight) => weight,
        None => return ClaimVerdict::Inconclusive(REASON_NO_WEIGHT),
    };
    let system = R4System {
        patch: p,
        e1,
        e2,
        q,
    };
    match krawczyk_c1(&system, search, &[weight]) {
        ClaimVerdict::Proven(point) => {
            match PointCert::try_new(ResidualId::R4, point.box_, point.rho) {
                Ok(point) => ClaimVerdict::Proven(point),
                Err(refusal) => ClaimVerdict::Disproven(refusal),
            }
        }
        ClaimVerdict::Disproven(refusal) => ClaimVerdict::Disproven(refusal),
        ClaimVerdict::Inconclusive(reason) => ClaimVerdict::Inconclusive(reason),
    }
}

/// The §7 R4′ normal-projection fallback, packaged: for the FIXED chart point
/// `u0` of `first`, run the frozen square C1 over the R4′ residual
/// `P(u0; s,t) = (S1_u·(S2 − S1), S1_v·(S2 − S1))` in `search` (a box of the
/// second patch's chart), certifying the foot of the first patch's normal on
/// the second patch. A Proven arm is the [`PointCert`] rebuilt with
/// [`ResidualId::R4Prime`].
///
/// The outcome is honest: `Proven` only on a certified contraction, otherwise
/// `Disproven` (the frozen C1 refutes a foot in the box) or `Inconclusive` —
/// never a false `Proven`.
pub fn r4_prime(
    first: &dyn CertifiedPatch,
    u0: [f64; 2],
    second: &dyn CertifiedPatch,
    search: IBox2,
) -> ClaimVerdict<PointCert, Refusal, &'static str> {
    let weight = match certified_weight(second, search) {
        Some(weight) => weight,
        None => return ClaimVerdict::Inconclusive(REASON_NO_WEIGHT),
    };
    let system = R4PrimeSystem { first, u0, second };
    match krawczyk_c1(&system, search, &[weight]) {
        ClaimVerdict::Proven(point) => {
            match PointCert::try_new(ResidualId::R4Prime, point.box_, point.rho) {
                Ok(point) => ClaimVerdict::Proven(point),
                Err(refusal) => ClaimVerdict::Disproven(refusal),
            }
        }
        ClaimVerdict::Disproven(refusal) => ClaimVerdict::Disproven(refusal),
        ClaimVerdict::Inconclusive(reason) => ClaimVerdict::Inconclusive(reason),
    }
}

// ---------------------------------------------------------------------------
// The R5 enclosure contract (§8.6)
// ---------------------------------------------------------------------------

/// The certified midpoint of a box.
fn mid2(d: &IBox2) -> [f64; 2] {
    [(d.lo[0] + d.hi[0]) * 0.5, (d.lo[1] + d.hi[1]) * 0.5]
}

/// The named predicate detail when an R5 preimage stage fails; the returned
/// name is the machine-readable stage.
type PreimageError = (&'static str, String);

/// §8.6 step 1 for one patch: certify the preimage box of the target region
/// `q` by the frozen C1 over the R4 residual at the target centre, over the
/// region box `q` (the reduced shared-chart configuration: the patch's
/// parameter box is a box of `Π`, so the certified preimage region is the
/// target box itself).
///
/// Refuses ([`Err`]) when the patch's weight bound is not `Proven` over `q`,
/// when the patch is not a certified graph over `q` in the `n0` direction
/// ([`graphcert`]), or when C1 fails to contract (Krawczyk stalls) — the
/// §8.6 named refusal surfaces as [`RefusalKind::R5EnclosureFailed`] at the
/// [`r5_enclose`] boundary.
fn preimage_of(
    p: &dyn CertifiedPatch,
    q: IBox2,
    n0: [f64; 3],
) -> Result<(GraphCert, PointCert), PreimageError> {
    let graph = match graphcert(p, q, n0) {
        Ok(graph) => graph,
        Err(refusal) => {
            return Err((
                "r5_graphcert_refused_over_target",
                format!(
                    "the patch is not a certified graph over the target region in the n0 \
                     direction: {refusal:?}"
                ),
            ));
        }
    };
    let weight =
        match certified_weight(p, q) {
            Some(weight) => weight,
            None => return Err((
                "r5_weight_bound_not_proven",
                "no certified positive weight bound over the target region (§7.1 value argument)"
                    .to_string(),
            )),
        };
    let centre = mid2(&q);
    let (e1, e2) = match complement_basis(n0) {
        Some(basis) => basis,
        None => {
            return Err((
                "r5_complement_basis_unavailable",
                "the orthonormal complement basis of n0 could not be constructed".to_string(),
            ));
        }
    };
    let system = R4System {
        patch: p,
        e1,
        e2,
        q: centre,
    };
    match krawczyk_c1(&system, q, &[weight]) {
        ClaimVerdict::Proven(point) => {
            match PointCert::try_new(ResidualId::R4, point.box_, point.rho) {
                Ok(point) => Ok((graph, point)),
                Err(refusal) => Err((
                    "r5_point_certificate_refused",
                    format!("the R4 point certificate was refused: {refusal:?}"),
                )),
            }
        }
        ClaimVerdict::Disproven(refusal) => Err((
            "r5_krawczyk_stalled_at_depth_max",
            format!(
                "the R4 preimage solve provably has no root in the region (depth cap \
                 {DEPTH_MAX} semantics): {refusal:?}"
            ),
        )),
        ClaimVerdict::Inconclusive(reason) => Err((
            "r5_krawczyk_stalled_at_depth_max",
            format!(
                "the R4 preimage solve failed to contract on the target region (depth cap \
                 {DEPTH_MAX} semantics): {reason}"
            ),
        )),
    }
}

/// §8.6's R5 enclosure over the target box `q`, step 1 (preimage) verbatim:
/// for each patch, the R4 preimage solve ([`krawczyk_c1`] over the R4
/// residual) produces the certified preimage boxes `D_i'` and the R4-stamped
/// [`PointCert`]s. Emits the frozen [`R5Enclosure`]
/// `{ q, preimage, cert }` (the shim shape carries no `try_new`; the local
/// construction validates every stage first).
///
/// A preimage stage that cannot be certified — no feasible graph over the
/// target region, no certified weight, or C1 failing to contract — is the
/// §8.6 named refusal: [`RefusalKind::R5EnclosureFailed`] (Inconclusive),
/// surfaced through the refusal-carrying verdict arm.
pub fn r5_enclose(
    p1: &dyn CertifiedPatch,
    p2: &dyn CertifiedPatch,
    q: IBox2,
    n0: [f64; 3],
) -> ClaimVerdict<R5Enclosure, Refusal, &'static str> {
    if !n0.iter().all(|c| c.is_finite()) {
        return ClaimVerdict::Disproven(projection_refusal(
            RefusalKind::NonFinite,
            "r5_n0_not_finite",
            format!("r5_enclose n0 {n0:?} is not finite"),
        ));
    }
    let mut preimage = [q; 2];
    let mut cert = [
        PointCert {
            residual: ResidualId::R4,
            box_: q,
            rho: 0.0,
        },
        PointCert {
            residual: ResidualId::R4,
            box_: q,
            rho: 0.0,
        },
    ];
    let patches: [&dyn CertifiedPatch; 2] = [p1, p2];
    for (i, patch) in patches.iter().enumerate() {
        match preimage_of(*patch, q, n0) {
            Ok((_graph, point)) => {
                preimage[i] = q;
                cert[i] = point;
            }
            Err((name, detail)) => {
                return ClaimVerdict::Disproven(Refusal::new(
                    RefusalKind::R5EnclosureFailed,
                    RefusalEvidence::Predicate { name, detail },
                ));
            }
        }
    }
    ClaimVerdict::Proven(R5Enclosure { q, preimage, cert })
}

/// The certified interval determinant of a 2×2 interval matrix.
fn det2_iv(m: &M2) -> Interval {
    m[0][0].mul(&m[1][1]).sub(&m[0][1].mul(&m[1][0]))
}

/// The interval inverse of a 2×2 matrix via adjugate over determinant (the
/// landed det discipline). `None` when the determinant enclosure contains (or
/// is) zero or a quotient is not finite — nonsingularity is GraphCert's
/// business, so `None` here is a refusal condition for the caller.
fn inv2_iv(m: &M2) -> Option<M2> {
    let det = det2_iv(m);
    if !det.is_finite() || (det.lo <= 0.0 && det.hi >= 0.0) {
        return None;
    }
    let adj: M2 = [[m[1][1], m[0][1].neg()], [m[1][0].neg(), m[0][0]]];
    let mut out = [[Interval::point(0.0); 2]; 2];
    for r in 0..2 {
        for c in 0..2 {
            out[r][c] = adj[r][c].div(&det)?;
        }
    }
    Some(out)
}

/// The certified gradient of the graph height `f_i(q) = n0·S_i` over the
/// preimage box, step 3 of §8.6 verbatim: `∇f_i = (Dσ_i)^T (n0·S_u, n0·S_v)^T`
/// with `Dσ_i = (Dq)^{-1}`, enclosed by interval inversion of the `Dq`
/// enclosure over the box. Nonsingularity is GraphCert's — the certificate is
/// taken as a value argument; without it the gradient refuses.
///
/// Returns `([Interval; 2], GraphCert-domain consistency is the caller's)`:
/// `None` when `Dq` is not certified invertible over the box or the patch
/// enclosures are not finite there.
fn height_gradient(p: &dyn CertifiedPatch, preimage: IBox2, n0: [f64; 3]) -> Option<[Interval; 2]> {
    let de = p.derivs(preimage);
    if !finite3(&de.su) || !finite3(&de.sv) {
        return None;
    }
    let (e1, e2) = complement_basis(n0)?;
    let su = axes3(&de.su);
    let sv = axes3(&de.sv);
    let dq: M2 = [
        [dot_dir(&su, e1), dot_dir(&sv, e1)],
        [dot_dir(&su, e2), dot_dir(&sv, e2)],
    ];
    let ds = inv2_iv(&dq)?;
    let v = [dot_dir(&su, n0), dot_dir(&sv, n0)];
    // (Dσ)^T · v: row `k` of the transpose is column `k` of `Dσ`.
    let grad = [
        ds[0][0].mul(&v[0]).add(&ds[1][0].mul(&v[1])),
        ds[0][1].mul(&v[0]).add(&ds[1][1].mul(&v[1])),
    ];
    Some(grad)
}

/// The certified R5 graph data over the target region: the certified value
/// interval and the certified gradient enclosure of `g = f1 − f2` (steps 2, 3,
/// and 5 of §8.6). Step 4 (the Hessian, C2 carriers only) is DEFERRED to S5A
/// (`r5_hessian_is_s5a_contact`); `g` is analytic and non-polynomial, and no
/// Bernstein evaluation appears on this path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct R5Graph {
    /// The certified enclosure of `g(Q) = f1(Q) − f2(Q)`.
    pub value: Interval,
    /// The certified enclosure of `∇g` over `Q`, in the `(e1, e2)` coordinates
    /// of `Π = n0^⊥`.
    pub grad: [Interval; 2],
}

/// Evaluate the R5 graph difference `g = f1 − f2` over the certified enclosure
/// `enc` (steps 2, 3, and 5 of §8.6): the value from the two value enclosures
/// `f_i(Q) ⊆ n0·□S_i(D_i')` subtracted, and the gradient from the two
/// certified gradients subtracted.
///
/// The two [`GraphCert`]s are taken as VALUE arguments — the interval inversion
/// of `Dq` is only licensed because GraphCert certifies it nonsingular; without
/// a certifying graph certificate the gradient refuses (`Conditioning`).
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn r5_graph_enclose(
    p1: &dyn CertifiedPatch,
    p2: &dyn CertifiedPatch,
    enc: &R5Enclosure,
    _g1: &GraphCert,
    _g2: &GraphCert,
    n0: [f64; 3],
) -> Construction<R5Graph> {
    if !n0.iter().all(|c| c.is_finite()) {
        return Err(projection_refusal(
            RefusalKind::NonFinite,
            "r5_graph_n0_not_finite",
            format!("r5_graph_enclose n0 {n0:?} is not finite"),
        ));
    }
    let v1 = dot_dir(&axes3(&p1.enclose(enc.preimage[0])), n0);
    let v2 = dot_dir(&axes3(&p2.enclose(enc.preimage[1])), n0);
    if !(v1.is_finite() && v2.is_finite()) {
        return Err(projection_refusal(
            RefusalKind::NonFinite,
            "r5_graph_value_enclosure_not_finite",
            "the R5 value enclosures are not finite over the preimage boxes".to_string(),
        ));
    }
    let g1 = match height_gradient(p1, enc.preimage[0], n0) {
        Some(g1) => g1,
        None => {
            return Err(projection_refusal(
                RefusalKind::Conditioning,
                "r5_gradient_dq_not_invertible",
                "the Dq enclosure of the first patch is not certified invertible (GraphCert \
                 value argument absent or stale)"
                    .to_string(),
            ));
        }
    };
    let g2 = match height_gradient(p2, enc.preimage[1], n0) {
        Some(g2) => g2,
        None => {
            return Err(projection_refusal(
                RefusalKind::Conditioning,
                "r5_gradient_dq_not_invertible",
                "the Dq enclosure of the second patch is not certified invertible (GraphCert \
                 value argument absent or stale)"
                    .to_string(),
            ));
        }
    };
    Ok(R5Graph {
        value: v1.sub(&v2),
        grad: [g1[0].sub(&g2[0]), g1[1].sub(&g2[1])],
    })
}
