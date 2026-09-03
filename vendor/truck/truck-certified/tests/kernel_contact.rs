//! BG-KV2-302-S5A integration tests: the tolerance-tagged contact classifier
//! (`kernel/contact.rs`) over the frozen `CertifiedPatchC2`/`C3` seams — the
//! §10.3 pipeline (Theorem 10.1, Corollary 10.2's classification table,
//! Proposition 10.3's honesty contract).
//!
//! The fixture carriers are polynomial graph patches `(u, v) -> (u, v, h(u,v))`
//! over the shared identity chart (the §10.3 reduced configuration over
//! `Π = n0^⊥` with `n0 = (0,0,1)`), with sound-but-dependency-loose
//! `CertifiedInterval` evaluation so the at-tolerance semantics of Prop 10.3
//! are exercised honestly: a perturbation below the interval resolution is
//! tagged `TangencyAtTolerance`, one above it is certified separated.

#![deny(clippy::unwrap_used)]

use std::fmt::Debug;

use truck_certified::kernel::certs::ContactCert;
use truck_certified::kernel::config;
use truck_certified::kernel::contact::{
    classify_c2, classify_c3, critical_point, ContactGradSystem, ContactReport, SeparationWitness,
};
use truck_certified::kernel::engine::{krawczyk_c1, SquareResidualEval};
use truck_certified::kernel::evidence::ClaimVerdict;
use truck_certified::kernel::graph::TopoNode;
use truck_certified::kernel::patch::{
    CertifiedPatch, CertifiedPatchC2, CertifiedPatchC3, CertifiedPositive, Cone, Degeneracy,
    DerivativeEnclosure, IBox2, IBox3, Pole, Reason, SecondDerivativeEnclosure, ThirdJetEnclosure,
};
use truck_certified::kernel::residual::ResidualId;
use truck_certified::kernel::{Interval, SignCert};

/// The search-box half width used by the at-tolerance fixtures (H-3: the gap
/// width over `[-h, h]^2` is ~`2·h²` `<= TOL_INTERSECTION` for `h = 1e-5`).
const HALF: f64 = 1e-5;
/// The wide search-box half width whose gap width exceeds the tolerance.
const WIDE_HALF: f64 = 1e-3;
/// The genuine-tangency plane height (the unit-sphere cap's tangent plane).
const PLANE_AT_TANGENCY: f64 = 1.0;
/// The deliberate near-tangency perturbation well above the tolerance.
const PERTURB_ABOVE: f64 = 1.0 + 1e-3;
/// The inside-tolerance perturbation (below the certified resolution).
const PERTURB_IN_TOL: f64 = 1.0 + 1e-12;

/// Extract the `Ok` of a fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => panic!("a construction that must succeed was refused: {err:?}"),
    }
}

/// A 2-axis parameter box.
fn box2(lo: [f64; 2], hi: [f64; 2]) -> IBox2 {
    construct(IBox2::try_new(lo, hi))
}

/// The standard at-tolerance search box about the contact chart point `(0,0)`.
fn search_box() -> IBox2 {
    box2([-HALF, -HALF], [HALF, HALF])
}

/// The wide search box (the shrink-and-retry arm).
fn wide_box() -> IBox2 {
    box2([-WIDE_HALF, -WIDE_HALF], [WIDE_HALF, WIDE_HALF])
}

/// An interval from raw endpoints.
fn iv(lo: f64, hi: f64) -> Interval {
    Interval { lo, hi }
}

/// A point interval.
fn pt(x: f64) -> Interval {
    Interval::point(x)
}

/// One certified positive unit weight.
fn positive_one() -> CertifiedPositive {
    construct(CertifiedPositive::try_new(1.0))
}

