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

//! The extracted-patch type freeze (ADM-SHIM, extends ADM-000-CONTRACT): the
//! layer-2 representation the admission lemma wave parallelizes against.
//!
//! ADM-000-CONTRACT froze the Theorem A adapter over the landed homogeneous
//! carrier `BSplineSurface<Vector4>`
//! ([`admit_tensor_spline_pair`](crate::construct::admission::admit_tensor_spline_pair))
//! and the theorem A–D carrier names, all refusing by default. This shim lands
//! the ONE representation that later lemma packets consume and produce:
//!
//! 1. **`TensorBernsteinPatch`** — the extracted per-knot-rectangle patch: the
//!    tensor-Bernstein numerator coefficients `Â` over the span (bidegrees
//!    recorded), the weight patch `Ŵ` with its **certified positive bracket**
//!    `[w₋, w₊]` (the Bernstein hull of the weight coefficients), the span's
//!    source-domain box, and the parent face/edge ids. Its refusing
//!    constructor certifies the bracket by the Bernstein convex-hull property
//!    and refuses [`ConstructRefusal::InvalidInput`] on any patch whose bracket
//!    cannot be certified strictly positive — never a silent non-positive
//!    weight.
//! 2. **The five lemma-kernel signatures**, declared REFUSING (D-shim, spine
//!    decision C7): every body lands in its owning lemma packet, each behind
//!    its admitting test. Here every body returns
//!    [`ConstructRefusal::Unfrozen`]:
//!    - `extract_patches` (L1): face → per-knot-span patches.
//!    - `patch_product` (L2): exact degree-grown patch product.
//!    - `normal_numerator` (L3): the polynomial normal numerator
//!      `M = W(A_u×A_v) − W_v(A_u×A) − W_u(A×A_v)` ([`MPolynomial`]).
//!    - `deflate_factor` / `seam_identified` (L4): the collapsed-boundary
//!      deflation and the seam-identification verdicts.
//!    - `certified_reciprocal_power` (Theorem D, L5): declared here ONLY as
//!      the truck-evidence `num/` signature type alias
//!      [`CertifiedReciprocalPower`]; the body is L5's, in truck-evidence (the
//!      F1 layer is respected — no evidence-layer body lands in this crate).
//! 3. **What does NOT land here.** Any kernel body, any corpus contact, and any
//!    boolean-boundary change. The adapters keep consuming the landed ADM-000
//!    carriers; this module EXTENDS, never edits,
//!    [`admission`](crate::construct::admission)'s landed types.
//!
//! **Representation contract (spec Theorem T3′).** On each knot rectangle of a
//! positive-weight tensor-spline face, Bézier extraction is an exact local
//! change of basis; the extracted span is re-parameterized to `[0, 1]²`, so a
//! patch writes `X = Â/Ŵ` with `Â` an `(m+1)×(n+1)` grid of `R³` coefficients
//! and `Ŵ` the same-shape scalar weight grid, both over the tensor-Bernstein
//! basis of the recorded bidegree `(m, n)`. The certified bracket is the
//! Bernstein hull of the weight coefficients — the axis hull `[min w_ij,
//! max w_ij]` — which encloses `Ŵ` over the whole span by the convex-hull
//! property of the (non-negative, partition-of-unity) Bernstein basis. A
//! bracket with `w₋ > 0` certifies `Ŵ` strictly positive over the span, so the
//! clearing `F = Ŵ_Y·Â − Ŵ_X·B̂` is exact (Theorem A). Knot insertion is a
//! convex operation, so an admissible face's strictly positive weight net
//! extracts to strictly positive span nets: every admissible face is
//! representable (no representation gap).
//!
//! **H-1.** This module carries `#![deny(clippy::unwrap_used)]`, no `unwrap`,
//! no `expect`, and no `panic!`, and adds no module-level `allow`.
//!
//! **H-3.** No bare absolute literals: every certified bound in the shipped
//! surface is named and documented.

use crate::construct::admission::EdgeId;
use crate::construct::refusal::ConstructRefusal;
use crate::construct::Interval;
use crate::kernel::patch::IBox2;
use truck_evidence::contact::implicit2d::ScalarNet2;
use truck_geometry::prelude::BSplineSurface;
use truck_geometry::prelude::Vector4;

/// The parent identity of an extracted patch: which face it came from and, for
/// a patch whose span meets a boundary edge of that face, which BRep edge.
///
/// `face` is the ordinal of the parent face within the processed face family
/// (the Theorem A pair `X` first, `Y` second; ADM-001's extraction owns the
/// concrete numbering). `edge` is present exactly when the span touches one of
/// the face's boundary edges — the L4 seam/deflation kernels key on it to know
/// which boundary a collapsed or identified edge belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatchParent {
    /// The ordinal of the parent face in the processed face family.
    pub face: usize,
    /// The boundary BRep edge this span meets, when it meets one.
    pub edge: Option<EdgeId>,
}

