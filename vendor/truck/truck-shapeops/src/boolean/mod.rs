//! BG-SOL-RW1-MATERIAL: the §13.1 material-state fragment-selection
//! primitive.
//!
//! The Boundary Rewrite decides one boundary fragment from the four material
//! witnesses around it, not from an orientation table. The verdicts are
//! factored through the theory's two-bit carrier algebra
//! (CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md §5.3–§5.5): each operand's
//! witness pair is a [`SideState`], the operation combines the two signatures
//! coordinatewise (§5.4), and the boundary extraction (§5.5) keeps the
//! fragment iff the two result sides differ (it is on the result's boundary),
//! oriented toward the empty (`m_R = 0`) side. No case enumeration. Pure
//! logic: this module decides, it does not touch shapes.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// BG-SOL-RW2-SPLIT: the fragment splitter.
pub mod split;

/// BG-SOL-RW3-CLASSIFY: the §12 fragment classifier (seed-and-propagate over
/// the parity graph, one certified seed per connected component).
pub mod classify;

/// BG-SOL-RW4-ASSEMBLE: the assembler and the `boolean()` entry.
pub mod assemble;

/// DEF-SEEDRAY-A: certified interval ray×carrier crossings for the four
/// seed-ray carriers (standalone; does not touch the live classify path).
pub mod ray_cert;

/// BIE-006-CLASSIFY: the sweep lift/path adapters + windowed sweep output
/// (the pipeline tie-in that lets a `SpineFrameSweep` face through the funnel).
mod sweep_lift;

/// Material membership of one side of a boundary fragment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    /// The side is inside solid A.
    pub in_a: bool,
    /// The side is inside solid B.
    pub in_b: bool,
}

/// The regularized Boolean operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoolOp {
    /// Union.
    Union,
    /// Intersection.
    Intersection,
    /// Difference: A minus B.
    Difference,
    /// Symmetric difference.
    Xor,
}

impl BoolOp {
    /// The truth function: whether a point in this state is material in
    /// the result.
    pub fn eval(&self, s: State) -> bool {
        match self {
            BoolOp::Union => s.in_a || s.in_b,
            BoolOp::Intersection => s.in_a && s.in_b,
            BoolOp::Difference => s.in_a && !s.in_b,
            BoolOp::Xor => s.in_a ^ s.in_b,
        }
    }

    /// The §5.4 coordinatewise combine of the two operands' side signatures
    /// into the result signature: `σ_{A∪B} = σ_A ∨ σ_B`,
    /// `σ_{A∩B} = σ_A ∧ σ_B`, `σ_{A∖B} = σ_A ∧ ¬σ_B`, and
    /// `σ_{A△B} = σ_A △ σ_B`. Per-bit this is exactly [`BoolOp::eval`] on
    /// each side of the fragment.
    pub fn combine(&self, a: SideState, b: SideState) -> SideState {
        match self {
            BoolOp::Union => a.or(b),
            BoolOp::Intersection => a.and(b),
            BoolOp::Difference => a.and(b.flip()),
            BoolOp::Xor => a.xor(b),
        }
    }
}

/// The four material witnesses around ONE boundary fragment, in the
/// fragment's own orientation: `-` is the side its normal points AWAY
/// from, `+` the side it points TO. For a fragment of A's boundary the
/// A pair is `(true, false)` (inside, outside); a coincident fragment
/// carries all four from the classification stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialState4 {
    /// The `-` side is inside A.
    pub a_minus: bool,
    /// The `+` side is inside A.
    pub a_plus: bool,
    /// The `-` side is inside B.
    pub b_minus: bool,
    /// The `+` side is inside B.
    pub b_plus: bool,
}

