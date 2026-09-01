//! The conical essential band: an annulus bounded by two complete, essential,
//! oppositely oriented cone parallels on **one nappe**, with the apex proved
//! outside.
//!
//! # The population, and why it is not the cylinder band again
//!
//! `docs/ABC_REMAINDER_DIAG.md` names 5,228 faces across 15 of the 20 ABC
//! models with one signature: a conical support, exactly two bounds, each bound
//! exactly one complete source `CIRCLE`, and a legacy terminal verdict of
//! `SyntheticSyntheticCrossing` — the two artificial join segments the legacy
//! cut plan invents, crossing each other. That is the same symptom the
//! cylinder band's 15,123 faces presented, and for the same reason: the face is
//! an annulus in a rank-one quotient, no planar polygon represents it, and
//! manufacturing one produces edges that are boundaries of nothing.
//!
//! So the *realization* is the same, and it is shared —
//! [`super::rank1_annulus`]. The **admission** is not, and three obligations
//! here have no counterpart in the cylinder work. Each is discharged below,
//! from the certified cone and the certified carriers, and none of them is
//! decided by numerical closeness.
//!
//! ## 1. Same nappe
//!
//! A STEP `conical_surface` is a mathematical cone, and a mathematical cone is
//! double-napped. Nothing in the surface restricts it to one side; that is the
//! trimmed face's job. Two complete circular parallels on *opposite* nappes
//! match this population's signature exactly — two bounds, two complete
//! circles, opposite induced windings — and they do not bound a regular
//! annular strip. Any path between them passes through the apex.
//!
//! The proof is the sign of the cone's own **generator coordinate**
//! `s = (x - apex) · axis` ([`super::cone::ConeSchema::generator_coordinate`]),
//! whose zero *is* the apex. Two carriers are on one nappe when their certified
//! `s`-enclosures both lie strictly on one side of zero — which is a statement
//! about two intervals and a point, not about how near the two circles look.
//!
//! ## 2. Apex exclusion
//!
//! At the apex the angular orbit collapses, the deck action stops being free,
//! and the rank-one annulus chart degenerates. The closed strip between the
//! carriers must therefore exclude it.
//!
//! Once both `s`-enclosures are strictly on one side of zero this follows
//! immediately — the closed interval between two strictly positive intervals is
//! strictly positive — so the two obligations are discharged by one classifier
//! ([`classify_nappes`]) and the clearance it certifies travels to the realizer
//! as [`super::rank1_annulus::FreeDeckAction::OneSingularOrbitExcluded`], which
//! re-checks it against the chains it is actually handed.
//!
//! A carrier whose enclosure *contains* zero is not rounded to a side. It exits
//! as [`ConicalBandExit::ApexContactUndecided`], which says the evidence does
//! not settle the question rather than picking an answer — the epistemic
//! distinction the packet requires and the one a tolerance comparison would
//! destroy.
//!
//! ## 3. Material authority is intrinsic, always
//!
//! 5,191 of the 5,228 faces declare exactly one `FACE_OUTER_BOUND`, and that
//! declaration is **not** used to select the region. A frustum band has no
//! intrinsically outer loop: both bounds are complete essential circles, and no
//! complete essential circle bounds a disk on a cone away from the apex, so
//! neither can be an outer bound in the sense ISO 10303-42 defines. Reading the
//! region off the declaration would be reading a fact the file cannot state.
//!
//! Authority comes from the completed certificate instead
//! ([`conical_band_material_authority`]): two ordered essential carriers on one
//! regular nappe, with the apex excluded, bound exactly one compact strip.
//! Two disjoint parallels cut the nappe into three pieces — the compact strip
//! between them, the punctured cone-point end below the inner one, and the
//! non-compact end above the outer one — and only the strip is bounded by
//! *both* circles. The declaration is retained as provenance
//! ([`ConicalSourceStanding`]) and nothing else.
//!
//! This is why an *absent* declaration is admitted here while the cylinder band
//! refuses one. On the cylinder the default route takes standing from the
//! source, so a missing standing is a real gap. Here no route ever consults it,
//! so requiring it would be requiring an irrelevant fact. Two or more declared
//! outer bounds still refuse: that is a conformance verdict this packet does
//! not extend a repair to, and the diagnosed population contains none.
//!
//! # The admitted case, in full
//!
//! - the support is a certified embedded cone ([`super::cone`]), so its apex is
//!   located and its half-angle certified;
//! - the source face has exactly two authoritative bounds;
//! - each bound is exactly **one** source edge use, closed on one source vertex
//!   by identity, carrying a complete source `CIRCLE` whose own certified
//!   placement is the cone parallel through that vertex — plane perpendicular
//!   to the axis, centre on the axis at the same generator coordinate, radius
//!   `slope · |s|`;
//! - each bound's terminal holonomy, solved by the shared deck walk and not
//!   assumed, is primitive (`±1`);
//! - the two induced homologies are opposite;
//! - both carriers lie strictly on one nappe, so the apex is excluded;
//! - the two carriers are certified **distinct**, through a strict separation
//!   of their certified generator enclosures;
//! - therefore the carriers are strictly ordered along the nappe and disjoint,
//!   and the compact strip between them is the material region.
//!
//! Everything else is refused by name. A bound carrying a partial arc, a
//! spline, a genuine ellipse or more than one edge use; a pair of carriers on
//! opposite nappes, at the apex, straddling it, or coincident; incompatible
//! windings; a multiply wound boundary — each has its own exit, and none is
//! repaired to make a later conjunct hold.

use super::super::source_evidence::{
    BoundId, EdgeUseId, SourceBoundInput, SourceFaceInput, SourceVertexKey,
};
use super::cone::{CertifiedEmbeddedCone, ConeSchema, Nappe};
use super::curve_witness::{CompleteCirclePlacement, SourceCurveFamily};
use super::planar_slice::{traverse_bound, SliceCategory, SliceExit, TriangulatedRegion};
use super::rank1_annulus::{
    self, AnnulusBoundary, AnnulusCell, AnnulusExit, AnnulusValidityReport, CarrierOrder,
    CompleteParallel, DeckJoinFailure, FreeDeckAction, RankOnePeriodicAnnulus,
};
use super::support::CurveSchema;
use truck_geometry::prelude::{InnerSpace, Point2, Point3};
use truck_topology::compress::OuterBoundStanding;

