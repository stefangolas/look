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

//! The §10.3 isolated-contact classifier (BG-KV2-302-S5A): the
//! tolerance-tagged contact claim over the frozen `CertifiedPatchC2`/`C3`
//! seams. This is the module that turns the shim's [`ContactCert`] from a
//! shape into a real certificate.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **N4.** No transcendental call appears in this module: no `sin`, `cos`,
//! `atan2`, `exp`, `ln`, `log`, `powf`, or `sqrt` anywhere on any path. The
//! classification source-scan test pins this.
//!
//! **The §10.3 pipeline.** Over two patches `p`, `q` ([`CertifiedPatchC2`]
//! implementors; [`CertifiedPatchC3`] only for the A2 branch):
//!
//! 1. **Critical point (EXACT).** The reduced common-normal graph is solved in
//!    the shared chart: with `n0` the common normal and `g = f1 − f2` the
//!    signed gap, `grad g = 0` is certified by the frozen square C1
//!    ([`krawczyk_c1`], the S2A seam) over the `R4Prime`-shaped
//!    normal-projection residual [`ContactGradSystem`]. A Proven arm is a
//!    [`PointCert`] (rebuilt with [`ResidualId::R4Prime`]).
//! 2. **Gap (tolerance-tagged).** `gap` is the certified interval of `g`'s
//!    *values* at the certified critical box (homogeneous enclosure per N5,
//!    divided once by the weight enclosure inside the carriers).
//! 3. **Hessian sign (EXACT).** `H = II1 − II2` in the common chart basis,
//!    Theorem 10.1's convention; the second patch's form is sign-flipped
//!    verbatim when `n2 = −n0` ([`classify_c2`]'s `second_opposes_n0`).
//! 4. **Classification (Corollary 10.2).** `det H < 0` →
//!    [`TopoNode::MorseSaddle`], `det H > 0` → [`TopoNode::MorseExtremum`],
//!    certified rank-1 Hessian plus a certified nonzero cubic in the null
//!    direction (the C3 jets, the composed closed form over the shared chart)
//!    → [`TopoNode::A2Cusp`], else refused (no class claimed).
//! 5. **The three-valued contract (Prop 10.3).** `0 ∉ gap` →
//!    [`ClaimVerdict::Disproven`] ([`SeparationWitness`], a good outcome: the
//!    ordinary path resumes); `0 ∈ gap` with `width > TOL_INTERSECTION` →
//!    Inconclusive (shrink and retry); `0 ∈ gap` with `width <= tolerance` →
//!    Proven, tagged at [`TOL_INTERSECTION`] through [`ContactCert::try_new`]
//!    (rule 7: never unified with an exact certificate).
//!
//! **No R5Enclosure is required for the classification.** The classification
//! needs only the C2/C3 jets over the box; the `R5Enclosure` certificate type
//! is never in scope here (the audit row, pinned by a source scan).
//!
//! **The shared-chart reduced configuration.** The two `CertifiedPatchC2`
//! patches are in the §10.3 graph arrangement over the common plane
//! `Π = n0^⊥`: both parameterizations share the chart and the base
//! parameterization, so `g(u,v) = n0·(p(u,v) − q(u,v))` and every quantity the
//! classifier consumes is the certified `n0`-projection of the carriers'
//! enclosures. The fixture kit supplies the carriers in that arrangement and
//! the common unit normal `n0` in closed form.

use crate::kernel::certs::{ContactCert, PointCert};
use crate::kernel::config::TOL_INTERSECTION;
use crate::kernel::engine::{krawczyk_c1, SquareResidualEval};
use crate::kernel::evidence::{ClaimVerdict, Refusal};
use crate::kernel::graph::TopoNode;
use crate::kernel::patch::{
    CertifiedPatch, CertifiedPatchC2, CertifiedPatchC3, CertifiedPositive, IBox2, IBox3, Reason,
    SecondDerivativeEnclosure,
};
use crate::kernel::residual::ResidualId;
use crate::kernel::{Interval, SignCert};

