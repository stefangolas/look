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

//! The swept-pair admission dispatch contract (ADM-000-CONTRACT): the frozen
//! contract of the swept-pair admission program — the theorems A–D carrier
//! names, the representation-adapter signature (Theorem A), the certificate
//! carrier types (Theorems B1/B2/C), and the refusing-by-default dispatch
//! rule. The design spec is `docs/SWEPT_PAIR_ADMISSION_SPEC.md`; its §3 table
//! is the single booking surface and this module lands the registered shapes.
//!
//! **What this packet lands.** Types, refusing constructors, and the mapping
//! only. No solver body lives anywhere in this file (D-shim discipline): the
//! adapter refuses `ConstructRefusal::InvalidInput`, every certificate
//! constructor refuses `ConstructRefusal::Unfrozen`, and the extraction
//! contract is recorded so ADM-001 implements the adapter and ADM-002 produces
//! the certificates without a signature change.
//!
//! **Theorem A — the adapter signature.** [`admit_tensor_spline_pair`]
//! consumes two positive-weight tensor-spline faces (B-spline/NURBS in the
//! homogeneous `BSplineSurface<Vector4>` carrier — Bézier extraction is an
//! exact local change of basis per knot rectangle) and yields the
//! POLYNOMIALIZED interaction system `F = Ŵ_Y·Â − Ŵ_X·B̂` over the extracted
//! spans (positive weights make the clearing exact), wired toward the landed
//! [`Ssi4System`](crate::construct::bie::ssi4::Ssi4System). The concrete
//! `SsiPairSystem` constructor is ADM-001's; here the adapter REFUSES
//! [`ConstructRefusal::InvalidInput`] (D-shim) — no pair is admitted yet. The
//! signature is stated over the landed carrier and refusal types without any
//! change to `bie/ssi4.rs` or the Krawczyk operator (frozen), so the stop
//! condition on an operator change is NOT triggered (assessed below).
//!
//! **Theorems B1/B2/C — the certificate carriers.** [`RegularPatch`] (the
//! Theorem B1 hemisphere regularity certificate over a face patch, carrying
//! the certified normal cone), [`CollapsedBoundary`] (Theorem B1 collapsed-edge
//! deflation: the boundary multiplicity `k`), [`SeamIdentified`] (Theorem B1
//! seam identification: the paired BRep edges), and [`TransversePair`] (Theorem
//! C transversality: the certified minimal `‖n_X × n_Y‖` enclosure with a
//! strictly positive lower bound, the landed `GateAdmission::TangencyFree`
//! margin shape). Production belongs to ADM-002-CERTIFICATES; every
//! constructor here refuses [`ConstructRefusal::Unfrozen`].
//!
//! **Dispatch rule (V5).** The boolean boundary consults admission BEFORE its
//! `NonCanonicalCarrier` refusal: not-yet-admitted carrier forms keep the
//! exact current refusal, and the adapter fires only where the old path
//! returned `NonCanonicalCarrier` — nothing already-green can reach it. The
//! `NonCanonicalCarrier` origin is the boolean boundary's envelope lift
//! (`Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)`), where
//! the PB-011C census recorded it (typed refusal at the boundary, before any
//! 4-D continuation budget is spent); the sweep×sweep pair cells refuse there
//! today. That matches the census site, so the stop condition on the refusal
//! origin is NOT triggered. In ADM-000 the adapter admits nothing, so the
//! boundary keeps returning `NonCanonicalCarrier` for every not-yet-admitted
//! form; admission widens case by case behind admitting tests (ADM-001+).
//!
//! **Lazy-extraction contract (scope decision 5).** The signature carries the
//! culling order ([`CullingOrder`]): face pairs are culled by
//! control-hull/AABB first, then the surviving pair's knot spans are
//! extracted. Extraction operators are cached per knot configuration. ADM-001
//! implements the adapter against this fixed order and the [`AdmitDomainHint`]
//! carrier without a signature change.
//!
//! **Spec §3 mapping rows.** adapter → [`Ssi4System`](crate::construct::bie::ssi4::Ssi4System)
//! (the polynomialized F-form drives the same frozen Krawczyk operator);
//! certificates → the landed certificate tuples (`GateAdmission::TangencyFree`
//! margin shape, the §4 evidence vocabulary); volume primitive → ADM-003's
//! booking. Registered in `docs/SWEPT_PAIR_ADMISSION_SPEC.md` §3.
//!
//! **H-1.** This module carries `#![deny(clippy::unwrap_used)]`, no `unwrap`,
//! no `expect`, no `panic!`, and no module-level `allow`.

