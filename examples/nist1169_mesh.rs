//! Diagnostic: examine the clean-state #1169 mesh — largest edges, off-surface
//! vertices, and the true surface area of the intended band region.

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

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: nist1169_mesh MODEL.step");
        return Ok(());
    }
    let table = load(&args[0])?;
    for (&shell_id, shell) in table.shell.iter() {
        let (cshell, _) = table
            .to_compressed_shell_with_losses(shell_id, shell)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
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
        use look::step::policy_geometry::PolicySurface;
        let outcome = look::step::policy_geometry::wrap_shell(
            cshell.clone(),
            look::step::meshing_policy::MeshingPolicy::DEFAULT,
        )
        .robust_triangulation_with_torus_outcome(
            tol,
            |s: &PolicySurface| look::step_lattice_of(s.inner()),
            |s: &PolicySurface| look::step_support_schema_of(s.inner()),
            |c: &look::step::policy_geometry::PolicyCurve| look::step_curve_schema_of(c.inner()),
            |s: &PolicySurface| look::step_cylinder_of(s.inner()),
            |c: &look::step::policy_geometry::PolicyCurve| {
                look::step_cylinder_curve_schema_of(c.inner())
            },
            |c: &look::step::policy_geometry::PolicyCurve| {
                look::step_cylinder_curve_family_of(c.inner())
            },
            |s: &PolicySurface| look::step_cone_of(s.inner()),
            |s: &PolicySurface| look::step::torus_deck::identify_source_torus_opt(s.inner()),
        );
        let meshed = &outcome.shell;
        for (i, face) in meshed.faces.iter().enumerate() {
            let Some(id) = face.provenance.best_id() else {
                continue;
            };
            if id.get() != 1169 {
                continue;
            }
            let Some(mesh) = &face.surface else { continue };
            let src = &cshell.faces[i].surface;
            let pos = mesh.positions();
            let ntri = mesh.tri_faces().len();
            println!("=== #1169 clean mesh: {ntri} tris, {} verts ===", pos.len());
            // Largest edges
            let mut edges: Vec<(f64, usize, usize)> = Vec::new();
            for tri in mesh.tri_faces() {
                let _a = pos[tri[0].pos];
                let _b = pos[tri[1].pos];
                let _c = pos[tri[2].pos];
                for (u, v) in [
                    (tri[0].pos, tri[1].pos),
                    (tri[1].pos, tri[2].pos),
                    (tri[2].pos, tri[0].pos),
                ] {
                    edges.push((pos[u].distance(pos[v]), u, v));
                }
            }
            edges.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            println!("largest edges:");
            for (d, u, v) in edges.iter().take(5) {
                let pu = pos[*u];
                let pv = pos[*v];
                let (pux, puy, puz) = (pu.x, pu.y, pu.z);
                let (pvx, pvy, pvz) = (pv.x, pv.y, pv.z);
                println!(
                    "  {d:.3}  verts {u}->{v}: ({pux:.2},{puy:.2},{puz:.2}) -> ({pvx:.2},{pvy:.2},{pvz:.2})"
                );
            }
            // Off-surface vertices
            let mut off: Vec<usize> = Vec::new();
            for (vi, v) in pos.iter().enumerate() {
                let res = match src.search_parameter(*v, None, 200) {
                    Some(uv) => src.subs(uv.0, uv.1).distance(*v),
                    None => f64::INFINITY,
                };
                if res > 1.0e-3 || !res.is_finite() {
                    off.push(vi);
                }
            }
            println!("off-surface vertices ({}) indices: {:?}", off.len(), off);
            // True surface area of the region u in [0.6,1], v in [0,1]
            let area = |umin: f64| -> f64 {
                let nu = 40;
                let nv = 160;
                let mut sum = 0.0;
                for i in 0..nu {
                    for j in 0..nv {
                        let u = umin + (1.0 - umin) * (i as f64 + 0.5) / nu as f64;
                        let v = (j as f64 + 0.5) / nv as f64;
                        let du = src.uder(u, v);
                        let dv = src.vder(u, v);
                        sum +=
                            du.cross(dv).magnitude() * (1.0 - umin) / nu as f64 * (1.0 / nv as f64);
                    }
                }
                sum
            };
            println!("true surface area u in [0.6,1]: {:.1}", area(0.6));
            println!("true surface area u in [0.65,1]: {:.1}", area(0.65));
            println!("true surface area u in [0.7,1]: {:.1}", area(0.7));
            println!("true surface area u in [0.0,1]: {:.1}", area(0.0));
        }
    }
    Ok(())
}
