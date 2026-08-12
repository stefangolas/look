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
/// The source-declared closure of a spline surface's two parameter axes.
///
/// This is explicit provenance, not inference: STEP declares
/// `B_SPLINE_SURFACE_WITH_KNOTS` (and the NURBS and uniform/quasi/bezier forms)
/// with a `u_closed`/`v_closed` pair, and only an explicit `.T.` counts. The
/// closure is *not* reconstructed from a periodic knot vector, a wrapped
/// control net, coincident endpoints, or numerical seam agreement — the source
/// declaration is the authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplineAxisClosure {
    /// The source-declared closure of the `u` axis.
    pub u_closed: bool,
    /// The source-declared closure of the `v` axis.
    pub v_closed: bool,
}

impl SplineAxisClosure {
    /// Neither axis declared closed.
    pub const OPEN: Self = Self {
        u_closed: false,
        v_closed: false,
    };
}

/// Read the source-declared spline-axis closure of every spline surface entity
/// a STEP table holds, keyed by surface entity id.
///
/// This is the provenance seam: the composition layer still holds the raw STEP
/// table (`b_spline_surface_with_knots`, `uniform_surface`,
/// `quasi_uniform_surface`, `bezier_surface`, and the NURBS wrapper), so the
/// `u_closed`/`v_closed` declaration is nameable here and nowhere downstream.
/// The converted `Surface` value the tessellator sees has erased it.
///
/// A `FaceProvenance.surface_id` (truck-topology) names the same entity id, so
/// the composition layer can attach the closure to each face's surface by that
/// id before the tessellator's lattice callback runs.
pub fn spline_closure_map(
    table: &truck_stepio::r#in::Table,
) -> std::collections::HashMap<u64, SplineAxisClosure> {
    use truck_stepio::r#in::ruststep::tables::EntityTable;

    let mut closures = std::collections::HashMap::new();
    let mut read = |id: u64, u_closed: bool, v_closed: bool| {
        closures.insert(id, SplineAxisClosure { u_closed, v_closed });
    };
    // The holder maps are keyed by entity id. Resolve each to its owned entity
    // to read the declaration. `get_owned` fails only on a dangling reference,
    // which would also have failed conversion.
    for &id in table.b_spline_surface_with_knots.keys() {
        if let Ok(owned) =
            EntityTable::<truck_stepio::r#in::BSplineSurfaceWithKnotsHolder>::get_owned(table, id)
        {
            read(id, owned.u_closed(), owned.v_closed());
        }
    }
    for &id in table.uniform_surface.keys() {
        if let Ok(owned) =
            EntityTable::<truck_stepio::r#in::UniformSurfaceHolder>::get_owned(table, id)
        {
            read(id, owned.u_closed(), owned.v_closed());
        }
    }
    for &id in table.quasi_uniform_surface.keys() {
        if let Ok(owned) =
            EntityTable::<truck_stepio::r#in::QuasiUniformSurfaceHolder>::get_owned(table, id)
        {
            read(id, owned.u_closed(), owned.v_closed());
        }
    }
    for &id in table.bezier_surface.keys() {
        if let Ok(owned) =
            EntityTable::<truck_stepio::r#in::BezierSurfaceHolder>::get_owned(table, id)
        {
            read(id, owned.u_closed(), owned.v_closed());
        }
    }
    for &id in table.rational_b_spline_surface.keys() {
        if let Ok(owned) =
            EntityTable::<truck_stepio::r#in::RationalBSplineSurfaceHolder>::get_owned(table, id)
        {
            read(id, owned.u_closed(), owned.v_closed());
        }
    }
    closures
}

/// The deck lattice of one STEP surface, established from its representation.
///
/// Every periodic axis this returns as a *generator* is exact by construction
/// of the parameterisation. Anything resting on an accessor result comes back
/// `Uncertified`: it is still usable by the legacy path, and it can never be
/// mistaken for a certified generator.
///
/// The composition layer that can name the concrete STEP surface should call
/// [`lattice_of_with_closure`], which additionally certifies a
/// source-declared-closed spline axis whose converted evaluator satisfies the
/// seam identification. This bare form carries no source provenance and is the
/// conservative fallback used by callers that only have the converted geometry.
pub fn lattice_of(surface: &Surface) -> CertifiedLattice {
    lattice_of_with_closure(surface, None)
}

