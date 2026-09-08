//! TOR-B — the ray-torus quartic crossings of the boolean classify stage
//! (`docs/TORUS_CONTACT_THEORY.md` v2 §2): the fixed-degree-4 polynomial in the
//! ray parameter, the parity classifier, the algebraic tangency typing and the
//! certified flank signs. Every assertion is on the public typed contacts of
//! `truck_shapeops::boolean::classify`.
//!
//! House-rule H-6: floats never appear in evidence here — these are assertions
//! on hand-built dyadic (and small-integer) witnesses, not on certified data.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp
)]

use truck_base::cgmath64::{InnerSpace, Point3, Vector3};
use truck_geometry::specifieds::Torus;
use truck_shapeops::boolean::classify::{
    ray_torus_contacts, ray_torus_quartic, TorusRayContactKind,
};

/// The coefficient agreement tolerance: both the implementation and the
/// reference are exact algebraic expansions of the same dyadic witness, so the
/// difference is rounding only (H-3: a dimensionless residual on the
/// coefficients, not a length).
const COEFFICIENT_RESIDUAL: f64 = 1.0e-9; // H-3: dimensionless coefficient residual

/// The position agreement tolerance on contact parameters and points (H-3:
/// relative to the unit-scale witnesses, not a length).
const CONTACT_RESIDUAL: f64 = 1.0e-6; // H-3: unit-scale contact residual

// ---------------------------------------------------------------------------
// small dense-polynomial arithmetic (the independent reference expansion)
// ---------------------------------------------------------------------------

type Poly = Vec<f64>;

/// The linear polynomial `[a, b]` in ascending powers.
fn poly_lin(a: f64, b: f64) -> Poly {
    vec![a, b]
}

/// The constant polynomial `[c]`.
fn poly_const(c: f64) -> Poly {
    vec![c]
}

fn poly_scale(p: &Poly, s: f64) -> Poly {
    p.iter().map(|c| c * s).collect()
}

fn poly_add(a: &Poly, b: &Poly) -> Poly {
    let n = a.len().max(b.len());
    let mut out = vec![0.0; n];
    for (i, c) in out.iter_mut().enumerate() {
        *c += a.get(i).copied().unwrap_or(0.0) + b.get(i).copied().unwrap_or(0.0);
    }
    out
}

fn poly_sub(a: &Poly, b: &Poly) -> Poly {
    let n = a.len().max(b.len());
    let mut out = vec![0.0; n];
    for (i, c) in out.iter_mut().enumerate() {
        *c += a.get(i).copied().unwrap_or(0.0) - b.get(i).copied().unwrap_or(0.0);
    }
    out
}

fn poly_mul(a: &Poly, b: &Poly) -> Poly {
    let mut out = vec![0.0; a.len() + b.len() - 1];
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            out[i + j] += ai * bj;
        }
    }
    out
}

/// The reference quartic coefficients, expanded coordinate-wise from the
/// canonical torus implicit `Φ = (ρ² + z² + R² − r²)² − 4R²ρ²` on the ray
/// (axis `z`). A deliberately different algebra path from the implementation's
/// vector-form coefficient derivation.
fn reference_quartic(torus: &Torus, p: Point3, d: Vector3) -> [f64; 5] {
    let q0 = p - torus.center();
    let x = poly_lin(q0.x, d.x);
    let y = poly_lin(q0.y, d.y);
    let z = poly_lin(q0.z, d.z);
    let rho2 = poly_add(&poly_mul(&x, &x), &poly_mul(&y, &y));
    let qq = poly_add(&rho2, &poly_mul(&z, &z));
    let c0 =
        torus.large_radius() * torus.large_radius() - torus.small_radius() * torus.small_radius();
    let inner = poly_add(&qq, &poly_const(c0));
    let g = poly_sub(
        &poly_mul(&inner, &inner),
        &poly_scale(&rho2, 4.0 * torus.large_radius() * torus.large_radius()),
    );
    let mut out = [0.0; 5];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = g.get(i).copied().unwrap_or(0.0);
    }
    out
}

/// The canonical test torus: `R = 2`, `r = 1`, centred at the origin.
fn test_torus() -> Torus {
    Torus::new(Point3::new(0.0, 0.0, 0.0), 2.0, 1.0)
}

/// The number of genuine crossings (odd-multiplicity roots) of the ray.
fn crossing_count(torus: &Torus, p: Point3, d: Vector3) -> usize {
    ray_torus_contacts(torus, p, d)
        .into_iter()
        .filter(|c| c.kind == TorusRayContactKind::Crossing)
        .count()
}

// ---------------------------------------------------------------------------
// Test 1: the quartic coefficients are exact against the reference expansion.
// ---------------------------------------------------------------------------