/// The two-bit side signature `σ_S(α) = (m⁻, m⁺)` of one operand on one atom
/// of a carrier arrangement (theory §5.3): `00` = exterior on both sides,
/// `11` = interior on both sides (the carrier is internal to the operand, not
/// a face of it), `10`/`01` = oriented boundary.
///
/// Stored as the raw two-bit `u8` with the `m⁻` bit in the high place and the
/// `m⁺` bit in the low place, so the state reads as the two-bit number
/// `(m⁻)(m⁺)` (`10` = `0b10` = canonical boundary, `01` = `0b01` = flipped
/// boundary). This is the carrier-relative representation of one operand's
/// witness pair: a [`MaterialState4`] is exactly two of these — one for each
/// operand — and the conversions are lossless both ways. The §5.4 algebra is
/// coordinatewise bitwise (`00..11` occupy distinct bit positions, so meet,
/// join, and symmetric difference are the packed `&`, `|`, `^`); the §5.5
/// boundary extraction is [`extract`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SideState(u8);

impl SideState {
    /// Build a side signature from the two side witnesses `(m⁻, m⁺)`.
    pub fn from_bits(minus: bool, plus: bool) -> Self {
        SideState(((minus as u8) << 1) | (plus as u8))
    }

    /// The two-bit state `00..11` with `m⁻` in the high bit, verbatim.
    pub fn state(self) -> u8 {
        self.0
    }

    /// The `m⁻` side witness (high bit).
    pub fn minus(self) -> bool {
        self.0 & 0b10 != 0
    }

    /// The `m⁺` side witness (low bit).
    pub fn plus(self) -> bool {
        self.0 & 0b01 != 0
    }

    /// Whether the carrier is internal to the operand — both side witnesses
    /// interior (state `11`).
    pub fn mid(self) -> bool {
        self.0 == 0b11
    }

    /// The coordinatewise complement `¬σ` of the §5.4 algebra: `00 ↔ 11` and
    /// `01 ↔ 10`.
    pub fn flip(self) -> Self {
        SideState(self.0 ^ 0b11)
    }

    /// The coordinatewise meet `σ ∧ τ` (theory §5.4): bitwise AND over the
    /// two packed side witnesses.
    pub fn and(self, other: SideState) -> Self {
        SideState(self.0 & other.0)
    }

    /// The coordinatewise join `σ ∨ τ` (theory §5.4): bitwise OR over the two
    /// packed side witnesses.
    pub fn or(self, other: SideState) -> Self {
        SideState(self.0 | other.0)
    }

    /// The coordinatewise symmetric difference `σ △ τ` (theory §5.4): bitwise
    /// XOR over the two packed side witnesses.
    pub fn xor(self, other: SideState) -> Self {
        SideState(self.0 ^ other.0)
    }
}

impl From<MaterialState4> for (SideState, SideState) {
    /// Factor the four witnesses into the two operands' side signatures
    /// `(σ_A, σ_B)` (theory §5.3). Lossless: the reverse conversion recovers
    /// the exact witnesses.
    fn from(m: MaterialState4) -> Self {
        (
            SideState::from_bits(m.a_minus, m.a_plus),
            SideState::from_bits(m.b_minus, m.b_plus),
        )
    }
}

impl From<(SideState, SideState)> for MaterialState4 {
    /// Reassemble the four witnesses from the two operands' side signatures.
    fn from(sides: (SideState, SideState)) -> Self {
        MaterialState4 {
            a_minus: sides.0.minus(),
            a_plus: sides.0.plus(),
            b_minus: sides.1.minus(),
            b_plus: sides.1.plus(),
        }
    }
}

