//! Diagnostic: per-face STEP tessellation timing through the exact production
//! path (policy-wrapped single-face shell + `robust_triangulation_with_torus_outcome`).
//!
//! Every face is timed sequentially with a BEGIN/END ledger so a face that
//! never returns is identified by its last unmatched BEGIN. The production
//! tolerance (model diagonal including edge samples) and the production
//! closures (lattice, schema, cylinder, cone, torus) are replicated exactly;
//! only the parallelism is removed so cost lands on the face that caused it.
//!
//! ```console
//! cargo run --release --example step_face_timing -- MODEL.step [--faces id1,id2,..] [--budget SECS] [--top N]
//! ```

use std::{collections::HashMap, env, path::PathBuf, time::Instant};

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{
    Table,
    step_geometry::{Curve3D, ElementarySurface, Surface},
};
use truck_topology::compress::{CompressedEdgeIndex, CompressedFace, CompressedShell};

use look::step::cone;
use look::step::cylinder;
use look::step::lattice;
use look::step::meshing_policy::MeshingPolicy;
use look::step::policy_geometry::{self, PolicyCurve, PolicySurface};
use look::step::torus_deck;

type Cshell = CompressedShell<Point3, Curve3D, Surface>;

const EDGE_SAMPLES: u32 = 4;
const MINIMUM_TOLERANCE: f64 = 1.0e-6;
const DEGENERATE_TOLERANCE: f64 = 1.0e-3;

fn surface_kind(surface: &Surface) -> &'static str {
    match surface {
        Surface::ElementarySurface(elementary) => match elementary {
            ElementarySurface::Plane(_) => "plane",
            ElementarySurface::Sphere(_) => "sphere",
            ElementarySurface::CylindricalSurface(_) => "cylinder",
            ElementarySurface::ToroidalSurface(_) => "torus",
            ElementarySurface::ConicalSurface(_) => "cone",
            ElementarySurface::DegenerateToroidalSurface(_) => "torus_degen",
        },
        Surface::SweptCurve(_) => "swept",
        Surface::BSplineSurface(_) => "bspline",
        Surface::NurbsSurface(_) => "nurbs",
        Surface::OffsetSurface(_) => "offset",
    }
}

/// A shell holding one face and only the edges that face uses, so tessellating
/// it exercises the ordinary production per-face path without re-tessellating
/// the rest of the shell's edges (which production does once, in parallel).
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
        vertices: shell.vertices.clone(),
        edges,
        faces: vec![CompressedFace {
            boundaries,
            orientation: face.orientation,
            surface: face.surface.clone(),
            provenance: face.provenance,
        }],
        source_geometric_uncertainty: shell.source_geometric_uncertainty,
    }
}

fn model_tolerance(shells: &[Cshell], policy: MeshingPolicy) -> f64 {
    let mut model = BoundingBox::<Point3>::new();
    for shell in shells {
        for vertex in &shell.vertices {
            model.push(*vertex);
        }
        for edge in &shell.edges {
            let (start, end) = edge.curve.range_tuple();
            for step in 0..=EDGE_SAMPLES {
                let t = start + (end - start) * f64::from(step) / f64::from(EDGE_SAMPLES);
                model.push(edge.curve.subs(t));
            }
        }
    }
    let scaled = model.diameter() * policy.relative_linear_deflection;
    if scaled.is_finite() && scaled > 0.0 {
        scaled.max(MINIMUM_TOLERANCE)
    } else {
        DEGENERATE_TOLERANCE
    }
}

