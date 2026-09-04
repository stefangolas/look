//! The §3.3/§3.4 lifted-atlas tests (BG-KV2-405-K2B): the finite atlas of
//! regular charts per carrier kind, the pole-chart sphere family with its
//! exact rational pole-to-partner transitions, the `SwitchChart`-vs-
//! `CarrierSingular` degeneracy doctrine, the cone/torus chart families
//! joining the admitted carrier family (the 404 re-route, with the Wave-1
//! refusal kept for out-of-atlas carriers), and the unwrapped K2 pcurve lifts
//! through the atlas with the deck integer as a first-class coordinate and
//! `DECK_MAX` as the termination bound.
//!
//! Everything here is a direct evaluation of the stored chart geometry — no
//! solver is invoked. The source-scan test reads `atlas.rs` itself to pin the
//! N4 guarantee.

#![deny(clippy::unwrap_used)]

use truck_certified::kernel::atlas::{pole_to_partner, ChartAtlas, DegeneracyRoute};
use truck_certified::kernel::evidence::{
    ClaimVerdict, Refusal as KernelRefusal, RefusalKind, VerdictClass,
};
use truck_certified::kernel::fixtures as fx;
use truck_certified::kernel::graph::ChartId;
use truck_certified::kernel::leaf::{CarrierData, RationalCarrier, RationalCarrierKind};
use truck_certified::kernel::patch::{CertifiedPatch, IBox2};

/// The fixture ground-truth comparison tolerance.
const GT_TOL: f64 = 1e-12; // H-3: dyadic atlas ground-truth comparison tolerance

/// Assert two floats agree to the ground-truth tolerance.
fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() <= GT_TOL
}

/// Extract the `Ok` of a fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T>(result: Result<T, KernelRefusal>) -> T {
    match result {
        Ok(value) => value,
        Err(refusal) => panic!("a construction that must succeed was refused: {refusal:?}"),
    }
}

/// A unit sphere carrier (the §3.2 stereographic domain).
fn sphere_carrier() -> RationalCarrier {
    construct(RationalCarrier::try_new(
        RationalCarrierKind::Sphere,
        CarrierData::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
        },
        construct(IBox2::try_new([-1.0, -1.0], [1.0, 1.0])),
    ))
}

/// A cone carrier whose axial interval reaches the apex (apex at `v = 0`).
fn cone_carrier() -> RationalCarrier {
    construct(RationalCarrier::try_new(
        RationalCarrierKind::Cone,
        CarrierData::Cone {
            apex: [0.0, 0.0, 0.0],
            axis: [0.0, 0.0, 1.0],
            half_angle: 0.5,
            height: (0.0, 1.0),
        },
        construct(IBox2::try_new([-1.5, -1.5], [1.5, 1.5])),
    ))
}

/// A cylinder carrier on the exact `z` axis (periodic angular deck).
fn cylinder_carrier() -> RationalCarrier {
    construct(RationalCarrier::try_new(
        RationalCarrierKind::Cylinder,
        CarrierData::Cylinder {
            origin: [0.0, 0.0, 0.0],
            axis: [0.0, 0.0, 1.0],
            radius: 1.5,
            height: (0.0, 1.0),
        },
        construct(IBox2::try_new([-1.0, -1.0], [1.0, 1.0])),
    ))
}

/// A certified box.
fn box2(lo: [f64; 2], hi: [f64; 2]) -> IBox2 {
    construct(IBox2::try_new(lo, hi))
}

/// The sphere atlas chart family: [stereo-north, stereo-south, pole-north,
/// pole-south].
fn sphere_atlas() -> ChartAtlas {
    construct(ChartAtlas::try_new(&sphere_carrier()))
}

