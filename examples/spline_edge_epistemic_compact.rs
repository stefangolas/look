//! Scratch epistemic probe (not for commit): compact per-edge classification
//! for spline/NURBS boundary edges of targeted faces.
//!   Canonical      : evaluation_range() realizes the complete source edge
//!                    (basis genuine throughout, endpoints match source vertices).
//!   SliverDegenerate: declared range extends past evaluation_range; sliver
//!                    evaluates to origin garbage; eval range is canonical.
//!   Ambiguous      : multiple source-consistent realizations remain.
//!   Inconsistent   : no genuinely evaluable traversal satisfies the vertices.
//! Also reports the minimum genuine-loop diameter over the eval range so a
//! sub-tolerance face is distinguishable from a source-inconsistent one.
//! Usage: spline_edge_epistemic_compact MODEL.step --faces id,id,...

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{Table, step_geometry::Curve3D};

fn curve_family(c: &Curve3D) -> &'static str {
    match c {
        Curve3D::BSplineCurve(_) => "bspline",
        Curve3D::NurbsCurve(_) => "nurbs",
        Curve3D::Conic(_) => "conic",
        Curve3D::Line(_) => "line",
        Curve3D::Polyline(_) => "polyline",
        Curve3D::PCurve(_) => "pcurve",
        _ => "other",
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("model path");
    let mut faces: Vec<u64> = Vec::new();
    for i in 2..args.len() {
        if args[i] == "--faces" {
            faces = args[i + 1]
                .split(',')
                .filter_map(|t| t.trim().parse::<u64>().ok())
                .collect();
        }
    }
    let bytes = std::fs::read(path)?;
    let text = std::str::from_utf8(&bytes)
        .map(std::borrow::Cow::Borrowed)
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());
    let mut exchange = match look::step::part21::parse(&text) {
        Ok(exchange) => exchange,
        Err(_) => ruststep::parser::parse(&text)
            .map_err(|e| anyhow::anyhow!("parse failed: {e}"))?,
    };
    let section = exchange.data.swap_remove(0);
    let table = Table::from_owned_data_section(section);

    let mut n_faces = 0usize;
    for (&shell_id, shell) in table.shell.iter() {
        let Ok((cshell, losses)) = table.to_compressed_shell_with_losses(shell_id, shell) else {
            continue;
        };
        let _ = losses;
        for face in &cshell.faces {
            let Some(id) = face.provenance.best_id().map(|id| id.get()) else {
                continue;
            };
            if !faces.contains(&id) {
                continue;
            }
            n_faces += 1;
            let s = &face.surface;
            let fam = format!("{:?}", look::step_support_schema_of(s));
            println!("FACE\t{id}\tfamily={fam}\tbounds={}", face.boundaries.len());
            for (bi, wire) in face.boundaries.iter().enumerate() {
                let mut closed = true;
                let mut first = None;
                let mut prev = None;
                let mut n_spline = 0;
                for (ei, idx_ref) in wire.iter().enumerate() {
                    let edge = &cshell.edges[idx_ref.index];
                    let (va, vb) = edge.vertices;
                    if ei == 0 {
                        first = Some(va);
                    }
                    if let Some(pv) = prev {
                        if pv != va {
                            closed = false;
                        }
                    }
                    prev = Some(vb);
                    let curve = &edge.curve;
                    let family = curve_family(curve);
                    if family != "bspline" && family != "nurbs" {
                        continue;
                    }
                    n_spline += 1;
                    let rt = curve.range_tuple();
                    let er = curve.evaluation_range();
                    let has_sliver = (rt.0 < er.0 - 1e-9) || (rt.1 > er.1 + 1e-9);
                    let c_rt_lo = curve.subs(rt.0);
                    let c_rt_hi = curve.subs(rt.1);
                    let c_er_lo = curve.subs(er.0);
                    let c_er_hi = curve.subs(er.1);
                    let origin_rt = c_rt_lo.distance(Point3::origin()) < 1e-6
                        && c_rt_hi.distance(Point3::origin()) < 1e-6;
                    let pv_a = cshell.vertices[va];
                    let pv_b = cshell.vertices[vb];
                    let d_er_lo = c_er_lo.distance(pv_a);
                    let d_er_hi = c_er_hi.distance(pv_b);
                    let d_rt_lo = c_rt_lo.distance(pv_a);
                    let d_rt_hi = c_rt_hi.distance(pv_b);
                    // genuine loop diameter over the eval range
                    let mut lo = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
                    let mut hi = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
                    for i in 0..=64 {
                        let t = er.0 + (er.1 - er.0) * i as f64 / 64.0;
                        let p = curve.subs(t);
                        lo.x = lo.x.min(p.x);
                        lo.y = lo.y.min(p.y);
                        lo.z = lo.z.min(p.z);
                        hi.x = hi.x.max(p.x);
                        hi.y = hi.y.max(p.y);
                        hi.z = hi.z.max(p.z);
                    }
                    let diag = (hi - lo).magnitude();
                    let closed_loop = va == vb;
                    let er_closed = c_er_lo.distance(c_er_hi) < 1e-6;
                    let class = if !has_sliver && d_er_lo < 1e-6 && d_er_hi < 1e-6 {
                        "Canonical"
                    } else if has_sliver && origin_rt && d_er_lo < 1e-6 && d_er_hi < 1e-6 {
                        "Canonical-sliver"
                    } else if d_er_lo > 1e-3 || d_er_hi > 1e-3 {
                        "Inconsistent"
                    } else {
                        "Other"
                    };
                    println!(
                        "EDGE\tface={id}\tbound={bi}\tidx={}\tori={}\tfamily={family}\t\
                         closed_edge={closed_loop}\ter_closed={er_closed}\t\
                         rt=({:.4},{:.4})\ter=({:.4},{:.4})\tsliver={has_sliver}\t\
                         rt_origin={origin_rt}\tres_er=({d_er_lo:.1e},{d_er_hi:.1e})\t\
                         res_rt=({d_rt_lo:.1e},{d_rt_hi:.1e})\tloop_diag={diag:.4e}\tclass={class}",
                        idx_ref.index,
                        idx_ref.orientation,
                        rt.0,
                        rt.1,
                        er.0,
                        er.1
                    );
                }
                if let Some(fv) = first {
                    if let Some(pv) = prev {
                        if fv != pv {
                            closed = false;
                        }
                    }
                }
                println!("BOUND\tface={id}\tbound={bi}\tn_spline={n_spline}\tcloses={closed}");
            }
        }
    }
    if n_faces == 0 {
        println!("NO TARGET FACES FOUND");
    }
    Ok(())
}
