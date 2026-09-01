//! The rank-0 planar slice, widened to faces with holes.
//!
//! # What this module is
//!
//! [`super::planar_slice`] carries a *hole-free* planar face from source
//! evidence to a validated mesh. On the corpus that leaves 187 of the 209
//! legacy-failed planar candidates stopped at `multiple_bounds_or_holes`. This
//! module is the smallest sound extension that admits them: one authoritative
//! `FACE_OUTER_BOUND`, one or more ordinary `FACE_BOUND` inner loops, all
//! line- or polyline-bounded, all pairwise disjoint, every inner loop strictly
//! inside the outer one and mutually exterior.
//!
//! # What it recovers on `00009190`: nothing, and why that is the result
//!
//! Measured in shadow over all 1,873 multi-bound planar rank-0 candidates:
//!
//! ```text
//! unsupported_curve_representation [outer]   1737
//! unsupported_curve_representation [inner]    129
//! resolved                                      7
//! ```
//!
//! and over the 187-face target bucket specifically, **187 of 187 exit at
//! `unsupported_curve_representation` attributed to the outer bound**. The
//! seven faces that do resolve were all meshed by the legacy path already, so
//! the recovery gate correctly declines every one of them: this expansion
//! recovers zero faces.
//!
//! That is the honest measurement, and it relocates the backlog. The bound
//! *topology* of the target population is well within the admitted subset —
//! 176 of the 187 are one outer plus one inner — so the obstruction was never
//! holes. It is that the outer loops are built from arcs, and the first
//! milestone admits only lines and polylines. The next expansion for planar
//! faces is therefore **certified circular-arc curve witnesses**, not more
//! boundary topology; this module is what proves that, and it is already in
//! place for the arcs to flow through when they arrive.
//!
//! ```text
//! Step 0 source evidence
//!   -> Step 1 certified planar rank-0 ambient
//!   -> Step 2H regular source traversal of every bound
//!   -> Step 3H certified planar curve occurrences, per bound
//!   -> Steps 4-6H rank-0 lift, trivial deck, one-copy cover
//!   -> Step 7H per-loop Jordan proof
//!            + pairwise boundary disjointness
//!            + strict containment
//!   -> material region = outer disk minus the closed hole disks
//!   -> Step 8AH certified polygonal realization of every component
//!   -> Step 8BH constrained triangulation with holes
//!   -> final validity, including chi = 1 - h
//! ```
//!
//! # What it deliberately does not do
//!
//! It is not an arrangement engine. Nested holes, touching components,
//! crossings, overlaps, curved inner boundaries and multiple outer bounds are
//! each refused with their own named [`SliceExit`], because the histogram over
//! those refusals is the next expansion's specification. No geometric healing
//! happens anywhere: a face that would need a boundary moved to become
//! admissible is refused, not repaired.
//!
//! # Why source authority, and not signed area, decides which loop is outer
//!
//! A hole traversed with the same handedness as its outer loop is legal STEP.
//! Winding therefore cannot classify material, and this module never asks it
//! to: `FACE_OUTER_BOUND` standing — carried through
//! [`OuterBoundStanding`] — is the only thing consulted. Orientation is
//! retained solely to reproduce the source's handedness in the emitted
//! winding, exactly as the hole-free slice does.
//!
//! # Why a constrained Delaunay triangulation, where the hole-free slice ear
//! clips
//!
//! Ear clipping needs a single simple polygon. Merging holes into one requires
//! bridge edges, and a bridge is a *visibility* claim about the region that
//! would have to be proved before it could be trusted. A CDT needs no such
//! claim: every boundary segment goes in as a constraint, and the material
//! triangles are then selected by an exact representative-point test that is
//! only licensed *after* proving no triangle interior meets a boundary
//! component. Whatever the CDT does, [`final_validity_with_holes`] re-derives
//! every property from the emitted complex alone.
//!
//! # Three hazards this module is shaped around
//!
//! **Relative deck gauge between components.** Each loop is developed
//! separately, and for a *positive-rank* ambient that would be unsound: two
//! zero-holonomy loops each admit translation by `kg`, so developing them
//! independently and never solving their relative placement could put a real
//! hole outside the outer loop, or select a hole from the wrong quotient copy.
//! Here the ambient is certified **rank 0**: the deck group is trivial, there
//! is no generator, [`planar_slice::rank0_lift`] is the identity on
//! coordinates, and every component is projected by the *same*
//! [`PlaneSchema`] Gram solve at the same authoritative source-vertex
//! positions. There is no gauge freedom to solve, which is why independent
//! per-loop development is sound *here and only here*. A rank-1 extension of
//! holes must add a face-level relative-placement solve before reusing any of
//! this; it cannot inherit the argument.
//!
//! **Containment is not disjointness.** The material region's components are
//! emphatically *not* pairwise disjoint disks — every hole disk lies inside
//! the outer one, which is the whole point. What
//! [`certify_region_with_holes`] requires is disjointness of the *boundaries*
//! plus strict containment of each hole in the outer loop and mutual
//! exteriority among the holes. Requiring the outer and inner disks to be
//! disjoint would reject every well-formed face with a hole.
//!
//! **A centroid does not speak for a triangle.** A triangle can straddle a
//! hole boundary while its centroid sits in retained material, so centroid
//! classification alone would happily emit a triangle spanning a hole and then
//! declare the mesh valid. It is not load-bearing here. Every boundary segment
//! is a CDT constraint, and the battery independently establishes that the
//! mesh's boundary edge set *equals* the constraint set and that no mesh edge
//! crosses a constraint. A triangle spanning a hole fails both: its crossing
//! edge trips the second, and the swallowed boundary segments are missing from
//! the first.