#[test]
fn ray_quartic_coefficients_exact_against_reference() {
    // A non-axis-aligned, non-trivial ray: any coefficient-vector bug in the
    // vector-form derivation shows up against the coordinate expansion.
    let torus = test_torus();
    for (p, d) in [
        (Point3::new(1.0, 1.0, 1.0), Vector3::new(1.0, 2.0, 3.0)),
        (Point3::new(3.0, 0.0, -5.0), Vector3::new(0.0, 0.0, 1.0)),
        (Point3::new(2.0, -5.0, 1.0), Vector3::new(0.0, 1.0, 0.0)),
        (Point3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0)),
    ] {
        let got = ray_torus_quartic(&torus, p, d);
        let reference = reference_quartic(&torus, p, d);
        for (g, r) in got.into_iter().zip(reference) {
            let scale = 1.0_f64.max(g.abs()).max(r.abs());
            assert!(
                (g - r).abs() <= COEFFICIENT_RESIDUAL * scale,
                "coefficient mismatch: p={p:?} d={d:?} got={got:?} reference={reference:?}"
            );
        }
    }

    // The exact dyadic witness: a ray from the hole's origin along +x into the
    // tube is `g(t) = t⁴ − 10t² + 9` (roots t = 1 and t = 3).
    let got = ray_torus_quartic(
        &torus,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
    );
    let exact = [9.0, 0.0, -10.0, 0.0, 1.0];
    for (g, e) in got.into_iter().zip(exact) {
        assert!(
            (g - e).abs() <= COEFFICIENT_RESIDUAL,
            "exact witness failed: {got:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 2: the parity classifier — inside ⟺ an odd number of odd-multiplicity
// positive roots.
// ---------------------------------------------------------------------------

#[test]
fn parity_classifier_inside_outside_correct() {
    let torus = test_torus();

    // Inside: the tube centreline point (2, 0, 0) is strictly inside; every
    // generic direction crosses the surface an odd number of times.
    let inside = Point3::new(2.0, 0.0, 0.0);
    for d in [
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(1.0, 1.0, 1.0),
    ] {
        let count = crossing_count(&torus, inside, d);
        assert_eq!(
            count % 2,
            1,
            "an inside point must see an odd crossing count (d={d:?}, count={count})"
        );
    }

    // Outside in the hole: the origin crosses the tube twice along +x.
    let hole = Point3::new(0.0, 0.0, 0.0);
    let count = crossing_count(&torus, hole, Vector3::new(1.0, 0.0, 0.0));
    assert_eq!(count, 2, "the origin ray crosses at t=1 and t=3");
    assert_eq!(count % 2, 0, "the origin is outside the torus");

    // Far outside along the axis: no contact at all.
    let far = Point3::new(0.0, 0.0, 10.0);
    let contacts = ray_torus_contacts(&torus, far, Vector3::new(0.0, 0.0, 1.0));
    assert!(contacts.is_empty(), "a far axis ray never meets the torus");
}

// ---------------------------------------------------------------------------
// Test 3: a tangent ray's double root is typed NOT crossing.
// ---------------------------------------------------------------------------

#[test]
fn tangent_ray_double_root_typed_not_crossing() {
    // The ray (3, 0, −5) + t·(0, 0, 1) touches the outer equator at (3, 0, 0),
    // t = 5, with `g(t) = (t−5)²((t−5)² + 24)`: a double root. The flank signs
    // on its two sides are equal, so the contact is typed `Grazing`, never a
    // crossing, and the crossing bit of the classify solve must not fire.
    let torus = test_torus();
    let p = Point3::new(3.0, 0.0, -5.0);
    let d = Vector3::new(0.0, 0.0, 1.0);

    let contacts = ray_torus_contacts(&torus, p, d);
    assert_eq!(
        contacts.len(),
        1,
        "the tangent ray has exactly one real root"
    );
    let contact = contacts.first().copied().unwrap();
    assert_eq!(
        contact.kind,
        TorusRayContactKind::Grazing,
        "the double root must be typed not crossing"
    );
    assert!(
        (contact.t - 5.0).abs() <= CONTACT_RESIDUAL,
        "the contact is at t=5, got {}",
        contact.t
    );
    let expected = Point3::new(3.0, 0.0, 0.0);
    assert!(
        (contact.point - expected).magnitude() <= CONTACT_RESIDUAL,
        "the contact point is the outer equator, got {:?}",
        contact.point
    );
    assert!(
        crossing_count(&torus, p, d) == 0,
        "a tangent ray contributes no crossing to the parity"
    );
}

// ---------------------------------------------------------------------------
// Test 4: the certified flank signs on a higher-contact fixture.
// ---------------------------------------------------------------------------

#[test]
fn flank_sign_certified_on_higher_contact_fixture() {
    // The ray (2, −5, 1) + t·(0, 1, 0) is tangent along the top parabolic
    // circle at (2, 0, 1), t = 5, with `g(t) = (t−5)⁴`: a QUADRUPLE root (even
    // contact of order four, where even `g″` vanishes). The certified flank
    // signs — `g` sampled immediately on the two sides of the isolated root —
    // are equal, so the contact is typed `Grazing`.
    let torus = test_torus();
    let p = Point3::new(2.0, -5.0, 1.0);
    let d = Vector3::new(0.0, 1.0, 0.0);

    let contacts = ray_torus_contacts(&torus, p, d);
    assert_eq!(
        contacts.len(),
        1,
        "the quadruple contact is the only real root"
    );
    let contact = contacts.first().copied().unwrap();
    assert_eq!(contact.kind, TorusRayContactKind::Grazing);
    assert!((contact.t - 5.0).abs() <= CONTACT_RESIDUAL);

    // The flank signs, certified directly from the quartic coefficients on the
    // two sides of the root: both strictly positive (`g = (t−5)⁴`), so the
    // crossing bit is false without assuming `g′ = 0 ⇒ non-crossing`.
    let g = ray_torus_quartic(&torus, p, d);
    let flank = 0.01;
    let g_lo = poly_eval(&g, contact.t - flank);
    let g_hi = poly_eval(&g, contact.t + flank);
    assert!(
        g_lo > 0.0 && g_hi > 0.0,
        "the flank signs must be equal (both positive), got {g_lo} and {g_hi}"
    );
    assert!(
        crossing_count(&torus, p, d) == 0,
        "an even higher contact contributes no crossing"
    );
}

/// Horner evaluation of the ascending monomial coefficients at `t`.
fn poly_eval(coeffs: &[f64], t: f64) -> f64 {
    coeffs.iter().rev().fold(0.0, |acc, c| acc * t + c)
}