/// Tessellate one face through the exact production entry point. Returns
/// (elapsed_ms, outcome, triangle count, surface kind, bound count).
fn time_face(
    shell: &Cshell,
    index: usize,
    tolerance: f64,
    policy: MeshingPolicy,
    closure_map: &HashMap<u64, look::step::lattice::SplineAxisClosure>,
) -> (f64, String, usize, &'static str, usize) {
    let one = single_face_shell(shell, index);
    let kind = surface_kind(&shell.faces[index].surface);
    let bounds = shell.faces[index].boundaries.len();
    let wrapped = policy_geometry::wrap_shell_with_closure(one, policy, closure_map);
    let started = Instant::now();
    let outcome = wrapped.robust_triangulation_with_torus_outcome(
        tolerance,
        |s: &PolicySurface| lattice::lattice_of_with_closure(s.inner(), s.source_closure()),
        |s: &PolicySurface| lattice::support_schema_of(s.inner()),
        |c: &PolicyCurve| lattice::curve_schema_of(c.inner()),
        |s: &PolicySurface| cylinder::identify_source_cylinder_opt(s.inner()),
        |c: &PolicyCurve| lattice::cylinder_curve_schema_of(c.inner()),
        |c: &PolicyCurve| lattice::cylinder_curve_family_of(c.inner()),
        |s: &PolicySurface| cone::identify_source_cone_opt(s.inner()),
        |s: &PolicySurface| torus_deck::identify_source_torus_opt(s.inner()),
    );
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    let (outcome, tris) = match outcome.face_failures.into_iter().next().flatten() {
        Some(failure) => (format!("Failed {:?}", failure.reason), 0),
        None => {
            let tris = outcome
                .shell
                .faces
                .first()
                .and_then(|f| f.surface.as_ref())
                .map(|m| m.tri_faces().len())
                .unwrap_or(0);
            ("Rendered".to_string(), tris)
        }
    };
    (elapsed_ms, outcome, tris, kind, bounds)
}

/// As [`time_face`], additionally reporting the meshed face's world-space
/// bounding box so a targeted correctness check can confirm the mesh covers
/// the face's own region rather than an artificial closure area.
fn time_face_extent(
    shell: &Cshell,
    index: usize,
    tolerance: f64,
    policy: MeshingPolicy,
    closure_map: &HashMap<u64, look::step::lattice::SplineAxisClosure>,
) -> (f64, String, usize, &'static str, usize, Option<(f64, f64, f64, f64, f64, f64)>) {
    let one = single_face_shell(shell, index);
    let kind = surface_kind(&shell.faces[index].surface);
    let bounds = shell.faces[index].boundaries.len();
    let wrapped = policy_geometry::wrap_shell_with_closure(one, policy, closure_map);
    let started = Instant::now();
    let outcome = wrapped.robust_triangulation_with_torus_outcome(
        tolerance,
        |s: &PolicySurface| lattice::lattice_of_with_closure(s.inner(), s.source_closure()),
        |s: &PolicySurface| lattice::support_schema_of(s.inner()),
        |c: &PolicyCurve| lattice::curve_schema_of(c.inner()),
        |s: &PolicySurface| cylinder::identify_source_cylinder_opt(s.inner()),
        |c: &PolicyCurve| lattice::cylinder_curve_schema_of(c.inner()),
        |c: &PolicyCurve| lattice::cylinder_curve_family_of(c.inner()),
        |s: &PolicySurface| cone::identify_source_cone_opt(s.inner()),
        |s: &PolicySurface| torus_deck::identify_source_torus_opt(s.inner()),
    );
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    let (outcome, tris, extent) = match outcome.face_failures.into_iter().next().flatten() {
        Some(failure) => (format!("Failed {:?}", failure.reason), 0, None),
        None => {
            let mesh = outcome.shell.faces.first().and_then(|f| f.surface.as_ref());
            let tris = mesh.map(|m| m.tri_faces().len()).unwrap_or(0);
            let extent = mesh.map(|m| {
                let pts = m.positions();
                let mut lo = [f64::INFINITY; 3];
                let mut hi = [f64::NEG_INFINITY; 3];
                for p in pts {
                    lo[0] = lo[0].min(p.x);
                    hi[0] = hi[0].max(p.x);
                    lo[1] = lo[1].min(p.y);
                    hi[1] = hi[1].max(p.y);
                    lo[2] = lo[2].min(p.z);
                    hi[2] = hi[2].max(p.z);
                }
                (lo[0], lo[1], lo[2], hi[0], hi[1], hi[2])
            });
            ("Rendered".to_string(), tris, extent)
        }
    };
    (elapsed_ms, outcome, tris, kind, bounds, extent)
}

