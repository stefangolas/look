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

//! The §3.3 lifted atlas: pole charts and the rational carrier family
//! (BG-KV2-405-K2B).
//!
//! This module completes the K2 substrate over the rational carriers of
//! §3.2 (BG-KV2-104-RATCARRIER): `Param` lifts whose deck integer is a
//! FIRST-CLASS coordinate, the finite atlas of regular charts per carrier
//! kind with chart ids, overlap regions, and exact affine/rational transition
//! maps with outward-rounded transport, and — as code — §3.4's chart-switch
//! doctrine: a rank-deficient parameterization switches chart when the image
//! is certified regular elsewhere (the pole case), and refuses/trims when the
//! degeneracy is a genuine carrier singularity (the cone apex case).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **N4 by construction.** No transcendental function appears in this module:
//! every chart map below is polynomial / interval-rational over the
//! `CertifiedInterval` primitive, so no `sin`, `cos`, `atan2`, `exp`, `ln`,
//! `log`, `powf`, or `sqrt` call can appear on any enclosure path (the
//! `no_transcendental_call_in_atlas_module` source test pins this).
//!
//! **The carrier families.** [`ChartAtlas`] owns the finite chart family of a
//! carrier:
//!
//! * *Plane* — one identity chart; the affine `(u, v)` plane chart; no period.
//! * *Cylinder* — one rational half-angle chart (the §3.2 form) with the
//!   surface's exact angular deck generator on the revolution axis when the
//!   axis is exactly a coordinate axis (see [`ChartLattice`]).
//! * *Sphere* — the pole-chart atlas. Two **pole charts** (the rationalized
//!   polar half-angle charts whose `u = tan(θ/2)` collapse onto the pole at
//!   `u = v = 0`) and their **partners** (the stereographic charts that are
//!   regular AT the same pole). An arc crossing `u = v = 0` on a pole chart
//!   continues on the partner chart — same arc, no valence change.
//! * *Cone* — the apex-excluding chart family. The cone's apex is the §3.4
//!   carrier-singularity case: no chart of the family reaches the apex, and a
//!   box that does routes [`DegeneracyRoute::CarrierSingular`]. The cone and
//!   torus carriers JOIN the admitted family here (the Wave-1 404 pending
//!   refusal in `kernel::rational` is a documented re-route: it stays live for
//!   out-of-atlas carriers, while the chart family itself is now constructed
//!   by this module's atlas implementors).
//! * *Torus* — the rational parameterization's chart family (the product of
//!   the two circle half-angle charts).
//!
//! The Cone/Torus chart leaves do not restate the deferred half-angle
//! *enclosure* machinery of §3.2: their `CertifiedPatch` surfaces return the
//! module's no-certificate markers (the crate's established pending form),
//! with the named reasons [`CONE_FORM_PENDING`] / [`TORUS_FORM_PENDING`]. What
//! this module certifies for them is the CHART structure — ids, regions,
//! apex-excluding boxes, transitions — and the §3.4 routing over it.
//!
//! **K2 lifts.** [`ChartAtlas::lift`] is the unwrapped pcurve lift: a `Param`
//! whose `u` is the canonical chart coordinate and whose `deck` integer
//! carries the winding. `Param(5.9)` stays `5.9` (deck `0`); the developed
//! coordinate `6.4` lifts to `deck 1, u = 6.4 − P` — the shim kit's
//! deck-wrap fixture (BG-KV2-000, fixture 5) now runs through the atlas.
//! The lift never wraps an already-lifted coordinate and refuses
//! [`RefusalKind::DeckExhausted`] (Inconclusive) when one edge would walk more
//! than the §0.4 ceiling [`crate::kernel::config::DECK_MAX`] deck crossings.

use crate::formal::exact::CertifiedInterval;
use crate::kernel::config::DECK_MAX;
use crate::kernel::evidence::{ClaimVerdict, Construction, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::graph::{ChartId, Param};
use crate::kernel::leaf::{CarrierData, RationalCarrier, RationalCarrierKind};
use crate::kernel::patch::{
    CertifiedPatch, CertifiedPositive, Cone, Degeneracy, DerivativeEnclosure, IBox2, IBox3, Pole,
    Reason,
};

/// The named reason a Cone chart leaf certifies nothing: an exactly rational
/// slope cannot be recovered from a stored half-angle without a transcendental
/// normalization, so the deferred cone enclosure machinery must receive the
/// slope as data. The chart family (apex-excluding regions, transitions) is
/// this module's contribution; the box-valued claims stay pending.
const CONE_FORM_PENDING: Reason = "cone_half_angle_slope_needs_data";
/// The named reason a Torus chart leaf certifies nothing: the deferred torus
/// rational enclosure machinery is a later packet. The chart family (the
/// product-of-half-angle sheets) is constructed here.
const TORUS_FORM_PENDING: Reason = "torus_rational_form_pending";
/// The reason a box cannot be certified over a carrier whose chart leaf is not
/// one of this atlas's own patches (an out-of-atlas `CertifiedPatch`).
const OUT_OF_ATLAS: Reason = "atlas_out_of_atlas_patch";

// ---------------------------------------------------------------------------
// §3.4 route vocabulary
// ---------------------------------------------------------------------------

/// The §3.4 route a degeneracy takes: either the parameterization is rank
/// deficient at a REGULAR image point and the arc continues on the partner
/// chart, or the degeneracy is a genuine carrier singularity and the arc is
/// refused/trimmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegeneracyRoute {
    /// The box reached a chart pole whose image is certified regular on the
    /// partner chart: switch to the partner (same arc, no valence change).
    SwitchChart {
        /// The partner chart to continue on.
        target: ChartId,
    },
    /// The box reached a carrier singularity (or no partner can certify the
    /// image): refuse/trim.
    CarrierSingular,
}

// ---------------------------------------------------------------------------
// Interval helpers (N4: all arithmetic is interval-rational)
// ---------------------------------------------------------------------------

/// The degenerate interval of a scalar.
fn ci(x: f64) -> CertifiedInterval {
    CertifiedInterval::point(x)
}

