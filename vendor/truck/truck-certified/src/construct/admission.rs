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

//! The swept-pair admission dispatch contract (ADM-000-CONTRACT) and the
//! Theorem A ASSEMBLY (ADM-001-ADAPTER): the frozen carrier names and the
//! refusing-by-default dispatch rule of ADM-000, extended by the ADM-001
//! assembly — the [`SsiPairSystem`] constructor composes the landed lemma
//! kernels (L1 exact Bézier extraction `extract_patches` per knot rectangle and
//! L2 tensor-Bernstein product `patch_product` per span pair) into the
//! polynomialized pair system `F = Ŵ_Y·Â − Ŵ_X·B̂`, and the adapter admits the
//! FIRST pair class (ruled-section lofts, scope decision 2) behind its
//! admitting conformance test. The design spec is
//! `docs/SWEPT_PAIR_ADMISSION_SPEC.md`; its §3 table is the single booking
//! surface and this module lands the registered shapes.
//!
//! **What ADM-000 landed.** The theorems A–D carrier names, the Theorem A
//! representation-adapter signature over the landed homogeneous
//! `BSplineSurface<Vector4>` carrier, and the V5 dispatch rule. In ADM-000 the
//! adapter refused [`ConstructRefusal::InvalidInput`] on every call and every
//! certificate constructor refused [`ConstructRefusal::Unfrozen`].
//!
//! **What ADM-001 lands.** The assembly and the first admitted class. No solver
//! body lives in this file — the assembly is exact coefficient algebra over the
//! frozen homogeneous carrier, and the Krawczyk operator / `bie/ssi4.rs` system
//! family are untouched (the ADM-000 mapping row holds, so the stop condition
//! on an operator change is NOT triggered). The certificate carriers keep
//! refusing [`ConstructRefusal::Unfrozen`] (production is ADM-002-CERTIFICATES);
//! general spline-section pairs still refuse typed exactly as today (V5).
//!
//! **Theorem A — the adapter signature.** [`admit_tensor_spline_pair`]
//! consumes two positive-weight tensor-spline faces (`X` first, `Y` second;
//! B-spline/NURBS in the homogeneous carrier — Bézier extraction is an exact
//! local change of basis per knot rectangle, and the `F = Ŵ_Y·Â − Ŵ_X·B̂`
//! clearing is exact because the weights are strictly positive). It returns the
//! assembled polynomialized interaction system [`SsiPairSystem`]: the two
//! faces' extracted tensor-Bernstein spans (the `X` face under face ordinal 0,
//! the `Y` face under ordinal 1) plus, per span pair, the exact clearing
//! assemblies of the L2 product lemma over the pair's shared chart.
//!
//! **The polynomialized form.** On each knot rectangle a positive-weight
//! rational face writes `X = Â/Ŵ_X` and `Y = B̂/Ŵ_Y` with `Â`, `B̂` the
//! tensor-Bernstein numerator nets and `Ŵ_X`, `Ŵ_Y` the strictly positive weight
//! fields. Over a shared chart the componentwise clearing identity
//!
//! ```text
//! F(u, v, s, t) = Ŵ_Y(s, t)·Â(u, v) − Ŵ_X(u, v)·B̂(s, t)
//!              = Ŵ_X(u, v)·Ŵ_Y(s, t)·(X(u, v) − Y(s, t))
//! ```
//!
//! is exact (positive weights make the clearing exact), so the zeros of the
//! polynomialized `F` are EXACTLY the zeros of the direct difference `X − Y`.
//! Each clearing TERM `Ŵ_Y·Â` / `Ŵ_X·B̂` is a product of a weight field
//! replicated across the three coordinates and a numerator field over the same
//! span — the algebra ADM-L2-PRODUCT proved in advance of this assembly — so
//! ADM-001 materializes the two term patches per shared-chart span pair with
//! the landed [`patch_product`](crate::construct::prod::patch_product), never
//! a sampled or truncated product. A span pair over distinct knot rectangles
//! stores no aligned term patches (the per-side spans alone determine the
//! product-chart residual, evaluated per side downstream).
//!
//! **Admission (scope decision 2, V5).** ADM-001 admits the FIRST pair class
//! only: the **ruled-section-loft pair** — a pair of faces whose every
//! extracted span is linear in the loft (station) axis (`v`-degree 1, the
//! straight-generator ruled-loft shape). A general spline-section pair (a face
//! with a higher `v`-degree span) refuses [`ConstructRefusal::InvalidInput`]
//! typed exactly as today; general sections admit ONLY after
//! ADM-002-CERTIFICATES lands its certificates (scope decision 2). The adapter
//! fires only where the old path returned `NonCanonicalCarrier` — nothing
//! already-green can reach it, and admission widens monotonically.
//!
//! **Lazy-extraction contract (scope decision 5).** The signature carries the
//! culling order ([`CullingOrder`]): face pairs are culled by control-hull/AABB
//! first, then the surviving pair's knot spans are extracted, with extraction
//! operators cached per knot configuration. The Theorem A adapter here is the
//! single-face-pair entry: it receives the already-culled face pair and
//! extracts that survivor's knot spans (the fixed `HullAabbThenKnotSpan`
//! order's second step), assembling the span-pair system over the extracted
//! spans in the fixed row-major `(x-span, y-span)` order. Span-level culling
//! and the operator cache belong to the downstream funnel enumeration
//! (ADM-004-FUNNEL-WIRING), which consumes this carrier's span-pair shape.
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
//! returned `NonCanonicalCarrier` — nothing already-green can reach it. In
//! ADM-001 the adapter admits only the ruled-section-loft class behind its
//! admitting test (ADM-001-ADAPTER), so the boundary keeps returning
//! `NonCanonicalCarrier` for every not-yet-admitted form; admission widens
//! case by case behind admitting tests (ADM-002+).
//!
//! **Spec §3 mapping rows.** adapter → [`Ssi4System`](crate::construct::bie::ssi4::Ssi4System)
//! (the polynomialized F-form drives the same frozen Krawczyk operator;
//! `bie/ssi4.rs` and the operator are untouched, no SPEC_GAP);
//! certificates → the landed certificate tuples (`GateAdmission::TangencyFree`
//! margin shape, the §4 evidence vocabulary); volume primitive → ADM-003's
//! booking. Registered in `docs/SWEPT_PAIR_ADMISSION_SPEC.md` §3.
//!
//! **H-1.** This module carries `#![deny(clippy::unwrap_used)]`, no `unwrap`,
//! no `expect`, no `panic!`, and no module-level `allow`.

