//! The Devil's Teapot: a closed vessel assembled from landed capability —
//! revolved polygon-silhouette body (Pappus-exact volume oracle), swept
//! spout with `LinearCorrespondence` flare, swept-tube handle — with the
//! loft-spout and junction-blend variants booked behind the CC ports.
//!
//! Everything the CC program will land is called through `CcPorts`, and its
//! (current) deferral is recorded as report evidence, never silently skipped.

use std::path::Path;

use truck_base::cgmath64::{Point2, Point3, Vector3};
use truck_base::evidence::{Certified, Outcome};
use truck_geometry::constructive::{
    FrameLaw, ProfileLaw, SamplingPolicy, SpineCurve, SpineFrameRecipe,
};
use truck_geometry::{arrange, canonical::Curve};
use truck_modeling::{Line, Solid, spine_sweep};
use truck_shapeops::facade::revolve;

use crate::cc_ports::{CanalCert, CcPorts, RadiusLaw};
use crate::harness::{
    BrepReport, CcPortReport, FacetReport, ShowcaseReport, brep_volume, census_summary,
    record_export, write_report, write_step, write_stl, write_solid_stl,
};
use crate::profile::regular_polygon;
use crate::spine::spline_through_points;

/// The portable table (serde; lengths in units of body radius R = 1).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TeapotTable {
    /// `(z, radius)` outer-wall stations, ascending z; the last is the rim.
    pub body_stations: Vec<(f64, f64)>,
    /// Vessel wall thickness (inner wall = outer radius - thickness).
    pub wall_thickness: f64,
    /// Foot height (solid base under the inner cavity).
    pub foot_height: f64,
    /// Spout spine points `(x, y, z)`; must be planar for `FixedPlane`.
    pub spout_points: Vec<(f64, f64, f64)>,
    /// The spout plane normal.
    pub spout_plane_normal: (f64, f64, f64),
    /// Spout base ring radius.
    pub spout_r0: f64,
    /// Spout tip ring radius.
    pub spout_r1: f64,
    /// Spout ring vertex count (>= 3).
    pub spout_ring: usize,
    /// Handle spine points (planar).
    pub handle_points: Vec<(f64, f64, f64)>,
    /// The handle plane normal.
    pub handle_plane_normal: (f64, f64, f64),
    /// Handle tube radius.
    pub handle_radius: f64,
    /// Handle ring vertex count.
    pub handle_ring: usize,
    /// Stations along each swept tube.
    pub stations: usize,
}

impl Default for TeapotTable {
    fn default() -> Self {
        let body_stations: Vec<(f64, f64)> = vec![
            (0.0, 0.5),
            (2.0, 2.5),
            (4.0, 0.5),
        ];
        TeapotTable {
            body_stations,
            wall_thickness: 0.25,
            foot_height: 0.0,
            spout_points: vec![
                (0.95, 0.0, 0.50),
                (1.25, 0.0, 0.75),
                (1.45, 0.0, 1.05),
                (1.55, 0.0, 1.35),
                (1.62, 0.0, 1.62),
                (1.74, 0.0, 1.86),
            ],
            spout_plane_normal: (0.0, 1.0, 0.0),
            spout_r0: 0.26,
            spout_r1: 0.16,
            spout_ring: 24,
            handle_points: (-3..=3)
                .map(|i| {
                    let theta = std::f64::consts::FRAC_PI_2 * (i as f64) / 3.0;
                    let (s, c) = (theta.sin(), theta.cos());
                    (0.0, 0.88 + 0.62 * s, 1.03 + 0.60 * c)
                })
                .collect(),
            handle_plane_normal: (1.0, 0.0, 0.0),
            handle_radius: 0.09,
            handle_ring: 16,
            stations: 96,
        }
    }
}

/// The closed polygon silhouette of the vessel wall in the xz half-plane:
/// outer wall up, across the flat rim, inner wall down to the cavity floor,
/// with the closing edge along the foot bottom. The last point is the inner
/// foot corner; the loop closes back to the first point (outer foot corner).
pub fn body_silhouette(t: &TeapotTable) -> Vec<Point2> {
    let mut silhouette: Vec<Point2> = Vec::new();
    for (z, r) in &t.body_stations {
        silhouette.push(Point2::new(*r, *z));
    }
    let (rim_z, rim_r) = t.body_stations[t.body_stations.len() - 1];
    silhouette.push(Point2::new(rim_r - t.wall_thickness, rim_z));
    for (z, r) in t.body_stations.iter().rev() {
        if *z < rim_z {
            silhouette.push(Point2::new(*r - t.wall_thickness, *z));
        }
    }
    silhouette
}

