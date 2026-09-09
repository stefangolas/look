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
//! and the operator cache belong to the funnel enumeration (ADM-004-FUNNEL-WIRING
//! lands the certified pair-cut row [`certify_admitted_cut_pair`] on this
//! carrier's span-pair shape; the Krawczyk operator is instantiated by the FSSI
//! solver layer downstream).
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

use crate::construct::deflate::{
    certify_deflated_interior, deflate_factor, seam_identified, DeflatedNumerator,
};
use crate::construct::extract::extract_patches;
use crate::construct::normal_cone::{
    assemble_normal_numerator, hemisphere_certificate, midpoint_normal_direction, normal_cone,
    NormalNumerator,
};
use crate::construct::patches::{PatchParent, PatchSide, TensorBernsteinPatch};
use crate::construct::prod::patch_product;
use crate::construct::refusal::ConstructRefusal;
use crate::construct::Interval;
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
/// instantiated downstream on this input shape — ADM-004-FUNNEL-WIRING lands
/// the integrated per-pair cut row
/// ([`certify_admitted_cut_pair`](crate::construct::admission::certify_admitted_cut_pair))
/// that wires this carrier with the certificate assembly and the Theorem C
/// transversality certificate.
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

// ---------------------------------------------------------------------------
// ADM-002-CERTIFICATES — the certificate assembly.
//
// This section lands the production of the Theorem B1/B2/C certificates: the
// four-outcome dispatch that composes the landed lemma kernels (L1 extraction
// substrate, L3 normal numerator / hemisphere certificate / normal cone, L4
// deflation / seam identity) into a per-face typed verdict. It is ASSEMBLY
// ONLY: no lemma kernel body and no shim type is edited here, and the frozen
// ADM-000 refusing constructors above are untouched. The dispatch PRODUCES the
// frozen carrier values ([`RegularPatch`], [`CollapsedBoundary`],
// [`SeamIdentified`], [`TransversePair`]) from the lemma outputs.
//
// **The four-outcome space (spec Theorem B1 outcome claim).** Every admitted
// patch resolves to exactly one of four verdicts ([`AdmissionCertificate`]):
// regular (the whole-span hemisphere pass), regular+seam (an exactly
// identified paired edge), regular-interior+collapsed-boundary (an exactly
// collapsed edge deflated by L4 and the quotient re-certified), or a typed
// refusal with the recorded evidence (a genuine parameter singularity — never
// silent). A patch that fits none of these four is a SPEC_GAP.
//
// **The dispatch.** Per patch the assembly tries, in order:
// 1. **Hemisphere pass ⇒ regular.** The L3 hemisphere certificate
//    (`min(bernstein coefficients of c·M) > 0` over the span) over the float
//    midpoint normal (and, failing that, the six unit-axis probes). The
//    certified cone payload is derived from the certificate direction and its
//    certified margin over the coefficient net.
// 2. **Collapse detected ⇒ deflate then re-certify the interior.** An exact
//    boundary collapse is divided out by the L4 kernel; the deflated interior
//    is re-certified by the quotient hemisphere test, and the multiplicity is
//    recorded in a [`CollapsedBoundary`].
// 3. **Seam identity ⇒ paired edges.** Two patches whose shared boundary
//    curves coincide exactly close as a [`SeamIdentified`] edge pair (an
//    intentional coincidence, not a singularity).
// 4. **Subdivision under budget; exhaustion ⇒ typed refusal.** Certificate
//    failures subdivide the span dyadically (per-cell normal-cone
//    certification) under the [`CertificateBudget`]; budget exhaustion returns
//    the [`GenuineSingularity`](AdmissionCertificate::GenuineSingularity)
//    verdict carrying the [`StallRecord`] — never silent.
//
// **FSSI-001 substrate handoff.** The normal-cone subsystem (L3) closes BOTH
// Theorem B1 regularity and Theorem C transversality. [`certify_transverse_pair`]
// composes two certified cones into the Theorem C [`TransversePair`] with the
// identical superadditive margin arithmetic the landed gate (`ssi_gate.rs`)
// applies to its per-side cones; the gate itself is untouched.
// ---------------------------------------------------------------------------

/// The default subdivision depth of a span certificate (scope decision 3).
const DEFAULT_SUBDIVISION_DEPTH: usize = 12;

/// The default number of unresolved dyadic cells allowed at one subdivision
/// level of a span certificate (scope decision 3).
const DEFAULT_SUBDIVISION_CELLS: usize = 4096;

/// The parameter midpoint of every float search in this module (the same
/// mid-square rule the L3 lemma kernel uses).
const SPAN_MIDPOINT: f64 = 0.5;

