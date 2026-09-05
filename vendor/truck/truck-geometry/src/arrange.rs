//! BG-SOL-S1-ARRANGE — the 2-D planar arrangement over analytic profiles.
//!
//! Turns a closed analytic profile (`Curve::Line`/`Curve::Circle` in the
//! plane) into a certified 2-D subdivision: vertices, half-edges and regions
//! with winding numbers. The critical path to M1 (certified planar
//! construction, docs/SOLVER_FAMILY_PLAN.md §7): rectangle − circle →
//! arrangement → profile with hole → direct extrude.
//!
//! Builds on the LANDED Phase-0 API: `truck_base::pred::orient2d` (the exact
//! crossing/winding predicate), `truck_base::contact::CurveContact` (the event
//! vocabulary S1 and the Contact Layer share), `truck_base::bounding_box::`
//! `BoundingBox<Point2>` (the domain box), and `truck_geometry::recognize` (to
//! read the analytic carriers off `Curve`).
//!
//! Target API (plan §4 Phase 1):
//!
//! ```rust,ignore
//! pub struct Arrangement {
//!     pub vertices: Vec<ArrVertex>,
//!     pub half_edges: Vec<ArrHalfEdge>,
//!     pub regions: Vec<ArrRegion>,
//! }
//! pub fn arrange(profile: &[Curve], domain: Option<BoundingBox<Point2>>) -> Outcome<Arrangement>;
//! ```
//!
//! v1 scope (documented in the packet): analytic Line/Circle profiles only;
//! exactly-representable (dyadic) vertices; the algebraic intersection-vertex
//! case is a documented refusal. House rules H-1..H-8 apply.
//!
//! The half-edge `next`/`prev` wiring is the standard "turn left at the
//! vertex" traversal: for a half-edge arriving at a vertex, `next` is the
//! outgoing half-edge immediately CLOCKWISE of the twin (equivalently, the
//! first exit counter-clockwise of the direction of arrival). Face tracing
//! with this rule yields each face cycle; a half-edge whose destination is a
//! degree-1 vertex terminates an OPEN boundary walk (`next == NO_NEXT`). A
//! closed loop appears in the tracing twice (its interior face cycle and its
//! exterior face cycle, which uses the twin half-edges); the region stage
//! merges the two into one geometric cycle (the CCW representative).

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::prelude::*;
use crate::recognize::{
    recognize_curve, CanonicalCarrier, CanonicalCarrierWitness, CanonicalCurve,
};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::f64::consts::{PI, TAU};
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, EnvelopeCase, Margin, Method, Modulus,
    Outcome, Prop, PropMap, Refusal, Truth, UnresolvedWitness,
};
use truck_base::pred::{orient2d, CertifiedPred, Orientation};

/// A vertex of the arrangement.
#[derive(Clone, Debug, PartialEq)]
pub struct ArrVertex {
    /// The vertex's 3-D position (z = 0 for the planar profile).
    pub point: Point3,
    /// Indices into `Arrangement::half_edges` of the edges originating here.
    pub incident: Vec<usize>,
}

/// A directed edge of the arrangement (a half-edge).
#[derive(Clone, Debug, PartialEq)]
pub struct ArrHalfEdge {
    /// The origin vertex (index into `vertices`).
    pub origin: usize,
    /// The twin half-edge (index into `half_edges`).
    pub twin: usize,
    /// The next half-edge around this edge's face, CCW.
    pub next: usize,
    /// The previous half-edge around this edge's face.
    pub prev: usize,
    /// Index into the input `profile` slice this edge lies on.
    pub curve: usize,
    /// Parameter window on that curve (in the curve's own parameter).
    pub u_range: (f64, f64),
}

/// A face of the planar subdivision.
#[derive(Clone, Debug, PartialEq)]
pub struct ArrRegion {
    /// The region's boundary half-edge cycles, in order. A region with a
    /// hole has MORE THAN ONE cycle: the first is the outer boundary (CCW),
    /// the rest are the holes (CW). M1's plate is the canonical case:
    /// `boundaries = [[outer rectangle cycle], [inner circle cycle]]`.
    /// A region's total boundary is the union of its cycles.
    pub boundaries: Vec<Vec<usize>>,
    /// The winding number of the region around any interior point.
    pub winding: i32,
    /// Whether the region is bounded (M1: the plate and the hole are
    /// bounded; the exterior is not).
    pub bounded: bool,
}

/// The planar subdivision of a closed analytic profile.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Arrangement {
    /// The vertices of the subdivision, deduplicated by exact position.
    pub vertices: Vec<ArrVertex>,
    /// The directed half-edges of the subdivision (each curve segment twice).
    pub half_edges: Vec<ArrHalfEdge>,
    /// The regions of the subdivision with their boundary cycles and winding.
    pub regions: Vec<ArrRegion>,
}

/// Sentinel `next`/`prev` index: the half-edge terminates (or begins) an open
/// boundary walk at a degree-1 vertex. `usize::MAX` is not a valid index.
const NO_NEXT: usize = usize::MAX;

/// The number of samples used to polygonize a full circle arc for the
/// point-in-loop / winding predicates.
const POLY_SAMPLES: usize = 16;

/// The packet's outcome result, spelled out to avoid `truck_geometry::errors`
/// `Result<T>` (whose error is `Error`) shadowing the standard two-parameter
/// form.
type S1Result<T> = std::result::Result<T, Refusal>;