/// The certified square of an interval `{x² : x ∈ i}` with outward rounding,
/// computed endpoint-wise (an interval times itself would widen across zero).
fn square(i: &CertifiedInterval) -> CertifiedInterval {
    let lo2 = i.lo * i.lo;
    let hi2 = i.hi * i.hi;
    if i.lo >= 0.0 {
        CertifiedInterval {
            lo: lo2.next_down(),
            hi: hi2.next_up(),
        }
    } else if i.hi <= 0.0 {
        CertifiedInterval {
            lo: hi2.next_down(),
            hi: lo2.next_up(),
        }
    } else {
        CertifiedInterval {
            lo: 0.0,
            hi: (if lo2 >= hi2 { lo2 } else { hi2 }).next_up(),
        }
    }
}

/// The two axis intervals of a parameter box.
fn axes(d: IBox2) -> (CertifiedInterval, CertifiedInterval) {
    (
        CertifiedInterval {
            lo: d.lo[0],
            hi: d.hi[0],
        },
        CertifiedInterval {
            lo: d.lo[1],
            hi: d.hi[1],
        },
    )
}

/// An `IBox3` from three interval components.
fn box3(x: CertifiedInterval, y: CertifiedInterval, z: CertifiedInterval) -> IBox3 {
    IBox3 {
        lo: [x.lo, y.lo, z.lo],
        hi: [x.hi, y.hi, z.hi],
    }
}

/// The module's "no certified patch" marker box: NaN bounds mean no certified
/// enclosure exists over this box.
fn no_patch_box() -> IBox3 {
    IBox3 {
        lo: [f64::NAN; 3],
        hi: [f64::NAN; 3],
    }
}

/// The module's "no certified patch" derivative marker.
fn no_patch_derivs() -> DerivativeEnclosure {
    DerivativeEnclosure {
        su: no_patch_box(),
        sv: no_patch_box(),
    }
}

/// The module's "no certified patch" normal-cone marker.
fn no_patch_cone() -> Cone {
    Cone {
        axis: [f64::NAN; 3],
        half_angle: f64::NAN,
    }
}

/// A `CertifiedPositive` construction from a certified positive lower bound.
fn positive_bound(lo: f64) -> Option<CertifiedPositive> {
    CertifiedPositive::try_new(lo).ok()
}

/// Classify an `EG − F²` enclosure into the regularity claim, with the
/// §3.2 analytic convention (a positive rational function is degenerate only
/// where it is exactly zero, so no `TOL_JACOBIAN` floor participates).
fn classify_egf2(
    d: IBox2,
    egf2: Option<CertifiedInterval>,
) -> ClaimVerdict<CertifiedPositive, Degeneracy, Reason> {
    match egf2 {
        Some(enclosure) if enclosure.lo > 0.0 && enclosure.is_finite() => {
            match positive_bound(enclosure.lo) {
                Some(bound) => ClaimVerdict::Proven(bound),
                None => ClaimVerdict::Inconclusive(OUT_OF_ATLAS),
            }
        }
        Some(enclosure) if enclosure.hi <= 0.0 => ClaimVerdict::Disproven(Degeneracy {
            box_: d,
            egf2: (enclosure.lo, enclosure.hi),
        }),
        _ => ClaimVerdict::Inconclusive(OUT_OF_ATLAS),
    }
}

/// Classify a denominator enclosure into the weight-bound claim.
fn classify_weight(
    d: IBox2,
    weight: &CertifiedInterval,
) -> ClaimVerdict<CertifiedPositive, Pole, Reason> {
    if weight.lo > 0.0 && weight.is_finite() {
        match positive_bound(weight.lo) {
            Some(bound) => ClaimVerdict::Proven(bound),
            None => ClaimVerdict::Inconclusive(OUT_OF_ATLAS),
        }
    } else if weight.hi <= 0.0 {
        ClaimVerdict::Disproven(Pole {
            box_: d,
            w: (weight.lo, weight.hi),
        })
    } else {
        ClaimVerdict::Inconclusive(OUT_OF_ATLAS)
    }
}

// ---------------------------------------------------------------------------
// Chart shapes
// ---------------------------------------------------------------------------

/// The exact deck lattice of a chart: on which axis, if any, the chart's
/// parameterization is periodic with a representation-derived generator.
///
/// Only exact generators are stored (an uncertified declared period is never a
/// deck generator). For the rational half-angle charts of this module the
/// surface's revolution axis carries the exact angular generator `2π`; planes
/// and the sphere's stereographic/polar charts are non-periodic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartLattice {
    /// The exact `u` deck generator, if the chart is periodic on `u`.
    pub u_period: Option<f64>,
    /// The exact `v` deck generator, if the chart is periodic on `v`.
    pub v_period: Option<f64>,
}

impl ChartLattice {
    /// A non-periodic lattice.
    pub const NONE: Self = Self {
        u_period: None,
        v_period: None,
    };

    /// Build a lattice, refusing a non-finite or non-positive generator.
    pub fn try_new(u_period: Option<f64>, v_period: Option<f64>) -> Construction<Self> {
        for (axis, period) in [("u", u_period), ("v", v_period)] {
            if let Some(p) = period {
                if !p.is_finite() {
                    return Err(refusal(
                        RefusalKind::NonFinite,
                        "lattice_period_not_finite",
                        format!("the {axis} deck generator {p} is not finite"),
                    ));
                }
                if p <= 0.0 {
                    return Err(refusal(
                        RefusalKind::ClaimRefuted,
                        "lattice_period_nonpositive",
                        format!("the {axis} deck generator {p} is not positive"),
                    ));
                }
            }
        }
        Ok(Self { u_period, v_period })
    }

    /// The exact `u` deck generator, if any.
    pub fn u_generator(&self) -> Option<f64> {
        self.u_period
    }

    /// The exact `v` deck generator, if any.
    pub fn v_generator(&self) -> Option<f64> {
        self.v_period
    }
}

/// The partner chart a pole switch routes to, together with the box at which
/// the partner certifies that the pole's image is regular.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Partner {
    /// The partner chart's id.
    pub chart: ChartId,
    /// The partner-chart box over which the image of the pole is certified
    /// regular (`Proven` regularity on the partner's own chart).
    pub regular_box: IBox2,
}

