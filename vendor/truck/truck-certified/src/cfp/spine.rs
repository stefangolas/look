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

//! The CFP spine freeze (CFP-000-SPINE): the shapes, gates, instrument schema,
//! and slot decision every CFP packet builds against.
//!
//! This is a freeze in the P0-FREEZE pattern (`ssi_types.rs` precedent): only
//! the shared shapes land here, with refusing constructors and verbatim
//! accessors. Nothing here evaluates, solves, isolates, or certifies
//! numerically. The five later packets (CFP-001/003/004/006 and their serial
//! followers) build against this module and never restate it. The mathematics
//! is frozen in `docs/CONTACT_FAST_PATH_BUILD_SPEC.md` (Theorems 1-4, §3) and
//! the live enclosure defects it corrects
//! (`docs/defects/DSC-BOUNDARY-SAMPLE-EXTENT-001.md`,
//! `docs/defects/NUM-SPLINE-ENCLOSURE-CONVERGENCE-001.md`).
//!
//! **D-shim (scope decision 1, verbatim).** Types and refusing constructors
//! only. Any method that would evaluate, solve, isolate, or certify NUMERICALLY
//! refuses (`InvalidInput`-shaped or a named case from the existing
//! vocabularies). The module doc says verbatim: "This module freezes shapes;
//! the CFP wave packets implement against it and never restate it."
//!
//! **D-reuse.** The crate's landed types are the vocabulary: `contract::Refusal`
//! for every refusing-constructor error (a construction outside a frozen rule
//! is `InvalidInput`), `formal/numeric.rs`'s [`PositiveFinite`] for certified
//! positive scalars, and the base evidence algebra
//! (`truck_base::evidence::{Outcome, Refusal}`) for the certified outcome
//! wrapper and the landed refusal causes. The module wraps/aliases; it never
//! duplicates a landed type under a new name, and it adds no new top-level
//! evidence kind (mapping rows in `docs/CERTIFICATE_MAPPING.md` section C).
//!
//! **Frozen scope decisions (do not relitigate):**
//!
//! - **SFC (decision 2).** The search/certificate split is type-level:
//!   [`FloatHint<T>`] is a float search result carrying NO evidence status and
//!   cannot be constructed into an evidence position; the certificate side is
//!   the landed exact/interval vocabulary. Search in floats, certify exactly —
//!   unviolatable by construction.
//! - **Localizable refusals (decision 3).** [`LocalizedRefusal`] wraps a landed
//!   evidence [`Refusal`](truck_base::evidence::Refusal) verbatim beside its
//!   stratum pair ([`StratumSide::A`] index, [`StratumSide::B`] index). Zero new
//!   top-level evidence kinds; face re-attribution happens in `truck-shapeops`
//!   (CFP-001), never here.
//! - **Slot resolution (decision 4).** The `SplineSsiEntry` slot grows ONE
//!   dispatch method — `fn dispatch_pair(&self, pair: &PairDescriptor, box_:
//!   &Box4) -> Outcome<…>` — declared as the [`SplineSsiEntryDispatch`]
//!   obligation below. The single registered entry routes internally by pair
//!   descriptor, so registration order is irrelevant. The impl lands in
//!   `truck-evidence/src/contact/mod.rs` under CFP-004; a one-line
//!   forward-declaration comment is enough here — no cross-crate dependency is
//!   added by this spine.
//! - **Instrument schema (decision 5).** Counter names and JSON key order are
//!   frozen as data ([`SCHEMA_KEYS`], [`InstrumentCounters`]). Each crate
//!   implements its own local counter struct with these field names (F1); the
//!   frozen order lets per-crate JSON outputs concatenate.
//! - **Gates (decision 6).** V5-pair (untouched paths, doc below), V5-boolean
//!   (the monotone-diff adjudication implemented on [`V5BooleanDiff`]), and the
//!   BG-ENC-002 convergence-assertion helper's signature (doc below).
//! - **Zero new top-level evidence kinds (decision 7).** The two new `Method`
//!   provenance tags (`ImplicitReduction`, `ConeCertificate`) are booked as
//!   mapping rows, not widenings of the base evidence types.

