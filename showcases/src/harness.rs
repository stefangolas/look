//! The showcase harness: BREP census + volume, certificate/report JSON,
//! STL + STEP export, and the report schema both the Rust battery and the
//! future truck123d suite assert against.

use std::collections::HashMap;
use std::fs;
use std::io::BufWriter;
use std::path::Path;

use serde::{Deserialize, Serialize};
use truck_base::cgmath64::*;
use truck_geotrait::ParametricSurface;
use truck_modeling::{Curve, Solid};
use truck_polymesh::PolygonMesh;
use truck_topology::EdgeID;

/// One swept part's facet-side report (from `facet_sweep`'s audit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetReport {
    pub law: String,
    pub triangle_count: usize,
    pub quad_count: usize,
    pub signed_volume: f64,
    pub winding_violations: usize,
    pub verdict: String,
    pub refusal: Option<String>,
}

/// One swept part's BREP-side report (from `spine_sweep`'s authored solid).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrepReport {
    pub law: String,
    pub face_count: usize,
    pub edge_use_pairs: usize,
    pub edge_nonpair_uses: usize,
    pub edge_orientation_sum: i64,
    pub signed_volume: f64,
    pub refusal: Option<String>,
}

/// The outcome of one export attempt. Failures are recorded, never fatal:
/// an export refusal is evidence like any other.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReport {
    pub kind: String,
    pub path: String,
    pub ok: bool,
    pub detail: Option<String>,
}

/// The whole-run report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowcaseReport {
    pub item: String,
    pub spine_total_length: f64,
    pub spine_samples: usize,
    pub facets: Vec<FacetReport>,
    pub breps: Vec<BrepReport>,
    pub exports: Vec<ExportReport>,
    pub booleans: Vec<CcPortReport>,
    pub cc_ports: Vec<CcPortReport>,
}

/// A CC-port probe outcome from the run (the CC readiness checklist).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcPortReport {
    pub port: String,
    pub status: String,
    pub detail: Option<String>,
}

/// The parametrization-signed volume of a BREP solid: `V = (1/6) * sum
/// a.(b x c)` over the fan triangulation of every face sampled on a fixed
/// grid (the `spine_sweep_conformance` ground-truth method, promoted to a
/// harness utility). The sign is the RAW parametrization sign, deliberately
/// NOT normalized: on the constructive sweeps it is a frame-handedness
/// diagnostic — see `docs/defects/ORI-FRAME-HANDEDNESS-001.md`. Grid is
/// 48x48 so a long curvy sweep is not undersampled; the conformance test's
/// straight prism is exact at 12.
pub fn brep_volume(solid: &Solid) -> f64 {
    let mut sum = 0.0;
    const N: usize = 96;
    for face in solid.face_iter() {
        let surface = face.surface();
        let (ur, vr) = surface.try_range_tuple();
        let (Some((u0, u1)), Some((v0, v1))) = (ur, vr) else {
            continue;
        };
        for i in 0..N {
            for j in 0..N {
                let a = surface.subs(
                    u0 + (u1 - u0) * (i as f64 / N as f64),
                    v0 + (v1 - v0) * (j as f64 / N as f64),
                );
                let b = surface.subs(
                    u0 + (u1 - u0) * ((i + 1) as f64 / N as f64),
                    v0 + (v1 - v0) * (j as f64 / N as f64),
                );
                let c = surface.subs(
                    u0 + (u1 - u0) * ((i + 1) as f64 / N as f64),
                    v0 + (v1 - v0) * ((j + 1) as f64 / N as f64),
                );
                let d = surface.subs(
                    u0 + (u1 - u0) * (i as f64 / N as f64),
                    v0 + (v1 - v0) * ((j + 1) as f64 / N as f64),
                );
                let o = Point3::new(0.0, 0.0, 0.0);
                sum += (a - o).dot((b - o).cross(c - o));
                sum += (a - o).dot((c - o).cross(d - o));
            }
        }
    }
    sum / 6.0
}

