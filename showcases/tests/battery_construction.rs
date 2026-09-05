use truck_base::cgmath64::{Point2, Point3, Vector3};
use truck_geometry::constructive::{
    FrameLaw, LineSpine, PolylineSpine, Profile2D, ProfileLaw, SamplingPolicy, ScalarLaw,
    SpineFrameRecipe,
};
use truck_modeling::{facet_sweep, spine_sweep};

fn stations(n: usize) -> Vec<f64> {
    SamplingPolicy::UniformCount { spine: n }.resolve(0.0, 1.0).expect("stations")
}

fn square(n: usize, r: f64) -> Profile2D {
    let vertices: Vec<Point2> = (0..n)
        .map(|i| {
            let t = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
            Point2::new(r * t.cos(), r * t.sin())
        })
        .collect();
    Profile2D::try_closed(vertices).expect("profile")
}

fn vertical_line_recipe(profile_law: ProfileLaw, frame_law: FrameLaw) -> SpineFrameRecipe<LineSpine, ProfileLaw, FrameLaw> {
    SpineFrameRecipe::new(
        LineSpine {
            start: Point3::new(0.0, 0.0, 0.0),
            end: Point3::new(0.0, 0.0, 2.0),
        },
        profile_law,
        frame_law,
    )
}

#[test]
fn polyline_corner_refuses_c1() {
    let spine = PolylineSpine::try_new(vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 1.0),
    ])
    .expect("polyline builds");
    let recipe = SpineFrameRecipe::new(
        spine,
        ProfileLaw::Constant(square(4, 0.2)),
        FrameLaw::FixedPlane { normal: Vector3::unit_y() },
    );
    let facet = facet_sweep::facet_sweep(&recipe, &stations(8), 4);
    assert!(facet.is_err(), "facet path must refuse a non-C1 spine");
    assert!(
        format!("{:?}", facet.err().unwrap()).contains("SpineNotC1"),
        "refusal must be the typed C1 gate"
    );
}

#[test]
fn architectural_up_parallel_to_tangent_refuses() {
    let recipe = vertical_line_recipe(
        ProfileLaw::Constant(square(4, 0.2)),
        FrameLaw::ArchitecturalUp { up: Vector3::unit_z() },
    );
    let facet = facet_sweep::facet_sweep(&recipe, &stations(4), 4);
    assert!(facet.is_err(), "up parallel to tangent must refuse");
    assert!(format!("{:?}", facet.err().unwrap()).contains("FrameSingular"));
}

#[test]
fn through_zero_scale_spine_sweep_refuses() {
    let recipe = vertical_line_recipe(
        ProfileLaw::Scale {
            profile: square(4, 0.2),
            scale: ScalarLaw::Linear { start: 1.0, end: -1.0 },
        },
        FrameLaw::FixedPlane { normal: Vector3::unit_x() },
    );
    let brep = spine_sweep::spine_sweep(&recipe, &stations(4));
    assert!(brep.is_err(), "through-zero scale must refuse at the BREP entry");
}

#[test]
fn through_zero_scale_facet_path_behavior() {
    let recipe = vertical_line_recipe(
        ProfileLaw::Scale {
            profile: square(4, 0.2),
            scale: ScalarLaw::Linear { start: 1.0, end: -1.0 },
        },
        FrameLaw::FixedPlane { normal: Vector3::unit_x() },
    );
    let facet = facet_sweep::facet_sweep(&recipe, &stations(4), 4);
    match facet {
        Ok(result) => panic!(
            "through-zero scale must refuse on the facet path — SEM-FACET-SCALE-ZERO-001 \
             reopened: accepted with volume {}",
            result.audit.signed_volume
        ),
        Err(e) => {
            assert!(
                format!("{e:?}").contains("ProfileCollapse"),
                "SEM-FACET-SCALE-ZERO-001 closed by CC-DEF-BREP-FIXES: the facet path \
                 must refuse with the typed collapse, got {e:?}"
            );
        }
    }
}

#[test]
fn correspondence_mismatch_spine_sweep_refuses() {
    let start = square(4, 0.2);
    let end = square(6, 0.1);
    let recipe = vertical_line_recipe(
        ProfileLaw::LinearCorrespondence { start, end },
        FrameLaw::FixedPlane { normal: Vector3::unit_x() },
    );
    let brep = spine_sweep::spine_sweep(&recipe, &stations(4));
    assert!(brep.is_err(), "mismatched correspondence must refuse at the BREP entry");
}

#[test]
fn correspondence_mismatch_facet_path_behavior() {
    let start = square(4, 0.2);
    let end = square(6, 0.1);
    let recipe = vertical_line_recipe(
        ProfileLaw::LinearCorrespondence { start, end },
        FrameLaw::FixedPlane { normal: Vector3::unit_x() },
    );
    let facet = facet_sweep::facet_sweep(&recipe, &stations(4), 4);
    match facet {
        Ok(result) => panic!(
            "mismatched correspondence must refuse on the facet path — \
             SEM-FACET-CORRESPONDENCE-TRUNCATION-001 reopened: accepted with \
             volume {}",
            result.audit.signed_volume
        ),
        Err(e) => {
            assert!(
                format!("{e:?}").contains("ProfileCorrespondenceMismatch"),
                "SEM-FACET-CORRESPONDENCE-TRUNCATION-001 closed by \
                 CC-DEF-BREP-FIXES: expected the typed correspondence refusal, \
                 got {e:?}"
            );
        }
    }
}