/// Dimensionless tolerance floor for the cone's own structural checks.
///
/// The same `1e-9` [`super::cone`] certifies the surface at and
/// [`super::curve_witness`] certifies a cylinder witness at, so a witness is
/// refused only when the discrepancy is orders of magnitude past floating
/// point noise. It is never used to decide a *semantic* question — not the
/// nappe, not carrier coincidence, not apex exclusion, not whether a source
/// curve is a circle. Those are settled by source family and by the sign and
/// order of certified enclosures; this bound only validates a construction the
/// source already authorized.
const RELATIVE_TOLERANCE: f64 = 1e-9;

/// The certified evaluation bound for one carrier's generator coordinate, at
/// that carrier's own scale.
///
/// Scaled by the largest of the carrier's distance from the apex, its radius
/// and one — the per-primitive discipline `FORMAL_SYSTEM.md` requires, rather
/// than one global constant that would be far too tight on a metre-scale cone
/// and far too loose on a millimetre-scale one.
fn generator_enclosure_bound(s: f64, radius: f64) -> f64 {
    RELATIVE_TOLERANCE * s.abs().max(radius).max(1.0)
}

/// The tolerance a witness's on-cone and placement checks are held to, at the
/// generator coordinate they are evaluated at.
fn scaled_tolerance(schema: &ConeSchema, s: f64) -> f64 {
    generator_enclosure_bound(s, schema.radius_at(s))
}

// ---------------------------------------------------------------------------
// The complete-circle-on-cone witness
// ---------------------------------------------------------------------------

/// Why a source curve could not be certified as a complete cone parallel.
///
/// Deliberately separate from [`super::curve_witness::WitnessFailure`], whose
/// `CircleNotACylinderParallel` obligation is the *wrong* one here: it requires
/// the circle's radius to be the support's single radius, and a cone has no
/// single radius. Sharing that type would be exactly the "cone obtains
/// admission by calling a cylinder-oriented helper" failure this packet
/// forbids.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConeWitnessFailure {
    /// An input coordinate was `NaN` or infinite.
    NonFiniteInput,
    /// The traversal start point is not on the certified cone.
    StartNotOnCone,
    /// The traversal start point is the apex, where the orbit collapses to a
    /// point and there is no parallel to be on.
    StartAtApex,
    /// The circle's own certified placement is not the cone parallel through
    /// its endpoint: its plane is not perpendicular to the axis, its centre is
    /// not on the axis, its centre is not at the endpoint's own generator
    /// coordinate, or its radius is not the one the certified half-angle
    /// predicts there.
    CircleNotAConeParallel,
    /// A complete-circle candidate was presented on an occurrence whose start
    /// and end are *different* source vertices, so no authoritative fact says
    /// the occurrence covers the circle's whole period.
    OccurrenceNotClosed,
}

impl ConeWitnessFailure {
    /// Which semantic category this failure belongs to.
    ///
    /// The line is authority, as everywhere else in the subtree. A point that
    /// is not on the cone the face is trimmed from contradicts a claim the
    /// source itself makes, as does a circle that is not the parallel it is
    /// presented as. A start at the apex is valid geometry outside the
    /// admitted subset. An occurrence whose source topology does not close
    /// establishes nothing either way.
    pub fn category(self) -> SliceCategory {
        match self {
            Self::NonFiniteInput => SliceCategory::OperationalFailure,
            Self::StartNotOnCone | Self::CircleNotAConeParallel => SliceCategory::Inconsistent,
            Self::StartAtApex => SliceCategory::Unsupported,
            Self::OccurrenceNotClosed => SliceCategory::Unresolved,
        }
    }

    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::NonFiniteInput => "cone_witness_non_finite_input",
            Self::StartNotOnCone => "cone_witness_start_not_on_cone",
            Self::StartAtApex => "cone_witness_start_at_apex",
            Self::CircleNotAConeParallel => "cone_witness_circle_not_a_cone_parallel",
            Self::OccurrenceNotClosed => "cone_witness_occurrence_not_closed",
        }
    }
}

/// One complete cone parallel, developed into the `(generator, angular)`
/// chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConeParallelWitness {
    /// The developed start point: `(generator_coordinate, angular)`.
    pub start: Point2,
    /// The developed end point, continuous with `start` in the universal
    /// cover: one full signed period away.
    pub end: Point2,
}

