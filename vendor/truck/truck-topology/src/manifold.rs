#![deny(clippy::unwrap_used)]

//! BG-CG-006-DIAG — the actionable manifold-diagnostics aggregate.
//!
//! Aggregates the shell substrate into one answer with per-entity detail:
//! what is wrong, where, and in which classification. Analysis only —
//! nothing here mutates its input, and no repair is offered (a separate
//! explicit op may apply a parity assignment later; the plan books it).

use crate::shell::ShellCondition;
use crate::{Edge, EdgeID, Face, FaceID, Shell, VertexID};
use std::collections::{HashMap, HashSet, VecDeque};
use std::marker::PhantomData;

/// How one vertex's link is shaped (plan §3.6, normative):
/// closed 2-manifold ⇒ the link is one cycle; manifold-with-boundary ⇒ one
/// path; two sheets touching at the vertex (or any branching) ⇒ nonmanifold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexLinkClass {
    /// The link is exactly one cycle — a closed 2-manifold at this vertex.
    ClosedCycle,
    /// The link is exactly one path — a manifold boundary at this vertex.
    BoundaryPath,
    /// The link is disconnected or has a vertex of degree ≠ 2 — sheets
    /// touch or branch here.
    NonManifold,
    /// No edge uses this vertex (degenerate).
    Isolated,
}

/// One vertex's diagnosis.
#[derive(Debug, Clone, PartialEq)]
pub struct VertexDiagnostic<P> {
    /// Which vertex.
    pub vertex: VertexID<P>,
    /// How its link is shaped.
    pub classification: VertexLinkClass,
}

/// How one edge is irregular. (An edge used by exactly two faces with
/// opposite effective directions is regular and gets NO entry; boundary
/// edges go to `ManifoldDiagnostics::boundary_edges`.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeIrregularity {
    /// More than two face-uses traverse the edge; `use_count` is the total.
    OverShared {
        /// The number of face-uses.
        use_count: usize,
    },
    /// The same face uses the edge twice (a fin).
    DoublyUsedByOneFace,
    /// Exactly two faces share the edge but traverse it in the SAME
    /// effective direction — an orientation conflict on this edge.
    SameDirectionUses,
}

/// One edge's diagnosis.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeDiagnostic<P, C> {
    /// Which edge.
    pub edge: EdgeID<C>,
    /// How it is irregular.
    pub classification: EdgeIrregularity,
    /// The public signature carries `P` (uniform with the other diagnostics)
    /// but an edge irregularity names only the edge, so the marker consumes
    /// the parameter (rustc E0392).
    _marker: PhantomData<P>,
}

/// One conflicting edge use pair, named by entity (plan §3.6: "the
/// conflicting edges/faces").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrientationConflict<P, C, S> {
    /// The shared edge.
    pub edge: EdgeID<C>,
    /// One incident face.
    pub face_a: FaceID<S>,
    /// The other incident face.
    pub face_b: FaceID<S>,
    /// The public signature carries `P` (uniform with the other diagnostics)
    /// but a conflict names only an edge and two faces, so the marker
    /// consumes the parameter (rustc E0392).
    _marker: PhantomData<P>,
}

/// The whole answer for one shell. Every field is derived from the substrate;
/// nothing here re-derives what `Shell` already knows.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifoldDiagnostics<P, C, S> {
    /// The substrate's own half-edge verdict, verbatim.
    pub shell_condition: ShellCondition,
    /// How many pieces the shell is in (`connected_components().len()`).
    pub connected_components: usize,
    /// Edges used by exactly one face, in deterministic order.
    pub boundary_edges: Vec<EdgeID<C>>,
    /// Per-edge irregularities, in deterministic order.
    pub irregular_edges: Vec<EdgeDiagnostic<P, C>>,
    /// Per-vertex link classifications, in deterministic order. Every vertex
    /// of the shell appears exactly once (not only singular ones) — a caller
    /// filtering for trouble filters on the classification.
    pub singular_vertices: Vec<VertexDiagnostic<P>>,
    /// Every orientation conflict found by the parity walk, in deterministic
    /// order. Empty iff the shell's face orientations are mutually
    /// consistent.
    pub orientation_conflicts: Vec<OrientationConflict<P, C, S>>,
}

