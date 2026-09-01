// Grandfathered (orchestrator amendment, BG-CK-P0-CRATE r3): moved
// verbatim from truck-meshalgo, whose crate never denied
// clippy::unwrap_used. The crate-level deny in lib.rs is H-1's contract
// for AUTHORED certified code; this module's pre-existing unwraps are
// inherited baseline content and must not be force-rewritten by the
// move packet. Do not add new unwraps under this allow.
#![allow(clippy::unwrap_used)]

//! Checkpoint 7: the exact two-phase working cover for a zero-holonomy
//! cylinder disk.
//!
//! # The two phases
//!
//! `FORMAL_SYSTEM.md`'s two-phase construction is: build `K_boundary`, the
//! deck indices whose translate of the placed developed boundary could meet
//! the boundary itself; then build `K_region`, the same question for the
//! bounded material region; then take `K_final = K_boundary ∪ K_region`.
//!
//! For this checkpoint's supported subset — one outer bound, no holes — the
//! material region's developed enclosure is contained in the boundary's own
//! enclosing box (there is no hole to carve a larger candidate region out
//! of), so both phases are computed from the *same* box and
//! [`build_working_cover`] says so rather than pretending a second,
//! independently-derived box was built. A hole-bearing expansion would give
//! `K_region` a genuinely different candidate box; recording both fields
//! separately, instead of collapsing them into one, is what keeps that
//! expansion from having to touch this function's signature.
//!
//! # Why this reuses [`deck_cover_interval`] rather than enumerating
//!
//! [`super::deck::deck_cover_interval`] (Milestone 1B) already computes the
//! conservative self-difference cover from two `f64` endpoints, with no
//! enumeration and no truncation of a wide-but-finite interval. This module
//! adds only the placement-aware bounding box the cylinder case needs before
//! calling it, and the materialization budget that gates turning a `Finite`
//! range into an actual list of deck indices.

use super::curve_witness::CurveOnCylinderWitness;
use super::deck::{
    deck_cover_interval, CertifiedDeckInterval, DeckBudget, DeckGenerator, DeckInterval,
    DeckOperationalFailure, DevelopedBox,
};
use super::planar_slice::SliceCategory;

/// Why the working cover could not be certified.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CylinderCoverExit {
    /// The placed developed boundary has no finite enclosing box (a
    /// non-finite coordinate reached this stage).
    BoundaryEnclosureUnavailable,
    /// The period is too small, at the enclosure's scale, for the arithmetic
    /// to resolve adjacent deck integers.
    CoverIndeterminate,
    /// The deck arithmetic itself failed (overflow).
    ArithmeticFailure(DeckOperationalFailure),
    /// The certified cover is finite but wider than the materialization
    /// budget permits.
    CoverBudgetExceeded {
        /// How many copies the certified cover would require.
        count: u64,
        /// How many the budget permits.
        cap: u64,
    },
}

impl CylinderCoverExit {
    /// Which semantic category this exit belongs to.
    pub fn category(self) -> SliceCategory {
        match self {
            Self::BoundaryEnclosureUnavailable => SliceCategory::Unresolved,
            Self::CoverIndeterminate => SliceCategory::Unresolved,
            Self::ArithmeticFailure(_) | Self::CoverBudgetExceeded { .. } => {
                SliceCategory::OperationalFailure
            }
        }
    }

    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::BoundaryEnclosureUnavailable => "cover_boundary_enclosure_unavailable",
            Self::CoverIndeterminate => "cover_indeterminate",
            Self::ArithmeticFailure(_) => "cover_arithmetic_failure",
            Self::CoverBudgetExceeded { .. } => "cover_budget_exceeded",
        }
    }
}

/// Step 7's product: the certified two-phase cover and the deck indices
/// actually materialized.
#[derive(Debug, Clone)]
pub struct WorkingCoverResult {
    /// The placed developed boundary's enclosing box.
    pub boundary_box: DevelopedBox,
    /// `K_boundary`: the cover of the boundary against itself.
    pub boundary_cover: CertifiedDeckInterval,
    /// `K_region`: the cover of the candidate material region against
    /// itself. Equal to `boundary_cover` in the supported hole-free subset;
    /// see the module docs.
    pub region_cover: CertifiedDeckInterval,
    /// `K_final = K_boundary ∪ K_region`.
    pub final_cover: CertifiedDeckInterval,
    /// The deck indices materialized from `final_cover`, within budget.
    pub materialized_copies: Vec<i64>,
}

