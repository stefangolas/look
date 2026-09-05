use truck_base::cgmath64::Point3;
use truck_geometry::arrange;
use truck_geometry::canonical::Curve;
use truck_modeling::{Line, revolve};
use showcases::profile as _;
use showcases::teapot::{TeapotTable, body_silhouette};

fn main() {
    let t = TeapotTable::default();
    let silhouette = body_silhouette(&t);
    println!("silhouette points: {}", silhouette.len());
    for (i, p) in silhouette.iter().enumerate() {
        println!("  {i}: ({:.6}, {:.6})", p.x, p.y);
    }
    let profile_curves: Vec<Curve> = silhouette
        .windows(2)
        .map(|w| Curve::from(Line(Point3::new(w[0].x, 0.0, w[0].y), Point3::new(w[1].x, 0.0, w[1].y))))
        .collect();
    let mut curves = profile_curves.clone();
    let first = silhouette[0];
    let last = silhouette[silhouette.len() - 1];
    curves.push(Curve::from(Line(
        Point3::new(last.x, 0.0, last.y),
        Point3::new(first.x, 0.0, first.y),
    )));
    let working: Vec<Curve> = curves
        .iter()
        .map(|c| match c {
            Curve::Line(Line(a, b)) => Curve::from(Line(Point3::new(a.x, a.z, 0.0), Point3::new(b.x, b.z, 0.0))),
            _ => unreachable!(),
        })
        .collect();
    match arrange::arrange(&working, None) {
        Ok(_) => println!("arrange: Ok"),
        Err(e) => {
            println!("arrange: Err {e:?}");
            return;
        }
    }
    let arrangement = arrange::arrange(&working, None).expect("arrange").value;
    match revolve::revolve_profile(&curves, &arrangement, std::f64::consts::TAU) {
        Ok(_) => println!("revolve: Ok"),
        Err(e) => println!("revolve: Err {e:?}"),
    }
}
