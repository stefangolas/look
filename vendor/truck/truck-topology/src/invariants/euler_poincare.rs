//! BG-INV-103: the Euler–Poincaré invariant checker (§1.1 invariant 3).
//!
//! For every face-connected component of a shell, counts DISTINCT vertices
//! and edges by id (the shell's own `edge_iter`/`vertex_iter` yield
//! duplicates) and faces directly, and requires χ = v − e + f to be even —
//! the Euler–Poincaré characteristic of any closed orientable component is
//! 2s − 2g. The relation is per connected component, not per shell. **Never
//! a substitute for the vertex-link invariant (BG-INV-102): a pinch point
//! satisfies Euler–Poincaré while its vertex link is not a single cycle.**

use crate::Shell;
use std::collections::HashSet;
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, Margin, Method, Modulus, Outcome, Prop,
    PropMap, Refusal, Truth,
};

/// BG-INV-103's counting core, pure: χ = v − e + f must be even (the
/// Euler–Poincaré characteristic of any closed orientable component is
/// 2s − 2g). Exposed so the parity logic is testable against synthetic
/// violator counts — on consistently built shells the parity is a
/// theorem, and this checker's job is to catch counting-machinery
/// regressions, not to out-think topology.
pub fn check_counts(vertices: usize, edges: usize, faces: usize) -> Outcome<()> {
    let chi = vertices as i64 - edges as i64 + faces as i64;
    if chi % 2 == 0 {
        let mut props = PropMap::new();
        props.set(Prop::EulerPoincare, Truth::True);
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
            prop: Prop::EulerPoincare,
            left: Truth::True,
            right: Truth::False,
        }))
    }
}

