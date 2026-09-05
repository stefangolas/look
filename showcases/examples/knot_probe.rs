use truck_base::cgmath64::*;
use truck_geometry::base::ParametricCurve;
use truck_geometry::nurbs::{BSplineCurve, KnotVec};
use showcases::spine::{CompositeSpec, composite_path, shift_to_ground};

fn max_interpolant_deviation(spec: &CompositeSpec, averaging: bool) -> (usize, f64, f64) {
    let mut path = composite_path(spec);
    shift_to_ground(&mut path, 0.0);
    let n = path.samples.len();
    let total = path.total_length;
    let params: Vec<(f64, Point3)> = path
        .samples
        .iter()
        .map(|(s, p)| (s / total, *p))
        .collect();

    let mut knots = vec![0.0f64; 4];
    if averaging {
        for j in 1..=(n - 4) {
            let u = (params[j - 1].0 + params[j].0 + params[j + 1].0) / 3.0;
            knots.push(u);
        }
    } else {
        for i in 1..=(n - 4) {
            knots.push(i as f64 / (n - 3) as f64);
        }
    }
    knots.extend_from_slice(&[1.0; 4]);
    let knot_vec = KnotVec::from(knots);
    let spline =
        match BSplineCurve::try_interpole(knot_vec, params.clone()) {
            Ok(s) => s,
            Err(e) => {
                println!("  interpole failed: {e:?}");
                return (n, f64::NAN, f64::NAN);
            }
        };

    let mut at_samples = 0.0f64;
    for (t, p) in &params {
        let d = (spline.subs(*t) - *p).magnitude();
        at_samples = at_samples.max(d);
    }
    let mut between = 0.0f64;
    let steps = 4000;
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let q = spline.subs(t);
        let m = q.x.abs().max(q.y.abs()).max(q.z.abs());
        between = between.max(m);
    }
    (n, at_samples, between)
}

fn main() {
    for samples in [48usize, 64, 96, 128, 160, 256] {
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
        let (n_u, at_u, btw_u) = max_interpolant_deviation(&spec, false);
        let (n_a, at_a, btw_a) = max_interpolant_deviation(&spec, true);
        println!(
            "n={n_u:4} uniform: at_sample_max={at_u:.3e} inter_sample_max_coord={btw_u:.3e} | n={n_a:4} averaged: at_sample_max={at_a:.3e} inter_sample_max_coord={btw_a:.3e}"
        );
    }
}
