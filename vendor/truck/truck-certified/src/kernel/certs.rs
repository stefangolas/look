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

//! The §8/§10.3/§11/§14.2 certificates (BG-KV2-000-CONTRACT).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **D-shim.** Refusing constructors only. The certificates below store the
//! frozen shapes; their numeric bodies are the wave packets' implementors. The
//! load-bearing §8.3 ban is typed: an [`ArcCert`] whose residual is `R2`
//! refuses with the exact packet spelling
//! `RefusalKind::Conditioning` + `VerdictClass::Inconclusive` + predicate
//! `R2_never_reaches_C2`.
//!
//! **§10.3 rule 7.** A tolerance-tagged contact claim never unifies with an
//! exact certificate: [`ContactCert`] is the Proven case only; the Disproven
//! (`0 ∉ gap`) and Inconclusive outcomes are `ClaimVerdict` arms owned by the
//! S5a packet, not this type.

use crate::kernel::config::{EPS_REP, RHO_MAX, TOL_INTERSECTION, TOL_JACOBIAN};
use crate::kernel::evidence::{Refusal, RefusalEvidence, RefusalKind, VerdictClass};
use crate::kernel::patch::{CertifiedNonzero, CertifiedPositive, IBox2};
use crate::kernel::residual::ResidualId;
use crate::kernel::{Interval, SignCert};

/// An orthonormal moving frame of the arc's `N`-dimensional state space
/// (§11): `q` is an `N x N` orthonormal matrix whose columns are basis
/// vectors, column 0 is the unit tangent `q_tau`, `z_hat` is the certified
/// normal direction, `q_perp` stores the column-wise complement of `q_tau` in
/// `q`, and `a` is the (finite) Jacobian matrix.
///
/// `q_perp` carries the `N - 1` perpendicular columns of `q` (columns `1..N`)
/// in its leading columns and re-stores `q_tau` as its final column so the
/// field keeps the square shape: `q_perp[i] == q[i + 1]` for `i < N - 1` and
/// `q_perp[N - 1] == q_tau == q[0]`.
///
/// Construct only through [`Frame::try_new`], which refuses non-finite data,
/// a non-unit `z_hat` or `q_tau`, non-orthonormal `q` columns, a `q_perp` that
/// is not the column-wise complement of `q_tau` in `q` (all within
/// [`TOL_JACOBIAN`]), and a non-finite `a`. `N` is `2..=7` in practice; for
/// `N = 1` nothing is checked beyond finiteness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame<const N: usize> {
    /// The certified normal direction (unit).
    pub z_hat: [f64; N],
    /// The `N x N` orthonormal frame matrix; column 0 is `q_tau`.
    pub q: [[f64; N]; N],
    /// The unit tangent, equal to column 0 of `q`.
    pub q_tau: [f64; N],
    /// The column-wise complement of `q_tau` in `q`.
    pub q_perp: [[f64; N]; N],
    /// The Jacobian matrix (finite).
    pub a: [[f64; N]; N],
}

/// A certified point of a sheet (§8.3): the residual that certified it, the
/// parameter box the certificate ran over, and the contraction rate `rho`.
///
/// Construct only through [`PointCert::try_new`], which refuses a
/// `rho > RHO_MAX` (Lemma 8.0's contraction acceptance).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointCert {
    /// The residual that certified the point.
    pub residual: ResidualId,
    /// The parameter box the certificate ran over.
    pub box_: IBox2,
    /// Lemma 8.0's contraction rate.
    pub rho: f64,
}

