//! BIE-007-GATES — χ valuation + mod-2 (Z₂) homology validity gate.
//!
//! The gate's tail layer: an Euler-characteristic valuation and a mod-2
//! homology check over the finite output complex. The implementation is dense
//! Z₂ linear algebra over bitmask rows — Gaussian elimination mod 2. No
//! homology library exists in-tree and none is added (scope decision 2).
//!
//! The pipeline is `diagnose → χ/homology → verdict` (spine §3, scope
//! decision 4): the landed manifold diagnostics ([`manifold::diagnose`]) stay
//! the first gate stage and this layer runs beside them. The gate consumes
//! the finite 2-complex of one shell — the `Shell<P, C, S>` every
//! regularized-boolean output exposes through its solid's boundary — so no
//! edit to `boolean/*` is needed.
//!
//! A homology mismatch is FAILED, never a warning: the typed [`Outcome`]
//! refuses, it does not annotate (booking §5). All iteration is topological
//! (face/edge/vertex index order), never hash-ordered, so identical ordered
//! input yields identical verdicts (spine §8).

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::collections::{HashMap, HashSet};

use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, Margin, Method, Modulus, Outcome, Prop,
    PropMap, Refusal, Truth,
};
use truck_topology::manifold;
use truck_topology::shell::ShellCondition;
use truck_topology::{Edge, EdgeID, Shell, VertexID};

/// The mod-2 (Z₂) Betti numbers `(b0, b1, b2)` of a finite complex.
///
/// `b_i` is the dimension of the `i`-th homology group over Z₂, read off the
/// ranks of the boundary maps `∂₂` and `∂₁` by Gaussian elimination mod 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BettiNumbers {
    /// `dim H₀` — the number of connected components of the vertex-edge
    /// graph (for a closed surface, the number of surface components).
    pub b0: usize,
    /// `dim H₁` — the rank of the first homology group over Z₂.
    pub b1: usize,
    /// `dim H₂` — the number of independent closed 2-cycles that do not
    /// bound (for one closed surface component over Z₂, exactly one).
    pub b2: usize,
}

/// The Euler characteristic and the Z₂ Betti numbers of one finite complex.
///
/// The refined cell census (`V`, `E`, `F`) is carried alongside the
/// invariants so a caller can check `χ = V − E + F` and audit the determinism
/// of the count. `E` counts the 1-cells of the refined complex — the real
/// edges plus the deterministic cross-cuts that cut faces-with-holes into
/// disks — so `χ = V − E + F` is the true surface Euler characteristic even
/// for a shell whose faces are annuli.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HomologyData {
    /// The number of distinct vertices (0-cells) of the complex.
    pub vertices: usize,
    /// The number of distinct 1-cells of the refined complex: the real edges
    /// plus the deterministic cross-cut edges that cut every face-with-holes
    /// into a disk.
    pub edges: usize,
    /// The number of faces (2-cells) of the complex.
    pub faces: usize,
    /// The Euler characteristic `χ = V − E + F`.
    pub chi: isize,
    /// The Z₂ Betti numbers `(b0, b1, b2)`.
    pub betti: BettiNumbers,
}

/// The verdict a [`GateReport`] carries.
///
/// A passing [`Outcome`] carries exactly one verdict; a complex that fails any
/// gate stage is refused as a typed [`Refusal`] and never reaches a report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateVerdict {
    /// The complex cleared every gate stage: it is a single connected closed
    /// orientable surface with even Euler characteristic and `H₂ ≅ H₀ ≅ Z₂`
    /// over Z₂ — the profile of a regularized-boolean output shell.
    Pass,
}

/// The χ/homology gate's report on a complex that cleared every stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateReport {
    /// The Euler characteristic `χ = V − E + F` of the complex.
    pub chi: isize,
    /// The Z₂ Betti numbers `(b0, b1, b2)`.
    pub betti: BettiNumbers,
    /// The gate's verdict. An `Ok` outcome only ever carries a passing
    /// verdict: a mismatch is a typed [`Refusal`], never an annotated pass.
    pub verdict: GateVerdict,
}

