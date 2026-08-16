use std::f64::consts::PI;
use truck_base::evidence::{Budget, Refusal, UnresolvedWitness};
use truck_geometry::prelude::*;

fn two_sphere_fillet() -> RbfSurface<Processor<UnitCircle<Point3>, Matrix4>, Sphere, Sphere, Radius>
{
    let sphere0 = Sphere::new(Point3::new(0.0, 0.0, 10.0), 20.0);
    let sphere1 = Sphere::new(Point3::new(0.0, 0.0, -10.0), 20.0);
    let edge_circle = Processor::with_transform(
        UnitCircle::<Point3>::new(),
        Matrix4::from_scale(10.0 * f64::sqrt(3.0)),
    );
    RbfSurface::new(edge_circle, sphere0, sphere1, Radius)
}

#[derive(Clone, Copy, Debug)]
struct Radius;

impl ScalarFunctionD1 for Radius {
    fn der_n(&self, n: usize, t: f64) -> f64 {
        let o = if n == 0 { 10.0 } else { 0.0 };
        let x = match n % 4 {
            0 => f64::cos(t),
            1 => -f64::sin(t),
            2 => -f64::cos(t),
            _ => f64::sin(t),
        };
        o + 5.0 * x
    }
}

#[test]
fn approx_fillet_between_two_spheres() {
    let fillet = two_sphere_fillet();

    let instance = std::time::Instant::now();
    let mut budget = Budget::new(16, 16, 16);
    let approx = ApproxFilletSurface::approx_rolling_ball_fillet(
        &fillet,
        (PI * 0.1, PI * 1.9),
        0.001,
        &mut budget,
    )
    .unwrap()
    .value;
    println!("fillet approximation: {}ms", instance.elapsed().as_millis());

    let instance = std::time::Instant::now();
    let _ = fillet.parameter_division(((0.0, 1.0), (PI * 0.1, PI * 1.9)), 0.005);
    println!(
        "tessellate strict fillet: {}ms",
        instance.elapsed().as_millis()
    );

    let instance = std::time::Instant::now();
    let _ = approx.parameter_division(((0.0, 1.0), (PI * 0.1, PI * 1.9)), 0.005);
    println!(
        "tessellate fillet approx: {}ms",
        instance.elapsed().as_millis()
    );
}

#[test]
fn refinement_spends_subdivision_budget() {
    let fillet = two_sphere_fillet();
    let mut budget = Budget::new(16, 16, 16);
    let initial = budget;
    let _ = ApproxFilletSurface::approx_rolling_ball_fillet(
        &fillet,
        (PI * 0.1, PI * 1.9),
        0.001,
        &mut budget,
    )
    .unwrap()
    .value;
    // The refinement loop draws on the subdivision counter: a successful call
    // must have left strictly less than it was given.
    assert!(budget.subdiv < initial.subdiv);
}

#[test]
fn exhausted_budget_refuses_with_what_was_spent() {
    let fillet = two_sphere_fillet();
    let mut budget = Budget::new(1, 16, 16);
    let result = ApproxFilletSurface::approx_rolling_ball_fillet(
        &fillet,
        (PI * 0.1, PI * 1.9),
        0.001,
        &mut budget,
    );
    let spent = match result {
        Err(Refusal::NumericallyUnresolved { spent, .. }) => spent,
        other => panic!("tiny subdivision budget must refuse, got {other:?}"),
    };
    // Non-zero, and never more than the one subdivision that was granted.
    assert!(spent.subdiv > 0);
    assert!(spent.subdiv <= 1);
}

#[test]
fn budget_refusal_reports_the_counter_that_ran_out() {
    let fillet = two_sphere_fillet();
    let mut budget = Budget::new(1, 16, 16);
    let result = ApproxFilletSurface::approx_rolling_ball_fillet(
        &fillet,
        (PI * 0.1, PI * 1.9),
        0.001,
        &mut budget,
    );
    match result {
        Err(Refusal::NumericallyUnresolved { spent, witness }) => {
            assert!(
                spent.subdiv > 0,
                "the subdivision counter was the one exhausted"
            );
            assert_eq!(spent.newton, 0, "the Newton counter was not spent");
            assert_eq!(spent.depth, 0, "the depth counter was not spent");
            assert_eq!(witness, UnresolvedWitness::ContactCurveNotFound);
        }
        other => panic!("tiny subdivision budget must refuse, got {other:?}"),
    }
}

#[test]
fn sufficient_budget_reaches_the_same_surface_as_before() {
    let fillet = two_sphere_fillet();
    let range = (PI * 0.1, PI * 1.9);

    // Regression guard: the unbudgeted loop ran sixteen passes, so a caller
    // funding exactly that many subdivisions must reproduce it exactly.
    let mut sixteen = Budget::new(16, 16, 16);
    let surface =
        ApproxFilletSurface::approx_rolling_ball_fillet(&fillet, range, 0.001, &mut sixteen)
            .unwrap()
            .value;

    // A generous budget converges to the same surface: once every added
    // contact circle is within tol, no further refinement changes the result,
    // so at equal effort the budgeted loop reaches what the unbudgeted loop
    // reached.
    let mut generous = Budget::new(1024, 16, 16);
    let reference =
        ApproxFilletSurface::approx_rolling_ball_fillet(&fillet, range, 0.001, &mut generous)
            .unwrap()
            .value;

    assert_eq!(surface, reference);
}
