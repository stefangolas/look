use truck_geometry::arrange;
use truck_geometry::canonical::Curve;
use truck_modeling::Line;
use truck_modeling::Point3;

fn loop_curves(pts: &[(f64, f64)]) -> Vec<Curve> {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            Curve::from(Line(
                Point3::new(a.0, a.1, 0.0),
                Point3::new(b.0, b.1, 0.0),
            ))
        })
        .collect()
}

fn probe(name: &str, pts: &[(f64, f64)]) {
    let curves = loop_curves(pts);
    match arrange::arrange(&curves, None) {
        Ok(_) => println!("{name}: Ok"),
        Err(e) => println!("{name}: Err {e:?}"),
    }
}

fn main() {
    probe(
        "landed-tube-shape-dyadic",
        &[(1.0, 0.0), (3.0, 0.0), (2.0, 2.0), (1.0, 2.0)],
    );
    probe(
        "vessel-dyadic-control",
        &[(1.0, 0.0), (2.0, 4.0), (1.0, 4.0), (0.5, 0.0)],
    );
    probe(
        "vessel-dyadic-near-parallel",
        &[(1.0, 0.0), (1.25, 4.0), (0.75, 4.0), (0.5625, 0.0)],
    );
    probe(
        "vessel-dyadic-exactly-parallel",
        &[(1.0, 0.0), (1.25, 4.0), (0.75, 4.0), (0.5, 0.0)],
    );
    probe(
        "vessel-nondyadic-control",
        &[(1.0, 0.0), (1.1, 1.0), (0.9, 1.0), (0.7, 0.0)],
    );
    probe(
        "five-gon-slanted-close-dyadic",
        &[(0.0, 0.0), (2.0, 0.0), (3.0, 2.0), (2.0, 4.0), (0.5, 4.0)],
    );
    probe(
        "five-gon-with-vertical-dyadic",
        &[(1.0, 0.0), (3.0, 0.0), (3.0, 2.0), (2.0, 2.0), (1.0, 2.0)],
    );
    probe(
        "outer-wall-5gon",
        &[(0.5, 0.0), (0.875, 0.5), (1.0, 1.0), (0.75, 1.5), (0.5, 1.75)],
    );
    probe(
        "outer-wall-4gon-truncated",
        &[(0.5, 0.0), (0.875, 0.5), (1.0, 1.0), (0.75, 1.5)],
    );
    probe(
        "outer-wall-5gon-fractions",
        &[(0.5, 0.0), (1.0, 0.5), (1.0, 1.0), (0.75, 1.5), (0.5, 1.75)],
    );
    probe(
        "lattice-clean-predict-Ok",
        &[(1.0, 0.0), (2.0, 2.0), (0.5, 2.0), (0.0, 0.0)],
    );
    probe(
        "lattice-dirty-predict-Err",
        &[(1.0, 0.0), (1.25, 1.0), (0.75, 1.0), (0.5625, 0.0)],
    );
    probe(
        "rhombic-vessel-predict-Ok",
        &[
            (0.5, 0.0),
            (2.5, 2.0),
            (0.5, 4.0),
            (0.25, 4.0),
            (2.25, 2.0),
            (0.25, 0.0),
        ],
    );
}
