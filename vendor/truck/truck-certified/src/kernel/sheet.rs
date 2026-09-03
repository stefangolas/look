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

//! The §11 exact-overlap sheet classifier (BG-KV2-403-S6): `SheetCert` for
//! real over the two recognized pairing classes — the same recognized rational
//! carrier with its closed-form ψ (plane/plane, cylinder/coaxial,
//! sphere/concentric), or two Bézier leaves with a certified affine map.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **N4.** No transcendental call appears in this module: no `sin`, `cos`,
//! `atan2`, `exp`, `ln`, `log`, `powf`, or `sqrt` anywhere on any path. The
//! no-transcendental source-scan test pins this.
//!
//! **The §11 conditions (verbatim).** A Sheet is legal on a box `D` iff
//!
//! 1. a certified ψ : D → D̃₂ exists — the recognized-carrier closed form
//!    (identity over the shared chart) or a certified affine map with float
//!    coefficients, transported with the §4.2 Rule B outward rounding;
//! 2. `S₁(u,v) = S₂(ψ(u,v))` by certified representational equality — the
//!    interval enclosure of the difference degenerates to the
//!    representable-zero class, never a tolerance comparison;
//! 3. `n₁ · (n₂ ∘ ψ)` is certified of constant sign (the sign is an output,
//!    [`CertifiedSign`]);
//! 4. `det Dψ` is certified nonzero.
//!
//! **Condition (2) over the [`CertifiedPatch`] seam.** The two patches are
//! consumed through the frozen [`CertifiedPatch`] trait (level C1 — overlap
//! needs no C2). The trait exposes no coefficient net, so the module certifies
//! representational equality exactly, never by a tolerance:
//!
//! * *Certified equal* — the two certified enclosures are **bit-identical**
//!   on the sheet box and on every certified dyadic sub-box of the refinement
//!   ([`representational_equality`]'s `Equal` arm). For the admitted class —
//!   the same carrier or the same leaf enumerated twice — the `CertifiedPatch`
//!   evaluation is a deterministic function of the representation, so exact
//!   oracle agreement over the refinement *is* the representational equality
//!   of the two surfaces over `D`.
//! * *Refuted* — certified separation at any certified grid vertex (the
//!   difference provably cannot be a representable zero) disproves the exact
//!   sheet: this is `Refuse(NearOverlap)`, backed `Disproven` of ExactSheet
//!   (§11, §17).
//! * *Inconclusive* — anything else is left undecided. A pair that merely
//!   comes close at certified resolution is **never admitted**: there is no
//!   tolerance-tagged sheet in this module (§21's deferred `ToleranceSheet`
//!   is enforced absent — `tolerance_sheet_is_not_admitted`).
//!
//! **Shape decision (recorded).** The packet freezes the real map type as
//! [`PsiMap { kind, coeffs, offset }`](PsiMap) with affine data in the six
//! floats and a refusing constructor. `PsiMapKind::Identity` and
//! `PsiMapKind::Affine` are constructible; `PsiMapKind::Bilinear` is refused —
//! a genuinely bilinear map adds a `u·v` cross term, two further coefficients
//! the frozen `{coeffs, offset}` shape cannot carry, so no certified bilinear
//! leaf map is constructible this wave (the affine class is exact in the
//! shape). `PsiMapKind::RecognizedCarrier` is refused too: a recognized
//! carrier's closed-form correspondence is the exact Identity/Affine map over
//! the shared chart, not a fourth coefficient-carrying kind. `SheetCert`
//! (frozen in [`certs`](crate::kernel::certs)) records `domain`, `psi_kind`,
//! and the shim-certified `det_dpsi`; the real `Sheet` carrying the full
//! `PsiMap` and boundary arcs is Wave-5 assembly's job.

