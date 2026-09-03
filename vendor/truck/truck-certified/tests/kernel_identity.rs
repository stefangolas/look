//! BG-KV2-103-IDENTITY integration tests: node identity Rules A/B/C and the
//! dyadic sampling join (spec §4.2/§4.3). No solver is implemented or invoked
//! here — every assertion is box containment, the typed implication relation,
//! exact f64 equality, or integer address arithmetic.

#![deny(clippy::unwrap_used)]

use std::collections::BTreeSet;

use truck_certified::kernel::certs::PointCert;
use truck_certified::kernel::evidence::{Refusal as KernelRefusal, RefusalKind, VerdictClass};
use truck_certified::kernel::identity::{
    join, refuse_custom_on_shared, rule_a, rule_b_transport, rule_c, sample_parameters,
    DyadicRequest, EdgeSampleSet, IdentityRule, IdentityVerdict, SamplingFlag,
};
use truck_certified::kernel::patch::IBox2;
use truck_certified::kernel::residual::ResidualId;

/// The contraction rate carried by every fixture certificate (<= RHO_MAX).
const RHO: f64 = 0.125;

/// Extract the `Ok` of a fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T>(result: Result<T, KernelRefusal>) -> T {
    match result {
        Ok(value) => value,
        Err(refusal) => panic!("a construction that must succeed was refused: {refusal:?}"),
    }
}

/// Extract the refusal kind of a construction that must refuse.
fn refusal_kind<T>(result: Result<T, KernelRefusal>) -> RefusalKind {
    match result {
        Ok(_) => panic!("expected a refusal, got an accepted construction"),
        Err(refusal) => refusal.kind,
    }
}

/// Exact `f64` equality, compared by bit pattern (the ground truths assert
/// exact representability, so an integer comparison is the right check).
fn bit_eq(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
}

/// A parameter box `[u_lo, u_hi] x [v_lo, v_hi]`.
fn box2(u_lo: f64, u_hi: f64, v_lo: f64, v_hi: f64) -> IBox2 {
    construct(IBox2::try_new([u_lo, v_lo], [u_hi, v_hi]))
}

/// A certificate at the given residual over a uniform-square box `[lo, hi]^2`.
fn cert(residual: ResidualId, lo: f64, hi: f64) -> PointCert {
    construct(PointCert::try_new(residual, box2(lo, hi, lo, hi), RHO))
}

/// A dyadic request on `[0, 1]`.
fn request(depth: u32, leaves: &[u64]) -> DyadicRequest {
    DyadicRequest {
        a: 0.0,
        b: 1.0,
        depth,
        leaves: leaves.iter().copied().collect(),
    }
}

/// Read an edge sample set back as a request so it can join again.
fn as_request(s: &EdgeSampleSet) -> DyadicRequest {
    DyadicRequest {
        a: s.a,
        b: s.b,
        depth: s.depth,
        leaves: s.nodes.clone(),
    }
}

/// Assert two sample sets are the same interval, the same depth, the same node
/// set, and produce the same parameter list in the same order.
fn assert_samples_same(s1: &EdgeSampleSet, s2: &EdgeSampleSet) {
    assert!(bit_eq(s1.a, s2.a), "interval lower ends differ");
    assert!(bit_eq(s1.b, s2.b), "interval upper ends differ");
    assert_eq!(s1.depth, s2.depth, "join depths differ");
    assert_eq!(s1.nodes, s2.nodes, "joined node sets differ");
    let p1 = sample_parameters(s1);
    let p2 = sample_parameters(s2);
    assert_eq!(p1.len(), p2.len(), "parameter list lengths differ");
    for (x, y) in p1.iter().zip(p2.iter()) {
        assert!(bit_eq(*x, *y), "parameter lists differ at {x:?} vs {y:?}");
    }
}

