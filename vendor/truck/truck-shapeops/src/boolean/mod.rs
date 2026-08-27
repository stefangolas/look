//! BG-SOL-RW1-MATERIAL: the §13.1 material-state fragment-selection
//! primitive.
//!
//! The Boundary Rewrite decides one boundary fragment from the four material
//! witnesses around it, not from an orientation table: it evaluates the
//! Boolean truth function on each side, keeps the fragment iff the two sides
//! differ (the fragment is on the result's boundary), and orients it toward
//! the empty (`m_R = 0`) side. No case enumeration. Pure logic: this module
//! decides, it does not touch shapes.

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

/// What the Boundary Rewrite does with one boundary fragment.
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

/// The §13.1 primitive: keep iff the sides differ; orient toward the
/// empty side.
pub fn fragment_decision(op: BoolOp, m: MaterialState4) -> FragmentDecision {
    let m_r_minus = op.eval(State {
        in_a: m.a_minus,
        in_b: m.b_minus,
    });
    let m_r_plus = op.eval(State {
        in_a: m.a_plus,
        in_b: m.b_plus,
    });
    if m_r_minus == m_r_plus {
        FragmentDecision::Discard
    } else {
        FragmentDecision::Keep { flip: !m_r_minus }
    }
}

#[cfg(test)]
mod tests {
    use super::{fragment_decision, BoolOp, FragmentDecision, MaterialState4, State};

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
}