/// What the Boundary Rewrite does with one boundary fragment.
///
/// The codomain is the theory's §5.5 three-valued shape
/// `{Keep{flip}, Discard}` (corollary T2.4): there is no "keep both" in the
/// decision. A coincident carrier pair that is kept is emitted by the
/// assembler as ONE canonical face plus its provenance row (`assemble.rs`,
/// the kept-pair branch), never as two coincident faces — the §5.5
/// corollary's requirement that `KeepBothSplit` means emitting disjoint
/// carrier atoms, not duplicate geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FragmentDecision {
    /// The fragment is on the result's boundary. `flip` says whether its
    /// orientation must be reversed so the normal points toward the
    /// empty (`m_R = 0`) side.
    Keep {
        /// Whether the fragment's orientation must be reversed.
        flip: bool,
    },
    /// Both sides have the same result material: interior or exterior of
    /// the result - the fragment is not on the boundary.
    Discard,
}

/// The §5.5 three-valued boundary extraction on a result side signature:
/// `00`/`11` (equal result material on both sides) drop, `10` keeps canonical
/// orientation, `01` keeps flipped. Equivalently, the extractor is
/// `m_R⁻ = m_R⁺ ⇒ Discard`, else `Keep { flip = ¬m_R⁻ }`. The carrier
/// decision is a function of the signature, never a policy.
pub fn extract(sigma: SideState) -> FragmentDecision {
    if sigma.minus() == sigma.plus() {
        FragmentDecision::Discard
    } else {
        FragmentDecision::Keep {
            flip: !sigma.minus(),
        }
    }
}

/// The §13.1 primitive, factored through the two-bit carrier algebra: factor
/// the four witnesses into the operands' side signatures (theory §5.3),
/// combine them under the operation (§5.4), and extract the boundary verdict
/// (§5.5) — keep iff the result's two sides differ, oriented toward the
/// empty side.
pub fn fragment_decision(op: BoolOp, m: MaterialState4) -> FragmentDecision {
    let sigma_a = SideState::from_bits(m.a_minus, m.a_plus);
    let sigma_b = SideState::from_bits(m.b_minus, m.b_plus);
    extract(op.combine(sigma_a, sigma_b))
}

#[cfg(test)]
mod tests {
    use super::{
        extract, fragment_decision, BoolOp, FragmentDecision, MaterialState4, SideState, State,
    };

    /// The four general-position fragment classes, each named by its
    /// `(A: a_minus,a_plus; B: b_minus,b_plus)` witnesses.
    fn a_outside_b() -> MaterialState4 {
        MaterialState4 {
            a_minus: true,
            a_plus: false,
            b_minus: false,
            b_plus: false,
        }
    }

    fn a_inside_b() -> MaterialState4 {
        MaterialState4 {
            a_minus: true,
            a_plus: false,
            b_minus: true,
            b_plus: true,
        }
    }

    fn b_outside_a() -> MaterialState4 {
        MaterialState4 {
            a_minus: false,
            a_plus: false,
            b_minus: true,
            b_plus: false,
        }
    }

    fn b_inside_a() -> MaterialState4 {
        MaterialState4 {
            a_minus: true,
            a_plus: true,
            b_minus: true,
            b_plus: false,
        }
    }

    /// The coincident orientation variants.
    fn identical() -> MaterialState4 {
        MaterialState4 {
            a_minus: true,
            a_plus: false,
            b_minus: true,
            b_plus: false,
        }
    }

    fn anti() -> MaterialState4 {
        MaterialState4 {
            a_minus: true,
            a_plus: false,
            b_minus: false,
            b_plus: true,
        }
    }

    fn interior() -> MaterialState4 {
        MaterialState4 {
            a_minus: true,
            a_plus: true,
            b_minus: true,
            b_plus: true,
        }
    }

    /// The two sides of a fragment evaluated through the truth function.
    fn sides(op: BoolOp, m: MaterialState4) -> (bool, bool) {
        let m_r_minus = op.eval(State {
            in_a: m.a_minus,
            in_b: m.b_minus,
        });
        let m_r_plus = op.eval(State {
            in_a: m.a_plus,
            in_b: m.b_plus,
        });
        (m_r_minus, m_r_plus)
    }