impl PatchParent {
    /// Builds a parent record from the face ordinal and the optional boundary
    /// edge.
    pub const fn new(face: usize, edge: Option<EdgeId>) -> Self {
        PatchParent { face, edge }
    }
}

/// The extracted tensor-Bernstein patch of one knot rectangle of a landed
/// tensor-spline face (ADM-SHIM): `X(u, v) = Â(u, v) / Ŵ(u, v)` over the
/// span's re-parameterized unit square `[0, 1]²`.
///
/// `numerator` holds the `(m+1)×(n+1)` grid of `R³` coefficients `Â` in
/// row-major order (rows over `u`, columns over `v`), `weights` holds the
/// same-shape scalar grid `Ŵ`, and `bracket` is the certified Bernstein hull
/// `[w₋, w₊]` of the weight coefficients with `w₋ > 0` — the certificate,
/// by the convex-hull property, that the weight field is strictly positive
/// over the whole span. `domain` is the span's source-domain box in the parent
/// face's parameter coordinates (the exact knot-rectangle interval per axis),
/// and `parent` names the face and, for boundary spans, the BRep edge.
///
/// The type is sealed behind the refusing constructor
/// [`TensorBernsteinPatch::try_new`]: a value whose weight bracket cannot be
/// certified positive never exists, so no later kernel ever sees a silent
/// non-positive weight.
#[derive(Debug, Clone, PartialEq)]
pub struct TensorBernsteinPatch {
    /// The tensor-Bernstein bidegree `(m, n)` of the patch.
    degree: (usize, usize),
    /// The `(m+1)×(n+1)` numerator grid `Â` (rows over `u`, columns over `v`).
    numerator: Vec<Vec<[f64; 3]>>,
    /// The `(m+1)×(n+1)` weight grid `Ŵ`, same shape as `numerator`.
    weights: Vec<Vec<f64>>,
    /// The certified positive bracket `[w₋, w₊]` of the weight coefficients.
    bracket: Interval,
    /// The span's source-domain box in the parent face's parameter plane.
    domain: IBox2,
    /// The parent face (and boundary edge) identity.
    parent: PatchParent,
}

impl TensorBernsteinPatch {
    /// The refusing constructor: certify the patch's representation or refuse.
    ///
    /// The weight grid must be a finite, rectangular, non-empty `(m+1)×(n+1)`
    /// grid and the numerator grid must have the identical shape with finite
    /// `R³` entries. The weight bracket is the Bernstein hull of the weight
    /// coefficients — the axis hull `[min w_ij, max w_ij]`, which encloses the
    /// weight field over the whole span by the convex-hull property of the
    /// non-negative, partition-of-unity Bernstein basis. A bracket with a
    /// strictly positive lower bound certifies `Ŵ > 0` over the whole span.
    /// A patch whose bracket cannot be certified positive — a weight grid with
    /// any non-positive coefficient — refuses
    /// [`ConstructRefusal::InvalidInput`], as does any malformed input. `domain`
    /// must carry finite, ordered axis bounds. The bidegree is read off the
    /// grid shape, never taken from the caller.
    pub fn try_new(
        numerator: Vec<Vec<[f64; 3]>>,
        weights: Vec<Vec<f64>>,
        domain: IBox2,
        parent: PatchParent,
    ) -> Result<Self, ConstructRefusal> {
        if !domain.lo.iter().all(|b| b.is_finite())
            || !domain.hi.iter().all(|b| b.is_finite())
            || domain
                .lo
                .iter()
                .zip(domain.hi.iter())
                .any(|(lo, hi)| lo > hi)
        {
            return Err(ConstructRefusal::InvalidInput);
        }
        if numerator.is_empty() || weights.is_empty() {
            return Err(ConstructRefusal::InvalidInput);
        }
        let rows = weights.len();
        let cols = weights[0].len();
        if cols == 0 || numerator.len() != rows {
            return Err(ConstructRefusal::InvalidInput);
        }
        for row in 0..rows {
            if weights[row].len() != cols
                || numerator[row].len() != cols
                || weights[row].iter().any(|w| !w.is_finite())
                || numerator[row]
                    .iter()
                    .any(|a| a.iter().any(|c| !c.is_finite()))
            {
                return Err(ConstructRefusal::InvalidInput);
            }
        }
        // The certified Bernstein hull: the exact axis hull of the weight
        // coefficients (a finite reduction in the fixed row-major order, C9).
        // No interval arithmetic is spent — the convex-hull property makes the
        // coefficient hull a certified enclosure of the field over the span,
        // and it is the tightest such certificate at this level.
        let mut w_lo = f64::INFINITY;
        let mut w_hi = f64::NEG_INFINITY;
        for row in &weights {
            for &w in row {
                if w < w_lo {
                    w_lo = w;
                }
                if w > w_hi {
                    w_hi = w;
                }
            }
        }
        let bracket = Interval { lo: w_lo, hi: w_hi };
        if bracket.lo <= 0.0 {
            return Err(ConstructRefusal::InvalidInput);
        }
        Ok(TensorBernsteinPatch {
            degree: (rows - 1, cols - 1),
            numerator,
            weights,
            bracket,
            domain,
            parent,
        })
    }

