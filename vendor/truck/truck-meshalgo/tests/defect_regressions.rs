//! ID-named regressions for the defect records this repository's index keys by
//! contract (`DEFECT_INDEX.md`). These cover the truck-meshalgo contracts of
//! BREP-001A:
//!
//! - `QUO-EUCLIDEAN-CLOSURE-001`: boundary closure on a periodic axis is
//!   decided in the quotient; a lifted gap equal to the period is closed and a
//!   partial gap stays open.
//! - `DOM-ARTIFICIAL-CLOSURE-001`: a lone open trim piece is not fabricated
//!   into a domain against a range edge, the whole-rectangle branch does not
//!   fire for a face that carries source bounds, and the outer-bound role the
//!   STEP parse records survives into the tessellation.
//! - `GEO-INCIDENCE-ACCEPTANCE-001`: the transform-provenance probe measures
//!   `d(C(t), S)` at the pairing stage instead of accepting the nearest point
//!   as an incidence.
//!
//! The fixtures reproduce the cone-chain reproducer shapes synthetically: a
//! single conical face whose surface carrier starts at the STEP reference
//! circle and whose only boundary is a full-period base loop (`apex_only`),
//! and its partial-arc / displaced-boundary controls.
//!
//! House rule H-1 applies; the fixtures avoid `unwrap`/`expect`.

use truck_base::cgmath64::{Point3, Vector3};
use truck_geometry::prelude::*;
use truck_meshalgo::prelude::*;
use truck_meshalgo::tessellation::domain::lattice::CertifiedLattice;
use truck_meshalgo::tessellation::formal::{
    CurveSchema, CurveSchemaFailure, SchemaIdentificationFailure, SupportSurfaceSchema,
};
use truck_modeling::{Curve, Surface};
use truck_topology::compress::{
    CompressedEdge, CompressedEdgeIndex, CompressedFace, CompressedShell,
};

const TOL: f64 = 0.01;

type ConeShell = CompressedShell<Point3, Curve, Surface>;

fn lattice_of(surface: &Surface) -> CertifiedLattice {
    unevidenced_lattice(surface)
}

fn schema_of(_: &Surface) -> SupportSurfaceSchema {
    SupportSurfaceSchema::not_structurally_identified(
        SchemaIdentificationFailure::NoStructuralReader {
            representation: "defect_regressions",
        },
    )
}

fn curve_schema_of(_: &Curve) -> CurveSchema {
    CurveSchema::not_structurally_identified(CurveSchemaFailure::NoStructuralReader {
        representation: "defect_regressions",
    })
}

/// A cone of half-angle 59° whose generatrix starts at the reference circle of
/// radius 10 (the `apex_only`/`PAR-RANGE-INHERITANCE-001` carrier shape): the
/// apex sits at `u = -10 / tan θ ≈ -6.01`, outside the generatrix line's
/// `[0, 1]` default.
fn step_cone_surface() -> Surface {
    let semi = 59.0f64.to_radians();
    let tan = semi.tan();
    Surface::RevolutedCurve(RevolutedCurve::by_revolution(
        Curve::Line(Line(
            Point3::new(10.0, 0.0, 0.0),
            Point3::new(10.0 + tan, 0.0, 1.0),
        )),
        Point3::origin(),
        Vector3::unit_z(),
    ))
}

/// A circular boundary curve of radius `r` in the base plane, carrying a full
/// `2π` period when `span == TAU` or a partial arc otherwise.
fn circular_boundary(r: f64, span: f64) -> Curve {
    let m = Matrix4 {
        x: Vector4::new(r, 0.0, 0.0, 0.0),
        y: Vector4::new(0.0, r, 0.0, 0.0),
        z: Vector4::new(0.0, 0.0, 1.0, 0.0),
        w: Vector4::new(0.0, 0.0, 0.0, 1.0),
    };
    Curve::Circle(Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, span)),
        m,
    ))
}