/// Certify a complete-circle witness on a cone: one occurrence that traverses
/// an entire cone parallel exactly once.
///
/// This is the case a trimmed parameter interval cannot express, and the
/// argument is [`super::curve_witness::complete_circle_witness`]'s, restated
/// against a cone's obligations. A source `edge_curve` on a full circle has one
/// vertex used for both of its ends, so an importer that recovers the trim by
/// solving each end onto the curve gets the *same* parameter twice and hands
/// downstream a degenerate interval `(u, u)` — not because the source declared
/// a zero sweep, but because two coincident endpoints carry no extent. The
/// extent is instead the circle's own complete period, and the direction is the
/// occurrence's own parameter sense.
///
/// The endpoint therefore cannot confirm the sweep, so that obligation is
/// discharged structurally instead: the circle's own certified placement must
/// **be** the cone parallel through that endpoint. On a cylinder that is three
/// facts; here it is four, and the extra one is the whole difference. A cone's
/// radius is a property of the *level*, not of the surface, so "the circle's
/// radius is the support's radius" is not a statement that can be made. What
/// replaces it is: the circle's centre sits at the endpoint's own generator
/// coordinate `s`, and its radius is `slope · |s|` — the radius the certified
/// half-angle predicts *there*. A circle of the right radius at the wrong level
/// is refused, and so is a circle at the right level with the wrong radius.
pub fn complete_cone_circle_witness(
    schema: &ConeSchema,
    placement: CompleteCirclePlacement,
    start: Point3,
    forward: bool,
) -> Result<ConeParallelWitness, ConeWitnessFailure> {
    let finite = |point: Point3| -> Result<Point3, ConeWitnessFailure> {
        match point.x.is_finite() && point.y.is_finite() && point.z.is_finite() {
            true => Ok(point),
            false => Err(ConeWitnessFailure::NonFiniteInput),
        }
    };
    let start = finite(start)?;
    let center = finite(placement.center)?;
    for coordinate in [
        placement.sweep_axis.x,
        placement.sweep_axis.y,
        placement.sweep_axis.z,
        placement.radius,
    ] {
        if !coordinate.is_finite() {
            return Err(ConeWitnessFailure::NonFiniteInput);
        }
    }

    let s = schema.generator_coordinate(start);
    let tolerance = scaled_tolerance(schema, s);

    if schema.radial_gap(start) > tolerance {
        return Err(ConeWitnessFailure::StartNotOnCone);
    }
    // The apex has no parallel through it: the orbit is a point. Refused here
    // on the *radius the level predicts*, which is zero exactly at `s = 0`, so
    // a face whose boundary genuinely runs through the apex is named rather
    // than developed into a chart that has degenerated.
    if !(schema.radius_at(s) > 0.0) {
        return Err(ConeWitnessFailure::StartAtApex);
    }

    // Unit length and parallelism are separate obligations, checked
    // separately, for the reason `complete_circle_witness` gives: a non-unit
    // vector at an oblique angle can land on `|dot| == 1`, and the sign taken
    // from it below would then be read off geometry never certified
    // perpendicular to the axis at all.
    let sweep_axis_length = placement.sweep_axis.magnitude();
    if (sweep_axis_length - 1.0).abs() > RELATIVE_TOLERANCE {
        return Err(ConeWitnessFailure::CircleNotAConeParallel);
    }
    let axis_alignment = placement.sweep_axis.dot(schema.axis());
    if (axis_alignment.abs() - 1.0).abs() > RELATIVE_TOLERANCE {
        return Err(ConeWitnessFailure::CircleNotAConeParallel);
    }
    // The centre is on the axis...
    let center_offset = center - schema.apex();
    let center_generator = center_offset.dot(schema.axis());
    let center_radial = center_offset - center_generator * schema.axis();
    if center_radial.magnitude() > tolerance {
        return Err(ConeWitnessFailure::CircleNotAConeParallel);
    }
    // ...at this endpoint's own level...
    if (center_generator - s).abs() > tolerance {
        return Err(ConeWitnessFailure::CircleNotAConeParallel);
    }
    // ...and the radius is the one the certified half-angle predicts there.
    if (placement.radius - schema.radius_at(s)).abs() > tolerance {
        return Err(ConeWitnessFailure::CircleNotAConeParallel);
    }

    // The sweep: the surface's *own* certified angular period, signed by the
    // occurrence's own parameter sense against the surface's own angular
    // sense, then composed with the selected edge-use direction exactly once.
    // `angular_coordinate` runs right-handedly about `axis` on both nappes —
    // the revolution rotates every point the same way, whichever side of the
    // apex it is on — so this sign rule is the cylinder's unchanged and needs
    // no nappe correction.
    let period = schema.deck_generator().signed_period().get();
    let parameter_sense = if axis_alignment > 0.0 { 1.0 } else { -1.0 };
    let selected_sense = if forward { 1.0 } else { -1.0 };
    let sweep = period * parameter_sense * selected_sense;

    // Both developed ends report the same generator coordinate value, bit for
    // bit: the occurrence begins and ends at one source vertex, whose single
    // canonical position is the only thing either end is computed from.
    let theta = schema.angular_coordinate(start);
    Ok(ConeParallelWitness {
        start: Point2::new(s, theta),
        end: Point2::new(s, theta + sweep),
    })
}

// ---------------------------------------------------------------------------
// Exits
// ---------------------------------------------------------------------------

/// Every way a face can fail to become a certified conical essential band, or
/// fail to be realized as one.
///
/// Each obligation the packet names has its own variant. Nothing collapses
/// into a generic "unsupported cone": a reader of the corpus reconciliation
/// must be able to tell an opposite-nappe pair from an apex-straddling one from
/// a spline boundary, because those three call for three different pieces of
/// work.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConicalBandExit {
    /// The face does not have exactly two authoritative bounds.
    NotTwoBounds {
        /// How many it declared.
        bounds: usize,
    },
    /// One bound's authoritative traversal did not close.
    BoundTraversal {
        /// Which bound.
        bound: BoundId,
        /// That stage's own exit, unchanged.
        exit: SliceExit,
    },
    /// One bound is not a single edge use, so it is not one complete circle
    /// however its pieces are shaped. The admitted class is exactly the
    /// diagnosed one; a multi-occurrence conical boundary is a different
    /// population and is left for one.
    BoundNotOneOccurrence {
        /// Which bound.
        bound: BoundId,
        /// How many occurrences its traversal produced.
        occurrences: usize,
    },
    /// One bound's single edge use does not carry a complete source circle: it
    /// is a partial arc, a line, a spline, a genuine ellipse, or a
    /// representation no structural reader admits.
    BoundNotACompleteSourceCircle {
        /// Which bound.
        bound: BoundId,
    },
    /// One bound's edge use has no certified vertex position.
    BoundVertexPositionMissing {
        /// Which bound.
        bound: BoundId,
    },
    /// One bound's complete circle could not be certified a cone parallel.
    BoundWitness {
        /// Which bound.
        bound: BoundId,
        /// Why.
        cause: ConeWitnessFailure,
    },
    /// One bound's deck walk did not resolve its winding.
    BoundDeckJoin {
        /// Which bound.
        bound: BoundId,
        /// That stage's own failure, unchanged.
        failure: DeckJoinFailure,
    },
    /// One bound's certified homology is not primitive: `0` is contractible
    /// and `|h| > 1` is multiply wound, which this cell does not model.
    BoundNotPrimitive {
        /// Which bound.
        bound: BoundId,
        /// The certified holonomy.
        homology: i64,
    },
    /// The two induced boundary homologies have the same sign. Refused, not
    /// repaired by reversing one of them.
    OrientationIncompatible {
        /// The first bound's homology.
        first: i64,
        /// The second bound's homology.
        second: i64,
    },
    /// The two carriers are certified to be on **opposite** nappes. They do
    /// not bound a regular annular strip: every path between them passes
    /// through the apex. A proved fact about the face.
    OppositeNappes {
        /// The first bound's certified nappe.
        first: Nappe,
        /// The second bound's certified nappe.
        second: Nappe,
    },
    /// At least one carrier's certified generator enclosure contains the apex,
    /// so that carrier's own nappe is not established and neither is apex
    /// exclusion.
    ///
    /// Deliberately *not* resolved to a side. The evidence available does not
    /// settle whether the closed carrier interval includes the apex, and
    /// choosing an answer would manufacture source intent from a tolerance.
    ApexContactUndecided,
    /// The two complete circles are certified to be the same circle: the
    /// generator enclosures overlap and are together no wider than one
    /// enclosure. A degenerate face, never an annulus.
    SameCarrier,
    /// The two generator enclosures overlap and no fact separates or
    /// identifies the two circles, so neither carrier relation is established
    /// and their order is unresolved.
    CarrierOrderUndecided,
    /// The source declared two or more `FACE_OUTER_BOUND` entities.
    ///
    /// A conformance verdict, and one this packet does not extend a repair to:
    /// the region here never came from the declaration in the first place, so
    /// a malformed declaration is reported rather than downgraded.
    MultipleOuterBoundsDeclared {
        /// How many the source declared.
        declared: u32,
    },
    /// The shared rank-one annulus realizer refused. Carries its exit
    /// unchanged.
    Realization(AnnulusExit),
}

