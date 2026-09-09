//! ADM-002-CERTIFICATES conformance: the certificate assembly over the landed
//! lemma kernels (L1 substrate, L3 normal numerator / hemisphere / cone, L4
//! deflation / seam). The three named tests are the contract:
//!
//! - `four_outcome_dispatch_exhaustive_on_fixtures` — the four-outcome
//!   dispatch (regular | regular+seam | regular-interior+collapsed-boundary |
//!   genuine singularity → typed refusal) is EXHAUSTIVE over the admission
//!   fixture kit: every fixture resolves to exactly one of the four verdicts,
//!   and the kit exercises all four classes — a fifth outcome is a SPEC_GAP.
//! - `certificates_compose_with_fssi001_gate` — the normal-cone subsystem (L3)
//!   feeds the FSSI-001 transversality gate: the same cone data this packet
//!   supplies certifies the Theorem C [`TransversePair`] AND drives the landed
//!   gate (`gate_admit`) to a `TangencyFree` verdict on the same box pair (the
//!   gate consumes the normal enclosures this cone data certifies; no gate
//!   edit — FSSI-001's landed code).
//! - `subdivision_stall_refuses_typed_not_silent` — certificate failures
//!   subdivide under budget; budget exhaustion is a typed refusal with the
//!   recorded stall, never silent.
//!
//! Fixtures: the shared ADM-SHIM kit (`tests/patch_fixtures.rs`) plus the local
//! genuine-singularity pinch patch (the fourth-outcome fixture). No solver is
//! called; every ground truth is a machine-checked certificate property.

#![deny(clippy::unwrap_used)]

mod patch_fixtures;

use patch_fixtures::unit_square_domain;
use truck_certified::construct::admission::{
    certify_patch_family, certify_regular_pair_transverse, certify_transverse_pair,
    AdmissionCertificate, CertificateBudget, RegularPatch, StallRecord,
};
use truck_certified::construct::normal_cone::{assemble_normal_numerator, normal_cone};
use truck_certified::construct::patches::{PatchParent, TensorBernsteinPatch};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::formal::exact::CertifiedInterval;
use truck_certified::hull::{hull_bernstein_2d, HullRefusal};
use truck_certified::ssi::ssi_gate::{gate_admit, GateAdmission, GateParams, NormalBox, SideBox};
use truck_certified::ssi::SsiRefusal;

/// The admission subdivision budget of the fixture battery (deep enough for the
/// decidable fixtures, capped for the stall fixture).
fn fixture_budget() -> CertificateBudget {
    CertificateBudget::default()
}

/// Unpack a fallible fixture construction, panicking with a message (never an
/// `unwrap`).
fn expect<T>(result: Result<T, &'static str>, what: &str) -> T {
    match result {
        Ok(value) => value,
        Err(message) => panic!("{what}: {message}"),
    }
}

/// The genuine-parameter-singularity fixture: `X(u, v) = (u², v², u·v)`, all
/// weights `1`. The normal numerator vanishes at the `(u, v) = (0, 0)` corner
/// (`X(0, 0)` is a pinch, not an intentional collapsed edge), so no dyadic
/// hemisphere direction ever certifies a box containing the corner and the
/// subdivision budget exhausts — the fourth (typed-refusal) outcome.
fn singular_pinch_patch() -> TensorBernsteinPatch {
    let numerator: Vec<Vec<[f64; 3]>> = vec![
        vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.25], [0.0, 1.0, 0.5]],
        vec![[1.0, 0.0, 0.0], [1.0, 0.0, 0.5], [1.0, 1.0, 1.0]],
    ];
    let weights: Vec<Vec<f64>> = vec![vec![1.0; 3]; 3];
    let parent = PatchParent::new(0, None);
    match TensorBernsteinPatch::try_new(numerator, weights, unit_square_domain(), parent) {
        Ok(patch) => patch,
        Err(refusal) => panic!("the singular pinch fixture refused construction: {refusal:?}"),
    }
}