/// The certified positive weight bound of a patch over a box.
fn weight_of(patch: &GraphPatch, d: IBox2) -> CertifiedPositive {
    match CertifiedPatch::weight_bound(patch, d) {
        Some(ClaimVerdict::Proven(weight)) => weight,
        other => panic!("the graph fixture must certify a weight bound: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// The polynomial graph fixture kit (CertifiedPatch / C2 / C3)
// ---------------------------------------------------------------------------

/// A bivariate polynomial `Σ c[i][j]·u^i·v^j` over the shared chart.
#[derive(Clone)]
struct Poly2 {
    /// `coeffs[i][j]` is the coefficient of `u^i·v^j`.
    coeffs: Vec<Vec<f64>>,
}

impl Poly2 {
    /// The zero polynomial.
    fn zero() -> Self {
        Poly2 { coeffs: Vec::new() }
    }

    /// The polynomial `Σ c·u^du·v^dv` over the given monomial terms.
    fn from_terms(terms: &[(usize, usize, f64)]) -> Self {
        let mut du = 0usize;
        let mut dv = 0usize;
        for (i, j, _) in terms {
            du = du.max(*i);
            dv = dv.max(*j);
        }
        let mut coeffs = vec![vec![0.0; dv + 1]; du + 1];
        for (i, j, c) in terms {
            coeffs[*i][*j] += c;
        }
        Poly2 { coeffs }
    }

    /// The degree in `u`, `0` for the zero polynomial.
    fn deg_u(&self) -> usize {
        if self.coeffs.is_empty() {
            0
        } else {
            self.coeffs.len() - 1
        }
    }

    /// The degree in `v`, `0` for the zero polynomial.
    fn deg_v(&self) -> usize {
        if self.coeffs.is_empty() {
            0
        } else {
            self.coeffs[0].len() - 1
        }
    }

    /// The certified interval evaluation over the box `d` (dependency-loose
    /// `CertifiedInterval` arithmetic; the result is a sound enclosure).
    fn eval(&self, d: IBox2) -> Interval {
        if self.coeffs.is_empty() {
            return pt(0.0);
        }
        let u = iv(d.lo[0], d.hi[0]);
        let v = iv(d.lo[1], d.hi[1]);
        let mut upow = vec![pt(1.0)];
        for _ in 1..=self.deg_u() {
            let next = upow.last().expect("power list is non-empty").mul(&u);
            upow.push(next);
        }
        let mut vpow = vec![pt(1.0)];
        for _ in 1..=self.deg_v() {
            let next = vpow.last().expect("power list is non-empty").mul(&v);
            vpow.push(next);
        }
        let mut acc = pt(0.0);
        for (i, row) in self.coeffs.iter().enumerate() {
            for (j, c) in row.iter().enumerate() {
                if *c == 0.0 {
                    continue;
                }
                let term = pt(*c).mul(&upow[i]).mul(&vpow[j]);
                acc = acc.add(&term);
            }
        }
        acc
    }

    /// The partial derivative in `u`.
    fn du(&self) -> Poly2 {
        if self.coeffs.is_empty() || self.deg_u() == 0 {
            return Poly2::zero();
        }
        let mut out = vec![Vec::new(); self.deg_u()];
        for i in 1..=self.deg_u() {
            let mut row = vec![0.0; self.deg_v() + 1];
            for (j, c) in self.coeffs[i].iter().enumerate() {
                row[j] = i as f64 * c;
            }
            out[i - 1] = row;
        }
        Poly2 { coeffs: out }
    }

    /// The partial derivative in `v`.
    fn dv(&self) -> Poly2 {
        if self.coeffs.is_empty() || self.deg_v() == 0 {
            return Poly2::zero();
        }
        let mut out = Vec::with_capacity(self.deg_u() + 1);
        for row in &self.coeffs {
            let mut out_row = vec![0.0; self.deg_v()];
            for (j, c) in row.iter().enumerate().skip(1) {
                out_row[j - 1] = j as f64 * c;
            }
            out.push(out_row);
        }
        Poly2 { coeffs: out }
    }
}

/// A polynomial graph patch `S(u, v) = (u, v, h(u, v))` over the shared
/// identity chart: a `CertifiedPatch`/`C2`/`C3` fixture carrier. The height
/// `h` is the polynomial stored in `z`.
#[derive(Clone)]
struct GraphPatch {
    /// The graph height polynomial.
    z: Poly2,
}

impl GraphPatch {
    /// A patch whose height is the given polynomial.
    fn from_terms(terms: &[(usize, usize, f64)]) -> Self {
        GraphPatch {
            z: Poly2::from_terms(terms),
        }
    }

    /// The constant-height plane `z = c`.
    fn plane(c: f64) -> Self {
        GraphPatch::from_terms(&[(0, 0, c)])
    }

    /// The unit-sphere cap local model `z = 1 − (u² + v²)/2`: the unit sphere
    /// tangent to `z = 1` at the chart origin, exact through its quadratic jet.
    fn sphere_cap() -> Self {
        GraphPatch::from_terms(&[(0, 0, PLANE_AT_TANGENCY), (2, 0, -0.5), (0, 2, -0.5)])
    }

    /// The saddle graph `z = −u·v`, tangent to `z = 0` at the origin with a
    /// crossing (indefinite) contact.
    fn saddle() -> Self {
        GraphPatch::from_terms(&[(1, 1, -1.0)])
    }

    /// The A2 cusp graph `z = u² + v³`, tangent to `z = 0` at the origin with
    /// a cubic degeneracy in the `v` direction.
    fn cusp() -> Self {
        GraphPatch::from_terms(&[(2, 0, 1.0), (0, 3, 1.0)])
    }

    /// A transverse slant plane `z = u` (no stationary gap with a flat plane).
    fn slant() -> Self {
        GraphPatch::from_terms(&[(1, 0, 1.0)])
    }
}

/// The certified box of an interval triple.
fn box3_of(x: Interval, y: Interval, z: Interval) -> IBox3 {
    construct(IBox3::try_new([x.lo, y.lo, z.lo], [x.hi, y.hi, z.hi]))
}

impl CertifiedPatch for GraphPatch {
    fn enclose(&self, d: IBox2) -> IBox3 {
        let x = iv(d.lo[0], d.hi[0]);
        let y = iv(d.lo[1], d.hi[1]);
        let z = self.z.eval(d);
        box3_of(x, y, z)
    }

    fn derivs(&self, d: IBox2) -> DerivativeEnclosure {
        let hu = self.z.du().eval(d);
        let hv = self.z.dv().eval(d);
        DerivativeEnclosure {
            su: box3_of(pt(1.0), pt(0.0), hu),
            sv: box3_of(pt(0.0), pt(1.0), hv),
        }
    }

    fn normal_cone(&self, d: IBox2) -> Cone {
        let _ = d;
        let de = self.derivs(d);
        let _ = de;
        // The graphs over the fixture chart are near-horizontal over the boxes
        // the tests use, so the open `+z` hemisphere cone is a sound normal
        // cone there. The axis is a unit coordinate vector and the half-angle
        // is in `[0, PI)`, so the cone always constructs.
        Cone {
            axis: [0.0, 0.0, 1.0],
            half_angle: std::f64::consts::FRAC_PI_2,
        }
    }

    fn regularity(&self, d: IBox2) -> ClaimVerdict<CertifiedPositive, Degeneracy, Reason> {
        let de = self.derivs(d);
        let e = dot_iv(de.su, de.su);
        let g = dot_iv(de.sv, de.sv);
        let f = dot_iv(de.su, de.sv);
        let egf2 = e.mul(&g).sub(&f.mul(&f));
        if egf2.lo > config::TOL_JACOBIAN {
            match CertifiedPositive::try_new(egf2.lo) {
                Ok(positive) => ClaimVerdict::Proven(positive),
                Err(_) => ClaimVerdict::Inconclusive("fixture_regularity_bound_refused"),
            }
        } else if egf2.hi < config::TOL_JACOBIAN {
            ClaimVerdict::Disproven(Degeneracy {
                box_: d,
                egf2: (egf2.lo, egf2.hi),
            })
        } else {
            ClaimVerdict::Inconclusive("fixture_regularity_straddles_floor")
        }
    }

    fn weight_bound(&self, d: IBox2) -> Option<ClaimVerdict<CertifiedPositive, Pole, Reason>> {
        let _ = d;
        // The identity-chart graph patches carry the constant unit weight.
        Some(ClaimVerdict::Proven(positive_one()))
    }
}

impl CertifiedPatchC2 for GraphPatch {
    fn second_derivs(&self, d: IBox2) -> SecondDerivativeEnclosure {
        let hu = self.z.du();
        let hv = self.z.dv();
        let huu = hu.du().eval(d);
        let huv = hu.dv().eval(d);
        let hvv = hv.dv().eval(d);
        SecondDerivativeEnclosure {
            suu: box3_of(pt(0.0), pt(0.0), huu),
            suv: box3_of(pt(0.0), pt(0.0), huv),
            svv: box3_of(pt(0.0), pt(0.0), hvv),
        }
    }
}

impl CertifiedPatchC3 for GraphPatch {
    fn third_jet(&self, d: IBox2) -> ThirdJetEnclosure {
        let hu = self.z.du();
        let hv = self.z.dv();
        let huuu = hu.du().du().eval(d);
        let huuv = hu.du().dv().eval(d);
        let huvv = hu.dv().dv().eval(d);
        let hvvv = hv.dv().dv().eval(d);
        ThirdJetEnclosure {
            suuu: box3_of(pt(0.0), pt(0.0), huuu),
            suuv: box3_of(pt(0.0), pt(0.0), huuv),
            suvv: box3_of(pt(0.0), pt(0.0), huvv),
            svvv: box3_of(pt(0.0), pt(0.0), hvvv),
        }
    }
}

/// A CertifiedPatchC2-only view of a patch: delegates the C1/C2 data but does
/// NOT implement `CertifiedPatchC3` — the A2 branch's trait boundary.
#[derive(Clone)]
struct WithoutC3<T>(T);

impl<T: CertifiedPatchC2> CertifiedPatch for WithoutC3<T> {
    fn enclose(&self, d: IBox2) -> IBox3 {
        CertifiedPatch::enclose(&self.0, d)
    }
    fn derivs(&self, d: IBox2) -> DerivativeEnclosure {
        CertifiedPatch::derivs(&self.0, d)
    }
    fn normal_cone(&self, d: IBox2) -> Cone {
        CertifiedPatch::normal_cone(&self.0, d)
    }
    fn regularity(&self, d: IBox2) -> ClaimVerdict<CertifiedPositive, Degeneracy, Reason> {
        CertifiedPatch::regularity(&self.0, d)
    }
    fn weight_bound(&self, d: IBox2) -> Option<ClaimVerdict<CertifiedPositive, Pole, Reason>> {
        CertifiedPatch::weight_bound(&self.0, d)
    }
}

impl<T: CertifiedPatchC2> CertifiedPatchC2 for WithoutC3<T> {
    fn second_derivs(&self, d: IBox2) -> SecondDerivativeEnclosure {
        CertifiedPatchC2::second_derivs(&self.0, d)
    }
}

/// The interval dot product of two enclosure boxes.
fn dot_iv(a: IBox3, b: IBox3) -> Interval {
    let x = iv(a.lo[0], a.hi[0]).mul(&iv(b.lo[0], b.hi[0]));
    let y = iv(a.lo[1], a.hi[1]).mul(&iv(b.lo[1], b.hi[1]));
    let z = iv(a.lo[2], a.hi[2]).mul(&iv(b.lo[2], b.hi[2]));
    x.add(&y).add(&z)
}

/// Extract the Proven arm of a claim (a test helper, never on certified code).
fn proven_contact(report: &ContactReport) -> ContactCert {
    match &report.claim {
        ClaimVerdict::Proven(cert) => *cert,
        ClaimVerdict::Disproven(witness) => {
            panic!("expected a Proven contact, got Disproven: {witness:?}")
        }
        ClaimVerdict::Inconclusive(reason) => {
            panic!("expected a Proven contact, got Inconclusive: {reason}")
        }
    }
}

/// Extract the Disproven arm of a claim.
fn disproven_witness(report: &ContactReport) -> SeparationWitness {
    match &report.claim {
        ClaimVerdict::Disproven(witness) => *witness,
        ClaimVerdict::Proven(cert) => panic!("expected a Disproven claim, got Proven: {cert:?}"),
        ClaimVerdict::Inconclusive(reason) => {
            panic!("expected a Disproven claim, got Inconclusive: {reason}")
        }
    }
}

/// The common unit normal at the contact: the `+z` axis.
const N0: [f64; 3] = [0.0, 0.0, 1.0];

#[test]
fn critical_point_of_nabla_g_certifies_square_c1() {
    // The genuine sphere-cap/plane tangency at the chart origin.
    let plane = GraphPatch::plane(PLANE_AT_TANGENCY);
    let cap = GraphPatch::sphere_cap();
    let b = search_box();

    let cert = match critical_point(&plane, &cap, N0, b) {
        ClaimVerdict::Proven(cert) => cert,
        ClaimVerdict::Disproven(refusal) => {
            panic!("the tangency critical point must certify Proven: {refusal:?}")
        }
        ClaimVerdict::Inconclusive(reason) => {
            panic!("the tangency critical point must certify Proven: {reason}")
        }
    };
    assert!(cert.rho <= config::RHO_MAX, "rho must satisfy the ceiling");
    assert_eq!(cert.box_, b);
    // The exact critical point is the chart origin (the model point (0,0,1)).
    assert!(
        cert.box_.lo[0] <= 0.0
            && 0.0 <= cert.box_.hi[0]
            && cert.box_.lo[1] <= 0.0
            && 0.0 <= cert.box_.hi[1],
        "the certified box must contain the chart-origin critical point"
    );
    assert_eq!(cert.residual, ResidualId::R4Prime);

    // S2A backing-table parity: the same residual through the frozen square
    // C1 (`krawczyk_c1`) certifies the same point over the same box.
    let weights = vec![weight_of(&plane, b), weight_of(&cap, b)];
    let system = ContactGradSystem {
        first: &plane,
        second: &cap,
        n0: N0,
    };
    assert_eq!(system.arity(), 2);
    match krawczyk_c1(&system, b, &weights) {
        ClaimVerdict::Proven(parity) => {
            assert_eq!(parity.box_, b, "the parity C1 runs over the same box");
            assert!(
                (parity.rho - cert.rho).abs() <= f64::EPSILON,
                "the classifier's critical point is the S2A C1's point"
            );
            // The engine stamps R1; the classifier rebuilds with R4Prime.
            assert_eq!(parity.residual, ResidualId::R1);
        }
        ClaimVerdict::Disproven(refusal) => {
            panic!("the S2A parity C1 must certify Proven: {refusal:?}")
        }
        ClaimVerdict::Inconclusive(reason) => {
            panic!("the S2A parity C1 must certify Proven: {reason}")
        }
    }
}

#[test]
fn hessian_sign_classifies_saddle_extremum_and_perturbed_refusal() {
    // MorseSaddle: the saddle surface tangent to `z = 0` with a crossing.
    let table = GraphPatch::plane(0.0);
    let saddle = GraphPatch::saddle();
    let b = search_box();
    let report = classify_c2(&table, &saddle, N0, false, b);
    assert_eq!(report.kind, Some(TopoNode::MorseSaddle));
    let cert = proven_contact(&report);
    assert_eq!(cert.hessian_sign, SignCert::Negative);
    assert!(cert.gap.contains(0.0));

    // The n2 = -n0 flip (Theorem 10.1 verbatim): flipping the second
    // carrier's normal convention leaves the classification invariant.
    let flipped = classify_c2(&table, &saddle, N0, true, b);
    assert_eq!(flipped.kind, Some(TopoNode::MorseSaddle));
    let flipped_cert = proven_contact(&flipped);
    assert_eq!(
        flipped_cert.hessian_sign, cert.hessian_sign,
        "the classification is invariant under the n2 = -n0 flip"
    );

    // MorseExtremum: the genuine sphere-cap/plane tangency.
    let plane = GraphPatch::plane(PLANE_AT_TANGENCY);
    let cap = GraphPatch::sphere_cap();
    let report = classify_c2(&plane, &cap, N0, false, b);
    assert_eq!(report.kind, Some(TopoNode::MorseExtremum));
    let cert = proven_contact(&report);
    assert_eq!(cert.hessian_sign, SignCert::Positive);
    assert!(cert.gap.contains(0.0));

    // The perturbed refusal: the plane lifted `1e-3` above the tangency is
    // certified separated, never a contact (Prop 10.3's honesty).
    let lifted = GraphPatch::plane(PERTURB_ABOVE);
    let report = classify_c2(&lifted, &cap, N0, false, b);
    let witness = disproven_witness(&report);
    assert!(
        !witness.gap.contains(0.0),
        "the lifted plane must carry a certified gap excluding zero"
    );
}

#[test]
fn contact_cert_three_valued_contract_holds() {
    let cap = GraphPatch::sphere_cap();
    let b = search_box();
    let wide = wide_box();

    // Arm 1 (Proven): the genuine contact.
    let genuine = GraphPatch::plane(PLANE_AT_TANGENCY);
    let cert = proven_contact(&classify_c2(&genuine, &cap, N0, false, b));
    assert_eq!(cert.tolerance, config::TOL_INTERSECTION);
    assert!(cert.gap.contains(0.0));
    assert!(cert.gap.width() <= cert.tolerance);

    // Arm 2 (Disproven): a separation well above the tolerance.
    let above = GraphPatch::plane(PERTURB_ABOVE);
    let witness = disproven_witness(&classify_c2(&above, &cap, N0, false, b));
    assert!(
        !witness.gap.contains(0.0),
        "a certified separation excludes zero from the gap"
    );

    // Arm 3 (Proven, tolerance-relative honesty): a perturbation inside the
    // tolerance cannot be distinguished from tangency (Prop 10.3), so the
    // claim is tagged TangencyAtTolerance — never an exact certificate.
    let in_tol = GraphPatch::plane(PERTURB_IN_TOL);
    let report = classify_c2(&in_tol, &cap, N0, false, b);
    let cert = proven_contact(&report);
    assert_eq!(cert.tolerance, config::TOL_INTERSECTION);
    assert!(
        cert.gap.contains(0.0),
        "an inside-tolerance perturbation is within the certified resolution"
    );
    assert!(cert.gap.width() <= cert.tolerance);

    // The Inconclusive arm (Prop 10.3: shrink and retry): the genuine pair
    // over a wide box has `0 ∈ gap` with `width > tolerance`.
    let report = classify_c2(&genuine, &cap, N0, false, wide);
    match &report.claim {
        ClaimVerdict::Inconclusive(_) => {}
        ClaimVerdict::Proven(cert) => {
            panic!("a wide-box gap must not certify at tolerance: {cert:?}")
        }
        ClaimVerdict::Disproven(witness) => {
            panic!("a wide-box genuine contact must not be Disproven: {witness:?}")
        }
    }

    // The certificate constructor is the Proven case ONLY (rule 7): it refuses
    // a gap excluding zero and a gap wider than the tolerance.
    let point = match critical_point(&genuine, &cap, N0, b) {
        ClaimVerdict::Proven(point) => point,
        other => panic!("the critical point must certify: {other:?}"),
    };
    let gap_away = pt(1e-3);
    assert!(ContactCert::try_new(point, gap_away, SignCert::Positive).is_err());
    let gap_wide = iv(-1e-3, 1e-3);
    assert!(ContactCert::try_new(point, gap_wide, SignCert::Positive).is_err());
}

#[test]
fn perturbed_near_tangency_returns_disproven_with_certified_gap() {
    // The deliberately perturbed near-tangency (plane `1e-3` above the cap):
    // the classifier must not fabricate a false contact; the Disproven arm
    // carries a certified gap excluding zero and the ordinary path resumes.
    let lifted = GraphPatch::plane(PERTURB_ABOVE);
    let cap = GraphPatch::sphere_cap();
    let b = search_box();
    let report = classify_c2(&lifted, &cap, N0, false, b);
    let witness = disproven_witness(&report);
    assert!(
        !witness.gap.contains(0.0),
        "the certified gap must exclude zero: {:?}",
        witness.gap
    );
    assert!(
        witness.gap.lo > 0.0 || witness.gap.hi < 0.0,
        "the certified gap must be signed away from zero"
    );

    // A separated pair with no stationary point (the transversal case never
    // consults the classifier for a node) must never issue a Proven contact.
    let base = GraphPatch::plane(0.0);
    let slant = GraphPatch::slant();
    let report = classify_c2(&base, &slant, N0, false, b);
    assert_eq!(report.kind, None);
    match &report.claim {
        ClaimVerdict::Proven(cert) => {
            panic!("a transversal pair must never certify a contact: {cert:?}")
        }
        ClaimVerdict::Disproven(_) | ClaimVerdict::Inconclusive(_) => {}
    }

    // Two separated parallel planes (no contact, certified separation).
    let far = GraphPatch::plane(PERTURB_ABOVE);
    let report = classify_c2(&base, &far, N0, false, b);
    let witness = disproven_witness(&report);
    assert!(!witness.gap.contains(0.0));
}

#[test]
fn genuine_contact_returns_tangency_at_tolerance() {
    let plane = GraphPatch::plane(PLANE_AT_TANGENCY);
    let cap = GraphPatch::sphere_cap();
    let b = search_box();

    let report = classify_c2(&plane, &cap, N0, false, b);
    assert_eq!(report.kind, Some(TopoNode::MorseExtremum));
    let cert = proven_contact(&report);

    // The tolerance-tagged claim: honest, never unified with an exact cert.
    assert_eq!(cert.tolerance, config::TOL_INTERSECTION);
    assert!(cert.gap.contains(0.0), "the tangency gap contains zero");
    assert!(
        cert.gap.width() <= cert.tolerance,
        "the tangency gap width {:?} fits the tolerance",
        cert.gap.width()
    );
    // The exact components are ordinary certificates.
    assert_eq!(cert.critical_point.residual, ResidualId::R4Prime);
    assert!(cert.critical_point.rho <= config::RHO_MAX);
    assert_eq!(cert.hessian_sign, SignCert::Positive);
}

#[test]
fn a2_cusp_branch_needs_c3_and_refuses_without_it() {
    // The A2 cusp fixture: `z = u² + v³` tangent to `z = 0` at the origin
    // (rank-1 Hessian `diag(2, 0)` with the cubic `v³` in the null direction).
    let table = GraphPatch::plane(0.0);
    let cusp = GraphPatch::cusp();
    let b = search_box();

    // The C2-only implementor (the trait boundary is the audit): the A2
    // branch refuses HighOrderJet — no class is claimed.
    let table_c2 = WithoutC3(table.clone());
    let cusp_c2 = WithoutC3(cusp.clone());
    let report = classify_c2(&table_c2, &cusp_c2, N0, false, b);
    assert_eq!(
        report.kind, None,
        "a C2-only implementor cannot certify the A2 cusp class"
    );
    assert!(
        !matches!(report.claim, ClaimVerdict::Proven(_)),
        "a C2-only implementor must refuse the cusp contact"
    );

    // The C3 implementor classifies the cusp: certified rank-1 Hessian plus a
    // certified nonzero cubic in the null direction.
    let report = classify_c3(&table, &cusp, N0, false, b);
    assert_eq!(report.kind, Some(TopoNode::A2Cusp));
}

#[test]
fn no_r5_enclosure_required_for_classification() {
    // The audit row (spec §20, S5a): the Corollary 10.2 classification needs
    // NO R5Enclosure in scope — only the C2/C3 jets over the box. The source
    // scan pins the token out of the module entirely.
    let source = include_str!("../src/kernel/contact.rs");
    let needle = "R5Enclosure";
    for (line_no, raw) in source.lines().enumerate() {
        let code = match raw.find("//") {
            Some(index) => &raw[..index],
            None => raw,
        };
        assert!(
            !code.contains(needle),
            "contact.rs must not reference the R5Enclosure certificate on line {}: {code}",
            line_no + 1
        );
    }

    // And the classification runs end to end on the C3 fixtures without one.
    let table = GraphPatch::plane(0.0);
    let cusp = GraphPatch::cusp();
    let b = search_box();
    let report = classify_c3(&table, &cusp, N0, false, b);
    assert_eq!(report.kind, Some(TopoNode::A2Cusp));
}

#[test]
fn no_transcendental_call_in_contact_module() {
    // N4: no transcendental function call may appear in contact.rs.
    let source = include_str!("../src/kernel/contact.rs");
    let banned = ["sin", "cos", "atan2", "exp", "ln", "log", "powf", "sqrt"];
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    for (line_no, raw) in source.lines().enumerate() {
        let code = match raw.find("//") {
            Some(index) => &raw[..index],
            None => raw,
        };
        for token in banned {
            for (at, _) in code.match_indices(token) {
                let after = at + token.len();
                let left_clear = code[..at].chars().next_back().is_none_or(|c| !is_word(c));
                let right_clear = code[after..].chars().next().is_none_or(|c| !is_word(c));
                assert!(
                    !(left_clear && right_clear),
                    "line {} carries the transcendental call token {token}: {code}",
                    line_no + 1
                );
            }
        }
    }
}
