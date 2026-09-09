//! ADM-L4-DEFLATE-SEAM conformance: the collapsed-edge deflation and
//! seam-identification lemmas (Theorem B1, lemma 4) over the ADM-SHIM frozen
//! patch type, exercised through the public kernels of
//! `construct::deflate`:
//!
//! - `deflation_divides_exactly_zero_remainder` — the closed-form
//!   apex-collapse fixture divides the known `(1−v)` factor out of the normal
//!   numerator with a PROVEN zero remainder: three exact divisions (the
//!   recorded multiplicity 3), never an approximate division, and a boundary
//!   that does not collapse exactly refuses typed;
//! - `multiplicity_iteration_handles_double_collapse` — the iteration counts a
//!   double collapse (multiplicity 2) and a single collapse (multiplicity 1)
//!   exactly, terminating each time at the quotient whose boundary slice is
//!   no longer an exact zero;
//! - `deflated_certificate_passes_interior_fixture` — the deflated certificate
//!   (`c·M* > 0` over the closed box, the hemisphere test applied to the
//!   quotient) passes on the deflated cone interior with the outward direction
//!   and refuses a direction that does not certify strict positivity;
//! - `seam_identity_exact_on_aligned_fixture` — the aligned sphere-octant pair
//!   closes the seam exactly (`A₀W₁ − A₁W₀ ≡ 0`), and a pair that is not a
//!   seam candidate is `Ok(false)`;
//! - `misaligned_seam_refuses_typed` — a seam-candidate pair whose shared
//!   boundary is perturbed (a near-seam) does NOT close exactly and REFUSES
//!   typed: near-miss is never reported as identity.

#![deny(clippy::unwrap_used)]

mod patch_fixtures;

use patch_fixtures::unit_square_domain;
use truck_certified::construct::admission::EdgeId;
use truck_certified::construct::deflate::{
    certify_deflated_interior, deflate_factor, seam_identified, DeflationCertificate,
};
use truck_certified::construct::patches::{PatchParent, PatchSide, TensorBernsteinPatch};
use truck_certified::construct::refusal::ConstructRefusal;

/// The binomial coefficient `C(n, k)` for the small fixture degrees.
fn binom(n: usize, k: usize) -> f64 {
    let k = k.min(n - k);
    let mut r = 1.0f64;
    for i in 1..=k {
        r *= (n - k + i) as f64 / i as f64;
    }
    r
}

/// Tensor-Bernstein evaluation of a patch's dehomogenized surface at `(u, v)`
/// in plain f64, for the fixture closed-form machine checks.
fn surface_point(patch: &TensorBernsteinPatch, u: f64, v: f64) -> [f64; 3] {
    let (m, n) = patch.degree();
    let bu: Vec<f64> = (0..=m)
        .map(|i| binom(m, i) * u.powi(i as i32) * (1.0 - u).powi((m - i) as i32))
        .collect();
    let bv: Vec<f64> = (0..=n)
        .map(|j| binom(n, j) * v.powi(j as i32) * (1.0 - v).powi((n - j) as i32))
        .collect();
    let mut acc = [0.0, 0.0, 0.0];
    let mut wacc = 0.0;
    for i in 0..=m {
        for j in 0..=n {
            let e = patch.numerator()[i][j];
            let s = bu[i] * bv[j];
            acc[0] += s * e[0];
            acc[1] += s * e[1];
            acc[2] += s * e[2];
            wacc += s * patch.weights()[i][j];
        }
    }
    [acc[0] / wacc, acc[1] / wacc, acc[2] / wacc]
}

/// Extract the `Ok` of a certified deflation (the fixture data is valid by
/// construction; the refusal arm is a test-bug panic, never an unwrap).
fn expect_deflate(patch: &TensorBernsteinPatch, side: PatchSide) -> DeflationCertificate {
    match deflate_factor(patch, side) {
        Ok(certificate) => certificate,
        Err(refusal) => panic!("expected an exact deflation on {side:?}, refused: {refusal:?}"),
    }
}

/// Assert `deflate_factor` refuses the exact expected typed outcome.
fn expect_deflate_refusal(
    patch: &TensorBernsteinPatch,
    side: PatchSide,
    expected: ConstructRefusal,
) {
    match deflate_factor(patch, side) {
        Ok(certificate) => panic!(
            "expected refusal {expected:?} on {side:?}, got an exact deflation of multiplicity \
             {}",
            certificate.multiplicity()
        ),
        Err(refusal) => assert_eq!(refusal, expected),
    }
}