/// A tube certificate for one arc segment (§8): the residual, the segment's
/// frame, the tangent-parameter interval `i_tau`, the perpendicular bound box,
/// the contraction rate `rho`, per-column Jacobian enclosures, and the
/// optional positive per-column weights.
///
/// Construct only through [`ArcCert::try_new`], which refuses `rho > RHO_MAX`,
/// a `R2` residual (§8.3 — R2 is never an instance of the tube certificate),
/// an empty `jac_encl`, and `weights: Some(v)` with an empty `v`.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcCert<const N: usize> {
    /// The residual the tube certificate runs over.
    pub residual: ResidualId,
    /// The segment's orthonormal frame.
    pub frame: Frame<N>,
    /// The certified tangent-parameter interval.
    pub i_tau: Interval,
    /// The perpendicular bound box (at the frame dimension `N`).
    pub b_perp: crate::kernel::patch::IBox<N>,
    /// Lemma 8.0's contraction rate.
    pub rho: f64,
    /// Per-column Jacobian enclosures; must be non-empty.
    pub jac_encl: Vec<[f64; 2]>,
    /// Optional per-column positive weights; `Some(v)` must be non-empty.
    pub weights: Option<Vec<CertifiedPositive>>,
}

/// An at-tolerance contact certificate (§10.3): the certified critical point,
/// the certified gap interval, the tolerance tag, and the certified Hessian
/// sign that isolates the tangency.
///
/// Construct only through [`ContactCert::try_new`], which derives the
/// tolerance from [`TOL_INTERSECTION`] and refuses unless `0 ∈ gap` AND
/// `width(gap) <= tolerance` — the Proven case ONLY (rule 7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContactCert {
    /// The certified critical point of the gap function.
    pub critical_point: PointCert,
    /// The certified enclosure of the gap (signed distance).
    pub gap: Interval,
    /// The tolerance tag of the claim ([`TOL_INTERSECTION`]).
    pub tolerance: f64,
    /// The certified Hessian sign at the critical point.
    pub hessian_sign: SignCert,
}

/// A graph certificate (§14.2): the domain box, the unit normal `n0`, and the
/// certified-nonzero determinant bound.
///
/// Construct only through [`GraphCert::try_new`], which refuses a non-unit
/// `n0` (unit slack [`TOL_JACOBIAN`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphCert {
    /// The graph domain box.
    pub domain: IBox2,
    /// The unit normal at the reference point.
    pub n0: [f64; 3],
    /// The certified-nonzero determinant bound.
    pub det_bound: CertifiedNonzero,
}

/// An R5 enclosure (§14): a box `q` whose certified preimages and certified
/// points close the R5 certificate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct R5Enclosure {
    /// The box the R5 certificate ran over.
    pub q: IBox2,
    /// The two certified preimage boxes.
    pub preimage: [IBox2; 2],
    /// The two certified points.
    pub cert: [PointCert; 2],
}

/// The recorded spelling deviation for §16's `psi: PsiMap` (§6): the kind of
/// the parameter map is frozen NOW so `Sheet` is stable; S6's real map type
/// arrives with its wave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PsiMapKind {
    /// The identity correspondence.
    Identity,
    /// An affine correspondence.
    Affine,
    /// A bilinear correspondence.
    Bilinear,
    /// A recognized carrier correspondence.
    RecognizedCarrier,
}

/// A sheet certificate: the domain box, the parameter-map kind, and the
/// certified-nonzero determinant of the map.
///
/// Construct only through [`SheetCert::try_new`], which refuses a degenerate
/// (`0`) map determinant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetCert {
    /// The sheet domain box.
    pub domain: IBox2,
    /// The parameter-map kind.
    pub psi_kind: PsiMapKind,
    /// The certified-nonzero determinant of the parameter map.
    pub det_dpsi: CertifiedNonzero,
}

