//! Checkpoint 8: arrangement and material-disk validation for a rank-1
//! cylinder disk.
//!
//! # Reuse, not a second kernel
//!
//! The developed boundary is a chain of straight segments in `(axial,
//! angular)` space — an axial line is literally straight, and a
//! circumferential arc is linear in the developed chart by the certified
//! convention (`FORMAL-006`, confirmed again by [`super::curve_witness`]) —
//! so it is exactly the shape [`super::planar_slice::jordan_arrangement_of`]
//! and [`super::planar_slice::bounded_material_region`] already certify. Both
//! are called unchanged; this module's only original content is the
//! cross-translate check the two-phase working cover
//! ([`super::cylinder_cover`]) exists to bound: the disk must be disjoint
//! from every *other* deck copy the cover materialized, not merely simple in
//! its own copy. That check reuses
//! [`super::planar_holes::classify_components`] and
//! [`super::planar_holes::point_strictly_inside`] — the same exact-predicate
//! kernel [`super::planar_holes`] already uses to relate an outer loop to its
//! holes — rather than a new pairwise-segment implementation.

#[cfg(test)]
use super::super::source_evidence::BoundId;
use super::super::source_evidence::{EdgeUseId, SourceVertexKey};
use super::curve_witness::CurveOnCylinderWitness;
use super::deck::DeckGenerator;
use super::numeric::NonNegativeFinite;
use super::planar_holes::{classify_components, point_strictly_inside, ComponentRelation};
use super::planar_slice::{
    bounded_material_region, jordan_arrangement_of, BoundedMaterialRegion, CertificateRoute,
    CertifiedPlanarCurveOccurrence, SimpleJordanArrangement, SliceCategory, SliceExit,
};
use truck_geometry::prelude::Point2;
use truck_topology::compress::OuterBoundStanding;

/// Why the cylinder arrangement or material-disk stage could not certify a
/// valid disk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CylinderArrangementExit {
    /// The base developed boundary failed one of the reused planar Jordan or
    /// material-selection obligations. Carries that stage's own exit
    /// unchanged.
    Base(SliceExit),
    /// A nonzero deck translate of the boundary crosses the base boundary.
    TranslateCrosses {
        /// The offending deck index.
        k: i64,
    },
    /// A nonzero deck translate touches the base boundary without crossing.
    TranslateTouches {
        /// The offending deck index.
        k: i64,
    },
    /// A nonzero deck translate shares a positive-length overlap with the
    /// base boundary.
    TranslateOverlaps {
        /// The offending deck index.
        k: i64,
    },
    /// A nonzero deck translate's boundary does not meet the base boundary
    /// but lies inside the base material disk (or the base boundary lies
    /// inside the translate's), so the two disks are not disjoint.
    TranslateDiskNotDisjoint {
        /// The offending deck index.
        k: i64,
    },
    /// Containment between the base disk and a nonzero translate could not
    /// be decided (the probe point landed exactly on a boundary).
    TranslateContainmentUndecided {
        /// The offending deck index.
        k: i64,
    },
}

impl CylinderArrangementExit {
    /// Which semantic category this exit belongs to.
    pub fn category(self) -> SliceCategory {
        match self {
            Self::Base(exit) => exit.category(),
            Self::TranslateCrosses { .. }
            | Self::TranslateTouches { .. }
            | Self::TranslateOverlaps { .. }
            | Self::TranslateDiskNotDisjoint { .. } => SliceCategory::Unsupported,
            Self::TranslateContainmentUndecided { .. } => SliceCategory::Unresolved,
        }
    }

    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Base(exit) => exit.tag(),
            Self::TranslateCrosses { .. } => "cylinder_translate_crosses",
            Self::TranslateTouches { .. } => "cylinder_translate_touches",
            Self::TranslateOverlaps { .. } => "cylinder_translate_overlaps",
            Self::TranslateDiskNotDisjoint { .. } => "cylinder_translate_disk_not_disjoint",
            Self::TranslateContainmentUndecided { .. } => {
                "cylinder_translate_containment_undecided"
            }
        }
    }
}