use crate::construct::refusal::ConstructRefusal;
use truck_geometry::prelude::{BSplineSurface, Vector4};

/// The polynomialized swept-pair interaction system (Theorem A result carrier).
///
/// `F = Ŵ_Y·Â − Ŵ_X·B̂` over the extracted tensor-Bernstein spans of the two
/// positive-weight faces, in the shape that ADM-001's constructor realizes
/// toward the landed
/// [`Ssi4System`](crate::construct::bie::ssi4::Ssi4System) and the frozen
/// Krawczyk operator (`truck-evidence` `num/krawczyk.rs`). Nothing is
/// constructible in ADM-000 — the adapter is the only entry and it refuses;
/// ADM-001 owns the constructor that builds a value. No carrier pair is
/// admitted until that constructor lands.
#[derive(Clone, Debug)]
pub struct SsiPairSystem {
    /// Sealed: ADM-001 owns the constructor. ADM-000 freezes the carrier name
    /// and the theorem-A signature, nothing more.
    _sealed: (),
}

/// The v1 lazy-extraction culling order (scope decision 5), carried by the
/// adapter signature so ADM-001 implements it without a signature change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CullingOrder {
    /// Cull face pairs by control-hull / AABB first, then extract the knot
    /// spans of the surviving pair (the fixed v1 order; extraction operators
    /// are cached per knot configuration).
    HullAabbThenKnotSpan,
}

impl CullingOrder {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::HullAabbThenKnotSpan => "hull_aabb_then_knot_span",
        }
    }
}

/// The domain hint carried by the Theorem A adapter signature.
///
/// The hint carries the lazy-extraction culling order (scope decision 5); the
/// participating parameter domains are the faces' declared clamped domains,
/// and the extraction/caching contract is recorded in the module doc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmitDomainHint {
    /// The v1 lazy-extraction culling order.
    pub order: CullingOrder,
}

/// The Theorem A representation adapter (D-shim in ADM-000).
///
/// Consumes two positive-weight tensor-spline faces (`X` first, `Y` second;
/// B-spline/NURBS in the homogeneous carrier — Bézier extraction is an exact
/// local change of basis per knot rectangle, and the `F = Ŵ_Y·Â − Ŵ_X·B̂`
/// clearing is exact because the weights are strictly positive). Returns the
/// polynomialized interaction system `SsiPairSystem`, wired toward the landed
/// [`Ssi4System`](crate::construct::bie::ssi4::Ssi4System). The concrete
/// constructor is ADM-001's; ADM-000 REFUSES [`ConstructRefusal::InvalidInput`]
/// on every call (D-shim) — no pair is admitted yet, so a boolean boundary
/// that consults this adapter keeps its current `NonCanonicalCarrier` refusal
/// unchanged (the V5 dispatch rule).
///
/// This signature is the frozen contract: it is stated over the landed
/// homogeneous carrier and refusal types with no change to `bie/ssi4.rs` and
/// no change to the (frozen) Krawczyk operator, so ADM-001 implements the body
/// behind the refusing-by-default posture without a signature change.
pub fn admit_tensor_spline_pair(
    _x: &BSplineSurface<Vector4>,
    _y: &BSplineSurface<Vector4>,
    _domain_hint: &AdmitDomainHint,
) -> Result<SsiPairSystem, ConstructRefusal> {
    Err(ConstructRefusal::InvalidInput)
}