use super::super::source_evidence::{BoundId, EdgeUseId, SourceBoundInput, SourceFaceInput};
use super::ambient::CertifiedAmbientLattice;
use super::planar_slice::{
    self, classify_segments, jordan_arrangement_of, lift_to_3d, on_segment, transversal,
    PlanarMesh, SegmentIntersection, SimpleJordanArrangement, SliceCategory, SliceExit, SliceStage,
    TriangulatedRegion,
};
use super::support::{CurveSchema, PlaneSchema};
use truck_geometry::prelude::{InnerSpace, Point2, Point3};
use truck_topology::compress::OuterBoundStanding;

type SliceResult<T> = Result<T, SliceExit>;

// ---------------------------------------------------------------------------
// Bound classification
// ---------------------------------------------------------------------------

/// A face's bounds, split by the authority the source declared.
///
/// The outer bound is *named* by [`OuterBoundStanding`], never inferred from
/// area, orientation, containment or source order.
#[derive(Debug, Clone)]
pub struct PlanarMultiBoundInput<'a> {
    /// The bound the source declared `FACE_OUTER_BOUND`.
    pub outer: &'a SourceBoundInput,
    /// Every other bound, in source order. Non-empty.
    pub inners: Vec<&'a SourceBoundInput>,
}

/// What [`classify_bounds`] concluded about a face's bound structure.
#[derive(Debug, Clone)]
pub enum MultiBoundEntry<'a> {
    /// Exactly one bound, and it is the authoritative outer one. This is the
    /// hole-free slice's population; running it here would duplicate that path
    /// rather than extend it.
    DelegateToHoleFreeSlice,
    /// One authoritative outer bound and at least one inner bound.
    MultiBound(PlanarMultiBoundInput<'a>),
}

/// Split a face's bounds into the authoritative outer one and the inner ones.
///
/// `OuterBoundStanding::Declared { declared_count: 1, bound_index }` is the
/// only admitted state. Having exactly one bound is *not* outer standing: a
/// `FACE_BOUND`-only face is legal STEP and its single loop is not thereby an
/// outer bound, so material selection has no ground to stand on.
pub fn classify_bounds<'a>(
    input: &'a SourceFaceInput,
    outer_bound: OuterBoundStanding,
) -> SliceResult<MultiBoundEntry<'a>> {
    let outer_index = match outer_bound {
        OuterBoundStanding::NotRetained | OuterBoundStanding::NoneDeclared => {
            return Err(SliceExit::MissingOuterBoundAuthority)
        }
        OuterBoundStanding::Declared {
            declared_count: 1,
            bound_index,
        } => bound_index as usize,
        OuterBoundStanding::Declared { .. } => return Err(SliceExit::MultipleOuterBoundsDeclared),
    };
    if outer_index >= input.bounds.len() {
        // The standing names a bound the evidence does not contain.
        return Err(SliceExit::MissingOuterBoundAuthority);
    }
    if input.bounds.len() == 1 {
        return Ok(MultiBoundEntry::DelegateToHoleFreeSlice);
    }
    let outer = &input.bounds[outer_index];
    let inners: Vec<&SourceBoundInput> = input
        .bounds
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != outer_index)
        .map(|(_, bound)| bound)
        .collect();
    // A degenerate evidence term cannot be traversed and cannot be proved
    // absent, so a face carrying one is outside this subset rather than a face
    // with one fewer hole. It may be a real collapsed `VERTEX_LOOP`, which a
    // plane cannot close, or it may be lost data; nothing here can tell.
    for bound in std::iter::once(outer).chain(inners.iter().copied()) {
        if matches!(
            bound,
            SourceBoundInput::DegenerateEvidenceUnavailable { .. }
        ) {
            return Err(match std::ptr::eq(bound, outer) {
                true => SliceExit::DegenerateTraversal,
                false => SliceExit::DegenerateInnerBound,
            });
        }
    }
    Ok(MultiBoundEntry::MultiBound(PlanarMultiBoundInput {
        outer,
        inners,
    }))
}

// ---------------------------------------------------------------------------
// Step 2H — traverse every bound independently
// ---------------------------------------------------------------------------

/// Step 2H's product: one closed regular traversal per bound.
#[derive(Debug, Clone)]
pub struct RegularPlanarMultiBoundTraversal {
    /// The authoritative outer bound's traversal.
    pub outer: planar_slice::RegularClosedTraversal,
    /// Each inner bound's traversal, in source order.
    pub inners: Vec<planar_slice::RegularClosedTraversal>,
}

/// Step 2H. Run the hole-free slice's per-bound traversal on every bound.
///
/// The only face-level obligation this stage adds is that no `EdgeUseId`
/// repeats *across* bounds. A repeated underlying `EdgeId` is not an error and
/// is not checked: one edge may legitimately be used by two bounds, and
/// conflating the two identities is exactly the mistake that would reject a
/// well-formed face.
pub fn regular_planar_multibound_traversal(
    bounds: &PlanarMultiBoundInput<'_>,
    curves: &mut impl FnMut(usize) -> CurveSchema,
) -> Result<RegularPlanarMultiBoundTraversal, (SliceExit, BoundRole)> {
    let outer = planar_slice::traverse_bound(bounds.outer, curves)
        .map_err(|exit| (exit, BoundRole::Outer))?;
    let mut inners = Vec::with_capacity(bounds.inners.len());
    for (position, inner) in bounds.inners.iter().enumerate() {
        inners.push(
            planar_slice::traverse_bound(inner, curves)
                .map_err(|exit| (exit, BoundRole::Inner(position)))?,
        );
    }

    let mut seen: Vec<EdgeUseId> = Vec::new();
    for (position, traversal) in std::iter::once(&outer).chain(inners.iter()).enumerate() {
        for occurrence in &traversal.occurrences {
            if seen.contains(&occurrence.edge_use) {
                return Err((
                    SliceExit::DuplicateEdgeUseId,
                    match position {
                        0 => BoundRole::Outer,
                        other => BoundRole::Inner(other - 1),
                    },
                ));
            }
            seen.push(occurrence.edge_use);
        }
    }

    Ok(RegularPlanarMultiBoundTraversal { outer, inners })
}