use crate::contract::Refusal;
use crate::formal::numeric::PositiveFinite;
use truck_base::evidence::Outcome;
use truck_base::evidence::Refusal as EvidenceRefusal;

/// A finite, ordered `(lo, hi)` axis interval.
///
/// Pure structure — no arithmetic lives in this module (D-shim).
fn interval_ok((lo, hi): (f64, f64)) -> bool {
    lo.is_finite() && hi.is_finite() && lo < hi
}

/// The two sides of a dispatched pair or a refused stratum pair.
///
/// Side `A` is the pair's first participant/stratum, side `B` the second
/// (decision 3). This is a frozen vocabulary tag, not a geometry type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StratumSide {
    /// The pair's first (left / `a`) side.
    A,
    /// The pair's second (right / `b`) side.
    B,
}

impl StratumSide {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::A => "a",
            Self::B => "b",
        }
    }
}

/// A stratum pair: one stratum index per side, canonically `A` first.
///
/// `(StratumSide::A, i, StratumSide::B, j)` names stratum `i` on the first
/// side and stratum `j` on the second. The canonical order is enforced by
/// [`LocalizedRefusal::new`]: any pairing that is not `A`-then-`B` is a
/// construction outside the frozen rule.
pub type StratumPair = (StratumSide, usize, StratumSide, usize);

/// A localizable refusal: a landed evidence refusal wrapped verbatim beside
/// the stratum pair it concerns (decision 3).
///
/// This is the CFP program's refusal wrapper — zero new top-level evidence
/// kinds: [`cause`](LocalizedRefusal::cause) is a landed
/// [`Refusal`](truck_base::evidence::Refusal) carried verbatim, and the
/// stratum pair localizes it to the two strata whose interaction refused.
/// Re-attribution to faces happens in `truck-shapeops` (CFP-001) through the
/// lift's index → face map, never here.
#[derive(Debug, Clone)]
pub struct LocalizedRefusal {
    /// The landed evidence refusal, verbatim.
    cause: EvidenceRefusal,
    /// The stratum pair `(A, i, B, j)` the refusal concerns.
    stratum_pair: StratumPair,
}

impl LocalizedRefusal {
    /// Wrap a landed evidence refusal with its stratum pair.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) any pairing that is not exactly one
    /// `A`-side stratum and one `B`-side stratum in canonical `A`-then-`B`
    /// order — a stratum pair always names one stratum per side.
    pub fn new(cause: EvidenceRefusal, stratum_pair: StratumPair) -> Result<Self, Refusal> {
        match stratum_pair {
            (StratumSide::A, _, StratumSide::B, _) => Ok(Self {
                cause,
                stratum_pair,
            }),
            _ => Err(Refusal::InvalidInput),
        }
    }

    /// The landed evidence refusal, verbatim.
    pub fn cause(&self) -> &EvidenceRefusal {
        &self.cause
    }

    /// The stratum pair `(A, i, B, j)`, verbatim.
    pub fn stratum_pair(&self) -> StratumPair {
        self.stratum_pair
    }
}

/// A 4-D parameter cell `(u, v, s, t)`, the sub-box convention of the CFP
/// dispatch and the `SplineSsiEntry` slot (the `(u, v) × (s, t)` box the
/// landed funnel already carries on `certify_spline_box`).
///
/// Frozen as a type alias so every CFP signature spells the same cell shape;
/// the four axes are the first carrier's two parameters then the second
/// carrier's two. A valid cell has finite, positively-wide axis intervals
/// ([`box4_ok`]).
pub type Box4 = [(f64, f64); 4];

/// Whether a [`Box4`] is a valid parameter cell: every axis interval finite
/// and positively wide (`lo < hi`), matching the refusing constructors of this
/// module.
pub fn box4_ok(box_: &Box4) -> bool {
    box_.iter().all(|axis| interval_ok(*axis))
}

