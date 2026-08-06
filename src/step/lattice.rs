//! Deck lattices read from the STEP surface representation.
//!
//! `REFINEMENT_AUDIT.md` §6 found seven live reads of `u_period()`/`v_period()`
//! in the tessellation path, and three qualities of evidence arriving at them
//! as indistinguishable `Option<f64>`: exact, accessor-only, and fabricated.
//! The accessor cannot tell them apart because it has already erased the
//! representation.
//!
//! `look` is the composition layer — the only crate that sees both
//! `truck_stepio`'s concrete `Surface` enum and `truck_meshalgo`'s tessellator
//! — so this is where the representation is still nameable and the evidence can
//! be established. The audit compared three placements and chose this one: a
//! period *witness* and a schema *failure* encode this project's formal system,
//! not general geometry, so putting them in a low-level trait crate would
//! invert the abstraction to solve a dependency obstacle.
//!
//! **Scope.** Lattice only. `REFINEMENT_AUDIT` §6.3 established that the
//! parameter domain is an *output* of lifting rather than an input, so nothing
//! here claims a domain, an extent, or a face context. The two
//! `try_range_tuple()` reads are deliberately untouched; they belong to the
//! next stage.

use truck_meshalgo::tessellation::domain::lattice::{Axis, AxisPeriodStatus, CertifiedLattice};
use truck_stepio::r#in::step_geometry::{ElementarySurface, Surface};

/// The deck lattice of one STEP surface, established from its representation.
///
/// Every periodic axis this returns as a *generator* is exact by construction
/// of the parameterisation. Anything resting on an accessor result comes back
/// `Uncertified`: it is still usable by the legacy path, and it can never be
/// mistaken for a certified generator.
pub fn lattice_of(surface: &Surface) -> CertifiedLattice {
    match surface {
        Surface::ElementarySurface(elementary) => elementary_lattice(elementary),

        // A B-spline or NURBS surface is periodic only if its source
        // representation says so — a periodic knot vector and a wrapped control
        // net. Neither is checked here, and `u_period()` on these types returns
        // `None` regardless, so there is nothing to certify and nothing to
        // preserve.
        Surface::BSplineSurface(_) | Surface::NurbsSurface(_) => CertifiedLattice::NON_PERIODIC,

        // A swept curve's periodicity follows from the swept profile and the
        // sweep, and an offset surface's from its basis. Reading either
        // structurally is not implemented, so the declared values are carried
        // forward as uncertified rather than silently promoted.
        Surface::SweptCurve(_) | Surface::OffsetSurface(_) => unevidenced(surface),
    }
}

fn elementary_lattice(surface: &ElementarySurface) -> CertifiedLattice {
    match surface {
        // A plane has no periodic direction at all.
        ElementarySurface::Plane(_) => CertifiedLattice::NON_PERIODIC,

        // `CylindricalSurface` and `ConicalSurface` are both
        // `Processor<RevolutedCurve<Line<Point3>>, Matrix4>`. The revolution
        // parameterises `v` as a rotation — `subs(u, v)` applies
        // `rotation_matrix(v)` — so `2π` is a property of the map and holds for
        // every generatrix. The generatrix axis is a straight line and carries
        // no period.
        //
        // The `Processor` may be inverted, and an inverted processor evaluates
        // `entity.subs(v, u)`: the axis the caller calls angular is then the
        // entity's generatrix axis. Every axis-indexed fact is therefore
        // restated in the caller's convention, or the exact `2π` would land on
        // the wrong axis — an error the bare accessors could not express.
        ElementarySurface::CylindricalSurface(processor)
        | ElementarySurface::ConicalSurface(processor) => {
            let lattice = CertifiedLattice::revolution(Axis::V, AxisPeriodStatus::NonPeriodic);
            orient(lattice, processor.orientation())
        }

        // A sphere is `Processor<Sphere, Matrix4>` — not a revolved curve, so
        // the revolution witness does not apply to it. Establishing its
        // longitude period structurally means reading `Sphere`'s own
        // parameterisation, which is not done here, and its poles are collapsed
        // strata that this stage does not model at all. Carried as declared.
        ElementarySurface::Sphere(_) => unevidenced_elementary(surface),

        // A torus is `Processor<Torus, Matrix4>` and is doubly periodic. Both
        // periods are real, but recovering the second one by wrapping
        // `curve.period()` would be exactly the error this patch exists to
        // prevent: an accessor result dressed in a stronger name. It stays
        // uncertified until the nested revolved representation is read
        // structurally.
        ElementarySurface::ToroidalSurface(_) => unevidenced_elementary(surface),
    }
}