/// The plane `X(s, t) = (s, t, −s − t)` (a constant `(1, 1, 1)`-normal
/// fixture) for the transversality composition.
fn plane_patch() -> TensorBernsteinPatch {
    let numerator: Vec<Vec<[f64; 3]>> = vec![
        vec![[0.0, 0.0, 0.0], [0.0, 1.0, -1.0]],
        vec![[1.0, 0.0, -1.0], [1.0, 1.0, -2.0]],
    ];
    let weights: Vec<Vec<f64>> = vec![vec![1.0, 1.0], vec![1.0, 1.0]];
    match TensorBernsteinPatch::try_new(
        numerator,
        weights,
        unit_square_domain(),
        PatchParent::new(0, None),
    ) {
        Ok(patch) => patch,
        Err(refusal) => panic!("the plane fixture refused construction: {refusal:?}"),
    }
}

/// A compact outcome tag of a family certificate.
fn tag(cert: &AdmissionCertificate) -> &'static str {
    cert.tag()
}

/// The outcome expected of each fixture of the admission kit.
fn expect_span_count(cert: &AdmissionCertificate, what: &str) {
    match cert {
        AdmissionCertificate::Regular { spans } => {
            assert_eq!(spans.len(), 1, "{what}: one whole-span certificate");
            assert!(
                spans.iter().all(|s| s.collapsed.is_none()),
                "{what}: a regular family carries no deflation record"
            );
        }
        _ => panic!("{what}: expected the regular outcome, got {}", tag(cert)),
    }
}

#[test]
fn four_outcome_dispatch_exhaustive_on_fixtures() {
    let budget = fixture_budget();

    // Outcome 1 — regular (hemisphere pass over the whole span): the bilinear
    // graph X(u, v) = (u, v, u·v).
    let bilinear = patch_fixtures::bilinear_patch();
    let cert = certify_patch_family(&[bilinear], &budget);
    expect_span_count(&cert, "bilinear");
    match &cert {
        AdmissionCertificate::Regular { spans } => {
            assert_eq!(spans[0].region, ((0.0, 1.0), (0.0, 1.0)));
            assert!(spans[0].regular.cone.s_up < 1.0);
        }
        _ => unreachable!("asserted by expect_span_count"),
    }

    // Outcome 3 — regular-interior + collapsed-boundary: the apex-collapse cone
    // (v = 1 collapses to the apex, recorded multiplicity 3).
    let cone = patch_fixtures::collapsed_edge_fixture();
    let cert = certify_patch_family(&[cone.patch], &budget);
    match &cert {
        AdmissionCertificate::RegularInteriorCollapsedBoundary { spans } => {
            assert_eq!(spans.len(), 1, "cone: one deflated-span certificate");
            let collapsed = spans[0]
                .collapsed
                .as_ref()
                .expect("the cone's collapse is deflated");
            assert_eq!(
                collapsed.multiplicity, cone.multiplicity,
                "the cone deflation records the divided (1−v)³ multiplicity"
            );
            assert!(spans[0].regular.cone.s_up < 1.0);
        }
        _ => panic!(
            "cone: expected the collapsed-boundary outcome, got {}",
            tag(&cert)
        ),
    }

    // Outcome 3 (second fixture) — the rational sphere octant (a loft apex
    // pole collapses along u = 1, deflated with multiplicity 1).
    let octant = patch_fixtures::rational_quadratic_volume_fixture();
    let cert = certify_patch_family(&[octant.patch], &budget);
    match &cert {
        AdmissionCertificate::RegularInteriorCollapsedBoundary { spans } => {
            assert_eq!(spans.len(), 1, "octant: one deflated-span certificate");
            let collapsed = spans[0]
                .collapsed
                .as_ref()
                .expect("the octant's pole is deflated");
            assert_eq!(
                collapsed.multiplicity, 1,
                "the octant pole collapses with multiplicity one"
            );
            assert!(spans[0].regular.cone.s_up < 1.0);
        }
        _ => panic!(
            "octant: expected the collapsed-boundary outcome, got {}",
            tag(&cert)
        ),
    }

    // Outcome 2 — regular + seam: the up/down sphere-octant pair shares the
    // exact base quarter-arc, so the family closes a SeamIdentified pair.
    let seam = patch_fixtures::seam_pair_fixture();
    let cert = certify_patch_family(&[seam.a, seam.b], &budget);
    match &cert {
        AdmissionCertificate::RegularWithSeam {
            spans,
            seam: identified,
        } => {
            assert_eq!(spans.len(), 2, "seam pair: two certified octant spans");
            assert_eq!(
                identified.paired, seam.paired,
                "the paired BRep edges close"
            );
        }
        _ => panic!(
            "seam pair: expected the regular+seam outcome, got {}",
            tag(&cert)
        ),
    }

    // Outcome 4 — genuine parameter singularity → typed refusal: the pinch.
    let pinch = singular_pinch_patch();
    let cert = certify_patch_family(&[pinch], &budget);
    match &cert {
        AdmissionCertificate::GenuineSingularity { stall } => {
            assert_eq!(
                stall.cause,
                ConstructRefusal::ConditioningBelowThreshold,
                "the pinch refuses typed, never silently"
            );
        }
        _ => panic!(
            "pinch: expected the genuine-singularity outcome, got {}",
            tag(&cert)
        ),
    }

    // The kit is EXHAUSTIVE over the four-outcome space: every fixture maps to
    // one of the four classes, and all four are exercised — a fixture that
    // produced a fifth outcome (or missed a class) is a SPEC_GAP.
    let observed: Vec<&str> = [
        tag(&certify_patch_family(
            &[patch_fixtures::bilinear_patch()],
            &budget,
        )),
        tag(&certify_patch_family(
            &[patch_fixtures::collapsed_edge_fixture().patch],
            &budget,
        )),
        tag(&certify_patch_family(
            &[patch_fixtures::rational_quadratic_volume_fixture().patch],
            &budget,
        )),
        tag(&certify_patch_family(
            &[
                patch_fixtures::seam_pair_fixture().a,
                patch_fixtures::seam_pair_fixture().b,
            ],
            &budget,
        )),
        tag(&certify_patch_family(&[singular_pinch_patch()], &budget)),
    ]
    .into_iter()
    .collect();
    for outcome in &observed {
        assert!(
            matches!(
                *outcome,
                "regular"
                    | "regular_with_seam"
                    | "regular_interior_collapsed_boundary"
                    | "genuine_singularity"
            ),
            "a fixture escaped the four-outcome space: {outcome}"
        );
    }
    let distinct: std::collections::BTreeSet<&&str> = observed.iter().collect();
    assert_eq!(
        distinct.len(),
        4,
        "the kit exercises all four outcome classes, observed {observed:?}"
    );
}