/// The carrier-class tag of one side of a dispatched pair.
///
/// The routing key of the CFP funnel's recognized carrier set: the five
/// canonical analytic classes plus the spline carrier (Theorem 4 admits planes
/// and quadrics — cylinder, sphere, cone — on the implicit arm; the torus is
/// explicitly excluded from that arm by its quartic implicit, §7). A carrier
/// outside this set is handled before the CFP stage and never reaches a
/// [`PairDescriptor`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarrierClass {
    /// A plane (affine implicit, Theorem 4 bidegree `(p, q)` preserved).
    Plane,
    /// A cylinder (quadric implicit, Theorem 4 bidegree `(2p, 2q)`).
    Cylinder,
    /// A sphere (quadric implicit).
    Sphere,
    /// A cone (quadric implicit).
    Cone,
    /// A torus — routed, but excluded from the implicit arm (§7).
    Torus,
    /// A spline carrier (the admitted non-rational `BSplineSurface`).
    Spline,
}

impl CarrierClass {
    /// A short stable tag, for diagnostics and instrument records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Plane => "plane",
            Self::Cylinder => "cylinder",
            Self::Sphere => "sphere",
            Self::Cone => "cone",
            Self::Torus => "torus",
            Self::Spline => "spline",
        }
    }
}

/// A dispatched pair descriptor: the carrier-class pair and both parameter
/// boxes (decision 4).
///
/// This is the routing key of the single `SplineSsiEntry` registered entry —
/// the entry routes internally by [`PairDescriptor`], so registration order is
/// irrelevant. Each side carries its carrier class and its parameter box (the
/// `(u, v)` window on the spline side, the analytic window on the other).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairDescriptor {
    /// The first side's carrier class.
    class_a: CarrierClass,
    /// The second side's carrier class.
    class_b: CarrierClass,
    /// The first side's parameter box `(u, v)`.
    box_a: ((f64, f64), (f64, f64)),
    /// The second side's parameter box `(s, t)`.
    box_b: ((f64, f64), (f64, f64)),
}

impl PairDescriptor {
    /// Build a pair descriptor from the two carrier classes and both parameter
    /// boxes.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a parameter box containing a
    /// non-finite, misordered, or degenerate (`lo >= hi`) axis interval — a
    /// carrier window is a positively-wide rectangle.
    pub fn new(
        class_a: CarrierClass,
        class_b: CarrierClass,
        box_a: ((f64, f64), (f64, f64)),
        box_b: ((f64, f64), (f64, f64)),
    ) -> Result<Self, Refusal> {
        if !interval_ok(box_a.0)
            || !interval_ok(box_a.1)
            || !interval_ok(box_b.0)
            || !interval_ok(box_b.1)
        {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self {
            class_a,
            class_b,
            box_a,
            box_b,
        })
    }

    /// The first side's carrier class, verbatim.
    pub fn class_a(&self) -> CarrierClass {
        self.class_a
    }

    /// The second side's carrier class, verbatim.
    pub fn class_b(&self) -> CarrierClass {
        self.class_b
    }

    /// The first side's parameter box `(u, v)`, verbatim.
    pub fn box_a(&self) -> ((f64, f64), (f64, f64)) {
        self.box_a
    }

    /// The second side's parameter box `(s, t)`, verbatim.
    pub fn box_b(&self) -> ((f64, f64), (f64, f64)) {
        self.box_b
    }
}

/// A float search result carrying NO evidence status (decision 2, SFC).
///
/// The CFP discipline splits every float heuristic into a *float search* and
/// an *exact certificate*: this type is the search half. It carries the float
/// hint value only — no `Method`, no truth value, no property map — and is
/// deliberately constructed so that it cannot occupy an evidence position: it
/// implements none of the evidence vocabulary, exposes no conversion into
/// [`Certified`](truck_base::evidence::Certified) or a
/// [`Certificate`](truck_base::evidence::Certificate), and the only way to
/// build one is through [`FloatHint::new`], which refuses a value that is not
/// an admissible search result ([`FloatSearchValue`]).
///
/// The exact verifier consumes the hint (through [`FloatHint::value`]) and
/// decides independently in exact/interval arithmetic. A `FloatHint` is never
/// itself the certified statement.
#[derive(Debug, Clone)]
pub struct FloatHint<T> {
    /// The float search result.
    value: T,
}

