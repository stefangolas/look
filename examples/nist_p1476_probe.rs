//! Temporary diagnostic: inspect face #1476 of nist_ctc_02_asme1_ap203 and
//! run the exact projection chain on its failing boundary corner point.

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{
    Table,
    step_geometry::{Curve3D, Surface},
};
use truck_topology::compress::{CompressedFace, CompressedShell};

type Cshell = CompressedShell<Point3, Curve3D, Surface>;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("model path");
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

    for (&shell_id, shell) in table.shell.iter() {
        let Ok((cshell, losses)) = table.to_compressed_shell_with_losses(shell_id, shell) else {
            continue;
        };
        for (idx, face) in cshell.faces.iter().enumerate() {
            let Some(id) = face.provenance.best_id().map(|id| id.get()) else {
                continue;
            };
            if id != 1476 {
                continue;
            }
            println!("=== FACE #1476 declared_face_index={idx} ===");
            println!("surface family via match");
            let s = &face.surface;
            println!("surface u_period={:?} v_period={:?}", s.u_period(), s.v_period());
            println!("surface range={:?}", s.try_range_tuple());
            for (bi, wire) in face.boundaries.iter().enumerate() {
                println!("-- bound {bi}: {} edges", wire.len());
                for (ei, idx_ref) in wire.iter().enumerate() {
                    let curve = &cshell.edges[idx_ref.index].curve;
                    let range = curve.range_tuple();
                    println!(
                        "   edge idx={} ori={} range={range:?} eval_range={:?}",
                        idx_ref.index,
                        idx_ref.orientation,
                        curve.evaluation_range()
                    );
                    let kind = match curve {
                        Curve3D::BSplineCurve(_) => "bspline",
                        Curve3D::NurbsCurve(_) => "nurbs",
                        Curve3D::Conic(_) => "conic",
                        Curve3D::Line(_) => "line",
                        Curve3D::Polyline(_) => "polyline",
                        Curve3D::PCurve(_) => "pcurve",
                    };
                    println!("   kind={kind}");
                    let (t0, t1) = curve.range_tuple();
                    let (e0, e1) = curve.evaluation_range();
                    println!(
                        "   subs(t0)={:?} subs(t1)={:?}",
                        curve.subs(t0),
                        curve.subs(t1)
                    );
                    println!(
                        "   subs(e0)={:?} subs(e1)={:?}",
                        curve.subs(e0),
                        curve.subs(e1)
                    );
                }
            }
            // Corner point = the failing first_failed_xyz observed in the
            // production run.
            let corner = Point3::new(0.0, 8.78948499401687, -251.697918515055);
            println!("== projection chain on corner {corner:?} ==");
            let h = |uv: Option<(f64, f64)>| {
                let (u, v) = uv.unwrap_or((0.0, 0.0));
                let r = s.search_parameter(corner, Some((u, v)), 100);
                print_proj("search_parameter(hint)", s, corner, r);
                let r = s.search_parameter(corner, None::<(f64, f64)>, 100);
                print_proj("search_parameter(None)", s, corner, r);
                let r = s.search_nearest_parameter(corner, Some((u, v)), 100);
                print_proj("search_nearest_parameter(hint)", s, corner, r);
                let r = s.search_nearest_parameter(corner, None::<(f64, f64)>, 100);
                print_proj("search_nearest_parameter(None)", s, corner, r);
            };
            h(None);
            // Also: the surface at the corner, and its derivatives.
            println!("subs(0,0)={:?}", s.subs(0.0, 0.0));
            println!("subs(0,0).dist(corner)={:?}", s.subs(0.0, 0.0).distance(corner));
            let (u0, v0) = (0.0f64, 0.0f64);
            println!("uder(0,0)={:?} vder(0,0)={:?}", s.uder(u0, v0), s.vder(u0, v0));
            println!("uder(-0.03125,0.5)={:?}", s.uder(-0.03125, 0.5));
            println!("vder(-0.03125,0.5)={:?}", s.vder(-0.03125, 0.5));
            println!("subs(-0.03125,0.5)={:?}", s.subs(-0.03125, 0.5));

            // Now the full circle polyline bound 1 samples over (2pi, 4pi) and
            // the production 4-link projection chain on each, with a carried
            // hint, to find which sample actually fails.
            println!("== production 4-link chain over circle polyline ==");
            let curve = &cshell.edges[4].curve;
            let poly = truck_meshalgo::rexport_polymesh::PolylineCurve::from_curve(
                curve,
                (2.0 * std::f64::consts::PI, 4.0 * std::f64::consts::PI),
                1.167047556871613,
            );
            let seam = poly[0];
            println!("seam pt={seam:?}");
            // Hypothesis: the surface's declared range extends below the true
            // knot support, so the hintless presearch evaluates the invalid
            // U<0 extension sliver and Newton degenerates there. Test with an
            // explicit presearch over the true support [0,1] x [0,1].
            let r = s.search_parameter(
                seam,
                truck_meshalgo::prelude::SPHint2D::Range((0.0, 1.0), (0.0, 1.0)),
                100,
            );
            print_proj("seam: search_parameter(Range(0..1,0..1))", s, seam, r);
            let r = s.search_parameter(seam, None::<(f64, f64)>, 100);
            print_proj("seam: search_parameter(None)", s, seam, r);
            let r = s.search_parameter(
                seam,
                truck_meshalgo::prelude::SPHint2D::Range((-0.0625, 1.0625), (0.0, 1.0)),
                100,
            );
            print_proj("seam: search_parameter(Range(raw))", s, seam, r);
            // Also test the raw declared-range presearch outcome directly.
            let best = truck_meshalgo::prelude::algo::surface::presearch(
                s,
                seam,
                ((-0.0625, 1.0625), (0.0, 1.0)),
                50,
            );
            println!("raw presearch best cell = {best:?} subs={:?} dist={:?}", s.subs(best.0, best.1), s.subs(best.0, best.1).distance(seam));
            let best2 = truck_meshalgo::prelude::algo::surface::presearch(
                s,
                seam,
                ((0.0, 1.0), (0.0, 1.0)),
                50,
            );
            println!("true presearch best cell = {best2:?} subs={:?} dist={:?}", s.subs(best2.0, best2.1), s.subs(best2.0, best2.1).distance(seam));
            let mut prev: Option<(f64, f64)> = None;
            for (i, p) in poly.iter().enumerate() {
                let out = chain(s, *p, prev);
                let out = match out {
                    Ok((u, v)) => format!("ok uv=({u:.6},{v:.6})"),
                    Err(e) => format!("FAILED: {e}"),
                };
                println!(
                    "sample {i} pt=({:.6},{:.6},{:.6}) -> {}",
                    p.x, p.y, p.z, out
                );
                prev = chain(s, *p, prev).ok();
            }
        }
    }
    Ok(())
}

