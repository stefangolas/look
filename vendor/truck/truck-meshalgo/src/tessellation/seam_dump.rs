//! DEF-TESS-ANALYTIC-SEAM-R2 seam-dump probe.
//!
//! cfg(test)-only. Reads a vendored fixture (or the in-memory
//! `special_cylinder` shell), tessellates it through the same entry points the
//! vendored tests use, and reports the pre-weld leftover boundary of the merged
//! `PolygonMesh` before and after `put_together_same_attrs`.
//!
//! The probe's job is to say *where* seam vertices disagree and by how much:
//! whether the leftover is (a) a parameterization mismatch (adjacent faces
//! sample the shared curve at different parameters), (b) an evaluation
//! asymmetry (identical parameters evaluate to different positions on the two
//! carriers), or (c) a face that emits no seam vertices at all. The two tests
//! at the bottom assert the fixed post-weld state for the fixtures whose seams
//! the vendored suite checks.

use crate::analyzers::Topology;
use crate::filters::OptimizingFilter;
use crate::tessellation::{MeshableShape, MeshedShape};
use truck_modeling::*;
use truck_polymesh::PolygonMesh;
use truck_topology::shell::ShellCondition;

type CompressedFixture = truck_topology::compress::CompressedSolid<Point3, Curve, Surface>;

const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/shape/");

/// The leftover undirected boundary edges of a polygon, as index chains, with
/// the convention of `Topology::extract_boundaries` (a boundary edge is one not
/// used by an opposite-oriented pair of triangles).
fn leftover_chains(poly: &PolygonMesh) -> Vec<Vec<usize>> {
    poly.extract_boundaries()
}

/// The leftover undirected boundary edges of a polygon (net usage nonzero), as
/// sorted (lo, hi) pairs.
fn undirected_boundary_edges(poly: &PolygonMesh) -> Vec<(usize, usize)> {
    use rustc_hash::FxHashMap as HashMap;
    let mut use_count: HashMap<(usize, usize), i32> = HashMap::default();
    for face in poly.faces().face_iter() {
        let len = face.len();
        for i in 0..len {
            let a = face[i].pos;
            let b = face[(i + 1) % len].pos;
            let key = if a < b { (a, b) } else { (b, a) };
            let sign = if a < b { 1 } else { -1 };
            *use_count.entry(key).or_insert(0) += sign;
        }
    }
    let mut boundary: Vec<(usize, usize)> = use_count
        .into_iter()
        .filter(|(_, count)| *count != 0)
        .map(|(key, _)| key)
        .collect();
    boundary.sort();
    boundary
}

fn report_seam(prefix: &str, name: &str, mut poly: PolygonMesh, weld_tol: f64) {
    let pre_edges = undirected_boundary_edges(&poly);
    let pre_chains = leftover_chains(&poly);
    eprintln!(
        "SEAM {prefix} {name} pre_weld_boundary_edges={} chains={} positions={}",
        pre_edges.len(),
        pre_chains.len(),
        poly.positions().len(),
    );
    poly.put_together_same_attrs(weld_tol)
        .remove_degenerate_faces()
        .remove_unused_attrs();
    let post_edges = undirected_boundary_edges(&poly);
    let condition = poly.shell_condition();
    eprintln!(
        "SEAM {prefix} {name} post_weld_boundary_edges={} positions={} condition={condition:?}",
        post_edges.len(),
        poly.positions().len(),
    );
}

fn compressed_fixture(name: &str) -> CompressedFixture {
    let bytes = std::fs::read(format!("{FIXTURE_DIR}{name}")).unwrap();
    serde_json::from_slice(bytes.as_slice()).unwrap()
}

/// The seam-dump probe the packet names: on `torus.json`, the pre-weld leftover
/// boundary must reduce to zero after the weld (the merged positions are the
/// identical shared-edge samples, so welding closes the shell exactly).
#[test]
fn seam_dump_torus_closes_after_weld() {
    let fixture: CompressedFixture = compressed_fixture("torus.json");
    let mut poly = fixture.triangulation(0.01).to_polygon();
    let pre_edges = undirected_boundary_edges(&poly);
    assert!(!pre_edges.is_empty(), "torus pre-weld seam edges expected");
    poly.put_together_same_attrs(TOLERANCE * 2.0)
        .remove_degenerate_faces()
        .remove_unused_attrs();
    let post_edges = undirected_boundary_edges(&poly);
    assert_eq!(
        post_edges.len(),
        0,
        "torus seam edges must vanish after the weld; pre-weld had {}",
        pre_edges.len()
    );
    let condition = poly.shell_condition();
    assert_eq!(condition, ShellCondition::Closed);
}

