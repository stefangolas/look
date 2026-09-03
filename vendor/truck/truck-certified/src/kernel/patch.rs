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

//! The §3.1 certified-patch traits and their shared shapes (BG-KV2-000-CONTRACT).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **D-shim.** The trait shapes are frozen; there are NO implementors in this
//! packet. The capability split is the spec's: `CertifiedPatchC2` is required
//! by R2, R7, and the contact classifier — NOT by R1 tracing, completeness, or
//! overlap; `CertifiedPatchC3` is required only by the A2 cusp classifier and
//! takes a BOX, not a point. Wave-1 implementors (BG-KV2-2xx/3xx/4xx) provide
//! the bodies against these frozen signatures.

use crate::kernel::config::EPS_REP;
use crate::kernel::evidence::{ClaimVerdict, Refusal, RefusalEvidence, RefusalKind};

/// A `N`-dimensional axis-aligned box `[lo, hi]`.
///
/// Construct only through [`IBox::try_new`], which refuses non-finite bounds
/// and any inverted (`lo[i] > hi[i]`) axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IBox<const N: usize> {
    /// The lower corner.
    pub lo: [f64; N],
    /// The upper corner.
    pub hi: [f64; N],
}

/// A box of the parameter plane.
pub type IBox2 = IBox<2>;
/// A box of the surface/derivative space.
pub type IBox3 = IBox<3>;

/// A certified strictly-positive scalar bound.
///
/// Construct only through [`CertifiedPositive::try_new`], which refuses a
/// non-positive or non-finite value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedPositive(f64);

/// A certified non-zero scalar; its sign is recorded by the value itself.
///
/// Construct only through [`CertifiedNonzero::try_new`], which refuses an
/// exactly-zero or non-finite value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedNonzero(f64);

/// A normal cone: an axis and a half-angle.
///
/// Construct only through [`Cone::try_new`], which refuses a half-angle
/// outside `[0, PI)` and a non-unit axis (unit slack [`EPS_REP`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cone {
    /// The cone axis (unit length).
    pub axis: [f64; 3],
    /// The cone half-angle, in `[0, PI)`.
    pub half_angle: f64,
}

/// First-derivative enclosures over a box: `su` and `sv` bound the two
/// partial derivatives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DerivativeEnclosure {
    /// The `u`-partial enclosure.
    pub su: IBox3,
    /// The `v`-partial enclosure.
    pub sv: IBox3,
}

/// Second-derivative enclosures over a box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SecondDerivativeEnclosure {
    /// The `uu`-partial enclosure.
    pub suu: IBox3,
    /// The `uv`-partial enclosure.
    pub suv: IBox3,
    /// The `vv`-partial enclosure.
    pub svv: IBox3,
}

/// Third-derivative (jet) enclosures over a box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThirdJetEnclosure {
    /// The `uuu`-partial enclosure.
    pub suuu: IBox3,
    /// The `uuv`-partial enclosure.
    pub suuv: IBox3,
    /// The `uvv`-partial enclosure.
    pub suvv: IBox3,
    /// The `vvv`-partial enclosure.
    pub svvv: IBox3,
}

/// A regularity degeneracy witness: the box where `EG - F^2` straddles or
/// excludes zero, and the `(lo, hi)` enclosure of `EG - F^2` over it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Degeneracy {
    /// The parameter box.
    pub box_: IBox2,
    /// The `EG - F^2` enclosure `(lo, hi)` over the box.
    pub egf2: (f64, f64),
}

/// A weight pole witness: the box and the `(lo, hi)` enclosure of the weight
/// over it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pole {
    /// The parameter box.
    pub box_: IBox2,
    /// The weight enclosure `(lo, hi)` over the box.
    pub w: (f64, f64),
}

/// A static reason string for an inconclusive verdict.
pub type Reason = &'static str;