/// Restate a lattice in the caller's axis convention.
fn orient(lattice: CertifiedLattice, upright: bool) -> CertifiedLattice {
    match upright {
        true => lattice,
        false => lattice.swapped(),
    }
}

fn unevidenced(surface: &Surface) -> CertifiedLattice {
    use truck_meshalgo::prelude::ParametricSurface;
    CertifiedLattice::from_unevidenced_accessors(surface.u_period(), surface.v_period())
}

fn unevidenced_elementary(surface: &ElementarySurface) -> CertifiedLattice {
    use truck_meshalgo::prelude::ParametricSurface;
    CertifiedLattice::from_unevidenced_accessors(surface.u_period(), surface.v_period())
}

// ---------------------------------------------------------------------------
// Structural support-surface schema
// ---------------------------------------------------------------------------

use truck_meshalgo::tessellation::formal::{
    CurveSchema, CurveSchemaFailure, SchemaIdentificationFailure, SupportSurfaceSchema,
    identify_line_segment, identify_plane, identify_polyline,
};
use truck_stepio::r#in::step_geometry::{Conic3D, Curve3D};

/// The authoritative support-surface schema of one STEP surface.
///
/// The companion to [`lattice_of`], and the reason this module is in `look`
/// rather than in the tessellator: this is the last layer that can still name
/// the concrete representation. `lattice_of` answers "what periods does this
/// surface have"; this answers "what *is* this surface", which is the question
/// the formal system's analytic rules are stated against.
///
/// The two are not redundant. Step 1's census found `0 / 24,199` faces
/// resolving to a certified ambient lattice, because
/// `CertifiedLattice::NON_PERIODIC` — what `lattice_of` returns for a plane —
/// is indistinguishable after construction from an accessor that returned
/// nothing, and a torus reaching this module returns exactly that. Only the
/// match arm below, on the entity type itself, separates them.
///
/// **Everything that is not a plane returns `NoStructuralReader`.** That is a
/// statement about this function, not about the surface: cylinders, cones,
/// spheres, tori, swept surfaces, offset surfaces and splines are all P2
/// coverage work, and each needs its own representation-derived witness. Naming
/// them individually rather than with a wildcard is what makes the obstruction
/// histogram tell the corpus which one to add next.
pub fn support_schema_of(surface: &Surface) -> SupportSurfaceSchema {
    let unread = |representation| {
        SupportSurfaceSchema::not_structurally_identified(
            SchemaIdentificationFailure::NoStructuralReader { representation },
        )
    };
    match surface {
        Surface::ElementarySurface(elementary) => match elementary {
            // The one structural reader that exists. `identify_plane` still
            // refuses a basis whose axes it cannot separate — a plane with a
            // degenerate basis has a genuinely periodic parameterisation, so
            // the entity type alone does not establish aperiodicity.
            ElementarySurface::Plane(plane) => identify_plane(plane),

            // `Processor<RevolutedCurve<Line<Point3>>, Matrix4>`. The expected
            // route is the revolved-surface schema plus the 2π angular
            // generator plus straight-generatrix aperiodicity, giving formal
            // rank 1. Not implemented; see `lattice_of` above, which already
            // certifies the same 2π on the legacy side.
            ElementarySurface::CylindricalSurface(_) => unread("cylindrical_surface"),
            ElementarySurface::ConicalSurface(_) => unread("conical_surface"),
            ElementarySurface::Sphere(_) => unread("spherical_surface"),
            ElementarySurface::ToroidalSurface(_) => unread("toroidal_surface"),
        },
        Surface::SweptCurve(_) => unread("swept_surface"),
        Surface::BSplineSurface(_) => unread("b_spline_surface"),
        Surface::NurbsSurface(_) => unread("rational_b_spline_surface"),
        Surface::OffsetSurface(_) => unread("offset_surface"),
    }
}

