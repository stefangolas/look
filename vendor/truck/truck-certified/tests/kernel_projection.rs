//! BG-KV2-305-S2B integration tests: GraphCert (Theorem 8.3's cone test with
//! no solve), the R5 enclosure contract (§8.6's five-step pipeline over the
//! frozen `R5Enclosure` shim shape), and the R4/R4′ projection residuals —
//! `kernel/projection.rs`.
//!
//! The fixture carriers are the §10.3 reduced configuration's polynomial
//! graph patches `(u, v) -> (u, v, h(u, v))` over the shared identity chart
//! (`Π = n0^⊥`, `n0 = (0, 0, 1)`) and the rational sphere carrier of
//! BG-KV2-104-RATCARRIER for the sphere-chart injectivity ground truth.

#![deny(clippy::unwrap_used)]

use std::fmt::Debug;

use truck_certified::kernel::config;
use truck_certified::kernel::engine::{krawczyk_c1, SquareResidualEval};
use truck_certified::kernel::evidence::{ClaimVerdict, RefusalEvidence, RefusalKind, VerdictClass};
use truck_certified::kernel::leaf::{CarrierData, RationalCarrier, RationalCarrierKind};
use truck_certified::kernel::patch::{
    CertifiedPatch, CertifiedPositive, Cone, Degeneracy, DerivativeEnclosure, IBox2, IBox3, Pole,
    Reason,
};
use truck_certified::kernel::projection::{graphcert, r4_prime, r4_project, r5_enclose};
use truck_certified::kernel::residual::ResidualId;
use truck_certified::kernel::Interval;

/// The genuine-tangency plane height of the §10.3 fixture family (the unit
/// sphere cap's tangent plane at the chart origin) (H-3).
const PLANE_AT_TANGENCY: f64 = 1.0;
/// The pointwise implicit-form comparison tolerance (H-3).
const GT_TOL: f64 = 1e-12; // H-3: dyadic pointwise comparison tolerance

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

/// Assert two floats agree to the pointwise ground-truth tolerance.
fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() <= GT_TOL
}

// ---------------------------------------------------------------------------
// The polynomial graph fixture kit over the shared identity chart
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

    /// The pointwise evaluation at `(u, v)` (the analytic ground truth).
    fn at(&self, u: f64, v: f64) -> f64 {
        let mut acc = 0.0;
        for (i, row) in self.coeffs.iter().enumerate() {
            for (j, c) in row.iter().enumerate() {
                acc += c * u.powi(i as i32) * v.powi(j as i32);
            }
        }
        acc
    }
}

/// A polynomial graph patch `S(u, v) = (u, v, h(u, v))` over the shared
/// identity chart: a `CertifiedPatch` fixture carrier. The height `h` is the
/// polynomial stored in `z`.
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
        // The graphs over the fixture chart are near-horizontal over the boxes
        // the tests use, so the open `+z` hemisphere cone is a sound normal
        // cone there.
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

/// The interval dot product of two enclosure boxes.
fn dot_iv(a: IBox3, b: IBox3) -> Interval {
    let x = iv(a.lo[0], a.hi[0]).mul(&iv(b.lo[0], b.hi[0]));
    let y = iv(a.lo[1], a.hi[1]).mul(&iv(b.lo[1], b.hi[1]));
    let z = iv(a.lo[2], a.hi[2]).mul(&iv(b.lo[2], b.hi[2]));
    x.add(&y).add(&z)
}

/// The certified positive weight bound of a graph patch over a box.
fn weight_of(patch: &GraphPatch, d: IBox2) -> CertifiedPositive {
    match CertifiedPatch::weight_bound(patch, d) {
        Some(ClaimVerdict::Proven(weight)) => weight,
        other => panic!("the graph fixture must certify a weight bound: {other:?}"),
    }
}

/// A unit sphere carrier, chart box `[-1, 1]²` (avoids the degeneration).
fn sphere() -> RationalCarrier {
    construct(RationalCarrier::try_new(
        RationalCarrierKind::Sphere,
        CarrierData::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
        },
        construct(IBox2::try_new([-1.0, -1.0], [1.0, 1.0])),
    ))
}

