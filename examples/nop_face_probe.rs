//! NoOdd target-face probe: run the full production tessellation pipeline on a
//! single face (identified by `source_face_id`) and report the terminal reason,
//! CDT stage counts, and validity certificate. Avoids tessellating the other
//! 22k-30k faces of a model.
//!
//! The face is placed alone in a shell that keeps the model's full vertex and
//! edge arrays, so edge indices and vertex handles are identical to the full
//! model (no re-indexing drift). Only the face list is narrowed.
//!
//! ```console
//! nop_face_probe MODEL.step FACE_ID[,FACE_ID...]
//! ```

use std::env;
use std::path::Path;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{
    Table,
    step_geometry::{Curve3D, Surface},
};
use truck_topology::compress::{CompressedShell, FaceProvenance};

type Cshell = CompressedShell<Point3, Curve3D, Surface>;

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

fn surface_kind(surface: &Surface) -> &'static str {
    match surface {
        Surface::ElementarySurface(e) => match e {
            truck_stepio::r#in::step_geometry::ElementarySurface::Plane(_) => "plane",
            truck_stepio::r#in::step_geometry::ElementarySurface::CylindricalSurface(_) => {
                "cylinder"
            }
            truck_stepio::r#in::step_geometry::ElementarySurface::ConicalSurface(_) => "cone",
            truck_stepio::r#in::step_geometry::ElementarySurface::Sphere(_) => "sphere",
            truck_stepio::r#in::step_geometry::ElementarySurface::ToroidalSurface(_) => "torus",
        },
        Surface::BSplineSurface(_) => "bspline",
        Surface::NurbsSurface(_) => "nurbs",
        Surface::SweptCurve(_) => "swept",
        Surface::OffsetSurface(_) => "offset",
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() < 2 {
        anyhow::bail!("usage: nop_face_probe MODEL.step FACE_ID[,FACE_ID...]");
    }
    let model = &args[0];
    let targets: Vec<u64> = args[1]
        .split(',')
        .filter_map(|s| s.trim().parse::<u64>().ok())
        .collect();

    let table = load(model)?;
    let closure_map = look::step::lattice::spline_closure_map(&table);

    let mut all_cshells: Vec<Cshell> = Vec::new();
    let mut shell_entities: Vec<u64> = Vec::new();
    for (&shell_entity, shell) in table.shell.iter() {
        if let Ok((cshell, _losses)) = table.to_compressed_shell_with_losses(shell_entity, shell) {
            shell_entities.push(shell_entity);
            all_cshells.push(cshell);
        }
    }
    let mut model_bbox = BoundingBox::<Point3>::new();
    for cshell in &all_cshells {
        for v in &cshell.vertices {
            model_bbox.push(*v);
        }
        for edge in &cshell.edges {
            let (a, b) = edge.curve.range_tuple();
            for i in 0..=4 {
                model_bbox.push(edge.curve.subs(a + (b - a) * f64::from(i) / 4.0));
            }
        }
    }
    let scaled = model_bbox.diameter() * 0.001;
    let model_tol = if scaled.is_finite() && scaled > 0.0 {
        scaled.max(1e-6)
    } else {
        1e-3
    };
    eprintln!(
        "MODEL_DIAMETER\t{:.9}\tCENSUS_TOL\t{:.9}",
        model_bbox.diameter(),
        model_tol
    );

    let mut found = std::collections::HashSet::new();
    for (cshell, shell_entity) in all_cshells.iter().zip(&shell_entities) {
        for (fi, face) in cshell.faces.iter().enumerate() {
            let Some(id) = face.provenance.best_id() else {
                continue;
            };
            let idv = id.get();
            if !targets.contains(&idv) {
                continue;
            }
            found.insert(idv);
            // Single-face shell: full vertex/edge arrays, one face entry.
            let single = CompressedShell {
                vertices: cshell.vertices.clone(),
                edges: cshell.edges.clone(),
                faces: vec![truck_topology::compress::CompressedFace {
                    boundaries: face.boundaries.clone(),
                    orientation: face.orientation,
                    surface: face.surface.clone(),
                    provenance: FaceProvenance {
                        definition_id: face.provenance.definition_id,
                        use_id: face.provenance.use_id,
                        surface_id: face.provenance.surface_id,
                        outer_bound: face.provenance.outer_bound,
                    },
                }],
                source_geometric_uncertainty: cshell.source_geometric_uncertainty,
            };

            use look::step::policy_geometry::{PolicyCurve, PolicySurface};
            let wrapped = look::step::policy_geometry::wrap_shell_with_closure(
                single,
                look::step::meshing_policy::MeshingPolicy::DEFAULT,
                &closure_map,
            );
            let tol: f64 = env::var("NOP_TOL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(model_tol);
            let outcome = wrapped.robust_triangulation_with_torus_outcome(
                tol,
                |s: &PolicySurface| look::step_lattice_of_with_closure(s.inner(), s.source_closure()),
                |s: &PolicySurface| look::step_support_schema_of(s.inner()),
                |c: &PolicyCurve| look::step_curve_schema_of(c.inner()),
                |s: &PolicySurface| look::step_cylinder_of(s.inner()),
                |c: &PolicyCurve| look::step_cylinder_curve_schema_of(c.inner()),
                |c: &PolicyCurve| look::step_cylinder_curve_family_of(c.inner()),
                |s: &PolicySurface| look::step_cone_of(s.inner()),
                |s: &PolicySurface| look::step::torus_deck::identify_source_torus_opt(s.inner()),
            );

            let meshed_face = &outcome.shell.faces[0];
            let triangles = meshed_face
                .surface
                .as_ref()
                .map_or(0, |m| m.tri_faces().len());
            let failure_reason = outcome
                .face_failures
                .get(0)
                .cloned()
                .flatten()
                .map(|f| f.reason);
            let diag = outcome
                .face_diagnoses
                .get(0)
                .and_then(|d| d.clone());
            println!("FACE\tmodel={model}\tsource_face_id={idv}\tkind={}\tshell_entity={shell_entity}\tface_index={fi}\ttol={tol}\ttriangles={triangles}\tterminal={:?}",
                surface_kind(&face.surface), failure_reason);
            if let Some(d) = &diag {
                println!("CDT\t{}", serde_json::to_string(&d.cdt_stages).unwrap_or_default());
                println!(
                    "DERIVED\t{}\tproj={:?}\tlift={:?}",
                    serde_json::to_string(&d.derived_bucket)
                        .unwrap_or_default(),
                    d.projection_status,
                    d.lift_status
                );
                if let Some(cert) = &d.validity_certificate {
                    println!("VALIDITY\t{}", serde_json::to_string(cert).unwrap_or_default());
                }
                if !d.boundary_pieces.is_empty() {
                    let summary: Vec<String> = d
                        .boundary_pieces
                        .iter()
                        .map(|p| {
                            format!(
                                "start={:?} end={:?} area={} closure={:?} pts={}",
                                p.start_uv, p.end_uv, p.signed_area, p.closure, p.point_count
                            )
                        })
                        .collect();
                    println!("PIECES\t{}", summary.join(" ; "));
                }
            }
        }
    }
    let missing: Vec<u64> = targets.iter().copied().filter(|t| !found.contains(t)).collect();
    if !missing.is_empty() {
        println!("MISSING\t{}", missing.iter().map(u64::to_string).collect::<Vec<_>>().join(","));
    }
    Ok(())
}