/// The revolved body: line edges only (the landed revolve's v1 domain), so
/// every slanted silhouette segment sweeps a canonical cone, every horizontal
/// segment a plane annulus. The profile must lie strictly at x > 0. The
/// arrangement is built over the working copy `(x, 0, z) -> (x, z, 0)` per
/// the landed `revolve_p5` test pattern.
pub fn body_solid(t: &TeapotTable) -> Outcome<Solid> {
    let silhouette = body_silhouette(t);
    let mut profile_curves: Vec<Curve> = silhouette
        .windows(2)
        .map(|w| {
            Curve::from(Line(
                Point3::new(w[0].x, 0.0, w[0].y),
                Point3::new(w[1].x, 0.0, w[1].y),
            ))
        })
        .collect();
    let first = silhouette[0];
    let last = silhouette[silhouette.len() - 1];
    profile_curves.push(Curve::from(Line(
        Point3::new(last.x, 0.0, last.y),
        Point3::new(first.x, 0.0, first.y),
    )));
    let working: Vec<Curve> = profile_curves
        .iter()
        .map(|c| match c {
            Curve::Line(Line(a, b)) => {
                Curve::from(Line(Point3::new(a.x, a.z, 0.0), Point3::new(b.x, b.z, 0.0)))
            }
            _ => unreachable!("line-only profile by construction"),
        })
        .collect();
    let arrangement = arrange::arrange(&working, None)?.value;
    revolve(&profile_curves, &arrangement, std::f64::consts::TAU)
}

/// The Pappus oracle for the revolved body: V = 2 * pi * x_centroid * area,
/// exact for any revolved planar region without self-intersection, so the
/// battery can hold the revolve to quadrature-level accuracy.
pub fn pappus_estimate(t: &TeapotTable) -> f64 {
    let silhouette = body_silhouette(t);
    let n = silhouette.len();
    let mut area = 0.0;
    let mut cx = 0.0;
    for i in 0..n {
        let a = silhouette[i];
        let b = silhouette[(i + 1) % n];
        let cross = a.x * b.y - b.x * a.y;
        area += cross;
        cx += (a.x + b.x) * cross;
    }
    area = area.abs() / 2.0;
    cx = cx.abs() / (6.0 * area);
    2.0 * std::f64::consts::PI * cx * area
}

fn tube_recipe(
    spine: &Curve,
    ring: usize,
    r0: f64,
    r1: f64,
    plane_normal: (f64, f64, f64),
) -> SpineFrameRecipe<Curve, ProfileLaw, FrameLaw> {
    let start = regular_polygon(r0, ring, 0.0).expect("start ring");
    let end = regular_polygon(r1, ring, 0.0).expect("end ring");
    SpineFrameRecipe::new(
        spine.clone(),
        ProfileLaw::LinearCorrespondence { start, end },
        FrameLaw::FixedPlane {
            normal: Vector3::new(plane_normal.0, plane_normal.1, plane_normal.2),
        },
    )
}