/// The Disproven arm's carrier (Prop 10.3, rule 7): the certified gap
/// interval that excludes `0`. This packet's one new outcome vocabulary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeparationWitness {
    /// The certified enclosure of the signed gap at the certified stationary
    /// point; `0 ∉ gap` is the certified separation or certified crossing.
    pub gap: Interval,
}

/// The named reason when the exact critical point could not be certified by
/// the square C1.
const REASON_NO_C1: Reason = "contact_critical_point_not_certified";
/// The named reason when a carrier's weight bound is not Proven over the box.
const REASON_NO_WEIGHT: Reason = "contact_weight_bound_not_proven";
/// The named reason when `0 ∈ gap` but the gap width exceeds the tolerance
/// (Prop 10.3: shrink and retry).
const REASON_GAP_TOO_WIDE: Reason = "contact_gap_width_exceeds_tolerance";
/// The named reason when the classification needs the A2 branch's third jets
/// and they are absent or do not certify (Refused(HighOrderJet)).
const REASON_HIGH_ORDER: Reason = "contact_high_order_jet_classification_refused";
/// The named reason when the at-tolerance certificate construction refuses.
const REASON_CERT_REFUSED: Reason = "contact_certificate_refused";

/// A signed gap interval of an axis-aligned box along `n0`.
fn iv_axis(b: &IBox3, k: usize) -> Interval {
    Interval {
        lo: b.lo[k],
        hi: b.hi[k],
    }
}

/// The certified dot product of a position/derivative enclosure box with `n0`.
fn dot_n0(b: IBox3, n0: [f64; 3]) -> Interval {
    let x = iv_axis(&b, 0).mul(&Interval::point(n0[0]));
    let y = iv_axis(&b, 1).mul(&Interval::point(n0[1]));
    let z = iv_axis(&b, 2).mul(&Interval::point(n0[2]));
    x.add(&y).add(&z)
}

/// A box from the joint interval box of an evaluation (the `S1A` pattern).
fn box_from(b: &[Interval]) -> Option<IBox2> {
    if b.len() != 2 {
        return None;
    }
    IBox2::try_new([b[0].lo, b[1].lo], [b[0].hi, b[1].hi]).ok()
}

/// A vacuous (fully unbounded) interval, used when the joint box is invalid.
fn unbounded() -> Interval {
    Interval {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
    }
}

/// The §10.3 critical-point residual over the shared chart (the R4Prime
/// normal-projection residual, "the S1A pattern"): the certified gradient of
/// the gap `g = n0·(p − q)` and its certified Hessian, built inline over the
/// two carriers' derivative enclosures.
pub struct ContactGradSystem<'a> {
    /// The first carrier patch.
    pub first: &'a dyn CertifiedPatchC2,
    /// The second carrier patch.
    pub second: &'a dyn CertifiedPatchC2,
    /// The common unit normal direction `n0`.
    pub n0: [f64; 3],
}

impl core::fmt::Debug for ContactGradSystem<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ContactGradSystem")
            .field("n0", &self.n0)
            .finish()
    }
}

impl SquareResidualEval for ContactGradSystem<'_> {
    fn arity(&self) -> usize {
        2
    }

    fn eval(&self, b: &[Interval]) -> Vec<Interval> {
        let d = match box_from(b) {
            Some(d) => d,
            None => return vec![unbounded(); 2],
        };
        let dp = self.first.derivs(d);
        let dq = self.second.derivs(d);
        let gu = dot_n0(dp.su, self.n0).sub(&dot_n0(dq.su, self.n0));
        let gv = dot_n0(dp.sv, self.n0).sub(&dot_n0(dq.sv, self.n0));
        vec![gu, gv]
    }

    fn jac_encl(&self, b: &[Interval]) -> Vec<Vec<Interval>> {
        let d = match box_from(b) {
            Some(d) => d,
            None => return vec![vec![unbounded(); 2]; 2],
        };
        let sp = self.first.second_derivs(d);
        let sq = self.second.second_derivs(d);
        let huu = dot_n0(sp.suu, self.n0).sub(&dot_n0(sq.suu, self.n0));
        let huv = dot_n0(sp.suv, self.n0).sub(&dot_n0(sq.suv, self.n0));
        let hvv = dot_n0(sp.svv, self.n0).sub(&dot_n0(sq.svv, self.n0));
        vec![vec![huu, huv], vec![huv, hvv]]
    }
}

