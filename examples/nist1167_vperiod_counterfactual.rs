//! Counterfactual probe: run the exact production tessellation path on
//! `nist_ctc_02_asme1_ap203.stp` with the source-declared V closure of the
//! two closed B-spline surfaces (#506 behind #1167, #507 behind #1169)
//! supplied to the lattice, and compare against the clean state.
//!
//! The lattice override is keyed on the *converted surface identity*, not on
//! face ids (which no production code may contain), and is applied only to the
//! two surfaces whose source STEP entity declares `v_closed = .T.`. Everything
//! else goes through `look::step_lattice_of` unchanged.
//!
//! Three lattice modes are exercised:
//!   clean    - `look::step_lattice_of` (current production; V nonperiodic)
//!   uncert   - V declared period 1.0 as `Uncertified` (drives the lift via
//!              `declared_period`, never offered as a generator)
//!   exact    - V period 1.0 as `Exact` with the revolution witness (a probe
//!              abuse of the witness; tests whether generator-only paths are
//!              part of the mechanism)
//!
//! This is a probe only; it writes no production code.

use std::env;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{
    Table,
    step_geometry::{Curve3D, Surface},
};

type Cshell = truck_topology::compress::CompressedShell<Point3, Curve3D, Surface>;

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

fn bbox_diag_of(mesh: &PolygonMesh) -> f64 {
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
    (hi - lo).magnitude()
}

fn max_edge_of(mesh: &PolygonMesh) -> f64 {
    let pos = mesh.positions();
    mesh.tri_faces()
        .iter()
        .map(|tri| {
            let a = pos[tri[0].pos];
            let b = pos[tri[1].pos];
            let c = pos[tri[2].pos];
            (b - a)
                .magnitude()
                .max((c - b).magnitude())
                .max((a - c).magnitude())
        })
        .fold(0.0f64, f64::max)
}

/// V-seam compatibility check: S(u,0) == S(u,1) and Sv(u,0) == Sv(u,1) at
/// several U samples. The prompt's remaining cheap check before production.
fn seam_compatibility(surface: &Surface) {
    if let Surface::BSplineSurface(surf) = surface {
        let (uk, vk) = surf.knot_vecs();
        let (_, vd) = surf.degrees();
        let lo = vk[vd];
        let hi = vk[vk.len() - 1 - vd];
        let mut pos_max = 0.0f64;
        let mut dv_max = 0.0f64;
        for i in 0..9 {
            let u = 0.125 * i as f64;
            let a = surf.subs(u, lo);
            let b = surf.subs(u, hi);
            pos_max = pos_max.max(a.distance(b));
            let da = surf.vder(u, lo);
            let db = surf.vder(u, hi);
            dv_max = dv_max.max((da - db).magnitude());
        }
        let vspan = hi - lo;
        let _ = (uk, vd);
        println!(
            "  seam: S(u,{lo}) vs S(u,{hi}) max_pos_res={pos_max:.3e} \
             max_Sv_res={dv_max:.3e} active_v_span={vspan}"
        );
    }
}