impl ConicalBandExit {
    /// Which semantic category this exit belongs to.
    ///
    /// The line is authority. A face that simply is not this cell — wrong
    /// bound count, a boundary that is not a complete circle, a non-primitive
    /// or same-signed homology, opposite nappes, a coincident carrier — is
    /// `Unsupported`: a proved fact about the face, outside the admitted
    /// subset. Missing or insufficient evidence — an undecided apex relation,
    /// an unresolved carrier order, an absent vertex position — is
    /// `Unresolved`, and never rounded into a verdict.
    pub fn category(self) -> SliceCategory {
        match self {
            Self::NotTwoBounds { .. }
            | Self::BoundNotOneOccurrence { .. }
            | Self::BoundNotACompleteSourceCircle { .. }
            | Self::BoundNotPrimitive { .. }
            | Self::OrientationIncompatible { .. }
            | Self::OppositeNappes { .. }
            | Self::SameCarrier => SliceCategory::Unsupported,

            Self::BoundVertexPositionMissing { .. }
            | Self::ApexContactUndecided
            | Self::CarrierOrderUndecided => SliceCategory::Unresolved,

            Self::BoundTraversal { exit, .. } => exit.category(),
            Self::BoundWitness { cause, .. } => cause.category(),
            Self::BoundDeckJoin { failure, .. } => deck_join_category(failure),
            Self::MultipleOuterBoundsDeclared { .. } => {
                SliceExit::MultipleOuterBoundsDeclared.category()
            }
            Self::Realization(exit) => exit.category(),
        }
    }

    /// A short stable tag, for probe and census records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::NotTwoBounds { .. } => "cone_band_not_two_bounds",
            Self::BoundTraversal { exit, .. } => exit.tag(),
            Self::BoundNotOneOccurrence { .. } => "cone_band_bound_not_one_occurrence",
            Self::BoundNotACompleteSourceCircle { .. } => {
                "cone_band_bound_not_a_complete_source_circle"
            }
            Self::BoundVertexPositionMissing { .. } => "cone_band_bound_vertex_position_missing",
            Self::BoundWitness { cause, .. } => cause.tag(),
            Self::BoundDeckJoin { failure, .. } => deck_join_tag(failure),
            Self::BoundNotPrimitive { .. } => "cone_band_bound_not_primitive",
            Self::OrientationIncompatible { .. } => "cone_band_orientation_incompatible",
            Self::OppositeNappes { .. } => "cone_band_opposite_nappes",
            Self::ApexContactUndecided => "cone_band_apex_contact_undecided",
            Self::SameCarrier => "cone_band_same_carrier",
            Self::CarrierOrderUndecided => "cone_band_carrier_order_undecided",
            Self::MultipleOuterBoundsDeclared { .. } => "cone_band_multiple_outer_bounds_declared",
            Self::Realization(exit) => exit.tag(),
        }
    }

    /// The stage this exit left from, for the funnel.
    pub fn stage(self) -> &'static str {
        match self {
            Self::NotTwoBounds { .. } => "bounds",
            Self::BoundTraversal { .. } => "traversal",
            Self::BoundNotOneOccurrence { .. }
            | Self::BoundNotACompleteSourceCircle { .. }
            | Self::BoundVertexPositionMissing { .. }
            | Self::BoundWitness { .. } => "circle",
            Self::BoundDeckJoin { .. } | Self::BoundNotPrimitive { .. } => "winding",
            Self::OrientationIncompatible { .. } => "band",
            Self::OppositeNappes { .. } | Self::ApexContactUndecided => "nappe",
            Self::SameCarrier | Self::CarrierOrderUndecided => "carrier",
            Self::MultipleOuterBoundsDeclared { .. } => "authority",
            Self::Realization(exit) => exit.stage(),
        }
    }
}

impl From<AnnulusExit> for ConicalBandExit {
    fn from(exit: AnnulusExit) -> Self {
        Self::Realization(exit)
    }
}

fn deck_join_category(failure: DeckJoinFailure) -> SliceCategory {
    match failure {
        DeckJoinFailure::NoCompatibleInteger { .. } => SliceCategory::Inconsistent,
        DeckJoinFailure::MultipleCompatibleIntegers { .. }
        | DeckJoinFailure::Indeterminate { .. } => SliceCategory::Unresolved,
        DeckJoinFailure::OperationalFailure { .. } => SliceCategory::OperationalFailure,
    }
}

fn deck_join_tag(failure: DeckJoinFailure) -> &'static str {
    match failure {
        DeckJoinFailure::NoCompatibleInteger { .. } => "cone_band_join_no_compatible_integer",
        DeckJoinFailure::MultipleCompatibleIntegers { .. } => {
            "cone_band_join_multiple_compatible_integers"
        }
        DeckJoinFailure::Indeterminate { .. } => "cone_band_join_indeterminate",
        DeckJoinFailure::OperationalFailure { .. } => "cone_band_join_operational_failure",
    }
}

// ---------------------------------------------------------------------------
// Boundary components
// ---------------------------------------------------------------------------

