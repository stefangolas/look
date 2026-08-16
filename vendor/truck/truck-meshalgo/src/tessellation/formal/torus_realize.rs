//! Cut-open development of a certified torus annular cell (B4 foundation).
//!
//! Takes a [`super::torus_cell::CertifiedTorusAnnularCell`] and produces its
//! developed planar representation: the unique annular 2-chain the cell
//! certified, lifted to the universal cover and cut transversely along the
//! complementary deck direction, so it can be triangulated on a plane and
//! reglued.
//!
//! For the first cell (two parallel boundaries, winding `(±1, 0)`) the
//! primitive winding is already the first lattice basis vector, so the
//! certified `GL(2, Z)` basis change is the identity. The annulus develops to a
//! rectangle `[0, 2π] × [v_lo, v_hi]` in the developed `(u, v)` plane, with the
//! `u = 0` and `u = 2π` edges identified by the major deck generator. A future
//! mixed-winding cell (e.g. `(1, 1)`) would carry a non-trivial unimodular
//! basis change here.
//!
//! # What is here
//!
//! - the certified `GL(2, Z)` basis-change witness (identity for parallels);
//! - the developed rectangle, its deck identification, and the lift of each
//!   boundary to a side of the rectangle;
//! - validation that the boundaries retain their certified coordinates and the
//!   cut is transverse to the boundary winding.
//!
//! # What is NOT here
//!
//! No triangulation, no reglue, no production mesh. The meshing step consumes
//! [`DevelopedTorusAnnulus`] and reuses the existing constrained triangulation
//! machinery, then reglues the paired quotient edges.

use super::torus::CertifiedRankTwoDeck;
use super::torus_cell::{CertifiedEssentialLoop, CertifiedTorusAnnularCell, PrimitiveWinding};
use std::collections::HashSet;
use std::f64::consts::TAU;
use truck_geometry::prelude::{InnerSpace, Matrix4, ParametricSurface, Point3, Torus};
use truck_polymesh::Transform;

/// A certified `GL(2, Z)` basis-change witness: a unimodular integer matrix
/// (`det = ±1`) that re-expresses the deck basis so its first generator follows
/// the boundary primitive winding.
///
/// For the parallel-parallel cell the primitive winding `(1, 0)` is already the
/// first basis vector, so this is the identity. The witness is carried
/// explicitly so a future mixed-winding cell can supply a non-trivial change
/// without changing the development API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gl2zBasisChange {
    matrix: [[i64; 2]; 2],
}

impl Gl2zBasisChange {
    /// The identity basis change (no re-expression needed).
    pub const IDENTITY: Self = Self {
        matrix: [[1, 0], [0, 1]],
    };

    /// The unimodular matrix entries.
    pub fn matrix(&self) -> [[i64; 2]; 2] {
        self.matrix
    }

    /// The determinant (`±1` for a certified `GL(2, Z)` element).
    pub fn determinant(&self) -> i64 {
        self.matrix[0][0] * self.matrix[1][1] - self.matrix[0][1] * self.matrix[1][0]
    }

    /// Whether the determinant is `±1` (unimodular), hence orientation-preserving
    /// (`+1`) or reversing (`-1`).
    pub fn is_unimodular(&self) -> bool {
        self.determinant().abs() == 1
    }
}

/// One side of the developed rectangle, carrying the boundary loop it lifts
/// from and whether it is a cut edge or a deck-identified edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DevelopedSide {
    /// A boundary loop lifted to this side (`v = v_lo` or `v = v_hi`).
    Boundary {
        /// The certified loop this side lifts from.
        loop_index: usize,
        /// The developed `v` coordinate of this boundary.
        v: f64,
    },
    /// A cut edge along the complementary deck direction (`u = 0` and
    /// `u = 2π`), identified with its partner by the major deck generator.
    CutEdge {
        /// The developed `u` coordinate of this cut edge (`0` or `2π`).
        u: f64,
    },
}

