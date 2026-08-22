//! BG-INV-106: representation in $\mathcal{G}$ within tau_rep (§1.1
//! invariant 6).
//!
//! Invariant 6 has two halves. This module certifies the structural half:
//! every edge's curve carrier and every face's surface carrier lies in the
//! canonical carrier set $\mathcal{G}$. Membership is NOT statically
//! decidable in general (the spec is explicit about this), so the checker
//! certifies the traversal + verdict machinery around an injected total
//! classifier — the same oracle-injection shape BG-INV-108 uses for
//! `nesting_forest(n, contains)`. The tau_rep half — geometry within tau_rep
//! of the ideal object — needs rep certificates (BG-FID-005's operator),
//! which do not exist yet, and is DEFERRED; this module does not fake or
//! stub it (see [`check`]).

#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::Shell;
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, Margin, Method, Modulus, Outcome, Prop,
    PropMap, Refusal, Truth,
};

/// Total classifier for membership in the canonical carrier set G.
/// Totality is the contract (every input gets an answer), NOT decidability;
/// the concrete impl over real carrier types lands with a later wiring
/// packet, the way BG-INV-108's contains-oracle did.
pub trait CarrierClassifier<C, S> {
    /// Classifies a curve carrier's membership in G.
    fn classify_curve(&self, c: &C) -> CarrierClass;
    /// Classifies a surface carrier's membership in G.
    fn classify_surface(&self, s: &S) -> CarrierClass;
}

/// The verdict on a carrier's membership in the canonical carrier set G.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarrierClass {
    /// A member of G: an analytic primitive, or a spline within the declared
    /// degree/span caps, or a degenerate (constant) carrier.
    InG,
    /// Outside G, or a spline beyond its caps.
    OutOfG,
}