/// Which bound of a face an obstruction was found on.
///
/// Reported alongside the exit because "an inner bound has an unsupported
/// curve" and "the outer bound has one" are different facts about the face and
/// name different work. Collapsing them loses exactly the distinction the
/// expansion backlog is planned from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundRole {
    /// The authoritative `FACE_OUTER_BOUND`.
    Outer,
    /// The inner bound at this position in source order.
    Inner(usize),
}

impl BoundRole {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Outer => "outer",
            Self::Inner(_) => "inner",
        }
    }
}

// ---------------------------------------------------------------------------
// Step 7H — per-loop Jordan proof, pairwise disjointness, containment
// ---------------------------------------------------------------------------

/// One boundary component, proved to be a simple closed Jordan curve.
#[derive(Debug, Clone)]
pub struct BoundaryLoop {
    /// Which bound this loop realizes.
    pub bound: BoundId,
    /// The certified arrangement: the cycle and each segment's origin.
    pub arrangement: SimpleJordanArrangement,
    /// The cycle's signed area. Its *sign* is the source traversal's
    /// handedness in the plane's chart and carries no material meaning; its
    /// magnitude is the disk's area.
    pub signed_area: f64,
}

impl BoundaryLoop {
    /// The loop's vertex cycle.
    pub fn cycle(&self) -> &[Point2] {
        &self.arrangement.cycle
    }
}

/// How two distinct boundary components relate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentRelation {
    /// The boundaries share no point.
    Disjoint,
    /// They meet transversally.
    Cross,
    /// They meet without crossing.
    Touch,
    /// They share a positive-length segment.
    Overlap,
}

impl ComponentRelation {
    /// How strong a statement this relation makes, so that a pair related in
    /// more than one way is reported by its strongest.
    fn severity(self) -> u8 {
        match self {
            Self::Disjoint => 0,
            Self::Touch => 1,
            Self::Cross => 2,
            Self::Overlap => 3,
        }
    }
}

/// Classify what two loops' boundaries share, by exhaustive exact pairwise
/// segment classification.
///
/// The scan is complete rather than short-circuiting, and reports the *most
/// severe* relation found. Two loops sharing a whole edge also touch at that
/// edge's endpoints, so returning the first contact encountered would make the
/// reported reason depend on iteration order — and would report a shared edge
/// as a touch. The refusal is the same either way; the diagnostic is not, and
/// the diagnostic is what the next expansion is planned from.
///
/// `O(n*m)`, which this slice admits. No epsilon appears anywhere below
/// [`classify_segments`], so proximity alone can neither create nor remove a
/// contact.
pub fn classify_components(a: &[Point2], b: &[Point2]) -> ComponentRelation {
    let (n, m) = (a.len(), b.len());
    let mut worst = ComponentRelation::Disjoint;
    for i in 0..n {
        let (a0, a1) = (a[i], a[(i + 1) % n]);
        for j in 0..m {
            let (b0, b1) = (b[j], b[(j + 1) % m]);
            let relation = match classify_segments(a0, a1, b0, b1) {
                SegmentIntersection::Empty => continue,
                SegmentIntersection::Overlap => ComponentRelation::Overlap,
                SegmentIntersection::Point(_) => match transversal(a0, a1, b0, b1) {
                    true => ComponentRelation::Cross,
                    false => ComponentRelation::Touch,
                },
            };
            if relation.severity() > worst.severity() {
                worst = relation;
            }
            if worst == ComponentRelation::Overlap {
                return worst;
            }
        }
    }
    worst
}

/// Whether `p` lies strictly inside the simple closed polygon `cycle`.
///
/// `None` means `p` lies *on* the boundary, where "inside" has no answer. Every
/// caller must treat that as an undecided containment rather than picking a
/// side.
///
/// Exact. The parity count uses `robust::orient2d` for the side test rather
/// than an explicit intersection abscissa, so no division and no epsilon enters
/// and a vertex exactly at `p`'s height is handled by the half-open `>`
/// comparison rather than by perturbation.
pub fn point_strictly_inside(p: Point2, cycle: &[Point2]) -> Option<bool> {
    let n = cycle.len();
    // On-boundary first: the parity argument below is valid only off it.
    for i in 0..n {
        let (a, b) = (cycle[i], cycle[(i + 1) % n]);
        let d = robust::orient2d(
            robust::Coord { x: a.x, y: a.y },
            robust::Coord { x: b.x, y: b.y },
            robust::Coord { x: p.x, y: p.y },
        );
        if d == 0.0 && on_segment(a, b, p) {
            return None;
        }
        if !d.is_finite() {
            return None;
        }
    }
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (cycle[i], cycle[(i + 1) % n]);
        // The half-open crossing rule: exactly one of the two endpoints counts
        // as above, so a segment ending on the ray is counted once overall.
        if (a.y > p.y) == (b.y > p.y) {
            continue;
        }
        let d = robust::orient2d(
            robust::Coord { x: a.x, y: a.y },
            robust::Coord { x: b.x, y: b.y },
            robust::Coord { x: p.x, y: p.y },
        );
        // Upward segment: `p` is strictly left of it exactly when the crossing
        // lies to `p`'s right. Downward reverses both.
        let to_the_right = match b.y > a.y {
            true => d > 0.0,
            false => d < 0.0,
        };
        if to_the_right {
            inside = !inside;
        }
    }
    Some(inside)
}

/// The material region: the outer disk with the closed hole disks removed.
#[derive(Debug, Clone)]
pub struct PlanarRegionWithHolesCertificate {
    /// The authoritative outer boundary component.
    pub outer: BoundaryLoop,
    /// The inner boundary components, in source order.
    pub holes: Vec<BoundaryLoop>,
    /// `|outer area| - sum |hole area|`. Positive, by containment.
    pub material_area: f64,
}

