//! Node identity Rules A/B/C and the dyadic sampling join (BG-KV2-103-IDENTITY).
//!
//! Wave-1 implementation packet (build spec §4). This module lands v2 spec
//! §4.2 — node identity through the three rules — and §4.3 — Theorem 4.1's
//! deterministic dyadic join — as NEW code in `kernel/identity.rs`, built
//! entirely on the shim types. There are no solver bodies here: the uniqueness
//! premises arrive as landed certificates (the shim's [`PointCert`]); the rules
//! are pure box/relation logic. The one landed D2 contradiction (shapeops'
//! `near_pt` node welding) is NOT touched — its replacement is a booked seam,
//! outside this write set.
//!
//! Pre-made decisions (packet tags; do not relitigate):
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`: it is authored certified code, not
//! moved baseline.
//!
//! **N/D2 audit.** Identity is decided by exact box containment and the shim's
//! typed implication relation only. No `dist < eps`-style comparison exists
//! anywhere in this module (a source test pins that): a node identity rule that
//! rounded to a tolerance would snap distinct nodes together — the D2
//! contradiction this module exists to prevent.
//!
//! **Refuse-not-snap.** Anything ambiguous is [`IdentityVerdict::NotCertified`].
//! Rule B's transport is the one refusing constructor: an exact deck shift
//! whose `deck * period` product is not exactly representable refuses
//! [`RefusalKind::NonFinite`] rather than silently rounding.
//!
//! **D-shim.** Every shape consumed here is a frozen shim shape: [`PointCert`],
//! [`IBox2`], [`ResidualId`], [`implication`], and the evidence vocabulary.
//! The §4.3 `SamplingFlag` is a local two-variant enum — the landed
//! `truck-geometry` `SamplingPolicy` is NOT modified (write-set discipline);
//! the C1 wave wires the real policy type to [`refuse_custom_on_shared`].

use std::collections::BTreeSet;

use crate::kernel::certs::PointCert;
use crate::kernel::evidence::{Construction, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::patch::IBox2;
use crate::kernel::residual::{implication, Implication, ResidualId};

/// The deepest dyadic depth this module can address: a leaf index at depth `d`
/// lives in `0..2^d`, and `2^d` must fit a `u64` for the exact integer
/// expansion of §4.3. `2^63` still fits; `2^64` does not.
const MAX_ADDRESSABLE_DEPTH: u32 = 63;

/// Which rule certified an equality (§4.2), for the evidence trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityRule {
    /// Rule A: equal residuals plus a union certificate whose box contains the
    /// union hull of the two certificate boxes.
    RuleA,
    /// Rule B: the certificate was transported across a deck translation or an
    /// affine leaf reparameterization before the equality was certified.
    RuleB,
    /// Rule C: the certificates were identified through a common weaker
    /// residual admitted by the shim's typed implication relation.
    RuleC,
}

/// The verdict of a node-identity rule (§4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityVerdict {
    /// The two certificate neighborhoods are certified equal by the cited rule;
    /// carries the rule for the evidence trail.
    CertifiedEqual {
        /// The rule that certified the equality.
        rule: IdentityRule,
    },
    /// No rule applies — the nodes are NOT certified equal (the caller refuses
    /// rather than snaps, §4.2 closing rule).
    NotCertified,
}

/// §4.2 Rule A: whether two certificates certify the same node by containment.
///
/// Requires `a.residual == b.residual == union_cert.residual` and that
/// `union_cert`'s box contains the union hull of `a`'s and `b`'s boxes
/// componentwise. The caller owes the C1 certificate on
/// `B* = hull(B1 ∪ B2)`; this function checks containment, it does not solve.
/// The union hull is exact `f64` min/max — no tolerance anywhere, and never an
/// intersection of `a`'s box with `b`'s box (the spec's named error).
pub fn rule_a(a: &PointCert, b: &PointCert, union_cert: &PointCert) -> IdentityVerdict {
    if a.residual != b.residual || b.residual != union_cert.residual {
        return IdentityVerdict::NotCertified;
    }
    if !contains_union_hull(union_cert.box_, a.box_, b.box_) {
        return IdentityVerdict::NotCertified;
    }
    IdentityVerdict::CertifiedEqual {
        rule: IdentityRule::RuleA,
    }
}

/// §4.2 Rule B: transport a certificate across a symmetry.
///
/// A deck translation shifts the box's `u`/`v` bounds by the exact integer
/// `deck * period` (`period` carries the deck generator's period on `u` and on
/// `v`; periods in this kernel are dyadic-representable by construction). The
/// shift is exact: the product is asserted exactly representable via the
/// two-product residual, and the transport refuses [`RefusalKind::NonFinite`]
/// otherwise — a silently rounded shift would be the D2 violation this module
/// exists to prevent.
///
/// An affine leaf reparameterization is applied as an outward-rounded interval
/// evaluation of the exact map: each computed bound is pushed one ULP outward
/// with std's `f64::next_down`/`f64::next_up` (bit-exact, deterministic, no
/// libm; the landed `deck.rs` steppers stay private and untouched). Outward
/// rounding preserves containment, which is all Rule A needs. The transported
/// certificate keeps the source residual and contraction rate.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn rule_b_transport(
    b: &PointCert,
    deck: (i32, i32),
    period: (f64, f64),
    affine: Option<[[f64; 2]; 2]>,
) -> Construction<PointCert> {
    let mut lo = b.box_.lo;
    let mut hi = b.box_.hi;
    if deck.0 != 0 || deck.1 != 0 {
        let shift_u = exact_deck_shift(deck.0, period.0)?;
        let shift_v = exact_deck_shift(deck.1, period.1)?;
        lo[0] += shift_u;
        hi[0] += shift_u;
        lo[1] += shift_v;
        hi[1] += shift_v;
    }
    if let Some(matrix) = affine {
        let (out_lo, out_hi) = outward_affine_enclosure(matrix, lo, hi);
        lo = out_lo;
        hi = out_hi;
    }
    let box_ = IBox2::try_new(lo, hi)?;
    PointCert::try_new(b.residual, box_, b.rho)
}

/// §4.2 Rule C: identify two certificates through a common weaker residual.
///
/// Searches the caller's union certificates for a residual `R` that both
/// certificates imply — `implication(a.residual, R)` and
/// `implication(b.residual, R)` are both non-`None` — then applies the Rule A
/// containment test against the union certificate stated for `R`. The triple
/// equality of Rule A is taken at `R`, so the two certificates are read at
/// their common weaker residual. The admissible set is the shim's typed
/// relation — this function adds no implications. Anything ambiguous, including
/// a union certificate whose own residual disagrees with its key, is
/// [`IdentityVerdict::NotCertified`].
pub fn rule_c(
    a: &PointCert,
    b: &PointCert,
    union_certs: &[(ResidualId, PointCert)],
) -> IdentityVerdict {
    for (r, cert) in union_certs {
        let implies = implication(a.residual, *r) != Implication::None
            && implication(b.residual, *r) != Implication::None;
        if !implies || cert.residual != *r {
            continue;
        }
        if contains_union_hull(cert.box_, a.box_, b.box_) {
            return IdentityVerdict::CertifiedEqual {
                rule: IdentityRule::RuleC,
            };
        }
    }
    IdentityVerdict::NotCertified
}

/// Whether `outer` contains the componentwise union hull of `a` and `b`.
///
/// The hull is the exact `f64` min/max of the two boxes per axis; containment
/// is inclusive. No tolerance participates.
fn contains_union_hull(outer: IBox2, a: IBox2, b: IBox2) -> bool {
    for axis in 0..2 {
        let hull_lo = a.lo[axis].min(b.lo[axis]);
        let hull_hi = a.hi[axis].max(b.hi[axis]);
        if outer.lo[axis] > hull_lo || outer.hi[axis] < hull_hi {
            return false;
        }
    }
    true
}

/// The exact deck shift `deck * period`, refusing when the product is not
/// finite or is not exactly representable.
///
/// A finite `f64` is a dyadic rational, so `deck * period` is exactly
/// representable exactly when the correctly-rounded product has a vanishing
/// two-product residual (`fma(deck, period, -product)`). The shift is then the
/// product itself; a silent rounding here would be the D2 violation.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn exact_deck_shift(deck: i32, period: f64) -> Result<f64, Refusal> {
    let scale = deck as f64;
    let shift = scale * period;
    if !shift.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "deck_shift_not_finite",
            format!("deck {deck} times period {period} is not finite"),
        ));
    }
    if scale.mul_add(period, -shift) == 0.0 {
        Ok(shift)
    } else {
        Err(refusal(
            RefusalKind::NonFinite,
            "deck_shift_not_exact",
            format!("deck {deck} times period {period} is not exactly representable"),
        ))
    }
}

/// The outward-rounded image of an axis-aligned box under an exact affine map.
///
/// Per output row the exact image extent is the sum over columns of the two
/// row/box products, ordered by the coefficient sign; each computed bound is
/// then pushed one ULP outward (`next_down` for the lower, `next_up` for the
/// upper). Overflow and non-finite inputs surface as non-finite bounds, which
/// the caller's refusing box constructor rejects.
fn outward_affine_enclosure(
    matrix: [[f64; 2]; 2],
    lo: [f64; 2],
    hi: [f64; 2],
) -> ([f64; 2], [f64; 2]) {
    let mut out_lo = [0.0f64; 2];
    let mut out_hi = [0.0f64; 2];
    for row in 0..2 {
        let mut lo_acc = 0.0f64;
        let mut hi_acc = 0.0f64;
        for col in 0..2 {
            let coeff = matrix[row][col];
            let edge_lo = coeff * lo[col];
            let edge_hi = coeff * hi[col];
            if coeff >= 0.0 {
                lo_acc += edge_lo;
                hi_acc += edge_hi;
            } else {
                lo_acc += edge_hi;
                hi_acc += edge_lo;
            }
        }
        out_lo[row] = lo_acc.next_down();
        out_hi[row] = hi_acc.next_up();
    }
    (out_lo, out_hi)
}

/// A dyadic refinement request on `[a, b]` (spec §4.3): a finite
/// prefix-closed set of binary node addresses at depth `d`.
#[derive(Debug, Clone)]
pub struct DyadicRequest {
    /// The interval's lower end.
    pub a: f64,
    /// The interval's upper end.
    pub b: f64,
    /// The common depth of the stored addresses.
    pub depth: u32,
    /// The stored leaf addresses at `depth` (each `k` in `0..2^depth`).
    pub leaves: BTreeSet<u64>,
}

/// The result of a dyadic [`join`]: the union of the requesters' address sets
/// at one common depth, over the base request's interval.
#[derive(Debug, Clone)]
pub struct EdgeSampleSet {
    /// The interval's lower end.
    pub a: f64,
    /// The interval's upper end.
    pub b: f64,
    /// The common depth of the stored node addresses.
    pub depth: u32,
    /// The union of node addresses, prefix-closed at `depth`.
    pub nodes: BTreeSet<u64>,
}

/// The §4.3 sampling-policy spelling local to this guard.
///
/// The landed `truck-geometry` `SamplingPolicy` is NOT modified (write-set
/// discipline); the C1 wave wires the real policy type to
/// [`refuse_custom_on_shared`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingFlag {
    /// A dyadic refinement request.
    Dyadic,
    /// A CustomParameters-style request.
    Custom,
}

/// §4.3 join: the set union of prefix-closed dyadic address sets.
///
/// Requests at a depth shallower than the common depth lift to it by exact
/// integer expansion: an address `k` at depth `d'` expands to the
/// `2^(d - d')` children `k * 2^(d - d') .. (k + 1) * 2^(d - d') - 1`
/// (prefix-closed semantics make the expansion exact integer work). The join
/// itself never compares floats; the only arithmetic is the integer expansion
/// and the [`BTreeSet`] union, so the result is associative, commutative, and
/// idempotent. A request whose depth exceeds the addressable maximum or whose
/// leaves escape `0..2^depth` refuses.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn join(base: DyadicRequest, others: &[DyadicRequest]) -> Construction<EdgeSampleSet> {
    if base.depth > MAX_ADDRESSABLE_DEPTH {
        return Err(unaddressable_depth(base.depth));
    }
    let base_bound = 1u64 << base.depth;
    if let Some(&leaf) = base.leaves.iter().find(|&&k| k >= base_bound) {
        return Err(leaf_out_of_range(leaf, base.depth));
    }
    let mut depth = base.depth;
    for other in others {
        if other.depth > MAX_ADDRESSABLE_DEPTH {
            return Err(unaddressable_depth(other.depth));
        }
        let bound = 1u64 << other.depth;
        if let Some(&leaf) = other.leaves.iter().find(|&&k| k >= bound) {
            return Err(leaf_out_of_range(leaf, other.depth));
        }
        depth = depth.max(other.depth);
    }
    let mut nodes = BTreeSet::new();
    lift_leaves(&base, depth, &mut nodes);
    for other in others {
        lift_leaves(other, depth, &mut nodes);
    }
    Ok(EdgeSampleSet {
        a: base.a,
        b: base.b,
        depth,
        nodes,
    })
}

