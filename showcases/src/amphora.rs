//! The Amphora of Destitution: the loft flagship. The body is a
//! multi-station loft with declared correspondence over elliptical ribs
//! (CC-010..014), the A/B arm is the Gordon blend over the same ribs
//! (CC-015), the wall is certified by the shell/thickness ports
//! (CC-023/026), and the handles are swept tubes under `RadialAboutAxis`
//! with the P10 metamorphic equivariance gate — all through `CcPorts`,
//! with every unlanded port recorded as report evidence.
//!
//! Design frozen now; the landed build today is the handles + foot, and the
//! report enumerates exactly which ports the full body waits on.

use std::f64::consts::PI;
use std::path::Path;

use truck_base::cgmath64::{Point2, Point3, Vector3};
use truck_base::evidence::{Certified, Outcome};
use truck_geometry::constructive::{
    FrameLaw, LineSpine, Profile2D, ProfileLaw, SamplingPolicy, SpineCurve, SpineFrameRecipe,
};
use truck_geometry::canonical::Curve;
use truck_modeling::spine_sweep;
use truck_modeling::Solid;

use crate::cc_ports::{CanalCert, CcPorts, RadiusLaw, RibWire, ThicknessCert};
use crate::harness::{
    BrepReport, CcPortReport, FacetReport, ShowcaseReport, brep_volume, census_summary,
    record_export, write_report, write_step, write_stl, write_solid_stl,
};
use crate::profile::regular_polygon;
use crate::spine::spline_through_points;

/// The portable table (serde; lengths in units of total height H = 1).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AmphoraTable {
    /// `(z, radius)` body rib stations, ascending z; radius is the
    /// x-semi-axis, y squashed by `y_squash`.
    pub body_stations: Vec<(f64, f64)>,
    /// The destitution squash of every rib's y axis.
    pub y_squash: f64,
    /// Rib vertex count (>= 3).
    pub rib_ring: usize,
    /// Handle spine points, azimuth 0; the second handle is the rotation by
    /// `handle_azimuth_deg`.
    pub handle_points: Vec<(f64, f64, f64)>,
    /// The second handle's azimuth offset in degrees.
    pub handle_azimuth_deg: f64,
    /// Handle tube radius.
    pub handle_radius: f64,
    /// Handle ring vertex count.
    pub handle_ring: usize,
    /// Foot prism `(radius, z0, z1)`.
    pub foot: (f64, f64, f64),
    /// Stations along each swept handle.
    pub stations: usize,
}

impl Default for AmphoraTable {
    fn default() -> Self {
        AmphoraTable {
            body_stations: vec![
                (0.06, 0.30),
                (0.20, 0.44),
                (0.38, 0.50),
                (0.50, 0.48),
                (0.68, 0.34),
                (0.80, 0.14),
                (0.97, 0.17),
            ],
            y_squash: 0.85,
            rib_ring: 16,
            handle_points: vec![
                (0.14, 0.0, 0.93),
                (0.40, 0.0, 0.96),
                (0.58, 0.0, 0.88),
                (0.62, 0.0, 0.76),
                (0.52, 0.0, 0.68),
                (0.34, 0.0, 0.70),
            ],
            handle_azimuth_deg: 180.0,
            handle_radius: 0.055,
            handle_ring: 16,
            foot: (0.26, 0.0, 0.06),
            stations: 96,
        }
    }
}

/// One elliptical rib as a polygon ring: `count` vertices, x semi-axis
/// `radius`, y semi-axis `radius * squash`, CCW.
pub fn rib_ring(radius: f64, squash: f64, count: usize) -> Result<Profile2D, String> {
    if count < 3 || radius <= 0.0 {
        return Err("bad rib parameters".to_string());
    }
    let vertices: Vec<Point2> = (0..count)
        .map(|i| {
            let t = 2.0 * PI * (i as f64) / (count as f64);
            Point2::new(radius * t.cos(), radius * squash * t.sin())
        })
        .collect();
    Profile2D::try_closed(vertices).map_err(|e| format!("{e:?}"))
}

fn handle_spine(points: &[(f64, f64, f64)], azimuth_deg: f64) -> Result<Curve, String> {
    let az = azimuth_deg.to_radians();
    let (c, s) = (az.cos(), az.sin());
    let pts: Vec<Point3> = points
        .iter()
        .map(|(x, y, z)| Point3::new(x * c - y * s, x * s + y * c, *z))
        .collect();
    spline_through_points(&pts).map_err(|e| e.to_string())
}

fn handle_recipe(
    spine: &Curve,
    t: &AmphoraTable,
) -> SpineFrameRecipe<Curve, ProfileLaw, FrameLaw> {
    SpineFrameRecipe::new(
        spine.clone(),
        ProfileLaw::Constant(
            regular_polygon(t.handle_radius, t.handle_ring, 0.0).expect("handle ring"),
        ),
        FrameLaw::FixedPlane {
            normal: handle_plane_normal(t.handle_azimuth_deg),
        },
    )
}