/// The union of two certified deck covers: a conservative superset of both.
///
/// `Indeterminate` dominates (neither phase resolved adjacent integers at
/// this scale), and two finite ranges union to their convex hull — itself
/// still a conservative superset, never a false negative, exactly the
/// discipline [`deck_cover_interval`] already commits to.
fn union_cover(a: CertifiedDeckInterval, b: CertifiedDeckInterval) -> CertifiedDeckInterval {
    match (a, b) {
        (CertifiedDeckInterval::Indeterminate, _) | (_, CertifiedDeckInterval::Indeterminate) => {
            CertifiedDeckInterval::Indeterminate
        }
        (CertifiedDeckInterval::Empty, other) | (other, CertifiedDeckInterval::Empty) => other,
        (
            CertifiedDeckInterval::Finite {
                min: a_min,
                max: a_max,
            },
            CertifiedDeckInterval::Finite {
                min: b_min,
                max: b_max,
            },
        ) => CertifiedDeckInterval::Finite {
            min: a_min.min(b_min),
            max: a_max.max(b_max),
        },
    }
}

/// The placed developed enclosing box of one occurrence: its two endpoints,
/// translated onto their certified deck copy.
///
/// Both curve families here develop to a straight segment in `(axial,
/// angular)` space (an axial line is literally straight; a circumferential
/// arc is linear in the angular coordinate by the certified convention), so
/// the endpoints' bounding box already contains the complete occurrence —
/// there is no interior extremum to miss, unlike a general curve.
fn placed_occurrence_box(
    witness: &CurveOnCylinderWitness,
    placement: i64,
    generator: &DeckGenerator,
) -> Result<DevelopedBox, CylinderCoverExit> {
    let shift = placement as f64 * generator.signed_period().get();
    let (start_axial, start_angular) = (witness.start.x, witness.start.y + shift);
    let (end_axial, end_angular) = (witness.end.x, witness.end.y + shift);
    if ![start_axial, start_angular, end_axial, end_angular]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err(CylinderCoverExit::BoundaryEnclosureUnavailable);
    }
    let axial = DeckInterval::from_f64(start_axial.min(end_axial), start_axial.max(end_axial))
        .map_err(|_| CylinderCoverExit::BoundaryEnclosureUnavailable)?;
    let angular = DeckInterval::from_f64(
        start_angular.min(end_angular),
        start_angular.max(end_angular),
    )
    .map_err(|_| CylinderCoverExit::BoundaryEnclosureUnavailable)?;
    Ok(DevelopedBox {
        first: axial,
        second: angular,
    })
}

fn union_box(a: DevelopedBox, b: DevelopedBox) -> DevelopedBox {
    DevelopedBox {
        first: DeckInterval::from_f64(
            a.first.lower().get().min(b.first.lower().get()),
            a.first.upper().get().max(b.first.upper().get()),
        )
        .expect("finite inputs stay finite under min/max"),
        second: DeckInterval::from_f64(
            a.second.lower().get().min(b.second.lower().get()),
            a.second.upper().get().max(b.second.upper().get()),
        )
        .expect("finite inputs stay finite under min/max"),
    }
}

