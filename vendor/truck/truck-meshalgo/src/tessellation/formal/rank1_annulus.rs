//! The shared realization of a rank-one periodic annulus.
//!
//! # What this module is, and what it deliberately is not
//!
//! A surface with a **free** rank-one deck action develops into a chart with
//! one aperiodic coordinate and one periodic one. Two complete, oppositely
//! oriented, essential loops in that chart — one turn each, at two separated
//! values of the aperiodic coordinate — bound a compact annular strip. Cutting
//! that strip once transversally to the deck direction opens it into a single
//! rectangle-like patch; the patch triangulates in the plane; and an explicit
//! **edge identification**, discharged by merging vertex *identities* rather
//! than coordinates, glues the cut back and recovers the annulus.
//!
//! Every step of that paragraph is chart arithmetic. None of it knows what the
//! ambient surface is. So it lives here once, and both the cylinder
//! ([`super::cylinder_band`]) and the cone ([`super::cone_band`]) reach it.
//!
//! **What does not live here is admission.** This module never decides that a
//! face *is* an annulus. It is handed two already-developed boundary chains
//! and a statement of the obligations the ambient certifier discharged, and its
//! job is to realize the strip those chains bound — or to refuse. The semantic
//! gates are genuinely different on the two supports and stay in their own
//! modules:
//!
//! ```text
//! cylinder    two complete parallels of one embedded cylinder are either the
//!             same circle or separated along the axis; the support has no
//!             singular orbit anywhere, so no strip of it can contain one
//!
//! cone        the carriers live at signed generator coordinates whose radii
//!             differ; both must be proved to lie strictly on one nappe, and
//!             the apex — the one orbit where the deck action is not free —
//!             must be proved to lie outside the closed carrier interval
//! ```
//!
//! # Why the obligations are re-verified here rather than trusted
//!
//! [`RankOnePeriodicAnnulus`] carries the ambient certifier's conclusions as
//! typed values, not as booleans, and [`realize`] **checks the ones it can
//! check** before it builds anything: opposite primitive homology from the
//! chains themselves, a strictly positive carrier separation, and — where the
//! support has a singular orbit — that the singular coordinate really does lie
//! strictly outside the closed span of both chains. A cell that named an
//! obligation it had not discharged is refused with
//! [`AnnulusExit::ObligationNotDischarged`] rather than realized.
//!
//! That is not defence in depth for its own sake. It is what makes "the cone
//! reuses the cylinder's realizer" a safe sentence: the realizer's own
//! preconditions are checked against the data it was actually given, so
//! reaching it through a new ambient adapter cannot smuggle in a fact that
//! adapter never proved.
//!
//! # Why the patch is not ear-clipped
//!
//! [`super::planar_slice::triangulate`] is reused everywhere the formal
//! subtree needs a *disk*, and this module reuses its certificate
//! ([`certified_polygonal_region`]) and its checker ([`final_validity`])
//! unchanged. It cannot supply the patch's triangles, for a structural
//! reason rather than a stylistic one: ear clipping a convex patch fans it
//! from one vertex, and that apex is one of the two developed lifts of a cut
//! vertex, so the fan always contains a triangle carrying *both* lifts. That
//! triangle is degenerate the moment the identification is discharged.
//! [`triangulate_annulus_patch`] walks the two boundary chains instead, which
//! by construction never puts both lifts of one cut vertex in one triangle —
//! and its output is then handed to the *same* [`final_validity`] battery, so
//! the strengthening is in the producer only, never in the checking.

use std::collections::{BTreeMap, BTreeSet};

use super::super::source_evidence::{BoundId, EdgeUseId, SourceVertexKey};
use super::deck::{
    solve_axis_aligned, DeckGenerator, DeckInterval, DeckOperationalFailure, DeckSolveResult,
    DevelopedBox,
};
use super::numeric::NonNegativeFinite;
use super::planar_slice::{
    bounded_material_region_of, certified_polygonal_region, final_validity, jordan_arrangement_of,
    CertificateRoute, CertifiedPlanarCurveOccurrence, CertifiedPolygonalRegion,
    FinalValidityReport, Rank0DevelopedBoundary, Rank0Displacement, SliceCategory, SliceExit,
    TriangulatedRegion,
};
use truck_geometry::prelude::{Point2, Point3};

/// The bound identity the two artificial cut sides are recorded under.
///
/// The cut sides are *not* source boundaries and must never be mistaken for
/// one, so they carry an identity no source bound can hold: source bounds are
/// indexed from zero by their position in the face, so `usize::MAX` is
/// unreachable for them by construction. Everything downstream that keys on
/// [`EdgeUseId`] therefore sees the cut for what it is.
pub const CUT_BOUND: BoundId = BoundId(usize::MAX);

/// The edge-use identity of the patch's right (leading) cut side.
pub const RIGHT_CUT: EdgeUseId = EdgeUseId::new(CUT_BOUND, 0);

/// The edge-use identity of the patch's left (trailing) cut side.
pub const LEFT_CUT: EdgeUseId = EdgeUseId::new(CUT_BOUND, 1);

/// The largest number of segments one developed arc is subdivided into.
///
/// The subdivision exists so the *lifted* chord stays within tolerance of the
/// physical circle (see [`arc_segment_count`]); the cap bounds the cost of
/// [`jordan_arrangement_of`]'s exhaustive `O(n^2)` pairwise certification on a
/// pathological tolerance, and a face that would need more is refused by the
/// polygonal certificate rather than approximated more coarsely in silence.
pub const MAXIMUM_ARC_SEGMENTS: usize = 512;

/// The fewest segments each boundary chain is cut into.
///
/// Three, for a topological reason and not a quality one. A chain with a
/// single segment puts its two endpoints — which are the two developed lifts
/// of one cut vertex — in one triangle of *any* triangulation, and that
/// triangle collapses when the identification is discharged. Two is already
/// enough to prevent it; three leaves a margin. Any physical tolerance asks
/// for far more than this, so the floor only guards a degenerate one.
pub const MINIMUM_CHAIN_SEGMENTS: usize = 3;

// ---------------------------------------------------------------------------
// Boundary components
// ---------------------------------------------------------------------------