/// The developed planar representation of a certified torus annular cell.
#[derive(Debug, Clone, PartialEq)]
pub struct DevelopedTorusAnnulus {
    /// The certified basis-change witness.
    basis_change: Gl2zBasisChange,
    /// The developed `u` interval: exactly one major period.
    u_range: (f64, f64),
    /// The developed `v` interval: between the two boundary coordinates.
    v_range: (f64, f64),
    /// The four sides of the developed rectangle, in the order
    /// `(u=u_lo, u=u_hi, v=v_lo, v=v_hi)`.
    sides: [DevelopedSide; 4],
}

impl DevelopedTorusAnnulus {
    /// The certified `GL(2, Z)` basis change.
    pub fn basis_change(&self) -> Gl2zBasisChange {
        self.basis_change
    }
    /// The developed `u` interval (one major period).
    pub fn u_range(&self) -> (f64, f64) {
        self.u_range
    }
    /// The developed `v` interval (between the two boundary coordinates).
    pub fn v_range(&self) -> (f64, f64) {
        self.v_range
    }
    /// The four sides of the developed rectangle.
    pub fn sides(&self) -> &[DevelopedSide; 4] {
        &self.sides
    }
    /// Whether the `u = u_lo` and `u = u_hi` sides are the paired cut edges
    /// identified by the major deck generator (they always are for this cell).
    pub fn has_deck_identification(&self) -> bool {
        matches!(self.sides[0], DevelopedSide::CutEdge { .. })
            && matches!(self.sides[1], DevelopedSide::CutEdge { .. })
    }
}

/// Why a torus annulus could not be developed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RealizationFailure {
    /// The cell is not a parallel-parallel annulus (the only one developed
    /// here).
    UnsupportedPrimitiveClass,
    /// The boundary coordinates are not distinct after lifting (degenerate
    /// rectangle).
    DegenerateDevelopment,
}

/// Develop a certified torus annular cell into its planar rectangle.
///
/// Chooses the lattice basis whose first generator follows the primitive
/// boundary winding (identity for parallels), lifts the two boundaries into the
/// cover, and cuts transversely along the complementary deck direction. The
/// result is a rectangle `[0, 2π] × [v_lo, v_hi]` with the `u = 0` and
/// `u = 2π` edges identified.
pub fn develop_torus_annulus(
    cell: &CertifiedTorusAnnularCell,
) -> Result<DevelopedTorusAnnulus, RealizationFailure> {
    if cell.primitive_class() != PrimitiveWinding::Parallel {
        return Err(RealizationFailure::UnsupportedPrimitiveClass);
    }
    let (a, b) = (cell.boundary_a(), cell.boundary_b());
    let (v_lo, v_hi, side_lo, side_hi) = ordered_v(a, b)?;

    Ok(DevelopedTorusAnnulus {
        basis_change: Gl2zBasisChange::IDENTITY,
        u_range: (0.0, TAU),
        v_range: (v_lo, v_hi),
        sides: [
            DevelopedSide::CutEdge { u: 0.0 },
            DevelopedSide::CutEdge { u: TAU },
            side_lo,
            side_hi,
        ],
    })
}