/// Step 7. Build the certified two-phase working cover from the developed
/// witnesses and their certified deck placements ([`ZeroHolonomyLift`]'s
/// output — the placements are trusted as already-certified `h = 0` input;
/// this function does not re-check holonomy).
///
/// `budget` gates only the *materialization* of translated copies: the
/// certified cover itself is never truncated for being wide, per
/// `FORMAL_SYSTEM.md`'s ban on that shortcut.
pub fn build_working_cover(
    witnesses: &[CurveOnCylinderWitness],
    placements: &[i64],
    generator: DeckGenerator,
    budget: DeckBudget,
) -> Result<WorkingCoverResult, CylinderCoverExit> {
    debug_assert_eq!(witnesses.len(), placements.len());

    let mut boxes = Vec::with_capacity(witnesses.len());
    for (witness, &placement) in witnesses.iter().zip(placements) {
        boxes.push(placed_occurrence_box(witness, placement, &generator)?);
    }
    let boundary_box = boxes
        .into_iter()
        .reduce(union_box)
        .ok_or(CylinderCoverExit::BoundaryEnclosureUnavailable)?;

    // The candidate material region's enclosure coincides with the
    // boundary's own, in the hole-free supported subset; see the module
    // docs. Both phases are therefore the identical computation on the
    // identical box, recorded as two fields rather than one so a
    // hole-bearing expansion can give `region_cover` its own box without
    // changing this function's shape.
    let region_box = boundary_box;

    let boundary_cover = deck_cover_interval(&generator, &boundary_box, &boundary_box)
        .map_err(CylinderCoverExit::ArithmeticFailure)?;
    let region_cover = deck_cover_interval(&generator, &region_box, &region_box)
        .map_err(CylinderCoverExit::ArithmeticFailure)?;
    let final_cover = union_cover(boundary_cover.clone(), region_cover.clone());

    if matches!(final_cover, CertifiedDeckInterval::Indeterminate) {
        return Err(CylinderCoverExit::CoverIndeterminate);
    }

    let materialized_copies = match &final_cover {
        CertifiedDeckInterval::Empty => Vec::new(),
        CertifiedDeckInterval::Indeterminate => unreachable!("handled above"),
        CertifiedDeckInterval::Finite { min, max } => {
            let count = final_cover
                .finite_count()
                .ok_or(CylinderCoverExit::CoverIndeterminate)?;
            if count > budget.deck_width_cap {
                return Err(CylinderCoverExit::CoverBudgetExceeded {
                    count,
                    cap: budget.deck_width_cap,
                });
            }
            (*min..=*max).collect()
        }
    };

    Ok(WorkingCoverResult {
        boundary_box,
        boundary_cover,
        region_cover,
        final_cover,
        materialized_copies,
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

    /// The same non-wrapping quad from `cylinder_lift`'s first milestone
    /// test: every placement is zero, and the cover need only include `k =
    /// 0` (plus whatever neighboring indices the boundary's own angular
    /// width brings in — width `1.2`, far short of the `2*PI` period, so
    /// only `k = 0` is ever compatible).
    #[test]
    fn a_non_wrapping_quad_needs_only_the_zero_copy() {
        let schema = z_cylinder(2.0, 5.0);
        let p0 = on_cylinder(&schema, 0.0, 0.2);
        let p1 = on_cylinder(&schema, 0.0, 1.4);
        let p2 = on_cylinder(&schema, 3.0, 1.4);
        let p3 = on_cylinder(&schema, 3.0, 0.2);
        let witnesses = vec![
            circumferential_arc_witness(&schema, p0, p1, 1.2).unwrap(),
            axial_line_witness(&schema, p1, p2).unwrap(),
            circumferential_arc_witness(&schema, p2, p3, -1.2).unwrap(),
            axial_line_witness(&schema, p3, p0).unwrap(),
        ];
        let placements = vec![0i64, 0, 0, 0];

        let result = build_working_cover(
            &witnesses,
            &placements,
            schema.deck_generator(),
            DeckBudget::FOR_TESTING,
        )
        .expect("a narrow non-wrapping quad certifies a small cover");

        assert!(result.materialized_copies.contains(&0));
        assert_eq!(result.boundary_cover, result.region_cover);
        assert_eq!(result.final_cover, result.boundary_cover);
    }

    /// A budget too small to materialize even the zero-width minimum cover
    /// is reported as an operational failure, not silently truncated.
    #[test]
    fn an_exhausted_budget_is_reported_as_operational_failure() {
        let schema = z_cylinder(2.0, 5.0);
        let p0 = on_cylinder(&schema, 0.0, 0.2);
        let p1 = on_cylinder(&schema, 0.0, 1.4);
        let p2 = on_cylinder(&schema, 3.0, 1.4);
        let p3 = on_cylinder(&schema, 3.0, 0.2);
        let witnesses = vec![
            circumferential_arc_witness(&schema, p0, p1, 1.2).unwrap(),
            axial_line_witness(&schema, p1, p2).unwrap(),
            circumferential_arc_witness(&schema, p2, p3, -1.2).unwrap(),
            axial_line_witness(&schema, p3, p0).unwrap(),
        ];
        let placements = vec![0i64, 0, 0, 0];
        let starved = DeckBudget { deck_width_cap: 0 };

        let exit = build_working_cover(&witnesses, &placements, schema.deck_generator(), starved)
            .expect_err("zero budget cannot materialize even one copy");
        assert!(matches!(
            exit,
            CylinderCoverExit::CoverBudgetExceeded { .. }
        ));
        assert_eq!(exit.category(), SliceCategory::OperationalFailure);
    }

    /// A boundary whose angular width *exceeds* one full period brings in
    /// more than one candidate deck index — the cover genuinely needs more
    /// than the trivial `{0}` copy, even though this checkpoint refuses to
    /// *realize* such a face (Checkpoint 6's holonomy gate handles that
    /// refusal; this is only the cover width, computed independently of
    /// whether the face is ultimately accepted).
    #[test]
    fn a_wide_boundary_yields_a_wider_cover() {
        let schema = z_cylinder(2.0, 5.0);
        // A single arc spanning more than one full turn: its bounding box is
        // wider than the period itself, so a translate one period away
        // still overlaps it.
        let sweep = std::f64::consts::TAU + 0.5;
        let p_start = on_cylinder(&schema, 0.0, 0.0);
        let p_end = on_cylinder(&schema, 0.0, sweep);
        let witnesses = vec![circumferential_arc_witness(&schema, p_start, p_end, sweep).unwrap()];
        let placements = vec![0i64];

        let result = build_working_cover(
            &witnesses,
            &placements,
            schema.deck_generator(),
            DeckBudget::FOR_TESTING,
        )
        .expect("a wide single-arc boundary still certifies a finite cover");
        assert!(
            result.materialized_copies.len() > 1,
            "a near-full-period box should pull in a neighboring deck index: {:?}",
            result.materialized_copies
        );
    }
}