use crate::kernel::certs::{PsiMapKind, SheetCert};
use crate::kernel::evidence::{ClaimVerdict, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::patch::{CertifiedPatch, IBox2, IBox3, Reason};
use crate::kernel::{Interval, SignCert};

/// The refinement depth of the certified representational-equality predicate:
/// the sheet box plus the dyadic sub-boxes to depth [`REFINE_DEPTH`] are all
/// certified exactly.
const REFINE_DEPTH: u32 = 2;

/// The exact-identity coefficient matrix of an Identity map.
const IDENTITY_COEFFS: [[f64; 2]; 2] = [[1.0, 0.0], [0.0, 1.0]];
/// The exact-identity offset of an Identity map.
const ZERO_OFFSET: [f64; 2] = [0.0, 0.0];

/// The named reason when a certified evaluation leaves the patch's certified
/// region (the `CertifiedPatch` markers are non-finite there).
const REASON_OUT_OF_CERTIFIED_DOMAIN: Reason = "sheet_out_of_certified_patch_domain";
/// The named reason when condition (2) is neither certified nor refuted.
const REASON_EQUALITY_INCONCLUSIVE: Reason = "sheet_representational_equality_not_certified";
/// The named reason when condition (3)'s normal dot cannot be certified of
/// constant sign over the box.
const REASON_SIGN_NOT_CONSTANT: Reason = "sheet_normal_dot_not_constant_sign";

/// The real parameter map of the §11 sheet claim (spec §16 `psi: PsiMap`),
/// frozen in the shim as [`PsiMapKind`] and given coefficient data by this
/// packet.
///
/// The map is `ψ(u,v) = offset + coeffs·(u,v)`, i.e. affine in the two chart
/// parameters with float coefficients; the six stored floats carry the affine
/// map exactly (the recorded shape decision — a bilinear `u·v` twist is not
/// representable in this shape and is refused at construction).
///
/// Construct only through [`PsiMap::try_new`], which refuses non-finite data,
/// an `Identity` kind whose data is not exactly the identity, an affine map
/// with exactly-zero determinant, and the [`PsiMapKind::Bilinear`] /
/// [`PsiMapKind::RecognizedCarrier`] tags (recorded shape decision).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PsiMap {
    /// Which map family this certified map belongs to.
    kind: PsiMapKind,
    /// The affine coefficient matrix: output row `i` is
    /// `offset[i] + Σ_j coeffs[i][j]·input[j]`.
    coeffs: [[f64; 2]; 2],
    /// The affine offset (the map value at `(0, 0)`).
    offset: [f64; 2],
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl PsiMap {
    /// Build a certified parameter map, refusing non-finite data, an Identity
    /// kind whose coefficients are not exactly the identity, an affine map
    /// with exactly-zero determinant, and the Bilinear / RecognizedCarrier
    /// tags (the frozen shape cannot carry a bilinear `u·v` cross term, and a
    /// recognized carrier's correspondence is the Identity/Affine closed form
    /// over its chart — both recorded shape decisions).
    pub fn try_new(
        kind: PsiMapKind,
        coeffs: [[f64; 2]; 2],
        offset: [f64; 2],
    ) -> Result<Self, Refusal> {
        if !coeffs.iter().flatten().all(|c| c.is_finite()) || !offset.iter().all(|c| c.is_finite())
        {
            return Err(refusal(
                RefusalKind::NonFinite,
                "psi_map_data_not_finite",
                "PsiMap coefficients and offset must be finite".to_string(),
            ));
        }
        match kind {
            PsiMapKind::Identity => {
                if coeffs != IDENTITY_COEFFS || offset != ZERO_OFFSET {
                    return Err(refusal(
                        RefusalKind::ClaimRefuted,
                        "psi_identity_coefficients_mismatch",
                        format!(
                            "an Identity PsiMap must carry the exact identity data; \
                             got coeffs {coeffs:?} offset {offset:?}"
                        ),
                    ));
                }
            }
            PsiMapKind::Affine => {
                let det = affine_det(coeffs);
                if det == 0.0 {
                    return Err(refusal(
                        RefusalKind::WeightDegenerate,
                        "psi_affine_det_zero",
                        format!(
                            "an affine PsiMap needs a certified-nonzero determinant; \
                             det(coeffs) = {det}"
                        ),
                    ));
                }
            }
            PsiMapKind::Bilinear => {
                return Err(refusal(
                    RefusalKind::ClaimRefuted,
                    "psi_bilinear_uv_twist_not_representable",
                    "the frozen PsiMap {coeffs, offset} shape cannot carry the u*v \
                     cross-term coefficients a bilinear map adds (recorded shape decision); \
                     the affine class is exact in this shape"
                        .to_string(),
                ));
            }
            PsiMapKind::RecognizedCarrier => {
                return Err(refusal(
                    RefusalKind::ClaimRefuted,
                    "psi_recognized_carrier_is_closed_form",
                    "a recognized carrier's closed-form correspondence is the exact \
                     Identity/Affine map over the shared chart; the RecognizedCarrier tag \
                     carries no map data (recorded shape decision)"
                        .to_string(),
                ));
            }
        }
        Ok(Self {
            kind,
            coeffs,
            offset,
        })
    }

    /// The certified identity map (recognized-carrier closed form over a
    /// shared chart, and the identity leaf correspondence).
    pub fn identity() -> Result<Self, Refusal> {
        Self::try_new(PsiMapKind::Identity, IDENTITY_COEFFS, ZERO_OFFSET)
    }

    /// The map family.
    pub fn kind(self) -> PsiMapKind {
        self.kind
    }

    /// The affine coefficient matrix.
    pub fn coeffs(self) -> [[f64; 2]; 2] {
        self.coeffs
    }

    /// The affine offset.
    pub fn offset(self) -> [f64; 2] {
        self.offset
    }

    /// The determinant of `Dψ` — constant for the affine map the shape
    /// carries. Condition (4) certifies it nonzero through the shim's
    /// [`SheetCert::try_new`].
    pub fn det_value(self) -> f64 {
        affine_det(self.coeffs)
    }

    /// The certified image of the box `d` under the map (§4.2 Rule B's
    /// outward-rounded transport). The exact identity map transports a box to
    /// itself with no arithmetic, so it is returned unchanged; every genuinely
    /// nontrivial affine image is pushed one ULP outward per bound so the
    /// transported box provably contains the exact image.
    pub fn image_box(self, d: IBox2) -> IBox2 {
        if self.coeffs == IDENTITY_COEFFS && self.offset == ZERO_OFFSET {
            return d;
        }
        let mut out_lo = [0.0f64; 2];
        let mut out_hi = [0.0f64; 2];
        for row in 0..2 {
            let mut lo_acc = 0.0f64;
            let mut hi_acc = 0.0f64;
            for col in 0..2 {
                let c = self.coeffs[row][col];
                let edge_lo = c * d.lo[col];
                let edge_hi = c * d.hi[col];
                if c >= 0.0 {
                    lo_acc += edge_lo;
                    hi_acc += edge_hi;
                } else {
                    lo_acc += edge_hi;
                    hi_acc += edge_lo;
                }
            }
            out_lo[row] = (lo_acc + self.offset[row]).next_down();
            out_hi[row] = (hi_acc + self.offset[row]).next_up();
        }
        IBox2 {
            lo: out_lo,
            hi: out_hi,
        }
    }
}

/// The verdict of condition (2), the certified representational equality over
/// the sheet box (see the module docs for the exact predicates).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SheetEquality {
    /// The certified enclosures agree bit-for-bit on the box and its certified
    /// refinement: representational equality is certified.
    Equal,
    /// Certified separation at a certified grid vertex refutes the exact
    /// sheet: the difference cannot be a representable zero.
    Refuted,
    /// Neither certified nor refuted at the certified resolution.
    Inconclusive,
}