/// Builds the arrangement of a closed analytic profile. The profile's loops
/// must be closed (each curve's end meets the next start within the
/// representation tolerance) and pairwise disjoint in the M1 contract;
/// interior crossings are supported by the machinery and reported as split
/// vertices (tests below prove it), but a self-intersecting single loop is
/// refused.
pub fn arrange(profile: &[Curve], domain: Option<BoundingBox<Point2>>) -> Outcome<Arrangement> {
    // Stage 1 — recognition, the z = 0 plane, and the loop structure.
    let mut carriers = Vec::with_capacity(profile.len());
    for c in profile {
        carriers.push(recognize(c)?);
    }
    let chains = build_chains(&carriers, profile.len());
    let mut chain_of = vec![0usize; profile.len()];
    for (ci, chain) in chains.iter().enumerate() {
        for &c in chain {
            if let Some(slot) = chain_of.get_mut(c) {
                *slot = ci;
            }
        }
    }
    // A multi-curve chain that fails to close is a broken loop: a
    // contradiction between the declared boundary and the geometry.
    for chain in &chains {
        if chain.len() < 2 {
            continue;
        }
        let first = match chain.first() {
            Some(&c) => c,
            None => continue,
        };
        let last = match chain.last() {
            Some(&c) => c,
            None => continue,
        };
        let start = match carriers.get(first) {
            Some(c) => c.subs(c.range().0),
            None => continue,
        };
        let end = match carriers.get(last) {
            Some(c) => c.subs(c.range().1),
            None => continue,
        };
        if (start - end).magnitude() > 64.0 * TOLERANCE {
            return Err(contradiction());
        }
    }

    // Stage 2 — pairwise intersections. Same-chain interior crossings are a
    // self-intersecting single loop, refused. The split parameters and points
    // are exact (dyadic) or an honest refusal.
    let mut splits: Vec<Vec<(f64, Point3)>> = vec![Vec::new(); profile.len()];
    for i in 0..profile.len() {
        for j in (i + 1)..profile.len() {
            let ci = match carriers.get(i) {
                Some(c) => c,
                None => continue,
            };
            let cj = match carriers.get(j) {
                Some(c) => c,
                None => continue,
            };
            let contacts = intersect(ci, cj)?;
            let same_chain = chain_of.get(i) == chain_of.get(j);
            for (ti, tj, pt) in contacts {
                let int_i = interior_param(ci, ti);
                let int_j = interior_param(cj, tj);
                if same_chain && int_i && int_j {
                    return Err(contradiction());
                }
                if int_i {
                    if let Some(s) = splits.get_mut(i) {
                        s.push((ti, pt));
                    }
                }
                if int_j {
                    if let Some(s) = splits.get_mut(j) {
                        s.push((tj, pt));
                    }
                }
            }
        }
    }

    // Stage 3 — vertex and edge construction. Vertices are deduplicated by
    // exact `Point3` equality (the vertices are exactly representable).
    let mut builder = Builder::new();
    for (i, carrier) in carriers.iter().enumerate() {
        let (t0, t1) = carrier.range();
        let start_point = pt3(carrier.subs(t0));
        // A full circle's end parameter maps back to the seam vertex (its
        // start); `subs(TAU)` is not exactly `subs(0)` in floats.
        let end_point = if carrier.is_full_circle() {
            start_point
        } else {
            pt3(carrier.subs(t1))
        };
        let mut entries = vec![(t0, start_point)];
        if let Some(s) = splits.get(i) {
            entries.extend(s.iter().copied());
        }
        entries.push((t1, end_point));
        entries.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
        let mut dedup: Vec<(f64, Point3)> = Vec::new();
        for e in entries {
            let dup = match dedup.last() {
                Some(last) => last.0 == e.0,
                None => false,
            };
            if !dup {
                dedup.push(e);
            }
        }
        for k in 0..dedup.len() {
            let (u0, p0) = match dedup.get(k) {
                Some(&x) => x,
                None => continue,
            };
            let (u1, p1) = match dedup.get(k + 1) {
                Some(&x) => x,
                None => break,
            };
            let v0 = builder.vertex_index(p0);
            let v1 = builder.vertex_index(p1);
            builder.add_half_edge(v0, v1, i, (u0, u1));
        }
    }

    // Stage 4 — the DCEL `next`/`prev` wiring via the turn-left traversal.
    let tangents: Vec<Vector2> = builder
        .half_edges
        .iter()
        .map(|he| half_edge_tangent(he, &carriers))
        .collect();
    for vertex in &mut builder.vertices {
        vertex.incident.sort_by(|&a, &b| {
            let da = tangents.get(a).copied().unwrap_or(Vector2::zero());
            let db = tangents.get(b).copied().unwrap_or(Vector2::zero());
            if angle_less(da, db) {
                Ordering::Less
            } else if angle_less(db, da) {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });
    }
    let (next_arr, prev_arr) = wire_next_prev(&builder.vertices, &builder.half_edges);
    for e in 0..builder.half_edges.len() {
        if let Some(he) = builder.half_edges.get_mut(e) {
            he.next = next_arr.get(e).copied().unwrap_or(NO_NEXT);
            he.prev = prev_arr.get(e).copied().unwrap_or(NO_NEXT);
        }
    }

    // Stage 5 — face tracing, cycle merging, region grouping and winding.
    let half_edges = builder.half_edges.clone();
    let vertices = builder.vertices.clone();
    let (closed, open) = trace_faces(&vertices, &half_edges);
    let merged = merge_duplicate_cycles(&closed, &half_edges, &carriers);
    let (children, roots) = nest_cycles(&merged, &half_edges, &carriers);
    let mut regions: Vec<ArrRegion> = Vec::new();
    let mut reps: Vec<Point2> = Vec::new();

    for idx in 0..merged.len() {
        let cyc = match merged.get(idx) {
            Some(c) => c,
            None => continue,
        };
        let outer_poly = cycle_polygon(cyc, &half_edges, &carriers);
        let mut child_polys = Vec::new();
        if let Some(ch) = children.get(idx) {
            for &c in ch {
                if let Some(ccycle) = merged.get(c) {
                    child_polys.push(cycle_polygon(ccycle, &half_edges, &carriers));
                }
            }
        }
        let rep = match representative_inside_outside(&outer_poly, &child_polys) {
            Some(p) => p,
            None => return Err(Refusal::Empty),
        };
        let mut boundaries = vec![cyc.clone()];
        if let Some(ch) = children.get(idx) {
            for &c in ch {
                if let Some(ccycle) = merged.get(c) {
                    boundaries.push(ccycle.clone());
                }
            }
        }
        let mut winding = 0i32;
        for boundary in &boundaries {
            let poly = cycle_polygon(boundary, &half_edges, &carriers);
            match polygon_winding(rep, &poly) {
                Some(w) => winding += w,
                None => return Err(numerically_unresolved()),
            }
        }
        regions.push(ArrRegion {
            boundaries,
            winding,
            bounded: true,
        });
        reps.push(rep);
    }

    if !merged.is_empty() {
        let all_polys: Vec<Vec<Point2>> = merged
            .iter()
            .map(|c| cycle_polygon(c, &half_edges, &carriers))
            .collect();
        let rep = match exterior_point(&all_polys) {
            Some(p) => p,
            None => return Err(Refusal::Empty),
        };
        let mut boundaries: Vec<Vec<usize>> = Vec::new();
        for &r in &roots {
            if let Some(cyc) = merged.get(r) {
                boundaries.push(cyc.clone());
            }
        }
        boundaries.extend(open.iter().cloned());
        let mut winding = 0i32;
        for &r in &roots {
            if let Some(cyc) = merged.get(r) {
                let poly = cycle_polygon(cyc, &half_edges, &carriers);
                match polygon_winding(rep, &poly) {
                    Some(w) => winding += w,
                    None => return Err(numerically_unresolved()),
                }
            }
        }
        regions.push(ArrRegion {
            boundaries,
            winding,
            bounded: false,
        });
        reps.push(rep);
    }

    if merged.is_empty() {
        for walk in &open {
            let rep = open_walk_rep(walk, &half_edges, &carriers);
            regions.push(ArrRegion {
                boundaries: vec![walk.clone()],
                winding: 0,
                bounded: false,
            });
            reps.push(rep.unwrap_or(Point2::new(0.0, 0.0)));
        }
    }

    // Stage 6 — the domain. A region wholly outside the domain is not
    // reported; `None` keeps the single winding-0 unbounded exterior.
    let mut kept_regions = Vec::new();
    for (idx, region) in regions.into_iter().enumerate() {
        let keep = match domain {
            Some(box_) => reps.get(idx).map(|&p| box_.contains(p)).unwrap_or(true),
            None => true,
        };
        if keep {
            kept_regions.push(region);
        }
    }

    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    let arrangement = Arrangement {
        vertices,
        half_edges,
        regions: kept_regions,
    };
    Ok(Certified::new(
        arrangement,
        Certificate {
            props,
            method: Method::Exact,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

// ---------------------------------------------------------------------------
// CC-024-OFFSET-EXACT — sharp (mitered) and concave-edge completion via the
// arrangement engine.
//
// Theory §3.4's sharp/concave strata are ARRANGEMENT OUTPUTS — never new
// `OffsetStratum` variants (the certified enum is read-only context here).
// Both completion rules are the extend-and-intersect rule evaluated on the
// two adjacent offset face carriers of a source edge, seen in the plane
// section of an extruded shell (this engine's domain). A CONVEX source wedge
// (dihedral half-angle θ < π/2) whose offset diverges the two adjacent offset
// faces is completed by extending both faces to their exact carrier
// intersection — the mitered stratum, whose reach ρ_A = |t|/sin θ is pinned
// by [`mitered_edge_reach`] and is NEVER the ball stratum's |t| shortcut
// (a convex corner of half-angle θ puts the completion at |t|/sin θ from its
// source). A CONCAVE (reflex) source wedge whose offset overlaps the two
// adjacent offset faces is completed by the concave rule: the cells the
// overlapping faces cover twice are marked and discarded, and each face is
// trimmed back to the crossing. Plane-face sections are the v1 canonical set
// here (they give an exact line); curved-face sections route through the
// landed certified pair machinery and refuse in this engine, as does the
// mirror regime of each rule. Everything below is ADDITIVE — the `arrange`
// entry above is untouched, so every fixture it already answers answers
// identically (the V5 identity gate).
// ---------------------------------------------------------------------------

/// Pins the theory §3.4 reach bound of a sharp (mitered) stratum as code:
/// `ρ_A = |t| / sin θ` for a source wedge of dihedral half-angle `θ` and
/// offset magnitude `|t|`. For `θ < π/2` the bound is STRICTLY greater than
/// `|t|` — the sharp completion point is not within `|t|` of its source, so a
/// mitered stratum must never carry the ball-stratum `|t|` shortcut. A
/// degenerate half-angle (`sin θ ≤ 0`: `θ ≤ 0`, `θ = π`, or non-finite input)
/// has no finite sharp completion; the bound recedes to infinity.
pub fn mitered_edge_reach(dihedral_half_angle: f64, t: f64) -> f64 {
    if !dihedral_half_angle.is_finite() || !t.is_finite() {
        return f64::INFINITY;
    }
    let s = dihedral_half_angle.sin();
    if s <= 0.0 {
        return f64::INFINITY;
    }
    t.abs() / s
}

/// One sharp (mitered) stratum of the arrangement engine: the completion of a
/// convex source edge whose adjacent offset faces diverge. `miter_point` is
/// the extend-and-intersect vertex of the two offset face carriers;
/// `reach` is the COMPUTED bound `|t| / sin θ` (never `|t|`), and for
/// `θ < π/2` the miter point is strictly farther from `source` than `|t|`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MiteredStratum {
    /// The convex source vertex the stratum completes (z = 0).
    pub source: Point3,
    /// The extend-and-intersect vertex of the two offset face carriers.
    pub miter_point: Point3,
    /// The unit direction from `source` to `miter_point`.
    pub direction: Vector2,
    /// The computed reach bound `ρ_A = |t| / sin θ`.
    pub reach: f64,
    /// The dihedral half-angle `θ` of the source wedge (half the CCW interior
    /// wedge swept from the first edge's end tangent to the second's start).
    pub half_angle: f64,
    /// The signed offset magnitude applied to each source edge section.
    pub offset: f64,
}

/// One boundary segment of the concave-trim output, tagged with its source
/// face section: `source == 0` names the first edge (`a`), `source == 1` the
/// second (`b`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OffsetSegment {
    /// Which adjacent source edge the segment lies on (0 = `a`, 1 = `b`).
    pub source: usize,
    /// One endpoint of the segment.
    pub from: Point3,
    /// The other endpoint of the segment.
    pub to: Point3,
}

/// The concave-edge completion of the arrangement engine (theory §3.4): two
/// adjacent offset faces of a reflex source edge overlap; the cell(s) the
/// overlapping adjacent offset face covers are marked and DISCARDED, and each
/// face is trimmed back to the crossing. Output: the surviving cells (the
/// offset-face boundary segments that stay on the completed boundary) plus
/// the trim curves (the discarded covered sub-segments), each with source
/// provenance through the [`OffsetSegment::source`] tag.
#[derive(Clone, Debug, PartialEq)]
pub struct ConcaveTrim {
    /// The reflex source vertex being completed (z = 0).
    pub vertex: Point3,
    /// The crossing of the two adjacent offset face carriers: the trim point.
    pub crossing: Point3,
    /// The computed reach `|t| / sin θ` of the crossing from `vertex`.
    pub reach: f64,
    /// The dihedral half-angle `θ` of the reflex wedge (half the CCW interior
    /// wedge, so `θ ∈ (π/2, π)`).
    pub half_angle: f64,
    /// The signed offset magnitude applied to each source edge section.
    pub offset: f64,
    /// The number of cells covered by the overlapping adjacent offset faces
    /// that were discarded (1 for a simple reflex completion).
    pub covered_cells: usize,
    /// The surviving offset-face boundary segments: one per source edge, each
    /// running from the crossing to that face's far end.
    pub surviving: Vec<OffsetSegment>,
    /// The discarded trim curves: one per source edge, each running from that
    /// face's vertex-near end to the crossing.
    pub trims: Vec<OffsetSegment>,
}

/// The completion rule's certificate. The completion evaluates analytic
/// carriers in floating point (only the `arrange` subdivision itself is
/// dyadic-exact), so the method is `Float` with an unbounded margin.
fn completion_certificate() -> Certificate {
    Certificate {
        props: PropMap::new(),
        method: Method::Float,
        budget_left: Budget::new(0, 0, 0),
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}

/// The vanishing-crossing guard of the extend-and-intersect rule. The
/// carriers are unit direction vectors, so the crossing denominator is the
/// sine of the carrier angle.
const MIN_CROSS: f64 = 1.0e-9; // H-3: dimensionless sine floor, not a length

/// Reads the planar Line section endpoints off a recognized carrier. A curved
/// (Circle) section refuses `NonCanonicalCarrier`: its miter/trim routes
/// through the landed certified pair machinery, out of this engine's v1
/// plane-face envelope.
fn line_section(c: Carrier2D) -> S1Result<(Point2, Point2)> {
    match c {
        Carrier2D::Line(Line(a, b)) => {
            if !a.x.is_finite() || !a.y.is_finite() || !b.x.is_finite() || !b.y.is_finite() {
                return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
            }
            Ok((a, b))
        }
        Carrier2D::Circle(_) => Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        )),
    }
}

/// The signed CCW wedge from `ta` to `tb` in `(0, TAU)` — the interior angle
/// of the source wedge at a vertex where edge `a` (end tangent `ta`) hands off
/// to edge `b` (start tangent `tb`) of a CCW profile.
fn ccw_wedge(ta: Vector2, tb: Vector2) -> S1Result<f64> {
    let la = ta.magnitude();
    let lb = tb.magnitude();
    if la == 0.0 || lb == 0.0 || !la.is_finite() || !lb.is_finite() {
        return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
    }
    let cross = ta.x * tb.y - ta.y * tb.x;
    let dot = ta.x * tb.x + ta.y * tb.y;
    let mut w = f64::atan2(cross, dot);
    if w < 0.0 {
        w += TAU;
    }
    Ok(w)
}

/// One offset face section: the parallel translate of a source edge section
/// by `offset` along the section's unit right (outward for a CCW profile)
/// normal. `from` is the offset of the source start; `to` of its end.
struct OffsetFace {
    /// The offset of the source edge's start.
    from: Point2,
    /// The offset of the source edge's end.
    to: Point2,
    /// The unit source-edge direction.
    unit: Vector2,
}

/// Translates a Line section by `offset` along its right normal.
fn offset_face(seg: (Point2, Point2), offset: f64) -> S1Result<OffsetFace> {
    let (a, b) = seg;
    let d = b - a;
    let m = d.magnitude();
    if m == 0.0 || !m.is_finite() {
        return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
    }
    let u = Vector2::new(d.x / m, d.y / m);
    let n = Vector2::new(u.y, -u.x);
    let shift = n * offset;
    Ok(OffsetFace {
        from: a + shift,
        to: b + shift,
        unit: u,
    })
}

/// Whether `p` lies strictly inside the span of the offset face section.
fn inside_span(p: Point2, face: &OffsetFace) -> bool {
    let v = face.to - face.from;
    let w = p - face.from;
    let l2 = v.x * v.x + v.y * v.y;
    if l2 == 0.0 {
        return false;
    }
    let s = (w.x * v.x + w.y * v.y) / l2;
    s > 0.0 && s < 1.0
}

/// The crossing of two offset face carriers, refused when they are
/// (near-)parallel.
fn faces_cross(a: &OffsetFace, b: &OffsetFace) -> S1Result<Point2> {
    let denom = a.unit.x * b.unit.y - a.unit.y * b.unit.x;
    if denom.abs() <= MIN_CROSS {
        return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
    }
    let r = b.from - a.from;
    let num = r.x * b.unit.y - r.y * b.unit.x;
    let s = num / denom;
    Ok(a.from + a.unit * s)
}

/// The mirror-regime refusal: the requested corner/side combination belongs
/// to the OTHER completion rule (or to a later envelope), never a silent
/// approximation here.
fn regime_refusal() -> Refusal {
    Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)
}

/// The sharp (mitered) completion of a CONVEX source edge: extends the two
/// adjacent offset face carriers and intersects them. The two source curves
/// must be the consecutive plane-face sections around the convex vertex
/// (first ends where the second starts), and the offset must drive the two
/// offset faces apart (diverge) — an overlapping convex offset is the trim
/// regime's domain and refuses here. Curved (Circle) face sections refuse:
/// a curved-face miter routes through the landed certified pair machinery.
pub fn mitered_stratum(a: &Curve, b: &Curve, offset: f64) -> Outcome<MiteredStratum> {
    let stratum = mitered_stratum_inner(a, b, offset)?;
    Ok(Certified::new(stratum, completion_certificate()))
}

fn mitered_stratum_inner(a: &Curve, b: &Curve, offset: f64) -> S1Result<MiteredStratum> {
    if !offset.is_finite() || offset == 0.0 {
        return Err(Refusal::Empty);
    }
    let seg_a = line_section(recognize(a)?)?;
    let seg_b = line_section(recognize(b)?)?;
    let a_end = seg_a.1;
    let b_start = seg_b.0;
    if (a_end - b_start).magnitude() > 64.0 * TOLERANCE {
        // The two sections are not consecutive around a shared vertex.
        return Err(Refusal::Empty);
    }
    let vertex = a_end;
    let ta = seg_a.1 - seg_a.0;
    let tb = seg_b.1 - seg_b.0;
    let interior = ccw_wedge(ta, tb)?;
    if interior == 0.0 || interior == PI {
        // A degenerate (straight) wedge has no corner at all.
        return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
    }
    if interior > PI {
        // A reflex wedge is the concave rule's domain.
        return Err(regime_refusal());
    }
    let half_angle = 0.5 * interior;
    let face_a = offset_face(seg_a, offset)?;
    let face_b = offset_face(seg_b, offset)?;
    let miter = faces_cross(&face_a, &face_b)?;
    if inside_span(miter, &face_a) || inside_span(miter, &face_b) {
        // The offset drives the adjacent faces ACROSS each other: the
        // completion is a covered-cell trim, not a sharp miter.
        return Err(regime_refusal());
    }
    let delta = miter - vertex;
    let len = delta.magnitude();
    if len == 0.0 || !len.is_finite() {
        return Err(Refusal::Empty);
    }
    Ok(MiteredStratum {
        source: pt3(vertex),
        miter_point: pt3(miter),
        direction: Vector2::new(delta.x / len, delta.y / len),
        reach: mitered_edge_reach(half_angle, offset),
        half_angle,
        offset,
    })
}

/// The concave-edge completion of a REFLEX source edge: marks the cell
/// covered by the two OVERLAPPING adjacent offset faces and discards it,
/// trimming each face back to the crossing. The two source curves must be the
/// consecutive plane-face sections around the reflex vertex (first ends where
/// the second starts) and the offset must overlap the two offset faces (their
/// carriers cross strictly inside both natural spans) — a diverging reflex
/// offset has no covered cell in this engine's v1 envelope and refuses.
pub fn concave_trim(a: &Curve, b: &Curve, offset: f64) -> Outcome<ConcaveTrim> {
    let trim = concave_trim_inner(a, b, offset)?;
    Ok(Certified::new(trim, completion_certificate()))
}

fn concave_trim_inner(a: &Curve, b: &Curve, offset: f64) -> S1Result<ConcaveTrim> {
    if !offset.is_finite() || offset == 0.0 {
        return Err(Refusal::Empty);
    }
    let seg_a = line_section(recognize(a)?)?;
    let seg_b = line_section(recognize(b)?)?;
    let a_end = seg_a.1;
    let b_start = seg_b.0;
    if (a_end - b_start).magnitude() > 64.0 * TOLERANCE {
        // The two sections are not consecutive around a shared vertex.
        return Err(Refusal::Empty);
    }
    let vertex = a_end;
    let ta = seg_a.1 - seg_a.0;
    let tb = seg_b.1 - seg_b.0;
    let interior = ccw_wedge(ta, tb)?;
    if interior == 0.0 || interior == PI {
        // A degenerate (straight) wedge has no corner at all.
        return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
    }
    if interior <= PI {
        // A convex wedge is the miter rule's domain.
        return Err(regime_refusal());
    }
    let half_angle = 0.5 * interior;
    let face_a = offset_face(seg_a, offset)?;
    let face_b = offset_face(seg_b, offset)?;
    let crossing = faces_cross(&face_a, &face_b)?;
    if !inside_span(crossing, &face_a) || !inside_span(crossing, &face_b) {
        // The offset drives the adjacent faces apart: no cell is covered and
        // no trim is produced at this reflex edge in the v1 envelope.
        return Err(regime_refusal());
    }
    // `face_a` runs from its far (start) end to its vertex end; `face_b` runs
    // from its vertex end to its far (end) end. The surviving boundary is the
    // far end up to the crossing on each face; the discarded covered cell is
    // the sub-segment between each face's vertex-near end and the crossing.
    let mut surviving = Vec::new();
    let mut trims = Vec::new();
    surviving.push(OffsetSegment {
        source: 0,
        from: pt3(crossing),
        to: pt3(face_a.from),
    });
    trims.push(OffsetSegment {
        source: 0,
        from: pt3(face_a.to),
        to: pt3(crossing),
    });
    surviving.push(OffsetSegment {
        source: 1,
        from: pt3(crossing),
        to: pt3(face_b.to),
    });
    trims.push(OffsetSegment {
        source: 1,
        from: pt3(face_b.from),
        to: pt3(crossing),
    });
    Ok(ConcaveTrim {
        vertex: pt3(vertex),
        crossing: pt3(crossing),
        reach: mitered_edge_reach(half_angle, offset),
        half_angle,
        offset,
        covered_cells: 1,
        surviving,
        trims,
    })
}

/// An exact dyadic rational `num * 2^exp` — the substrate for the certified
/// intersection vertices. Arithmetic is checked; overflow is an honest refusal.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Dyad {
    num: i128,
    exp: i32,
}

