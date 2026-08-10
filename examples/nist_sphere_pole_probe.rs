//! Temporary diagnostic (NIST-RECOVERY P2): trace the boundary lift walk on the
//! sphere-pole faces of nist_18 (`nist_ctc_02_asme1_ap242-e2.stp`, faces
//! #2559 #2562 #2564 #2567) and report, at each exhausted ambiguous step,
//! whether the sample is a pole singularity, what the leaving edge's longitude
//! is (closed-form sphere inverse on a non-pole point of the leaving source
//! edge), and what the already-lifted incoming longitude was.
//!
//! ```console
//! cargo run --release --example nist_sphere_pole_probe -- MODEL.stp
//! ```

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{
    Table,
    step_geometry::{Curve3D, Surface},
};
use truck_topology::compress::CompressedShell;

type Cshell = CompressedShell<Point3, Curve3D, Surface>;

const MAX_REFINEMENTS: usize = 8;
const AMBIGUOUS_STEP_FRACTION: f64 = 0.45;

fn get_mindiff(u: f64, u0: f64, up: f64) -> f64 {
    u + f64::round((u0 - u) / up) * up
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("model path");
    let tol = std::env::var("TRUCK_POLE_PROBE_TOL")
        .ok()
        .and_then(|raw| raw.parse::<f64>().ok())
        .unwrap_or(0.05);
    let targets: Vec<u64> = std::env::var("TRUCK_POLE_PROBE_TARGET")
        .ok()
        .map(|raw| {
            raw.split(',')
                .filter_map(|t| t.trim().parse::<u64>().ok())
                .collect()
        })
        .unwrap_or_else(|| vec![2559, 2562, 2564, 2567]);
    let bytes = std::fs::read(&path)?;
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

    // Compute the production tolerance the same way the census does: model
    // diameter over every converted shell's vertices and edge samples.
    let mut shells: Vec<Cshell> = Vec::new();
    for (&shell_id, shell) in table.shell.iter() {
        if let Ok((cshell, losses)) = table.to_compressed_shell_with_losses(shell_id, shell) {
            let _ = losses;
            shells.push(cshell);
        }
    }
    let tol = match std::env::var("TRUCK_POLE_PROBE_TOL") {
        Ok(raw) => raw.parse::<f64>().unwrap_or(tol),
        Err(_) => {
            let mut model = truck_meshalgo::prelude::BoundingBox::<Point3>::new();
            for cshell in &shells {
                for v in &cshell.vertices {
                    model.push(*v);
                }
                for edge in &cshell.edges {
                    let (a, b) = edge.curve.range_tuple();
                    for i in 0..=4u32 {
                        model.push(edge.curve.subs(a + (b - a) * f64::from(i) / 4.0));
                    }
                }
            }
            let scaled = model.diameter() * 0.001;
            if scaled.is_finite() && scaled > 0.0 {
                scaled.max(1.0e-6)
            } else {
                1.0e-3
            }
        }
    };
    println!("production-style tolerance = {tol:.6e}");

    for cshell in &shells {
        for (idx, face) in cshell.faces.iter().enumerate() {
            let Some(id) = face.provenance.best_id().map(|id| id.get()) else {
                continue;
            };
            if !targets.contains(&id) {
                continue;
            }
            println!("=== FACE #{id} declared_face_index={idx} ===");
            trace_face(cshell, face, id, tol);
        }
    }
    Ok(())
}

fn edge_kind(curve: &Curve3D) -> &'static str {
    match curve {
        Curve3D::BSplineCurve(_) => "bspline",
        Curve3D::NurbsCurve(_) => "nurbs",
        Curve3D::Conic(_) => "conic",
        Curve3D::Line(_) => "line",
        Curve3D::Polyline(_) => "polyline",
        Curve3D::PCurve(_) => "pcurve",
    }
}

