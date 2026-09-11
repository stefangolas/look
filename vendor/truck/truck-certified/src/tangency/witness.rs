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

//! The one-witness exact verifier (CTE-001-QPOLY; theory §4.2, scope decision
//! 2 and 4).
//!
//! This module implements the frozen one-verifier contract
//! [`ExactWitnessVerifier`] over the ℚ-polynomial substrate [`QPoly`]: it
//! expands `s·f − (q²a + Σᵢ wᵢPᵢ)` over ℚ and compares coefficients to zero
//! **exactly**, then enforces the interval nonvanishing predicates `0 ∉ ŝ(B)`
//! and (when `q` is present) `0 ∉ â(B)` against the caller-supplied
//! enclosures. ONE code path — the [`WitnessSide`] tag selects nothing except
//! documentation. Producers never verify; the verifier never produces.
//!
//! The verifier is *total and side-parametric*: every failure — a polynomial
//! shape that disagrees with the supplied system polynomials, an identity
//! whose residual does not vanish exactly, a missing enclosure, or an
//! enclosure containing `0` — is a [`WitnessRefusal`], never a panic and never
//! an unchecked assumption.
//!
//! **Pending refusal.** The provenance fast path is wired as a stub behind
//! [`provenance_witness`], which refuses with the named pending cause
//! [`WITNESS_PRODUCER_PACKET_PENDING`] until CTE-005/007 wire real provenance
//! (the `cone_torus_carrier_packet_pending` precedent, `kernel/rational.rs`).
//! Normal-form/RUR producers are likewise booked open (theory §6.1) and are
//! NOT implemented here.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`,
//! and adds no module-level `allow`.

use crate::contract::Refusal;
use crate::formal::curve2d::SourceEntityId;
use crate::tangency::qpoly::{QCoeff, QPoly};
use crate::tangency::shapes::{ExactVanishingWitness, ExactWitnessVerifier, WitnessSide};
use std::cmp::Ordering;

/// The named pending cause under which [`provenance_witness`] refuses: the
/// provenance fast path is CTE-005/007's work, so the producing stub refuses
/// until then (the `a2_branch_packet_pending` / `cone_torus_carrier_packet_pending`
/// precedent).
pub const WITNESS_PRODUCER_PACKET_PENDING: &str = "witness_producer_packet_pending";

/// The CTE-001 refusal vocabulary (H-2): named cases only, no catch-all.
///
/// - [`WitnessRefusal::Overflow`] — exact `i128` coefficient arithmetic
///   overflowed: a named event (never a wrap, never a panic). A real overflow
///   on a fixture is a degree/coefficient-scale decision the loop must make.
/// - [`WitnessRefusal::InvalidInput`] — a construction or call outside a
///   frozen rule: an arity mismatch, a denominator of zero, a missing
///   nonvanishing enclosure, an enclosure containing `0`, or a witness whose
///   recorded polynomials disagree with the system polynomials supplied to the
///   verifier.
/// - [`WitnessRefusal::Pending`] — a producer that is booked open and refuses
///   with its named pending cause until its owning packet lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessRefusal {
    /// Exact `i128` coefficient arithmetic overflowed.
    Overflow,
    /// A construction or verification outside a frozen rule.
    InvalidInput,
    /// A producer that is booked open; carries the named pending cause.
    Pending(&'static str),
}

impl WitnessRefusal {
    /// A short stable diagnostic tag.
    pub const fn tag(self) -> &'static str {
        match self {
            WitnessRefusal::Overflow => "witness_coefficient_overflow",
            WitnessRefusal::InvalidInput => "witness_invalid_input",
            WitnessRefusal::Pending(cause) => cause,
        }
    }
}

/// Whether an interval strictly excludes `0` — the strict-exclusion rule of
/// `KrawczykCertificate3`'s determinant and `Rank2Chart`'s `det_a`: an
/// enclosure containing `0` inclusive of a `0` endpoint does not certify
/// nonvanishing. A non-finite or misordered interval also does not.
fn strictly_excludes_zero((lo, hi): (f64, f64)) -> bool {
    lo.is_finite() && hi.is_finite() && lo <= hi && !(lo <= 0.0 && 0.0 <= hi)
}

/// Map an exactness-layer refusal onto the shape layer's [`Refusal`]
/// vocabulary (the `ExactWitnessVerifier` contract is frozen on
/// `contract::Refusal`; the named CTE-001 cause is recorded here and in the
/// mapping table).
fn to_refusal(_: WitnessRefusal) -> Refusal {
    Refusal::InvalidInput
}

/// Expand `s·f − (q²a + Σᵢ wᵢ·Pᵢ)` over ℚ and return whether it is exactly
/// zero.
///
/// The correction polynomials `Pᵢ` are taken from `p` (the system
/// polynomials the caller is certifying against); each must equal the
/// polynomial recorded in the witness's `i`-th term, else the witness is not
/// evidence about the supplied system and the call refuses.
fn residual_is_zero(
    witness: &ExactVanishingWitness<QPoly>,
    f: &QPoly,
    p: &[&QPoly],
) -> Result<bool, WitnessRefusal> {
    let terms = witness.terms();
    if terms.len() != p.len() {
        return Err(WitnessRefusal::InvalidInput);
    }
    for ((_w, recorded), &given) in terms.iter().zip(p.iter()) {
        if recorded != given {
            return Err(WitnessRefusal::InvalidInput);
        }
    }

    let s = witness.s();
    let lhs = s.mul(f)?;

    let mut rhs = QPoly::zero(f.arity());
    if let Some((q, a)) = witness.q2a() {
        rhs = rhs.add(&q.mul(q)?.mul(a)?)?;
    }
    for ((w, _), &pi) in terms.iter().zip(p.iter()) {
        rhs = rhs.add(&w.mul(pi)?)?;
    }
    let diff = lhs.sub(&rhs)?;
    Ok(diff.is_zero())
}

impl ExactWitnessVerifier<QPoly> for ExactVanishingWitness<QPoly> {
    fn verify(
        &self,
        f: &QPoly,
        p: &[&QPoly],
        s_encl: Option<(f64, f64)>,
        a_encl: Option<(f64, f64)>,
    ) -> Result<(), Refusal> {
        // The interval nonvanishing predicates are REQUIRED: ŝ(B) always,
        // and â(B) exactly when the witness carries the q²a term (theory
        // §4.2). A missing enclosure cannot certify nonvanishing.
        let s_encl = s_encl.ok_or(Refusal::InvalidInput)?;
        if !strictly_excludes_zero(s_encl) {
            return Err(Refusal::InvalidInput);
        }
        match (self.q2a().is_some(), a_encl) {
            (true, Some(a_encl)) => {
                if !strictly_excludes_zero(a_encl) {
                    return Err(Refusal::InvalidInput);
                }
            }
            (true, None) | (false, Some(_)) => return Err(Refusal::InvalidInput),
            (false, None) => {}
        }

        let zero = residual_is_zero(self, f, p).map_err(to_refusal)?;
        if zero {
            Ok(())
        } else {
            Err(Refusal::InvalidInput)
        }
    }
}