use crate::construct::extract::extract_patches;
use crate::construct::patches::{PatchParent, TensorBernsteinPatch};
use crate::construct::prod::patch_product;
use crate::construct::refusal::ConstructRefusal;
use truck_geometry::prelude::{BSplineSurface, Vector4};

/// The polynomialized swept-pair interaction system (Theorem A result carrier,
/// ADM-001-ADAPTER assembly landed).
///
/// `F = Ŵ_Y·Â − Ŵ_X·B̂` over the extracted tensor-Bernstein spans of the two
/// positive-weight faces, in the per-side span-pair shape the landed Ssi4
/// square-system family consumes (per-side carriers, never a materialized
/// 4-axis grid). The carrier holds the extracted spans of the `X` face (face
/// ordinal 0) and of the `Y` face (face ordinal 1), plus, per span pair, the
/// exact clearing term patches of the L2 product lemma over the pair's shared
/// chart. The carrier performs no solving: the Krawczyk operator is
/// instantiated downstream on this input shape (ADM-004-FUNNEL-WIRING).
///
/// ADM-001 lands the constructor (the assembly over the proven lemmas) and
/// widens the adapter to the first admitted class (ruled-section lofts, scope
/// decision 2). The certificate carriers keep refusing
/// [`ConstructRefusal::Unfrozen`]: general spline-section pairs admit ONLY
/// after ADM-002-CERTIFICATES lands those carriers' production, so until then
/// the adapter refuses them typed exactly as today.
#[derive(Clone, Debug)]
pub struct SsiPairSystem {
    /// The extracted spans of the first (`X`) face, under face ordinal 0.
    x_spans: Vec<TensorBernsteinPatch>,
    /// The extracted spans of the second (`Y`) face, under face ordinal 1.
    y_spans: Vec<TensorBernsteinPatch>,
    /// The per-span-pair clearing assemblies of the polynomialized system.
    pairs: Vec<SpanPairClearing>,
    /// The lazy-extraction domain hint the assembly was called with.
    hint: AdmitDomainHint,
}