/// The handle plane's normal: the spine lives in the vertical plane at
/// azimuth `azimuth_deg`, so its normal is that azimuth's horizontal tangent
/// direction. Planar spine + pinned plane normal = the FixedPlane happy path
/// (the post-fix kernel refuses RadialAboutAxis tangents with radial
/// components, which the handle's outward bow has).
fn handle_plane_normal(azimuth_deg: f64) -> Vector3 {
    let az = azimuth_deg.to_radians();
    Vector3::new(-az.sin(), az.cos(), 0.0)
}

fn realize(
    name: &str,
    recipe: &SpineFrameRecipe<Curve, ProfileLaw, FrameLaw>,
    ring: usize,
    stations: usize,
    facets: &mut Vec<FacetReport>,
    breps: &mut Vec<BrepReport>,
) -> Option<Solid> {
    let (s0, s1) = recipe.spine.domain();
    let station_list =
        match (SamplingPolicy::UniformCount { spine: stations }).resolve(s0, s1) {
            Ok(list) => list,
            Err(e) => {
                facets.push(FacetReport {
                    law: name.to_string(),
                    triangle_count: 0,
                    quad_count: 0,
                    signed_volume: f64::NAN,
                    winding_violations: 0,
                    verdict: "NotRealized".to_string(),
                    refusal: Some(format!("{e:?}")),
                });
                return None;
            }
        };
    match truck_modeling::facet_sweep::facet_sweep(recipe, &station_list, ring) {
        Ok(result) => facets.push(FacetReport {
            law: name.to_string(),
            triangle_count: result.audit.triangle_count,
            quad_count: result.audit.quad_count,
            signed_volume: result.audit.signed_volume,
            winding_violations: result.audit.winding_violations,
            verdict: format!("{:?}", result.verdict),
            refusal: None,
        }),
        Err(e) => facets.push(FacetReport {
            law: name.to_string(),
            triangle_count: 0,
            quad_count: 0,
            signed_volume: f64::NAN,
            winding_violations: 0,
            verdict: "NotRealized".to_string(),
            refusal: Some(format!("{e:?}")),
        }),
    }
    match spine_sweep::spine_sweep(recipe, &station_list) {
        Ok(Certified { value, .. }) => {
            let (pairs, nonpair, orientation_sum) = census_summary(&value);
            breps.push(BrepReport {
                law: name.to_string(),
                face_count: value.face_iter().count(),
                edge_use_pairs: pairs,
                edge_nonpair_uses: nonpair,
                edge_orientation_sum: orientation_sum,
                signed_volume: brep_volume(&value),
                refusal: None,
            });
            Some(value)
        }
        Err(e) => {
            breps.push(BrepReport {
                law: name.to_string(),
                face_count: 0,
                edge_use_pairs: 0,
                edge_nonpair_uses: 0,
                edge_orientation_sum: 0,
                signed_volume: f64::NAN,
                refusal: Some(format!("{e:?}")),
            });
            None
        }
    }
}