/// A tube-overlap certificate (§14): a shared point and a certified C1 bound
/// at or below [`EPS_REP`].
///
/// Construct only through [`TubeOverlapCert::try_new`], which refuses a
/// `c1_bound > EPS_REP` and any non-finite data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TubeOverlapCert {
    /// A point shared by the two arcs.
    pub shared_point: [f64; 3],
    /// The certified C1 bound at the shared point.
    pub c1_bound: f64,
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl<const N: usize> Frame<N> {
    /// Build a frame, validating orthonormality and the complement relation.
    pub fn try_new(
        z_hat: [f64; N],
        q: [[f64; N]; N],
        q_tau: [f64; N],
        q_perp: [[f64; N]; N],
        a: [[f64; N]; N],
    ) -> Result<Self, Refusal> {
        if !all_finite_frame(z_hat, &q, q_tau, &q_perp, &a) {
            return Err(refusal(
                RefusalKind::NonFinite,
                "frame_data_not_finite",
                "frame data is not finite".to_string(),
            ));
        }
        if N == 1 {
            return Ok(Self {
                z_hat,
                q,
                q_tau,
                q_perp,
                a,
            });
        }
        if !is_unit_vector(&z_hat, TOL_JACOBIAN) {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "frame_z_hat_not_unit",
                format!("z_hat {z_hat:?} is not unit to {TOL_JACOBIAN}"),
            ));
        }
        if !is_unit_vector(&q_tau, TOL_JACOBIAN) {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "frame_q_tau_not_unit",
                format!("q_tau {q_tau:?} is not unit to {TOL_JACOBIAN}"),
            ));
        }
        for c in 0..N {
            if !is_unit_vector(&q[c], TOL_JACOBIAN) {
                return Err(refusal(
                    RefusalKind::ClaimRefuted,
                    "frame_q_column_not_unit",
                    format!("q column {c} is not unit to {TOL_JACOBIAN}"),
                ));
            }
            for d in (c + 1)..N {
                if dot_vector(&q[c], &q[d]).abs() > TOL_JACOBIAN {
                    return Err(refusal(
                        RefusalKind::ClaimRefuted,
                        "frame_q_columns_not_orthogonal",
                        format!(
                            "q columns {c} and {d} are not orthogonal (dot {})",
                            dot_vector(&q[c], &q[d])
                        ),
                    ));
                }
            }
        }
        if !vector_close(&q_tau, &q[0], TOL_JACOBIAN) {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "frame_q_tau_not_column_zero_of_q",
                "q_tau must equal column 0 of q".to_string(),
            ));
        }
        for i in 0..(N - 1) {
            if !vector_close(&q_perp[i], &q[i + 1], TOL_JACOBIAN) {
                return Err(refusal(
                    RefusalKind::ClaimRefuted,
                    "frame_q_perp_not_complement_of_q_tau",
                    format!("q_perp column {i} is not q column {}", i + 1),
                ));
            }
        }
        if !vector_close(&q_perp[N - 1], &q_tau, TOL_JACOBIAN) {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "frame_q_perp_not_complement_of_q_tau",
                "q_perp final column must re-store q_tau".to_string(),
            ));
        }
        Ok(Self {
            z_hat,
            q,
            q_tau,
            q_perp,
            a,
        })
    }
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl PointCert {
    /// Build a point certificate, refusing a `rho > RHO_MAX` (Lemma 8.0's
    /// contraction acceptance) or a non-finite `rho`.
    pub fn try_new(residual: ResidualId, box_: IBox2, rho: f64) -> Result<Self, Refusal> {
        if !rho.is_finite() {
            return Err(refusal(
                RefusalKind::NonFinite,
                "point_rho_not_finite",
                format!("rho {rho} is not finite"),
            ));
        }
        if rho > RHO_MAX {
            return Err(refusal(
                RefusalKind::Conditioning,
                "point_rho_exceeds_rho_max",
                format!("rho {rho} exceeds RHO_MAX {RHO_MAX}"),
            ));
        }
        Ok(Self {
            residual,
            box_,
            rho,
        })
    }
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl<const N: usize> ArcCert<N> {
    /// Build a tube certificate. Refuses a `rho > RHO_MAX`, a `R2` residual
    /// (§8.3), an empty `jac_encl`, and `weights: Some(v)` with an empty `v`.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        residual: ResidualId,
        frame: Frame<N>,
        i_tau: Interval,
        b_perp: crate::kernel::patch::IBox<N>,
        rho: f64,
        jac_encl: Vec<[f64; 2]>,
        weights: Option<Vec<CertifiedPositive>>,
    ) -> Result<Self, Refusal> {
        if residual == ResidualId::R2 {
            return Err(Refusal::with_backing(
                RefusalKind::Conditioning,
                VerdictClass::Inconclusive,
                RefusalEvidence::Predicate {
                    name: "R2_never_reaches_C2",
                    detail: "R2 is never an instance of the tube certificate (§8.3)".to_string(),
                },
            ));
        }
        if !rho.is_finite() {
            return Err(refusal(
                RefusalKind::NonFinite,
                "arc_rho_not_finite",
                format!("rho {rho} is not finite"),
            ));
        }
        if rho > RHO_MAX {
            return Err(refusal(
                RefusalKind::Conditioning,
                "arc_rho_exceeds_rho_max",
                format!("rho {rho} exceeds RHO_MAX {RHO_MAX}"),
            ));
        }
        if jac_encl.is_empty() {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "arc_jac_encl_empty",
                "jac_encl must not be empty".to_string(),
            ));
        }
        if let Some(weights) = &weights {
            if weights.is_empty() {
                return Err(refusal(
                    RefusalKind::ClaimRefuted,
                    "arc_weights_empty",
                    "weights, when present, must not be empty".to_string(),
                ));
            }
        }
        Ok(Self {
            residual,
            frame,
            i_tau,
            b_perp,
            rho,
            jac_encl,
            weights,
        })
    }
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl ContactCert {
    /// Build the Proven-case contact certificate (§10.3). Refuses unless
    /// `0 ∈ gap` AND `width(gap) <= tolerance`, with the tolerance derived from
    /// [`TOL_INTERSECTION`].
    pub fn try_new(
        critical_point: PointCert,
        gap: Interval,
        hessian_sign: SignCert,
    ) -> Result<Self, Refusal> {
        if !gap.contains(0.0) {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "contact_gap_excludes_zero",
                format!("gap {gap:?} does not contain 0"),
            ));
        }
        if gap.width() > TOL_INTERSECTION {
            return Err(refusal(
                RefusalKind::R5EnclosureFailed,
                "contact_gap_width_exceeds_tolerance",
                format!(
                    "gap width {} exceeds tolerance {TOL_INTERSECTION}",
                    gap.width()
                ),
            ));
        }
        Ok(Self {
            critical_point,
            gap,
            tolerance: TOL_INTERSECTION,
            hessian_sign,
        })
    }
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl GraphCert {
    /// Build a graph certificate, refusing a non-unit `n0` (unit slack
    /// [`TOL_JACOBIAN`]).
    pub fn try_new(domain: IBox2, n0: [f64; 3], det: f64) -> Result<Self, Refusal> {
        if !n0.iter().all(|c| c.is_finite()) {
            return Err(refusal(
                RefusalKind::NonFinite,
                "graph_cert_n0_not_finite",
                format!("n0 {n0:?} is not finite"),
            ));
        }
        let norm = (n0[0] * n0[0] + n0[1] * n0[1] + n0[2] * n0[2]).sqrt();
        if (norm - 1.0).abs() > TOL_JACOBIAN {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "graph_cert_n0_not_unit",
                format!("n0 {n0:?} has norm {norm}, not unit to {TOL_JACOBIAN}"),
            ));
        }
        let det_bound = CertifiedNonzero::try_new(det)?;
        Ok(Self {
            domain,
            n0,
            det_bound,
        })
    }
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl SheetCert {
    /// Build a sheet certificate, refusing a degenerate (`0`) map determinant.
    pub fn try_new(domain: IBox2, psi_kind: PsiMapKind, det: f64) -> Result<Self, Refusal> {
        let det_dpsi = CertifiedNonzero::try_new(det)?;
        Ok(Self {
            domain,
            psi_kind,
            det_dpsi,
        })
    }
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl TubeOverlapCert {
    /// Build a tube-overlap certificate, refusing a `c1_bound > EPS_REP` and
    /// any non-finite data.
    pub fn try_new(shared_point: [f64; 3], c1_bound: f64) -> Result<Self, Refusal> {
        if !shared_point.iter().all(|c| c.is_finite()) {
            return Err(refusal(
                RefusalKind::NonFinite,
                "tube_overlap_shared_point_not_finite",
                format!("shared_point {shared_point:?} is not finite"),
            ));
        }
        if !c1_bound.is_finite() {
            return Err(refusal(
                RefusalKind::NonFinite,
                "tube_overlap_c1_bound_not_finite",
                format!("c1_bound {c1_bound} is not finite"),
            ));
        }
        if c1_bound < 0.0 {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "tube_overlap_c1_bound_negative",
                format!("c1_bound {c1_bound} is negative"),
            ));
        }
        if c1_bound > EPS_REP {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "tube_overlap_c1_bound_exceeds_eps_rep",
                format!("c1_bound {c1_bound} exceeds EPS_REP {EPS_REP}"),
            ));
        }
        Ok(Self {
            shared_point,
            c1_bound,
        })
    }
}