impl SsiPairSystem {
    /// Assembles the polynomialized pair system over two positive-weight faces.
    ///
    /// Every knot rectangle of both faces is Bézier-extracted (L1, exact) and
    /// re-parented to the ADM-001 pair numbering (`X` face ordinal 0, `Y` face
    /// ordinal 1). For every `(x-span, y-span)` pair in the fixed row-major
    /// order the exact clearing term patches of the L2 product lemma are
    /// assembled whenever the two spans' source-domain boxes coincide exactly
    /// (a shared chart): term 1 = `Ŵ_Y·Â`, term 2 = `Ŵ_X·B̂`, each the
    /// componentwise product of a weight-replicated field and a numerator-only
    /// field over the shared span. A span pair over distinct rectangles stores
    /// no aligned terms — its product-chart residual is the per-side separable
    /// difference of the two stored spans.
    ///
    /// # Errors
    ///
    /// Refuses [`ConstructRefusal::InvalidInput`] when either face does not
    /// extract (an empty, ragged, or non-finite net, an unclamped or
    /// degenerate knot grid, or a span whose weight bracket cannot be
    /// certified positive) or when either face contributes no span.
    pub fn assemble(
        x: &BSplineSurface<Vector4>,
        y: &BSplineSurface<Vector4>,
        hint: &AdmitDomainHint,
    ) -> Result<Self, ConstructRefusal> {
        let x_spans = extract_patches(x)?;
        let y_spans = extract_patches(y)?;
        Self::from_spans(x_spans, y_spans, *hint)
    }

    /// Builds the carrier from already-extracted span stacks (the L1 results
    /// of the two faces, in order), owning the ADM-001 pair numbering.
    fn from_spans(
        x_spans: Vec<TensorBernsteinPatch>,
        y_spans: Vec<TensorBernsteinPatch>,
        hint: AdmitDomainHint,
    ) -> Result<Self, ConstructRefusal> {
        if x_spans.is_empty() || y_spans.is_empty() {
            return Err(ConstructRefusal::InvalidInput);
        }
        let x_spans = x_spans
            .iter()
            .map(|span| reparent(span, 0))
            .collect::<Result<Vec<_>, _>>()?;
        let y_spans = y_spans
            .iter()
            .map(|span| reparent(span, 1))
            .collect::<Result<Vec<_>, _>>()?;
        let mut pairs = Vec::with_capacity(x_spans.len() * y_spans.len());
        for (ix, x_span) in x_spans.iter().enumerate() {
            for (iy, y_span) in y_spans.iter().enumerate() {
                let aligned_terms = if x_span.domain() == y_span.domain() {
                    // Shared-chart span pair: the two clearing terms of
                    // `F = Ŵ_Y·Â − Ŵ_X·B̂` are products of a weight field
                    // replicated across the three coordinates and a numerator
                    // field over the SAME span — exactly the L2 product-lemma
                    // composition (never a sampled or truncated product).
                    let term1 =
                        patch_product(&weight_replicated(y_span)?, &numerator_only(x_span)?)?;
                    let term2 =
                        patch_product(&weight_replicated(x_span)?, &numerator_only(y_span)?)?;
                    Some([term1, term2])
                } else {
                    None
                };
                pairs.push(SpanPairClearing {
                    x_span: ix,
                    y_span: iy,
                    aligned_terms,
                });
            }
        }
        Ok(SsiPairSystem {
            x_spans,
            y_spans,
            pairs,
            hint,
        })
    }

    /// The extracted spans of the first (`X`) face (face ordinal 0).
    pub fn x_spans(&self) -> &[TensorBernsteinPatch] {
        &self.x_spans
    }

    /// The extracted spans of the second (`Y`) face (face ordinal 1).
    pub fn y_spans(&self) -> &[TensorBernsteinPatch] {
        &self.y_spans
    }

    /// The per-span-pair clearing assemblies, in the fixed `(x, y)` row-major
    /// order of the extraction.
    pub fn span_pairs(&self) -> &[SpanPairClearing] {
        &self.pairs
    }

    /// The number of assembled `(x-span, y-span)` pairs.
    pub fn span_pair_count(&self) -> usize {
        self.pairs.len()
    }

    /// The lazy-extraction domain hint the assembly was called with.
    pub fn hint(&self) -> AdmitDomainHint {
        self.hint
    }
}