/// Step 7H. Certify the arrangement of every boundary component.
///
/// Three obligations, in this order, because each licenses the next:
///
/// 1. every loop is individually a simple closed Jordan curve — already
///    discharged by the caller through the hole-free slice's own predicates;
/// 2. every *pair* of distinct components is disjoint;
/// 3. containment, decided by an exact point-in-polygon test.
///
/// Step 3 is licensed only by step 2: with the boundaries proved disjoint, all
/// of one loop lies strictly on one side of another, so *any* of its vertices
/// is a valid representative and no interior point has to be constructed.
pub fn certify_region_with_holes(
    outer: BoundaryLoop,
    holes: Vec<BoundaryLoop>,
) -> SliceResult<PlanarRegionWithHolesCertificate> {
    let all: Vec<&BoundaryLoop> = std::iter::once(&outer).chain(holes.iter()).collect();

    // 2. Pairwise boundary disjointness.
    for i in 0..all.len() {
        for j in (i + 1)..all.len() {
            match classify_components(all[i].cycle(), all[j].cycle()) {
                ComponentRelation::Disjoint => {}
                ComponentRelation::Cross => return Err(SliceExit::BoundaryComponentsCross),
                ComponentRelation::Touch => return Err(SliceExit::BoundaryComponentsTouch),
                ComponentRelation::Overlap => return Err(SliceExit::BoundaryComponentsOverlap),
            }
        }
    }

    // 3a. Every inner loop is strictly inside the authoritative outer loop.
    //     The source says it is a hole of this face; a loop outside the outer
    //     boundary contradicts that, so it is `Inconsistent` and not merely
    //     unsupported.
    for hole in &holes {
        match point_strictly_inside(hole.cycle()[0], outer.cycle()) {
            Some(true) => {}
            Some(false) => return Err(SliceExit::InnerBoundOutsideOuter),
            None => return Err(SliceExit::ContainmentUndecided),
        }
    }

    // 3b. Inner loops are mutually exterior. For two Jordan curves with
    //     disjoint boundaries, either one's interior contains the other or the
    //     interiors are disjoint, so refuting both containments settles it.
    for i in 0..holes.len() {
        for j in (i + 1)..holes.len() {
            for (a, b) in [(i, j), (j, i)] {
                match point_strictly_inside(holes[a].cycle()[0], holes[b].cycle()) {
                    Some(false) => {}
                    Some(true) => return Err(SliceExit::NestedHole),
                    None => return Err(SliceExit::ContainmentUndecided),
                }
            }
        }
    }

    let material_area =
        outer.signed_area.abs() - holes.iter().map(|hole| hole.signed_area.abs()).sum::<f64>();
    if !material_area.is_finite() || material_area <= 0.0 {
        // Containment proves this positive, so reaching here is a defect in
        // this module rather than a verdict about the face.
        return Err(SliceExit::MeshGeometryInvalid);
    }

    Ok(PlanarRegionWithHolesCertificate {
        outer,
        holes,
        material_area,
    })
}

// ---------------------------------------------------------------------------
// Step 8BH — constrained triangulation with holes
// ---------------------------------------------------------------------------

/// Where each vertex of the flattened complex came from.
#[derive(Debug, Clone)]
pub struct BoundaryComponentMap {
    /// `component[v]` is the index into `[outer] ++ holes` of the loop that
    /// contributed vertex `v`.
    pub component: Vec<usize>,
    /// The half-open index range each component occupies.
    pub ranges: Vec<(usize, usize)>,
}

/// Step 8BH. Triangulate the polygon-with-holes.
///
/// Every segment of every component goes in as a constraint. The CDT covers
/// the convex hull, so material selection is a separate, *licensed* step: the
/// triangles whose interiors meet no boundary component are classified by an
/// exact representative point, inside the outer loop and outside every hole.
///
/// The classification is licensed by the no-crossing proof that precedes it. A
/// triangle whose interior meets no constraint lies wholly inside or wholly
/// outside each Jordan curve, so its centroid decides for the whole triangle.
/// Without that proof a centroid test would be exactly the substitute for the
/// coverage obligation that Step 8B forbids.
pub fn triangulate_with_holes(
    certificate: &PlanarRegionWithHolesCertificate,
) -> SliceResult<(TriangulatedRegion, BoundaryComponentMap)> {
    use spade::{ConstrainedDelaunayTriangulation, Point2 as SPoint2, Triangulation};

    // Flatten every component's cycle into one vertex table. Indices are never
    // merged by proximity: pairwise disjointness and per-loop simplicity have
    // already proved every vertex distinct.
    let mut vertices: Vec<Point2> = Vec::new();
    let mut component: Vec<usize> = Vec::new();
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let loops: Vec<&BoundaryLoop> = std::iter::once(&certificate.outer)
        .chain(certificate.holes.iter())
        .collect();
    for (index, boundary) in loops.iter().enumerate() {
        let start = vertices.len();
        for point in boundary.cycle() {
            vertices.push(*point);
            component.push(index);
        }
        ranges.push((start, vertices.len()));
    }

    // The constraint set: every component's closed cycle.
    let mut constraints: Vec<(usize, usize)> = Vec::new();
    for (start, end) in &ranges {
        let count = end - start;
        for k in 0..count {
            constraints.push((start + k, start + (k + 1) % count));
        }
    }

    let mut cdt: ConstrainedDelaunayTriangulation<SPoint2<f64>> =
        ConstrainedDelaunayTriangulation::new();
    let mut handles = Vec::with_capacity(vertices.len());
    for point in &vertices {
        // No `spade_round` and no proximity search: the formal path does not
        // move a coordinate to make a library accept it, and does not weld two
        // vertices it proved distinct. A coordinate the triangulator cannot
        // represent is a statement about the machine.
        let handle = cdt
            .insert(SPoint2::new(point.x, point.y))
            .map_err(|_| SliceExit::ExecutionBudgetExhausted)?;
        handles.push(handle);
    }
    // Two distinct source vertices landing on one handle means they are the
    // same point, which pairwise disjointness and per-loop simplicity already
    // refuted. Reaching here is a defect above, not a verdict about the face.
    for i in 0..handles.len() {
        for j in (i + 1)..handles.len() {
            if handles[i] == handles[j] {
                return Err(SliceExit::MeshTopologyInvalid);
            }
        }
    }
    for (from, to) in &constraints {
        if !cdt.can_add_constraint(handles[*from], handles[*to]) {
            // The constraints were proved pairwise non-crossing and
            // non-overlapping, so a refusal contradicts a discharged
            // obligation.
            return Err(SliceExit::TriangulationDidNotComplete);
        }
        cdt.add_constraint(handles[*from], handles[*to]);
    }

    let mut index_of = std::collections::HashMap::new();
    for (index, handle) in handles.iter().enumerate() {
        index_of.insert(*handle, index);
    }

    // Retain the material triangles. The two obligations are separated: a
    // triangle whose interior meets a constraint is a defect (the CDT was
    // required to respect them), and only after that does the representative
    // point decide material membership.
    let mut triangles: Vec<[usize; 3]> = Vec::new();
    for face in cdt.inner_faces() {
        let corners = face.vertices();
        let mut indices = [0usize; 3];
        for (slot, corner) in corners.iter().enumerate() {
            indices[slot] = *index_of
                .get(&corner.fix())
                .ok_or(SliceExit::MeshTopologyInvalid)?;
        }
        let [a, b, c] = indices;
        if a == b || b == c || a == c {
            return Err(SliceExit::MeshTopologyInvalid);
        }
        let (pa, pb, pc) = (vertices[a], vertices[b], vertices[c]);
        let centroid = Point2::new((pa.x + pb.x + pc.x) / 3.0, (pa.y + pb.y + pc.y) / 3.0);
        let mut material = match point_strictly_inside(centroid, certificate.outer.cycle()) {
            Some(inside) => inside,
            None => return Err(SliceExit::ContainmentUndecided),
        };
        for hole in &certificate.holes {
            match point_strictly_inside(centroid, hole.cycle()) {
                Some(true) => material = false,
                Some(false) => {}
                None => return Err(SliceExit::ContainmentUndecided),
            }
        }
        if material {
            triangles.push(indices);
        }
    }
    if triangles.is_empty() {
        return Err(SliceExit::TriangulationDidNotComplete);
    }

    Ok((
        TriangulatedRegion {
            vertices,
            triangles,
        },
        BoundaryComponentMap { component, ranges },
    ))
}