#[test]
fn rule_a_identifies_same_residual_unique_root_in_union() {
    let a = cert(ResidualId::R1, 0.4, 0.6);
    let b = cert(ResidualId::R1, 0.5, 0.7);
    let union_cert = cert(ResidualId::R1, 0.35, 0.75);
    assert!(
        matches!(
            rule_a(&a, &b, &union_cert),
            IdentityVerdict::CertifiedEqual {
                rule: IdentityRule::RuleA
            }
        ),
        "equal residuals with a containing union hull must certify"
    );
    let tight = cert(ResidualId::R1, 0.4, 0.7);
    assert!(
        matches!(
            rule_a(&a, &b, &tight),
            IdentityVerdict::CertifiedEqual {
                rule: IdentityRule::RuleA
            }
        ),
        "a union hull that exactly equals the union cert box still certifies"
    );
    assert!(
        matches!(
            rule_a(&a, &a, &union_cert),
            IdentityVerdict::CertifiedEqual {
                rule: IdentityRule::RuleA
            }
        ),
        "identical neighborhoods certify through their own union hull"
    );
}

#[test]
fn rule_a_refuses_different_residuals_and_noncontained_unions() {
    let a = cert(ResidualId::R1, 0.4, 0.6);
    let b = cert(ResidualId::R1, 0.5, 0.7);
    assert_eq!(
        rule_a(&a, &b, &cert(ResidualId::R2, 0.35, 0.75)),
        IdentityVerdict::NotCertified,
        "a union certificate on a different residual must refuse"
    );
    assert_eq!(
        rule_a(
            &a,
            &cert(ResidualId::R2, 0.5, 0.7),
            &cert(ResidualId::R1, 0.35, 0.75)
        ),
        IdentityVerdict::NotCertified,
        "differing source residuals must refuse"
    );
    assert_eq!(
        rule_a(&a, &b, &cert(ResidualId::R1, 0.5, 0.75)),
        IdentityVerdict::NotCertified,
        "a union cert box that excludes part of the hull must refuse"
    );
    assert_eq!(
        rule_a(&a, &b, &cert(ResidualId::R1, 0.35, 0.65)),
        IdentityVerdict::NotCertified,
        "a union cert box not containing the hull must refuse"
    );
}

#[test]
fn rule_b_transports_deck_translations_exactly() {
    let src = cert(ResidualId::R1, 0.2, 0.3);
    let deck_one = construct(rule_b_transport(&src, (1, 0), (1.0, 1.0), None));
    assert!(bit_eq(deck_one.box_.lo[0], 1.2), "u lo must be 1.2 exactly");
    assert!(bit_eq(deck_one.box_.hi[0], 1.3), "u hi must be 1.3 exactly");
    assert!(bit_eq(deck_one.box_.lo[1], 0.2), "v lo is untouched");
    assert!(bit_eq(deck_one.box_.hi[1], 0.3), "v hi is untouched");
    assert_eq!(deck_one.residual, ResidualId::R1);
    assert!(bit_eq(deck_one.rho, RHO), "rho is transported unchanged");

    let half = cert(ResidualId::R1, 0.25, 0.5);
    let deck_two = construct(rule_b_transport(&half, (2, 0), (0.5, 0.5), None));
    assert!(
        bit_eq(deck_two.box_.lo[0], 1.25),
        "0.25 + 2*0.5 must be 1.25 exactly"
    );
    assert!(
        bit_eq(deck_two.box_.hi[0], 1.5),
        "0.5 + 1.0 must be 1.5 exactly"
    );
    assert!(bit_eq(deck_two.box_.lo[1], 0.25));
    assert!(bit_eq(deck_two.box_.hi[1], 0.5));
}

