//! ABC-REMAINDER-DIAG-001 — what *source authority* every imported face still
//! carries, independent of what tessellation later did with it.
//!
//! The band sweep answered "how far did the cylinder-band route get"; the
//! `TRUCK_FACE_DIAG_JSONL` records answer "where did the realizer stop". Neither
//! answers the question this packet opens with: **what did the source actually
//! say about this face**, and is the obstruction that the file withheld
//! something, that the importer dropped something, or that the mathematics is
//! missing?
//!
//! So this reads the imported shell — the same `Table` production reads, through
//! the same `to_compressed_shell_with_losses` — and prints, for every surviving
//! face, the evidence a material-authority proof would have to start from:
//!
//! - the `FACE_OUTER_BOUND` standing STEP declared ([`OuterBoundStanding`]),
//!   including the contradictory *multiply declared* state, kept apart from
//!   "the source declared none" and from "no stage retained it";
//! - the certified deck lattice of the supporting surface, generators only, so
//!   a merely *declared* period never inflates the rank;
//! - the per-bound edge-use count and imported curve family multiset, which is
//!   what distinguishes "one bound that is one complete circle" from "one bound
//!   of four line segments" without asserting anything about either;
//! - both production curve classifiers' verdicts (rank-0 and rank-1), so an
//!   unread curve is attributable to a gate rather than guessed at.
//!
//! It is a pure observer. It runs no tessellation, admits nothing, refuses
//! nothing, and cannot move a face between rendered and lost. Every field is
//! either a source datum or a production classifier's own tag; nothing here
//! infers intent from geometric appearance — no "largest bound is the outer
//! one", no "nearly circular is a circle". The join key is `source_face_id`,
//! the same key `face_census --ledger` and `band_curve_probe` use.
//!
//! ```console
//! remainder_probe MODEL.step > model.faces.tsv
//! ```

use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;

use truck_stepio::r#in::Table;
use truck_stepio::r#in::step_geometry::{Conic3D, Curve3D, ElementarySurface, Surface, SweptCurve};
use truck_topology::compress::OuterBoundStanding;

/// How many bounds are spelled out per face before the rest are summarised.
///
/// A 26-bound face is real (`00001116` has one) and its full signature is not
/// worth a kilobyte of ledger; the count is retained either way, so nothing
/// that aggregation needs is lost with the tail.
const BOUND_SIGNATURE_LIMIT: usize = 8;

/// The imported curve family's own name, abbreviated for the multiset.
///
/// Deliberately the *imported* variant and not the raw STEP entity: since the
/// source-family repair the two agree on conics, and `band_curve_probe`'s
/// header explains why they are still reported separately. `Ci`/`El` are
/// distinct here precisely because collapsing them is the defect that packet
/// fixed.
fn curve_abbrev(curve: &Curve3D) -> &'static str {
    match curve {
        Curve3D::Line(_) => "Ln",
        Curve3D::Polyline(_) => "Pl",
        Curve3D::Conic(Conic3D::Circle(_)) => "Ci",
        Curve3D::Conic(Conic3D::Ellipse(_)) => "El",
        Curve3D::Conic(Conic3D::Hyperbola(_)) => "Hy",
        Curve3D::Conic(Conic3D::Parabola(_)) => "Pa",
        Curve3D::BSplineCurve(_) => "Bs",
        Curve3D::NurbsCurve(_) => "Nu",
        Curve3D::PCurve(_) => "Pc",
    }
}

fn surface_kind(surface: &Surface) -> &'static str {
    match surface {
        Surface::ElementarySurface(e) => match e {
            ElementarySurface::Plane(_) => "plane",
            ElementarySurface::Sphere(_) => "sphere",
            ElementarySurface::CylindricalSurface(_) => "cylinder",
            ElementarySurface::ToroidalSurface(_) => "torus",
            ElementarySurface::ConicalSurface(_) => "cone",
            ElementarySurface::DegenerateToroidalSurface(_) => "torus_degen",
        },
        Surface::SweptCurve(s) => match s {
            SweptCurve::ExtrudedCurve(_) => "extruded",
            SweptCurve::RevolutedCurve(_) => "revolved",
        },
        Surface::BSplineSurface(_) => "bspline",
        Surface::NurbsSurface(_) => "nurbs",
        Surface::OffsetSurface(_) => "offset",
    }
}

