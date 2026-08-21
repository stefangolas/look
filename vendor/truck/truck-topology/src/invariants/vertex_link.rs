//! BG-INV-102: vertex link is a single cycle (§1.1 invariant 2).
//!
//! Wraps `Shell::singular_vertices()` in the evidence algebra. The single-cycle
//! conclusion is valid only on a closed shell; that dependency is documented on
//! [`check`], not enforced here (the closure pre-check belongs to BG-INV-101's
//! checker).

use crate::Shell;
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, Margin, Method, Modulus, Outcome, Prop,
    PropMap, Refusal, Truth,
};

/// BG-INV-102: vertex link is a single cycle (§1.1 invariant 2).
///
/// Wraps `Shell::singular_vertices()`. **The single-cycle conclusion is
/// valid only on a closed shell**: `singular_vertices` tests link
/// CONNECTIVITY, and the "every link node has degree 2" step comes from
/// coedge pairing (BG-INV-101). On an open shell a connected link may be
/// a path rather than a cycle. Callers should run BG-INV-101 first, or
/// accept that this checker certifies link connectivity only.
/// Localisation: the violating vertices are `singular_vertices()`'s own
/// return value.
///
/// # Examples
/// ```
/// use truck_topology::*;
///
/// let v = Vertex::news(&[(), (), (), ()]);
/// let edge = [
///     Edge::new(&v[0], &v[1], ()),
///     Edge::new(&v[1], &v[2], ()),
///     Edge::new(&v[2], &v[0], ()),
///     Edge::new(&v[1], &v[3], ()),
///     Edge::new(&v[3], &v[2], ()),
///     Edge::new(&v[0], &v[3], ()),
/// ];
/// let wire = vec![
///     wire![&edge[0], &edge[3], &edge[4], &edge[2]],
///     wire![&edge[1], &edge[2], &edge[5], &edge[3].inverse()],
/// ];
/// let shell: Shell<_, _, _> = wire
///     .into_iter()
///     .map(|w| Face::new(vec![w], ()))
///     .collect();
/// assert!(truck_topology::invariants::vertex_link::check(&shell).is_ok());
/// ```
pub fn check<P, C, S>(shell: &Shell<P, C, S>) -> Outcome<()> {
    if shell.singular_vertices().is_empty() {
        let mut props = PropMap::new();
        props.set(Prop::VertexLink, Truth::True);
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
    } else {
        Err(Refusal::Contradictory(ContradictionWitness {
            prop: Prop::VertexLink,
            left: Truth::True,
            right: Truth::False,
        }))
    }
}

#[cfg(test)]
mod tests {
    #![deny(clippy::unwrap_used)]
    // The witnesses below are copied verbatim from `singular_vertices`' own
    // doctests in shell.rs; array/Vec indexing there is test scaffolding, not
    // a kernel data path (the invariants module otherwise denies indexing).
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::*;

    #[test]
    fn vertex_link_regular_shell_holds() {
        // A regular manifold (Möbius bundle) from singular_vertices' doctest.
        let v = Vertex::news([(), (), (), ()]);
        let edge = [
            Edge::new(&v[0], &v[1], ()),
            Edge::new(&v[1], &v[2], ()),
            Edge::new(&v[2], &v[0], ()),
            Edge::new(&v[1], &v[3], ()),
            Edge::new(&v[3], &v[2], ()),
            Edge::new(&v[0], &v[3], ()),
        ];
        let wire = vec![
            wire![&edge[0], &edge[3], &edge[4], &edge[2]],
            wire![&edge[1], &edge[2], &edge[5], &edge[3].inverse()],
        ];
        let shell: Shell<_, _, _> = wire.into_iter().map(|w| Face::new(vec![w], ())).collect();
        assert!(shell.singular_vertices().is_empty());
        let outcome = check(&shell);
        assert!(
            matches!(&outcome, Ok(Certified { value: (), .. })),
            "regular shell must certify a hold, got {outcome:?}"
        );
        if let Ok(Certified { cert, .. }) = &outcome {
            assert_eq!(cert.props.get(Prop::VertexLink), Truth::True);
        }
    }