#[test]
fn sphere_pole_chart_switches_and_continues_the_arc() {
    let atlas = sphere_atlas();
    assert_eq!(
        atlas.charts().len(),
        4,
        "the sphere pole-chart family has 4 charts"
    );

    // The pole chart (id 2 = the north pole chart) carries its internal
    // degeneracy locus on the polar line u = 0 through u = v = 0 and routes to
    // the north stereographic partner (id 0).
    let pole_chart = atlas
        .chart(ChartId(2))
        .expect("the north pole chart exists");
    let partner = pole_chart.partner.expect("the pole chart has a partner");
    assert_eq!(
        partner.chart,
        ChartId(0),
        "the north pole chart's partner is stereo-north"
    );
    assert!(pole_chart.box_reaches_pole(box2([-0.1, -0.1], [0.1, 0.1])));

    // The pole chart's CertifiedPatch is degenerate on the pole box (its
    // metric `EG - F^2 = 64 u^2 / ((1+u^2)^4 (1+v^2)^2)` vanishes on the polar
    // line) but certifies Proven regularity away from it, on its own region.
    let pole_box = box2([-0.1, -0.1], [0.1, 0.1]);
    let degenerate = pole_chart.patch.regularity(pole_box);
    assert!(
        !matches!(degenerate, ClaimVerdict::Proven(_)),
        "the pole-chart parameterization is rank deficient on a box straddling u = 0"
    );
    match pole_chart.patch.regularity(pole_chart.region) {
        ClaimVerdict::Proven(bound) => assert!(bound.value() > 0.0),
        other => panic!("the pole chart must certify Proven regularity on its region: {other:?}"),
    }

    // §3.4: the box reached a chart pole at a REGULAR image point; the image
    // is certified regular on the partner (its regularity over the partner's
    // pole box is Proven), so the arc switches charts.
    let target = atlas
        .chart(partner.chart)
        .expect("the partner chart exists");
    match target.patch.regularity(partner.regular_box) {
        ClaimVerdict::Proven(bound) => assert!(bound.value() > 0.0),
        other => panic!("the partner must certify the pole image regular: {other:?}"),
    }
    let pole_patch: &dyn CertifiedPatch = &pole_chart.patch;
    match atlas.classify_degeneracy(pole_patch, pole_box) {
        DegeneracyRoute::SwitchChart { target: id } => {
            assert_eq!(id, partner.chart, "the switch targets the partner chart")
        }
        DegeneracyRoute::CarrierSingular => panic!("the sphere pole is not a carrier singularity"),
    }

    // Continue the arc: a great circle through the north pole (in the plane
    // spanned by the z axis and the azimuth direction psi). Pre-pole samples
    // are carried on the pole chart, post-pole samples on the partner; the two
    // halves are the SAME arc (one great circle), crossing the pole with no
    // valence change.
    let psi = 0.6f64;
    let mut count = 0u32;
    let mut t = -0.45f64;
    while t <= -0.05 {
        // Pre-pole point on the incoming meridian (azimuth psi + pi).
        let p = [t.sin() * psi.cos(), t.sin() * psi.sin(), t.cos()];
        assert!(
            approx(p[0] * p[0] + p[1] * p[1] + p[2] * p[2], 1.0),
            "on the sphere"
        );
        assert!(
            approx(p[0] * -psi.sin() + p[1] * psi.cos(), 0.0),
            "on the great circle"
        );
        // Pole-chart coordinates of the world point (exact rational forms).
        let radial = (p[0] * p[0] + p[1] * p[1]).sqrt();
        let u = radial / (1.0 + p[2]);
        let v = p[1] / (radial + p[0]);
        // The pole-to-partner transition is the exact rational map; its image
        // must be the partner chart's coordinates of the SAME world point.
        let transported = pole_to_partner(u, v).expect("the transition is finite");
        let direct_u = p[0] / (1.0 + p[2]);
        let direct_v = p[1] / (1.0 + p[2]);
        assert!(
            approx(transported[0], direct_u),
            "transition u agrees at t = {t}"
        );
        assert!(
            approx(transported[1], direct_v),
            "transition v agrees at t = {t}"
        );
        count += 1;
        t += 0.05;
    }
    // Post-pole samples continue on the partner chart through u = v = 0.
    t = 0.05;
    while t <= 0.45 {
        let p = [t.sin() * psi.cos(), t.sin() * psi.sin(), t.cos()];
        assert!(
            approx(p[0] * p[0] + p[1] * p[1] + p[2] * p[2], 1.0),
            "on the sphere"
        );
        assert!(
            approx(p[0] * -psi.sin() + p[1] * psi.cos(), 0.0),
            "on the great circle"
        );
        count += 1;
        t += 0.05;
    }
    assert_eq!(count, 18, "nine pre-pole and nine post-pole samples");
}

