//! CFP-002-INSTRUMENT (certified side): the process-local instrument counters
//! of the spline-carrier admission path.
//!
//! F1 forbids a shared counter implementation between `truck-evidence` and
//! `truck-certified`, so each crate lands its own counter struct whose field
//! names and JSON key order are frozen at the CFP spine
//! (`crate::cfp::spine`, decision 5). This module is the certified crate's
//! instance: the canonical [`InstrumentCounters`] record is reused for the
//! frozen-schema snapshot (it is this crate's own type), backed by a
//! process-global gate (default OFF, enabled once by the `TRUCK_CFP_INSTRUMENT`
//! environment variable) and the two hooks this packet owns here — the
//! knot-span distribution and the composed-bidegree distribution, both fed from
//! [`patch_admit::admit_surface`](crate::patch_admit::admit_surface) per
//! admitted carrier.
//!
//! **Gated recording, zero overhead when off.** A record hook is a single
//! relaxed atomic load plus a branch when the gate is closed: no allocation, no
//! lock, and no change to any verdict (counters on or off produce identical
//! verdicts on identical input). When the gate is open the hook locks the
//! process-global record and accumulates keyed bins — the instrumentation cost
//! only exists in instrumented runs.
//!
//! **Counter semantics.** Every schema scalar counts the events recorded for
//! that counter (the histograms' population — a fresh process is all zeros);
//! the per-key distributions are read from [`knot_span_distribution`] and
//! [`bidegree_distribution`]. `cells_visited`, `distinct_side_boxes`, and
//! `unresolved_provenance` ride the same hook after CFP-001 — their increment
//! sites do not exist yet, so they are declared and left at zero, never stubbed
//! with fake increments.

use crate::cfp::InstrumentCounters;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::OnceLock;

/// The environment variable that enables the instrument counters (read once).
const ENV_INSTRUMENT: &str = "TRUCK_CFP_INSTRUMENT";

/// The keyed distribution state behind the admit-path counters.
///
/// A plain value type so the counting logic is unit-testable without the
/// process-global gate; the production record path is the gated
/// [`record_admit`] entry point.
#[derive(Debug, Default)]
pub struct AdmitHistograms {
    /// The histogram population: the number of admitted carriers recorded
    /// (each admit contributes one knot-span record and one bidegree record).
    admits: u64,
    /// Per-carrier knot-span-cell counts (`span cells → number of carriers`).
    /// The key is the number of knot-span cells of the admitted carrier (its
    /// Bézier patch count — the CFP-005 span-BVH leaf count), in deterministic
    /// ascending order.
    knot_span_bins: BTreeMap<u64, u64>,
    /// Per-carrier composed-bidegree records (`(du, dv) → number of carriers`),
    /// the D3 revisit input, keyed in deterministic ascending order. Every
    /// recorded bidegree lies within the admission budget
    /// ([`MAX_BIDEGREE`](crate::patch_admit::MAX_BIDEGREE)).
    bidegree_bins: BTreeMap<(u64, u64), u64>,
}

impl AdmitHistograms {
    /// Record one admitted carrier of `span_cells` knot-span cells and bidegree
    /// `(du, dv)`. Infallible (saturating counters; never a panic on the
    /// geometry path).
    pub fn record_admit(&mut self, span_cells: u64, du: u64, dv: u64) {
        self.admits = self.admits.saturating_add(1);
        let span_slot = self.knot_span_bins.entry(span_cells).or_insert(0u64);
        *span_slot = span_slot.saturating_add(1);
        let degree_slot = self.bidegree_bins.entry((du, dv)).or_insert(0u64);
        *degree_slot = degree_slot.saturating_add(1);
    }

    /// The number of admitted-carrier records (the histogram population).
    pub fn admits(&self) -> u64 {
        self.admits
    }

    /// Whether no admit has been recorded.
    pub fn is_empty(&self) -> bool {
        self.admits == 0
    }

    /// The number of recorded carriers with exactly `span_cells` knot-span
    /// cells.
    pub fn knot_span_bin(&self, span_cells: u64) -> u64 {
        self.knot_span_bins.get(&span_cells).copied().unwrap_or(0)
    }