#[test]
fn rule_b_transports_affine_reparams_with_outward_rounding() {
    let src = cert(ResidualId::R1, 0.2, 0.3);
    let scale_u = [[2.0, 0.0], [0.0, 1.0]];
    let t = construct(rule_b_transport(&src, (0, 0), (1.0, 1.0), Some(scale_u)));

    let exact_lo_u = 2.0 * 0.2;
    let exact_hi_u = 2.0 * 0.3;
    assert!(
        t.box_.lo[0] <= exact_lo_u,
        "u lo must not exceed the exact extent"
    );
    assert!(
        t.box_.hi[0] >= exact_hi_u,
        "u hi must not fall short of the exact extent"
    );
    assert!(
        bit_eq(t.box_.lo[0], exact_lo_u.next_down()),
        "u lo is one ULP outward"
    );
    assert!(
        bit_eq(t.box_.hi[0], exact_hi_u.next_up()),
        "u hi is one ULP outward"
    );

    let exact_lo_v = 1.0 * 0.2;
    let exact_hi_v = 1.0 * 0.3;
    assert!(
        t.box_.lo[1] <= exact_lo_v,
        "v lo must not exceed the exact extent"
    );
    assert!(
        t.box_.hi[1] >= exact_hi_v,
        "v hi must not fall short of the exact extent"
    );
    assert!(
        bit_eq(t.box_.lo[1], exact_lo_v.next_down()),
        "v lo is one ULP outward"
    );
    assert!(
        bit_eq(t.box_.hi[1], exact_hi_v.next_up()),
        "v hi is one ULP outward"
    );

    assert!(
        t.box_.lo[0] <= 0.4 && 0.6 <= t.box_.hi[0],
        "scaled u points are contained"
    );
    assert!(
        t.box_.lo[1] <= 0.2 && 0.3 <= t.box_.hi[1],
        "scaled v points are contained"
    );
    assert_eq!(t.residual, ResidualId::R1);
    assert!(bit_eq(t.rho, RHO));
}

#[test]
fn rule_c_identifies_through_implication_only() {
    let strong = cert(ResidualId::R2, 0.4, 0.6);
    let weak = cert(ResidualId::R1, 0.5, 0.7);
    let unions = vec![
        (ResidualId::R1, cert(ResidualId::R1, 0.35, 0.75)),
        (ResidualId::R2, cert(ResidualId::R2, 0.2, 0.9)),
    ];
    assert!(
        matches!(
            rule_c(&strong, &weak, &unions),
            IdentityVerdict::CertifiedEqual {
                rule: IdentityRule::RuleC
            }
        ),
        "R2 must identify with R1 through the R2 ⊒ R1 implication"
    );

    let not_containing = vec![(ResidualId::R1, cert(ResidualId::R1, 0.45, 0.75))];
    assert_eq!(
        rule_c(&strong, &weak, &not_containing),
        IdentityVerdict::NotCertified,
        "the implication alone is not enough; the R union hull must contain"
    );

    let terminal = vec![
        (ResidualId::R1, cert(ResidualId::R1, 0.35, 0.75)),
        (ResidualId::R2, cert(ResidualId::R2, 0.35, 0.75)),
        (ResidualId::R8, cert(ResidualId::R8, 0.35, 0.75)),
    ];
    for top in [ResidualId::R7, ResidualId::R8, ResidualId::R9] {
        let top_cert = cert(top, 0.4, 0.6);
        assert_eq!(
            rule_c(&top_cert, &weak, &terminal),
            IdentityVerdict::NotCertified,
            "{top:?} certifies nothing below it (the §4.2 table)"
        );
    }
}

#[test]
fn identity_never_uses_distance_or_tolerance() {
    const SRC: &str = include_str!("../src/kernel/identity.rs");
    let forbidden = [
        "dist",
        "distance",
        "hypot",
        "abs",
        "sqrt",
        "tol",
        "eps",
        "epsilon",
        "tolerance",
        "1e",
    ];
    let mut violations = Vec::new();
    for (index, raw) in SRC.lines().enumerate() {
        let line = match raw.find("//") {
            Some(split) => &raw[..split],
            None => raw,
        };
        let code = line.to_lowercase();
        for token in forbidden {
            if code.contains(token) {
                violations.push(format!("line {} (`{token}`): {}", index + 1, raw.trim()));
                break;
            }
        }
    }
    assert!(
        violations.is_empty(),
        "kernel/identity.rs must decide identity by exact containment, never by \
         distance or tolerance; found:\n{}",
        violations.join("\n")
    );
}

