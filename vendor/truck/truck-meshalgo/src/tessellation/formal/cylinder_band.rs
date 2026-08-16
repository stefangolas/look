//! The two-bound cylindrical band: an annulus bounded by two complete,
//! essential, oppositely oriented cylinder parallels.
//!
//! # What the legacy path does, and why this is not that
//!
//! A face trimmed by two independently closed periodic loops has no planar
//! boundary. The legacy tessellator manufactures one anyway, by joining the
//! two loops with artificial segments so that a single planar polygon exists
//! to hand to a triangulator. Those segments are not boundaries of anything,
//! they are placed by proximity, and on `00009190` they cross each other —
//! which is exactly the `SyntheticSyntheticCrossing` population this module
//! targets.
//!
//! The band is not a planar region and no planar polygon represents it. What
//! *does* represent it is a cut-open fundamental domain: cut the annulus once
//! transversally, develop the result into the cylinder's universal cover as a
//! single rectangle-like patch, and carry an explicit **edge identification**
//! saying that the patch's two artificial sides are one and the same cut. The
//! identification is discharged after triangulation by merging vertex
//! *identities* — never by welding coordinates — so no artificial edge
//! survives into the mesh.
//!
//! # The admitted case, in full
//!
//! - the support is a certified embedded cylinder ([`super::cylinder`]);
//! - the source face has exactly two authoritative bounds;
//! - each bound develops into one complete **simple** cylinder parallel:
//!   every occurrence is a circumferential arc, the developed chain is
//!   strictly monotone in the angular coordinate, and the terminal holonomy
//!   is primitive (`±1`);
//! - the two induced homologies are opposite;
//! - the two complete physical circle carriers are certified **distinct**,
//!   through a strict separation of their certified axial enclosures;
//! - therefore the carriers are strictly ordered along the axis and disjoint,
//!   and the compact strip between them is the material region.
//!
//! Two complete parallels on one cylinder are either the same circle or
//! separated along the axis; there is no third possibility. So a pair whose
//! enclosures overlap is *not* an annulus, and is reported as
//! [`BandExit::SameCarrier`] (when source vertex identity certifies the two
//! bounds run over the same vertices) or [`BandExit::CarrierIdentityUndecided`]
//! (when nothing in the source separates them). Neither reaches the realizer.
//!
//! Cones, tori, holes, islands, multiply wound boundaries, intersecting or
//! tangent boundaries and singular supports are refused by name, not
//! attempted.
//!
//! # The one repaired source defect
//!
//! ABC `00009190` declares **two** `FACE_OUTER_BOUND` entities on every one
//! of its 1,968 band faces. ISO 10303-42 permits at most one, so the file is
//! malformed and [`SliceExit::MultipleOuterBoundsDeclared`] stays the correct
//! conformance verdict. The annotation is also unsatisfiable on the geometry
//! it annotates: no complete essential parallel bounds a disk on a cylinder,
//! so neither loop can be an outer bound in the sense 10303-42 defines. The
//! exporter applied a simply-connected notion to an annulus.
//!
//! [`band_material_authority`] therefore downgrades *only* those two
//! qualifiers to ordinary source bounds and takes material standing from the
//! completed band certificate instead — see
//! [`BandMaterialAuthority::IntrinsicBandCertificate`]. Loop identity,
//! traversal sense, induced homology and carrier order are all retained and
//! all still had to be certified first. The resulting mesh is marked
//! [`SourceConformance::RecoveredFromMalformedSource`], so a recovery from a
//! broken file is never reported as a clean read.
//!
//! The repair is scoped to exactly that pattern on exactly a certified band.
//! Missing standing is not repaired, three declared outer bounds are not
//! repaired, and the generic planar entry
//! [`super::planar_slice::bounded_material_region`] is unchanged.
//!
//! # Why the material region is forced, not chosen
//!
//! Two disjoint parallels cut the cylinder into three pieces: the compact
//! strip between them, and two non-compact ends. Only the strip is bounded by
//! *both* circles, so a face whose entire boundary is those two circles is
//! the strip. Nothing here reverses a boundary to make an orientation work:
//! the opposite-homology requirement is checked and refused, never repaired.
//!
//! # Where the realization lives
//!
//! Everything from "cut the strip open" onwards is chart arithmetic that knows
//! nothing about cylinders, so it lives in [`super::rank1_annulus`] and the
//! cone's essential band ([`super::cone_band`]) reaches the same code. What
//! stays here is the whole of the paragraph above: the cylinder-specific
//! certification, which is the only thing that decides whether a face is a
//! band at all.
//!
//! The split changed no arithmetic and no operation order, so a face's mesh is
//! what it was. The realizer additionally re-checks the obligations this
//! module names when it hands them over — see
//! [`super::rank1_annulus::RankOnePeriodicAnnulus`] — which for a certified
//! cylinder band always hold by construction.