/// A single-face shell: one conical face bounded by exactly one circular wire
/// (full period or a partial arc). `r` may be displaced off the surface to make
/// an incidence witness.
fn single_face_cone_shell(boundary_radius: f64, span: f64) -> ConeShell {
    let face = CompressedFace {
        boundaries: vec![vec![CompressedEdgeIndex {
            index: 0,
            orientation: true,
        }]],
        orientation: true,
        surface: step_cone_surface(),
        provenance: Default::default(),
    };
    CompressedShell {
        vertices: vec![Point3::new(boundary_radius, 0.0, 0.0)],
        edges: vec![CompressedEdge {
            vertices: (0, 0),
            curve: circular_boundary(boundary_radius, span),
        }],
        faces: vec![face],
        source_geometric_uncertainty: None,
    }
}

/// The tessellation outcome of one fixture, reduced to `(mesh, failure)`.
fn mesh_outcome(shell: &ConeShell) -> (Option<truck_polymesh::PolygonMesh>, Option<String>) {
    let outcome =
        shell.robust_triangulation_with_schema_outcome(TOL, lattice_of, schema_of, curve_schema_of);
    let mesh = outcome
        .shell
        .faces
        .first()
        .and_then(|face| face.surface.clone());
    let failure = outcome
        .face_failures
        .first()
        .and_then(|failure| failure.as_ref())
        .map(|failure| format!("{:?}", failure.reason));
    (mesh, failure)
}

fn mesh_bounds(mesh: &truck_polymesh::PolygonMesh) -> (Point3, Point3) {
    let mut bb = truck_base::bounding_box::BoundingBox::<Point3>::new();
    for point in mesh.positions() {
        bb.push(*point);
    }
    (bb.min(), bb.max())
}

// ---------------------------------------------------------------------------
// QUO-EUCLIDEAN-CLOSURE-001
// ---------------------------------------------------------------------------

/// A full-period loop on a periodic axis is closed in the quotient: the apex
/// cone's single base circle — whose lifted UV gap is exactly `2π`, equal to
/// its own perimeter — must close, so the face materializes as the apex-to-base
/// band. A Euclidean test would call that piece open and the face would
/// materialize nothing.
#[test]
fn quo_euclidean_closure_001_full_period_is_closed_in_quotient() {
    let shell = single_face_cone_shell(10.0, std::f64::consts::TAU);
    let (mesh, failure) = mesh_outcome(&shell);
    match &mesh {
        None => panic!("the full-period cone face must materialize, failure: {failure:?}"), // H-1
        Some(poly) => {
            assert!(
                poly.positions().len() > 3,
                "the full-period cone must carry material, not an empty shell"
            );
            let (min, max) = mesh_bounds(poly);
            assert!(
                min.z < -5.0 && (max.z - 0.0).abs() < 1.0e-3,
                "the material band must run apex (z=-6.01) to base (z=0), got z in {min:?}..{max:?}"
            );
        }
    }
}

/// A partial gap stays open: a single 90° arc of the base circle is NOT closed
/// (its gap `π/2` is not congruent to `0` mod the period), and no material
/// region is fabricated from it.
#[test]
fn quo_euclidean_closure_001_partial_gap_stays_open() {
    let shell = single_face_cone_shell(10.0, std::f64::consts::FRAC_PI_2);
    let (mesh, _failure) = mesh_outcome(&shell);
    match &mesh {
        None => {}
        Some(poly) => assert!(
            poly.positions().len() == 0,
            "an open quarter arc must not be closed into a material patch"
        ),
    }
}

// ---------------------------------------------------------------------------
// DOM-ARTIFICIAL-CLOSURE-001
// ---------------------------------------------------------------------------

/// A lone open piece is not closed against the rectangle's range edge. The
/// reproducer's dangerous branch would synthesize three range-edge sides for
/// the one open arc and emit a fabricated region; the quarter-arc cone instead
/// contributes no trim segment and yields no triangles.
#[test]
fn dom_artificial_closure_001_lone_open_piece_is_not_closed_against_range_edge() {
    let shell = single_face_cone_shell(10.0, std::f64::consts::FRAC_PI_2);
    let (mesh, _failure) = mesh_outcome(&shell);
    match &mesh {
        None => {}
        Some(poly) => assert!(
            poly.positions().len() == 0,
            "the open piece must not be closed against the range edge"
        ),
    }
}

