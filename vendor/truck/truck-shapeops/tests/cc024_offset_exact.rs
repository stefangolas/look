//! CC-024-OFFSET-EXACT — sharp (mitered) and concave-edge completion via the
//! arrangement engine.
//!
//! The sharp/concave strata are ARRANGEMENT OUTPUTS (theory §3.4): the two
//! adjacent offset faces of a source edge, seen in the plane section of an
//! extruded shell, are completed by the extend-and-intersect rule — a convex
//! wedge whose offset faces DIVERGE is mitered (`MiteredStratum`, reach
//! ρ_A = |t|/sin θ, NEVER the |t| shortcut), a reflex wedge whose offset
//! faces OVERLAP is trimmed (`ConcaveTrim`: the cells the overlapping
//! adjacent offset faces cover are marked and discarded). The concave-trim
//! path is a NEW entry point next to `arrange`, so every fixture `arrange`
//! already answers keeps answering IDENTICALLY (the V5 identity gate, test
//! `existing_arrange_behavior_bit_identical_on_its_own_fixtures`). The trim
//! output carries provenance through the boolean `StratumRef` convention via
//! `boolean::assemble::trim_provenance`.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. This file is integration-test assertions on hand-built
// dyadic witnesses — not such a path.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::HashSet;
use std::f64::consts::{PI, TAU};
use truck_base::cgmath64::{Matrix4, Point2, Point3, Vector4};
use truck_geometry::arrange::{
    arrange, concave_trim, mitered_edge_reach, mitered_stratum, ConcaveTrim, MiteredStratum,
    OffsetSegment,
};
use truck_geometry::canonical::Curve;
use truck_geometry::prelude::*;
use truck_modeling::extrude::extrude_profile;
use truck_shapeops::boolean::assemble::trim_provenance;
use truck_shapeops::boolean::split::{SolidRef, StratumRef};
use truck_topology::{Edge, EdgeID, Shell};

/// The absolute comparison tolerance for the unit-scale dyadic fixtures
/// (H-3: all fixture numbers are dyadic in base two — quarters, halves and
/// integer offsets — so the tolerance only absorbs the transcendental
/// evaluations, never the geometry decisions).
const TOL: f64 = 1.0e-9; // H-3: dimensionless absolute tolerance for unit-scale fixtures

/// The section-geometry tolerance for the shell provenance matches (H-3:
/// the extruded shell re-realises the profile sections as edges; the profile
/// coordinates are exact, so this only absorbs float re-arithmetic).
const GEOM_TOL: f64 = 1.0e-4; // H-3: absolute section-match tolerance for provenance resolution

// ---------------------------------------------------------------------------
// construction helpers (dyadic fixtures only; copied verbatim in style from
// the boolean_m2 battery)
// ---------------------------------------------------------------------------

/// A z = 0 point.
fn p3(x: f64, y: f64) -> Point3 {
    Point3::new(x, y, 0.0)
}

/// A planar point.
fn p2(x: f64, y: f64) -> Point2 {
    Point2::new(x, y)
}

/// A z = 0 `Curve::Line`.
fn line(a: Point2, b: Point2) -> Curve {
    Curve::Line(Line(p3(a.x, a.y), p3(b.x, b.y)))
}

/// A placed full-period circle at `center` with radius `r`.
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

/// The 4x4 block profile, CCW.
fn square_profile() -> Vec<Curve> {
    vec![
        line(p2(0.0, 0.0), p2(4.0, 0.0)),
        line(p2(4.0, 0.0), p2(4.0, 4.0)),
        line(p2(4.0, 4.0), p2(0.0, 4.0)),
        line(p2(0.0, 4.0), p2(0.0, 0.0)),
    ]
}

/// The rectangle-plus-hole profile of `arrange`'s own fixture (four lines +
/// one full circle).
fn plate_with_hole_profile() -> Vec<Curve> {
    let mut profile = square_profile();
    profile.push(circle(p2(2.0, 2.0), 1.0));
    profile
}

/// The L reflex corner sections around the single concave vertex at (1, 1):
/// `a` = (4,1)→(1,1) and `b` = (1,1)→(1,4).
fn l_reflex_sections() -> (Curve, Curve) {
    (
        line(p2(4.0, 1.0), p2(1.0, 1.0)),
        line(p2(1.0, 1.0), p2(1.0, 4.0)),
    )
}