/// One chart of a carrier's finite atlas: an id, its certified-regular region,
/// its deck lattice, its `CertifiedPatch` leaf, and the §3.4 switch data
/// (a chart-internal pole locus and the partner chart that covers it
/// regularly).
#[derive(Debug, Clone, PartialEq)]
pub struct Chart {
    /// The chart's id.
    pub id: ChartId,
    /// The carrier family the chart belongs to.
    pub kind: RationalCarrierKind,
    /// The certified-regular region of this chart (its own `CertifiedPatch`
    /// certifies `Proven` regularity over sub-boxes of this region).
    pub region: IBox2,
    /// The chart's deck lattice.
    pub lattice: ChartLattice,
    /// The chart's `CertifiedPatch` leaf.
    pub patch: ChartPatch,
    /// The chart's internal rank-deficiency locus (a pole or the cone apex),
    /// when this chart is degenerate there.
    pub pole: Option<IBox2>,
    /// The partner chart a pole switch routes to, when there is one.
    pub partner: Option<Partner>,
}

impl Chart {
    /// The chart id.
    pub fn id(&self) -> ChartId {
        self.id
    }

    /// Whether the box intersects the chart's internal degeneracy locus.
    pub fn box_reaches_pole(&self, box_: IBox2) -> bool {
        match self.pole {
            Some(locus) => boxes_overlap(box_, locus),
            None => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Chart families
// ---------------------------------------------------------------------------

/// The finite atlas of regular charts of one rational carrier (§3.3): the
/// chart bookkeeping over the §3.2 enclosure machinery.
#[derive(Debug, Clone)]
pub struct ChartAtlas {
    /// The carrier family the atlas is over.
    kind: RationalCarrierKind,
    /// The charts of the family, in a fixed construction order.
    charts: Vec<Chart>,
}

impl ChartAtlas {
    /// Build the finite atlas of charts over the carrier.
    ///
    /// The admission route of the cone/torus carriers: the chart family is
    /// constructed here (the §3.3 bookkeeping), where the Wave-1
    /// `kernel::rational::admit` refusal (with its pending name) remains
    /// available for out-of-atlas carriers.
    pub fn try_new(carrier: &RationalCarrier) -> Construction<Self> {
        let kind = carrier.kind;
        let charts = match kind {
            RationalCarrierKind::Plane => plane_charts(carrier)?,
            RationalCarrierKind::Cylinder => cylinder_charts(carrier)?,
            RationalCarrierKind::Sphere => sphere_charts(carrier)?,
            RationalCarrierKind::Cone => cone_charts(carrier)?,
            RationalCarrierKind::Torus => torus_charts(carrier)?,
        };
        Ok(Self { kind, charts })
    }

    /// The carrier family the atlas is over.
    pub fn kind(&self) -> RationalCarrierKind {
        self.kind
    }

    /// The charts of the family.
    pub fn charts(&self) -> &[Chart] {
        &self.charts
    }

    /// The chart with the given id, if present.
    pub fn chart(&self, id: ChartId) -> Option<&Chart> {
        self.charts.iter().find(|chart| chart.id == id)
    }

    /// The deck generator of the chart's `u` axis, when the chart is periodic.
    pub fn u_period(&self, id: ChartId) -> Option<f64> {
        self.chart(id).and_then(|chart| chart.lattice.u_generator())
    }

    /// §3.4 `classify_degeneracy`: classify a box over one of this atlas's
    /// chart patches into the degeneracy route.
    ///
    /// `p` must be the `CertifiedPatch` leaf of one of this atlas's own
    /// charts (obtainable through [`Chart::patch`]). The two outcomes are
    /// [`DegeneracyRoute::SwitchChart`] — the box reached a chart pole whose
    /// image is certified regular on the partner chart (the partner's
    /// `CertifiedPatch` certifies `Proven` regularity over the partner's
    /// [`Partner::regular_box`]) — and [`DegeneracyRoute::CarrierSingular`] —
    /// the box reached a genuine carrier singularity (the cone apex) or no
    /// partner can certify the image, so the caller refuses/trims. An
    /// out-of-atlas `CertifiedPatch` routes `CarrierSingular`.
    pub fn classify_degeneracy(&self, p: &dyn CertifiedPatch, box_: IBox2) -> DegeneracyRoute {
        for chart in &self.charts {
            if same_patch(p, &chart.patch) {
                return self.route_for(chart, box_);
            }
        }
        DegeneracyRoute::CarrierSingular
    }

    /// §3.4 routing over one of the atlas's own charts.
    fn route_for(&self, chart: &Chart, box_: IBox2) -> DegeneracyRoute {
        if chart.box_reaches_pole(box_) {
            if let Some(partner) = chart.partner {
                if let Some(target) = self.chart(partner.chart) {
                    if matches!(
                        target.patch.regularity(partner.regular_box),
                        ClaimVerdict::Proven(_)
                    ) {
                        return DegeneracyRoute::SwitchChart {
                            target: partner.chart,
                        };
                    }
                }
            }
            DegeneracyRoute::CarrierSingular
        } else {
            DegeneracyRoute::CarrierSingular
        }
    }

    /// The unwrapped K2 pcurve lift through the atlas: lift the developed
    /// coordinate `raw_u` of the periodic chart `id` to a `Param` whose `u` is
    /// canonical in `[0, period)` and whose `deck` integer carries the winding
    /// relative to `from`'s deck.
    ///
    /// The lift never wraps an already-lifted coordinate: `Param(5.9)` stays
    /// `5.9` (deck `0`), and the developed `6.4` lifts to `deck +1` with
    /// canonical `u = 6.4 − period`. The edge bound is the §0.4 ceiling: a
    /// single edge whose `|deck − from.deck|` would exceed
    /// [`DECK_MAX`](crate::kernel::config::DECK_MAX) refuses
    /// [`RefusalKind::DeckExhausted`] (Inconclusive) — the deck-exhaustion
    /// termination of helical lifts.
    pub fn lift(&self, id: ChartId, from: &Param, raw_u: f64) -> Construction<Param> {
        let period = match self.u_period(id) {
            Some(period) => period,
            None => {
                return Err(refusal(
                    RefusalKind::ClaimRefuted,
                    "lift_chart_not_periodic",
                    format!("chart {id:?} has no exact u deck generator"),
                ))
            }
        };
        if from.chart != id {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "lift_chart_mismatch",
                format!("the lift base {from:?} is not on chart {id:?}"),
            ));
        }
        lift_periodic(id, period, from, raw_u)
    }
}

// ---------------------------------------------------------------------------
// Carrier chart families
// ---------------------------------------------------------------------------

/// A finite non-degenerate `IBox2`, used where the family geometry demands one.
fn box2(lo: [f64; 2], hi: [f64; 2]) -> IBox2 {
    IBox2 { lo, hi }
}

/// The plane family: one identity chart, non-periodic.
#[allow(clippy::result_large_err)]
fn plane_charts(carrier: &RationalCarrier) -> Construction<Vec<Chart>> {
    let region = carrier.domain;
    Ok(vec![Chart {
        id: ChartId(0),
        kind: RationalCarrierKind::Plane,
        region,
        lattice: ChartLattice::NONE,
        patch: ChartPatch::delegate(carrier.clone()),
        pole: None,
        partner: None,
    }])
}

/// An exactly rational orthonormal circle frame `(e1, e2)` for the plane
/// orthogonal to a coordinate axis (N4: any other axis would need a
/// transcendental normalization to frame).
fn circle_frame(axis: [f64; 3]) -> Option<([f64; 3], [f64; 3])> {
    let x = axis[0];
    let y = axis[1];
    let z = axis[2];
    if x == 0.0 && y == 0.0 && (z == 1.0 || z == -1.0) {
        return Some(([1.0, 0.0, 0.0], [0.0, z, 0.0]));
    }
    if y == 0.0 && z == 0.0 && (x == 1.0 || x == -1.0) {
        return Some(([0.0, 1.0, 0.0], [0.0, 0.0, x]));
    }
    if x == 0.0 && z == 0.0 && (y == 1.0 || y == -1.0) {
        return Some(([0.0, 0.0, 1.0], [y, 0.0, 0.0]));
    }
    None
}

/// The cylinder family: one rational half-angle chart (the §3.2 form), with
/// the exact angular deck generator on the revolution axis when the axis is a
/// coordinate axis.
#[allow(clippy::result_large_err)]
fn cylinder_charts(carrier: &RationalCarrier) -> Construction<Vec<Chart>> {
    let axis = match carrier.data {
        CarrierData::Cylinder { axis, .. } => axis,
        _ => {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "atlas_cylinder_data",
                "the cylinder family needs a cylinder carrier".to_string(),
            ))
        }
    };
    let region = carrier.domain;
    let periodic = circle_frame(axis).is_some();
    let lattice = if periodic {
        ChartLattice::try_new(Some(std::f64::consts::TAU), None)?
    } else {
        ChartLattice::NONE
    };
    Ok(vec![Chart {
        id: ChartId(0),
        kind: RationalCarrierKind::Cylinder,
        region,
        lattice,
        patch: ChartPatch::delegate(carrier.clone()),
        pole: None,
        partner: None,
    }])
}