/// The validity gate over the finite output complex (spine §3, frozen
/// contract): `diagnose → χ/homology → verdict`.
///
/// Stage 1 runs the landed manifold diagnostics on the shell and refuses on
/// the coedge/orientation faults it reports: a non-`Closed` shell condition, a
/// boundary edge, an irregular (over-shared or same-face) edge use, or an
/// orientation conflict. (The diagnostics' per-vertex link classification is
/// deliberately NOT a hard condition: a regularized-boolean rim is a two-edge
/// digon whose seam vertices the link classifier over-reports as
/// `NonManifold`, so a valid output would be refused.)
///
/// Stage 2 computes χ and the Z₂ Betti numbers and refuses unless the complex
/// has the profile of one connected closed orientable surface — `b0 = b2 = 1`
/// (mod-2 Poincaré duality `H₂ ≅ H₀`) and an even Euler characteristic —
/// which is exactly the profile every regularized-boolean output shell has. A
/// mismatch is a typed refusal, never a warning.
pub fn chi_homology_gate<P, C, S>(complex: &Shell<P, C, S>) -> Outcome<GateReport> {
    let diagnosis = manifold::diagnose(complex);
    if diagnosis.shell_condition != ShellCondition::Closed
        || !diagnosis.boundary_edges.is_empty()
        || !diagnosis.irregular_edges.is_empty()
        || !diagnosis.orientation_conflicts.is_empty()
    {
        return Err(coedge_pairing_refusal());
    }
    let Some(homology) = mod2_homology(complex) else {
        return Err(homology_refusal());
    };
    if homology.betti.b0 != 1 || homology.betti.b2 != 1 || homology.chi % 2 != 0 {
        return Err(homology_refusal());
    }
    let cert = Certificate {
        props: PropMap::new(),
        // The gate is discrete integer algebra over the substrate's exact
        // entity identities: no float arithmetic enters, so `Exact` is the
        // honest method (H-6).
        method: Method::Exact,
        budget_left: Budget::new(0, 0, 0),
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    };
    Ok(Certified::new(
        GateReport {
            chi: homology.chi,
            betti: homology.betti,
            verdict: GateVerdict::Pass,
        },
        cert,
    ))
}