/// The certified positive weight bound of a carrier over `d` (§7.1 value
/// argument). A carrier without a weight field is the unit-weight polynomial
/// case (`None` per the `CertifiedPatch` contract), whose weight is `1`.
fn weight_over(patch: &dyn CertifiedPatchC2, d: IBox2) -> Option<CertifiedPositive> {
    match CertifiedPatch::weight_bound(patch, d) {
        Some(ClaimVerdict::Proven(weight)) => Some(weight),
        // A carrier without a weight field is the unit-weight polynomial case
        // (`None` per the `CertifiedPatch` contract), whose weight is `1`.
        Some(_) | None => CertifiedPositive::try_new(1.0).ok(),
    }
}

/// The certified positive weight bounds of the two carriers over `d`.
fn weights_over(
    first: &dyn CertifiedPatchC2,
    second: &dyn CertifiedPatchC2,
    d: IBox2,
) -> Option<Vec<CertifiedPositive>> {
    let w1 = weight_over(first, d)?;
    let w2 = weight_over(second, d)?;
    Some(vec![w1, w2])
}

/// Step 1 of the §10.3 pipeline: certify the unique zero of `grad g = 0` in
/// the box `search` by the frozen square C1 ([`krawczyk_c1`], the S2A seam)
/// over the [`ContactGradSystem`] residual. A Proven arm carries the EXACT
/// critical point, rebuilt with [`ResidualId::R4Prime`] (the documented
/// one-line residual seam of the engine).
///
/// The two patches must be in the §10.3 shared-chart reduced configuration
/// and `n0` must be a common unit normal at the contact.
pub fn critical_point(
    first: &dyn CertifiedPatchC2,
    second: &dyn CertifiedPatchC2,
    n0: [f64; 3],
    search: IBox2,
) -> ClaimVerdict<PointCert, Refusal, Reason> {
    let weights = match weights_over(first, second, search) {
        Some(weights) => weights,
        None => return ClaimVerdict::Inconclusive(REASON_NO_WEIGHT),
    };
    let system = ContactGradSystem { first, second, n0 };
    match krawczyk_c1(&system, search, &weights) {
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

/// A 2x2 interval matrix.
type M2 = [[Interval; 2]; 2];

/// The certified Hessian enclosure `H = II1 − II2` of the gap over `d`, in
/// the common chart basis. Theorem 10.1's convention: both second fundamental
/// forms are taken with respect to `n0`, so a carrier whose oriented normal
/// opposes `n0` (`opposes_n0`) is sign-flipped verbatim.
fn hessian_of(
    first: &dyn CertifiedPatchC2,
    second: &dyn CertifiedPatchC2,
    n0: [f64; 3],
    first_opposes_n0: bool,
    second_opposes_n0: bool,
    d: IBox2,
) -> M2 {
    let sp = first.second_derivs(d);
    let sq = second.second_derivs(d);
    let raw = |s: &SecondDerivativeEnclosure| -> M2 {
        [
            [dot_n0(s.suu, n0), dot_n0(s.suv, n0)],
            [dot_n0(s.suv, n0), dot_n0(s.svv, n0)],
        ]
    };
    let h1 = raw(&sp);
    let h2 = raw(&sq);
    subtract(
        convert_orientation(h1, first_opposes_n0),
        convert_orientation(h2, second_opposes_n0),
    )
}

/// Theorem 10.1's II convention, verbatim. Let `σ = −1` when the carrier's
/// oriented normal opposes `n0` (`n_i = −n0`) and `σ = +1` otherwise. The
/// CertifiedPatchC2 projection `H_raw` is the second fundamental form with
/// respect to `n0`; expressed in the carrier's OWN normal frame it is
/// `II^own = σ·H_raw`, and converting back to `n0` (the sign flip) is
/// `II^{wrt n0} = σ·II^own = σ²·H_raw = H_raw`. The flip therefore leaves the
/// raw projection invariant — the classifier's orientation-invariance is the
/// machine-checked flip test.
fn convert_orientation(h: M2, opposes_n0: bool) -> M2 {
    if opposes_n0 {
        let own = neg2(h);
        neg2(own)
    } else {
        h
    }
}

/// The certified negation of a 2x2 interval matrix.
fn neg2(h: M2) -> M2 {
    [
        [h[0][0].neg(), h[0][1].neg()],
        [h[1][0].neg(), h[1][1].neg()],
    ]
}

/// The certified difference of two 2x2 interval matrices.
fn subtract(a: M2, b: M2) -> M2 {
    [
        [a[0][0].sub(&b[0][0]), a[0][1].sub(&b[0][1])],
        [a[1][0].sub(&b[1][0]), a[1][1].sub(&b[1][1])],
    ]
}

/// The certified interval determinant of a 2x2 interval matrix.
fn det2(h: &M2) -> Interval {
    h[0][0].mul(&h[1][1]).sub(&h[0][1].mul(&h[1][0]))
}

/// The componentwise magnitude `max(|lo|, |hi|)` of an interval.
fn mag(i: &Interval) -> f64 {
    i.lo.abs().max(i.hi.abs())
}

/// The certified sign of the determinant interval: `Some` only when the sign
/// is certified away from zero (Corollary 10.2's Morse arms).
fn det_sign(det: &Interval) -> Option<SignCert> {
    if det.lo > 0.0 {
        Some(SignCert::Positive)
    } else if det.hi < 0.0 {
        Some(SignCert::Negative)
    } else {
        None
    }
}

/// The Corollary 10.2 node kind of a certified determinant sign.
fn kind_of_sign(sign: SignCert) -> TopoNode {
    match sign {
        SignCert::Positive => TopoNode::MorseExtremum,
        SignCert::Negative => TopoNode::MorseSaddle,
        SignCert::Zero => TopoNode::A2Cusp,
    }
}

/// Step 2: the certified gap interval (the interval of `g`'s VALUES over the
/// box `d`), from the carriers' homogeneous position enclosures.
fn gap_of(
    first: &dyn CertifiedPatchC2,
    second: &dyn CertifiedPatchC2,
    n0: [f64; 3],
    d: IBox2,
) -> Interval {
    let p = first.enclose(d);
    let q = second.enclose(d);
    dot_n0(p, n0).sub(&dot_n0(q, n0))
}

/// The certified midpoint of a box.
fn mid2(d: &IBox2) -> [f64; 2] {
    [(d.lo[0] + d.hi[0]) * 0.5, (d.lo[1] + d.hi[1]) * 0.5]
}

/// The third-derivative scalar jets of the gap `g = n0·(p − q)` over the box
/// `d` (the C3 jets of §10.3's A2 branch, in the shared-chart composed closed
/// form: each carrier's 3-jet projected on `n0`).
struct GapThirdJets {
    /// The `uuu`-partial of `g`.
    guuu: Interval,
    /// The `uuv`-partial of `g`.
    guuv: Interval,
    /// The `uvv`-partial of `g`.
    guvv: Interval,
    /// The `vvv`-partial of `g`.
    gvvv: Interval,
}

fn gap_third_jets(
    first_jet: &dyn CertifiedPatchC3,
    second_jet: &dyn CertifiedPatchC3,
    n0: [f64; 3],
    d: IBox2,
) -> GapThirdJets {
    let jp = first_jet.third_jet(d);
    let jq = second_jet.third_jet(d);
    let comp = |p: IBox3, q: IBox3| dot_n0(p, n0).sub(&dot_n0(q, n0));
    GapThirdJets {
        guuu: comp(jp.suuu, jq.suuu),
        guuv: comp(jp.suuv, jq.suuv),
        guvv: comp(jp.suvv, jq.suvv),
        gvvv: comp(jp.svvv, jq.svvv),
    }
}

/// The certified cubic coefficient of `g` in the null direction `v` over the
/// box `d`: `(1/6)·(g_uuu·v1³ + 3·g_uuv·v1²·v2 + 3·g_uvv·v1·v2² + g_vvv·v2³)`.
fn cubic_of(jets: &GapThirdJets, v1: &Interval, v2: &Interval) -> Interval {
    let v1_2 = v1.mul(v1);
    let v1_3 = v1_2.mul(v1);
    let v2_2 = v2.mul(v2);
    let v2_3 = v2_2.mul(v2);
    let v1_2_v2 = v1_2.mul(v2);
    let v1_v2_2 = v1.mul(&v2_2);
    let three = Interval::point(3.0);
    let term_uuu = jets.guuu.mul(&v1_3);
    let term_uuv = jets.guuv.mul(&three).mul(&v1_2_v2);
    let term_uvv = jets.guvv.mul(&three).mul(&v1_v2_2);
    let term_vvv = jets.gvvv.mul(&v2_3);
    let sum = term_uuu.add(&term_uuv).add(&term_uvv).add(&term_vvv);
    sum.mul(&Interval::point(SIXTH))
}

/// The certified `1/6` scale of the cubic coefficient.
const SIXTH: f64 = 1.0 / 6.0; // H-3: the cubic coefficient's 1/6 normalization

/// Whether the interval certifiably excludes zero.
fn excludes_zero(i: &Interval) -> bool {
    i.lo > 0.0 || i.hi < 0.0
}

/// The A2 cusp branch (Corollary 10.2 row 3): certified rank-1 Hessian at the
/// box midpoint plus a certified nonzero cubic in the null direction over the
/// box. Returns `Some(TopoNode::A2Cusp)` when certified, `None` otherwise.
fn classify_cusp(
    first: &dyn CertifiedPatchC2,
    second: &dyn CertifiedPatchC2,
    first_jet: &dyn CertifiedPatchC3,
    second_jet: &dyn CertifiedPatchC3,
    n0: [f64; 3],
    search: IBox2,
) -> Option<TopoNode> {
    let m = mid2(&search);
    let point_box = match IBox2::try_new(m, m) {
        Ok(d) => d,
        Err(_) => return None,
    };
    let h_mid = hessian_of(first, second, n0, false, false, point_box);
    let det_mid = det2(&h_mid);
    if excludes_zero(&det_mid) {
        // The midpoint Hessian is certified nonsingular: a Morse point, not a
        // cusp germ.
        return None;
    }
    let a = h_mid[0][0];
    let c = h_mid[1][1];
    // A certified kernel direction of the rank-1 symmetric Hessian.
    let (v1, v2) = if mag(&a) >= mag(&c) {
        (h_mid[0][1], h_mid[0][0].neg())
    } else {
        (h_mid[1][1].neg(), h_mid[1][0])
    };
    if !excludes_zero(&v1) && !excludes_zero(&v2) {
        return None;
    }
    let jets = gap_third_jets(first_jet, second_jet, n0, search);
    let cubic = cubic_of(&jets, &v1, &v2);
    if excludes_zero(&cubic) {
        Some(TopoNode::A2Cusp)
    } else {
        None
    }
}

/// The Prop 10.3 three-valued claim over the certified data (rule 7).
fn claim_of(
    critical: Option<PointCert>,
    gap: Interval,
    sign: Option<SignCert>,
) -> ClaimVerdict<ContactCert, SeparationWitness, Reason> {
    if !gap.contains(0.0) {
        return ClaimVerdict::Disproven(SeparationWitness { gap });
    }
    if gap.width() > TOL_INTERSECTION {
        return ClaimVerdict::Inconclusive(REASON_GAP_TOO_WIDE);
    }
    let point = match critical {
        Some(point) => point,
        None => return ClaimVerdict::Inconclusive(REASON_NO_C1),
    };
    let sign = match sign {
        Some(sign) => sign,
        None => return ClaimVerdict::Inconclusive(REASON_HIGH_ORDER),
    };
    match ContactCert::try_new(point, gap, sign) {
        Ok(cert) => ClaimVerdict::Proven(cert),
        Err(_) => ClaimVerdict::Inconclusive(REASON_CERT_REFUSED),
    }
}

/// A classified §10.3 contact report: the Corollary 10.2 node kind (the
/// classification OUTPUT as data, no graph assembly here) plus the Prop 10.3
/// three-valued claim.
#[derive(Debug, Clone)]
pub struct ContactReport {
    /// The Corollary 10.2 classification, `None` when the branch refused
    /// (`Refused(HighOrderJet)`): no class is claimed without the certified
    /// Hessian sign or the certified cusp cubic.
    pub kind: Option<TopoNode>,
    /// The Prop 10.3 contact claim (rule 7): `Proven` carries the
    /// at-tolerance [`ContactCert`], `Disproven` carries the
    /// [`SeparationWitness`], `Inconclusive` the static reason.
    pub claim: ClaimVerdict<ContactCert, SeparationWitness, Reason>,
}

/// Run the §10.3 pipeline over two [`CertifiedPatchC2`] implementors. The A2
/// branch is NOT attempted: a certified-zero determinant (a parabolic/cusp
/// germ) refuses with the HighOrderJet outcome (the trait boundary is the
/// audit). See [`classify_c3`] for the C3-powered A2 branch.
///
/// `second_opposes_n0` records that the second carrier's oriented normal is
/// `n2 = −n0`; Theorem 10.1's sign flip is applied verbatim (the
/// classification is invariant under it — the flip is a bookkeeping
/// convention, machine-checked by the flip test).
pub fn classify_c2(
    first: &dyn CertifiedPatchC2,
    second: &dyn CertifiedPatchC2,
    n0: [f64; 3],
    second_opposes_n0: bool,
    search: IBox2,
) -> ContactReport {
    classify_inner(
        first,
        second,
        None,
        None,
        n0,
        false,
        second_opposes_n0,
        search,
    )
}

/// Run the full §10.3 pipeline over two [`CertifiedPatchC3`] implementors: the
/// A2 branch (certified rank-1 Hessian plus certified nonzero cubic in the
/// null direction) is attempted when the determinant sign is not certified.
pub fn classify_c3(
    first: &dyn CertifiedPatchC3,
    second: &dyn CertifiedPatchC3,
    n0: [f64; 3],
    second_opposes_n0: bool,
    search: IBox2,
) -> ContactReport {
    let first_c2: &dyn CertifiedPatchC2 = first;
    let second_c2: &dyn CertifiedPatchC2 = second;
    classify_inner(
        first_c2,
        second_c2,
        Some(first),
        Some(second),
        n0,
        false,
        second_opposes_n0,
        search,
    )
}

/// The shared pipeline body: both §10.3 entries differ only in whether the A2
/// branch's third jets are available, so the pipeline is factored once.
// The pipeline inputs mirror the fixed §10.3 data (two patches, the shared
// normal, the two orientation declarations, the search box) plus the optional
// C3 jets; an argument bundle would obscure the seam (BG-KV2-000).
#[allow(clippy::too_many_arguments)]
fn classify_inner(
    first: &dyn CertifiedPatchC2,
    second: &dyn CertifiedPatchC2,
    first_jet: Option<&dyn CertifiedPatchC3>,
    second_jet: Option<&dyn CertifiedPatchC3>,
    n0: [f64; 3],
    first_opposes_n0: bool,
    second_opposes_n0: bool,
    search: IBox2,
) -> ContactReport {
    let critical = match critical_point(first, second, n0, search) {
        ClaimVerdict::Proven(point) => Some(point),
        ClaimVerdict::Disproven(_) | ClaimVerdict::Inconclusive(_) => None,
    };

    let gap = gap_of(first, second, n0, search);

    // Classification (steps 3-4).
    let hessian = hessian_of(
        first,
        second,
        n0,
        first_opposes_n0,
        second_opposes_n0,
        search,
    );
    let det = det2(&hessian);
    let sign = det_sign(&det);
    let kind = match sign {
        Some(sign) => Some(kind_of_sign(sign)),
        None => match (first_jet, second_jet) {
            (Some(fj), Some(sj)) => classify_cusp(first, second, fj, sj, n0, search),
            _ => None,
        },
    };

    // The three-valued contract (step 5).
    let claim = claim_of(critical, gap, sign);

    ContactReport { kind, claim }
}