use super::super::source_evidence::{
    BoundId, EdgeUseId, SourceBoundInput, SourceFaceInput, SourceVertexKey,
};
use super::curve_witness::{SourceCurveFamily, WitnessClass};
use super::cylinder::{CertifiedEmbeddedCylinder, CylinderSchema};
use super::cylinder_lift::{develop_traversal_from_source, propagate_placements, CylinderLiftExit};
use super::cylinder_mesh::lift_to_cylinder;
use super::planar_slice::{
    traverse_bound, FinalValidityReport, SliceCategory, SliceExit, TriangulatedRegion,
};
use super::rank1_annulus::{
    AnnulusBoundary, AnnulusCell, AnnulusExit, AnnulusValidityReport, CarrierOrder, FreeDeckAction,
    RankOnePeriodicAnnulus,
};
use super::support::CurveSchema;
use truck_geometry::prelude::{Point2, Point3};
use truck_topology::compress::OuterBoundStanding;

pub use super::rank1_annulus::{
    CompleteParallel, CutOpenDomainPlan, EdgeIdentification, PlanarPatch,
};

/// What the regluded annular complex was proved to be.
///
/// The shared realizer's report, under this module's historical name.
pub type BandValidityReport = AnnulusValidityReport;

// ---------------------------------------------------------------------------
// Material authority
// ---------------------------------------------------------------------------

/// Whether an accepted band's source annotations conformed to ISO 10303-42.
///
/// Carried on the realized mesh so a consumer can tell a mesh derived from a
/// conforming file from one that required the repair below. A recovery is not
/// silently promoted to a clean read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceConformance {
    /// The source's bound annotations were conformant and were used as given.
    Conforming,
    /// The source was malformed and a named, bounded repair was applied. The
    /// mesh is sound; the *file* is not.
    RecoveredFromMalformedSource(NonconformantRepair),
}

/// The one nonconformant source pattern this module repairs.
///
/// Deliberately an enum with a single variant rather than a bare flag: adding
/// a second repair should be a visible decision at this type, not a quiet
/// widening of an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NonconformantRepair {
    /// The face declared two `FACE_OUTER_BOUND` entities.
    ///
    /// ISO 10303-42 permits at most one, so the file is malformed and
    /// [`SliceExit::MultipleOuterBoundsDeclared`] remains the correct
    /// *conformance* verdict — it is what a validator should report. But the
    /// annotation is also unsatisfiable on this face's own geometry: both
    /// bounds are complete essential parallels, and no complete essential
    /// parallel bounds a disk on a cylinder, so neither loop can individually
    /// be an outer bound in the sense 10303-42 defines. The exporter's error
    /// is applying a simply-connected notion to an annulus.
    ///
    /// The repair therefore downgrades *only* the two outer-bound qualifiers
    /// to ordinary source bounds. Every other fact about the loops — vertex
    /// identity, traversal sense, induced homology, carrier order — is
    /// retained untouched and still had to be certified before this point.
    /// Material standing then comes from
    /// [`BandMaterialAuthority::IntrinsicBandCertificate`], not from either
    /// discarded qualifier.
    TwoOuterBoundsOnCertifiedBand,
}

/// Where an accepted band's material-region standing comes from.
///
/// Constructed only by [`band_material_authority`], which is the single place
/// the admission rule lives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BandMaterialAuthority {
    /// The source declared exactly one outer bound and it was used as given.
    SourceDeclared {
        /// Which bound, as the source indexed it.
        bound_index: u32,
    },
    /// The completed band certificate itself fixes the material region.
    ///
    /// The region is the closed axial interval between the two certified
    /// carriers crossed with one angular period, modulo the certified deck
    /// identification. That is a positive statement proved by
    /// [`certify_cylinder_band`] and [`plan_cut_open`] — not a guess that one
    /// loop is outer, and not an inference from area, order or orientation.
    IntrinsicBandCertificate {
        /// The repair that made this route necessary.
        repair: NonconformantRepair,
    },
}

impl BandMaterialAuthority {
    /// The conformance verdict this standing implies.
    pub fn conformance(self) -> SourceConformance {
        match self {
            Self::SourceDeclared { .. } => SourceConformance::Conforming,
            Self::IntrinsicBandCertificate { repair } => {
                SourceConformance::RecoveredFromMalformedSource(repair)
            }
        }
    }
}