impl Dyad {
    /// The exact dyadic representation of `v`; `None` for non-finite input.
    fn from_f64(v: f64) -> Option<Dyad> {
        if v == 0.0 {
            return Some(Dyad { num: 0, exp: 0 });
        }
        if !v.is_finite() {
            return None;
        }
        let bits = v.to_bits();
        let sign = if bits >> 63 == 1 { -1i128 } else { 1i128 };
        let exp_bits = ((bits >> 52) & 0x7FF) as i32;
        let frac = bits & 0xF_FFFF_FFFF_FFFF;
        if exp_bits == 0 {
            Some(Dyad {
                num: sign * frac as i128,
                exp: -1074,
            })
        } else {
            Some(Dyad {
                num: sign * ((1i128 << 52) | frac as i128),
                exp: exp_bits - 1023 - 52,
            })
        }
    }

    fn is_zero(&self) -> bool {
        self.num == 0
    }

    /// Normalizes the mantissa (strips trailing powers of two, adjusting the
    /// exponent), keeping intermediate products inside i128 for the v1
    /// coordinate magnitudes.
    fn normalized(self) -> Dyad {
        if self.num == 0 {
            return Dyad { num: 0, exp: 0 };
        }
        let tz = self.num.trailing_zeros() as i32;
        Dyad {
            num: self.num >> tz,
            exp: self.exp + tz,
        }
    }