/// Diagnoses a shell: the aggregate of the substrate with per-entity detail.
/// Never panics, never mutates, never repairs.
pub fn diagnose<P, C, S>(shell: &Shell<P, C, S>) -> ManifoldDiagnostics<P, C, S> {
    let vertex_ordinal: HashMap<VertexID<P>, usize> = shell
        .vertex_iter()
        .enumerate()
        .map(|(i, vertex)| (vertex.id(), i))
        .collect();
    let edge_ordinal: HashMap<EdgeID<C>, usize> = shell
        .edge_iter()
        .enumerate()
        .map(|(i, edge)| (edge.id(), i))
        .collect();

    // Edge census: every face-use of every edge, with its effective
    // direction (stored direction XOR the face's orientation).
    let mut uses: HashMap<EdgeID<C>, Vec<(usize, bool)>> = HashMap::new();
    for (face_ord, face) in shell.face_iter().enumerate() {
        let face_orientation = face.orientation();
        for wire in face.absolute_boundaries() {
            for edge in wire {
                let effective_direction = edge.orientation() ^ face_orientation;
                uses.entry(edge.id())
                    .or_default()
                    .push((face_ord, effective_direction));
            }
        }
    }

    let mut boundary_edges: Vec<EdgeID<C>> = Vec::new();
    let mut irregular_edges: Vec<EdgeDiagnostic<P, C>> = Vec::new();
    let mut conflicts: Vec<(usize, usize, usize)> = Vec::new();
    let mut ordered_edges: Vec<(usize, EdgeID<C>)> = uses
        .keys()
        .filter_map(|edge| edge_ordinal.get(edge).map(|&ord| (ord, *edge)))
        .collect();
    ordered_edges.sort_by_key(|&(ord, _)| ord);
    for (ord, edge) in ordered_edges {
        let uses_on_edge = match uses.get(&edge) {
            Some(list) => list,
            None => continue,
        };
        match uses_on_edge.len() {
            1 => boundary_edges.push(edge),
            2 => {
                let (face_a, direction_a) = match uses_on_edge.first() {
                    Some(use_) => *use_,
                    None => continue,
                };
                let (face_b, direction_b) = match uses_on_edge.get(1) {
                    Some(use_) => *use_,
                    None => continue,
                };
                if face_a == face_b {
                    irregular_edges.push(EdgeDiagnostic {
                        edge,
                        classification: EdgeIrregularity::DoublyUsedByOneFace,
                        _marker: PhantomData,
                    });
                } else if direction_a == direction_b {
                    irregular_edges.push(EdgeDiagnostic {
                        edge,
                        classification: EdgeIrregularity::SameDirectionUses,
                        _marker: PhantomData,
                    });
                    let (low, high) = if face_a < face_b {
                        (face_a, face_b)
                    } else {
                        (face_b, face_a)
                    };
                    conflicts.push((ord, low, high));
                }
            }
            count => irregular_edges.push(EdgeDiagnostic {
                edge,
                classification: EdgeIrregularity::OverShared { use_count: count },
                _marker: PhantomData,
            }),
        }
    }

    // Vertex links: each face occurrence of a vertex contributes one link
    // edge between its predecessor and successor in the effective wire.
    let mut link_edges: HashMap<VertexID<P>, Vec<LinkEdge<P>>> = HashMap::new();
    for face in shell.face_iter() {
        for wire in face.boundaries() {
            let edges: Vec<&Edge<P, C>> = wire.iter().collect();
            let count = edges.len();
            if count == 0 {
                continue;
            }
            for (i, current) in edges.iter().enumerate() {
                let prev = match edges.get((i + count - 1) % count) {
                    Some(edge) => *edge,
                    None => continue,
                };
                let current = *current;
                let vertex = current.front().id();
                let pred = prev.front().id();
                let succ = current.back().id();
                link_edges.entry(vertex).or_default().push((pred, succ));
            }
        }
    }
    let mut ordered_vertices: Vec<(usize, VertexID<P>)> = vertex_ordinal
        .keys()
        .filter_map(|vertex| vertex_ordinal.get(vertex).map(|&ord| (ord, *vertex)))
        .collect();
    ordered_vertices.sort_by_key(|&(ord, _)| ord);
    let singular_vertices: Vec<VertexDiagnostic<P>> = ordered_vertices
        .into_iter()
        .map(|(_, vertex)| {
            let edges_at = link_edges.get(&vertex);
            let classification = match edges_at {
                Some(list) if !list.is_empty() => classify_link(list),
                _ => VertexLinkClass::Isolated,
            };
            VertexDiagnostic {
                vertex,
                classification,
            }
        })
        .collect();

    // Orientation conflicts, ordered by (edge ordinal, face ordinal).
    conflicts.sort_by_key(|&(ord, low, high)| (ord, low, high));
    let edges: Vec<EdgeID<C>> = shell.edge_iter().map(|edge| edge.id()).collect();
    let faces: Vec<FaceID<S>> = shell.face_iter().map(Face::id).collect();
    let orientation_conflicts: Vec<OrientationConflict<P, C, S>> = conflicts
        .iter()
        .filter_map(|&(ord, face_a_ord, face_b_ord)| {
            let edge = *edges.get(ord)?;
            let face_a = *faces.get(face_a_ord)?;
            let face_b = *faces.get(face_b_ord)?;
            Some(OrientationConflict {
                edge,
                face_a,
                face_b,
                _marker: PhantomData,
            })
        })
        .collect();

    ManifoldDiagnostics {
        shell_condition: shell.shell_condition(),
        connected_components: shell.connected_components().len(),
        boundary_edges,
        irregular_edges,
        singular_vertices,
        orientation_conflicts,
    }
}