/// Condition (2): certify `S₁(u,v) = S₂(ψ(u,v))` on `domain` by certified
/// representational equality.
///
/// Equality is decided over the certified refinement of `domain` (the box and
/// its dyadic sub-boxes to depth [`REFINE_DEPTH`]). Refutation runs first over
/// the certified grid vertices: two certified enclosures that are disjoint in
/// any coordinate at a vertex certify that the surfaces differ there, so the
/// exact-sheet claim is refuted (`Refuse(NearOverlap)`, Disproven of
/// ExactSheet). No tolerance participates anywhere on this path.
fn representational_equality(
    p1: &dyn CertifiedPatch,
    p2: &dyn CertifiedPatch,
    domain: IBox2,
    psi: PsiMap,
) -> SheetEquality {
    let edges = grid_edges(domain);
    for &u in &edges[0] {
        for &v in &edges[1] {
            let point = IBox2 {
                lo: [u, v],
                hi: [u, v],
            };
            let e1 = p1.enclose(point);
            let e2 = p2.enclose(psi.image_box(point));
            if !box3_finite(&e1) || !box3_finite(&e2) {
                return SheetEquality::Inconclusive;
            }
            if separated(&e1, &e2) {
                return SheetEquality::Refuted;
            }
        }
    }
    for d in refine_boxes(domain) {
        let e1 = p1.enclose(d);
        let e2 = p2.enclose(psi.image_box(d));
        if !box3_finite(&e1) || !box3_finite(&e2) {
            return SheetEquality::Inconclusive;
        }
        if e1.lo != e2.lo || e1.hi != e2.hi {
            return SheetEquality::Inconclusive;
        }
    }
    SheetEquality::Equal
}

