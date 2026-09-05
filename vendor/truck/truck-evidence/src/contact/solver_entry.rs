//! CL-006-SOLVER-ENTRY — the restricted solver entry.
//!
//! The dependency-inverted seam of the Carrier Lift program: the certified
//! restricted-pair engine (BIE-002's `Ssi4System`/`krawczyk` machinery) is a
//! `truck-certified` construction that the boolean crates cannot name —
//! `truck-evidence` and `truck-shapeops` sit upstream of `truck-certified` in
//! the dependency direction, so the funnel (the Contact Layer dispatcher) could
//! never reach the certified engine and answered every restricted sweep pair
//! with the typed [`NumericallyUnresolved`](crate::outcome::Refusal)
//! shortcut. This module inverts the entry:
//!
//! - [`RestrictedSolverEntry`] — the pure interface the funnel needs. Given a
//!   sweep stratum against a canonical stratum (a restricted pair), it returns
//!   the certified outcome vocabulary BIE-000 froze: certified chart samples /
//!   typed `Unresolved { κ, cell, slope }` / landed refusal.
//! - the set-once runtime registry ([`set_restricted_solver`] /
//!   [`take_restricted_solver`] / [`dispatch_restricted_sweep`]). Registration
//!   is an explicit call, never a cargo feature and never a `#[ctor]`; the
//!   certified crate registers at its init (the kernel's entry point wires it).
//!   With no engine registered the funnel answers the SAME typed
//!   `NumericallyUnresolved` as before this packet (the registry-flip test pins
//!   both arms).
//!
//! The trait returns the landed outcome vocabulary only (H-2): the mirror
//! types below are plain parameter-cell / sample records on the
//! `truck-evidence` side — the certified engine's `WitnessCell`/`ChartSample`
//! are `truck-certified` types this crate cannot name, so the impl in
//! `truck-certified` adapts its landed `CertifiedChartCurve` output onto them.
//! Zero new `Refusal` arms (the standing rule).
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, no `panic!`, and no
//! out-of-range indexing (the crate-level `deny` covers it). A poisoned
//! registry lock degrades to "no engine registered", never a panic.

use super::BoundedStratum;
use std::sync::Mutex;
use truck_base::cgmath64::Point3;
use truck_base::evidence::{Budget, Certificate, Refusal};

/// A `(u, v) × (s, t)` parameter-cell record: the four scalar parameter
/// intervals of the product-domain cell the restricted solver bisects and
/// reports when it cannot certify an answer (the engine-side `WitnessCell`
/// mirror; the labels name the two carriers' parameter boxes in a fixed order).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParameterCell {
    /// The A-carrier first parameter interval.
    pub u: (f64, f64),
    /// The A-carrier second parameter interval.
    pub v: (f64, f64),
    /// The B-carrier first parameter interval.
    pub s: (f64, f64),
    /// The B-carrier second parameter interval.
    pub t: (f64, f64),
}

/// One certified sample of an interaction branch (the engine-side
/// `ChartSample` mirror): the certified 4-D parameter cell (the certified
/// statement), the float chart centre and the float model point (diagnostics,
/// H-6: the cell is the certified statement), and the Krawczyk certificate of
/// the cell.
#[derive(Clone, Debug)]
pub struct ChartSample {
    /// The certified 4-D parameter cell: it contains exactly one solution of
    /// the localized system — one certified sample of the branch.
    pub cell: ParameterCell,
    /// The float chart centre of the cell (diagnostics; the cell is the
    /// certified statement).
    pub chart: [f64; 4],
    /// The model point at the cell centre (float diagnostics, H-6).
    pub centre: Point3,
    /// The Krawczyk certificate of the cell.
    pub certificate: Certificate,
}

/// A certified interaction-branch solve (the engine-side
/// `CertifiedChartCurve` mirror): the certified chart samples plus the solve
/// certificate (its `budget_left` is the spent bookkeeping the funnel records).
#[derive(Clone, Debug)]
pub struct CertifiedChart {
    /// Ordered certified samples along the branch.
    pub samples: Vec<ChartSample>,
    /// The solve certificate (interval method; `budget_left` after the solve).
    pub certificate: Certificate,
}