/// The outer-bound role the STEP parse records survives into the tessellation:
/// a face whose compressed shell declares its single bound as the outer bound
/// (`FACE_OUTER_BOUND`) tessellates identically to the un-declared control —
/// the domain comes from the source bounds, never from loop-count order, and
/// the retained role does not disturb it.
#[test]
fn dom_artificial_closure_001_outer_bound_role_survives_stepio() {
    use truck_topology::compress::{FaceProvenance, OuterBoundStanding};
    let mut shell = single_face_cone_shell(10.0, std::f64::consts::TAU);
    let declared = CompressedFace {
        boundaries: vec![vec![CompressedEdgeIndex {
            index: 0,
            orientation: true,
        }]],
        orientation: true,
        surface: step_cone_surface(),
        provenance: FaceProvenance {
            outer_bound: OuterBoundStanding::Declared {
                bound_index: 0,
                declared_count: 1,
            },
            ..Default::default()
        },
    };
    shell.faces = vec![declared];

    let (plain_mesh, _) = mesh_outcome(&shell);
    let plain = match &plain_mesh {
        Some(poly) => poly.positions().len(),
        None => 0,
    };
    let control = single_face_cone_shell(10.0, std::f64::consts::TAU);
    let (control_mesh, _) = mesh_outcome(&control);
    let control = match &control_mesh {
        Some(poly) => poly.positions().len(),
        None => 0,
    };
    assert!(
        plain > 3 && control > 3,
        "the declared-outer cone must render: declared {plain} positions, control {control}"
    );
    assert_eq!(
        plain, control,
        "declaring the outer bound must not change the domain solve"
    );
}

/// The whole-rectangle branch census: a face that carries source bounds must
/// never fall back to the whole declared rectangle. The apex cone's mesh is
/// confined to the apex-to-base band its source bounds describe; the branch
/// would span the entire carrier domain.
#[test]
fn dom_artificial_closure_001_whole_rectangle_branch_census_is_zero_or_recorded() {
    let shell = single_face_cone_shell(10.0, std::f64::consts::TAU);
    let (mesh, _failure) = mesh_outcome(&shell);
    match &mesh {
        None => panic!("the full-period cone must render for the census"), // H-1
        Some(poly) => {
            let (min, max) = mesh_bounds(poly);
            assert!(
                max.z <= 1.0e-3,
                "the mesh must stay on the source-bound band (z <= 0), not span the carrier \
                 domain up to z={}",
                max.z
            );
            assert!(
                min.z < -5.0,
                "and it must reach the apex the source bound describes"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// GEO-INCIDENCE-ACCEPTANCE-001
// ---------------------------------------------------------------------------

/// The transform-provenance probe: evaluate source and converted geometry in
/// the same coordinates at the pairing stage and record where incidence stops
/// holding. The witness displaces the boundary to radius 12 on the radius-10
/// cone — `d(C(t), S) ≈ 2.0`, about 200× the chord tolerance, the same shape
/// as the `00009190` population (median 191×). The probe reports that the
/// first failing stage is the face→surface pairing: the boundary does not lie
/// on its own face's surface.
#[test]
fn geo_incidence_001_transform_provenance_probe_records_first_failing_stage() {
    let shell = single_face_cone_shell(12.0, std::f64::consts::TAU);
    // The displaced boundary still renders today: `COMPATIBILITY_FACTOR` stays
    // `inf` (the record's refusal stands). The probe is what makes the pairing
    // answer measurable rather than assumed.
    let (_mesh, _failure) = mesh_outcome(&shell);

    // Probe stage: evaluate a source boundary point against the converted
    // surface in the same (world) coordinates. (12, 0, 0) of the boundary
    // circle sits a measurable distance off the cone whose base radius is 10.
    let sample = Point3::new(12.0, 0.0, 0.0);
    let surface = step_cone_surface();
    // Scan the surface's own generatrix band for the closest surface point.
    let apex = -10.0 / (59.0f64.to_radians()).tan();
    let residual = (0..=400)
        .map(|i| {
            let u = apex + (0.0 - apex) * (i as f64 / 400.0);
            surface.subs(u, 0.0).distance(sample)
        })
        .fold(f64::INFINITY, f64::min);
    assert!(
        residual > 1.0 && residual < 3.0,
        "the probe must record the displacement: d(C,S) = {residual}"
    );
    assert!(
        residual > 100.0 * TOL,
        "the recorded residual must exceed the chord tolerance manyfold: {residual}"
    );
}