/// As [`lattice_of`], with the source-declared spline-axis closure supplied by
/// the composition layer.
///
/// The source closure is the authority; the evaluator seam compatibility check
/// can only reject an incompatible conversion, never establish closure. See the
/// stage-B theorem in the periodic-cover handoff.
pub fn lattice_of_with_closure(
    surface: &Surface,
    closure: Option<SplineAxisClosure>,
) -> CertifiedLattice {
    match surface {
        Surface::ElementarySurface(elementary) => elementary_lattice(elementary),

        // A B-spline or NURBS surface is periodic only if its source
        // representation says so. The source closure, when the composition
        // layer supplies it, certifies a generator on exactly the declared
        // axis, gated on the converted evaluator's seam compatibility.
        // Without provenance the axis stays `NonPeriodic` — nothing is
        // inferred from the knot vector or the control net.
        Surface::BSplineSurface(_) | Surface::NurbsSurface(_) => spline_lattice(surface, closure),

        // A swept curve's periodicity follows from the swept profile and the
        // sweep, and an offset surface's from its basis. Reading either
        // structurally is not implemented, so the declared values are carried
        // forward as uncertified rather than silently promoted.
        Surface::SweptCurve(_) | Surface::OffsetSurface(_) => unevidenced(surface),
    }
}

/// Certify the source-declared closure of a converted spline surface.
///
/// The theorem, stated for one axis `A` with active interval `[a, b]`:
///
/// > the STEP source declares `A` closed, and the converted spline evaluator
/// > satisfies the seam identification `S(·, a) == S(·, b)` (position and
/// > first derivative) over the active interval; therefore `P = b - a` is a
/// > valid topological lattice generator on `A`.
///
/// The source declaration is read from the composition layer's provenance, not
/// inferred. The seam check is a *compatibility* test: it certifies nothing by
/// itself and only declines to certify a conversion whose evaluator does not
/// realise the source's closure. A declared axis whose seam check fails stays
/// `NonPeriodic` rather than being silently promoted.
fn spline_lattice(surface: &Surface, closure: Option<SplineAxisClosure>) -> CertifiedLattice {
    use truck_meshalgo::prelude::BoundedSurface;

    let Some(closure) = closure else {
        return CertifiedLattice::NON_PERIODIC;
    };

    // The active interval of a converted spline is its basis-valid interior
    // rectangle (`BoundedSurface::evaluation_range`), which can be strictly
    // narrower than the declared knot range when STEP's end knots are not
    // clamped. That is the domain the generic evaluator genuinely supports.
    let ((u0, u1), (v0, v1)) = match surface {
        Surface::BSplineSurface(spline) => BoundedSurface::evaluation_range(spline),
        Surface::NurbsSurface(spline) => BoundedSurface::evaluation_range(spline),
        _ => return CertifiedLattice::NON_PERIODIC,
    };
    let spline_axis = |axis: Axis, declared: bool, (a, b): (f64, f64)| {
        match declared {
        // Only an explicit source `.T.` reaches this arm. Without it the axis
        // stays non-periodic even if the geometry happens to close numerically.
        false => AxisPeriodStatus::NonPeriodic,
        true => match spline_seam_compatible(surface, axis, (a, b)) {
            Some(period) => AxisPeriodStatus::Exact {
                period,
                witness: truck_meshalgo::tessellation::domain::lattice::PeriodWitness::
                    SourceDeclaredClosedSplineAxis,
            },
            // The source declared closure but the converted evaluator does not
            // identify the seam — an incompatible conversion. Do not certify.
            None => AxisPeriodStatus::NonPeriodic,
        },
    }
    };
    let u = spline_axis(Axis::U, closure.u_closed, (u0, u1));
    let v = spline_axis(Axis::V, closure.v_closed, (v0, v1));
    CertifiedLattice { u, v }
}