fn special_cylinder_model() -> Shell {
    let v0 = builder::vertex(Point3::new(0.0, 1.0, 0.0));
    let v1 = builder::vertex(Point3::new(0.0, 1.0, 1.0));
    let v2 = builder::vertex(Point3::new(0.0, -1.0, 0.0));
    let v3 = builder::vertex(Point3::new(0.0, -1.0, 1.0));

    let edge0 = builder::line(&v0, &v1);
    let edge1 = builder::line(&v2, &v3);
    let edge2 = builder::circle_arc(&v0, &v2, Point3::new(-1.0, 0.0, 0.0));
    let edge3 = builder::circle_arc(&v2, &v0, Point3::new(1.0, 0.0, 0.0));
    let edge4 = builder::circle_arc(&v1, &v3, Point3::new(-1.0, 0.0, 1.0));
    let edge5 = builder::circle_arc(&v3, &v1, Point3::new(1.0, 0.0, 1.0));

    let face0 =
        builder::try_attach_plane(&[vec![edge2.inverse(), edge3.inverse()].into()]).unwrap();
    let face1 = builder::try_attach_plane(&[vec![edge4.clone(), edge5.clone()].into()]).unwrap();

    let surface_row = RevolutedCurve::<Curve>::by_revolution(
        Line(Point3::new(1.0, 0.0, 1.0), Point3::new(1.0, 0.0, 0.0)).into(),
        Point3::origin(),
        Vector3::unit_z(),
    );
    let surface: Surface = Surface::RevolutedCurve(surface_row);

    let face2 = Face::new(
        vec![vec![edge2, edge1.clone(), edge4.inverse(), edge0.inverse()].into()],
        surface.clone(),
    );
    let face3 = Face::new(
        vec![vec![edge3, edge0, edge5.inverse(), edge1.inverse()].into()],
        surface,
    );

    vec![face0, face1, face2, face3].into()
}

/// The analytic-cap-on-cylinder fixture the vendored `special_cylinder` tests
/// use: its cap/wall seams must close after the weld.
#[test]
fn seam_dump_special_cylinder_closes_after_weld() {
    let shell = special_cylinder_model();
    let meshed = shell.triangulation(0.01);
    let mut poly = meshed.to_polygon();
    report_seam("shell", "special_cylinder", poly.clone(), TOLERANCE);
    poly.put_together_same_attrs(TOLERANCE)
        .remove_degenerate_faces()
        .remove_unused_attrs();
    assert_eq!(undirected_boundary_edges(&poly).len(), 0);
    assert_eq!(poly.shell_condition(), ShellCondition::Closed);

    let csolid = special_cylinder_model().compress();
    let meshed = csolid.triangulation(0.01);
    let mut poly = meshed.to_polygon();
    report_seam("csolid", "special_cylinder", poly.clone(), TOLERANCE);
    poly.put_together_same_attrs(TOLERANCE)
        .remove_degenerate_faces()
        .remove_unused_attrs();
    assert_eq!(undirected_boundary_edges(&poly).len(), 0);
    assert_eq!(poly.shell_condition(), ShellCondition::Closed);
}

/// Diagnostic cross-check on the closed analytic fixtures, kept as a readable
/// report rather than an assertion (the vendored `solid_is_closed` /
/// `csolid_is_closed` tests are the authority).
#[test]
fn seam_dump_report_analytic_fixtures() {
    for name in [
        "bottle.json",
        "punched-cube.json",
        "torus-punched-cube.json",
        "sphere.json",
        "torus.json",
    ] {
        let fixture: Solid = {
            let bytes = std::fs::read(format!("{FIXTURE_DIR}{name}")).unwrap();
            serde_json::from_slice(bytes.as_slice()).unwrap()
        };
        let poly = fixture.triangulation(0.01).to_polygon();
        report_seam("solid", name, poly, TOLERANCE * 2.0);
        let fixture: CompressedFixture = {
            let bytes = std::fs::read(format!("{FIXTURE_DIR}{name}")).unwrap();
            serde_json::from_slice(bytes.as_slice()).unwrap()
        };
        let poly = fixture.triangulation(0.01).to_polygon();
        report_seam("csolid", name, poly, TOLERANCE * 2.0);
    }
}
