//! Record what `PolyBoundary::new` actually sees, face by face.
//!
//! The untrimmed-face bug is argued from a code path; this observes it. Each
//! face is tessellated on its own, sequentially, with `TRUCK_PROBE_BOUNDARY`
//! set so the fork prints one `PROBE` line per face before this prints its
//! own. The two lines pair up because nothing runs in parallel.
//!
//! Columns: the surface kind, how many closed and open wire pieces the face
//! offered, each closed loop's signed area in uv, whether the surface has a
//! parameter rectangle, whether the synthetic rectangle was appended, and what
//! came out — triangles and the 3D diameter of the result against the
//! diameter of the face's own boundary. A face whose mesh is far larger than
//! its boundary is inverted.
//!
//! ```console
//! TRUCK_PROBE_BOUNDARY=1 cargo run --release --example boundary_probe -- MODEL.step
//! ```

use std::{collections::HashMap, env};

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{
    Table,
    step_geometry::{ElementarySurface, Surface},
};
use truck_topology::compress::{CompressedEdgeIndex, CompressedFace, CompressedShell};

type Cshell = CompressedShell<Point3, truck_stepio::r#in::step_geometry::Curve3D, Surface>;

const RELATIVE_TOLERANCE: f64 = 0.001;
const DEGENERATE_TOLERANCE: f64 = 1.0e-3;

fn surface_kind(surface: &Surface) -> &'static str {
    match surface {
        Surface::ElementarySurface(elementary) => match elementary {
            ElementarySurface::Plane(_) => "plane",
            ElementarySurface::Sphere(_) => "sphere",
            ElementarySurface::CylindricalSurface(_) => "cylinder",
            ElementarySurface::ToroidalSurface(_) => "torus",
            ElementarySurface::ConicalSurface(_) => "cone",
        },
        Surface::SweptCurve(_) => "swept",
        Surface::BSplineSurface(_) => "bspline",
        Surface::NurbsSurface(_) => "nurbs",
        Surface::OffsetSurface(_) => "offset",
    }
}

/// A shell holding one face and only the edges that face uses. Copied from
/// `face_profile`: handing the whole shell to `cshell_tessellation` once per
/// face re-tessellates every edge each time.
fn single_face_shell(shell: &Cshell, index: usize) -> Cshell {
    let face = &shell.faces[index];
    let mut edges = Vec::new();
    let mut seen: HashMap<usize, usize> = HashMap::new();
    let mut boundaries = Vec::with_capacity(face.boundaries.len());
    for wire in &face.boundaries {
        let mut rewired = Vec::with_capacity(wire.len());
        for edge in wire {
            if edge.index >= shell.edges.len() {
                continue;
            }
            let remapped = *seen.entry(edge.index).or_insert_with(|| {
                edges.push(shell.edges[edge.index].clone());
                edges.len() - 1
            });
            rewired.push(CompressedEdgeIndex {
                index: remapped,
                orientation: edge.orientation,
            });
        }
        boundaries.push(rewired);
    }
    CompressedShell {
        vertices: Vec::new(),
        edges,
        faces: vec![CompressedFace {
            boundaries,
            orientation: face.orientation,
            surface: face.surface.clone(),
            // Keep the source identity of the face this was lifted from.
            provenance: face.provenance,
        }],
        source_geometric_uncertainty: None,
    }
}

/// The 3D extent of a face's own boundary, sampled off the edge curves.
///
/// Vertices alone are not enough: a circle carries one vertex, so a
/// circle-bounded face measures as a point and every mesh looks enormous
/// beside it. That mistake produced the "710 untrimmed faces" figure.
fn boundary_diameter(shell: &Cshell, index: usize, tolerance: f64) -> f64 {
    let mut bounds = BoundingBox::<Point3>::new();
    for wire in &shell.faces[index].boundaries {
        for edge in wire {
            let Some(entry) = shell.edges.get(edge.index) else {
                continue;
            };
            let curve = &entry.curve;
            for point in PolylineCurve::from_curve(curve, curve.range_tuple(), tolerance).iter() {
                bounds.push(*point);
            }
        }
    }
    bounds.diameter()
}

fn main() -> anyhow::Result<()> {
    let models: Vec<String> = env::args().skip(1).collect();
    if models.is_empty() {
        anyhow::bail!("usage: boundary_probe MODEL.step [MODEL.step ...]");
    }
    if env::var_os("TRUCK_PROBE_BOUNDARY").is_none() {
        eprintln!("note: TRUCK_PROBE_BOUNDARY is unset, so no PROBE lines will appear");
    }

    for model in &models {
        println!("\n=== {model} ===");
        let bytes = std::fs::read(model)?;
        let text = std::str::from_utf8(&bytes)
            .map(std::borrow::Cow::Borrowed)
            .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());
        let mut exchange = match look::step::part21::parse(&text) {
            Ok(exchange) => exchange,
            Err(_) => ruststep::parser::parse(&text)
                .map_err(|error| anyhow::anyhow!("failed to parse STEP: {error}"))?,
        };
        let section = exchange.data.swap_remove(0);
        let table = Table::from_owned_data_section(section);
        let shells = table
            .shell
            .iter()
            .filter_map(|(&shell_id, shell)| table.to_compressed_shell(shell_id, shell).ok())
            .collect::<Vec<_>>();

        let mut model_box = BoundingBox::<Point3>::new();
        for shell in &shells {
            for vertex in &shell.vertices {
                model_box.push(*vertex);
            }
        }
        let scaled = model_box.diameter() * RELATIVE_TOLERANCE;
        let tolerance = if scaled.is_finite() && scaled > 0.0 {
            scaled
        } else {
            DEGENERATE_TOLERANCE
        };
        println!("{} shells, tolerance {tolerance:.9}\n", shells.len());

        for (shell_index, shell) in shells.iter().enumerate() {
            for index in 0..shell.faces.len() {
                let bdry = boundary_diameter(shell, index, tolerance);
                let one = single_face_shell(shell, index);
                let meshed = one.robust_triangulation(tolerance);
                let mesh = &meshed.faces[0].surface;
                let (tris, mesh_diameter) = match mesh {
                    Some(mesh) => {
                        let mut bounds = BoundingBox::<Point3>::new();
                        for position in mesh.positions() {
                            bounds.push(*position);
                        }
                        (mesh.tri_faces().len(), bounds.diameter())
                    }
                    None => (0, 0.0),
                };
                let ratio = match bdry > 0.0 {
                    true => mesh_diameter / bdry,
                    false => f64::NAN,
                };
                println!(
                    "FACE shell={shell_index} face={index} kind={} bdry_diam={bdry:.6} \
                     mesh_diam={mesh_diameter:.6} ratio={ratio:.2} tris={tris}{}",
                    surface_kind(&shell.faces[index].surface),
                    match mesh.is_none() {
                        true => " DROPPED",
                        false => "",
                    }
                );
            }
        }
    }
    Ok(())
}