/// Computes χ and the Z₂ Betti numbers of one shell's finite complex.
///
/// Vertices and faces are counted as distinct entities (by id) in
/// deterministic first-seen order over `face_iter()` / wire order. A face of a
/// regularized-boolean shell is a compact surface patch that may have several
/// boundary wires (an annulus, e.g. a cap with a hole), which is NOT a disk
/// and so is not a valid CW 2-cell: the raw `V − E + F` over its wires is not
/// the surface Euler characteristic and its raw 1-skeleton is disconnected.
/// The complex is therefore refined deterministically: every face with `w`
/// boundary wires gets `w − 1` virtual cross-cut 1-cells joining each extra
/// wire's first vertex to the first wire's first vertex. A cross-cut appears
/// twice in the split face's boundary and cancels mod 2, so `∂₂` rows are
/// unchanged, but the refined 1-skeleton now reflects the closed surface's
/// true connectivity and the refined `V − E + F` is the true Euler
/// characteristic. This subdivision preserves the topology, so the mod-2
/// homology of the refined complex is the homology of the shell.
///
/// The boundary maps `∂₁: C₁ → C₀` (each 1-cell to its two endpoint vertices,
/// which cancel mod 2 on a loop) and `∂₂: C₂ → C₁` (each face to the mod-2 sum
/// of its boundary edges) are built as bitmask rows and reduced by Gaussian
/// elimination mod 2. Returns `None` only for a complex whose rows are not a
/// valid chain complex (`im ∂₂ ⊄ ker ∂₁`), which cannot arise from a shell of
/// closed wire faces but is refused defensively rather than trusted.
pub fn mod2_homology<P, C, S>(shell: &Shell<P, C, S>) -> Option<HomologyData> {
    let faces = shell.face_iter().count();

    let mut unique_edges: Vec<Edge<P, C>> = Vec::new();
    let mut seen_edges: HashSet<EdgeID<C>> = HashSet::new();
    for face in shell.face_iter() {
        for wire in face.absolute_boundaries() {
            for edge in wire.iter() {
                let id = edge.id();
                if seen_edges.insert(id) {
                    unique_edges.push(edge.clone());
                }
            }
        }
    }
    let edges = unique_edges.len();

    let mut unique_vertices: Vec<VertexID<P>> = Vec::new();
    let mut seen_vertices: HashSet<VertexID<P>> = HashSet::new();
    for edge in &unique_edges {
        let (front, back) = edge.absolute_ends();
        for vertex in [front.id(), back.id()] {
            if seen_vertices.insert(vertex) {
                unique_vertices.push(vertex);
            }
        }
    }
    let vertices = unique_vertices.len();

    let mut vertex_ordinal: HashMap<VertexID<P>, usize> = HashMap::new();
    for (ordinal, id) in unique_vertices.iter().enumerate() {
        vertex_ordinal.insert(*id, ordinal);
    }
    let mut edge_ordinal: HashMap<EdgeID<C>, usize> = HashMap::new();
    for (ordinal, edge) in unique_edges.iter().enumerate() {
        edge_ordinal.insert(edge.id(), ordinal);
    }

    // Refinement cross-cuts: for each face with several boundary wires, join
    // every extra wire's first vertex to the first wire's first vertex. These
    // virtual 1-cells are appended after the real edges; no `∂₂` row touches
    // them (each appears twice and cancels mod 2).
    let mut crosscuts: Vec<(VertexID<P>, VertexID<P>)> = Vec::new();
    for face in shell.face_iter() {
        let wires = face.absolute_boundaries();
        let anchor = wires
            .first()
            .and_then(|wire| wire.iter().next())
            .map(|edge| edge.front().id());
        for wire in wires.iter().skip(1) {
            let Some(anchor) = anchor else {
                break;
            };
            let Some(first) = wire.iter().next() else {
                continue;
            };
            crosscuts.push((anchor, first.front().id()));
        }
    }
    let edges_total = edges + crosscuts.len();

    // ∂₁: each real edge, then each cross-cut, maps to its two endpoints.
    let mut boundary1: Vec<Vec<u64>> = Vec::new();
    for edge in &unique_edges {
        let mut row = vec![0u64; num_limbs(vertices)];
        let (front, back) = edge.absolute_ends();
        if let Some(ordinal) = vertex_ordinal.get(&front.id()) {
            toggle_bit(&mut row, *ordinal);
        }
        if let Some(ordinal) = vertex_ordinal.get(&back.id()) {
            toggle_bit(&mut row, *ordinal);
        }
        boundary1.push(row);
    }
    for (front, back) in &crosscuts {
        let mut row = vec![0u64; num_limbs(vertices)];
        if let Some(ordinal) = vertex_ordinal.get(front) {
            toggle_bit(&mut row, *ordinal);
        }
        if let Some(ordinal) = vertex_ordinal.get(back) {
            toggle_bit(&mut row, *ordinal);
        }
        boundary1.push(row);
    }

    // ∂₂: each face (one row per face) maps to its boundary edges.
    let mut boundary2: Vec<Vec<u64>> = Vec::new();
    for face in shell.face_iter() {
        let mut row = vec![0u64; num_limbs(edges)];
        for wire in face.absolute_boundaries() {
            for edge in wire.iter() {
                if let Some(ordinal) = edge_ordinal.get(&edge.id()) {
                    toggle_bit(&mut row, *ordinal);
                }
            }
        }
        boundary2.push(row);
    }

    let rank1 = row_rank_mod2(&mut boundary1);
    let rank2 = row_rank_mod2(&mut boundary2);

    let b0 = vertices as isize - rank1 as isize;
    let b1 = edges_total as isize - rank1 as isize - rank2 as isize;
    let b2 = faces as isize - rank2 as isize;
    if b0 < 0 || b1 < 0 || b2 < 0 {
        return None;
    }
    let chi = vertices as isize - edges_total as isize + faces as isize;
    Some(HomologyData {
        vertices,
        edges: edges_total,
        faces,
        chi,
        betti: BettiNumbers {
            b0: b0 as usize,
            b1: b1 as usize,
            b2: b2 as usize,
        },
    })
}