    #[test]
    fn vertex_link_singular_vertex_violates() {
        // The closed-and-connected singular witness from singular_vertices'
        // doctest: v[0] is the vertex whose link is not a single cycle.
        let v = Vertex::news([(); 7]);
        let edge = [
            Edge::new(&v[0], &v[1], ()), // 0
            Edge::new(&v[0], &v[2], ()), // 1
            Edge::new(&v[0], &v[3], ()), // 2
            Edge::new(&v[1], &v[2], ()), // 3
            Edge::new(&v[2], &v[3], ()), // 4
            Edge::new(&v[3], &v[1], ()), // 5
            Edge::new(&v[0], &v[4], ()), // 6
            Edge::new(&v[0], &v[5], ()), // 7
            Edge::new(&v[0], &v[6], ()), // 8
            Edge::new(&v[4], &v[5], ()), // 9
            Edge::new(&v[5], &v[6], ()), // 10
            Edge::new(&v[6], &v[4], ()), // 11
        ];
        let wire = vec![
            wire![&edge[0].inverse(), &edge[1], &edge[3].inverse()],
            wire![&edge[1].inverse(), &edge[2], &edge[4].inverse()],
            wire![&edge[2].inverse(), &edge[0], &edge[5].inverse()],
            wire![&edge[3], &edge[4], &edge[5]],
            wire![&edge[6].inverse(), &edge[7], &edge[9].inverse()],
            wire![&edge[7].inverse(), &edge[8], &edge[10].inverse()],
            wire![&edge[8].inverse(), &edge[6], &edge[11].inverse()],
            wire![&edge[9], &edge[10], &edge[11]],
        ];
        let shell: Shell<_, _, _> = wire.into_iter().map(|w| Face::new(vec![w], ())).collect();
        assert_eq!(shell.singular_vertices(), vec![v[0].clone()]);
        let outcome = check(&shell);
        assert!(
            matches!(
                &outcome,
                Err(Refusal::Contradictory(w))
                    if w.prop == Prop::VertexLink
                        && w.left == Truth::True
                        && w.right == Truth::False
            ),
            "singular shell must refuse the vertex-link claim, got {outcome:?}"
        );
    }

    #[test]
    fn vertex_link_documented_dependency_on_closed() {
        // This is the documented open-shell limitation: a single closed
        // triangular wire as one face is OPEN (every edge used once, no
        // opposite-sense partner), every vertex link is a connected path of
        // two edges rather than a cycle, and yet `singular_vertices` is empty
        // — it tests link connectivity, and connectivity does not imply the
        // single cycle off a closed shell. The checker must not refuse what
        // it cannot conclude: it certifies a hold.
        let v = Vertex::news([(), (), ()]);
        let edge = [
            Edge::new(&v[0], &v[1], ()),
            Edge::new(&v[1], &v[2], ()),
            Edge::new(&v[2], &v[0], ()),
        ];
        let shell: Shell<_, _, _> = vec![Face::new(vec![wire![&edge[0], &edge[1], &edge[2]]], ())]
            .into_iter()
            .collect();
        assert!(shell.singular_vertices().is_empty());
        assert!(check(&shell).is_ok());
    }

    #[test]
    fn vertex_link_certificate_names_the_invariant() {
        // The holds certificate claims ONLY its own property: the vertex-link
        // claim is True, coedge pairing (BG-INV-101's invariant) stays
        // Unknown, and the certificate is the structural empty shape.
        let v = Vertex::news([(), (), (), ()]);
        let edge = [
            Edge::new(&v[0], &v[1], ()),
            Edge::new(&v[1], &v[2], ()),
            Edge::new(&v[2], &v[0], ()),
            Edge::new(&v[1], &v[3], ()),
            Edge::new(&v[3], &v[2], ()),
            Edge::new(&v[0], &v[3], ()),
        ];
        let wire = vec![
            wire![&edge[0], &edge[3], &edge[4], &edge[2]],
            wire![&edge[1], &edge[2], &edge[5], &edge[3].inverse()],
        ];
        let shell: Shell<_, _, _> = wire.into_iter().map(|w| Face::new(vec![w], ())).collect();
        let outcome = check(&shell);
        assert!(
            matches!(&outcome, Ok(Certified { value: (), .. })),
            "regular shell must certify a hold, got {outcome:?}"
        );
        if let Ok(Certified { cert, .. }) = &outcome {
            assert_eq!(cert.method, Method::None);
            assert_eq!(cert.budget_left, Budget::new(0, 0, 0));
            assert_eq!(cert.props.get(Prop::VertexLink), Truth::True);
            assert_eq!(cert.props.get(Prop::CoedgePairing), Truth::Unknown);
        }
    }
}