/// The authoritative schema of one STEP edge curve.
///
/// The curve counterpart of [`support_schema_of`], and the input to Step 3's
/// certificate-route decision. Only the two families whose planar projection is
/// *exact* are read: a line segment and a polyline each map, segment for
/// segment, to the 2D chain the plane's affine inverse produces, so the
/// whole-interval curve-on-surface obligation is discharged by the
/// representation rather than by a numerical bound.
///
/// A `Line` in `truck_stepio` is `Line(a, b)` on the parameter range `0..=1` —
/// the representation *is* the trimmed segment — so there is no separate
/// trimming to reconcile.
///
/// Everything else is refused by name. Circles and ellipses arrive as
/// `Conic(Ellipse(..))` and need an analytic arc bound; splines need a
/// whole-interval flatness certificate; a `PCurve` needs the source
/// representation contract of Step 3's route A, which `truck-stepio` does not
/// carry today. Each is a separate P2 expansion and the corpus ranks them.
pub fn curve_schema_of(curve: &Curve3D) -> CurveSchema {
    let unread = |representation| {
        CurveSchema::not_structurally_identified(CurveSchemaFailure::NoStructuralReader {
            representation,
        })
    };
    match curve {
        Curve3D::Line(line) => identify_line_segment(line),
        Curve3D::Polyline(polyline) => identify_polyline(&polyline.0),
        // The planar rank-0 path refuses every conic exactly as before; the
        // source family only makes the refusal say which entity it was.
        // Admitting a circular arc here is Milestone B, not this change.
        Curve3D::Conic(Conic3D::Circle(_)) => unread("circle"),
        Curve3D::Conic(Conic3D::Ellipse(_)) => unread("ellipse"),
        Curve3D::Conic(Conic3D::Hyperbola(_)) => unread("hyperbola"),
        Curve3D::Conic(Conic3D::Parabola(_)) => unread("parabola"),
        Curve3D::BSplineCurve(_) => unread("b_spline_curve"),
        Curve3D::NurbsCurve(_) => unread("rational_b_spline_curve"),
        Curve3D::PCurve(_) => unread("pcurve"),
    }
}

// ---------------------------------------------------------------------------
// Rank-1 cylinder curve classification (Task 2)
// ---------------------------------------------------------------------------

use crate::step::circular_arc::{decode_source_circle, decode_transformed_circle};
use truck_meshalgo::tessellation::formal::{CompleteCirclePlacement, SourceCurveFamily};