    /// The number of recorded carriers of admitted bidegree `(du, dv)`.
    pub fn bidegree_bin(&self, du: u64, dv: u64) -> u64 {
        self.bidegree_bins.get(&(du, dv)).copied().unwrap_or(0)
    }

    /// The knot-span-cell distribution, in ascending cell-count order: each
    /// entry is `(span cells, carrier count)`.
    pub fn knot_span_distribution(&self) -> Vec<(u64, u64)> {
        self.knot_span_bins.iter().map(|(&k, &v)| (k, v)).collect()
    }

    /// The composed-bidegree distribution, in ascending degree order: each
    /// entry is `((du, dv), carrier count)`.
    pub fn bidegree_distribution(&self) -> Vec<((u64, u64), u64)> {
        self.bidegree_bins.iter().map(|(&k, &v)| (k, v)).collect()
    }

    /// The frozen-schema snapshot: both owned scalars carry the histogram
    /// population (every admit records one knot-span datum and one bidegree
    /// datum), and the three post-CFP-001 scalars stay at zero.
    pub fn snapshot(&self) -> InstrumentCounters {
        InstrumentCounters::new(self.admits, 0, self.admits, 0, 0, 0)
    }
}

/// The process-global admit-path record, present only once recording begins.
static LIVE: Mutex<Option<AdmitHistograms>> = Mutex::new(None);

/// The test-only gate override, kept in its own `#[cfg(test)]` module so a
/// non-test build carries neither the static nor the import.
#[cfg(test)]
mod test_gate {
    use std::sync::atomic::{AtomicBool, Ordering};

    /// When true, every record hook no-ops regardless of the environment.
    pub(super) static FORCE_OFF: AtomicBool = AtomicBool::new(false);

    /// Force the gate closed and drop any recorded state, so a test observes a
    /// fresh process deterministically.
    pub(super) fn force_off_and_clear() {
        FORCE_OFF.store(true, Ordering::Relaxed);
        if let Ok(mut guard) = super::LIVE.lock() {
            *guard = None;
        }
    }
}

/// Whether the environment enables instrumentation.
fn env_instrument_on() -> bool {
    std::env::var(ENV_INSTRUMENT).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "on" | "yes"
        )
    })
}

/// Whether the instrument gate is open. Read once from `TRUCK_CFP_INSTRUMENT`
/// on first use and cached, so the hot-path hook is one atomic load plus a
/// branch when the gate is closed.
fn instrument_enabled() -> bool {
    #[cfg(test)]
    if test_gate::FORCE_OFF.load(std::sync::atomic::Ordering::Relaxed) {
        return false;
    }
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(env_instrument_on)
}

/// Run `f` against the process-global record, creating it on first use.
fn with_live(f: impl FnOnce(&mut AdmitHistograms)) {
    if let Ok(mut guard) = LIVE.lock() {
        let live = guard.get_or_insert_with(AdmitHistograms::default);
        f(live);
    }
}

/// Record one admitted carrier (gated): `span_cells` knot-span cells and
/// bidegree `(du, dv)` — the `patch_admit::admit_surface` hook. No-ops when
/// instrumentation is off. Infallible.
pub fn record_admit(span_cells: usize, du: usize, dv: usize) {
    if !instrument_enabled() {
        return;
    }
    with_live(|live| live.record_admit(span_cells as u64, du as u64, dv as u64));
}

/// The frozen-schema snapshot of the process-global admit record (all zeros
/// when nothing was recorded — the fresh-process baseline).
pub fn snapshot() -> InstrumentCounters {
    match LIVE.lock() {
        Ok(guard) => match guard.as_ref() {
            Some(live) => live.snapshot(),
            None => InstrumentCounters::default(),
        },
        Err(_) => InstrumentCounters::default(),
    }
}