impl<T: FloatSearchValue> FloatHint<T> {
    /// Wrap a float search result.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a value that is not an admissible
    /// search result (see [`FloatSearchValue`]). There is deliberately no
    /// infallible constructor and no evidence-side conversion: a float search
    /// result is never an evidence position.
    pub fn new(value: T) -> Result<Self, Refusal> {
        if !value.is_admissible() {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self { value })
    }

    /// The float search result, verbatim.
    pub fn value(&self) -> &T {
        &self.value
    }
}

/// Admissibility of a float search result: what [`FloatHint::new`] refuses.
///
/// A float hint is admissible when it is a usable *input to an exact
/// verifier*: finite. A hint may be zero or misdirected — that is a search
/// miss the exact verify reports, not an inadmissible value.
pub trait FloatSearchValue {
    /// Whether this value is admissible as a float search result.
    fn is_admissible(&self) -> bool;
}

impl FloatSearchValue for f64 {
    fn is_admissible(&self) -> bool {
        self.is_finite()
    }
}

impl<const N: usize> FloatSearchValue for [f64; N] {
    fn is_admissible(&self) -> bool {
        self.iter().all(|component| component.is_finite())
    }
}

/// A cone-disjointness verdict (Gauss-map certificates, spec §3, decision 6).
///
/// Consumed by CFP-006. Until that packet lands, the verdict is **produced
/// behind a pending refusal**: this spine freezes the shape and the refusing
/// constructor of the data-carrying arm only — no production path exists here,
/// and any numeric request that would yield a verdict refuses
/// ([`Refusal::Unfrozen`]). The unit arms are shape declarations; the
/// certified production functions that establish them land with CFP-006.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConeVerdict {
    /// The pair is loop-free on the cell (Sinha/Sederberg parallel-normal or
    /// collinear-normal certificate; every branch meets the cell boundary).
    LoopFree,
    /// The pair is transversal on the cell by proof (Proposition 2 — the CTE
    /// cascade is unreachable by proof here).
    TransversalByProof,
    /// A fixed continuation axis with certified positive margin (Proposition
    /// 2.1), so `ConditioningBelowThreshold` cannot fire along the branch.
    AxisFixed {
        /// The fixed continuation axis, one of the four chart coordinates.
        axis: usize,
        /// The certified positive margin on that axis.
        margin: PositiveFinite,
    },
}

impl ConeVerdict {
    /// Build the `AxisFixed` arm.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) an axis outside `0..4` (the four
    /// chart coordinates) or a non-finite / non-positive margin. The margin is
    /// carried as a certified positive scalar ([`PositiveFinite`]).
    pub fn axis_fixed(axis: usize, margin: f64) -> Result<Self, Refusal> {
        if axis > 3 {
            return Err(Refusal::InvalidInput);
        }
        let margin = PositiveFinite::new(margin).map_err(|_| Refusal::InvalidInput)?;
        Ok(Self::AxisFixed { axis, margin })
    }

    /// The fixed continuation axis of an `AxisFixed` verdict.
    pub fn axis(&self) -> Option<usize> {
        match self {
            Self::AxisFixed { axis, .. } => Some(*axis),
            _ => None,
        }
    }

    /// The certified positive margin of an `AxisFixed` verdict.
    pub fn margin(&self) -> Option<PositiveFinite> {
        match self {
            Self::AxisFixed { margin, .. } => Some(*margin),
            _ => None,
        }
    }

    /// A short stable tag, for diagnostics.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::LoopFree => "cone_verdict_loop_free",
            Self::TransversalByProof => "cone_verdict_transversal_by_proof",
            Self::AxisFixed { .. } => "cone_verdict_axis_fixed",
        }
    }
}

// ---------------------------------------------------------------------------
// Instrument schema (decision 5)
// ---------------------------------------------------------------------------