/// The cylinder-only companion to [`curve_schema_of`]'s Step-2 admission
/// gate.
///
/// `regular_traversal`/`build_cylinder_face`'s traversal gate only asks
/// whether *some* structural reader succeeded
/// (`CurveSchema::is_structurally_identified`) — it never reads the schema's
/// content for the cylinder route, because
/// [`truck_meshalgo::tessellation::formal::develop_traversal_from_source`]
/// re-derives the curve family and signed sweep independently from
/// [`cylinder_curve_family_of`]. So this function only needs to say "line",
/// "certified circular arc", or "not admitted" — it does not need to carry
/// the arc's numbers, unlike `cylinder_curve_family_of` below.
///
/// Kept separate from [`curve_schema_of`] rather than changing that
/// function's `Conic` arm: the planar rank-0 path must keep refusing a
/// circle/ellipse exactly as it does today (a certified circular arc is not
/// yet a planar rank-0 witness — that is Milestone B), and folding this
/// admission into the one function shared by both routes would let a
/// planar-only caller silently admit a curve family its own downstream
/// stage cannot use.
pub fn cylinder_curve_schema_of(curve: &Curve3D) -> CurveSchema {
    match curve {
        Curve3D::Line(line) => identify_line_segment(line),
        Curve3D::Polyline(polyline) => identify_polyline(&polyline.0),
        // A source `circle` is read on its own family's authority; a source
        // `ellipse` still has to prove circularity exactly, and so never
        // becomes an arc however nearly circular it is.
        Curve3D::Conic(Conic3D::Circle(circle)) => match decode_source_circle(circle) {
            Ok(_) => CurveSchema::CircularArc,
            Err(cause) => CurveSchema::not_structurally_identified(
                CurveSchemaFailure::NoStructuralReader {
                    representation: cause.tag(),
                },
            ),
        },
        Curve3D::Conic(Conic3D::Ellipse(ellipse)) => match decode_transformed_circle(ellipse) {
            Ok(_) => CurveSchema::CircularArc,
            Err(cause) => CurveSchema::not_structurally_identified(
                CurveSchemaFailure::NoStructuralReader {
                    representation: cause.tag(),
                },
            ),
        },
        _ => curve_schema_of(curve),
    }
}