/// The six unit-axis probe directions of the hemisphere search: pure float
/// search candidates (SFC — the certificate is the interval evaluation), never
/// certified bounds.
const AXIS_SEARCH_DIRECTIONS: [[f64; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
];

/// A compact sub-rectangle `((u_lo, u_hi), (v_lo, v_hi))` of the unit square
/// `[0, 1]²` (the dyadic cells of a span's subdivision).
pub type UnitBox2 = ((f64, f64), (f64, f64));

/// The dyadic subdivision budget of one span certificate (scope decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertificateBudget {
    /// The largest subdivision depth (number of halving steps per axis) before
    /// the certificate stalls.
    pub max_depth: usize,
    /// The largest number of unresolved dyadic cells the certificate may hold
    /// at one level before it stalls.
    pub max_cells: usize,
}

impl Default for CertificateBudget {
    fn default() -> Self {
        CertificateBudget {
            max_depth: DEFAULT_SUBDIVISION_DEPTH,
            max_cells: DEFAULT_SUBDIVISION_CELLS,
        }
    }
}

/// The recorded stall of a budget-exhausted span certificate (scope decision
/// 3): certificate failures subdivide under budget; budget exhaustion is a
/// typed refusal with this record — never silent.
#[derive(Debug, Clone, PartialEq)]
pub struct StallRecord {
    /// The typed refusal of the stall.
    pub cause: ConstructRefusal,
    /// The subdivision depth reached before the budget halted the search.
    pub depth_reached: usize,
    /// The number of dyadic cells still unresolved at the halt.
    pub unresolved_cells: usize,
    /// The best certified margin observed (`margin.lo` of the hemisphere
    /// certificate at the midpoint direction), or `None` when no hemisphere
    /// certificate fired at all.
    pub best_margin: Option<f64>,
}

/// The certified regularity evidence of one extracted span (ADM-002 output).
///
/// `regular` carries the certified normal cone of the regular region: the whole
/// span when the hemisphere certificate fired, or the deflated interior (the
/// quotient `M*` of the exactly divided boundary factor) when [`CollapsedBoundary`]
/// records a deflation. A whole-box span has `collapsed == None`; a deflated
/// span records the divided multiplicity.
#[derive(Debug, Clone, PartialEq)]
pub struct SpanCertificate {
    /// The parent face (and optional boundary edge) of the certified span.
    pub parent: PatchParent,
    /// The unit-square region of the span the certificate covers (the whole
    /// span, or a dyadic cell when the certificate came from subdivision).
    pub region: UnitBox2,
    /// The certified regularity of the span.
    pub regular: RegularPatch,
    /// The deflation record when the regularity is the deflated interior.
    pub collapsed: Option<CollapsedBoundary>,
}

/// The four-outcome certificate space (Theorem B1 outcome claim) of an
/// admitted patch family.
///
/// The dispatch is EXHAUSTIVE: every fixture of the admission kit resolves to
/// exactly one of these four verdicts. A patch family that fits none of them is
/// a SPEC_GAP (the theory doc's outcome claim would be falsified).
#[derive(Debug, Clone, PartialEq)]
pub enum AdmissionCertificate {
    /// Every span certified regular on its whole box (hemisphere pass); no
    /// deflation, no seam.
    Regular {
        /// The certified regular spans of the family.
        spans: Vec<SpanCertificate>,
    },
    /// The family's spans certify regular and an exactly identified seam pairs
    /// two of the family's BRep edges.
    RegularWithSeam {
        /// The certified regular spans of the family.
        spans: Vec<SpanCertificate>,
        /// The certified seam of the family.
        seam: SeamIdentified,
    },
    /// Some span's exactly collapsed boundary was deflated (L4) and its
    /// interior re-certified regular.
    RegularInteriorCollapsedBoundary {
        /// The certified regular spans of the family (deflated spans carry
        /// their [`CollapsedBoundary`]).
        spans: Vec<SpanCertificate>,
    },
    /// A genuine parameter singularity: the span could not be certified even
    /// under the subdivision budget, refused typed with the recorded stall —
    /// never silent.
    GenuineSingularity {
        /// The never-silent stall record of the refusal.
        stall: StallRecord,
    },
}

impl AdmissionCertificate {
    /// A short stable tag of the outcome, for diagnostics.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Regular { .. } => "regular",
            Self::RegularWithSeam { .. } => "regular_with_seam",
            Self::RegularInteriorCollapsedBoundary { .. } => "regular_interior_collapsed_boundary",
            Self::GenuineSingularity { .. } => "genuine_singularity",
        }
    }
}

/// The certified two-cone angular-separation gap of Theorem C: `sin α − s_X −
/// s_Y` with `sin α` the sine of the (projective) angle between the certified
/// anchors, in the superadditive margin shape the landed gate applies. `None`
/// when the cones do not certify separation.
fn angular_separation_gap(a: &NormalCone, b: &NormalCone) -> Option<f64> {
    let sin_alpha = certified_sin_angle(a.anchor, b.anchor)?;
    let gap = Interval::point(sin_alpha)
        .sub(&Interval::point(a.s_up))
        .sub(&Interval::point(b.s_up));
    if gap.lo.is_finite() && gap.lo > 0.0 {
        Some(gap.lo)
    } else {
        None
    }
}

