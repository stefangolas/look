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

//! The construct refusal vocabulary (CC-000-CONTRACT, spine decision C4).
//!
//! The theory §9 taxonomy lands as a dedicated enum — NOT as new variants on
//! `truck_base::evidence::Refusal` (whose envelope is frozen and consumed
//! workspace-wide) and NOT on `kernel::evidence::Refusal` (KV2-scoped).
//!
//! **C4 mapping note.** `RankDeficientContact` and `ConditioningBelowThreshold`
//! coexist deliberately: the construct enum carries the theory name, while the
//! conversion to `contract::Refusal::ConditioningBelowThreshold` at the frozen
//! contract boundary is documented, not conflated.
//!
//! **Freeze.** The variant set below is frozen in this packet. Growth is a
//! CC-000 amendment, never a wave-worker decision.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`.

/// A certified construction's refusal vocabulary (spine decision C4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructRefusal {
    /// A weight field that does not certify strictly positive over its
    /// domain (loft/canal weight-field admission).
    NonPositiveWeightField,
    /// A banded collocation system with a pivot interval containing zero
    /// (P1 banded solve, seam S3).
    SingularInterpolationSystem,
    /// Section correspondence cannot be resolved: the argmin enclosures
    /// overlap, never a proximity tie-break (seam S9).
    AmbiguousCorrespondence,
    /// A focal degeneracy of the construction (focal surface / evolute
    /// contact geometry).
    FocalDegeneracy,
    /// A canal-surface radius law that is not regular along the arc
    /// (seam S10).
    CanalSingular,
    /// The k=3 contact system is rank-deficient (seam S11).
    RankDeficientContact,
    /// A contact with a face outside the intended support set.
    UnintendedContact,
    /// The piece union is not an embedded graph star over the projection
    /// (graph-disk refusal path).
    StarNotEmbedded,
    /// No admissible projection plane for the disk pieces (seam S6 refusal
    /// path).
    NoAdmissibleProjection,
    /// A non-generic thickness event in the offset/shell construction.
    NonGenericThicknessEvent,
    /// The ordering of thickness events is ambiguous on the enclosure
    /// evidence (P4 argmin-with-margin, seam S5).
    AmbiguousEventOrdering,
    /// The request is invalid input: outside a frozen rule.
    InvalidInput,
    /// A certified margin or conditioning bound fell below the normative
    /// threshold (maps to `contract::Refusal::ConditioningBelowThreshold` at
    /// the frozen-contract boundary — documented, not conflated).
    ConditioningBelowThreshold,
    /// The refusing-stub marker (spine decision C7): a type or function that
    /// is frozen now but whose production belongs to a later wave packet.
    Unfrozen,
}

impl ConstructRefusal {
    /// The stable diagnostic tag: the variant name itself (the
    /// `MapRefusal::tag` precedent in `certified_map.rs`).
    pub fn tag(self) -> &'static str {
        match self {
            Self::NonPositiveWeightField => "NonPositiveWeightField",
            Self::SingularInterpolationSystem => "SingularInterpolationSystem",
            Self::AmbiguousCorrespondence => "AmbiguousCorrespondence",
            Self::FocalDegeneracy => "FocalDegeneracy",
            Self::CanalSingular => "CanalSingular",
            Self::RankDeficientContact => "RankDeficientContact",
            Self::UnintendedContact => "UnintendedContact",
            Self::StarNotEmbedded => "StarNotEmbedded",
            Self::NoAdmissibleProjection => "NoAdmissibleProjection",
            Self::NonGenericThicknessEvent => "NonGenericThicknessEvent",
            Self::AmbiguousEventOrdering => "AmbiguousEventOrdering",
            Self::InvalidInput => "InvalidInput",
            Self::ConditioningBelowThreshold => "ConditioningBelowThreshold",
            Self::Unfrozen => "Unfrozen",
        }
    }
}