/// The distance from `p` to the INFINITE line through `a` and `b` (the plane
/// at z = 0).
fn dist_point_line(p: Point2, a: Point2, b: Point2) -> f64 {
    let ab = b - a;
    let ap = p - a;
    let len = ab.magnitude();
    if len == 0.0 {
        return ap.magnitude();
    }
    (ab.x * ap.y - ab.y * ap.x).abs() / len
}

/// The distance from `p` to the SEGMENT `a`–`b`.
fn dist_point_segment(p: Point2, a: Point2, b: Point2) -> f64 {
    let ab = b - a;
    let l2 = ab.x * ab.x + ab.y * ab.y;
    if l2 == 0.0 {
        return (p - a).magnitude();
    }
    let ap = p - a;
    let s = ((ap.x * ab.x + ap.y * ab.y) / l2).clamp(0.0, 1.0);
    let foot = a + ab * s;
    (p - foot).magnitude()
}

/// The endpoints of a curve section (first/last sample of its parameter
/// division).
fn section_ends(c: &Curve, tol: f64) -> (Point3, Point3) {
    let pts = c.parameter_division(c.range_tuple(), tol).1;
    (*pts.first().unwrap(), *pts.last().unwrap())
}

/// Whether two curve sections coincide as undirected segments.
fn sections_match(a: &Curve, b: &Curve, tol: f64) -> bool {
    let (a0, a1) = section_ends(a, tol);
    let (b0, b1) = section_ends(b, tol);
    let forward = (a0 - b0).magnitude() <= tol && (a1 - b1).magnitude() <= tol;
    let reverse = (a0 - b1).magnitude() <= tol && (a1 - b0).magnitude() <= tol;
    forward || reverse
}

/// Fetches the shell edge a `StratumRef::Edge` names, under the
/// `lift_edges`-style first-occurrence / flat-position convention (only solid
/// `A` is meaningful for the provenance bridge).
fn shell_edge_at(
    shell: &Shell<Point3, Curve, Surface>,
    r: StratumRef,
) -> Option<Edge<Point3, Curve>> {
    let (face, edge_pos) = match r {
        StratumRef::Edge {
            solid: SolidRef::A,
            face,
            edge,
        } => (face, edge),
        _ => return None,
    };
    let mut seen: HashSet<EdgeID<Curve>> = HashSet::new();
    let mut found = None;
    'outer: for (fi, f) in shell.face_iter().enumerate() {
        if fi > face {
            break;
        }
        let mut flat = 0usize;
        for wire in f.absolute_boundaries() {
            for edge in wire.edge_iter() {
                if !seen.insert(edge.id()) {
                    if fi == face {
                        flat += 1;
                    }
                    continue;
                }
                if fi == face {
                    if flat == edge_pos {
                        found = Some(edge.clone());
                        break 'outer;
                    }
                    flat += 1;
                }
            }
        }
    }
    found
}

// ---------------------------------------------------------------------------
// Test 1 (required): `mitered_edge_reach` pins the |t|/sin θ bound as code.
// ---------------------------------------------------------------------------