/// A certified lower bound of `sin ∠(a, b)` between two unit-ish float
/// directions (the anchors of two certified cones), outward-rounded.
fn certified_sin_angle(a: [f64; 3], b: [f64; 3]) -> Option<f64> {
    if a.iter().any(|c| !c.is_finite()) || b.iter().any(|c| !c.is_finite()) {
        return None;
    }
    let cross = [
        Interval::point(a[1])
            .mul(&Interval::point(b[2]))
            .sub(&Interval::point(a[2]).mul(&Interval::point(b[1]))),
        Interval::point(a[2])
            .mul(&Interval::point(b[0]))
            .sub(&Interval::point(a[0]).mul(&Interval::point(b[2]))),
        Interval::point(a[0])
            .mul(&Interval::point(b[1]))
            .sub(&Interval::point(a[1]).mul(&Interval::point(b[0]))),
    ];
    let mut cross2 = Interval::point(0.0);
    for iv in &cross {
        cross2 = cross2.add(&iv.mul(iv));
    }
    let cross_norm = cross2.sqrt()?;
    let a_norm = norm_enclosure(a)?;
    let b_norm = norm_enclosure(b)?;
    let denom = a_norm.hi * b_norm.hi;
    if !denom.is_finite() || denom <= 0.0 {
        return None;
    }
    let sin_alpha = Interval::point(cross_norm.lo).div(&Interval::point(denom))?;
    if sin_alpha.lo.is_finite() {
        Some(sin_alpha.lo)
    } else {
        None
    }
}

/// The certified enclosure `[|v|_lo, |v|_hi]` of the norm of a float vector.
fn norm_enclosure(v: [f64; 3]) -> Option<Interval> {
    let mut acc = Interval::point(0.0);
    for x in &v {
        let iv = Interval::point(*x);
        acc = acc.add(&iv.mul(&iv));
    }
    acc.sqrt()
}

/// The certified normal-cone separation certificate of a patch pair (Theorem
/// C, scope decision 2): compose two certified cones into the transversality
/// certificate of the pair's product box.
///
/// `delta = (lo, hi)` is a certified enclosure of the minimal cross-product
/// magnitude `‖n̂_X × n̂_Y‖` over the product box for the UNIT normal
/// directions of the two cones: `lo = sin α − s_X − s_Y > 0` (the certified
/// angular-separation gap, the identical superadditive margin the landed gate
/// derives from its per-side cones) and `hi = 1`. A strictly positive `lo`
/// certifies `rank DF = 3` on `Σ ∩ B` (the FSSI-001 substrate handoff — the
/// gate itself is untouched; it consumes the normal enclosures this cone data
/// certifies). Refuses [`ConstructRefusal::ConditioningBelowThreshold`] when
/// the cones do not separate.
pub fn certify_transverse_pair(
    a: &NormalCone,
    b: &NormalCone,
) -> Result<TransversePair, ConstructRefusal> {
    match angular_separation_gap(a, b) {
        Some(lo) => Ok(TransversePair { delta: (lo, 1.0) }),
        None => Err(ConstructRefusal::ConditioningBelowThreshold),
    }
}

/// The same Theorem C certificate over two admitted regular patches: their
/// certified cones are the payload.
pub fn certify_regular_pair_transverse(
    a: &RegularPatch,
    b: &RegularPatch,
) -> Result<TransversePair, ConstructRefusal> {
    certify_transverse_pair(&a.cone, &b.cone)
}

/// The whole unit square as a [`UnitBox2`].
fn whole_span_box() -> UnitBox2 {
    ((0.0, 1.0), (0.0, 1.0))
}

/// The four dyadic quadrants of a unit-square box.
fn quadrants(b: UnitBox2) -> [UnitBox2; 4] {
    let ((u_lo, u_hi), (v_lo, v_hi)) = b;
    let u_mid = 0.5 * (u_lo + u_hi);
    let v_mid = 0.5 * (v_lo + v_hi);
    [
        ((u_lo, u_mid), (v_lo, v_mid)),
        ((u_mid, u_hi), (v_lo, v_mid)),
        ((u_lo, u_mid), (v_mid, v_hi)),
        ((u_mid, u_hi), (v_mid, v_hi)),
    ]
}