#[test]
fn certificates_compose_with_fssi001_gate() {
    // The normal-cone data the admission supplies: the bilinear graph's cone
    // over the compact box [0.48, 0.52]² and the plane X = (s, t, −s − t)'s
    // cone (normals near (1, 1, 1)/√3) over the same box. The two direction
    // sets separate widely (near-orthogonal anchors), so Theorem C certifies
    // transversality on the product box.
    let bilinear = patch_fixtures::bilinear_patch();
    let m_bilinear = expect(
        assemble_normal_numerator(&bilinear).map_err(|_| "the bilinear patch assembles"),
        "bilinear numerator",
    );
    let box_side: ((f64, f64), (f64, f64)) = ((0.48, 0.52), (0.48, 0.52));
    let cone_bilinear = normal_cone(&m_bilinear, box_side)
        .expect("the bilinear cone certifies over the compact box");

    let plane = plane_patch();
    let m_plane = expect(
        assemble_normal_numerator(&plane).map_err(|_| "the plane patch assembles"),
        "plane numerator",
    );
    let cone_plane =
        normal_cone(&m_plane, box_side).expect("the plane cone certifies over the compact box");

    // Theorem C from the two cones (this packet's transverse certificate).
    let transverse = certify_transverse_pair(&cone_bilinear, &cone_plane)
        .expect("the separated cones certify the Theorem C transverse pair");
    assert!(
        transverse.delta.0 > 0.0 && transverse.delta.0 < transverse.delta.1,
        "the transverse margin is a strictly positive certified enclosure: {:?}",
        transverse.delta
    );

    // The same cones carried by admitted regular spans compose identically.
    let regular_a = RegularPatch {
        cone: cone_bilinear,
    };
    let regular_b = RegularPatch { cone: cone_plane };
    let via_regulars = certify_regular_pair_transverse(&regular_a, &regular_b)
        .expect("the regular-patch cones certify the transverse pair");
    assert_eq!(via_regulars.delta, transverse.delta);

    // FSSI-001 substrate handoff: the LANDED gate consumes the normal
    // enclosures this cone data certifies. The per-axis Bernstein hulls of the
    // two numerators over the certified boxes are exactly the gate's per-side
    // normal boxes, so the gate must reach TangencyFree on the same box pair.
    let grids_bilinear = axis_grids(&m_bilinear);
    let grids_plane = axis_grids(&m_plane);
    let side_bilinear = |_: SideBox| normal_box_of(&grids_bilinear, box_side);
    let side_plane = |_: SideBox| normal_box_of(&grids_plane, box_side);
    let params = GateParams {
        max_cells: 20_000,
        max_level: 12,
    };
    let side: SideBox = [(0.48, 0.52), (0.48, 0.52)];
    match gate_admit(side_bilinear, side_plane, side, side, &params) {
        Ok(GateAdmission::TangencyFree { margin }) => {
            assert!(
                margin.0 > 0.0,
                "the landed gate certifies a strictly positive margin: {margin:?}"
            );
        }
        Ok(other) => panic!(
            "the landed gate must admit the separated box pair, got {}",
            other.tag()
        ),
        Err(refusal) => panic!(
            "the landed gate refused the separated box pair: {}",
            refusal.tag()
        ),
    }
}