/// The outer-bound standing, flattened to a tag plus the two numbers the tag
/// alone cannot carry.
///
/// `declared_count` matters on its own: two `FACE_OUTER_BOUND` entities on one
/// face is the malformed-source class the cylinder-band route recovers, and it
/// must stay visible on faces where no band certificate exists to recover it.
fn outer_fields(standing: OuterBoundStanding) -> (&'static str, String, String) {
    match standing {
        OuterBoundStanding::Declared {
            bound_index,
            declared_count,
        } => (
            standing.tag(),
            declared_count.to_string(),
            bound_index.to_string(),
        ),
        _ => (standing.tag(), "0".into(), "-".into()),
    }
}

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
    let Some(path) = args.first() else {
        eprintln!("usage: remainder_probe MODEL.step");
        return Ok(());
    };
    let table = load(path)?;

    let mut faces = 0usize;
    let mut by_standing: BTreeMap<&'static str, usize> = BTreeMap::new();

    for (&shell_id, shell) in table.shell.iter() {
        let Ok((cshell, _losses)) = table.to_compressed_shell_with_losses(shell_id, shell) else {
            continue;
        };
        for face in &cshell.faces {
            faces += 1;
            let id = face
                .provenance
                .best_id()
                .map(|id| id.get().to_string())
                .unwrap_or_else(|| "-".into());
            let kind = surface_kind(&face.surface);
            let (standing, declared_count, outer_index) = outer_fields(face.provenance.outer_bound);
            *by_standing
                .entry(face.provenance.outer_bound.tag())
                .or_default() += 1;

            // Periodicity, from the production classifier. `certified` counts
            // deck generators; `declared` counts what the accessors report. The
            // two disagreeing is itself a finding — an axis that is declared and
            // uncertified has no generator to quotient by.
            let lattice = look::step_lattice_of(&face.surface);
            let certified_rank = lattice.certified_rank();
            let declared_rank = usize::from(lattice.declared_u_period().is_some())
                + usize::from(lattice.declared_v_period().is_some());
            let support = look::step_support_schema_of(&face.surface).tag();
            let cylinder = match look::step_cylinder_of(&face.surface) {
                Ok(_) => "certified",
                Err(tag) => tag,
            };

            let mut uses = 0usize;
            let mut unread_rank0 = 0usize;
            let mut unread_rank1 = 0usize;
            let mut families: BTreeMap<&'static str, usize> = BTreeMap::new();
            let mut signature = String::new();
            for (bound_index, boundary) in face.boundaries.iter().enumerate() {
                let mut bound_families: BTreeMap<&'static str, usize> = BTreeMap::new();
                for edge_index in boundary.iter() {
                    let curve = &cshell.edges[edge_index.index].curve;
                    uses += 1;
                    let abbrev = curve_abbrev(curve);
                    *families.entry(abbrev).or_default() += 1;
                    *bound_families.entry(abbrev).or_default() += 1;
                    // Both production gates, because they differ: the rank-1
                    // classifier reads a circular arc the rank-0 one does not,
                    // and a curve unread by *both* is a different finding from
                    // one unread only off the cylinder route.
                    if !look::step_curve_schema_of(curve).is_structurally_identified() {
                        unread_rank0 += 1;
                    }
                    if !look::step_cylinder_curve_schema_of(curve).is_structurally_identified() {
                        unread_rank1 += 1;
                    }
                }
                if bound_index < BOUND_SIGNATURE_LIMIT {
                    if bound_index > 0 {
                        signature.push(';');
                    }
                    let _ = write!(signature, "{}[", boundary.len());
                    for (position, (abbrev, count)) in bound_families.iter().enumerate() {
                        if position > 0 {
                            signature.push(',');
                        }
                        let _ = write!(signature, "{abbrev}{count}");
                    }
                    signature.push(']');
                }
            }
            if face.boundaries.len() > BOUND_SIGNATURE_LIMIT {
                let _ = write!(
                    signature,
                    ";+{}",
                    face.boundaries.len() - BOUND_SIGNATURE_LIMIT
                );
            }
            let multiset = families
                .iter()
                .map(|(abbrev, count)| format!("{abbrev}{count}"))
                .collect::<Vec<_>>()
                .join(",");

            println!(
                "FACE\tsource_face_id={id}\tsurface_kind={kind}\touter_standing={standing}\t\
                 outer_declared_count={declared_count}\touter_bound_index={outer_index}\t\
                 bounds={}\tedge_uses={uses}\tdeclared_rank={declared_rank}\t\
                 certified_rank={certified_rank}\tsupport={support}\tcylinder={cylinder}\t\
                 curves={multiset}\tbound_signature={signature}\t\
                 unread_rank0={unread_rank0}\tunread_rank1={unread_rank1}",
                face.boundaries.len(),
            );
        }
    }

    eprintln!("remainder_probe: {faces} imported faces");
    for (tag, count) in &by_standing {
        eprintln!("  outer_standing {tag:20} {count:8}");
    }
    Ok(())
}