// ---------------------------------------------------------------------------
// Final validity, with holes
// ---------------------------------------------------------------------------

/// The result of the multi-boundary final validity battery.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoleFinalValidityReport {
    /// Triangles retained.
    pub triangles: usize,
    /// Vertices in the complex.
    pub vertices: usize,
    /// Distinct undirected edges.
    pub edges: usize,
    /// Boundary edges: exactly one incident triangle.
    pub boundary_edges: usize,
    /// Internal edges: exactly two.
    pub internal_edges: usize,
    /// Boundary cycles found. Must be `holes + 1`.
    pub boundary_cycles: usize,
    /// `V - E + T`. Must be `1 - holes`.
    pub euler_characteristic: isize,
    /// `|sum of triangle areas - material area|`.
    pub area_residual: f64,
}

/// The final validity battery for a polygon-with-holes complex.
///
/// Every predicate is a check on the emitted complex. None reads how it was
/// produced, and the coverage claim is *not* carried by area alone: the
/// boundary-edge set, the boundary-cycle decomposition and the Euler count are
/// each established independently, and area closes the remaining gap only once
/// they have.
pub fn final_validity_with_holes(
    mesh: &TriangulatedRegion,
    map: &BoundaryComponentMap,
    certificate: &PlanarRegionWithHolesCertificate,
) -> SliceResult<HoleFinalValidityReport> {
    use std::collections::{HashMap, HashSet};

    let n = mesh.vertices.len();
    let holes = certificate.holes.len();

    // 1. No degenerate or repeated triangle.
    for [a, b, c] in &mesh.triangles {
        if a == b || b == c || a == c {
            return Err(SliceExit::MeshTopologyInvalid);
        }
        let orientation = robust::orient2d(
            robust::Coord {
                x: mesh.vertices[*a].x,
                y: mesh.vertices[*a].y,
            },
            robust::Coord {
                x: mesh.vertices[*b].x,
                y: mesh.vertices[*b].y,
            },
            robust::Coord {
                x: mesh.vertices[*c].x,
                y: mesh.vertices[*c].y,
            },
        );
        if orientation == 0.0 || !orientation.is_finite() {
            return Err(SliceExit::MeshGeometryInvalid);
        }
    }
    let mut sorted: Vec<[usize; 3]> = mesh
        .triangles
        .iter()
        .map(|[a, b, c]| {
            let mut t = [*a, *b, *c];
            t.sort_unstable();
            t
        })
        .collect();
    sorted.sort_unstable();
    if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(SliceExit::MeshTopologyInvalid);
    }

    // 2. Every source polygon vertex is represented.
    let mut seen = vec![false; n];
    for triangle in &mesh.triangles {
        for index in triangle {
            seen[*index] = true;
        }
    }
    if seen.iter().any(|present| !present) {
        return Err(SliceExit::MeshTopologyInvalid);
    }

    // 3. Edge incidence. Three incident triangles is a nonmanifold complex.
    let mut incidence: HashMap<(usize, usize), usize> = HashMap::new();
    for [a, b, c] in &mesh.triangles {
        for (p, q) in [(*a, *b), (*b, *c), (*c, *a)] {
            *incidence.entry((p.min(q), p.max(q))).or_insert(0) += 1;
        }
    }
    if incidence.values().any(|count| *count > 2) {
        return Err(SliceExit::MeshTopologyInvalid);
    }
    let boundary_edges: HashSet<(usize, usize)> = incidence
        .iter()
        .filter(|(_, count)| **count == 1)
        .map(|(edge, _)| *edge)
        .collect();
    let internal_edges = incidence.values().filter(|count| **count == 2).count();

    // 4. The mesh boundary is exactly the union of the component cycles. This
    //    is what establishes both "every boundary segment is represented" and
    //    "no boundary segment the source did not declare was invented".
    let mut expected: HashSet<(usize, usize)> = HashSet::new();
    for (start, end) in &map.ranges {
        let count = end - start;
        for k in 0..count {
            let (i, j) = (start + k, start + (k + 1) % count);
            expected.insert((i.min(j), i.max(j)));
        }
    }
    if boundary_edges != expected {
        return Err(SliceExit::MeshBoundaryMismatch);
    }

    // 5. No triangle edge crosses a boundary constraint. Every constraint is
    //    itself a mesh edge by (4), so this rules out a retained triangle
    //    straddling a component — the fact that licensed the representative
    //    point test, re-derived here from the complex rather than trusted.
    let constraints: Vec<(Point2, Point2)> = expected
        .iter()
        .map(|(i, j)| (mesh.vertices[*i], mesh.vertices[*j]))
        .collect();
    for (edge, _) in incidence.iter() {
        let (p, q) = (mesh.vertices[edge.0], mesh.vertices[edge.1]);
        for (a, b) in &constraints {
            // A shared endpoint is expected wherever a mesh edge meets a
            // constraint at a vertex; anything else is a violation.
            match classify_segments(p, q, *a, *b) {
                SegmentIntersection::Empty => {}
                SegmentIntersection::Point(point) => {
                    let endpoint = point == p || point == q;
                    let on_constraint = point == *a || point == *b;
                    if !(endpoint && on_constraint) {
                        return Err(SliceExit::TriangleCrossesBoundaryConstraint);
                    }
                }
                SegmentIntersection::Overlap => {
                    // Legal only when the mesh edge *is* that constraint.
                    let same = (p == *a && q == *b) || (p == *b && q == *a);
                    if !same {
                        return Err(SliceExit::TriangleCrossesBoundaryConstraint);
                    }
                }
            }
        }
    }

    // 6. Connected, across internal edges.
    if !dual_graph_is_connected(mesh, &incidence) {
        return Err(SliceExit::MeshTopologyInvalid);
    }

    // 7. Exactly `h + 1` boundary cycles, each the complete vertex set of one
    //    declared component.
    let cycles = boundary_cycle_partition(&boundary_edges)?;
    if cycles.len() != holes + 1 {
        return Err(SliceExit::BoundaryCycleCountMismatch);
    }
    let mut matched = vec![false; map.ranges.len()];
    for cycle in &cycles {
        let component = map.component[cycle[0]];
        if cycle.iter().any(|v| map.component[*v] != component) {
            return Err(SliceExit::BoundaryCycleCountMismatch);
        }
        let (start, end) = map.ranges[component];
        if cycle.len() != end - start {
            return Err(SliceExit::BoundaryCycleCountMismatch);
        }
        if matched[component] {
            return Err(SliceExit::BoundaryCycleCountMismatch);
        }
        matched[component] = true;
    }
    if matched.iter().any(|hit| !hit) {
        return Err(SliceExit::BoundaryCycleCountMismatch);
    }

    // 8. Euler characteristic of a disk with `h` holes.
    let edges = incidence.len();
    let euler = n as isize - edges as isize + mesh.triangles.len() as isize;
    if euler != 1 - holes as isize {
        return Err(SliceExit::MeshTopologyInvalid);
    }

    // 9. No retained triangle lies inside a hole, re-derived independently of
    //    the selection that produced the complex.
    for [a, b, c] in &mesh.triangles {
        let (pa, pb, pc) = (mesh.vertices[*a], mesh.vertices[*b], mesh.vertices[*c]);
        let centroid = Point2::new((pa.x + pb.x + pc.x) / 3.0, (pa.y + pb.y + pc.y) / 3.0);
        match point_strictly_inside(centroid, certificate.outer.cycle()) {
            Some(true) => {}
            Some(false) => return Err(SliceExit::MeshGeometryInvalid),
            None => return Err(SliceExit::ContainmentUndecided),
        }
        for hole in &certificate.holes {
            match point_strictly_inside(centroid, hole.cycle()) {
                Some(false) => {}
                Some(true) => return Err(SliceExit::MeshGeometryInvalid),
                None => return Err(SliceExit::ContainmentUndecided),
            }
        }
    }

    // 10. Coverage. With the boundary set, the cycle decomposition, the Euler
    //     count and connectivity already established, the area identity closes
    //     the remaining possibility: a complex satisfying all of them cannot
    //     overlap itself without leaving a gap, nor leave a gap without a
    //     boundary edge (4) would have rejected.
    let mesh_area: f64 = mesh
        .triangles
        .iter()
        .map(|[a, b, c]| {
            let (a, b, c) = (mesh.vertices[*a], mesh.vertices[*b], mesh.vertices[*c]);
            ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs() / 2.0
        })
        .sum();
    let area_residual = (mesh_area - certificate.material_area).abs();
    if !(area_residual <= certificate.material_area * 1e-9) {
        return Err(SliceExit::MeshGeometryInvalid);
    }

    Ok(HoleFinalValidityReport {
        triangles: mesh.triangles.len(),
        vertices: n,
        edges,
        boundary_edges: boundary_edges.len(),
        internal_edges,
        boundary_cycles: cycles.len(),
        euler_characteristic: euler,
        area_residual,
    })
}