    #[test]
    fn material_state_reproduces_regularized_orientation_table() {
        let a_out = a_outside_b();
        let a_in = a_inside_b();
        let b_out = b_outside_a();
        let b_in = b_inside_a();

        // Union keeps each solid's own exterior faces unflipped, discards
        // the cavity walls.
        assert_eq!(
            fragment_decision(BoolOp::Union, a_out),
            FragmentDecision::Keep { flip: false }
        );
        assert_eq!(
            fragment_decision(BoolOp::Union, b_out),
            FragmentDecision::Keep { flip: false }
        );
        assert_eq!(
            fragment_decision(BoolOp::Union, a_in),
            FragmentDecision::Discard
        );
        assert_eq!(
            fragment_decision(BoolOp::Union, b_in),
            FragmentDecision::Discard
        );

        // Intersection keeps the cavity walls unflipped, discards the
        // exterior faces.
        assert_eq!(
            fragment_decision(BoolOp::Intersection, a_in),
            FragmentDecision::Keep { flip: false }
        );
        assert_eq!(
            fragment_decision(BoolOp::Intersection, b_in),
            FragmentDecision::Keep { flip: false }
        );
        assert_eq!(
            fragment_decision(BoolOp::Intersection, a_out),
            FragmentDecision::Discard
        );
        assert_eq!(
            fragment_decision(BoolOp::Intersection, b_out),
            FragmentDecision::Discard
        );

        // Difference (A - B) keeps A's exterior unflipped and B's cavity
        // wall FLIPPED: at A-inside-B the A side of the result is empty, so
        // the normal must reverse toward it.
        assert_eq!(
            fragment_decision(BoolOp::Difference, a_out),
            FragmentDecision::Keep { flip: false }
        );
        assert_eq!(
            fragment_decision(BoolOp::Difference, b_in),
            FragmentDecision::Keep { flip: true }
        );
        assert_eq!(
            fragment_decision(BoolOp::Difference, a_in),
            FragmentDecision::Discard
        );
        assert_eq!(
            fragment_decision(BoolOp::Difference, b_out),
            FragmentDecision::Discard
        );

        // Xor keeps each solid's exterior faces and both cavity walls, the
        // cavity walls re-oriented outward of the respective remainder.
        assert_eq!(
            fragment_decision(BoolOp::Xor, a_out),
            FragmentDecision::Keep { flip: false }
        );
        assert_eq!(
            fragment_decision(BoolOp::Xor, b_out),
            FragmentDecision::Keep { flip: false }
        );
        // A-inside-B is the cell most likely to be misremembered: xor keeps
        // the side pairs `op(1,1)=0 / op(0,1)=1` -> keep, flip = !0 = true.
        assert_eq!(
            fragment_decision(BoolOp::Xor, a_in),
            FragmentDecision::Keep { flip: true }
        );
        assert_eq!(
            fragment_decision(BoolOp::Xor, b_in),
            FragmentDecision::Keep { flip: true }
        );
    }

