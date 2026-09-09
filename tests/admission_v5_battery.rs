//! ADM-004-FUNNEL-WIRING root battery — the certified swept-pair cut and the
//! V5 identity battery over every landed green path reachable from the root
//! crate (the committed root test the program-end verification inherits).
//!
//! The three required tests:
//!
//! 1. **`admitted_pair_cut_certifies_end_to_end`** — the FIRST end-to-end
//!    certified cut between two spline-loft solids' faces: the certified
//!    pipeline (Theorem A admission ADM-001 → ADM-002 certificate assembly on
//!    both faces → Theorem C transversality over the shared whole-span product
//!    chart) certifies on a pair of shallow parabolic-profile ruled-section
//!    loft faces, and the deterministic cut row answers bit-identically on an
//!    identical re-run (determinism, scope decision 4).
//! 2. **`v5_battery_every_landed_green_path_bit_identical`** — the landed
//!    green paths this root battery can reach (canonical analytic loci,
//!    restricted sweeps, the admitted cut) each answer BIT-IDENTICALLY on two
//!    identical runs, and the twelve PB-011B-lifted F1 rows' evidence plus the
//!    PB-011C census records stay on `corpus/ttc/SKIPS.json` (the admission
//!    consult holds the V5 line — nothing already-green flips).
//! 3. **`non_admitted_carriers_refuse_unchanged`** — not-yet-admitted carrier
//!    forms (general spline-section pairs, non-positive-weight faces, the
//!    certificate carriers' refusing constructors) keep their exact typed
//!    refusals, identically on re-runs (admission widens monotonically or not
//!    at all).
//!
//! House rules: every fallible construction unpacks through explicit panic
//! arms; fixture nets are positive-weight clamped homogeneous
//! `BSplineSurface<Vector4>` faces over the unit square.

use std::path::PathBuf;

use truck_certified::construct::admission::{
    AdmitDomainHint, CertificateBudget, CollapsedBoundary, CullingOrder, EdgeId, NormalCone,
    RegularPatch, SeamIdentified, TransversePair, admit_tensor_spline_pair,
    certify_admitted_cut_pair,
};
use truck_certified::construct::bie::WitnessCell;
use truck_certified::construct::bie::fixtures::{plane_sphere_fixture, sweep_plane_fixture};
use truck_certified::construct::bie::ssi4::{
    RestrictedChart, Ssi4Parameters, certify_restricted_pair,
};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_evidence::{Budget, Certified};
use truck_geometry::prelude::{BSplineSurface, KnotVec, Vector4};

/// The domain hint of the Theorem A adapter signature (the frozen v1 culling
/// order).
fn domain_hint() -> AdmitDomainHint {
    AdmitDomainHint {
        order: CullingOrder::HullAabbThenKnotSpan,
    }
}