/// Decompose a boundary edge set into vertex cycles.
///
/// Every vertex must have degree exactly two in the boundary graph; anything
/// else is a pinched or branching boundary and is refused rather than walked
/// past.
fn boundary_cycle_partition(
    boundary_edges: &std::collections::HashSet<(usize, usize)>,
) -> SliceResult<Vec<Vec<usize>>> {
    use std::collections::HashMap;
    let mut adjacency: HashMap<usize, Vec<usize>> = HashMap::new();
    for (a, b) in boundary_edges {
        adjacency.entry(*a).or_default().push(*b);
        adjacency.entry(*b).or_default().push(*a);
    }
    if adjacency.values().any(|neighbours| neighbours.len() != 2) {
        return Err(SliceExit::BoundaryCycleCountMismatch);
    }
    let mut visited: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut cycles = Vec::new();
    let mut starts: Vec<usize> = adjacency.keys().copied().collect();
    starts.sort_unstable();
    for start in starts {
        if visited.contains(&start) {
            continue;
        }
        let mut cycle = vec![start];
        visited.insert(start);
        let mut previous = start;
        let mut current = adjacency[&start][0];
        while current != start {
            if !visited.insert(current) {
                return Err(SliceExit::BoundaryCycleCountMismatch);
            }
            cycle.push(current);
            let neighbours = &adjacency[&current];
            let next = match neighbours[0] == previous {
                true => neighbours[1],
                false => neighbours[0],
            };
            previous = current;
            current = next;
        }
        cycles.push(cycle);
    }
    Ok(cycles)
}