/// One assembled `(x-span, y-span)` pair of the polynomialized system.
///
/// The pair names its two source spans by index into the system's
/// [`x_spans`](SsiPairSystem::x_spans) / [`y_spans`](SsiPairSystem::y_spans)
/// stacks. When the two spans' source-domain boxes coincide exactly, the pair
/// additionally carries the two exact clearing TERM patches of the L2 product
/// lemma: `terms[0] = Ŵ_Y·Â` (term 1) and `terms[1] = Ŵ_X·B̂` (term 2) over the
/// shared chart — the polynomialized clearing numerator of the pair is their
/// coefficient difference. A pair over distinct rectangles stores no aligned
/// terms; its product-chart residual is the per-side separable difference of
/// the two stored spans.
#[derive(Clone, Debug)]
pub struct SpanPairClearing {
    /// The index of the `X`-side span in [`SsiPairSystem::x_spans`].
    x_span: usize,
    /// The index of the `Y`-side span in [`SsiPairSystem::y_spans`].
    y_span: usize,
    /// The two aligned clearing term patches (`Ŵ_Y·Â`, `Ŵ_X·B̂`), present only
    /// for a shared-chart span pair.
    aligned_terms: Option<[TensorBernsteinPatch; 2]>,
}

impl SpanPairClearing {
    /// The index of the `X`-side span in the system's `x` stack.
    pub fn x_span(&self) -> usize {
        self.x_span
    }

    /// The index of the `Y`-side span in the system's `y` stack.
    pub fn y_span(&self) -> usize {
        self.y_span
    }

    /// The two aligned clearing term patches over the shared chart
    /// (`terms[0] = Ŵ_Y·Â`, `terms[1] = Ŵ_X·B̂`), when the span pair's source
    /// boxes coincide.
    pub fn aligned_terms(&self) -> Option<&[TensorBernsteinPatch; 2]> {
        self.aligned_terms.as_ref()
    }
}

/// Re-encodes an extracted span under the ADM-001 pair numbering (`face`
/// ordinal 0 for the `X` face, 1 for the `Y` face), preserving every other
/// field (the re-parented span re-certifies its weight bracket through the
/// refusing constructor — a landed span is positive by construction).
fn reparent(
    span: &TensorBernsteinPatch,
    face: usize,
) -> Result<TensorBernsteinPatch, ConstructRefusal> {
    TensorBernsteinPatch::try_new(
        span.numerator().to_vec(),
        span.weights().to_vec(),
        span.domain(),
        PatchParent::new(face, span.parent().edge),
    )
}

/// The numerator-only re-encoding of a span: the same `R³` numerator field `Â`
/// with a unit weight grid, over the same span. The L2 clearing products
/// multiply this "numerator field" patch against a weight-replicated patch of
/// the other span (Theorem A's clearing algebra, `patch_product` on the shared
/// span).
fn numerator_only(span: &TensorBernsteinPatch) -> Result<TensorBernsteinPatch, ConstructRefusal> {
    let unit_weights: Vec<Vec<f64>> = span
        .weights()
        .iter()
        .map(|row| vec![1.0; row.len()])
        .collect();
    TensorBernsteinPatch::try_new(
        span.numerator().to_vec(),
        unit_weights,
        span.domain(),
        span.parent(),
    )
}

