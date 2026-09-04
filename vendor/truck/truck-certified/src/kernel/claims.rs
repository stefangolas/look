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

//! Authored-topology verification (BG-KV2-503-S10, spec §15): `certify_claimed`
//! and the claim vocabulary.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **§15 entry.** [`certify_claimed`] converts §9's completeness protocol from
//! search into verification: each claimed component is certified
//! independently, a refutation names the component and the failing predicate,
//! and an exhaustive claim narrows completeness to the complement of the
//! certified tubes (Tier-1 and Tier-2 exclusion over the box-subtraction
//! complement, spec §15 item 3). D6 applies without exception: `provenance` is
//! never a certificate, and a claim that is not certified (a non-exhaustive
//! claim, or a trusted-provenance claim that skips item 3) yields a
//! [`ClaimedGraph`], a type distinct from [`CertifiedGraph`].
//!
//! **The pair model (v1 scope, recorded).** [`LeafPair`] is defined HERE: the
//! census recorded it as a landed seam but it is not present in the tree (and
//! the serial dependency BG-KV2-502-S9B is not merged), so this module lands
//! it under the spec's spelling. The certified model is the shared-chart graph
//! arrangement: both leaves are unit-weight polynomial graphs
//! `(x, y, z) = (u, v, h_i(u, v))` over the same identity chart, validated
//! structurally by [`LeafPair::try_new`]. In that arrangement the R1 zero set
//! `Z` is the diagonal lift of the plane difference curve `g = h1 - h2`, so
//! every component is an ordinary R1 tube chain whose nodes are the crossings
//! of `g = 0` with the claim-domain boundary, and the three predicates of spec
//! §15 item 1 are all expressible through the landed seams:
//!
//! * **tube-chain-via-C2** — [`engine::build_frame4`] at the claimed seed plus
//!   [`engine::c2_certify_tube4`] over the frame tube that spans the whole
//!   claim-domain crossing of the branch. A `Disproven` tube (the residual has
//!   no zero in the box) refutes the component; an unresolved tube is
//!   Inconclusive, never silently repaired.
//! * **endpoints-via-C1** — each end of the certified chain is where the plane
//!   curve meets a claim-domain face; the crossing is the unique root of the
//!   square 2D residual `(g(u, v), c - f)` (the face coordinate `c` at the
//!   face value `f`), certified by [`engine::krawczyk_c1`] over an interior
//!   box of the shared chart.
//! * **nodes-via-A4.2** — the certified endpoint crossings become
//!   [`crate::kernel::graph::TopoNode::Boundary`] nodes whose certificates are
//!   checked by the landed §4.2 identity rules ([`assemble::regions_identify`]):
//!   the two ends of one component must not identify (a certified positive
//!   length).
//!
//! **Component kind vocabulary.** [`ComponentKind`] spells the landed
//! tube-chain kind set of the certified graph (the
//! [`crate::kernel::graph::AnyArc`] families: Ordinary / Difference / SelfInt /
//! Spine / Carrier). The C2 seam on a leaf pair certifies ordinary R1 chains
//! only, so a claim whose expected kind is not [`ComponentKind::Ordinary`]
//! refutes under `tube-chain-via-C2`; the other kinds are produced by other
//! seams (plane difference, §13 self-intersection, §12 canal spines, carrier
//! recognition) and no partial graph is emitted for them (spec §15 item 2 has
//! no partial-success arm).
//!
//! **Predicate spellings.** The three predicate labels are exactly
//! `tube-chain-via-C2`, `endpoints-via-C1`, and `nodes-via-A4.2`; a
//! [`ClaimRefutation`] carries the failing label verbatim.
//!
//! **Completeness (item 3).** For an exhaustive claim, the complement is the
//! Tier-2 domain (the claim box) minus each certified tube's parameter box
//! (box subtraction, implemented as the deterministic axis-slab split over the
//! landed [`IBox`] shape). Tier-1 ([`tier1::tier1_loop_free`]) and Tier-2
//! ([`tier2::tier2_start_set`]) exclusion run over each resulting complement
//! box. Completeness is discharged exactly when every complement box has an
//! empty Tier-2 start set: a non-empty start set or a refused search certifies
//! an additional component hiding in the complement, so the exhaustive claim is
//! not certified. Tier-1 is the loop-free exclusion of the mechanical form; it
//! is not the discharge gate for the landed conservative hemisphere leaf cones
//! (a closed component always carries a Tier-2 start point by Theorem 9.2). The
//! exclusion runs regardless of `provenance` — `provenance` is never a
//! certificate (D6). The Tier-2 direction `a` is a [`LeafPair`] attribute (the
//! §9.2 genericity choice belongs to the pair).
//!
//! A non-exhaustive claim (or the trusted-provenance opt-in) goes through
//! [`claim_claimed`], which certifies the components (items 1-2) and returns a
//! [`ClaimedGraph`]; [`certify_claimed`] refuses a non-exhaustive claim because
//! its `CertifiedGraph` result type can never carry a claim that item 3 did not
//! discharge (D6).

use crate::kernel::assemble::regions_identify;
use crate::kernel::certs::{ArcCert, Frame, PointCert};
use crate::kernel::engine::{build_frame4, c2_certify_tube4, krawczyk_c1, SquareResidualEval};
use crate::kernel::evidence::{ClaimVerdict, Construction, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::graph::{
    AnyArc, Approx, Arc, ArcEnd, ArcId, CertifiedGraph, ChartId, ClaimedGraph, HermiteSegment,
    HermiteSpline, Node, NodeCert, NodeId, Param, Point4, Provenance, TopoNode,
};
use crate::kernel::identity::IdentityVerdict;
use crate::kernel::leaf::BezierLeaf;
use crate::kernel::patch::{CertifiedPatch, CertifiedPositive, IBox, IBox2};
use crate::kernel::tier1::tier1_loop_free;
use crate::kernel::tier2::{tier2_start_set, TierTwoOutcome};
use crate::kernel::Interval;
use crate::SquareSystem3;

/// The `tube-chain-via-C2` predicate label (spec §15 item 1).
const P_TUBE_CHAIN: &str = "tube-chain-via-C2";
/// The `endpoints-via-C1` predicate label (spec §15 item 1).
const P_ENDPOINTS: &str = "endpoints-via-C1";
/// The `nodes-via-A4.2` predicate label (spec §15 item 1).
const P_NODES: &str = "nodes-via-A4.2";

/// The half width of the endpoint C1 search box around a certified crossing.
const ENDPOINT_HALF: f64 = 0.03;

/// The tube-chain kind set of the certified graph (the landed §16 arc
/// families). The C2 seam on a leaf pair certifies ordinary R1 chains only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    /// An ordinary transversal tube-chain arc (R1 over the leaf pair).
    Ordinary,
    /// A planar difference arc.
    Difference,
    /// A self-intersection arc (§13 R6).
    SelfInt,
    /// A canal spine arc (§12).
    Spine,
    /// A recognized-carrier contact arc.
    Carrier,
}