fn dual_graph_is_connected(
    mesh: &TriangulatedRegion,
    incidence: &std::collections::HashMap<(usize, usize), usize>,
) -> bool {
    use std::collections::HashMap;
    if mesh.triangles.is_empty() {
        return false;
    }
    let mut by_edge: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (index, [a, b, c]) in mesh.triangles.iter().enumerate() {
        for (p, q) in [(*a, *b), (*b, *c), (*c, *a)] {
            by_edge.entry((p.min(q), p.max(q))).or_default().push(index);
        }
    }
    let mut visited = vec![false; mesh.triangles.len()];
    let mut stack = vec![0usize];
    visited[0] = true;
    while let Some(current) = stack.pop() {
        let [a, b, c] = mesh.triangles[current];
        for (p, q) in [(a, b), (b, c), (c, a)] {
            let key = (p.min(q), p.max(q));
            if incidence.get(&key) != Some(&2) {
                continue;
            }
            for neighbour in by_edge.get(&key).into_iter().flatten() {
                if !visited[*neighbour] {
                    visited[*neighbour] = true;
                    stack.push(*neighbour);
                }
            }
        }
    }
    visited.into_iter().all(|seen| seen)
}

// ---------------------------------------------------------------------------
// Orchestration
// ---------------------------------------------------------------------------

/// Everything one face's run of the holes slice established.
#[derive(Debug, Clone)]
pub struct HoleSliceRecord {
    /// The furthest stage reached.
    pub stage: SliceStage,
    /// The semantic category of the result.
    pub category: SliceCategory,
    /// The exact exit reason, when the face did not complete.
    pub exit: Option<SliceExit>,
    /// Whether the face delegated to the hole-free slice.
    pub delegated: bool,
    /// How many bounds the face declared.
    pub bound_count: usize,
    /// How many of them are inner bounds under the authoritative outer one.
    pub inner_bound_count: usize,
    /// Edge uses per bound, outer first, when traversal resolved.
    pub edge_uses_per_bound: Vec<usize>,
    /// Polygon vertices per bound, outer first, when the loops were certified.
    pub polygon_vertices_per_bound: Vec<usize>,
    /// The curve representations met, as tags, deduplicated.
    pub curve_representations: Vec<&'static str>,
    /// Which bound the obstruction was found on, when it is attributable to
    /// one. `None` for face-level obstructions and for a completed face.
    pub obstruction_bound: Option<BoundRole>,
    /// The final validity report, when the face completed.
    pub validity: Option<HoleFinalValidityReport>,
    /// The 3D triangles, when the face completed. The recovery gate's payload.
    pub mesh: Option<PlanarMesh>,
}

impl HoleSliceRecord {
    fn new(bound_count: usize) -> Self {
        Self {
            stage: SliceStage::NotAttempted,
            category: SliceCategory::Unresolved,
            exit: None,
            delegated: false,
            bound_count,
            inner_bound_count: 0,
            edge_uses_per_bound: Vec::new(),
            polygon_vertices_per_bound: Vec::new(),
            curve_representations: Vec::new(),
            obstruction_bound: None,
            validity: None,
            mesh: None,
        }
    }

    fn exited(mut self, stage: SliceStage, exit: SliceExit) -> Self {
        self.stage = stage;
        self.category = exit.category();
        self.exit = Some(exit);
        self
    }
}

