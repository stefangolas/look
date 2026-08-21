//! BG-INV-105: domain–boundary correspondence (§1.1 invariant 5).
//!
//! The boundary wires of a face correspond, edge use by edge use, to curves
//! on the face's surface domain. This module certifies the topological half
//! of that correspondence; the pcurve-carrying half waits on pcurve wiring
//! (see [`check`]).

use crate::Face;
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, Margin, Method, Modulus, Outcome, Prop,
    PropMap, Refusal, Truth,
};

/// BG-INV-105: domain–boundary correspondence (§1.1 invariant 5), the
/// topological core: every boundary wire of `face` is a closed loop and
/// the face has at least one.
///
/// The FULL invariant — the wires tracing the parameter domain's
/// boundary, edge use by edge use — needs pcurves attached to the edge
/// uses (BG-CE-001's `PC` payload, still unwired in this tree) and is
/// NOT checked here; this checker certifies the topological half only.
/// Localise a violation with the wire index: the refusal's `prop` names
/// the invariant; the offending wire is the first index for which
/// `is_closed()` is false (or index 0 when there are no wires at all).
///
/// ```
/// use truck_topology::*;
/// use truck_topology::invariants::domain_boundary::check;
///
/// let v = Vertex::news(&[(); 3]);
/// let e0 = Edge::new(&v[0], &v[1], ());
/// let e1 = Edge::new(&v[1], &v[2], ());
/// let e2 = Edge::new(&v[2], &v[0], ());
/// let face = Face::new(vec![wire![&e0, &e1, &e2]], ());
/// assert!(check(&face).is_ok());
/// ```
pub fn check<P, C, S>(face: &Face<P, C, S>) -> Outcome<()> {
    let boundaries = face.boundaries();
    if boundaries.is_empty() {
        return Err(Refusal::Contradictory(ContradictionWitness {
            prop: Prop::DomainBoundary,
            left: Truth::True,
            right: Truth::False,
        }));
    }
    if boundaries.iter().any(|wire| !wire.is_closed()) {
        return Err(Refusal::Contradictory(ContradictionWitness {
            prop: Prop::DomainBoundary,
            left: Truth::True,
            right: Truth::False,
        }));
    }
    let mut props = PropMap::new();
    props.set(Prop::DomainBoundary, Truth::True);
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

#[cfg(test)]
#[allow(clippy::indexing_slicing)] // H-1: test-only indexing of hand-built face witnesses over `()`, not a kernel path
mod tests {
    #![deny(clippy::unwrap_used)]
    use super::*;
    use crate::*;

    #[test]
    fn domain_boundary_closed_wires_hold() {
        let v = Vertex::news([(); 3]);
        let wire = wire![
            Edge::new(&v[0], &v[1], ()),
            Edge::new(&v[1], &v[2], ()),
            Edge::new(&v[2], &v[0], ()),
        ];
        let face = Face::new(vec![wire], ());
        let out = check(&face);
        assert!(out.is_ok());
        if let Ok(certified) = out {
            assert_eq!(certified.cert.props.get(Prop::DomainBoundary), Truth::True);
        }
    }

    #[test]
    fn domain_boundary_open_wire_violates() {
        let v = Vertex::news([(); 3]);
        let wire = wire![Edge::new(&v[0], &v[1], ()), Edge::new(&v[1], &v[2], ()),];
        assert!(!wire.is_closed());
        // `Face::new` would refuse the non-closed wire (remove_try panics), so
        // the violating witness is built unchecked.
        let face = Face::new_unchecked(vec![wire], ());
        let w = match check(&face) {
            Err(Refusal::Contradictory(witness)) => witness,
            other => unreachable!("expected Contradictory, got {other:?}"),
        };
        assert_eq!(w.prop, Prop::DomainBoundary);
        assert_eq!(w.left, Truth::True);
        assert_eq!(w.right, Truth::False);
    }

    #[test]
    fn domain_boundary_no_boundaries_violates() {
        let face = Face::<(), (), ()>::new(vec![], ());
        let w = match check(&face) {
            Err(Refusal::Contradictory(witness)) => witness,
            other => unreachable!("expected Contradictory, got {other:?}"),
        };
        assert_eq!(w.prop, Prop::DomainBoundary);
        assert_eq!(w.left, Truth::True);
        assert_eq!(w.right, Truth::False);
    }

    #[test]
    fn domain_boundary_certificate_names_the_invariant() {
        let v = Vertex::news([(); 3]);
        let wire = wire![
            Edge::new(&v[0], &v[1], ()),
            Edge::new(&v[1], &v[2], ()),
            Edge::new(&v[2], &v[0], ()),
        ];
        let face = Face::new(vec![wire], ());
        let out = check(&face);
        assert!(out.is_ok());
        if let Ok(certified) = out {
            let cert = &certified.cert;
            assert_eq!(cert.method, Method::None);
            assert_eq!(cert.budget_left, Budget::new(0, 0, 0));
            assert_eq!(cert.props.get(Prop::CoedgePairing), Truth::Unknown);
        }
    }
}
