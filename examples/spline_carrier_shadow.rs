//! Wave-2 spline-on-carrier shadow probe (Part A1).
//!
//! Runs [`look::step::spline_carrier::certify_spline_carrier`] over every
//! B-spline/NURBS boundary edge of every cylinder/cone face, in *shadow mode*
//! — no production admission, no rendering change. Emits one JSONL row per
//! spline edge so a report can join it to the post-cone `diag.jsonl` and
//! classify the diagnosed spline-only band population.
//!
//! The three carrier queries mirror the relations the band boundary-witness
//! route consumes: `StraightLine` (surface-independent, maps to
//! `SourceCurveFamily::Line`), `ConstantLinearCoordinate` (the axial/generator
//! coordinate — a circumferential parallel), and `CircularArcOn` (a NURBS
//! lying on the surface's parallel circle through the trim start). The
//! candidate circle is derived structurally from the certified surface axis
//! and the curve's own start point, never fitted.

use std::path::Path;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{
    Table,
    step_geometry::{Curve3D, ElementarySurface, Surface},
};

use look::step::spline_carrier::{
    CarrierQuery, CirclePlacement, LinearCarrierCoordinate, certify_spline_carrier,
};

fn load_table(path: &str) -> anyhow::Result<Table> {
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

fn is_spline(curve: &Curve3D) -> bool {
    matches!(curve, Curve3D::BSplineCurve(_) | Curve3D::NurbsCurve(_))
}

fn spline_repr(curve: &Curve3D) -> &'static str {
    match curve {
        Curve3D::BSplineCurve(_) => "bspline",
        Curve3D::NurbsCurve(_) => "nurbs",
        _ => "other",
    }
}

struct SurfaceInfo {
    kind: &'static str,
    axis: Vector3,
    origin: Point3,
    coord_name: &'static str,
}

fn surface_info(surface: &Surface) -> Option<SurfaceInfo> {
    match surface {
        Surface::ElementarySurface(ElementarySurface::CylindricalSurface(_)) => {
            let cyl = look::step_cylinder_of(surface).ok()?;
            let schema = cyl.schema();
            Some(SurfaceInfo {
                kind: "cylinder",
                axis: schema.axis(),
                origin: schema.origin(),
                coord_name: "axial",
            })
        }
        Surface::ElementarySurface(ElementarySurface::ConicalSurface(_)) => {
            let cone = look::step_cone_of(surface).ok()?;
            let schema = cone.schema();
            Some(SurfaceInfo {
                kind: "cone",
                axis: schema.axis(),
                origin: schema.apex(),
                coord_name: "generator",
            })
        }
        _ => None,
    }
}

/// The surface's parallel circle through `point`, derived structurally from
/// the certified axis and the curve's own start point: the centre is the
/// orthogonal projection of `point` onto the axis, the radius is the
/// point-to-axis distance, the normal is the axis. No fitting.
fn parallel_circle(info: &SurfaceInfo, point: Point3) -> CirclePlacement {
    let center = info.origin + info.axis * (point - info.origin).dot(info.axis);
    let radius = (point - center).magnitude();
    CirclePlacement {
        center,
        radius,
        normal: info.axis,
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: cargo run --example spline_carrier_shadow -- MODEL.step [MORE.step ...]");
        return Ok(());
    }

    let mut total_edges = 0usize;
    for model_path in &args {
        let model_name = Path::new(model_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(model_path);
        let table = match load_table(model_path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("skip {model_path}: {e}");
                continue;
            }
        };

        let mut shells = Vec::new();
        for (_, shell) in table.shell.iter() {
            if let Ok((cshell, _)) = table.to_compressed_shell_with_losses(shell) {
                shells.push(cshell);
            }
        }

        for shell in &shells {
            for face in &shell.faces {
                let info = match surface_info(&face.surface) {
                    Some(i) => i,
                    None => continue,
                };
                let face_id = face
                    .provenance
                    .best_id()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "#?".into());

                for wire in &face.boundaries {
                    for edge_ref in wire {
                        let Some(edge) = shell.edges.get(edge_ref.index) else {
                            continue;
                        };
                        if !is_spline(&edge.curve) {
                            continue;
                        }
                        let (t0, t1) = edge.curve.range_tuple();
                        let start = edge.curve.subs(t0);
                        let repr = spline_repr(&edge.curve);

                        let straight = certify_spline_carrier(
                            &edge.curve,
                            CarrierQuery::StraightLine,
                            (t0, t1),
                        );
                        let coord = certify_spline_carrier(
                            &edge.curve,
                            CarrierQuery::ConstantLinearCoordinate(LinearCarrierCoordinate {
                                axis: info.axis,
                                origin: info.origin,
                                name: info.coord_name,
                            }),
                            (t0, t1),
                        );
                        let circle = certify_spline_carrier(
                            &edge.curve,
                            CarrierQuery::CircularArcOn(parallel_circle(&info, start)),
                            (t0, t1),
                        );

                        total_edges += 1;
                        println!(
                            "{{\"model\":\"{}\",\"face_id\":\"{}\",\"surface\":\"{}\",\"curve\":\"{}\",\"straight\":\"{}\",\"coord\":\"{}\",\"circle\":\"{}\"}}",
                            model_name,
                            face_id,
                            info.kind,
                            repr,
                            straight.tag(),
                            coord.tag(),
                            circle.tag()
                        );
                    }
                }
            }
        }
    }

    eprintln!("spline_carrier_shadow: {total_edges} spline edges certified");
    Ok(())
}