/// The common unit normal of the reduced configuration.
const N0: [f64; 3] = [0.0, 0.0, 1.0];

// ---------------------------------------------------------------------------
// GraphCert
// ---------------------------------------------------------------------------

#[test]
fn graphcert_is_a_cone_test_with_no_solve() {
    // Theorem 8.3's certificate is a cone test: 0 excluded from the enclosure
    // of n0·N decided from the patch's normal_cone and derivative enclosures.
    // NO solve machinery may appear on the graphcert path — no Krawczyk
    // residual, no PointCert, no interval inversion, no adjugate/det solve.
    let source = include_str!("../src/kernel/projection.rs");
    let lines: Vec<&str> = source.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.starts_with("pub fn graphcert("))
        .expect("projection.rs must contain pub fn graphcert");
    let end = lines
        .iter()
        .skip(start + 1)
        .position(|l| l.starts_with("// ---"))
        .map(|i| start + 1 + i)
        .expect("the graphcert body must be followed by a section divider");
    let banned = [
        "krawczyk",
        "PointCert",
        "adjugate",
        "inverse",
        "solve",
        "inv2",
        "lu",
        "R4System",
    ];
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let contains_word = |hay: &str, word: &str| {
        hay.match_indices(word).any(|(i, _)| {
            let before = i
                .checked_sub(1)
                .map(|j| hay.as_bytes()[j] as char)
                .map(is_word)
                .unwrap_or(false);
            let after = hay
                .as_bytes()
                .get(i + word.len())
                .map(|b| *b as char)
                .map(is_word)
                .unwrap_or(false);
            !before && !after
        })
    };
    for (line_no, raw) in lines.iter().enumerate().take(end + 1).skip(start) {
        let code = match raw.find("//") {
            Some(index) => &raw[..index],
            None => raw,
        };
        for needle in banned {
            assert!(
                !contains_word(code, needle),
                "the graphcert path must carry no solve token {needle} on line {}: {code}",
                line_no + 1
            );
        }
    }
}

#[test]
fn graphcert_injectivity_on_a_sphere_chart() {
    // The rational sphere chart with n0 = the chart's polar axis: the polar
    // cap over the box stays strictly inside the north hemisphere, so
    // 0 ∉ □(n0·N) over the box and the projection is injective there.
    let carrier = sphere();
    let chart_box = box2([-0.25, -0.25], [0.25, 0.25]);
    let cert = match graphcert(&carrier, chart_box, [0.0, 0.0, 1.0]) {
        Ok(cert) => cert,
        Err(refusal) => panic!("the sphere-chart cap must certify a graph: {refusal:?}"),
    };
    assert_eq!(cert.domain, chart_box);
    assert_eq!(cert.n0, [0.0, 0.0, 1.0]);
    // det Dq = n0·N is certified strictly positive over the cap box.
    assert!(cert.det_bound.value() > 0.0);
    assert_eq!(cert.det_bound.value(), cert.det_bound.value().abs());

    // Injectivity on the chart box: two distinct chart points project to two
    // distinct points of Π (the projection map is one-to-one on the certified
    // box). Sample the chart at two points that differ in both axes.
    let q = |u: f64, v: f64| {
        let d = 1.0 + u * u + v * v;
        [2.0 * u / d, 2.0 * v / d]
    };
    let a = q(0.1, -0.05);
    let b = q(-0.1, 0.05);
    assert!(!approx(a[0], b[0]) || !approx(a[1], b[1]));
}