/// The frozen instrument counter names, in the frozen JSON key order.
///
/// Per-crate counter structs (F1 forbids a shared implementation) must carry
/// exactly these field names and emit JSON in exactly this key order, so the
/// per-crate outputs concatenate into one comparable stream. CFP-002 owns the
/// first three counters; `cells_visited`, `distinct_side_boxes`, and
/// `unresolved_provenance` ride the same hook after CFP-001.
pub const SCHEMA_KEYS: [&str; 6] = [
    "knot_span_count",
    "stage5_pair_class",
    "composed_bidegree",
    "cells_visited",
    "distinct_side_boxes",
    "unresolved_provenance",
];

/// The canonical instrument counter record (decision 5, schema data).
///
/// This spine freezes the schema — the six counter names and their JSON key
/// order — as data. Each crate implements its own local counter struct with
/// these field names (F1); this canonical record is the reference exemplar and
/// the round-trip target: [`InstrumentCounters::to_schema_json`] emits a JSON
/// object in the frozen key order and [`InstrumentCounters::from_schema_json`]
/// parses exactly that schema back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InstrumentCounters {
    /// Knot-span distribution on admitted carriers (CFP-002).
    knot_span_count: u64,
    /// Pair-type histogram count at stage-5 entry (CFP-002).
    stage5_pair_class: u64,
    /// Composed-degree-vs-budget record (CFP-002).
    composed_bidegree: u64,
    /// Cells visited (rides the CFP-001 hook).
    cells_visited: u64,
    /// Distinct side boxes (rides the CFP-001 hook).
    distinct_side_boxes: u64,
    /// Unresolved-provenance split (rides the CFP-001 hook).
    unresolved_provenance: u64,
}

impl InstrumentCounters {
    /// Build a canonical counter record from the six counters in the frozen
    /// field order. Infallible: every counter is a `u64`, and the schema is
    /// fixed at the type.
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

    /// Serialize to the frozen JSON schema: one object whose keys appear in
    /// the frozen [`SCHEMA_KEYS`] order.
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

    /// Parse the frozen JSON schema back into a counter record.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) any text that is not a JSON object
    /// whose keys are exactly [`SCHEMA_KEYS`] in the frozen order with `u64`
    /// values — a counter stream is only comparable when every crate emitted
    /// the same schema.
    pub fn from_schema_json(text: &str) -> Result<Self, Refusal> {
        let inner = text
            .trim()
            .strip_prefix('{')
            .and_then(|rest| rest.strip_suffix('}'))
            .ok_or(Refusal::InvalidInput)?;
        let mut fields: Vec<u64> = Vec::with_capacity(SCHEMA_KEYS.len());
        for (slot, part) in inner.split(',').enumerate() {
            let key = SCHEMA_KEYS.get(slot).ok_or(Refusal::InvalidInput)?;
            let (key_part, value_part) = part.split_once(':').ok_or(Refusal::InvalidInput)?;
            if key_part.trim().trim_matches('"') != *key {
                return Err(Refusal::InvalidInput);
            }
            let value: u64 = value_part
                .trim()
                .trim_matches('"')
                .parse()
                .map_err(|_| Refusal::InvalidInput)?;
            fields.push(value);
        }
        if fields.len() != SCHEMA_KEYS.len() {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self::new(
            fields[0], fields[1], fields[2], fields[3], fields[4], fields[5],
        ))
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

// ---------------------------------------------------------------------------
// Gates (decision 6): V5-pair, V5-boolean, BG-ENC-002 convergence assertion
// ---------------------------------------------------------------------------

// Gate definitions frozen here as docs + assertion helpers (decision 6).
//
// - **V5-pair.** Landed canonical×canonical *pair* answers stay bit-identical
//   (they never traverse the refactored paths). This is a regression gate on
//   the landed closed-form machinery: a CFP diff that changes a canonical
//   pair answer is a regression, period. No helper is implemented here — the
//   assertion is a run of the landed pair paths against their recorded
//   outputs, which is CFP's implementing packets' work, not a shape.
// - **V5-boolean.** CFP-001 intentionally widens the boolean screen; the
//   change is *monotone* — boundary samples lie on the image, so a widened
//   screen can only add contact events, never remove them. A diff is accepted
//   iff the new event set is a SUPERSET of the old on the same inputs. The
//   adjudication record and its helper are implemented below
//   ([`V5BooleanDiff`], [`V5BooleanDiff::adjudicate`]).
// - **BG-ENC-002 convergence assertion helper's signature** (frozen here;
//   implemented by CFP-001's tests against the landed enclosures):
//
//   ```text
//   fn assert_enclosure_converges(
//       enclose: &dyn Fn([(f64, f64); 2]) -> [(f64, f64); 3],
//       outer: [(f64, f64); 2],
//       mid: [(f64, f64); 2],
//       inner: [(f64, f64); 2],
//   ) -> Result<(), Refusal>
//   ```
//
//   asserting, on the F-C2 nested boxes, that the three enclosure widths are
//   non-increasing and strictly decrease somewhere (the permanent guard on
//   `NUM-SPLINE-ENCLOSURE-CONVERGENCE-001`).
//
// **SFC.** Every float heuristic in the program decomposes into (float
// search, exact verify); a float that is neither evidence nor a hint consumed
// by an exact check is refused at review ([`FloatHint`] types the hint half).

/// A deterministic contact-event set: sorted, deduplicated event ids.
///
/// The V5-boolean adjudication compares event sets on the same inputs, so a
/// set needs a canonical, deterministic ordering — never hash iteration
/// (determinism house rule).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactEventSet {
    /// The sorted, deduplicated event ids.
    ids: Vec<u64>,
}

impl ContactEventSet {
    /// Build a contact-event set from any id collection, normalizing to
    /// sorted, deduplicated order. Infallible: normalization cannot fail.
    pub fn new(mut ids: Vec<u64>) -> Self {
        ids.sort_unstable();
        ids.dedup();
        Self { ids }
    }