/// The process-global knot-span-cell distribution, in ascending cell-count
/// order: each entry is `(span cells, carrier count)`.
pub fn knot_span_distribution() -> Vec<(u64, u64)> {
    match LIVE.lock() {
        Ok(guard) => match guard.as_ref() {
            Some(live) => live.knot_span_distribution(),
            None => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

/// The process-global composed-bidegree distribution, in ascending degree
/// order: each entry is `((du, dv), carrier count)`.
pub fn bidegree_distribution() -> Vec<((u64, u64), u64)> {
    match LIVE.lock() {
        Ok(guard) => match guard.as_ref() {
            Some(live) => live.bidegree_distribution(),
            None => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch_admit::{admit_surface, MAX_BIDEGREE};
    use truck_geometry::prelude::{BSplineSurface, KnotVec, Vector4};

    /// A clamped uniform homogeneous carrier: `degree` in both axes, `div_u` ×
    /// `div_v` knot spans, planar control net with unit weights (D2).
    fn uniform_surface(degree: usize, div_u: usize, div_v: usize) -> BSplineSurface<Vector4> {
        let uknot = KnotVec::uniform_knot(degree, div_u);
        let vknot = KnotVec::uniform_knot(degree, div_v);
        let n_u = uknot.len() - degree - 1;
        let n_v = vknot.len() - degree - 1;
        let ctrl = (0..n_u)
            .map(|i| {
                (0..n_v)
                    .map(|j| {
                        Vector4::new(i as f64 * 0.25, j as f64 * 0.25, (i * j) as f64 * 0.1, 1.0)
                    })
                    .collect()
            })
            .collect();
        BSplineSurface::new((uknot, vknot), ctrl)
    }

    #[test]
    fn counter_increment_knot_span_distribution() {
        // A fresh process (gate forced off, recorded state cleared): a
        // multi-span admit pass records nothing through the process-global
        // hook, and the frozen scalars stay zero.
        test_gate::force_off_and_clear();

        // A clamped degree-3 carrier with 2 u-spans × 3 v-spans admits to 6
        // knot-span cells (Bézier patches).
        let surface = uniform_surface(3, 2, 3);
        let stack = admit_surface(&surface).expect("a clamped uniform carrier admits");
        assert_eq!(stack.len(), 6, "2 × 3 knot spans decompose into 6 cells");
        assert_eq!(
            snapshot().values(),
            [0, 0, 0, 0, 0, 0],
            "the closed gate records nothing on the admit pass"
        );

        // The record path captures the span count as the key of the
        // distribution (exactly the value the admit hook records).
        let mut hist = AdmitHistograms::default();
        hist.record_admit(
            stack.len() as u64,
            surface.udegree() as u64,
            surface.vdegree() as u64,
        );
        assert_eq!(hist.knot_span_bin(6), 1);
        assert_eq!(hist.knot_span_distribution(), vec![(6, 1)]);
        assert_eq!(hist.snapshot().knot_span_count(), 1);
        assert_eq!(hist.admits(), 1);
        assert!(!hist.is_empty());
    }

    #[test]
    fn counter_increment_composed_bidegree() {
        test_gate::force_off_and_clear();

        // The recorded bidegree is the admitted carrier's own bidegree — the
        // per-side factor of the composed 4-axis system — against MAX_BIDEGREE.
        assert_eq!(MAX_BIDEGREE, 8, "the D3 budget constant is 8");

        let mut hist = AdmitHistograms::default();

        // A multi-span degree-(3, 3) carrier records the (3, 3) bin.
        let cubic = uniform_surface(3, 2, 2);
        let cubic_stack = admit_surface(&cubic).expect("a clamped bicubic carrier admits");
        assert_eq!(cubic_stack.len(), 4);
        hist.record_admit(
            cubic_stack.len() as u64,
            cubic.udegree() as u64,
            cubic.vdegree() as u64,
        );
        assert_eq!(hist.bidegree_bin(3, 3), 1);

        // A carrier at the top of the budget (degree (8, 8) = MAX_BIDEGREE)
        // still admits and records its bidegree against the budget.
        let top = uniform_surface(MAX_BIDEGREE, 1, 1);
        let top_stack = admit_surface(&top).expect("a carrier at the degree budget admits");
        assert_eq!(top_stack.len(), 1);
        hist.record_admit(
            top_stack.len() as u64,
            top.udegree() as u64,
            top.vdegree() as u64,
        );
        assert_eq!(hist.bidegree_bin(8, 8), 1);

        // The distribution holds both records in deterministic order and the
        // frozen scalar is the histogram population.
        assert_eq!(hist.bidegree_distribution(), vec![((3, 3), 1), ((8, 8), 1)]);
        assert_eq!(hist.snapshot().composed_bidegree(), 2);
    }
}