/// Develop one authoritative bound and certify it a complete cone parallel.
///
/// Reuses [`traverse_bound`] — Step 2's authoritative, source-identity-only
/// traversal — unchanged, and then adds exactly what the admitted class means
/// beyond "a closed traversal": one occurrence, closed on one source vertex by
/// identity, carrying a complete source circle that is this cone's parallel
/// through that vertex, with a primitive winding **solved** by the shared deck
/// walk rather than assumed from the fact that the sweep was built as one
/// period.
///
/// That last point is worth stating plainly, because it looks circular and is
/// not. [`complete_cone_circle_witness`] builds the developed end from the
/// surface's certified period; [`rank1_annulus::propagate_deck_placements`]
/// then asks the certified deck solver, with its own arithmetic enclosure,
/// which integer multiple of the generator that displacement is. A witness
/// whose sweep did not land on a single period — because the period, the
/// generator or the placement disagreed — fails there rather than being taken
/// on trust.
pub fn develop_complete_cone_parallel(
    bound: &SourceBoundInput,
    schema: &ConeSchema,
    curves: &mut impl FnMut(usize) -> CurveSchema,
    vertex_position: &impl Fn(SourceVertexKey) -> Option<Point3>,
    family_of: &impl Fn(EdgeUseId) -> Option<SourceCurveFamily>,
) -> Result<CompleteParallel, ConicalBandExit> {
    let id = bound.id();
    let traversal = traverse_bound(bound, curves)
        .map_err(|exit| ConicalBandExit::BoundTraversal { bound: id, exit })?;
    if traversal.occurrences.len() != 1 {
        return Err(ConicalBandExit::BoundNotOneOccurrence {
            bound: id,
            occurrences: traversal.occurrences.len(),
        });
    }
    let occurrence = &traversal.occurrences[0];

    // The source's own family decides admission, never a coordinate. A partial
    // arc, a spline, a genuine ellipse and an unreadable representation all
    // land here, and all are refused rather than approximated into a circle.
    let Some(SourceCurveFamily::CompleteCircle { placement }) = family_of(occurrence.edge_use)
    else {
        return Err(ConicalBandExit::BoundNotACompleteSourceCircle { bound: id });
    };

    // Closed by *identity*, not by coincidence. Two distinct source vertices
    // at one point are a zero-length edge, not a closed one.
    if occurrence.start_vertex != occurrence.end_vertex {
        return Err(ConicalBandExit::BoundWitness {
            bound: id,
            cause: ConeWitnessFailure::OccurrenceNotClosed,
        });
    }
    let start = vertex_position(occurrence.start_vertex)
        .ok_or(ConicalBandExit::BoundVertexPositionMissing { bound: id })?;

    let witness = complete_cone_circle_witness(schema, placement, start, occurrence.forward)
        .map_err(|cause| ConicalBandExit::BoundWitness { bound: id, cause })?;

    let walk = rank1_annulus::propagate_deck_placements(
        &[(witness.start, witness.end)],
        schema.deck_generator(),
    )
    .map_err(|failure| ConicalBandExit::BoundDeckJoin { bound: id, failure })?;

    if walk.holonomy != 1 && walk.holonomy != -1 {
        return Err(ConicalBandExit::BoundNotPrimitive {
            bound: id,
            homology: walk.holonomy,
        });
    }

    Ok(CompleteParallel {
        bound: id,
        edge_uses: vec![occurrence.edge_use],
        start_vertices: vec![occurrence.start_vertex],
        starts: vec![witness.start],
        terminal: witness.end,
        homology: walk.holonomy,
    })
}

// ---------------------------------------------------------------------------
// Physical carriers
// ---------------------------------------------------------------------------

/// The complete physical circle a boundary component is carried by.
///
/// A circle on a certified cone is fixed by three facts, and this type carries
/// all three: the support cone (shared by both carriers of a band, and
/// recorded by the caller), the **generator coordinate** of the axis-normal
/// plane it lies in — named by the observed extent of every developed endpoint
/// the complete component visits together with the certified bound each was
/// evaluated to, rather than by a single rounded level — and the complete
/// extent, which is `winding = ±1` because the component was certified a
/// complete parallel before a carrier was built for it. The radius is not a
/// fourth fact: on a cone it is `slope · |s|`, a consequence of the first two.
///
/// No mean, no centroid, no single sampled point, no polyline point count and
/// no epsilon chosen here enters any of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConeCircleCarrier {
    /// Which bound this carrier came from.
    pub bound: BoundId,
    /// The lowest generator coordinate observed anywhere on the complete
    /// circle.
    pub observed_low: f64,
    /// The highest generator coordinate observed anywhere on the complete
    /// circle.
    pub observed_high: f64,
    /// The certified per-coordinate evaluation bound.
    pub enclosure: f64,
    /// The circle's radius, at its own certified level.
    pub radius: f64,
    /// The complete extent, in turns: `+1` or `-1`.
    pub winding: i64,
}

impl ConeCircleCarrier {
    /// The low end of the certified enclosure of the circle's generator
    /// coordinate.
    pub fn generator_low(&self) -> f64 {
        self.observed_low - self.enclosure
    }

    /// The high end of the certified enclosure of the circle's generator
    /// coordinate.
    pub fn generator_high(&self) -> f64 {
        self.observed_high + self.enclosure
    }

    /// Which nappe the whole certified enclosure lies on, or `None` when the
    /// enclosure contains the apex and the question is not settled.
    ///
    /// The apex-exclusion and same-nappe obligations are both read off this:
    /// a carrier with a nappe has, by the definition above, a certified
    /// enclosure that does not contain zero.
    pub fn certified_nappe(&self) -> Option<Nappe> {
        if self.generator_low() > 0.0 {
            Some(Nappe::Positive)
        } else if self.generator_high() < 0.0 {
            Some(Nappe::Negative)
        } else {
            None
        }
    }

    /// The certified clearance between this carrier's enclosure and the apex,
    /// or `0.0` when the enclosure reaches or contains it.
    pub fn apex_clearance(&self) -> f64 {
        match self.certified_nappe() {
            Some(Nappe::Positive) => self.generator_low(),
            Some(Nappe::Negative) => -self.generator_high(),
            None => 0.0,
        }
    }
}

/// Build the complete physical circle carrier for a certified parallel.
pub fn carrier_of(parallel: &CompleteParallel, schema: &ConeSchema) -> ConeCircleCarrier {
    let (low, high) = parallel.aperiodic_extent();
    let level = 0.5 * (low + high);
    let radius = schema.radius_at(level);
    ConeCircleCarrier {
        bound: parallel.bound,
        observed_low: low,
        observed_high: high,
        enclosure: generator_enclosure_bound(level, radius),
        radius,
        winding: parallel.homology,
    }
}