/// The sphere pole-chart atlas (§3.3): the polar half-angle pole charts
/// (degenerate along `u = 0`, at the pole `u = v = 0`) and their stereographic
/// partners (regular AT the same pole).
///
/// Chart ids (fixed order):
/// 0. the north stereographic chart (regular at the north pole),
/// 1. the south stereographic chart (regular at the south pole),
/// 2. the north pole chart (`u = tan(θ/2)`, `v = tan(φ/2)`; partner id 0),
/// 3. the south pole chart (partner id 1).
#[allow(clippy::result_large_err)]
fn sphere_charts(carrier: &RationalCarrier) -> Construction<Vec<Chart>> {
    let (center, radius) = match carrier.data {
        CarrierData::Sphere { center, radius } => (center, radius),
        _ => {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "atlas_sphere_data",
                "the sphere family needs a sphere carrier".to_string(),
            ))
        }
    };
    // The certified-regular region of the stereographic charts (finite boxes
    // well away from the chart degeneration at infinity).
    let stereo_region = box2([-1.5, -1.5], [1.5, 1.5]);
    let north = Chart {
        id: ChartId(0),
        kind: RationalCarrierKind::Sphere,
        region: stereo_region,
        lattice: ChartLattice::NONE,
        patch: ChartPatch::delegate(carrier.clone()),
        pole: None,
        partner: None,
    };
    let south = Chart {
        id: ChartId(1),
        kind: RationalCarrierKind::Sphere,
        region: stereo_region,
        lattice: ChartLattice::NONE,
        patch: ChartPatch::south_sphere(center, radius),
        pole: None,
        partner: None,
    };
    // The polar pole charts: u = tan(theta/2), v = tan(phi/2). The
    // parameterization collapses on the whole u = 0 polar line; the pole locus
    // is the thin u-band through u = v = 0.
    let pole_region = box2([0.5, -1.5], [2.0, 1.5]);
    let pole_locus = box2([-0.05, -1.5], [0.05, 1.5]);
    let north_pole_box = box2([-0.01, -0.01], [0.01, 0.01]);
    let south_pole_box = north_pole_box;
    let north_pole = Chart {
        id: ChartId(2),
        kind: RationalCarrierKind::Sphere,
        region: pole_region,
        lattice: ChartLattice::NONE,
        patch: ChartPatch::pole_sphere(center, radius, 1.0),
        pole: Some(pole_locus),
        partner: Some(Partner {
            chart: north.id,
            regular_box: north_pole_box,
        }),
    };
    let south_pole = Chart {
        id: ChartId(3),
        kind: RationalCarrierKind::Sphere,
        region: pole_region,
        lattice: ChartLattice::NONE,
        patch: ChartPatch::pole_sphere(center, radius, -1.0),
        pole: Some(pole_locus),
        partner: Some(Partner {
            chart: south.id,
            regular_box: south_pole_box,
        }),
    };
    Ok(vec![north, south, north_pole, south_pole])
}