#[test]
fn mitered_wedge_edge_reach_bound_is_t_over_sin_theta() {
    // Dyadic-angle ground truths: ρ = |t| / sin θ for the exact sines.
    assert!((mitered_edge_reach(PI / 4.0, 1.0) - 2.0_f64.sqrt()).abs() < TOL); // H-3: 1/sin(pi/4) = sqrt(2)
    assert!((mitered_edge_reach(PI / 6.0, 1.0) - 2.0).abs() < TOL); // H-3: 1/sin(pi/6) = 2
    assert!((mitered_edge_reach(PI / 3.0, 2.0) - 4.0 / 3.0_f64.sqrt()).abs() < TOL); // H-3: 2/sin(pi/3) = 4/sqrt(3)
                                                                                     // The bound uses |t|: both offset signs answer identically.
    assert_eq!(
        mitered_edge_reach(PI / 4.0, -1.0),
        mitered_edge_reach(PI / 4.0, 1.0)
    );

    // Sweep of convex half-angles θ ∈ (0, π/2): the reach is |t|/sin θ and is
    // STRICTLY greater than |t| everywhere on the sweep.
    for &t in &[0.5, 1.0, 1.5, -2.0] {
        let mut k = 1usize;
        while k < 12 {
            let theta = PI * (k as f64) / 24.0;
            let reach = mitered_edge_reach(theta, t);
            let expected = t.abs() / theta.sin();
            assert!((reach - expected).abs() <= TOL); // H-3: reach pinned to |t|/sin(theta)
            assert!(
                reach > t.abs(),
                "convex reach must exceed |t| at θ = {theta}"
            );
            assert!(reach.is_finite());
            k += 1;
        }
    }

    // The degenerate half-angles have no finite sharp completion, and the
    // flat half-angle θ = π/2 pins the bound exactly at |t| (sin θ = 1).
    assert!(mitered_edge_reach(0.0, 1.0).is_infinite());
    assert_eq!(mitered_edge_reach(PI / 2.0, 1.0), 1.0);
    assert!(mitered_edge_reach(f64::INFINITY, 1.0).is_infinite());
    assert!(mitered_edge_reach(PI / 4.0, f64::INFINITY).is_infinite());
    assert!(mitered_edge_reach(PI / 4.0, f64::NAN).is_infinite());
}

// ---------------------------------------------------------------------------
// Test 2 (required): the concave-edge trim discards the covered cells.
// ---------------------------------------------------------------------------

#[test]
fn concave_edge_trim_discards_covered_cells() {
    // The L reflex vertex at (1, 1): `a` = (4,1)→(1,1), `b` = (1,1)→(1,4).
    // Offsetting outward by 0.5 overlaps the two adjacent offset faces across
    // the covered square cell (1,1)–(1.5,1.5); the trim rule marks that cell
    // and discards it, trimming each face back to the crossing (1.5, 1.5).
    let a = line(p2(4.0, 1.0), p2(1.0, 1.0));
    let b = line(p2(1.0, 1.0), p2(1.0, 4.0));
    let trim: ConcaveTrim = concave_trim(&a, &b, 0.5)
        .expect("the reflex trim resolves")
        .value;

    // The reflex wedge is 270° interior, so the reach is |t|/sin(3π/4).
    assert_eq!(trim.vertex, p3(1.0, 1.0));
    assert_eq!(trim.crossing, p3(1.5, 1.5));
    assert_eq!(trim.covered_cells, 1);
    assert!((trim.reach - 0.5 * 2.0_f64.sqrt()).abs() < TOL); // H-3: 0.5/sin(3pi/4)
    assert!(trim.reach > 0.5);
    assert!((trim.half_angle - 3.0 * PI / 4.0).abs() < TOL); // H-3: half of the 270° wedge

    // Surviving cells: each offset face from the crossing to its far end.
    assert_eq!(trim.surviving.len(), 2);
    assert_eq!(
        trim.surviving[0],
        OffsetSegment {
            source: 0,
            from: p3(1.5, 1.5),
            to: p3(4.0, 1.5),
        }
    );
    assert_eq!(
        trim.surviving[1],
        OffsetSegment {
            source: 1,
            from: p3(1.5, 1.5),
            to: p3(1.5, 4.0),
        }
    );

    // Trim curves: the covered sub-segments between each face's vertex-near
    // end and the crossing, discarded from the boundary.
    assert_eq!(trim.trims.len(), 2);
    assert_eq!(
        trim.trims[0],
        OffsetSegment {
            source: 0,
            from: p3(1.0, 1.5),
            to: p3(1.5, 1.5),
        }
    );
    assert_eq!(
        trim.trims[1],
        OffsetSegment {
            source: 1,
            from: p3(1.5, 1.0),
            to: p3(1.5, 1.5),
        }
    );

    // The surviving cells and the trims partition each natural offset span.
    let natural_a = (p3(4.0, 1.5) - p3(1.0, 1.5)).magnitude();
    let kept_a = (trim.surviving[0].to - trim.surviving[0].from).magnitude();
    let cut_a = (trim.trims[0].to - trim.trims[0].from).magnitude();
    assert!((kept_a + cut_a - natural_a).abs() < TOL); // H-3: survivor + trim = natural span
    let natural_b = (p3(1.5, 4.0) - p3(1.5, 1.0)).magnitude();
    let kept_b = (trim.surviving[1].to - trim.surviving[1].from).magnitude();
    let cut_b = (trim.trims[1].to - trim.trims[1].from).magnitude();
    assert!((kept_b + cut_b - natural_b).abs() < TOL); // H-3: survivor + trim = natural span

    // The covered cell is GONE from the surviving geometry: its interior
    // point (1.25, 1.25) stays clear of both surviving segments.
    let interior = p2(1.25, 1.25);
    let sv0 = (
        p2(trim.surviving[0].from.x, trim.surviving[0].from.y),
        p2(trim.surviving[0].to.x, trim.surviving[0].to.y),
    );
    let sv1 = (
        p2(trim.surviving[1].from.x, trim.surviving[1].from.y),
        p2(trim.surviving[1].to.x, trim.surviving[1].to.y),
    );
    assert!(dist_point_segment(interior, sv0.0, sv0.1) > 0.3);
    assert!(dist_point_segment(interior, sv1.0, sv1.1) > 0.3);
}