#[test]
fn subdivision_stall_refuses_typed_not_silent() {
    // The pinch is a genuine parameter singularity: the hemisphere pass fails,
    // no exact collapse deflates, and no dyadic cell containing the corner ever
    // certifies — so the certificate subdivides under budget and, at
    // exhaustion, refuses typed with the recorded stall. Never silent.
    let pinch = singular_pinch_patch();
    let budget = CertificateBudget {
        max_depth: 6,
        max_cells: 512,
    };
    let cert = certify_patch_family(&[pinch], &budget);
    let stall: &StallRecord = match &cert {
        AdmissionCertificate::GenuineSingularity { stall } => stall,
        _ => panic!(
            "the pinch must stall to the typed refusal, got {}",
            tag(&cert)
        ),
    };
    assert_eq!(
        stall.cause,
        ConstructRefusal::ConditioningBelowThreshold,
        "the stall is a typed refusal"
    );
    assert!(
        stall.depth_reached > 0 && stall.unresolved_cells > 0,
        "the stall records the subdivision reached (depth {}) and the unresolved \
         cells ({}) — never a silent drop",
        stall.depth_reached,
        stall.unresolved_cells
    );

    // A genuinely decidable family does NOT stall under the same budget.
    let bilinear = patch_fixtures::bilinear_patch();
    let cert = certify_patch_family(&[bilinear], &budget);
    assert!(
        matches!(cert, AdmissionCertificate::Regular { .. }),
        "a regular fixture resolves under the same budget: {}",
        tag(&cert)
    );
}

/// The per-axis coefficient grids of an assembled normal numerator (rows over
/// `u`, columns over `v`), in the layout `hull_bernstein_2d` consumes.
fn axis_grids(m: &truck_certified::construct::normal_cone::NormalNumerator) -> [Vec<Vec<f64>>; 3] {
    let (du, dv) = m.degree();
    let width = dv + 1;
    let mut grids: [Vec<Vec<f64>>; 3] = [
        vec![vec![0.0; width]; du + 1],
        vec![vec![0.0; width]; du + 1],
        vec![vec![0.0; width]; du + 1],
    ];
    for (row, coeff) in m.coeffs().iter().enumerate() {
        let i = row / width;
        let j = row % width;
        grids[0][i][j] = coeff[0];
        grids[1][i][j] = coeff[1];
        grids[2][i][j] = coeff[2];
    }
    grids
}

/// The certified normal enclosure of one side over its certified box, in the
/// gate's [`NormalBox`] shape.
fn normal_box_of(
    grids: &[Vec<Vec<f64>>; 3],
    box_: ((f64, f64), (f64, f64)),
) -> Result<NormalBox, SsiRefusal> {
    let mut out = [CertifiedInterval::point(0.0); 3];
    for (k, cell) in out.iter_mut().enumerate() {
        match hull_bernstein_2d(&grids[k], box_.0, box_.1) {
            Ok(hull) => *cell = hull,
            Err(HullRefusal::EnclosureUnavailable | HullRefusal::DomainNotCompact) => {
                return Err(SsiRefusal::InvalidInput)
            }
        }
    }
    Ok(out)
}
