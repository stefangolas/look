#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//! The SSI branch-tracing continuation loop (BG-CK-P2-TRACE).
//!
//! This module is the WAVE-ONLY realization of the trace discipline against the
//! shim's frozen types and the fixture kit. It is built in wave mode: the loop
//! drives a SOLVER-PRIVATE per-box certifier interface ([`BranchCertifier`],
//! [`BranchBox`], [`BranchStep`]) that W1's certificate evaluator is adapted to
//! at integration. Nothing here evaluates a real 3x3 Krawczyk operator; the
//! synthetic certifiers of this module's tests walk the fixture kit's known
//! geometry and report fixture outcomes, exactly as the wave split prescribes.
//!
//! # The discipline this module owns
//!
//! - **Seed.** A branch is traced from one isolated [`KrawczykCertificate3`]
//!   (the shim's Krawczyk3 output shape). The loop hands the seed to the
//!   certifier as the first box hint and consumes the certified steps it
//!   reports.
//! - **Identity recurrence.** A step whose box equals the first box's identity
//!   closes the branch: [`TraceOutcome::ClosedLoop`] (the `closed_loop_pair()`
//!   fixture's ground truth).
//! - **Domain exit, no refusal.** A step whose box has left the chart domain is
//!   a natural end: [`TraceOutcome::Terminated`].
//! - **Turning-point switching is the frozen both-certificate rule.** A
//!   coordinate switch is emitted ONLY as a [`CoordinateSwitch`] carrying both
//!   certificates (the frozen F3 contract in `contract.rs`; no default, no
//!   heuristic reseed). A certifier that reports a switch box with ONE
//!   certificate is a refusal, never a reseed; the loop refuses with the named
//!   conditioning case. A box that changes its continuation coordinate without
//!   reporting a certified switch is likewise refused. The loop itself never
//!   reseeds.
//! - **Refusals are named.** Every refusal the loop can emit wraps a LANDED
//!   named cause through [`TraceRefusal`]; there is no catch-all and no stringly
//!   refusal.
//!
//! # Branch records and germs
//!
//! Every emitted [`TraceStep`] carries the [`BranchIncidence`]-shaped record
//! (span + certified parameter enclosure + germ + deck label, `formal/contact.rs`)
//! and the certified continuation coordinate for its box. Germ classification
//! follows the `span.rs` discipline: a zero first jet reads the next nonzero
//! jet ([`classify_branch_germ`]); its correctness is machine-checked against
//! the `germ_ladder()` fixture in the in-module tests.
//!
//! H-1: this module (including its `#[cfg(test)]` code) is written under the
//! crate-level `deny` lint with no module-level opt-out, matching the HULL
//! precedent.

use crate::contract::{ContinuationCoordinate, CoordinateSwitch, Refusal};
use crate::formal::span::BranchGerm;
use crate::ssi_types::{
    KrawczykCertificate3, SquareSystem3, TraceOutcome, TraceRefusal, TraceStep,
};

/// One per-box Krawczyk step, as the loop consumes it.
///
/// Solver-private (the HULL precedent): NOT public API, not re-exported. At
/// integration the orchestrator adapters W1's certificate evaluator to this
/// shape. The exact shape (`BranchBox` naming, argument order, error side) is
/// this packet's to fix; what stays frozen is the one-per-box shape, the
/// `pub(crate)` reach, and the named-refusal error side.
///
/// A certifier is the SOLVER side: it walks the branch geometry it was built
/// from and reports, per box hint, either a certified [`BranchStep`] or a named
/// [`TraceRefusal`]. It never performs the loop's discipline (closure, domain
/// exit, both-certificate switching) itself; those are the loop's.
#[allow(dead_code)] // wave-private seam: exercised by this module's cfg(test) certifiers, consumed by W1's adapter at integration
pub(crate) trait BranchCertifier {
    /// Certify one parameter box along the branch.
    ///
    /// `hint` carries the previous certified step (or, on the first call, the
    /// seed [`KrawczykCertificate3`]). The certifier answers with the certified
    /// step for the requested box — an advance along the branch, a turning-point
    /// switch report, or a named refusal. The certifier must keep producing
    /// certified boxes until the branch closes, leaves the chart domain, hits a
    /// certified switch, or refuses; the loop decides which of those happened.
    fn step(&mut self, hint: &BranchBox) -> Result<BranchStep, TraceRefusal>;
}

/// One parameter-box request the loop addresses to a [`BranchCertifier`].
///
/// `previous` is the last certified step (absent on the seed box); `seed` is
/// the isolated Krawczyk certificate the branch is seeded from, present only on
/// the first request. The hint is a REQUEST the certifier answers, not geometry
/// the loop computes.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // wave-private seam, see [`BranchCertifier`]
pub(crate) struct BranchBox {
    /// The previous certified step along the branch, absent on the seed box.
    previous: Option<TraceStep>,
    /// The seed Krawczyk certificate, present only on the first request.
    seed: Option<KrawczykCertificate3>,
}

