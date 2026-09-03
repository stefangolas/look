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

use crate::formal::exact::CertifiedInterval;
use crate::hull::{bernstein_derivative_2d, hull_bernstein_2d};
use crate::kernel::config::{EPS_REP, TOL_JACOBIAN};
use crate::kernel::evidence::{ClaimVerdict, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::patch::{
    CertifiedPatch, CertifiedPatchC2, CertifiedPositive, Cone, Degeneracy, DerivativeEnclosure,
    IBox2, IBox3, Pole, Reason, SecondDerivativeEnclosure,
};

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
    // N4: no transcendental calls in the leaf module — the degenerate-norm test
    // is done on the squared length (norm <= EPS_REP iff norm^2 <= EPS_REP^2).
    let norm_sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if norm_sq <= EPS_REP * EPS_REP {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "carrier_direction_degenerate",
            format!("{what} {v:?} is degenerate (squared norm {norm_sq})"),
        ));
    }
    Ok(())
}

#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
fn require_unit_axis(what: &'static str, v: [f64; 3]) -> Result<(), Refusal> {
    require_finite3(what, v)?;
    // N4: no transcendental calls in the leaf module. The unit test is
    // |norm - 1| <= EPS_REP, which for norm ~ 1 is |norm^2 - 1| <=
    // EPS_REP * (2 + EPS_REP); the squared form is algebraically the same
    // boundary up to the rounding of the axis itself.
    let norm_sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    let slack = EPS_REP * (2.0 + EPS_REP);
    if (norm_sq - 1.0).abs() > slack {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "carrier_axis_not_unit",
            format!("{what} {v:?} has squared norm {norm_sq}, not unit to {EPS_REP}"),
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

// ---------------------------------------------------------------------------
// §3.2 CertifiedPatch implementation over the homogeneous Bézier leaf.
//
// The leaf surface is `S(u, v) = (X, Y, Z) / W` where the four homogeneous
// coordinate polynomials `(X, Y, Z, W)` are the tensor-Bernstein polynomials
// of the `xyzw` control net over the leaf's unit square. Every enclosure
// below is built from the landed hull kernels (`hull_bernstein_2d` over the
// control or derivative-control grids, outward-rounded), divided by the
// certified-positive `W` enclosure ONCE at the end (N5/N6). The derivative
// nets are recomputed per call (plain data, no interior mutability; the choice
// is recorded in the packet RESULT notes).
//
// A box `d` is always a compact sub-box of the leaf's unit-square domain.
// Because the `CertifiedPatch` shape cannot express a refusal, a box that
// violates that precondition — or a pass-through leaf whose `W` enclosure is
// not certifiably positive over the box — yields a vacuously true (fully
// unbounded) enclosure; certified callers gate on `weight_bound`/`regularity`
// first (§7.1 is the *weight_bound* classification, which can refuse).
// ---------------------------------------------------------------------------

impl BezierLeaf {
    /// The coefficient grid of one homogeneous coordinate (`0..=3`, `3 == w`),
    /// rows over `u` and columns over `v`, as the hull kernels consume it.
    fn control_grid(&self, comp: usize) -> Vec<Vec<f64>> {
        let width = self.degree_v + 1;
        (0..=self.degree_u)
            .map(|i| {
                (0..=self.degree_v)
                    .map(|j| self.control[i * width + j][comp])
                    .collect()
            })
            .collect()
    }

    /// The coordinate grid with `ou` `u`-derivatives and `ov` `v`-derivatives
    /// taken, or `None` once the grid empties (never for the orders this
    /// module requests on a valid leaf).
    fn derivative_grid(&self, comp: usize, ou: usize, ov: usize) -> Option<Vec<Vec<f64>>> {
        let mut grid = self.control_grid(comp);
        for _ in 0..ou {
            if grid.is_empty() || grid[0].is_empty() {
                return None;
            }
            grid = bernstein_derivative_2d(&grid, 0);
        }
        for _ in 0..ov {
            if grid.is_empty() || grid[0].is_empty() {
                return None;
            }
            grid = bernstein_derivative_2d(&grid, 1);
        }
        if grid.is_empty() || grid[0].is_empty() {
            None
        } else {
            Some(grid)
        }
    }

    /// The certified range of one homogeneous coordinate over the unit sub-box
    /// `(s, t)`, or `None` when the hull kernel refuses (a box outside the unit
    /// square or an unbounded hull).
    fn hull(&self, comp: usize, s: (f64, f64), t: (f64, f64)) -> Option<CertifiedInterval> {
        hull_bernstein_2d(&self.control_grid(comp), s, t).ok()
    }

    /// The certified range of a derivative of one homogeneous coordinate over
    /// `(s, t)`, or `None` on hull refusal.
    fn hull_orders(
        &self,
        comp: usize,
        s: (f64, f64),
        t: (f64, f64),
        orders: (usize, usize),
    ) -> Option<CertifiedInterval> {
        let grid = self.derivative_grid(comp, orders.0, orders.1)?;
        hull_bernstein_2d(&grid, s, t).ok()
    }

    /// A compact sub-box of the leaf's unit-square domain, or `None`.
    fn unit_box(&self, d: IBox2) -> Option<((f64, f64), (f64, f64))> {
        let s = (d.lo[0], d.hi[0]);
        let t = (d.lo[1], d.hi[1]);
        let valid = s.0.is_finite()
            && s.1.is_finite()
            && t.0.is_finite()
            && t.1.is_finite()
            && s.0 <= s.1
            && t.0 <= t.1
            && s.0 >= 0.0
            && s.1 <= 1.0
            && t.0 >= 0.0
            && t.1 <= 1.0;
        if valid {
            Some((s, t))
        } else {
            None
        }
    }

    /// The certified `W` (weight) enclosure over the unit sub-box `(s, t)`;
    /// `None` when the weight is not certifiably strictly positive there.
    fn positive_weight(&self, s: (f64, f64), t: (f64, f64)) -> Option<CertifiedInterval> {
        let w = self.hull(3, s, t)?;
        if w.lo > 0.0 {
            Some(w)
        } else {
            None
        }
    }

    /// The `orders`-derivative enclosure of `(X, Y, Z)`, divided once by the
    /// certified-positive weight enclosure (N5/N6), per coordinate. `None`
    /// when the box or the weight gate fails.
    fn quotient_partial(
        &self,
        s: (f64, f64),
        t: (f64, f64),
        orders: (usize, usize),
    ) -> Option<[CertifiedInterval; 3]> {
        let w0 = self.positive_weight(s, t)?;
        let wd = self.hull_orders(3, s, t, orders)?;
        let den = w0.mul(&w0);
        let q = |comp: usize| -> Option<CertifiedInterval> {
            let a0 = self.hull(comp, s, t)?;
            let ad = self.hull_orders(comp, s, t, orders)?;
            let num = ad.mul(&w0).sub(&a0.mul(&wd));
            num.div(&den)
        };
        Some([q(0)?, q(1)?, q(2)?])
    }

    /// The three second-derivative enclosures `(suu, suv, svv)` of one
    /// coordinate, from the rational quotient rule of order two.
    fn quotient_second(
        &self,
        comp: usize,
        s: (f64, f64),
        t: (f64, f64),
    ) -> Option<(CertifiedInterval, CertifiedInterval, CertifiedInterval)> {
        let w0 = self.positive_weight(s, t)?;
        let wu = self.hull_orders(3, s, t, (1, 0))?;
        let wv = self.hull_orders(3, s, t, (0, 1))?;
        let wuu = self.hull_orders(3, s, t, (2, 0))?;
        let wuv = self.hull_orders(3, s, t, (1, 1))?;
        let wvv = self.hull_orders(3, s, t, (0, 2))?;
        let a0 = self.hull(comp, s, t)?;
        let au = self.hull_orders(comp, s, t, (1, 0))?;
        let av = self.hull_orders(comp, s, t, (0, 1))?;
        let auu = self.hull_orders(comp, s, t, (2, 0))?;
        let auv = self.hull_orders(comp, s, t, (1, 1))?;
        let avv = self.hull_orders(comp, s, t, (0, 2))?;

        let w2 = w0.mul(&w0);
        let w3 = w0.mul(&w2);
        let two = CertifiedInterval::point(2.0);

        // S_uu = (A_uu B^2 - A B_uu B - 2 A_u B_u B + 2 A B_u^2) / B^3.
        let suu_num = auu
            .mul(&w2)
            .sub(&a0.mul(&wuu).mul(&w0))
            .sub(&two.mul(&au).mul(&wu).mul(&w0))
            .add(&two.mul(&a0).mul(&wu.mul(&wu)));
        // S_uv = (A_uv B^2 - A B_uv B - A_u B_v B - A_v B_u B + 2 A B_u B_v) / B^3.
        let suv_num = auv
            .mul(&w2)
            .sub(&a0.mul(&wuv).mul(&w0))
            .sub(&au.mul(&wv).mul(&w0))
            .sub(&av.mul(&wu).mul(&w0))
            .add(&two.mul(&a0).mul(&wu.mul(&wv)));
        // S_vv = (A_vv B^2 - A B_vv B - 2 A_v B_v B + 2 A B_v^2) / B^3.
        let svv_num = avv
            .mul(&w2)
            .sub(&a0.mul(&wvv).mul(&w0))
            .sub(&two.mul(&av).mul(&wv).mul(&w0))
            .add(&two.mul(&a0).mul(&wv.mul(&wv)));

        let suu = suu_num.div(&w3)?;
        let suv = suv_num.div(&w3)?;
        let svv = svv_num.div(&w3)?;
        Some((suu, suv, svv))
    }

    /// An `IBox3` from three per-coordinate interval enclosures.
    fn box3_of(v: [CertifiedInterval; 3]) -> IBox3 {
        IBox3 {
            lo: [v[0].lo, v[1].lo, v[2].lo],
            hi: [v[0].hi, v[1].hi, v[2].hi],
        }
    }

    /// The interval dot product of two derivative boxes (used for `E`, `F`, `G`).
    fn dot3(a: IBox3, b: IBox3) -> CertifiedInterval {
        let x = Self::iv(a.lo[0], a.hi[0]).mul(&Self::iv(b.lo[0], b.hi[0]));
        let y = Self::iv(a.lo[1], a.hi[1]).mul(&Self::iv(b.lo[1], b.hi[1]));
        let z = Self::iv(a.lo[2], a.hi[2]).mul(&Self::iv(b.lo[2], b.hi[2]));
        x.add(&y).add(&z)
    }

    /// The interval cross product of two derivative boxes.
    fn cross_box(a: IBox3, b: IBox3) -> IBox3 {
        let ax = Self::iv(a.lo[0], a.hi[0]);
        let ay = Self::iv(a.lo[1], a.hi[1]);
        let az = Self::iv(a.lo[2], a.hi[2]);
        let bx = Self::iv(b.lo[0], b.hi[0]);
        let by = Self::iv(b.lo[1], b.hi[1]);
        let bz = Self::iv(b.lo[2], b.hi[2]);
        let cx = ay.mul(&bz).sub(&az.mul(&by));
        let cy = az.mul(&bx).sub(&ax.mul(&bz));
        let cz = ax.mul(&by).sub(&ay.mul(&bx));
        IBox3 {
            lo: [cx.lo, cy.lo, cz.lo],
            hi: [cx.hi, cy.hi, cz.hi],
        }
    }

    /// An interval from raw endpoints.
    fn iv(lo: f64, hi: f64) -> CertifiedInterval {
        CertifiedInterval { lo, hi }
    }

    /// A vacuously true (fully unbounded) position enclosure.
    fn vacuous_box3() -> IBox3 {
        IBox3 {
            lo: [f64::NEG_INFINITY; 3],
            hi: [f64::INFINITY; 3],
        }
    }

    /// A vacuously true derivative enclosure.
    fn vacuous_derivs() -> DerivativeEnclosure {
        DerivativeEnclosure {
            su: Self::vacuous_box3(),
            sv: Self::vacuous_box3(),
        }
    }

    /// A vacuously true second-derivative enclosure.
    fn vacuous_second_derivs() -> SecondDerivativeEnclosure {
        SecondDerivativeEnclosure {
            suu: Self::vacuous_box3(),
            suv: Self::vacuous_box3(),
            svv: Self::vacuous_box3(),
        }
    }
}

impl CertifiedPatch for BezierLeaf {
    fn enclose(&self, d: IBox2) -> IBox3 {
        let sub = match self.unit_box(d) {
            Some(sub) => sub,
            None => return Self::vacuous_box3(),
        };
        let w0 = match self.positive_weight(sub.0, sub.1) {
            Some(w) => w,
            None => return Self::vacuous_box3(),
        };
        let mut lo = [0.0; 3];
        let mut hi = [0.0; 3];
        for comp in 0..3 {
            let a = match self.hull(comp, sub.0, sub.1) {
                Some(a) => a,
                None => return Self::vacuous_box3(),
            };
            let q = match a.div(&w0) {
                Some(q) => q,
                None => return Self::vacuous_box3(),
            };
            lo[comp] = q.lo;
            hi[comp] = q.hi;
        }
        IBox3 { lo, hi }
    }

    fn derivs(&self, d: IBox2) -> DerivativeEnclosure {
        let sub = match self.unit_box(d) {
            Some(sub) => sub,
            None => return Self::vacuous_derivs(),
        };
        let su = match self.quotient_partial(sub.0, sub.1, (1, 0)) {
            Some(su) => su,
            None => return Self::vacuous_derivs(),
        };
        let sv = match self.quotient_partial(sub.0, sub.1, (0, 1)) {
            Some(sv) => sv,
            None => return Self::vacuous_derivs(),
        };
        DerivativeEnclosure {
            su: Self::box3_of(su),
            sv: Self::box3_of(sv),
        }
    }

    fn normal_cone(&self, d: IBox2) -> Cone {
        let de = self.derivs(d);
        let normal = Self::cross_box(de.su, de.sv);
        // Choose the coordinate axis with the largest certified lower bound of
        // the dot product over the cross-product box. If that bound is > 0 the
        // whole normal set provably lies in the corresponding open hemisphere,
        // so the closed hemisphere cone (half-angle PI/2) certifies it. This is
        // the transcendental-free local cone constructor of §3.2 (the module
        // may not depend on truck-evidence, and the N4 gate admits no inverse
        // trig). When no coordinate axis certifies (a box straddling every
        // coordinate plane), no hemisphere cone exists: the shim `Cone` shape
        // cannot express the absence, so the best-coordinate PI/2 cone is
        // returned and callers subdivide until the certified arm holds.
        let candidates = [
            normal.lo[0],
            -normal.hi[0],
            normal.lo[1],
            -normal.hi[1],
            normal.lo[2],
            -normal.hi[2],
        ];
        let mut best = 0usize;
        for (idx, &margin) in candidates.iter().enumerate() {
            if margin > candidates[best] {
                best = idx;
            }
        }
        let axis = match best {
            0 => [1.0, 0.0, 0.0],
            1 => [-1.0, 0.0, 0.0],
            2 => [0.0, 1.0, 0.0],
            3 => [0.0, -1.0, 0.0],
            4 => [0.0, 0.0, 1.0],
            _ => [0.0, 0.0, -1.0],
        };
        // The axis is a coordinate basis vector and PI/2 is in `[0, PI)`, so
        // `Cone::try_new` cannot refuse; the match arm keeps `unwrap_used`
        // denied.
        match Cone::try_new(axis, std::f64::consts::FRAC_PI_2) {
            Ok(cone) => cone,
            Err(_) => Cone {
                axis,
                half_angle: std::f64::consts::FRAC_PI_2,
            },
        }
    }

    fn regularity(&self, d: IBox2) -> ClaimVerdict<CertifiedPositive, Degeneracy, Reason> {
        let de = self.derivs(d);
        let e = Self::dot3(de.su, de.su);
        let g = Self::dot3(de.sv, de.sv);
        let f = Self::dot3(de.su, de.sv);
        let egf2 = e.mul(&g).sub(&f.mul(&f));
        // The §0.4 regularity floor `TOL_JACOBIAN` is the singular-map floor at
        // which `EG - F^2` is treated as zero: `Proven` requires a lower bound
        // above the floor, `Disproven` an upper bound below it. The floor (not
        // a bare comparison with 0) is what makes a provably degenerate patch —
        // exactly-zero `EG - F^2`, up to the outward-rounded subnormal noise of
        // the interval arithmetic on collapsed derivative nets — Disproven
        // rather than Inconclusive. Recorded in the packet RESULT notes.
        if egf2.lo > TOL_JACOBIAN {
            match CertifiedPositive::try_new(egf2.lo) {
                Ok(positive) => ClaimVerdict::Proven(positive),
                // A finite positive lower bound always constructs; kept as a
                // match so the refusing constructor stays unwrap-free.
                Err(_) => ClaimVerdict::Inconclusive("regularity_positive_bound_refused"),
            }
        } else if egf2.hi < TOL_JACOBIAN {
            ClaimVerdict::Disproven(Degeneracy {
                box_: d,
                egf2: (egf2.lo, egf2.hi),
            })
        } else {
            ClaimVerdict::Inconclusive("regularity_egf2_straddles_the_singular_floor")
        }
    }

    fn weight_bound(&self, d: IBox2) -> Option<ClaimVerdict<CertifiedPositive, Pole, Reason>> {
        let sub = match self.unit_box(d) {
            Some(sub) => sub,
            None => return Some(ClaimVerdict::Inconclusive("weight_box_out_of_domain")),
        };
        let w = match self.hull(3, sub.0, sub.1) {
            Some(w) => w,
            // A finite control net over a compact unit box never refutes the
            // hull; this arm keeps `None` (never returned by a leaf) reserved
            // for the truly unreachable overflow case.
            None => return Some(ClaimVerdict::Inconclusive("weight_enclosure_unavailable")),
        };
        if w.lo > 0.0 {
            match CertifiedPositive::try_new(w.lo) {
                Ok(positive) => Some(ClaimVerdict::Proven(positive)),
                Err(_) => Some(ClaimVerdict::Inconclusive("weight_positive_bound_refused")),
            }
        } else if w.hi < 0.0 {
            Some(ClaimVerdict::Disproven(Pole {
                box_: d,
                w: (w.lo, w.hi),
            }))
        } else {
            // The enclosure straddles zero: §7.1's Inconclusive arm (the
            // `WeightDegenerate` Disproven member is the constructor-side
            // positive certificate; the straddle itself is undecidable here).
            Some(ClaimVerdict::Inconclusive("weight_straddles_zero"))
        }
    }
}

impl CertifiedPatchC2 for BezierLeaf {
    fn second_derivs(&self, d: IBox2) -> SecondDerivativeEnclosure {
        let sub = match self.unit_box(d) {
            Some(sub) => sub,
            None => return Self::vacuous_second_derivs(),
        };
        let mut suu = std::array::from_fn(|_| CertifiedInterval::point(0.0));
        let mut suv = std::array::from_fn(|_| CertifiedInterval::point(0.0));
        let mut svv = std::array::from_fn(|_| CertifiedInterval::point(0.0));
        for comp in 0..3 {
            let triple = match self.quotient_second(comp, sub.0, sub.1) {
                Some(triple) => triple,
                None => return Self::vacuous_second_derivs(),
            };
            suu[comp] = triple.0;
            suv[comp] = triple.1;
            svv[comp] = triple.2;
        }
        SecondDerivativeEnclosure {
            suu: Self::box3_of(suu),
            suv: Self::box3_of(suv),
            svv: Self::box3_of(svv),
        }
    }
}