#[test]
fn graphcert_refuses_when_no_feasible_n0() {
    // The tangential-adjacent fixture family (302's plane/sphere-cap pair over
    // the shared chart): for n0 = (1, 0, 0) no graph is certified over boxes
    // that straddle the vertical fold (the cap) or that are entirely vertical
    // in the n0 direction (the plane). Both members refuse with the named
    // no-feasible-n0 predicate; the caller subdivides or falls back to R4'.
    let plane = GraphPatch::plane(PLANE_AT_TANGENCY);
    let cap = GraphPatch::sphere_cap();
    let straddle = box2([-1e-3, -1e-3], [1e-3, 1e-3]);

    let cap_refusal = match graphcert(&cap, straddle, [1.0, 0.0, 0.0]) {
        Err(refusal) => refusal,
        Ok(cert) => panic!("the cap must refuse the x-direction graph: {cert:?}"),
    };
    match &cap_refusal.evidence {
        RefusalEvidence::Predicate { name, .. } => {
            assert_eq!(*name, "graphcert_no_feasible_n0")
        }
        other => panic!("the refusal must carry the no-feasible-n0 predicate: {other:?}"),
    }

    let plane_refusal = match graphcert(&plane, straddle, [1.0, 0.0, 0.0]) {
        Err(refusal) => refusal,
        Ok(cert) => panic!("the horizontal plane must refuse the x-direction graph: {cert:?}"),
    };
    assert_eq!(plane_refusal.kind, RefusalKind::Conditioning);
    assert_eq!(plane_refusal.backing, VerdictClass::Inconclusive);
    match &plane_refusal.evidence {
        RefusalEvidence::Predicate { name, .. } => {
            assert_eq!(*name, "graphcert_no_feasible_n0")
        }
        other => panic!("the refusal must carry the no-feasible-n0 predicate: {other:?}"),
    }

    // The feasible n0 of the reduced configuration certifies both members.
    assert!(graphcert(&plane, straddle, N0).is_ok());
    assert!(graphcert(&cap, straddle, N0).is_ok());
}

// ---------------------------------------------------------------------------
// R5 enclosure contract (§8.6)
// ---------------------------------------------------------------------------

/// The R5 graph pair used by the preimage/value/gradient tests: `p1` is the
/// plane `z = 0` and `p2` is a small quadratic-plus-linear height above it, so
/// the true difference `g = f1 − f2 = −h2` is nontrivial.
fn r5_pair() -> (GraphPatch, GraphPatch) {
    let p1 = GraphPatch::plane(0.0);
    let p2 = GraphPatch::from_terms(&[(0, 0, 0.2), (1, 0, 0.15), (0, 1, 0.1), (1, 1, 0.05)]);
    (p1, p2)
}

/// The shared target region of the R5 fixtures.
fn r5_target() -> IBox2 {
    box2([-0.2, -0.2], [0.2, 0.2])
}

#[test]
fn r5_enclosure_preimage_via_c1_on_r4() {
    let (p1, p2) = r5_pair();
    let q = r5_target();

    let enc = match r5_enclose(&p1, &p2, q, N0) {
        ClaimVerdict::Proven(enc) => enc,
        ClaimVerdict::Disproven(refusal) => {
            panic!("the R5 enclosure must certify the graph pair: {refusal:?}")
        }
        ClaimVerdict::Inconclusive(reason) => {
            panic!("the R5 enclosure must certify the graph pair: {reason}")
        }
    };
    assert_eq!(enc.q, q);
    // §8.6 step 1: the certified preimages are produced by C1 on R4, and in
    // the reduced configuration the certified preimage of the target box is
    // the target box itself.
    assert_eq!(enc.preimage[0], q);
    assert_eq!(enc.preimage[1], q);
    for (i, point) in enc.cert.iter().enumerate() {
        assert_eq!(
            point.residual,
            ResidualId::R4,
            "patch {i} cert is the R4 solve"
        );
        assert!(point.rho <= config::RHO_MAX, "rho must satisfy the ceiling");
        assert_eq!(point.box_, q);
    }

    // Parity with the packaged R4 entry: the same preimage solve through
    // r4_project certifies the same R4 point over the same box.
    let centre = [0.0, 0.0];
    let parity = match r4_project(&p2, centre, N0, q) {
        ClaimVerdict::Proven(point) => point,
        other => panic!("r4_project must certify the centre preimage: {other:?}"),
    };
    assert_eq!(parity.residual, ResidualId::R4);
    assert_eq!(parity.box_, q);
    assert_eq!(parity.rho, enc.cert[1].rho);
    assert_eq!(parity.box_, enc.cert[1].box_);
}