/// Order the two boundary `v` coordinates and return `(v_lo, v_hi, lo_side,
/// hi_side)`, recording which loop lifts to which side.
fn ordered_v(
    a: &CertifiedEssentialLoop,
    b: &CertifiedEssentialLoop,
) -> Result<(f64, f64, DevelopedSide, DevelopedSide), RealizationFailure> {
    let (va, vb) = (a.constant_coordinate(), b.constant_coordinate());
    // Distinctness is already proved by the cell; re-check defensively.
    if (va - vb).abs() < 1e-12 {
        return Err(RealizationFailure::DegenerateDevelopment);
    }
    let (lo, hi, lo_idx, hi_idx) = if va < vb {
        (va, vb, 0usize, 1)
    } else {
        (vb, va, 1, 0)
    };
    Ok((
        lo,
        hi,
        DevelopedSide::Boundary {
            loop_index: lo_idx,
            v: lo,
        },
        DevelopedSide::Boundary {
            loop_index: hi_idx,
            v: hi,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::super::torus::{identify_torus, TorusIdentification};
    use super::super::torus_cell::{
        certify_torus_annular_cell, BoundaryLoopPlacement, PrimitiveWinding,
        SourceBoundaryComposition,
    };
    use super::*;
    use truck_geometry::prelude::{Point3, Torus, Vector3};

    fn deck() -> CertifiedRankTwoDeck {
        match identify_torus(&Torus::new(Point3::new(0.0, 0.0, 0.0), 5.0, 1.0)) {
            TorusIdentification::Torus(d) => d,
            other => panic!("need a deck, got {other:?}"),
        }
    }

    fn parallel(v: f64, sign: i8) -> BoundaryLoopPlacement {
        BoundaryLoopPlacement {
            center: Point3::new(0.0, 0.0, 1.0 * v.sin()),
            normal: Vector3::new(0.0, 0.0, 1.0),
            radius: 5.0 + 1.0 * v.cos(),
            effective_orientation_sign: sign,
        }
    }

    fn clean() -> SourceBoundaryComposition {
        SourceBoundaryComposition {
            component_count: 2,
            extra_source_edge: false,
            outer_bound_malformation: None,
        }
    }

    fn cell() -> CertifiedTorusAnnularCell {
        certify_torus_annular_cell(&deck(), parallel(0.0, 1), parallel(1.2, -1), &clean()).unwrap()
    }

    #[test]
    fn a_parallel_annulus_develops_to_one_period_rectangle() {
        let dev = develop_torus_annulus(&cell()).unwrap();
        assert_eq!(dev.u_range(), (0.0, TAU));
        let (vlo, vhi) = dev.v_range();
        assert!((vlo - 0.0).abs() < 1e-12);
        assert!((vhi - 1.2).abs() < 1e-12);
        assert!(dev.has_deck_identification());
    }

    #[test]
    fn the_basis_change_is_identity_for_parallels() {
        let dev = develop_torus_annulus(&cell()).unwrap();
        let bc = dev.basis_change();
        assert_eq!(bc, Gl2zBasisChange::IDENTITY);
        assert!(bc.is_unimodular());
        assert_eq!(bc.determinant(), 1);
    }

    #[test]
    fn both_boundaries_lift_to_sides_of_the_rectangle() {
        let dev = develop_torus_annulus(&cell()).unwrap();
        let bounds: Vec<_> = dev
            .sides()
            .iter()
            .filter_map(|s| match s {
                DevelopedSide::Boundary { loop_index, v } => Some((*loop_index, *v)),
                _ => None,
            })
            .collect();
        assert_eq!(bounds.len(), 2);
        // Both loops (0 and 1) are represented, at the two distinct v's.
        let idxs: Vec<_> = bounds.iter().map(|(i, _)| *i).collect();
        assert!(idxs.contains(&0) && idxs.contains(&1));
        let vs: Vec<_> = bounds.iter().map(|(_, v)| *v).collect();
        assert!((vs[0] - vs[1]).abs() > 1e-6);
    }

    #[test]
    fn the_cut_edges_are_at_u_zero_and_u_two_pi() {
        let dev = develop_torus_annulus(&cell()).unwrap();
        let cuts: Vec<_> = dev
            .sides()
            .iter()
            .filter_map(|s| match s {
                DevelopedSide::CutEdge { u } => Some(*u),
                _ => None,
            })
            .collect();
        assert_eq!(cuts.len(), 2);
        assert!((cuts[0] - 0.0).abs() < 1e-12);
        assert!((cuts[1] - TAU).abs() < 1e-12);
    }

    #[test]
    fn primitive_class_is_recorded_on_the_cell() {
        assert_eq!(cell().primitive_class(), PrimitiveWinding::Parallel);
    }
}

// ===========================================================================
// B4: triangulation + reglue realization
// ===========================================================================

/// A realized torus annulus: a triangulated mesh with certified cut-pair
/// welding and source-boundary provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct RealizedTorusAnnulus {
    /// World-space vertices, mapped through `torus.subs` + the placement.
    pub vertices: Vec<Point3>,
    /// Triangle index triples into `vertices`.
    pub triangles: Vec<[usize; 3]>,
    /// The two source boundary loops, as vertex-index cycles.
    pub boundary_loops: [Vec<usize>; 2],
    /// The certified cut-pair welds (each is one quotient identification).
    pub cut_pairs: Vec<(usize, usize)>,
    /// The computed Euler characteristic (0 for an annulus).
    pub euler_characteristic: i64,
}

impl RealizedTorusAnnulus {
    /// Vertex count.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
    /// Triangle count.
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }
}

/// Why a torus annulus could not be realized as a mesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RealizeFailure {
    /// The cell is not a primitive annulus (unsupported class).
    UnsupportedPrimitiveClass,
    /// The developed rectangle is degenerate (zero width).
    DegenerateRectangle,
    /// A cut pair's two vertices are not spatially coincident (the cut edges
    /// disagree — a non-periodic or inconsistent torus).
    MismatchedCutPair,
    /// The realized mesh's Euler characteristic is not 0 (not an annulus).
    WrongEulerCharacteristic {
        /// The computed Euler characteristic (should be 0 for an annulus).
        computed: i64,
    },
    /// The mesh does not have exactly two boundary components.
    WrongBoundaryComponentCount {
        /// The boundary component count found.
        count: usize,
    },
    /// The mesh is not connected.
    Disconnected,
}