    #[test]
    fn material_state_decides_coincident_fragments() {
        // Coincident fragments carry all four witnesses; every cell falls
        // out of the rule with no special-casing.

        // Identical orientation (A: 1,0; B: 1,0): the solids coincide at
        // this fragment.
        let same = identical();
        // A ∪ A = A: still the result's boundary, no flip.
        assert_eq!(
            fragment_decision(BoolOp::Union, same),
            FragmentDecision::Keep { flip: false }
        );
        // A ∩ A = A: still the result's boundary, no flip.
        assert_eq!(
            fragment_decision(BoolOp::Intersection, same),
            FragmentDecision::Keep { flip: false }
        );
        // A − A = ∅: the coincident face is interior to the empty result.
        assert_eq!(
            fragment_decision(BoolOp::Difference, same),
            FragmentDecision::Discard
        );
        // A △ A = ∅.
        assert_eq!(
            fragment_decision(BoolOp::Xor, same),
            FragmentDecision::Discard
        );

        // Anti-oriented (A: 1,0; B: 0,1): the solids butt against each
        // other at this fragment.
        let butt = anti();
        // The face is interior to the union.
        assert_eq!(
            fragment_decision(BoolOp::Union, butt),
            FragmentDecision::Discard
        );
        // And interior to the (empty) intersection.
        assert_eq!(
            fragment_decision(BoolOp::Intersection, butt),
            FragmentDecision::Discard
        );
        // A − B keeps the face as its boundary, unflipped: the A side is
        // full, the B side empty.
        assert_eq!(
            fragment_decision(BoolOp::Difference, butt),
            FragmentDecision::Keep { flip: false }
        );
        // A △ B: each side is in exactly one solid, so both sides are
        // material and the face is interior to the symmetric difference.
        assert_eq!(
            fragment_decision(BoolOp::Xor, butt),
            FragmentDecision::Discard
        );

        // Fully interior to both (A: 1,1; B: 1,1): degenerate but decidable.
        let deep = interior();
        // Both sides inside A ∪ B: the face is interior.
        assert_eq!(
            fragment_decision(BoolOp::Union, deep),
            FragmentDecision::Discard
        );
        // Both sides inside A ∩ B: the face is interior.
        assert_eq!(
            fragment_decision(BoolOp::Intersection, deep),
            FragmentDecision::Discard
        );
        // Both sides inside B: both sides are excluded from A − B.
        assert_eq!(
            fragment_decision(BoolOp::Difference, deep),
            FragmentDecision::Discard
        );
        // Both sides in both solids: xor is false on both sides.
        assert_eq!(
            fragment_decision(BoolOp::Xor, deep),
            FragmentDecision::Discard
        );
    }

