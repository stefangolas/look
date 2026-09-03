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

//! The kernel-v2 claim/refusal algebra (§2) and the §17 refusal taxonomy
//! (BG-KV2-000-CONTRACT).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **D-shim.** Types and refusing constructors only. Any method that would
//! evaluate, solve, isolate, or certify NUMERICALLY refuses with a named
//! `RefusalKind` (or returns `RefusalKind`-carrying data for later use). The
//! §2 rules 2/4/6 are enforced by shape here: `Inconclusive` is a verdict
//! variant (a refusal is never a "silent no"), evidence names a residual when
//! one is at fault, accepted objects carry no refusal, and `PartialGraph` only
//! ever appears inside `Refusal::partial`.
//!
//! **Refusal-kind discipline.** The 25 §17 variants are used for geometry and
//! topology *outcomes*; a refusing constructor that is given caller data
//! violating a numeric precondition refuses with the variant that names the
//! violated class and carries the specifics in a
//! [`RefusalEvidence::Predicate`] name (never inventing an outcome kind for a
//! caller bug):
//!
//! * a required-strictly-positive scalar that is `<= 0`, or a required-nonzero
//!   scalar that is exactly `0` → [`RefusalKind::WeightDegenerate`] (Disproven);
//! * non-finite data → [`RefusalKind::NonFinite`] (Disproven);
//! * structural violations (inverted bounds, count mismatch, out-of-range
//!   angle, non-unit direction, empty data, ...) → [`RefusalKind::ClaimRefuted`]
//!   (Disproven) — the input data refutes the certificate the constructor
//!   would have issued;
//! * an acceptance ceiling that is not met ([`crate::kernel::config::RHO_MAX`],
//!   [`crate::kernel::config::TOL_INTERSECTION`]) → the variant whose class the
//!   ceiling belongs to (typically [`RefusalKind::Conditioning`], Inconclusive).

use crate::kernel::graph::PartialGraph;
use crate::kernel::patch::IBox2;
use crate::kernel::residual::ResidualId;

/// A proposition about an object that already exists (spec §2): it is either
/// certified true, certified false, or left undecided.
///
/// The three-way split is the §2 rule 2 shape — no boolean, no silent default:
/// `Proven` carries the certificate, `Disproven` carries the refuting
/// evidence, `Inconclusive` carries the residual-based reason.
#[derive(Debug, Clone)]
pub enum ClaimVerdict<T, E, R> {
    /// The proposition is certified true.
    Proven(T),
    /// The proposition is certified false; `E` is the refuting certificate.
    Disproven(E),
    /// The proposition is neither proven nor refuted; `R` names why.
    Inconclusive(R),
}

/// The outcome of an attempt to construct an object (spec §2): an accepted
/// construction carries the object, a refused one carries a [`Refusal`].
///
/// Accepted objects never carry a refusal, and every refusal is a named
/// [`RefusalKind`] with a backing [`VerdictClass`] (rules 2/4/6 by shape).
pub type Construction<T> = Result<T, Refusal>;

/// The verdict class a refusal lands in: the claim is refuted, or the claim
/// could not be decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictClass {
    /// The claim is refuted: the construction disproves it.
    Disproven,
    /// The claim is not decidable from the current evidence.
    Inconclusive,
}

/// A refusal of a certified construction or claim (spec §17 taxonomy).
///
/// Every refusal names its [`kind`](Self::kind), the [`backing`](Self::backing)
/// class (§17 table), the concrete [`evidence`](Self::evidence) that grounds
/// it, and — only for a construction that was refused mid-way through a graph
/// build — the [`partial`](Self::partial) graph assembled so far. A
/// `PartialGraph` only ever appears inside `Refusal::partial` (rule 6).
#[derive(Debug, Clone)]
pub struct Refusal {
    /// The §17 refusal kind.
    pub kind: RefusalKind,
    /// The §17 backing class of this kind.
    pub backing: VerdictClass,
    /// The concrete evidence grounding the refusal.
    pub evidence: RefusalEvidence,
    /// The partially assembled graph, present only when a construction was
    /// refused mid-build. Absent for certificate and predicate refusals.
    pub partial: Option<PartialGraph>,
}

impl Refusal {
    /// Build a refusal with the kind's §17 default backing.
    pub fn new(kind: RefusalKind, evidence: RefusalEvidence) -> Self {
        let backing = default_backing(kind);
        Self {
            kind,
            backing,
            evidence,
            partial: None,
        }
    }

    /// Build a refusal with an explicit backing class.
    ///
    /// Used where a §17 kind legitimately splits between the two classes —
    /// the §7.1 `WeightDegenerate` Disproven-or-Inconclusive pair — and for
    /// the load-bearing §8.3 `R2_never_reaches_C2` refusal.
    pub fn with_backing(
        kind: RefusalKind,
        backing: VerdictClass,
        evidence: RefusalEvidence,
    ) -> Self {
        Self {
            kind,
            backing,
            evidence,
            partial: None,
        }
    }