impl BranchBox {
    /// The box hint for the seed request.
    #[allow(dead_code)] // wave-private seam, see [`BranchCertifier`]
    fn seed(seed: &KrawczykCertificate3) -> Self {
        Self {
            previous: None,
            seed: Some(*seed),
        }
    }

    /// The box hint advancing from `previous`.
    #[allow(dead_code)] // wave-private seam, see [`BranchCertifier`]
    fn advance(previous: &TraceStep) -> Self {
        Self {
            previous: Some(*previous),
            seed: None,
        }
    }
}

/// One certified per-box outcome a [`BranchCertifier`] may report.
///
/// The variants are exactly the solver-side facts the loop's discipline needs:
/// a certified continuation box, or a turning-point switch report (both-certificate
/// rule). A natural branch end inside the chart and every refusal path are NOT
/// verdicts: the loop decides domain exit from the box it is handed, and
/// refusals travel on the `Err` side.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // wave-private seam, see [`BranchCertifier`]
pub(crate) enum BranchStep {
    /// A certified continuation box along the branch.
    Advance(TraceStep),
    /// A turning-point switch report. The both-certificate rule is enforced by
    /// the LOOP on this report: an `outgoing` certificate present and
    /// consistent with the traced coordinate yields
    /// [`TraceOutcome::Switched`]; anything less is a named refusal, never a
    /// reseed.
    Switch(SwitchReport),
}

/// A certifier's report of a turning-point switch box.
///
/// `step` is the switch box certified under the INCOMING continuation
/// coordinate. `outgoing` is the OUTGOING coordinate's certificate for the
/// switch box when the certifier could certify it; `None` is exactly the
/// one-certificate case the frozen F3 rule refuses.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // wave-private seam, see [`BranchCertifier`]
pub(crate) struct SwitchReport {
    /// The switch box, certified under the incoming coordinate.
    step: TraceStep,
    /// The outgoing coordinate's certificate, when both certify.
    outgoing: Option<ContinuationCoordinate>,
}

/// Trace one branch from one isolated Krawczyk seed certificate.
///
/// The loop steps boxes along the branch through `certifier` and applies its
/// own discipline to the reported steps:
///
/// - a step whose box equals the first box's identity closes the branch
///   ([`TraceOutcome::ClosedLoop`], identity recurrence);
/// - a step whose box has left `domain` ends the branch without a refusal
///   ([`TraceOutcome::Terminated`]);
/// - a certified switch report carrying BOTH certificates yields
///   [`TraceOutcome::Switched`] with the frozen [`CoordinateSwitch`];
/// - a switch report carrying one certificate, or a box that changes its
///   continuation coordinate without a certified switch, is a named refusal
///   ([`TraceOutcome::Refused`]) — never a reseed, never a default;
/// - a named certifier refusal propagates verbatim.
///
/// `domain` is the chart rectangle as four axis intervals in `(u, v, s, t)`
/// order (the two surface charts the branch lives in).
#[allow(dead_code)] // wave-private seam: driven by the cfg(test) certifiers; W1's adapter plugs in at integration
pub(crate) fn trace_branch<C: BranchCertifier>(
    seed: &KrawczykCertificate3,
    domain: [(f64, f64); 4],
    certifier: &mut C,
) -> TraceOutcome {
    let mut steps: Vec<TraceStep> = Vec::new();
    let mut running: Option<ContinuationCoordinate> = None;
    loop {
        let hint = match steps.last() {
            Some(previous) => BranchBox::advance(previous),
            None => BranchBox::seed(seed),
        };
        match certifier.step(&hint) {
            Err(refusal) => return TraceOutcome::Refused(refusal),
            Ok(BranchStep::Advance(step)) => {
                if let Some(first) = steps.first() {
                    // Identity recurrence: the branch closed on itself.
                    if step.chart_box() == first.chart_box() {
                        steps.push(step);
                        return TraceOutcome::ClosedLoop { steps };
                    }
                }
                if let Some(running_coordinate) = running {
                    // The frozen both-certificate rule: a box that silently
                    // changes its continuation coordinate is an under-certified
                    // switch — refuse, never reseed.
                    if step.coordinate().index != running_coordinate.index {
                        return TraceOutcome::Refused(TraceRefusal::Conditioning(
                            Refusal::ConditioningBelowThreshold,
                        ));
                    }
                }
                if !box_inside_domain(step.chart_box(), domain) {
                    // Domain exit with no refusal.
                    return TraceOutcome::Terminated { steps };
                }
                running = Some(step.coordinate());
                steps.push(step);
            }
            Ok(BranchStep::Switch(report)) => {
                let SwitchReport { step, outgoing } = report;
                let Some(outgoing_coordinate) = outgoing else {
                    // One certificate at a switch box: the outgoing square
                    // system could not certify. Refuse with the frozen
                    // conditioning cause; never reseed, never a default.
                    return TraceOutcome::Refused(TraceRefusal::Conditioning(
                        Refusal::ConditioningBelowThreshold,
                    ));
                };
                let running_ok = match running {
                    Some(running_coordinate) => {
                        running_coordinate.index == outgoing_coordinate.index
                    }
                    None => false,
                };
                if !running_ok {
                    // The switch is not a switch FROM the coordinate the branch
                    // was traced under: a request outside the frozen rule.
                    return TraceOutcome::Refused(TraceRefusal::Conditioning(
                        Refusal::InvalidInput,
                    ));
                }
                let incoming_coordinate = step.coordinate();
                if incoming_coordinate.index == outgoing_coordinate.index {
                    // A switch must change the continuation coordinate.
                    return TraceOutcome::Refused(TraceRefusal::Conditioning(
                        Refusal::InvalidInput,
                    ));
                }
                let switch = CoordinateSwitch {
                    outgoing: outgoing_coordinate,
                    incoming: incoming_coordinate,
                };
                steps.push(step);
                return TraceOutcome::Switched { steps, switch };
            }
        }
    }
}