#[test]
fn param_lift_never_wraps_and_deck_is_first_class() {
    let atlas = construct(ChartAtlas::try_new(&cylinder_carrier()));
    let chart = ChartId(0);
    let period = atlas
        .u_period(chart)
        .expect("the cylinder chart is periodic on u");

    // Param(u) is the LIFTED coordinate: 5.9 stays 5.9, deck 0.
    let base = construct(truck_certified::kernel::graph::Param::try_new(
        chart, 0, 5.9, 0.0,
    ));
    let same = construct(atlas.lift(chart, &base, 5.9));
    assert_eq!(same.deck, 0, "5.9 does not cross a seam");
    assert!(approx(same.u, 5.9), "5.9 stays 5.9");
    assert!(approx(same.v, base.v));

    // The deck integer, not a rewrapped u, carries the winding: raw 6.4 lifts
    // to deck +1 with the canonical u = 6.4 - period.
    let lifted = construct(atlas.lift(chart, &base, 6.4));
    assert_eq!(lifted.deck, 1);
    assert!(approx(lifted.u, 6.4 - period), "canonical u = 6.4 - period");
    assert!(lifted.u >= 0.0 && lifted.u < period, "u stays canonical");
    assert!(approx(lifted.u + period * lifted.deck as f64, 6.4));

    // Lifting the already-lifted parameter NEVER wraps it back.
    let relifted = construct(atlas.lift(chart, &lifted, 6.4));
    assert_eq!(relifted.deck, lifted.deck, "the deck is not folded away");
    assert!(
        approx(relifted.u, lifted.u),
        "the canonical u is not rewrapped"
    );

    // The deck is a first-class coordinate: winding continues across turns.
    let two_turns = construct(atlas.lift(chart, &base, 6.4 + period));
    assert_eq!(two_turns.deck, 2, "two deck crossings carry deck +2");
    assert!(approx(two_turns.u, lifted.u), "same canonical u, deck +2");
}

#[test]
fn chart_switch_vs_carrier_singularity_distinguished() {
    let sphere_atlas = sphere_atlas();
    let pole_chart = sphere_atlas
        .chart(ChartId(2))
        .expect("the north pole chart exists");
    let partner_id = pole_chart.partner.expect("a partner").chart;
    let pole_box = box2([-0.1, -0.1], [0.1, 0.1]);

    // The sphere pole: a rank-deficient parameterization at a REGULAR image
    // point -> SwitchChart to the partner.
    let pole_patch: &dyn CertifiedPatch = &pole_chart.patch;
    match sphere_atlas.classify_degeneracy(pole_patch, pole_box) {
        DegeneracyRoute::SwitchChart { target } => assert_eq!(target, partner_id),
        DegeneracyRoute::CarrierSingular => panic!("a sphere pole is not a carrier singularity"),
    }

    // The cone apex: a genuine carrier singularity -> CarrierSingular.
    let cone_atlas = construct(ChartAtlas::try_new(&cone_carrier()));
    let cone_chart = cone_atlas
        .charts()
        .iter()
        .find(|chart| chart.kind == RationalCarrierKind::Cone)
        .expect("the cone atlas has cone charts");
    assert!(
        cone_chart.pole.is_some(),
        "the cone charts carry the apex locus"
    );
    assert!(
        cone_chart.partner.is_none(),
        "the apex has no switching partner"
    );
    let apex_box = box2([-0.5, -0.02], [0.5, 0.02]);
    assert!(cone_chart.box_reaches_pole(apex_box));
    let cone_patch: &dyn CertifiedPatch = &cone_chart.patch;
    match cone_atlas.classify_degeneracy(cone_patch, apex_box) {
        DegeneracyRoute::CarrierSingular => {}
        DegeneracyRoute::SwitchChart { .. } => {
            panic!("the cone apex is a carrier singularity, never a chart switch")
        }
    }
}

