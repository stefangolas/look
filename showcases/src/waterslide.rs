//! The Waterslide of Ruin: one composite spine (drop → helix → runout), one
//! closed U-chute profile widening into the runout, realized under THREE
//! frame laws — `ParallelTransport`, `ArchitecturalUp`, `RadialAboutAxis` —
//! through both realization backends (`facet_sweep` mesh, `spine_sweep`
//! authored-topology BREP), plus the splashdown pool and support tower as
//! authored prisms and the chute∪pool union through the landed boolean.
//!
//! The frame-law trio is the demo's centerpiece: the same spine under three
//! frame laws produces three visibly different chutes (the transported frame
//! stays put through the helix; the architectural-up frame rolls with the up
//! vector; the radial frame tips the chute outward), and the difference is
//! certified, not eyeballed: three facet audits and three BREP volumes land
//! in one report.

use std::path::Path;

use truck_base::cgmath64::{Point3, Vector3};
use truck_base::evidence::{Certified, Outcome};
use truck_geometry::constructive::{
    FrameLaw, LineSpine, Profile2D, ProfileLaw, SamplingPolicy, ScalarLaw, SpineCurve,
    SpineFrameRecipe,
};
use truck_modeling::spine_sweep;
use truck_modeling::{Curve, Solid, facet_sweep};
use truck_shapeops::facade::{Mode, boolean_op};

use crate::cc_ports::{CanalCert, CcPorts};
use crate::harness::{BrepReport, CcPortReport, FacetReport, ShowcaseReport, brep_volume, census_summary, record_export, write_report, write_step, write_stl};
use crate::profile::{regular_polygon, trapezoid_chute};
use crate::spine::{CompositeSpec, composite_path, shift_to_ground, spline_from_path};

/// The portable table (serde). Every field is a physical length/angle; the
/// whole model is determined by this data.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WaterslideTable {
    pub drop_length: f64,
    pub drop_angle_deg: f64,
    pub transition_radius: f64,
    pub helix_radius: f64,
    pub helix_turns: f64,
    pub helix_slope_deg: f64,
    pub runout_length: f64,
    pub spine_samples: usize,
    pub chute_width: f64,
    pub chute_wall_height: f64,
    pub chute_top_fraction: f64,
    pub chute_wall_thickness: f64,
    pub chute_floor_thickness: f64,
    pub runout_widening: f64,
    pub stations: usize,
    pub pool_radius: f64,
    pub pool_depth: f64,
    pub pool_rim_height: f64,
    pub pool_center_fraction: f64,
    pub tower_radius: f64,
    pub tower_clearance: f64,
}

impl Default for WaterslideTable {
    fn default() -> Self {
        WaterslideTable {
            drop_length: 8.0,
            drop_angle_deg: 50.0,
            transition_radius: 6.0,
            helix_radius: 5.4,
            helix_turns: 2.5,
            helix_slope_deg: 12.0,
            runout_length: 9.0,
            spine_samples: 48,
            chute_width: 1.2,
            chute_wall_height: 0.54,
            chute_top_fraction: 0.35,
            chute_wall_thickness: 0.072,
            chute_floor_thickness: 0.096,
            runout_widening: 1.35,
            stations: 120,
            pool_radius: 3.0,
            pool_depth: 1.2,
            pool_rim_height: 0.25,
            pool_center_fraction: 0.5,
            tower_radius: 0.9,
            tower_clearance: 0.4,
        }
    }
}

impl WaterslideTable {
    /// The composite-path spec view of the table.
    pub fn path_spec(&self) -> CompositeSpec {
        CompositeSpec {
            drop_length: self.drop_length,
            drop_angle_deg: self.drop_angle_deg,
            transition_radius: self.transition_radius,
            helix_radius: self.helix_radius,
            helix_turns: self.helix_turns,
            helix_slope_deg: self.helix_slope_deg,
            runout_length: self.runout_length,
            samples: self.spine_samples,
        }
    }
}

/// The chute profile ring size (vertex count) — `facet_sweep`'s
/// `ring_resolution` must equal the profile vertex count.
pub const CHUTE_RING: usize = 4;
/// The prism (pool/tower) ring size.
pub const PRISM_RING: usize = 8;

fn chute_profile(t: &WaterslideTable) -> Result<Profile2D, String> {
    trapezoid_chute(t.chute_width, t.chute_wall_height, t.chute_top_fraction)
        .map_err(|e| format!("{e:?}"))
}

