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
    SchemaIdentificationFailure, SupportSurfaceSchema, identify_plane,
};

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