fn dot_vector(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn is_unit_vector(v: &[f64], tol: f64) -> bool {
    let norm = dot_vector(v, v).sqrt();
    (norm - 1.0).abs() <= tol
}

fn vector_close(a: &[f64], b: &[f64], tol: f64) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() <= tol)
}

fn all_finite_frame<const N: usize>(
    z_hat: [f64; N],
    q: &[[f64; N]; N],
    q_tau: [f64; N],
    q_perp: &[[f64; N]; N],
    a: &[[f64; N]; N],
) -> bool {
    z_hat.iter().all(|c| c.is_finite())
        && q_tau.iter().all(|c| c.is_finite())
        && q.iter().flatten().all(|c| c.is_finite())
        && q_perp.iter().flatten().all(|c| c.is_finite())
        && a.iter().flatten().all(|c| c.is_finite())
}

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}

/// The arity-3 zero-dimensional certificate (R8-class C1; the recorded
/// additive spelling for the spec's n-generic `PointCert`, §8.2/§16): the
/// residual that certified it, the 3D R8 domain box it ran over, and the
/// contraction rate `rho`.
///
/// This type is ADDITIVE: the frozen [`PointCert`] (whose `box_` is an
/// `IBox2`) is untouched, and the spec's `PointCert { box_: IBox }` is
/// spelled arity-specifically at n = 2 (`PointCert`) and n = 3 (`PointCert3`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointCert3 {
    /// The residual that certified the point.
    pub residual: ResidualId,
    /// The 3D box the certificate ran over.
    pub box_: crate::kernel::patch::IBox3,
    /// Lemma 8.0's contraction rate.
    pub rho: f64,
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl PointCert3 {
    /// Build an arity-3 point certificate, refusing a `rho > RHO_MAX` (Lemma
    /// 8.0's contraction acceptance), a non-finite `rho`, or a non-finite box
    /// (the same gate as [`PointCert::try_new`], plus the finite-box gate the
    /// R8 entry requires).
    pub fn try_new(
        residual: ResidualId,
        box_: crate::kernel::patch::IBox3,
        rho: f64,
    ) -> Result<Self, Refusal> {
        if !rho.is_finite() {
            return Err(refusal(
                RefusalKind::NonFinite,
                "point3_rho_not_finite",
                format!("rho {rho} is not finite"),
            ));
        }
        if rho > RHO_MAX {
            return Err(refusal(
                RefusalKind::Conditioning,
                "point3_rho_exceeds_rho_max",
                format!("rho {rho} exceeds RHO_MAX {RHO_MAX}"),
            ));
        }
        if !box_.lo.iter().chain(box_.hi.iter()).all(|c| c.is_finite()) {
            return Err(refusal(
                RefusalKind::NonFinite,
                "point3_box_not_finite",
                format!("box {box_:?} is not finite"),
            ));
        }
        Ok(Self {
            residual,
            box_,
            rho,
        })
    }
}
