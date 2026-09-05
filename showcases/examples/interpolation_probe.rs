use truck_base::cgmath64::*;
use truck_geometry::base::{BoundedCurve, ParametricCurve};
use truck_geometry::canonical::Curve;
use truck_geometry::nurbs::{BSplineCurve, KnotVec};
use showcases::spine::{CompositeSpec, composite_path, shift_to_ground};

fn main() {
    let spec = CompositeSpec {
        drop_length: 8.0,
        drop_angle_deg: 50.0,
        transition_radius: 6.0,
        helix_radius: 5.4,
        helix_turns: 2.5,
        helix_slope_deg: 12.0,
        runout_length: 9.0,
        samples: 160,
    };
    let mut path = composite_path(&spec);
    shift_to_ground(&mut path, 0.0);
    let n = path.samples.len();
    let total = path.total_length;
    println!("n={n} total={total}");

    let mut knots = vec![0.0f64; 4];
    for i in 1..=(n - 4) {
        knots.push(i as f64 / (n - 3) as f64);
    }
    knots.extend_from_slice(&[1.0; 4]);
    let knot_vec = KnotVec::from(knots);
    let params: Vec<(f64, Point3)> = path
        .samples
        .iter()
        .map(|(s, p)| (s / total, *p))
        .collect();
    let spline = BSplineCurve::try_interpole(knot_vec, params.clone()).expect("interpole");
    println!("knot range: {:?}", spline.range_tuple());
    for idx in [0usize, 1, n / 4, n / 2, 3 * n / 4, n - 1] {
        let (s, p) = path.samples[idx];
        let t = s / total;
        let q_raw = spline.subs(t);
        let curve = Curve::from(spline.clone());
        let (r0, r1) = curve.try_range_tuple().expect("range");
        let q_curve = curve.subs(r0 + (r1 - r0) * t);
        println!(
            "idx={idx:3} t={t:.6} dev_raw={:.3e} dev_curve={:.3e}",
            (q_raw - p).magnitude(),
            (q_curve - p).magnitude()
        );
    }
}