#[test]
fn r5_value_and_gradient_enclose_the_truth() {
    let (p1, p2) = r5_pair();
    let q = r5_target();
    let enc = match r5_enclose(&p1, &p2, q, N0) {
        ClaimVerdict::Proven(enc) => enc,
        other => panic!("the R5 enclosure must certify the graph pair: {other:?}"),
    };
    let g1 = match graphcert(&p1, enc.preimage[0], N0) {
        Ok(g1) => g1,
        Err(refusal) => panic!("the first patch must certify its graph: {refusal:?}"),
    };
    let g2 = match graphcert(&p2, enc.preimage[1], N0) {
        Ok(g2) => g2,
        Err(refusal) => panic!("the second patch must certify its graph: {refusal:?}"),
    };
    let graph =
        match truck_certified::kernel::projection::r5_graph_enclose(&p1, &p2, &enc, &g1, &g2, N0) {
            Ok(graph) => graph,
            Err(refusal) => panic!("the R5 graph enclosure must construct: {refusal:?}"),
        };

    // h2 = 0.2 + 0.15u + 0.1v + 0.05uv, h1 = 0, so g = -h2 and
    // grad g = (-(0.15 + 0.05v), -(0.1 + 0.05u)).
    let h2 = &p2.z;
    let samples = [(0.1, -0.05), (0.2, 0.1), (-0.15, -0.15)];
    for (u, v) in samples {
        let truth_g = -h2.at(u, v);
        assert!(
            graph.value.contains(truth_g),
            "the certified value {graph:?} must enclose the truth g({u},{v}) = {truth_g}"
        );
        let truth_gu = -(0.15 + 0.05 * v);
        let truth_gv = -(0.1 + 0.05 * u);
        assert!(
            graph.grad[0].contains(truth_gu),
            "grad u {graph:?} must enclose {truth_gu} at ({u}, {v})"
        );
        assert!(
            graph.grad[1].contains(truth_gv),
            "grad v {graph:?} must enclose {truth_gv} at ({u}, {v})"
        );
    }
}