/// Whether a 4D chart box is fully contained in the chart domain.
#[allow(dead_code)] // helper of the wave-private trace loop, see [`trace_branch`]
fn box_inside_domain(box_: [(f64, f64); 4], domain: [(f64, f64); 4]) -> bool {
    box_.iter()
        .zip(domain.iter())
        .all(|((lo, hi), (domain_lo, domain_hi))| lo >= domain_lo && hi <= domain_hi)
}

/// The jet-vanishing threshold of the germ classifier.
///
/// The `germ_ladder()` fixtures state their classes through exact small jets at
/// dyadic event coordinates; zero-vs-nonzero is decided far from every jet value
/// this ladder produces. This is a synthetic direct-evaluation check of the
/// classification discipline (H-3), never a certified enclosure comparison.
#[allow(dead_code)] // shared by [`classify_branch_germ`] and the module's cfg(test) certifiers
const GERM_JET_EPSILON: f64 = 1e-9;

/// Whether `event` lies strictly inside `chart_box` on every axis.
#[allow(dead_code)] // helper of the wave-private germ classifier, see [`classify_branch_germ`]
fn event_is_strictly_interior(event: (f64, f64, f64, f64), chart_box: [(f64, f64); 4]) -> bool {
    let coords = [event.0, event.1, event.2, event.3];
    coords
        .iter()
        .zip(chart_box.iter())
        .all(|(coord, (lo, hi))| lo < coord && coord < hi)
}