/// One authoritative bound, developed into one complete simple essential loop
/// with a certified primitive homology.
///
/// Everything the source declared survives: the [`BoundId`], the ordered
/// [`EdgeUseId`] sequence, and the source vertex identities the traversal
/// joined on. Nothing here is keyed on a coordinate.
///
/// The chart is `(aperiodic, periodic)`: `x` is the support's aperiodic
/// coordinate (a cylinder's axial coordinate, a cone's signed generator
/// coordinate) and `y` the developed periodic one, left unwrapped.
#[derive(Debug, Clone)]
pub struct CompleteParallel {
    /// Which authoritative bound this is.
    pub bound: BoundId,
    /// The ordered edge-use identities, in source cyclic order.
    pub edge_uses: Vec<EdgeUseId>,
    /// The source vertices the traversal starts each occurrence at, in the
    /// same order.
    pub start_vertices: Vec<SourceVertexKey>,
    /// The developed start point of each occurrence, already placed on its
    /// certified deck copy: `(aperiodic, periodic)`, periodic unwrapped.
    pub starts: Vec<Point2>,
    /// The developed end point of the *last* occurrence, placed. This is the
    /// second developed lift of `starts[0]`'s source vertex, one full deck
    /// period away — the boundary is closed because `end = start + h g`, not
    /// because a segment joins them.
    pub terminal: Point2,
    /// The certified terminal holonomy: `+1` or `-1`.
    pub homology: i64,
}

impl CompleteParallel {
    /// The developed periodic coordinate this component's traversal begins at.
    pub fn start_angular(&self) -> f64 {
        self.starts[0].y
    }

    /// The source vertex the cut may be taken at: the first occurrence's
    /// start vertex.
    pub fn cut_vertex(&self) -> SourceVertexKey {
        self.start_vertices[0]
    }

    /// Every source vertex this component visits.
    pub fn source_vertices(&self) -> BTreeSet<SourceVertexKey> {
        self.start_vertices.iter().copied().collect()
    }

    /// The lowest developed periodic coordinate anywhere on the component.
    pub fn lowest_angular(&self) -> f64 {
        self.starts
            .iter()
            .chain(std::iter::once(&self.terminal))
            .fold(f64::INFINITY, |low, point| low.min(point.y))
    }

    /// The observed extent of the component's aperiodic coordinate, as
    /// `(low, high)` over every developed endpoint it visits.
    pub fn aperiodic_extent(&self) -> (f64, f64) {
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for point in self.starts.iter().chain(std::iter::once(&self.terminal)) {
            low = low.min(point.x);
            high = high.max(point.x);
        }
        (low, high)
    }
}

// ---------------------------------------------------------------------------
// The realization contract
// ---------------------------------------------------------------------------

/// How the two carriers were proved distinct and ordered.
///
/// An enum with one variant rather than a bare pair, so that admitting a
/// second way to order two carriers is a visible decision at this type instead
/// of a quiet widening of an existing one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CarrierOrder {
    /// The two carriers' certified enclosures of the aperiodic coordinate are
    /// strictly disjoint, so the carriers cannot be the same loop and are
    /// strictly ordered.
    DisjointEnclosures {
        /// `true` when the first (source-order) boundary is the lower one.
        first_is_lower: bool,
        /// The certified gap between the two enclosures. Must be `> 0`.
        separation: f64,
    },
}

/// Why the deck action is free on the whole closed strip.
///
/// This is the obligation with no counterpart in the cylinder work, and it is
/// the reason the two cells cannot share one admission rule. Where the deck
/// action is not free the angular orbit collapses, the rank-one annulus chart
/// degenerates, and the strip is not an annulus at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FreeDeckAction {
    /// The support has **no** singular orbit anywhere: every orbit of the deck
    /// action is a regular loop, at every value of the aperiodic coordinate.
    ///
    /// An embedded cylinder of certified positive radius is this case, and it
    /// is why the cylinder band never had to prove apex exclusion: there is
    /// nothing to exclude.
    GloballyRegularSupport,
    /// The support has exactly one singular orbit, at a named value of the
    /// aperiodic chart coordinate, and the closed carrier interval was proved
    /// to exclude it.
    ///
    /// A cone's apex is this case. [`realize`] re-checks the exclusion against
    /// the chains it was actually handed, so naming this variant does not by
    /// itself buy admission.
    OneSingularOrbitExcluded {
        /// The aperiodic chart coordinate of the singular orbit.
        singular_coordinate: f64,
        /// The certified clearance between the singular coordinate and the
        /// nearer carrier enclosure. Must be `> 0`.
        clearance: f64,
    },
}

/// Which named atlas cell's certificate fixes the material region.
///
/// Carried so the realized annulus can say *what it is*, and so a consumer can
/// tell a cylinder band from a conical band without re-deriving it. The cell's
/// own module is where the standing was actually decided — see
/// [`super::cylinder_band::band_material_authority`] and
/// [`super::cone_band::conical_band_material_authority`]; this is the label
/// that travels with the result, never a substitute for either.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnnulusCell {
    /// [`super::cylinder_band::CertifiedCylinderBand`].
    CylinderEssentialBand,
    /// [`super::cone_band::CertifiedConicalEssentialBand`].
    ConicalEssentialBand,
}

impl AnnulusCell {
    /// A short stable tag, for probe and census records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::CylinderEssentialBand => "cylinder_essential_band",
            Self::ConicalEssentialBand => "conical_essential_band",
        }
    }
}

/// One of the annulus's two boundary components, with the physical scale its
/// own subdivision is measured against.
#[derive(Debug, Clone, Copy)]
pub struct AnnulusBoundary<'a> {
    /// The developed component.
    pub parallel: &'a CompleteParallel,
    /// The radius of the physical circle this component is carried by.
    ///
    /// Its own, not the support's: on a cylinder both carriers share the
    /// support's single radius, but on a cone the two carriers have different
    /// radii and each chain's chord must be subdivided against the one it
    /// actually lies on.
    pub carrier_radius: f64,
}

/// Everything the shared realizer needs, and nothing else.
///
/// Constructed by an ambient cell's own route — [`super::cylinder_band::run_cylinder_band`]
/// and [`super::cone_band::run_conical_essential_band`] — each of which holds a
/// completed cell certificate at the point it builds one. The two boundaries
/// are in the **source's own bound order**, which is the order the cut-open
/// patch traverses them in.
#[derive(Debug, Clone, Copy)]
pub struct RankOnePeriodicAnnulus<'a> {
    /// The earlier-numbered source bound's component.
    pub first: AnnulusBoundary<'a>,
    /// The later-numbered source bound's component.
    pub second: AnnulusBoundary<'a>,
    /// The support's signed deck period, on the chart's periodic axis.
    pub period: f64,
    /// How the carriers were proved distinct and ordered.
    pub carrier_order: CarrierOrder,
    /// Why the deck action is free on the closed strip between them.
    pub free_deck_action: FreeDeckAction,
    /// Which cell certified the material region.
    pub cell: AnnulusCell,
}

/// Which named obligation a caller claimed but did not discharge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnnulusObligation {
    /// The two components' homologies are not opposite primitives.
    OppositePrimitiveHomology,
    /// The claimed carrier separation is not strictly positive.
    StrictCarrierSeparation,
    /// The claimed singular orbit is not strictly outside the closed span of
    /// both developed chains, or its clearance is not strictly positive.
    SingularOrbitOutsideClosedStrip,
}