fn realize_tube(
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

fn facet_stl(
    report: &mut ShowcaseReport,
    out_dir: &Path,
    name: &str,
    recipe: &SpineFrameRecipe<Curve, ProfileLaw, FrameLaw>,
    ring: usize,
    stations: usize,
) {
    let (s0, s1) = recipe.spine.domain();
    let Ok(station_list) = (SamplingPolicy::UniformCount { spine: stations }).resolve(s0, s1)
    else {
        return;
    };
    if let Ok(result) = truck_modeling::facet_sweep::facet_sweep(recipe, &station_list, ring) {
        let p = out_dir.join(format!("{name}.stl"));
        record_export(report, "stl", &p, write_stl(&result.mesh, &p));
    }
}

/// Builds the teapot into `out_dir`.
pub fn build(t: &TeapotTable, out_dir: &Path, ports: &dyn CcPorts) -> Result<ShowcaseReport, String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;

    let mut report = ShowcaseReport {
        item: "teapot".to_string(),
        spine_total_length: 0.0,
        spine_samples: t.spout_points.len(),
        facets: Vec::new(),
        breps: Vec::new(),
        exports: Vec::new(),
        booleans: Vec::new(),
        cc_ports: Vec::new(),
    };

    let body = body_solid(t);
    match &body {
        Ok(Certified { value, .. }) => {
            let (pairs, nonpair, orientation_sum) = census_summary(value);
            report.breps.push(BrepReport {
                law: "body_revolve".to_string(),
                face_count: value.face_iter().count(),
                edge_use_pairs: pairs,
                edge_nonpair_uses: nonpair,
                edge_orientation_sum: orientation_sum,
                signed_volume: brep_volume(value),
                refusal: None,
            });
            record_export(
                &mut report,
                "step",
                &out_dir.join("teapot_body.step"),
                write_step(value, &out_dir.join("teapot_body.step")),
            );
            record_export(
                &mut report,
                "stl",
                &out_dir.join("teapot_body.stl"),
                write_solid_stl(value, &out_dir.join("teapot_body.stl")),
            );
        }
        Err(e) => report.breps.push(BrepReport {
            law: "body_revolve".to_string(),
            face_count: 0,
            edge_use_pairs: 0,
            edge_nonpair_uses: 0,
            edge_orientation_sum: 0,
            signed_volume: f64::NAN,
            refusal: Some(format!("{e:?}")),
        }),
    }

    let spout_pts: Vec<Point3> = t
        .spout_points
        .iter()
        .map(|(x, y, z)| Point3::new(*x, *y, *z))
        .collect();
    let spout_spine = spline_through_points(&spout_pts).map_err(|e| e.to_string())?;
    let spout_recipe = tube_recipe(
        &spout_spine,
        t.spout_ring,
        t.spout_r0,
        t.spout_r1,
        t.spout_plane_normal,
    );
    let spout = realize_tube(
        "spout_sweep",
        &spout_recipe,
        t.spout_ring,
        t.stations,
        &mut report.facets,
        &mut report.breps,
    );

    let handle_pts: Vec<Point3> = t
        .handle_points
        .iter()
        .map(|(x, y, z)| Point3::new(*x, *y, *z))
        .collect();
    let handle_spine = spline_through_points(&handle_pts).map_err(|e| e.to_string())?;
    let handle_recipe = tube_recipe(
        &handle_spine,
        t.handle_ring,
        t.handle_radius,
        t.handle_radius,
        t.handle_plane_normal,
    );
    let handle = realize_tube(
        "handle_sweep",
        &handle_recipe,
        t.handle_ring,
        t.stations,
        &mut report.facets,
        &mut report.breps,
    );

    if let Some(spout) = &spout {
        record_export(
            &mut report,
            "step",
            &out_dir.join("teapot_spout.step"),
            write_step(spout, &out_dir.join("teapot_spout.step")),
        );
    }
    if let Some(handle) = &handle {
        record_export(
            &mut report,
            "step",
            &out_dir.join("teapot_handle.step"),
            write_step(handle, &out_dir.join("teapot_handle.step")),
        );
    }
    facet_stl(&mut report, out_dir, "teapot_spout", &spout_recipe, t.spout_ring, t.stations);
    facet_stl(&mut report, out_dir, "teapot_handle", &handle_recipe, t.handle_ring, t.stations);

    match (&body, &spout) {
        (Ok(Certified { value: body, .. }), Some(spout)) => {
            let mut budget = truck_base::evidence::Budget::new(100_000, 100_000, 100);
            let union = truck_shapeops::facade::boolean_op(
                body,
                truck_shapeops::facade::Mode::Add,
                spout,
                &mut budget,
            );
            report.booleans.push(CcPortReport {
                port: "boolean_union_body_spout".to_string(),
                status: if union.is_ok() { "certified" } else { "refused" }.to_string(),
                detail: Some(match &union {
                    Ok(u) => format!("union faces={}", u.value.face_iter().count()),
                    Err(e) => format!("{e:?}"),
                }),
            });
        }
        _ => report.booleans.push(CcPortReport {
            port: "boolean_union_body_spout".to_string(),
            status: "skipped".to_string(),
            detail: Some("an input solid refused".to_string()),
        }),
    }

    let loft_spout = ports.loft(&[], &Default::default());
    report.cc_ports.push(CcPortReport {
        port: "loft_spout_variant".to_string(),
        status: if loft_spout.is_ok() { "certified" } else { "deferred" }.to_string(),
        detail: Some(match &loft_spout {
            Ok(_) => "loft realized".to_string(),
            Err(e) => format!("{e:?}"),
        }),
    });
    if let Some(spout) = &spout {
        let blend = ports.blend_var_radius(
            spout,
            (
                Point3::new(t.spout_points[0].0, t.spout_points[0].1, t.spout_points[0].2),
                Point3::new(
                    t.spout_points[t.spout_points.len() - 1].0,
                    t.spout_points[t.spout_points.len() - 1].1,
                    t.spout_points[t.spout_points.len() - 1].2,
                ),
            ),
            &RadiusLaw::Linear(0.09, 0.015),
        );
        report.cc_ports.push(CcPortReport {
            port: "junction_blend_var_radius".to_string(),
            status: if blend.is_ok() { "certified" } else { "deferred" }.to_string(),
            detail: Some(match &blend {
                Ok(_) => "blend realized".to_string(),
                Err(e) => format!("{e:?}"),
            }),
        });
    }
    let canal: Outcome<CanalCert> = ports.canal_regularity(&spout_spine, t.spout_r1);
    report.cc_ports.push(CcPortReport {
        port: "canal_regularity_spout_spine".to_string(),
        status: if canal.is_ok() { "certified" } else { "deferred" }.to_string(),
        detail: Some(match &canal {
            Ok(c) => format!("regular={} min_r={}", c.value.regular, c.value.min_curvature_radius),
            Err(e) => format!("{e:?}"),
        }),
    });

    write_report(&report, &out_dir.join("teapot_report.json")).map_err(|e| e.to_string())?;
    Ok(report)
}