/// The weight-replicated re-encoding of a span: a field whose three coordinates
/// each equal the span's weight field `Ŵ`, over a unit weight grid, over the
/// same span. Its componentwise product with a numerator-only patch is exactly
/// `Ŵ` times the numerator field (L2 clearing term).
fn weight_replicated(
    span: &TensorBernsteinPatch,
) -> Result<TensorBernsteinPatch, ConstructRefusal> {
    let replicated: Vec<Vec<[f64; 3]>> = span
        .weights()
        .iter()
        .map(|row| row.iter().map(|&w| [w, w, w]).collect())
        .collect();
    let unit_weights: Vec<Vec<f64>> = span
        .weights()
        .iter()
        .map(|row| vec![1.0; row.len()])
        .collect();
    TensorBernsteinPatch::try_new(replicated, unit_weights, span.domain(), span.parent())
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

/// The Theorem A representation adapter (ADM-001-ADAPTER).
///
/// Consumes two positive-weight tensor-spline faces (`X` first, `Y` second;
/// B-spline/NURBS in the homogeneous carrier — Bézier extraction is an exact
/// local change of basis per knot rectangle, and the `F = Ŵ_Y·Â − Ŵ_X·B̂`
/// clearing is exact because the weights are strictly positive). Returns the
/// assembled polynomialized interaction system [`SsiPairSystem`], in the
/// per-side span-pair shape the landed Ssi4 square-system family consumes.
///
/// **Admission (scope decision 2, V5).** The adapter fires only where the old
/// path returned `NonCanonicalCarrier` — nothing already-green can reach it.
/// ADM-001 admits the FIRST pair class only: the **ruled-section-loft pair**,
/// a pair of faces whose every extracted span is linear in the loft (station)
/// axis (`v`-degree 1, the straight-generator ruled-loft shape). A general
/// spline-section pair refuses [`ConstructRefusal::InvalidInput`] typed exactly
/// as today — general sections admit only after ADM-002-CERTIFICATES lands
/// (its carriers' constructors still refuse [`ConstructRefusal::Unfrozen`]).
pub fn admit_tensor_spline_pair(
    x: &BSplineSurface<Vector4>,
    y: &BSplineSurface<Vector4>,
    domain_hint: &AdmitDomainHint,
) -> Result<SsiPairSystem, ConstructRefusal> {
    // The fixed v1 lazy-extraction order (scope decision 5): the adapter is
    // the single-face-pair entry, so it receives the already-hull-culled
    // survivor and extracts that survivor's knot spans (the order's second
    // step). Every extraction and every class-gate refusal below is typed.
    let x_spans = extract_patches(x)?;
    let y_spans = extract_patches(y)?;
    if !admitted_ruled_section_loft_pair(&x_spans, &y_spans) {
        return Err(ConstructRefusal::InvalidInput);
    }
    SsiPairSystem::from_spans(x_spans, y_spans, *domain_hint)
}

/// The ADM-001 admitted first class (scope decision 2): the **ruled-section
/// loft pair**. Every extracted span of both faces must be linear in the loft
/// (station) axis — `v`-degree 1 — the straight-generator ruled-loft shape.
/// A pair whose faces carry a higher `v`-degree span (general spline sections)
/// is NOT admitted and refuses typed, exactly as today.
fn admitted_ruled_section_loft_pair(
    x_spans: &[TensorBernsteinPatch],
    y_spans: &[TensorBernsteinPatch],
) -> bool {
    let ruled = |spans: &[TensorBernsteinPatch]| {
        !spans.is_empty() && spans.iter().all(|span| span.degree().1 == 1)
    };
    ruled(x_spans) && ruled(y_spans)
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

    /// A ruled-section loft face `X(u, v) = (u + offset_x, u², v)` — bidegree
    /// `(2, 1)`, every extracted span linear in the loft axis `v` (the ADM-001
    /// admitted class shape), unit weights.
    fn ruled_loft_face(offset_x: f64) -> BSplineSurface<Vector4> {
        let knots = (
            KnotVec::from(vec![0.0f64, 0.0, 0.0, 1.0, 1.0, 1.0]),
            KnotVec::from(vec![0.0f64, 0.0, 1.0, 1.0]),
        );
        // x = u (offset), y = u², z = v over the degree-(2, 1) net.
        let u_coeff = [0.0, 0.5, 1.0];
        let u2_coeff = [0.0, 0.0, 1.0];
        let v_coeff = [0.0, 1.0];
        let mut control_points: Vec<Vec<Vector4>> = Vec::new();
        for (&xu, &yu) in u_coeff.iter().zip(u2_coeff.iter()) {
            let mut row: Vec<Vector4> = Vec::new();
            for &zv in &v_coeff {
                row.push(Vector4::new(xu + offset_x, yu, zv, 1.0));
            }
            control_points.push(row);
        }
        BSplineSurface::new(knots, control_points)
    }

    #[test]
    fn admission_contract_refuses_by_default() {
        // The Theorem A adapter is monotone-widening (V5): a general
        // spline-section pair (the biquadratic offset pair below is NOT a
        // ruled-section loft — its spans are `v`-degree 2) still refuses
        // InvalidInput: nothing is admitted except the first class
        // (ruled-section lofts), behind its admitting test.
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
            "a non-admitted (general-section) pair must refuse InvalidInput"
        );

        // A ruled-section loft pair IS admitted (the ADM-001 first class).
        let loft_x = ruled_loft_face(0.0);
        let loft_y = ruled_loft_face(0.25);
        let admitted = admit_tensor_spline_pair(&loft_x, &loft_y, &hint)
            .map(|system| system.span_pair_count() >= 1);
        assert_eq!(
            admitted,
            Ok(true),
            "a ruled-section loft pair must admit and assemble a non-empty system"
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