    #[test]
    fn material_state_flips_orient_toward_the_empty_side() {
        // Every Keep cell from the two tests above: (op, witnesses,
        // expected flip).
        let keep_cells: &[(BoolOp, MaterialState4, bool)] = &[
            (BoolOp::Union, a_outside_b(), false),
            (BoolOp::Union, b_outside_a(), false),
            (BoolOp::Intersection, a_inside_b(), false),
            (BoolOp::Intersection, b_inside_a(), false),
            (BoolOp::Difference, a_outside_b(), false),
            (BoolOp::Difference, b_inside_a(), true),
            (BoolOp::Xor, a_outside_b(), false),
            (BoolOp::Xor, a_inside_b(), true),
            (BoolOp::Xor, b_outside_a(), false),
            (BoolOp::Xor, b_inside_a(), true),
            (BoolOp::Union, identical(), false),
            (BoolOp::Intersection, identical(), false),
            (BoolOp::Difference, anti(), false),
        ];
        for &(op, m, expected_flip) in keep_cells {
            let (m_r_minus, m_r_plus) = sides(op, m);
            assert_ne!(m_r_minus, m_r_plus);
            assert_eq!(
                fragment_decision(op, m),
                FragmentDecision::Keep {
                    flip: expected_flip
                }
            );
            // With m_R_minus != m_R_plus, applying `flip` makes the
            // outward (pointed-to) side the one whose result material is
            // false: the empty side.
            let outward_material = if expected_flip { m_r_minus } else { m_r_plus };
            assert!(
                !outward_material,
                "kept fragment must orient toward the empty side"
            );
        }

        // Definitional completeness: over all 16 witness combinations and
        // all four ops, the decision is Discard iff the two sides evaluate
        // equal through `BoolOp::eval`.
        for a_minus in [true, false] {
            for a_plus in [true, false] {
                for b_minus in [true, false] {
                    for b_plus in [true, false] {
                        let m = MaterialState4 {
                            a_minus,
                            a_plus,
                            b_minus,
                            b_plus,
                        };
                        for op in [
                            BoolOp::Union,
                            BoolOp::Intersection,
                            BoolOp::Difference,
                            BoolOp::Xor,
                        ] {
                            let (m_r_minus, m_r_plus) = sides(op, m);
                            match fragment_decision(op, m) {
                                FragmentDecision::Discard => {
                                    assert_eq!(m_r_minus, m_r_plus);
                                }
                                FragmentDecision::Keep { flip } => {
                                    assert_ne!(m_r_minus, m_r_plus);
                                    let outward = if flip { m_r_minus } else { m_r_plus };
                                    assert!(
                                        !outward,
                                        "kept fragment must orient toward the empty side"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// The pre-refactor `fragment_decision`: evaluate the truth function on
    /// each side (`sides`) and apply the §5.5 extractor
    /// `m_R⁻ = m_R⁺ ⇒ Discard`, else `Keep { flip = ¬m_R⁻ }`.
    fn reference_fragment_decision(op: BoolOp, m: MaterialState4) -> FragmentDecision {
        let (m_r_minus, m_r_plus) = sides(op, m);
        if m_r_minus == m_r_plus {
            FragmentDecision::Discard
        } else {
            FragmentDecision::Keep { flip: !m_r_minus }
        }
    }

    #[test]
    fn side_state_from_material_state4_is_lossless() {
        // Every one of the 16 witness combinations factors into a `(σ_A, σ_B)`
        // pair of two-bit signatures and reassembles unchanged.
        for a_minus in [true, false] {
            for a_plus in [true, false] {
                for b_minus in [true, false] {
                    for b_plus in [true, false] {
                        let m = MaterialState4 {
                            a_minus,
                            a_plus,
                            b_minus,
                            b_plus,
                        };
                        let (sigma_a, sigma_b) = <(SideState, SideState)>::from(m);
                        assert_eq!(sigma_a.minus(), m.a_minus);
                        assert_eq!(sigma_a.plus(), m.a_plus);
                        assert_eq!(sigma_b.minus(), m.b_minus);
                        assert_eq!(sigma_b.plus(), m.b_plus);
                        assert_eq!(MaterialState4::from((sigma_a, sigma_b)), m);
                    }
                }
            }
        }
    }

    #[test]
    fn truth_rows_reproduced_by_exhaustion() {
        // The eight derived truth rows of §5.9, relative to a fixed carrier
        // orientation: `(σ_A, σ_B, op)` → result signature and the extraction
        // action each row records.
        let rows: &[(SideState, SideState, BoolOp, SideState, FragmentDecision)] = &[
            (
                SideState::from_bits(true, false),
                SideState::from_bits(true, false),
                BoolOp::Union,
                SideState::from_bits(true, false),
                FragmentDecision::Keep { flip: false },
            ),
            (
                SideState::from_bits(true, false),
                SideState::from_bits(true, false),
                BoolOp::Intersection,
                SideState::from_bits(true, false),
                FragmentDecision::Keep { flip: false },
            ),
            (
                SideState::from_bits(true, false),
                SideState::from_bits(true, false),
                BoolOp::Difference,
                SideState::from_bits(false, false),
                FragmentDecision::Discard,
            ),
            (
                SideState::from_bits(true, false),
                SideState::from_bits(false, true),
                BoolOp::Union,
                SideState::from_bits(true, true),
                FragmentDecision::Discard,
            ),
            (
                SideState::from_bits(true, false),
                SideState::from_bits(false, true),
                BoolOp::Intersection,
                SideState::from_bits(false, false),
                FragmentDecision::Discard,
            ),
            (
                SideState::from_bits(true, false),
                SideState::from_bits(false, true),
                BoolOp::Difference,
                SideState::from_bits(true, false),
                FragmentDecision::Keep { flip: false },
            ),
            (
                SideState::from_bits(true, true),
                SideState::from_bits(true, false),
                BoolOp::Difference,
                SideState::from_bits(false, true),
                FragmentDecision::Keep { flip: true },
            ),
            (
                SideState::from_bits(false, false),
                SideState::from_bits(true, false),
                BoolOp::Union,
                SideState::from_bits(true, false),
                FragmentDecision::Keep { flip: false },
            ),
        ];
        for &(sigma_a, sigma_b, op, result, decision) in rows {
            assert_eq!(op.combine(sigma_a, sigma_b), result);
            assert_eq!(
                fragment_decision(op, MaterialState4::from((sigma_a, sigma_b))),
                decision
            );
        }

        // Full exhaustion over all 4×4×4 `(σ_A, σ_B, op)` triples in fixed
        // order: the combined result signature's bits are the per-side truth
        // values, and `extract` is exactly the §5.5 extractor
        // `m_R⁻ = m_R⁺ ⇒ Discard`, else `Keep { flip = ¬m_R⁻ }`.
        for raw_a in 0u8..4 {
            for raw_b in 0u8..4 {
                let sigma_a = SideState::from_bits((raw_a & 0b10) != 0, (raw_a & 0b01) != 0);
                let sigma_b = SideState::from_bits((raw_b & 0b10) != 0, (raw_b & 0b01) != 0);
                let m = MaterialState4::from((sigma_a, sigma_b));
                for op in [
                    BoolOp::Union,
                    BoolOp::Intersection,
                    BoolOp::Difference,
                    BoolOp::Xor,
                ] {
                    let sigma_r = op.combine(sigma_a, sigma_b);
                    let (m_r_minus, m_r_plus) = sides(op, m);
                    assert_eq!(sigma_r.minus(), m_r_minus);
                    assert_eq!(sigma_r.plus(), m_r_plus);
                    let decision = extract(sigma_r);
                    match decision {
                        FragmentDecision::Discard => {
                            assert_eq!(m_r_minus, m_r_plus);
                        }
                        FragmentDecision::Keep { flip } => {
                            assert_ne!(m_r_minus, m_r_plus);
                            assert_eq!(flip, !m_r_minus);
                        }
                    }
                    assert_eq!(fragment_decision(op, m), decision);
                }
            }
        }
    }

    #[test]
    fn difference_is_not_associative_pin() {
        // Theory §5.7 / T2.5: difference is not associative in the two-bit
        // carrier algebra, mirroring the sets:
        // `(σ_A ∧ ¬σ_B) ∧ ¬σ_C ≠ σ_A ∧ ¬(σ_B ∧ ¬σ_C)` on the witness triple
        // `(11, 11, 11)`. This is correct behaviour — `(A∖B)∖C ≠ A∖(B∖C)` —
        // deliberately not "fixed" by reassociation.
        let a = SideState::from_bits(true, true);
        let b = SideState::from_bits(true, true);
        let c = SideState::from_bits(true, true);
        let left_grouped = a.and(b.flip()).and(c.flip());
        let right_grouped = a.and(b.and(c.flip()).flip());
        assert_eq!(left_grouped, SideState::from_bits(false, false));
        assert_eq!(right_grouped, SideState::from_bits(true, true));
        assert_ne!(left_grouped, right_grouped);
    }

    #[test]
    fn fragment_decision_congruence_after_refactor() {
        // The refactor is verdict-preserving: the SideState-path
        // `fragment_decision` agrees with the pre-refactor eval-per-side
        // extractor on every one of the 16 witness combinations under all
        // four operations.
        for a_minus in [true, false] {
            for a_plus in [true, false] {
                for b_minus in [true, false] {
                    for b_plus in [true, false] {
                        let m = MaterialState4 {
                            a_minus,
                            a_plus,
                            b_minus,
                            b_plus,
                        };
                        for op in [
                            BoolOp::Union,
                            BoolOp::Intersection,
                            BoolOp::Difference,
                            BoolOp::Xor,
                        ] {
                            assert_eq!(
                                fragment_decision(op, m),
                                reference_fragment_decision(op, m)
                            );
                        }
                    }
                }
            }
        }
    }
}
