//! Screen 3 — band_curve_probe: what the *imported* curve representation is for
//! every face carrying an edge no structural reader could read.
//!
//! `SliceExit::UnsupportedCurveRepresentation` is raised at one seam — the
//! Step-2 traversal gate in `planar_slice::traverse_bound`, on
//! `!CurveSchema::is_structurally_identified()` — and the exit keeps none of
//! *why*. The cause is right there in the schema it threw away:
//! `CurveSchemaFailure::NoStructuralReader { representation }` carries the
//! reader's own tag for the representation it refused.
//!
//! So this reads the identical production classifier
//! (`look::step_cylinder_curve_schema_of`) over the identical imported shells
//! and prints the tag. It is a pure observer: it runs no tessellation, admits
//! nothing, and cannot move a face between rendered and lost. The band route's
//! own verdict comes from `face_census --ledger`; the two are joined on
//! `source_face_id` by `benchmarks/band_report.py`.
//!
//! Emits one `EDGE` line per unread edge use and one `FACE` line per face that
//! has at least one, so the output is proportional to the failing population
//! rather than to the model.
//!
//! ```console
//! band_curve_probe MODEL.step > model.curves.tsv
//! ```

use std::collections::BTreeMap;
use std::env;

use look::step::circular_arc::shadow_classify_circularity;
use truck_meshalgo::prelude::{InnerSpace, Transform, Vector3};
use truck_meshalgo::tessellation::formal::{CurveSchema, CurveSchemaFailure};
use truck_stepio::r#in::Table;
use truck_stepio::r#in::step_geometry::{Conic3D, Curve3D, ElementarySurface, Surface, SweptCurve};

/// How far a refused `Conic(Ellipse)` actually is from exact circularity.
///
/// `decode_transformed_circle`'s Gram predicate is *exact*: a one-ULP
/// discrepancy in `g00 == g11` or `g01 == 0` is certified non-circular, which
/// is sound but says nothing about magnitude. A genuine STEP `ellipse` and a
/// STEP `circle` whose direction cosines merely do not round-trip to a bitwise
/// similarity both arrive as `arc_non_circular_affine_image`, and the census
/// has to tell them apart.
///
/// So this reports the discrepancy the pre-P0 classifier measured, in units of
/// `f64::EPSILON`, alongside that classifier's own three-way verdict. Both are
/// observations. Neither is used to admit anything: the shadow classifier is
/// documented as diagnostics-only and this is a diagnostics-only binary.
fn circularity_discrepancy(ellipse: &truck_stepio::r#in::step_geometry::Ellipse<
    truck_meshalgo::prelude::Point3,
    truck_meshalgo::prelude::Matrix4,
>) -> (String, &'static str) {
    let transform = *ellipse.transform();
    let basis_cos = transform.transform_vector(Vector3::new(1.0, 0.0, 0.0));
    let basis_sin = transform.transform_vector(Vector3::new(0.0, 1.0, 0.0));
    let len_cos_sq = basis_cos.dot(basis_cos);
    let len_sin_sq = basis_sin.dot(basis_sin);
    let scale = len_cos_sq.max(len_sin_sq);
    let worst = if scale > 0.0 && scale.is_finite() {
        let length_mismatch = (len_cos_sq - len_sin_sq).abs() / scale;
        let orthogonality = basis_cos.dot(basis_sin).abs() / scale;
        length_mismatch.max(orthogonality) / f64::EPSILON
    } else {
        f64::NAN
    };
    let shadow = match shadow_classify_circularity(ellipse) {
        Ok(()) => "shadow_circle",
        Err(failure) => failure.tag(),
    };
    (format!("{worst:.3e}"), shadow)
}

/// The imported `Curve3D` variant's own name.
///
/// Deliberately the *imported* name and not the raw STEP entity type: the two
/// disagree on real files, and that disagreement is the finding. `CIRCLE` and
/// `ELLIPSE` both import as `Conic(Ellipse)`, so this cannot distinguish them
/// and does not pretend to — `benchmarks/step_entity_chain.py` reads the raw
/// entity for that.
fn imported_variant(curve: &Curve3D) -> &'static str {
    match curve {
        Curve3D::Line(_) => "Line",
        Curve3D::Polyline(_) => "Polyline",
        Curve3D::Conic(Conic3D::Ellipse(_)) => "Conic(Ellipse)",
        Curve3D::Conic(Conic3D::Hyperbola(_)) => "Conic(Hyperbola)",
        Curve3D::Conic(Conic3D::Parabola(_)) => "Conic(Parabola)",
        Curve3D::BSplineCurve(_) => "BSplineCurve",
        Curve3D::NurbsCurve(_) => "NurbsCurve",
        Curve3D::PCurve(_) => "PCurve",
    }
}

