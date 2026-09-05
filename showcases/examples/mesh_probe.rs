use truck_base::cgmath64::{Point3, Vector3};
use truck_geometry::constructive::*;
use showcases::profile::trapezoid_chute;
use showcases::spine::{CompositeSpec, composite_path, shift_to_ground, spline_from_path};
use truck_geometry::constructive::SpineCurve;

fn main() {
    for samples in [48usize, 64, 96, 128, 160] {
        sweep_for_samples(samples);
    }
}

fn sweep_for_samples(samples: usize) {
    let spec = CompositeSpec {
        drop_length: 8.0,
        drop_angle_deg: 50.0,
        transition_radius: 6.0,
        helix_radius: 5.4,
        helix_turns: 2.5,
        helix_slope_deg: 12.0,
        runout_length: 9.0,
        samples,
    };
    let mut path = composite_path(&spec);
    shift_to_ground(&mut path, 0.0);
    let spine = spline_from_path(&path).expect("spine");
    let chute = trapezoid_chute(1.2, 0.54, 0.35).expect("profile");
    let a = 50.0f64.to_radians();
    let recipe = SpineFrameRecipe::new(
        spine,
        ProfileLaw::Scale {
            profile: chute,
            scale: ScalarLaw::Linear { start: 1.0, end: 1.35 },
        },
        FrameLaw::ParallelTransport { initial_normal: Vector3::new(a.sin(), 0.0, a.cos()) },
    );
    let stations = SamplingPolicy::UniformCount { spine: 120 }.resolve(0.0, 1.0).expect("stations");
    let result = match truck_modeling::facet_sweep::facet_sweep(&recipe, &stations, 4) {
        Ok(r) => r,
        Err(e) => {
            println!("samples={samples}: facet refused {e:?}");
            return;
        }
    };
    let positions = &result.mesh.attributes().positions;
    let mut bad = 0usize;
    let mut max_abs = 0.0f64;
    let mut first_bad_station = None;
    for (i, p) in positions.iter().enumerate() {
        let m = p.x.abs().max(p.y.abs()).max(p.z.abs());
        max_abs = max_abs.max(m);
        if m > 100.0 {
            bad += 1;
            if first_bad_station.is_none() {
                first_bad_station = Some(i / 4);
            }
        }
    }
    println!(
        "samples={samples:4} total_pos={} bad={bad:3} max_abs_coord={max_abs:.3e} first_bad_station={first_bad_station:?} volume={:.3}",
        positions.len(),
        result.audit.signed_volume
    );
}