impl AnnulusObligation {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::OppositePrimitiveHomology => "annulus_opposite_primitive_homology",
            Self::StrictCarrierSeparation => "annulus_strict_carrier_separation",
            Self::SingularOrbitOutsideClosedStrip => "annulus_singular_orbit_in_closed_strip",
        }
    }
}

/// Every way the shared realization can fail.
///
/// Admission failures are *not* here: a face that is not an annulus never
/// reaches this module, and the cell that refused it names the reason in its
/// own vocabulary. What is here is a caller that claimed an obligation it did
/// not hold, and the realization stages themselves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnnulusExit {
    /// The caller presented a contract whose own named obligation does not
    /// hold on the data it handed over. A defect in the calling cell.
    ObligationNotDischarged {
        /// Which one.
        obligation: AnnulusObligation,
    },
    /// No authoritative source endpoint offered a uniquely certified periodic
    /// coordinate to cut at.
    CutCoordinateUnavailable,
    /// The cut-open patch failed one of the reused planar obligations —
    /// Jordan simplicity, material selection, the polygonal certificate, or
    /// the final validity battery. Carries that stage's exit unchanged.
    Patch(SliceExit),
    /// The identification collapsed a triangle: two of its corners are the
    /// same regluded vertex.
    RegluedDegenerateTriangle,
    /// An artificial cut edge survived the identification as a physical mesh
    /// boundary.
    RegluedCutSurvives,
    /// The regluded complex is not connected.
    RegluedNotConnected,
    /// The regluded complex does not have exactly two boundary components.
    RegluedBoundaryComponents {
        /// How many it has.
        components: usize,
    },
    /// The regluded complex's Euler characteristic is not zero.
    RegluedEulerCharacteristic {
        /// What it is.
        characteristic: i64,
    },
    /// Two triangles traverse a shared edge in the same direction, so the
    /// regluded complex is not consistently oriented.
    RegluedOrientationInconsistent,
    /// A lifted vertex was not finite.
    LiftNotFinite,
}

impl AnnulusExit {
    /// Which semantic category this exit belongs to.
    ///
    /// Every variant here is `OperationalFailure` or forwards one, with the
    /// single exception of the cut coordinate, which is missing *evidence*
    /// about the source rather than a defect in this module. A reglue
    /// predicate failing after every input obligation was discharged is a
    /// defect here, not a verdict about the face; and an obligation the caller
    /// claimed but does not hold is a defect in the caller.
    pub fn category(self) -> SliceCategory {
        match self {
            Self::CutCoordinateUnavailable => SliceCategory::Unresolved,
            Self::Patch(exit) => exit.category(),
            Self::ObligationNotDischarged { .. }
            | Self::RegluedDegenerateTriangle
            | Self::RegluedCutSurvives
            | Self::RegluedNotConnected
            | Self::RegluedBoundaryComponents { .. }
            | Self::RegluedEulerCharacteristic { .. }
            | Self::RegluedOrientationInconsistent
            | Self::LiftNotFinite => SliceCategory::OperationalFailure,
        }
    }

    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::ObligationNotDischarged { obligation } => obligation.tag(),
            Self::CutCoordinateUnavailable => "band_cut_coordinate_unavailable",
            Self::Patch(exit) => exit.tag(),
            Self::RegluedDegenerateTriangle => "band_reglue_degenerate_triangle",
            Self::RegluedCutSurvives => "band_reglue_cut_survives",
            Self::RegluedNotConnected => "band_reglue_not_connected",
            Self::RegluedBoundaryComponents { .. } => "band_reglue_boundary_components",
            Self::RegluedEulerCharacteristic { .. } => "band_reglue_euler_characteristic",
            Self::RegluedOrientationInconsistent => "band_reglue_orientation_inconsistent",
            Self::LiftNotFinite => "band_lift_not_finite",
        }
    }

    /// The stage this exit left from, for the funnel.
    pub fn stage(self) -> &'static str {
        match self {
            Self::ObligationNotDischarged { .. } => "contract",
            Self::CutCoordinateUnavailable => "plan",
            Self::Patch(_) => "patch",
            Self::RegluedDegenerateTriangle
            | Self::RegluedCutSurvives
            | Self::RegluedNotConnected
            | Self::RegluedBoundaryComponents { .. }
            | Self::RegluedEulerCharacteristic { .. }
            | Self::RegluedOrientationInconsistent
            | Self::LiftNotFinite => "reglue",
        }
    }
}

// ---------------------------------------------------------------------------
// Contract verification
// ---------------------------------------------------------------------------