/// The default certificate subdivision budget of the admitted cut pipeline.
fn fixture_budget() -> CertificateBudget {
    CertificateBudget::default()
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A shallow parabolic-profile ruled-section loft face
/// `X(u, v) = (u, v, u + c·u²)` — bidegree `(2, 1)`, every extracted span
/// linear in the loft axis `v` (the ADM-001 admitted class shape), unit
/// weights. The normals `(−(1 + 2c·u), 0, 1)` lie in the `xz` plane near
/// `(−1, 0, 1)`.
fn shallow_loft_xz(c: f64) -> BSplineSurface<Vector4> {
    let knots = (
        KnotVec::from(vec![0.0f64, 0.0, 0.0, 1.0, 1.0, 1.0]),
        KnotVec::from(vec![0.0f64, 0.0, 1.0, 1.0]),
    );
    let u_coeff = [0.0, 0.5, 1.0];
    let profile = [0.0f64, 0.5, 1.0 + c];
    let v_coeff = [0.0, 1.0];
    let mut control_points: Vec<Vec<Vector4>> = Vec::new();
    for (&x, &z) in u_coeff.iter().zip(profile.iter()) {
        let mut row: Vec<Vector4> = Vec::new();
        for &v in &v_coeff {
            row.push(Vector4::new(x, v, z, 1.0));
        }
        control_points.push(row);
    }
    BSplineSurface::new(knots, control_points)
}

/// A shallow parabolic-profile ruled-section loft face
/// `Y(u, v) = (u, u + c·u², v)` — bidegree `(2, 1)`, every extracted span
/// linear in the loft axis `v` (the ADM-001 admitted class shape), unit
/// weights. The normals `(1 + 2c·u, −1, 0)` lie in the `xy` plane near
/// `(1, −1, 0)`.
fn shallow_loft_xy(c: f64) -> BSplineSurface<Vector4> {
    let knots = (
        KnotVec::from(vec![0.0f64, 0.0, 0.0, 1.0, 1.0, 1.0]),
        KnotVec::from(vec![0.0f64, 0.0, 1.0, 1.0]),
    );
    let u_coeff = [0.0, 0.5, 1.0];
    let profile = [0.0f64, 0.5, 1.0 + c];
    let v_coeff = [0.0, 1.0];
    let mut control_points: Vec<Vec<Vector4>> = Vec::new();
    for (&x, &y) in u_coeff.iter().zip(profile.iter()) {
        let mut row: Vec<Vector4> = Vec::new();
        for &v in &v_coeff {
            row.push(Vector4::new(x, y, v, 1.0));
        }
        control_points.push(row);
    }
    BSplineSurface::new(knots, control_points)
}

/// A biquadratic graph face `(u + offset_x, v, u·v)` (bidegree `(2, 2)`) — a
/// general spline-section carrier, NOT the admitted ruled-section class.
fn graph_face(offset_x: f64) -> BSplineSurface<Vector4> {
    let knots = (
        KnotVec::from(vec![0.0f64, 0.0, 0.0, 1.0, 1.0, 1.0]),
        KnotVec::from(vec![0.0f64, 0.0, 0.0, 1.0, 1.0, 1.0]),
    );
    let p = |x: f64, y: f64| Vector4::new(x + offset_x, y, x * y, 1.0);
    let control_points = vec![
        vec![p(0.0, 0.0), p(0.5, 0.0), p(1.0, 0.0)],
        vec![p(0.0, 0.5), p(0.5, 0.5), p(1.0, 0.5)],
        vec![p(0.0, 1.0), p(0.5, 1.0), p(1.0, 1.0)],
    ];
    BSplineSurface::new(knots, control_points)
}

/// A face whose weight field is NOT certifiably positive (a negative
/// homogeneous weight at a corner): extraction must refuse typed.
fn non_positive_weight_face() -> BSplineSurface<Vector4> {
    let knots = KnotVec::from(vec![0.0, 0.0, 1.0, 1.0]);
    let ctrl = vec![
        vec![
            Vector4::new(0.0, 0.0, 0.0, -0.5),
            Vector4::new(0.0, 1.0, 0.0, 1.0),
        ],
        vec![
            Vector4::new(1.0, 0.0, 0.0, 1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        ],
    ];
    BSplineSurface::new((knots.clone(), knots), ctrl)
}

// ---------------------------------------------------------------------------
// Required test 1: the first end-to-end certified cut between two spline-loft
// solids' faces
// ---------------------------------------------------------------------------

#[test]
fn admitted_pair_cut_certifies_end_to_end() {
    // Two spline-loft solids meet at a pair of shallow parabolic-profile
    // ruled-section loft faces: `X = (u, v, u + c·u²)` (the side of the loft
    // swept over the `(u, u + c·u²)` profile along `v`) and
    // `Y = (u, u + c·u², v)`. Both are ruled-section loft carriers (every
    // extracted span is `v`-degree 1), so the certified cut pipeline admits
    // them (ADM-001), certifies both faces regular on their whole spans
    // (ADM-002), and closes the pair with the Theorem C transversality
    // certificate — a strictly positive certified `‖n̂_X × n̂_Y‖` margin over
    // the shared whole-span product chart.
    let x = shallow_loft_xz(0.05);
    let y = shallow_loft_xy(0.05);

    let run = || match certify_admitted_cut_pair(&x, &y, &domain_hint(), &fixture_budget()) {
        Ok(row) => row,
        Err(refusal) => {
            panic!("the admitted pair cut must certify end to end, refused {refusal:?}")
        }
    };

    let first = run();
    assert_eq!(
        first.span_pair_count(),
        1,
        "the single-span fixture assembles one span pair"
    );
    assert_eq!(
        first.x_certificate.tag(),
        "regular",
        "the X loft face family certifies regular over its whole span"
    );
    assert_eq!(
        first.y_certificate.tag(),
        "regular",
        "the Y loft face family certifies regular over its whole span"
    );
    assert!(
        first.x_whole_box_regular().is_some() && first.y_whole_box_regular().is_some(),
        "both loft faces carry a whole-box regularity certificate"
    );
    assert!(
        first.transverse.delta.0 > 0.0 && first.transverse.delta.0 < first.transverse.delta.1,
        "the certified cut closes with a strictly positive transverse margin: {:?}",
        first.transverse.delta
    );

    // Determinism: an identical second cut is bit-identical (the binding's
    // determinism rule, extended to the admitted pair pipeline).
    let second = run();
    assert_eq!(
        format!("{first:?}"),
        format!("{second:?}"),
        "an identical admitted pair cut answers bit-identically"
    );
}

// ---------------------------------------------------------------------------
// Required test 2: the V5 identity battery across every landed green path the
// root crate can reach
// ---------------------------------------------------------------------------

/// The registered restricted sweep battery rows of the landed restricted 4-D
/// arm, exactly as the certified crate's own V5 battery drives them: plane ×
/// sphere and sweep × plane through `certify_restricted_pair` with default
/// parameters. Two identical runs must produce bit-identical certified
/// samples (V5: no verdict flips through the admitted pipeline).
fn restricted_sweep_verdicts() -> Vec<String> {
    let ps = plane_sphere_fixture();
    let ps_a = RestrictedChart::from_plane(ps.plane);
    let ps_b = RestrictedChart::from_sphere(ps.sphere);

    let sp = sweep_plane_fixture();
    let sp_a = match RestrictedChart::circular_sweep(
        sp.sweep.spine_from,
        sp.sweep.spine_to,
        sp.sweep.radius_start,
        sp.sweep.radius_end,
        sp.sweep.s0,
        sp.sweep.s1,
        sp.sweep.v0,
        sp.sweep.v1,
    ) {
        Some(chart) => chart,
        None => panic!("the fixture sweep spine is non-degenerate"),
    };
    let sp_b = RestrictedChart::from_plane(sp.plane);

    let battery: Vec<(&str, RestrictedChart, RestrictedChart, WitnessCell)> = vec![
        ("plane_x_sphere", ps_a, ps_b, ps.cell),
        ("sweep_x_plane", sp_a, sp_b, sp.cell),
    ];
    let params = Ssi4Parameters::default();
    let mut out = Vec::with_capacity(battery.len());
    for (name, a, b, cell) in battery {
        let mut budget = Budget::new(0, 0, 0);
        match certify_restricted_pair(a, b, cell, &params, &mut budget) {
            Ok(Certified { value, .. }) => out.push(format!("{name}: {value:?}")),
            Err(refusal) => {
                panic!("{name}: the landed green pair must certify, refused {refusal:?}")
            }
        }
    }
    out
}

/// The canonical analytic-locus row of the battery: the plane × sphere
/// canonical pair through the evidence contact funnel (the landed analytic FF
/// arm — an exact analytic locus). Two identical runs must produce
/// bit-identical certified contact complexes.
fn canonical_analytic_locus_verdict() -> String {
    use truck_evidence::contact::{BoundedStratum, ContactLocus, contact};
    use truck_geometry::recognize::CanonicalSurface;

    let ps = plane_sphere_fixture();
    let plane = BoundedStratum::Face {
        surface: CanonicalSurface::Plane(ps.plane),
        u_range: (-2.0, 2.0),
        v_range: (-2.0, 2.0),
    };
    let sphere = BoundedStratum::Face {
        surface: CanonicalSurface::Sphere(ps.sphere),
        u_range: (0.0, std::f64::consts::PI),
        v_range: (0.0, std::f64::consts::TAU),
    };
    let mut budget = Budget::new(0, 0, 0);
    let certified = match contact(&plane, &sphere, &mut budget) {
        Ok(Certified { value, .. }) => value,
        Err(refusal) => {
            panic!("the canonical plane × sphere pair must certify, refused {refusal:?}")
        }
    };
    let loci: Vec<&str> = certified
        .contacts
        .iter()
        .map(|record| match &record.locus {
            ContactLocus::Analytic(_) => "analytic",
            ContactLocus::Coincident => "coincident",
            ContactLocus::Point(_) => "point",
            ContactLocus::BoundedCurve { .. } => "bounded_curve",
            ContactLocus::ValidatedBranchCover(_) => "branch_cover",
        })
        .collect();
    assert!(
        !loci.is_empty(),
        "the transverse canonical pair must produce at least one analytic locus"
    );
    format!("{certified:?}")
}

/// The recorded lifted-rows evidence of `corpus/ttc/SKIPS.json`: the twelve
/// PB-011B-lifted canonical-cutter F1 rows (the "12") and the PB-011C census
/// records of the eight swept × swept rows stay staged under the
/// swept-carrier reason, and no C row carries a lift marker. This is the
/// machine-recorded admission boundary the facade consult mirrors (the "1"
/// lifted F1 row `f1/monocoque` of the PB-011 wave is covered by the same
/// evidence check below).
fn lifted_f1_rows_evidence() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus/ttc/SKIPS.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("corpus/ttc/SKIPS.json must be readable: {e}"));
    let value: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("corpus/ttc/SKIPS.json must parse: {e}"));
    let rows = value
        .get("rows")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("SKIPS.json must carry a rows array"));
    let note_of = |id: &str| -> String {
        rows.iter()
            .find(|row| row.get("id").and_then(serde_json::Value::as_str) == Some(id))
            .and_then(|row| row.get("note"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string()
    };

    // The 12 canonical-cutter F1 rows PB-011B lifted (evidence marker
    // `PB-011B LIFT EVIDENCE`).
    const B_COHORT: [&str; 12] = [
        "f1/floor",
        "f1/diffuser",
        "f1/suspension_front",
        "f1/suspension_rear",
        "f1/steering_rack",
        "f1/track_rod_left",
        "f1/track_rod_right",
        "f1/corner_fl",
        "f1/corner_fr",
        "f1/corner_rl",
        "f1/corner_rr",
        "f1/drs_actuator",
    ];
    for id in B_COHORT {
        assert!(
            note_of(id).contains("PB-011B LIFT EVIDENCE"),
            "lifted F1 row {id} must carry its PB-011B lift evidence marker"
        );
    }

    // The 8 swept × swept F1 rows PB-011C ran census-first: every row stays
    // staged under the swept-carrier reason with a typed-refusal census record
    // and no lift marker — the boundary ADM-004's admission consult mirrors.
    const C_COHORT: [&str; 8] = [
        "f1/airbox",
        "f1/beam_wing",
        "f1/details",
        "f1/drivetrain",
        "f1/drs_flap",
        "f1/engine_cover",
        "f1/power_unit",
        "f1/rear_wing",
    ];
    for id in C_COHORT {
        let note = note_of(id);
        assert!(
            note.contains("PB-011C CENSUS") && note.contains("typed-refusal"),
            "swept × swept row {id} must file its typed-refusal census record"
        );
        assert!(
            !note.contains("PB-011C LIFT EVIDENCE"),
            "swept × swept row {id} must not carry a lift marker"
        );
    }

    // The "1": the `f1/monocoque` row of the PB-011 wave carries its lift
    // evidence marker (the second-door lift recorded beside the census).
    assert!(
        note_of("f1/monocoque").contains("PB-011 LIFT EVIDENCE"),
        "the monocoque row must carry its PB-011 lift evidence marker"
    );
}

#[test]
fn v5_battery_every_landed_green_path_bit_identical() {
    // (a) Restricted sweeps: the landed green restricted battery (plane ×
    // sphere, sweep × plane) certifies bit-identically on identical re-runs
    // through the certified restricted engine.
    let first = restricted_sweep_verdicts();
    let second = restricted_sweep_verdicts();
    assert_eq!(
        first, second,
        "the restricted-sweep green battery answers bit-identically on identical re-runs (V5)"
    );

    // (b) Canonical analytic loci: the plane × sphere canonical pair answers
    // bit-identically through the evidence contact funnel's analytic arm.
    let first_analytic = canonical_analytic_locus_verdict();
    let second_analytic = canonical_analytic_locus_verdict();
    assert_eq!(
        first_analytic, second_analytic,
        "the canonical analytic locus answers bit-identically on an identical re-run (V5)"
    );

    // (c) The admitted cut row (ADM-004): the certified pair cut between the
    // two spline-loft faces certifies bit-identically (the new green path
    // inherits the battery).
    let x = shallow_loft_xz(0.05);
    let y = shallow_loft_xy(0.05);
    let cut = |pipeline: String| {
        let row = certify_admitted_cut_pair(&x, &y, &domain_hint(), &fixture_budget())
            .expect("the admitted pair cut certifies");
        format!("{pipeline}: {row:?}")
    };
    assert_eq!(
        cut("cut(spline-loft,spline-loft)".to_string()),
        cut("cut(spline-loft,spline-loft)".to_string()),
        "the admitted cut answers bit-identically on an identical re-run (V5)"
    );

    // (d) The recorded lifted-rows evidence holds: the twelve PB-011B-lifted
    // F1 rows' lift markers and the PB-011C census records stay on the SKIPS
    // file, unchanged through the admission consult.
    lifted_f1_rows_evidence();
}

// ---------------------------------------------------------------------------
// Required test 3: non-admitted carriers refuse unchanged
// ---------------------------------------------------------------------------

#[test]
fn non_admitted_carriers_refuse_unchanged() {
    // A general spline-section pair (biquadratic graph faces, NOT the admitted
    // ruled-section class) keeps the adapter's exact typed InvalidInput refusal
    // through the certified cut pipeline, identically on re-runs (no verdict
    // flip).
    let bowl = graph_face(0.0);
    let octant = graph_face(3.0);
    let refusal =
        |x: &BSplineSurface<Vector4>, y: &BSplineSurface<Vector4>| match certify_admitted_cut_pair(
            x,
            y,
            &domain_hint(),
            &fixture_budget(),
        ) {
            Err(ConstructRefusal::InvalidInput) => "InvalidInput".to_string(),
            other => panic!("a non-admitted carrier pair must refuse InvalidInput, got {other:?}"),
        };
    for _ in 0..2 {
        assert_eq!(
            refusal(&bowl, &octant),
            "InvalidInput",
            "a general spline-section pair keeps its typed refusal"
        );
    }

    // A non-positive-weight face refuses at extraction (typed), identically.
    let bad = non_positive_weight_face();
    for _ in 0..2 {
        match certify_admitted_cut_pair(&bad, &bowl, &domain_hint(), &fixture_budget()) {
            Err(ConstructRefusal::InvalidInput) => {}
            other => panic!("a non-positive-weight face must refuse InvalidInput, got {other:?}"),
        }
    }

    // The certificate carriers keep refusing Unfrozen: their frozen
    // constructors are untouched by the wiring.
    let cone = NormalCone {
        anchor: [0.0, 0.0, 1.0],
        s_up: 0.5,
    };
    assert_eq!(RegularPatch::try_new(cone), Err(ConstructRefusal::Unfrozen));
    assert_eq!(
        CollapsedBoundary::try_new(2),
        Err(ConstructRefusal::Unfrozen)
    );
    assert_eq!(
        SeamIdentified::try_new((EdgeId(0), EdgeId(1))),
        Err(ConstructRefusal::Unfrozen)
    );
    assert_eq!(
        TransversePair::try_new((1.0e-2, 1.0e-1)),
        Err(ConstructRefusal::Unfrozen)
    );
    assert!(
        matches!(
            admit_tensor_spline_pair(&bowl, &octant, &domain_hint()),
            Err(ConstructRefusal::InvalidInput)
        ),
        "the bare adapter keeps the exact typed refusal on the repeated general pair"
    );
}