/// The provenance fast path: a constructor from already-known polynomial data.
///
/// **Pending refusal stub.** Real provenance — construction identity, fillet
/// spine, exact seating constraints — is CTE-005/007's work, so this stub
/// ALWAYS refuses with the named pending cause
/// [`WITNESS_PRODUCER_PACKET_PENDING`] (the `cone_torus_carrier_packet_pending`
/// precedent). No stub logic, no fake results (scope decision 5). When real
/// provenance lands, its identity still flows through the ONE verifier
/// ([`ExactWitnessVerifier`]) — provenance is a hint about what to TRY, never a
/// trust level (scope decision 4).
pub fn provenance_witness(
    _side: WitnessSide,
    _s: QPoly,
    _terms: Vec<(QPoly, QPoly)>,
    _q2a: Option<(QPoly, QPoly)>,
) -> Result<ExactVanishingWitness<QPoly>, WitnessRefusal> {
    Err(WitnessRefusal::Pending(WITNESS_PRODUCER_PACKET_PENDING))
}

// ---------------------------------------------------------------------------
// RDEF-M3-WITNESS-TIER: the exact quadric-pencil witness tier (spec §5.2)
// ---------------------------------------------------------------------------
//
// This section lands the exact witness tier W1–W3 of the rank-deficient
// contact program (`docs/RANK_DEFICIENT_CONTACT_SPEC.md` §5.1–5.2, §9 M3):
//
// - **W1 quadric pencils.** Exact classification of canonical quadric
//   carriers (plane, sphere, cylinder, cone) by exact rational arithmetic on
//   the carrier definitions — the exact-classification route of
//   Dupont–Lazard–Lazard–Petitjean. Tori are quartic and are EXCLUDED here
//   (they belong to the numeric tier, M4).
// - **W2 coincident carriers.** Exact rational equality of the carrier
//   definitions, or the importer provenance `SourceEntityId`. The crate's
//   importer seam does NOT carry a `FaceProvenance` record — only
//   `SourceEntityId` (`formal::curve2d`, a `u64` document entity id) is
//   retained — so the provenance fast path is keyed on that id, and its
//   absence is reported, never invented by sampling.
// - **W3 construction witnesses.** A fillet/blend construction must EMIT the
//   exact contact-curve certificate. A bare flag cannot construct a
//   [`ConstructionWitness`]: the exact curve must lie on its carrier, checked
//   exactly.
//
// No tolerance, no epsilon and no numeric classification appears here; every
// comparison is an exact rational comparison (`QCoeff`), and every refusal is
// a named [`WitnessTierRefusal`] case.

/// The named refusal vocabulary of the exact witness tier (RDEF-M3). Named
/// cases only, no catch-all; no tolerance and no numeric classification is
/// performed in this tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessTierRefusal {
    /// A carrier pair outside the exact quadric-pencil scope (the mixed
    /// cylinder×cone and cone×cone pencils, and an off-axis sphere×cone pair,
    /// are not classified in M3).
    UnsupportedCarrierPair,
    /// A torus carrier: quartic, excluded from the exact tier (numeric tier).
    TorusExcluded,
    /// A degenerate carrier definition: zero normal/axis, or a non-positive
    /// radius/tangent square.
    DegenerateCarrier,
    /// Exact `i128` arithmetic overflowed.
    ArithmeticOverflow,
    /// A construction witness's curve does not lie exactly on its carrier (a
    /// flag is not a certificate).
    CurveOffCarrier,
}

impl WitnessTierRefusal {
    /// A short stable diagnostic tag.
    pub const fn tag(self) -> &'static str {
        match self {
            WitnessTierRefusal::UnsupportedCarrierPair => "witness_tier_unsupported_pair",
            WitnessTierRefusal::TorusExcluded => "witness_tier_torus_excluded",
            WitnessTierRefusal::DegenerateCarrier => "witness_tier_degenerate_carrier",
            WitnessTierRefusal::ArithmeticOverflow => "witness_tier_arithmetic_overflow",
            WitnessTierRefusal::CurveOffCarrier => "witness_tier_curve_off_carrier",
        }
    }
}

fn q_one() -> QCoeff {
    QCoeff::one()
}

fn q_int(n: i128) -> QCoeff {
    QCoeff::from_int(n)
}

fn q_neg(a: QCoeff) -> Result<QCoeff, WitnessTierRefusal> {
    a.neg().map_err(|_| WitnessTierRefusal::ArithmeticOverflow)
}

fn q_add(a: QCoeff, b: QCoeff) -> Result<QCoeff, WitnessTierRefusal> {
    a.add(&b)
        .map_err(|_| WitnessTierRefusal::ArithmeticOverflow)
}

fn q_sub(a: QCoeff, b: QCoeff) -> Result<QCoeff, WitnessTierRefusal> {
    a.sub(&b)
        .map_err(|_| WitnessTierRefusal::ArithmeticOverflow)
}

fn q_mul(a: QCoeff, b: QCoeff) -> Result<QCoeff, WitnessTierRefusal> {
    a.mul(&b)
        .map_err(|_| WitnessTierRefusal::ArithmeticOverflow)
}

/// Exact rational division. Refuses a zero divisor.
fn q_div(a: QCoeff, b: QCoeff) -> Result<QCoeff, WitnessTierRefusal> {
    if b.is_zero() {
        return Err(WitnessTierRefusal::DegenerateCarrier);
    }
    let recip =
        QCoeff::new(b.den(), b.num()).map_err(|_| WitnessTierRefusal::ArithmeticOverflow)?;
    q_mul(a, recip)
}

/// The exact sign of a rational: `-1`, `0` or `1`.
fn q_sign(a: QCoeff) -> i32 {
    let n = a.num();
    if n > 0 {
        1
    } else if n < 0 {
        -1
    } else {
        0
    }
}

/// Exact rational comparison (the denominators are positive by construction).
fn q_cmp(a: QCoeff, b: QCoeff) -> Result<Ordering, WitnessTierRefusal> {
    let lhs = a
        .num()
        .checked_mul(b.den())
        .ok_or(WitnessTierRefusal::ArithmeticOverflow)?;
    let rhs = b
        .num()
        .checked_mul(a.den())
        .ok_or(WitnessTierRefusal::ArithmeticOverflow)?;
    Ok(lhs.cmp(&rhs))
}