/// Report the interior sampling-grid dimensions the tessellator would build
/// for one face (`parameter_division` over the boundary's UV bounding box,
/// matching `insert_surface`), without tessellating it. `udiv × vdiv` is the
/// grid-cell count, one interior vertex (and one constraint edge pair) per
/// cell.
fn probe_grid(
    face_id: u64,
    shell: &Cshell,
    index: usize,
    tolerance: f64,
    policy: MeshingPolicy,
    closure_map: &HashMap<u64, look::step::lattice::SplineAxisClosure>,
) {
    use truck_geometry::prelude::ParameterDivision2D;
    let one = single_face_shell(shell, index);
    let wrapped = policy_geometry::wrap_shell_with_closure(one, policy, closure_map);
    let surface = &wrapped.faces[0].surface;
    // Project every boundary sample to UV (same nearest-parameter oracle the
    // tessellator's `by_search_nearest_parameter` uses) and take the bbox, as
    // `insert_surface` does.
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for edge in &wrapped.edges {
        let curve = edge.curve.inner();
        let poly = PolylineCurve::from_curve(curve, curve.range_tuple(), tolerance);
        for pt in poly.0.iter() {
            if let Some((u, v)) = surface.search_nearest_parameter(*pt, None, 60) {
                lo[0] = lo[0].min(u);
                hi[0] = hi[0].max(u);
                lo[1] = lo[1].min(v);
                hi[1] = hi[1].max(v);
            }
        }
    }
    if !lo[0].is_finite() {
        eprintln!("GRID\tface={face_id}\tNO_UV");
        return;
    }
    let range = ((lo[0], hi[0]), (lo[1], hi[1]));
    let (udiv, vdiv) = surface.parameter_division(range, tolerance);
    eprintln!(
        "GRID\tface={face_id}\turange=({:.4},{:.4})\tvrange=({:.4},{:.4})\tudiv={}\tvdiv={}\tcells={}",
        lo[0],
        hi[0],
        lo[1],
        hi[1],
        udiv.len(),
        vdiv.len(),
        udiv.len().saturating_mul(vdiv.len())
    );
}

/// Scan one face's boundary edges: per-edge polyline length (the `B` that
/// `PolyBoundary::include` and `PolyBoundary::locate` walk) and total points.
fn scan_edges(face_id: u64, shell: &Cshell, index: usize, tolerance: f64) {
    let face = &shell.faces[index];
    let mut total_points = 0usize;
    let mut total_edges = 0usize;
    let mut per_edge = Vec::new();
    for (wire_idx, wire) in face.boundaries.iter().enumerate() {
        let mut wire_points = 0usize;
        for edge in wire {
            let Some(entry) = shell.edges.get(edge.index) else {
                eprintln!("EDGESCAN\tface={face_id}\twire={wire_idx}\tdangling_ref");
                continue;
            };
            let curve = &entry.curve;
            let range = curve.range_tuple();
            let poly = PolylineCurve::from_curve(curve, range, tolerance);
            let len = poly.len();
            total_points += len;
            wire_points += len;
            total_edges += 1;
            per_edge.push((wire_idx, len));
        }
        eprintln!(
            "EDGESCAN\tface={face_id}\twire={wire_idx}\tedges={}\twire_points={wire_points}",
            total_edges
        );
    }
    eprintln!(
        "EDGESCAN_TOTAL\tface={face_id}\tedges={total_edges}\tboundary_points={total_points}"
    );
    for (wire_idx, len) in per_edge.iter().take(32) {
        eprintln!("EDGESCAN_POINT\tface={face_id}\twire={wire_idx}\tpoints={len}");
    }
}