/// BG-INV-106: representation in $\mathcal{G}$ within tau_rep (§1.1
/// invariant 6), the structural half: every edge's curve carrier and every
/// face's surface carrier is a member of the canonical carrier set
/// $\mathcal{G}$, as decided by the injected total [`CarrierClassifier`].
///
/// Membership is not statically decidable in general, so the classifier is
/// an injected oracle: totality (every carrier gets an answer) is the
/// contract, and an instance the classifier cannot decide MUST be classified
/// [`CarrierClass::OutOfG`] by the classifier's own implementor — a
/// conservative answer, never a third state here.
///
/// The tau_rep half — geometry within tau_rep of the ideal object — needs rep
/// certificates (BG-FID-005's operator), which do not exist yet, and is
/// DEFERRED. No certificate claiming the tau_rep half is emitted: the Ok
/// certificate only records `Prop::Representation` for the structural half.
///
/// Every edge is walked through [`Shell::edge_iter`] and every face through
/// [`Shell::face_iter`]; duplicates (one carrier shared by two faces) are
/// classified twice — classification is pure. The first offending entity
/// decides the verdict. Localise a violation by re-running the
/// classification: the first entity in `edge_iter`/`face_iter` order whose
/// carrier classifies `OutOfG` is the offender (degenerate edges are not
/// special-cased — whether a constant carrier is in G is the classifier's
/// decision, not this checker's).
///
/// ```
/// use truck_topology::invariants::representation::{
///     CarrierClass, CarrierClassifier, check,
/// };
/// use truck_topology::{Edge, Face, Shell, Vertex, wire};
///
/// struct AllInG;
/// impl CarrierClassifier<(), ()> for AllInG {
///     fn classify_curve(&self, _c: &()) -> CarrierClass { CarrierClass::InG }
///     fn classify_surface(&self, _s: &()) -> CarrierClass { CarrierClass::InG }
/// }
///
/// let v = Vertex::news(&[(); 3]);
/// let e0 = Edge::new(&v[0], &v[1], ());
/// let e1 = Edge::new(&v[1], &v[2], ());
/// let e2 = Edge::new(&v[2], &v[0], ());
/// let shell = Shell::from(vec![Face::new(vec![wire![&e0, &e1, &e2]], ())]);
/// assert!(check(&shell, &AllInG).is_ok());
/// ```
pub fn check<P, C, S, K>(shell: &Shell<P, C, S>, classifier: &K) -> Outcome<()>
where
    K: CarrierClassifier<C, S>,
{
    for edge in shell.edge_iter() {
        if classifier.classify_curve(edge.shared_curve()) == CarrierClass::OutOfG {
            return Err(Refusal::Contradictory(ContradictionWitness {
                prop: Prop::Representation,
                left: Truth::True,
                right: Truth::False,
            }));
        }
    }
    for face in shell.face_iter() {
        if classifier.classify_surface(face.shared_surface()) == CarrierClass::OutOfG {
            return Err(Refusal::Contradictory(ContradictionWitness {
                prop: Prop::Representation,
                left: Truth::True,
                right: Truth::False,
            }));
        }
    }
    let mut props = PropMap::new();
    props.set(Prop::Representation, Truth::True);
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
#[allow(clippy::indexing_slicing)] // H-1: test-only indexing of hand-built cube witnesses over `()`, not a kernel path
mod tests {
    #![deny(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::shell::ShellCondition;
    use crate::*;

    /// The `Closed` cube-shell witness copied verbatim from
    /// `coedge_pairing::tests::closed_cube_shell`: the 8-vertex, 12-edge,
    /// 6-wire cube construction with `shell[5].invert()`.
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

    /// Answers `InG` for every carrier.
    struct AllInG;

    impl CarrierClassifier<(), ()> for AllInG {
        fn classify_curve(&self, _c: &()) -> CarrierClass {
            CarrierClass::InG
        }
        fn classify_surface(&self, _s: &()) -> CarrierClass {
            CarrierClass::InG
        }
    }

    /// Answers `OutOfG` for exactly one surface: the `flip_at`-th surface
    /// classification (`Cell` so the classifier needs no `&mut self`).
    struct OneSurfaceOutOfG {
        flip_at: usize,
        calls: std::cell::Cell<usize>,
    }

    impl CarrierClassifier<(), ()> for OneSurfaceOutOfG {
        fn classify_curve(&self, _c: &()) -> CarrierClass {
            CarrierClass::InG
        }
        fn classify_surface(&self, _s: &()) -> CarrierClass {
            let n = self.calls.get();
            self.calls.set(n + 1);
            if n == self.flip_at {
                CarrierClass::OutOfG
            } else {
                CarrierClass::InG
            }
        }
    }

    /// Answers `OutOfG` for exactly one curve: the `flip_at`-th curve
    /// classification.
    struct OneCurveOutOfG {
        flip_at: usize,
        calls: std::cell::Cell<usize>,
    }

    impl CarrierClassifier<(), ()> for OneCurveOutOfG {
        fn classify_curve(&self, _c: &()) -> CarrierClass {
            let n = self.calls.get();
            self.calls.set(n + 1);
            if n == self.flip_at {
                CarrierClass::OutOfG
            } else {
                CarrierClass::InG
            }
        }
        fn classify_surface(&self, _s: &()) -> CarrierClass {
            CarrierClass::InG
        }
    }

    #[test]
    fn representation_canonical_shell_holds() {
        let shell = closed_cube_shell();
        let out = check(&shell, &AllInG);
        assert!(out.is_ok());
        if let Ok(certified) = out {
            assert_eq!(certified.cert.props.get(Prop::Representation), Truth::True);
        }
    }

    #[test]
    fn representation_out_of_g_face_violates() {
        let shell = closed_cube_shell();
        // Flip the class of one surface via the classifier's own state.
        let classifier = OneSurfaceOutOfG {
            flip_at: 3,
            calls: std::cell::Cell::new(0),
        };
        let witness = match check(&shell, &classifier) {
            Err(Refusal::Contradictory(witness)) => witness,
            other => unreachable!("expected Contradictory, got {other:?}"),
        };
        assert_eq!(witness.prop, Prop::Representation);
        assert_eq!(witness.left, Truth::True);
        assert_eq!(witness.right, Truth::False);
    }

    #[test]
    fn representation_out_of_g_edge_violates() {
        let shell = closed_cube_shell();
        // Flip the class of one curve via the classifier's own state.
        let classifier = OneCurveOutOfG {
            flip_at: 0,
            calls: std::cell::Cell::new(0),
        };
        let witness = match check(&shell, &classifier) {
            Err(Refusal::Contradictory(witness)) => witness,
            other => unreachable!("expected Contradictory, got {other:?}"),
        };
        assert_eq!(witness.prop, Prop::Representation);
        assert_eq!(witness.left, Truth::True);
        assert_eq!(witness.right, Truth::False);
    }

    #[test]
    fn representation_empty_shell_holds_vacuously() {
        let shell = Shell::<(), (), ()>::new();
        let out = check(&shell, &AllInG);
        assert!(out.is_ok());
        if let Ok(certified) = out {
            assert_eq!(certified.cert.props.get(Prop::Representation), Truth::True);
        }
    }

    #[test]
    fn representation_certificate_names_the_invariant() {
        let shell = closed_cube_shell();
        let out = check(&shell, &AllInG);
        assert!(out.is_ok());
        if let Ok(certified) = out {
            let cert = &certified.cert;
            assert_eq!(cert.method, Method::None);
            assert_eq!(cert.budget_left, Budget::new(0, 0, 0));
            assert_eq!(cert.props.get(Prop::Representation), Truth::True);
        }

        let classifier = OneSurfaceOutOfG {
            flip_at: 0,
            calls: std::cell::Cell::new(0),
        };
        let out = check(&shell, &classifier);
        let witness = match out {
            Err(Refusal::Contradictory(witness)) => witness,
            other => unreachable!("expected Contradictory, got {other:?}"),
        };
        assert_eq!(witness.prop, Prop::Representation);
    }
}