fn v_sub(a: &[QCoeff; 3], b: &[QCoeff; 3]) -> Result<[QCoeff; 3], WitnessTierRefusal> {
    Ok([q_sub(a[0], b[0])?, q_sub(a[1], b[1])?, q_sub(a[2], b[2])?])
}

fn v_scale(a: &[QCoeff; 3], s: QCoeff) -> Result<[QCoeff; 3], WitnessTierRefusal> {
    Ok([q_mul(a[0], s)?, q_mul(a[1], s)?, q_mul(a[2], s)?])
}

fn v_dot(a: &[QCoeff; 3], b: &[QCoeff; 3]) -> Result<QCoeff, WitnessTierRefusal> {
    let x = q_mul(a[0], b[0])?;
    let y = q_mul(a[1], b[1])?;
    let z = q_mul(a[2], b[2])?;
    q_add(q_add(x, y)?, z)
}

fn v_cross(a: &[QCoeff; 3], b: &[QCoeff; 3]) -> Result<[QCoeff; 3], WitnessTierRefusal> {
    Ok([
        q_sub(q_mul(a[1], b[2])?, q_mul(a[2], b[1])?)?,
        q_sub(q_mul(a[2], b[0])?, q_mul(a[0], b[2])?)?,
        q_sub(q_mul(a[0], b[1])?, q_mul(a[1], b[0])?)?,
    ])
}

fn v_is_zero(a: &[QCoeff; 3]) -> bool {
    a[0].is_zero() && a[1].is_zero() && a[2].is_zero()
}

/// Whether `b = λ·a` exactly for some rational `λ`. `None` when the vectors
/// are not proportional. Refuses only a degenerate (all-zero) `a`.
fn proportional(a: &[QCoeff; 3], b: &[QCoeff; 3]) -> Result<Option<QCoeff>, WitnessTierRefusal> {
    let k = a
        .iter()
        .position(|c| !c.is_zero())
        .ok_or(WitnessTierRefusal::DegenerateCarrier)?;
    let lambda = q_div(b[k], a[k])?;
    for i in 0..3 {
        if q_mul(lambda, a[i])? != b[i] {
            return Ok(None);
        }
    }
    Ok(Some(lambda))
}

/// An exact canonical carrier over ℚ: the plane, sphere, cylinder and cone
/// classes of the W1 witness tier (spec §5.2). Tori are excluded (quartic).
///
/// Every field is an exact rational; the `radius_sq`/`tan_sq` fields are the
/// SQUARES of the radius and of the half-angle tangent, so no irrational
/// quantity is ever represented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactCarrier {
    /// The affine plane `normal · x = offset`.
    Plane {
        /// The plane's normal (nonzero).
        normal: [QCoeff; 3],
        /// The plane's affine offset.
        offset: QCoeff,
    },
    /// The sphere `|x − center|² = radius_sq`.
    Sphere {
        /// The sphere's centre.
        center: [QCoeff; 3],
        /// The squared radius (strictly positive).
        radius_sq: QCoeff,
    },
    /// The cylinder: points whose squared distance to the axis line
    /// `axis_point + t·axis_dir` equals `radius_sq`.
    Cylinder {
        /// A point on the axis.
        axis_point: [QCoeff; 3],
        /// The axis direction (nonzero).
        axis_dir: [QCoeff; 3],
        /// The squared radius (strictly positive).
        radius_sq: QCoeff,
    },
    /// The right circular cone with `apex`, `axis_dir` and squared half-angle
    /// tangent `tan_sq`: `|x − apex|²·|axis_dir|² = (1 + tan_sq)·((x − apex) ·
    /// axis_dir)²`.
    Cone {
        /// The cone apex.
        apex: [QCoeff; 3],
        /// The axis direction (nonzero).
        axis_dir: [QCoeff; 3],
        /// The squared half-angle tangent (strictly positive).
        tan_sq: QCoeff,
    },
}

impl ExactCarrier {
    /// The plane `normal · x = offset` from integer data.
    pub fn plane(normal: [i128; 3], offset: i128) -> Result<Self, WitnessTierRefusal> {
        let n = [q_int(normal[0]), q_int(normal[1]), q_int(normal[2])];
        if v_is_zero(&n) {
            return Err(WitnessTierRefusal::DegenerateCarrier);
        }
        Ok(ExactCarrier::Plane {
            normal: n,
            offset: q_int(offset),
        })
    }

    /// The sphere `|x − center|² = radius_sq` from integer data.
    pub fn sphere(center: [i128; 3], radius_sq: i128) -> Result<Self, WitnessTierRefusal> {
        if radius_sq <= 0 {
            return Err(WitnessTierRefusal::DegenerateCarrier);
        }
        Ok(ExactCarrier::Sphere {
            center: [q_int(center[0]), q_int(center[1]), q_int(center[2])],
            radius_sq: q_int(radius_sq),
        })
    }

    /// The cylinder about `axis_point + t·axis_dir` with squared radius
    /// `radius_sq`, from integer data.
    pub fn cylinder(
        axis_point: [i128; 3],
        axis_dir: [i128; 3],
        radius_sq: i128,
    ) -> Result<Self, WitnessTierRefusal> {
        let dir = [q_int(axis_dir[0]), q_int(axis_dir[1]), q_int(axis_dir[2])];
        if v_is_zero(&dir) || radius_sq <= 0 {
            return Err(WitnessTierRefusal::DegenerateCarrier);
        }
        Ok(ExactCarrier::Cylinder {
            axis_point: [
                q_int(axis_point[0]),
                q_int(axis_point[1]),
                q_int(axis_point[2]),
            ],
            axis_dir: dir,
            radius_sq: q_int(radius_sq),
        })
    }

    /// The right circular cone with `apex`, `axis_dir` and squared half-angle
    /// tangent `tan_sq`, from integer data.
    pub fn cone(
        apex: [i128; 3],
        axis_dir: [i128; 3],
        tan_sq: i128,
    ) -> Result<Self, WitnessTierRefusal> {
        let dir = [q_int(axis_dir[0]), q_int(axis_dir[1]), q_int(axis_dir[2])];
        if v_is_zero(&dir) || tan_sq <= 0 {
            return Err(WitnessTierRefusal::DegenerateCarrier);
        }
        Ok(ExactCarrier::Cone {
            apex: [q_int(apex[0]), q_int(apex[1]), q_int(apex[2])],
            axis_dir: dir,
            tan_sq: q_int(tan_sq),
        })
    }
}

/// The exact contact class of a canonical carrier pair (the lattice-v2
/// children of `rank_deficient`, spec CHK-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactContactClass {
    /// The carriers do not meet.
    Empty,
    /// A regular (transverse) intersection curve.
    Regular,
    /// An isolated tangential contact point.
    TangentPoint,
    /// Tangential contact along a curve.
    TangentCurve,
    /// A one-dimensional contact locus carrying a singular point (e.g. a
    /// plane through a cone apex: two generators crossing at the apex).
    TangentCrossing,
    /// The carriers coincide (W2).
    Coincident,
}