/// The exact rational transition of a pole-chart point into its stereographic
/// partner chart near the pole (both the north and the south pairs share the
/// formula in their own pole-centred coordinates).
///
/// A pole-chart point `(u, v) = (tan(θ/2), tan(φ/2))` maps onto the partner
/// stereographic chart at `(U, V) = (u(1 − v²)/(1 + v²), 2uv/(1 + v²))` — the
/// partner's `(0, 0)` is the same pole, and an arc crossing `u = v = 0`
/// continues there with no valence change. `None` when the arithmetic is not
/// finite.
pub fn pole_to_partner(u: f64, v: f64) -> Option<[f64; 2]> {
    if !u.is_finite() || !v.is_finite() {
        return None;
    }
    let d = 1.0 + v * v;
    let uu = u * (1.0 - v * v) / d;
    let vv = 2.0 * u * v / d;
    if uu.is_finite() && vv.is_finite() {
        Some([uu, vv])
    } else {
        None
    }
}

/// The outward-rounded box transport of [`pole_to_partner`]: the exact rational
/// transition evaluated on the box as a `CertifiedInterval` map, so the result
/// provably contains the image of every point of the box. `None` when the
/// transported arithmetic is not finite.
pub fn pole_to_partner_box(box_: IBox2) -> Option<IBox2> {
    let u = CertifiedInterval {
        lo: box_.lo[0],
        hi: box_.hi[0],
    };
    let v = CertifiedInterval {
        lo: box_.lo[1],
        hi: box_.hi[1],
    };
    let d = ci(1.0).add(&square(&v));
    let uu = ci(1.0).sub(&square(&v)).mul(&u).div(&d)?;
    let vv = ci(2.0).mul(&u).mul(&v).div(&d)?;
    if !uu.is_finite() || !vv.is_finite() {
        return None;
    }
    Some(IBox2 {
        lo: [uu.lo, vv.lo],
        hi: [uu.hi, vv.hi],
    })
}

/// The axial lower bound of the cone's apex-excluding charts: the cone apex is
/// a carrier singularity, so the chart family keeps a clearance away from the
/// apex when the carrier's axial interval reaches it.
fn cone_apex_clearance(carrier: &RationalCarrier) -> Option<f64> {
    match carrier.data {
        CarrierData::Cone { height, .. } => {
            let lo = height.0;
            let hi = height.1;
            if lo > 0.0 {
                Some(lo)
            } else if hi > 0.0 {
                Some(hi * 0.1)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// The cone family: apex-excluding charts. The cone's apex is the §3.4
/// carrier-singularity case, so each chart's certified region keeps a
/// clearance above the apex plane and every chart carries the apex locus as
/// its internal degeneracy (`CarrierSingular` route, no partner).
#[allow(clippy::result_large_err)]
fn cone_charts(carrier: &RationalCarrier) -> Construction<Vec<Chart>> {
    if circle_frame(cone_axis(carrier)?).is_none() {
        return Err(refusal(
            RefusalKind::CarrierSingularity,
            "cone_axis_not_coordinate",
            "the cone chart family requires an exactly coordinate axis (an exactly \
             rational circle frame)"
                .to_string(),
        ));
    }
    let axial_lo = match cone_apex_clearance(carrier) {
        Some(lo) => lo,
        None => {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "cone_height_degenerate",
                "the cone axial interval must be non-degenerate".to_string(),
            ))
        }
    };
    let axial_hi = match carrier.data {
        CarrierData::Cone { height, .. } => height.1,
        _ => {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "atlas_cone_data",
                "the cone family needs a cone carrier".to_string(),
            ))
        }
    };
    let apex_locus = box2([-1.5, -0.05], [1.5, 0.05]);
    let family = |id: u32, u: (f64, f64)| Chart {
        id: ChartId(id),
        kind: RationalCarrierKind::Cone,
        region: box2([u.0, axial_lo], [u.1, axial_hi]),
        lattice: ChartLattice::NONE,
        patch: ChartPatch::cone(),
        pole: Some(apex_locus),
        partner: None,
    };
    Ok(vec![family(0, (-1.0, 1.0)), family(1, (1.0, 3.0))])
}

/// The torus family: the rational parameterization's chart family, as the
/// product of the two circle half-angle sheets.
#[allow(clippy::result_large_err)]
fn torus_charts(carrier: &RationalCarrier) -> Construction<Vec<Chart>> {
    if circle_frame(torus_axis(carrier)?).is_none() {
        return Err(refusal(
            RefusalKind::CarrierSingularity,
            "torus_axis_not_coordinate",
            "the torus chart family requires an exactly coordinate axis".to_string(),
        ));
    }
    let region = box2([-1.0, -1.0], [1.0, 1.0]);
    let mut charts = Vec::new();
    for id in 0u32..4 {
        charts.push(Chart {
            id: ChartId(id),
            kind: RationalCarrierKind::Torus,
            region,
            lattice: ChartLattice::NONE,
            patch: ChartPatch::torus(),
            pole: None,
            partner: None,
        });
    }
    Ok(charts)
}

#[allow(clippy::result_large_err)]
fn cone_axis(carrier: &RationalCarrier) -> Construction<[f64; 3]> {
    match carrier.data {
        CarrierData::Cone { axis, .. } => Ok(axis),
        _ => Err(refusal(
            RefusalKind::ClaimRefuted,
            "atlas_cone_data",
            "the cone family needs a cone carrier".to_string(),
        )),
    }
}

#[allow(clippy::result_large_err)]
fn torus_axis(carrier: &RationalCarrier) -> Construction<[f64; 3]> {
    match carrier.data {
        CarrierData::Torus { axis, .. } => Ok(axis),
        _ => Err(refusal(
            RefusalKind::ClaimRefuted,
            "atlas_torus_data",
            "the torus family needs a torus carrier".to_string(),
        )),
    }
}

/// Whether two boxes overlap (closed, componentwise).
fn boxes_overlap(a: IBox2, b: IBox2) -> bool {
    a.lo[0] <= b.hi[0] && b.lo[0] <= a.hi[0] && a.lo[1] <= b.hi[1] && b.lo[1] <= a.hi[1]
}