/// How two certified carriers sit relative to the apex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NappeRelation {
    /// Both certified enclosures lie strictly on one side of the apex, so the
    /// carriers are on one nappe and the closed interval between them — which
    /// lies between two intervals of one sign — excludes the apex.
    SameNappe {
        /// Which nappe.
        nappe: Nappe,
        /// The certified clearance between the apex and the nearer of the two
        /// enclosures. Strictly positive.
        apex_clearance: f64,
    },
    /// The two certified enclosures lie strictly on opposite sides. A proved
    /// fact about the face, and the reason it is not an annulus.
    OppositeNappes {
        /// The first carrier's nappe.
        first: Nappe,
        /// The second carrier's nappe.
        second: Nappe,
    },
    /// At least one enclosure contains the apex, so its nappe is not
    /// established. Neither of the other two verdicts may be taken.
    Undecided,
}

/// Classify two certified carriers against the apex.
///
/// The whole same-nappe and apex-exclusion argument, in one place and in one
/// arithmetic step: a carrier's nappe is the side its *certified enclosure*
/// lies on, and an enclosure that straddles zero has no side. Nothing here
/// compares the two circles to each other, measures a distance between them,
/// or consults a radius; two circles a micron apart on one nappe and two
/// circles a kilometre apart on one nappe are the same verdict, because the
/// question is which side of the apex each is on and not how far apart they
/// are.
///
/// # Why the enclosure width cannot manufacture a verdict
///
/// The enclosure carries a relative bound, and it is worth being precise about
/// what that bound can and cannot do, because "a tolerance decides the nappe"
/// would be exactly the unsound reading this cell must not have.
///
/// Widening an enclosure is **monotone toward refusal**. A carrier has a
/// certified nappe only when its whole enclosure clears zero, so widening can
/// move `Some(nappe)` to `None` and can never move `None` to `Some`, and it can
/// never move `Some(Positive)` to `Some(Negative)` or back — the two conditions
/// are `low > 0` and `high < 0`, and no width satisfies both. So the bound's
/// only power is to *withhold* a verdict, which turns into
/// [`ConicalBandExit::ApexContactUndecided`] and refuses the face.
///
/// A false verdict would therefore need the enclosure to be too **narrow**: the
/// evaluated generator coordinate would have to clear zero by more than the
/// bound while the true coordinate did not, which means an evaluation error of
/// more than `1e-9` relative on a dot product — some seven orders of magnitude
/// past what one subtraction and one dot product can produce. The direction
/// that fails safe is the direction a tolerance error actually takes here.
pub fn classify_nappes(first: &ConeCircleCarrier, second: &ConeCircleCarrier) -> NappeRelation {
    match (first.certified_nappe(), second.certified_nappe()) {
        (Some(a), Some(b)) if a == b => NappeRelation::SameNappe {
            nappe: a,
            apex_clearance: first.apex_clearance().min(second.apex_clearance()),
        },
        (Some(first), Some(second)) => NappeRelation::OppositeNappes { first, second },
        _ => NappeRelation::Undecided,
    }
}

/// How two complete physical circles on one cone nappe relate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConeCarrierRelation {
    /// Certified distinct: the two generator enclosures are strictly disjoint,
    /// so the circles cannot be the same circle and are strictly ordered along
    /// the nappe. Names which carrier is lower in the generator coordinate.
    DistinctCarrier {
        /// `true` when the first carrier is the lower one.
        first_is_lower: bool,
        /// The certified gap between the two enclosures.
        separation: f64,
    },
    /// Certified coincident: one certified enclosure covers every generator
    /// coordinate observed on *either* circle, and both have the same complete
    /// extent — so they are the same circle. On a cone equal levels force
    /// equal radii, so no separate radius comparison is needed or made.
    SameCarrier,
    /// Neither relation is established: the enclosures meet, so the circles
    /// are not separated, but the observed extents are together too wide for
    /// one enclosure to cover, so they are not certified equal either.
    Undecided,
}

/// Classify two complete circles on one cone, from their certified generator
/// enclosures alone.
///
/// Deliberately three-way, for the reason [`super::cylinder_band::classify_carriers`]
/// gives: floating evaluation of a level admits three states and not two, and
/// only the first admits a band, so all three outcomes are safe.
///
/// Note what "lower" means here and what it does not. It is the order in the
/// signed generator coordinate, which on the positive nappe is also the order
/// in radius and on the negative nappe is the *reverse* of it. The realizer
/// needs only that the two chains are separated and ordered in the chart's
/// aperiodic coordinate, which this is; nothing downstream reads "lower" as
/// "smaller circle".
pub fn classify_cone_carriers(
    first: &ConeCircleCarrier,
    second: &ConeCircleCarrier,
) -> ConeCarrierRelation {
    if first.generator_high() < second.generator_low() {
        return ConeCarrierRelation::DistinctCarrier {
            first_is_lower: true,
            separation: second.generator_low() - first.generator_high(),
        };
    }
    if second.generator_high() < first.generator_low() {
        return ConeCarrierRelation::DistinctCarrier {
            first_is_lower: false,
            separation: first.generator_low() - second.generator_high(),
        };
    }
    let union_low = first.observed_low.min(second.observed_low);
    let union_high = first.observed_high.max(second.observed_high);
    let covered = union_high - union_low <= first.enclosure.max(second.enclosure);
    match covered && first.winding.abs() == second.winding.abs() {
        true => ConeCarrierRelation::SameCarrier,
        false => ConeCarrierRelation::Undecided,
    }
}

// ---------------------------------------------------------------------------
// Material authority
// ---------------------------------------------------------------------------

/// What the source said about outer-bound standing on an accepted conical
/// band.
///
/// Provenance, and only provenance. See the module docs: the material region
/// on a frustum band cannot come from this, so it is recorded rather than
/// consulted, and a census can still tell a conformant file from one that said
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConicalSourceStanding {
    /// The source declared exactly one outer bound.
    SingleOuterBoundDeclared {
        /// Which bound, as the source indexed it.
        bound_index: u32,
    },
    /// The source declared no outer bound, or the importer did not retain one.
    ///
    /// Admitted, because no route here ever reads it. Recorded, because
    /// "the file was silent" and "the file said one" are different facts and
    /// collapsing them would let a silent file read as a conformant one.
    NoOuterBoundRetained,
}

impl ConicalSourceStanding {
    /// A short stable tag, for probe and census records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::SingleOuterBoundDeclared { .. } => "single_outer_bound_declared",
            Self::NoOuterBoundRetained => "no_outer_bound_retained",
        }
    }
}