/// A certified per-face normal cone (Theorem B1 substrate).
///
/// The cone is anchored at the float midpoint normal direction `anchor` with a
/// certified bound `s_up` on the sine of the half-angle: for every surface
/// normal in the face's box, `sin ∠(n, anchor) ≤ s_up < 1` (the public shape of
/// the landed gate's cone; `ssi_gate.rs`). A `RegularPatch` carries it as the
/// evidence that the parameterization is regular over the certified box.
/// Production belongs to ADM-002-CERTIFICATES; the refusing constructor below
/// is the frozen posture until then.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalCone {
    /// The anchor direction (the float midpoint normal, rounded to dyadic).
    pub anchor: [f64; 3],
    /// A certified upper bound of the sine of the cone half-angle, `< 1`.
    pub s_up: f64,
}

impl NormalCone {
    /// The refusing constructor (production is ADM-002-CERTIFICATES).
    pub fn try_new(_anchor: [f64; 3], _s_up: f64) -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}

/// The Theorem B1 hemisphere regularity certificate (a regular face patch).
///
/// A face sub-box certified regular by the hemisphere certificate: a dyadic
/// direction `c` picked from the floating midpoint normal with
/// `min(bernstein coefficients of c·M) > 0` over the box (M the polynomial
/// normal numerator) — the convex-hull property makes the whole box regular.
/// The certified normal [`cone`](Self::cone) is the certificate's payload.
/// Production belongs to ADM-002-CERTIFICATES.
#[derive(Debug, Clone, PartialEq)]
pub struct RegularPatch {
    /// The certified normal cone of the regular patch.
    pub cone: NormalCone,
}

impl RegularPatch {
    /// The refusing constructor (production is ADM-002-CERTIFICATES).
    pub fn try_new(_cone: NormalCone) -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}

/// A BRep-edge identifier of a face's boundary enumeration (seam evidence).
///
/// An ordinal handle naming an edge of the certified boundary (the seam's
/// edge complex); a [`SeamIdentified`] pairs two such handles. The precise
/// enumeration contract is ADM-002's production detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeId(pub usize);

/// The Theorem B1 collapsed-edge deflation certificate.
///
/// A boundary stratum intentionally collapsed to a point carries the known
/// factor `(1−v)^k` in the normal numerator `M`; this carrier records the
/// multiplicity `k` ADM-002 divided out and certified the quotient over.
/// Production belongs to ADM-002-CERTIFICATES.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollapsedBoundary {
    /// The multiplicity of the collapsed boundary edge.
    pub multiplicity: usize,
}

impl CollapsedBoundary {
    /// The refusing constructor (production is ADM-002-CERTIFICATES).
    pub fn try_new(_multiplicity: usize) -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}

/// The Theorem B1 seam-identification certificate.
///
/// `X(u, 0) = X(u, 1)` certified exactly (`A_0·W_1 − A_1·W_0 ≡ 0` over aligned
/// degrees/knots): the identified seam is a pair of BRep edges, not a
/// singularity. Production belongs to ADM-002-CERTIFICATES.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeamIdentified {
    /// The two paired BRep edges of the certified seam.
    pub paired: (EdgeId, EdgeId),
}

impl SeamIdentified {
    /// The refusing constructor (production is ADM-002-CERTIFICATES).
    pub fn try_new(_paired: (EdgeId, EdgeId)) -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}

/// The Theorem C transversality certificate of a pair product box.
///
/// `delta` is the certified enclosure `(lo, hi)` of the minimal cross-product
/// magnitude `‖n_X × n_Y‖` over the product box with `lo > 0` — the landed
/// `GateAdmission::TangencyFree { margin }` shape, which certifies `rank DF =
/// 3` on `Σ ∩ box`. Production belongs to ADM-002-CERTIFICATES.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransversePair {
    /// The certified minimal `‖n_X × n_Y‖` enclosure, `(lo, hi)` with `lo > 0`.
    pub delta: (f64, f64),
}