/// The certified normal cone of a scalar-dotted field net: given the row-major
/// `R³` coefficient net of the field, the float anchor direction `c`, and the
/// certified lower bound `margin_lo` of `min(c·field)` over the box (the
/// certificate that every coefficient — and hence every field value — lies in
/// the open hemisphere about `c`).
///
/// The bound is the DIRECTIONAL one: a Bernstein field value is a convex
/// combination of its coefficients, so `∠(field(u, v), c) ≤ max_i ∠(coeff_i, c)`
/// over the whole box (`c·field ≥ margin_lo > 0` keeps every angle acute), and
/// `sin ∠(coeff_i, c)` is bounded outward-rounded coefficient by coefficient.
/// `s_up` is the certified maximum; `None` when it does not stay strictly below
/// `1` (no single hemisphere certifies).
fn cone_from_net(
    coeffs: &[[f64; 3]],
    degree: (usize, usize),
    anchor: [f64; 3],
    margin_lo: f64,
) -> Option<NormalCone> {
    let (du, dv) = degree;
    let width = dv + 1;
    if coeffs.is_empty()
        || coeffs.len() != (du + 1) * width
        || anchor.iter().any(|c| !c.is_finite())
        || !margin_lo.is_finite()
        || margin_lo <= 0.0
    {
        return None;
    }
    let a_norm = norm_enclosure(anchor)?;
    if !a_norm.lo.is_finite() || a_norm.lo <= 0.0 {
        return None;
    }
    let mut s_up = 0.0f64;
    for coeff in coeffs {
        let c_norm = norm_enclosure(*coeff)?;
        if !c_norm.lo.is_finite() || c_norm.lo <= 0.0 {
            return None;
        }
        let cross = [
            Interval::point(coeff[1])
                .mul(&Interval::point(anchor[2]))
                .sub(&Interval::point(coeff[2]).mul(&Interval::point(anchor[1]))),
            Interval::point(coeff[2])
                .mul(&Interval::point(anchor[0]))
                .sub(&Interval::point(coeff[0]).mul(&Interval::point(anchor[2]))),
            Interval::point(coeff[0])
                .mul(&Interval::point(anchor[1]))
                .sub(&Interval::point(coeff[1]).mul(&Interval::point(anchor[0]))),
        ];
        // A certified upper bound of |coeff × anchor|: per-axis abs-upper
        // squares (a colinear coefficient crosses to a zero-containing interval,
        // which is why the signed square is never formed).
        let mut cross2 = Interval::point(0.0);
        for iv in &cross {
            let far = iv.lo.abs().max(iv.hi.abs());
            cross2 = cross2.add(&Interval::point(far).mul(&Interval::point(far)));
        }
        let cross_hi = cross2.sqrt()?.hi;
        let denom = Interval::point(c_norm.lo).mul(&Interval::point(a_norm.lo));
        let sin_i = Interval::point(cross_hi).div(&denom)?.hi;
        if sin_i > s_up {
            s_up = sin_i;
        }
    }
    if !s_up.is_finite() || s_up >= 1.0 {
        return None;
    }
    Some(NormalCone { anchor, s_up })
}

/// The whole-box hemisphere certificate: certify regularity over the whole span
/// by the L3 hemisphere test, searching the float midpoint normal first and the
/// six unit-axis probes after, and derive the certified cone payload from the
/// certificate that fires.
fn whole_box_regular_span(
    patch: &TensorBernsteinPatch,
    m: &NormalNumerator,
) -> Option<SpanCertificate> {
    let mut directions: Vec<[f64; 3]> = Vec::new();
    if let Some(mid) = midpoint_normal_direction(patch) {
        directions.push(mid);
    }
    directions.extend_from_slice(&AXIS_SEARCH_DIRECTIONS);
    for dir in directions {
        let cert = match hemisphere_certificate(m, dir) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if !cert.certified {
            continue;
        }
        if let Some(cone) = cone_from_net(m.coeffs(), m.degree(), cert.direction, cert.margin.lo) {
            return Some(SpanCertificate {
                parent: patch.parent(),
                region: whole_span_box(),
                regular: RegularPatch { cone },
                collapsed: None,
            });
        }
    }
    None
}

/// A float search direction for a deflated quotient's interior certificate: the
/// normalized field value at the span midpoint (and, as fallbacks, its
/// negation and the six unit-axis probes).
fn quotient_candidate_directions(quotient: &DeflatedNumerator) -> Vec<[f64; 3]> {
    let mut dirs: Vec<[f64; 3]> = Vec::new();
    if let Some(center) = normalize_float(eval_net_at(
        quotient.coeffs(),
        quotient.degree(),
        SPAN_MIDPOINT,
        SPAN_MIDPOINT,
    )) {
        dirs.push(center);
        dirs.push([-center[0], -center[1], -center[2]]);
    }
    dirs.extend_from_slice(&AXIS_SEARCH_DIRECTIONS);
    dirs
}