impl From<SliceExit> for CylinderArrangementExit {
    fn from(value: SliceExit) -> Self {
        Self::Base(value)
    }
}

/// Step 7/8's rank-1 product: the base developed disk, certified simple and
/// certified disjoint from every other deck copy the working cover
/// materialized.
#[derive(Debug, Clone)]
pub struct CertifiedCylinderDisk {
    /// The base (deck index `0` relative to the fixed initial placement)
    /// material region, exactly as the reused planar machinery certifies it.
    pub material: BoundedMaterialRegion,
    /// Every other materialized deck index checked disjoint against it, in
    /// the order checked.
    pub translates_checked: Vec<i64>,
}

/// Build the placed developed occurrence list [`jordan_arrangement_of`]
/// expects, from certified witnesses and their certified deck placements.
///
/// Each witness is already linear in the developed chart (see the module
/// docs), so its two-point `points` chain carries zero represented
/// approximation error — [`NonNegativeFinite::new`] of `0.0` is exact, not a
/// placeholder. `start_vertex`/`end_vertex` are not reused by
/// [`jordan_arrangement_of`] or [`bounded_material_region`] and are recorded
/// as [`SourceVertexKey::Absent`] rather than invented.
///
/// `pub`, not `pub(super)`: [`certify_cylinder_mesh`]'s production caller
/// needs the identical occurrence list [`certify_cylinder_disk`] built
/// internally, and re-deriving a second copy by hand outside `formal` would
/// risk it silently drifting from the one the disk was actually certified
/// against. A visibility widening only — the construction itself is
/// untouched.
///
/// [`certify_cylinder_mesh`]: super::cylinder_mesh::certify_cylinder_mesh
pub fn placed_occurrences(
    edge_uses: &[EdgeUseId],
    witnesses: &[CurveOnCylinderWitness],
    placements: &[i64],
    generator: &DeckGenerator,
) -> Vec<CertifiedPlanarCurveOccurrence> {
    let period = generator.signed_period().get();
    edge_uses
        .iter()
        .zip(witnesses)
        .zip(placements)
        .map(|((edge_use, witness), &placement)| {
            let shift = placement as f64 * period;
            let start = Point2::new(witness.start.x, witness.start.y + shift);
            let end = Point2::new(witness.end.x, witness.end.y + shift);
            CertifiedPlanarCurveOccurrence {
                edge_use: *edge_use,
                start_vertex: SourceVertexKey::Absent,
                end_vertex: SourceVertexKey::Absent,
                points: vec![start, end],
                route: CertificateRoute::AnalyticCylinderDevelopment,
                endpoint_reconciliation: NonNegativeFinite::new(0.0)
                    .expect("zero is a valid nonnegative bound"),
            }
        })
        .collect()
}

/// Shift every vertex of a cycle by `k` deck periods on the angular
/// (second developed) axis.
fn translated_cycle(cycle: &[Point2], k: i64, generator: &DeckGenerator) -> Vec<Point2> {
    let shift = k as f64 * generator.signed_period().get();
    cycle
        .iter()
        .map(|p| Point2::new(p.x, p.y + shift))
        .collect()
}