    /// An empty event set.
    pub fn empty() -> Self {
        Self { ids: Vec::new() }
    }

    /// The event ids in sorted order, verbatim.
    pub fn ids(&self) -> &[u64] {
        &self.ids
    }

    /// The number of events.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Whether `other` is a subset of `self` (self ⊇ other).
    pub fn is_superset_of(&self, other: &Self) -> bool {
        self.difference(other).is_empty()
    }

    /// The events of `self` not present in `other`, in sorted order.
    pub fn difference(&self, other: &Self) -> Vec<u64> {
        let other_set: std::collections::HashSet<u64> = other.ids.iter().copied().collect();
        self.ids
            .iter()
            .copied()
            .filter(|id| !other_set.contains(id))
            .collect()
    }
}

/// Why a V5-boolean diff refused adjudication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum V5AdjudicationRefusal {
    /// The new event set removed events the old set had — the diff is not
    /// monotone, so it is a regression (or a revert trigger), never an
    /// accepted widening. Carries the dropped event ids in sorted order.
    NonMonotone {
        /// The events present in the old set and missing from the new one.
        dropped: Vec<u64>,
    },
}

/// The V5-boolean adjudication record (decision 6): one recorded output diff
/// on one pair of inputs.
///
/// A boolean output diff is recorded as (same inputs — the pair descriptor —
/// old event set, new event set). The diff is accepted iff the new event set
/// is a SUPERSET of the old: CFP-001's widened screen can only add contact
/// events. Every diff is adjudicated and recorded, never silently accepted,
/// never a revert trigger.
#[derive(Debug, Clone, PartialEq)]
pub struct V5BooleanDiff {
    /// The inputs the diff was measured on (the pair descriptor, hence "same
    /// inputs" is structural).
    pair: PairDescriptor,
    /// The pre-CFP contact-event set on those inputs.
    old_events: ContactEventSet,
    /// The post-CFP contact-event set on those inputs.
    new_events: ContactEventSet,
}

impl V5BooleanDiff {
    /// Record a boolean output diff on one pair of inputs.
    pub fn new(
        pair: PairDescriptor,
        old_events: ContactEventSet,
        new_events: ContactEventSet,
    ) -> Self {
        Self {
            pair,
            old_events,
            new_events,
        }
    }