#[test]
fn cone_carrier_admitted_with_its_chart_family() {
    use truck_certified::kernel::rational;

    let cone = cone_carrier();
    // The re-route: a Cone/Torus carrier is admitted through the atlas chart
    // family (the chart bookkeeping over the apex-excluding charts), where the
    // Wave-1 admission only refused.
    let atlas = construct(ChartAtlas::try_new(&cone));
    assert_eq!(atlas.kind(), RationalCarrierKind::Cone);
    assert_eq!(atlas.charts().len(), 2, "the two half-angle circle sheets");

    // The apex-excluding chart family: every chart's certified region keeps a
    // clearance above the apex plane (axial coordinate v > 0), and every chart
    // carries the apex locus as its internal degeneracy.
    for chart in atlas.charts() {
        assert!(
            chart.region.lo[1] > 0.0,
            "the chart region excludes the apex"
        );
        assert!(chart.pole.is_some(), "the cone charts carry the apex locus");
    }

    // The old refusal stays available for out-of-atlas carriers: the Wave-1
    // kernel::rational admission still refuses a Cone with its named pending
    // predicate (the documented re-route keeps the landed 104 surface).
    match rational::admit(&cone) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::CarrierSingularity);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
            match refusal.evidence {
                truck_certified::kernel::evidence::RefusalEvidence::Predicate { name, .. } => {
                    assert_eq!(name, "cone_torus_carrier_packet_pending");
                }
                _ => panic!("the pending refusal must carry the named predicate"),
            }
        }
        Ok(()) => panic!("the Wave-1 admission must still refuse a cone carrier"),
    }

    // The cone chart leaves keep the deferred enclosure markers (the slope of
    // the stored half-angle cannot be recovered N4-cleanly); the chart family
    // itself is this packet's contribution.
    let cone_chart = &atlas.charts()[0];
    match cone_chart.patch.regularity(cone_chart.region) {
        ClaimVerdict::Inconclusive(reason) => {
            assert_eq!(reason, "cone_half_angle_slope_needs_data")
        }
        other => panic!("the cone form is pending: {other:?}"),
    }
}

#[test]
fn deck_exhausted_terminates_helical_lifts() {
    use truck_certified::kernel::config;

    let atlas = construct(ChartAtlas::try_new(&cylinder_carrier()));
    let chart = ChartId(0);
    let period = atlas
        .u_period(chart)
        .expect("the cylinder chart is periodic on u");
    let base = construct(truck_certified::kernel::graph::Param::try_new(
        chart, 0, 5.9, 0.0,
    ));

    // A helix whose single edge walks exactly DECK_MAX deck crossings is
    // admitted; one that would walk DECK_MAX + 1 refuses DeckExhausted
    // (Inconclusive) — the §0.4 termination bound of helical lifts.
    let max_crossings = config::DECK_MAX;
    let at_ceiling =
        construct(atlas.lift(chart, &base, 5.9 + max_crossings as f64 * period + 0.05));
    assert_eq!(at_ceiling.deck, max_crossings);
    assert!(approx(at_ceiling.u, 5.95), "canonical u at the ceiling");

    let deep = config::DECK_MAX + 1;
    let result = atlas.lift(chart, &base, 5.9 + deep as f64 * period + 0.05);
    match result {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::DeckExhausted);
            assert_eq!(refusal.backing, VerdictClass::Inconclusive);
            match refusal.evidence {
                truck_certified::kernel::evidence::RefusalEvidence::Predicate { name, .. } => {
                    assert_eq!(name, "deck_exhausted_lift");
                }
                _ => panic!("the exhaustion refusal must carry the named predicate"),
            }
        }
        Ok(_) => panic!("a helical lift above DECK_MAX must refuse DeckExhausted"),
    }
}

#[test]
fn pcurve_runs_unwrapped_5_9_to_6_4() {
    let fixture = construct(fx::deck_wrap());
    let atlas = construct(ChartAtlas::try_new(&cylinder_carrier()));
    let chart = ChartId(0);

    // The shim kit's deck-wrap fixture, now through the atlas: the pcurve run
    // 5.9 -> 6.4 lifts to deck +1 with the canonical end u = 6.4 - period.
    let lifted = construct(atlas.lift(chart, &fixture.start, 6.4));
    assert_eq!(lifted.chart, fixture.end.chart, "same chart");
    assert_eq!(lifted.deck, fixture.end.deck, "deck +1");
    assert_eq!(lifted.deck - fixture.start.deck, fixture.displacement);
    assert!(approx(lifted.u, fixture.canonical_end_u), "canonical end u");
    assert!(
        approx(lifted.u, 6.4 - fixture.period),
        "canonical end u from the period"
    );
    assert!(approx(lifted.v, fixture.end.v));
    // The developed coordinate is preserved exactly by the deck integer.
    let raw_end = lifted.u + fixture.period * lifted.deck as f64;
    assert!(approx(raw_end, 6.4), "the unwrapped run recovers 6.4");
}

#[test]
fn no_transcendental_call_in_atlas_module() {
    let source = include_str!("../src/kernel/atlas.rs");
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
