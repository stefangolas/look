//! Probe: run the exact production tessellation path on `nist_ctc_02_asme1_ap203`
//! and dump geometry evidence for faces `#1167` (lost) and `#1169` (rendered).
//!
//! The purpose is to decide whether `#1169`'s 89-triangle mesh is the correct
//! annular band (which would weaken the closure hypothesis) or a degenerate
//! collapsed artifact (which would support it).

use std::env;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::Table;

fn load(path: &str) -> anyhow::Result<Table> {
    let bytes = std::fs::read(path)?;
    let text = std::str::from_utf8(&bytes)
        .map(std::borrow::Cow::Borrowed)
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());
    let mut exchange = match look::step::part21::parse(&text) {
        Ok(exchange) => exchange,
        Err(_) => ruststep::parser::parse(&text)
            .map_err(|error| anyhow::anyhow!("failed to parse STEP: {error}"))?,
    };
    let section = exchange.data.swap_remove(0);
    Ok(Table::from_owned_data_section(section))
}

fn area_of(mesh: &PolygonMesh) -> f64 {
    let pos = mesh.positions();
    mesh.tri_faces()
        .iter()
        .map(|tri| {
            let a = pos[tri[0].pos];
            let b = pos[tri[1].pos];
            let c = pos[tri[2].pos];
            (b - a).cross(c - a).magnitude() * 0.5
        })
        .sum()
}

fn bbox_of(mesh: &PolygonMesh) -> (Point3, Point3) {
    let mut lo = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut hi = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for v in mesh.positions() {
        lo.x = lo.x.min(v.x);
        lo.y = lo.y.min(v.y);
        lo.z = lo.z.min(v.z);
        hi.x = hi.x.max(v.x);
        hi.y = hi.y.max(v.y);
        hi.z = hi.z.max(v.z);
    }
    (lo, hi)
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: nist1167_production_mesh MODEL.step");
        return Ok(());
    }
    let model_path = &args[0];
    let table = load(model_path)?;
    let closure_map = look::step::lattice::spline_closure_map(&table);
    let mut done = false;
    for (shell_idx, (&shell_id, shell)) in table.shell.iter().enumerate() {
        let (cshell, losses) = table
            .to_compressed_shell_with_losses(shell_id, shell)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        println!(
            "=== shell #{shell_idx} conversion losses: {} ===",
            losses.len()
        );

        let mut model_bbox = BoundingBox::<Point3>::new();
        for v in &cshell.vertices {
            model_bbox.push(*v);
        }
        for edge in &cshell.edges {
            let (a, b) = edge.curve.range_tuple();
            for i in 0..=4 {
                model_bbox.push(edge.curve.subs(a + (b - a) * f64::from(i) / 4.0));
            }
        }
        let scaled = model_bbox.diameter() * 0.001;
        let tol = if scaled.is_finite() && scaled > 0.0 {
            scaled.max(1.0e-6)
        } else {
            1.0e-3
        };
        println!("shell #{shell_idx}: model tol = {tol:.6e}");

        use look::step::policy_geometry::PolicyCurve;
        let outcome = look::step::policy_geometry::wrap_shell_with_closure(
            cshell.clone(),
            look::step::meshing_policy::MeshingPolicy::DEFAULT,
            &closure_map,
        )
        .robust_triangulation_with_torus_outcome(
            tol,
            |s: &look::step::policy_geometry::PolicySurface| {
                look::step_lattice_of_with_closure(s.inner(), s.source_closure())
            },
            |s: &look::step::policy_geometry::PolicySurface| {
                look::step_support_schema_of(s.inner())
            },
            |c: &PolicyCurve| look::step_curve_schema_of(c.inner()),
            |s: &look::step::policy_geometry::PolicySurface| look::step_cylinder_of(s.inner()),
            |c: &PolicyCurve| look::step_cylinder_curve_schema_of(c.inner()),
            |c: &PolicyCurve| look::step_cylinder_curve_family_of(c.inner()),
            |s: &look::step::policy_geometry::PolicySurface| look::step_cone_of(s.inner()),
            |s: &look::step::policy_geometry::PolicySurface| {
                look::step::torus_deck::identify_source_torus_opt(s.inner())
            },
        );
        let meshed = &outcome.shell;
        for (i, face) in meshed.faces.iter().enumerate() {
            let Some(id) = face.provenance.best_id() else {
                continue;
            };
            let id_u64 = id.get();
            if id_u64 != 1167 && id_u64 != 1169 {
                continue;
            }
            done = true;
            let tag = format!("#{id_u64}");
            let fail = outcome.face_failures.get(i).cloned().flatten();
            let fail_str = fail
                .map(|f| format!("{:?}", f.reason))
                .unwrap_or_else(|| "-".into());
            // The original (unmeshed) face carries the source surface; use it to
            // measure vertex-on-surface residuals of the produced mesh.
            let src_surface = &cshell.faces[i].surface;
            match &face.surface {
                Some(mesh) if !mesh.faces().is_empty() => {
                    let (lo, hi) = bbox_of(mesh);
                    let diag = (hi - lo).magnitude();
                    let area = area_of(mesh);
                    let max_edge = mesh
                        .tri_faces()
                        .iter()
                        .map(|tri| {
                            let a = mesh.positions()[tri[0].pos];
                            let b = mesh.positions()[tri[1].pos];
                            let c = mesh.positions()[tri[2].pos];
                            (b - a)
                                .magnitude()
                                .max((c - b).magnitude())
                                .max((a - c).magnitude())
                        })
                        .fold(0.0f64, f64::max);
                    // Vertex-on-surface residual: every emitted vertex must lie
                    // on/near the intended B-spline surface.
                    let mut vmax = 0.0f64;
                    let mut vsum = 0.0f64;
                    let mut von = 0usize;
                    let nv = mesh.positions().len();
                    for v in mesh.positions() {
                        let res = match src_surface.search_parameter(*v, None, 200) {
                            Some(uv) => src_surface.subs(uv.0, uv.1).distance(*v),
                            None => f64::INFINITY,
                        };
                        vmax = vmax.max(res);
                        vsum += res;
                        if res <= 1.0e-3 {
                            von += 1;
                        }
                    }
                    let (lox, loy, loz) = (lo.x, lo.y, lo.z);
                    let (hix, hiy, hiz) = (hi.x, hi.y, hi.z);
                    let ntri = mesh.tri_faces().len();
                    let npos = mesh.positions().len();
                    println!(
                        "FACE {tag}: RENDERED tris={ntri} verts={npos} area={area:.6} bbox_lo=({lox:.4},{loy:.4},{loz:.4}) bbox_hi=({hix:.4},{hiy:.4},{hiz:.4}) diag={diag:.4} max_edge={max_edge:.4}"
                    );
                    println!(
                        "FACE {tag}: vertex-on-surface residual max={vmax:.3e} mean={:.3e} on_surface(<=1e-3)={von}/{nv}",
                        vsum / nv as f64
                    );
                }
                Some(_) => {
                    println!("FACE {tag}: EMPTY (Some mesh, 0 triangles) failure={fail_str}");
                }
                None => {
                    println!("FACE {tag}: NO SURFACE failure={fail_str}");
                }
            }
        }
        if done {
            break;
        }
    }
    Ok(())
}