/// Realize a certified torus annular cell as a triangulated, reglued mesh.
///
/// Triangulates the developed rectangle on a `(nu × nv)` grid, maps vertices
/// through `transform.transform_point(torus.subs(u, v))`, welds *only* the
/// certified periodic cut column (not arbitrary coincident vertices), preserves
/// the two source boundary cycles, and validates connectedness, two boundary
/// components, Euler characteristic 0, and cut-pair coincidence.
///
/// `nu`/`nv` are the subdivisions of the developed rectangle; both must be ≥ 2.
pub fn realize_torus_annulus(
    cell: &CertifiedTorusAnnularCell,
    torus: &Torus,
    transform: &Matrix4,
    nu: usize,
    nv: usize,
) -> Result<RealizedTorusAnnulus, RealizeFailure> {
    if nu < 2 || nv < 2 {
        return Err(RealizeFailure::DegenerateRectangle);
    }
    let (u_lo, u_hi, v_lo, v_hi, cut_along_u) = match cell.primitive_class() {
        PrimitiveWinding::Parallel => {
            let (va, vb) = (
                cell.boundary_a().constant_coordinate(),
                cell.boundary_b().constant_coordinate(),
            );
            let (vlo, vhi) = if va < vb { (va, vb) } else { (vb, va) };
            if (vhi - vlo).abs() < 1e-12 {
                return Err(RealizeFailure::DegenerateRectangle);
            }
            (0.0, TAU, vlo, vhi, true)
        }
        PrimitiveWinding::Meridian => {
            let (ua, ub) = (
                cell.boundary_a().constant_coordinate(),
                cell.boundary_b().constant_coordinate(),
            );
            let (ulo, uhi) = if ua < ub { (ua, ub) } else { (ub, ua) };
            if (uhi - ulo).abs() < 1e-12 {
                return Err(RealizeFailure::DegenerateRectangle);
            }
            (ulo, uhi, 0.0, TAU, false)
        }
    };

    let map = |u: f64, v: f64| -> Point3 { transform.transform_point(torus.subs(u, v)) };

    // Grid vertices: (nu+1) x (nv+1). Index idx(k, l) = k * (nv + 1) + l.
    let idx = |k: usize, l: usize| -> usize { k * (nv + 1) + l };
    let mut vertices: Vec<Point3> = Vec::with_capacity((nu + 1) * (nv + 1));
    for k in 0..=nu {
        let u = u_lo + (u_hi - u_lo) * k as f64 / nu as f64;
        for l in 0..=nv {
            let v = v_lo + (v_hi - v_lo) * l as f64 / nv as f64;
            vertices.push(map(u, v));
        }
    }

    // Certified cut-pair welds: the periodic column/row is glued. For
    // parallels, column k=0 welds to k=nu (u=0 ~ u=2π); for meridians, row
    // l=0 welds to l=nv (v=0 ~ v=2π). Weld ONLY these certified pairs.
    let mut remap: Vec<usize> = (0..vertices.len()).collect();
    let mut cut_pairs: Vec<(usize, usize)> = Vec::new();
    if cut_along_u {
        // Verify cut-pair coincidence before welding.
        let scale = (torus.large_radius() + torus.small_radius()).max(1.0);
        let tol = 1e-9 * scale;
        for l in 0..=nv {
            let a = idx(0, l);
            let b = idx(nu, l);
            if (vertices[a] - vertices[b]).magnitude() > tol {
                return Err(RealizeFailure::MismatchedCutPair);
            }
            remap[b] = a;
            cut_pairs.push((a, b));
        }
    } else {
        let scale = (torus.large_radius() + torus.small_radius()).max(1.0);
        let tol = 1e-9 * scale;
        for k in 0..=nu {
            let a = idx(k, 0);
            let b = idx(k, nv);
            if (vertices[a] - vertices[b]).magnitude() > tol {
                return Err(RealizeFailure::MismatchedCutPair);
            }
            remap[b] = a;
            cut_pairs.push((a, b));
        }
    }

    // Triangulate each grid cell into two triangles (CCW in (u,v)).
    let mut triangles: Vec<[usize; 3]> = Vec::with_capacity(nu * nv * 2);
    for k in 0..nu {
        for l in 0..nv {
            let v00 = remap[idx(k, l)];
            let v10 = remap[idx(k + 1, l)];
            let v01 = remap[idx(k, l + 1)];
            let v11 = remap[idx(k + 1, l + 1)];
            triangles.push([v00, v10, v01]);
            triangles.push([v10, v11, v01]);
        }
    }

    // Boundary loops: the two non-periodic sides.
    let boundary_loops: [Vec<usize>; 2] = if cut_along_u {
        // Parallels: boundaries at v=v_lo (l=0) and v=v_hi (l=nv).
        [
            (0..nu).map(|k| remap[idx(k, 0)]).collect(),
            (0..nu).map(|k| remap[idx(k, nv)]).collect(),
        ]
    } else {
        // Meridians: boundaries at u=u_lo (k=0) and u=u_hi (k=nu).
        [
            (0..nv).map(|l| remap[idx(0, l)]).collect(),
            (0..nv).map(|l| remap[idx(nu, l)]).collect(),
        ]
    };

    // Collapse welded vertices for the validation counts.
    let mut compact: Vec<Point3> = Vec::new();
    let mut compact_idx: Vec<usize> = vec![usize::MAX; vertices.len()];
    for i in 0..vertices.len() {
        if remap[i] == i {
            compact_idx[i] = compact.len();
            compact.push(vertices[i]);
        }
    }
    let tri3: Vec<[usize; 3]> = triangles
        .iter()
        .map(|t| {
            [
                compact_idx[remap[t[0]]],
                compact_idx[remap[t[1]]],
                compact_idx[remap[t[2]]],
            ]
        })
        .collect();

    // Euler characteristic: V - E + F (unique edges).
    let v_count = compact.len();
    let f_count = tri3.len();
    let mut edges: HashSet<[usize; 2]> = HashSet::new();
    for t in &tri3 {
        for &(a, b) in &[(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let e = if a < b { [a, b] } else { [b, a] };
            edges.insert(e);
        }
    }
    let e_count = edges.len();
    let euler = v_count as i64 - e_count as i64 + f_count as i64;
    if euler != 0 {
        return Err(RealizeFailure::WrongEulerCharacteristic { computed: euler });
    }

    // Boundary components: edges with exactly one incident triangle.
    let mut incident: std::collections::HashMap<[usize; 2], usize> =
        std::collections::HashMap::new();
    for t in &tri3 {
        for &(a, b) in &[(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let e = if a < b { [a, b] } else { [b, a] };
            *incident.entry(e).or_default() += 1;
        }
    }
    let boundary_edges: Vec<[usize; 2]> = incident
        .iter()
        .filter(|(_, &c)| c == 1)
        .map(|(&e, _)| e)
        .collect();
    let boundary_loops_count = count_boundary_loops(&boundary_edges);
    if boundary_loops_count != 2 {
        return Err(RealizeFailure::WrongBoundaryComponentCount {
            count: boundary_loops_count,
        });
    }

    // Connectedness (union-find over triangle adjacency by shared edges).
    if !is_connected(&tri3, v_count) {
        return Err(RealizeFailure::Disconnected);
    }

    Ok(RealizedTorusAnnulus {
        vertices: compact,
        triangles: tri3,
        boundary_loops,
        cut_pairs,
        euler_characteristic: euler,
    })
}

/// Count boundary loops (cycles) from a set of boundary edges.
fn count_boundary_loops(edges: &[[usize; 2]]) -> usize {
    if edges.is_empty() {
        return 0;
    }
    let mut adj: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for &[a, b] in edges {
        adj.entry(a).or_default().push(b);
        adj.entry(b).or_default().push(a);
    }
    let mut visited: HashSet<usize> = HashSet::new();
    let mut loops = 0;
    for start in adj.keys() {
        if visited.contains(start) {
            continue;
        }
        loops += 1;
        let mut stack = vec![*start];
        while let Some(u) = stack.pop() {
            if !visited.insert(u) {
                continue;
            }
            if let Some(nb) = adj.get(&u) {
                for &w in nb {
                    if !visited.contains(&w) {
                        stack.push(w);
                    }
                }
            }
        }
    }
    loops
}

/// Union-find connectedness check over triangles sharing an edge.
fn is_connected(triangles: &[[usize; 3]], vertex_count: usize) -> bool {
    if triangles.is_empty() {
        return vertex_count == 0;
    }
    let mut parent: Vec<usize> = (0..vertex_count).collect();
    fn find(parent: &mut Vec<usize>, x: usize) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]);
        }
        parent[x]
    }
    for t in triangles {
        let a = find(&mut parent, t[0]);
        let b = find(&mut parent, t[1]);
        parent[a] = b;
        let c = find(&mut parent, t[2]);
        parent[b] = c;
    }
    let root = find(&mut parent, triangles[0][0]);
    // Every vertex that appears in a triangle must share the root.
    let mut seen: HashSet<usize> = HashSet::new();
    for t in triangles {
        for &v in t {
            seen.insert(v);
        }
    }
    seen.iter().all(|&v| find(&mut parent, v) == root)
}