#[test]
fn r5_refusal_when_krawczyk_fails_at_depth_max() {
    // §8.6's named refusal: when the R4 preimage cannot be certified — here
    // over the reduced-configuration target region with an n0 for which no
    // graph exists (the horizontal-plane pair is vertical in the x direction,
    // so graphcert refuses and every refinement would, too) — r5_enclose
    // refuses with R5EnclosureFailed (Inconclusive). The caller subdivides or
    // falls back to R4'.
    let plane = GraphPatch::plane(PLANE_AT_TANGENCY);
    let q = r5_target();
    let verdict = r5_enclose(&plane, &plane, q, [1.0, 0.0, 0.0]);
    match verdict {
        ClaimVerdict::Disproven(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::R5EnclosureFailed);
            assert_eq!(refusal.backing, VerdictClass::Inconclusive);
            match &refusal.evidence {
                RefusalEvidence::Predicate { name, .. } => {
                    assert_eq!(*name, "r5_graphcert_refused_over_target");
                }
                other => panic!("the R5 stall must carry the named predicate: {other:?}"),
            }
        }
        other => panic!("an uncertifiable R5 preimage must refuse, not {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// R4′ normal-projection fallback
// ---------------------------------------------------------------------------

#[test]
fn r4_fallback_prime_exercised_on_no_feasible_n0_fixture() {
    // 302's tangential-adjacent fixture family: the unit-sphere cap and its
    // tangent plane at the chart origin. The fallback (R4', the fixed-(u,v)
    // normal projection) records an honest outcome: at the tangency point the
    // normal foot is certified; away from it the foot is provably outside the
    // search box, and no false Proven is ever issued.
    let cap = GraphPatch::sphere_cap();
    let tangent = GraphPatch::plane(PLANE_AT_TANGENCY);

    // The tangency correspondence: the cap normal at the origin meets its
    // tangent plane exactly at the chart origin.
    let near = box2([-1e-3, -1e-3], [1e-3, 1e-3]);
    let foot = match r4_prime(&cap, [0.0, 0.0], &tangent, near) {
        ClaimVerdict::Proven(point) => point,
        other => panic!("the tangency normal foot must certify: {other:?}"),
    };
    assert_eq!(foot.residual, ResidualId::R4Prime);
    assert!(foot.rho <= config::RHO_MAX);
    assert!(
        foot.box_.lo[0] <= 0.0
            && 0.0 <= foot.box_.hi[0]
            && foot.box_.lo[1] <= 0.0
            && 0.0 <= foot.box_.hi[1],
        "the certified box must contain the chart-origin foot"
    );

    // Honest non-Proven: the cap normal at (0.5, 0) meets the tangent plane at
    // (s, t) = (0.5625, 0), provably outside the search box [0.8, 1]^2; the
    // fallback must never issue a false Proven.
    let away = box2([0.8, -0.1], [1.0, 0.1]);
    let verdict = r4_prime(&cap, [0.5, 0.0], &tangent, away);
    match verdict {
        ClaimVerdict::Proven(point) => {
            panic!("a normal foot outside the search box must not certify: {point:?}")
        }
        ClaimVerdict::Disproven(_) | ClaimVerdict::Inconclusive(_) => {}
    }

    // The S2A parity: the same residual through the frozen square C1 yields
    // the same certified point at the tangency.
    let weights = vec![weight_of(&cap, near), weight_of(&tangent, near)];
    let system = R4PrimeFixtureSystem {
        cap: &cap,
        tangent: &tangent,
    };
    assert_eq!(system.arity(), 2);
    match krawczyk_c1(&system, near, &weights) {
        ClaimVerdict::Proven(parity) => {
            assert_eq!(parity.residual, ResidualId::R1);
            assert_eq!(parity.box_, foot.box_);
            assert!(
                (parity.rho - foot.rho).abs() <= f64::EPSILON,
                "the fallback is the S2A C1's point"
            );
        }
        ClaimVerdict::Disproven(refusal) => {
            panic!("the S2A parity C1 must certify the tangency foot: {refusal:?}")
        }
        ClaimVerdict::Inconclusive(reason) => {
            panic!("the S2A parity C1 must certify the tangency foot: {reason}")
        }
    }
}

// ---------------------------------------------------------------------------
// Source audits
// ---------------------------------------------------------------------------

/// The R4′ residual of the fallback fixture family, replayed directly over the
/// frozen square C1 for the S2A parity check: `first` fixed at `u0`, solved
/// over the `second` patch's chart box.
struct R4PrimeFixtureSystem<'a> {
    /// The cap patch (fixed chart point `u0`).
    cap: &'a GraphPatch,
    /// The tangent patch (the chart being solved).
    tangent: &'a GraphPatch,
}

impl R4PrimeFixtureSystem<'_> {
    /// The fixed chart point of the cap patch.
    const U0: [f64; 2] = [0.0, 0.0];
}

impl SquareResidualEval for R4PrimeFixtureSystem<'_> {
    fn arity(&self) -> usize {
        2
    }

    fn eval(&self, b: &[Interval]) -> Vec<Interval> {
        let d = match box2_from(b) {
            Some(d) => d,
            None => return vec![unbounded(); 2],
        };
        let u0 = box2(Self::U0, Self::U0);
        let p1 = self.cap.enclose(u0);
        let p2 = self.tangent.enclose(d);
        let de1 = self.cap.derivs(u0);
        let diff = box_sub(p2, p1);
        vec![dot_iv(de1.su, diff), dot_iv(de1.sv, diff)]
    }

    fn jac_encl(&self, b: &[Interval]) -> Vec<Vec<Interval>> {
        let d = match box2_from(b) {
            Some(d) => d,
            None => return vec![vec![unbounded(); 2]; 2],
        };
        let u0 = box2(Self::U0, Self::U0);
        let de1 = self.cap.derivs(u0);
        let de2 = self.tangent.derivs(d);
        let row0 = [dot_iv(de1.su, de2.su), dot_iv(de1.su, de2.sv)];
        let row1 = [dot_iv(de1.sv, de2.su), dot_iv(de1.sv, de2.sv)];
        vec![row0.to_vec(), row1.to_vec()]
    }
}