/// Where an accepted conical band's material-region standing comes from.
///
/// One variant, and that is the statement: on this cell there is exactly one
/// source of material authority and it is never the file. Adding a second
/// should be a visible decision at this type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConicalBandMaterialAuthority {
    /// The completed conical-band certificate itself fixes the material
    /// region: the unique compact strip between two ordered essential carriers
    /// on one regular nappe, with the apex excluded, crossed with one angular
    /// period modulo the certified deck identification.
    ///
    /// A positive statement proved by [`certify_conical_essential_band`] — not
    /// a guess that one loop is outer, and not an inference from area, order,
    /// radius or orientation.
    IntrinsicConicalBandCertificate {
        /// What the source declared, retained as provenance.
        standing: ConicalSourceStanding,
    },
}

impl ConicalBandMaterialAuthority {
    /// The retained source standing.
    pub fn standing(self) -> ConicalSourceStanding {
        match self {
            Self::IntrinsicConicalBandCertificate { standing } => standing,
        }
    }
}

/// Decide a certified conical band's material-region standing.
///
/// The admission rule, in one place. Holding a `&CertifiedConicalEssentialBand`
/// is the precondition, and it is itself the proof of every conjunct the rule
/// requires: exactly two bounds, both complete essential circles on this cone,
/// both on one nappe with the apex excluded, physically distinct carriers in
/// certified order, compatible induced orientations, and hence a unique compact
/// strip between them. There is no way to ask for this standing without one.
///
/// The source declaration is read only to be *recorded*, with one exception
/// that is a refusal rather than a use: two or more declared outer bounds is a
/// malformed file, and this packet reports that rather than repairing it.
pub fn conical_band_material_authority(
    outer_bound: OuterBoundStanding,
    band: &CertifiedConicalEssentialBand,
) -> Result<ConicalBandMaterialAuthority, ConicalBandExit> {
    let _ = band;
    let standing = match outer_bound {
        OuterBoundStanding::Declared {
            declared_count: 1,
            bound_index,
        } => ConicalSourceStanding::SingleOuterBoundDeclared { bound_index },
        OuterBoundStanding::Declared { declared_count, .. } => {
            return Err(ConicalBandExit::MultipleOuterBoundsDeclared {
                declared: declared_count,
            });
        }
        OuterBoundStanding::NotRetained | OuterBoundStanding::NoneDeclared => {
            ConicalSourceStanding::NoOuterBoundRetained
        }
    };
    Ok(ConicalBandMaterialAuthority::IntrinsicConicalBandCertificate { standing })
}

// ---------------------------------------------------------------------------
// Band certification
// ---------------------------------------------------------------------------

/// A certified conical essential band: the compact strip between two distinct,
/// essential, oppositely oriented complete parallels on one nappe of one cone,
/// with the apex proved outside.
#[derive(Debug, Clone)]
pub struct CertifiedConicalEssentialBand {
    /// The document entity this face came from, when the importer retained it.
    pub source_face_id: Option<u64>,
    /// The certified embedded cone the band is trimmed from.
    pub cone: CertifiedEmbeddedCone,
    /// The nappe both carriers were certified to lie on.
    pub nappe: Nappe,
    /// The certified clearance between the apex and the nearer carrier
    /// enclosure. Strictly positive.
    pub apex_clearance: f64,
    /// The boundary component whose carrier is lower in the **generator
    /// coordinate**. On the positive nappe that is also the smaller circle; on
    /// the negative nappe it is the larger one. Nothing downstream reads it as
    /// a radius order.
    pub lower_boundary: CompleteParallel,
    /// The boundary component whose carrier is upper in the generator
    /// coordinate.
    pub upper_boundary: CompleteParallel,
    /// The lower carrier.
    pub lower_carrier: ConeCircleCarrier,
    /// The upper carrier.
    pub upper_carrier: ConeCircleCarrier,
    /// The certified gap between the two carriers' enclosures.
    pub separation: f64,
    /// The cone's signed deck period.
    pub period: f64,
}

impl CertifiedConicalEssentialBand {
    /// The two boundary components in the source's own bound order, which is
    /// the order the cut-open patch traverses them in.
    fn in_source_order(&self) -> (&CompleteParallel, &CompleteParallel) {
        match self.lower_boundary.bound.0 <= self.upper_boundary.bound.0 {
            true => (&self.lower_boundary, &self.upper_boundary),
            false => (&self.upper_boundary, &self.lower_boundary),
        }
    }

    /// The carrier belonging to a component, by bound identity.
    fn carrier_for(&self, parallel: &CompleteParallel) -> &ConeCircleCarrier {
        match parallel.bound == self.lower_boundary.bound {
            true => &self.lower_carrier,
            false => &self.upper_carrier,
        }
    }
}

