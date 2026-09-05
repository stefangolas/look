use truck_base::cgmath64::{Point3, Vector3};
use truck_geometry::constructive::*;
use showcases::profile::u_chute;
use showcases::spine::{CompositeSpec, composite_path, shift_to_ground, spline_from_path};
use truck_geometry::constructive::SpineCurve;

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
    let (s0, s1) = spine.domain();
    println!("domain: ({s0}, {s1})");

    let chute = u_chute(1.2, 0.54, 0.072, 0.096).expect("chute");
    println!("chute vertices: {}", chute.vertices.len());
    let a = 50.0f64.to_radians();
    let laws: Vec<(&str, FrameLaw)> = vec![
        ("parallel", FrameLaw::ParallelTransport { initial_normal: Vector3::new(a.sin(), 0.0, a.cos()) }),
        ("architectural", FrameLaw::ArchitecturalUp { up: Vector3::unit_z() }),
        ("radial", FrameLaw::RadialAboutAxis { origin: Point3::new(0.0, 0.0, 0.0), axis: Vector3::unit_z() }),
        ("fixed", FrameLaw::FixedPlane { normal: Vector3::unit_y() }),
    ];
    for (name, law) in laws {
        let recipe = SpineFrameRecipe::new(
            spine.clone(),
            ProfileLaw::Scale { profile: chute.clone(), scale: ScalarLaw::Linear { start: 1.0, end: 1.35 } },
            law,
        );
        let mut failures = 0;
        for si in 0..=8 {
            let s = s0 + (s1 - s0) * (si as f64) / 8.0;
            match recipe.position(s, 0.0) {
                Ok(p) => println!("  {name} s={s:.4} ok p=({:.3},{:.3},{:.3})", p.x, p.y, p.z),
                Err(e) => {
                    println!("  {name} s={s:.4} ERR {e:?}");
                    failures += 1;
                }
            }
        }
        println!("{name}: {failures} failures");
    }
}