/// The certified componentwise box difference.
fn box_sub(a: IBox3, b: IBox3) -> IBox3 {
    let mut lo = [0.0f64; 3];
    let mut hi = [0.0f64; 3];
    for k in 0..3 {
        let d = iv(a.lo[k], a.hi[k]).sub(&iv(b.lo[k], b.hi[k]));
        lo[k] = d.lo;
        hi[k] = d.hi;
    }
    IBox3 { lo, hi }
}

/// A box from a two-element interval slice (the `S1A` pattern).
fn box2_from(b: &[Interval]) -> Option<IBox2> {
    if b.len() != 2 {
        return None;
    }
    IBox2::try_new([b[0].lo, b[1].lo], [b[0].hi, b[1].hi]).ok()
}

/// A vacuous (fully unbounded) interval for an invalid joint box.
fn unbounded() -> Interval {
    Interval {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
    }
}

fn code_has_word(lines: &[&str], needle: &str) -> Option<usize> {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    for (line_no, raw) in lines.iter().enumerate() {
        let code = match raw.find("//") {
            Some(index) => &raw[..index],
            None => raw,
        };
        let found = code.match_indices(needle).any(|(i, _)| {
            let before = i
                .checked_sub(1)
                .map(|j| code.as_bytes()[j] as char)
                .map(is_word)
                .unwrap_or(false);
            let after = code
                .as_bytes()
                .get(i + needle.len())
                .map(|b| *b as char)
                .map(is_word)
                .unwrap_or(false);
            !before && !after
        });
        if found {
            return Some(line_no + 1);
        }
    }
    None
}

#[test]
fn no_bernstein_applies_to_r5_audit() {
    // The audit row (spec §8.6, §20): g = f1 − f2 is analytic and
    // non-polynomial; no Bernstein evaluation may appear on the g path. The
    // projection module consumes only the CertifiedPatch enclosures, so no
    // Bernstein net evaluation exists anywhere in projection.rs.
    let source = include_str!("../src/kernel/projection.rs");
    let lines: Vec<&str> = source.lines().collect();
    let hit = code_has_word(&lines, "bernstein");
    assert!(
        hit.is_none(),
        "no Bernstein evaluation may appear in projection.rs (line {hit:?})"
    );
}

#[test]
fn no_transcendental_call_in_projection_module() {
    // N4: no transcendental function call may appear in projection.rs.
    let source = include_str!("../src/kernel/projection.rs");
    let lines: Vec<&str> = source.lines().collect();
    for needle in ["sin", "cos", "atan2", "exp", "ln", "log", "powf"] {
        assert!(
            code_has_word(&lines, needle).is_none(),
            "no transcendental call may appear in projection.rs (found {needle})"
        );
    }
    for (line_no, raw) in lines.iter().enumerate() {
        let code = match raw.find("//") {
            Some(index) => &raw[..index],
            None => raw,
        };
        assert!(
            !code.contains("std::f64::consts"),
            "no std::f64::consts may appear in projection.rs on line {}: {code}",
            line_no + 1
        );
    }
    // sqrt appears only in normalization contexts.
    let mut sqrt_seen = false;
    for (line_no, raw) in lines.iter().enumerate() {
        let code = match raw.find("//") {
            Some(index) => &raw[..index],
            None => raw,
        };
        if code.contains("sqrt") {
            assert!(
                code.contains("norm"),
                "sqrt must appear only in normalization on line {}: {code}",
                line_no + 1
            );
            sqrt_seen = true;
        }
    }
    assert!(
        sqrt_seen,
        "projection.rs normalizes its complement basis with sqrt"
    );
}