    fn add(&self, other: &Dyad) -> Option<Dyad> {
        if self.is_zero() {
            return Some(*other);
        }
        if other.is_zero() {
            return Some(*self);
        }
        let e = self.exp.min(other.exp);
        let a = self.num.checked_shl((self.exp - e) as u32)?;
        let b = other.num.checked_shl((other.exp - e) as u32)?;
        Some(
            Dyad {
                num: a.checked_add(b)?,
                exp: e,
            }
            .normalized(),
        )
    }

    fn sub(&self, other: &Dyad) -> Option<Dyad> {
        if other.is_zero() {
            return Some(*self);
        }
        if self.is_zero() {
            return Some(Dyad {
                num: -other.num,
                exp: other.exp,
            });
        }
        let e = self.exp.min(other.exp);
        let a = self.num.checked_shl((self.exp - e) as u32)?;
        let b = other.num.checked_shl((other.exp - e) as u32)?;
        Some(
            Dyad {
                num: a.checked_sub(b)?,
                exp: e,
            }
            .normalized(),
        )
    }

    fn mul(&self, other: &Dyad) -> Option<Dyad> {
        Some(
            Dyad {
                num: self.num.checked_mul(other.num)?,
                exp: self.exp + other.exp,
            }
            .normalized(),
        )
    }

    /// The exact square root when it is a dyadic rational; `None` when the
    /// radicand is not a perfect square (an algebraic vertex, refused).
    fn sqrt_exact(&self) -> Option<Dyad> {
        if self.num < 0 {
            return None;
        }
        if self.is_zero() {
            return Some(Dyad { num: 0, exp: 0 });
        }
        let tz = self.num.trailing_zeros() as i32;
        let odd = (self.num >> tz) as u128;
        let k = isqrt_u128(odd)?;
        let e = self.exp + tz;
        if e % 2 != 0 {
            return None;
        }
        Some(Dyad {
            num: k as i128,
            exp: e / 2,
        })
    }

    /// The exact `f64` value when exactly representable; `None` otherwise.
    fn to_f64_exact(self) -> Option<f64> {
        if self.is_zero() {
            return Some(0.0);
        }
        let negative = self.num < 0;
        let mag = self.num.unsigned_abs();
        let tz = mag.trailing_zeros() as i32;
        let m = mag >> tz;
        let e = self.exp + tz;
        if m > (1u128 << 53) {
            return None;
        }
        let bit_len = 128 - m.leading_zeros() as i32;
        let exp = (bit_len - 1) + e;
        if !(-1074..=1023).contains(&exp) {
            return None;
        }
        let sign = if negative { 1u64 << 63 } else { 0 };
        if exp >= -1022 {
            let ef = (exp + 1023) as u64;
            let fr = ((m - (1u128 << (bit_len - 1))) as u64) << (52 - (bit_len - 1));
            Some(f64::from_bits(sign | (ef << 52) | fr))
        } else {
            let ee = e + 1074;
            if ee < 0 {
                return None;
            }
            let fr = m << ee;
            if fr >= (1u128 << 52) {
                return None;
            }
            Some(f64::from_bits(sign | fr as u64))
        }
    }
}

/// A 2-D point carried in exact dyadic arithmetic.
#[derive(Clone, Copy)]
struct D2 {
    x: Dyad,
    y: Dyad,
}

impl D2 {
    fn from_point2(p: Point2) -> Option<D2> {
        Some(D2 {
            x: Dyad::from_f64(p.x)?,
            y: Dyad::from_f64(p.y)?,
        })
    }

    fn sub(&self, other: &D2) -> Option<D2> {
        Some(D2 {
            x: self.x.sub(&other.x)?,
            y: self.y.sub(&other.y)?,
        })
    }

    fn dot(&self, other: &D2) -> Option<Dyad> {
        self.x.mul(&other.x)?.add(&self.y.mul(&other.y)?)
    }

    fn cross(&self, other: &D2) -> Option<Dyad> {
        self.x.mul(&other.y)?.sub(&self.y.mul(&other.x)?)
    }
}

/// The placed planar circle: position, radius, trimmed angle range and the
/// in-plane basis columns of the placement (`e_u`/`e_v` from the transform's
/// x/y columns), with the reversed-parameter (inverted processor) flag.
#[derive(Clone, Copy, Debug)]
struct CircleCarrier {
    center: Point2,
    radius: f64,
    t0: f64,
    t1: f64,
    e_u: Vector2,
    e_v: Vector2,
    reversed: bool,
}

impl CircleCarrier {
    fn subs(&self, t: f64) -> Point2 {
        let phi = if self.reversed {
            self.t0 + self.t1 - t
        } else {
            t
        };
        self.center + self.e_u * phi.cos() + self.e_v * phi.sin()
    }

    fn tangent(&self, t: f64) -> Vector2 {
        let phi = if self.reversed {
            self.t0 + self.t1 - t
        } else {
            t
        };
        let d = -self.e_u * phi.sin() + self.e_v * phi.cos();
        if self.reversed {
            -d
        } else {
            d
        }
    }

    /// The parameter of `p` on the circle, in the curve's own parameter (the
    /// trimmed angle range, honoring a reversed placement).
    fn param_of_point(&self, p: Point2) -> f64 {
        let v = p - self.center;
        let eu2 = self.e_u.dot(self.e_u);
        let ev2 = self.e_v.dot(self.e_v);
        let cos_t = if eu2 > 0.0 {
            v.dot(self.e_u) / eu2
        } else {
            0.0
        };
        let sin_t = if ev2 > 0.0 {
            v.dot(self.e_v) / ev2
        } else {
            0.0
        };
        let mut ang = f64::atan2(sin_t, cos_t);
        if ang < 0.0 {
            ang += TAU;
        }
        let ang = if self.reversed {
            self.t0 + self.t1 - ang
        } else {
            ang
        };
        let mut out = ang;
        while out < self.t0 {
            out += TAU;
        }
        while out > self.t1 {
            out -= TAU;
        }
        out
    }
}

/// The recognized 2-D analytic carrier of a profile curve.
#[derive(Clone, Copy, Debug)]
enum Carrier2D {
    Line(Line<Point2>),
    Circle(CircleCarrier),
}

impl Carrier2D {
    fn range(&self) -> (f64, f64) {
        match self {
            Carrier2D::Line(_) => (0.0, 1.0),
            Carrier2D::Circle(c) => (c.t0, c.t1),
        }
    }

    fn subs(&self, t: f64) -> Point2 {
        match self {
            Carrier2D::Line(Line(a, b)) => *a + (*b - *a) * t,
            Carrier2D::Circle(c) => c.subs(t),
        }
    }

    fn tangent(&self, t: f64) -> Vector2 {
        match self {
            Carrier2D::Line(Line(a, b)) => *b - *a,
            Carrier2D::Circle(c) => c.tangent(t),
        }
    }

    fn is_full_circle(&self) -> bool {
        match self {
            Carrier2D::Circle(c) => c.t1 - c.t0 == TAU,
            Carrier2D::Line(_) => false,
        }
    }
}

/// Recognizes a profile curve as a planar analytic carrier, refusing anything
/// outside the Line/Circle envelope or off the plane z = 0.
fn recognize(c: &Curve) -> S1Result<Carrier2D> {
    match recognize_curve(c) {
        CanonicalCarrierWitness::ExactCanonical {
            carrier: CanonicalCarrier::Curve(CanonicalCurve::Line(Line(a, b))),
            map: _,
        } => {
            if a.z.abs() > 64.0 * TOLERANCE || b.z.abs() > 64.0 * TOLERANCE {
                return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
            }
            let l = Line(Point2::new(a.x, a.y), Point2::new(b.x, b.y));
            if l.0 == l.1 {
                return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
            }
            Ok(Carrier2D::Line(l))
        }
        CanonicalCarrierWitness::ExactCanonical {
            carrier: CanonicalCarrier::Curve(CanonicalCurve::Circle(p)),
            map: _,
        } => {
            let Matrix4 { x, y, z: _, w } = *p.transform();
            let center3 = w.to_point();
            let radius = x.magnitude();
            let (t0, t1) = p.range_tuple();
            let z_dev = w.z.abs() + (x.z * x.z + y.z * y.z).sqrt();
            if !radius.is_finite()
                || radius <= 0.0
                || z_dev > 64.0 * TOLERANCE
                || !t0.is_finite()
                || !t1.is_finite()
                || t1 - t0 <= 0.0
            {
                return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
            }
            Ok(Carrier2D::Circle(CircleCarrier {
                center: Point2::new(center3.x, center3.y),
                radius,
                t0,
                t1,
                e_u: Vector2::new(x.x, x.y),
                e_v: Vector2::new(y.x, y.y),
                reversed: !p.orientation(),
            }))
        }
        _ => Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        )),
    }
}