/// Classify one real source edge's curve into the [`SourceCurveFamily`]
/// [`truck_meshalgo::tessellation::formal::identify_source_curve_witness`]
/// needs — the production route (Task 2's `classify_cylinder_edge_use`).
///
/// `None` means this curve is not one of the two admitted rank-1 families
/// (axial line, circumferential arc); the caller reports `Unsupported` for
/// that edge use rather than guessing. A non-circular ellipse (a genuine
/// STEP `ellipse`, not a `circle`) is `None` here for the same reason it is
/// `CurveSchema::NotStructurallyIdentified` above: this session admits only
/// exact circles, never approximates a general ellipse as one.
///
/// The returned `parameter_interval` is the source curve's own directed
/// interval — [`crate::step::circular_arc::CertifiedCircularArc::source_interval`]
/// — in the curve's own direction. `develop_traversal_from_source` applies
/// the traversal's own composed sense (`occurrence.forward`) on top of this,
/// exactly once, so this function must never pre-apply it.
pub fn cylinder_curve_family_of(curve: &Curve3D) -> Option<SourceCurveFamily> {
    match curve {
        Curve3D::Line(_) => Some(SourceCurveFamily::Line),
        // Both conic families reach the same body; they differ only in which
        // circularity obligation the decoder discharges. Splitting the *arms*
        // rather than the bodies keeps the interval, sweep-axis fold and
        // complete-circle rules below in one place, so a source `circle` and a
        // source `ellipse` that both certify are treated identically from here
        // on -- which is the point: the family decides admission, never
        // semantics.
        Curve3D::Conic(conic @ (Conic3D::Circle(_) | Conic3D::Ellipse(_))) => {
            let (ellipse, decoded) = match conic {
                Conic3D::Circle(circle) => (circle, decode_source_circle(circle)),
                Conic3D::Ellipse(ellipse) => (ellipse, decode_transformed_circle(ellipse)),
                _ => unreachable!("the arm pattern admits only the two conic families"),
            };
            let arc = decoded.ok()?;
            let (t0, t1) = arc.source_interval();
            if t0 != t1 {
                return Some(SourceCurveFamily::CircularArc {
                    parameter_interval: (t0, t1),
                });
            }
            // A collapsed interval is not a declared zero sweep. The importer
            // recovers an `edge_curve`'s trim by solving each of its two
            // vertex points onto the curve
            // (`truck_stepio::in::EdgeCurveHolder::sub_parse_curve3d`), and a
            // full circle's edge uses *one* vertex for both ends — so the two
            // solves return the identical parameter and the extent the source
            // did declare, the circle's whole period, is gone. What survives
            // is the circle's own placement, which is exactly what
            // `SourceCurveFamily::CompleteCircle` carries. The occurrence's
            // own source topology still has to close before that period is
            // accepted as its extent; `identify_source_curve_witness` refuses
            // it otherwise, so a genuinely zero-length circular edge between
            // two *distinct* coincident vertices is never read as a full turn.
            //
            // `sweep_axis` folds the converted curve's parameter sense in
            // exactly once, the same fold `source_interval` already carries
            // for a non-degenerate arc and the one place the degenerate
            // interval loses it: `Processor::orientation() == false` means
            // walking the converted domain forward reads the entity's angle
            // backward, so the axis about which *this* curve's parameter
            // advances right-handedly is the entity normal negated.
            let sweep_axis = match ellipse.orientation() {
                true => arc.normal(),
                false => -arc.normal(),
            };
            Some(SourceCurveFamily::CompleteCircle {
                placement: CompleteCirclePlacement {
                    center: arc.center(),
                    sweep_axis,
                    radius: arc.radius().get(),
                    curve_orientation: ellipse.orientation(),
                },
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod cylinder_curve_tests {
    use super::*;
    use truck_meshalgo::prelude::Point3;
    use truck_stepio::r#in::step_geometry::{Line as StepLine, Processor, TrimmedCurve, UnitCircle};

    /// A conic carrying *no* source family, so it must prove circularity
    /// exactly. Deliberately still `Conic3D::Ellipse`: these tests are what
    /// keeps the unauthorized path honest.
    fn circle_curve(range: (f64, f64)) -> Curve3D {
        Curve3D::Conic(Conic3D::Ellipse(Processor::new(TrimmedCurve::new(
            UnitCircle::new(),
            range,
        ))))
    }

    /// A conic the source declared to be a `circle`, under the importer's own
    /// derived placement — orthonormal only to rounding, exactly as every real
    /// `CIRCLE` in the corpus arrives.
    fn source_circle_curve(range: (f64, f64)) -> Curve3D {
        use truck_meshalgo::prelude::{InnerSpace, Matrix4, Vector3};
        let z = Vector3::new(0.3, 0.5, 0.81).normalize();
        let reference = Vector3::new(0.77, -0.13, 0.62);
        let x = (reference - reference.dot(z) * z).normalize();
        let y = z.cross(x);
        let transform = Matrix4::from_cols(
            x.extend(0.0),
            y.extend(0.0),
            z.extend(0.0),
            Vector3::new(0.0, 0.0, 0.0).extend(1.0),
        );
        Curve3D::Conic(Conic3D::Circle(Processor::with_transform(
            TrimmedCurve::new(UnitCircle::new(), range),
            transform,
        )))
    }

    #[test]
    fn a_source_circle_with_finite_precision_orientation_is_admitted() {
        let curve = source_circle_curve((0.2, 1.4));
        // The same representation with its family erased is refused, which is
        // what the corpus was hitting 20,388 times.
        let erased = match &curve {
            Curve3D::Conic(Conic3D::Circle(circle)) => {
                Curve3D::Conic(Conic3D::Ellipse(*circle))
            }
            _ => unreachable!(),
        };
        assert!(!cylinder_curve_schema_of(&erased).is_structurally_identified());
        assert!(cylinder_curve_family_of(&erased).is_none());

        assert!(cylinder_curve_schema_of(&curve).is_structurally_identified());
        let family = cylinder_curve_family_of(&curve).expect("a source circle classifies");
        assert!(matches!(
            family,
            SourceCurveFamily::CircularArc {
                parameter_interval: (t0, t1)
            } if (t0 - 0.2).abs() < 1e-12 && (t1 - 1.4).abs() < 1e-12
        ));
    }

    #[test]
    fn a_nearly_circular_source_ellipse_is_still_not_a_circle() {
        // One ULP of anisotropy, declared as an `ellipse`. No source family
        // admits it and the exact predicate refuses it, so it stays an
        // ellipse — the property the whole repair had to preserve.
        let semi = 1.0_f64;
        let other = f64::from_bits(semi.to_bits() + 1);
        let transform =
            truck_meshalgo::prelude::Matrix4::from_nonuniform_scale(semi, other, semi);
        let curve = Curve3D::Conic(Conic3D::Ellipse(Processor::with_transform(
            TrimmedCurve::new(UnitCircle::new(), (0.0, 1.0)),
            transform,
        )));
        assert!(!cylinder_curve_schema_of(&curve).is_structurally_identified());
        assert!(cylinder_curve_family_of(&curve).is_none());
    }

    #[test]
    fn a_nonuniformly_transformed_source_circle_is_refused() {
        let base = match source_circle_curve((0.0, 1.0)) {
            Curve3D::Conic(Conic3D::Circle(circle)) => circle,
            _ => unreachable!(),
        };
        let squashed = truck_meshalgo::prelude::Matrix4::from_nonuniform_scale(2.0, 1.0, 1.0)
            * *base.transform();
        let curve = Curve3D::Conic(Conic3D::Circle(Processor::with_transform(
            TrimmedCurve::new(UnitCircle::new(), (0.0, 1.0)),
            squashed,
        )));
        assert!(!cylinder_curve_schema_of(&curve).is_structurally_identified());
        assert!(cylinder_curve_family_of(&curve).is_none());
    }

    #[test]
    fn a_source_circle_with_a_collapsed_interval_is_still_a_complete_circle() {
        // The complete-circle rule is unchanged by the repair: a collapsed
        // interval means the importer solved one vertex twice, and what
        // survives is the circle's placement. The source topology still has to
        // close before that period is accepted as the extent — that check
        // lives in `identify_source_curve_witness` and is untouched here.
        let family = cylinder_curve_family_of(&source_circle_curve((0.7, 0.7)))
            .expect("a collapsed source circle classifies");
        let SourceCurveFamily::CompleteCircle { placement } = family else {
            panic!("expected a complete circle, got {family:?}");
        };
        assert!((placement.radius - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_source_circle_folds_the_curve_orientation_exactly_once() {
        use truck_meshalgo::prelude::Invertible;
        let mut reversed = match source_circle_curve((0.2, 1.4)) {
            Curve3D::Conic(Conic3D::Circle(circle)) => circle,
            _ => unreachable!(),
        };
        reversed.invert();
        let family = cylinder_curve_family_of(&Curve3D::Conic(Conic3D::Circle(reversed)))
            .expect("a reversed source circle classifies");
        assert!(matches!(
            family,
            SourceCurveFamily::CircularArc {
                parameter_interval: (t0, t1)
            } if (t0 - 1.4).abs() < 1e-12 && (t1 - 0.2).abs() < 1e-12
        ));
    }

    #[test]
    fn the_planar_curve_schema_refuses_both_conic_families() {
        // The planar rank-0 path is untouched by the repair. It refuses a
        // source `circle` exactly as it refuses an unauthorized one; only the
        // reported representation name distinguishes them.
        for curve in [source_circle_curve((0.0, 1.0)), circle_curve((0.0, 1.0))] {
            assert!(!curve_schema_of(&curve).is_structurally_identified());
            assert!(cylinder_curve_schema_of(&curve).is_structurally_identified());
        }
    }

    #[test]
    fn a_real_line_classifies_as_the_line_family() {
        let line = Curve3D::Line(StepLine(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)));
        assert!(matches!(
            cylinder_curve_family_of(&line),
            Some(SourceCurveFamily::Line)
        ));
        assert!(cylinder_curve_schema_of(&line).is_structurally_identified());
    }

    #[test]
    fn a_real_circular_arc_classifies_with_its_own_source_interval() {
        let curve = circle_curve((0.2, 1.4));
        let family = cylinder_curve_family_of(&curve).expect("a circle classifies");
        assert!(matches!(
            family,
            SourceCurveFamily::CircularArc {
                parameter_interval: (t0, t1)
            } if (t0 - 0.2).abs() < 1e-12 && (t1 - 1.4).abs() < 1e-12
        ));
        assert!(cylinder_curve_schema_of(&curve).is_structurally_identified());
        assert_eq!(cylinder_curve_schema_of(&curve).polygonal(), None);
    }

    #[test]
    fn a_non_circular_ellipse_is_not_admitted() {
        let transform = truck_meshalgo::prelude::Matrix4::from_nonuniform_scale(2.0, 1.0, 1.0);
        let curve = Curve3D::Conic(Conic3D::Ellipse(Processor::with_transform(
            TrimmedCurve::new(UnitCircle::new(), (0.0, 1.0)),
            transform,
        )));
        assert!(cylinder_curve_family_of(&curve).is_none());
        assert!(!cylinder_curve_schema_of(&curve).is_structurally_identified());
    }

    #[test]
    fn a_spline_stays_unadmitted_exactly_as_the_planar_path_refuses_it() {
        let curve = Curve3D::Conic(Conic3D::Hyperbola(Processor::new(TrimmedCurve::new(
            truck_stepio::r#in::step_geometry::UnitHyperbola::new(),
            (0.0, 1.0),
        ))));
        assert!(cylinder_curve_family_of(&curve).is_none());
        assert_eq!(cylinder_curve_schema_of(&curve).tag(), curve_schema_of(&curve).tag());
    }

    /// The planar path's own admission is unchanged: a circle is still
    /// refused by `curve_schema_of` (Milestone B territory, not this
    /// session), even though `cylinder_curve_schema_of` now admits it.
    #[test]
    fn the_planar_curve_schema_still_refuses_a_circle() {
        let curve = circle_curve((0.0, 1.0));
        assert!(!curve_schema_of(&curve).is_structurally_identified());
        assert!(cylinder_curve_schema_of(&curve).is_structurally_identified());
    }
}

#[cfg(test)]
mod schema_tests {
    use super::*;
    use truck_meshalgo::prelude::{EuclideanSpace, Point3, Vector3};
    use truck_stepio::r#in::step_geometry::{Plane, Processor, Torus};

    fn a_plane() -> Surface {
        Surface::ElementarySurface(ElementarySurface::Plane(Plane::new(
            Point3::new(1.0, 2.0, 3.0),
            Point3::new(2.0, 2.0, 3.0),
            Point3::new(1.0, 3.0, 3.0),
        )))
    }

    #[test]
    fn a_step_plane_is_structurally_identified() {
        let schema = support_schema_of(&a_plane());
        let plane = schema.plane().expect("a STEP plane is a plane");
        assert_eq!(plane.origin(), Point3::new(1.0, 2.0, 3.0));
        assert_eq!(plane.u_axis(), Vector3::new(1.0, 0.0, 0.0));
        assert_eq!(plane.v_axis(), Vector3::new(0.0, 1.0, 0.0));
    }

    /// The corpus case: a torus's legacy lattice is `NonPeriodic` on both axes
    /// because its accessors return `None`, and it is doubly periodic all the
    /// same. The schema reader is what keeps the two apart.
    #[test]
    fn a_torus_reports_no_structural_reader_though_its_lattice_looks_aperiodic() {
        let torus = Surface::ElementarySurface(ElementarySurface::ToroidalSurface(Processor::new(
            Torus::new(Point3::origin(), 2.0, 1.0),
        )));
        assert_eq!(
            support_schema_of(&torus),
            SupportSurfaceSchema::not_structurally_identified(
                SchemaIdentificationFailure::NoStructuralReader {
                    representation: "toroidal_surface"
                }
            )
        );
        // And the legacy lattice certifies nothing for it either way: this
        // torus's accessors do return 2π, so it lands in `Uncertified` rather
        // than `NonPeriodic` — but `certified_rank()` is 0 for both it and a
        // plane, which is exactly the collapse the schema reader undoes.
        assert_eq!(lattice_of(&torus).certified_rank(), 0);
        assert_eq!(lattice_of(&a_plane()).certified_rank(), 0);
        assert_eq!(lattice_of(&a_plane()), CertifiedLattice::NON_PERIODIC);
    }
}