/// Classifies the link multigraph at one vertex.
///
/// Any link vertex of degree ≥ 3 (branching) is nonmanifold; a link that is
/// all degree 2 in one component is a closed cycle; a single component with
/// exactly two degree-1 endpoints (all other degrees 2) is a boundary path;
/// everything else (disconnected, or a mixed shape) is nonmanifold.
fn classify_link<P>(edges_at: &[(VertexID<P>, VertexID<P>)]) -> VertexLinkClass {
    let mut adjacency: HashMap<VertexID<P>, Vec<VertexID<P>>> = HashMap::new();
    for &(pred, succ) in edges_at {
        adjacency.entry(pred).or_default().push(succ);
        adjacency.entry(succ).or_default().push(pred);
    }
    let mut branched = false;
    let mut all_degree_two = true;
    let mut degree_one = 0;
    for neighbors in adjacency.values() {
        let degree = neighbors.len();
        if degree == 1 {
            degree_one += 1;
        } else if degree != 2 {
            branched = true;
        }
        if degree != 2 {
            all_degree_two = false;
        }
    }
    let components = link_components(&adjacency);
    if branched {
        VertexLinkClass::NonManifold
    } else if all_degree_two {
        if components == 1 {
            VertexLinkClass::ClosedCycle
        } else {
            VertexLinkClass::NonManifold
        }
    } else if degree_one == 2 && components == 1 {
        VertexLinkClass::BoundaryPath
    } else {
        VertexLinkClass::NonManifold
    }
}

/// Counts the connected components of a small undirected graph given as an
/// adjacency map.
fn link_components<P>(adjacency: &HashMap<VertexID<P>, Vec<VertexID<P>>>) -> usize {
    let mut seen: HashSet<VertexID<P>> = HashSet::new();
    let mut count = 0;
    for start in adjacency.keys() {
        let start = *start;
        if !seen.insert(start) {
            continue;
        }
        count += 1;
        let mut stack: Vec<VertexID<P>> = vec![start];
        while let Some(current) = stack.pop() {
            if let Some(neighbors) = adjacency.get(&current) {
                for &next in neighbors {
                    if seen.insert(next) {
                        stack.push(next);
                    }
                }
            }
        }
    }
    count
}

