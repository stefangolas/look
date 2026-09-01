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

//! The Phase-0 contract freeze (BG-CK-P0-FREEZE): F1 witness edge, F2 bound
//! policy, F3 continuation coordinates.
//!
//! The four contract-freeze decisions of the certified-kernel plan are
//! irrecoverable later, so they are made HERE, pre-made, the way BG-CG-000
//! froze the §3.5 certificate mapping. This module is the FROZEN TEXT made
//! typecheckable: the three decisions are quoted verbatim below, each tagged
//! with its plan section. Every evaluator refuses — there is no numerical
//! implementation in the freeze; the types and the DECISIONS are the
//! deliverable. Phase-1 packets implement against this module and never
//! relitigate it.
//!
//! **F1 — witness edge (plan §1, §3 Phase 0, §4).** The certified Edge carries
//! the fiber-product witness — the pcurve pair, BOTH surface handles, and the
//! enclosures — not a fitted spline with error bars. Spline emission happens
//! at export/meshing only. A downstream consumer that wants a polyline gets it
//! from the witness at export time; the witness itself is the identity claim
//! "there was never a second edge", and it is NEVER a spline carrier. (Mapping
//! section C row 2: the witness stays attached to the edge; only derived facts
//! with `Method` tags enter row-set A carriers.)
//!
//! **F2 — per-quantity bound policy (plan D2 scope statement).** The class-3
//! rational bounds decompose into five named quantities, and EACH gets one
//! pre-made choice between the two sanctioned mechanisms (named interval
//! composition vs auxiliary root isolation):
//!
//! | Quantity | Choice | Mechanism |
//! |---|---|---|
//! | normal admissibility: certified lower bound on `|Sᵤ × Sᵥ|` | interval composition | fixed named composition: hull-bounded first-derivative patches, interval cross product, directed rounding at the leaves |
//! | curvature (rational in derivatives through order 2) | interval composition + isolation guard | value from the named composition of hull-bounded derivative enclosures; the well-definedness of the division (denominator ≠ 0) is certified by AUXILIARY ROOT ISOLATION on the denominator polynomial via `bezier_isect` — never by interval sign-testing alone |
//! | rational NURBS numerator/denominator | interval composition | homogeneous control points bounded separately (hulls), division under directed rounding |
//! | rational NURBS quotient (the divided value) | interval composition | directed-rounded division of the two enclosures above |
//! | any FUTURE quantity not in this table | unspecified — refuses | a quantity outside the frozen table is a SPEC_GAP: the policy records `Unfrozen` and the constructor refuses `InvalidInput`; widening the table is an orchestrator spec edit, never a worker decision |
//!
//! **F3 — continuation-coordinate contract (plan §2 class 2 generic).** The
//! class-2 Krawczyk operator runs on SQUARE 3×3 systems only (the
//! pseudo-inverse-preconditioned rectangular route is explicitly rejected).
//! Per box, ONE continuation coordinate is selected by this frozen rule: the
//! coordinate `i` whose certified ∂H_i/∂t_i enclosure over the box is strictly
//! away from zero with the LARGEST relative margin (|lower bound| / box extent
//! in t_i); ties break to the LOWEST index (deterministic — no hash order). If
//! NO coordinate certifies away-from-zero, the box refuses
//! `ConditioningBelowThreshold` — it is never retried with a weaker test.
//! Turning-point SWITCHING is a certified event: at a switch box, BOTH square
//! systems (the outgoing coordinate's and the incoming coordinate's) are
//! certified by their own Krawczyk calls, and the traced branch records a
//! `CoordinateSwitch` carrying both certificates. A heuristic reseed without
//! both certificates is a contract violation.
//!
//! The refusal vocabulary below is the certified layer's own, per
//! `docs/CERTIFICATE_MAPPING.md` section C row 1: no top-level variant is
//! added to the base `truck_base::evidence::Refusal`; the certified-layer
//! failure witnesses live here, in `truck-certified`.

use crate::formal::numeric::{FiniteF64, PositiveFinite};
use truck_base::evidence::Method;

