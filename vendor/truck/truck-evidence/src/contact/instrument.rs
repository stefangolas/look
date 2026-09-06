//! CFP-002-INSTRUMENT (evidence side): the process-local instrument counters of
//! the contact funnel.
//!
//! F1 forbids a shared counter implementation between `truck-evidence` and
//! `truck-certified`, so each crate lands its own counter struct whose field
//! names and JSON key order are frozen at the CFP spine
//! (`truck-certified/src/cfp/spine.rs`, decision 5). This module is the
//! evidence crate's instance: a schema-identical [`InstrumentCounters`] mirror,
//! a process-global gate (default OFF, enabled once by the `TRUCK_CFP_INSTRUMENT`
//! environment variable), and the one hook this packet owns here — the stage-5
//! pair-class histogram [`record_stage5_pair_class`] fed at the funnel's
//! validated-FF / torus-FF / spline-SSI dispatch sites in `contact/mod.rs` (the
//! entries that reach the general validated FF / spline SSI arm).
//!
//! **Gated recording, zero overhead when off.** A record hook is a single
//! relaxed atomic load plus a branch when the gate is closed: no allocation, no
//! lock, and no change to any verdict (counters on or off produce identical
//! verdicts on identical input). When the gate is open the hook locks the
//! process-global record and accumulates keyed bins — the instrumentation cost
//! only exists in instrumented runs.
//!
//! **The counter vocabulary (decision 5).** Every crate serializes the six
//! frozen keys in the frozen order; this crate's scalar
//! [`InstrumentCounters::stage5_pair_class`] is the histogram population (the
//! number of stage-5 entries recorded); the other five scalars stay at zero —
//! their increment sites belong to later packets and are never stubbed here.
//! The per-class detail is read from [`stage5_pair_class_count`], and the pair
//! classes are keyed in the fixed vocabulary order
//! `Plane | Cylinder | Sphere | Cone | Torus | Spline | Sweep | Other`
//! (determinism: never hash iteration).

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use truck_geometry::recognize::CanonicalSurface;

/// The environment variable that enables the instrument counters (read once).
const ENV_INSTRUMENT: &str = "TRUCK_CFP_INSTRUMENT";

/// The frozen instrument counter names, in the frozen JSON key order (the CFP
/// spine's `SCHEMA_KEYS`, verbatim; decision 5).
pub const SCHEMA_KEYS: [&str; 6] = [
    "knot_span_count",
    "stage5_pair_class",
    "composed_bidegree",
    "cells_visited",
    "distinct_side_boxes",
    "unresolved_provenance",
];

/// The schema-identical snapshot of this crate's instrument counters (decision
/// 5): six `u64` counters with the field names and JSON key order frozen at the
/// CFP spine.
///
/// This is the evidence crate's local mirror of the spine's canonical
/// `InstrumentCounters` (F1 forbids the cross-crate dependency). It is
/// diagnostics data only — never evidence, never serialized into a
/// `Certificate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InstrumentCounters {
    /// Knot-span distribution on admitted carriers (owned by `truck-certified`).
    knot_span_count: u64,
    /// Pair-type histogram count at stage-5 entry (this crate's owned counter).
    stage5_pair_class: u64,
    /// Composed-degree-vs-budget record (owned by `truck-certified`).
    composed_bidegree: u64,
    /// Cells visited (rides the CFP-001 hook; not yet incremented).
    cells_visited: u64,
    /// Distinct side boxes (rides the CFP-001 hook; not yet incremented).
    distinct_side_boxes: u64,
    /// Unresolved-provenance split (rides the CFP-001 hook; not yet
    /// incremented).
    unresolved_provenance: u64,
}

impl InstrumentCounters {
    /// Build a counter record from the six counters in the frozen field order.
    pub const fn new(
        knot_span_count: u64,
        stage5_pair_class: u64,
        composed_bidegree: u64,
        cells_visited: u64,
        distinct_side_boxes: u64,
        unresolved_provenance: u64,
    ) -> Self {
        Self {
            knot_span_count,
            stage5_pair_class,
            composed_bidegree,
            cells_visited,
            distinct_side_boxes,
            unresolved_provenance,
        }
    }