impl ExactContactClass {
    /// A short stable diagnostic tag.
    pub const fn tag(self) -> &'static str {
        match self {
            ExactContactClass::Empty => "empty",
            ExactContactClass::Regular => "regular",
            ExactContactClass::TangentPoint => "tangent_point",
            ExactContactClass::TangentCurve => "tangent_curve",
            ExactContactClass::TangentCrossing => "tangent_crossing",
            ExactContactClass::Coincident => "coincident",
        }
    }

    /// The lattice-v2 `local_dim` this class fixes (spec CHK-5). `None` when
    /// the class does not determine one.
    pub fn local_dim(self) -> Option<i32> {
        match self {
            ExactContactClass::Empty => Some(0),
            ExactContactClass::Regular => Some(1),
            ExactContactClass::TangentPoint => Some(0),
            ExactContactClass::TangentCurve => Some(1),
            ExactContactClass::TangentCrossing => Some(1),
            ExactContactClass::Coincident => Some(2),
        }
    }
}

/// Whether two carrier definitions describe exactly the same point set (the
/// W2 exact-rational-equality predicate). Distinct variants never coincide.
pub fn same_carrier(a: &ExactCarrier, b: &ExactCarrier) -> Result<bool, WitnessTierRefusal> {
    match (a, b) {
        (
            ExactCarrier::Plane {
                normal: n1,
                offset: d1,
            },
            ExactCarrier::Plane {
                normal: n2,
                offset: d2,
            },
        ) => match proportional(n1, n2)? {
            None => Ok(false),
            Some(lambda) => Ok(q_mul(lambda, *d1)? == *d2),
        },
        (
            ExactCarrier::Sphere {
                center: c1,
                radius_sq: r1,
            },
            ExactCarrier::Sphere {
                center: c2,
                radius_sq: r2,
            },
        ) => Ok(c1 == c2 && r1 == r2),
        (
            ExactCarrier::Cylinder {
                axis_point: p1,
                axis_dir: v1,
                radius_sq: r1,
            },
            ExactCarrier::Cylinder {
                axis_point: p2,
                axis_dir: v2,
                radius_sq: r2,
            },
        ) => {
            if r1 != r2 || proportional(v1, v2)?.is_none() {
                return Ok(false);
            }
            let delta = v_sub(p2, p1)?;
            if v_is_zero(&delta) {
                return Ok(true);
            }
            Ok(proportional(v1, &delta)?.is_some())
        }
        (
            ExactCarrier::Cone {
                apex: a1,
                axis_dir: v1,
                tan_sq: k1,
            },
            ExactCarrier::Cone {
                apex: a2,
                axis_dir: v2,
                tan_sq: k2,
            },
        ) => Ok(a1 == a2 && k1 == k2 && proportional(v1, v2)?.is_some()),
        _ => Ok(false),
    }
}

fn plane_plane(
    n1: &[QCoeff; 3],
    n2: &[QCoeff; 3],
) -> Result<ExactContactClass, WitnessTierRefusal> {
    if v_is_zero(&v_cross(n1, n2)?) {
        Ok(ExactContactClass::Empty)
    } else {
        Ok(ExactContactClass::Regular)
    }
}

fn plane_sphere(
    n: &[QCoeff; 3],
    d: QCoeff,
    c: &[QCoeff; 3],
    radius_sq: QCoeff,
) -> Result<ExactContactClass, WitnessTierRefusal> {
    let s = q_sub(v_dot(n, c)?, d)?;
    let lhs = q_mul(s, s)?;
    let rhs = q_mul(radius_sq, v_dot(n, n)?)?;
    match q_cmp(lhs, rhs)? {
        Ordering::Less => Ok(ExactContactClass::Regular),
        Ordering::Equal => Ok(ExactContactClass::TangentPoint),
        Ordering::Greater => Ok(ExactContactClass::Empty),
    }
}

fn plane_cylinder(
    n: &[QCoeff; 3],
    d: QCoeff,
    axis_point: &[QCoeff; 3],
    axis_dir: &[QCoeff; 3],
    radius_sq: QCoeff,
) -> Result<ExactContactClass, WitnessTierRefusal> {
    let m = v_dot(n, axis_dir)?;
    if !m.is_zero() {
        // A plane with a component along the axis cuts the infinite cylinder
        // in an ellipse: a regular intersection curve.
        return Ok(ExactContactClass::Regular);
    }
    let s = q_sub(v_dot(n, axis_point)?, d)?;
    let lhs = q_mul(s, s)?;
    let rhs = q_mul(radius_sq, v_dot(n, n)?)?;
    match q_cmp(lhs, rhs)? {
        Ordering::Less => Ok(ExactContactClass::Regular),
        Ordering::Equal => Ok(ExactContactClass::TangentCurve),
        Ordering::Greater => Ok(ExactContactClass::Empty),
    }
}

fn det3(m: [[QCoeff; 3]; 3]) -> Result<QCoeff, WitnessTierRefusal> {
    let m00 = q_mul(
        m[0][0],
        q_sub(q_mul(m[1][1], m[2][2])?, q_mul(m[1][2], m[2][1])?)?,
    )?;
    let m01 = q_mul(
        m[0][1],
        q_sub(q_mul(m[1][0], m[2][2])?, q_mul(m[1][2], m[2][0])?)?,
    )?;
    let m02 = q_mul(
        m[0][2],
        q_sub(q_mul(m[1][0], m[2][1])?, q_mul(m[1][1], m[2][0])?)?,
    )?;
    q_sub(q_add(m00, q_neg(m01)?)?, m02)
}

#[allow(clippy::needless_range_loop)] // fixed 3x3 minor sweep
fn rank_le_one(m: &[[QCoeff; 3]; 3]) -> Result<bool, WitnessTierRefusal> {
    for i in 0..3 {
        for j in (i + 1)..3 {
            for k in 0..3 {
                for l in (k + 1)..3 {
                    let minor = q_sub(q_mul(m[i][k], m[j][l])?, q_mul(m[i][l], m[j][k])?)?;
                    if !minor.is_zero() {
                        return Ok(false);
                    }
                }
            }
        }
    }
    Ok(true)
}

