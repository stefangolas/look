//! Scratch probe (Phase 2): reconcile the #19018 production-tolerance
//! discrepancy. Computes the whole-model diameter exactly as production does
//! (all converted shells, vertices + edge samples over `range_tuple` with
//! `EDGE_SAMPLES=4`), derives the production tolerance through the same policy
//! path, then runs the exact production tessellation entry point on the shell
//! containing the target face at (a) the exact production tolerance and (b) a
//! hardcoded 1e-3, reporting the target face's triangles/region under each.
//! Usage: spline_edge_00007667_tolreconcile MODEL.step --face 19018

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{Table, step_geometry::Curve3D};

const RELATIVE_TOLERANCE: f64 = 0.001;
const MINIMUM_TOLERANCE: f64 = 1.0e-6;
const EDGE_SAMPLES: u32 = 4;

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

    // Whole-model bbox exactly as production does it: every converted shell's
    // vertices plus EDGE_SAMPLES samples per edge over range_tuple().
    let mut converted = Vec::new();
    for (&shell_id, shell) in table.shell.iter() {
        if let Ok(cshell) = table.to_compressed_shell_with_losses(shell_id, shell) {
            converted.push(cshell.0);
        }
    }
    let mut model = BoundingBox::<Point3>::new();
    for shell in &converted {
        for v in &shell.vertices {
            model.push(*v);
        }
        for edge in &shell.edges {
            let (a, b) = edge.curve.range_tuple();
            for i in 0..=EDGE_SAMPLES {
                model.push(
                    edge.curve
                        .subs(a + (b - a) * f64::from(i) / f64::from(EDGE_SAMPLES)),
                );
            }
        }
    }
    let diameter = model.diameter();
    let scaled = diameter * RELATIVE_TOLERANCE;
    let production_tolerance = if scaled.is_finite() && scaled > 0.0 {
        scaled.max(MINIMUM_TOLERANCE)
    } else {
        1.0e-3
    };
    println!("model diameter={diameter:.6} production tolerance={production_tolerance:.6e}");

    // Locate the shell + face index for the target.
    let (shell_idx, face_idx) = converted
        .iter()
        .enumerate()
        .find_map(|(si, shell)| {
            shell
                .faces
                .iter()
                .position(|f| f.provenance.best_id().map(|id| id.get()) == Some(face_filter))
                .map(|fi| (si, fi))
        })
        .expect("target face not found in any converted shell");
    println!("face {face_filter} shell={shell_idx} face_idx={face_idx}");

    let tessellate = |tolerance: f64, label: &str| {
        let shell = &converted[shell_idx];
        use look::step::policy_geometry::{PolicyCurve, PolicySurface};
        let outcome = look::step::policy_geometry::wrap_shell(
            shell.clone(),
            look::step::meshing_policy::MeshingPolicy::DEFAULT,
        )
        .robust_triangulation_with_torus_outcome(
            tolerance,
            |s: &PolicySurface| look::step_lattice_of(s.inner()),
            |s: &PolicySurface| look::step_support_schema_of(s.inner()),
            |c: &PolicyCurve| look::step_curve_schema_of(c.inner()),
            |s: &PolicySurface| look::step_cylinder_of(s.inner()),
            |c: &PolicyCurve| look::step_cylinder_curve_schema_of(c.inner()),
            |c: &PolicyCurve| look::step_cylinder_curve_family_of(c.inner()),
            |s: &PolicySurface| look::step_cone_of(s.inner()),
            |s: &PolicySurface| look::step::torus_deck::identify_source_torus_opt(s.inner()),
        );
        let meshed = &outcome.shell;
        match &meshed.faces[face_idx].surface {
            Some(mesh) if mesh.faces().is_empty() => {
                println!("  [{label}] tol={tolerance:.6e} face {face_filter}: MeshedToNothing");
            }
            Some(mesh) => {
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
                    "  [{label}] tol={tolerance:.6e} face {face_filter}: triangles={} bbox_diag={:.4e}",
                    mesh.tri_faces().len(),
                    (hi - lo).magnitude()
                );
            }
            None => println!(
                "  [{label}] tol={tolerance:.6e} face {face_filter}: NoSurfaceProduced ({:?})",
                outcome.face_failures[face_idx]
            ),
        }
    };

    // Exact production tolerance (whole-model).
    tessellate(production_tolerance, "production");
    // Hardcoded 1e-3, as the earlier scratch probe did.
    tessellate(1.0e-3, "hardcoded1e-3");
    // A couple of bracketing tolerances to show the sensitivity window.
    tessellate(5.0e-4, "5e-4");
    tessellate(2.0e-3, "2e-3");

    // Also record the exact vertex roots for the target face's shared edge,
    // using the refined solver, so the record carries them again.
    let shell = &converted[shell_idx];
    let face = &shell.faces[face_idx];
    let mut shared: Option<usize> = None;
    for wire in &face.boundaries {
        for idx in wire {
            if shell.edges[idx.index].curve.range_tuple().0 < -0.4 {
                shared = Some(idx.index);
            }
        }
    }
    if let Some(si) = shared {
        let edge = &shell.edges[si];
        let (va, vb) = edge.vertices;
        let (pv_a, pv_b) = (shell.vertices[va], shell.vertices[vb]);
        let curve = &edge.curve;
        let er = curve.evaluation_range();
        let (t_a, r_a) = root_for(curve, er, pv_a);
        let (t_b, r_b) = root_for(curve, er, pv_b);
        println!(
            "shared edge idx={si} verts=({va},{vb}) t_a={t_a:.9} r_a={r_a:.2e} t_b={t_b:.9} r_b={r_b:.2e} wrap={}",
            t_a > t_b
        );
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