fn print_proj(
    label: &str,
    s: &Surface,
    point: Point3,
    r: Option<(f64, f64)>,
) {
    match r {
        Some((u, v)) => {
            let res = s.subs(u, v).distance(point);
            println!("{label}: uv=({u:.6},{v:.6}) residual={res:.6e}");
        }
        None => {
            println!("{label}: None (did not converge)");
        }
    }
}

/// The production 4-link projection chain, first success wins, returning the
/// chosen UV or a failure tag.
fn chain(s: &Surface, point: Point3, hint: Option<(f64, f64)>) -> Result<(f64, f64), String> {
    let with_residual = |r: Option<(f64, f64)>| -> Option<(f64, f64, f64)> {
        r.and_then(|(u, v)| {
            let res = s.subs(u, v).distance(point);
            Some((u, v, res))
        })
    };
    let r1 = with_residual(s.search_parameter(point, hint, 100));
    if let Some((u, v, res)) = r1 {
        return Ok((u, v));
    }
    let r2 = with_residual(s.search_parameter(point, None::<(f64, f64)>, 100));
    if let Some((u, v, res)) = r2 {
        return Ok((u, v));
    }
    let r3 = with_residual(s.search_nearest_parameter(point, hint, 100));
    if let Some((u, v, res)) = r3 {
        return Ok((u, v));
    }
    let r4 = with_residual(s.search_nearest_parameter(point, None::<(f64, f64)>, 100));
    if let Some((u, v, res)) = r4 {
        return Ok((u, v));
    }
    Err(format!(
        "all 4 links failed (hint={hint:?})"
    ))
}