/// Partitions the profile into maximal chains: consecutive curves whose end
/// meets the next start within tolerance share a chain.
fn build_chains(carriers: &[Carrier2D], len: usize) -> Vec<Vec<usize>> {
    let mut chains: Vec<Vec<usize>> = Vec::new();
    for i in 0..len {
        let prev_end = match chains
            .last()
            .and_then(|chain| chain.last())
            .and_then(|&prev| carriers.get(prev))
        {
            Some(c) => c.subs(c.range().1),
            None => {
                chains.push(vec![i]);
                continue;
            }
        };
        let cur_start = match carriers.get(i) {
            Some(c) => c.subs(c.range().0),
            None => {
                chains.push(vec![i]);
                continue;
            }
        };
        if (prev_end - cur_start).magnitude() <= 64.0 * TOLERANCE {
            if let Some(chain) = chains.last_mut() {
                chain.push(i);
            }
        } else {
            chains.push(vec![i]);
        }
    }
    chains
}

/// The direction a half-edge leaves its origin, from the carrier tangent at
/// the start parameter (negated for a reversed parameter window).
fn half_edge_tangent(he: &ArrHalfEdge, carriers: &[Carrier2D]) -> Vector2 {
    let t0 = match carriers.get(he.curve) {
        Some(c) => c.tangent(he.u_range.0),
        None => Vector2::zero(),
    };
    if he.u_range.1 >= he.u_range.0 {
        t0
    } else {
        -t0
    }
}

/// Whether the angle of `a` precedes the angle of `b` in CCW order from the
/// positive x-axis, decided by half-plane then `orient2d` (no `atan2`).
fn angle_less(a: Vector2, b: Vector2) -> bool {
    let upper = |v: Vector2| v.y > 0.0 || (v.y == 0.0 && v.x >= 0.0);
    let (ua, ub) = (upper(a), upper(b));
    if ua != ub {
        return ua;
    }
    match orient2d(
        Point2::new(0.0, 0.0),
        Point2::new(a.x, a.y),
        Point2::new(b.x, b.y),
    ) {
        CertifiedPred::Proven(Orientation::CounterClockwise) => true,
        CertifiedPred::Proven(Orientation::Clockwise) => false,
        _ => false,
    }
}

/// Wires `next`/`prev`: `next(e)` is the outgoing half-edge at `dest(e)`
/// immediately CLOCKWISE of `twin(e)` (the turn-left traversal); a degree-1
/// destination terminates the walk (`NO_NEXT`).
fn wire_next_prev(vertices: &[ArrVertex], half_edges: &[ArrHalfEdge]) -> (Vec<usize>, Vec<usize>) {
    let mut next_arr = vec![NO_NEXT; half_edges.len()];
    for e in 0..half_edges.len() {
        let he = match half_edges.get(e) {
            Some(he) => he,
            None => continue,
        };
        // The destination vertex of `e` is the origin of its twin.
        let vertex = match half_edges.get(he.twin) {
            Some(tw) => match vertices.get(tw.origin) {
                Some(v) => v,
                None => continue,
            },
            None => continue,
        };
        let len = vertex.incident.len();
        if len <= 1 {
            continue;
        }
        let pos = match vertex.incident.iter().position(|&h| h == he.twin) {
            Some(pos) => pos,
            None => continue,
        };
        if let Some(&n) = vertex.incident.get((pos + len - 1) % len) {
            if let Some(slot) = next_arr.get_mut(e) {
                *slot = n;
            }
        }
    }
    let mut prev_arr = vec![NO_NEXT; half_edges.len()];
    for e in 0..half_edges.len() {
        let n = match next_arr.get(e) {
            Some(&n) if n != NO_NEXT => n,
            _ => continue,
        };
        if let Some(slot) = prev_arr.get_mut(n) {
            *slot = e;
        }
    }
    (next_arr, prev_arr)
}

/// The vertex/edge builder. Vertices are deduplicated by exact `Point3`
/// equality through a bit-encoded key.
struct Builder {
    vertices: Vec<ArrVertex>,
    half_edges: Vec<ArrHalfEdge>,
    vmap: HashMap<(u64, u64, u64), usize>,
}

impl Builder {
    fn new() -> Self {
        Builder {
            vertices: Vec::new(),
            half_edges: Vec::new(),
            vmap: HashMap::new(),
        }
    }

    fn vertex_index(&mut self, p: Point3) -> usize {
        let key = point_key(p);
        if let Some(&idx) = self.vmap.get(&key) {
            return idx;
        }
        let idx = self.vertices.len();
        self.vmap.insert(key, idx);
        self.vertices.push(ArrVertex {
            point: p,
            incident: Vec::new(),
        });
        idx
    }

    fn add_half_edge(&mut self, v0: usize, v1: usize, curve: usize, range: (f64, f64)) {
        let he_idx = self.half_edges.len();
        self.half_edges.push(ArrHalfEdge {
            origin: v0,
            twin: he_idx + 1,
            next: NO_NEXT,
            prev: NO_NEXT,
            curve,
            u_range: (range.0, range.1),
        });
        self.half_edges.push(ArrHalfEdge {
            origin: v1,
            twin: he_idx,
            next: NO_NEXT,
            prev: NO_NEXT,
            curve,
            u_range: (range.1, range.0),
        });
        if let Some(v) = self.vertices.get_mut(v0) {
            v.incident.push(he_idx);
        }
        if let Some(v) = self.vertices.get_mut(v1) {
            v.incident.push(he_idx + 1);
        }
    }
}

/// The exact bit key of a point; `+0.0` and `-0.0` collapse to one key.
fn point_key(p: Point3) -> (u64, u64, u64) {
    (f64_bits(p.x), f64_bits(p.y), f64_bits(p.z))
}

fn f64_bits(x: f64) -> u64 {
    if x == 0.0 {
        0
    } else {
        x.to_bits()
    }
}

/// The intersection contacts of two carriers: `(param on a, param on b, point)`.
/// Exact where the vertices are dyadic; `Err` otherwise.
fn intersect(a: &Carrier2D, b: &Carrier2D) -> S1Result<Vec<(f64, f64, Point3)>> {
    match (a, b) {
        (Carrier2D::Line(l1), Carrier2D::Line(l2)) => line_line_intersection(*l1, *l2),
        (Carrier2D::Line(l), Carrier2D::Circle(c)) => line_circle_intersection(*l, c),
        (Carrier2D::Circle(c), Carrier2D::Line(l)) => {
            let contacts = line_circle_intersection(*l, c)?;
            Ok(contacts.into_iter().map(|(t, u, p)| (u, t, p)).collect())
        }
        (Carrier2D::Circle(c1), Carrier2D::Circle(c2)) => circle_circle_intersection(c1, c2),
    }
}

/// Whether `t` is strictly interior to the curve's parameter range.
fn interior_param(c: &Carrier2D, t: f64) -> bool {
    match c {
        Carrier2D::Line(_) => t > 0.0 && t < 1.0,
        Carrier2D::Circle(c) => t > c.t0 && t < c.t1,
    }
}

/// Line/Line: the crossing decision from `orient2d` (the four endpoint
/// configurations), the parameters and point from Cramer's rule in scaled
/// integer arithmetic. Collinear interval overlap is `Err(Refusal::Empty)`.
fn line_line_intersection(l1: Line<Point2>, l2: Line<Point2>) -> S1Result<Vec<(f64, f64, Point3)>> {
    let da = d2_result(D2::from_point2(l1.0))?;
    let db = d2_result(D2::from_point2(l1.1))?;
    let dc = d2_result(D2::from_point2(l2.0))?;
    let dd = d2_result(D2::from_point2(l2.1))?;
    let r = d2_result(db.sub(&da))?;
    let s = d2_result(dd.sub(&dc))?;
    let q = d2_result(dc.sub(&da))?;
    let denom = dyad_result(r.cross(&s))?;

    let o1 = sign_of(orient2d(l1.0, l1.1, l2.0))?;
    let o2 = sign_of(orient2d(l1.0, l1.1, l2.1))?;
    let o3 = sign_of(orient2d(l2.0, l2.1, l1.0))?;
    let o4 = sign_of(orient2d(l2.0, l2.1, l1.1))?;
    let proper = o1 != 0 && o2 != 0 && o3 != 0 && o4 != 0 && o1 == -o2 && o3 == -o4;
    if proper {
        return Ok(vec![cramer_params(&da, &r, &q, &s, &denom)?]);
    }
    if denom.is_zero() {
        // Parallel: only an actually collinear pair can overlap; distinct
        // parallel lines never meet.
        if o1 == 0 && o2 == 0 && o3 == 0 && o4 == 0 {
            return collinear_overlap(l1, l2);
        }
        return Ok(vec![]);
    }
    let (t, u, pt) = cramer_params(&da, &r, &q, &s, &denom)?;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Ok(vec![(t, u, pt)])
    } else {
        Ok(vec![])
    }
}