    /// The frozen counter names in the frozen JSON key order.
    pub fn schema_keys() -> &'static [&'static str; 6] {
        &SCHEMA_KEYS
    }

    /// The counter values in the frozen field order.
    pub fn values(&self) -> [u64; 6] {
        [
            self.knot_span_count,
            self.stage5_pair_class,
            self.composed_bidegree,
            self.cells_visited,
            self.distinct_side_boxes,
            self.unresolved_provenance,
        ]
    }

    /// Serialize to the frozen JSON schema: one object whose keys appear in the
    /// frozen [`SCHEMA_KEYS`] order (identical shape to the spine's canonical
    /// serialization, so per-crate outputs concatenate into one stream).
    pub fn to_schema_json(&self) -> String {
        let mut out = String::from("{");
        for (idx, (key, value)) in SCHEMA_KEYS.iter().zip(self.values().iter()).enumerate() {
            if idx > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(key);
            out.push('"');
            out.push(':');
            out.push_str(&value.to_string());
        }
        out.push('}');
        out
    }

    /// The `knot_span_count` counter, verbatim.
    pub fn knot_span_count(&self) -> u64 {
        self.knot_span_count
    }

    /// The `stage5_pair_class` counter, verbatim.
    pub fn stage5_pair_class(&self) -> u64 {
        self.stage5_pair_class
    }

    /// The `composed_bidegree` counter, verbatim.
    pub fn composed_bidegree(&self) -> u64 {
        self.composed_bidegree
    }

    /// The `cells_visited` counter, verbatim.
    pub fn cells_visited(&self) -> u64 {
        self.cells_visited
    }

    /// The `distinct_side_boxes` counter, verbatim.
    pub fn distinct_side_boxes(&self) -> u64 {
        self.distinct_side_boxes
    }

    /// The `unresolved_provenance` counter, verbatim.
    pub fn unresolved_provenance(&self) -> u64 {
        self.unresolved_provenance
    }
}

/// The recognized carrier class of one side of a dispatched stage-5 pair, in
/// the frozen vocabulary order
/// `Plane | Cylinder | Sphere | Cone | Torus | Spline | Sweep | Other`.
///
/// This is the evidence-side routing key of the stage-5 pair-class histogram
/// (CFP-004 removes the `(Plane | quadric) × Spline` class from the 4-D
/// machinery, and `f` is that class's share of stage-5 entries). The vocabulary
/// is fixed here so pair-class keys never depend on hash iteration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CarrierKind {
    /// A plane (affine implicit).
    Plane,
    /// A cylinder (quadric implicit).
    Cylinder,
    /// A sphere (quadric implicit).
    Sphere,
    /// A cone (quadric implicit).
    Cone,
    /// A torus.
    Torus,
    /// A spline carrier (the admitted `BSplineSurface`).
    Spline,
    /// A sweep face stratum.
    Sweep,
    /// Any carrier outside the recognized analytic classes.
    Other,
}

impl CarrierKind {
    /// A short stable tag, for diagnostics and instrument records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Plane => "plane",
            Self::Cylinder => "cylinder",
            Self::Sphere => "sphere",
            Self::Cone => "cone",
            Self::Torus => "torus",
            Self::Spline => "spline",
            Self::Sweep => "sweep",
            Self::Other => "other",
        }
    }

    /// The index of this class in the frozen vocabulary order, used to build
    /// the fixed-order pair key (no hash iteration, determinism rule).
    fn rank(self) -> usize {
        match self {
            Self::Plane => 0,
            Self::Cylinder => 1,
            Self::Sphere => 2,
            Self::Cone => 3,
            Self::Torus => 4,
            Self::Spline => 5,
            Self::Sweep => 6,
            Self::Other => 7,
        }
    }

    /// The recognized class of a canonical analytic surface. A `Placed`
    /// canonical carrier (an analytic carrier under an affine placement) is
    /// `Other`: its world-frame geometry is decided by the conjugation arms
    /// before any stage-5 entry.
    pub fn from_canonical(surface: &CanonicalSurface) -> Self {
        match surface {
            CanonicalSurface::Plane(_) => Self::Plane,
            CanonicalSurface::Cylinder(_) => Self::Cylinder,
            CanonicalSurface::Sphere(_) => Self::Sphere,
            CanonicalSurface::Cone(_) => Self::Cone,
            CanonicalSurface::Torus(_) => Self::Torus,
            CanonicalSurface::Placed(_) => Self::Other,
        }
    }
}

/// The keyed live counter state behind the stage-5 histogram.
///
/// A plain value type so the counting logic is unit-testable without the
/// process-global gate; the production record path is the gated
/// [`record_stage5_pair_class`] / [`record_stage5_canonical`] /
/// [`record_stage5_spline_analytic`] entry points.
#[derive(Debug, Default)]
pub struct Stage5Histogram {
    /// The histogram population: the number of stage-5 entries recorded.
    entries: u64,
    /// Per-pair-class bins, keyed by the canonicalized pair ranks in the fixed
    /// vocabulary order (sorted so `(a, b)` and `(b, a)` share a bin).
    bins: BTreeMap<(usize, usize), u64>,
}