/// Assert `seam_identified` refuses the exact expected typed outcome.
fn expect_seam_refusal(
    a: &TensorBernsteinPatch,
    b: &TensorBernsteinPatch,
    expected: ConstructRefusal,
) {
    match seam_identified(a, b) {
        Ok(closed) => panic!("expected refusal {expected:?}, got a seam verdict {closed}"),
        Err(refusal) => assert_eq!(refusal, expected),
    }
}

/// The exact-arithmetic double-collapse fixture (multiplicity 2):
/// `X(u, v) = ((1−v)·u, (1−v)²·u, (1−v)²)`, weights all 1. The `v = 1` edge
/// collapses to the origin and the normal numerator carries exactly
/// `(1−v)²` (`X_u×X_v = (1−v)²·(−2(1−v), 2, −u)`).
fn double_collapse_patch() -> TensorBernsteinPatch {
    let numerator: Vec<Vec<[f64; 3]>> = vec![
        vec![[0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        vec![[1.0, 1.0, 1.0], [0.5, 0.0, 0.0], [0.0, 0.0, 0.0]],
    ];
    let weights: Vec<Vec<f64>> = vec![vec![1.0, 1.0, 1.0], vec![1.0, 1.0, 1.0]];
    match TensorBernsteinPatch::try_new(
        numerator,
        weights,
        unit_square_domain(),
        PatchParent::new(0, None),
    ) {
        Ok(patch) => patch,
        Err(refusal) => panic!("the double-collapse fixture refused construction: {refusal:?}"),
    }
}

/// The exact-arithmetic single-collapse fixture (multiplicity 1):
/// `X(u, v) = (1−v)·(u, u, 1)`, weights all 1. The `v = 1` edge collapses to
/// the origin and the normal numerator carries exactly `(1−v)`
/// (`X_u×X_v = −(1−v)·(1, −1, 0)`).
fn single_collapse_patch() -> TensorBernsteinPatch {
    let numerator: Vec<Vec<[f64; 3]>> = vec![
        vec![[0.0, 0.0, 1.0], [0.0, 0.0, 0.0]],
        vec![[1.0, 1.0, 1.0], [0.0, 0.0, 0.0]],
    ];
    let weights: Vec<Vec<f64>> = vec![vec![1.0, 1.0], vec![1.0, 1.0]];
    match TensorBernsteinPatch::try_new(
        numerator,
        weights,
        unit_square_domain(),
        PatchParent::new(0, Some(EdgeId(2))),
    ) {
        Ok(patch) => patch,
        Err(refusal) => panic!("the single-collapse fixture refused construction: {refusal:?}"),
    }
}

#[test]
fn deflation_divides_exactly_zero_remainder() {
    // The closed-form quarter-cone fixture: the v = 1 edge collapses to the
    // apex P, and the normal numerator carries the known factor (1−v)³ (the
    // recorded deflation multiplicity).
    let fixture = patch_fixtures::collapsed_edge_fixture();
    assert_eq!(fixture.multiplicity, 3);
    assert_eq!(fixture.collapsed_side, PatchSide::SideVMax);

    let certificate = expect_deflate(&fixture.patch, fixture.collapsed_side);
    assert_eq!(
        certificate.multiplicity(),
        fixture.multiplicity,
        "the deflation divides the (1−v) factor out exactly, three times"
    );
    assert_eq!(certificate.boundary(), PatchSide::SideVMax);

    // The quotient M* is the fully deflated normal numerator: v-degree 0 (all
    // three (1−v) factors divided out), still carrying the base-arc direction.
    let quotient = certificate.quotient();
    assert_eq!(quotient.degree(), (5, 0));
    assert!(
        quotient
            .coeffs()
            .iter()
            .any(|e| e[0] != 0.0 || e[1] != 0.0 || e[2] != 0.0),
        "the deflated quotient is not the zero polynomial"
    );

    // A boundary that does not collapse exactly is never divided as if exact:
    // the bilinear graph has no v = 1 collapse, so the kernel refuses typed —
    // the zero remainder is proven, not assumed.
    let bilinear = patch_fixtures::bilinear_patch();
    expect_deflate_refusal(
        &bilinear,
        PatchSide::SideVMax,
        ConstructRefusal::InvalidInput,
    );
}

#[test]
fn multiplicity_iteration_handles_double_collapse() {
    // The double-collapse fixture: X = ((1−v)u, (1−v)²u, (1−v)²), whose normal
    // numerator carries exactly (1−v)². The iteration must divide twice — and
    // stop, not greedily divide a quotient whose slice is no longer zero.
    let double = double_collapse_patch();
    let collapsed = surface_point(&double, 0.5, 1.0);
    assert_eq!(
        collapsed,
        [0.0, 0.0, 0.0],
        "the v = 1 edge collapses to the origin"
    );

    let certificate = expect_deflate(&double, PatchSide::SideVMax);
    assert_eq!(
        certificate.multiplicity(),
        2,
        "the double collapse divides (1−v) exactly twice"
    );
    // The quotient still has one v-span left (v-degree 1): two exact divisions
    // reduced the multiplicity-3 net to its multiplicity-1 residue, whose
    // boundary slice is no longer an exact zero — the iteration terminated
    // exactly at the double collapse.
    let quotient = certificate.quotient();
    assert_eq!(quotient.degree(), (1, 1));
    assert!(
        quotient
            .coeffs()
            .iter()
            .any(|e| e[0] != 0.0 || e[1] != 0.0 || e[2] != 0.0),
        "the double-collapse quotient is not the zero polynomial"
    );

    // The same iteration distinguishes a single collapse (multiplicity 1) from
    // the double: X = (1−v)(u, u, 1) divides exactly once.
    let single = single_collapse_patch();
    let certificate = expect_deflate(&single, PatchSide::SideVMax);
    assert_eq!(
        certificate.multiplicity(),
        1,
        "the single collapse divides (1−v) exactly once"
    );
}

#[test]
fn deflated_certificate_passes_interior_fixture() {
    // Deflate the apex-collapse cone, then apply the hemisphere certificate to
    // the quotient M*: c·M* > 0 over the closed box certifies the regular
    // interior (v < 1), the only rank defect being the intentional collapse.
    let fixture = patch_fixtures::collapsed_edge_fixture();
    let certificate = expect_deflate(&fixture.patch, fixture.collapsed_side);
    assert_eq!(certificate.multiplicity(), fixture.multiplicity);

    // The cone opens away from its apex along +z: every coefficient of the
    // deflated normal numerator has a strictly positive z-component, so the
    // +z direction certifies the interior with a strictly positive margin.
    let margin = match certify_deflated_interior(&certificate, [0.0, 0.0, 1.0]) {
        Ok(margin) => margin,
        Err(refusal) => panic!("the deflated cone interior must certify, refused: {refusal:?}"),
    };
    assert!(
        margin > 0.0,
        "the certified coefficient margin is strictly positive"
    );

    // The certificate is not vacuous: the inward direction makes every
    // coefficient dot negative, and the kernel refuses rather than certify.
    assert_eq!(
        certify_deflated_interior(&certificate, [0.0, 0.0, -1.0]),
        Err(ConstructRefusal::ConditioningBelowThreshold)
    );
}

#[test]
fn seam_identity_exact_on_aligned_fixture() {
    // The up/down sphere-octant pair shares the exact base quarter-arc as its
    // u = 0 boundary: the aligned boundary rows are bit-identical, so
    // A₀W₁ − A₁W₀ ≡ 0 exactly and the seam closes.
    let fixture = patch_fixtures::seam_pair_fixture();
    assert!(
        fixture.a.parent().edge.is_some() && fixture.b.parent().edge.is_some(),
        "the seam pair carries the paired BRep-edge handles"
    );
    match seam_identified(&fixture.a, &fixture.b) {
        Ok(true) => {}
        Ok(false) => panic!("the exactly-aligned seam pair must close the seam"),
        Err(refusal) => panic!("the exactly-aligned seam pair must not refuse: {refusal:?}"),
    }

    // A pair that is not a seam candidate — no boundary BRep edge to pair — is
    // Ok(false): there is no seam fact to close or refuse.
    let bilinear = patch_fixtures::bilinear_patch();
    assert_eq!(seam_identified(&bilinear, &bilinear), Ok(false));
}

#[test]
fn misaligned_seam_refuses_typed() {
    // Take the exactly-aligned seam pair and perturb one coefficient of a's
    // shared u = 0 boundary row: the boundaries are now a near-seam — close
    // but not identical. The exact identity A₀W₁ − A₁W₀ ≡ 0 fails, and the
    // kernel REFUSES typed: near-miss is never reported as identity.
    let fixture = patch_fixtures::seam_pair_fixture();

    let mut numerator = fixture.a.numerator().to_vec();
    numerator[0][1][0] += 1.0e-3;
    let perturbed = match TensorBernsteinPatch::try_new(
        numerator,
        fixture.a.weights().to_vec(),
        unit_square_domain(),
        fixture.a.parent(),
    ) {
        Ok(patch) => patch,
        Err(refusal) => panic!("the perturbed seam patch refused construction: {refusal:?}"),
    };
    expect_seam_refusal(&perturbed, &fixture.b, ConstructRefusal::InvalidInput);
}
