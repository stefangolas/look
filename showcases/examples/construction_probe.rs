use truck_base::cgmath64::{Point2, Point3, Vector3};
use truck_geometry::constructive::*;
use truck_modeling::facet_sweep;

fn square(n: usize, r: f64) -> Profile2D {
    let vertices: Vec<Point2> = (0..n)
        .map(|i| {
            let t = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
            Point2::new(r * t.cos(), r * t.sin())
        })
        .collect();
    Profile2D::try_closed(vertices).expect("profile")
}

fn stations(n: usize) -> Vec<f64> {
    SamplingPolicy::UniformCount { spine: n }.resolve(0.0, 1.0).expect("stations")
}

fn main() {
    let line_recipe = SpineFrameRecipe::new(
        LineSpine { start: Point3::new(0.0, 0.0, 0.0), end: Point3::new(0.0, 0.0, 2.0) },
        ProfileLaw::Scale { profile: square(4, 0.2), scale: ScalarLaw::Linear { start: 1.0, end: -1.0 } },
        FrameLaw::FixedPlane { normal: Vector3::unit_x() },
    );
    match facet_sweep::facet_sweep(&line_recipe, &stations(4), 4) {
        Ok(r) => println!("scale-zero: Ok tri={} quad={} v={} winding={}",
            r.audit.triangle_count, r.audit.quad_count, r.audit.signed_volume, r.audit.winding_violations),
        Err(e) => println!("scale-zero: Err {e:?}"),
    }

    let poly = PolylineSpine::try_new(vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 1.0),
    ]).expect("polyline");
    println!("polyline domain: {:?}", SpineCurve::domain(&poly));
    let poly_recipe = SpineFrameRecipe::new(
        poly,
        ProfileLaw::Constant(square(4, 0.2)),
        FrameLaw::FixedPlane { normal: Vector3::unit_x() },
    );
    match facet_sweep::facet_sweep(&poly_recipe, &stations(8), 4) {
        Ok(r) => println!("polyline: Ok v={} winding={}", r.audit.signed_volume, r.audit.winding_violations),
        Err(e) => println!("polyline: Err {e:?}"),
    }

    let corr = SpineFrameRecipe::new(
        LineSpine { start: Point3::new(0.0, 0.0, 0.0), end: Point3::new(0.0, 0.0, 2.0) },
        ProfileLaw::LinearCorrespondence { start: square(4, 0.2), end: square(6, 0.1) },
        FrameLaw::FixedPlane { normal: Vector3::unit_x() },
    );
    match facet_sweep::facet_sweep(&corr, &stations(4), 4) {
        Ok(r) => println!("correspondence: Ok v={} winding={}", r.audit.signed_volume, r.audit.winding_violations),
        Err(e) => println!("correspondence: Err {e:?}"),
    }
}
