use showcases::cc_ports::LandedPorts;
use showcases::waterslide::{WaterslideTable, build};
use truck_geometry::constructive::LineSpine;

fn out_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("showcase_waterslide_{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn three_frame_laws_realize_with_matching_volumes() {
    let table = WaterslideTable {
        stations: 48,
        spine_samples: 48,
        ..WaterslideTable::default()
    };
    let dir = out_dir("frames");
    let report = build(&table, &dir, &LandedPorts).expect("waterslide builds");

    assert_eq!(report.facets.len(), 3, "three frame laws attempted");
    assert_eq!(report.breps.len(), 3);

    for facet in &report.facets {
        if facet.law == "radial" {
            assert!(
                facet.refusal.as_deref().unwrap_or_default().contains("FrameSingular"),
                "ORI-FRAME-ORTHONORMALITY-GATE-001 closed by CC-DEF-BREP-FIXES: \
                 RadialAboutAxis now refuses tangents with a radial component — \
                 the drop and runout qualify; got {:?}",
                facet.refusal
            );
            continue;
        }
        assert!(
            facet.refusal.is_none(),
            "facet realization refused for {}: {:?}",
            facet.law,
            facet.refusal
        );
        assert_eq!(facet.winding_violations, 0, "winding audit must be clean");
        assert_eq!(facet.verdict, "CertifiedWithinTolerance");
    }

    for b in &report.breps {
        if b.law == "radial" {
            assert!(b.refusal.is_some(), "radial BREP must refuse typed");
            continue;
        }
        assert!(b.refusal.is_none(), "BREP refused for {}: {:?}", b.law, b.refusal);
        assert_eq!(b.edge_nonpair_uses, 0, "closed shell: every edge used exactly twice");
        assert_eq!(b.edge_orientation_sum, 0, "consistent orientation");
        assert_eq!(b.face_count, 6, "4 side faces + 2 caps, station-independent");
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn frame_laws_are_right_handed_and_agree() {
    let table = WaterslideTable {
        stations: 48,
        spine_samples: 48,
        ..WaterslideTable::default()
    };
    let dir = out_dir("handedness");
    let report = build(&table, &dir, &LandedPorts).expect("waterslide builds");

    let by_law = |name: &str| {
        report
            .breps
            .iter()
            .find(|b| b.law == name)
            .expect("law present")
            .signed_volume
    };
    let parallel = by_law("parallel");
    let architectural = by_law("architectural");

    assert!(
        parallel * architectural > 0.0,
        "ORI-FRAME-HANDEDNESS-001 closed by CC-DEF-BREP-FIXES (frame_up is \
         right-handed now): the two laws must produce consistently-oriented \
         solids with the same raw-volume sign. parallel={parallel} \
         architectural={architectural}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn frame_laws_enclose_congruent_or_sane_volumes() {
    let table = WaterslideTable {
        stations: 48,
        spine_samples: 48,
        ..WaterslideTable::default()
    };
    let dir = out_dir("volumes");
    let report = build(&table, &dir, &LandedPorts).expect("waterslide builds");
    let by_law = |name: &str| {
        report
            .breps
            .iter()
            .find(|b| b.law == name)
            .expect("law present")
            .signed_volume
            .abs()
    };
    let parallel = by_law("parallel");
    let architectural = by_law("architectural");

    assert!(
        (parallel - architectural).abs() / parallel < 0.10,
        "the transported and architectural frames differ only by transport \
         holonomy through the helix, so volumes stay close: \
         {parallel} vs {architectural}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn facet_volume_near_analytic_estimate() {
    let table = WaterslideTable {
        stations: 48,
        spine_samples: 48,
        ..WaterslideTable::default()
    };
    let dir = out_dir("parity");
    let report = build(&table, &dir, &LandedPorts).expect("waterslide builds");

    let facet_v = report.facets[0].signed_volume;
    let cross_section_area = (table.chute_width + table.chute_width * table.chute_top_fraction)
        / 2.0
        * table.chute_wall_height;
    let widening_integral = 1.0 + (table.runout_widening - 1.0) / 2.0
        + (table.runout_widening - 1.0) * (table.runout_widening - 1.0) / 3.0;
    let estimate = cross_section_area * report.spine_total_length * widening_integral;
    assert!(
        (facet_v - estimate).abs() / estimate < 0.15,
        "facet volume {facet_v} must track the Pappus-corrected prismatic \
         estimate {estimate} (area x arclength x widening^2 integral; the \
         helix curvature and the profile's centroid offset are second-order \
         at this scale)"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn determinism_same_table_same_report() {
    let table = WaterslideTable {
        stations: 24,
        spine_samples: 48,
        ..WaterslideTable::default()
    };
    let d1 = out_dir("det1");
    let d2 = out_dir("det2");
    let r1 = build(&table, &d1, &LandedPorts).expect("first run");
    let r2 = build(&table, &d2, &LandedPorts).expect("second run");

    let strip = |r: &showcases::harness::ShowcaseReport| {
        serde_json::to_string(&(
            r.facets.clone(),
            r.breps.clone(),
            r.spine_total_length,
            r.spine_samples,
        ))
        .expect("serialize")
    };
    assert_eq!(strip(&r1), strip(&r2), "determinism: identical tables must produce identical certificates");
    std::fs::remove_dir_all(&d1).ok();
    std::fs::remove_dir_all(&d2).ok();
}

#[test]
fn exports_record_outcomes() {
    let table = WaterslideTable {
        stations: 24,
        spine_samples: 48,
        ..WaterslideTable::default()
    };
    let dir = out_dir("exports");
    let report = build(&table, &dir, &LandedPorts).expect("waterslide builds");
    assert!(
        report.exports.iter().any(|e| e.kind == "stl" && e.ok),
        "at least one STL export must succeed: {:?}",
        report.exports
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn cc_ports_defer_with_typed_refusals() {    let table = WaterslideTable {
        stations: 24,
        spine_samples: 48,
        ..WaterslideTable::default()
    };
    let dir = out_dir("cc");
    let report = build(&table, &dir, &LandedPorts).expect("waterslide builds");
    let canal = report
        .cc_ports
        .iter()
        .find(|p| p.port == "canal_regularity_chute_spine")
        .expect("canal probe present");
    assert_eq!(canal.status, "deferred");
    let clear = report
        .cc_ports
        .iter()
        .find(|p| p.port == "clear_chute_tower")
        .expect("clear probe present");
    assert_eq!(clear.status, "deferred");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn brep_volume_matches_facet_on_small_case() {
    let table = WaterslideTable {
        stations: 16,
        spine_samples: 48,
        ..WaterslideTable::default()
    };
    let dir = out_dir("small");
    let report = build(&table, &dir, &LandedPorts).expect("waterslide builds");
    assert!(
        report.breps[0].signed_volume.abs() > 0.0,
        "nonzero enclosed volume"
    );
    assert!(brep_volume_report_ok(&report));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn concave_u_chute_refuses_on_the_facet_path() {
    use showcases::profile::u_chute;
    use truck_base::cgmath64::InnerSpace;
    use truck_geometry::constructive::{FrameLaw, ProfileLaw, SamplingPolicy, SpineFrameRecipe};

    // PB-003-CONCAVE-CAPS (e4537d4) landed the deterministic ear-clip cap
    // path: a concave ring is TRIANGULATED on the facet path, never refused.
    // This expectation flipped WITH that capability landing (deliberate, not
    // weakened): the concave U-chute BUILDS, and the flip pins the strongest
    // honest claims the run supports — a clean-winding certified mesh whose
    // signed volume and world bounds agree with the analytic reference facts.
    let (width, wall_height, wall_thickness, floor_thickness) = (1.2, 0.54, 0.072, 0.096);
    let profile =
        u_chute(width, wall_height, wall_thickness, floor_thickness).expect("u profile builds");
    let spine = LineSpine {
        start: truck_base::cgmath64::Point3::new(0.0, 0.0, 0.0),
        end: truck_base::cgmath64::Point3::new(0.0, 0.0, 1.0),
    };
    let recipe = SpineFrameRecipe::new(
        spine,
        ProfileLaw::Constant(profile),
        FrameLaw::FixedPlane {
            normal: truck_base::cgmath64::Vector3::unit_x(),
        },
    );
    let stations = SamplingPolicy::UniformCount { spine: 8 }
        .resolve(0.0, 1.0)
        .expect("stations");
    let realized = match truck_modeling::facet_sweep::facet_sweep(&recipe, &stations, 8) {
        Ok(realized) => realized,
        Err(error) => panic!(
            "concave U-chute must BUILD on the facet path (PB-003-CONCAVE-CAPS \
             ear-clips non-convex caps now), refused with {error:?}"
        ),
    };

    assert_eq!(
        realized.audit.winding_violations, 0,
        "the ear-clipped concave caps must close the mesh"
    );
    assert_eq!(
        realized.verdict,
        truck_modeling::facet_sweep::FacetVerdict::CertifiedWithinTolerance,
        "a clean winding audit with a finite nonzero volume certifies"
    );

    // Analytic reference facts (closed form, independent of the facet path):
    // the U cross-section is a floor slab over the inner width plus two wall
    // columns of `wall_thickness` x `wall_height`; the straight unit sweep is
    // the prism of that area (constant profile, no holonomy).
    let inner_half = width / 2.0 - wall_thickness;
    let cross_section_area =
        2.0 * inner_half * floor_thickness + 2.0 * wall_thickness * wall_height;
    let reference_volume = cross_section_area * (spine.end - spine.start).magnitude();
    assert!(
        (realized.audit.signed_volume - reference_volume).abs() / reference_volume <= 1.0e-6,
        "built volume {} must pin the analytic prism volume {}",
        realized.audit.signed_volume,
        reference_volume
    );

    // World bounds: the FixedPlane { unit_x } frame maps profile-up onto
    // `-unit_y` and profile-across onto `unit_x`, swept along +z, so the mesh
    // envelope is exactly `[-hw, hw] x [-wall_height, 0] x [0, 1]`.
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in realized.mesh.positions() {
        lo[0] = lo[0].min(p.x);
        lo[1] = lo[1].min(p.y);
        lo[2] = lo[2].min(p.z);
        hi[0] = hi[0].max(p.x);
        hi[1] = hi[1].max(p.y);
        hi[2] = hi[2].max(p.z);
    }
    let expected_lo = [-width / 2.0, -wall_height, 0.0];
    let expected_hi = [width / 2.0, 0.0, 1.0];
    for axis in 0..3 {
        assert!(
            (lo[axis] - expected_lo[axis]).abs() <= 1.0e-9
                && (hi[axis] - expected_hi[axis]).abs() <= 1.0e-9,
            "axis {axis} mesh bounds [{}, {}] must pin the analytic envelope \
             [{}, {}]",
            lo[axis],
            hi[axis],
            expected_lo[axis],
            expected_hi[axis]
        );
    }
}

#[test]
fn facet_mesh_stays_within_path_bounds() {
    use showcases::spine::{composite_path, shift_to_ground};

    let table = WaterslideTable::default();
    let dir = out_dir("bounds");
    let report = build(&table, &dir, &LandedPorts).expect("waterslide builds");
    let stl_export = report
        .exports
        .iter()
        .find(|e| e.kind == "stl" && e.ok)
        .expect("an STL export must exist for the bounds guard");

    let mut path = composite_path(&table.path_spec());
    shift_to_ground(&mut path, 0.0);
    let mut bound = 0.0f64;
    for (_, p) in &path.samples {
        bound = bound.max(p.x.abs().max(p.y.abs()).max(p.z.abs()));
    }
    assert!(
        bound < 60.0,
        "NUM-INTERPOLE-OVERSHOOT-001 guard: the path itself must stay in the \
         stable interpolation regime (n = spine_samples); got extent {bound}"
    );
    let stl = std::fs::read(&stl_export.path).expect("stl readable");
    assert!(
        stl.len() > 84,
        "STL must contain triangles: {}",
        stl_export.path
    );
    std::fs::remove_dir_all(&dir).ok();
}

fn brep_volume_report_ok(report: &showcases::harness::ShowcaseReport) -> bool {
    report
        .breps
        .iter()
        .filter(|b| b.refusal.is_none())
        .all(|b| b.signed_volume.is_finite() && b.signed_volume.abs() > 0.0)
}
