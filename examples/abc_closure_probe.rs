//! Temporary diagnostic: inspect a closed spline edge of an ABC face and
//! evaluate the curve in the sliver region outside the interior knot span.
//!
//! Usage: `cargo run --release --example abc_closure_probe -- MODEL.step FACE_ID`

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{
    Table,
    step_geometry::{Curve3D, Surface},
};
use truck_topology::compress::{CompressedShell};

type Cshell = CompressedShell<Point3, Curve3D, Surface>;

fn face_surface_kind(surface: &Surface) -> &'static str {
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
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("model path");
    let target: u64 = args.next().map(|s| s.parse().unwrap()).unwrap_or(120193);
    let scan_all = std::env::var_os("ABC_SCAN").is_some();
    let scan_targets: Option<Vec<u64>> = std::env::var_os("ABC_SCAN_IDS").map(|v| {
        v.to_string_lossy()
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect()
    });
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
        let Ok((cshell, _losses)) = table.to_compressed_shell_with_losses(shell_id, shell) else {
            continue;
        };
        for (idx, face) in cshell.faces.iter().enumerate() {
            let Some(id) = face.provenance.best_id().map(|id| id.get()) else {
                continue;
            };
            if id != target {
                if !scan_all
                    && !scan_targets.as_ref().is_some_and(|t| t.contains(&id))
                {
                    continue;
                }
            }
            if scan_all || scan_targets.is_some() {
                // Compact scan mode: one line per closed spline edge.
                for wire in face.boundaries.iter() {
                    for idx_ref in wire.iter() {
                        let curve = &cshell.edges[idx_ref.index].curve;
                        let verts = &cshell.edges[idx_ref.index].vertices;
                        let same_vertex = verts.0 == verts.1;
                        if !same_vertex {
                            continue;
                        }
                        if !matches!(curve, Curve3D::BSplineCurve(_) | Curve3D::NurbsCurve(_)) {
                            continue;
                        }
                        let er = curve.evaluation_range();
                        let (e0, e1) = er;
                        let q0 = curve.subs(e0);
                        let q1 = curve.subs(e1);
                    let rt = curve.range_tuple();
                    let p0 = curve.subs(rt.0);
                    let er = curve.evaluation_range();
                    println!(
                        "     PRED: rt={rt:?} ev={er:?} p.o.u(-0.0625)={} p.o.u(-0.03)={} p.o.u(0)={} p.o.u(1)={} p.o.u(1.03)={} p.o.u(1.0625)={}",
                        curve.basis_is_partition_of_unity(rt.0),
                        curve.basis_is_partition_of_unity(-0.03),
                        curve.basis_is_partition_of_unity(er.0),
                        curve.basis_is_partition_of_unity(er.1),
                        curve.basis_is_partition_of_unity(1.03),
                        curve.basis_is_partition_of_unity(rt.1)
                    );
                        let origin_garbage = p0.to_vec().magnitude() < 1e-6;
                        let sliver = rt != er;
                        println!(
                            "SCAN\tface=#{id}\tkind={}\td01={:.6}\tdrange_ends={:.6}\tsliver_origin={}\trange_neq_eval={sliver}",
                            face_surface_kind(&face.surface),
                            q0.distance(q1),
                            p0.distance(curve.subs(rt.1)),
                            origin_garbage
                        );
                    }
                }
                continue;
            }
            println!("=== FACE #{target} declared_face_index={idx} ===");
            for (bi, wire) in face.boundaries.iter().enumerate() {
                println!("-- bound {bi}: {} edges", wire.len());
                for (ei, idx_ref) in wire.iter().enumerate() {
                    let curve = &cshell.edges[idx_ref.index].curve;
                    let verts = &cshell.edges[idx_ref.index].vertices;
                    let v0 = cshell.vertices[verts.0];
                    let v1 = cshell.vertices[verts.1];
                    let rt = curve.range_tuple();
                    let er = curve.evaluation_range();
                    let same_vertex = verts.0 == verts.1;
                    println!(
                        "   edge idx={} ori={} same_vertex={same_vertex} range={rt:?} eval={er:?}",
                        idx_ref.index, idx_ref.orientation
                    );
                    println!(
                        "     vertex0={:?} vertex1={:?} dist={:.3e}",
                        v0, v1, v0.distance(v1)
                    );
                    let (r0, r1) = rt;
                    let p0 = curve.subs(r0);
                    let p1 = curve.subs(r1);
                    println!(
                        "     subs(range.0)=({:.6},{:.6},{:.6}) |p0|={:.3e} d(v0,p0)={:.3e}",
                        p0.x,
                        p0.y,
                        p0.z,
                        p0.to_vec().magnitude(),
                        v0.distance(p0)
                    );
                    println!(
                        "     subs(range.1)=({:.6},{:.6},{:.6}) |p1|={:.3e} d(v0,p1)={:.3e}",
                        p1.x,
                        p1.y,
                        p1.z,
                        p1.to_vec().magnitude(),
                        v0.distance(p1)
                    );
                    println!(
                        "     d(subs(r0),subs(r1))={:.3e} (closure over full range)",
                        p0.distance(p1)
                    );
                    let (e0, e1) = er;
                    let q0 = curve.subs(e0);
                    let q1 = curve.subs(e1);
                    println!(
                        "     eval ends: subs(eval.0)=({:.6},{:.6},{:.6}) subs(eval.1)=({:.6},{:.6},{:.6}) d={:.3e}",
                        q0.x, q0.y, q0.z, q1.x, q1.y, q1.z, q0.distance(q1)
                    );
                    println!(
                        "     d(subs(eval.0),vertex0)={:.3e} d(subs(eval.1),vertex0)={:.3e}",
                        v0.distance(q0),
                        v0.distance(q1)
                    );
                    // Sample the actual polyline over each domain.
                    for (label, rng) in [("range_tuple", rt), ("evaluation_range", er)] {
                        let poly = truck_polymesh::PolylineCurve::from_curve(curve, rng, 0.001);
                        println!(
                            "     polyline[{label}]: len={} span3d={:.3e}",
                            poly.len(),
                            poly.iter()
                                .skip(1)
                                .zip(poly.iter())
                                .map(|(a, b)| a.distance(*b))
                                .sum::<f64>()
                        );
                        for (i, p) in poly.iter().enumerate() {
                            if label == "range_tuple" && i % 8 == 0 || label == "evaluation_range" {
                                println!("       [{label}][{i}] = ({:.5},{:.5},{:.5})", p.x, p.y, p.z);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