/// Re-check the contract's own named obligations against the data it carries.
///
/// See the module docs for why this exists. Each check is a restatement of an
/// obligation the ambient cell already discharged, evaluated against the
/// developed chains actually presented — so a cell that named a fact it did
/// not prove is caught here rather than realized.
fn verify_contract(annulus: &RankOnePeriodicAnnulus<'_>) -> Result<(), AnnulusExit> {
    let refuse = |obligation| Err(AnnulusExit::ObligationNotDischarged { obligation });

    // Opposite primitive homology, read off the chains themselves rather than
    // taken on the caller's word. `±1` each and summing to zero is the whole
    // statement: `0` is contractible and `|h| > 1` is multiply wound.
    let (a, b) = (
        annulus.first.parallel.homology,
        annulus.second.parallel.homology,
    );
    if a.abs() != 1 || b.abs() != 1 || a + b != 0 {
        return refuse(AnnulusObligation::OppositePrimitiveHomology);
    }

    let CarrierOrder::DisjointEnclosures { separation, .. } = annulus.carrier_order;
    if !(separation > 0.0) || !separation.is_finite() {
        return refuse(AnnulusObligation::StrictCarrierSeparation);
    }

    if let FreeDeckAction::OneSingularOrbitExcluded {
        singular_coordinate,
        clearance,
    } = annulus.free_deck_action
    {
        if !(clearance > 0.0) || !clearance.is_finite() || !singular_coordinate.is_finite() {
            return refuse(AnnulusObligation::SingularOrbitOutsideClosedStrip);
        }
        // The closed strip spans every aperiodic coordinate either chain
        // presents; the singular orbit must lie strictly outside all of it.
        let (first_low, first_high) = annulus.first.parallel.aperiodic_extent();
        let (second_low, second_high) = annulus.second.parallel.aperiodic_extent();
        let low = first_low.min(second_low);
        let high = first_high.max(second_high);
        if !(singular_coordinate < low || singular_coordinate > high) {
            return refuse(AnnulusObligation::SingularOrbitOutsideClosedStrip);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Cut-open construction
// ---------------------------------------------------------------------------

/// The explicit identification of the patch's two artificial sides.
///
/// The two sides are the two developed lifts of one cut arc, so the pairing
/// is `(left, v) ~ (right, v)` vertex by vertex. Each pair names two cycle
/// positions that are the two lifts of *one source vertex*, one deck period
/// apart — which is why discharging the identification is a statement about
/// identity and not about proximity.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeIdentification {
    /// The identified vertex pairs, as `(left cycle index, right cycle
    /// index)`.
    pub pairs: Vec<(usize, usize)>,
    /// The cycle position of the right cut segment.
    pub right_segment: usize,
    /// The cycle position of the left cut segment.
    pub left_segment: usize,
}

/// The plan for cutting one certified annulus open into a single patch.
#[derive(Debug, Clone)]
pub struct CutOpenDomainPlan {
    /// The periodic coordinate the annulus is cut at.
    pub cut_angular: f64,
    /// The authoritative source vertex that coordinate was read from.
    pub cut_vertex: SourceVertexKey,
    /// The deck shift applied to the first (source-order) component, in
    /// periods.
    pub first_shift: i64,
    /// The deck shift applied to the second component, in periods.
    pub second_shift: i64,
}

/// Choose the annulus's cut, deterministically and from source data alone.
///
/// The cut coordinate is the periodic coordinate of the lexicographically
/// earliest authoritative source endpoint that has a uniquely certified one:
/// bounds are ordered by their source position, occurrences by their position
/// within the bound, so the earliest such endpoint is the first occurrence of
/// the earlier bound. No centroid, no midpoint, and no geometric optimisation
/// enters the choice — two runs over the same file cut in the same place.
///
/// Each component is then placed in the single deck copy that carries it into
/// the period window *starting at the cut*: the copy whose lowest developed
/// periodic coordinate lies in `[cut, cut + period)`. Both components then
/// cover the same turn of the support, which is what "cut here" means — the
/// component that supplied the cut lands on `[cut, cut + period]` exactly,
/// whichever way round it winds.
///
/// This is a placement, not a re-derivation: it shifts a whole certified
/// chain by an integer number of periods and changes no coordinate's
/// relationship to any other. Choosing the copy by the chain's *start*
/// instead would put a component that winds the other way a full period below
/// the cut, which is a legal developed image of the same annulus but a useless
/// one — the two chains would then share no periodic extent at all.
///
/// The cut is an artifact of the quotient, not source material authority: it
/// carries [`CUT_BOUND`], no source bound can hold that identity, and
/// [`reglue`] removes it again.
pub fn plan_cut_open(
    first: &CompleteParallel,
    second: &CompleteParallel,
    period: f64,
) -> Result<CutOpenDomainPlan, AnnulusExit> {
    let cut_vertex = first.cut_vertex();
    if !cut_vertex.is_identified() {
        return Err(AnnulusExit::CutCoordinateUnavailable);
    }
    let cut_angular = first.start_angular();
    if !cut_angular.is_finite() {
        return Err(AnnulusExit::CutCoordinateUnavailable);
    }
    let magnitude = period.abs();
    let shift_of = |low: f64| -> i64 { -(((low - cut_angular) / magnitude).floor() as i64) };
    Ok(CutOpenDomainPlan {
        cut_angular,
        cut_vertex,
        first_shift: shift_of(first.lowest_angular()),
        second_shift: shift_of(second.lowest_angular()),
    })
}

/// The cut-open planar patch: one rectangle-like domain in the developed
/// chart, certified simple, with its material region selected and its
/// artificial sides recorded as an identification rather than as boundary.
#[derive(Debug, Clone)]
pub struct PlanarPatch {
    /// The certified polygonal region, through the reused planar
    /// certificates.
    pub region: CertifiedPolygonalRegion,
    /// The identification of the two artificial cut sides.
    pub identification: EdgeIdentification,
    /// How many vertices of the cycle belong to the lower-in-cycle-order
    /// chain, counting both lifts of its cut vertex.
    pub first_chain_vertices: usize,
    /// The sign of the periodic direction both chains advance in.
    pub direction: f64,
}

/// How many segments one developed arc is cut into so that its *lifted* chord
/// stays within `tolerance` of the physical circle.
///
/// A chord subtending `delta` on a circle of radius `r` departs from it by at
/// most the sagitta `r (1 - cos(delta / 2))`, so `k` segments over a sweep
/// `s` need `r (1 - cos(s / 2k)) <= tolerance`, i.e. `k >= s / (2 acos(1 -
/// tolerance / r))`. That is a bound on the realized geometry, derived from
/// the certified radius and the caller's own tolerance; it is not a shape
/// heuristic and it does not touch the developed chart, where the arc is
/// exactly the straight segment the witness already represents.
pub fn arc_segment_count(sweep: f64, radius: f64, tolerance: f64, minimum: usize) -> usize {
    let minimum = minimum.max(1);
    let sweep = sweep.abs();
    if !sweep.is_finite() || !radius.is_finite() || radius <= 0.0 {
        return MAXIMUM_ARC_SEGMENTS;
    }
    if !(tolerance > 0.0) {
        return MAXIMUM_ARC_SEGMENTS;
    }
    let cosine = (1.0 - tolerance / radius).clamp(-1.0, 1.0);
    let half = cosine.acos();
    let needed = match half > 0.0 {
        true => (sweep / (2.0 * half)).ceil(),
        false => MAXIMUM_ARC_SEGMENTS as f64,
    };
    if !needed.is_finite() || needed >= MAXIMUM_ARC_SEGMENTS as f64 {
        return MAXIMUM_ARC_SEGMENTS.max(minimum);
    }
    (needed as usize).clamp(minimum, MAXIMUM_ARC_SEGMENTS)
}

/// Subdivide one placed developed arc into `count` collinear points.
///
/// The developed image of a loop at constant aperiodic coordinate *is* a
/// straight segment, so every interpolated point lies exactly on it: the
/// subdivision adds resolution to the lift without adding any approximation to
/// the developed chart. That is why the patch's polygonal certificate stays
/// exact, and why [`certified_polygonal_region`]'s exactness guard is
/// satisfied here for the same reason it is for a line-bounded planar face,
/// not by an exemption.
fn subdivide(start: Point2, end: Point2, count: usize) -> Vec<Point2> {
    let mut points = Vec::with_capacity(count + 1);
    points.push(start);
    for step in 1..count {
        let t = step as f64 / count as f64;
        points.push(Point2::new(
            start.x + t * (end.x - start.x),
            start.y + t * (end.y - start.y),
        ));
    }
    points.push(end);
    points
}

/// Build one chain's placed, subdivided occurrences.
///
/// Each occurrence runs from its own placed developed start to the *next*
/// occurrence's placed developed start — the terminal for the last one — so
/// the chain is closed by the source's own vertex identities and by the deck
/// placement, never by reconciling two independently rounded copies of one
/// endpoint.
fn chain_occurrences(
    parallel: &CompleteParallel,
    shift: f64,
    radius: f64,
    tolerance: f64,
) -> Vec<CertifiedPlanarCurveOccurrence> {
    let count = parallel.starts.len();
    let minimum_per_occurrence = MINIMUM_CHAIN_SEGMENTS.div_ceil(count);
    let shifted = |p: Point2| Point2::new(p.x, p.y + shift);
    (0..count)
        .map(|index| {
            let start = shifted(parallel.starts[index]);
            let end = match index + 1 == count {
                true => shifted(parallel.terminal),
                false => shifted(parallel.starts[index + 1]),
            };
            let segments =
                arc_segment_count(end.y - start.y, radius, tolerance, minimum_per_occurrence);
            CertifiedPlanarCurveOccurrence {
                edge_use: parallel.edge_uses[index],
                start_vertex: parallel.start_vertices[index],
                end_vertex: match index + 1 == count {
                    true => parallel.start_vertices[0],
                    false => parallel.start_vertices[index + 1],
                },
                points: subdivide(start, end, segments),
                route: CertificateRoute::AnalyticCylinderDevelopment,
                endpoint_reconciliation: NonNegativeFinite::new(0.0)
                    .expect("zero is a valid nonnegative bound"),
            }
        })
        .collect()
}

/// One artificial cut side, as an occurrence.
///
/// It carries [`CUT_BOUND`], which no source bound can hold, so nothing
/// downstream can mistake it for a physical boundary. It exists only to close
/// the patch's cycle and is removed again by the identification.
fn cut_occurrence(id: EdgeUseId, from: Point2, to: Point2) -> CertifiedPlanarCurveOccurrence {
    CertifiedPlanarCurveOccurrence {
        edge_use: id,
        start_vertex: SourceVertexKey::Absent,
        end_vertex: SourceVertexKey::Absent,
        points: vec![from, to],
        route: CertificateRoute::AnalyticCylinderDevelopment,
        endpoint_reconciliation: NonNegativeFinite::new(0.0)
            .expect("zero is a valid nonnegative bound"),
    }
}

/// Cut a certified annulus open into one planar patch.
///
/// The cycle is, in order: the first component's physical boundary, the right
/// artificial cut side, the second component's physical boundary (which runs
/// in the reverse periodic direction, since its homology is the opposite one),
/// and the left artificial cut side. The two cut sides are exactly one deck
/// period apart and are the two lifts of one cut arc — a fact this function
/// establishes by *construction*, not by measuring them afterwards.
///
/// [`jordan_arrangement_of`] and [`bounded_material_region_of`] then certify
/// the patch simple and select its bounded complementary component, with the
/// same arithmetic as the planar slice. That component is the compact strip
/// between the two carriers, so the material region is established **before**
/// any triangle exists.
///
/// The difference from the planar slice is *where the standing to select it
/// comes from*, and that question is settled by the calling cell before this
/// function is reached — see [`super::cylinder_band::band_material_authority`]
/// and [`super::cone_band::conical_band_material_authority`].
pub fn cut_open(
    annulus: &RankOnePeriodicAnnulus<'_>,
    plan: &CutOpenDomainPlan,
    tolerance: f64,
) -> Result<PlanarPatch, AnnulusExit> {
    let magnitude = annulus.period.abs();
    let (first, second) = (annulus.first.parallel, annulus.second.parallel);

    let first_shift = plan.first_shift as f64 * magnitude;
    let second_shift = plan.second_shift as f64 * magnitude;

    let first_chain =
        chain_occurrences(first, first_shift, annulus.first.carrier_radius, tolerance);
    let second_chain = chain_occurrences(
        second,
        second_shift,
        annulus.second.carrier_radius,
        tolerance,
    );

    let first_terminal = Point2::new(first.terminal.x, first.terminal.y + first_shift);
    let first_origin = Point2::new(first.starts[0].x, first.starts[0].y + first_shift);
    let second_terminal = Point2::new(second.terminal.x, second.terminal.y + second_shift);
    let second_origin = Point2::new(second.starts[0].x, second.starts[0].y + second_shift);

    let mut occurrences = first_chain;
    let first_chain_segments: usize = occurrences
        .iter()
        .map(|occurrence| occurrence.points.len() - 1)
        .sum();
    occurrences.push(cut_occurrence(RIGHT_CUT, first_terminal, second_origin));
    let second_chain_segments: usize = second_chain
        .iter()
        .map(|occurrence| occurrence.points.len() - 1)
        .sum();
    occurrences.extend(second_chain);
    occurrences.push(cut_occurrence(LEFT_CUT, second_terminal, first_origin));

    let arrangement = jordan_arrangement_of(&occurrences).map_err(AnnulusExit::Patch)?;
    let material = bounded_material_region_of(arrangement).map_err(AnnulusExit::Patch)?;

    let developed = Rank0DevelopedBoundary {
        displacements: vec![Rank0Displacement; occurrences.len()],
        occurrences,
    };
    let region =
        certified_polygonal_region(material, &developed, tolerance).map_err(AnnulusExit::Patch)?;

    // Cycle layout, by construction: the first chain contributes one vertex
    // per segment starting at index 0, the right cut contributes the first
    // chain's terminal, the second chain follows, and the left cut
    // contributes the second chain's terminal last.
    let first_terminal_index = first_chain_segments;
    let second_origin_index = first_chain_segments + 1;
    let second_terminal_index = first_chain_segments + 1 + second_chain_segments;

    Ok(PlanarPatch {
        region,
        identification: EdgeIdentification {
            // `(left, v) ~ (right, v)`: the left side runs from the second
            // chain's terminal to the first chain's origin, the right side
            // from the first chain's terminal to the second chain's origin.
            // Each pair is two developed lifts of one source vertex.
            pairs: vec![
                (second_terminal_index, second_origin_index),
                (0, first_terminal_index),
            ],
            right_segment: first_terminal_index,
            left_segment: second_terminal_index,
        },
        first_chain_vertices: first_chain_segments + 1,
        direction: first.homology as f64 * annulus.period.signum(),
    })
}

// ---------------------------------------------------------------------------
// Realization
// ---------------------------------------------------------------------------

/// Triangulate the cut-open patch by walking its two boundary chains.
///
/// The patch is a rectangle-like domain between two chains that are each
/// strictly monotone in the periodic coordinate and separated in the aperiodic
/// one, both certified upstream. Advancing whichever chain's next vertex
/// comes first in that shared direction sweeps the patch exactly once, giving
/// `n - 2` triangles over the polygon's own vertices with no Steiner point —
/// the same shape [`final_validity`] checks a reused ear-clipped disk against,
/// which is why that battery is applied to this output unchanged.
///
/// Why not [`super::planar_slice::triangulate`]: see the module docs. Its fan
/// output always carries both developed lifts of one cut vertex in a single
/// triangle, which the identification then collapses.
pub fn triangulate_annulus_patch(patch: &PlanarPatch) -> Result<TriangulatedRegion, AnnulusExit> {
    let vertices = patch.region.region.boundary.cycle.clone();
    let total = vertices.len();
    let first_count = patch.first_chain_vertices;
    if first_count < 3 || total < first_count + 3 {
        return Err(AnnulusExit::Patch(SliceExit::TriangulationDidNotComplete));
    }

    // The first chain in cycle order; the second chain reversed, so both run
    // in the same periodic direction.
    let lower: Vec<usize> = (0..first_count).collect();
    let upper: Vec<usize> = (first_count..total).rev().collect();

    let progress = |index: usize| patch.direction * vertices[index].y;

    let mut triangles = Vec::with_capacity(total - 2);
    let (mut i, mut j) = (0usize, 0usize);
    while i + 1 < lower.len() || j + 1 < upper.len() {
        // The two diagonals the identification cannot survive. `lower[0]` is
        // identified with `lower[nl - 1]` and `upper[0]` with `upper[nu - 1]`,
        // so a triangle edge joining `lower[0]` to `upper[nu - 1]`, or
        // `lower[nl - 1]` to `upper[0]`, becomes a *third* copy of the cut
        // edge once the identification is discharged. Each arises only by
        // running one chain to its end before the other has moved at all, so
        // each is excluded by holding the last step of a chain back until the
        // other chain has taken one. The strip is unaffected everywhere else:
        // the guards can only fire on a patch whose two chains barely overlap
        // in the periodic coordinate, and the reused validity battery still
        // checks the result.
        let lower_would_finish_alone = i + 2 == lower.len() && j == 0 && upper.len() >= 2;
        let upper_would_finish_alone = j + 2 == upper.len() && i == 0 && lower.len() >= 2;
        let advance_lower = if i + 1 >= lower.len() {
            false
        } else if j + 1 >= upper.len() {
            true
        } else if lower_would_finish_alone {
            false
        } else if upper_would_finish_alone {
            true
        } else {
            progress(lower[i + 1]) <= progress(upper[j + 1])
        };
        match advance_lower {
            true => {
                triangles.push([lower[i], lower[i + 1], upper[j]]);
                i += 1;
            }
            false => {
                triangles.push([lower[i], upper[j + 1], upper[j]]);
                j += 1;
            }
        }
    }

    // Match the cycle's own handedness, so the complex is oriented the way
    // the certified region is. The walk emits one consistent handedness; this
    // only chooses which.
    let reference = patch.region.region.signed_area;
    if let Some(first) = triangles.first() {
        let orientation = orient(&vertices, *first);
        if orientation * reference < 0.0 {
            for triangle in &mut triangles {
                triangle.swap(1, 2);
            }
        }
    }

    Ok(TriangulatedRegion {
        vertices,
        triangles,
    })
}

fn orient(vertices: &[Point2], triangle: [usize; 3]) -> f64 {
    let [a, b, c] = triangle;
    let (a, b, c) = (vertices[a], vertices[b], vertices[c]);
    robust::orient2d(
        robust::Coord { x: a.x, y: a.y },
        robust::Coord { x: b.x, y: b.y },
        robust::Coord { x: c.x, y: c.y },
    )
}

/// What the regluded annular complex was proved to be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnnulusValidityReport {
    /// Triangles.
    pub triangles: usize,
    /// Distinct vertices after the identification.
    pub vertices: usize,
    /// Edges with exactly one incident triangle.
    pub boundary_edges: usize,
    /// Edges with exactly two.
    pub interior_edges: usize,
    /// Connected components of the boundary. Two, for an annulus.
    pub boundary_components: usize,
    /// `V - E + F`. Zero, for an annulus.
    pub euler_characteristic: i64,
}

/// A realized rank-one periodic annulus: the validated annular mesh.
#[derive(Debug, Clone)]
pub struct RealizedAnnulus {
    /// Which cell certified the material region.
    pub cell: AnnulusCell,
    /// The developed complex, after the identification was discharged.
    pub developed: TriangulatedRegion,
    /// The cut-open patch's own final validity report, before the reglue.
    pub patch_validity: FinalValidityReport,
    /// The annular complex's validity report, after it.
    pub validity: AnnulusValidityReport,
    /// The developed vertices lifted onto the support, in the same order as
    /// `developed.vertices`.
    pub physical_vertices: Vec<Point3>,
}

/// Discharge the identification and validate the annulus.
///
/// The merge is by *identity*: each pair names two cycle positions the
/// construction already knows to be the two developed lifts of one source
/// vertex, and the right-hand one is rewritten to the left-hand one. No
/// coordinate is compared, no proximity threshold exists, and two vertices
/// that happen to coincide are not merged unless the identification says so.
///
/// The artificial sides disappear as a consequence, not as a separate step:
/// once both their endpoint pairs are identified they are the same edge, and
/// that edge then carries two triangles instead of one, so it is interior.
/// The check that it really did is [`AnnulusExit::RegluedCutSurvives`].
///
/// `lift` is the ambient support's own developed-to-physical map: the one
/// place in the realization that knows what the surface is. Every vertex it
/// is handed lies on one of the two carriers by construction — the walk
/// introduces no Steiner point — so the lift is exact wherever it is applied.
pub fn reglue(
    patch: &PlanarPatch,
    developed: TriangulatedRegion,
    cell: AnnulusCell,
    lift: &impl Fn(&TriangulatedRegion) -> Vec<Point3>,
) -> Result<RealizedAnnulus, AnnulusExit> {
    let patch_validity = final_validity(&developed, &patch.region).map_err(AnnulusExit::Patch)?;

    let total = developed.vertices.len();
    let mut representative: Vec<usize> = (0..total).collect();
    for &(left, right) in &patch.identification.pairs {
        if left >= total || right >= total {
            return Err(AnnulusExit::RegluedCutSurvives);
        }
        let (keep, drop) = (left.min(right), left.max(right));
        representative[drop] = keep;
    }
    // One pass suffices: no pair's target is itself identified away, because
    // the four cut endpoints are four distinct cycle positions forming two
    // disjoint pairs.
    let mut compacted: Vec<Option<usize>> = vec![None; total];
    let mut vertices = Vec::with_capacity(total);
    for index in 0..total {
        if representative[index] == index {
            compacted[index] = Some(vertices.len());
            vertices.push(developed.vertices[index]);
        }
    }
    let remap = |index: usize| compacted[representative[index]].expect("representatives are kept");

    let mut triangles = Vec::with_capacity(developed.triangles.len());
    for triangle in &developed.triangles {
        let [a, b, c] = *triangle;
        let mapped = [remap(a), remap(b), remap(c)];
        if mapped[0] == mapped[1] || mapped[1] == mapped[2] || mapped[0] == mapped[2] {
            return Err(AnnulusExit::RegluedDegenerateTriangle);
        }
        triangles.push(mapped);
    }

    // Directed edge incidence, which decides orientation consistency and
    // manifoldness together: in a consistently oriented complex every
    // directed edge occurs exactly once.
    let mut directed: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut undirected: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for triangle in &triangles {
        let [a, b, c] = *triangle;
        for (p, q) in [(a, b), (b, c), (c, a)] {
            if !directed.insert((p, q)) {
                return Err(AnnulusExit::RegluedOrientationInconsistent);
            }
            *undirected.entry((p.min(q), p.max(q))).or_insert(0) += 1;
        }
    }
    if undirected.values().any(|count| *count > 2) {
        return Err(AnnulusExit::RegluedOrientationInconsistent);
    }

    let boundary: Vec<(usize, usize)> = undirected
        .iter()
        .filter(|(_, count)| **count == 1)
        .map(|(edge, _)| *edge)
        .collect();
    let interior_edges = undirected.values().filter(|count| **count == 2).count();

    // The cut must not have survived. Its two sides became one edge under the
    // identification; if that edge is still a boundary edge, the reglue did
    // not happen.
    let cut_edge = {
        let left = patch.identification.pairs[0];
        let right = patch.identification.pairs[1];
        let p = remap(left.0);
        let q = remap(right.0);
        (p.min(q), p.max(q))
    };
    if undirected.get(&cut_edge) != Some(&2) {
        return Err(AnnulusExit::RegluedCutSurvives);
    }

    if !is_connected(vertices.len(), &triangles) {
        return Err(AnnulusExit::RegluedNotConnected);
    }

    let boundary_components = count_boundary_components(&boundary);
    if boundary_components != 2 {
        return Err(AnnulusExit::RegluedBoundaryComponents {
            components: boundary_components,
        });
    }

    let characteristic =
        vertices.len() as i64 - (boundary.len() + interior_edges) as i64 + triangles.len() as i64;
    if characteristic != 0 {
        return Err(AnnulusExit::RegluedEulerCharacteristic { characteristic });
    }

    let developed = TriangulatedRegion {
        vertices,
        triangles,
    };
    let physical_vertices = lift(&developed);
    if physical_vertices
        .iter()
        .any(|p| !(p.x.is_finite() && p.y.is_finite() && p.z.is_finite()))
    {
        return Err(AnnulusExit::LiftNotFinite);
    }

    Ok(RealizedAnnulus {
        cell,
        validity: AnnulusValidityReport {
            triangles: developed.triangles.len(),
            vertices: developed.vertices.len(),
            boundary_edges: boundary.len(),
            interior_edges,
            boundary_components,
            euler_characteristic: characteristic,
        },
        developed,
        patch_validity,
        physical_vertices,
    })
}

/// Whether the complex is connected, walking vertex adjacency across
/// triangles.
fn is_connected(vertices: usize, triangles: &[[usize; 3]]) -> bool {
    if vertices == 0 {
        return false;
    }
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); vertices];
    for [a, b, c] in triangles {
        for (p, q) in [(*a, *b), (*b, *c), (*c, *a)] {
            adjacency[p].push(q);
            adjacency[q].push(p);
        }
    }
    let mut seen = vec![false; vertices];
    let mut stack = vec![0usize];
    seen[0] = true;
    let mut count = 1;
    while let Some(current) = stack.pop() {
        for &next in &adjacency[current] {
            if !seen[next] {
                seen[next] = true;
                count += 1;
                stack.push(next);
            }
        }
    }
    count == vertices
}