impl Stage5Histogram {
    /// Record one stage-5 entry of the given carrier-class pair. Infallible
    /// (saturating counters; never a panic on the geometry path).
    pub fn record(&mut self, a: CarrierKind, b: CarrierKind) {
        let key = Self::pair_key(a, b);
        let slot = self.bins.entry(key).or_insert(0u64);
        *slot = slot.saturating_add(1);
        self.entries = self.entries.saturating_add(1);
    }

    /// The number of stage-5 entries recorded in the bin of the given (order
    /// insensitive) carrier-class pair.
    pub fn bin_count(&self, a: CarrierKind, b: CarrierKind) -> u64 {
        self.bins.get(&Self::pair_key(a, b)).copied().unwrap_or(0)
    }

    /// The number of stage-5 entries recorded across all pair classes.
    pub fn len(&self) -> u64 {
        self.entries
    }

    /// Whether no stage-5 entry has been recorded.
    pub fn is_empty(&self) -> bool {
        self.entries == 0
    }

    /// The frozen-schema snapshot of this histogram: the population lands in
    /// the `stage5_pair_class` field, every other scalar stays at zero (their
    /// increment sites do not exist yet).
    pub fn snapshot(&self) -> InstrumentCounters {
        InstrumentCounters::new(0, self.entries, 0, 0, 0, 0)
    }

    /// The canonical fixed-order key of a carrier-class pair: both arguments'
    /// ranks sorted, so the histogram is order-insensitive (a geometric pair
    /// arrives in either dispatch order) and never hash-ordered.
    fn pair_key(a: CarrierKind, b: CarrierKind) -> (usize, usize) {
        let (i, j) = (a.rank(), b.rank());
        if i <= j {
            (i, j)
        } else {
            (j, i)
        }
    }
}

/// The process-global stage-5 histogram, present only once recording begins.
static LIVE: Mutex<Option<Stage5Histogram>> = Mutex::new(None);

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

/// Run `f` against the process-global histogram, creating it on first use.
fn with_live(f: impl FnOnce(&mut Stage5Histogram)) {
    if let Ok(mut guard) = LIVE.lock() {
        let live = guard.get_or_insert_with(Stage5Histogram::default);
        f(live);
    }
}

/// Record one stage-5 entry of a carrier-class pair (gated).
///
/// No-ops when instrumentation is off; when on, increments the pair-class bin
/// and the `stage5_pair_class` population. Infallible.
pub fn record_stage5_pair_class(a: CarrierKind, b: CarrierKind) {
    if !instrument_enabled() {
        return;
    }
    with_live(|live| live.record(a, b));
}

/// Record one stage-5 entry of a canonical-analytic FF pair (gated): the
/// evidence funnel's validated-FF / torus-FF dispatch sites classify each side
/// through [`CarrierKind::from_canonical`].
pub fn record_stage5_canonical(a: &CanonicalSurface, b: &CanonicalSurface) {
    if !instrument_enabled() {
        return;
    }
    with_live(|live| {
        live.record(
            CarrierKind::from_canonical(a),
            CarrierKind::from_canonical(b),
        );
    });
}

/// Record one stage-5 entry of a spline × analytic pair (gated): the funnel's
/// spline-SSI dispatch site (`spline_analytic_contact`) classifies the analytic
/// side and tags the spline side.
pub fn record_stage5_spline_analytic(analytic: &CanonicalSurface) {
    if !instrument_enabled() {
        return;
    }
    with_live(|live| {
        live.record(CarrierKind::Spline, CarrierKind::from_canonical(analytic));
    });
}

/// The frozen-schema snapshot of the process-global stage-5 histogram (all
/// zeros when nothing was recorded — the fresh-process baseline).
pub fn snapshot() -> InstrumentCounters {
    match LIVE.lock() {
        Ok(guard) => match guard.as_ref() {
            Some(live) => live.snapshot(),
            None => InstrumentCounters::default(),
        },
        Err(_) => InstrumentCounters::default(),
    }
}