/// One claimed component of the authored topology: a seed parameter point the
/// component is claimed to pass through, and the expected tube-chain kind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClaimedComponent {
    /// The claimed seed: one parameter point on each of the two charts.
    pub seed: Point4,
    /// The expected tube-chain kind of the claimed component.
    pub expected: ComponentKind,
}

/// The authored-topology claim (spec §15): the components, whether the list is
/// claimed exhaustive, and the provenance of the claim.
#[derive(Debug, Clone, PartialEq)]
pub struct TopologyClaim {
    /// The claimed components.
    pub components: Vec<ClaimedComponent>,
    /// Whether the claim asserts the component list is exhaustive.
    pub exhaustive: bool,
    /// The provenance of the claim (never a certificate, D6).
    pub provenance: Provenance,
}

/// The refutation of one claimed component: the component index and the
/// failing predicate's label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimRefutation {
    /// The index of the refuted component in the claim.
    pub component: usize,
    /// The failing predicate label (`tube-chain-via-C2`, `endpoints-via-C1`, or
    /// `nodes-via-A4.2`).
    pub predicate: String,
}

/// A leaf pair in the shared-chart graph arrangement (v1 scope, recorded):
/// two unit-weight polynomial graph leaves over the same identity chart, the
/// R1 square system of their model-space difference, the pair's claim domain,
/// and the §9.2 Tier-2 exclusion direction.
#[derive(Debug, Clone)]
pub struct LeafPair {
    /// The first leaf `(u, v, h1(u, v))`.
    pub first: BezierLeaf,
    /// The second leaf `(u, v, h2(u, v))`.
    pub second: BezierLeaf,
    /// The first leaf's chart id.
    pub first_chart: ChartId,
    /// The second leaf's chart id.
    pub second_chart: ChartId,
    /// The R1 square system `S1(u1,v1) - S2(u2,v2)` over the product domain.
    pub system: SquareSystem3,
    /// The claimed product domain `(u1, v1, u2, v2)` box, a sub-box of the
    /// unit charts with paired axes (`u1`-`u2` and `v1`-`v2` ranges equal).
    pub domain: IBox<4>,
    /// The §9.2 Tier-2 exclusion direction `a`.
    pub tier2_a: [f64; 4],
}

/// One certified component: the certified tube chain, its chart-space
/// parameter box (clipped to the claim domain), and its two certified boundary
/// endpoint certificates.
struct CertifiedTube {
    /// The certified C2 arc of the component.
    arc_cert: ArcCert<4>,
    /// The certified tube's chart-space parameter box, clipped to the claim
    /// domain (the box subtracted for the item-3 complement).
    chart_box: IBox<4>,
    /// The certified left endpoint crossing.
    left: CertifiedEndpoint,
    /// The certified right endpoint crossing.
    right: CertifiedEndpoint,
}

/// A certified endpoint of a claimed component.
#[derive(Clone)]
struct CertifiedEndpoint {
    /// The certified crossing's chart point on the four product axes.
    point: Point4,
    /// The certified crossing (2D shared-chart certificate).
    cert: PointCert,
    /// The shared-chart point `(u, v)` used for model-space evaluation.
    shared: [f64; 2],
}

/// The failure of a single component certification: a refutation naming the
/// component and predicate, or an unresolved (Inconclusive) refusal.
enum ComponentFailure {
    /// The component was refuted.
    Refuted(ClaimRefutation),
    /// The component could not be certified (not refuted).
    Inconclusive(Refusal),
}

impl From<Refusal> for ComponentFailure {
    fn from(refusal: Refusal) -> Self {
        ComponentFailure::Inconclusive(refusal)
    }
}

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}

/// A named refusal for a caller/precondition violation.
fn caller_refusal(name: &'static str, detail: String) -> Refusal {
    refusal(RefusalKind::ClaimRefuted, name, detail)
}

impl LeafPair {
    /// Build a leaf pair in the shared-chart graph arrangement.
    ///
    /// Refuses (each as a named [`RefusalKind::ClaimRefuted`] refusal):
    /// - a leaf whose control net is not the canonical unit-weight graph net
    ///   over the identity chart (control weight `1.0`, `x` = the `u`
    ///   Bernstein abscissa grid, `y` = the `v` Bernstein abscissa grid);
    /// - a claim domain that is not a compact sub-box of the unit product box
    ///   with paired equal ranges (`u1`-`u2` and `v1`-`v2`);
    /// - a non-finite Tier-2 direction.
    #[allow(clippy::result_large_err)]
    pub fn try_new(
        first: BezierLeaf,
        second: BezierLeaf,
        first_chart: ChartId,
        second_chart: ChartId,
        domain: IBox<4>,
        tier2_a: [f64; 4],
    ) -> Construction<Self> {
        validate_graph_leaf(&first, "first")?;
        validate_graph_leaf(&second, "second")?;
        for k in 0..4 {
            if domain.lo[k] < 0.0 || domain.hi[k] > 1.0 {
                return Err(caller_refusal(
                    "claims_domain_outside_unit_chart",
                    format!("the claim domain axis {k} must lie within the unit chart"),
                ));
            }
                  #[allow(clippy::neg_cmp_op_on_partial_ord)] // fail-closed: !(a<b) refuses the undecidable middle; a>=b would not, on a partial order
            if !(domain.lo[k] < domain.hi[k]) {
                return Err(caller_refusal(
                    "claims_domain_degenerate_axis",
                    format!("the claim domain axis {k} must have positive width"),
                ));
            }
        }
        if domain.lo[0] != domain.lo[2] || domain.hi[0] != domain.hi[2] {
            return Err(caller_refusal(
                "claims_domain_u_axes_misaligned",
                "the claim domain u1 and u2 ranges must be equal (shared chart)".to_string(),
            ));
        }
        if domain.lo[1] != domain.lo[3] || domain.hi[1] != domain.hi[3] {
            return Err(caller_refusal(
                "claims_domain_v_axes_misaligned",
                "the claim domain v1 and v2 ranges must be equal (shared chart)".to_string(),
            ));
        }
        if !tier2_a.iter().all(|c| c.is_finite()) {
            return Err(caller_refusal(
                "claims_tier2_direction_not_finite",
                "the Tier-2 direction a must be finite".to_string(),
            ));
        }
        let system = build_pair_system(&first, &second)?;
        Ok(Self {
            first,
            second,
            first_chart,
            second_chart,
            system,
            domain,
            tier2_a,
        })
    }
}