/// Whether `p` is the `CertifiedPatch` of one of this atlas's own chart
/// patches, by object address (the chart leaves are stored in the atlas, so a
/// patch obtained through [`Chart::patch`] is address-identical to the stored
/// one). The comparison is on the data pointer only: a fat-pointer equality
/// would also compare the vtable, which is not the identity this scan needs.
fn same_patch(p: &dyn CertifiedPatch, patch: &ChartPatch) -> bool {
    let a: *const dyn CertifiedPatch = p;
    let b: *const dyn CertifiedPatch = patch;
    a as *const () == b as *const ()
}

// ---------------------------------------------------------------------------
// The chart CertifiedPatch leaves
// ---------------------------------------------------------------------------

/// A `CertifiedPatch` chart leaf of a carrier atlas. The public leaf shape is
/// the carrier's form; construction goes through the [`ChartAtlas`] builder.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartPatch {
    form: PatchForm,
}

/// The concrete chart evaluation form a leaf carries. Plane, Cylinder, and the
/// north stereographic sphere are the §3.2 carrier forms (delegated to the
/// carrier's own `CertifiedPatch`); the south stereographic and the two polar
/// pole charts are evaluated by this module's interval forms; the cone and
/// torus forms keep the deferred pending markers.
#[derive(Debug, Clone, PartialEq)]
enum PatchForm {
    /// A §3.2 carrier whose own `CertifiedPatch` is the chart (Plane,
    /// Cylinder, north-sphere).
    Delegate(RationalCarrier),
    /// The south stereographic sphere `X = c + r(2u, 2v, u²+v²−1)/(1+u²+v²)`.
    SouthSphere {
        /// The sphere center.
        center: [f64; 3],
        /// The sphere radius.
        radius: f64,
    },
    /// A polar half-angle pole chart with pole `u = v = 0` (sign `+1` north,
    /// `−1` south) whose `u = 0` line collapses onto the pole.
    PoleSphere {
        /// The sphere center.
        center: [f64; 3],
        /// The sphere radius.
        radius: f64,
        /// `+1` for the north pole chart, `−1` for the south.
        sign: f64,
    },
    /// The deferred cone form (pending enclosure markers).
    Cone,
    /// The deferred torus form (pending enclosure markers).
    Torus,
}

impl ChartPatch {
    /// A leaf over a §3.2 carrier form (Plane, Cylinder, north sphere).
    pub fn delegate(carrier: RationalCarrier) -> Self {
        Self {
            form: PatchForm::Delegate(carrier),
        }
    }

    /// The south stereographic sphere chart leaf.
    fn south_sphere(center: [f64; 3], radius: f64) -> Self {
        Self {
            form: PatchForm::SouthSphere { center, radius },
        }
    }

    /// A polar pole-chart leaf.
    fn pole_sphere(center: [f64; 3], radius: f64, sign: f64) -> Self {
        Self {
            form: PatchForm::PoleSphere {
                center,
                radius,
                sign,
            },
        }
    }

    /// The deferred cone chart leaf.
    fn cone() -> Self {
        Self {
            form: PatchForm::Cone,
        }
    }

    /// The deferred torus chart leaf.
    fn torus() -> Self {
        Self {
            form: PatchForm::Torus,
        }
    }
}

impl CertifiedPatch for ChartPatch {
    fn enclose(&self, d: IBox2) -> IBox3 {
        match &self.form {
            PatchForm::Delegate(carrier) => carrier.enclose(d),
            PatchForm::SouthSphere { center, radius } => south_sphere_position(*center, *radius, d),
            PatchForm::PoleSphere {
                center,
                radius,
                sign,
            } => pole_sphere_position(*center, *radius, *sign, d),
            PatchForm::Cone | PatchForm::Torus => no_patch_box(),
        }
    }

    fn derivs(&self, d: IBox2) -> DerivativeEnclosure {
        match &self.form {
            PatchForm::Delegate(carrier) => carrier.derivs(d),
            PatchForm::SouthSphere { radius, .. } => south_sphere_derivs(*radius, d),
            PatchForm::PoleSphere { radius, sign, .. } => pole_sphere_derivs(*radius, *sign, d),
            PatchForm::Cone | PatchForm::Torus => no_patch_derivs(),
        }
    }

    fn normal_cone(&self, d: IBox2) -> Cone {
        match &self.form {
            PatchForm::Delegate(carrier) => carrier.normal_cone(d),
            PatchForm::SouthSphere { .. } | PatchForm::PoleSphere { .. } => {
                let de = self.derivs(d);
                hemisphere_cone(de)
            }
            PatchForm::Cone | PatchForm::Torus => no_patch_cone(),
        }
    }

    fn regularity(&self, d: IBox2) -> ClaimVerdict<CertifiedPositive, Degeneracy, Reason> {
        match &self.form {
            PatchForm::Delegate(carrier) => carrier.regularity(d),
            PatchForm::SouthSphere { radius, .. } => {
                classify_egf2(d, south_sphere_egf2(*radius, d))
            }
            PatchForm::PoleSphere { radius, .. } => classify_egf2(d, pole_sphere_egf2(*radius, d)),
            PatchForm::Cone => ClaimVerdict::Inconclusive(CONE_FORM_PENDING),
            PatchForm::Torus => ClaimVerdict::Inconclusive(TORUS_FORM_PENDING),
        }
    }

    fn weight_bound(&self, d: IBox2) -> Option<ClaimVerdict<CertifiedPositive, Pole, Reason>> {
        match &self.form {
            PatchForm::Delegate(carrier) => carrier.weight_bound(d),
            PatchForm::SouthSphere { .. } => Some(classify_weight(d, &sphere_weight(d))),
            PatchForm::PoleSphere { .. } => Some(classify_weight(d, &pole_weight(d))),
            PatchForm::Cone => Some(ClaimVerdict::Inconclusive(CONE_FORM_PENDING)),
            PatchForm::Torus => Some(ClaimVerdict::Inconclusive(TORUS_FORM_PENDING)),
        }
    }
}

/// The denominator `1 + u² + v²` of the stereographic sphere forms.
fn sphere_weight(d: IBox2) -> CertifiedInterval {
    let (u, v) = axes(d);
    ci(1.0).add(&square(&u)).add(&square(&v))
}