#[test]
fn dyadic_join_is_associative_commutative_idempotent() {
    let a = request(1, &[0]);
    let b = request(2, &[1, 2]);
    let c = request(3, &[0, 5, 7]);

    let single = construct(join(a.clone(), &[]));
    let idem = construct(join(a.clone(), &[a.clone()]));
    assert_samples_same(&single, &idem);
    assert_eq!(
        idem.nodes,
        BTreeSet::from([0]),
        "the depth-1 leaf 0 joins with itself unchanged"
    );

    let left = construct(join(a.clone(), &[b.clone(), c.clone()]));
    let right = construct(join(b.clone(), &[a.clone(), c.clone()]));
    let third = construct(join(c.clone(), &[a.clone(), b.clone()]));
    assert_samples_same(&left, &right);
    assert_samples_same(&left, &third);

    let ab = construct(join(a.clone(), &[b.clone()]));
    let assoc_left = construct(join(as_request(&ab), &[c.clone()]));
    let bc = construct(join(b.clone(), &[c.clone()]));
    let assoc_right = construct(join(a.clone(), &[as_request(&bc)]));
    assert_samples_same(&assoc_left, &assoc_right);

    let expected = construct(join(a.clone(), &[b.clone(), c.clone()]));
    assert_eq!(
        expected.nodes,
        BTreeSet::from([0, 1, 2, 3, 4, 5, 7]),
        "the depth-3 union of the corpus"
    );
}

#[test]
fn dyadic_join_is_order_independent_under_randomized_gather() {
    let corpus = vec![
        request(1, &[0]),
        request(2, &[1, 2]),
        request(3, &[0, 5, 7]),
        request(2, &[0, 3]),
        request(3, &[2, 6]),
        request(1, &[1]),
    ];
    let expected = construct(join(corpus[0].clone(), &corpus[1..]));

    let mut state: u64 = 0x243F_6A88_85A3_08D3;
    let mut order: Vec<usize> = (0..corpus.len()).collect();
    for _ in 0..40 {
        shuffle(&mut order, &mut state);
        let base = corpus[order[0]].clone();
        let others: Vec<DyadicRequest> = order[1..].iter().map(|&i| corpus[i].clone()).collect();
        let gathered = construct(join(base, &others));
        assert_samples_same(&gathered, &expected);
    }
}

#[test]
fn nondyadic_shared_request_refuses() {
    assert_eq!(
        refusal_kind(refuse_custom_on_shared(2, SamplingFlag::Custom)),
        RefusalKind::NonDyadicSharedRequest,
        "a custom request on a shared edge must refuse NonDyadicSharedRequest"
    );
    let refusal = match refuse_custom_on_shared(2, SamplingFlag::Custom) {
        Err(r) => r,
        Ok(()) => panic!("the shared custom request must refuse"),
    };
    assert_eq!(
        refusal.backing,
        VerdictClass::Disproven,
        "NonDyadicSharedRequest backs Disproven (§17)"
    );
    assert!(matches!(
        refuse_custom_on_shared(2, SamplingFlag::Dyadic),
        Ok(())
    ));
    assert!(matches!(
        refuse_custom_on_shared(1, SamplingFlag::Custom),
        Ok(())
    ));
    assert!(matches!(
        refuse_custom_on_shared(1, SamplingFlag::Dyadic),
        Ok(())
    ));
}

/// One step of the fixed-seed LCG used to randomize gather orders.
fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// Fisher-Yates shuffle over a fixed-seed LCG (deterministic, recorded seed).
fn shuffle(order: &mut [usize], state: &mut u64) {
    for i in (1..order.len()).rev() {
        let j = (lcg(state) % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
}
