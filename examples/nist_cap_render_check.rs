//! Temporary diagnostic (NIST-RECOVERY P3b): tessellate a model with the
//! production meshing policy, report per targeted source face the bounding box
//! of the produced mesh (so a silent wrong-side render is visible), and
//! optionally dump the whole shell mesh to an STL for visual inspection.

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
    let path = std::env::args().nth(1).expect("model path");
    let targets: Vec<u64> = std::env::var("TRUCK_CAP_CHECK_TARGET")
        .ok()
        .map(|raw| {
            raw.split(',')
                .filter_map(|t| t.trim().parse::<u64>().ok())
                .collect()
        })
        .unwrap_or_else(|| vec![1954, 353]);
    let table = load(&path)?;
    let mut bbox = BoundingBox::<Point3>::new();
    for (_, shell) in table.shell.iter() {
        if let Ok((cshell, _)) = table.to_compressed_shell_with_losses(shell) {
            for v in &cshell.vertices {
                bbox.push(*v);
            }
            for edge in &cshell.edges {
                let (a, b) = edge.curve.range_tuple();
                for i in 0..=4u32 {
                    bbox.push(edge.curve.subs(a + (b - a) * f64::from(i) / 4.0));
                }
            }
        }
    }
    let scaled = bbox.diameter() * 0.001;
    let tolerance = scaled.max(1.0e-6).min(1.0e3);
    println!(
        "model bbox={bbox:?} diameter={:.4} tolerance={tolerance:.6e}",
        bbox.diameter()
    );

    let out_path = std::env::var("TRUCK_CAP_DUMP").ok();
    let mut whole = truck_meshalgo::prelude::PolygonMesh::default();
    for (_, shell) in table.shell.iter() {
        let Ok((cshell, _)) = table.to_compressed_shell_with_losses(shell) else {
            continue;
        };
        use look::step::policy_geometry::{PolicyCurve, PolicySurface};
        let outcome = look::step::policy_geometry::wrap_shell(
            cshell,
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
        for (i, face) in outcome.shell.faces.iter().enumerate() {
            let Some(id) = face.provenance.best_id().map(|id| id.get()) else {
                continue;
            };
            if let Some(mesh) = &face.surface {
                whole.merge(mesh.clone());
                if targets.contains(&id) {
                    let mut fb = BoundingBox::<Point3>::new();
                    for p in mesh.positions() {
                        fb.push(*p);
                    }
                    println!(
                        "FACE #{id} idx={i} triangles={} bbox={fb:?}",
                        mesh.tri_faces().len()
                    );
                }
            }
        }
    }
    if let Some(dest) = out_path {
        let mut obj = String::new();
        for p in whole.positions() {
            obj.push_str(&format!("v {} {} {}\n", p.x, p.y, p.z));
        }
        for f in whole.faces().tri_faces() {
            obj.push_str(&format!("f {} {} {}\n", f[0].pos + 1, f[1].pos + 1, f[2].pos + 1));
        }
        std::fs::write(&dest, obj)?;
        println!("dumped mesh to {dest} ({} triangles)", whole.tri_faces().len());
    }
    Ok(())
}