/// Certify a two-bound conical face as an essential band.
///
/// Every conjunct of the admitted case is checked here, in an order chosen so
/// the *specific* obstruction is named: bound count, then each component's own
/// completeness, then the orientation, then the nappe and the apex, then the
/// carrier order. Nothing is repaired to make a later conjunct hold.
///
/// The nappe question is asked **before** the carrier order, deliberately. Two
/// circles on opposite nappes can have perfectly well separated, strictly
/// ordered generator enclosures — a frustum-shaped pair of numbers — so a
/// carrier-order check run first would report a clean order on a face that is
/// not an annulus at all. Ordering only means something once both carriers are
/// known to be on one regular nappe.
pub fn certify_conical_essential_band(
    source_face_id: Option<u64>,
    cone: CertifiedEmbeddedCone,
    input: &SourceFaceInput,
    curves: &mut impl FnMut(usize) -> CurveSchema,
    vertex_position: &impl Fn(SourceVertexKey) -> Option<Point3>,
    family_of: &impl Fn(EdgeUseId) -> Option<SourceCurveFamily>,
) -> Result<CertifiedConicalEssentialBand, ConicalBandExit> {
    if input.bounds.len() != 2 {
        return Err(ConicalBandExit::NotTwoBounds {
            bounds: input.bounds.len(),
        });
    }
    let schema = cone.schema().clone();

    let first = develop_complete_cone_parallel(
        &input.bounds[0],
        &schema,
        curves,
        vertex_position,
        family_of,
    )?;
    let second = develop_complete_cone_parallel(
        &input.bounds[1],
        &schema,
        curves,
        vertex_position,
        family_of,
    )?;

    // Opposite induced orientations. Checked before anything geometric so that
    // a same-signed pair is reported as the orientation fact it is, and never
    // repaired by reversing one component.
    if first.homology + second.homology != 0 {
        return Err(ConicalBandExit::OrientationIncompatible {
            first: first.homology,
            second: second.homology,
        });
    }

    let first_carrier = carrier_of(&first, &schema);
    let second_carrier = carrier_of(&second, &schema);

    // Same nappe, and therefore apex exclusion. See the module docs.
    let (nappe, apex_clearance) = match classify_nappes(&first_carrier, &second_carrier) {
        NappeRelation::SameNappe {
            nappe,
            apex_clearance,
        } => (nappe, apex_clearance),
        NappeRelation::OppositeNappes { first, second } => {
            return Err(ConicalBandExit::OppositeNappes { first, second });
        }
        NappeRelation::Undecided => return Err(ConicalBandExit::ApexContactUndecided),
    };

    let (first_is_lower, separation) = match classify_cone_carriers(&first_carrier, &second_carrier)
    {
        ConeCarrierRelation::DistinctCarrier {
            first_is_lower,
            separation,
        } => (first_is_lower, separation),
        ConeCarrierRelation::SameCarrier => return Err(ConicalBandExit::SameCarrier),
        ConeCarrierRelation::Undecided => return Err(ConicalBandExit::CarrierOrderUndecided),
    };

    let (lower_boundary, upper_boundary, lower_carrier, upper_carrier) = match first_is_lower {
        true => (first, second, first_carrier, second_carrier),
        false => (second, first, second_carrier, first_carrier),
    };

    Ok(CertifiedConicalEssentialBand {
        source_face_id,
        period: schema.deck_generator().signed_period().get(),
        cone,
        nappe,
        apex_clearance,
        lower_boundary,
        upper_boundary,
        lower_carrier,
        upper_carrier,
        separation,
    })
}

// ---------------------------------------------------------------------------
// Realization, through the shared rank-one annulus realizer
// ---------------------------------------------------------------------------

/// Present a certified conical band to the shared realizer.
///
/// The handover, and the one place to read what the cone claims against what it
/// proved:
///
/// - the two components are in the source's own bound order;
/// - each carries **its own** radius, because a cone's two carriers do not
///   share one — this is what makes each chain's chord subdivision a bound on
///   the circle it actually lies on rather than on some other circle of the
///   same surface;
/// - [`CarrierOrder::DisjointEnclosures`] is [`ConeCarrierRelation::DistinctCarrier`]
///   in the chart's own terms;
/// - [`FreeDeckAction::OneSingularOrbitExcluded`] names the apex at generator
///   coordinate `0.0` — which it is, by the definition of the coordinate — and
///   carries the clearance [`classify_nappes`] certified. The realizer
///   re-checks that zero lies strictly outside the closed span of both chains,
///   so this claim buys nothing on its own.
fn realization_contract(
    band: &CertifiedConicalEssentialBand,
    authority: ConicalBandMaterialAuthority,
) -> RankOnePeriodicAnnulus<'_> {
    // `authority` is a proof token, not data — the same discipline the
    // cylinder band applies. Requiring it by value keeps the standing step
    // impossible to skip.
    let _: ConicalBandMaterialAuthority = authority;
    let (first, second) = band.in_source_order();
    let first_is_lower = first.bound == band.lower_boundary.bound;
    RankOnePeriodicAnnulus {
        first: AnnulusBoundary {
            parallel: first,
            carrier_radius: band.carrier_for(first).radius,
        },
        second: AnnulusBoundary {
            parallel: second,
            carrier_radius: band.carrier_for(second).radius,
        },
        period: band.period,
        carrier_order: CarrierOrder::DisjointEnclosures {
            first_is_lower,
            separation: band.separation,
        },
        free_deck_action: FreeDeckAction::OneSingularOrbitExcluded {
            // The apex is at generator coordinate zero by construction: the
            // coordinate is `(x - apex) · axis`.
            singular_coordinate: 0.0,
            clearance: band.apex_clearance,
        },
        cell: AnnulusCell::ConicalEssentialBand,
    }
}

/// The conical band's complete product: a validated annular mesh on the cone.
#[derive(Debug, Clone)]
pub struct CertifiedConicalBandMesh {
    /// The developed complex, after the identification was discharged.
    pub developed: TriangulatedRegion,
    /// The annular complex's validity report.
    pub validity: AnnulusValidityReport,
    /// The developed vertices lifted onto the cone, in the same order as
    /// `developed.vertices`.
    pub physical_vertices: Vec<Point3>,
    /// The nappe the band was certified on.
    pub nappe: Nappe,
    /// What the source declared about outer-bound standing, retained as
    /// provenance. The material region did not come from it.
    pub standing: ConicalSourceStanding,
}

/// The whole conical band path, composed: certify, take standing, realize.
///
/// The order is the safety argument, as it is on the cylinder: the standing
/// question cannot be asked until there is a `CertifiedConicalEssentialBand` to
/// ask it against, and the realizer cannot be reached until the standing
/// question has been answered.
pub fn run_conical_essential_band(
    source_face_id: Option<u64>,
    cone: CertifiedEmbeddedCone,
    input: &SourceFaceInput,
    outer_bound: OuterBoundStanding,
    curves: &mut impl FnMut(usize) -> CurveSchema,
    vertex_position: &impl Fn(SourceVertexKey) -> Option<Point3>,
    family_of: &impl Fn(EdgeUseId) -> Option<SourceCurveFamily>,
    tolerance: f64,
) -> Result<(CertifiedConicalEssentialBand, CertifiedConicalBandMesh), ConicalBandExit> {
    let band = certify_conical_essential_band(
        source_face_id,
        cone,
        input,
        curves,
        vertex_position,
        family_of,
    )?;
    let authority = conical_band_material_authority(outer_bound, &band)?;
    let annulus = realization_contract(&band, authority);
    let schema = band.cone.schema();
    let realized = rank1_annulus::realize(&annulus, tolerance, &|region| {
        region
            .vertices
            .iter()
            .map(|point| schema.point_at(point.x, point.y))
            .collect()
    })
    .map_err(ConicalBandExit::from)?;
    let mesh = CertifiedConicalBandMesh {
        developed: realized.developed,
        validity: realized.validity,
        physical_vertices: realized.physical_vertices,
        nappe: band.nappe,
        standing: authority.standing(),
    };
    Ok((band, mesh))
}

#[cfg(test)]
mod tests;