impl TransversePair {
    /// The refusing constructor (production is ADM-002-CERTIFICATES).
    pub fn try_new(_delta: (f64, f64)) -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use truck_geometry::prelude::KnotVec;

    /// A positive-weight homogeneous Bézier patch over the unit square
    /// `[0, 1]²`, offset along `x`, with all weights `1` (a valid
    /// positive-weight tensor-spline face for exercising the adapter).
    fn bezier_patch(offset: f64) -> BSplineSurface<Vector4> {
        let knots = (
            KnotVec::from(vec![0.0f64, 0.0, 0.0, 1.0, 1.0, 1.0]),
            KnotVec::from(vec![0.0f64, 0.0, 0.0, 1.0, 1.0, 1.0]),
        );
        let p = |x: f64, y: f64| Vector4::new(x + offset, y, x * y, 1.0);
        let control_points = vec![
            vec![p(0.0, 0.0), p(0.5, 0.0), p(1.0, 0.0)],
            vec![p(0.0, 0.5), p(0.5, 0.5), p(1.0, 0.5)],
            vec![p(0.0, 1.0), p(0.5, 1.0), p(1.0, 1.0)],
        ];
        BSplineSurface::new(knots, control_points)
    }

    #[test]
    fn admission_contract_refuses_by_default() {
        // The Theorem A adapter is a D-shim: even a valid positive-weight pair
        // refuses InvalidInput — nothing is admitted until ADM-001 lands the
        // constructor. A boolean boundary consulting admission therefore keeps
        // its current NonCanonicalCarrier refusal unchanged (V5 rule).
        let x = bezier_patch(0.0);
        let y = bezier_patch(3.0);
        let hint = AdmitDomainHint {
            order: CullingOrder::HullAabbThenKnotSpan,
        };
        assert!(
            matches!(
                admit_tensor_spline_pair(&x, &y, &hint),
                Err(ConstructRefusal::InvalidInput)
            ),
            "the D-shim adapter must refuse every pair by default"
        );

        // The certificate carriers carry refusing constructors only: their
        // production is ADM-002-CERTIFICATES, so nothing certifies here.
        let cone = NormalCone {
            anchor: [0.0, 0.0, 1.0],
            s_up: 0.5,
        };
        assert_eq!(RegularPatch::try_new(cone), Err(ConstructRefusal::Unfrozen));
        assert_eq!(
            CollapsedBoundary::try_new(2),
            Err(ConstructRefusal::Unfrozen)
        );
        assert_eq!(
            SeamIdentified::try_new((EdgeId(0), EdgeId(1))),
            Err(ConstructRefusal::Unfrozen)
        );
        assert_eq!(
            TransversePair::try_new((1.0e-2, 1.0e-1)),
            Err(ConstructRefusal::Unfrozen)
        );
    }

    /// The exact Theorem A fn-pointer type, factored so the whole signature is
    /// nameable in one place.
    type AdapterSignature = fn(
        &BSplineSurface<Vector4>,
        &BSplineSurface<Vector4>,
        &AdmitDomainHint,
    ) -> Result<SsiPairSystem, ConstructRefusal>;

    #[test]
    fn adapter_signature_types_are_compile_checked() {
        // The exact Theorem A fn type must be nameable with the frozen
        // argument and result types: two positive-weight tensor-spline faces
        // (the landed homogeneous `BSplineSurface<Vector4>` carrier) plus the
        // domain hint, returning `Result<SsiPairSystem, ConstructRefusal>`.
        // Binding the item to its annotated fn-pointer type is the compile
        // check of the whole signature.
        let _signature: AdapterSignature = admit_tensor_spline_pair;

        // The frozen culling-order tag is part of the carried contract.
        assert_eq!(
            CullingOrder::HullAabbThenKnotSpan.tag(),
            "hull_aabb_then_knot_span"
        );
    }
}