/// The per-edge use census of a solid's shell: every edge id with its use
/// count and the sum of its signed orientations (a closed manifold shell has
/// every sum == 0 and every count == 2).
pub fn edge_census(solid: &Solid) -> HashMap<EdgeID<Curve>, (usize, i32)> {
    let mut census: HashMap<EdgeID<Curve>, (usize, i32)> = HashMap::new();
    for face in solid.face_iter() {
        for wire in face.absolute_boundaries() {
            for edge in wire.iter() {
                let entry = census.entry(edge.id()).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += if edge.orientation() { 1 } else { -1 };
            }
        }
    }
    census
}

/// Aggregated census facts for the report.
pub fn census_summary(solid: &Solid) -> (usize, usize, i64) {
    let census = edge_census(solid);
    let pairs = census.len();
    let bad = census.values().filter(|(uses, _)| *uses != 2).count();
    let orientation_sum: i64 = census.values().map(|(_, signed)| *signed as i64).sum();
    (pairs, bad, orientation_sum)
}

/// Writes one facet mesh as binary STL.
pub fn write_stl(mesh: &PolygonMesh, path: &Path) -> Result<(), String> {
    let file = fs::File::create(path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);
    truck_polymesh::stl::write(mesh, &mut writer, truck_polymesh::stl::StlType::Binary)
        .map_err(|e| e.to_string())
}

/// Writes one BREP solid as STEP (the compressed-topology route used by
/// `truck-stepio`'s own output tests). A carrier the STEP writer cannot
/// display surfaces as a captured `fmt::Error`, never a panic: an export
/// refusal is evidence, recorded by `record_export` like any other.
pub fn write_step(solid: &Solid, path: &Path) -> Result<(), String> {
    use truck_stepio::out::{StepDataDisplay, StepDisplay, StepModel};
    let compressed = solid.compress();
    let display = StepDataDisplay::new(StepModel::from(&compressed), 1);
    let display = StepDisplay::new(Default::default(), display);
    let mut step_string = String::new();
    std::fmt::write(&mut step_string, format_args!("{display}"))
        .map_err(|e| format!("step display failed: {e}"))?;
    fs::write(path, step_string).map_err(|e| e.to_string())
}

/// Tessellates one B-rep solid through the landed meshalgo (per-shell
/// certified triangulation — the same topology-preserving path the kernel's
/// own gates use) and writes it as binary STL. This is the display route for
/// parts realized by the exact backends (`spine_sweep`, `revolve`): the
/// solid is the geometry of record, the mesh is only its tessellation.
///
/// NOTE (kernel trap): tessellating circle-carrying solids panics under
/// debug assertions (the booked topology self-loop trap). Run showcase bins
/// in the `quick` or `release` profile, or accept the panic as the trap's
/// signature in debug.
pub fn write_solid_stl(solid: &Solid, path: &Path) -> Result<(), String> {
    use truck_meshalgo::prelude::*;
    use truck_polymesh::{Faces, StandardAttributes, StandardVertex};
    let mut positions: Vec<Point3> = Vec::new();
    let mut faces = Faces::<StandardVertex>::default();
    for shell in solid.boundaries() {
        let mesh: PolygonMesh = shell.triangulation(0.005).to_polygon();
        let base = positions.len();
        let mut attrs = mesh.attributes().clone();
        positions.extend(attrs.positions.drain(..));
        for face in mesh.faces().face_iter() {
            let mut tri = [face[0], face[1], face[2]];
            for v in &mut tri {
                *v = StandardVertex {
                    pos: v.pos + base,
                    uv: None,
                    nor: None,
                };
            }
            faces.push(tri);
        }
    }
    let mesh = PolygonMesh::new(
        StandardAttributes {
            positions,
            ..Default::default()
        },
        faces,
    );
    write_stl(&mesh, path)
}

/// Records one export attempt into the report's export list.
pub fn record_export(report: &mut ShowcaseReport, kind: &str, path: &Path, result: Result<(), String>) {
    report.exports.push(ExportReport {
        kind: kind.to_string(),
        path: path.display().to_string(),
        ok: result.is_ok(),
        detail: result.err(),
    });
}

/// Writes the report JSON.
pub fn write_report(report: &ShowcaseReport, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}