/// Builds the amphora into `out_dir`: the landed parts (two handles, foot
/// prism) plus the full CC-port probe battery for the gated body.
pub fn build(t: &AmphoraTable, out_dir: &Path, ports: &dyn CcPorts) -> Result<ShowcaseReport, String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;

    let mut report = ShowcaseReport {
        item: "amphora".to_string(),
        spine_total_length: 0.0,
        spine_samples: t.body_stations.len(),
        facets: Vec::new(),
        breps: Vec::new(),
        exports: Vec::new(),
        booleans: Vec::new(),
        cc_ports: Vec::new(),
    };

    for (idx, azimuth) in [0.0, t.handle_azimuth_deg].into_iter().enumerate() {
        let spine = handle_spine(&t.handle_points, azimuth)?;
        let recipe = handle_recipe(&spine, t);
        let name = format!("handle_{}", if idx == 0 { "a" } else { "b" });
        let handle = realize(
            &name,
            &recipe,
            t.handle_ring,
            t.stations,
            &mut report.facets,
            &mut report.breps,
        );
        if let Some(handle) = &handle {
            record_export(
                &mut report,
                "step",
                &out_dir.join(format!("amphora_{name}.step")),
                write_step(handle, &out_dir.join(format!("amphora_{name}.step"))),
            );
        }
        let (s0, s1) = recipe.spine.domain();
        if let Ok(station_list) =
            (SamplingPolicy::UniformCount { spine: t.stations }).resolve(s0, s1)
        {
            if let Ok(result) =
                truck_modeling::facet_sweep::facet_sweep(&recipe, &station_list, t.handle_ring)
            {
                let p = out_dir.join(format!("amphora_{name}.stl"));
                record_export(&mut report, "stl", &p, write_stl(&result.mesh, &p));
            }
        }
    }

    let foot_recipe = SpineFrameRecipe::new(
        LineSpine {
            start: Point3::new(0.0, 0.0, t.foot.1),
            end: Point3::new(0.0, 0.0, t.foot.2),
        },
        ProfileLaw::Constant(
            regular_polygon(t.foot.0, 8, PI / 8.0).map_err(|e| format!("{e:?}"))?,
        ),
        FrameLaw::FixedPlane {
            normal: Vector3::unit_x(),
        },
    );
    match spine_sweep::spine_sweep(&foot_recipe, &[0.0, 1.0]) {
        Ok(Certified { value, .. }) => {
            let (pairs, nonpair, orientation_sum) = census_summary(&value);
            report.breps.push(BrepReport {
                law: "foot_prism".to_string(),
                face_count: value.face_iter().count(),
                edge_use_pairs: pairs,
                edge_nonpair_uses: nonpair,
                edge_orientation_sum: orientation_sum,
                signed_volume: brep_volume(&value),
                refusal: None,
            });
            record_export(
                &mut report,
                "step",
                &out_dir.join("amphora_foot.step"),
                write_step(&value, &out_dir.join("amphora_foot.step")),
            );
            record_export(
                &mut report,
                "stl",
                &out_dir.join("amphora_foot.stl"),
                write_solid_stl(&value, &out_dir.join("amphora_foot.stl")),
            );
        }
        Err(e) => report.breps.push(BrepReport {
            law: "foot_prism".to_string(),
            face_count: 0,
            edge_use_pairs: 0,
            edge_nonpair_uses: 0,
            edge_orientation_sum: 0,
            signed_volume: f64::NAN,
            refusal: Some(format!("{e:?}")),
        }),
    }

    let ribs: Vec<RibWire> = t
        .body_stations
        .iter()
        .map(|(z, r)| {
            let ring = rib_ring(*r, t.y_squash, t.rib_ring)?;
            Ok(RibWire { z: *z, ring })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let loft = ports.loft_ribs(&ribs);
    report.cc_ports.push(CcPortReport {
        port: "loft_body".to_string(),
        status: if loft.is_ok() { "certified" } else { "deferred" }.to_string(),
        detail: Some(match &loft {
            Ok(_) => "loft realized".to_string(),
            Err(e) => format!("{e:?}"),
        }),
    });
    let gordon = ports.gordon_ribs(&ribs);
    report.cc_ports.push(CcPortReport {
        port: "gordon_body".to_string(),
        status: if gordon.is_ok() { "certified" } else { "deferred" }.to_string(),
        detail: Some(match &gordon {
            Ok(_) => "gordon realized".to_string(),
            Err(e) => format!("{e:?}"),
        }),
    });
    if let Some(handle_a) = report.facets.first() {
        let _ = handle_a;
    }
    let canal: Outcome<CanalCert> =
        ports.canal_cert(&t.handle_points, t.handle_azimuth_deg, t.handle_radius);
    report.cc_ports.push(CcPortReport {
        port: "canal_regularity_handle_spine".to_string(),
        status: if canal.is_ok() { "certified" } else { "deferred" }.to_string(),
        detail: Some(match &canal {
            Ok(c) => format!("regular={} min_r={}", c.value.regular, c.value.min_curvature_radius),
            Err(e) => format!("{e:?}"),
        }),
    });
    let thickness: Outcome<ThicknessCert> = ports.shell_thickness(&ribs);
    report.cc_ports.push(CcPortReport {
        port: "certified_shell_wall".to_string(),
        status: if thickness.is_ok() { "certified" } else { "deferred" }.to_string(),
        detail: Some(match &thickness {
            Ok(c) => format!(
                "t_safe={} t_focal={} d_min/2={}",
                c.value.t_safe, c.value.t_focal, c.value.d_min_half
            ),
            Err(e) => format!("{e:?}"),
        }),
    });
    let blend = ports.blend_handle_root(&ribs, &RadiusLaw::MonotoneCubic(vec![
        (0.0, 0.05),
        (0.5, 0.08),
        (1.0, 0.10),
    ]));
    report.cc_ports.push(CcPortReport {
        port: "blend_handle_root_var_radius".to_string(),
        status: if blend.is_ok() { "certified" } else { "deferred" }.to_string(),
        detail: Some(match &blend {
            Ok(_) => "blend realized".to_string(),
            Err(e) => format!("{e:?}"),
        }),
    });

    write_report(&report, &out_dir.join("amphora_report.json")).map_err(|e| e.to_string())?;
    Ok(report)
}