/// Classify the real solution set of the 2-D conic whose symmetric 3×3
/// matrix is `m` (the restricted cone form on a plane). Exact over ℚ.
fn classify_conic(m: [[QCoeff; 3]; 3]) -> Result<ExactContactClass, WitnessTierRefusal> {
    let det = det3(m)?;
    if !det.is_zero() {
        return Ok(ExactContactClass::Regular);
    }
    if rank_le_one(&m)? {
        // A double line: the plane is tangent to the cone along a generator.
        return Ok(ExactContactClass::TangentCurve);
    }
    // Rank 2.
    let a11 = m[0][0];
    let a12 = m[0][1];
    let a22 = m[1][1];
    let det_a = q_sub(q_mul(a11, a22)?, q_mul(a12, a12)?)?;
    let sign_a = q_sign(det_a);
    if sign_a < 0 {
        // Two real lines crossing at the apex.
        return Ok(ExactContactClass::TangentCrossing);
    }
    if sign_a == 0 {
        // Rank-2 conic with a singular quadratic block: parallel-line residue.
        return Ok(ExactContactClass::Empty);
    }
    // det_a > 0: complete the square. `c' = c − bᵀ A⁻¹ b`, so
    // `c'·det_a = c·det_a − (a22 b1² − 2 a12 b1 b2 + a11 b2²)`.
    let b1 = m[0][2];
    let b2 = m[1][2];
    let c = m[2][2];
    let t = q_add(
        q_sub(
            q_mul(a22, q_mul(b1, b1)?)?,
            q_mul(q_int(2), q_mul(a12, q_mul(b1, b2)?)?)?,
        )?,
        q_mul(a11, q_mul(b2, b2)?)?,
    )?;
    let cprime_num = q_sub(q_mul(c, det_a)?, t)?;
    match q_sign(cprime_num) {
        0 => Ok(ExactContactClass::TangentPoint),
        s if s == q_sign(a11) => Ok(ExactContactClass::Empty),
        _ => Ok(ExactContactClass::Regular),
    }
}

fn plane_basis(n: &[QCoeff; 3]) -> Result<([QCoeff; 3], [QCoeff; 3]), WitnessTierRefusal> {
    let e1 = if !n[0].is_zero() || !n[1].is_zero() {
        [n[1], q_neg(n[0])?, QCoeff::zero()]
    } else {
        [QCoeff::one(), QCoeff::zero(), QCoeff::zero()]
    };
    let e2 = v_cross(n, &e1)?;
    Ok((e1, e2))
}

fn plane_cone(
    n: &[QCoeff; 3],
    d: QCoeff,
    apex: &[QCoeff; 3],
    v: &[QCoeff; 3],
    tan_sq: QCoeff,
) -> Result<ExactContactClass, WitnessTierRefusal> {
    let nn = v_dot(n, n)?;
    let vv = v_dot(v, v)?;
    let one_plus_k = q_add(q_one(), tan_sq)?;
    let (e1, e2) = plane_basis(n)?;
    let o_global = v_scale(n, q_div(d, nn)?)?;
    let q0 = v_sub(&o_global, apex)?;

    let quad = |e: &[QCoeff; 3]| -> Result<QCoeff, WitnessTierRefusal> {
        let ee = v_dot(e, e)?;
        let ev = v_dot(e, v)?;
        q_sub(q_mul(ee, vv)?, q_mul(one_plus_k, q_mul(ev, ev)?)?)
    };
    let bilin = |a: &[QCoeff; 3], b: &[QCoeff; 3]| -> Result<QCoeff, WitnessTierRefusal> {
        let ab = v_dot(a, b)?;
        let av = v_dot(a, v)?;
        let bv = v_dot(b, v)?;
        q_sub(q_mul(ab, vv)?, q_mul(one_plus_k, q_mul(av, bv)?)?)
    };

    let a11 = quad(&e1)?;
    let a22 = quad(&e2)?;
    let a12 = bilin(&e1, &e2)?;
    let b1 = bilin(&q0, &e1)?;
    let b2 = bilin(&q0, &e2)?;
    let c = quad(&q0)?;
    classify_conic([[a11, a12, b1], [a12, a22, b2], [b1, b2, c]])
}

fn sphere_sphere(
    c1: &[QCoeff; 3],
    r1: QCoeff,
    c2: &[QCoeff; 3],
    r2: QCoeff,
) -> Result<ExactContactClass, WitnessTierRefusal> {
    let delta = v_sub(c1, c2)?;
    let d2 = v_dot(&delta, &delta)?;
    let s = q_sub(d2, q_add(r1, r2)?)?;
    let four = q_mul(q_int(4), q_mul(r1, r2)?)?;
    let s2 = q_mul(s, s)?;
    match q_cmp(s2, four)? {
        Ordering::Equal => Ok(ExactContactClass::TangentPoint),
        Ordering::Greater => Ok(ExactContactClass::Empty),
        Ordering::Less => Ok(ExactContactClass::Regular),
    }
}

/// The squared perpendicular distance from `p` to the line
/// `axis_point + t·axis_dir`.
fn point_axis_dist_sq(
    p: &[QCoeff; 3],
    axis_point: &[QCoeff; 3],
    axis_dir: &[QCoeff; 3],
) -> Result<QCoeff, WitnessTierRefusal> {
    let delta = v_sub(p, axis_point)?;
    let t = q_div(v_dot(&delta, axis_dir)?, v_dot(axis_dir, axis_dir)?)?;
    let proj = v_scale(axis_dir, t)?;
    let perp = v_sub(&delta, &proj)?;
    v_dot(&perp, &perp)
}

fn cylinder_cylinder(
    p1: &[QCoeff; 3],
    v1: &[QCoeff; 3],
    r1: QCoeff,
    p2: &[QCoeff; 3],
    v2: &[QCoeff; 3],
    r2: QCoeff,
) -> Result<ExactContactClass, WitnessTierRefusal> {
    if !v_is_zero(&v_cross(v1, v2)?) {
        // Non-parallel axes: a generic transversal quartic curve.
        return Ok(ExactContactClass::Regular);
    }
    let delta = v_sub(p2, p1)?;
    let t = q_div(v_dot(&delta, v1)?, v_dot(v1, v1)?)?;
    let proj = v_scale(v1, t)?;
    let perp = v_sub(&delta, &proj)?;
    let d2 = v_dot(&perp, &perp)?;
    let s = q_sub(d2, q_add(r1, r2)?)?;
    let four = q_mul(q_int(4), q_mul(r1, r2)?)?;
    let s2 = q_mul(s, s)?;
    match q_cmp(s2, four)? {
        Ordering::Equal => Ok(ExactContactClass::TangentCurve),
        Ordering::Greater => Ok(ExactContactClass::Empty),
        Ordering::Less => Ok(ExactContactClass::Regular),
    }
}