    /// The inputs the diff was measured on, verbatim.
    pub fn pair(&self) -> &PairDescriptor {
        &self.pair
    }

    /// The old (pre-CFP) event set, verbatim.
    pub fn old_events(&self) -> &ContactEventSet {
        &self.old_events
    }

    /// The new (post-CFP) event set, verbatim.
    pub fn new_events(&self) -> &ContactEventSet {
        &self.new_events
    }

    /// Whether the diff is monotone (the new set is a superset of the old).
    pub fn is_monotone(&self) -> bool {
        self.new_events.is_superset_of(&self.old_events)
    }

    /// Adjudicate the diff under the V5-boolean gate.
    ///
    /// Accepts iff the new event set is a SUPERSET of the old on the same
    /// inputs; refuses [`V5AdjudicationRefusal::NonMonotone`] with the dropped
    /// events otherwise. A regression suite that reports a removal is reporting
    /// a real non-monotone change, never an accepted widening.
    pub fn adjudicate(&self) -> Result<(), V5AdjudicationRefusal> {
        let dropped = self.old_events.difference(&self.new_events);
        if dropped.is_empty() {
            Ok(())
        } else {
            Err(V5AdjudicationRefusal::NonMonotone { dropped })
        }
    }
}

// ---------------------------------------------------------------------------
// Decision 4: the SplineSsiEntry dispatch-method obligation
// ---------------------------------------------------------------------------

/// Decision 4 — the `SplineSsiEntry::dispatch_pair` dispatch-method obligation.
///
/// The landed single-slot `SplineSsiEntry` hook
/// (`truck-evidence/src/contact/mod.rs`) grows ONE dispatch method whose
/// signature is frozen here (slot resolution decided at the spine, executed by
/// CFP-004):
///
/// ```text
/// fn dispatch_pair(&self, pair: &PairDescriptor, box_: &Box4) -> Outcome<Payload>
/// ```
///
/// — `PairDescriptor` is the routing key above (carrier-class pair + both
/// parameter boxes), `Box4` is the `(u, v) × (s, t)` sub-box of the pair's
/// chart, and the payload is the pair-type dispatch answer CFP-004 owns (the
/// spine's `Outcome<…>` stays open). The single registered entry routes
/// internally by pair descriptor, making registration order irrelevant.
///
/// The impl lands in `truck-evidence/src/contact/mod.rs` under CFP-004; this
/// one-line forward declaration is the freeze — no cross-crate dependency is
/// added here, and no numeric work happens in this module (D-shim).
pub trait SplineSsiEntryDispatch {
    /// The dispatch answer payload the implementing entry produces (decided by
    /// CFP-004; the spine leaves it open as `Outcome<…>`).
    type Payload;

    /// Route one carrier-class pair on one sub-box by its [`PairDescriptor`].
    ///
    /// `box_` must be a sub-box of the pair's joint chart (`box4_ok`), within
    /// the two parameter boxes the descriptor carries. A spine-side
    /// implementation refuses (no numeric evaluation exists before CFP-004).
    fn dispatch_pair(&self, pair: &PairDescriptor, box_: &Box4) -> Outcome<Self::Payload>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localizable_refusal_carries_stratum_pair() {
        // A canonical A-then-B pairing admits and carries the cause verbatim.
        let refusal = LocalizedRefusal::new(
            EvidenceRefusal::Empty,
            (StratumSide::A, 2, StratumSide::B, 7),
        )
        .expect("an A-then-B stratum pair admits");
        assert!(matches!(refusal.cause(), EvidenceRefusal::Empty));
        assert_eq!(
            refusal.stratum_pair(),
            (StratumSide::A, 2, StratumSide::B, 7)
        );
        assert_eq!(StratumSide::A.tag(), "a");
        assert_eq!(StratumSide::B.tag(), "b");

        // A pairing that is not exactly one A-side and one B-side stratum in
        // canonical order refuses (construction outside the frozen rule).
        let same_side = LocalizedRefusal::new(
            EvidenceRefusal::Empty,
            (StratumSide::A, 0, StratumSide::A, 1),
        );
        assert!(matches!(same_side, Err(Refusal::InvalidInput)));
        let reversed = LocalizedRefusal::new(
            EvidenceRefusal::Empty,
            (StratumSide::B, 0, StratumSide::A, 1),
        );
        assert!(matches!(reversed, Err(Refusal::InvalidInput)));
        let both_b = LocalizedRefusal::new(
            EvidenceRefusal::Empty,
            (StratumSide::B, 0, StratumSide::B, 1),
        );
        assert!(matches!(both_b, Err(Refusal::InvalidInput)));
    }