/// Decide a certified band's material-region standing.
///
/// The admission rule, in one place. A band reaches the intrinsic route only
/// when the source's outer annotation is the specific unsatisfiable pattern
/// described in [`NonconformantRepair::TwoOuterBoundsOnCertifiedBand`] *and*
/// `band` is a completed [`CertifiedCylinderBand`] — which is itself the
/// proof of every conjunct the recovery rule requires: exactly two source
/// bounds, both complete essential parallels, physically distinct carriers in
/// certified axial order, compatible induced orientations, and hence a unique
/// compact strip between them. Holding a `&CertifiedCylinderBand` is how that
/// precondition is enforced; there is no way to ask for this standing without
/// one.
///
/// Note what is *not* admitted. A missing or unretained standing still
/// refuses, even on a perfect band: absent provenance is a gap in this
/// pipeline, not a statement by the file, and repairing a fact nobody stated
/// is exactly the guess [`OuterBoundStanding`] exists to prevent. Three or
/// more declared outer bounds also still refuse — the two-bound band is the
/// only shape whose annotation is provably unsatisfiable, so it is the only
/// one repaired.
pub fn band_material_authority(
    outer_bound: OuterBoundStanding,
    band: &CertifiedCylinderBand,
) -> Result<BandMaterialAuthority, BandExit> {
    let _ = band;
    match outer_bound {
        OuterBoundStanding::Declared {
            declared_count: 1,
            bound_index,
        } => Ok(BandMaterialAuthority::SourceDeclared { bound_index }),
        OuterBoundStanding::Declared {
            declared_count: 2, ..
        } => Ok(BandMaterialAuthority::IntrinsicBandCertificate {
            repair: NonconformantRepair::TwoOuterBoundsOnCertifiedBand,
        }),
        OuterBoundStanding::Declared { .. } => {
            Err(BandExit::Patch(SliceExit::MultipleOuterBoundsDeclared))
        }
        OuterBoundStanding::NotRetained | OuterBoundStanding::NoneDeclared => {
            Err(BandExit::Patch(SliceExit::MissingOuterBoundAuthority))
        }
    }
}

// ---------------------------------------------------------------------------
// Exits
// ---------------------------------------------------------------------------