/// The denominator `(1 + u²)(1 + v²)` of the polar pole-chart forms.
fn pole_weight(d: IBox2) -> CertifiedInterval {
    let (u, v) = axes(d);
    ci(1.0).add(&square(&u)).mul(&ci(1.0).add(&square(&v)))
}

/// The certified south-stereographic position enclosure over `d`.
fn south_sphere_position(center: [f64; 3], radius: f64, d: IBox2) -> IBox3 {
    let (u, v) = axes(d);
    let u2 = square(&u);
    let v2 = square(&v);
    let weight = ci(1.0).add(&u2).add(&v2);
    let qx = ci(2.0).mul(&u).div(&weight);
    let qy = ci(2.0).mul(&v).div(&weight);
    let qz = u2.add(&v2).sub(&ci(1.0)).div(&weight);
    let q = match (qx, qy, qz) {
        (Some(x), Some(y), Some(z)) => [x, y, z],
        _ => return no_patch_box(),
    };
    let x = ci(center[0]).add(&ci(radius).mul(&q[0]));
    let y = ci(center[1]).add(&ci(radius).mul(&q[1]));
    let z = ci(center[2]).add(&ci(radius).mul(&q[2]));
    box3(x, y, z)
}

/// The certified polar pole-chart position enclosure over `d`.
///
/// With `s = u²`, `t = v²`, `w = (1 + s)(1 + t)` the map is
/// `(2u(1−t), 4uv, ±(1−s)(1+t)) / w` for unit radius.
fn pole_sphere_position(center: [f64; 3], radius: f64, sign: f64, d: IBox2) -> IBox3 {
    let (u, v) = axes(d);
    let u2 = square(&u);
    let v2 = square(&v);
    let weight = ci(1.0).add(&u2).mul(&ci(1.0).add(&v2));
    let nx = ci(2.0).mul(&u).mul(&ci(1.0).sub(&v2));
    let ny = ci(4.0).mul(&u).mul(&v);
    let nz = ci(sign).mul(&ci(1.0).sub(&u2)).mul(&ci(1.0).add(&v2));
    let qx = nx.div(&weight);
    let qy = ny.div(&weight);
    let qz = nz.div(&weight);
    let q = match (qx, qy, qz) {
        (Some(x), Some(y), Some(z)) => [x, y, z],
        _ => return no_patch_box(),
    };
    let x = ci(center[0]).add(&ci(radius).mul(&q[0]));
    let y = ci(center[1]).add(&ci(radius).mul(&q[1]));
    let z = ci(center[2]).add(&ci(radius).mul(&q[2]));
    box3(x, y, z)
}

/// The certified `EG − F²` of the south stereographic sphere over `d`: the
/// mirror image of the §3.2 north form, so `EG − F² = 16 r⁴ / w⁴`.
fn south_sphere_egf2(radius: f64, d: IBox2) -> Option<CertifiedInterval> {
    let weight = sphere_weight(d);
    let weight4 = square(&square(&weight));
    let r2 = radius * radius;
    ci(16.0 * r2 * r2).div(&weight4)
}

/// The certified `EG − F²` of a polar pole chart over `d`. The polar map has
/// the collapse-free area scale `r² sinθ`, so with `u = tan(θ/2)` the chart
/// metric is
/// `EG − F² = 64 r⁴ u² / ((1 + u²)⁴ (1 + v²)²)`,
/// which vanishes exactly on the polar line `u = 0`.
fn pole_sphere_egf2(radius: f64, d: IBox2) -> Option<CertifiedInterval> {
    let (u, v) = axes(d);
    let a = ci(1.0).add(&square(&u));
    let b = ci(1.0).add(&square(&v));
    let num = ci(64.0 * radius * radius * radius * radius).mul(&square(&u));
    let den = square(&square(&a)).mul(&square(&b));
    num.div(&den)
}

/// The certified first-derivative enclosures of the south stereographic
/// sphere over `d` (the §3.2 north closed forms with the `z` partials
/// reflected).
fn south_sphere_derivs(radius: f64, d: IBox2) -> DerivativeEnclosure {
    let (u, v) = axes(d);
    let u2 = square(&u);
    let v2 = square(&v);
    let weight = ci(1.0).add(&u2).add(&v2);
    let weight2 = square(&weight);
    let divided = |n: &CertifiedInterval| -> Option<CertifiedInterval> {
        let q = n.div(&weight2)?;
        Some(q.mul(&ci(radius)))
    };
    // x_u, y_u, z_u; x_v, y_v, z_v (unit-radius numerators).
    let su0 = divided(&ci(2.0).mul(&ci(1.0).sub(&u2).add(&v2)));
    let su1 = divided(&ci(-4.0).mul(&u).mul(&v));
    let su2 = divided(&ci(4.0).mul(&u));
    let sv0 = divided(&ci(-4.0).mul(&u).mul(&v));
    let sv1 = divided(&ci(2.0).mul(&ci(1.0).add(&u2).sub(&v2)));
    let sv2 = divided(&ci(4.0).mul(&v));
    match (su0, su1, su2, sv0, sv1, sv2) {
        (Some(su0), Some(su1), Some(su2), Some(sv0), Some(sv1), Some(sv2)) => DerivativeEnclosure {
            su: box3(su0, su1, su2),
            sv: box3(sv0, sv1, sv2),
        },
        _ => no_patch_derivs(),
    }
}