/// How many connected components the boundary edge set forms.
fn count_boundary_components(boundary: &[(usize, usize)]) -> usize {
    let mut adjacency: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for &(a, b) in boundary {
        adjacency.entry(a).or_default().push(b);
        adjacency.entry(b).or_default().push(a);
    }
    let mut seen: BTreeSet<usize> = BTreeSet::new();
    let mut components = 0;
    for &vertex in adjacency.keys() {
        if seen.contains(&vertex) {
            continue;
        }
        components += 1;
        let mut stack = vec![vertex];
        seen.insert(vertex);
        while let Some(current) = stack.pop() {
            for &next in adjacency.get(&current).into_iter().flatten() {
                if seen.insert(next) {
                    stack.push(next);
                }
            }
        }
    }
    components
}

/// The whole shared realization, composed: verify the contract, plan the cut,
/// cut open, triangulate, reglue and lift.
///
/// This is the single entry point an ambient cell should use. Its stages are
/// public individually so a focused test can drive one, but a production
/// caller that assembles them by hand would be reimplementing the order the
/// safety argument depends on.
pub fn realize(
    annulus: &RankOnePeriodicAnnulus<'_>,
    tolerance: f64,
    lift: &impl Fn(&TriangulatedRegion) -> Vec<Point3>,
) -> Result<RealizedAnnulus, AnnulusExit> {
    verify_contract(annulus)?;
    let plan = plan_cut_open(
        annulus.first.parallel,
        annulus.second.parallel,
        annulus.period,
    )?;
    let patch = cut_open(annulus, &plan, tolerance)?;
    let developed = triangulate_annulus_patch(&patch)?;
    reglue(&patch, developed, annulus.cell, lift)
}

