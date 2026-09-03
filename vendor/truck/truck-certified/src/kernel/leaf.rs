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

//! The §3.2 leaf shapes: rational Bézier leaves and rational carriers
//! (BG-KV2-000-CONTRACT).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **D-shim.** Type shapes only — no extraction, no evaluation, no
//! dehomogenization. Carriers are rational per §3.2/N4: the quadrics (sphere,
//! cylinder, cone) carry an explicit rational half-angle where applicable. A
//! transcendental-only carrier is out of the shim's vocabulary:
//! [`RefusalKind::TranscendentalCarrier`] is constructible by callers.
//!
//! **Positive control weights.** [`BezierLeaf::try_new`] enforces strictly
//! positive homogeneous control weights as the constructor-level precondition;
//! the per-box `weight_bound` certificate is derived later by the implementor
//! wave (the §7.4 fixture pins the straddle case).

use crate::kernel::config::EPS_REP;
use crate::kernel::evidence::{Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::patch::IBox2;

/// A rational Bézier surface leaf (spec §3.2, N5): the homogeneous `xyzw`
/// control net over the integer grid `(degree_u + 1) x (degree_v + 1)`.
///
/// Construct only through [`BezierLeaf::try_new`], which refuses a control
/// count that does not match the degrees, a zero degree, non-finite
/// coordinates, and a non-positive control weight.
#[derive(Debug, Clone, PartialEq)]
pub struct BezierLeaf {
    /// The degree in `u`.
    pub degree_u: usize,
    /// The degree in `v`.
    pub degree_v: usize,
    /// The homogeneous `xyzw` control points, row-major over `(u, v)`.
    pub control: Vec<[f64; 4]>,
}

/// The rational carrier family (spec §3.2/N4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RationalCarrierKind {
    /// A planar carrier.
    Plane,
    /// A spherical carrier.
    Sphere,
    /// A cylindrical carrier.
    Cylinder,
    /// A conical carrier.
    Cone,
    /// A toroidal carrier.
    Torus,
}

/// A rational carrier: a rational surface of a recognized family plus the
/// parameter domain it is certified over.
///
/// Construct only through [`RationalCarrier::try_new`], which refuses
/// non-finite data, non-positive radii, non-unit axes, degenerate `u`/`v`
/// directions, and any half-angle outside `(0, PI)`.
#[derive(Debug, Clone, PartialEq)]
pub struct RationalCarrier {
    /// Which rational family this carrier is.
    pub kind: RationalCarrierKind,
    /// The family-specific carrier data.
    pub data: CarrierData,
    /// The parameter domain the carrier is used over.
    pub domain: IBox2,
}

/// The family-specific carrier data (spec §3.2/N4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CarrierData {
    /// A plane through `origin` spanned by `u_dir`, `v_dir`.
    Plane {
        /// A point on the plane.
        origin: [f64; 3],
        /// The `u` direction.
        u_dir: [f64; 3],
        /// The `v` direction.
        v_dir: [f64; 3],
    },
    /// A sphere with the given center and radius.
    Sphere {
        /// The sphere center.
        center: [f64; 3],
        /// The sphere radius.
        radius: f64,
    },
    /// A cylinder with axis through `origin`, of `radius`, over the axial
    /// `height` interval `(lo, hi)`.
    Cylinder {
        /// A point on the cylinder axis.
        origin: [f64; 3],
        /// The unit cylinder axis.
        axis: [f64; 3],
        /// The cylinder radius.
        radius: f64,
        /// The axial extent `(lo, hi)` along the axis.
        height: (f64, f64),
    },
    /// A cone with `apex` and unit `axis`, rational half-angle and axial
    /// `height` interval `(lo, hi)`.
    Cone {
        /// The cone apex.
        apex: [f64; 3],
        /// The unit cone axis.
        axis: [f64; 3],
        /// The cone half-angle, in `(0, PI)`.
        half_angle: f64,
        /// The axial extent `(lo, hi)` along the axis.
        height: (f64, f64),
    },
    /// A torus with `center`, unit `axis`, major radius `major_r` and minor
    /// radius `minor_r`.
    Torus {
        /// The torus center.
        center: [f64; 3],
        /// The unit torus axis.
        axis: [f64; 3],
        /// The major radius.
        major_r: f64,
        /// The minor radius.
        minor_r: f64,
    },
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl BezierLeaf {
    /// Build a leaf, refusing a mismatched control count, a zero degree,
    /// non-finite coordinates, or a non-positive control weight.
    pub fn try_new(
        degree_u: usize,
        degree_v: usize,
        control: Vec<[f64; 4]>,
    ) -> Result<Self, Refusal> {
        if degree_u == 0 || degree_v == 0 {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "bezier_zero_degree",
                format!("leaf degrees ({degree_u}, {degree_v}) must be positive"),
            ));
        }
        let expected = (degree_u + 1) * (degree_v + 1);
        if control.len() != expected {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "bezier_control_count_mismatch",
                format!(
                    "control net has {} points, degrees ({degree_u}, {degree_v}) require {expected}",
                    control.len()
                ),
            ));
        }
        for (i, p) in control.iter().enumerate() {
            for c in p {
                if !c.is_finite() {
                    return Err(refusal(
                        RefusalKind::NonFinite,
                        "bezier_coordinate_not_finite",
                        format!("control point {i} has a non-finite coordinate: {p:?}"),
                    ));
                }
            }
            if p[3] <= 0.0 {
                return Err(refusal(
                    RefusalKind::WeightDegenerate,
                    "bezier_control_weight_not_positive",
                    format!("control point {i} has weight {} which is not > 0", p[3]),
                ));
            }
        }
        Ok(Self {
            degree_u,
            degree_v,
            control,
        })
    }
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl RationalCarrier {
    /// Build a rational carrier, validating the family-specific data.
    pub fn try_new(
        kind: RationalCarrierKind,
        data: CarrierData,
        domain: IBox2,
    ) -> Result<Self, Refusal> {
        validate_data(&data)?;
        Ok(Self { kind, data, domain })
    }
}