    /// The tensor-Bernstein bidegree `(m, n)` of the patch.
    pub fn degree(&self) -> (usize, usize) {
        self.degree
    }

    /// The `(m+1)×(n+1)` numerator grid `Â` (rows over `u`, columns over `v`).
    pub fn numerator(&self) -> &[Vec<[f64; 3]>] {
        &self.numerator
    }

    /// The `(m+1)×(n+1)` weight grid `Ŵ` (same shape as [`numerator`](Self::numerator)).
    pub fn weights(&self) -> &[Vec<f64>] {
        &self.weights
    }

    /// The certified positive bracket `[w₋, w₊]` of the weight coefficients.
    pub fn weight_bracket(&self) -> Interval {
        self.bracket
    }

    /// The span's source-domain box in the parent face's parameter plane.
    pub fn domain(&self) -> IBox2 {
        self.domain
    }

    /// The parent face (and optional boundary edge) identity.
    pub fn parent(&self) -> PatchParent {
        self.parent
    }
}

/// A side of a patch's rectangular parameter domain, in the fixed edge
/// enumeration the L4 boundary kernels speak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchSide {
    /// The `u = 0` edge.
    SideUMin,
    /// The `u = 1` edge.
    SideUMax,
    /// The `v = 0` edge.
    SideVMin,
    /// The `v = 1` edge.
    SideVMax,
}

impl PatchSide {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::SideUMin => "side_u_min",
            Self::SideUMax => "side_u_max",
            Self::SideVMin => "side_v_min",
            Self::SideVMax => "side_v_max",
        }
    }
}

/// The R³-valued polynomial normal numerator `M` of a rational patch
/// (Theorem B1 substrate, L3).
///
/// `M = W(A_u×A_v) − W_v(A_u×A) − W_u(A×A_v)` satisfies `X_u×X_v = M/W³`, so
/// regularity of the rational patch is a POLYNOMIAL question about `M`. The
/// assembly data is a single tensor-Bernstein polynomial of bidegree `degree`
/// over the patch's unit square: `coeffs` holds the `(du+1)×(dv+1)` grid of
/// `R³` coefficients in row-major order (rows over `u`, columns over `v`), the
/// same basis convention as [`TensorBernsteinPatch`], so the Bernstein
/// coefficients of the scalar `c·M` are the component-wise dots of `c` with
/// the grid — exactly the data the Theorem B1 hemisphere certificate needs,
/// with no basis conversion.
///
/// Production (the assembled value) belongs to the L3 lemma packet;
/// [`normal_numerator`] is the refusing signature here.
#[derive(Debug, Clone, PartialEq)]
pub struct MPolynomial {
    /// The tensor-Bernstein bidegree `(du, dv)` of the assembly data.
    degree: (usize, usize),
    /// The row-major `(du+1)×(dv+1)` coefficient grid of `M`.
    coeffs: Vec<[f64; 3]>,
}

impl MPolynomial {
    /// The tensor-Bernstein bidegree of the assembly data.
    pub fn degree(&self) -> (usize, usize) {
        self.degree
    }

    /// The row-major coefficient grid of `M` (`width = dv + 1`).
    pub fn coeffs(&self) -> &[[f64; 3]] {
        &self.coeffs
    }
}

/// The collapsed-boundary deflation certificate data (Theorem B1, L4).
///
/// A boundary stratum intentionally collapsed to a point gives a known factor
/// `(1−t)^k` in the normal numerator `M`; `deflate_factor` divides it out. The
/// result records the multiplicity `k` divided out and the quotient `M*` that
/// the hemisphere certificate then certifies over the deflated boundary.
/// Production belongs to the L4 lemma packet.
#[derive(Debug, Clone, PartialEq)]
pub struct DeflatedFactor {
    /// The multiplicity of the divided-out boundary factor.
    pub multiplicity: usize,
    /// The quotient normal numerator after the deflation.
    pub quotient: MPolynomial,
}

