//! The Contact Fast-Path (CFP) program substrate (CFP-000-SPINE).
//!
//! This module freezes the shapes, fixture kit, gate definitions, instrument
//! schema, and the `SplineSsiEntry` slot decision of the CFP program BEFORE
//! any geometry lands (`docs/CONTACT_FAST_PATH_BUILD_SPEC.md` §2). The five
//! later packets (CFP-001/003/004/006 and their serial followers) build
//! against this module and never restate it.
//!
//! **D-shim (scope decision 1, verbatim).** Types and refusing constructors
//! only — nothing here evaluates, solves, isolates, or certifies numerically.
//! Any method that would evaluate NUMERICALLY refuses (`InvalidInput`-shaped
//! or a named case from the existing vocabularies). The module doc says
//! verbatim: "This module freezes shapes; the CFP wave packets implement
//! against it and never restate it."
//!
//! **Zero new top-level evidence kinds.** The refusal vocabulary wraps landed
//! causes verbatim (decision 3, [`spine::LocalizedRefusal`]); no `Refusal`,
//! `UnresolvedWitness`, `EnvelopeCase`, or `Prop` arm is added. The two new
//! `Method` provenance tags of the program (`ImplicitReduction`,
//! `ConeCertificate`) are booked as mapping rows in
//! `docs/CERTIFICATE_MAPPING.md` section C — certificate-provenance records of
//! the producing packets, not widenings of the base evidence types (decision
//! 7).
//!
//! **SFC (decision 2).** The search/certificate split is type-level:
//! [`spine::FloatHint`] carries float search results with no evidence status
//! and cannot occupy an evidence position; the exact certificate types are the
//! landed evidence vocabulary.

#[doc(hidden)]
pub mod fixtures;
pub mod spine;

pub use spine::{
    Box4, CarrierClass, ConeVerdict, ContactEventSet, FloatHint, FloatSearchValue,
    InstrumentCounters, LocalizedRefusal, PairDescriptor, SplineSsiEntryDispatch, StratumSide,
    V5AdjudicationRefusal, V5BooleanDiff,
};