/// Replicate the per-edge source traversal + sampling the tessellator performs
/// (production `tessellate_edge`), timing each step. This isolates the edge
/// phase from the face phase: if an edge hangs here, it hangs the face.
fn trace_edges(face_id: u64, shell: &Cshell, index: usize, tolerance: f64) {
    use truck_meshalgo::tessellation::source_edge::{self, SourceEdgeTraversal};
    let source_tolerance = shell
        .source_geometric_uncertainty
        .filter(|u| u.is_finite() && *u > 0.0)
        .unwrap_or(source_edge::SOURCE_INCIDENCE_TOLERANCE);
    let face = &shell.faces[index];
    for (wire_idx, wire) in face.boundaries.iter().enumerate() {
        for (use_idx, edge_idx) in wire.iter().enumerate() {
            let Some(entry) = shell.edges.get(edge_idx.index) else {
                continue;
            };
            let curve = &entry.curve;
            let (start_pos, end_pos) = match (
                shell.vertices.get(entry.vertices.0),
                shell.vertices.get(entry.vertices.1),
            ) {
                (Some(s), Some(e)) => (s, e),
                _ => {
                    eprintln!(
                        "EDGETRACE\tface={face_id}\twire={wire_idx}\tuse={use_idx}\tMISSING_VERTEX"
                    );
                    continue;
                }
            };
            let t0 = Instant::now();
            let traversal = source_edge::establish_source_edge_traversal(
                curve,
                start_pos,
                end_pos,
                entry.vertices.0 == entry.vertices.1,
                source_tolerance,
                tolerance,
            );
            let est_ms = t0.elapsed().as_secs_f64() * 1000.0;
            let (points, samp_ms, tag) = match &traversal {
                SourceEdgeTraversal::CanonicalByEvalRange { range } => {
                    let t1 = Instant::now();
                    let poly = PolylineCurve::from_curve(curve, *range, tolerance);
                    (poly.len() as usize, t1.elapsed().as_secs_f64() * 1000.0, "eval_range")
                }
                SourceEdgeTraversal::CanonicalBySourceInterval { traversal, .. } => {
                    let t1 = Instant::now();
                    let poly = source_edge::sample_traversal(curve, traversal, tolerance);
                    (poly.len() as usize, t1.elapsed().as_secs_f64() * 1000.0, "source_interval")
                }
                SourceEdgeTraversal::Unresolved { reason } => (0, 0.0, *reason),
            };
            eprintln!(
                "EDGETRACE\tface={face_id}\twire={wire_idx}\tuse={use_idx}\test_ms={est_ms:.3}\tsamp_ms={samp_ms:.3}\tpoints={points}\ttag={tag}"
            );
        }
    }
}

/// Replicate the one-open-piece closure `PolyBoundary::new_with_join` performs
/// and measure every `polyline_on_surface` walk (open-piece endpoint to each
/// declared-range rectangle corner, and the rectangle edges) in step count and
/// time at the mesh tolerance. A walk across a wide declared range explodes;
/// this identifies which one.
fn probe_pcurve(
    face_id: u64,
    shell: &Cshell,
    index: usize,
    tolerance: f64,
    policy: MeshingPolicy,
    closure_map: &HashMap<u64, look::step::lattice::SplineAxisClosure>,
) {
    use truck_geometry::prelude::*;
    let one = single_face_shell(shell, index);
    let wrapped = policy_geometry::wrap_shell_with_closure(one, policy, closure_map);
    let surface = &wrapped.faces[0].surface;
    let (urange, vrange) = surface.try_range_tuple();
    eprintln!(
        "PCURVE\tface={face_id}\tdeclared_urange={urange:?}\tdeclared_vrange={vrange:?}"
    );
    let (Some((u0, u1)), Some((v0, v1))) = (urange, vrange) else {
        eprintln!("PCURVE\tface={face_id}\tNO_RECT");
        return;
    };
    let walk = |from: (f64, f64), to: (f64, f64)| -> (usize, f64) {
        let line = Line(
            truck_geometry::prelude::Point2::new(from.0, from.1),
            truck_geometry::prelude::Point2::new(to.0, to.1),
        );
        let t0 = Instant::now();
        let pcurve = PCurve::new(line, surface);
        let (div, _) = pcurve.parameter_division(pcurve.range_tuple(), tolerance);
        (div.len(), t0.elapsed().as_secs_f64() * 1000.0)
    };
    // Project the boundary polyline to UV (as the boundary construction does).
    let mut all: Vec<(f64, f64)> = Vec::new();
    for edge in &wrapped.edges {
        let curve = edge.curve.inner();
        let poly = PolylineCurve::from_curve(curve, curve.range_tuple(), tolerance);
        for pt in poly.0.iter() {
            if let Some((u, v)) = surface.search_nearest_parameter(*pt, None, 60) {
                all.push((u, v));
            }
        }
    }
    let corners = [
        (u0, v0),
        (u1, v0),
        (u1, v1),
        (u0, v1),
    ];
    for (i, from) in all.iter().enumerate().step_by(4).take(16) {
        let (steps, ms) = walk(*from, corners[0]);
        if steps > 4 {
            eprintln!(
                "PCURVE\tface={face_id}\tfrom_pt={i}->corner00\tsteps={steps}\tms={ms:.2}"
            );
        }
    }
    // Rectangle-edge walks, in both directions, plus endpoint-to-each-corner.
    for (name, from, to) in [
        ("edge_u0", (u0, v0), (u0, v1)),
        ("edge_u1", (u1, v0), (u1, v1)),
        ("edge_v0", (u0, v0), (u1, v0)),
        ("edge_v1", (u0, v1), (u1, v1)),
    ] {
        let (steps, ms) = walk(from, to);
        eprintln!("PCURVE\tface={face_id}\t{name}\tsteps={steps}\tms={ms:.2}");
    }
    // The open piece's endpoints: the pair across the big seam gap.
    let mut best = (0usize, 0.0f64);
    for (from, to) in [(all[0], *all.last().unwrap()), (*all.last().unwrap(), all[0])] {
        let (steps, ms) = walk(from, to);
        if steps > best.0 {
            best = (steps, ms);
        }
    }
    eprintln!(
        "PCURVE\tface={face_id}\tseam_walk\tsteps={}\tms={:.2}",
        best.0, best.1
    );
}