/// The stage-1 refusal: the shell condition / coedge pairing is broken
/// (a boundary, an over-shared or same-direction edge use).
fn coedge_pairing_refusal() -> Refusal {
    Refusal::Contradictory(ContradictionWitness {
        prop: Prop::CoedgePairing,
        left: Truth::True,
        right: Truth::False,
    })
}

/// The stage-2 refusal: the χ/homology profile is not that of one connected
/// closed orientable surface.
fn homology_refusal() -> Refusal {
    Refusal::Contradictory(ContradictionWitness {
        prop: Prop::EulerPoincare,
        left: Truth::True,
        right: Truth::False,
    })
}

/// The number of 64-bit limbs needed to address `bits` distinct columns.
fn num_limbs(bits: usize) -> usize {
    bits.div_ceil(64)
}

/// Splits a column index into its limb and in-limb bit position.
fn split_index(index: usize) -> (usize, u64) {
    (index / 64, (index % 64) as u64)
}

/// Whether bit `index` of `row` is set.
fn bit_at(row: &[u64], index: usize) -> bool {
    let (limb, bit) = split_index(index);
    row.get(limb)
        .is_some_and(|word| ((word >> bit) & 1u64) != 0)
}

/// Adds one (mod 2) to bit `index` of `row`: a second occurrence of the same
/// column in one face cancels, exactly as an edge traversed twice would.
fn toggle_bit(row: &mut [u64], index: usize) {
    let (limb, bit) = split_index(index);
    if let Some(word) = row.get_mut(limb) {
        *word ^= 1u64 << bit;
    }
}

/// XORs `other` into `target` (both are mod-2 rows of the same width).
fn xor_row(target: &mut [u64], other: &[u64]) {
    for (word, mask) in target.iter_mut().zip(other.iter()) {
        *word ^= *mask;
    }
}

/// The row rank of a matrix over Z₂ by Gaussian elimination.
///
/// Each row is a bitmask of equal width. Columns are processed left to right;
/// every nonzero pivot eliminates its column from all other rows, so the
/// number of pivots found is the rank. Deterministic and allocation-local —
/// no geometry, no floats, no failure modes.
fn row_rank_mod2(rows: &mut [Vec<u64>]) -> usize {
    let mut columns = 0usize;
    for row in rows.iter() {
        columns = columns.max(row.len());
    }
    columns *= 64;
    let mut pivot = 0usize;
    for index in 0..columns {
        let Some(found) = find_pivot_row(rows, pivot, index) else {
            continue;
        };
        if found != pivot {
            rows.swap(pivot, found);
        }
        let Some(basis) = rows.get(pivot).cloned() else {
            break;
        };
        for (at, row) in rows.iter_mut().enumerate() {
            if at == pivot {
                continue;
            }
            if bit_at(row, index) {
                xor_row(row, &basis);
            }
        }
        pivot += 1;
    }
    pivot
}

/// The first row at or after `from` whose bit `index` is set.
fn find_pivot_row(rows: &[Vec<u64>], from: usize, index: usize) -> Option<usize> {
    rows.iter()
        .enumerate()
        .skip(from)
        .find(|(_, row)| bit_at(row, index))
        .map(|(at, _)| at)
}