/// The certified contract's refusal vocabulary.
///
/// Named cases only — no catch-all — matching the refusal shape of
/// `formal/outcome.rs`. This is the certified layer's own vocabulary
/// (`docs/CERTIFICATE_MAPPING.md` section C row 1): the base
/// `truck_base::evidence::Refusal` is untouched, and the certified-layer
/// failure witnesses live in `truck-certified`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// F2: a quantity outside the frozen table is a SPEC_GAP. Widening the
    /// table is an orchestrator spec edit, never a worker decision.
    Unfrozen,
    /// A construction outside a frozen rule: the request is invalid input.
    InvalidInput,
    /// F3: no coordinate of the square system certifies away-from-zero over
    /// the box. The box refuses — it is never retried with a weaker test.
    ConditioningBelowThreshold,
}

/// A certified interval enclosure: lower/upper as certified interval values,
/// `Method`-tagged (H-6).
///
/// The reuse scan the packet prescribes (`formal/numeric.rs` and
/// `formal/evidence.rs`) finds no certified interval *bound* type:
/// `numeric.rs` holds checked scalars (`FiniteF64`, `NonNegativeFinite`,
/// `PositiveFinite`) and `evidence.rs`'s `ClosedInterval` is a parameter-domain
/// type, not a bound. So the minimal tagged enclosure is defined here, exactly
/// as the packet's "otherwise" branch prescribes.
///
/// This is a certificate *carrier*, not an interval algebra: it performs no
/// arithmetic. The crate's one interval algebra — `formal/exact.rs`'s
/// `CertifiedInterval`, outward-rounded and untouched — remains the single
/// primitive Phase-1 composes (D2's parsimony rule: one primitive, composed).
///
/// A float estimate never enters this struct: the method is fixed at
/// `Method::Interval` (F1/H-6).
///
/// ```
/// use truck_certified::contract::IntervalEnclosure;
/// use truck_base::evidence::Method;
///
/// let enclosure = IntervalEnclosure::new(0.0, 1.0).expect("a valid interval");
/// assert_eq!(enclosure.method(), Method::Interval);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntervalEnclosure {
    lower: FiniteF64,
    upper: FiniteF64,
    method: Method,
}

impl IntervalEnclosure {
    /// Build an enclosure, refusing a non-finite or misordered pair.
    ///
    /// The method tag is fixed at `Method::Interval`: interval work only, a
    /// float estimate never enters the struct (F1/H-6).
    pub fn new(lower: f64, upper: f64) -> Result<Self, Refusal> {
        let lower = FiniteF64::new(lower).map_err(|_| Refusal::InvalidInput)?;
        let upper = FiniteF64::new(upper).map_err(|_| Refusal::InvalidInput)?;
        if lower.get() > upper.get() {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self {
            lower,
            upper,
            method: Method::Interval,
        })
    }

    /// The certified lower bound.
    pub fn lower(&self) -> FiniteF64 {
        self.lower
    }

    /// The certified upper bound.
    pub fn upper(&self) -> FiniteF64 {
        self.upper
    }

    /// The method tag. Fixed at `Method::Interval` in the freeze.
    pub fn method(&self) -> Method {
        self.method
    }
}

/// F1: the fiber-product witness. The certified Edge IS this; a spline view
/// is derived at export only (a future `ExportView` type, not a field).
///
/// The witness is NEVER a spline carrier. The negative is compile-level: this
/// snippet must not compile, because `WitnessEdge` has no spline/Bézier
/// emission accessor:
///
/// ```compile_fail
/// let edge: truck_certified::contract::WitnessEdge<u8, u8> = unimplemented!();
/// let _spline = edge.spline();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct WitnessEdge<S, C> {
    /// The pcurve on `surface_a`, in the support surface's own chart (the
    /// identify_plane retained-basis doctrine: never orthogonalised, never
    /// normalised).
    pub pcurve_a: C,
    /// The pcurve on `surface_b`, in the support surface's own chart.
    pub pcurve_b: C,
    /// Both support surfaces. Handles, not copies.
    pub surface_a: S,
    /// Both support surfaces. Handles, not copies.
    pub surface_b: S,
    /// An enclosure for `pcurve_a` over its domain. `Method::Interval` per
    /// H-6 — the witness is interval work; a float estimate never enters this
    /// struct.
    pub enclosure_a: IntervalEnclosure,
    /// An enclosure for `pcurve_b` over its domain. `Method::Interval` per
    /// H-6.
    pub enclosure_b: IntervalEnclosure,
}