/// The collapsed-edge deflation certificate: find the exactly collapsed
/// boundary side, divide the known factor out (L4), and certify the deflated
/// interior — producing the [`CollapsedBoundary`] record and the certified
/// interior regular patch.
fn deflated_span_certificate(patch: &TensorBernsteinPatch) -> Option<SpanCertificate> {
    const SIDES: [PatchSide; 4] = [
        PatchSide::SideUMin,
        PatchSide::SideUMax,
        PatchSide::SideVMin,
        PatchSide::SideVMax,
    ];
    for side in SIDES {
        let deflation = match deflate_factor(patch, side) {
            Ok(d) => d,
            Err(_) => continue,
        };
        if deflation.multiplicity() == 0 {
            continue;
        }
        let quotient = deflation.quotient();
        for dir in quotient_candidate_directions(quotient) {
            let margin = match certify_deflated_interior(&deflation, dir) {
                Ok(margin) => margin,
                Err(_) => continue,
            };
            if let Some(cone) = cone_from_net(quotient.coeffs(), quotient.degree(), dir, margin) {
                return Some(SpanCertificate {
                    parent: patch.parent(),
                    region: whole_span_box(),
                    regular: RegularPatch { cone },
                    collapsed: Some(CollapsedBoundary {
                        multiplicity: deflation.multiplicity(),
                    }),
                });
            }
        }
    }
    None
}

/// The subdivision certifier (scope decision 3): certify the remaining dyadic
/// cells of a span by per-cell normal cones under the budget. Every cell
/// certified returns its spans; an unresolved cell at the budget is the
/// never-silent typed refusal with the stall record.
fn subdivide_span(
    m: &NormalNumerator,
    parent: PatchParent,
    best_margin: Option<f64>,
    budget: &CertificateBudget,
) -> Result<Vec<SpanCertificate>, StallRecord> {
    let mut level: Vec<UnitBox2> = vec![whole_span_box()];
    let mut depth = 0usize;
    let mut spans: Vec<SpanCertificate> = Vec::new();
    loop {
        let mut next: Vec<UnitBox2> = Vec::new();
        for cell in level {
            match normal_cone(m, cell) {
                Some(cone) => spans.push(SpanCertificate {
                    parent,
                    region: cell,
                    regular: RegularPatch { cone },
                    collapsed: None,
                }),
                None => next.extend(quadrants(cell)),
            }
        }
        if next.is_empty() {
            return Ok(spans);
        }
        if depth + 1 >= budget.max_depth || next.len() > budget.max_cells {
            return Err(StallRecord {
                cause: ConstructRefusal::ConditioningBelowThreshold,
                depth_reached: depth + 1,
                unresolved_cells: next.len(),
                best_margin,
            });
        }
        level = next;
        depth += 1;
    }
}

/// The per-span certification: the four-outcome dispatch for one extracted
/// span, in the fixed order of scope decision 1 — hemisphere pass ⇒ regular;
/// collapse ⇒ deflate then re-certify the interior; else subdivide under
/// budget and stall typed at exhaustion.
fn certify_span(
    patch: &TensorBernsteinPatch,
    budget: &CertificateBudget,
) -> Result<Vec<SpanCertificate>, StallRecord> {
    let m = match assemble_normal_numerator(patch) {
        Ok(m) => m,
        Err(cause) => {
            return Err(StallRecord {
                cause,
                depth_reached: 0,
                unresolved_cells: 1,
                best_margin: None,
            })
        }
    };
    if let Some(span) = whole_box_regular_span(patch, &m) {
        return Ok(vec![span]);
    }
    let best_margin = match midpoint_normal_direction(patch) {
        Some(dir) => match hemisphere_certificate(&m, dir) {
            Ok(cert) => Some(cert.margin.lo),
            Err(_) => None,
        },
        None => None,
    };
    if let Some(span) = deflated_span_certificate(patch) {
        return Ok(vec![span]);
    }
    subdivide_span(&m, patch.parent(), best_margin, budget)
}

// ---------------------------------------------------------------------------
// ADM-004-FUNNEL-WIRING — the admitted-pair cut row.
//
// This section wires the landed admission pieces into the boolean boundary for
// ONE face pair of a swept-pair cut: the Theorem A adapter (ADM-001) admits
// the pair, the certificate assembly (ADM-002) certifies both faces' extracted
// span families, and the Theorem C transversality certificate (ADM-002)
// closes the pair over the shared whole-span product chart. The assembled row
// is the deterministic per-pair carrier the funnel enumeration below and the
// registered certified solver consume (the locus emission into the landed
// `ContactLocus` vocabulary rides those registered arms — never a splitter
// edit, FSSI-LAYER).
//
// **Admission consult (V5).** The entry calls the refusing-by-default adapter
// first: a pair outside the admitted class keeps the adapter's exact typed
// refusal, so nothing already-green can reach the certificates (admission
// widens monotonically or not at all).
//
// **Determinism.** Same pair → identical assembly rows → identical
// certificates and identical transverse margin: the row's `Debug` rendering is
// bit-identical on identical input (the binding's determinism rule, extended
// to admitted pairs).
// ---------------------------------------------------------------------------