    /// Attach the partially assembled graph carried by a mid-build refusal.
    pub fn with_partial(mut self, partial: PartialGraph) -> Self {
        self.partial = Some(partial);
        self
    }
}

/// The concrete evidence that grounds a [`Refusal`] (spec §2, §17).
#[derive(Debug, Clone)]
pub enum RefusalEvidence {
    /// A residual certificate refused over a box: the residual id, the box the
    /// certificate ran over, and a fixed note naming the failed invariant.
    Residual {
        /// Which residual of the §7 family was being certified.
        residual: ResidualId,
        /// The box the residual certificate ran over.
        box_: IBox2,
        /// A fixed note naming the failed invariant.
        note: &'static str,
    },
    /// A named predicate refusal with a free-form detail string.
    Predicate {
        /// The stable predicate name (kebab-case, machine-readable).
        name: &'static str,
        /// A human-readable detail of the violated invariant.
        detail: String,
    },
    /// A bare refusal carrying no witness beyond its kind.
    None,
}

/// The §17 refusal taxonomy: all 25 variants, doc-commented with their backing
/// class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefusalKind {
    /// The spine is not C1 at the certified join. Backing: Disproven.
    SpineNotC1,
    /// The spine/frame construction ran into a singular frame. Backing:
    /// Disproven.
    FrameSingular,
    /// The profile collapsed (zero profile extent). Backing: Disproven.
    ProfileCollapse,
    /// The profile correspondence between carriers mismatches. Backing:
    /// Disproven.
    ProfileCorrespondenceMismatch,
    /// A model quantity is not finite where the certificate requires a finite
    /// value. Backing: Disproven.
    NonFinite,
    /// A winding-number audit failed. Backing: Disproven.
    WindingAuditFailed,
    /// A shared-topology request that is not dyadic. Backing: Disproven.
    NonDyadicSharedRequest,
    /// The carrier surface is singular over the box. Backing: Disproven.
    CarrierSingularity,
    /// The chart was exhausted before the step could be certified. Backing:
    /// Disproven.
    ChartExhausted,
    /// The curve/carrier is transcendental-only; no rational certification is
    /// possible. Backing: Disproven.
    TranscendentalCarrier,
    /// A weight (or required-positive quantity) degenerated to or through
    /// zero. Backing: Disproven (of the positive certificate); the §7.1
    /// straddle variant may be re-classed Inconclusive via `with_backing`.
    WeightDegenerate,
    /// The deck traversal ceiling was hit. Backing: Inconclusive.
    DeckExhausted,
    /// A conditioning bound was exceeded (frame rebuild territory). Backing:
    /// Inconclusive.
    Conditioning,
    /// The curve is tangential (tangent degeneracy). Backing: Inconclusive.
    TangentialCurve,
    /// A higher-order jet is required beyond the certified order. Backing:
    /// Inconclusive.
    HighOrderJet,
    /// The start set is incomplete. Backing: Inconclusive.
    IncompleteStartSet,
    /// An R5 enclosure failed to certify. Backing: Inconclusive.
    R5EnclosureFailed,
    /// A trim/clip operation failed. Backing: Inconclusive.
    TrimClipFailed,
    /// Two objects are near-overlapping (of `ExactSheet`); the exact-sheet
    /// claim is refuted. Backing: Disproven.
    NearOverlap,
    /// The offset is degenerate. Backing: Disproven.
    OffsetDegenerate,
    /// The offset swallowed its own tail. Backing: Disproven.
    OffsetSwallowtail,
    /// A corner of the arrangement is unsolved. Backing: Inconclusive.
    CornerUnsolved,
    /// A sliver or near-overlap prevents the certificate. Backing:
    /// Inconclusive.
    SliverOrNearOverlap,
    /// A certified claim was refuted by the construction's own evidence.
    /// Backing: Disproven.
    ClaimRefuted,
    /// The budget (depth/turns) was exhausted before the certificate closed.
    /// Backing: Inconclusive.
    Budget,
}

/// The §17 backing class of each [`RefusalKind`], exactly per the spec table.
pub fn default_backing(kind: RefusalKind) -> VerdictClass {
    use RefusalKind::*;
    use VerdictClass::*;
    match kind {
        // §17 Inconclusive class.
        DeckExhausted | Conditioning | TangentialCurve | HighOrderJet | IncompleteStartSet
        | R5EnclosureFailed | TrimClipFailed | CornerUnsolved | SliverOrNearOverlap | Budget => {
            Inconclusive
        }
        // Every remaining §17 variant refutes: the constructive/carrier set,
        // the weight/nonzero degeneracies, the near-overlap (of ExactSheet)
        // disproof, and the refuted-claim refusal.
        SpineNotC1
        | FrameSingular
        | ProfileCollapse
        | ProfileCorrespondenceMismatch
        | NonFinite
        | WindingAuditFailed
        | NonDyadicSharedRequest
        | CarrierSingularity
        | ChartExhausted
        | TranscendentalCarrier
        | WeightDegenerate
        | NearOverlap
        | OffsetDegenerate
        | OffsetSwallowtail
        | ClaimRefuted => Disproven,
    }
}