fn trace_face(cshell: &Cshell, face: &truck_topology::compress::CompressedFace<Surface>, id: u64, tol: f64) {
    let s = &face.surface;
    println!(
        "surface u_period={:?} v_period={:?} range={:?}",
        s.u_period(),
        s.v_period(),
        s.try_range_tuple()
    );
    println!("surface orientation(subs identity)=");
    println!("  subs(u,v) for a few:");
    let ups = s.u_period();
    let vps = s.v_period();

    // Replicate tessellate_edge polyline sampling.
    let mut polylines: Vec<Vec<Point3>> = Vec::new();
    for (wi, wire) in face.boundaries.iter().enumerate() {
        println!("-- bound {wi}: {} edges", wire.len());
        for (ei, idx_ref) in wire.iter().enumerate() {
            let edge = &cshell.edges[idx_ref.index];
            let curve = &edge.curve;
            let mut range = curve.evaluation_range();
            if edge.vertices.0 == edge.vertices.1 && (range.1 - range.0).abs() < 1e-4 {
                if let Some(period) = curve.period() {
                    if period > 1e-4 {
                        range = (range.0, range.0 + period);
                    }
                }
            }
            let mut poly = truck_meshalgo::rexport_polymesh::PolylineCurve::from_curve(
                curve, range, tol,
            );
            if poly.len() <= 2 && range.1 - range.0 > 1e-4 {
                let mut pts = Vec::new();
                const STEPS: usize = 16;
                for i in 0..=STEPS {
                    let t = range.0 + (i as f64 / STEPS as f64) * (range.1 - range.0);
                    pts.push(curve.subs(t));
                }
                poly = truck_meshalgo::rexport_polymesh::PolylineCurve::from(pts);
            }
            println!(
                "  edge[{ei}] idx={} kind={} ori={} range=({:.6},{:.6}) pts={}",
                idx_ref.index,
                edge_kind(curve),
                idx_ref.orientation,
                range.0,
                range.1,
                poly.len()
            );
            // Production (`create_boundary`): `orientation == false` uses the
            // curve inverted (`curve.inverse()`), `orientation == true` uses it
            // as stored.
            let pts: Vec<Point3> = if idx_ref.orientation {
                poly.into_iter().collect()
            } else {
                let mut pts: Vec<Point3> = poly.into_iter().collect();
                pts.reverse();
                pts
            };
            polylines.push(pts);
        }
    }

    // Replicate PolyBoundaryPiece::try_new boundary point assembly.
    let mut bdry3d: Vec<Point3> = Vec::new();
    for poly in &polylines {
        if poly.len() == 2 {
            const N: usize = 8;
            for i in 0..N {
                let frac = i as f64 / N as f64;
                bdry3d.push(poly[0] + (poly[1] - poly[0]) * frac);
            }
        } else {
            let n = poly.len().saturating_sub(1);
            bdry3d.extend(poly.iter().take(n).copied());
        }
    }
    if bdry3d.is_empty() {
        println!("  (no boundary points)");
        return;
    }
    bdry3d.push(bdry3d[0]);

    // Replicate the lift walk.
    let proj = |surface: &Surface, pt: Point3, hint: Option<(f64, f64)>| -> Option<(f64, f64)> {
        surface
            .search_parameter(pt, hint, 100)
            .or_else(|| surface.search_parameter(pt, None, 100))
            .or_else(|| surface.search_nearest_parameter(pt, hint, 100))
            .or_else(|| surface.search_nearest_parameter(pt, None, 100))
    };

    let mut previous: Option<(f64, f64)> = None;
    let mut previous_pt: Option<Point3> = None;
    let mut origin: Option<(f64, f64, Point3)> = None;
    let mut steps: Vec<(usize, Point3, Option<(f64, f64)>, bool)> = Vec::new();
    for (bi, point) in bdry3d.iter().enumerate() {
        let mut refinements = 0usize;
        let mut pending: Vec<(Point3, bool)> = vec![(*point, false)];
        while let Some((pt, synthetic)) = pending.pop() {
            let Some((mut u, mut v)) = proj(s, pt, previous) else {
                println!("  [b{bi}] pt=({:.4},{:.4},{:.4}) PROJECTION FAILED", pt.x, pt.y, pt.z);
                continue;
            };
            let raw = (u, v);
            if steps.len() <= 400 {
                println!(
                    "  [b{bi}] pt=({:.6},{:.6},{:.6}) raw=({:.6},{:.6}) prev={previous:?}",
                    pt.x, pt.y, pt.z, raw.0, raw.1
                );
            }
            if let (Some(up), Some((u0, _))) = (ups, previous) {
                u = get_mindiff(u, u0, up);
            }
            if let (Some(vp), Some((_, v0))) = (vps, previous) {
                v = get_mindiff(v, v0, vp);
            }
            if let (Some((u0, v0)), Some(pp)) = (previous, previous_pt) {
                let ambiguous = |now: f64, before: f64, period: Option<f64>| {
                    period.is_some_and(|period| f64::abs(now - before) >= AMBIGUOUS_STEP_FRACTION * period)
                };
                if ambiguous(u, u0, ups) || ambiguous(v, v0, vps) {
                    let pole = singular_pole_check(s, u, v, pt);
                    if steps.len() <= 200 {
                        steps.push((bi, pt, Some((u, v)), synthetic));
                    }
                    if !synthetic && origin.is_none() {
                        origin = Some((u, v, pt));
                    }
                    if !synthetic && steps.len() <= 200 {
                        println!(
                            "  [b{bi}] AMBIG origin raw=({:.6},{:.6}) chosen=({:.6},{:.6}) prev=({:.6},{:.6}) pole={pole:?}",
                            raw.0, raw.1, u, v, u0, v0
                        );
                        if let Some(p) = pole {
                            // leaving-edge longitude via a non-pole point on the
                            // next source sample run.
                            report_leaving_longitude(s, &bdry3d, bi, p);
                        }
                    }
                    if refinements < MAX_REFINEMENTS {
                        refinements += 1;
                        pending.push((pt, synthetic));
                        pending.push((pp.midpoint(pt), true));
                        continue;
                    }
                    println!(
                        "  [b{bi}] EXHAUSTED ambiguous raw=({:.6},{:.6}) chosen=({:.6},{:.6}) prev=({:.6},{:.6}) pole={pole:?}",
                        raw.0, raw.1, u, v, u0, v0
                    );
                    // DESIGN VALIDATION: previous is the pole candidate. Report
                    // the singularity at previous, the incoming longitude (the
                    // sample before the pole in bdry3d), the outgoing longitude
                    // (origin, the leaving edge's first real sample), and the
                    // branch get_mindiff would produce.
                    if let (Some((pu, pv)), Some(pp_pt)) = (previous, previous_pt) {
                        let ud = s.uder(pu, pv);
                        let vd = s.vder(pu, pv);
                        println!(
                            "    PREV singular: uder_mag={:.3e} vder_mag={:.3e} prev_uv=({pu:.6},{pv:.6})",
                            ud.magnitude(),
                            vd.magnitude()
                        );
                        // incoming longitude: sample before the pole in bdry3d.
                        let pole_idx = bdry3d.iter().position(|q| q.distance(pp_pt) < 1e-3);
                        println!("    pole bdry3d idx={pole_idx:?}");
                        if let Some(pi) = pole_idx {
                            if pi > 0 {
                                let prev_sample = bdry3d[pi - 1];
                                if let Some((pu2, pv2)) = proj(s, prev_sample, None) {
                                    println!(
                                        "    incoming sample=({:.4},{:.4},{:.4}) proj=(u={pu2:.6},v={pv2:.6})",
                                        prev_sample.x, prev_sample.y, prev_sample.z
                                    );
                                }
                            }
                        }
                        // outgoing longitude: origin UV.
                        if let Some((ou, ov, o_pt)) = origin {
                            println!(
                                "    origin (leaving first sample) uv=({ou:.6},{ov:.6}) pt=({:.4},{:.4},{:.4})",
                                o_pt.x, o_pt.y, o_pt.z
                            );
                        }
                    }
                    // fall through: record and stop like production.
                    return;
                }
            }
            previous = Some((u, v));
            previous_pt = Some(pt);
        }
    }
    println!("  lift completed: {} lifted points (last {previous:?})", steps.len());
}