// ---------------------------------------------------------------------------
// Deck-lift arithmetic
// ---------------------------------------------------------------------------

/// Why one join of the deck walk did not resolve to a unique integer.
///
/// Chart-neutral: the walk sees two developed points and a generator, and
/// nothing about what surface they came from. [`super::cylinder_lift::CylinderLiftExit`]
/// forwards these unchanged, so the cylinder's own vocabulary is unaffected.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeckJoinFailure {
    /// The join's developed displacement is not compatible with any deck
    /// integer: the shared source vertex is proved inconsistent with the
    /// certified period.
    NoCompatibleInteger {
        /// Index of the join, between occurrence `join_index` and the next one
        /// in cyclic order.
        join_index: usize,
    },
    /// The join's evidence admits more than one deck integer.
    MultipleCompatibleIntegers {
        /// See [`Self::NoCompatibleInteger`].
        join_index: usize,
    },
    /// The period is too small, at the displacement's scale, for the
    /// arithmetic enclosure to resolve which deck integer applies.
    Indeterminate {
        /// See [`Self::NoCompatibleInteger`].
        join_index: usize,
    },
    /// The deck solver could not complete the join arithmetically.
    OperationalFailure {
        /// See [`Self::NoCompatibleInteger`].
        join_index: usize,
        /// Why.
        failure: DeckOperationalFailure,
    },
}