/// BG-INV-103: Euler–Poincaré (§1.1 invariant 3) over a shell's
/// connected components.
///
/// Counts DISTINCT vertices and edges by id per component (the shell's
/// `edge_iter`/`vertex_iter` yield duplicates) and faces directly.
/// **Never a substitute for the vertex-link invariant (BG-INV-102): a
/// pinch point satisfies Euler–Poincaré while its vertex link is not a
/// single cycle.** On consistently built shells the parity is a theorem;
/// this checker is the regression net for the counting machinery.
///
/// # Examples
/// ```
/// use truck_topology::*;
/// use truck_topology::invariants::euler_poincare::check;
/// let [v0, v1, v2, v3, v4, v5, v6, v7] = [
///     Vertex::new(()), Vertex::new(()), Vertex::new(()), Vertex::new(()),
///     Vertex::new(()), Vertex::new(()), Vertex::new(()), Vertex::new(()),
/// ];
/// let e0 = Edge::new(&v0, &v1, ());
/// let e1 = Edge::new(&v1, &v2, ());
/// let e2 = Edge::new(&v2, &v3, ());
/// let e3 = Edge::new(&v3, &v0, ());
/// let e4 = Edge::new(&v0, &v4, ());
/// let e5 = Edge::new(&v1, &v5, ());
/// let e6 = Edge::new(&v2, &v6, ());
/// let e7 = Edge::new(&v3, &v7, ());
/// let e8 = Edge::new(&v4, &v5, ());
/// let e9 = Edge::new(&v5, &v6, ());
/// let e10 = Edge::new(&v6, &v7, ());
/// let e11 = Edge::new(&v7, &v4, ());
/// let wire = vec![
///     wire![&e0, &e1, &e2, &e3],
///     wire![&e0.inverse(), &e4, &e8, &e5.inverse()],
///     wire![&e1.inverse(), &e5, &e9, &e6.inverse()],
///     wire![&e2.inverse(), &e6, &e10, &e7.inverse()],
///     wire![&e3.inverse(), &e7, &e11, &e4.inverse()],
///     wire![&e8, &e9, &e10, &e11],
/// ];
/// let mut faces: Vec<Face<(), (), ()>> =
///     wire.into_iter().map(|w| Face::new(vec![w], ())).collect();
/// if let Some(face) = faces.get_mut(5) {
///     face.invert();
/// }
/// let shell: Shell<(), (), ()> = Shell::from(faces);
/// assert!(check(&shell).is_ok());
/// ```
pub fn check<P, C, S>(shell: &Shell<P, C, S>) -> Outcome<()> {
    for component in shell.connected_components() {
        let v = component
            .vertex_iter()
            .map(|x| x.id())
            .collect::<HashSet<_>>()
            .len();
        let e = component
            .edge_iter()
            .map(|x| x.id())
            .collect::<HashSet<_>>()
            .len();
        let f = component.face_iter().count();
        check_counts(v, e, f)?;
    }
    let mut props = PropMap::new();
    props.set(Prop::EulerPoincare, Truth::True);
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
mod tests {
    #![deny(clippy::unwrap_used)]
    use super::*;
    use crate::*;

    /// The `Closed` doctest witness from `shell.rs`: V=8, E=12, F=6, χ=2.
    fn cube() -> Shell<(), (), ()> {
        let [v0, v1, v2, v3, v4, v5, v6, v7] = [
            Vertex::new(()),
            Vertex::new(()),
            Vertex::new(()),
            Vertex::new(()),
            Vertex::new(()),
            Vertex::new(()),
            Vertex::new(()),
            Vertex::new(()),
        ];
        let e0 = Edge::new(&v0, &v1, ());
        let e1 = Edge::new(&v1, &v2, ());
        let e2 = Edge::new(&v2, &v3, ());
        let e3 = Edge::new(&v3, &v0, ());
        let e4 = Edge::new(&v0, &v4, ());
        let e5 = Edge::new(&v1, &v5, ());
        let e6 = Edge::new(&v2, &v6, ());
        let e7 = Edge::new(&v3, &v7, ());
        let e8 = Edge::new(&v4, &v5, ());
        let e9 = Edge::new(&v5, &v6, ());
        let e10 = Edge::new(&v6, &v7, ());
        let e11 = Edge::new(&v7, &v4, ());
        let wire = vec![
            wire![&e0, &e1, &e2, &e3],
            wire![&e0.inverse(), &e4, &e8, &e5.inverse()],
            wire![&e1.inverse(), &e5, &e9, &e6.inverse()],
            wire![&e2.inverse(), &e6, &e10, &e7.inverse()],
            wire![&e3.inverse(), &e7, &e11, &e4.inverse()],
            wire![&e8, &e9, &e10, &e11],
        ];
        let mut faces: Vec<Face<(), (), ()>> =
            wire.into_iter().map(|w| Face::new(vec![w], ())).collect();
        if let Some(face) = faces.get_mut(5) {
            face.invert();
        }
        Shell::from(faces)
    }

    /// A closed tetrahedron: V=4, E=6, F=4, χ=2.
    fn tetrahedron() -> Shell<(), (), ()> {
        let [v0, v1, v2, v3] = [
            Vertex::new(()),
            Vertex::new(()),
            Vertex::new(()),
            Vertex::new(()),
        ];
        let e0 = Edge::new(&v0, &v1, ());
        let e1 = Edge::new(&v0, &v2, ());
        let e2 = Edge::new(&v0, &v3, ());
        let e3 = Edge::new(&v1, &v2, ());
        let e4 = Edge::new(&v1, &v3, ());
        let e5 = Edge::new(&v2, &v3, ());
        let wire = vec![
            wire![&e0, &e3, &e1.inverse()],
            wire![&e1, &e5, &e2.inverse()],
            wire![&e2, &e4.inverse(), &e0.inverse()],
            wire![&e3, &e5, &e4.inverse()],
        ];
        let mut faces: Vec<Face<(), (), ()>> =
            wire.into_iter().map(|w| Face::new(vec![w], ())).collect();
        if let Some(face) = faces.get_mut(3) {
            face.invert();
        }
        Shell::from(faces)
    }

    #[test]
    fn euler_poincare_closed_cube_holds() {
        let shell = cube();
        assert!(matches!(
            check(&shell),
            Ok(certified)
                if certified.cert.props.get(Prop::EulerPoincare) == Truth::True
        ));
    }

    #[test]
    fn euler_poincare_two_components_each_hold() {
        let mut shell = tetrahedron();
        let mut other = tetrahedron();
        shell.append(&mut other);
        assert_eq!(shell.connected_components().len(), 2);
        assert!(check(&shell).is_ok());
    }

    #[test]
    fn euler_poincare_odd_counts_violate() {
        assert!(matches!(
            check_counts(3, 3, 1),
            Err(Refusal::Contradictory(witness))
                if witness.prop == Prop::EulerPoincare
        ));
        assert!(matches!(
            check_counts(5, 4, 2),
            Err(Refusal::Contradictory(witness))
                if witness.prop == Prop::EulerPoincare
        ));
        assert!(check_counts(8, 12, 6).is_ok());
        assert!(check_counts(2, 3, 3).is_ok());
    }

    #[test]
    fn euler_poincare_never_substitutes_for_vertex_link() {
        // The `singular_vertices` doctest witness: two tetrahedra pinched at
        // v0, so v0's link is not a single cycle.
        let [v0, v1, v2, v3, v4, v5, v6] = [
            Vertex::new(()),
            Vertex::new(()),
            Vertex::new(()),
            Vertex::new(()),
            Vertex::new(()),
            Vertex::new(()),
            Vertex::new(()),
        ];
        let e0 = Edge::new(&v0, &v1, ());
        let e1 = Edge::new(&v0, &v2, ());
        let e2 = Edge::new(&v0, &v3, ());
        let e3 = Edge::new(&v1, &v2, ());
        let e4 = Edge::new(&v2, &v3, ());
        let e5 = Edge::new(&v3, &v1, ());
        let e6 = Edge::new(&v0, &v4, ());
        let e7 = Edge::new(&v0, &v5, ());
        let e8 = Edge::new(&v0, &v6, ());
        let e9 = Edge::new(&v4, &v5, ());
        let e10 = Edge::new(&v5, &v6, ());
        let e11 = Edge::new(&v6, &v4, ());
        let wire = vec![
            wire![&e0.inverse(), &e1, &e3.inverse()],
            wire![&e1.inverse(), &e2, &e4.inverse()],
            wire![&e2.inverse(), &e0, &e5.inverse()],
            wire![&e3, &e4, &e5],
            wire![&e6.inverse(), &e7, &e9.inverse()],
            wire![&e7.inverse(), &e8, &e10.inverse()],
            wire![&e8.inverse(), &e6, &e11.inverse()],
            wire![&e9, &e10, &e11],
        ];
        let shell: Shell<(), (), ()> = wire.into_iter().map(|w| Face::new(vec![w], ())).collect();
        assert_eq!(shell.singular_vertices(), vec![v0.clone()]);
        // Count first: the witness's face-connected components are the two
        // tetrahedral halves, each with V=4, E=6, F=4 (χ=2, even). The
        // whole-shell count V=7, E=12, F=8 would give χ=3 — odd only because
        // the shared pinch vertex is double counted, which is exactly why the
        // relation is checked per connected component.
        for component in shell.connected_components() {
            let v = component
                .vertex_iter()
                .map(|x| x.id())
                .collect::<HashSet<_>>()
                .len();
            let e = component
                .edge_iter()
                .map(|x| x.id())
                .collect::<HashSet<_>>()
                .len();
            let f = component.face_iter().count();
            assert_eq!(v, 4);
            assert_eq!(e, 6);
            assert_eq!(f, 4);
        }
        // Euler–Poincaré passing does NOT clear the vertex link: `check` is
        // Ok here while v0's link is not a single cycle.
        assert!(check(&shell).is_ok());
    }
}