/// The L1 kernel (refusing in ADM-SHIM): extract the per-knot-span
/// tensor-Bernstein patches of a positive-weight homogeneous tensor-spline
/// face.
///
/// The face is the landed homogeneous carrier of the Theorem A adapter
/// (`BSplineSurface<Vector4>`). Each knot rectangle is Bézier-extracted
/// (an exact local change of basis), re-parameterized to `[0, 1]²`, and
/// returned as a [`TensorBernsteinPatch`] carrying its source-domain box and
/// parent identity. The body and its admitting test land in the L1 lemma
/// packet; here the kernel refuses [`ConstructRefusal::Unfrozen`].
pub fn extract_patches(
    _face: &BSplineSurface<Vector4>,
) -> Result<Vec<TensorBernsteinPatch>, ConstructRefusal> {
    Err(ConstructRefusal::Unfrozen)
}

/// The L2 kernel (refusing in ADM-SHIM): the exact degree-grown product of two
/// extracted tensor-Bernstein patches.
///
/// The product is the exact multiplication of the two patches' numerator and
/// weight fields on their aligned unit squares, recorded at the grown bidegree
/// (never a sampled or truncated product). The body and its admitting test land
/// in the L2 lemma packet; here the kernel refuses
/// [`ConstructRefusal::Unfrozen`].
pub fn patch_product(
    _a: &TensorBernsteinPatch,
    _b: &TensorBernsteinPatch,
) -> Result<TensorBernsteinPatch, ConstructRefusal> {
    Err(ConstructRefusal::Unfrozen)
}

/// The L3 kernel (refusing in ADM-SHIM): the polynomial normal numerator
/// `M = W(A_u×A_v) − W_v(A_u×A) − W_u(A×A_v)` of a rational patch.
///
/// The returned [`MPolynomial`] is the verified assembly data for
/// `X_u×X_v = M/W³`. The body and its admitting test land in the L3 lemma
/// packet; here the kernel refuses [`ConstructRefusal::Unfrozen`].
pub fn normal_numerator(_patch: &TensorBernsteinPatch) -> Result<MPolynomial, ConstructRefusal> {
    Err(ConstructRefusal::Unfrozen)
}

/// The L4 kernel (refusing in ADM-SHIM): divide the known collapsed-boundary
/// factor out of a patch's normal numerator.
///
/// For a boundary side collapsed to a point the normal numerator carries the
/// known factor `(1−t)^k`; the kernel divides it out and certifies the
/// quotient over the deflated side (repeated for higher-order collapses). The
/// body and its admitting test land in the L4 lemma packet; here the kernel
/// refuses [`ConstructRefusal::Unfrozen`].
pub fn deflate_factor(
    _patch: &TensorBernsteinPatch,
    _boundary: PatchSide,
) -> Result<DeflatedFactor, ConstructRefusal> {
    Err(ConstructRefusal::Unfrozen)
}

/// The L4 kernel (refusing in ADM-SHIM): the seam-identification verdict over
/// an aligned patch pair.
///
/// Two patches whose shared boundary coincides exactly (`A_a·W_b − A_b·W_a ≡ 0`
/// over aligned degrees/parameterizations) are seam-identified: the shared
/// boundary is a certified pair of edges, not a singularity. The body and its
/// admitting test land in the L4 lemma packet; here the kernel refuses
/// [`ConstructRefusal::Unfrozen`].
pub fn seam_identified(
    _patch_a: &TensorBernsteinPatch,
    _patch_b: &TensorBernsteinPatch,
) -> Result<bool, ConstructRefusal> {
    Err(ConstructRefusal::Unfrozen)
}

/// The Theorem D certified reciprocal-power signature (the truck-evidence
/// `num/` signature L5 lands against — type alias only, F1 layer respected).
///
/// `certified_reciprocal_power(w, p, target_error)` truncates the
/// reciprocal-power polynomialization of the weight polynomial `w` (a scalar
/// bivariate Bernstein net over the unit square) after `m` terms, returning
/// the truncated polynomial `Q_m` and the certified uniform remainder bound
/// `ε_m` with `|W⁻ᵖ − Q_m| ≤ ε_m` over the unit square. The BODY is L5's, in
/// `truck-evidence/src/num/`; this crate only freezes the signature shape so
/// the lemma wave types against it, and the type is chosen from the evidence
/// layer's own nameable types so L5 can implement it without a truck-certified
/// dependency.
pub type CertifiedReciprocalPower = fn(
    w: &ScalarNet2,
    p: u32,
    target_error: f64,
) -> Result<(ScalarNet2, f64), truck_base::evidence::Refusal>;