fn surface_kind(surface: &Surface) -> &'static str {
    match surface {
        Surface::ElementarySurface(e) => match e {
            ElementarySurface::Plane(_) => "plane",
            ElementarySurface::Sphere(_) => "sphere",
            ElementarySurface::CylindricalSurface(_) => "cylinder",
            ElementarySurface::ToroidalSurface(_) => "torus",
            ElementarySurface::ConicalSurface(_) => "cone",
        },
        Surface::SweptCurve(s) => match s {
            SweptCurve::ExtrudedCurve(_) => "extruded",
            SweptCurve::RevolutedCurve(_) => "revolved",
        },
        Surface::BSplineSurface(_) => "bspline",
        Surface::NurbsSurface(_) => "nurbs",
        Surface::OffsetSurface(_) => "offset",
    }
}

/// The schema verdict as a stable tag, plus the reader's own cause.
///
/// `CircularArc` and the polygonal variants are structurally identified and so
/// pass the Step-2 gate; only `NotStructurallyIdentified` reaches the exit this
/// probe exists to explain.
fn verdict(schema: &CurveSchema) -> (&'static str, String) {
    match schema {
        CurveSchema::LineSegment(_) => ("line_segment", "-".into()),
        CurveSchema::Polyline(_) => ("polyline", "-".into()),
        CurveSchema::CircularArc => ("circular_arc", "-".into()),
        CurveSchema::NotStructurallyIdentified(failure) => (
            "not_structurally_identified",
            match failure {
                CurveSchemaFailure::NoStructuralReader { representation } => {
                    (*representation).to_string()
                }
                CurveSchemaFailure::VertexNotFinite { cause } => {
                    format!("vertex_not_finite:{cause:?}")
                }
                CurveSchemaFailure::FewerThanTwoVertices => "fewer_than_two_vertices".into(),
            },
        ),
    }
}

fn load(path: &str) -> anyhow::Result<Table> {
    let bytes = std::fs::read(path)?;
    let text = std::str::from_utf8(&bytes)
        .map(std::borrow::Cow::Borrowed)
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());
    let mut exchange = match look::step::part21::parse(&text) {
        Ok(exchange) => exchange,
        Err(_) => ruststep::parser::parse(&text)
            .map_err(|error| anyhow::anyhow!("failed to parse STEP: {error}"))?,
    };
    let section = exchange.data.swap_remove(0);
    Ok(Table::from_owned_data_section(section))
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(path) = args.first() else {
        eprintln!("usage: band_curve_probe MODEL.step");
        return Ok(());
    };
    let table = load(path)?;

    let mut unread_faces = 0usize;
    let mut unread_edges = 0usize;
    // Model-level histogram of the reader's own causes, so a run is readable
    // without the aggregator.
    let mut causes: BTreeMap<String, usize> = BTreeMap::new();

    for (_, shell) in table.shell.iter() {
        let Ok((cshell, _losses)) = table.to_compressed_shell_with_losses(shell) else {
            continue;
        };
        for face in &cshell.faces {
            let id = face
                .provenance
                .best_id()
                .map(|id| id.get().to_string())
                .unwrap_or_else(|| "-".into());
            let kind = surface_kind(&face.surface);
            let mut face_unread = 0usize;
            let mut face_uses = 0usize;
            for (bound_index, boundary) in face.boundaries.iter().enumerate() {
                for (use_index, edge_index) in boundary.iter().enumerate() {
                    face_uses += 1;
                    let curve = &cshell.edges[edge_index.index].curve;
                    // The production rank-1 classifier, unmodified. Reading it
                    // here is what makes this probe a measurement of the
                    // shipped path rather than of a second opinion.
                    let schema = look::step_cylinder_curve_schema_of(curve);
                    if schema.is_structurally_identified() {
                        continue;
                    }
                    let (tag, cause) = verdict(&schema);
                    face_unread += 1;
                    unread_edges += 1;
                    *causes.entry(cause.clone()).or_default() += 1;
                    // Only a conic can have a circularity discrepancy to
                    // report; every other refusal is a missing reader, not a
                    // near miss.
                    let (ulps, shadow) = match curve {
                        Curve3D::Conic(Conic3D::Ellipse(ellipse)) => {
                            circularity_discrepancy(ellipse)
                        }
                        _ => ("-".into(), "-"),
                    };
                    println!(
                        "EDGE\tsource_face_id={id}\tsurface_kind={kind}\t\
                         bound_index={bound_index}\tuse_index={use_index}\t\
                         edge_index={}\torientation={}\timported={}\tschema={tag}\tcause={cause}\t\
                         circularity_ulps={ulps}\tshadow={shadow}",
                        edge_index.index,
                        edge_index.orientation,
                        imported_variant(curve),
                    );
                }
            }
            if face_unread > 0 {
                unread_faces += 1;
                println!(
                    "FACE\tsource_face_id={id}\tsurface_kind={kind}\tbounds={}\t\
                     edge_uses={face_uses}\tunread_edge_uses={face_unread}",
                    face.boundaries.len(),
                );
            }
        }
    }

    eprintln!("band_curve_probe: {unread_faces} faces carry {unread_edges} unread edge uses");
    for (cause, count) in &causes {
        eprintln!("  {cause:44} {count:6}");
    }
    Ok(())
}