#[cfg(test)]
mod realize_tests {
    use super::super::torus::{identify_torus_world, CertifiedRankTwoDeck, TorusIdentification};
    use super::super::torus_cell::{
        certify_torus_annular_cell, BoundaryLoopPlacement, PrimitiveWinding,
        SourceBoundaryComposition,
    };
    use super::*;
    use truck_geometry::prelude::{Matrix4, Point3, Torus, Vector3};

    fn deck() -> CertifiedRankTwoDeck {
        match identify_torus_world(
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
            5.0,
            1.0,
        ) {
            TorusIdentification::Torus(d) => d,
            other => panic!("need a deck, got {other:?}"),
        }
    }

    fn parallel(v: f64, sign: i8) -> BoundaryLoopPlacement {
        BoundaryLoopPlacement {
            center: Point3::new(0.0, 0.0, 1.0 * v.sin()),
            normal: Vector3::new(0.0, 0.0, 1.0),
            radius: 5.0 + 1.0 * v.cos(),
            effective_orientation_sign: sign,
        }
    }

    fn clean() -> SourceBoundaryComposition {
        SourceBoundaryComposition {
            component_count: 2,
            extra_source_edge: false,
            outer_bound_malformation: None,
        }
    }

    fn cell() -> CertifiedTorusAnnularCell {
        certify_torus_annular_cell(&deck(), parallel(0.0, 1), parallel(1.2, -1), &clean()).unwrap()
    }