/// How many samples per axis the seam compatibility check evaluates.
const SEAM_SAMPLES: usize = 8;
/// A relative tolerance for the seam identification, as a fraction of the
/// larger active-axis span.
///
/// The measured seam residuals for the corpus's source-closed splines are at
/// machine precision (`S` ~ `7e-14`, `Sv` ~ `9e-13` on a 300-unit model), and
/// this tolerance sits five orders above that while remaining five orders
/// below the render chord tolerance — a genuinely incompatible conversion,
/// whose seam gap is macroscopic, is rejected decisively.
const SEAM_RELATIVE_TOLERANCE: f64 = 1.0e-8;

/// Check the seam identification of a converted spline on one axis.
///
/// Evaluates `S` and its first derivative at `8` samples along the other axis,
/// comparing the two seam endpoints. Returns `Some(period)` with
/// `period = b - a` when every sample satisfies the position and derivative
/// identification; `None` when the conversion does not realise the source's
/// declared closure. The check is a *rejection* gate, never the source of the
/// closure.
fn spline_seam_compatible(surface: &Surface, axis: Axis, (a, b): (f64, f64)) -> Option<f64> {
    use truck_meshalgo::prelude::{BoundedSurface, InnerSpace, ParametricSurface};

    let (span, other) = match axis {
        Axis::U => {
            let ((_, _), (v0, v1)) = match surface {
                Surface::BSplineSurface(spline) => BoundedSurface::evaluation_range(spline),
                Surface::NurbsSurface(spline) => BoundedSurface::evaluation_range(spline),
                _ => return None,
            };
            (b - a, (v0, v1))
        }
        Axis::V => {
            let ((u0, u1), (_, _)) = match surface {
                Surface::BSplineSurface(spline) => BoundedSurface::evaluation_range(spline),
                Surface::NurbsSurface(spline) => BoundedSurface::evaluation_range(spline),
                _ => return None,
            };
            (b - a, (u0, u1))
        }
    };
    let tolerance = span.abs().max(1.0) * SEAM_RELATIVE_TOLERANCE;

    let (o0, o1) = other;
    for i in 0..=SEAM_SAMPLES {
        let t = o0 + (o1 - o0) * (i as f64 / SEAM_SAMPLES as f64);
        // Seam on `u`: `S(u0, v) == S(u1, v)`; seam on `v`:
        // `S(u, v0) == S(u, v1)`.
        let (p0, p1) = match axis {
            Axis::U => (surface.subs(a, t), surface.subs(b, t)),
            Axis::V => (surface.subs(t, a), surface.subs(t, b)),
        };
        if (p0 - p1).magnitude() > tolerance {
            return None;
        }
        let (d0, d1) = match axis {
            Axis::U => (surface.uder(a, t), surface.uder(b, t)),
            Axis::V => (surface.vder(t, a), surface.vder(t, b)),
        };
        if (d0 - d1).magnitude() > tolerance {
            return None;
        }
    }
    Some(b - a)
}

/// One spline axis' native evaluator interval, when certified.
pub type SplineAxisRange = Option<(f64, f64)>;

