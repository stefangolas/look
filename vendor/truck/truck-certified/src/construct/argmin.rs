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

//! The P4 argmin-with-margin operator (CC-003-ARGMIN, spine seam S5).
//!
//! **Theory §1 P4 contract (verbatim in substance).** The operator certifies
//! STRICT SEPARATION, never intent: it returns `i*` only if
//! `sup[λ_{i*}] < inf[λ_j]` for every `j != i*`; enclosure overlap — including
//! a tie in the supremum among distinct indices, which leaves the argmin
//! undecided — is a typed refusal, never a proximity tie-break, never a value
//! comparison, never an epsilon slack, and never a "closest wins" fallback.
//!
//! The consumer contracts (CC-013 cyclic correspondence disambiguation,
//! CC-026 thickness event selection, CC-030 blend event ordering) use this
//! operator to disambiguate cyclic shifts, event orderings, and thickness
//! candidates; every consumer must handle the refusal as a typed outcome —
//! [`ConstructRefusal::AmbiguousEventOrdering`] — never as a fallback
//! heuristic.
//!
//! **Determinism (C9).** The candidate scan is a single forward pass in index
//! order: no reordering, no sorting, no hash-iteration-dependent output.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`.

use crate::construct::refusal::ConstructRefusal;
use crate::construct::Interval;

/// Select the unique strictly separated argmin of a set of certified interval
/// enclosures (spine seam S5, theory §1 P4 argmin-with-margin).
///
/// The operator certifies strict separation, never intent: it returns `i*`
/// only if `sup[λ_{i*}] < inf[λ_j]` for every `j != i*`. The candidate `i*`
/// is the index whose enclosure's UPPER bound is the smallest, chosen by one
/// deterministic forward scan in index order (no reordering, no sorting); the
/// strict `<` update keeps the first index on a tied supremum, and that tie is
/// never broken by value — it refuses in the separation check below. After
/// selection the strict-separation condition is verified against every other
/// index; any violation — overlap, a touching boundary, or a tied supremum
/// among distinct indices — refuses. There is no epsilon slack and no "closest
/// wins" fallback.
///
/// # Errors
///
/// * An empty slice, or any enclosure bound that is `NaN` or non-finite,
///   refuses [`ConstructRefusal::InvalidInput`].
/// * Enclosures that are not strictly separated refuse
///   [`ConstructRefusal::AmbiguousEventOrdering`].
pub fn argmin_margin(enclosures: &[Interval]) -> Result<usize, ConstructRefusal> {
    if enclosures.is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }

    // One forward scan in index order (C9). NaN or non-finite bounds anywhere
    // refuse InvalidInput. The candidate is the index with the strictly
    // smallest upper bound; a strict `<` update never breaks a tied supremum —
    // the tie surfaces as AmbiguousEventOrdering in the separation check.
    let mut candidate = 0usize;
    let mut candidate_sup = enclosures[0].hi;
    for (k, enclosure) in enclosures.iter().enumerate() {
        if !enclosure.lo.is_finite() || !enclosure.hi.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
        if enclosure.hi < candidate_sup {
            candidate = k;
            candidate_sup = enclosure.hi;
        }
    }

    // Strict separation check: sup[i*] < inf[j] for every j != i*. Any
    // violation refuses AmbiguousEventOrdering — the certified refusal, never
    // a fallback pick.
    for (j, enclosure) in enclosures.iter().enumerate() {
        if j != candidate && candidate_sup >= enclosure.lo {
            return Err(ConstructRefusal::AmbiguousEventOrdering);
        }
    }

    Ok(candidate)
}