/// Step 7 and 8. Certify the base developed boundary as a simple Jordan
/// curve with a bounded material region (reusing
/// [`jordan_arrangement_of`]/[`bounded_material_region`] unchanged), then
/// certify the resulting disk disjoint from every other deck copy
/// [`super::cylinder_cover::WorkingCoverResult::materialized_copies`]
/// produced.
///
/// `base_placements` are the same certified placements
/// [`super::cylinder_lift::propagate_and_classify_holonomy`] returned (deck
/// index `0` of the working cover, in the two-phase construction's terms).
/// `materialized_copies` names every deck index the cover certified might
/// matter; this function checks the base disk against the translate of the
/// *whole* boundary by each one, skipping the identity translate.
pub fn certify_cylinder_disk(
    edge_uses: &[EdgeUseId],
    witnesses: &[CurveOnCylinderWitness],
    base_placements: &[i64],
    generator: DeckGenerator,
    outer_bound: OuterBoundStanding,
    materialized_copies: &[i64],
) -> Result<CertifiedCylinderDisk, CylinderArrangementExit> {
    let occurrences = placed_occurrences(edge_uses, witnesses, base_placements, &generator);
    let arrangement: SimpleJordanArrangement = jordan_arrangement_of(&occurrences)?;
    let material = bounded_material_region(arrangement, outer_bound)?;

    let base_cycle = &material.boundary.cycle;
    let mut translates_checked = Vec::new();
    for &k in materialized_copies {
        if k == 0 {
            continue;
        }
        let other = translated_cycle(base_cycle, k, &generator);
        match classify_components(base_cycle, &other) {
            ComponentRelation::Cross => {
                return Err(CylinderArrangementExit::TranslateCrosses { k })
            }
            ComponentRelation::Touch => {
                return Err(CylinderArrangementExit::TranslateTouches { k })
            }
            ComponentRelation::Overlap => {
                return Err(CylinderArrangementExit::TranslateOverlaps { k })
            }
            ComponentRelation::Disjoint => {
                // No boundary contact: decide whether one disk still nests
                // inside the other by testing one representative vertex.
                match point_strictly_inside(other[0], base_cycle) {
                    Some(true) => {
                        return Err(CylinderArrangementExit::TranslateDiskNotDisjoint { k })
                    }
                    Some(false) => {}
                    None => {
                        return Err(CylinderArrangementExit::TranslateContainmentUndecided { k })
                    }
                }
            }
        }
        translates_checked.push(k);
    }

    Ok(CertifiedCylinderDisk {
        material,
        translates_checked,
    })
}

#[cfg(test)]
mod tests {
    use super::super::curve_witness::{axial_line_witness, circumferential_arc_witness};
    use super::super::cylinder::{identify_cylinder, CylinderIdentification};
    use super::*;
    use truck_geometry::prelude::{Line, Point3, RevolutedCurve, Vector3};

    fn z_cylinder(radius: f64, h: f64) -> super::super::cylinder::CylinderSchema {
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(radius, 0.0, 0.0), Point3::new(radius, 0.0, h)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        match identify_cylinder(&revo) {
            CylinderIdentification::Cylinder(c) => c.schema().clone(),
            other => panic!("expected a certified cylinder, got {other:?}"),
        }
    }

    fn on_cylinder(schema: &super::super::cylinder::CylinderSchema, z: f64, theta: f64) -> Point3 {
        schema.origin()
            + z * schema.axis()
            + schema.radius().get() * theta.cos() * schema.radial_x()
            + schema.radius().get() * theta.sin() * schema.radial_y()
    }

    fn declared_outer() -> OuterBoundStanding {
        OuterBoundStanding::Declared {
            bound_index: 0,
            declared_count: 1,
        }
    }

    fn quad_witnesses(
        schema: &super::super::cylinder::CylinderSchema,
    ) -> (Vec<EdgeUseId>, Vec<CurveOnCylinderWitness>) {
        let p0 = on_cylinder(schema, 0.0, 0.2);
        let p1 = on_cylinder(schema, 0.0, 1.4);
        let p2 = on_cylinder(schema, 3.0, 1.4);
        let p3 = on_cylinder(schema, 3.0, 0.2);
        let witnesses = vec![
            circumferential_arc_witness(schema, p0, p1, 1.2).unwrap(),
            axial_line_witness(schema, p1, p2).unwrap(),
            circumferential_arc_witness(schema, p2, p3, -1.2).unwrap(),
            axial_line_witness(schema, p3, p0).unwrap(),
        ];
        let edge_uses = (0..4).map(|i| EdgeUseId::new(BoundId(0), i)).collect();
        (edge_uses, witnesses)
    }