/// The cover→native evaluator intervals of a converted spline, one per axis,
/// certified exactly when the axis carries the `SourceDeclaredClosedSplineAxis`
/// witness.
///
/// The quotient adapter needs, for each source-certified closed spline axis,
/// the native *evaluation interval* `[a, b]` — not just the period `P = b - a`,
/// because `a` need not be zero. This derives that once from the same lattice
/// and the same `evaluation_range` the certification theorem used, so the
/// evaluator map never re-derives periodicity numerically. Analytic surfaces
/// and non-periodic axes return `None` on every axis.
pub fn spline_quotient_axes(
    surface: &Surface,
    closure: Option<SplineAxisClosure>,
) -> (SplineAxisRange, SplineAxisRange) {
    use truck_meshalgo::prelude::BoundedSurface;
    use truck_meshalgo::tessellation::domain::lattice::{AxisPeriodStatus, PeriodWitness};

    let lattice = lattice_of_with_closure(surface, closure);
    let ((u0, u1), (v0, v1)) = match surface {
        Surface::BSplineSurface(spline) => BoundedSurface::evaluation_range(spline),
        Surface::NurbsSurface(spline) => BoundedSurface::evaluation_range(spline),
        _ => ((0.0, 0.0), (0.0, 0.0)),
    };
    let axis = |status: AxisPeriodStatus, (a, b): (f64, f64)| match status {
        AxisPeriodStatus::Exact {
            witness: PeriodWitness::SourceDeclaredClosedSplineAxis,
            ..
        } => Some((a, b)),
        _ => None,
    };
    (axis(lattice.u, (u0, u1)), axis(lattice.v, (v0, v1)))
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
        // the revolution witness does not apply to it. Its azimuth is a
        // rotation about the polar axis by construction of `Sphere`'s
        // parameterisation (`subs(u, v)` enters the azimuth only through
        // `(cos v, sin v)`), so `2π` is read from the primitive, on the axis
        // that parameterises it, and the polar (latitude) axis has no period.
        // The `Processor` may be inverted, and `orient` restates every
        // axis-indexed fact in the caller's convention.
        ElementarySurface::Sphere(processor) => {
            let lattice = CertifiedLattice::sphere_azimuth(Axis::U);
            orient(lattice, processor.orientation())
        }

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
/// A circle or ellipse arrives as `Conic(..)` and is read through the same
/// certified decoders the rank-1 route uses ([`cylinder_curve_schema_of`]), so
/// there is exactly one circle reader in the workspace and a source `ellipse`
/// still has to prove circularity exactly. Reading it here **grants nothing to
/// the polygonal path**: `CurveSchema::circular_arc()` and
/// `CurveSchema::polygonal()` are disjoint accessors, and Step 3's polygonal
/// route (`certified_planar_curves`) still exits
/// `UnsupportedCurveRepresentation` on an arc. What changes is only that the
/// refusal now happens at Step 3, where the face's curve families are known,
/// instead of at Step 2's bare identification gate — which is what lets the
/// developed-curve track see the arc at all.
///
/// Everything else is refused by name: splines need a whole-interval flatness
/// certificate; a `PCurve` needs the source representation contract of Step 3's
/// route A, which `truck-stepio` does not carry today. Each is a separate P2
/// expansion and the corpus ranks them.
pub fn curve_schema_of(curve: &Curve3D) -> CurveSchema {
    let unread = |representation| {
        CurveSchema::not_structurally_identified(CurveSchemaFailure::NoStructuralReader {
            representation,
        })
    };
    match curve {
        Curve3D::Line(line) => identify_line_segment(line),
        Curve3D::Polyline(polyline) => identify_polyline(&polyline.0),
        Curve3D::Conic(Conic3D::Circle(circle)) => arc_schema(decode_source_circle(circle)),
        Curve3D::Conic(Conic3D::Ellipse(ellipse)) => arc_schema(decode_transformed_circle(ellipse)),
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

use crate::step::circular_arc::{
    CertifiedCircularArc, CircularArcAdapterFailure, decode_source_circle,
    decode_transformed_circle,
};
use truck_meshalgo::tessellation::formal::{
    CircularArcPlacement3, CompleteCirclePlacement, SourceCurveFamily,
};

/// Turn a certified-circle decode into the schema, preserving the decoder's
/// own refusal tag.
///
/// The single place a `CertifiedCircularArc` becomes a
/// [`CircularArcPlacement3`]. A failed decode is `NoStructuralReader` carrying
/// the decoder's cause, so "this ellipse is not exactly circular" and "no
/// reader exists for this representation" stay distinguishable in the corpus
/// histogram.
fn arc_schema(decoded: Result<CertifiedCircularArc, CircularArcAdapterFailure>) -> CurveSchema {
    match decoded {
        Ok(arc) => CurveSchema::CircularArc(CircularArcPlacement3 {
            center: arc.center(),
            cos_basis: arc.basis_cos(),
            sin_basis: arc.basis_sin(),
            parameter_interval: arc.source_interval(),
        }),
        Err(cause) => {
            CurveSchema::not_structurally_identified(CurveSchemaFailure::NoStructuralReader {
                representation: cause.tag(),
            })
        }
    }
}

/// The cylinder route's Step-2 admission gate.
///
/// `regular_traversal`/`build_cylinder_face`'s traversal gate only asks
/// whether *some* structural reader succeeded
/// (`CurveSchema::is_structurally_identified`) — it never reads the schema's
/// content for the cylinder route, because
/// [`truck_meshalgo::tessellation::formal::develop_traversal_from_source`]
/// re-derives the curve family and signed sweep independently from
/// [`cylinder_curve_family_of`].
///
/// This used to differ from [`curve_schema_of`]: the planar rank-0 path
/// refused a circle at Step 2 and the cylinder path admitted one, so folding
/// them together would have let a planar-only caller past a gate its own
/// downstream stage could not honour. That is no longer the case —
/// `curve_schema_of` reads the same certified decoders, and the planar path's
/// polygonal Step 3 refuses an arc on the schema's *content*
/// (`CurveSchema::polygonal()` is `None`) rather than on its identification.
/// The two seams are therefore one function now. The name is kept because the
/// tessellation entry point threads it as a distinct adapter, and collapsing
/// that signature is a separate change.
pub fn cylinder_curve_schema_of(curve: &Curve3D) -> CurveSchema {
    curve_schema_of(curve)
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
    use truck_meshalgo::prelude::{InnerSpace, Point3};
    use truck_stepio::r#in::step_geometry::{
        Line as StepLine, Processor, TrimmedCurve, UnitCircle,
    };

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
            Curve3D::Conic(Conic3D::Circle(circle)) => Curve3D::Conic(Conic3D::Ellipse(*circle)),
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
        let transform = truck_meshalgo::prelude::Matrix4::from_nonuniform_scale(semi, other, semi);
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
    fn the_planar_curve_schema_identifies_a_conic_without_making_it_polygonal() {
        // Milestone B moved the refusal, not the admission. Both conic
        // families are now *identified* — so Step 2's bare identification gate
        // admits them and the developed-curve track can see the arc — while
        // `polygonal()` stays `None`, which is what Step 3's polygonal route
        // reads. A face bounded by arcs therefore still exits
        // `UnsupportedCurveRepresentation`; it just does so at the stage that
        // knows why.
        for curve in [source_circle_curve((0.0, 1.0)), circle_curve((0.0, 1.0))] {
            let schema = curve_schema_of(&curve);
            assert!(schema.is_structurally_identified());
            assert!(schema.polygonal().is_none());
            assert!(schema.circular_arc().is_some());
            assert!(cylinder_curve_schema_of(&curve).is_structurally_identified());
        }
    }

    #[test]
    fn the_planar_curve_schema_carries_the_arc_placement_the_source_declared() {
        // The placement must be the source curve's own basis and its own
        // trimmed interval, so that developing it into a planar chart is an
        // affine map and nothing has to be re-derived from endpoints.
        let schema = curve_schema_of(&circle_curve((0.2, 1.4)));
        let placement = schema.circular_arc().expect("a circle carries a placement");
        assert_eq!(placement.parameter_interval, (0.2, 1.4));
        // `evaluate(t) == center + cos(t) * cos_basis + sin(t) * sin_basis`,
        // checked at the interval's own start rather than at a canonical
        // parameter, because the interval is the authoritative fact.
        let t = placement.parameter_interval.0;
        let evaluated =
            placement.center + t.cos() * placement.cos_basis + t.sin() * placement.sin_basis;
        let decoded = decode_transformed_circle(match &circle_curve((0.2, 1.4)) {
            Curve3D::Conic(Conic3D::Ellipse(e)) => e,
            _ => unreachable!("the fixture is a conic"),
        })
        .expect("the fixture decodes");
        assert!((evaluated - decoded.evaluate(t)).magnitude2() < 1e-24);
    }

    #[test]
    fn a_real_line_classifies_as_the_line_family() {
        let line = Curve3D::Line(StepLine(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        ));
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
        assert_eq!(
            cylinder_curve_schema_of(&curve).tag(),
            curve_schema_of(&curve).tag()
        );
    }

    /// The two seams now read circles identically, so the cylinder route's
    /// admission cannot drift from the planar one. This is the property that
    /// replaced "the planar path refuses a circle": there is one circle reader
    /// in the workspace, and both callers see its answer.
    #[test]
    fn both_seams_read_a_circle_the_same_way() {
        let curve = circle_curve((0.0, 1.0));
        assert!(curve_schema_of(&curve).is_structurally_identified());
        assert_eq!(
            curve_schema_of(&curve).tag(),
            cylinder_curve_schema_of(&curve).tag()
        );
        assert_eq!(
            curve_schema_of(&curve).circular_arc().copied(),
            cylinder_curve_schema_of(&curve).circular_arc().copied()
        );
    }
}

#[cfg(test)]
mod source_closure_tests {
    use super::*;
    use truck_geometry::prelude::{BSplineSurface, KnotVec, Point3};
    use truck_meshalgo::tessellation::domain::lattice::AxisPeriodStatus;

    /// A degree-2 clamped spline over `[0,1]²` whose `v` direction is a
    /// genuinely closed strip: the last control row equals the first, and the
    /// penultimate row is the reflection `2·P0 − P1`, so position *and* first
    /// derivative close exactly at the seam (measured seam residual ~1e-16).
    ///
    /// The source declaration of closure is *not* read from this geometry: the
    /// caller supplies the declaration, and the evaluator seam check only has
    /// to accept an actually-compatible conversion.
    fn closed_v_spline() -> Surface {
        let knots = (KnotVec::uniform_knot(2, 2), KnotVec::uniform_knot(2, 2));
        let column = |r: f64, z: f64| {
            let p0 = Point3::new(r, 0.0, z);
            let p1 = Point3::new(r * 0.3, r, z);
            let p2 = Point3::new(p0.x * 2.0 - p1.x, p0.y * 2.0 - p1.y, z);
            vec![p0, p1, p2, p0]
        };
        let control_points = vec![
            column(1.0, 0.0),
            column(1.3, 0.5),
            column(1.0, 1.0),
            column(0.7, 1.5),
        ];
        Surface::BSplineSurface(BSplineSurface::new(knots, control_points))
    }

    /// A spline whose `v` knot structure is *not* clamped (the left end knot
    /// has multiplicity 2 rather than `degree + 1`), so its basis-valid
    /// `evaluation_range` is narrower than its declared range. Still source
    /// open in the test's declaration.
    fn unclamped_v_spline() -> Surface {
        let knots = (
            KnotVec::uniform_knot(2, 2),
            KnotVec::from(vec![0.0, 0.0, 0.25, 0.5, 0.75, 1.0, 1.0]),
        );
        let column = |z: f64| {
            vec![
                Point3::new(1.0, 0.0, z),
                Point3::new(0.0, 1.0, z),
                Point3::new(-1.0, 0.0, z),
                Point3::new(0.0, -1.0, z),
            ]
        };
        let control_points = vec![column(0.0), column(0.5), column(1.0), column(1.5)];
        Surface::BSplineSurface(BSplineSurface::new(knots, control_points))
    }

    fn v_closed() -> SplineAxisClosure {
        SplineAxisClosure {
            u_closed: false,
            v_closed: true,
        }
    }

    fn expect_exact_v_period(surface: &Surface, closure: SplineAxisClosure, period: f64) {
        let lattice = lattice_of_with_closure(surface, Some(closure));
        match lattice.v {
            AxisPeriodStatus::Exact {
                period: got,
                witness: truck_meshalgo::tessellation::domain::lattice::PeriodWitness::
                    SourceDeclaredClosedSplineAxis,
            } => assert_eq!(got, period, "certified V period mismatch"),
            other => panic!(
                "expected exact V period {period} with SourceDeclaredClosedSplineAxis, got {other:?}"
            ),
        }
    }

    fn expect_v_non_periodic(surface: &Surface, closure: SplineAxisClosure) {
        let lattice = lattice_of_with_closure(surface, Some(closure));
        assert!(
            matches!(lattice.v, AxisPeriodStatus::NonPeriodic),
            "expected non-periodic V, got {lattice:?}"
        );
    }

    /// A1 — explicit source V closed: the seam-compatible conversion certifies
    /// the exact V period `b - a` with the spline witness.
    #[test]
    fn a1_source_closed_v_certifies_exact_v_period() {
        expect_exact_v_period(&closed_v_spline(), v_closed(), 1.0);
    }

    /// A2 — explicit source V open: no period, even though the declared flag is
    /// read. The declaration is the authority.
    #[test]
    fn a2_source_open_v_is_non_periodic() {
        expect_v_non_periodic(&closed_v_spline(), SplineAxisClosure::OPEN);
    }

    /// A3 — source open + coincident seam geometry: the surface is genuinely
    /// closed at its seam (the closure is not inferred from that).
    #[test]
    fn a3_source_open_with_coincident_seam_geometry_stays_open() {
        expect_v_non_periodic(&closed_v_spline(), SplineAxisClosure::OPEN);
    }

    /// A4 — source open + repeated control net (first control row == last, the
    /// geometric symptom of a wrapped net): still no period.
    #[test]
    fn a4_source_open_with_repeated_control_net_stays_open() {
        expect_v_non_periodic(&closed_v_spline(), SplineAxisClosure::OPEN);
    }

    /// A5 — source open + unclamped knot structure: still no period.
    #[test]
    fn a5_source_open_with_unclamped_knots_stays_open() {
        expect_v_non_periodic(&unclamped_v_spline(), SplineAxisClosure::OPEN);
    }

    /// A certified V period must never be fabricated from a source-open
    /// declaration, and a source-closed declaration whose converted evaluator
    /// does *not* identify the seam must not be promoted either. Both gates are
    /// on the same declaration pair.
    #[test]
    fn a_declared_closure_with_an_incompatible_conversion_is_not_certified() {
        // The unclamped spline's evaluator does not close its V seam (its basis
        // domain is narrower than a full period), so even a source-closed
        // declaration must decline the certification.
        expect_v_non_periodic(&unclamped_v_spline(), v_closed());
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

    /// A sphere's azimuth is a rotation about the polar axis by construction
    /// of the primitive, so `lattice_of` reads `2π` from the representation
    /// rather than from an accessor. The polar axis carries no period.
    #[test]
    fn a_sphere_certifies_its_azimuth_as_a_generator() {
        use truck_geometry::prelude::Sphere;
        use truck_stepio::r#in::step_geometry::Sphere as StepSphere;
        let sphere = Surface::ElementarySurface(ElementarySurface::Sphere(Processor::new(
            StepSphere(Sphere::new(Point3::origin(), 1.0)),
        )));
        let lattice = lattice_of(&sphere);
        // The stepio `Sphere` wrapper puts longitude on caller-`u`; an upright
        // processor keeps it there.
        assert_eq!(lattice.u_generator(), Some(std::f64::consts::PI * 2.0));
        assert_eq!(lattice.v_generator(), None);
        assert_eq!(lattice.certified_rank(), 1);
    }

    /// An inverted processor restates every axis-indexed fact in the caller's
    /// convention: the certified azimuth moves to the caller's `v`.
    #[test]
    fn an_inverted_sphere_puts_the_certified_azimuth_on_v() {
        use truck_geometry::prelude::Sphere;
        use truck_stepio::r#in::step_geometry::Sphere as StepSphere;
        let mut processor = Processor::new(StepSphere(Sphere::new(Point3::origin(), 1.0)));
        use truck_meshalgo::prelude::Invertible;
        processor.invert();
        let sphere = Surface::ElementarySurface(ElementarySurface::Sphere(processor));
        let lattice = lattice_of(&sphere);
        assert_eq!(lattice.v_generator(), Some(std::f64::consts::PI * 2.0));
        assert_eq!(lattice.u_generator(), None);
        assert_eq!(lattice.certified_rank(), 1);
    }
}
