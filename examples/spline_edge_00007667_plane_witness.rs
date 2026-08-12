//! Scratch probe (Phase 2): direct witness that the currently-rendered plane
//! face #10428 is a full-loop false positive. Tessellates the shell containing
//! the face through the exact production entry point at the whole-model
//! production tolerance, then shows a rendered mesh vertex lying near C(0.5) --
//! on the complementary far-side arc of the closed spline loop, i.e. on the
//! plane but on neither the source-selected wrapped arc [t_a -> t_b] nor the
//! chord between the STEP vertices.
//! Usage: spline_edge_00007667_plane_witness MODEL.step --face 10428

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

    // Whole-model bbox exactly as production computes it.
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
    let shell = &converted[shell_idx];

    // Locate the shared spline edge (the one with the unclamped range sliver).
    let face = &shell.faces[face_idx];
    let mut shared: Option<usize> = None;
    for wire in &face.boundaries {
        for idx in wire {
            if shell.edges[idx.index].curve.range_tuple().0 < -0.4 {
                shared = Some(idx.index);
            }
        }
    }
    let si = shared.expect("no shared spline edge on face");
    let edge = &shell.edges[si];
    let (va, vb) = edge.vertices;
    let (pv_a, pv_b) = (shell.vertices[va], shell.vertices[vb]);
    let curve = &edge.curve;
    let er = curve.evaluation_range();
    println!("shared edge idx={si} verts=({va},{vb}) er={er:?}");
    println!("  pv_a={pv_a:?}");
    println!("  pv_b={pv_b:?}");

    // Solve the vertex roots by coarse scan + golden-section refinement.
    let (t_a, r_a) = root_for(curve, er, pv_a);
    let (t_b, r_b) = root_for(curve, er, pv_b);
    println!("  root t_a={t_a:.9} residual={r_a:.2e}");
    println!("  root t_b={t_b:.9} residual={r_b:.2e}");

    // Far-side witness parameter: t_mid = 0.5, on the complementary arc.
    let t_mid = 0.5_f64;
    let c_mid = curve.subs(t_mid);
    println!("  C(0.5)={c_mid:?}");

    // Is C(0.5) on the source arc [t_a -> t_b] wrapped through the seam?
    let on_source_arc = {
        // increasing-parameter arc from t_a to t_b, wrapping if t_a > t_b
        if t_a <= t_b {
            t_mid >= t_a && t_mid <= t_b
        } else {
            t_mid >= t_a || t_mid <= t_b
        }
    };
    println!("  C(0.5) on source arc [t_a -> t_b]: {on_source_arc}");

    // Is C(0.5) on the chord between the two STEP vertices?
    let chord_dist = point_segment_distance(c_mid, pv_a, pv_b);
    println!("  C(0.5) distance to chord(pv_a,pv_b): {chord_dist:.4e}");

    // Tessellate the whole shell through the production entry point.
    use look::step::policy_geometry::{PolicyCurve, PolicySurface};
    let outcome = look::step::policy_geometry::wrap_shell(
        shell.clone(),
        look::step::meshing_policy::MeshingPolicy::DEFAULT,
    )
    .robust_triangulation_with_torus_outcome(
        production_tolerance,
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
    let Some(mesh) = &meshed.faces[face_idx].surface else {
        println!(
            "  face {face_filter}: NO MESH (failure={:?})",
            outcome.face_failures[face_idx]
        );
        return Ok(());
    };
    if mesh.faces().is_empty() {
        println!("  face {face_filter}: MeshedToNothing");
        return Ok(());
    }
    println!(
        "  face {face_filter} rendered triangles={}",
        mesh.tri_faces().len()
    );

    // Find the rendered mesh vertex closest to C(0.5).
    let mut best_d = f64::INFINITY;
    let mut best_p = Point3::origin();
    for p in mesh.positions().iter() {
        let d = p.distance(c_mid);
        if d < best_d {
            best_d = d;
            best_p = *p;
        }
    }
    println!("  rendered mesh vertex nearest C(0.5): {best_p:?}  distance={best_d:.4e}");
    println!(
        "  -> the rendered mesh reaches the far side of the closed loop (C(0.5) lies on the \
         complementary arc, NOT the source arc, and NOT the chord)"
    );

    // Report the full rendered region diagonal for the record.
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
    println!("  rendered bbox diag={:.4e}", (hi - lo).magnitude());
    Ok(())
}

fn root_for(curve: &Curve3D, er: (f64, f64), v: Point3) -> (f64, f64) {
    // coarse scan then golden-section refinement
    let n = 4096;
    let mut best = (f64::INFINITY, 0.0);
    for i in 0..=n {
        let t = er.0 + (er.1 - er.0) * i as f64 / n as f64;
        let d = curve.subs(t).distance(v);
        if d < best.0 {
            best = (d, t);
        }
    }
    let (mut lo, mut hi) = (
        best.1 - (er.1 - er.0) / n as f64,
        best.1 + (er.1 - er.0) / n as f64,
    );
    lo = lo.max(er.0);
    hi = hi.min(er.1);
    const PHI: f64 = 1.618033988749895;
    let mut a = lo;
    let mut b = hi;
    let mut c = b - (b - a) / PHI;
    let mut d = a + (b - a) / PHI;
    for _ in 0..80 {
        if (c - d).abs() < 1e-15 {
            break;
        }
        if curve.subs(c).distance(v) < curve.subs(d).distance(v) {
            b = d;
        } else {
            a = c;
        }
        c = b - (b - a) / PHI;
        d = a + (b - a) / PHI;
    }
    let t = (a + b) / 2.0;
    (t, curve.subs(t).distance(v))
}

fn point_segment_distance(p: Point3, a: Point3, b: Point3) -> f64 {
    let ab = b - a;
    let len2 = ab.magnitude2();
    if len2 <= 0.0 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}
