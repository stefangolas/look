//! Diagnostic: what actually controls tessellation density.
//!
//! Runs the production tessellation entry point at several tolerances and
//! reports, per surface kind: faces rendered, triangles, and the edge-curve
//! sample counts the same tolerance produces. Temporary instrumentation for the
//! coarse-mesh investigation; not part of any shipped path.

use std::collections::BTreeMap;
use std::env;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{Table, step_geometry::Surface};

const RELATIVE_TOLERANCE: f64 = 0.001;
const EDGE_SAMPLES: u32 = 4;

fn surface_kind(surface: &Surface) -> &'static str {
    use truck_stepio::r#in::step_geometry::{ElementarySurface, SweptCurve};
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

#[derive(Default, Clone, Copy)]
struct Row {
    faces: usize,
    rendered: usize,
    triangles: usize,
}

fn main() -> anyhow::Result<()> {
    let path = env::args().nth(1).expect("usage: tolerance_probe <step>");
    let text = std::fs::read_to_string(&path)?;
    let mut exchange =
        ruststep::parser::parse(&text).map_err(|e| anyhow::anyhow!("parse failed: {e}"))?;
    let section = exchange.data.swap_remove(0);
    let table = Table::from_owned_data_section(section);

    let mut converted = Vec::new();
    for (&shell_id, shell) in table.shell.iter() {
        if let Ok(cshell) = table.to_compressed_shell(shell_id, shell) {
            converted.push(cshell);
        }
    }

    let mut model = BoundingBox::<Point3>::new();
    for shell in &converted {
        for v in &shell.vertices {
            model.push(*v);
        }
        for edge in &shell.edges {
            let (a, b) = edge.curve.range_tuple();
            for i in 0..=EDGE_SAMPLES {
                model.push(
                    edge.curve
                        .subs(a + (b - a) * f64::from(i) / f64::from(EDGE_SAMPLES)),
                );
            }
        }
    }
    let diameter = model.diameter();
    let production = diameter * RELATIVE_TOLERANCE;
    println!("model bbox min={:?} max={:?}", model.min(), model.max());
    println!("diameter={diameter:.6}  production tolerance={production:.6e}");
    let edges: usize = converted.iter().map(|s| s.edges.len()).sum();
    let faces: usize = converted.iter().map(|s| s.faces.len()).sum();
    println!("shells={} edges={edges} faces={faces}", converted.len());

    let factors: Vec<f64> = env::args()
        .skip(2)
        .map(|a| a.parse().expect("factor"))
        .collect();
    let factors = if factors.is_empty() {
        vec![100.0, 10.0, 1.0, 0.1, 0.01]
    } else {
        factors
    };

    for factor in factors {
        let tol = production * factor;
        // Curve stage, measured on its own: how many polyline points the same
        // tolerance yields over every edge of the model.
        let mut curve_points = 0usize;
        let mut curve_max = 0usize;
        for shell in &converted {
            for edge in &shell.edges {
                let poly = PolylineCurve::from_curve(&edge.curve, edge.curve.range_tuple(), tol);
                curve_points += poly.len();
                curve_max = curve_max.max(poly.len());
            }
        }

        let mut rows: BTreeMap<&'static str, Row> = BTreeMap::new();
        let mut total = 0usize;
        for shell in &converted {
            let kinds: Vec<&'static str> = shell
                .faces
                .iter()
                .map(|f| surface_kind(&f.surface))
                .collect();
            let outcome = shell.robust_triangulation_with_torus_outcome(
                tol,
                look::step_lattice_of,
                look::step_support_schema_of,
                look::step_curve_schema_of,
                look::step_cylinder_of,
                look::step_cylinder_curve_schema_of,
                look::step_cylinder_curve_family_of,
                look::step_cone_of,
                look::step::torus_deck::identify_source_torus_opt,
            );
            for (i, face) in outcome.shell.faces.iter().enumerate() {
                let kind = if i < kinds.len() { kinds[i] } else { "?" };
                let tris = face.surface.as_ref().map_or(0, |m| m.tri_faces().len());
                let row = rows.entry(kind).or_default();
                row.faces += 1;
                if tris > 0 {
                    row.rendered += 1;
                }
                row.triangles += tris;
                total += tris;
            }
        }
        println!(
            "\n=== factor={factor} tol={tol:.6e} total_triangles={total} \
             curve_points={curve_points} max_edge_points={curve_max}"
        );
        for (kind, row) in &rows {
            println!(
                "  {kind:10} faces={:5} rendered={:5} triangles={:7} tri/face={:.1}",
                row.faces,
                row.rendered,
                row.triangles,
                row.triangles as f64 / row.rendered.max(1) as f64
            );
        }
    }
    Ok(())
}