/// Condition (3): certify `n₁ · (n₂ ∘ ψ)` of constant sign over the box.
///
/// `n₁ = S¹_u × S¹_v` is enclosed over `domain`; `n₂` is enclosed over the
/// certified image `ψ(domain)`. The interval dot product over the box with a
/// certified sign margin (lower bound strictly positive, or upper bound
/// strictly negative) is the certified constant-sign output; anything else is
/// Inconclusive — the sign is never guessed.
pub fn normal_dot_sign(
    p1: &dyn CertifiedPatch,
    p2: &dyn CertifiedPatch,
    domain: IBox2,
    psi: PsiMap,
) -> ClaimVerdict<SignCert, Refusal, Reason> {
    let image = psi.image_box(domain);
    let d1 = p1.derivs(domain);
    let d2 = p2.derivs(image);
    if !box3_finite(&d1.su) || !box3_finite(&d1.sv) || !box3_finite(&d2.su) || !box3_finite(&d2.sv)
    {
        return ClaimVerdict::Inconclusive(REASON_OUT_OF_CERTIFIED_DOMAIN);
    }
    let n1 = cross3(&d1.su, &d1.sv);
    let n2 = cross3(&d2.su, &d2.sv);
    let dot = dot3(&n1, &n2);
    if dot.lo > 0.0 {
        ClaimVerdict::Proven(SignCert::Positive)
    } else if dot.hi < 0.0 {
        ClaimVerdict::Proven(SignCert::Negative)
    } else {
        ClaimVerdict::Inconclusive(REASON_SIGN_NOT_CONSTANT)
    }
}

/// The §11 exact-sheet claim over the two certified patches and the certified
/// map `psi` (spec section 11 VERBATIM conditions 1–4).
///
/// Returns `Proven(SheetCert)` when all four conditions certify over `domain`,
/// `Disproven(Refusal)` when the exact sheet is refuted (the certified
/// `NearOverlap` disproof), and `Inconclusive(Reason)` when the sheet is
/// neither proven nor refuted at the certified resolution. A tolerance-tagged
/// sheet is never constructed anywhere on this path.
#[allow(clippy::result_large_err)]
pub fn exact_sheet(
    p1: &dyn CertifiedPatch,
    p2: &dyn CertifiedPatch,
    domain: IBox2,
    psi: PsiMap,
) -> ClaimVerdict<SheetCert, Refusal, Reason> {
    // Condition (1): the certified map exists and transports the domain into
    // p2's certified region (a map whose image leaves the certified patch
    // domain cannot certify a sheet there).
    let image = psi.image_box(domain);
    if !box3_finite(&p1.enclose(domain)) || !box3_finite(&p2.enclose(image)) {
        return ClaimVerdict::Inconclusive(REASON_OUT_OF_CERTIFIED_DOMAIN);
    }
    // Condition (2): certified representational equality.
    match representational_equality(p1, p2, domain, psi) {
        SheetEquality::Equal => {}
        SheetEquality::Refuted => return ClaimVerdict::Disproven(near_overlap()),
        SheetEquality::Inconclusive => {
            return ClaimVerdict::Inconclusive(REASON_EQUALITY_INCONCLUSIVE)
        }
    }
    // Condition (3): the normal dot is certified of constant sign.
    match normal_dot_sign(p1, p2, domain, psi) {
        ClaimVerdict::Proven(_) => {}
        ClaimVerdict::Disproven(refusal) => return ClaimVerdict::Disproven(refusal),
        ClaimVerdict::Inconclusive(reason) => return ClaimVerdict::Inconclusive(reason),
    }
    // Condition (4): det Dpsi certified nonzero by the shim constructor.
    match SheetCert::try_new(domain, psi.kind(), psi.det_value()) {
        Ok(cert) => ClaimVerdict::Proven(cert),
        Err(refusal) => ClaimVerdict::Disproven(refusal),
    }
}