/// Classify the branch germ at an event by reading the next nonzero jet.
///
/// Implements the `span.rs` discipline for the trace: a zero first jet reads the
/// next nonzero jet (`k = min { j >= 1 : C^(j) != 0 }`). The classifier reads
/// jets of the reduced branch profile of the kit's diagonal-lift model (the
/// graph-pair geometry all of `germ_ladder()` realizes) through the kit's
/// plain-`f64` direct-evaluation helpers; classification correctness is
/// machine-checked against the ladder in this module's tests.
///
/// Decision order:
/// - an event on or outside the box boundary needs an endpoint certificate the
///   declared policy does not implement: [`BranchGerm::Unresolved`];
/// - a nonzero profile first jet in both physical coordinates is
///   [`BranchGerm::Regular`];
/// - a vanishing profile first jet with a nonzero second jet is
///   [`BranchGerm::StationaryRegular`] of order two (the ladder's stationary
///   rung) — a profile first jet that vanishes with every readable (up-to-second)
///   jet vanishing too cannot certify a finite order at this policy and is
///   [`BranchGerm::Unresolved`];
/// - a fully vanishing first jet reads the second jet: a nonzero second jet is a
///   cuspidal (tangent-collapsing) level curve, [`BranchGerm::CuspCandidate`];
///   an all-vanishing readable second jet is a collapsed (non-1D) stratum,
///   [`BranchGerm::Singular`].
#[allow(dead_code)] // wave-private germ discipline, machine-checked against germ_ladder in this module's tests
pub(crate) fn classify_branch_germ(
    system: &SquareSystem3,
    chart_box: [(f64, f64); 4],
    event: (f64, f64, f64, f64),
) -> BranchGerm {
    if !event_is_strictly_interior(event, chart_box) {
        return BranchGerm::Unresolved;
    }
    let degrees = system.degrees();
    let grids = system.grids();
    // First partials of every component along every chart axis at the event.
    let mut first = [[0.0f64; 4]; 3];
    for (component, grid) in grids.iter().enumerate() {
        for (axis, cell) in first[component].iter_mut().enumerate() {
            match crate::ssi_fixtures::partial_grid4_axis(grid, degrees, axis, event) {
                Some(value) => *cell = value,
                // A first jet that cannot be read is not a certified class.
                None => return BranchGerm::Unresolved,
            }
        }
    }
    // The profile is the branch's separation component: the component whose
    // first partials along the second patch's chart axes (`s`, `t`) are
    // smallest — the surface difference is carried perpendicular to the shared
    // physical coordinates. Lowest index breaks a tie.
    let mut profile = 0usize;
    let mut dependence = f64::INFINITY;
    for (component, row) in first.iter().enumerate() {
        let second_chart_dependence = row[2].abs() + row[3].abs();
        if second_chart_dependence < dependence {
            dependence = second_chart_dependence;
            profile = component;
        }
    }
    // Profile jets along the physical coordinates, chained through the identity
    // diagonal (s = u, t = v) of the kit's model.
    let gu = first[profile][0] + first[profile][2];
    let gv = first[profile][1] + first[profile][3];
    let second_along = |axis: usize| -> f64 {
        crate::ssi_fixtures::second_partial_grid4_axis(&grids[profile], degrees, axis, event)
            .map_or(0.0, |value| value)
    };
    let quu = second_along(0) + second_along(2);
    let qvv = second_along(1) + second_along(3);
    let is_zero = |value: f64| value.abs() < GERM_JET_EPSILON; // H-3
    if !is_zero(gu) && !is_zero(gv) {
        return BranchGerm::Regular;
    }
    if is_zero(gu) && is_zero(gv) {
        // First jet vanishes: read the next nonzero jet.
        if is_zero(quu) && is_zero(qvv) {
            return BranchGerm::Singular;
        }
        return BranchGerm::CuspCandidate;
    }
    // Exactly one profile first jet is nonzero: the branch is a graph whose
    // ordinate is stationary at the event. Read the ordinate's second jet.
    if is_zero(gu) {
        // `v` as a graph over `u`: v' = 0, v'' = -quu / gv.
        let v_second = -quu / gv;
        if is_zero(v_second) {
            return BranchGerm::Unresolved;
        }
        return BranchGerm::StationaryRegular {
            first_nonzero_order: 2,
        };
    }
    // `u` as a graph over `v`: u' = 0, u'' = -qvv / gu.
    let u_second = -qvv / gu;
    if is_zero(u_second) {
        return BranchGerm::Unresolved;
    }
    BranchGerm::StationaryRegular {
        first_nonzero_order: 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::IntervalEnclosure;
    use crate::formal::contact::{BranchIncidence, GenericUnresolved};
    use crate::hull::HullRefusal;
    use crate::ssi_fixtures;
    use std::collections::VecDeque;

    /// The trajectory this module's synthetic certifiers follow.
    const HALF_WIDTH: f64 = 0.02;

    /// Exit the test process on an unexpected construction refusal.
    ///
    /// The crate-level H-1 deny leaves the tests no extracting call that would
    /// breach it; a refused fixture construction is an environment error,
    /// reported through the process exit status (nonzero => test failure).
    fn refuse_ok<T>(result: Result<T, Refusal>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => {
                eprintln!("unexpected construction refusal: {error:?}");
                std::process::exit(1);
            }
        }
    }

    fn fail(message: &str) -> ! {
        eprintln!("{message}");
        std::process::exit(1);
    }

    /// A seed certificate for the fixtures: a valid strict-inclusion
    /// Krawczyk3-shaped box near the seed point (the reduced axes are the
    /// continuation certificate's reduced unknowns; the synthetic certifiers
    /// never read its numbers).
    fn seed_certificate() -> Option<KrawczykCertificate3> {
        let half = 0.02;
        let quarter = 0.01;
        KrawczykCertificate3::new(
            [
                (0.5 - half, 0.5 + half),
                (0.5 - half, 0.5 + half),
                (0.5 - half, 0.5 + half),
            ],
            [
                (0.5 - quarter, 0.5 + quarter),
                (0.5 - quarter, 0.5 + quarter),
                (0.5 - quarter, 0.5 + quarter),
            ],
            (1.0, 2.0),
        )
        .ok()
    }

    /// The chart domain of a fixture system as four axis intervals.
    fn chart_domain(system: &SquareSystem3) -> [(f64, f64); 4] {
        let maps = system.domain_maps();
        [
            (maps.0, maps.1),
            (maps.2, maps.3),
            (maps.4, maps.5),
            (maps.6, maps.7),
        ]
    }

    /// A synthetic continuation-coordinate certificate for the fixture walks.
    fn coordinate(index: usize) -> Result<ContinuationCoordinate, Refusal> {
        Ok(ContinuationCoordinate {
            index,
            relative_margin: IntervalEnclosure::new(0.5, 1.0)?,
        })
    }

    /// Build one certified step: a box around `center` under `coordinate_index`
    /// with `germ` and the branch-incidence record `incidence`.
    fn step_at(
        center: (f64, f64, f64, f64),
        germ: BranchGerm,
        incidence: BranchIncidence,
        coordinate_index: usize,
    ) -> Result<TraceStep, Refusal> {
        let chart_box = [
            (center.0 - HALF_WIDTH, center.0 + HALF_WIDTH),
            (center.1 - HALF_WIDTH, center.1 + HALF_WIDTH),
            (center.2 - HALF_WIDTH, center.2 + HALF_WIDTH),
            (center.3 - HALF_WIDTH, center.3 + HALF_WIDTH),
        ];
        TraceStep::new(chart_box, germ, incidence, coordinate(coordinate_index)?)
    }

    /// A point on the `closed_loop_pair()` circle at angle `theta`.
    fn circle_point(center: (f64, f64), radius: f64, theta: f64) -> (f64, f64, f64, f64) {
        let u = center.0 + radius * theta.cos();
        let v = center.1 + radius * theta.sin();
        (u, v, u, v)
    }

    /// A point on the `well_conditioned_root()` branch at continuation `s`.
    fn branch_point(s: f64) -> (f64, f64, f64, f64) {
        let v = 0.25 + 0.5 * s;
        (s, v, s, v)
    }

    /// Whether `system`'s stored `F` vanishes at a point (the branch ground
    /// truth), to within the direct-evaluation tolerance (H-3).
    fn point_is_on_branch(system: &SquareSystem3, point: (f64, f64, f64, f64)) -> bool {
        match ssi_fixtures::eval_system(system, point) {
            Some(values) => values.iter().all(|value| value.abs() < GERM_JET_EPSILON), // H-3
            None => false,
        }
    }

    /// Assert one step's incidence record round-trips against a reference.
    fn assert_step_carries_incidence(step: &TraceStep, reference: &BranchIncidence) {
        let incidence = step.incidence();
        assert_eq!(incidence.span_id, reference.span_id, "span id round-trip");
        assert_eq!(
            incidence.provenance, reference.provenance,
            "provenance round-trip"
        );
        assert_eq!(
            incidence.parameter, reference.parameter,
            "enclosure round-trip"
        );
        assert_eq!(
            incidence.location, reference.location,
            "location round-trip"
        );
        assert_eq!(incidence.side, reference.side, "canonical side round-trip");
        assert_eq!(incidence.deck, reference.deck, "deck label round-trip");
        assert_eq!(incidence.germ, step.germ(), "germ travels on the step");
        assert_eq!(incidence, *reference, "full incidence record round-trips");
    }

    /// A scripted certifier: a hand-written [`BranchCertifier`] whose branch was
    /// computed from the fixture geometry by the test helpers. It hands the
    /// loop one certified outcome per call and refuses (named) when exhausted —
    /// the loop's scenarios always end before exhaustion.
    struct ScriptedCertifier {
        outcomes: VecDeque<BranchStep>,
    }

    impl ScriptedCertifier {
        fn from_geometry(steps: Vec<TraceStep>, final_outcome: BranchStep) -> Self {
            let mut outcomes: VecDeque<BranchStep> =
                steps.into_iter().map(BranchStep::Advance).collect();
            outcomes.push_back(final_outcome);
            Self { outcomes }
        }

        fn refusing(outcome: BranchStep) -> Self {
            let mut outcomes = VecDeque::new();
            outcomes.push_back(outcome);
            Self { outcomes }
        }
    }

    impl BranchCertifier for ScriptedCertifier {
        fn step(&mut self, _hint: &BranchBox) -> Result<BranchStep, TraceRefusal> {
            match self.outcomes.pop_front() {
                Some(outcome) => Ok(outcome),
                None => Err(TraceRefusal::Conditioning(Refusal::InvalidInput)),
            }
        }
    }

    /// A certifier that refuses its first (and every) request with `refusal`.
    struct RefusingCertifier {
        refusal: TraceRefusal,
    }

    impl BranchCertifier for RefusingCertifier {
        fn step(&mut self, _hint: &BranchBox) -> Result<BranchStep, TraceRefusal> {
            Err(self.refusal)
        }
    }

    /// Build the closed-loop walk's certified steps: 11 geometric boxes around
    /// the fixture circle (angles `pi/2 + k*pi/6`, `k = 1..=11`) bracketed by
    /// two boxes at the `first_seed`, so the closing box equals the first box's
    /// identity exactly.
    fn closed_loop_steps(
        pair: &ssi_fixtures::ClosedLoopPair,
        incidence: BranchIncidence,
    ) -> Vec<TraceStep> {
        let mut steps = Vec::with_capacity(13);
        let first = refuse_ok(step_at(pair.first_seed, BranchGerm::Regular, incidence, 2));
        steps.push(first);
        for k in 1..=11 {
            let theta = std::f64::consts::FRAC_PI_2 + k as f64 * std::f64::consts::FRAC_PI_6;
            let point = circle_point(pair.center, pair.radius, theta);
            steps.push(refuse_ok(step_at(point, BranchGerm::Regular, incidence, 2)));
        }
        steps.push(first);
        steps
    }

    /// Build the boundary walk's certified steps: 10 boxes on the
    /// `well_conditioned_root()` branch from `s = 0.5` to `s = 0.95`, then one
    /// final box at `s = 1.0` whose interval leaves the chart domain.
    fn boundary_steps(incidence: BranchIncidence) -> Vec<TraceStep> {
        let mut steps = Vec::with_capacity(11);
        for k in 0..=9 {
            let s = 0.5 + k as f64 * 0.05;
            steps.push(refuse_ok(step_at(
                branch_point(s),
                BranchGerm::Regular,
                incidence,
                2,
            )));
        }
        steps.push(refuse_ok(step_at(
            branch_point(1.0),
            BranchGerm::Regular,
            incidence,
            2,
        )));
        steps
    }

    #[test]
    fn trace_loop_walks_fixture_closed_loop_to_identity_recurrence() {
        let pair = refuse_ok(ssi_fixtures::closed_loop_pair());
        let incidence = ssi_fixtures::sample_trace_incidence();
        let Some(seed) = seed_certificate() else {
            fail("seed certificate refused");
        };
        let domain = chart_domain(&pair.system);
        let mut certifier =
            ScriptedCertifier::from_geometry(closed_loop_steps(&pair, incidence), {
                // The final scripted box IS the closing recurrence; the queue is
                // exhausted right after it, so cap with an unreachable refusal.
                BranchStep::Switch(SwitchReport {
                    step: refuse_ok(step_at(pair.first_seed, BranchGerm::Regular, incidence, 3)),
                    outgoing: Some(refuse_ok(coordinate(3))),
                })
            });

        let outcome = trace_branch(&seed, domain, &mut certifier);
        let steps = match outcome {
            TraceOutcome::ClosedLoop { steps } => steps,
            _ => fail("expected ClosedLoop from the fixture's closed branch"),
        };
        assert!(!steps.is_empty(), "a closed loop traces steps");
        let first = &steps[0];
        let closing = &steps[steps.len() - 1];
        assert_eq!(
            closing.chart_box(),
            first.chart_box(),
            "identity recurrence: closing box id equals the first box id"
        );
        assert_eq!(
            steps.len(),
            13,
            "twelve geometric boxes plus the recurrence"
        );
        for step in &steps {
            assert_step_carries_incidence(step, &incidence);
            assert_eq!(step.germ(), BranchGerm::Regular);
        }
        // The synthetic walk really is the fixture's branch: every box center is
        // on the stored zero set.
        for step in &steps {
            let box_ = step.chart_box();
            let center = (
                0.5 * (box_[0].0 + box_[0].1),
                0.5 * (box_[1].0 + box_[1].1),
                0.5 * (box_[2].0 + box_[2].1),
                0.5 * (box_[3].0 + box_[3].1),
            );
            assert!(
                point_is_on_branch(&pair.system, center),
                "the traced box center lies on the fixture branch"
            );
        }
    }

    #[test]
    fn trace_loop_terminates_at_domain_boundary() {
        let well = refuse_ok(ssi_fixtures::well_conditioned_root());
        let incidence = ssi_fixtures::sample_trace_incidence();
        let Some(seed) = seed_certificate() else {
            fail("seed certificate refused");
        };
        let domain = chart_domain(&well.system);
        let mut certifier = ScriptedCertifier::from_geometry(boundary_steps(incidence), {
            BranchStep::Switch(SwitchReport {
                step: refuse_ok(step_at(
                    branch_point(1.05),
                    BranchGerm::Regular,
                    incidence,
                    3,
                )),
                outgoing: Some(refuse_ok(coordinate(3))),
            })
        });

        let outcome = trace_branch(&seed, domain, &mut certifier);
        let steps = match outcome {
            TraceOutcome::Terminated { steps } => steps,
            _ => fail("expected Terminated when the branch leaves the domain"),
        };
        assert!(!steps.is_empty(), "a domain exit traces in-domain steps");
        for step in &steps {
            assert!(
                step.chart_box().iter().zip(domain.iter()).all(
                    |((lo, hi), (domain_lo, domain_hi))| { lo >= domain_lo && hi <= domain_hi }
                ),
                "every terminated step lies inside the chart domain"
            );
            assert_step_carries_incidence(step, &incidence);
            let box_ = step.chart_box();
            let center = (
                0.5 * (box_[0].0 + box_[0].1),
                0.5 * (box_[1].0 + box_[1].1),
                0.5 * (box_[2].0 + box_[2].1),
                0.5 * (box_[3].0 + box_[3].1),
            );
            assert!(
                point_is_on_branch(&well.system, center),
                "the traced box center lies on the fixture branch"
            );
        }
    }

    #[test]
    fn trace_switch_requires_both_certificates_and_refuses_otherwise() {
        let pair = refuse_ok(ssi_fixtures::closed_loop_pair());
        let incidence = ssi_fixtures::sample_trace_incidence();
        let Some(seed) = seed_certificate() else {
            fail("seed certificate refused");
        };
        let domain = chart_domain(&pair.system);

        // Both certificates: the branch is traced under the `s` coordinate
        // (index 2) from the top of the circle to the rightmost point, where s
        // is maximal and the branch must turn into the `t` coordinate (index 3).
        let mut advance_steps = Vec::new();
        for k in 0..3 {
            let theta = std::f64::consts::FRAC_PI_2 - k as f64 * std::f64::consts::FRAC_PI_6;
            advance_steps.push(refuse_ok(step_at(
                circle_point(pair.center, pair.radius, theta),
                BranchGerm::Regular,
                incidence,
                2,
            )));
        }
        let switch_point = circle_point(pair.center, pair.radius, 0.0);
        let switch_step = refuse_ok(step_at(switch_point, BranchGerm::Regular, incidence, 3));
        let outgoing = refuse_ok(coordinate(2));
        let incoming = switch_step.coordinate();
        let mut certifier = ScriptedCertifier::from_geometry(advance_steps.clone(), {
            BranchStep::Switch(SwitchReport {
                step: switch_step,
                outgoing: Some(outgoing),
            })
        });
        let outcome = trace_branch(&seed, domain, &mut certifier);
        let (steps, switch) = match outcome {
            TraceOutcome::Switched { steps, switch } => (steps, switch),
            _ => fail("expected Switched when the switch reports both certificates"),
        };
        assert_eq!(
            steps.len(),
            advance_steps.len() + 1,
            "steps up to and including the switch box"
        );
        assert_eq!(switch.outgoing.index, outgoing.index, "outgoing coordinate");
        assert_eq!(switch.incoming.index, incoming.index, "incoming coordinate");
        assert_eq!(
            switch,
            CoordinateSwitch { outgoing, incoming },
            "frozen switch"
        );
        let last = &steps[steps.len() - 1];
        assert_eq!(
            last.chart_box(),
            switch_step.chart_box(),
            "switch box included"
        );
        assert_eq!(
            last.coordinate().index,
            3,
            "switch box under the incoming coordinate"
        );
        assert!(
            point_is_on_branch(&pair.system, switch_point),
            "the switch box lies on the fixture branch"
        );

        // One certificate only: the same switch point reported without the
        // outgoing certificate is a named refusal, never a reseed or a default.
        let mut certifier = ScriptedCertifier::refusing(BranchStep::Switch(SwitchReport {
            step: refuse_ok(step_at(switch_point, BranchGerm::Regular, incidence, 3)),
            outgoing: None,
        }));
        let outcome = trace_branch(&seed, domain, &mut certifier);
        match outcome {
            TraceOutcome::Refused(TraceRefusal::Conditioning(
                Refusal::ConditioningBelowThreshold,
            )) => {}
            other => fail(&format!(
                "a one-certificate switch must refuse conditioning, got {other:?}"
            )),
        }
    }

    #[test]
    fn trace_germ_classification_reads_next_nonzero_jet() {
        let ladder = refuse_ok(ssi_fixtures::germ_ladder());
        assert_eq!(ladder.len(), 5, "one fixture per BranchGerm variant");
        for fixture in &ladder {
            let classified =
                classify_branch_germ(&fixture.system, fixture.chart_box, fixture.event);
            assert_eq!(
                classified, fixture.germ,
                "the germ ladder rung classifies to its documented germ"
            );
        }
        // The span.rs discipline is visible on the ladder: the regular and
        // stationary rungs share a nonzero first jet in `v`, and the stationary
        // rung's `u` first jet vanishes (its ordinate has a second-order
        // stationary point).
        let regular = &ladder[0];
        assert_eq!(regular.germ, BranchGerm::Regular);
        let stationary = &ladder[1];
        assert_eq!(
            stationary.germ,
            BranchGerm::StationaryRegular {
                first_nonzero_order: 2
            }
        );
        assert!(stationary.event_is_interior());
        assert!(
            !ladder[4].event_is_interior(),
            "the unresolved event is a boundary event"
        );
    }

    #[test]
    fn trace_steps_carry_branch_incidence_records() {
        let pair = refuse_ok(ssi_fixtures::closed_loop_pair());
        let incidence = ssi_fixtures::sample_trace_incidence();
        let Some(seed) = seed_certificate() else {
            fail("seed certificate refused");
        };
        let domain = chart_domain(&pair.system);
        let mut certifier =
            ScriptedCertifier::from_geometry(closed_loop_steps(&pair, incidence), {
                BranchStep::Switch(SwitchReport {
                    step: refuse_ok(step_at(pair.first_seed, BranchGerm::Regular, incidence, 3)),
                    outgoing: Some(refuse_ok(coordinate(3))),
                })
            });
        let outcome = trace_branch(&seed, domain, &mut certifier);
        let steps = match outcome {
            TraceOutcome::ClosedLoop { steps } => steps,
            _ => fail("expected ClosedLoop"),
        };
        assert!(!steps.is_empty());
        for step in &steps {
            // Span + certified parameter enclosure + germ + deck label.
            assert_step_carries_incidence(step, &incidence);
            assert_eq!(step.incidence().span_id, incidence.span_id);
            assert_eq!(step.incidence().parameter, incidence.parameter);
            assert_eq!(step.incidence().deck, incidence.deck);
            assert_eq!(step.incidence().germ, step.germ());
        }
    }

    #[test]
    fn trace_refusals_are_named_cases() {
        let Some(seed) = seed_certificate() else {
            fail("seed certificate refused");
        };
        // A refusal-free placeholder domain is only reached by certifiers whose
        // first step already refuses, so any finite domain works.
        let domain = [(0.0, 1.0); 4];

        // The three TraceRefusal families the loop can emit, each wrapping a
        // landed named cause. The match below has no catch-all arm: it compiles
        // only because every TraceRefusal variant is a named case.
        let one_certificate_refusal = {
            let mut certifier = ScriptedCertifier::refusing(BranchStep::Switch(SwitchReport {
                step: refuse_ok(step_at(
                    branch_point(0.5),
                    BranchGerm::Regular,
                    ssi_fixtures::sample_trace_incidence(),
                    1,
                )),
                outgoing: None,
            }));
            match trace_branch(&seed, domain, &mut certifier) {
                TraceOutcome::Refused(refusal) => refusal,
                _ => fail("the one-certificate switch must refuse"),
            }
        };
        match one_certificate_refusal {
            TraceRefusal::Conditioning(Refusal::ConditioningBelowThreshold) => {}
            other => fail(&format!("expected the conditioning cause, got {other:?}")),
        }

        let scenarios: Vec<(&str, TraceRefusal)> = vec![
            (
                "trace_refused_conditioning",
                TraceRefusal::Conditioning(Refusal::ConditioningBelowThreshold),
            ),
            (
                "trace_refused_hull_enclosure_unavailable",
                TraceRefusal::Hull(HullRefusal::EnclosureUnavailable),
            ),
            (
                "unresolved_clustered_roots",
                TraceRefusal::Unresolved(GenericUnresolved::ClusteredRoots),
            ),
        ];
        for (expected_tag, refusal) in scenarios {
            let mut certifier = RefusingCertifier { refusal };
            let outcome = trace_branch(&seed, domain, &mut certifier);
            let produced = match outcome {
                TraceOutcome::Refused(refusal) => refusal,
                _ => fail("a refusing certifier must yield Refused"),
            };
            let tag = match produced {
                TraceRefusal::Conditioning(cause) => {
                    match cause {
                        Refusal::ConditioningBelowThreshold => {}
                        Refusal::InvalidInput => {}
                        Refusal::Unfrozen => {}
                    }
                    produced.tag()
                }
                TraceRefusal::Hull(cause) => {
                    match cause {
                        HullRefusal::EnclosureUnavailable => {}
                        HullRefusal::DomainNotCompact => {}
                    }
                    produced.tag()
                }
                TraceRefusal::Unresolved(cause) => {
                    let _ = cause.tag();
                    produced.tag()
                }
            };
            assert_eq!(tag, expected_tag, "refusals keep their stable named tags");
        }
    }
}