fn sphere_cylinder(
    c: &[QCoeff; 3],
    sphere_radius_sq: QCoeff,
    axis_point: &[QCoeff; 3],
    axis_dir: &[QCoeff; 3],
    cylinder_radius_sq: QCoeff,
) -> Result<ExactContactClass, WitnessTierRefusal> {
    let d2 = point_axis_dist_sq(c, axis_point, axis_dir)?;
    let s = q_sub(d2, q_add(sphere_radius_sq, cylinder_radius_sq)?)?;
    let four = q_mul(q_int(4), q_mul(sphere_radius_sq, cylinder_radius_sq)?)?;
    let s2 = q_mul(s, s)?;
    match q_cmp(s2, four)? {
        Ordering::Equal => {
            if d2.is_zero() {
                // Concentric equal-radius sphere and cylinder meet in a circle.
                Ok(ExactContactClass::Regular)
            } else {
                Ok(ExactContactClass::TangentPoint)
            }
        }
        Ordering::Greater => Ok(ExactContactClass::Empty),
        Ordering::Less => Ok(ExactContactClass::Regular),
    }
}

fn sphere_cone(
    c: &[QCoeff; 3],
    sphere_radius_sq: QCoeff,
    apex: &[QCoeff; 3],
    v: &[QCoeff; 3],
    tan_sq: QCoeff,
) -> Result<ExactContactClass, WitnessTierRefusal> {
    let delta = v_sub(c, apex)?;
    let vv = v_dot(v, v)?;
    let h = v_dot(&delta, v)?;
    let d2 = v_dot(&delta, &delta)?;
    // The exact classification here is the on-axis sphere×cone pencil; an
    // off-axis sphere×cone is booked open (a finding, not a sampled guess).
    if q_mul(d2, vv)? != q_mul(h, h)? {
        return Err(WitnessTierRefusal::UnsupportedCarrierPair);
    }
    let apex_on_sphere = d2 == sphere_radius_sq;
    let lhs = q_mul(q_mul(sphere_radius_sq, q_add(q_one(), tan_sq)?)?, vv)?;
    let rhs = q_mul(q_mul(h, h)?, tan_sq)?;
    match q_cmp(lhs, rhs)? {
        Ordering::Equal => Ok(ExactContactClass::TangentCurve),
        Ordering::Less => Ok(ExactContactClass::Empty),
        Ordering::Greater => {
            if apex_on_sphere {
                Ok(ExactContactClass::TangentCrossing)
            } else {
                Ok(ExactContactClass::Regular)
            }
        }
    }
}

/// W1: exact classification of a canonical carrier pair (spec §5.2). Returns
/// the exact contact class, or a named [`WitnessTierRefusal`] for a pair
/// outside the M3 exact scope. Coincidence is decided first by
/// [`same_carrier`]; use [`quadric_pencil_with_provenance`] to include the W2
/// provenance route.
pub fn quadric_pencil(
    a: &ExactCarrier,
    b: &ExactCarrier,
) -> Result<ExactContactClass, WitnessTierRefusal> {
    if same_carrier(a, b)? {
        return Ok(ExactContactClass::Coincident);
    }
    match (a, b) {
        (ExactCarrier::Plane { normal: n1, .. }, ExactCarrier::Plane { normal: n2, .. }) => {
            plane_plane(n1, n2)
        }
        (
            ExactCarrier::Plane {
                normal: n,
                offset: d,
            },
            ExactCarrier::Sphere {
                center: c,
                radius_sq: r,
            },
        )
        | (
            ExactCarrier::Sphere {
                center: c,
                radius_sq: r,
            },
            ExactCarrier::Plane {
                normal: n,
                offset: d,
            },
        ) => plane_sphere(n, *d, c, *r),
        (
            ExactCarrier::Plane {
                normal: n,
                offset: d,
            },
            ExactCarrier::Cylinder {
                axis_point: p,
                axis_dir: v,
                radius_sq: r,
            },
        )
        | (
            ExactCarrier::Cylinder {
                axis_point: p,
                axis_dir: v,
                radius_sq: r,
            },
            ExactCarrier::Plane {
                normal: n,
                offset: d,
            },
        ) => plane_cylinder(n, *d, p, v, *r),
        (
            ExactCarrier::Plane {
                normal: n,
                offset: d,
            },
            ExactCarrier::Cone {
                apex: ap,
                axis_dir: v,
                tan_sq: k,
            },
        )
        | (
            ExactCarrier::Cone {
                apex: ap,
                axis_dir: v,
                tan_sq: k,
            },
            ExactCarrier::Plane {
                normal: n,
                offset: d,
            },
        ) => plane_cone(n, *d, ap, v, *k),
        (
            ExactCarrier::Sphere {
                center: c1,
                radius_sq: r1,
            },
            ExactCarrier::Sphere {
                center: c2,
                radius_sq: r2,
            },
        ) => sphere_sphere(c1, *r1, c2, *r2),
        (
            ExactCarrier::Cylinder {
                axis_point: p1,
                axis_dir: v1,
                radius_sq: r1,
            },
            ExactCarrier::Cylinder {
                axis_point: p2,
                axis_dir: v2,
                radius_sq: r2,
            },
        ) => cylinder_cylinder(p1, v1, *r1, p2, v2, *r2),
        (
            ExactCarrier::Sphere {
                center: c,
                radius_sq: r,
            },
            ExactCarrier::Cone {
                apex: ap,
                axis_dir: v,
                tan_sq: k,
            },
        )
        | (
            ExactCarrier::Cone {
                apex: ap,
                axis_dir: v,
                tan_sq: k,
            },
            ExactCarrier::Sphere {
                center: c,
                radius_sq: r,
            },
        ) => sphere_cone(c, *r, ap, v, *k),
        (
            ExactCarrier::Sphere {
                center: c,
                radius_sq: r,
            },
            ExactCarrier::Cylinder {
                axis_point: p,
                axis_dir: v,
                radius_sq: rc,
            },
        )
        | (
            ExactCarrier::Cylinder {
                axis_point: p,
                axis_dir: v,
                radius_sq: rc,
            },
            ExactCarrier::Sphere {
                center: c,
                radius_sq: r,
            },
        ) => sphere_cylinder(c, *r, p, v, *rc),
        _ => Err(WitnessTierRefusal::UnsupportedCarrierPair),
    }
}

/// The identity route of a carrier pair (spec §5.2 W2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarrierIdentity {
    /// The two carriers carry the same importer `SourceEntityId`.
    ProvenanceShared(SourceEntityId),
    /// The two carrier definitions are exactly equal over ℚ.
    DefinitionsEqual,
    /// Distinct carriers: fall through to the W1 classifier.
    Distinct,
}