/// Generate `a + (b - a) * k / 2^d` for every address `k`, in ascending
/// address order (the fixed formula in the fixed order).
pub fn sample_parameters(s: &EdgeSampleSet) -> Vec<f64> {
    let scale = if s.depth <= MAX_ADDRESSABLE_DEPTH {
        (1u64 << s.depth) as f64
    } else {
        f64::INFINITY
    };
    let width = s.b - s.a;
    s.nodes
        .iter()
        .map(|&k| s.a + width * (k as f64) / scale)
        .collect()
}

/// The §4.3 shared-edge guard: a CustomParameters-style request on an edge
/// incident to more than one face refuses [`RefusalKind::NonDyadicSharedRequest`]
/// (Disproven); a dyadic request is always admitted.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
pub fn refuse_custom_on_shared(face_count: usize, policy: SamplingFlag) -> Construction<()> {
    if face_count > 1 && policy == SamplingFlag::Custom {
        return Err(Refusal::new(
            RefusalKind::NonDyadicSharedRequest,
            RefusalEvidence::Predicate {
                name: "non_dyadic_shared_request",
                detail: format!(
                    "a custom sample on an edge shared by {face_count} faces is not dyadic"
                ),
            },
        ));
    }
    Ok(())
}

/// Expand one request's leaves to `depth` (which is at least the request's own
/// depth) into `nodes`, by exact integer bit operations.
fn lift_leaves(req: &DyadicRequest, depth: u32, nodes: &mut BTreeSet<u64>) {
    let span = 1u64 << (depth - req.depth);
    for &leaf in &req.leaves {
        let start = leaf * span;
        nodes.extend(start..(start + span));
    }
}

/// The refusal for a request deeper than the `u64` address space.
fn unaddressable_depth(depth: u32) -> Refusal {
    refusal(
        RefusalKind::ClaimRefuted,
        "dyadic_depth_unaddressable",
        format!("depth {depth} exceeds the addressable maximum {MAX_ADDRESSABLE_DEPTH}"),
    )
}

/// The refusal for a leaf address that escapes `0..2^depth`.
fn leaf_out_of_range(leaf: u64, depth: u32) -> Refusal {
    refusal(
        RefusalKind::ClaimRefuted,
        "dyadic_leaf_out_of_range",
        format!("leaf {leaf} is not below 2^{depth}"),
    )
}

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}