/// One certified cut row of the admitted pair pipeline (ADM-004-FUNNEL-WIRING):
/// the integrated per-pair outcome of the admission consult at the boolean
/// boundary for a pair of faces of two swept carriers.
///
/// The row carries the Theorem A assembled polynomialized system ([`system`]),
/// both faces' certified span-family outcomes ([`x_certificate`] /
/// [`y_certificate`]), and the Theorem C transversality certificate
/// ([`transverse`]) — the certified `‖n̂_X × n̂_Y‖ ≥ lo > 0` margin over the
/// shared whole-span product chart. The Krawczyk operator is instantiated
/// downstream on the carried [`SsiPairSystem`] span-pair shape (the FSSI
/// solver layer), never here.
#[derive(Clone, Debug)]
pub struct CertifiedCutPair {
    /// The Theorem A polynomialized pair system of the admitted pair.
    pub system: SsiPairSystem,
    /// The certified span-family outcome of the `X` face (face ordinal 0).
    pub x_certificate: AdmissionCertificate,
    /// The certified span-family outcome of the `Y` face (face ordinal 1).
    pub y_certificate: AdmissionCertificate,
    /// The Theorem C transversality certificate over the shared whole-span
    /// product chart (a strictly positive certified margin when present).
    pub transverse: TransversePair,
}

impl CertifiedCutPair {
    /// The number of enumerated span pairs of the admitted system (the funnel
    /// enumeration consumes this carrier's span-pair shape).
    pub fn span_pair_count(&self) -> usize {
        self.system.span_pair_count()
    }

    /// The certified whole-box regularity payload of the `X` face family
    /// (the span certificate covering the whole span, when the family has
    /// one).
    pub fn x_whole_box_regular(&self) -> Option<&RegularPatch> {
        whole_box_regular_cone(&self.x_certificate)
    }

    /// The certified whole-box regularity payload of the `Y` face family
    /// (the span certificate covering the whole span, when the family has
    /// one).
    pub fn y_whole_box_regular(&self) -> Option<&RegularPatch> {
        whole_box_regular_cone(&self.y_certificate)
    }
}

/// The whole-span regularity certificate of a certified face family: the
/// FIRST certified span covering the whole unit square with no deflation
/// record (the payload whose cone closes the Theorem C transversality
/// certificate over the shared whole-span product chart). `None` when the
/// family has no such span (a subdivided or deflated-only family, or a typed
/// singularity stall).
fn whole_box_regular_cone(certificate: &AdmissionCertificate) -> Option<&RegularPatch> {
    let spans = match certificate {
        AdmissionCertificate::Regular { spans }
        | AdmissionCertificate::RegularWithSeam { spans, .. }
        | AdmissionCertificate::RegularInteriorCollapsedBoundary { spans } => spans,
        AdmissionCertificate::GenuineSingularity { .. } => return None,
    };
    spans
        .iter()
        .find(|s| s.region == whole_span_box() && s.collapsed.is_none())
        .map(|s| &s.regular)
}

/// The ADM-004 certified cut of one admitted face pair: wire the landed
/// admission pieces (adapter + certificates + transversality) into the boolean
/// boundary for the admitted pair and return the deterministic
/// [`CertifiedCutPair`] row.
///
/// **Pipeline.** (1) The Theorem A adapter ([`admit_tensor_spline_pair`])
/// admits the pair behind the refusing-by-default dispatch rule — a
/// not-yet-admitted carrier pair keeps the adapter's exact typed refusal
/// (V5). (2) The certificate assembly ([`certify_patch_family`]) certifies
/// each face's extracted span family. (3) The two faces' whole-box regular
/// cones compose into the Theorem C [`TransversePair`] via
/// [`certify_transverse_pair`]; a family without a whole-box regular span, or
/// a pair whose cones do not certify separation, refuses
/// [`ConstructRefusal::ConditioningBelowThreshold`] typed (the shared
/// whole-span product chart is not certifiably transverse).
///
/// Deterministic: identical input produces a bit-identical row.
pub fn certify_admitted_cut_pair(
    x: &BSplineSurface<Vector4>,
    y: &BSplineSurface<Vector4>,
    domain_hint: &AdmitDomainHint,
    budget: &CertificateBudget,
) -> Result<CertifiedCutPair, ConstructRefusal> {
    let system = admit_tensor_spline_pair(x, y, domain_hint)?;
    let x_certificate = certify_patch_family(system.x_spans(), budget);
    let y_certificate = certify_patch_family(system.y_spans(), budget);
    let x_regular = whole_box_regular_cone(&x_certificate)
        .ok_or(ConstructRefusal::ConditioningBelowThreshold)?;
    let y_regular = whole_box_regular_cone(&y_certificate)
        .ok_or(ConstructRefusal::ConditioningBelowThreshold)?;
    let transverse = certify_transverse_pair(&x_regular.cone, &y_regular.cone)?;
    Ok(CertifiedCutPair {
        system,
        x_certificate,
        y_certificate,
        transverse,
    })
}