/// A certified deck placement for every occurrence, and the certified terminal
/// holonomy itself.
#[derive(Debug, Clone)]
pub struct DeckPlacementWalk {
    /// The certified deck integer for each occurrence, in traversal order,
    /// with `placements[0] == 0` by the walk's fixed starting placement.
    pub placements: Vec<i64>,
    /// The certified terminal holonomy, in units of the deck generator: the
    /// deck integer the closing join implies for occurrence `0`, read against
    /// the fixed `n_0 = 0`.
    pub holonomy: i64,
    /// How many joins were checked, including the final-to-initial wrap.
    pub joins_checked: usize,
}

/// The number of chained elementary floating-point operations a certified
/// join tolerance must cover.
///
/// A developed coordinate at one end of a join is built from a bounded,
/// countable chain: a subtraction (`x - origin`), a dot product (two
/// multiplies and an add) against a recorded basis vector, and — on the
/// periodic axis — an `atan2` evaluation or a `theta_start + declared_sweep`
/// addition. Each elementary op contributes at most one ULP of *relative*
/// error to a result of that op's own magnitude (IEEE 754 correct rounding),
/// and [`DeckGenerator`]'s own shift arithmetic (`k * period`, folded in by the
/// placement) adds one more. Eight covers that chain with headroom to spare
/// without being so wide that a real discrepancy between two *different*
/// physical vertices could hide inside it — the scale it multiplies, in
/// [`certified_join_tolerance`], is the local magnitude of the specific pair
/// of values being compared, per `FORMAL_SYSTEM.md`'s per-primitive
/// discipline, never a global caller-chosen constant.
const JOIN_EVALUATION_ULPS: f64 = 8.0;