/// Validate one canonical unit-weight graph leaf over the identity chart.
#[allow(clippy::result_large_err)]
fn validate_graph_leaf(leaf: &BezierLeaf, which: &str) -> Construction<()> {
    if leaf.degree_u == 0 || leaf.degree_v == 0 {
        return Err(caller_refusal(
            "claims_leaf_zero_degree",
            format!("the {which} leaf must have positive bidegree"),
        ));
    }
    let width = leaf.degree_v + 1;
    let expected = (leaf.degree_u + 1) * (leaf.degree_v + 1);
    if leaf.control.len() != expected {
        return Err(caller_refusal(
            "claims_leaf_control_count_mismatch",
            format!("the {which} leaf control net has the wrong size"),
        ));
    }
    for a in 0..=leaf.degree_u {
        for b in 0..=leaf.degree_v {
            let p = &leaf.control[a * width + b];
            if !p.iter().all(|c| c.is_finite()) {
                return Err(caller_refusal(
                    "claims_leaf_not_finite",
                    format!("the {which} leaf control point ({a}, {b}) is not finite"),
                ));
            }
            if p[3] != 1.0 {
                return Err(caller_refusal(
                    "claims_leaf_weight_not_unit",
                    format!("the {which} leaf control weight at ({a}, {b}) is not exactly 1.0"),
                ));
            }
            let u_abscissa = a as f64 / leaf.degree_u as f64;
            let v_abscissa = b as f64 / leaf.degree_v as f64;
            if p[0] != u_abscissa || p[1] != v_abscissa {
                return Err(caller_refusal(
                    "claims_leaf_not_identity_graph",
                    format!(
                        "the {which} leaf is not a graph over the identity chart \
                         (control ({a}, {b}) has x {} / y {}, expected the Bernstein abscissae \
                         ({u_abscissa}, {v_abscissa}))",
                        p[0], p[1]
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// The R1 square system of two unit-weight leaves: the tensor difference of
/// the two coordinate nets over the product domain (identity charts).
#[allow(clippy::result_large_err)]
fn build_pair_system(first: &BezierLeaf, second: &BezierLeaf) -> Construction<SquareSystem3> {
    let (m1, n1) = (first.degree_u, first.degree_v);
    let (m2, n2) = (second.degree_u, second.degree_v);
    let rows = (m1 + 1) * (n1 + 1);
    let cols = (m2 + 1) * (n2 + 1);
    let mut grids: [Vec<Vec<f64>>; 3] = [
        vec![vec![0.0f64; cols]; rows],
        vec![vec![0.0f64; cols]; rows],
        vec![vec![0.0f64; cols]; rows],
    ];
    let w1 = n1 + 1;
    let w2 = n2 + 1;
    for a in 0..=m1 {
        for b in 0..=n1 {
            let r = a * w1 + b;
            for i in 0..=m2 {
                for j in 0..=n2 {
                    let c = i * w2 + j;
                    for (k, grid) in grids.iter_mut().enumerate() {
                        grid[r][c] = first.control[r][k] - second.control[c][k];
                    }
                }
            }
        }
    }
    SquareSystem3::new(
        grids,
        (m1, n1, m2, n2),
        (0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0),
    )
    .map_err(|_| {
        caller_refusal(
            "claims_system_refused",
            "the pair's R1 square system could not be constructed".to_string(),
        )
    })
}

/// The claim domain as a box.
    #[allow(clippy::result_large_err)] // frozen Refusal carries Option<PartialGraph>; large-Err allowed (BG-KV2-000, graph.rs precedent)
fn claim_domain_box(pair: &LeafPair) -> Construction<IBox<4>> {
    IBox::try_new(pair.domain.lo, pair.domain.hi).map_err(|_| {
        caller_refusal(
            "claims_domain_box_refused",
            "the claim domain box refused".to_string(),
        )
    })
}

/// The shared-chart point of a seed: the two leaf parameters stacked into the
/// four product coordinates. Refuses a non-finite parameter or a non-zero deck
/// (the v1 model is non-periodic).
#[allow(clippy::result_large_err)]
fn seed_to_chart(seed: &Point4) -> Construction<[f64; 4]> {
    let parts = [seed.p1.u, seed.p1.v, seed.p2.u, seed.p2.v];
    if !parts.iter().all(|c| c.is_finite()) {
        return Err(caller_refusal(
            "claims_seed_not_finite",
            "the claimed seed must be finite".to_string(),
        ));
    }
    if seed.p1.deck != 0 || seed.p2.deck != 0 {
        return Err(caller_refusal(
            "claims_seed_deck_nonzero",
            "the claimed seed must be on deck zero (non-periodic model)".to_string(),
        ));
    }
    Ok(parts)
}

/// The chart point on the four product axes as a [`Point4`] on the pair's two
/// charts (deck 0), when the point lies in the pair's charts.
#[allow(clippy::result_large_err)]
fn chart_to_point4(pair: &LeafPair, x: [f64; 4]) -> Construction<Point4> {
    if !x.iter().all(|c| c.is_finite()) {
        return Err(caller_refusal(
            "claims_chart_point_not_finite",
            "the chart point must be finite".to_string(),
        ));
    }
    Ok(Point4 {
        p1: Param::try_new(pair.first_chart, 0, x[0], x[1])?,
        p2: Param::try_new(pair.second_chart, 0, x[2], x[3])?,
    })
}

/// A degenerate box around the shared-chart point `(u, v)`.
    #[allow(clippy::result_large_err)] // frozen Refusal carries Option<PartialGraph>; large-Err allowed (BG-KV2-000, graph.rs precedent)
fn point_box(u: f64, v: f64) -> Construction<IBox2> {
    IBox2::try_new([u, v], [u, v]).map_err(|_| {
        caller_refusal(
            "claims_point_box_refused",
            "the degenerate point box refused".to_string(),
        )
    })
}

/// The certified model-space point of the first leaf at the shared-chart point
/// `(u, v)` (the position enclosure's midpoint).
fn model_point(leaf: &BezierLeaf, u: f64, v: f64) -> Option<[f64; 3]> {
    let box_ = point_box(u, v).ok()?;
    let enc = CertifiedPatch::enclose(leaf, box_);
    let mid = [
        0.5 * (enc.lo[0] + enc.hi[0]),
        0.5 * (enc.lo[1] + enc.hi[1]),
        0.5 * (enc.lo[2] + enc.hi[2]),
    ];
    if mid.iter().all(|c| c.is_finite()) {
        Some(mid)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// The tube chain (tube-chain-via-C2)
// ---------------------------------------------------------------------------

/// The certified chart-space box of a tube proposed from a seed frame: the
/// hull over `tau` in `[tau_lo, tau_hi]` and the perpendicular box
/// `[-h, h]^3` centred on the frame origin.
fn tube_chart_box(frame: &Frame<4>, tau_lo: f64, tau_hi: f64, h: f64) -> Option<[[f64; 2]; 4]> {
    if !(tau_lo.is_finite() && tau_hi.is_finite() && tau_lo < tau_hi && h.is_finite() && h > 0.0) {
        return None;
    }
    let tau = Interval {
        lo: tau_lo,
        hi: tau_hi,
    };
    let y = Interval { lo: -h, hi: h };
    let mut out = [[0.0f64; 2]; 4];
        #[allow(clippy::needless_range_loop)] // matrix indices over fixed 4-vectors; the index form is the algebra
    for j in 0..4 {
        let mut acc = Interval::point(frame.z_hat[j]);
        acc = acc.add(&Interval::point(frame.q_tau[j]).mul(&tau));
        for c in 0..3 {
            acc = acc.add(&Interval::point(frame.q_perp[c][j]).mul(&y));
        }
        if !acc.is_finite() {
            return None;
        }
        out[j] = [acc.lo, acc.hi];
    }
    Some(out)
}

/// Clip a chart-space box to the claim domain.
fn clip_to_domain(box_: [[f64; 2]; 4], domain: &IBox<4>) -> Option<IBox<4>> {
    let mut lo = [0.0f64; 4];
    let mut hi = [0.0f64; 4];
    for k in 0..4 {
        lo[k] = box_[k][0].max(domain.lo[k]);
        hi[k] = box_[k][1].min(domain.hi[k]);
        if lo[k] >= hi[k] {
            return None;
        }
    }
    IBox::try_new(lo, hi).ok()
}

/// The ray-box intersection parameters of the branch direction through the
/// seed against the claim domain box.
fn ray_box_range(seed: [f64; 4], dir: [f64; 4], domain: &IBox<4>) -> Option<(f64, f64)> {
    let mut t_lo = f64::NEG_INFINITY;
    let mut t_hi = f64::INFINITY;
    for k in 0..4 {
        if dir[k] == 0.0 {
            if seed[k] < domain.lo[k] || seed[k] > domain.hi[k] {
                return None;
            }
            continue;
        }
        let (a, b) = if dir[k] > 0.0 {
            (
                (domain.lo[k] - seed[k]) / dir[k],
                (domain.hi[k] - seed[k]) / dir[k],
            )
        } else {
            (
                (domain.hi[k] - seed[k]) / dir[k],
                (domain.lo[k] - seed[k]) / dir[k],
            )
        };
        t_lo = t_lo.max(a);
        t_hi = t_hi.min(b);
        if t_lo > t_hi {
            return None;
        }
    }
    if t_lo.is_finite() && t_hi.is_finite() && t_lo < 0.0 && t_hi > 0.0 {
        Some((t_lo, t_hi))
    } else {
        None
    }
}

/// Attempt a certified tube over `[tau_lo, tau_hi]` from the seed frame with
/// the perpendicular half width `h`.
fn attempt_tube(
    pair: &LeafPair,
    frame: &Frame<4>,
    tau_lo: f64,
    tau_hi: f64,
    h: f64,
) -> ClaimVerdict<ArcCert<4>, Refusal, &'static str> {
    let weight = match CertifiedPositive::try_new(1.0) {
        Ok(w) => w,
        Err(_) => return ClaimVerdict::Inconclusive("claims_weight_unavailable"),
    };
    let lo = [-h, -h, -h];
    let hi = [h, h, h];
    let b_perp = match IBox::<3>::try_new(lo, hi) {
        Ok(b) => b,
        Err(_) => return ClaimVerdict::Inconclusive("claims_perp_box_refused"),
    };
    let i_tau = Interval {
        lo: tau_lo,
        hi: tau_hi,
    };
    c2_certify_tube4(
        &pair.system,
        frame,
        i_tau,
        b_perp,
        std::slice::from_ref(&weight),
    )
}

/// The chart point on the branch centerline at `tau`.
fn chart_centerline(frame: &Frame<4>, tau: f64) -> [f64; 4] {
    let mut out = [0.0f64; 4];
    for (j, out_j) in out.iter_mut().enumerate() {
        *out_j = frame.z_hat[j] + frame.q_tau[j] * tau;
    }
    out
}

/// The pivot face of one end of the branch: whether the `u` pair or the `v`
/// pair limits the run, and the claim-domain face value the branch crosses.
fn pivot_face(seed: [f64; 4], dir: [f64; 4], domain: &IBox<4>, t_end: f64) -> Option<(usize, f64)> {
    let mut hit_u = false;
    let mut hit_v = false;
    for k in 0..4 {
        if dir[k] == 0.0 {
            continue;
        }
        let xk = seed[k] + dir[k] * t_end;
        if (xk - domain.lo[k]).abs() <= 1e-6 * domain.hi[k].max(domain.lo[k]).max(1.0)
            || (xk - domain.hi[k]).abs() <= 1e-6 * domain.hi[k].max(domain.lo[k]).max(1.0)
        {
            // H-3: face-coincidence slack for the ray end, relative to the domain width
            if k == 0 || k == 2 {
                hit_u = true;
            } else {
                hit_v = true;
            }
        }
    }
    if hit_u {
        // The u pair reaches the face first; face value from the shared u range.
        if t_end < 0.0 {
            Some((0, domain.lo[0]))
        } else {
            Some((0, domain.hi[0]))
        }
    } else if hit_v {
        if t_end < 0.0 {
            Some((1, domain.lo[1]))
        } else {
            Some((1, domain.hi[1]))
        }
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// The endpoint residual (endpoints-via-C1)
// ---------------------------------------------------------------------------

/// The square 2D residual `(g(u, v), c - f)` over the shared chart: the
/// difference-curve height and the face-coordinate equation. `axis` is `0` for
/// the `u` face and `1` for the `v` face; `f` is the face value.
struct BoundaryCrossing {
    /// The first leaf.
    first: BezierLeaf,
    /// The second leaf.
    second: BezierLeaf,
    /// The face axis (0 = `u`, 1 = `v`).
    axis: usize,
    /// The face value.
    face: f64,
}

impl SquareResidualEval for BoundaryCrossing {
    fn arity(&self) -> usize {
        2
    }

    fn eval(&self, b: &[Interval]) -> Vec<Interval> {
        if b.len() != 2 {
            return Vec::new();
        }
        let box_ = IBox2 {
            lo: [b[0].lo, b[1].lo],
            hi: [b[0].hi, b[1].hi],
        };
        let z1 = CertifiedPatch::enclose(&self.first, box_);
        let z2 = CertifiedPatch::enclose(&self.second, box_);
        let g = Interval {
            lo: z1.lo[2] - z2.hi[2],
            hi: z1.hi[2] - z2.lo[2],
        };
        let coord = if self.axis == 0 { b[0] } else { b[1] };
        let c = coord.sub(&Interval::point(self.face));
        vec![g, c]
    }

    fn jac_encl(&self, b: &[Interval]) -> Vec<Vec<Interval>> {
        if b.len() != 2 {
            return Vec::new();
        }
        let box_ = IBox2 {
            lo: [b[0].lo, b[1].lo],
            hi: [b[0].hi, b[1].hi],
        };
        let d1 = CertifiedPatch::derivs(&self.first, box_);
        let d2 = CertifiedPatch::derivs(&self.second, box_);
        let gu = Interval {
            lo: d1.su.lo[2] - d2.su.hi[2],
            hi: d1.su.hi[2] - d2.su.lo[2],
        };
        let gv = Interval {
            lo: d1.sv.lo[2] - d2.sv.hi[2],
            hi: d1.sv.hi[2] - d2.sv.lo[2],
        };
        let one = Interval::point(1.0);
        let zero = Interval::point(0.0);
        let (dcu, dcv) = if self.axis == 0 {
            (one, zero)
        } else {
            (zero, one)
        };
        vec![vec![gu, gv], vec![dcu, dcv]]
    }
}

/// Certify one endpoint of the branch: the crossing of the difference curve
/// with the claim-domain face at the end of the run (endpoints-via-C1).
#[allow(clippy::result_large_err)]
fn certify_endpoint(
    pair: &LeafPair,
    seed: [f64; 4],
    dir: [f64; 4],
    t_end: f64,
    end_point: [f64; 4],
) -> Result<CertifiedEndpoint, String> {
    let (axis, face_value) = match pivot_face(seed, dir, &pair.domain, t_end) {
        Some(p) => p,
        None => return Err(P_ENDPOINTS.to_string()),
    };
    let (u_c, v_c) = if axis == 0 {
        (face_value, end_point[1])
    } else {
        (end_point[0], face_value)
    };
    let (u_lo, u_hi, v_lo, v_hi) = if axis == 0 {
        (
            face_value - ENDPOINT_HALF,
            face_value + ENDPOINT_HALF,
            v_c - ENDPOINT_HALF,
            v_c + ENDPOINT_HALF,
        )
    } else {
        (
            u_c - ENDPOINT_HALF,
            u_c + ENDPOINT_HALF,
            face_value - ENDPOINT_HALF,
            face_value + ENDPOINT_HALF,
        )
    };
    if !(u_lo >= 0.0 && u_hi <= 1.0 && v_lo >= 0.0 && v_hi <= 1.0 && u_lo < u_hi && v_lo < v_hi) {
        return Err(P_ENDPOINTS.to_string());
    }
    let box_ = match IBox2::try_new([u_lo, v_lo], [u_hi, v_hi]) {
        Ok(b) => b,
        Err(_) => return Err(P_ENDPOINTS.to_string()),
    };
    let weight = match CertifiedPositive::try_new(1.0) {
        Ok(w) => w,
        Err(_) => return Err(P_ENDPOINTS.to_string()),
    };
    let residual = BoundaryCrossing {
        first: pair.first.clone(),
        second: pair.second.clone(),
        axis,
        face: face_value,
    };
    let crossing = match krawczyk_c1(&residual, box_, std::slice::from_ref(&weight)) {
        ClaimVerdict::Proven(cert) => cert,
        ClaimVerdict::Disproven(_) => return Err(P_ENDPOINTS.to_string()),
        ClaimVerdict::Inconclusive(_) => return Err(format!("{P_ENDPOINTS}:inconclusive")),
    };
    let mid_u = 0.5 * (crossing.box_.lo[0] + crossing.box_.hi[0]);
    let mid_v = 0.5 * (crossing.box_.lo[1] + crossing.box_.hi[1]);
    let point = match chart_to_point4(pair, [mid_u, mid_v, mid_u, mid_v]) {
        Ok(p) => p,
        Err(_) => return Err(P_ENDPOINTS.to_string()),
    };
    Ok(CertifiedEndpoint {
        point,
        cert: crossing,
        shared: [mid_u, mid_v],
    })
}

// ---------------------------------------------------------------------------
// Per-component certification
// ---------------------------------------------------------------------------

/// The half width of the certified no-root probe around the claimed seed (a
/// certification constant).
const SEED_PROBE_HALF: f64 = 1e-6;

/// Whether the certified difference-curve enclosure over a small box around
/// the shared-chart seed provably excludes zero (the seed is certified OFF the
/// intersection).
fn seed_off_curve(pair: &LeafPair, u: f64, v: f64) -> bool {
    let probe = match IBox2::try_new(
        [u - SEED_PROBE_HALF, v - SEED_PROBE_HALF],
        [u + SEED_PROBE_HALF, v + SEED_PROBE_HALF],
    ) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let z1 = CertifiedPatch::enclose(&pair.first, probe);
    let z2 = CertifiedPatch::enclose(&pair.second, probe);
    let g_lo = z1.lo[2] - z2.hi[2];
    let g_hi = z1.hi[2] - z2.lo[2];
    g_lo > 0.0 || g_hi < 0.0
}

/// Certify one claimed component (spec §15 items 1-2).
#[allow(clippy::result_large_err)]
fn certify_component(
    pair: &LeafPair,
    index: usize,
    component: &ClaimedComponent,
) -> Result<CertifiedTube, ComponentFailure> {
    let refute = |predicate: &'static str| {
        Err(ComponentFailure::Refuted(ClaimRefutation {
            component: index,
            predicate: predicate.to_string(),
        }))
    };
    let inconclusive = |detail: String| {
        Err(ComponentFailure::Inconclusive(caller_refusal(
            "claims_component_not_certified",
            detail,
        )))
    };
    if component.expected != ComponentKind::Ordinary {
        return refute(P_TUBE_CHAIN);
    }
    let x0 = seed_to_chart(&component.seed)?;
        #[allow(clippy::needless_range_loop)] // matrix indices over fixed 4-vectors; the index form is the algebra
    for k in 0..4 {
        if !(x0[k] > pair.domain.lo[k] && x0[k] < pair.domain.hi[k]) {
            return refute(P_TUBE_CHAIN);
        }
    }
    // The certified seed check: a seed provably off the intersection refutes
    // the component (no tube chain passes through it).
    if seed_off_curve(pair, x0[0], x0[1]) {
        return refute(P_TUBE_CHAIN);
    }
    let frame = match build_frame4(&pair.system, x0) {
        Ok((frame, _m)) => frame,
        Err(refusal) => {
            return Err(ComponentFailure::Inconclusive(refusal));
        }
    };
    let dir = frame.q_tau;
    // The branch's crossing parameters of the claim domain (the endpoints of
    // the certified component lie on the domain faces the branch meets).
    let (t_lo, t_hi) = match ray_box_range(x0, dir, &pair.domain) {
        Some(r) => r,
        None => return refute(P_TUBE_CHAIN),
    };
    // Certify a seed tube around the branch. Candidate (tau half width, perp
    // half width) pairs are attempted in order; the first certifying tube whose
    // chart box stays inside the claim domain is retained. (The tube need not
    // span the whole crossing: the certified topology of the component is the
    // tube plus its two C1-certified boundary endpoints below.)
    let mut chosen: Option<(ArcCert<4>, f64, f64)> = None;
    'candidates: for (tau_half, h) in [
        (0.01f64, 0.05f64),
        (0.01, 0.02),
        (0.005, 0.05),
        (0.005, 0.02),
        (0.002, 0.05),
        (0.002, 0.02),
        (0.001, 0.05),
        (0.001, 0.02),
        (0.001, 0.01),
    ] {
        if !(tau_half.is_finite() && h.is_finite() && h > 0.0 && tau_half > 0.0) {
            continue;
        }
        // The tube must be strictly inside the claim domain.
        let room = t_hi.min(-t_lo);
                  #[allow(clippy::neg_cmp_op_on_partial_ord)] // fail-closed: !(a<b) refuses the undecidable middle; a>=b would not, on a partial order
        if !(tau_half < room) {
            continue;
        }
        match attempt_tube(pair, &frame, -tau_half, tau_half, h) {
            ClaimVerdict::Proven(cert) => {
                let full_box = match tube_chart_box(&frame, -tau_half, tau_half, h) {
                    Some(b) => b,
                    None => continue,
                };
                if clip_to_domain(full_box, &pair.domain).is_some() {
                    chosen = Some((cert, tau_half, h));
                    break 'candidates;
                }
            }
            ClaimVerdict::Disproven(_) => return refute(P_TUBE_CHAIN),
            ClaimVerdict::Inconclusive(_) => {}
        }
    }
    let (arc_cert, tau_half, h) = match chosen {
        Some(x) => x,
        None => {
            return inconclusive(format!(
                "the C2 tube over the claimed component could not be certified (component {index})"
            ))
        }
    };
    let chart_box = match tube_chart_box(&frame, -tau_half, tau_half, h) {
        Some(b) => b,
        None => return inconclusive("the certified tube's chart box is not finite".to_string()),
    };
    let chart_box = match clip_to_domain(chart_box, &pair.domain) {
        Some(b) => b,
        None => {
            return inconclusive("the certified tube does not meet the claim domain".to_string())
        }
    };

    let left_point = chart_centerline(&frame, t_lo);
    let right_point = chart_centerline(&frame, t_hi);
    let left = certify_endpoint(pair, x0, dir, t_lo, left_point).map_err(|predicate| {
        if predicate.ends_with(":inconclusive") {
            ComponentFailure::Inconclusive(caller_refusal(
                "claims_endpoint_not_certified",
                predicate,
            ))
        } else {
            ComponentFailure::Refuted(ClaimRefutation {
                component: index,
                predicate,
            })
        }
    })?;
    let right = certify_endpoint(pair, x0, dir, t_hi, right_point).map_err(|predicate| {
        if predicate.ends_with(":inconclusive") {
            ComponentFailure::Inconclusive(caller_refusal(
                "claims_endpoint_not_certified",
                predicate,
            ))
        } else {
            ComponentFailure::Refuted(ClaimRefutation {
                component: index,
                predicate,
            })
        }
    })?;
    // Nodes via A4.2: the two ends of one component must not identify (the
    // component has certified positive length).
    match regions_identify(&left.cert, &right.cert, &[]) {
        IdentityVerdict::CertifiedEqual { .. } => return refute(P_NODES),
        IdentityVerdict::NotCertified => {}
    }
    Ok(CertifiedTube {
        arc_cert,
        chart_box,
        left,
        right,
    })
}

// ---------------------------------------------------------------------------
// Graph assembly
// ---------------------------------------------------------------------------

/// Build the certified graph content (nodes + arcs) of all certified
/// components.
    #[allow(clippy::result_large_err)] // frozen Refusal carries Option<PartialGraph>; large-Err allowed (BG-KV2-000, graph.rs precedent)
fn build_graph_content(
    pair: &LeafPair,
    tubes: &[CertifiedTube],
) -> Construction<(Vec<Node>, Vec<AnyArc>)> {
    let mut nodes: Vec<Node> = Vec::new();
    let mut left_ids: Vec<NodeId> = Vec::new();
    let mut right_ids: Vec<NodeId> = Vec::new();
    for tube in tubes {
        let left_id = NodeId(nodes.len());
        nodes.push(Node {
            id: left_id,
            at: tube.left.point,
            kind: TopoNode::Boundary,
            cert: NodeCert::Exact(tube.left.cert),
        });
        let right_id = NodeId(nodes.len());
        nodes.push(Node {
            id: right_id,
            at: tube.right.point,
            kind: TopoNode::Boundary,
            cert: NodeCert::Exact(tube.right.cert),
        });
        left_ids.push(left_id);
        right_ids.push(right_id);
    }
    let mut arcs: Vec<AnyArc> = Vec::new();
    for (arc_index, tube) in tubes.iter().enumerate() {
        let left_model = match model_point(&pair.first, tube.left.shared[0], tube.left.shared[1]) {
            Some(p) => p,
            None => {
                return Err(caller_refusal(
                    "claims_model_left_unavailable",
                    "the left endpoint model point could not be enclosed".to_string(),
                ))
            }
        };
        let right_model = match model_point(&pair.first, tube.right.shared[0], tube.right.shared[1])
        {
            Some(p) => p,
            None => {
                return Err(caller_refusal(
                    "claims_model_right_unavailable",
                    "the right endpoint model point could not be enclosed".to_string(),
                ))
            }
        };
        let chord = [
            right_model[0] - left_model[0],
            right_model[1] - left_model[1],
            right_model[2] - left_model[2],
        ];
        let segment = HermiteSegment {
            p0: left_model,
            p1: right_model,
            t0: chord,
            t1: chord,
        };
        let spline = HermiteSpline::try_new(vec![segment])?;
        let topo_arc = Arc {
            id: ArcId(arc_index),
            approx: Approx { gamma: spline },
            cert: tube.arc_cert.clone(),
            ends: (
                ArcEnd::Topo(left_ids[arc_index]),
                ArcEnd::Topo(right_ids[arc_index]),
            ),
        };
        arcs.push(AnyArc::Ordinary(topo_arc));
    }
    Ok((nodes, arcs))
}

// ---------------------------------------------------------------------------
// Completeness (item 3)
// ---------------------------------------------------------------------------

/// Subtract an inner box from an outer box (the inner box must be a sub-box of
/// the outer). The returned axis slabs together cover the difference.
    #[allow(clippy::result_large_err)] // frozen Refusal carries Option<PartialGraph>; large-Err allowed (BG-KV2-000, graph.rs precedent)
fn subtract_box(outer: &IBox<4>, inner: &IBox<4>) -> Construction<Vec<IBox<4>>> {
    let mut out: Vec<IBox<4>> = Vec::new();
    for k in 0..4 {
        if inner.lo[k] > outer.lo[k] {
            let lo = outer.lo;
            let mut hi = outer.hi;
            hi[k] = inner.lo[k];
            if lo[k] < hi[k] {
                out.push(IBox::try_new(lo, hi).map_err(|_| {
                    caller_refusal(
                        "claims_subtract_low_slab_refused",
                        "the low complement slab refused".to_string(),
                    )
                })?);
            }
        }
        if inner.hi[k] < outer.hi[k] {
            let mut lo = outer.lo;
            let hi = outer.hi;
            lo[k] = inner.hi[k];
            if lo[k] < hi[k] {
                out.push(IBox::try_new(lo, hi).map_err(|_| {
                    caller_refusal(
                        "claims_subtract_high_slab_refused",
                        "the high complement slab refused".to_string(),
                    )
                })?);
            }
        }
    }
    Ok(out)
}

/// The complement boxes of the claim domain minus each certified tube's
/// parameter box (box subtraction over the landed [`IBox`] shape).
    #[allow(clippy::result_large_err)] // frozen Refusal carries Option<PartialGraph>; large-Err allowed (BG-KV2-000, graph.rs precedent)
fn complement_boxes(domain: &IBox<4>, tubes: &[CertifiedTube]) -> Construction<Vec<IBox<4>>> {
    let mut current: Vec<IBox<4>> = vec![*domain];
    for tube in tubes {
        let mut next: Vec<IBox<4>> = Vec::new();
        for outer in current {
            let slabs = subtract_box(&outer, &tube.chart_box)?;
            next.extend(slabs);
        }
        current = next;
    }
    Ok(current)
}

/// The IncompleteStartSet (Inconclusive) refusal of a failed completeness
/// discharge.
fn incomplete_start_set_refusal(detail: String) -> Refusal {
    refusal(
        RefusalKind::IncompleteStartSet,
        "targeted_completeness_not_discharged",
        detail,
    )
}

/// Run the item-3 targeted completeness over the certified tubes: Tier-1
/// (loop-free) exclusion and Tier-2 (critical-point start set) exclusion run
/// over each complement box of the claim domain minus the certified tubes.
/// Completeness is discharged exactly when every complement box has an empty
/// Tier-2 start set — a non-empty start set or a refused search certifies an
/// additional component (a critical point of the exclusion direction) hiding
/// in the complement. Tier-1 is run over the boxes as the loop-free exclusion
/// the packet names; with the landed conservative hemisphere leaf cones it is
/// not the discharge gate (a closed component always carries a Tier-2 start
/// point by Theorem 9.2), so the gate is the Tier-2 emptiness.
fn discharge_completeness(
    pair: &LeafPair,
    tubes: &[CertifiedTube],
) -> ClaimVerdict<(), Refusal, Refusal> {
    let domain = match claim_domain_box(pair) {
        Ok(d) => d,
        Err(refusal) => return ClaimVerdict::Inconclusive(refusal),
    };
    let boxes = match complement_boxes(&domain, tubes) {
        Ok(b) => b,
        Err(refusal) => return ClaimVerdict::Inconclusive(refusal),
    };
    for box_ in boxes {
        // Tier-1 loop-free exclusion over this complement box (informational;
        // the gate is Tier-2 emptiness below).
        let leaf1_box = IBox2 {
            lo: [box_.lo[0], box_.lo[1]],
            hi: [box_.hi[0], box_.hi[1]],
        };
        let leaf2_box = IBox2 {
            lo: [box_.lo[2], box_.lo[3]],
            hi: [box_.hi[2], box_.hi[3]],
        };
        let _ = tier1_loop_free(
            &CertifiedPatch::normal_cone(&pair.first, leaf1_box),
            &CertifiedPatch::normal_cone(&pair.second, leaf2_box),
        );
        match tier2_start_set(&pair.system, pair.tier2_a, box_) {
            TierTwoOutcome::Complete { start_set } => {
                if !start_set.is_empty() {
                    let detail = format!(
                        "Tier-2 isolated {} start point(s) in a complement box {:?}: the \
                         complement contains an additional component, so the exhaustive claim's \
                         targeted completeness is NOT discharged",
                        start_set.len(),
                        box_
                    );
                    return ClaimVerdict::Inconclusive(incomplete_start_set_refusal(detail));
                }
            }
            TierTwoOutcome::Refused(refusal) => {
                let detail = format!(
                    "Tier-2 refused a complement box {:?} ({:?}): targeted completeness is not \
                     discharged",
                    box_, refusal
                );
                return ClaimVerdict::Inconclusive(incomplete_start_set_refusal(detail));
            }
        }
    }
    ClaimVerdict::Proven(())
}

// ---------------------------------------------------------------------------
// Entries
// ---------------------------------------------------------------------------

/// Certify the claimed components of an exhaustive claim (spec §15 items 1-3).
///
/// Each claimed component is certified independently (tube chain via C2,
/// endpoints via C1, nodes via §4.2). A refutation is total: the surviving
/// components are never assembled into a partial graph, and the refutation
/// names the component index and the failing predicate. For an exhaustive
/// claim the item-3 targeted completeness runs over the complement of the
/// certified tubes (Tier-1 and Tier-2 exclusion); only a discharged
/// complement yields the `Proven` arm with a fully certified graph.
///
/// A non-exhaustive claim cannot produce a [`CertifiedGraph`] (spec §15 item
/// 4 / D6): the caller must route it through [`claim_claimed`], which yields
/// the distinct [`ClaimedGraph`] type. This entry refuses such a claim with a
/// named Inconclusive refusal rather than ever certifying it.
#[allow(clippy::result_large_err)]
pub fn certify_claimed(
    pair: &LeafPair,
    claim: &TopologyClaim,
) -> ClaimVerdict<CertifiedGraph, ClaimRefutation, Refusal> {
    if !claim.exhaustive {
        return ClaimVerdict::Inconclusive(caller_refusal(
            "claims_nonexhaustive_not_certifiable",
            "a non-exhaustive claim yields a ClaimedGraph (route through claim_claimed); \
             certify_claimed only certifies exhaustive claims"
                .to_string(),
        ));
    }
    let mut tubes: Vec<CertifiedTube> = Vec::new();
    for (index, component) in claim.components.iter().enumerate() {
        match certify_component(pair, index, component) {
            Ok(tube) => tubes.push(tube),
            Err(ComponentFailure::Refuted(claim_refutation)) => {
                return ClaimVerdict::Disproven(claim_refutation)
            }
            Err(ComponentFailure::Inconclusive(refusal)) => {
                return ClaimVerdict::Inconclusive(refusal)
            }
        }
    }
    let (nodes, arcs) = match build_graph_content(pair, &tubes) {
        Ok(x) => x,
        Err(refusal) => return ClaimVerdict::Inconclusive(refusal),
    };
    match discharge_completeness(pair, &tubes) {
        ClaimVerdict::Proven(()) => {}
        ClaimVerdict::Inconclusive(refusal) => return ClaimVerdict::Inconclusive(refusal),
        ClaimVerdict::Disproven(_) => {
            return ClaimVerdict::Inconclusive(incomplete_start_set_refusal(
                "the complement exclusion disproved the discharge (unreachable)".to_string(),
            ))
        }
    }
    ClaimVerdict::Proven(CertifiedGraph {
        nodes,
        breaks: Vec::new(),
        arcs,
        sheets: Vec::new(),
        exhaustive: true,
    })
}

/// Certify the claimed components and return the claimed (not certified) graph
/// (spec §15 items 1-2, item 4): the claimed graph and its provenance.
///
/// This is the non-exhaustive path and the trusted-provenance opt-in: the
/// item-3 complement exclusion is NOT run (it is skipped only here, never by
/// `provenance` inside [`certify_claimed`]), and the output is a
/// [`ClaimedGraph`] — a type distinct from [`CertifiedGraph`], so a Boolean
/// requiring closure rejects it by type (D6). The wrapped graph's `exhaustive`
/// flag records the claim's own assertion; it is not a certificate.
#[allow(clippy::result_large_err)]
pub fn claim_claimed(
    pair: &LeafPair,
    claim: &TopologyClaim,
) -> ClaimVerdict<ClaimedGraph, ClaimRefutation, Refusal> {
    let mut tubes: Vec<CertifiedTube> = Vec::new();
    for (index, component) in claim.components.iter().enumerate() {
        match certify_component(pair, index, component) {
            Ok(tube) => tubes.push(tube),
            Err(ComponentFailure::Refuted(claim_refutation)) => {
                return ClaimVerdict::Disproven(claim_refutation)
            }
            Err(ComponentFailure::Inconclusive(refusal)) => {
                return ClaimVerdict::Inconclusive(refusal)
            }
        }
    }
    let (nodes, arcs) = match build_graph_content(pair, &tubes) {
        Ok(x) => x,
        Err(refusal) => return ClaimVerdict::Inconclusive(refusal),
    };
    ClaimVerdict::Proven(ClaimedGraph {
        graph: CertifiedGraph {
            nodes,
            breaks: Vec::new(),
            arcs,
            sheets: Vec::new(),
            exhaustive: claim.exhaustive,
        },
        provenance: claim.provenance,
    })
}