/// The admission dispatch over a patch family (the extracted spans of an
/// admitting unit): certify every span, resolve the family's seam identities,
/// and return the exhaustive four-outcome verdict.
pub fn certify_patch_family(
    patches: &[TensorBernsteinPatch],
    budget: &CertificateBudget,
) -> AdmissionCertificate {
    let mut spans: Vec<SpanCertificate> = Vec::new();
    for patch in patches {
        match certify_span(patch, budget) {
            Ok(mut span) => spans.append(&mut span),
            Err(stall) => return AdmissionCertificate::GenuineSingularity { stall },
        }
    }
    // Seam identity: the first exactly identified patch pair, in fixed order.
    for i in 0..patches.len() {
        for j in (i + 1)..patches.len() {
            let (ea, eb) = match (patches[i].parent().edge, patches[j].parent().edge) {
                (Some(ea), Some(eb)) => (ea, eb),
                _ => continue,
            };
            match seam_identified(&patches[i], &patches[j]) {
                Ok(true) => {
                    return AdmissionCertificate::RegularWithSeam {
                        spans,
                        seam: SeamIdentified { paired: (ea, eb) },
                    }
                }
                _ => continue,
            }
        }
    }
    if spans.iter().any(|s| s.collapsed.is_some()) {
        AdmissionCertificate::RegularInteriorCollapsedBoundary { spans }
    } else {
        AdmissionCertificate::Regular { spans }
    }
}

/// Normalize a nonzero float vector (the SFC float search; `None` on a
/// degenerate or non-finite vector).
fn normalize_float(v: Option<[f64; 3]>) -> Option<[f64; 3]> {
    let v = v?;
    if v.iter().any(|c| !c.is_finite()) {
        return None;
    }
    let norm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if !norm.is_finite() || norm <= 0.0 {
        return None;
    }
    Some([v[0] / norm, v[1] / norm, v[2] / norm])
}

/// The plain-float tensor-Bernstein evaluation of a flat row-major `R³` net at
/// `(u, v)` (the SFC float searches of the certificate; never a certificate).
fn eval_net_at(coeffs: &[[f64; 3]], degree: (usize, usize), u: f64, v: f64) -> Option<[f64; 3]> {
    let (du, dv) = degree;
    let width = dv + 1;
    if coeffs.is_empty() || coeffs.len() != (du + 1) * width {
        return None;
    }
    let bu = bernstein_weights(du, u)?;
    let bv = bernstein_weights(dv, v)?;
    let mut acc = [0.0f64; 3];
    for i in 0..=du {
        for j in 0..=dv {
            let c = coeffs[i * width + j];
            let factor = bu[i] * bv[j];
            acc[0] += factor * c[0];
            acc[1] += factor * c[1];
            acc[2] += factor * c[2];
        }
    }
    if acc.iter().all(|c| c.is_finite()) {
        Some(acc)
    } else {
        None
    }
}

/// The degree-`degree` Bernstein basis values `Bᵢ(degree)(t)`, or `None` when
/// the evaluation point is outside `[0, 1]`.
fn bernstein_weights(degree: usize, t: f64) -> Option<Vec<f64>> {
    if !t.is_finite() || !(0.0..=1.0).contains(&t) {
        return None;
    }
    let mut out = Vec::with_capacity(degree + 1);
    for i in 0..=degree {
        let c = binomial(degree, i);
        out.push(c * t.powi(i as i32) * (1.0 - t).powi((degree - i) as i32));
    }
    Some(out)
}