// ---------------------------------------------------------------------------
// Test 3 (required): the concave-trim path is additive — the surviving cells
// carry provenance through the boolean `StratumRef` convention.
// ---------------------------------------------------------------------------

#[test]
fn concave_trim_source_tags_resolve_through_stratum_refs() {
    // The reflex trim tags every surviving cell and trim curve with its source
    // section (`source` 0/1, the two adjacent edges of the concave vertex);
    // `trim_provenance` resolves a tagged section to the shell `StratumRef`
    // that realises it (the boolean convention landed in assemble.rs). The
    // reflex completion is pure plane geometry on the two adjacent sections;
    // the bridge is demonstrated on the block prism, whose edges ARE the
    // arranged block-profile sections (the same resolution the offset flow
    // performs on the source shell).
    let (a, b) = l_reflex_sections();
    let trim: ConcaveTrim = concave_trim(&a, &b, 0.5)
        .expect("the reflex trim resolves")
        .value;
    assert_eq!(trim.covered_cells, 1);
    assert_eq!(trim.surviving[0].source, 0);
    assert_eq!(trim.surviving[1].source, 1);
    assert_eq!(trim.trims[0].source, 0);
    assert_eq!(trim.trims[1].source, 1);

    // The block prism realises its profile sections as shell edges; the
    // bridge resolves them to distinct, geometry-matching `StratumRef::Edge`s.
    let profile = square_profile();
    let ok = arrange(&profile, None).expect("the block profile arranges");
    let solid = extrude_profile(&profile, &ok.value, 1.0)
        .expect("the block prism extrudes")
        .value;
    let shell = solid
        .boundaries()
        .first()
        .expect("a one-shell solid")
        .clone();
    let sources = vec![profile[0].clone(), profile[1].clone()];
    let refs = trim_provenance(&shell, &sources, GEOM_TOL)
        .expect("both sections are realised as shell edges")
        .value;
    assert_eq!(refs.len(), 2);
    assert_ne!(
        refs[0], refs[1],
        "the two sections are distinct shell edges"
    );
    for (source, r) in sources.iter().zip(refs.iter()) {
        let edge = shell_edge_at(&shell, *r).expect("the ref names a real shell edge");
        assert!(
            sections_match(source, &edge.curve(), GEOM_TOL),
            "the ref must name the shell edge realising its source section"
        );
        assert!(matches!(
            r,
            StratumRef::Edge {
                solid: SolidRef::A,
                ..
            }
        ));
    }
}

// ---------------------------------------------------------------------------
// Test 4 (required): the mitered stratum carries the COMPUTED reach, not |t|.
// ---------------------------------------------------------------------------