#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
fn validate_data(data: &CarrierData) -> Result<(), Refusal> {
    match *data {
        CarrierData::Plane {
            origin,
            u_dir,
            v_dir,
        } => {
            require_finite3("plane_origin", origin)?;
            require_direction("plane_u_dir", u_dir)?;
            require_direction("plane_v_dir", v_dir)?;
        }
        CarrierData::Sphere { center, radius } => {
            require_finite3("sphere_center", center)?;
            require_positive("sphere_radius", radius)?;
        }
        CarrierData::Cylinder {
            origin,
            axis,
            radius,
            height,
        } => {
            require_finite3("cylinder_origin", origin)?;
            require_unit_axis("cylinder_axis", axis)?;
            require_positive("cylinder_radius", radius)?;
            require_height("cylinder_height", height)?;
        }
        CarrierData::Cone {
            apex,
            axis,
            half_angle,
            height,
        } => {
            require_finite3("cone_apex", apex)?;
            require_unit_axis("cone_axis", axis)?;
            if !half_angle.is_finite() || !(0.0..std::f64::consts::PI).contains(&half_angle) {
                return Err(refusal(
                    RefusalKind::ClaimRefuted,
                    "carrier_cone_half_angle_out_of_range",
                    format!("carrier cone half-angle {half_angle} outside (0, PI)"),
                ));
            }
            require_height("cone_height", height)?;
        }
        CarrierData::Torus {
            center,
            axis,
            major_r,
            minor_r,
        } => {
            require_finite3("torus_center", center)?;
            require_unit_axis("torus_axis", axis)?;
            require_positive("torus_major_radius", major_r)?;
            require_positive("torus_minor_radius", minor_r)?;
        }
    }
    Ok(())
}

#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
fn require_finite3(what: &'static str, v: [f64; 3]) -> Result<(), Refusal> {
    if !v.iter().all(|c| c.is_finite()) {
        return Err(refusal(
            RefusalKind::NonFinite,
            "carrier_coordinate_not_finite",
            format!("{what} {v:?} is not finite"),
        ));
    }
    Ok(())
}

#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
fn require_direction(what: &'static str, v: [f64; 3]) -> Result<(), Refusal> {
    require_finite3(what, v)?;
    let norm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if norm <= EPS_REP {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "carrier_direction_degenerate",
            format!("{what} {v:?} is degenerate (norm {norm})"),
        ));
    }
    Ok(())
}

#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
fn require_unit_axis(what: &'static str, v: [f64; 3]) -> Result<(), Refusal> {
    require_finite3(what, v)?;
    let norm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if (norm - 1.0).abs() > EPS_REP {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "carrier_axis_not_unit",
            format!("{what} {v:?} has norm {norm}, not unit to {EPS_REP}"),
        ));
    }
    Ok(())
}

#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
fn require_positive(what: &'static str, value: f64) -> Result<(), Refusal> {
    if !value.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "carrier_radius_not_finite",
            format!("{what} {value} is not finite"),
        ));
    }
    if value <= 0.0 {
        return Err(refusal(
            RefusalKind::WeightDegenerate,
            "carrier_radius_not_positive",
            format!("{what} {value} is not > 0"),
        ));
    }
    Ok(())
}

#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
fn require_height(what: &'static str, height: (f64, f64)) -> Result<(), Refusal> {
    if !height.0.is_finite() || !height.1.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "carrier_height_not_finite",
            format!("{what} {height:?} is not finite"),
        ));
    }
    if height.0 > height.1 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "carrier_height_inverted",
            format!("{what} {height:?} is inverted"),
        ));
    }
    Ok(())
}

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}