    #[test]
    fn instrument_schema_round_trip() {
        // The frozen key order, verbatim.
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

        // Serialization emits the frozen key order exactly.
        let counters = InstrumentCounters::new(7, 1, 3, 12, 9, 0);
        let json = counters.to_schema_json();
        assert_eq!(
            json,
            "{\"knot_span_count\":7,\"stage5_pair_class\":1,\"composed_bidegree\":3,\
             \"cells_visited\":12,\"distinct_side_boxes\":9,\"unresolved_provenance\":0}"
        );

        // And the schema parses back to the identical record.
        let parsed =
            InstrumentCounters::from_schema_json(&json).expect("the frozen schema round-trips");
        assert_eq!(parsed, counters);

        // A key order violation refuses: the schema is a frozen contract.
        let reordered = "{\"knot_span_count\":7,\"composed_bidegree\":3,\"stage5_pair_class\":1,\
             \"cells_visited\":12,\"distinct_side_boxes\":9,\"unresolved_provenance\":0}";
        assert!(matches!(
            InstrumentCounters::from_schema_json(reordered),
            Err(Refusal::InvalidInput)
        ));
        // A missing key refuses.
        let missing = "{\"knot_span_count\":7,\"stage5_pair_class\":1,\"composed_bidegree\":3,\
                        \"cells_visited\":12,\"distinct_side_boxes\":9}";
        assert!(matches!(
            InstrumentCounters::from_schema_json(missing),
            Err(Refusal::InvalidInput)
        ));
    }

    #[test]
    fn spline_entry_dispatch_method_signature_freezes() {
        // Decision 4, compile-time shape assertion: the obligation spells
        // exactly `fn dispatch_pair(&self, pair: &PairDescriptor, box_: &Box4)
        // -> Outcome<Payload>`. Binding the associated method to a function
        // item of that shape is the assertion; the payload is left open
        // (`Outcome<…>`), exactly as the spine freeze decides.
        struct ProbePayload;
        struct DispatchProbe;
        impl SplineSsiEntryDispatch for DispatchProbe {
            type Payload = ProbePayload;

            fn dispatch_pair(
                &self,
                _pair: &PairDescriptor,
                _box_: &Box4,
            ) -> Outcome<Self::Payload> {
                // No numeric evaluation in this freeze: a spine-side dispatch
                // refuses. Only the signature is pinned here.
                Err(EvidenceRefusal::Empty)
            }
        }

        let _signature: fn(&DispatchProbe, &PairDescriptor, &Box4) -> Outcome<ProbePayload> =
            <DispatchProbe as SplineSsiEntryDispatch>::dispatch_pair;

        // The frozen routing key carries the carrier-class pair and both
        // parameter boxes; the sub-box is a valid 4-D cell.
        let pair = PairDescriptor::new(
            CarrierClass::Spline,
            CarrierClass::Plane,
            ((0.0, 1.0), (0.0, 1.0)),
            ((0.0, 1.0), (0.0, 1.0)),
        )
        .expect("a valid pair descriptor");
        let box_: Box4 = [(0.0, 0.5), (0.0, 0.5), (0.0, 1.0), (0.0, 1.0)];
        assert!(box4_ok(&box_));

        let outcome =
            <DispatchProbe as SplineSsiEntryDispatch>::dispatch_pair(&DispatchProbe, &pair, &box_);
        assert!(matches!(outcome, Err(EvidenceRefusal::Empty)));
    }
}