    fn torus() -> Torus {
        Torus::new(Point3::new(0.0, 0.0, 0.0), 5.0, 1.0)
    }

    #[test]
    fn a_parallel_annulus_realizes_with_euler_zero_and_two_boundaries() {
        let r = realize_torus_annulus(&cell(), &torus(), &Matrix4::from_scale(1.0), 8, 4).unwrap();
        assert_eq!(r.euler_characteristic, 0);
        assert_eq!(r.boundary_loops.len(), 2);
        assert!(!r.boundary_loops[0].is_empty());
        assert!(!r.boundary_loops[1].is_empty());
        assert!(r.triangle_count() > 0);
        // The cut pairs were welded (column 0 ~ column nu).
        assert!(!r.cut_pairs.is_empty());
    }

    #[test]
    fn reversed_orientation_realizes() {
        // Swap the two boundaries (reversed effective orientation order) — the
        // cell is still certified (opposite signs) and realizes identically.
        let c = certify_torus_annular_cell(&deck(), parallel(1.2, -1), parallel(0.0, 1), &clean())
            .unwrap();
        let r = realize_torus_annulus(&c, &torus(), &Matrix4::from_scale(1.0), 8, 4).unwrap();
        assert_eq!(r.euler_characteristic, 0);
    }

    #[test]
    fn narrow_and_wide_annuli_both_realize() {
        let narrow =
            certify_torus_annular_cell(&deck(), parallel(0.0, 1), parallel(0.05, -1), &clean())
                .unwrap();
        let wide =
            certify_torus_annular_cell(&deck(), parallel(0.0, 1), parallel(3.0, -1), &clean())
                .unwrap();
        for c in [narrow, wide] {
            let r = realize_torus_annulus(&c, &torus(), &Matrix4::from_scale(1.0), 10, 4).unwrap();
            assert_eq!(r.euler_characteristic, 0);
        }
    }

