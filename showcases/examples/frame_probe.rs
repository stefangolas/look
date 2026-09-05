//! Frame-law handedness probe: for each frame law, evaluates the recipe
//! frame along the waterslide spine and checks the kernel's own normative
//! convention (mod.rs): unit lengths, pairwise orthogonality, and
//! right-handedness `tangent x normal == binormal`.

use truck_base::cgmath64::*;
use truck_geometry::constructive::{FrameLaw, Profile2D, ProfileLaw, SpineCurve, SpineFrameRecipe};
use showcases::profile::trapezoid_chute;
use showcases::spine::{CompositeSpec, composite_path, shift_to_ground, spline_from_path};

fn frame_report(name: &str, law: FrameLaw, spine: &truck_modeling::Curve, chute: &Profile2D) {
    let recipe = SpineFrameRecipe::new(spine.clone(), ProfileLaw::Constant(chute.clone()), law);
    let (s0, s1) = recipe.spine.domain();
    println!("== {name} ==");
    let mut rh_bad = 0;
    let mut orth_bad = 0;
    for i in 0..=12 {
        let s = s0 + (s1 - s0) * (i as f64) / 12.0;
        let f = match recipe.frame(s) {
            Ok(f) => f,
            Err(e) => {
                println!("  s={s:.4} FRAME ERR {e:?}");
                continue;
            }
        };
        let (t, n, b) = (f.tangent, f.normal, f.binormal);
        let rh = (t.cross(n) - b).magnitude();
        let orth = t.dot(n).abs().max(t.dot(b).abs()).max(n.dot(b).abs());
        if rh > 1e-9 || orth > 1e-9 {
            if rh > 1e-9 {
                rh_bad += 1;
            }
            if orth > 1e-9 {
                orth_bad += 1;
            }
            println!(
                "  s={s:.4} t x n - b = {rh:.3e}, max |dot| = {orth:.3e}"
            );
        }
    }
    println!("  handedness violations: {rh_bad}, non-orthogonal: {orth_bad}");
}

fn main() {
    let spec = CompositeSpec {
        drop_length: 8.0,
        drop_angle_deg: 50.0,
        transition_radius: 6.0,
        helix_radius: 5.4,
        helix_turns: 2.5,
        helix_slope_deg: 12.0,
        runout_length: 9.0,
        samples: 48,
    };
    let mut path = composite_path(&spec);
    shift_to_ground(&mut path, 0.0);
    let spine = spline_from_path(&path).expect("spine");
    let chute = trapezoid_chute(1.2, 0.54, 0.35).expect("profile");
    let a = 50.0f64.to_radians();

    frame_report(
        "parallel",
        FrameLaw::ParallelTransport { initial_normal: Vector3::new(a.sin(), 0.0, a.cos()) },
        &spine,
        &chute,
    );
    frame_report(
        "architectural",
        FrameLaw::ArchitecturalUp { up: Vector3::unit_z() },
        &spine,
        &chute,
    );
    frame_report(
        "radial",
        FrameLaw::RadialAboutAxis { origin: Point3::new(0.0, 0.0, 0.0), axis: Vector3::unit_z() },
        &spine,
        &chute,
    );
    frame_report(
        "fixed",
        FrameLaw::FixedPlane { normal: Vector3::unit_y() },
        &spine,
        &chute,
    );
}