/// Detect the pole singularity on the periodic axis. Returns a tag naming the
/// collapsed axis and the sampled singular direction magnitude.
fn singular_pole_check(
    s: &Surface,
    u: f64,
    v: f64,
    pt: Point3,
) -> Option<(&'static str, f64)> {
    let ud = s.uder(u, v);
    let vd = s.vder(u, v);
    // The collapsed axis is the one whose derivative is so_small at the sample.
    let u_small = ud.so_small();
    let v_small = vd.so_small();
    // Also check that the sample really is a pole: the 3D point should be at a
    // longitude-independent location. The world-space radial direction at the
    // sample, compared against the two partials, is the cleanest certificate.
    let res = s.subs(u, v).distance(pt);
    match (u_small, v_small) {
        (true, false) => Some(("u", ud.magnitude())),
        (false, true) => Some(("v", vd.magnitude())),
        (true, true) => Some(("both", ud.magnitude().min(vd.magnitude()))),
        (false, false) => {
            // Not singular at the chosen UV. Still may be a pole if the UV is
            // only approximately at it; report residual.
            if res > 1e-3 {
                Some(("not_singular", res))
            } else {
                None
            }
        }
    }
}

/// The leaving edge's longitude is the longitude of any non-pole point on it.
/// `bi` is the index of the current (pole) boundary point; the leaving edge is
/// the source edge that contains the *next* boundary samples.
fn report_leaving_longitude(s: &Surface, bdry3d: &[Point3], bi: usize, pole: (&'static str, f64)) {
    let next = bdry3d.get(bi + 1).copied();
    let Some(next_pt) = next else {
        println!("    leaving edge: no next sample");
        return;
    };
    let r = s.search_parameter(next_pt, None::<(f64, f64)>, 100);
    match r {
        Some((u, v)) => {
            let res = s.subs(u, v).distance(next_pt);
            let (lu, lv) = if pole.0 == "u" {
                (v, u)
            } else {
                (u, v)
            };
            println!(
                "    leaving lon: next_pt=({:.4},{:.4},{:.4}) proj=(u={u:.6},v={v:.6}) \
                 residual={res:.3e} (collapsed axis {}; longitude axis pair={lu:.6})",
                next_pt.x, next_pt.y, next_pt.z, pole.0
            );
        }
        None => println!("    leaving edge next point projection failed"),
    }
}