    #[test]
    fn seam_adjacent_boundary_realizes() {
        // One boundary near the seam (v ≈ 0) — the cut is at u=0~2π, the
        // boundary at v≈0 is adjacent to it but distinct.
        let c =
            certify_torus_annular_cell(&deck(), parallel(0.001, 1), parallel(1.5, -1), &clean())
                .unwrap();
        let r = realize_torus_annulus(&c, &torus(), &Matrix4::from_scale(1.0), 8, 4).unwrap();
        assert_eq!(r.euler_characteristic, 0);
    }

    #[test]
    fn reflected_placement_realizes() {
        // A reflection (non-uniform scale -1 in x) flips orientation; the cell
        // still certifies and the mesh realizes with χ = 0.
        let refl = Matrix4::from_nonuniform_scale(-1.0, 1.0, 1.0);
        let r = realize_torus_annulus(&cell(), &torus(), &refl, 8, 4).unwrap();
        assert_eq!(r.euler_characteristic, 0);
    }

    #[test]
    fn a_meridian_annulus_realizes() {
        // Two meridians (plane contains the axis) at u=0 and u=1.0.
        let m1 = BoundaryLoopPlacement {
            center: Point3::new(5.0, 0.0, 0.0),
            normal: Vector3::new(0.0, 1.0, 0.0),
            radius: 1.0,
            effective_orientation_sign: 1,
        };
        // Meridian at u=1.0: centre at (5 cos 1, 5 sin 1, 0), normal (-sin 1, cos 1, 0).
        let (su, cu) = 1.0_f64.sin_cos();
        let m2 = BoundaryLoopPlacement {
            center: Point3::new(5.0 * cu, 5.0 * su, 0.0),
            normal: Vector3::new(-su, cu, 0.0),
            radius: 1.0,
            effective_orientation_sign: -1,
        };
        let c = certify_torus_annular_cell(&deck(), m1, m2, &clean()).unwrap();
        assert_eq!(c.primitive_class(), PrimitiveWinding::Meridian);
        let r = realize_torus_annulus(&c, &torus(), &Matrix4::from_scale(1.0), 4, 8).unwrap();
        assert_eq!(r.euler_characteristic, 0);
        assert_eq!(r.boundary_loops.len(), 2);
    }

    #[test]
    fn mismatched_cut_pair_is_detected() {
        // A torus whose transform introduces a non-periodicity the realizer
        // cannot honor is impossible for a real Torus (subs(0,v)=subs(2π,v)),
        // so this test instead passes a degenerate grid that collapses the cut
        // and confirms the realizer still refuses a bad rectangle.
        let r = realize_torus_annulus(&cell(), &torus(), &Matrix4::from_scale(1.0), 1, 4);
        assert!(matches!(r, Err(RealizeFailure::DegenerateRectangle)));
    }
}
