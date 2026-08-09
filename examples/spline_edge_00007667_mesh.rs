//! Scratch probe (Phase 1): tessellate the shell containing a target face and
//! report the rendered mesh extent of the target face, to determine whether the
//! currently-rendered plane face #10428 represents the source sliver (line +
//! wrapped arc) or a full-loop false-positive region.
//! Usage: spline_edge_00007667_mesh MODEL.step --face 10428

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{Table, step_geometry::Curve3D};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("model path");
    let mut face_filter: Option<u64> = None;
    for i in 2..args.len() {
        if args[i] == "--face" {
            face_filter = args[i + 1].parse().ok();
        }
    }
    let face_filter = face_filter.expect("--face ID");
    let bytes = std::fs::read(path)?;
    let text = std::str::from_utf8(&bytes)
        .map(std::borrow::Cow::Borrowed)
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());
    let mut exchange = match look::step::part21::parse(&text) {
        Ok(exchange) => exchange,
        Err(_) => {
            ruststep::parser::parse(&text).map_err(|e| anyhow::anyhow!("parse failed: {e}"))?
        }
    };
    let section = exchange.data.swap_remove(0);
    let table = Table::from_owned_data_section(section);

    for (_, shell) in table.shell.iter() {
        let Ok((cshell, losses)) = table.to_compressed_shell_with_losses(shell) else {
            continue;
        };
        let _ = losses;
        let Some(face_idx) = cshell
            .faces
            .iter()
            .position(|f| f.provenance.best_id().map(|id| id.get()) == Some(face_filter))
        else {
            continue;
        };
        let face = &cshell.faces[face_idx];
        println!(
            "face {face_filter} shell_face_idx={face_idx} family={:?} bounds={}",
            look::step_support_schema_of(&face.surface),
            face.boundaries.len()
        );
        // The source sliver extent: the wrapped arc from t_a to t_b.
        let mut shared: Option<usize> = None;
        for wire in &face.boundaries {
            for idx in wire {
                if cshell.edges[idx.index].curve.range_tuple().0 < -0.4 {
                    shared = Some(idx.index);
                }
            }
        }
        if let Some(si) = shared {
            let edge = &cshell.edges[si];
            let (va, vb) = edge.vertices;
            let (pv_a, pv_b) = (cshell.vertices[va], cshell.vertices[vb]);
            let curve = &edge.curve;
            let er = curve.evaluation_range();
            let (t_a, _) = root_for(curve, er, pv_a);
            let (t_b, _) = root_for(curve, er, pv_b);
            println!("  shared edge idx={si} verts=({va},{vb})");
            println!(
                "  source arc: t_a={t_a:.6} t_b={t_b:.6} span={:.6} wrap={}",
                (if t_a > t_b {
                    t_b + 1.0 - t_a
                } else {
                    t_b - t_a
                }),
                t_a > t_b
            );
            let mut lo = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
            let mut hi = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
            let n = 128;
            for i in 0..=n {
                let tau = t_a
                    + (if t_a > t_b {
                        t_b + 1.0 - t_a
                    } else {
                        t_b - t_a
                    }) * i as f64
                        / n as f64;
                let t = if tau > 1.0 { tau - 1.0 } else { tau };
                let p = curve.subs(t);
                lo.x = lo.x.min(p.x);
                lo.y = lo.y.min(p.y);
                lo.z = lo.z.min(p.z);
                hi.x = hi.x.max(p.x);
                hi.y = hi.y.max(p.y);
                hi.z = hi.z.max(p.z);
            }
            println!(
                "  source arc bbox: [{lo:?}] .. [{hi:?}] diag={:.4e}",
                (hi - lo).magnitude()
            );
        }

        // Tessellate the whole shell with look's production entry point and
        // report the target face's triangle extent.
        let mut model = truck_meshalgo::prelude::BoundingBox::<Point3>::new();
        for v in &cshell.vertices {
            model.push(*v);
        }
        for e in &cshell.edges {
            let (s, e_) = e.curve.range_tuple();
            for step in 0..=4 {
                let t = s + (e_ - s) * step as f64 / 4.0;
                model.push(e.curve.subs(t));
            }
        }
        let diameter = model.diameter();
        let tolerance = (diameter * 0.001).max(1.0e-6);
        println!("  model diameter={diameter:.4} production tolerance={tolerance:.4e}");
        let outcome = truck_meshalgo::tessellation::LatticeMeshableShape::<_, _>::robust_triangulation_with_torus_outcome(
            &look::step::policy_geometry::wrap_shell(
                cshell.clone(),
                look::step::meshing_policy::MeshingPolicy::default(),
            ),
            tolerance,
            |s: &look::step::policy_geometry::PolicySurface| look::step::lattice::lattice_of(s.inner()),
            |s: &look::step::policy_geometry::PolicySurface| look::step::lattice::support_schema_of(s.inner()),
            |c: &look::step::policy_geometry::PolicyCurve| look::step::lattice::curve_schema_of(c.inner()),
            |s: &look::step::policy_geometry::PolicySurface| look::step::cylinder::identify_source_cylinder_opt(s.inner()),
            |c: &look::step::policy_geometry::PolicyCurve| look::step::lattice::cylinder_curve_schema_of(c.inner()),
            |c: &look::step::policy_geometry::PolicyCurve| look::step::lattice::cylinder_curve_family_of(c.inner()),
            |s: &look::step::policy_geometry::PolicySurface| look::step::cone::identify_source_cone_opt(s.inner()),
            |s: &look::step::policy_geometry::PolicySurface| look::step::torus_deck::identify_source_torus_opt(s.inner()),
        );
        let meshed = outcome.shell;
        if let Some(mesh) = &meshed.faces[face_idx].surface {
            let mut lo = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
            let mut hi = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
            for p in mesh.positions().iter() {
                lo.x = lo.x.min(p.x);
                lo.y = lo.y.min(p.y);
                lo.z = lo.z.min(p.z);
                hi.x = hi.x.max(p.x);
                hi.y = hi.y.max(p.y);
                hi.z = hi.z.max(p.z);
            }
            println!(
                "  RENDERED face {face_filter}: triangles={} bbox=[{lo:?}]..[{hi:?}] diag={:.4e}",
                mesh.faces().len(),
                (hi - lo).magnitude()
            );
        } else {
            println!(
                "  RENDERED face {face_filter}: NO MESH (failure={:?})",
                outcome.face_failures[face_idx]
            );
        }
        break;
    }
    Ok(())
}

fn root_for(curve: &Curve3D, er: (f64, f64), v: Point3) -> (f64, f64) {
    let n = 8192;
    let mut best = (f64::INFINITY, 0.0);
    for i in 0..=n {
        let t = er.0 + (er.1 - er.0) * i as f64 / n as f64;
        let d = curve.subs(t).distance(v);
        if d < best.0 {
            best = (d, t);
        }
    }
    best
}
