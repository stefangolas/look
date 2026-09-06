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

//! The one-witness exact verifier (CTE-001-QPOLY; theory §4.2, scope decision
//! 2 and 4).
//!
//! This module implements the frozen one-verifier contract
//! [`ExactWitnessVerifier`] over the ℚ-polynomial substrate [`QPoly`]: it
//! expands `s·f − (q²a + Σᵢ wᵢPᵢ)` over ℚ and compares coefficients to zero
//! **exactly**, then enforces the interval nonvanishing predicates `0 ∉ ŝ(B)`
//! and (when `q` is present) `0 ∉ â(B)` against the caller-supplied
//! enclosures. ONE code path — the [`WitnessSide`] tag selects nothing except
//! documentation. Producers never verify; the verifier never produces.
//!
//! The verifier is *total and side-parametric*: every failure — a polynomial
//! shape that disagrees with the supplied system polynomials, an identity
//! whose residual does not vanish exactly, a missing enclosure, or an
//! enclosure containing `0` — is a [`WitnessRefusal`], never a panic and never
//! an unchecked assumption.
//!
//! **Pending refusal.** The provenance fast path is wired as a stub behind
//! [`provenance_witness`], which refuses with the named pending cause
//! [`WITNESS_PRODUCER_PACKET_PENDING`] until CTE-005/007 wire real provenance
//! (the `cone_torus_carrier_packet_pending` precedent, `kernel/rational.rs`).
//! Normal-form/RUR producers are likewise booked open (theory §6.1) and are
//! NOT implemented here.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`,
//! and adds no module-level `allow`.

use crate::contract::Refusal;
use crate::tangency::qpoly::QPoly;
use crate::tangency::shapes::{ExactVanishingWitness, ExactWitnessVerifier, WitnessSide};

/// The named pending cause under which [`provenance_witness`] refuses: the
/// provenance fast path is CTE-005/007's work, so the producing stub refuses
/// until then (the `a2_branch_packet_pending` / `cone_torus_carrier_packet_pending`
/// precedent).
pub const WITNESS_PRODUCER_PACKET_PENDING: &str = "witness_producer_packet_pending";

/// The CTE-001 refusal vocabulary (H-2): named cases only, no catch-all.
///
/// - [`WitnessRefusal::Overflow`] — exact `i128` coefficient arithmetic
///   overflowed: a named event (never a wrap, never a panic). A real overflow
///   on a fixture is a degree/coefficient-scale decision the loop must make.
/// - [`WitnessRefusal::InvalidInput`] — a construction or call outside a
///   frozen rule: an arity mismatch, a denominator of zero, a missing
///   nonvanishing enclosure, an enclosure containing `0`, or a witness whose
///   recorded polynomials disagree with the system polynomials supplied to the
///   verifier.
/// - [`WitnessRefusal::Pending`] — a producer that is booked open and refuses
///   with its named pending cause until its owning packet lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessRefusal {
    /// Exact `i128` coefficient arithmetic overflowed.
    Overflow,
    /// A construction or verification outside a frozen rule.
    InvalidInput,
    /// A producer that is booked open; carries the named pending cause.
    Pending(&'static str),
}

impl WitnessRefusal {
    /// A short stable diagnostic tag.
    pub const fn tag(self) -> &'static str {
        match self {
            WitnessRefusal::Overflow => "witness_coefficient_overflow",
            WitnessRefusal::InvalidInput => "witness_invalid_input",
            WitnessRefusal::Pending(cause) => cause,
        }
    }
}

/// Whether an interval strictly excludes `0` — the strict-exclusion rule of
/// `KrawczykCertificate3`'s determinant and `Rank2Chart`'s `det_a`: an
/// enclosure containing `0` inclusive of a `0` endpoint does not certify
/// nonvanishing. A non-finite or misordered interval also does not.
fn strictly_excludes_zero((lo, hi): (f64, f64)) -> bool {
    lo.is_finite() && hi.is_finite() && lo <= hi && !(lo <= 0.0 && 0.0 <= hi)
}

/// Map an exactness-layer refusal onto the shape layer's [`Refusal`]
/// vocabulary (the `ExactWitnessVerifier` contract is frozen on
/// `contract::Refusal`; the named CTE-001 cause is recorded here and in the
/// mapping table).
fn to_refusal(_: WitnessRefusal) -> Refusal {
    Refusal::InvalidInput
}

/// Expand `s·f − (q²a + Σᵢ wᵢ·Pᵢ)` over ℚ and return whether it is exactly
/// zero.
///
/// The correction polynomials `Pᵢ` are taken from `p` (the system
/// polynomials the caller is certifying against); each must equal the
/// polynomial recorded in the witness's `i`-th term, else the witness is not
/// evidence about the supplied system and the call refuses.
fn residual_is_zero(
    witness: &ExactVanishingWitness<QPoly>,
    f: &QPoly,
    p: &[&QPoly],
) -> Result<bool, WitnessRefusal> {
    let terms = witness.terms();
    if terms.len() != p.len() {
        return Err(WitnessRefusal::InvalidInput);
    }
    for ((_w, recorded), &given) in terms.iter().zip(p.iter()) {
        if recorded != given {
            return Err(WitnessRefusal::InvalidInput);
        }
    }

    let s = witness.s();
    let lhs = s.mul(f)?;

    let mut rhs = QPoly::zero(f.arity());
    if let Some((q, a)) = witness.q2a() {
        rhs = rhs.add(&q.mul(q)?.mul(a)?)?;
    }
    for ((w, _), &pi) in terms.iter().zip(p.iter()) {
        rhs = rhs.add(&w.mul(pi)?)?;
    }
    let diff = lhs.sub(&rhs)?;
    Ok(diff.is_zero())
}