/// The five quantities of the F2 bound table, in table order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    /// Normal admissibility: certified lower bound on `|Sᵤ × Sᵥ|`.
    NormalAdmissibility,
    /// Curvature, rational in derivatives through order 2.
    Curvature,
    /// The rational NURBS numerator (homogeneous control points, hulls).
    RationalNumerator,
    /// The rational NURBS denominator (homogeneous control points, hulls).
    RationalDenominator,
    /// The rational NURBS quotient (the divided value).
    RationalQuotient,
}

/// A quantity's pre-made F2 choice between the two sanctioned mechanisms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundMechanism {
    /// Fixed named interval composition with directed rounding at the leaves.
    IntervalComposition,
    /// Interval composition PLUS an auxiliary root-isolation guard on the
    /// denominator polynomial (the frozen Curvature choice).
    IntervalCompositionWithRootIsolationGuard,
    /// A quantity outside the frozen table: a SPEC_GAP. Never a frozen row.
    Unfrozen,
}

/// One row of the frozen F2 table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundPolicyRow {
    quantity: Quantity,
    mechanism: BoundMechanism,
}

impl BoundPolicyRow {
    /// Build a policy row, refusing any construction outside the frozen rules.
    ///
    /// F2: the Curvature row's well-definedness guard is AUXILIARY ROOT
    /// ISOLATION — a composition-only curvature row (denominator certified by
    /// interval sign-testing alone) refuses `InvalidInput`. An `Unfrozen` row
    /// is likewise a contradiction of the frozen table, because every
    /// `Quantity` variant is already in it.
    pub fn new(quantity: Quantity, mechanism: BoundMechanism) -> Result<Self, Refusal> {
        match (quantity, mechanism) {
            (Quantity::Curvature, BoundMechanism::IntervalComposition)
            | (_, BoundMechanism::Unfrozen) => Err(Refusal::InvalidInput),
            _ => Ok(Self {
                quantity,
                mechanism,
            }),
        }
    }

    /// Which quantity.
    pub fn quantity(&self) -> Quantity {
        self.quantity
    }

    /// The frozen choice.
    pub fn mechanism(&self) -> BoundMechanism {
        self.mechanism
    }
}

/// The F2 table as data: five named rows, exactly the frozen table.
///
/// Construction only through [`BoundPolicy::frozen`]; every other construction
/// path refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundPolicy {
    rows: [BoundPolicyRow; 5],
}

impl BoundPolicy {
    /// The five-row frozen table, matching the F2 table row for row.
    pub fn frozen() -> Self {
        Self {
            rows: [
                BoundPolicyRow {
                    quantity: Quantity::NormalAdmissibility,
                    mechanism: BoundMechanism::IntervalComposition,
                },
                BoundPolicyRow {
                    quantity: Quantity::Curvature,
                    mechanism: BoundMechanism::IntervalCompositionWithRootIsolationGuard,
                },
                BoundPolicyRow {
                    quantity: Quantity::RationalNumerator,
                    mechanism: BoundMechanism::IntervalComposition,
                },
                BoundPolicyRow {
                    quantity: Quantity::RationalDenominator,
                    mechanism: BoundMechanism::IntervalComposition,
                },
                BoundPolicyRow {
                    quantity: Quantity::RationalQuotient,
                    mechanism: BoundMechanism::IntervalComposition,
                },
            ],
        }
    }

    /// The five rows, in F2 table order.
    pub fn rows(&self) -> &[BoundPolicyRow] {
        &self.rows
    }

    /// The frozen row for a quantity, when it is in the table.
    pub fn row_for(&self, quantity: Quantity) -> Option<&BoundPolicyRow> {
        self.rows.iter().find(|row| row.quantity == quantity)
    }
}