/// The `Refuse(NearOverlap)` disproof: the pair provably admits no exact ψ
/// over the domain (certified separation at a certified grid vertex), backed
/// `Disproven` of ExactSheet per §11/§17.
fn near_overlap() -> Refusal {
    Refusal::new(
        RefusalKind::NearOverlap,
        RefusalEvidence::Predicate {
            name: "exact_sheet_representational_equality_refuted",
            detail: "the two patches admit no exact psi over the sheet box \
                     (certified separation at a certified grid vertex): Refuse(NearOverlap)"
                .to_string(),
        },
    )
}

/// The determinant of a 2×2 matrix.
fn affine_det(m: [[f64; 2]; 2]) -> f64 {
    m[0][0] * m[1][1] - m[0][1] * m[1][0]
}

/// The certified grid edges of `d` at [`REFINE_DEPTH`]: `(u_edges, v_edges)`,
/// each closed including both endpoints.
fn grid_edges(d: IBox2) -> [Vec<f64>; 2] {
    let n = 1usize << REFINE_DEPTH;
    let denom = n as f64;
    let u_edges: Vec<f64> = (0..=n)
        .map(|i| d.lo[0] + (d.hi[0] - d.lo[0]) * (i as f64) / denom)
        .collect();
    let v_edges: Vec<f64> = (0..=n)
        .map(|j| d.lo[1] + (d.hi[1] - d.lo[1]) * (j as f64) / denom)
        .collect();
    [u_edges, v_edges]
}

/// The certified refinement of `d`: the box itself plus every dyadic sub-box
/// down to depth [`REFINE_DEPTH`]. All bounds are finite and ordered by
/// construction, so the boxes are built as struct literals (no refusing
/// constructor is needed on a certified subdivision of a valid box).
fn refine_boxes(d: IBox2) -> Vec<IBox2> {
    let mut out = Vec::new();
    out.push(d);
    for depth in 1..=REFINE_DEPTH {
        let n = 1usize << depth;
        let denom = n as f64;
        let uw = (d.hi[0] - d.lo[0]) / denom;
        let vw = (d.hi[1] - d.lo[1]) / denom;
        for i in 0..n {
            for j in 0..n {
                let lo0 = d.lo[0] + uw * (i as f64);
                let lo1 = d.lo[1] + vw * (j as f64);
                out.push(IBox2 {
                    lo: [lo0, lo1],
                    hi: [lo0 + uw, lo1 + vw],
                });
            }
        }
    }
    out
}

/// The certified interval of one axis of an `IBox3`.
fn axis(b: &IBox3, k: usize) -> Interval {
    Interval {
        lo: b.lo[k],
        hi: b.hi[k],
    }
}

/// Whether an `IBox3` is fully finite (a certified evaluation exists).
fn box3_finite(b: &IBox3) -> bool {
    b.lo.iter().chain(b.hi.iter()).all(|c| c.is_finite())
}

/// The certified cross product of two derivative boxes.
fn cross3(a: &IBox3, b: &IBox3) -> IBox3 {
    let ax = axis(a, 0);
    let ay = axis(a, 1);
    let az = axis(a, 2);
    let bx = axis(b, 0);
    let by = axis(b, 1);
    let bz = axis(b, 2);
    let cx = ay.mul(&bz).sub(&az.mul(&by));
    let cy = az.mul(&bx).sub(&ax.mul(&bz));
    let cz = ax.mul(&by).sub(&ay.mul(&bx));
    IBox3 {
        lo: [cx.lo, cy.lo, cz.lo],
        hi: [cx.hi, cy.hi, cz.hi],
    }
}

/// The certified dot product of two boxes.
fn dot3(a: &IBox3, b: &IBox3) -> Interval {
    let x = axis(a, 0).mul(&axis(b, 0));
    let y = axis(a, 1).mul(&axis(b, 1));
    let z = axis(a, 2).mul(&axis(b, 2));
    x.add(&y).add(&z)
}

/// Whether two certified boxes are provably disjoint: some coordinate of one
/// lies entirely below the same coordinate of the other.
fn separated(a: &IBox3, b: &IBox3) -> bool {
    (0..3).any(|k| a.hi[k] < b.lo[k] || b.hi[k] < a.lo[k])
}

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}