#[test]
fn mitered_stratum_carries_computed_reach_not_t() {
    // The convex square corner at (4, 0): `a` = (0,0)→(4,0), `b` = (4,0)→(4,4),
    // offset outward by 1. The two adjacent offset faces (each at distance 1
    // from its source edge) DIVERGE, so the sharp completion extends both and
    // intersects them at the miter vertex.
    let profile = square_profile();
    let a = profile[0].clone();
    let b = profile[1].clone();
    let stratum: MiteredStratum = mitered_stratum(&a, &b, 1.0)
        .expect("the square corner miters")
        .value;

    assert_eq!(stratum.source, p3(4.0, 0.0));
    assert!((stratum.half_angle - PI / 4.0).abs() < TOL); // H-3: half of the 90° wedge
    assert_eq!(stratum.miter_point, p3(5.0, -1.0));

    // The reach is the COMPUTED bound |t|/sin θ = √2 — strictly greater than
    // |t|, never the ball-stratum shortcut.
    assert!((stratum.reach - 2.0_f64.sqrt()).abs() < TOL); // H-3: 1/sin(pi/4) = sqrt(2)
    assert!(stratum.reach > 1.0);
    assert!(stratum.reach > stratum.offset.abs());
    assert!((stratum.reach - mitered_edge_reach(stratum.half_angle, stratum.offset)).abs() < TOL); // H-3: reach is the pinned bound

    // The geometry agrees with the bound: the miter vertex sits at the
    // computed reach from the source ...
    let d = (stratum.miter_point - stratum.source).magnitude();
    assert!((d - stratum.reach).abs() < TOL); // H-3: miter vertex distance equals the reach bound
    assert!(d > 1.0, "the miter point is NOT within |t| of its source");

    // ... and the extend-and-intersect rule was evaluated on the landed face
    // carriers: each offset face passes through the miter vertex at distance
    // |t| from its source edge.
    let m2 = p2(stratum.miter_point.x, stratum.miter_point.y);
    let d_line_a = dist_point_line(m2, p2(0.0, 0.0), p2(4.0, 0.0));
    let d_line_b = dist_point_line(m2, p2(4.0, 0.0), p2(4.0, 4.0));
    assert!((d_line_a - 1.0).abs() < TOL); // H-3: offset face a is at distance |t|
    assert!((d_line_b - 1.0).abs() < TOL); // H-3: offset face b is at distance |t|

    // The direction field points along the miter bisector (unit).
    let expect_dir = 1.0 / 2.0_f64.sqrt();
    assert!((stratum.direction.x - expect_dir).abs() < TOL); // H-3: bisector x component
    assert!((stratum.direction.y + expect_dir).abs() < TOL); // H-3: bisector y component
}

// ---------------------------------------------------------------------------
// Test 5 (required): the V5 identity gate — every fixture `arrange` already
// answers answers IDENTICALLY after this packet (additive paths only).
// ---------------------------------------------------------------------------