/// Every way a face can fail to become a certified cylinder band, or fail to
/// be realized as one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BandExit {
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
    /// One bound could not be developed into the universal cover, or its deck
    /// placement did not resolve.
    BoundDevelopment {
        /// Which bound.
        bound: BoundId,
        /// That stage's own exit, unchanged.
        exit: CylinderLiftExit,
    },
    /// One bound carries an occurrence that is not a circumferential arc, so
    /// it is not a cylinder parallel at all.
    BoundNotAParallel {
        /// Which bound.
        bound: BoundId,
    },
    /// One bound's developed chain reverses direction, so it is not a
    /// *simple* parallel.
    BoundNotMonotone {
        /// Which bound.
        bound: BoundId,
    },
    /// One bound's certified homology is not primitive: `0` is contractible
    /// (the [`super::cylinder_arrangement`] population, not this one) and
    /// `|h| > 1` is multiply wound, which this slice does not model.
    BoundNotPrimitive {
        /// Which bound.
        bound: BoundId,
        /// The certified holonomy.
        homology: i64,
    },
    /// The two complete circles are certified to be the same circle: the
    /// enclosures overlap and the two bounds run over the same source
    /// vertices. A degenerate face, never an annulus.
    SameCarrier,
    /// The two enclosures overlap and no source fact separates or identifies
    /// the two circles, so neither carrier relation is established.
    CarrierIdentityUndecided,
    /// The two induced boundary homologies have the same sign. Refused, not
    /// repaired by reversing one of them.
    OrientationIncompatible {
        /// The first bound's homology.
        first: i64,
        /// The second bound's homology.
        second: i64,
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

impl From<AnnulusExit> for BandExit {
    /// Forward the shared realizer's exit into this module's own vocabulary,
    /// one variant to one variant.
    ///
    /// [`AnnulusExit::ObligationNotDischarged`] has no historical counterpart
    /// because it cannot arise here: a completed [`CertifiedCylinderBand`]
    /// proves every obligation the contract names, so the realizer's re-check
    /// is a restatement rather than a second gate. It maps onto
    /// [`BandExit::Patch`] with the triangulation exit so that, if it ever did
    /// fire, it would be reported as the operational failure it is rather than
    /// as a verdict about the face.
    fn from(exit: AnnulusExit) -> Self {
        match exit {
            AnnulusExit::ObligationNotDischarged { .. } => {
                Self::Patch(SliceExit::TriangulationDidNotComplete)
            }
            AnnulusExit::CutCoordinateUnavailable => Self::CutCoordinateUnavailable,
            AnnulusExit::Patch(exit) => Self::Patch(exit),
            AnnulusExit::RegluedDegenerateTriangle => Self::RegluedDegenerateTriangle,
            AnnulusExit::RegluedCutSurvives => Self::RegluedCutSurvives,
            AnnulusExit::RegluedNotConnected => Self::RegluedNotConnected,
            AnnulusExit::RegluedBoundaryComponents { components } => {
                Self::RegluedBoundaryComponents { components }
            }
            AnnulusExit::RegluedEulerCharacteristic { characteristic } => {
                Self::RegluedEulerCharacteristic { characteristic }
            }
            AnnulusExit::RegluedOrientationInconsistent => Self::RegluedOrientationInconsistent,
            AnnulusExit::LiftNotFinite => Self::LiftNotFinite,
        }
    }
}

impl BandExit {
    /// Which semantic category this exit belongs to.
    ///
    /// The line is authority, as everywhere else in the subtree. A face that
    /// simply is not a band — wrong bound count, a non-parallel bound, a
    /// non-primitive or same-signed homology, a coincident carrier — is
    /// `Unsupported`: a proved fact about the face, outside the admitted
    /// subset. Missing evidence (an undecided carrier, no cut coordinate) is
    /// `Unresolved`. A reglue predicate failing *after* every input
    /// obligation was discharged is a defect in this module, not a verdict
    /// about the face, so it is `OperationalFailure`.
    pub fn category(self) -> SliceCategory {
        match self {
            Self::NotTwoBounds { .. }
            | Self::BoundNotAParallel { .. }
            | Self::BoundNotMonotone { .. }
            | Self::BoundNotPrimitive { .. }
            | Self::SameCarrier
            | Self::OrientationIncompatible { .. } => SliceCategory::Unsupported,

            Self::CarrierIdentityUndecided | Self::CutCoordinateUnavailable => {
                SliceCategory::Unresolved
            }

            Self::BoundTraversal { exit, .. } | Self::Patch(exit) => exit.category(),
            Self::BoundDevelopment { exit, .. } => exit.category(),

            Self::RegluedDegenerateTriangle
            | Self::RegluedCutSurvives
            | Self::RegluedNotConnected
            | Self::RegluedBoundaryComponents { .. }
            | Self::RegluedEulerCharacteristic { .. }
            | Self::RegluedOrientationInconsistent
            | Self::LiftNotFinite => SliceCategory::OperationalFailure,
        }
    }

    /// A short stable tag, for probe and DIAG-001 records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::NotTwoBounds { .. } => "band_not_two_bounds",
            Self::BoundTraversal { exit, .. } => exit.tag(),
            Self::BoundDevelopment { exit, .. } => exit.tag(),
            Self::BoundNotAParallel { .. } => "band_bound_not_a_parallel",
            Self::BoundNotMonotone { .. } => "band_bound_not_monotone",
            Self::BoundNotPrimitive { .. } => "band_bound_not_primitive",
            Self::SameCarrier => "band_same_carrier",
            Self::CarrierIdentityUndecided => "band_carrier_identity_undecided",
            Self::OrientationIncompatible { .. } => "band_orientation_incompatible",
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
            Self::NotTwoBounds { .. } => "bounds",
            Self::BoundTraversal { .. } => "traversal",
            Self::BoundDevelopment { .. } => "development",
            Self::BoundNotAParallel { .. }
            | Self::BoundNotMonotone { .. }
            | Self::BoundNotPrimitive { .. } => "parallel",
            Self::SameCarrier | Self::CarrierIdentityUndecided => "carrier",
            Self::OrientationIncompatible { .. } => "band",
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
// Boundary components
// ---------------------------------------------------------------------------

/// Develop one authoritative bound and certify it a complete simple parallel.
///
/// Reuses [`traverse_bound`] (Step 2's authoritative, source-identity-only
/// traversal), [`develop_traversal_from_source`] (Step 4's production witness
/// route, which reads each occurrence's curve family and declared sweep from
/// its own source representation) and [`propagate_placements`] (Step 5's deck
/// walk) without modification. The three obligations added here are exactly
/// the ones "complete simple parallel" means beyond "closed developed chain":
/// every occurrence circumferential, the chain strictly monotone, and the
/// holonomy primitive.
pub fn develop_complete_parallel(
    bound: &SourceBoundInput,
    schema: &CylinderSchema,
    curves: &mut impl FnMut(usize) -> CurveSchema,
    vertex_position: &impl Fn(SourceVertexKey) -> Option<Point3>,
    family_of: &impl Fn(EdgeUseId) -> SourceCurveFamily,
) -> Result<CompleteParallel, BandExit> {
    let id = bound.id();
    let traversal = traverse_bound(bound, curves)
        .map_err(|exit| BandExit::BoundTraversal { bound: id, exit })?;
    let developed = develop_traversal_from_source(&traversal, schema, vertex_position, family_of)
        .map_err(|exit| BandExit::BoundDevelopment { bound: id, exit })?;

    // A parallel is a circle at constant axial coordinate. An axial line in
    // the bound proves the component is not one, before any placement is
    // consulted.
    if developed
        .witnesses
        .iter()
        .any(|witness| witness.class != WitnessClass::CircumferentialArc)
    {
        return Err(BandExit::BoundNotAParallel { bound: id });
    }

    let generator = schema.deck_generator();
    let walk = propagate_placements(&developed, generator)
        .map_err(|exit| BandExit::BoundDevelopment { bound: id, exit })?;

    // Primitive: exactly one turn around the cylinder. `0` is contractible
    // and `|h| > 1` is multiply wound; both are refused by name.
    if walk.holonomy != 1 && walk.holonomy != -1 {
        return Err(BandExit::BoundNotPrimitive {
            bound: id,
            homology: walk.holonomy,
        });
    }

    // Place every occurrence on its certified deck copy. This is the only
    // arithmetic applied to a witness, and it is the deck displacement the
    // walk certified — never a nearest-copy choice.
    let period = generator.signed_period().get();
    let starts: Vec<Point2> = developed
        .witnesses
        .iter()
        .zip(&walk.placements)
        .map(|(witness, &placement)| {
            Point2::new(witness.start.x, witness.start.y + placement as f64 * period)
        })
        .collect();
    let last = developed.witnesses.len() - 1;
    let terminal = Point2::new(
        developed.witnesses[last].end.x,
        developed.witnesses[last].end.y + walk.placements[last] as f64 * period,
    );

    // Simple: the developed chain advances in one angular direction only. A
    // chain that reverses covers some angle twice and is not a simple circle,
    // even when its holonomy is primitive.
    let direction = walk.holonomy as f64 * period.signum();
    let mut previous = starts[0].y;
    for point in starts[1..].iter().chain(std::iter::once(&terminal)) {
        if (point.y - previous) * direction <= 0.0 {
            return Err(BandExit::BoundNotMonotone { bound: id });
        }
        previous = point.y;
    }

    Ok(CompleteParallel {
        bound: id,
        edge_uses: traversal
            .occurrences
            .iter()
            .map(|occurrence| occurrence.edge_use)
            .collect(),
        start_vertices: traversal
            .occurrences
            .iter()
            .map(|occurrence| occurrence.start_vertex)
            .collect(),
        starts,
        terminal,
        homology: walk.holonomy,
    })
}

// ---------------------------------------------------------------------------
// Physical carriers
// ---------------------------------------------------------------------------

/// The complete physical circle a boundary component is carried by.
///
/// A circle on a certified embedded cylinder is fixed by four facts, and this
/// type carries all four: the support cylinder (shared by both carriers of a
/// band, and recorded by the caller), the circle's radius, the axis-normal
/// plane it lies in — named by the **observed extent** of every developed
/// endpoint the complete component visits, together with the certified bound
/// each of those was evaluated to, rather than by a single rounded level —
/// and the complete extent, which is `winding = ±1` because the component was
/// certified a complete simple parallel before a carrier was built for it.
///
/// The observed extent is `[observed_low, observed_high]` over *every*
/// endpoint of the complete circle, and `enclosure` is the radius-scaled
/// bound [`super::curve_witness`] already certifies each witness's
/// on-cylinder and constant-axial-coordinate claims at. The circle's true
/// axial level therefore lies in `[axial_low(), axial_high()]`. No mean, no
/// centroid, no single sampled point, no polyline point count and no epsilon
/// chosen here enters any of it.
#[derive(Debug, Clone, PartialEq)]
pub struct CylinderCircleCarrier {
    /// Which bound this carrier came from.
    pub bound: BoundId,
    /// The lowest axial coordinate observed anywhere on the complete circle.
    pub observed_low: f64,
    /// The highest axial coordinate observed anywhere on the complete circle.
    pub observed_high: f64,
    /// The certified per-coordinate evaluation bound.
    pub enclosure: f64,
    /// The circle's radius: the cylinder's own, since the circle is a
    /// complete parallel of it.
    pub radius: f64,
    /// The complete extent, in turns: `+1` or `-1`.
    pub winding: i64,
}

impl CylinderCircleCarrier {
    /// The low end of the certified enclosure of the circle's axial level.
    pub fn axial_low(&self) -> f64 {
        self.observed_low - self.enclosure
    }

    /// The high end of the certified enclosure of the circle's axial level.
    pub fn axial_high(&self) -> f64 {
        self.observed_high + self.enclosure
    }

    /// The circle's centre, given the cylinder it is a parallel of: the axis
    /// point at the enclosure's midpoint.
    pub fn centre(&self, schema: &CylinderSchema) -> Point3 {
        schema.origin() + 0.5 * (self.axial_low() + self.axial_high()) * schema.axis()
    }
}

/// The radius-scaled bound each developed axial coordinate is certified to.
///
/// This is [`super::curve_witness`]'s own `RELATIVE_TOLERANCE * max(radius,
/// 1)` — the bound at which every witness feeding this carrier already had
/// its "on the cylinder" and "at constant axial coordinate" claims accepted.
/// Reusing it, rather than choosing a fresh constant here, is what makes the
/// enclosure a restatement of an obligation already discharged instead of a
/// new tolerance.
fn axial_enclosure_bound(schema: &CylinderSchema) -> f64 {
    1.0e-9 * schema.radius().get().max(1.0)
}

/// Build the complete physical circle carrier for a certified parallel.
pub fn carrier_of(parallel: &CompleteParallel, schema: &CylinderSchema) -> CylinderCircleCarrier {
    let bound = axial_enclosure_bound(schema);
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for point in parallel
        .starts
        .iter()
        .chain(std::iter::once(&parallel.terminal))
    {
        low = low.min(point.x);
        high = high.max(point.x);
    }
    CylinderCircleCarrier {
        bound: parallel.bound,
        observed_low: low,
        observed_high: high,
        enclosure: bound,
        radius: schema.radius().get(),
        winding: parallel.homology,
    }
}

/// How two complete physical circles on one cylinder relate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CarrierRelation {
    /// Certified distinct: the two axial enclosures are strictly disjoint, so
    /// the circles cannot be the same circle and are strictly ordered along
    /// the axis. Names which carrier is lower.
    DistinctCarrier {
        /// `true` when the first carrier is the lower one.
        first_is_lower: bool,
        /// The certified gap between the two enclosures.
        separation: f64,
    },
    /// Certified coincident: the two complete circles have the same support
    /// cylinder, the same radius, the same complete extent, and one certified
    /// enclosure covers every axial coordinate observed on *either* of them —
    /// so they are the same circle.
    SameCarrier,
    /// Neither relation is established: the enclosures meet, so the circles
    /// are not separated, but the observed extents are together too wide for
    /// one enclosure to cover, so they are not certified equal either.
    Undecided,
}

/// Classify two complete physical circles on one cylinder, from their
/// complete geometry alone.
///
/// Two complete parallels of one cylinder are the same circle exactly when
/// they have the same radius, the same complete extent and the same axial
/// level. All three are decided here on the *complete* certified geometry —
/// the observed axial extent of every endpoint of each circle, against the
/// bound those coordinates were certified to — never on a mean parameter
/// value, a centroid, one sampled point, one projected vertex, a polyline
/// point count, a source-representation coincidence, or a threshold chosen
/// here.
///
/// The classifier is deliberately three-way, because floating evaluation of a
/// level admits exactly three states and not two:
///
/// - **Distinct.** The two enclosures are strictly disjoint, so no pair of
///   true levels inside them can be equal. The circles are proved different
///   and strictly ordered along the axis.
/// - **Same.** The union of the two *observed* extents is no wider than one
///   certified enclosure, so every axial coordinate either circle presents
///   is consistent with one single level — at the very precision at which
///   every constituent witness was already accepted onto this cylinder.
///   Together with equal radius and equal complete extent, that is complete
///   circle equality.
/// - **Undecided.** Everything else. Not rounded into either verdict.
///
/// Only the first admits a band, so all three outcomes are safe.
///
/// Source-representation facts are deliberately *not* consulted here. Two
/// bounds running over the identical source vertex cycle is strong evidence
/// of a duplicated representation, and
/// [`carriers_share_source_cycle`] reports it, but it is a statement about
/// how the file was written rather than about where the two circles are, so
/// it corroborates this verdict and never substitutes for it.
pub fn classify_carriers(
    first: &CylinderCircleCarrier,
    second: &CylinderCircleCarrier,
) -> CarrierRelation {
    if first.axial_high() < second.axial_low() {
        return CarrierRelation::DistinctCarrier {
            first_is_lower: true,
            separation: second.axial_low() - first.axial_high(),
        };
    }
    if second.axial_high() < first.axial_low() {
        return CarrierRelation::DistinctCarrier {
            first_is_lower: false,
            separation: first.axial_low() - second.axial_high(),
        };
    }

    // Same support cylinder, same complete extent, same circle plane. The
    // radius comparison is exact because both carriers read it from the one
    // certified schema the face is trimmed from; it is stated rather than
    // assumed so the certificate names every fact a circle is fixed by.
    let same_circle_family =
        first.radius == second.radius && first.winding.abs() == second.winding.abs();
    let union_low = first.observed_low.min(second.observed_low);
    let union_high = first.observed_high.max(second.observed_high);
    let covered = union_high - union_low <= first.enclosure.max(second.enclosure);
    match same_circle_family && covered {
        true => CarrierRelation::SameCarrier,
        false => CarrierRelation::Undecided,
    }
}

/// Whether two boundary components run over the identical set of source
/// vertices.
///
/// Corroboration only, never a carrier test: a duplicated representation of
/// one circle presents this way, but so could two genuinely distinct circles
/// if a file reused vertex entities, and a single circle written out twice
/// with fresh vertices would not present this way at all. The physical
/// verdict is [`classify_carriers`]'s.
pub fn carriers_share_source_cycle(first: &CompleteParallel, second: &CompleteParallel) -> bool {
    let (first, second) = (first.source_vertices(), second.source_vertices());
    !first.is_empty() && first.iter().all(|vertex| vertex.is_identified()) && first == second
}

// ---------------------------------------------------------------------------
// Band certification
// ---------------------------------------------------------------------------

/// A certified cylinder band: the compact strip between two distinct,
/// essential, oppositely oriented complete parallels of one cylinder.
#[derive(Debug, Clone)]
pub struct CertifiedCylinderBand {
    /// The document entity this face came from, when the importer retained it.
    pub source_face_id: Option<u64>,
    /// The certified embedded cylinder the band is trimmed from.
    pub cylinder: CertifiedEmbeddedCylinder,
    /// The boundary component whose carrier is lower along the axis.
    pub lower_boundary: CompleteParallel,
    /// The boundary component whose carrier is upper along the axis.
    pub upper_boundary: CompleteParallel,
    /// The lower carrier.
    pub lower_carrier: CylinderCircleCarrier,
    /// The upper carrier.
    pub upper_carrier: CylinderCircleCarrier,
    /// The certified gap between the two carriers' enclosures.
    pub separation: f64,
    /// The cylinder's signed deck period.
    pub period: f64,
}

impl CertifiedCylinderBand {
    /// The two boundary components in the source's own bound order, which is
    /// the order the cut-open patch traverses them in.
    fn in_source_order(&self) -> (&CompleteParallel, &CompleteParallel) {
        match self.lower_boundary.bound.0 <= self.upper_boundary.bound.0 {
            true => (&self.lower_boundary, &self.upper_boundary),
            false => (&self.upper_boundary, &self.lower_boundary),
        }
    }
}

/// Certify a two-bound cylinder face as a band.
///
/// Every conjunct of the admitted case is checked here, in an order chosen so
/// the *specific* obstruction is named: bound count, then each component's own
/// completeness, then the carriers, then the orientation. Nothing is repaired
/// to make a later conjunct hold.
pub fn certify_cylinder_band(
    source_face_id: Option<u64>,
    cylinder: CertifiedEmbeddedCylinder,
    input: &SourceFaceInput,
    curves: &mut impl FnMut(usize) -> CurveSchema,
    vertex_position: &impl Fn(SourceVertexKey) -> Option<Point3>,
    family_of: &impl Fn(EdgeUseId) -> SourceCurveFamily,
) -> Result<CertifiedCylinderBand, BandExit> {
    if input.bounds.len() != 2 {
        return Err(BandExit::NotTwoBounds {
            bounds: input.bounds.len(),
        });
    }
    let schema = cylinder.schema().clone();

    let first = develop_complete_parallel(
        &input.bounds[0],
        &schema,
        curves,
        vertex_position,
        family_of,
    )?;
    let second = develop_complete_parallel(
        &input.bounds[1],
        &schema,
        curves,
        vertex_position,
        family_of,
    )?;

    // Opposite induced orientations. Checked before the carriers so that a
    // same-signed pair is reported as the orientation fact it is, and never
    // repaired by reversing one component.
    if first.homology + second.homology != 0 {
        return Err(BandExit::OrientationIncompatible {
            first: first.homology,
            second: second.homology,
        });
    }

    let first_carrier = carrier_of(&first, &schema);
    let second_carrier = carrier_of(&second, &schema);
    let (first_is_lower, separation) = match classify_carriers(&first_carrier, &second_carrier) {
        CarrierRelation::DistinctCarrier {
            first_is_lower,
            separation,
        } => (first_is_lower, separation),
        CarrierRelation::SameCarrier => return Err(BandExit::SameCarrier),
        CarrierRelation::Undecided => return Err(BandExit::CarrierIdentityUndecided),
    };

    let (lower_boundary, upper_boundary, lower_carrier, upper_carrier) = match first_is_lower {
        true => (first, second, first_carrier, second_carrier),
        false => (second, first, second_carrier, first_carrier),
    };

    Ok(CertifiedCylinderBand {
        source_face_id,
        period: schema.deck_generator().signed_period().get(),
        cylinder,
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

/// Present a certified band to the shared realizer.
///
/// This is the whole of the handover, and it is deliberately one short
/// function so that what the cylinder claims can be read against what it
/// proved. Each obligation below is a restatement of a conjunct
/// [`certify_cylinder_band`] already discharged:
///
/// - the two components are in the source's own bound order, which is the
///   order the cut-open patch traverses them in;
/// - both carriers have the cylinder's own single certified radius, because a
///   complete parallel of a cylinder is a circle of that radius;
/// - [`CarrierOrder::DisjointEnclosures`] is exactly
///   [`CarrierRelation::DistinctCarrier`], restated in the chart's own terms;
/// - [`FreeDeckAction::GloballyRegularSupport`] is the cylinder's structural
///   fact and the reason this module never had an apex obligation: an embedded
///   cylinder of certified positive radius has **no** singular orbit at any
///   axial coordinate, so no strip of it can contain one. That is a statement
///   about the support, proved by [`super::cylinder::identify_cylinder`], not
///   an absence of contradiction.
fn realization_contract<'a>(
    band: &'a CertifiedCylinderBand,
    authority: BandMaterialAuthority,
) -> RankOnePeriodicAnnulus<'a> {
    // `authority` is a proof token, not data. Which of the cut-open cycle's
    // two complementary components is bounded is settled by the Jordan curve
    // theorem and does not vary with where the standing came from. What the
    // token establishes is that *some* route granted material standing at all
    // — the check `band_material_authority` is the only constructor of.
    // Requiring it by value keeps that step impossible to skip.
    let _: BandMaterialAuthority = authority;
    let (first, second) = band.in_source_order();
    let radius = band.cylinder.schema().radius().get();
    let first_is_lower = std::ptr::eq(first, &band.lower_boundary);
    RankOnePeriodicAnnulus {
        first: AnnulusBoundary {
            parallel: first,
            carrier_radius: radius,
        },
        second: AnnulusBoundary {
            parallel: second,
            carrier_radius: radius,
        },
        period: band.period,
        carrier_order: CarrierOrder::DisjointEnclosures {
            first_is_lower,
            separation: band.separation,
        },
        free_deck_action: FreeDeckAction::GloballyRegularSupport,
        cell: AnnulusCell::CylinderEssentialBand,
    }
}

/// Choose the band's cut. See [`super::rank1_annulus::plan_cut_open`].
pub fn plan_cut_open(band: &CertifiedCylinderBand) -> Result<CutOpenDomainPlan, BandExit> {
    let (first, second) = band.in_source_order();
    super::rank1_annulus::plan_cut_open(first, second, band.period).map_err(BandExit::from)
}

/// Cut a certified band open into one planar patch. See
/// [`super::rank1_annulus::cut_open`].
pub fn cut_open(
    band: &CertifiedCylinderBand,
    plan: &CutOpenDomainPlan,
    authority: BandMaterialAuthority,
    tolerance: f64,
) -> Result<PlanarPatch, BandExit> {
    let annulus = realization_contract(band, authority);
    super::rank1_annulus::cut_open(&annulus, plan, tolerance).map_err(BandExit::from)
}

/// Triangulate the cut-open patch. See
/// [`super::rank1_annulus::triangulate_annulus_patch`].
pub fn triangulate_band_patch(patch: &PlanarPatch) -> Result<TriangulatedRegion, BandExit> {
    super::rank1_annulus::triangulate_annulus_patch(patch).map_err(BandExit::from)
}

/// The band's complete product: a validated annular mesh on the cylinder.
#[derive(Debug, Clone)]
pub struct CertifiedBandMesh {
    /// The developed complex, after the identification was discharged.
    pub developed: TriangulatedRegion,
    /// The cut-open patch's own final validity report, before the reglue.
    pub patch_validity: FinalValidityReport,
    /// The annular complex's validity report, after it.
    pub validity: BandValidityReport,
    /// The developed vertices lifted onto the cylinder, in the same order as
    /// `developed.vertices`.
    pub physical_vertices: Vec<Point3>,
    /// Whether the source this mesh came from was conformant, or was repaired
    /// by a named nonconformant normalization.
    ///
    /// [`reglue`] cannot know this — it sees a patch, not a file — so it
    /// writes the conservative value and [`run_cylinder_band`] overwrites it
    /// with the authority's own verdict. The mesh is equally valid either
    /// way; what differs is what may be claimed about the *file*.
    pub conformance: SourceConformance,
}

/// Discharge the identification and validate the annulus. See
/// [`super::rank1_annulus::reglue`].
///
/// The lift handed to the shared realizer is [`lift_to_cylinder`], the one
/// place in the realization that knows the support is a cylinder.
pub fn reglue(
    patch: &PlanarPatch,
    developed: TriangulatedRegion,
    schema: &CylinderSchema,
) -> Result<CertifiedBandMesh, BandExit> {
    let realized = super::rank1_annulus::reglue(
        patch,
        developed,
        AnnulusCell::CylinderEssentialBand,
        &|region| lift_to_cylinder(region, schema),
    )
    .map_err(BandExit::from)?;
    Ok(CertifiedBandMesh {
        // The conservative default. `run_cylinder_band` replaces it with the
        // authority's verdict; a caller driving `reglue` directly gets the
        // claim that assumes least about the source.
        conformance: SourceConformance::Conforming,
        developed: realized.developed,
        patch_validity: realized.patch_validity,
        validity: realized.validity,
        physical_vertices: realized.physical_vertices,
    })
}

/// The whole band path, composed: certify, plan the cut, cut open,
/// triangulate, reglue and lift.
pub fn run_cylinder_band(
    source_face_id: Option<u64>,
    cylinder: CertifiedEmbeddedCylinder,
    input: &SourceFaceInput,
    outer_bound: OuterBoundStanding,
    curves: &mut impl FnMut(usize) -> CurveSchema,
    vertex_position: &impl Fn(SourceVertexKey) -> Option<Point3>,
    family_of: &impl Fn(EdgeUseId) -> SourceCurveFamily,
    tolerance: f64,
) -> Result<(CertifiedCylinderBand, CertifiedBandMesh), BandExit> {
    let band = certify_cylinder_band(
        source_face_id,
        cylinder,
        input,
        curves,
        vertex_position,
        family_of,
    )?;
    // Strictly after certification, and that order is the safety argument:
    // the nonconformant repair is admissible only *because* the band is
    // already proved, so the authority question cannot be asked until there
    // is a `CertifiedCylinderBand` to ask it against.
    let authority = band_material_authority(outer_bound, &band)?;
    let plan = plan_cut_open(&band)?;
    let patch = cut_open(&band, &plan, authority, tolerance)?;
    let developed = triangulate_band_patch(&patch)?;
    let mut mesh = reglue(&patch, developed, band.cylinder.schema())?;
    mesh.conformance = authority.conformance();
    Ok((band, mesh))
}

#[cfg(test)]
mod tests;