/// A deterministic adjacency row: the neighbor face and its shared edges.
type AdjacencyRow<'a, P, C, S> = (&'a Face<P, C, S>, Vec<EdgeID<C>>);

/// One link edge contributed by a face occurrence of a vertex: the vertex's
/// predecessor and successor in that wire's cyclic order.
type LinkEdge<P> = (VertexID<P>, VertexID<P>);

/// The effective direction of a face's use of `edge`: the stored (absolute)
/// direction XOR the face's orientation, matching the census.
fn edge_effective_direction<P, C, S>(face: &Face<P, C, S>, edge: EdgeID<C>) -> Option<bool> {
    for wire in face.absolute_boundaries() {
        for edge_use in wire {
            if edge_use.id() == edge {
                return Some(edge_use.orientation() ^ face.orientation());
            }
        }
    }
    None
}

/// A consistent orientation parity assignment (face ordinal -> flip flag:
/// `true` = the face's stored orientation is already consistent), or `None`
/// when the shell's orientations conflict. BFS over `face_adjacency()`
/// starting from the lowest-ordinal face assigned `true`; crossing a shared
/// edge to the next face requires opposite effective edge directions.
/// Deterministic; analysis only — applying an assignment is somebody else's
/// explicit op.
pub fn orientation_parity<P, C, S>(shell: &Shell<P, C, S>) -> Option<HashMap<FaceID<S>, bool>> {
    let face_ordinal: HashMap<FaceID<S>, usize> = shell
        .face_iter()
        .enumerate()
        .map(|(i, face)| (face.id(), i))
        .collect();
    let mut edge_ordinal: HashMap<EdgeID<C>, usize> = HashMap::new();
    let mut next_edge_ordinal = 0;
    for face in shell.face_iter() {
        for wire in face.absolute_boundaries() {
            for edge in wire {
                let id = edge.id();
                edge_ordinal.entry(id).or_insert_with(|| {
                    let ordinal = next_edge_ordinal;
                    next_edge_ordinal += 1;
                    ordinal
                });
            }
        }
    }
    let adjacency = shell.face_adjacency();
    let mut parity: HashMap<FaceID<S>, bool> = HashMap::new();
    let mut queue: VecDeque<&Face<P, C, S>> = VecDeque::new();
    let faces: Vec<&Face<P, C, S>> = shell.face_iter().collect();
    for &seed in &faces {
        if parity.contains_key(&seed.id()) {
            continue;
        }
        parity.insert(seed.id(), true);
        queue.push_back(seed);
        while let Some(current) = queue.pop_front() {
            let adjacents = match adjacency.get(current) {
                Some(list) => list,
                None => continue,
            };
            let mut ordered: Vec<AdjacencyRow<'_, P, C, S>> = adjacents
                .iter()
                .map(|adjacent| {
                    let mut common: Vec<EdgeID<C>> = adjacent.common_edges.to_vec();
                    common
                        .sort_by_key(|edge| edge_ordinal.get(edge).copied().unwrap_or(usize::MAX));
                    (adjacent.face, common)
                })
                .collect();
            ordered.sort_by_key(|(face, _)| {
                face_ordinal.get(&face.id()).copied().unwrap_or(usize::MAX)
            });
            for (next_face, common) in ordered {
                for edge in common {
                    let direction_a = edge_effective_direction(current, edge);
                    let direction_b = edge_effective_direction(next_face, edge);
                    let (direction_a, direction_b) = match (direction_a, direction_b) {
                        (Some(a), Some(b)) => (a, b),
                        _ => continue,
                    };
                    if direction_a == direction_b {
                        return None;
                    }
                }
                if let std::collections::hash_map::Entry::Vacant(entry) =
                    parity.entry(next_face.id())
                {
                    entry.insert(true);
                    queue.push_back(next_face);
                }
            }
        }
    }
    Some(parity)
}