fn chute_recipe(
    spine: &Curve,
    profile: &Profile2D,
    t: &WaterslideTable,
    law: FrameLaw,
) -> SpineFrameRecipe<Curve, ProfileLaw, FrameLaw> {
    SpineFrameRecipe::new(
        spine.clone(),
        ProfileLaw::Scale {
            profile: profile.clone(),
            scale: ScalarLaw::Linear {
                start: 1.0,
                end: t.runout_widening,
            },
        },
        law,
    )
}

fn realize(
    law_name: &str,
    recipe: &SpineFrameRecipe<Curve, ProfileLaw, FrameLaw>,
    ring: usize,
    stations: usize,
    facets: &mut Vec<FacetReport>,
    breps: &mut Vec<BrepReport>,
) -> Option<Solid> {
    let (s0, s1) = recipe.spine.domain();
    let station_list = match (SamplingPolicy::UniformCount { spine: stations }).resolve(s0, s1) {
        Ok(list) => list,
        Err(e) => {
            facets.push(FacetReport {
                law: law_name.to_string(),
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

    let mesh_outcome: Result<facet_sweep::FacetSweepResult, _> =
        facet_sweep::facet_sweep(recipe, &station_list, ring);
    match mesh_outcome {
        Ok(result) => facets.push(FacetReport {
            law: law_name.to_string(),
            triangle_count: result.audit.triangle_count,
            quad_count: result.audit.quad_count,
            signed_volume: result.audit.signed_volume,
            winding_violations: result.audit.winding_violations,
            verdict: format!("{:?}", result.verdict),
            refusal: None,
        }),
        Err(e) => facets.push(FacetReport {
            law: law_name.to_string(),
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
                law: law_name.to_string(),
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
                law: law_name.to_string(),
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

fn prism(base: Point3, radius: f64, z0: f64, z1: f64, profile_phase: f64) -> Result<Solid, String> {
    let spine = LineSpine {
        start: Point3::new(base.x, base.y, z0),
        end: Point3::new(base.x, base.y, z1),
    };
    let profile =
        regular_polygon(radius, PRISM_RING, profile_phase).map_err(|e| format!("{e:?}"))?;
    let recipe = SpineFrameRecipe::new(
        spine,
        ProfileLaw::Constant(profile),
        FrameLaw::FixedPlane {
            normal: Vector3::unit_x(),
        },
    );
    match spine_sweep::spine_sweep(&recipe, &[0.0, 1.0]) {
        Ok(Certified { value, .. }) => Ok(value),
        Err(e) => Err(format!("{e:?}")),
    }
}

/// Builds the whole waterslide into `out_dir`: three frame-law realizations,
/// pool + tower prisms, the chute∪pool union attempt, exports, and the
/// report. `ports` is the CC anti-corruption layer; probes run and their
/// (currently deferred) outcomes land in the report.
pub fn build(
    t: &WaterslideTable,
    out_dir: &Path,
    ports: &dyn CcPorts,
) -> Result<ShowcaseReport, String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;

    let mut path = composite_path(&t.path_spec());
    shift_to_ground(&mut path, 0.0);
    let spine = spline_from_path(&path).map_err(|e| e.to_string())?;

    let chute = chute_profile(t)?;

    let a = t.drop_angle_deg.to_radians();
    let laws: Vec<(&str, FrameLaw)> = vec![
        (
            "parallel",
            FrameLaw::ParallelTransport {
                initial_normal: Vector3::new(a.sin(), 0.0, a.cos()),
            },
        ),
        (
            "architectural",
            FrameLaw::ArchitecturalUp {
                up: Vector3::unit_z(),
            },
        ),
        (
            "radial",
            FrameLaw::RadialAboutAxis {
                origin: Point3::new(path.tower_axis.x, path.tower_axis.y, 0.0),
                axis: Vector3::unit_z(),
            },
        ),
    ];

    let mut report = ShowcaseReport {
        item: "waterslide".to_string(),
        spine_total_length: path.total_length,
        spine_samples: path.samples.len(),
        facets: Vec::new(),
        breps: Vec::new(),
        exports: Vec::new(),
        cc_ports: Vec::new(),
        booleans: Vec::new(),
    };

    let mut chute_solid: Option<Solid> = None;
    for (name, law) in &laws {
        let recipe = chute_recipe(&spine, &chute, t, *law);
        let solid = realize(
            name,
            &recipe,
            CHUTE_RING,
            t.stations,
            &mut report.facets,
            &mut report.breps,
        );
        if *name == "parallel" {
            chute_solid = solid;
        }
    }

    let pool_center = Point3::new(
        path.runout_end.x + path.runout_dir.x * t.pool_radius * t.pool_center_fraction,
        path.runout_end.y + path.runout_dir.y * t.pool_radius * t.pool_center_fraction,
        0.0,
    );
    let pool = prism(
        pool_center,
        t.pool_radius,
        -t.pool_depth,
        t.pool_rim_height,
        std::f64::consts::FRAC_PI_8,
    );
    let tower_height = (path.helix_entry.z - t.tower_clearance).max(1.0);
    let tower = prism(path.tower_axis, t.tower_radius, 0.0, tower_height, 0.0);

    match (&chute_solid, &pool, &tower) {
        (Some(chute), Ok(pool), _) => {
            let mut budget = truck_base::evidence::Budget::new(100_000, 100_000, 100);
            match boolean_op(chute, Mode::Add, pool, &mut budget) {
                Ok(Certified { value, .. }) => {
                    let faces = value.face_iter().count();
                    report.booleans.push(CcPortReport {
                        port: "boolean_union_chute_pool".to_string(),
                        status: "certified".to_string(),
                        detail: Some(format!("faces={faces}")),
                    });
                    record_export(
                        &mut report,
                        "step",
                        &out_dir.join("waterslide_union.step"),
                        write_step(&value, &out_dir.join("waterslide_union.step")),
                    );
                }
                Err(e) => report.booleans.push(CcPortReport {
                    port: "boolean_union_chute_pool".to_string(),
                    status: "refused".to_string(),
                    detail: Some(format!("{e:?}")),
                }),
            }
        }
        (None, _, _) => report.booleans.push(CcPortReport {
            port: "boolean_union_chute_pool".to_string(),
            status: "skipped".to_string(),
            detail: Some("chute BREP refused; nothing to union".to_string()),
        }),
        (_, Err(pool_err), _) => report.booleans.push(CcPortReport {
            port: "boolean_union_chute_pool".to_string(),
            status: "skipped".to_string(),
            detail: Some(format!("pool prism refused: {pool_err}")),
        }),
    }

    if let Ok(pool) = &pool {
        record_export(
            &mut report,
            "step",
            &out_dir.join("pool.step"),
            write_step(pool, &out_dir.join("pool.step")),
        );
    }
    if let Ok(tower) = &tower {
        record_export(
            &mut report,
            "step",
            &out_dir.join("tower.step"),
            write_step(tower, &out_dir.join("tower.step")),
        );
    }
    if let Some(chute) = &chute_solid {
        record_export(
            &mut report,
            "step",
            &out_dir.join("waterslide_chute.step"),
            write_step(chute, &out_dir.join("waterslide_chute.step")),
        );
    }

    for (name, law) in &laws {
        let recipe = chute_recipe(&spine, &chute, t, *law);
        let (s0, s1) = recipe.spine.domain();
        if let Ok(station_list) =
            (SamplingPolicy::UniformCount { spine: t.stations }).resolve(s0, s1)
        {
            if let Ok(result) = facet_sweep::facet_sweep(&recipe, &station_list, CHUTE_RING) {
                let p = out_dir.join(format!("waterslide_{name}.stl"));
                record_export(&mut report, "stl", &p, write_stl(&result.mesh, &p));
            }
        }
    }

    let canal: Outcome<CanalCert> = ports.canal_regularity(&spine, t.chute_floor_thickness);
    report.cc_ports.push(CcPortReport {
        port: "canal_regularity_chute_spine".to_string(),
        status: if canal.is_ok() { "certified" } else { "deferred" }.to_string(),
        detail: Some(match &canal {
            Ok(c) => format!("regular={} min_r={}", c.value.regular, c.value.min_curvature_radius),
            Err(e) => format!("{e:?}"),
        }),
    });

    if let (Some(chute), Ok(tower)) = (&chute_solid, &tower) {
        let clearance = ports.clear(chute, tower, t.helix_radius - t.tower_radius);
        report.cc_ports.push(CcPortReport {
            port: "clear_chute_tower".to_string(),
            status: if clearance.is_ok() { "certified" } else { "deferred" }.to_string(),
            detail: Some(match &clearance {
                Ok(c) => format!("distance={} margin={}", c.value.distance, c.value.margin),
                Err(e) => format!("{e:?}"),
            }),
        });
    }

    write_report(&report, &out_dir.join("waterslide_report.json")).map_err(|e| e.to_string())?;
    Ok(report)
}