#[test]
fn existing_arrange_behavior_bit_identical_on_its_own_fixtures() {
    // Rectangle-with-hole (arrange's own fixture): the same structure, twice,
    // bit-identical.
    let profile = plate_with_hole_profile();
    let first = arrange(&profile, None).unwrap().value;
    let second = arrange(&profile, None).unwrap().value;
    assert_eq!(first, second, "arrange answers bit-identically on repeat");
    assert_eq!(first.vertices.len(), 5);
    assert_eq!(first.regions.len(), 3);
    let exterior = first.regions.iter().find(|r| !r.bounded).unwrap();
    assert_eq!(exterior.winding, 0);
    assert_eq!(exterior.boundaries.len(), 1);
    assert_eq!(exterior.boundaries.first().unwrap().len(), 4);
    let plate = first
        .regions
        .iter()
        .find(|r| r.bounded && r.boundaries.len() == 2)
        .unwrap();
    assert!(plate.winding == 1 || plate.winding == -1);
    let cycle_lens: Vec<usize> = plate.boundaries.iter().map(|b| b.len()).collect();
    assert!(cycle_lens.contains(&4));
    assert!(cycle_lens.contains(&1));
    let hole = first
        .regions
        .iter()
        .find(|r| r.bounded && r.boundaries.len() == 1)
        .unwrap();
    assert!(hole.winding == 1 || hole.winding == -1);
    assert_eq!(hole.boundaries.first().unwrap().len(), 1);

    // Crossing lines split at the intersection: the same vertex, twice,
    // bit-identical.
    let crossing = vec![
        line(p2(0.0, 0.0), p2(2.0, 2.0)),
        line(p2(0.0, 2.0), p2(2.0, 0.0)),
    ];
    let first = arrange(&crossing, None).unwrap().value;
    let second = arrange(&crossing, None).unwrap().value;
    assert_eq!(first, second, "arrange answers bit-identically on repeat");
    let vertex = first
        .vertices
        .iter()
        .find(|v| v.point == p3(1.0, 1.0))
        .unwrap();
    assert_eq!(vertex.incident.len(), 4);
    assert_eq!(first.regions.len(), 4);
    for region in &first.regions {
        assert!(!region.bounded);
        assert_eq!(region.winding, 0);
    }

    // Line-circle crossing is dyadic-exact.
    let mixed = vec![line(p2(-1.0, 0.0), p2(3.0, 0.0)), circle(p2(1.0, 0.0), 1.0)];
    let first = arrange(&mixed, None).unwrap().value;
    let second = arrange(&mixed, None).unwrap().value;
    assert_eq!(first, second, "arrange answers bit-identically on repeat");
    assert!(first.vertices.iter().any(|v| v.point == p3(0.0, 0.0)));
    assert!(first.vertices.iter().any(|v| v.point == p3(2.0, 0.0)));
    let circle_arcs = first
        .half_edges
        .iter()
        .filter(|he| he.curve == 1 && he.u_range.0 < he.u_range.1)
        .count();
    assert_eq!(circle_arcs, 2);

    // A pure circle still winds once.
    let disk = vec![circle(p2(0.0, 0.0), 1.0)];
    let first = arrange(&disk, None).unwrap().value;
    let second = arrange(&disk, None).unwrap().value;
    assert_eq!(first, second, "arrange answers bit-identically on repeat");
    assert_eq!(first.regions.len(), 2);
    assert_eq!(first.regions.iter().find(|r| r.bounded).unwrap().winding, 1);
    assert_eq!(
        first.regions.iter().find(|r| !r.bounded).unwrap().winding,
        0
    );

    // A self-intersecting single loop is still refused.
    let bowtie = vec![
        line(p2(0.0, 0.0), p2(2.0, 2.0)),
        line(p2(2.0, 2.0), p2(0.0, 2.0)),
        line(p2(0.0, 2.0), p2(2.0, 0.0)),
        line(p2(2.0, 0.0), p2(0.0, 0.0)),
    ];
    assert!(arrange(&bowtie, None).is_err());
}

// ---------------------------------------------------------------------------
// Supporting: the completion rules refuse what is outside their v1 envelope
// (recorded here so the envelope is a typed outcome, never an approximation).
// ---------------------------------------------------------------------------

#[test]
fn completion_rules_refuse_the_mirror_regime_and_curved_sections() {
    // The convex miter rule refuses a reflex wedge.
    let reflex_a = line(p2(4.0, 1.0), p2(1.0, 1.0));
    let reflex_b = line(p2(1.0, 1.0), p2(1.0, 4.0));
    assert!(mitered_stratum(&reflex_a, &reflex_b, 0.5).is_err());

    // The concave trim rule refuses a convex wedge.
    let convex_a = line(p2(0.0, 0.0), p2(4.0, 0.0));
    let convex_b = line(p2(4.0, 0.0), p2(4.0, 4.0));
    assert!(concave_trim(&convex_a, &convex_b, 1.0).is_err());

    // Curved (Circle) face sections refuse both rules: a curved-face miter
    // routes through the landed certified pair machinery.
    let curved = circle(p2(0.0, 0.0), 1.0);
    let straight = line(p2(0.0, 0.0), p2(4.0, 0.0));
    assert!(mitered_stratum(&curved, &straight, 1.0).is_err());
    assert!(concave_trim(&straight, &curved, 1.0).is_err());

    // Non-consecutive sections (no shared vertex) refuse both rules.
    let far_a = line(p2(0.0, 0.0), p2(2.0, 0.0));
    let far_b = line(p2(5.0, 5.0), p2(5.0, 9.0));
    assert!(mitered_stratum(&far_a, &far_b, 1.0).is_err());
    assert!(concave_trim(&far_a, &far_b, 1.0).is_err());
}