/// The certified first-derivative enclosures of a polar pole chart over `d`.
///
/// The partials come from the homogeneous quotient rule: for `q = N/W` with
/// the unit-radius numerators `N = (2u(1−v²), 4uv, ±(1−u²)(1+v²))` and the
/// weight `W = (1 + u²)(1 + v²)`,
/// `q_• = (N_• W − N W_•) / W²`, evaluated outward-rounded per box.
fn pole_sphere_derivs(radius: f64, sign: f64, d: IBox2) -> DerivativeEnclosure {
    let (u, v) = axes(d);
    let u2 = square(&u);
    let v2 = square(&v);
    let one = ci(1.0);
    let two = ci(2.0);
    let four = ci(4.0);
    let weight = one.add(&u2).mul(&one.add(&v2));
    let weight2 = square(&weight);

    // Numerator components and their partials.
    let nx = two.mul(&u).mul(&one.sub(&v2));
    let ny = four.mul(&u).mul(&v);
    let nz = ci(sign).mul(&one.sub(&u2)).mul(&one.add(&v2));
    let nxu = two.mul(&one.sub(&v2));
    let nxv = ci(-4.0).mul(&u).mul(&v);
    let nyu = four.mul(&v);
    let nyv = four.mul(&u);
    let nzu = ci(-2.0 * sign).mul(&u).mul(&one.add(&v2));
    let nzv = ci(2.0 * sign).mul(&v).mul(&one.sub(&u2));
    let wu = two.mul(&u).mul(&one.add(&v2));
    let wv = two.mul(&v).mul(&one.add(&u2));

    let axis = |n: &CertifiedInterval, nd: &CertifiedInterval, wd: &CertifiedInterval| {
        let num = nd.mul(&weight).sub(&n.mul(wd));
        num.div(&weight2)
    };
    let qx_u = axis(&nx, &nxu, &wu);
    let qy_u = axis(&ny, &nyu, &wu);
    let qz_u = axis(&nz, &nzu, &wu);
    let qx_v = axis(&nx, &nxv, &wv);
    let qy_v = axis(&ny, &nyv, &wv);
    let qz_v = axis(&nz, &nzv, &wv);
    let scaled = |q: Option<CertifiedInterval>| q.map(|q| q.mul(&ci(radius)));
    match (
        scaled(qx_u),
        scaled(qy_u),
        scaled(qz_u),
        scaled(qx_v),
        scaled(qy_v),
        scaled(qz_v),
    ) {
        (Some(su0), Some(su1), Some(su2), Some(sv0), Some(sv1), Some(sv2)) => DerivativeEnclosure {
            su: box3(su0, su1, su2),
            sv: box3(sv0, sv1, sv2),
        },
        _ => no_patch_derivs(),
    }
}

/// A normal cone for the sphere chart forms: the certified hemisphere about
/// the coordinate axis with the largest certified normal-margin over the box,
/// else the best-coordinate `π/2` cone (the crate's "certifies nothing,
/// subdivide" pattern, [`CertifiedPatch`] for a Bézier leaf).
fn hemisphere_cone(de: DerivativeEnclosure) -> Cone {
    let normal = cross_box(de.su, de.sv);
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
    match Cone::try_new(axis, std::f64::consts::FRAC_PI_2) {
        Ok(cone) => cone,
        Err(_) => Cone {
            axis,
            half_angle: std::f64::consts::FRAC_PI_2,
        },
    }
}

/// The interval cross product of two derivative boxes.
fn cross_box(a: IBox3, b: IBox3) -> IBox3 {
    let ax = CertifiedInterval {
        lo: a.lo[0],
        hi: a.hi[0],
    };
    let ay = CertifiedInterval {
        lo: a.lo[1],
        hi: a.hi[1],
    };
    let az = CertifiedInterval {
        lo: a.lo[2],
        hi: a.hi[2],
    };
    let bx = CertifiedInterval {
        lo: b.lo[0],
        hi: b.hi[0],
    };
    let by = CertifiedInterval {
        lo: b.lo[1],
        hi: b.hi[1],
    };
    let bz = CertifiedInterval {
        lo: b.lo[2],
        hi: b.hi[2],
    };
    let cx = ay.mul(&bz).sub(&az.mul(&by));
    let cy = az.mul(&bx).sub(&ax.mul(&bz));
    let cz = ax.mul(&by).sub(&ay.mul(&bx));
    IBox3 {
        lo: [cx.lo, cy.lo, cz.lo],
        hi: [cx.hi, cy.hi, cz.hi],
    }
}

// ---------------------------------------------------------------------------
// K2 lifts: the deck integer as a first-class coordinate
// ---------------------------------------------------------------------------

/// Unwrap the developed coordinate `raw_u` of a periodic chart into the deck
/// `floor(raw_u / period)` and the canonical `u ∈ [0, period)`.
///
/// The unwrap NEVER wraps an already-unwrapped coordinate: a `Param` at
/// `u = 5.9` (deck `0`) stays `u = 5.9`; the developed `6.4` unwraps to
/// `deck 1`, `u = 6.4 − period`. The deck integer, not a re-wrapped `u`,
/// carries the winding.
fn unwrap_deck(period: f64, raw_u: f64) -> (i32, f64) {
    let quotient = raw_u / period;
    let mut deck = quotient.floor() as i32;
    let mut u = raw_u - (deck as f64) * period;
    if u < 0.0 {
        deck -= 1;
        u += period;
    } else if u >= period {
        deck += 1;
        u -= period;
    }
    (deck, u)
}

/// The unwrapped pcurve lift: from the base `Param` on a periodic chart with
/// the exact generator `period`, lift the developed coordinate `raw_u`.
///
/// The result keeps `from`'s `v`, canonical `u` in `[0, period)`, and the
/// winding `deck`. The §0.4 ceiling is enforced per edge: `|deck − from.deck|
/// > DECK_MAX` refuses [`RefusalKind::DeckExhausted`] (Inconclusive).
pub fn lift_periodic(chart: ChartId, period: f64, from: &Param, raw_u: f64) -> Construction<Param> {
    if !raw_u.is_finite() {
        return Err(refusal(
            RefusalKind::NonFinite,
            "lift_raw_not_finite",
            format!("the developed coordinate {raw_u} is not finite"),
        ));
    }
    if from.chart != chart {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "lift_chart_mismatch",
            format!("the lift base {from:?} is not on chart {chart:?}"),
        ));
    }
    let (deck, u) = unwrap_deck(period, raw_u);
    let displacement = (deck as i64) - (from.deck as i64);
    if displacement.unsigned_abs() > DECK_MAX as u64 {
        return Err(Refusal::new(
            RefusalKind::DeckExhausted,
            RefusalEvidence::Predicate {
                name: "deck_exhausted_lift",
                detail: format!(
                    "the single edge {from:?} -> deck {deck} walks |{displacement}| \
                     deck crossings, above the per-edge ceiling {DECK_MAX}"
                ),
            },
        ));
    }
    Param::try_new(chart, deck, u, from.v)
}

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}