/// W2: decide the exact identity of a carrier pair. The provenance fast path
/// is taken only on exact equality of the importer `SourceEntityId`; absent
/// provenance is not invented. Equality of the carrier definitions is the
/// exact-rational fallback.
pub fn carrier_identity(
    a: &ExactCarrier,
    b: &ExactCarrier,
    provenance: Option<(SourceEntityId, SourceEntityId)>,
) -> Result<CarrierIdentity, WitnessTierRefusal> {
    if let Some((pa, pb)) = provenance {
        if pa == pb {
            return Ok(CarrierIdentity::ProvenanceShared(pa));
        }
    }
    if same_carrier(a, b)? {
        Ok(CarrierIdentity::DefinitionsEqual)
    } else {
        Ok(CarrierIdentity::Distinct)
    }
}

/// W2 + W1: classify a carrier pair, taking the exact coincidence witness
/// (shared provenance or equal definitions) before the W1 pencil
/// classification.
pub fn quadric_pencil_with_provenance(
    a: &ExactCarrier,
    b: &ExactCarrier,
    provenance: Option<(SourceEntityId, SourceEntityId)>,
) -> Result<ExactContactClass, WitnessTierRefusal> {
    match carrier_identity(a, b, provenance)? {
        CarrierIdentity::ProvenanceShared(_) | CarrierIdentity::DefinitionsEqual => {
            Ok(ExactContactClass::Coincident)
        }
        CarrierIdentity::Distinct => quadric_pencil(a, b),
    }
}

/// An exact contact curve on a carrier: the certificate a fillet/blend
/// construction must EMIT (spec §5.2 W3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactContactCurve {
    /// The straight contact line `point + t·dir`.
    Line {
        /// A point on the line.
        point: [QCoeff; 3],
        /// The line direction (nonzero).
        dir: [QCoeff; 3],
    },
    /// A contact circle.
    Circle {
        /// The circle centre.
        center: [QCoeff; 3],
        /// The circle plane normal (nonzero).
        normal: [QCoeff; 3],
        /// The squared circle radius (strictly positive).
        radius_sq: QCoeff,
    },
}

impl ExactContactCurve {
    /// The line `point + t·dir` from integer data.
    pub fn line(point: [i128; 3], dir: [i128; 3]) -> Result<Self, WitnessTierRefusal> {
        let d = [q_int(dir[0]), q_int(dir[1]), q_int(dir[2])];
        if v_is_zero(&d) {
            return Err(WitnessTierRefusal::DegenerateCarrier);
        }
        Ok(ExactContactCurve::Line {
            point: [q_int(point[0]), q_int(point[1]), q_int(point[2])],
            dir: d,
        })
    }

    /// A circle from integer data.
    pub fn circle(
        center: [i128; 3],
        normal: [i128; 3],
        radius_sq: i128,
    ) -> Result<Self, WitnessTierRefusal> {
        let n = [q_int(normal[0]), q_int(normal[1]), q_int(normal[2])];
        if v_is_zero(&n) || radius_sq <= 0 {
            return Err(WitnessTierRefusal::DegenerateCarrier);
        }
        Ok(ExactContactCurve::Circle {
            center: [q_int(center[0]), q_int(center[1]), q_int(center[2])],
            normal: n,
            radius_sq: q_int(radius_sq),
        })
    }
}

/// W3: a construction contact certificate. A fillet/blend construction must
/// emit this exact contact-curve certificate — a bare flag is not
/// constructible, because [`ConstructionWitness::new`] verifies exactly that
/// the curve lies on its carrier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructionWitness {
    carrier: ExactCarrier,
    curve: ExactContactCurve,
}

impl ConstructionWitness {
    /// Build a construction witness, refusing
    /// ([`WitnessTierRefusal::CurveOffCarrier`]) a curve that does not lie
    /// exactly on the carrier. The construction's `tangent_curve` class is the
    /// only outcome.
    pub fn new(
        carrier: ExactCarrier,
        curve: ExactContactCurve,
    ) -> Result<Self, WitnessTierRefusal> {
        if !curve_lies_on_carrier(&carrier, &curve)? {
            return Err(WitnessTierRefusal::CurveOffCarrier);
        }
        Ok(Self { carrier, curve })
    }

    /// The carrier the construction curve lies on.
    pub fn carrier(&self) -> &ExactCarrier {
        &self.carrier
    }

    /// The exact contact curve, verbatim.
    pub fn curve(&self) -> &ExactContactCurve {
        &self.curve
    }

    /// The construction's exact class: always [`ExactContactClass::TangentCurve`].
    pub fn class(&self) -> ExactContactClass {
        ExactContactClass::TangentCurve
    }
}