/// The bounded surface patch a per-quantity bound would be evaluated over.
///
/// Phase 0 carries the patch identity only; the hull-bounded derivative
/// patches arrive with the Phase-1 numerical work. The freeze performs no
/// numerics, so no hull data is needed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedSurfaceInput {
    /// A stable per-patch index.
    pub patch_index: usize,
}

/// F2: the per-quantity bound, dispatching on the frozen table.
///
/// The freeze performs no numerics, so every call refuses. A quantity in the
/// frozen table pins its sanctioned mechanism but has no Phase-0 evaluation —
/// requesting the numeric bound is a construction outside the frozen rules
/// (`InvalidInput`). A quantity outside the table is a SPEC_GAP (`Unfrozen`);
/// widening the table is an orchestrator spec edit, never a worker decision.
pub fn certified_bound(
    quantity: Quantity,
    _patch: &BoundedSurfaceInput,
) -> Result<IntervalEnclosure, Refusal> {
    match BoundPolicy::frozen().row_for(quantity) {
        Some(_) => Err(Refusal::InvalidInput),
        None => Err(Refusal::Unfrozen),
    }
}

/// F3: which coordinate runs the square system, and why.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContinuationCoordinate {
    /// 0-based coordinate index.
    pub index: usize,
    /// The certified away-from-zero margin of ∂H_i/∂t_i over the box, relative
    /// to the box's t_i extent. `Method::Interval`.
    pub relative_margin: IntervalEnclosure,
}

/// F3: the input to a per-box coordinate selection: a SQUARE 3×3 system's
/// certified diagonal-derivative data over the box.
///
/// The class-2 Krawczyk operator runs on square 3×3 systems only; the
/// pseudo-inverse-preconditioned rectangular route is rejected (F3).
#[derive(Debug, Clone, PartialEq)]
pub struct SquareSystemInput {
    /// The certified ∂H_i/∂t_i enclosures over the box, one per coordinate.
    pub diagonal_derivatives: [IntervalEnclosure; 3],
    /// The box extent in each coordinate t_i, strictly positive.
    pub extents: [PositiveFinite; 3],
}

/// F3: a turning-point switch. Both fields are REQUIRED certificates — there
/// is no default, no `Option`, no reseed path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoordinateSwitch {
    /// The outgoing coordinate's certificate.
    pub outgoing: ContinuationCoordinate,
    /// The incoming coordinate's certificate.
    pub incoming: ContinuationCoordinate,
}

/// F3: the per-box coordinate selection. Deterministic.
///
/// The coordinate `i` whose certified ∂H_i/∂t_i enclosure over the box is
/// strictly away from zero with the LARGEST relative margin
/// (|lower bound| / box extent in t_i); ties break to the LOWEST index (no
/// hash order). If NO coordinate certifies away-from-zero, the box refuses
/// [`Refusal::ConditioningBelowThreshold`] — it is never retried with a weaker
/// test.
///
/// Phase 0 computes the *decision* from the certified inputs (the selection is
/// exact); the numeric margin it attaches is a provisional degenerate interval
/// — Phase 1 replaces the float ratio with the directed-rounded interval
/// division, keeping the `Method::Interval` tag this freeze pins.
pub fn select_continuation_coordinate(
    system: &SquareSystemInput,
) -> Result<ContinuationCoordinate, Refusal> {
    let mut best: Option<(usize, f64)> = None;
    for (index, derivative) in system.diagonal_derivatives.iter().enumerate() {
        let lower = derivative.lower().get();
        let upper = derivative.upper().get();
        let strictly_away_from_zero = lower > 0.0 || upper < 0.0;
        if !strictly_away_from_zero {
            continue;
        }
        let margin = lower.abs() / system.extents[index].get();
        if best.is_none_or(|(_, best_margin)| margin > best_margin) {
            best = Some((index, margin));
        }
    }
    let (index, margin) = best.ok_or(Refusal::ConditioningBelowThreshold)?;
    let relative_margin =
        IntervalEnclosure::new(margin, margin).map_err(|_| Refusal::InvalidInput)?;
    Ok(ContinuationCoordinate {
        index,
        relative_margin,
    })
}