impl ExactWitnessVerifier<QPoly> for ExactVanishingWitness<QPoly> {
    fn verify(
        &self,
        f: &QPoly,
        p: &[&QPoly],
        s_encl: Option<(f64, f64)>,
        a_encl: Option<(f64, f64)>,
    ) -> Result<(), Refusal> {
        // The interval nonvanishing predicates are REQUIRED: ŝ(B) always,
        // and â(B) exactly when the witness carries the q²a term (theory
        // §4.2). A missing enclosure cannot certify nonvanishing.
        let s_encl = s_encl.ok_or(Refusal::InvalidInput)?;
        if !strictly_excludes_zero(s_encl) {
            return Err(Refusal::InvalidInput);
        }
        match (self.q2a().is_some(), a_encl) {
            (true, Some(a_encl)) => {
                if !strictly_excludes_zero(a_encl) {
                    return Err(Refusal::InvalidInput);
                }
            }
            (true, None) | (false, Some(_)) => return Err(Refusal::InvalidInput),
            (false, None) => {}
        }

        let zero = residual_is_zero(self, f, p).map_err(to_refusal)?;
        if zero {
            Ok(())
        } else {
            Err(Refusal::InvalidInput)
        }
    }
}

/// The provenance fast path: a constructor from already-known polynomial data.
///
/// **Pending refusal stub.** Real provenance — construction identity, fillet
/// spine, exact seating constraints — is CTE-005/007's work, so this stub
/// ALWAYS refuses with the named pending cause
/// [`WITNESS_PRODUCER_PACKET_PENDING`] (the `cone_torus_carrier_packet_pending`
/// precedent). No stub logic, no fake results (scope decision 5). When real
/// provenance lands, its identity still flows through the ONE verifier
/// ([`ExactWitnessVerifier`]) — provenance is a hint about what to TRY, never a
/// trust level (scope decision 4).
pub fn provenance_witness(
    _side: WitnessSide,
    _s: QPoly,
    _terms: Vec<(QPoly, QPoly)>,
    _q2a: Option<(QPoly, QPoly)>,
) -> Result<ExactVanishingWitness<QPoly>, WitnessRefusal> {
    Err(WitnessRefusal::Pending(WITNESS_PRODUCER_PACKET_PENDING))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tangency::qpoly::QPoly;
    use crate::tangency::shapes::{ExactVanishingWitness, ExactWitnessVerifier, WitnessSide};

    /// The F5 A₂ hand witness with the multiplier triple `(s, a, w1)`.
    ///
    /// The F5 ground truth is `s = a = 1`, `q = u1`, reduced `f = u1²` on
    /// `G = 0`. In exact integer polynomial data over variables `u1 = x0`,
    /// `v1 = x1`: `q = u1`, `f = u1² + v1` (so on `G = 0`, i.e. `v1 = 0`,
    /// the reduced contact function is exactly `u1²`), `G1 = v1`, `G2 = 0`.
    /// The identity `s·f = q²·a + w1·G1 + w2·G2` holds exactly over ℚ iff
    /// the multipliers agree (`s·(u1² + v1) = a·u1² + w1·v1` requires
    /// `s = a = w1`).
    fn f5_witness(s: i128, a: i128, w1: i128) -> ExactVanishingWitness<QPoly> {
        let arity = 2;
        let u1 = QPoly::from_int_terms(arity, &[(&[1, 0], 1)]).expect("u1 is a valid term");
        let v1 = QPoly::from_int_terms(arity, &[(&[0, 1], 1)]).expect("v1 is a valid term");
        let s_poly = QPoly::from_int_terms(arity, &[(&[0, 0], s)]).expect("the s constant");
        let a_poly = QPoly::from_int_terms(arity, &[(&[0, 0], a)]).expect("the a constant");
        let w1_poly = QPoly::from_int_terms(arity, &[(&[0, 0], w1)]).expect("the w1 constant");
        let terms = vec![(w1_poly, v1), (QPoly::zero(arity), QPoly::zero(arity))];
        ExactVanishingWitness::new(WitnessSide::A2, s_poly, terms, Some((u1, a_poly)))
            .expect("the F5 A2 witness has the A2 term shape")
    }

    /// The system polynomials `f` and `P = (G1, G2)` of the F5 hand witness.
    fn f5_system() -> (QPoly, Vec<QPoly>) {
        let arity = 2;
        let u1_sq = QPoly::from_int_terms(arity, &[(&[2, 0], 1)]).expect("u1^2 is a valid term");
        let v1 = QPoly::from_int_terms(arity, &[(&[0, 1], 1)]).expect("v1 is a valid term");
        let f = u1_sq.add(&v1).expect("u1^2 + v1");
        let g1 = QPoly::from_int_terms(arity, &[(&[0, 1], 1)]).expect("v1 is a valid term");
        let g2 = QPoly::zero(arity);
        (f, vec![g1, g2])
    }

    #[test]
    fn verifier_accepts_f5_hand_witness() {
        let witness = f5_witness(1, 1, 1);
        let (f, p) = f5_system();
        let refs: Vec<&QPoly> = p.iter().collect();
        // s = a = 1: any strictly positive enclosure certifies ŝ and â.
        let result =
            ExactWitnessVerifier::verify(&witness, &f, &refs, Some((0.5, 1.5)), Some((0.5, 1.5)));
        assert!(
            result.is_ok(),
            "the F5 hand witness is an exact identity and must verify: {result:?}"
        );
    }

    #[test]
    fn verifier_rejects_perturbed_identity() {
        // Perturb the multiplier `a` of the F5 hand witness from the constant
        // 1 to the constant 2 while `s` and `w1` stay at 1: the identity
        // s·f = q²·a + Σ wᵢPᵢ no longer holds over ℚ
        // (u1² + v1 ≠ 2·u1² + v1), and the verifier must refuse.
        let perturbed = f5_witness(1, 2, 1);
        let (f, p) = f5_system();
        let refs: Vec<&QPoly> = p.iter().collect();

        let result =
            ExactWitnessVerifier::verify(&perturbed, &f, &refs, Some((0.5, 1.5)), Some((0.5, 1.5)));
        assert!(
            result.is_err(),
            "a witness perturbed by one coefficient must not verify"
        );
    }

    #[test]
    fn nonvanishing_multiplier_is_interval_certified() {
        let witness = f5_witness(1, 1, 1);
        let (f, p) = f5_system();
        let refs: Vec<&QPoly> = p.iter().collect();

        // ŝ containing 0 (interior or at an endpoint) refuses.
        let straddling =
            ExactWitnessVerifier::verify(&witness, &f, &refs, Some((-1.0, 1.0)), Some((0.5, 1.5)));
        assert!(straddling.is_err(), "ŝ straddling 0 must refuse");
        let endpoint_zero =
            ExactWitnessVerifier::verify(&witness, &f, &refs, Some((0.0, 1.5)), Some((0.5, 1.5)));
        assert!(
            endpoint_zero.is_err(),
            "ŝ with a 0 endpoint must refuse (strict exclusion)"
        );

        // â containing 0 refuses.
        let a_straddling =
            ExactWitnessVerifier::verify(&witness, &f, &refs, Some((0.5, 1.5)), Some((-1.0, 1.0)));
        assert!(a_straddling.is_err(), "â straddling 0 must refuse");

        // Strict exclusion accepts.
        let strict =
            ExactWitnessVerifier::verify(&witness, &f, &refs, Some((0.5, 1.5)), Some((0.5, 1.5)));
        assert!(strict.is_ok(), "strictly excluded multipliers must certify");

        // A strictly negative ŝ/â pair also certifies: the exact identity is
        // signed, so the all-negative witness (s = a = w1 = −1) verifies
        // against strictly negative enclosures.
        let negative = f5_witness(-1, -1, -1);
        let strict_negative = ExactWitnessVerifier::verify(
            &negative,
            &f,
            &refs,
            Some((-1.5, -0.5)),
            Some((-1.5, -0.5)),
        );
        assert!(
            strict_negative.is_ok(),
            "strictly negative multipliers also exclude 0 and certify"
        );

        // A missing mandatory enclosure refuses (it cannot certify).
        let missing_s = ExactWitnessVerifier::verify(&witness, &f, &refs, None, Some((0.5, 1.5)));
        assert!(missing_s.is_err(), "a missing ŝ must refuse");
        let missing_a = ExactWitnessVerifier::verify(&witness, &f, &refs, Some((0.5, 1.5)), None);
        assert!(
            missing_a.is_err(),
            "an A2 witness without â must refuse (q²a present)"
        );
    }

    #[test]
    fn provenance_producer_refuses_pending() {
        assert_eq!(
            WITNESS_PRODUCER_PACKET_PENDING,
            "witness_producer_packet_pending"
        );
        let arity = 2;
        let u1 = QPoly::from_int_terms(arity, &[(&[1, 0], 1)]).expect("u1 is a valid term");
        let refused = provenance_witness(
            WitnessSide::A2,
            QPoly::one(arity),
            vec![
                (QPoly::zero(arity), QPoly::zero(arity)),
                (QPoly::zero(arity), QPoly::zero(arity)),
            ],
            Some((u1, QPoly::one(arity))),
        );
        match refused {
            Err(WitnessRefusal::Pending(cause)) => {
                assert_eq!(cause, WITNESS_PRODUCER_PACKET_PENDING);
                assert_eq!(cause, "witness_producer_packet_pending");
            }
            other => panic!(
                "the provenance producer must refuse with the named pending cause, got {other:?}"
            ),
        }
    }
}