/// The exact Cramer parameters `(t on l1, u on l2, point)`, refusing when the
/// scaled-integer arithmetic overflows or the values are not exactly
/// representable.
fn cramer_params(da: &D2, r: &D2, q: &D2, s: &D2, denom: &Dyad) -> S1Result<(f64, f64, Point3)> {
    let num_t = dyad_result(q.cross(s))?;
    let num_u = dyad_result(q.cross(r))?;
    let t = ratio_f64(&num_t, denom)?;
    let u = ratio_f64(&num_u, denom)?;
    let px_num = dyad_result(da.x.mul(denom))?
        .add(&dyad_result(num_t.mul(&r.x))?)
        .ok_or_else(numerically_unresolved)?;
    let py_num = dyad_result(da.y.mul(denom))?
        .add(&dyad_result(num_t.mul(&r.y))?)
        .ok_or_else(numerically_unresolved)?;
    let px = ratio_f64(&px_num, denom)?;
    let py = ratio_f64(&py_num, denom)?;
    Ok((t, u, Point3::new(px, py, 0.0)))
}

/// Collinear segments: a positive-length overlap is the S5.3 2-D overlap case,
/// refused as `Empty`; otherwise no vertex is added.
fn collinear_overlap(l1: Line<Point2>, l2: Line<Point2>) -> S1Result<Vec<(f64, f64, Point3)>> {
    let Line(a, b) = l1;
    let da = d2_result(D2::from_point2(a))?;
    let db = d2_result(D2::from_point2(b))?;
    let dc = d2_result(D2::from_point2(l2.0))?;
    let dd = d2_result(D2::from_point2(l2.1))?;
    let r = d2_result(db.sub(&da))?;
    let rr = dyad_result(r.dot(&r))?;
    let tc_num = dyad_result(d2_result(dc.sub(&da))?.dot(&r))?;
    let td_num = dyad_result(d2_result(dd.sub(&da))?.dot(&r))?;
    let tc = ratio_f64(&tc_num, &rr)?;
    let td = ratio_f64(&td_num, &rr)?;
    let lo = tc.min(td);
    let hi = tc.max(td);
    let o_lo = lo.max(0.0);
    let o_hi = hi.min(1.0);
    if o_hi > o_lo {
        Err(Refusal::Empty)
    } else {
        Ok(vec![])
    }
}

/// Line/Circle: the quadratic `(d·d)t² + 2(f·d)t + (f·f − r²)` solved exactly.
/// When the discriminant is not a perfect square of a dyadic rational the
/// vertices are algebraic and v1 refuses.
fn line_circle_intersection(
    line: Line<Point2>,
    circle: &CircleCarrier,
) -> S1Result<Vec<(f64, f64, Point3)>> {
    let Line(a, b) = line;
    let da = d2_result(D2::from_point2(a))?;
    let db = d2_result(D2::from_point2(b))?;
    let dc = d2_result(D2::from_point2(circle.center))?;
    let r = d2_result(db.sub(&da))?;
    let f = d2_result(da.sub(&dc))?;
    let rdy = dyad_result(Dyad::from_f64(circle.radius))?;
    let dd = dyad_result(r.dot(&r))?;
    let fd = dyad_result(f.dot(&r))?;
    let ff = dyad_result(f.dot(&f))?;
    let r2 = dyad_result(rdy.mul(&rdy))?;
    let c_ = dyad_result(ff.sub(&r2))?;
    let disc = dyad_result(dyad_result(fd.mul(&fd))?.sub(&dyad_result(dd.mul(&c_))?))?;
    if disc.num < 0 {
        return Ok(vec![]);
    }
    let s = match disc.sqrt_exact() {
        Some(s) => s,
        None => return Err(numerically_unresolved()),
    };
    let neg_fd = Dyad {
        num: -fd.num,
        exp: fd.exp,
    };
    let t1_num = dyad_result(neg_fd.add(&s))?;
    let t2_num = dyad_result(neg_fd.sub(&s))?;
    let t1 = ratio_f64(&t1_num, &dd)?;
    let t2 = ratio_f64(&t2_num, &dd)?;
    let p1 = line_point(&da, &r, &t1_num, &dd)?;
    let p2 = line_point(&da, &r, &t2_num, &dd)?;
    let c1 = circle.param_of_point(p1);
    let c2 = circle.param_of_point(p2);
    Ok(vec![(t1, c1, pt3(p1)), (t2, c2, pt3(p2))])
}

/// The point `a + (num_t / denom) · r` in exact arithmetic.
fn line_point(da: &D2, r: &D2, num_t: &Dyad, denom: &Dyad) -> S1Result<Point2> {
    let px_num = dyad_result(da.x.mul(denom))?
        .add(&dyad_result(num_t.mul(&r.x))?)
        .ok_or_else(numerically_unresolved)?;
    let py_num = dyad_result(da.y.mul(denom))?
        .add(&dyad_result(num_t.mul(&r.y))?)
        .ok_or_else(numerically_unresolved)?;
    Ok(Point2::new(
        ratio_f64(&px_num, denom)?,
        ratio_f64(&py_num, denom)?,
    ))
}

/// Circle/Circle via the radical axis. Coincident circles are refused as
/// `Empty`; the roots are exact when the discriminant is a perfect square.
fn circle_circle_intersection(
    c1: &CircleCarrier,
    c2: &CircleCarrier,
) -> S1Result<Vec<(f64, f64, Point3)>> {
    let d1 = d2_result(D2::from_point2(c1.center))?;
    let d2 = d2_result(D2::from_point2(c2.center))?;
    let n = d2_result(d2.sub(&d1))?;
    let r1 = dyad_result(Dyad::from_f64(c1.radius))?;
    let r2 = dyad_result(Dyad::from_f64(c2.radius))?;
    let a = dyad_result(n.dot(&n))?;
    if a.is_zero() {
        if r1.num == r2.num && r1.exp == r2.exp {
            return Err(Refusal::Empty);
        }
        return Ok(vec![]);
    }
    let b = dyad_result(n.dot(&d1))?;
    let m1 = dyad_result(d1.dot(&d1))?;
    let m2 = dyad_result(d2.dot(&d2))?;
    let r1_2 = dyad_result(r1.mul(&r1))?;
    let r2_2 = dyad_result(r2.mul(&r2))?;
    let t1 = dyad_result(m2.sub(&m1))?;
    let t2 = dyad_result(t1.add(&r1_2))?;
    let kb = dyad_result(t2.sub(&r2_2))?;
    let k = Dyad {
        num: kb.num,
        exp: kb.exp - 1,
    };
    let ra = dyad_result(r1_2.mul(&a))?;
    let kminb = dyad_result(k.sub(&b))?;
    let kminb2 = dyad_result(kminb.mul(&kminb))?;
    let disc = dyad_result(ra.sub(&kminb2))?;
    if disc.num < 0 {
        return Ok(vec![]);
    }
    let t = match disc.sqrt_exact() {
        Some(t) => t,
        None => return Err(numerically_unresolved()),
    };
    let x0 = dyad_result(d1.x.mul(&a))?;
    let y0 = dyad_result(d1.y.mul(&a))?;
    let nxkb = dyad_result(n.x.mul(&kminb))?;
    let nykb = dyad_result(n.y.mul(&kminb))?;
    let tnx = dyad_result(t.mul(&n.x))?;
    let tny = dyad_result(t.mul(&n.y))?;
    let p1x_num = dyad_result(x0.add(&nxkb))?;
    let p1x = dyad_result(p1x_num.sub(&tny))?;
    let p1y_num = dyad_result(y0.add(&nykb))?;
    let p1y = dyad_result(p1y_num.add(&tnx))?;
    let p2x_num = dyad_result(x0.add(&nxkb))?;
    let p2x = dyad_result(p2x_num.add(&tny))?;
    let p2y_num = dyad_result(y0.add(&nykb))?;
    let p2y = dyad_result(p2y_num.sub(&tnx))?;
    let p1 = Point2::new(ratio_f64(&p1x, &a)?, ratio_f64(&p1y, &a)?);
    let p2 = Point2::new(ratio_f64(&p2x, &a)?, ratio_f64(&p2y, &a)?);
    Ok(vec![
        (c1.param_of_point(p1), c2.param_of_point(p1), pt3(p1)),
        (c2.param_of_point(p2), c2.param_of_point(p2), pt3(p2)),
    ])
}

/// Integer square root of a `u128`, `None` when not a perfect square.
fn isqrt_u128(n: u128) -> Option<u128> {
    if n == 0 {
        return Some(0);
    }
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    if x * x == n {
        Some(x)
    } else {
        None
    }
}

fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

/// The exact `f64` value of the dyadic rational `num / den`, when exactly
/// representable.
fn ratio_to_f64(num: Dyad, den: Dyad) -> Option<f64> {
    if den.is_zero() {
        return None;
    }
    if num.is_zero() {
        return Some(0.0);
    }
    let mut n = num.num;
    let mut d = den.num;
    let mut e = num.exp - den.exp;
    if d < 0 {
        n = -n;
        d = -d;
    }
    let g = gcd_u128(n.unsigned_abs(), d as u128) as i128;
    n /= g;
    d /= g;
    if d & (d - 1) != 0 {
        return None;
    }
    e -= d.trailing_zeros() as i32;
    Dyad { num: n, exp: e }.to_f64_exact()
}

fn d2_result(x: Option<D2>) -> S1Result<D2> {
    x.ok_or_else(numerically_unresolved)
}

fn dyad_result(x: Option<Dyad>) -> S1Result<Dyad> {
    x.ok_or_else(numerically_unresolved)
}

fn ratio_f64(num: &Dyad, den: &Dyad) -> S1Result<f64> {
    ratio_to_f64(*num, *den).ok_or_else(numerically_unresolved)
}

fn pt3(p: Point2) -> Point3 {
    Point3::new(p.x, p.y, 0.0)
}

fn sign_of(o: CertifiedPred) -> S1Result<i32> {
    match o {
        CertifiedPred::Proven(Orientation::CounterClockwise) => Ok(1),
        CertifiedPred::Proven(Orientation::Clockwise) => Ok(-1),
        CertifiedPred::Proven(Orientation::Collinear) => Ok(0),
        CertifiedPred::Unresolved(_) => Err(numerically_unresolved()),
    }
}