    /// A narrow non-wrapping quad certifies: it is simple, has a bounded
    /// material region, and its only materialized deck copy is its own
    /// (`k = 0`), so there is nothing to check it against.
    #[test]
    fn a_narrow_quad_certifies_with_no_other_copies_to_check() {
        let schema = z_cylinder(2.0, 5.0);
        let (edge_uses, witnesses) = quad_witnesses(&schema);
        let placements = vec![0i64, 0, 0, 0];

        let disk = certify_cylinder_disk(
            &edge_uses,
            &witnesses,
            &placements,
            schema.deck_generator(),
            declared_outer(),
            &[0],
        )
        .expect("a narrow quad is a valid disk with only the identity copy in its cover");
        assert!(disk.translates_checked.is_empty());
        assert!(disk.material.signed_area.abs() > 0.0);
    }

    /// A quad whose angular width exceeds one full period must be refused
    /// against its own translate, not silently accepted. The placements here
    /// are the ones the checkpoint-6 join solve would actually certify for
    /// this boundary (computed by hand and asserted, not assumed): the
    /// sweep exceeds `2*PI`, so the axial-line joins each need a nonzero
    /// deck integer even though the overall holonomy is still `h = 0`.
    #[test]
    fn a_self_overlapping_translate_is_refused() {
        let schema = z_cylinder(2.0, 5.0);
        let sweep = 6.5; // > TAU = 6.283...
        let p0 = on_cylinder(&schema, 0.0, 0.0);
        let p1 = on_cylinder(&schema, 0.0, sweep);
        let p2 = on_cylinder(&schema, 3.0, sweep);
        let p3 = on_cylinder(&schema, 3.0, 0.0);
        let witnesses = vec![
            circumferential_arc_witness(&schema, p0, p1, sweep).unwrap(),
            axial_line_witness(&schema, p1, p2).unwrap(),
            circumferential_arc_witness(&schema, p2, p3, -sweep).unwrap(),
            axial_line_witness(&schema, p3, p0).unwrap(),
        ];
        let edge_uses: Vec<_> = (0..4).map(|i| EdgeUseId::new(BoundId(0), i)).collect();
        // n_0 = 0 fixed; join0 needs k=+1 (the arc's sweep overtakes one
        // period), join1 needs k=0 (axial, no angular change), join2 needs
        // k=-1 (the return arc gives it back), join3 (wrap) needs k=0 --
        // net holonomy 0.
        let placements = vec![0i64, 1, 1, 0];

        let exit = certify_cylinder_disk(
            &edge_uses,
            &witnesses,
            &placements,
            schema.deck_generator(),
            declared_outer(),
            &[-1, 0, 1],
        )
        .expect_err("a wide quad must not certify as disjoint from its own translate");
        assert_eq!(exit.category(), SliceCategory::Unsupported);
        assert!(matches!(
            exit,
            CylinderArrangementExit::TranslateCrosses { .. }
                | CylinderArrangementExit::TranslateTouches { .. }
                | CylinderArrangementExit::TranslateOverlaps { .. }
                | CylinderArrangementExit::TranslateDiskNotDisjoint { .. }
        ));
    }

    /// A base-stage failure (here: too few occurrences to close a Jordan
    /// curve) is propagated through unchanged from the reused planar
    /// machinery.
    #[test]
    fn a_base_stage_failure_is_reported_through_unchanged() {
        let schema = z_cylinder(2.0, 5.0);
        let p0 = on_cylinder(&schema, 0.0, 0.2);
        let p1 = on_cylinder(&schema, 0.0, 1.4);
        let p2 = on_cylinder(&schema, 3.0, 1.4);
        let witnesses = vec![
            circumferential_arc_witness(&schema, p0, p1, 1.2).unwrap(),
            axial_line_witness(&schema, p1, p2).unwrap(),
        ];
        let edge_uses: Vec<_> = (0..2).map(|i| EdgeUseId::new(BoundId(0), i)).collect();
        let placements = vec![0i64, 0];

        let exit = certify_cylinder_disk(
            &edge_uses,
            &witnesses,
            &placements,
            schema.deck_generator(),
            declared_outer(),
            &[0],
        )
        .expect_err("two occurrences cannot close a Jordan curve");
        assert!(matches!(exit, CylinderArrangementExit::Base(_)));
    }
}