fn main() -> anyhow::Result<()> {
    let mut models = Vec::<PathBuf>::new();
    let mut target_ids: Vec<u64> = Vec::new();
    let mut skip_ids: Vec<u64> = Vec::new();
    let mut budget_s = 60u64;
    let mut top = 20usize;
    let mut tol_scale = 1.0f64;
    let mut grid_only = false;
    let mut edge_scan = false;
    let mut trace_edges_flag = false;
    let mut pcurve_flag = false;
    let mut assy_flag = false;
    let mut surface_census = false;
    let mut extent_flag = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--faces" => {
                let raw = args.next().unwrap_or_default();
                target_ids = raw
                    .split(',')
                    .filter_map(|s| s.trim().parse::<u64>().ok())
                    .collect();
            }
            "--skip" => {
                let raw = args.next().unwrap_or_default();
                skip_ids = raw
                    .split(',')
                    .filter_map(|s| s.trim().parse::<u64>().ok())
                    .collect();
            }
            "--budget" => budget_s = args.next().unwrap_or_default().parse()?,
            "--top" => top = args.next().unwrap_or_default().parse()?,
            "--tol-scale" => tol_scale = args.next().unwrap_or_default().parse()?,
            "--grid" => grid_only = true,
            "--edge-scan" => edge_scan = true,
            "--edge-trace" => trace_edges_flag = true,
            "--pcurve" => pcurve_flag = true,
            "--assy" => assy_flag = true,
            "--surface-census" => surface_census = true,
            "--extent" => extent_flag = true,
            other => models.push(PathBuf::from(other)),
        }
    }
    if models.is_empty() {
        anyhow::bail!("usage: step_face_timing MODEL.step [--faces id1,id2,..] [--budget SECS] [--top N]");
    }

    let model = &models[0];
    eprintln!("MODEL\t{}", model.display());
    let bytes = std::fs::read(model)?;
    eprintln!("SIZE_MB\t{:.1}", bytes.len() as f64 / 1e6);

    let text = std::str::from_utf8(&bytes)
        .map(std::borrow::Cow::Borrowed)
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());

    let t0 = Instant::now();
    let mut exchange = match look::step::part21::parse(&text) {
        Ok(exchange) => exchange,
        Err(_) => ruststep::parser::parse(&text)
            .map_err(|error| anyhow::anyhow!("failed to parse STEP: {error}"))?,
    };
    eprintln!("PARSE_MS\t{:.1}", t0.elapsed().as_secs_f64() * 1000.0);
    let section = exchange.data.swap_remove(0);

    let t0 = Instant::now();
    let table = Table::from_owned_data_section(section);
    eprintln!("TABLE_MS\t{:.1}", t0.elapsed().as_secs_f64() * 1000.0);

    let closure_map = lattice::spline_closure_map(&table);
    let policy = MeshingPolicy::DEFAULT;

    if surface_census {
        use truck_stepio::r#in::ruststep::ast::Name;
        use truck_stepio::r#in::ruststep::tables::PlaceHolder;
        let surface_kind_of = |entity: u64| -> &'static str {
            if table.plane.contains_key(&entity) {
                "PLANE"
            } else if table.spherical_surface.contains_key(&entity) {
                "SPHERICAL_SURFACE"
            } else if table.cylindrical_surface.contains_key(&entity) {
                "CYLINDRICAL_SURFACE"
            } else if table.toroidal_surface.contains_key(&entity) {
                "TOROIDAL_SURFACE"
            } else if table.conical_surface.contains_key(&entity) {
                "CONICAL_SURFACE"
            } else if table.b_spline_surface_with_knots.contains_key(&entity) {
                "B_SPLINE_SURFACE_WITH_KNOTS"
            } else if table.bezier_surface.contains_key(&entity) {
                "BEZIER_SURFACE"
            } else if table.quasi_uniform_surface.contains_key(&entity) {
                "QUASI_UNIFORM_SURFACE"
            } else if table.uniform_surface.contains_key(&entity) {
                "UNIFORM_SURFACE"
            } else if table.rational_b_spline_surface.contains_key(&entity) {
                "RATIONAL_B_SPLINE_SURFACE"
            } else if table.offset_surface.contains_key(&entity) {
                "OFFSET_SURFACE"
            } else if table.surface_of_linear_extrusion.contains_key(&entity) {
                "SURFACE_OF_LINEAR_EXTRUSION"
            } else if table.surface_of_revolution.contains_key(&entity) {
                "SURFACE_OF_REVOLUTION"
            } else {
                "OTHER"
            }
        };
        let mut lost = 0usize;
        for (&shell_id, shell) in table.shell.iter() {
            let (converted, losses) =
                match table.to_compressed_shell_with_losses(shell_id, shell) {
                    Ok(res) => res,
                    Err(_) => continue,
                };
            for loss in &losses {
                let face_def = loss.provenance.definition_id.map(|v| v.get());
                let kind = face_def
                    .and_then(|id| table.face_surface.get(&id))
                    .and_then(|fs| match &fs.face_geometry {
                        PlaceHolder::Ref(Name::Entity(idx)) => Some(*idx),
                        _ => None,
                    })
                    .map(|surface_entity| surface_kind_of(surface_entity))
                    .unwrap_or("UNRESOLVED");
                lost += 1;
                let surface_entity = face_def
                    .and_then(|id| table.face_surface.get(&id))
                    .and_then(|fs| match &fs.face_geometry {
                        PlaceHolder::Ref(Name::Entity(idx)) => Some(*idx),
                        _ => None,
                    });
                eprintln!(
                    "LOST\tface={}\tshell={shell_id}\ttag={}\tsurface_kind={kind}\tsurface_entity={}",
                    loss.provenance
                        .best_id()
                        .map(|v| v.get())
                        .unwrap_or(0),
                    loss.reason.tag(),
                    surface_entity.map(|e| e.to_string()).unwrap_or_else(|| "none".into())
                );
            }
        }
        eprintln!("LOST_TOTAL\t{lost}");
        return Ok(());
    }

    if assy_flag {
        use truck_assembly::assy::*;
        use truck_stepio::common::PartAttrs;
        use truck_stepio::r#in::convert::{NodeMatrix, ProductShape};
        let assy = table
            .step_assy()
            .map_err(|error| anyhow::anyhow!("failed to build the STEP assembly graph: {error}"))?;
        let mapped = assy.map(
            |node: &NodeEntity<Vec<ProductShape>, PartAttrs>| NodeEntity {
                shape: &node.shape,
                attrs: node.attrs.clone(),
            },
            |edge: &EdgeEntity<NodeMatrix, PartAttrs>| EdgeEntity {
                matrix: Matrix4::try_from(&edge.matrix).unwrap(),
                attrs: edge.attrs.clone(),
            },
        );
        for (i, node) in mapped.all_nodes().enumerate() {
            let ids = node
                .shape()
                .iter()
                .filter_map(|shape| match shape {
                    ProductShape::Solid(_, ids) | ProductShape::Shells(_, ids) => Some(ids),
                    ProductShape::Matrix(_) => None,
                })
                .flatten()
                .copied()
                .collect::<Vec<_>>();
            eprintln!(
                "ASSY_NODE\t#{i}\tname='{}'\tshells={:?}",
                node.entity().attrs.name, ids
            );
        }
        return Ok(());
    }

    let t0 = Instant::now();
    let mut shells = Vec::new();
    let mut conversion_losses = 0usize;
    let mut declared = 0usize;
    for (&shell_id, shell) in table.shell.iter() {
        declared += shell.cfs_faces.len();
        match table.to_compressed_shell_with_losses(shell_id, shell) {
            Ok((converted, losses)) => {
                conversion_losses += losses.len();
                shells.push(converted);
            }
            Err(error) => eprintln!("SHELL_CONVERT_ERR\t{error}"),
        }
    }
    eprintln!(
        "CONVERT_MS\t{:.1}\tSHELLS\t{}\tDECLARED\t{declared}\tCONVERSION_LOSSES\t{conversion_losses}",
        t0.elapsed().as_secs_f64() * 1000.0,
        shells.len()
    );

    let tolerance = model_tolerance(&shells, policy) * tol_scale;
    let total_faces: usize = shells.iter().map(|s| s.faces.len()).sum();
    eprintln!("TOLERANCE\t{tolerance:.9}\tTOTAL_FACES\t{total_faces}\tTOL_SCALE\t{tol_scale}");

    let by_id: HashMap<u64, (usize, usize)> = {
        let mut map = HashMap::new();
        for (s, shell) in shells.iter().enumerate() {
            for (f, face) in shell.faces.iter().enumerate() {
                if let Some(id) = face.provenance.best_id() {
                    map.insert(id.get(), (s, f));
                }
            }
        }
        map
    };

    let pass_started = Instant::now();
    let mut records: Vec<(u64, usize, usize, f64, String, usize, &'static str, usize)> = Vec::new();
    let mut exhausted = false;
    let mut faced = 0usize;

    if !target_ids.is_empty() {
        // Targeted mode: time only the requested source face ids, each once.
        for id in &target_ids {
            if pass_started.elapsed().as_secs() >= budget_s {
                exhausted = true;
                break;
            }
            match by_id.get(id) {
                Some(&(s, f)) => {
                    if grid_only {
                        probe_grid(*id, &shells[s], f, tolerance, policy, &closure_map);
                        continue;
                    }
                    if edge_scan {
                        scan_edges(*id, &shells[s], f, tolerance);
                        continue;
                    }
                    if trace_edges_flag {
                        trace_edges(*id, &shells[s], f, tolerance);
                        continue;
                    }
                    if pcurve_flag {
                        probe_pcurve(*id, &shells[s], f, tolerance, policy, &closure_map);
                        continue;
                    }
                    eprintln!("BEGIN\tface={id}\tshell={s}\tidx={f}\tt_ms={:.1}", pass_started.elapsed().as_secs_f64() * 1000.0);
                    if extent_flag {
                        let (ms, outcome, tris, kind, bounds, ext) =
                            time_face_extent(&shells[s], f, tolerance, policy, &closure_map);
                        eprintln!(
                            "END\tface={id}\tshell={s}\tidx={f}\telapsed_ms={ms:.2}\toutcome={outcome}\ttris={tris}\textent={ext:?}"
                        );
                        records.push((*id, s, f, ms, outcome, tris, kind, bounds));
                    } else {
                        let (ms, outcome, tris, kind, bounds) = time_face(&shells[s], f, tolerance, policy, &closure_map);
                        eprintln!("END\tface={id}\tshell={s}\tidx={f}\telapsed_ms={ms:.2}\toutcome={outcome}\ttris={tris}");
                        records.push((*id, s, f, ms, outcome, tris, kind, bounds));
                    }
                }
                None => {
                    eprintln!("BEGIN\tface={id}\tNOT_FOUND");
                    eprintln!("END\tface={id}\tNOT_FOUND");
                }
            }
        }
    } else {
        'outer: for (s, shell) in shells.iter().enumerate() {
            for (f, face) in shell.faces.iter().enumerate() {
                faced += 1;
                if pass_started.elapsed().as_secs() >= budget_s {
                    exhausted = true;
                    break 'outer;
                }
                let id = face.provenance.best_id().map(|v| v.get());
                if let Some(id) = id {
                    if skip_ids.contains(&id) {
                        continue;
                    }
                }
                eprintln!(
                    "BEGIN\tface={id:?}\tshell={s}\tidx={f}\tt_ms={:.1}",
                    pass_started.elapsed().as_secs_f64() * 1000.0
                );
                let (ms, outcome, tris, kind, bounds) = time_face(&shells[s], f, tolerance, policy, &closure_map);
                eprintln!(
                    "END\tface={id:?}\tshell={s}\tidx={f}\telapsed_ms={ms:.2}\toutcome={outcome}\ttris={tris}\tkind={kind}\tbounds={bounds}"
                );
                records.push((
                    id.unwrap_or(u64::MAX),
                    s,
                    f,
                    ms,
                    outcome,
                    tris,
                    kind,
                    bounds,
                ));
            }
        }
    }

    let elapsed = pass_started.elapsed().as_secs_f64();
    eprintln!(
        "PASS_DONE\tfaces_timed={}\telapsed_s={elapsed:.1}\texhausted={exhausted}\tlast_begin_unmatched={}",
        records.len(),
        faced
    );

    let total: f64 = records.iter().map(|r| r.3).sum();
    eprintln!("TOTAL_MS\t{total:.1}\tfaces\t{}", records.len());

    let mut ranked = records.iter().collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    eprintln!("\nTOP {} SLOW FACES", ranked.len().min(top));
    for r in ranked.iter().take(top) {
        eprintln!(
            "FACE\t{:<9} shell={} idx={} kind={:<8} ms={:>10.2} share={:>5.1}% tris={:>6} bounds={} outcome={}",
            r.0,
            r.1,
            r.2,
            r.6,
            r.3,
            100.0 * r.3 / total.max(1e-9),
            r.5,
            r.7,
            r.4
        );
    }

    let mut cumulative = 0.0f64;
    for (rank, r) in ranked.iter().enumerate() {
        cumulative += r.3;
        if cumulative * 2.0 >= total && total > 0.0 {
            eprintln!(
                "HALF_TIME_IN\t{} of {} faces ({:.2}%)",
                rank + 1,
                ranked.len(),
                100.0 * (rank + 1) as f64 / ranked.len() as f64
            );
            break;
        }
    }

    // Per-class rollup. A class mean alone cannot distinguish "uniformly slow"
    // from "fast except for a pathological tail", and those point at different
    // code, so carry the median/p95/max of the per-face distribution too.
    struct KindStat {
        samples: Vec<f64>,
        ms: f64,
        tris: usize,
        failures: usize,
    }
    impl Default for KindStat {
        fn default() -> Self {
            Self { samples: Vec::new(), ms: 0.0, tris: 0, failures: 0 }
        }
    }
    let mut by_kind: HashMap<&'static str, KindStat> = HashMap::new();
    for r in &records {
        let e = by_kind.entry(r.6).or_default();
        e.samples.push(r.3);
        e.ms += r.3;
        e.tris += r.5;
        if r.4 != "Rendered" {
            e.failures += 1;
        }
    }
    let mut kinds = by_kind.into_iter().collect::<Vec<_>>();
    for (_, stat) in kinds.iter_mut() {
        stat.samples
            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    }
    kinds.sort_by(|a, b| b.1.ms.partial_cmp(&a.1.ms).unwrap_or(std::cmp::Ordering::Equal));
    let quantile = |sorted: &[f64], q: f64| -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }
        let idx = ((sorted.len() as f64 - 1.0) * q).round() as usize;
        sorted[idx]
    };
    eprintln!("BY_KIND");
    eprintln!(
        "HEAD\t{:<12} {:>6} {:>11} {:>7} {:>10} {:>10} {:>10} {:>10} {:>10} {:>9} {:>6}",
        "kind", "faces", "total_ms", "share", "us/face", "med_us", "p95_us", "max_us", "tris",
        "us/tri", "fail"
    );
    for (kind, stat) in &kinds {
        let count = stat.samples.len().max(1);
        eprintln!(
            "KIND\t{:<12} {:>6} {:>11.1} {:>6.1}% {:>10.0} {:>10.0} {:>10.0} {:>10.0} {:>10} {:>9.0} {:>6}",
            kind,
            stat.samples.len(),
            stat.ms,
            100.0 * stat.ms / total.max(1e-9),
            stat.ms * 1000.0 / count as f64,
            quantile(&stat.samples, 0.50) * 1000.0,
            quantile(&stat.samples, 0.95) * 1000.0,
            stat.samples.last().copied().unwrap_or(0.0) * 1000.0,
            stat.tris,
            stat.ms * 1.0e6 / (stat.tris.max(1) as f64) / 1000.0,
            stat.failures
        );
    }

    Ok(())
}