fn numerically_unresolved() -> Refusal {
    Refusal::NumericallyUnresolved {
        spent: Budget::new(0, 0, 0),
        witness: UnresolvedWitness::RootNotIsolated,
    }
}

fn contradiction() -> Refusal {
    Refusal::Contradictory(ContradictionWitness {
        prop: Prop::DomainBoundary,
        left: Truth::False,
        right: Truth::True,
    })
}

/// Traces the face walks: open boundary walks start at degree-1-origin
/// half-edges and terminate at `NO_NEXT`; the remaining half-edges form closed
/// face cycles.
fn trace_faces(
    vertices: &[ArrVertex],
    half_edges: &[ArrHalfEdge],
) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let mut visited = vec![false; half_edges.len()];
    let mut closed = Vec::new();
    let mut open = Vec::new();
    for e in 0..half_edges.len() {
        if visited.get(e).copied().unwrap_or(true) {
            continue;
        }
        let deg1 = half_edges
            .get(e)
            .and_then(|he| vertices.get(he.origin))
            .map(|v| v.incident.len() <= 1)
            .unwrap_or(false);
        if deg1 {
            let mut walk = Vec::new();
            let mut cur = e;
            loop {
                walk.push(cur);
                if let Some(slot) = visited.get_mut(cur) {
                    *slot = true;
                }
                let n = match half_edges.get(cur) {
                    Some(he) => he.next,
                    None => NO_NEXT,
                };
                if n == NO_NEXT {
                    break;
                }
                cur = n;
            }
            open.push(walk);
        }
    }
    for e in 0..half_edges.len() {
        if visited.get(e).copied().unwrap_or(true) {
            continue;
        }
        let mut cyc = Vec::new();
        let mut cur = e;
        loop {
            cyc.push(cur);
            if let Some(slot) = visited.get_mut(cur) {
                *slot = true;
            }
            let n = match half_edges.get(cur) {
                Some(he) => he.next,
                None => NO_NEXT,
            };
            if n == NO_NEXT {
                // A dangling reverse half-edge of an open walk (its origin is
                // not degree-1, so pass 1 never reached it); keep it as a
                // single-edge open walk rather than a closed cycle.
                open.push(vec![cur]);
                cyc.clear();
                break;
            }
            if n == e {
                break;
            }
            cur = n;
        }
        if !cyc.is_empty() {
            closed.push(cyc);
        }
    }
    (closed, open)
}

/// Merges the two tracings of each geometric loop (the CCW interior face cycle
/// and the CW exterior face cycle over the twins) into one representative.
fn merge_duplicate_cycles(
    closed: &[Vec<usize>],
    half_edges: &[ArrHalfEdge],
    carriers: &[Carrier2D],
) -> Vec<Vec<usize>> {
    let mut used = vec![false; closed.len()];
    let mut merged = Vec::new();
    for i in 0..closed.len() {
        if used.get(i).copied().unwrap_or(true) {
            continue;
        }
        let cyc_i = match closed.get(i) {
            Some(c) => c,
            None => continue,
        };
        let sig = cycle_signature(cyc_i, half_edges);
        let mut group = vec![i];
        for j in (i + 1)..closed.len() {
            if used.get(j).copied().unwrap_or(true) {
                continue;
            }
            let same = closed
                .get(j)
                .map(|c| cycle_signature(c, half_edges) == sig)
                .unwrap_or(false);
            if same {
                if let Some(slot) = used.get_mut(j) {
                    *slot = true;
                }
                group.push(j);
            }
        }
        if let Some(slot) = used.get_mut(i) {
            *slot = true;
        }
        let rep = group
            .iter()
            .copied()
            .find(|&g| {
                closed
                    .get(g)
                    .map(|c| signed_area(&cycle_polygon(c, half_edges, carriers)) > 0.0)
                    .unwrap_or(false)
            })
            .unwrap_or(i);
        if let Some(cyc) = closed.get(rep) {
            merged.push(cyc.clone());
        }
    }
    merged
}

/// The unordered multiset of `(curve, u-range)` segments a cycle covers — the
/// geometric identity of a closed loop, independent of traversal direction.
fn cycle_signature(cyc: &[usize], half_edges: &[ArrHalfEdge]) -> Vec<(usize, u64, u64)> {
    let mut sig = Vec::with_capacity(cyc.len());
    for &h in cyc {
        if let Some(he) = half_edges.get(h) {
            let (u0, u1) = he.u_range;
            sig.push((
                he.curve,
                u0.to_bits().min(u1.to_bits()),
                u0.to_bits().max(u1.to_bits()),
            ));
        }
    }
    sig.sort_unstable();
    sig
}

/// Whether `inner`'s polygon lies strictly inside `outer`'s polygon (every
/// vertex of `inner` has nonzero winding against `outer`).
fn cycle_inside(
    inner: &[usize],
    outer: &[usize],
    half_edges: &[ArrHalfEdge],
    carriers: &[Carrier2D],
) -> bool {
    let outer_poly = cycle_polygon(outer, half_edges, carriers);
    if outer_poly.is_empty() {
        return false;
    }
    let inner_poly = cycle_polygon(inner, half_edges, carriers);
    inner_poly
        .iter()
        .all(|&p| point_in_poly(p, &outer_poly).unwrap_or(false))
}

/// A monotone size proxy for a cycle (its polygon's bounding-box area).
fn cycle_size(cyc: &[usize], half_edges: &[ArrHalfEdge], carriers: &[Carrier2D]) -> f64 {
    let poly = cycle_polygon(cyc, half_edges, carriers);
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in poly {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    (max_x - min_x) * (max_y - min_y)
}

/// The nesting forest of closed cycles: `children[c]` are the cycles whose
/// direct parent is `c`; `roots` are the outermost cycles.
fn nest_cycles(
    merged: &[Vec<usize>],
    half_edges: &[ArrHalfEdge],
    carriers: &[Carrier2D],
) -> (Vec<Vec<usize>>, Vec<usize>) {
    let n = merged.len();
    let mut parent = vec![None; n];
    let sizes: Vec<f64> = (0..n)
        .map(|k| match merged.get(k) {
            Some(c) => cycle_size(c, half_edges, carriers),
            None => 0.0,
        })
        .collect();
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let (inner, outer) = match (merged.get(i), merged.get(j)) {
                (Some(a), Some(b)) => (a, b),
                _ => continue,
            };
            if !cycle_inside(inner, outer, half_edges, carriers) {
                continue;
            }
            let better = match parent.get(i).copied().flatten() {
                None => true,
                Some(p) => {
                    let sj = sizes.get(j).copied().unwrap_or(0.0);
                    let sp = sizes.get(p).copied().unwrap_or(0.0);
                    sj < sp
                }
            };
            if better {
                if let Some(slot) = parent.get_mut(i) {
                    *slot = Some(j);
                }
            }
        }
    }
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        if let Some(p) = parent.get(i).copied().flatten() {
            if let Some(ch) = children.get_mut(p) {
                ch.push(i);
            }
        }
    }
    let roots = (0..n)
        .filter(|&i| parent.get(i).copied().flatten().is_none())
        .collect();
    (children, roots)
}

/// The polygonization of a face cycle: each half-edge is sampled over its
/// parameter window (lines at their endpoints, arcs finely enough to resolve
/// point-in-loop decisions).
fn cycle_polygon(cyc: &[usize], half_edges: &[ArrHalfEdge], carriers: &[Carrier2D]) -> Vec<Point2> {
    let mut out = Vec::new();
    for &h in cyc {
        let he = match half_edges.get(h) {
            Some(he) => he,
            None => continue,
        };
        let carrier = match carriers.get(he.curve) {
            Some(c) => c,
            None => continue,
        };
        let (u0, u1) = he.u_range;
        let span = (u1 - u0).abs();
        let steps = match carrier {
            Carrier2D::Line(_) => 1usize,
            Carrier2D::Circle(_) => {
                let n = (POLY_SAMPLES as f64 * span / TAU).ceil() as usize;
                2usize.max(n)
            }
        };
        for k in 0..=steps {
            let t = u0 + (u1 - u0) * (k as f64 / steps as f64);
            out.push(carrier.subs(t));
        }
    }
    out
}

/// The signed polygon area (shoelace); positive means counter-clockwise.
fn signed_area(poly: &[Point2]) -> f64 {
    let mut s = 0.0;
    for (a, b) in poly.iter().zip(poly.iter().skip(1)) {
        s += a.x * b.y - a.y * b.x;
    }
    if let (Some(a), Some(b)) = (poly.last(), poly.first()) {
        s += a.x * b.y - a.y * b.x;
    }
    0.5 * s
}

/// The winding number of `p` over a polygonized loop, via the ray-casting rule
/// driven by `orient2d`. `None` if any crossing decision is unresolved.
fn polygon_winding(p: Point2, poly: &[Point2]) -> Option<i32> {
    let mut w = 0i32;
    for (a, b) in poly.iter().zip(poly.iter().skip(1)) {
        w += edge_winding(p, *a, *b)?;
    }
    if let (Some(a), Some(b)) = (poly.last(), poly.first()) {
        w += edge_winding(p, *a, *b)?;
    }
    Some(w)
}

/// One edge's signed crossing contribution to the winding of `p`.
fn edge_winding(p: Point2, a: Point2, b: Point2) -> Option<i32> {
    if a.y <= p.y {
        if b.y > p.y {
            match orient2d(a, b, p) {
                CertifiedPred::Proven(Orientation::CounterClockwise) => Some(1),
                CertifiedPred::Proven(Orientation::Clockwise)
                | CertifiedPred::Proven(Orientation::Collinear) => Some(0),
                CertifiedPred::Unresolved(_) => None,
            }
        } else {
            Some(0)
        }
    } else if b.y <= p.y {
        match orient2d(a, b, p) {
            CertifiedPred::Proven(Orientation::Clockwise) => Some(-1),
            CertifiedPred::Proven(Orientation::CounterClockwise)
            | CertifiedPred::Proven(Orientation::Collinear) => Some(0),
            CertifiedPred::Unresolved(_) => None,
        }
    } else {
        Some(0)
    }
}

