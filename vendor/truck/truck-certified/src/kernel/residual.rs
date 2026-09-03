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

//! The §7 residual family and the §4.2 implication Rule C (BG-KV2-000-CONTRACT).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **D-shim.** The residual *ids* are frozen here; the residual *bodies* are
//! the wave packets' implementors. `implication` is the complete §4.2 rule: a
//! total function over the relation that admits exactly the identity
//! (`Equivalent`), `R2 ⊒ R1` (`Stronger`), and nothing else — `R8`, `R9`,
//! `R7 ⊒ nothing` per §4.2 ("R8, R9 ⊒ nothing; R7 ⊒ nothing"). The A/B chart
//! variants of R6 are one id: Theorem 13.3's transition is the consumer's
//! concern, not this relation's. The integration contract test pins the full
//! 11×11 table.

/// A residual of the §7 family. `Carrier` is the carrier-geometry residual;
/// `R1`..`R9`, `R4Prime` are the §7 residual certificates (bodies implement in
/// the wave packets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResidualId {
    /// §7 R1 residual.
    R1,
    /// §7 R2 residual.
    R2,
    /// §7 R3 residual.
    R3,
    /// §7 R4 residual.
    R4,
    /// §7 R4′ residual.
    R4Prime,
    /// §7 R5 residual.
    R5,
    /// §7 R6 residual (the A/B chart variants are one id).
    R6,
    /// §7 R7 residual.
    R7,
    /// §7 R8 residual.
    R8,
    /// §7 R9 residual.
    R9,
    /// The carrier-geometry residual.
    Carrier,
}

/// The §4.2 Rule C implication between two residuals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Implication {
    /// The residuals are the same certificate: certifying the stronger
    /// certifies the weaker and vice versa.
    Equivalent,
    /// Certifying the first residual strictly certifies the second.
    Stronger,
    /// No implication holds between the two residuals.
    None,
}

/// §4.2 Rule C: whether certifying `stronger` implies certifying `weaker`.
///
/// The relation admits exactly: identity (`Equivalent`, every residual is
/// equivalent to itself), `R2 ⊒ R1` (`Stronger`), and nothing else — including
/// `R8`, `R9`, and `R7` with anything. `R6` is one id, so its only relations
/// are its self-identities.
pub fn implication(stronger: ResidualId, weaker: ResidualId) -> Implication {
    use Implication::*;
    use ResidualId::*;
    match (stronger, weaker) {
        (R2, R1) => Stronger,
        (a, b) if a == b => Equivalent,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 11 ids, in declaration order.
    const ALL: [ResidualId; 11] = [
        ResidualId::R1,
        ResidualId::R2,
        ResidualId::R3,
        ResidualId::R4,
        ResidualId::R4Prime,
        ResidualId::R5,
        ResidualId::R6,
        ResidualId::R7,
        ResidualId::R8,
        ResidualId::R9,
        ResidualId::Carrier,
    ];

    /// The expected cell of the full 11×11 table.
    fn expect_cell(stronger: ResidualId, weaker: ResidualId) -> Implication {
        if matches!((stronger, weaker), (ResidualId::R2, ResidualId::R1)) {
            Implication::Stronger
        } else if stronger == weaker {
            Implication::Equivalent
        } else {
            Implication::None
        }
    }

    #[test]
    fn implication_table_is_exactly_rule_c() {
        for &stronger in &ALL {
            for &weaker in &ALL {
                assert_eq!(
                    implication(stronger, weaker),
                    expect_cell(stronger, weaker),
                    "implication({stronger:?} ⊒ {weaker:?})"
                );
            }
        }
    }
}
