//! Focused Stage A+B witness: #506/#507 must receive the source-certified
//! V period (1.0) through the normal *wrapped production* lattice callback —
//! `lattice_of_with_closure(s.inner(), s.source_closure())` on the
//! `wrap_shell_with_closure` output — with witness
//! `SourceDeclaredClosedSplineAxis`.

use std::env;

use truck_meshalgo::tessellation::domain::lattice::{AxisPeriodStatus, PeriodWitness};
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
        eprintln!("usage: nist1167_source_closure_probe MODEL.step");
        return Ok(());
    }
    let model_path = &args[0];

    let table = load(model_path)?;
    let closure_map = look::step::lattice::spline_closure_map(&table);

    let mut witnesses = Vec::new();
    for (shell_idx, (&shell_id, shell)) in table.shell.iter().enumerate() {
        let (cshell, _losses) = table
            .to_compressed_shell_with_losses(shell_id, shell)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let wrapped = look::step::policy_geometry::wrap_shell_with_closure(
            cshell,
            look::step::meshing_policy::MeshingPolicy::DEFAULT,
            &closure_map,
        );
        if let Ok(ids) = std::env::var("PROBE_LIST_ALL")
            && ids == "1"
        {
            for (face_idx, face) in wrapped.faces.iter().enumerate() {
                println!(
                    "ALL face idx {face_idx}: best={:?} surface_id={:?} closure={:?}",
                    face.provenance.best_id().map(|id| id.get()),
                    face.provenance.surface_id.map(|id| id.get()),
                    face.surface.source_closure(),
                );
            }
        }
        for (face_idx, face) in wrapped.faces.iter().enumerate() {
            let surface_id = face.provenance.surface_id.map(|id| id.get());
            let best = face.provenance.best_id().map(|id| id.get());
            let is_target = surface_id == Some(506)
                || surface_id == Some(507)
                || best == Some(1167)
                || best == Some(1169);
            if !is_target {
                continue;
            }
            let source = face
                .provenance
                .surface_id
                .and_then(|id| closure_map.get(&id.get()).copied());
            let lattice = look::step::lattice::lattice_of_with_closure(
                face.surface.inner(),
                face.surface.source_closure(),
            );
            let (tag_u, tag_v) = match (lattice.u, lattice.v) {
                (
                    AxisPeriodStatus::Exact { period, witness },
                    AxisPeriodStatus::Exact {
                        period: period_v,
                        witness: witness_v,
                    },
                ) => (
                    format!("Exact[{period}]/{witness:?}"),
                    format!("Exact[{period_v}]/{witness_v:?}"),
                ),
                (AxisPeriodStatus::Exact { period, witness }, AxisPeriodStatus::NonPeriodic) => {
                    (format!("Exact[{period}]/{witness:?}"), "NonPeriodic".into())
                }
                (AxisPeriodStatus::NonPeriodic, AxisPeriodStatus::Exact { period, witness }) => {
                    ("NonPeriodic".into(), format!("Exact[{period}]/{witness:?}"))
                }
                _ => (format!("{:?}", lattice.u), format!("{:?}", lattice.v)),
            };
            println!(
                "face #{best:?} (shell #{shell_idx}, face idx {face_idx}): surface_id={surface_id:?} source_closure={source:?} production lattice u={tag_u} v={tag_v}"
            );
            witnesses.push((
                best.unwrap(),
                face.surface.source_closure(),
                lattice.v.generator(),
                lattice.v,
            ));
        }
    }

    for (id, source, generator, status) in &witnesses {
        assert!(
            source.is_some(),
            "face #{id}: expected a source closure to be attached"
        );
        assert_eq!(*generator, Some(1.0), "face #{id}: expected V period 1.0");
        assert!(
            matches!(
                status,
                AxisPeriodStatus::Exact {
                    witness: PeriodWitness::SourceDeclaredClosedSplineAxis,
                    ..
                }
            ),
            "face #{id}: expected SourceDeclaredClosedSplineAxis witness"
        );
        println!("PASS face #{id}: V period = 1.0, witness = SourceDeclaredClosedSplineAxis");
    }
    if witnesses.is_empty() {
        anyhow::bail!("no #506/#507 faces found");
    }
    Ok(())
}