/// Whether `p` is strictly inside the polygonized loop (nonzero winding).
fn point_in_poly(p: Point2, poly: &[Point2]) -> Option<bool> {
    Some(polygon_winding(p, poly)? != 0)
}

/// A point strictly inside `outer` and strictly outside every `hole`, from
/// candidates: the centroid, inward-nudged edge midpoints, and a bbox grid.
fn representative_inside_outside(outer: &[Point2], holes: &[Vec<Point2>]) -> Option<Point2> {
    let mut candidates = Vec::new();
    if let Some(c) = polygon_centroid(outer) {
        candidates.push(c);
    }
    let mut edges: Vec<(Point2, Point2)> = outer
        .iter()
        .zip(outer.iter().skip(1))
        .map(|(&a, &b)| (a, b))
        .collect();
    if let (Some(&a), Some(&b)) = (outer.first(), outer.last()) {
        edges.push((a, b));
    }
    for (a, b) in edges {
        let mid = Point2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
        let dir = b - a;
        let nudge = Vector2::new(-dir.y, dir.x) * (64.0 * TOLERANCE);
        candidates.push(mid + nudge);
    }
    if let Some((min, max)) = bbox_limits(outer) {
        let (min_x, min_y) = (min.x, min.y);
        let (max_x, max_y) = (max.x, max.y);
        const GRID: usize = 8;
        for gi in 0..=GRID {
            for gj in 0..=GRID {
                let p = Point2::new(
                    min_x + (max_x - min_x) * (gi as f64 / GRID as f64),
                    min_y + (max_y - min_y) * (gj as f64 / GRID as f64),
                );
                candidates.push(p);
            }
        }
    }
    for c in candidates {
        let in_outer = point_in_poly(c, outer).unwrap_or(false);
        if !in_outer {
            continue;
        }
        let in_hole = holes.iter().any(|h| point_in_poly(c, h).unwrap_or(false));
        if !in_hole {
            return Some(c);
        }
    }
    None
}

/// A point strictly outside every given polygon (outside the union bounding
/// box, nudged by the representation tolerance).
fn exterior_point(polys: &[Vec<Point2>]) -> Option<Point2> {
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for poly in polys {
        for p in poly {
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
    }
    if !max_x.is_finite() || !max_y.is_finite() {
        return None;
    }
    Some(Point2::new(
        max_x + 64.0 * TOLERANCE,
        max_y + 64.0 * TOLERANCE,
    ))
}

/// A point on the left of an open boundary walk (nudged from the first
/// half-edge's midpoint along its left normal).
fn open_walk_rep(
    walk: &[usize],
    half_edges: &[ArrHalfEdge],
    carriers: &[Carrier2D],
) -> Option<Point2> {
    let he = walk.first().copied().and_then(|h| half_edges.get(h))?;
    let carrier = carriers.get(he.curve)?;
    let (u0, u1) = he.u_range;
    let p = carrier.subs(0.5 * (u0 + u1));
    let d = half_edge_tangent(he, carriers);
    let len = d.magnitude();
    if len == 0.0 {
        return Some(p);
    }
    let n = Vector2::new(-d.y / len, d.x / len);
    Some(p + n * (64.0 * TOLERANCE))
}

fn polygon_centroid(poly: &[Point2]) -> Option<Point2> {
    let mut area = 0.0;
    let mut cx = 0.0;
    let mut cy = 0.0;
    for (a, b) in poly.iter().zip(poly.iter().skip(1)) {
        let cross = a.x * b.y - a.y * b.x;
        area += cross;
        cx += (a.x + b.x) * cross;
        cy += (a.y + b.y) * cross;
    }
    if let (Some(a), Some(b)) = (poly.last(), poly.first()) {
        let cross = a.x * b.y - a.y * b.x;
        area += cross;
        cx += (a.x + b.x) * cross;
        cy += (a.y + b.y) * cross;
    }
    if area == 0.0 {
        return None;
    }
    Some(Point2::new(cx / (3.0 * area), cy / (3.0 * area)))
}

fn bbox_limits(poly: &[Point2]) -> Option<(Point2, Point2)> {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in poly {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    if !min_x.is_finite() {
        return None;
    }
    Some((Point2::new(min_x, min_y), Point2::new(max_x, max_y)))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    fn p3(x: f64, y: f64) -> Point3 {
        Point3::new(x, y, 0.0)
    }

    fn line(a: Point2, b: Point2) -> Curve {
        Curve::Line(Line(p3(a.x, a.y), p3(b.x, b.y)))
    }

    fn circle(center: Point2, r: f64) -> Curve {
        let m = Matrix4 {
            x: Vector4::new(r, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, r, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(center.x, center.y, 0.0, 1.0),
        };
        Curve::Circle(Processor::with_transform(
            TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
            m,
        ))
    }

    fn pt2(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn arrange_rectangle_with_hole_has_three_regions() {
        let profile = vec![
            line(pt2(0.0, 0.0), pt2(4.0, 0.0)),
            line(pt2(4.0, 0.0), pt2(4.0, 4.0)),
            line(pt2(4.0, 4.0), pt2(0.0, 4.0)),
            line(pt2(0.0, 4.0), pt2(0.0, 0.0)),
            circle(pt2(2.0, 2.0), 1.0),
        ];
        let ok = arrange(&profile, None).unwrap();
        let arr = &ok.value;
        assert_eq!(arr.vertices.len(), 5);
        assert_eq!(arr.regions.len(), 3);

        let exterior = arr.regions.iter().find(|r| !r.bounded).unwrap();
        assert_eq!(exterior.winding, 0);
        assert_eq!(exterior.boundaries.len(), 1);
        assert_eq!(exterior.boundaries.first().unwrap().len(), 4);

        let plate = arr
            .regions
            .iter()
            .find(|r| r.bounded && r.boundaries.len() == 2)
            .unwrap();
        assert!(plate.winding == 1 || plate.winding == -1);
        let cycle_lens: Vec<usize> = plate.boundaries.iter().map(|b| b.len()).collect();
        assert!(cycle_lens.contains(&4));
        assert!(cycle_lens.contains(&1));

        let hole = arr
            .regions
            .iter()
            .find(|r| r.bounded && r.boundaries.len() == 1)
            .unwrap();
        assert!(hole.winding == 1 || hole.winding == -1);
        assert_eq!(hole.boundaries.first().unwrap().len(), 1);
    }

    #[test]
    fn arrange_crossing_lines_split_at_the_intersection() {
        let profile = vec![
            line(pt2(0.0, 0.0), pt2(2.0, 2.0)),
            line(pt2(0.0, 2.0), pt2(2.0, 0.0)),
        ];
        let ok = arrange(&profile, None).unwrap();
        let arr = &ok.value;
        let crossing = arr
            .vertices
            .iter()
            .find(|v| v.point == p3(1.0, 1.0))
            .unwrap();
        assert_eq!(crossing.incident.len(), 4);
        assert_eq!(arr.regions.len(), 4);
        for region in &arr.regions {
            assert!(!region.bounded);
            assert_eq!(region.winding, 0);
        }
    }

    #[test]
    fn arrange_line_circle_crossing_is_dyadic_exact() {
        let profile = vec![
            line(pt2(-1.0, 0.0), pt2(3.0, 0.0)),
            circle(pt2(1.0, 0.0), 1.0),
        ];
        let ok = arrange(&profile, None).unwrap();
        let arr = &ok.value;
        assert!(arr.vertices.iter().any(|v| v.point == p3(0.0, 0.0)));
        assert!(arr.vertices.iter().any(|v| v.point == p3(2.0, 0.0)));
        let circle_arcs = arr
            .half_edges
            .iter()
            .filter(|he| he.curve == 1 && he.u_range.0 < he.u_range.1)
            .count();
        assert_eq!(circle_arcs, 2);
    }

    #[test]
    fn arrange_self_intersecting_profile_is_refused() {
        let profile = vec![
            line(pt2(0.0, 0.0), pt2(2.0, 2.0)),
            line(pt2(2.0, 2.0), pt2(0.0, 2.0)),
            line(pt2(0.0, 2.0), pt2(2.0, 0.0)),
            line(pt2(2.0, 0.0), pt2(0.0, 0.0)),
        ];
        assert!(arrange(&profile, None).is_err());
    }

    #[test]
    fn arrange_circle_winding_is_one() {
        let profile = vec![circle(pt2(0.0, 0.0), 1.0)];
        let ok = arrange(&profile, None).unwrap();
        let arr = &ok.value;
        assert_eq!(arr.regions.len(), 2);
        let interior = arr.regions.iter().find(|r| r.bounded).unwrap();
        assert_eq!(interior.winding, 1);
        let exterior = arr.regions.iter().find(|r| !r.bounded).unwrap();
        assert_eq!(exterior.winding, 0);

        // The winding of the interior point (0, 0) over the circle loop is
        // exactly +1 for the CCW parameterization.
        let cycle = interior.boundaries.first().unwrap();
        let mut poly = Vec::new();
        for &h in cycle {
            let he = arr.half_edges.get(h).unwrap();
            let (u0, u1) = he.u_range;
            let steps = 32usize;
            for k in 0..=steps {
                let t = u0 + (u1 - u0) * (k as f64 / steps as f64);
                let p = profile.get(he.curve).unwrap().subs(t);
                poly.push(Point2::new(p.x, p.y));
            }
        }
        assert_eq!(polygon_winding(Point2::new(0.0, 0.0), &poly).unwrap(), 1);
    }
}