/// The exact integer binomial `C(n, k)` as a float (the span degrees stay
/// small, so the product loop is exact-safe).
fn binomial(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut r = 1.0f64;
    for t in 1..=k {
        r = r * (n - k + t) as f64 / t as f64;
    }
    r
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

    /// A shallow parabolic-profile loft face
    /// `X(u, v) = (u, v, u + c·u²)` — bidegree `(2, 1)`, every extracted span
    /// linear in the loft axis `v` (the ADM-001 admitted class shape), unit
    /// weights. The normals `(−(1 + 2c·u), 0, 1)` lie in the `xz` plane near
    /// the `(−1, 0, 1)` direction with a small certified spread.
    fn shallow_loft_xz(c: f64) -> BSplineSurface<Vector4> {
        let knots = (
            KnotVec::from(vec![0.0f64, 0.0, 0.0, 1.0, 1.0, 1.0]),
            KnotVec::from(vec![0.0f64, 0.0, 1.0, 1.0]),
        );
        let u_coeff = [0.0, 0.5, 1.0];
        let profile = [0.0f64, 0.5, 1.0 + c];
        let v_coeff = [0.0, 1.0];
        let mut control_points: Vec<Vec<Vector4>> = Vec::new();
        for (&u, &z) in u_coeff.iter().zip(profile.iter()) {
            let mut row: Vec<Vector4> = Vec::new();
            for &v in &v_coeff {
                row.push(Vector4::new(u, v, z, 1.0));
            }
            control_points.push(row);
        }
        BSplineSurface::new(knots, control_points)
    }

    /// A shallow parabolic-profile loft face
    /// `Y(u, v) = (u, u + c·u², v)` — bidegree `(2, 1)`, every extracted span
    /// linear in the loft axis `v` (the ADM-001 admitted class shape), unit
    /// weights. The normals `(1 + 2c·u, −1, 0)` lie in the `xy` plane near the
    /// `(1, −1, 0)` direction with a small certified spread.
    fn shallow_loft_xy(c: f64) -> BSplineSurface<Vector4> {
        let knots = (
            KnotVec::from(vec![0.0f64, 0.0, 0.0, 1.0, 1.0, 1.0]),
            KnotVec::from(vec![0.0f64, 0.0, 1.0, 1.0]),
        );
        let u_coeff = [0.0, 0.5, 1.0];
        let profile = [0.0f64, 0.5, 1.0 + c];
        let v_coeff = [0.0, 1.0];
        let mut control_points: Vec<Vec<Vector4>> = Vec::new();
        for (&u, &y) in u_coeff.iter().zip(profile.iter()) {
            let mut row: Vec<Vector4> = Vec::new();
            for &v in &v_coeff {
                row.push(Vector4::new(u, y, v, 1.0));
            }
            control_points.push(row);
        }
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

    #[test]
    fn certify_admitted_cut_pair_wires_admission_end_to_end() {
        // ADM-004-FUNNEL-WIRING: the certified cut of one admitted face pair
        // wires the landed pieces — the Theorem A adapter admits, the ADM-002
        // certificate assembly certifies both faces, and the Theorem C
        // transversality certificate closes the shared whole-span product
        // chart. The fixture: two shallow parabolic-profile ruled-section loft
        // faces whose certified normal cones separate — `X = (u, v, u + c·u²)`
        // (normals near the `xz`-plane direction `(−1, 0, 1)`) against
        // `Y = (u, u + c·u², v)` (normals near the `xy`-plane direction
        // `(1, −1, 0)`) — so the certified cut certifies with a strictly
        // positive transverse margin.
        let x = shallow_loft_xz(0.05);
        let y = shallow_loft_xy(0.05);
        let hint = AdmitDomainHint {
            order: CullingOrder::HullAabbThenKnotSpan,
        };
        let budget = CertificateBudget::default();

        let run = || match certify_admitted_cut_pair(&x, &y, &hint, &budget) {
            Ok(row) => row,
            Err(refusal) => panic!("the admitted pair cut must certify, refused {refusal:?}"),
        };

        let first = run();
        assert_eq!(
            first.system.span_pair_count(),
            1,
            "the single-span fixture assembles one span pair"
        );
        assert_eq!(
            first.x_certificate.tag(),
            "regular",
            "the X face family certifies regular over its whole span"
        );
        assert_eq!(
            first.y_certificate.tag(),
            "regular",
            "the Y face family certifies regular over its whole span"
        );
        assert!(
            first.x_whole_box_regular().is_some() && first.y_whole_box_regular().is_some(),
            "both faces carry a whole-box regularity certificate"
        );
        assert!(
            first.transverse.delta.0 > 0.0 && first.transverse.delta.0 < first.transverse.delta.1,
            "the certified cut closes with a strictly positive transverse margin: {:?}",
            first.transverse.delta
        );

        // Determinism: an identical second cut is bit-identical (the binding's
        // determinism rule, extended to the admitted pair pipeline).
        let second = run();
        assert_eq!(
            format!("{first:?}"),
            format!("{second:?}"),
            "an identical admitted pair cut answers bit-identically"
        );
    }

    #[test]
    fn certify_admitted_cut_pair_keeps_admission_refusals_unchanged() {
        // A general spline-section pair (NOT the admitted ruled-section class)
        // keeps the adapter's exact typed refusal through the cut entry (V5:
        // nothing already-green reaches the certificates, admission widens
        // monotonically or not at all).
        let x = bezier_patch(0.0);
        let y = bezier_patch(3.0);
        let hint = AdmitDomainHint {
            order: CullingOrder::HullAabbThenKnotSpan,
        };
        let budget = CertificateBudget::default();
        let run = || certify_admitted_cut_pair(&x, &y, &hint, &budget);
        for attempt in ["first", "identical second"] {
            match run() {
                Err(ConstructRefusal::InvalidInput) => {}
                other => panic!(
                    "{attempt} run: a non-admitted carrier pair must keep its typed \
                     InvalidInput refusal (V5, no verdict flip), got {other:?}"
                ),
            }
        }
    }
}