/// The restricted outcome vocabulary the funnel consumes (BIE-000 shape,
/// mapped onto the landed evidence taxonomy — zero new `Refusal` arms).
#[derive(Clone, Debug)]
pub enum RestrictedSolve {
    /// A certified interaction-branch solve with certified chart samples.
    Certified(CertifiedChart),
    /// The typed unresolved verdict with the κ / cell / slope witness of the
    /// engine (BIE-000's `Unresolved { κ, cell, slope }`).
    Unresolved {
        /// The conditioning / curvature witness that kept the cell from a
        /// certified answer (the restricted-pair diagnostic).
        kappa: f64,
        /// The `(u, v) × (s, t)` parameter cell that stayed unresolved.
        cell: ParameterCell,
        /// The §5.4 slope diagnostic of the unresolved cell.
        slope: f64,
        /// What was spent before the engine gave up.
        spent: Budget,
    },
    /// A landed typed refusal, passed through unchanged.
    Refused(Refusal),
}

/// The dependency-inverted entry the funnel dispatches through: given the
/// restricted sweep pair (one [`BoundedStratum::Sweep`] stratum against a
/// canonical stratum), return the certified outcome vocabulary.
///
/// The impl lives in `truck-certified` (it adapts the landed
/// `CertifiedChartCurve` output of BIE-002's restricted engine); the funnel
/// never names the engine. A pair the impl cannot express as a restricted
/// solve answers [`RestrictedSolve::Refused`] with the typed
/// `NumericallyUnresolved` default, never a guess.
pub trait RestrictedSolverEntry: Send + Sync {
    /// Certifies one restricted sweep pair and returns the certified outcome
    /// vocabulary. Total over the pair: every outcome is typed, never a panic.
    fn certify_sweep_pair(
        &self,
        lhs: &BoundedStratum,
        rhs: &BoundedStratum,
        budget: &mut Budget,
    ) -> RestrictedSolve;
}

/// The registry slot refused a registration (a solver was already set, or the
/// lock was poisoned). Registration is set-once (determinism).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RestrictedSolverSetError;

/// The process-wide set-once registry slot of the restricted solver entry.
///
/// Registration is runtime-explicit (never a cargo feature, never a `#[ctor]`).
/// The slot is empty until the certified crate registers at its init; with an
/// empty slot the funnel answers the same typed `NumericallyUnresolved` as
/// before this packet.
static REGISTRY: Mutex<Option<Box<dyn RestrictedSolverEntry>>> = Mutex::new(None);

/// Whether a restricted solver is currently registered.
pub fn registered() -> bool {
    match REGISTRY.lock() {
        Ok(guard) => guard.is_some(),
        Err(_) => false,
    }
}

/// Sets the restricted solver entry, once.
///
/// A second registration while the slot is occupied refuses
/// ([`RestrictedSolverSetError`]); the caller is expected to [`take`] a stale
/// entry before re-registering (determinism: set-once semantics).
///
/// [`take`]: take_restricted_solver
pub fn set_restricted_solver(
    entry: impl RestrictedSolverEntry + 'static,
) -> Result<(), RestrictedSolverSetError> {
    match REGISTRY.lock() {
        Ok(mut guard) => {
            if guard.is_some() {
                return Err(RestrictedSolverSetError);
            }
            *guard = Some(Box::new(entry));
            Ok(())
        }
        Err(_) => Err(RestrictedSolverSetError),
    }
}

/// Empties the registry slot, returning whether an entry was present.
///
/// This is the explicit clearing arm of the registry (the registry-flip test
/// uses it to restore the no-engine baseline). A cleared slot can be filled
/// again by [`set_restricted_solver`].
pub fn take_restricted_solver() -> bool {
    match REGISTRY.lock() {
        Ok(mut guard) => guard.take().is_some(),
        Err(_) => false,
    }
}

/// Dispatches one restricted sweep pair through the registered solver.
///
/// Returns `None` when no engine is registered (or the lock is poisoned): the
/// funnel then answers today's typed `NumericallyUnresolved` — the
/// registry-flip baseline arm.
pub fn dispatch_restricted_sweep(
    lhs: &BoundedStratum,
    rhs: &BoundedStratum,
    budget: &mut Budget,
) -> Option<RestrictedSolve> {
    match REGISTRY.lock() {
        Ok(guard) => guard
            .as_deref()
            .map(|entry| entry.certify_sweep_pair(lhs, rhs, budget)),
        Err(_) => None,
    }
}