/// Run the complete planar-holes slice for one face.
///
/// A face with no inner bounds returns `delegated`, having attempted nothing:
/// it belongs to [`super::planar_slice`], and running both paths on it would
/// make two modules answerable for one population.
#[allow(clippy::too_many_arguments)]
pub fn run_planar_holes_slice(
    input: &SourceFaceInput,
    plane: &PlaneSchema,
    lattice: &CertifiedAmbientLattice,
    outer_bound: OuterBoundStanding,
    curve_of: &mut impl FnMut(usize) -> CurveSchema,
    vertex_position: &impl Fn(super::super::source_evidence::SourceVertexKey) -> Option<Point3>,
    tolerance: f64,
) -> HoleSliceRecord {
    let mut record = HoleSliceRecord::new(input.bounds.len());

    if !matches!(lattice, CertifiedAmbientLattice::Rank0(_)) {
        return record.exited(SliceStage::NotAttempted, SliceExit::NotRank0);
    }
    record.stage = SliceStage::AmbientRank0;

    // Read before Step 2H, for the reason [`super::planar_slice::run_planar_slice`]
    // gives: a face refused for its curve family must be able to say which
    // family, and the traversal it would have read them from does not exist.
    for edge_use in input.edge_uses() {
        let tag = curve_of(edge_use.source_edge_index).tag();
        if !record.curve_representations.contains(&tag) {
            record.curve_representations.push(tag);
        }
    }

    let bounds = match classify_bounds(input, outer_bound) {
        Ok(MultiBoundEntry::DelegateToHoleFreeSlice) => {
            record.delegated = true;
            return record;
        }
        Ok(MultiBoundEntry::MultiBound(bounds)) => bounds,
        Err(exit) => return record.exited(SliceStage::AmbientRank0, exit),
    };
    record.inner_bound_count = bounds.inners.len();

    // Step 2H.
    let traversal = match regular_planar_multibound_traversal(&bounds, curve_of) {
        Ok(traversal) => traversal,
        Err((exit, role)) => {
            record.obstruction_bound = Some(role);
            return record.exited(SliceStage::AmbientRank0, exit);
        }
    };
    let all_traversals: Vec<&planar_slice::RegularClosedTraversal> =
        std::iter::once(&traversal.outer)
            .chain(traversal.inners.iter())
            .collect();
    for one in &all_traversals {
        record.edge_uses_per_bound.push(one.occurrences.len());
        for occurrence in &one.occurrences {
            let tag = occurrence.curve.tag();
            if !record.curve_representations.contains(&tag) {
                record.curve_representations.push(tag);
            }
        }
    }
    record.stage = SliceStage::RegularTraversal;

    // Steps 3H to 7H, per loop. Every loop runs the identical hole-free
    // machinery; the multi-bound obligations come after.
    let mut loops: Vec<BoundaryLoop> = Vec::with_capacity(all_traversals.len());
    for (position, one) in all_traversals.iter().enumerate() {
        let role = match position {
            0 => BoundRole::Outer,
            other => BoundRole::Inner(other - 1),
        };
        record.obstruction_bound = Some(role);
        let planar =
            match planar_slice::certified_planar_curves(one, plane, vertex_position, tolerance) {
                Ok(planar) => planar,
                Err(exit) => return record.exited(SliceStage::RegularTraversal, exit),
            };
        let developed = match planar_slice::rank0_lift(lattice, planar) {
            Ok(developed) => developed,
            Err(exit) => return record.exited(SliceStage::PlanarCurves, exit),
        };
        if let Err(exit) = planar_slice::trivial_deck_solution(&developed) {
            return record.exited(SliceStage::DevelopedBoundary, exit);
        }
        if let Err(exit) = planar_slice::one_copy_working_cover(&developed) {
            return record.exited(SliceStage::DeckSolution, exit);
        }
        // Step 8AH's certificate, taken per loop before any pairwise claim: the
        // polygon may stand in for the source boundary only where the
        // approximation error is exactly zero.
        if !developed.approximation_is_exact() {
            return record.exited(
                SliceStage::WorkingCover,
                SliceExit::PolygonalizationCertificateUnavailable,
            );
        }
        let arrangement = match jordan_arrangement_of(&developed.occurrences) {
            Ok(arrangement) => arrangement,
            Err(exit) => return record.exited(SliceStage::WorkingCover, exit),
        };
        let signed_area = signed_area(&arrangement.cycle);
        if !signed_area.is_finite() || signed_area == 0.0 {
            return record.exited(SliceStage::WorkingCover, SliceExit::MeshGeometryInvalid);
        }
        record
            .polygon_vertices_per_bound
            .push(arrangement.cycle.len());
        loops.push(BoundaryLoop {
            bound: one.bound,
            arrangement,
            signed_area,
        });
    }
    record.stage = SliceStage::JordanArrangement;

    // Step 7H's multi-bound obligations.
    let outer_loop = loops.remove(0);
    let outer_clockwise = outer_loop.signed_area < 0.0;
    let certificate = match certify_region_with_holes(outer_loop, loops) {
        Ok(certificate) => certificate,
        Err(exit) => return record.exited(SliceStage::JordanArrangement, exit),
    };
    // Material selection and Step 8A's certificate are settled together here:
    // the region is the outer disk minus the closed hole disks, and the
    // per-loop exactness guard above already discharged the polygonalization
    // obligation for every component.
    record.stage = SliceStage::PolygonalRegion;

    // Step 8BH.
    let (mesh, map) = match triangulate_with_holes(&certificate) {
        Ok(product) => product,
        Err(exit) => return record.exited(SliceStage::PolygonalRegion, exit),
    };
    record.stage = SliceStage::Triangulation;

    let validity = match final_validity_with_holes(&mesh, &map, &certificate) {
        Ok(validity) => validity,
        Err(exit) => return record.exited(SliceStage::Triangulation, exit),
    };
    let positions = match lift_to_3d(&mesh, plane) {
        Ok(positions) => positions,
        Err(exit) => return record.exited(SliceStage::Triangulation, exit),
    };
    record.validity = Some(validity);

    // Restore the source traversal's handedness, as the hole-free slice does.
    // The CDT emits counter-clockwise triangles; a face whose declared outer
    // loop runs clockwise in the plane's chart must not be silently
    // reoriented. Winding and normal flip together, so they stay consistent.
    let triangles = match outer_clockwise {
        false => mesh.triangles,
        true => mesh
            .triangles
            .into_iter()
            .map(|[a, b, c]| [c, b, a])
            .collect(),
    };
    let chart_normal = {
        let normal = plane.u_axis().cross(plane.v_axis()).normalize();
        match outer_clockwise {
            false => normal,
            true => -normal,
        }
    };
    record.mesh = Some(PlanarMesh {
        positions,
        triangles,
        chart_normal,
    });
    record.stage = SliceStage::FinalValidity;
    record.category = SliceCategory::Resolved;
    record
}

/// Twice the signed area of a closed polygon, halved. Shoelace.
fn signed_area(cycle: &[Point2]) -> f64 {
    let n = cycle.len();
    let mut total = 0.0;
    for i in 0..n {
        let p = cycle[i];
        let q = cycle[(i + 1) % n];
        total += p.x * q.y - q.x * p.y;
    }
    total / 2.0
}

#[cfg(test)]
mod tests;