/// The certified enclosure radius for one developed coordinate's join
/// discrepancy: [`JOIN_EVALUATION_ULPS`] machine epsilons of the larger
/// operand magnitude, floored at `scale_floor` so a join whose true value is
/// near zero still gets a nonzero, scale-appropriate enclosure (the floor for
/// the periodic axis is the deck period itself, since the shift arithmetic's
/// error scales with the period regardless of how small the residual angle
/// is).
fn certified_join_tolerance(b: f64, a: f64, scale_floor: f64) -> f64 {
    let scale = b.abs().max(a.abs()).max(scale_floor);
    JOIN_EVALUATION_ULPS * f64::EPSILON * scale
}

/// Solve one join: the deck integer `k` with `A + k g` compatible with `B`,
/// where `B` is the unplaced developed end of one occurrence and `A` the
/// unplaced developed start of the next.
///
/// Each component is widened by [`certified_join_tolerance`] into `[value -
/// tolerance, value + tolerance]` rather than passed to the deck solver as a
/// bit-exact point. Both sides of a join are evaluated through genuinely
/// different code paths for the same physical vertex — one occurrence's
/// `atan2`-based periodic coordinate, the other's `theta_start +
/// declared_sweep` arithmetic — so their difference at a true zero-holonomy
/// join is not exactly `0.0`, only within a certified number of ULPs of it.
/// Without this enclosure the deck solver would correctly, but uselessly,
/// refuse every join as `NoCompatibleInteger`: a zero-width interval demands
/// exact equality to an integer multiple of the period, which floating
/// evaluation of two different expressions never gives even when the
/// underlying claim is true.
fn solve_join(
    generator: &DeckGenerator,
    b: Point2,
    a: Point2,
) -> Result<DeckSolveResult, DeckOperationalFailure> {
    // The developed convention is (aperiodic = First, periodic = Second); see
    // `super::cylinder`'s and `super::cone`'s module docs.
    let aperiodic_tolerance = certified_join_tolerance(b.x, a.x, 1.0);
    let periodic_tolerance = certified_join_tolerance(b.y, a.y, generator.period_magnitude().get());
    let aperiodic = DeckInterval::from_f64(
        b.x - a.x - aperiodic_tolerance,
        b.x - a.x + aperiodic_tolerance,
    )
    .map_err(|_| DeckOperationalFailure::ArithmeticOverflow)?;
    let periodic = DeckInterval::from_f64(
        b.y - a.y - periodic_tolerance,
        b.y - a.y + periodic_tolerance,
    )
    .map_err(|_| DeckOperationalFailure::ArithmeticOverflow)?;
    let displacement = DevelopedBox {
        first: aperiodic,
        second: periodic,
    };
    solve_axis_aligned(generator, &displacement)
}

/// Propagate deck placements around a developed boundary from a fixed initial
/// placement and report the terminal holonomy, taking no verdict on its value.
///
/// `chain` is one `(start, end)` pair per occurrence, in traversal order, in
/// the *unplaced* developed frame. Every join is classified before any
/// placement past it is used, so a join that does not resolve uniquely stops
/// the walk with the specific [`DeckJoinFailure`] naming it — this function
/// never guesses a placement to keep going. Summing the individual developed
/// displacements around the loop instead would silently discard exactly the
/// information the holonomy *is*.
pub fn propagate_deck_placements(
    chain: &[(Point2, Point2)],
    generator: DeckGenerator,
) -> Result<DeckPlacementWalk, DeckJoinFailure> {
    let count = chain.len();
    // A degenerate (empty) boundary has no closed walk to classify; the
    // caller's traversal-continuity check already refuses this shape before
    // reaching here in every real path.
    debug_assert!(count > 0, "traverse_bound refuses an empty traversal");

    let mut placements = vec![0i64; count];

    let classify = |join_index: usize,
                    result: Result<DeckSolveResult, DeckOperationalFailure>|
     -> Result<i64, DeckJoinFailure> {
        match result {
            Ok(DeckSolveResult::Unique(k)) => Ok(k),
            Ok(DeckSolveResult::NoCompatibleInteger) => {
                Err(DeckJoinFailure::NoCompatibleInteger { join_index })
            }
            Ok(DeckSolveResult::MultipleCompatibleIntegers) => {
                Err(DeckJoinFailure::MultipleCompatibleIntegers { join_index })
            }
            Ok(DeckSolveResult::Indeterminate) => {
                Err(DeckJoinFailure::Indeterminate { join_index })
            }
            Err(failure) => Err(DeckJoinFailure::OperationalFailure {
                join_index,
                failure,
            }),
        }
    };

    // Propagate n_1 .. n_{count-1} from the fixed n_0 = 0. Every join here
    // uses the *raw* (unplaced) developed end and start; the running
    // placement is folded in only through the accumulated `k`s, never by
    // re-deriving a coordinate.
    for i in 0..count.saturating_sub(1) {
        let b = chain[i].1;
        let a = chain[i + 1].0;
        let k = classify(i, solve_join(&generator, b, a))?;
        placements[i + 1] = placements[i] + k;
    }

    // The final join closes the cycle back to occurrence 0 without
    // overwriting its fixed placement: the candidate it implies for n_0 is
    // read off directly as the terminal holonomy, and the walk stays "cut
    // open".
    let last = count - 1;
    let k_last = classify(last, solve_join(&generator, chain[last].1, chain[0].0))?;
    let holonomy = placements[last] + k_last;

    Ok(DeckPlacementWalk {
        placements,
        holonomy,
        joins_checked: count,
    })
}
