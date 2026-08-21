//! BG-INV-101: coedge pairing (§1.1 invariant 1).
//!
//! Every non-degenerate edge of a solid boundary is shared by exactly two
//! faces of opposite sense. Wraps the [`ShellCondition`] machinery in the
//! evidence algebra: [`Shell::shell_condition`] returning `Closed` certifies
//! the pairing; `Irregular` (some edge in more than two faces), `Regular`
//! (some pair same-sense) and `Oriented` (some edge in fewer than two faces —
//! an open boundary) all violate it. The "declared even number" and "declared
//! 1" clauses are the caller's assertion about open boundaries; they cannot
//! be checked from the shell alone and are out of scope until a topology
//! carries the declaration.

use crate::shell::ShellCondition;
use crate::Shell;
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, Margin, Method, Modulus, Outcome, Prop,
    PropMap, Refusal, Truth,
};

/// BG-INV-101: coedge pairing (§1.1 invariant 1) — every edge of a solid
/// boundary is shared by exactly two faces of opposite sense.
///
/// Wraps `Shell::shell_condition()`: `Closed` holds; `Irregular` (some
/// edge in more than two faces), `Regular` (some pair same-sense) and
/// `Oriented` (some edge in fewer than two faces — an open boundary)
/// all violate the pairing. The "declared even number" and "declared 1"
/// clauses of the invariant are the CALLER's assertion about open
/// boundaries; they cannot be checked from the shell alone and are out
/// of scope until a topology carries the declaration. Localise a
/// violation with the shell's own `edge_iter` — the Boundaries pass in
/// `shell.rs` is the reference grouping.
///
/// ```
/// use truck_topology::*;
/// use truck_topology::invariants::coedge_pairing::check;
///
/// // A two-triangle "pillow" over the same three vertices: the second face
/// // traces the boundary edge-wise inverted, so every edge is used exactly
/// // twice with opposite sense.
/// let v = Vertex::news(&[(); 3]);
/// let e0 = Edge::new(&v[0], &v[1], ());
/// let e1 = Edge::new(&v[1], &v[2], ());
/// let e2 = Edge::new(&v[2], &v[0], ());
/// let shell = Shell::from(vec![
///     Face::new(vec![wire![&e0, &e1, &e2]], ()),
///     Face::new(vec![wire![&e0.inverse(), &e2.inverse(), &e1.inverse()]], ()),
/// ]);
/// assert!(check(&shell).is_ok());
/// ```
pub fn check<P, C, S>(shell: &Shell<P, C, S>) -> Outcome<()> {
    match shell.shell_condition() {
        ShellCondition::Closed => {
            let mut props = PropMap::new();
            props.set(Prop::CoedgePairing, Truth::True);
            Ok(Certified::new(
                (),
                Certificate {
                    props,
                    method: Method::None,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ))
        }
        _ => Err(Refusal::Contradictory(ContradictionWitness {
            prop: Prop::CoedgePairing,
            left: Truth::True,
            right: Truth::False,
        })),
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // H-1: test-only indexing of hand-built shell witnesses over `()`, not a kernel path
mod tests {
    #![deny(clippy::unwrap_used)]
    use super::*;
    use crate::*;

    /// The `Closed` doctest witness of `ShellCondition::Closed` in
    /// `shell.rs`: the 8-vertex, 12-edge, 6-wire cube construction with
    /// `shell[5].invert()`.
    fn closed_cube_shell() -> Shell<(), (), ()> {
        let v = Vertex::news([(); 8]);
        let edge = [
            Edge::new(&v[0], &v[1], ()),
            Edge::new(&v[1], &v[2], ()),
            Edge::new(&v[2], &v[3], ()),
            Edge::new(&v[3], &v[0], ()),
            Edge::new(&v[0], &v[4], ()),
            Edge::new(&v[1], &v[5], ()),
            Edge::new(&v[2], &v[6], ()),
            Edge::new(&v[3], &v[7], ()),
            Edge::new(&v[4], &v[5], ()),
            Edge::new(&v[5], &v[6], ()),
            Edge::new(&v[6], &v[7], ()),
            Edge::new(&v[7], &v[4], ()),
        ];
        let wire = vec![
            wire![&edge[0], &edge[1], &edge[2], &edge[3]],
            wire![&edge[0].inverse(), &edge[4], &edge[8], &edge[5].inverse()],
            wire![&edge[1].inverse(), &edge[5], &edge[9], &edge[6].inverse()],
            wire![&edge[2].inverse(), &edge[6], &edge[10], &edge[7].inverse()],
            wire![&edge[3].inverse(), &edge[7], &edge[11], &edge[4].inverse()],
            wire![&edge[8], &edge[9], &edge[10], &edge[11]],
        ];
        let mut shell: Shell<_, _, _> = wire.into_iter().map(|w| Face::new(vec![w], ())).collect();
        shell[5].invert();
        assert_eq!(shell.shell_condition(), ShellCondition::Closed);
        shell
    }

    #[test]
    fn coedge_pairing_closed_shell_holds() {
        let shell = closed_cube_shell();
        let out = check(&shell);
        assert!(out.is_ok());
        if let Ok(certified) = out {
            assert_eq!(certified.cert.props.get(Prop::CoedgePairing), Truth::True);
        }
    }

    #[test]
    fn coedge_pairing_three_faces_one_edge_violates() {
        let v = Vertex::news([(); 5]);
        let edge = [
            Edge::new(&v[0], &v[1], ()),
            Edge::new(&v[0], &v[2], ()),
            Edge::new(&v[0], &v[3], ()),
            Edge::new(&v[0], &v[4], ()),
            Edge::new(&v[1], &v[2], ()),
            Edge::new(&v[1], &v[3], ()),
            Edge::new(&v[1], &v[4], ()),
        ];
        let wire = vec![
            wire![&edge[0], &edge[4], &edge[1].inverse()],
            wire![&edge[0], &edge[5], &edge[2].inverse()],
            wire![&edge[0], &edge[6], &edge[3].inverse()],
        ];
        let shell: Shell<_, _, _> = wire.into_iter().map(|w| Face::new(vec![w], ())).collect();
        assert_eq!(shell.shell_condition(), ShellCondition::Irregular);
        let witness = match check(&shell) {
            Err(Refusal::Contradictory(witness)) => witness,
            other => unreachable!("expected Contradictory, got {other:?}"),
        };
        assert_eq!(witness.prop, Prop::CoedgePairing);
        assert_eq!(witness.left, Truth::True);
        assert_eq!(witness.right, Truth::False);
    }

    #[test]
    fn coedge_pairing_open_boundary_violates() {
        let v = Vertex::news([(); 6]);
        let edge = [
            Edge::new(&v[0], &v[1], ()),
            Edge::new(&v[0], &v[2], ()),
            Edge::new(&v[1], &v[2], ()),
            Edge::new(&v[1], &v[3], ()),
            Edge::new(&v[1], &v[4], ()),
            Edge::new(&v[2], &v[4], ()),
            Edge::new(&v[2], &v[5], ()),
            Edge::new(&v[3], &v[4], ()),
            Edge::new(&v[4], &v[5], ()),
        ];
        let wire = vec![
            wire![&edge[0], &edge[2], &edge[1].inverse()],
            wire![&edge[3], &edge[7], &edge[4].inverse()],
            wire![&edge[5], &edge[8], &edge[6].inverse()],
            wire![&edge[2].inverse(), &edge[4], &edge[5].inverse()],
        ];
        let shell: Shell<_, _, _> = wire.into_iter().map(|w| Face::new(vec![w], ())).collect();
        assert_eq!(shell.shell_condition(), ShellCondition::Oriented);
        let witness = match check(&shell) {
            Err(Refusal::Contradictory(witness)) => witness,
            other => unreachable!("expected Contradictory, got {other:?}"),
        };
        assert_eq!(witness.prop, Prop::CoedgePairing);
        assert_eq!(witness.left, Truth::True);
        assert_eq!(witness.right, Truth::False);
    }

    #[test]
    fn coedge_pairing_certificate_names_the_invariant() {
        let shell = closed_cube_shell();
        let out = check(&shell);
        assert!(out.is_ok());
        if let Ok(certified) = out {
            let cert = &certified.cert;
            assert_eq!(cert.method, Method::None);
            assert_eq!(cert.budget_left, Budget::new(0, 0, 0));
            assert_eq!(cert.props.get(Prop::SoundEnclosure), Truth::Unknown);
        }
    }
}