/// Whether `curve` lies exactly on `carrier`.
fn curve_lies_on_carrier(
    carrier: &ExactCarrier,
    curve: &ExactContactCurve,
) -> Result<bool, WitnessTierRefusal> {
    match (carrier, curve) {
        (ExactCarrier::Plane { normal, offset }, ExactContactCurve::Line { point, dir }) => {
            Ok(v_dot(normal, dir)?.is_zero() && v_dot(normal, point)? == *offset)
        }
        (
            ExactCarrier::Cylinder {
                axis_point,
                axis_dir,
                radius_sq,
            },
            ExactContactCurve::Line { point, dir },
        ) => {
            if !v_is_zero(&v_cross(axis_dir, dir)?) {
                return Ok(false);
            }
            Ok(point_axis_dist_sq(point, axis_point, axis_dir)? == *radius_sq)
        }
        (
            ExactCarrier::Sphere { center, radius_sq },
            ExactContactCurve::Circle {
                center: cc,
                normal,
                radius_sq: rsq,
            },
        ) => {
            let axis = v_sub(cc, center)?;
            if !v_is_zero(&axis) && !v_is_zero(&v_cross(&axis, normal)?) {
                return Ok(false);
            }
            let d2 = v_dot(&axis, &axis)?;
            Ok(q_add(d2, *rsq)? == *radius_sq)
        }
        (
            ExactCarrier::Cylinder {
                axis_point,
                axis_dir,
                radius_sq,
            },
            ExactContactCurve::Circle {
                center,
                normal,
                radius_sq: rsq,
            },
        ) => {
            if !v_is_zero(&v_cross(axis_dir, normal)?)
                || !point_axis_dist_sq(center, axis_point, axis_dir)?.is_zero()
            {
                return Ok(false);
            }
            Ok(*rsq == *radius_sq)
        }
        _ => Err(WitnessTierRefusal::UnsupportedCarrierPair),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tangency::qpoly::QPoly;
    use crate::tangency::shapes::{ExactVanishingWitness, ExactWitnessVerifier, WitnessSide};

    /// The F5 A₂ hand witness with the multiplier triple `(s, a, w1)`.
    ///
    /// The F5 ground truth is `s = a = 1`, `q = u1`, reduced `f = u1²` on
    /// `G = 0`. In exact integer polynomial data over variables `u1 = x0`,
    /// `v1 = x1`: `q = u1`, `f = u1² + v1` (so on `G = 0`, i.e. `v1 = 0`,
    /// the reduced contact function is exactly `u1²`), `G1 = v1`, `G2 = 0`.
    /// The identity `s·f = q²·a + w1·G1 + w2·G2` holds exactly over ℚ iff
    /// the multipliers agree (`s·(u1² + v1) = a·u1² + w1·v1` requires
    /// `s = a = w1`).
    fn f5_witness(s: i128, a: i128, w1: i128) -> ExactVanishingWitness<QPoly> {
        let arity = 2;
        let u1 = QPoly::from_int_terms(arity, &[(&[1, 0], 1)]).expect("u1 is a valid term");
        let v1 = QPoly::from_int_terms(arity, &[(&[0, 1], 1)]).expect("v1 is a valid term");
        let s_poly = QPoly::from_int_terms(arity, &[(&[0, 0], s)]).expect("the s constant");
        let a_poly = QPoly::from_int_terms(arity, &[(&[0, 0], a)]).expect("the a constant");
        let w1_poly = QPoly::from_int_terms(arity, &[(&[0, 0], w1)]).expect("the w1 constant");
        let terms = vec![(w1_poly, v1), (QPoly::zero(arity), QPoly::zero(arity))];
        ExactVanishingWitness::new(WitnessSide::A2, s_poly, terms, Some((u1, a_poly)))
            .expect("the F5 A2 witness has the A2 term shape")
    }

    /// The system polynomials `f` and `P = (G1, G2)` of the F5 hand witness.
    fn f5_system() -> (QPoly, Vec<QPoly>) {
        let arity = 2;
        let u1_sq = QPoly::from_int_terms(arity, &[(&[2, 0], 1)]).expect("u1^2 is a valid term");
        let v1 = QPoly::from_int_terms(arity, &[(&[0, 1], 1)]).expect("v1 is a valid term");
        let f = u1_sq.add(&v1).expect("u1^2 + v1");
        let g1 = QPoly::from_int_terms(arity, &[(&[0, 1], 1)]).expect("v1 is a valid term");
        let g2 = QPoly::zero(arity);
        (f, vec![g1, g2])
    }

    #[test]
    fn verifier_accepts_f5_hand_witness() {
        let witness = f5_witness(1, 1, 1);
        let (f, p) = f5_system();
        let refs: Vec<&QPoly> = p.iter().collect();
        // s = a = 1: any strictly positive enclosure certifies ŝ and â.
        let result =
            ExactWitnessVerifier::verify(&witness, &f, &refs, Some((0.5, 1.5)), Some((0.5, 1.5)));
        assert!(
            result.is_ok(),
            "the F5 hand witness is an exact identity and must verify: {result:?}"
        );
    }

    #[test]
    fn verifier_rejects_perturbed_identity() {
        // Perturb the multiplier `a` of the F5 hand witness from the constant
        // 1 to the constant 2 while `s` and `w1` stay at 1: the identity
        // s·f = q²·a + Σ wᵢPᵢ no longer holds over ℚ
        // (u1² + v1 ≠ 2·u1² + v1), and the verifier must refuse.
        let perturbed = f5_witness(1, 2, 1);
        let (f, p) = f5_system();
        let refs: Vec<&QPoly> = p.iter().collect();

        let result =
            ExactWitnessVerifier::verify(&perturbed, &f, &refs, Some((0.5, 1.5)), Some((0.5, 1.5)));
        assert!(
            result.is_err(),
            "a witness perturbed by one coefficient must not verify"
        );
    }

    #[test]
    fn nonvanishing_multiplier_is_interval_certified() {
        let witness = f5_witness(1, 1, 1);
        let (f, p) = f5_system();
        let refs: Vec<&QPoly> = p.iter().collect();

        // ŝ containing 0 (interior or at an endpoint) refuses.
        let straddling =
            ExactWitnessVerifier::verify(&witness, &f, &refs, Some((-1.0, 1.0)), Some((0.5, 1.5)));
        assert!(straddling.is_err(), "ŝ straddling 0 must refuse");
        let endpoint_zero =
            ExactWitnessVerifier::verify(&witness, &f, &refs, Some((0.0, 1.5)), Some((0.5, 1.5)));
        assert!(
            endpoint_zero.is_err(),
            "ŝ with a 0 endpoint must refuse (strict exclusion)"
        );

        // â containing 0 refuses.
        let a_straddling =
            ExactWitnessVerifier::verify(&witness, &f, &refs, Some((0.5, 1.5)), Some((-1.0, 1.0)));
        assert!(a_straddling.is_err(), "â straddling 0 must refuse");

        // Strict exclusion accepts.
        let strict =
            ExactWitnessVerifier::verify(&witness, &f, &refs, Some((0.5, 1.5)), Some((0.5, 1.5)));
        assert!(strict.is_ok(), "strictly excluded multipliers must certify");

        // A strictly negative ŝ/â pair also certifies: the exact identity is
        // signed, so the all-negative witness (s = a = w1 = −1) verifies
        // against strictly negative enclosures.
        let negative = f5_witness(-1, -1, -1);
        let strict_negative = ExactWitnessVerifier::verify(
            &negative,
            &f,
            &refs,
            Some((-1.5, -0.5)),
            Some((-1.5, -0.5)),
        );
        assert!(
            strict_negative.is_ok(),
            "strictly negative multipliers also exclude 0 and certify"
        );

        // A missing mandatory enclosure refuses (it cannot certify).
        let missing_s = ExactWitnessVerifier::verify(&witness, &f, &refs, None, Some((0.5, 1.5)));
        assert!(missing_s.is_err(), "a missing ŝ must refuse");
        let missing_a = ExactWitnessVerifier::verify(&witness, &f, &refs, Some((0.5, 1.5)), None);
        assert!(
            missing_a.is_err(),
            "an A2 witness without â must refuse (q²a present)"
        );
    }

    #[test]
    fn provenance_producer_refuses_pending() {
        assert_eq!(
            WITNESS_PRODUCER_PACKET_PENDING,
            "witness_producer_packet_pending"
        );
        let arity = 2;
        let u1 = QPoly::from_int_terms(arity, &[(&[1, 0], 1)]).expect("u1 is a valid term");
        let refused = provenance_witness(
            WitnessSide::A2,
            QPoly::one(arity),
            vec![
                (QPoly::zero(arity), QPoly::zero(arity)),
                (QPoly::zero(arity), QPoly::zero(arity)),
            ],
            Some((u1, QPoly::one(arity))),
        );
        match refused {
            Err(WitnessRefusal::Pending(cause)) => {
                assert_eq!(cause, WITNESS_PRODUCER_PACKET_PENDING);
                assert_eq!(cause, "witness_producer_packet_pending");
            }
            other => panic!(
                "the provenance producer must refuse with the named pending cause, got {other:?}"
            ),
        }
    }
}
