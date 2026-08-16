//! Wires that are known to close *topologically*.
//!
//! A face is trimmed by its boundary, and a boundary is only a boundary if it
//! closes: walking the edges in order, each one must end where the next begins,
//! and the last must end where the first began. An open chain does not separate
//! inside from outside, so trimming with one is meaningless — but it is not
//! *inert*. The tessellator joins the loose ends with whatever is nearest and
//! produces a smooth, plausible, wrong region.
//!
//! Fail-whole-bound conversion made every edge in a wire resolve or none, which
//! rules out the wire with a hole punched in the middle. It cannot rule out a
//! wire whose edges all resolved and simply do not join up, and it held only
//! because one function happened to be written correctly.
//! [`TopologicallyClosedWire`] is the type that makes it hold everywhere: the
//! only way to obtain one is [`TopologicallyClosedWire::try_new`], which walks
//! the chain.
//!
//! Closure is checked on vertex identity, not position. Two `VERTEX_POINT`
//! entities at the same coordinates are two vertices, and a wire that relies on
//! them coinciding is relying on the exporter, not on the topology.
//!
//! **The qualifier in the name is load-bearing** (`MATHEMATICAL_FOUNDATION.md`
//! §24). Three different closure propositions exist and no one of them implies
//! another: closure by vertex identity (`TOP-004`, this type), metric endpoint
//! agreement (`ARR-001`), and closure modulo the period lattice (`QUO-002`).
//! This type establishes the first only. An unqualified `ClosedWire` — which is
//! what this was called until §33a item 1 — asserts all three to every reader,
//! and inviting that inference is exactly the failure the architecture exists
//! to prevent.

use truck_topology::compress::CompressedEdgeIndex;

/// `TOP-004`. A wire whose edges join end to end and return to their start, by
/// vertex identity.
///
/// It proves `end(e_i) == start(e_{i+1})` cyclically and nothing else. In
/// particular it does **not** prove that the endpoints meet within tolerance in
/// space (`ARR-001`), nor that the lifted loop closes modulo the period lattice
/// (`QUO-002`).
#[derive(Debug, Clone)]
pub struct TopologicallyClosedWire(Vec<CompressedEdgeIndex>);

impl TopologicallyClosedWire {
    /// Walk the chain, or refuse it.
    ///
    /// `endpoints` gives the `(front, back)` vertices of an edge by position,
    /// in the edge's own direction; the wire's own orientation flag decides
    /// which end is entered first. `None` from it means an edge the wire names
    /// is not in the arena, which is already a broken wire.
    pub fn try_new(
        edges: Vec<CompressedEdgeIndex>,
        endpoints: impl Fn(usize) -> Option<(usize, usize)>,
    ) -> Option<Self> {
        // An empty wire bounds nothing, and would vacuously satisfy every check
        // below. Refuse it explicitly rather than by accident.
        if edges.is_empty() {
            return None;
        }
        let mut walked = Vec::with_capacity(edges.len());
        for edge in &edges {
            let (front, back) = endpoints(edge.index)?;
            walked.push(match edge.orientation {
                true => (front, back),
                false => (back, front),
            });
        }
        // Pairing the walk with itself rotated by one asks "does each edge end
        // where the next begins" and "does the last close back to the first" as
        // the same question. A single edge is checked against itself, which is
        // right: a lone edge bounds a face only if it is a full loop, and a full
        // circle's start and end vertex are the same entity.
        let closes = walked
            .iter()
            .zip(walked.iter().cycle().skip(1))
            .all(|((_, leaves), (enters, _))| leaves == enters);
        closes.then_some(Self(edges))
    }

    /// The edges, once the caller needs the bare representation truck requires.
    pub fn into_edges(self) -> Vec<CompressedEdgeIndex> {
        self.0
    }

    /// Returns a slice of the edges.
    pub fn edges(&self) -> &[CompressedEdgeIndex] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wire(spec: &[(usize, bool)]) -> Vec<CompressedEdgeIndex> {
        spec.iter()
            .map(|&(index, orientation)| CompressedEdgeIndex { index, orientation })
            .collect()
    }

    /// Edges 0,1,2 form a triangle over vertices 0,1,2.
    fn triangle(index: usize) -> Option<(usize, usize)> {
        match index {
            0 => Some((0, 1)),
            1 => Some((1, 2)),
            2 => Some((2, 0)),
            // A stray edge between two unrelated vertices.
            3 => Some((7, 8)),
            _ => None,
        }
    }

    #[test]
    fn a_triangle_closes() {
        let w = wire(&[(0, true), (1, true), (2, true)]);
        assert!(TopologicallyClosedWire::try_new(w, triangle).is_some());
    }

    #[test]
    fn a_reversed_traversal_closes() {
        // The same triangle walked backwards: order reversed and every edge
        // entered from its far end.
        let w = wire(&[(2, false), (1, false), (0, false)]);
        assert!(TopologicallyClosedWire::try_new(w, triangle).is_some());
    }

    #[test]
    fn a_chain_that_does_not_return_is_refused() {
        // 0 -> 1 -> 2 is a path, not a loop; it never gets back to vertex 0.
        let w = wire(&[(0, true), (1, true)]);
        assert!(TopologicallyClosedWire::try_new(w, triangle).is_none());
    }

    #[test]
    fn an_edge_that_does_not_join_is_refused() {
        // Every edge resolves, so fail-whole-bound conversion accepts this. It
        // still does not close, which is the case this type exists for.
        let w = wire(&[(0, true), (1, true), (3, true), (2, true)]);
        assert!(TopologicallyClosedWire::try_new(w, triangle).is_none());
    }

    #[test]
    fn a_wrongly_oriented_edge_is_refused() {
        let w = wire(&[(0, true), (1, false), (2, true)]);
        assert!(TopologicallyClosedWire::try_new(w, triangle).is_none());
    }

    #[test]
    fn a_single_full_loop_closes() {
        // A circle: one edge whose start and end are the same vertex.
        let w = wire(&[(0, true)]);
        assert!(TopologicallyClosedWire::try_new(w, |_| Some((5, 5))).is_some());
    }

    #[test]
    fn a_single_open_edge_is_refused() {
        let w = wire(&[(0, true)]);
        assert!(TopologicallyClosedWire::try_new(w, |_| Some((5, 6))).is_none());
    }

    #[test]
    fn an_empty_wire_is_refused() {
        assert!(TopologicallyClosedWire::try_new(Vec::new(), triangle).is_none());
    }

    #[test]
    fn an_unresolvable_edge_is_refused() {
        let w = wire(&[(0, true), (99, true)]);
        assert!(TopologicallyClosedWire::try_new(w, triangle).is_none());
    }
}