/// The per-pair-class bin count of the process-global stage-5 histogram.
pub fn stage5_pair_class_count(a: CarrierKind, b: CarrierKind) -> u64 {
    match LIVE.lock() {
        Ok(guard) => match guard.as_ref() {
            Some(live) => live.bin_count(a, b),
            None => 0,
        },
        Err(_) => 0,
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. Unit-test assertions on hand-built dyadic witnesses are
// not such a path; the unwraps below cannot fire for the values constructed.
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::contact::{contact, BoundedStratum};
    use crate::Budget;
    use std::f64::consts::FRAC_PI_4;
    use truck_geometry::prelude::*;

    #[test]
    fn counter_increment_stage5_pair_class() {
        // A synthetic stage-5 pair records into the fixed-order class key; the
        // pair is order-insensitive (both dispatch orders share a bin) and the
        // schema scalar is the histogram population.
        let mut hist = Stage5Histogram::default();
        assert!(hist.is_empty());
        hist.record(CarrierKind::Plane, CarrierKind::Spline);
        hist.record(CarrierKind::Spline, CarrierKind::Plane);
        hist.record(CarrierKind::Cylinder, CarrierKind::Spline);
        assert_eq!(hist.bin_count(CarrierKind::Plane, CarrierKind::Spline), 2);
        assert_eq!(hist.bin_count(CarrierKind::Spline, CarrierKind::Plane), 2);
        assert_eq!(
            hist.bin_count(CarrierKind::Cylinder, CarrierKind::Spline),
            1
        );
        assert_eq!(hist.bin_count(CarrierKind::Cylinder, CarrierKind::Cone), 0);
        assert_eq!(hist.len(), 3);

        // The snapshot carries the population in the frozen `stage5_pair_class`
        // slot and leaves every other field at zero.
        let counters = hist.snapshot();
        assert_eq!(counters.stage5_pair_class(), 3);
        assert_eq!(counters.knot_span_count(), 0);
        assert_eq!(counters.composed_bidegree(), 0);
        assert_eq!(counters.cells_visited(), 0);
        assert_eq!(counters.distinct_side_boxes(), 0);
        assert_eq!(counters.unresolved_provenance(), 0);

        // The recognized-class classification of canonical carriers uses the
        // same vocabulary keys.
        let sphere = CanonicalSurface::Sphere(Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0));
        let plane = CanonicalSurface::Plane(Plane::xy());
        assert!(matches!(
            CarrierKind::from_canonical(&sphere),
            CarrierKind::Sphere
        ));
        assert!(matches!(
            CarrierKind::from_canonical(&plane),
            CarrierKind::Plane
        ));
        assert_eq!(CarrierKind::Plane.tag(), "plane");
        assert_eq!(CarrierKind::Spline.tag(), "spline");
        assert_eq!(CarrierKind::Sweep.tag(), "sweep");
        assert_eq!(CarrierKind::Other.tag(), "other");
    }

    #[test]
    fn instrument_output_json_key_order_matches_spine() {
        // The per-crate output serializes in the spine's frozen key order,
        // exactly the canonical `InstrumentCounters` shape (decision 5).
        assert_eq!(
            InstrumentCounters::schema_keys(),
            &[
                "knot_span_count",
                "stage5_pair_class",
                "composed_bidegree",
                "cells_visited",
                "distinct_side_boxes",
                "unresolved_provenance"
            ]
        );
        let counters = InstrumentCounters::new(7, 1, 3, 12, 9, 0);
        assert_eq!(
            counters.to_schema_json(),
            "{\"knot_span_count\":7,\"stage5_pair_class\":1,\"composed_bidegree\":3,\
             \"cells_visited\":12,\"distinct_side_boxes\":9,\"unresolved_provenance\":0}"
        );
    }

    #[test]
    fn counters_are_zero_on_fresh_process() {
        // A fresh process (gate forced off, recorded state cleared): a full
        // contact pass over a pair that dispatches to the stage-5 validated-FF
        // arm records nothing, and every schema field stays zero.
        test_gate::force_off_and_clear();

        let cyl = BoundedStratum::Face {
            surface: CanonicalSurface::Cylinder(
                Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
                    .expect("a unit cylinder is a valid carrier")
                    .value,
            ),
            u_range: (0.8, 1.3),
            v_range: (0.8, 1.2),
        };
        let cone = BoundedStratum::Face {
            surface: CanonicalSurface::Cone(
                Cone::new(Point3::new(10.0, 0.0, 0.0), FRAC_PI_4)
                    .expect("a dyadic cone is a valid carrier")
                    .value,
            ),
            u_range: (0.0, std::f64::consts::PI),
            v_range: (0.8, 1.2),
        };
        let mut budget = Budget::new(4096, 0, 0);
        let out = contact(&cyl, &cone, &mut budget);
        assert!(
            out.is_ok(),
            "the offset cylinder×cone pair dispatches through the validated arm"
        );
        assert_eq!(
            snapshot().values(),
            [0, 0, 0, 0, 0, 0],
            "a fresh process with the gate closed records nothing"
        );

        // Direct record calls are equally gated: with the environment unset and
        // the gate closed they no-op.
        record_stage5_pair_class(CarrierKind::Plane, CarrierKind::Spline);
        record_stage5_canonical(
            &CanonicalSurface::Plane(Plane::xy()),
            &CanonicalSurface::Sphere(Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0)),
        );
        assert_eq!(
            snapshot().values(),
            [0, 0, 0, 0, 0, 0],
            "the closed gate leaves every schema field at zero"
        );
    }
}