// `Refusal` carries `Option<PartialGraph>` per the frozen §2 shape; boxing it
// would deviate from the contract, so the refusing constructors are allowed
// the large-Err lint (BG-KV2-000-CONTRACT).
#[allow(clippy::result_large_err)]
impl<const N: usize> IBox<N> {
    /// Build a box, refusing non-finite bounds or any inverted axis
    /// (`lo[i] > hi[i]`).
    pub fn try_new(lo: [f64; N], hi: [f64; N]) -> Result<Self, Refusal> {
        for i in 0..N {
            if !lo[i].is_finite() || !hi[i].is_finite() {
                return Err(refusal(
                    RefusalKind::NonFinite,
                    "ibox_bound_not_finite",
                    format!("box bound {i} is not finite: [{}, {}]", lo[i], hi[i]),
                ));
            }
        }
        for i in 0..N {
            if lo[i] > hi[i] {
                return Err(refusal(
                    RefusalKind::ClaimRefuted,
                    "ibox_inverted",
                    format!("box axis {i} is inverted: lo {} > hi {}", lo[i], hi[i]),
                ));
            }
        }
        Ok(Self { lo, hi })
    }
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl CertifiedPositive {
    /// Build a certified positive bound, refusing a non-finite or non-positive
    /// value. A non-positive value degenerates the positive certificate.
    pub fn try_new(value: f64) -> Result<Self, Refusal> {
        if !value.is_finite() {
            return Err(refusal(
                RefusalKind::NonFinite,
                "positive_bound_not_finite",
                format!("positive bound {value} is not finite"),
            ));
        }
        if value <= 0.0 {
            return Err(refusal(
                RefusalKind::WeightDegenerate,
                "positive_bound_not_strictly_positive",
                format!("positive bound {value} is not > 0"),
            ));
        }
        Ok(Self(value))
    }

    /// The certified positive bound.
    pub fn value(&self) -> f64 {
        self.0
    }
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl CertifiedNonzero {
    /// Build a certified non-zero bound, refusing a non-finite or exactly-zero
    /// value. An exactly-zero value degenerates the non-zero certificate.
    pub fn try_new(value: f64) -> Result<Self, Refusal> {
        if !value.is_finite() {
            return Err(refusal(
                RefusalKind::NonFinite,
                "nonzero_bound_not_finite",
                format!("nonzero bound {value} is not finite"),
            ));
        }
        if value == 0.0 {
            return Err(refusal(
                RefusalKind::WeightDegenerate,
                "nonzero_bound_is_zero",
                "nonzero bound is exactly 0".to_string(),
            ));
        }
        Ok(Self(value))
    }

    /// The certified non-zero bound.
    pub fn value(&self) -> f64 {
        self.0
    }
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl Cone {
    /// Build a normal cone, refusing a half-angle outside `[0, PI)` and a
    /// non-unit axis (unit slack [`EPS_REP`]).
    pub fn try_new(axis: [f64; 3], half_angle: f64) -> Result<Self, Refusal> {
        if !half_angle.is_finite() || !(0.0..std::f64::consts::PI).contains(&half_angle) {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "cone_half_angle_out_of_range",
                format!("half-angle {half_angle} outside [0, PI)"),
            ));
        }
        if !axis.iter().all(|c| c.is_finite()) {
            return Err(refusal(
                RefusalKind::NonFinite,
                "cone_axis_not_finite",
                format!("cone axis {axis:?} is not finite"),
            ));
        }
        if !is_unit(axis, EPS_REP) {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "cone_axis_not_unit",
                format!("cone axis {axis:?} is not unit to {EPS_REP}"),
            ));
        }
        Ok(Self { axis, half_angle })
    }
}

/// The §3.1 certified-patch contract, level C1: a patch over a parameter box
/// answers enclosures and the two certificates that need only first
/// derivatives.
pub trait CertifiedPatch {
    /// The certified position enclosure of the patch over the box `d`.
    fn enclose(&self, d: IBox2) -> IBox3;
    /// The certified first-derivative enclosures of the patch over `d`.
    fn derivs(&self, d: IBox2) -> DerivativeEnclosure;
    /// A normal cone certified to contain the patch normal over `d`.
    fn normal_cone(&self, d: IBox2) -> Cone;
    /// Certify regularity (`EG - F^2` away from zero) over `d`: `Proven`
    /// carries a certified positive lower bound, `Disproven` a degeneracy
    /// witness, `Inconclusive` the reason.
    fn regularity(&self, d: IBox2) -> ClaimVerdict<CertifiedPositive, Degeneracy, Reason>;
    /// Certify the weight over `d`. `None` when the patch has no weight field
    /// (a polynomial, unit-weight patch).
    fn weight_bound(&self, d: IBox2) -> Option<ClaimVerdict<CertifiedPositive, Pole, Reason>>;
}

/// The §3.1 level C2 contract: adds the second-derivative enclosure. Required
/// by R2, R7, and the contact classifier; NOT required by R1 tracing,
/// completeness, or overlap.
pub trait CertifiedPatchC2: CertifiedPatch {
    /// The certified second-derivative enclosures over `d`.
    fn second_derivs(&self, d: IBox2) -> SecondDerivativeEnclosure;
}

/// The §3.1 level C3 contract: adds the third-jet enclosure. Required only by
/// the A2 cusp classifier, and takes a BOX, not a point.
pub trait CertifiedPatchC3: CertifiedPatchC2 {
    /// The certified third-derivative (jet) enclosures over `d`.
    fn third_jet(&self, d: IBox2) -> ThirdJetEnclosure;
}

fn is_unit(v: [f64; 3], slack: f64) -> bool {
    let norm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    (norm - 1.0).abs() <= slack
}

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}