fn report_face(
    tag: &str,
    mesh_opt: &Option<PolygonMesh>,
    failure: Option<truck_meshalgo::tessellation::TessellationFailureReason>,
    src_surface: &Surface,
) {
    match mesh_opt {
        Some(mesh) if !mesh.faces().is_empty() => {
            let ntri = mesh.tri_faces().len();
            let npos = mesh.positions().len();
            let area = area_of(mesh);
            let diag = bbox_diag_of(mesh);
            let max_edge = max_edge_of(mesh);
            let mut vmax = 0.0f64;
            let mut von = 0usize;
            let mut off: Vec<f64> = Vec::new();
            let mut uv_lo = Point2::new(f64::INFINITY, f64::INFINITY);
            let mut uv_hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
            let mut far_out = 0usize;
            for (vi, v) in mesh.positions().iter().enumerate() {
                let res = match src_surface.search_parameter(*v, None, 200) {
                    Some(uv) => src_surface.subs(uv.0, uv.1).distance(*v),
                    None => f64::INFINITY,
                };
                vmax = vmax.max(res);
                if res <= 1.0e-3 {
                    von += 1;
                } else {
                    off.push(res);
                }
                if let Some(uv) = mesh.uv_coords().get(vi) {
                    uv_lo.x = uv_lo.x.min(uv.x);
                    uv_lo.y = uv_lo.y.min(uv.y);
                    uv_hi.x = uv_hi.x.max(uv.x);
                    uv_hi.y = uv_hi.y.max(uv.y);
                    if uv.y > 1.1 || uv.y < -0.1 || uv.x > 1.1 || uv.x < -0.1 {
                        far_out += 1;
                    }
                }
            }
            let off_str = if off.is_empty() {
                "none".to_string()
            } else {
                off.iter()
                    .map(|r| format!("{r:.2e}"))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            let (ulox, uloy) = (uv_lo.x, uv_lo.y);
            let (uhix, uhixy) = (uv_hi.x, uv_hi.y);
            println!(
                "FACE {tag}: RENDERED tris={ntri} verts={npos} area={area:.3} diag={diag:.3} \
                     max_edge={max_edge:.3} on_surface={von}/{npos} off_surface={off_str} \
                     uv_lo=({ulox:.3},{uloy:.3}) uv_hi=({uhix:.3},{uhixy:.3}) \
                     out_of_domain_uv={far_out}"
            );
            if far_out > 0 {
                let mut shown = 0;
                for (vi, v) in mesh.positions().iter().enumerate() {
                    if let Some(uv) = mesh.uv_coords().get(vi) {
                        if uv.y > 1.1 || uv.y < -0.1 || uv.x > 1.1 || uv.x < -0.1 {
                            let (ux, uy) = (uv.x, uv.y);
                            let (vx, vy, vz) = (v.x, v.y, v.z);
                            println!(
                                "  OOD[{shown}] uv=({ux:.4},{uy:.4}) pos=({vx:.4},{vy:.4},{vz:.4})"
                            );
                            shown += 1;
                            if shown >= 6 {
                                break;
                            }
                        }
                    }
                }
            }
        }
        Some(mesh) => {
            let f = failure
                .map(|r| format!("{r:?}"))
                .unwrap_or_else(|| "-".into());
            println!(
                "FACE {tag}: EMPTY tris=0 verts={} failure={f}",
                mesh.positions().len()
            );
        }
        None => {
            let f = failure
                .map(|r| format!("{r:?}"))
                .unwrap_or_else(|| "-".into());
            println!("FACE {tag}: NO SURFACE failure={f}");
        }
    }
}

fn main() -> anyhow::Result<()> {
    use look::step::policy_geometry::PolicySurface;
    use truck_meshalgo::tessellation::domain::lattice::{Axis, AxisPeriodStatus, CertifiedLattice};

    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: nist1167_vperiod_counterfactual MODEL.step");
        return Ok(());
    }
    let model_path = &args[0];
    let table = load(model_path)?;
    let mut done = false;
    for (shell_idx, (&shell_id, shell)) in table.shell.iter().enumerate() {
        let (cshell, losses) = table
            .to_compressed_shell_with_losses(shell_id, shell)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        println!(
            "=== shell #{shell_idx} conversion losses: {} ===",
            losses.len()
        );

        // Identify the two target surfaces by face provenance (#1167, #1169).
        let mut target_surfaces: Vec<Surface> = Vec::new();
        let mut target_indices: Vec<(u64, usize)> = Vec::new();
        for (i, f) in cshell.faces.iter().enumerate() {
            if let Some(id) = f.provenance.best_id() {
                let g = id.get();
                if g == 1167 || g == 1169 {
                    target_surfaces.push(f.surface.clone());
                    target_indices.push((g, i));
                }
            }
        }
        if target_surfaces.is_empty() {
            continue;
        }
        for s in &target_surfaces {
            seam_compatibility(s);
        }

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

        let modes: [(&str, CertifiedLattice); 3] = [
            (
                "clean",
                CertifiedLattice::NON_PERIODIC,
            ),
            (
                "uncert-v1.0",
                CertifiedLattice {
                    u: AxisPeriodStatus::NonPeriodic,
                    v: AxisPeriodStatus::Uncertified { declared: 1.0 },
                },
            ),
            (
                "exact-v1.0",
                CertifiedLattice {
                    u: AxisPeriodStatus::NonPeriodic,
                    v: AxisPeriodStatus::Exact {
                        period: 1.0,
                        witness: truck_meshalgo::tessellation::domain::lattice::PeriodWitness::ExactRevolutionAngle,
                    },
                },
            ),
        ];

        for (mode, witness_lattice) in modes {
            println!("--- MODE {mode} ---");
            let lattice_of = |s: &PolicySurface| {
                let inner = s.inner();
                if target_surfaces.iter().any(|t| t == inner) {
                    witness_lattice
                } else {
                    look::step_lattice_of(inner)
                }
            };
            let outcome = look::step::policy_geometry::wrap_shell(
                cshell.clone(),
                look::step::meshing_policy::MeshingPolicy::DEFAULT,
            )
            .robust_triangulation_with_torus_outcome(
                tol,
                lattice_of,
                |s: &PolicySurface| look::step_support_schema_of(s.inner()),
                |c: &look::step::policy_geometry::PolicyCurve| {
                    look::step_curve_schema_of(c.inner())
                },
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
            for &(g, i) in &target_indices {
                let tag = format!("#{g}");
                let failure = outcome.face_failures.get(i).cloned().flatten();
                let fail_str = failure
                    .as_ref()
                    .map(|f| format!("{:?}", f.reason))
                    .unwrap_or_else(|| "-".into());
                let lattice_for_face = {
                    // Recompute the lattice this face would have received.
                    let surface = &cshell.faces[i].surface;
                    if target_surfaces.iter().any(|t| t == surface) {
                        witness_lattice
                    } else {
                        look::step_lattice_of(surface)
                    }
                };
                let rank = lattice_for_face.certified_rank();
                let pu = lattice_for_face.declared_u_period().is_some();
                let pv = lattice_for_face.declared_v_period().is_some();
                let vgen = lattice_for_face.v_generator().is_some();
                println!(
                    "META {tag} mode={mode} lattice_rank={rank} periodic=u:{pu}/v:{pv} \
                     v_generator={vgen} outcome={fail_str}"
                );
                report_face(
                    &tag,
                    &meshed.faces[i].surface,
                    failure.as_ref().map(|f| f.reason),
                    &cshell.faces[i].surface,
                );
            }
            // AXIS bookkeeping helper is unused; Axis imported for the witness.
            let _ = Axis::U;
        }
        done = true;
        break;
    }
    if !done {
        anyhow::bail!("no shell found");
    }
    Ok(())
}
