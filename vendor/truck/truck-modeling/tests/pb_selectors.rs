//! PB-001-SELECTORS — the four required acceptance tests for the scoped
//! selector layer (`truck_modeling::selectors`).
//!
//! 1. `face_iteration_yields_stable_order` — two iterations of one solid give
//!    the identical refs; a translated copy gives EntityId-consistent refs.
//! 2. `centroid_and_aabb_match_brute` — per-face centroid/AABB facts agree
//!    with an independent brute dense sample on three solids.
//! 3. `axis_sort_group_filter_semantics` — sort/group/filter/take compose into
//!    the documented selection (top face by Z, faces on a plane, last edge).
//! 4. `edge_resolution_names_blend_targets` — the resolved edge names are
//!    exactly what the landed `fillet` resolver accepts: a round trip against
//!    the landed D2 acceptance predicate over a box's four vertical edges.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry; integration-test assertions on hand-built witnesses are
// not such a path. The deny list above stays; the match helpers below unwind
// via `panic!` so the deny lints stay satisfied (the recognize.rs precedent).
#![allow(clippy::panic)]
// Indexing is used freely on hand-built local `Vec`s in this test crate (the
// integration-test equivalent of the recognize.rs test-module precedent).
#![allow(clippy::indexing_slicing)]

use truck_base::bounding_box::BoundingBox;
use truck_base::cgmath64::{EuclideanSpace, InnerSpace, Vector3};
use truck_base::evidence::Outcome;
use truck_modeling::cad::translate_solid;
use truck_modeling::selectors::{edges, faces, Axis, EdgeName, FaceRef};
use truck_modeling::ParametricSurface;
use truck_modeling::{primitive, Curve, Plane, Point3, Solid, Surface};
use truck_topology::{Edge, EntityId, Selector};

/// The per-face facts tolerance of test 2. // H-3: the module's box faces are
/// closed-form at dyadic vertices; the brute region sample agrees to well
/// below this.
const FACTS_EPS: f64 = 1.0e-6;
/// The brute region-sample agreement tolerance of test 2. // H-3: a uniform
/// grid over the face polygon converges to the region centroid only up to a
/// boundary-layer `O(extent / grid)` term; 0.02 covers the 4-unit fixtures at
/// the 256-grid brute.
const BRUTE_EPS: f64 = 2.0e-2;
/// The filter tolerance of test 3. // H-3: box vertices sit exactly on the
/// plane coordinates tested; this only absorbs float noise.
const PLANE_TOL: f64 = 1.0e-6;
/// The moved-copy facts tolerance of test 1. // H-3: a translated box keeps
/// exact planar facts shifted by exactly the translation vector.
const COPY_EPS: f64 = 1.0e-6;
/// Mirrors the landed rewrite resolver's insertion tolerance class. // H-3:
/// the landed `rewrite::resolve` matches edge endpoints within 1e-2.
const INSERTION_TOL: f64 = 1.0e-2;
/// The dense brute region-grid resolution of test 2 (independent of the
/// module's closed-form facts).
const BRUTE_GRID: usize = 256;
/// Unwraps an `Outcome` via `match` + `panic!` so the deny lints stay
/// satisfied (the recognize.rs test-module precedent).
fn expect_ok<T>(r: Outcome<T>) -> T {
    match r {
        Ok(ok) => ok.value,
        Err(refusal) => panic!("expected a certified value, got {refusal:?}"),
    }
}

/// Unwraps an `Option` via `match` + `panic!` so the deny lints stay
/// satisfied (the recognize.rs test-module precedent).
fn unwrap_some<T>(value: Option<T>, message: &str) -> T {
    match value {
        Some(v) => v,
        None => panic!("{message}"),
    }
}

/// The planar face polygon data of a brute sample: the carrier plane, the
/// world boundary loop, and its affine (u, v) projection.
type FaceUv = (Plane, Vec<Point3>, Vec<(f64, f64)>);

/// A `[min, max]` cuboid solid on the world axes.
fn box_solid(min: [f64; 3], max: [f64; 3]) -> Solid {
    primitive::cuboid(BoundingBox::from_iter([
        Point3::new(min[0], min[1], min[2]),
        Point3::new(max[0], max[1], max[2]),
    ]))
}

/// The outermost selector of an EntityId-derived ref (panics on a non-Sel id:
/// the test only feeds refs produced by `selectors::faces`).
fn top_selector(id: &EntityId) -> Selector {
    match id {
        EntityId::Sel { selector, .. } => *selector,
        _ => panic!("ref id is not a Sel"),
    }
}

/// The outer boundary loop of a planar face (single wire; the fixtures are
/// simple boxes) in world points and in the carrier plane's affine (u, v).
fn face_polygon_uv(face: &FaceRef) -> Option<FaceUv> {
    let plane = match face.face().shared_surface() {
        Surface::Plane(plane) => *plane,
        _ => return None,
    };
    let wire = face.face().absolute_boundaries().first()?;
    let mut world = Vec::new();
    for edge in wire.edge_iter() {
        world.push(edge.front().point());
    }
    let mut uv = Vec::with_capacity(world.len());
    for point in &world {
        let g = plane.get_parameter(*point);
        uv.push((g.x, g.y));
    }
    Some((plane, world, uv))
}

/// Ray-casting point-in-polygon test on an affine (u, v) polygon.
fn uv_inside(p: (f64, f64), poly: &[(f64, f64)]) -> bool {
    let (x, y) = p;
    let mut inside = false;
    let n = poly.len();
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if (yi > y) != (yj > y) {
            let cross = (xj - xi) * (y - yi) / (yj - yi) + xi;
            if x < cross {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// The brute facts of one face: a dense uniform sample of the face REGION
/// (never the untrimmed carrier domain), computed independently of the module
/// by interior inclusion. Returns `(centroid, aabb_min, aabb_max)`.
fn brute_face_fact(face: &FaceRef) -> (Point3, Point3, Point3) {
    let Some((plane, world, uv)) = face_polygon_uv(face) else {
        // Non-planar faces are not present in the box fixtures.
        return (
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 0.0),
        );
    };
    let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in &world {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        min.z = min.z.min(p.z);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
        max.z = max.z.max(p.z);
    }
    let mut umin = f64::INFINITY;
    let mut umax = f64::NEG_INFINITY;
    let mut vmin = f64::INFINITY;
    let mut vmax = f64::NEG_INFINITY;
    for (u, v) in &uv {
        umin = umin.min(*u);
        umax = umax.max(*u);
        vmin = vmin.min(*v);
        vmax = vmax.max(*v);
    }
    let mut sum = Vector3::new(0.0, 0.0, 0.0);
    let mut count = 0.0;
    for i in 0..BRUTE_GRID {
        for j in 0..BRUTE_GRID {
            let u = umin + (umax - umin) * ((i as f64 + 0.5) / BRUTE_GRID as f64);
            let v = vmin + (vmax - vmin) * ((j as f64 + 0.5) / BRUTE_GRID as f64);
            if uv_inside((u, v), &uv) {
                sum += plane.subs(u, v).to_vec();
                count += 1.0;
            }
        }
    }
    let centroid = if count > 0.0 {
        Point3::from_vec(sum / count)
    } else {
        Point3::new(0.0, 0.0, 0.0)
    };
    (centroid, min, max)
}

// ---------------------------------------------------------------------------
// Test 1: deterministic iteration order; EntityId-consistent refs.
// ---------------------------------------------------------------------------

#[test]
fn face_iteration_yields_stable_order() {
    let solid = box_solid([0.0, 0.0, 0.0], [4.0, 4.0, 2.0]);
    let root = EntityId::src(7);

    // Two iterations of the same solid yield identical refs, in the same
    // order, with bit-identical facts.
    let first = faces(&solid, root.clone()).into_vec();
    let second = faces(&solid, root.clone()).into_vec();
    assert_eq!(first.len(), 6);
    assert_eq!(first.len(), second.len());
    for (a, b) in first.iter().zip(second.iter()) {
        assert_eq!(a.id(), b.id());
        assert_eq!(a.fact().centroid, b.fact().centroid);
        assert_eq!(a.fact().aabb.min(), b.fact().aabb.min());
        assert_eq!(a.fact().aabb.max(), b.fact().aabb.max());
    }

    // A translated copy yields EntityId-consistent refs: the same selector
    // path per structural position, and the facts shifted by the translation.
    let t = Vector3::new(1.0, -2.0, 3.0);
    let moved = expect_ok(translate_solid(&solid, t));
    let moved_root = EntityId::src(8);
    let moved_faces = faces(&moved, moved_root.clone()).into_vec();
    assert_eq!(moved_faces.len(), first.len());
    for (original, shifted) in first.iter().zip(moved_faces.iter()) {
        assert_eq!(top_selector(original.id()), top_selector(shifted.id()));
        let expect_centroid = original.fact().centroid + t;
        let expect_min = original.fact().aabb.min() + t;
        let expect_max = original.fact().aabb.max() + t;
        assert!(
            (shifted.fact().centroid - expect_centroid).magnitude() <= COPY_EPS,
            "centroid shifted by t: {:?} vs {:?}",
            shifted.fact().centroid,
            expect_centroid
        );
        assert!(
            (shifted.fact().aabb.min() - expect_min).magnitude() <= COPY_EPS,
            "aabb min shifted by t"
        );
        assert!(
            (shifted.fact().aabb.max() - expect_max).magnitude() <= COPY_EPS,
            "aabb max shifted by t"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 2: centroid + AABB facts match the brute dense sample.
// ---------------------------------------------------------------------------

#[test]
fn centroid_and_aabb_match_brute() {
    let solids = [
        box_solid([0.0, 0.0, 0.0], [4.0, 4.0, 2.0]),
        box_solid([-2.0, -1.0, 1.0], [1.0, 5.0, 3.0]),
        box_solid([1.0, 2.0, -3.0], [3.0, 6.0, -1.0]),
    ];

    for (k, solid) in solids.iter().enumerate() {
        let selected = faces(solid, EntityId::src(k as u64)).into_vec();
        assert_eq!(selected.len(), 6);
        for face in &selected {
            let (brute_centroid, brute_min, brute_max) = brute_face_fact(face);
            let fact = face.fact();
            assert!(
                (fact.centroid - brute_centroid).magnitude() <= BRUTE_EPS,
                "solid {k} centroid {:?} vs brute {:?}",
                fact.centroid,
                brute_centroid
            );
            assert!(
                (fact.aabb.min() - brute_min).magnitude() <= FACTS_EPS,
                "solid {k} aabb min {:?} vs brute {:?}",
                fact.aabb.min(),
                brute_min
            );
            assert!(
                (fact.aabb.max() - brute_max).magnitude() <= FACTS_EPS,
                "solid {k} aabb max {:?} vs brute {:?}",
                fact.aabb.max(),
                brute_max
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Test 3: sort / group / filter / take compose into the documented selection.
// ---------------------------------------------------------------------------

#[test]
fn axis_sort_group_filter_semantics() {
    let solid = box_solid([0.0, 0.0, 0.0], [4.0, 4.0, 2.0]);
    let root = EntityId::src(9);

    // Top face by Z: sort ascending on the centroid's z, take the last one.
    let top = unwrap_some(
        faces(&solid, root.clone()).sort_by_axis(Axis::Z).last(),
        "a box has faces",
    );
    assert!((top.fact().centroid.z - 2.0).abs() <= FACTS_EPS);
    assert!((top.fact().aabb.max().z - 2.0).abs() <= FACTS_EPS);

    // Faces on a plane: the z = 2 plane selects exactly the top face.
    let top_plane = Plane::new(
        Point3::new(0.0, 0.0, 2.0),
        Point3::new(1.0, 0.0, 2.0),
        Point3::new(0.0, 1.0, 2.0),
    );
    let on_top_plane = faces(&solid, root.clone())
        .filter_by_plane(&top_plane, PLANE_TOL)
        .into_vec();
    assert_eq!(on_top_plane.len(), 1);
    assert!((on_top_plane[0].fact().centroid.z - 2.0).abs() <= FACTS_EPS);

    // Faces on the x = 4 plane: exactly the one side face at x = 4.
    let side_plane = Plane::new(
        Point3::new(4.0, 0.0, 0.0),
        Point3::new(4.0, 1.0, 0.0),
        Point3::new(4.0, 0.0, 1.0),
    );
    let on_side_plane = faces(&solid, root.clone())
        .filter_by_plane(&side_plane, PLANE_TOL)
        .into_vec();
    assert_eq!(on_side_plane.len(), 1);
    assert!((on_side_plane[0].fact().centroid.x - 4.0).abs() <= FACTS_EPS);

    // Group by Z: the box faces group into z = 0 (bottom), z = 1 (the four
    // sides), z = 2 (top), ascending.
    let groups = faces(&solid, root.clone()).group_by_axis(Axis::Z);
    assert_eq!(groups.len(), 3);
    let keys: Vec<f64> = groups.iter().map(|(key, _)| *key).collect();
    assert!((keys[0] - 0.0).abs() <= FACTS_EPS);
    assert!((keys[1] - 1.0).abs() <= FACTS_EPS);
    assert!((keys[2] - 2.0).abs() <= FACTS_EPS);
    assert_eq!(groups[0].1.len(), 1);
    assert_eq!(groups[1].1.len(), 4);
    assert_eq!(groups[2].1.len(), 1);
    assert!((groups[2].1.refs()[0].fact().centroid.z - 2.0).abs() <= FACTS_EPS);

    // Take then last: taking the sorted selection and popping the last gives
    // the same top face.
    let top_again = unwrap_some(
        faces(&solid, root.clone())
            .sort_by_axis(Axis::Z)
            .take(6)
            .last(),
        "take keeps a top face",
    );
    assert_eq!(top.id(), top_again.id());

    // Edges: 12 unique edges on a box; take and last compose deterministically.
    let all_edges = edges(&solid, root.clone()).into_vec();
    assert_eq!(all_edges.len(), 12);
    let last_edge = unwrap_some(edges(&solid, root.clone()).last(), "a box has edges");
    assert_eq!(all_edges[all_edges.len() - 1].id(), last_edge.id());
    assert_eq!(edges(&solid, root.clone()).take(3).len(), 3);
}

// ---------------------------------------------------------------------------
// Test 4: resolved edge names are what the landed `fillet` accepts.
// ---------------------------------------------------------------------------

/// The straight-edge name fields `FilletSpec`/`ChamferSpec` carry (the
/// endpoint pair). Kept field-identical to the landed spec structs so the
/// resolved name fills them verbatim; the radius is the caller's fillet
/// parameter, added when the name is lifted into a `FilletSpec` row.
#[derive(Clone, Copy, Debug)]
struct FilletRow {
    a: Point3,
    b: Point3,
}

/// The unique edges of a solid, each with its face-use count, enumerated
/// independently of `selectors` (the round trip must not reuse the resolver).
fn unique_edges_with_uses(solid: &Solid) -> Vec<(Edge<Point3, Curve>, usize)> {
    let mut uniq: Vec<Edge<Point3, Curve>> = Vec::new();
    for face in solid.face_iter() {
        for wire in face.absolute_boundaries() {
            for edge in wire.edge_iter() {
                if !uniq.iter().any(|u| u.is_same(edge)) {
                    uniq.push(edge.clone());
                }
            }
        }
    }
    let mut out = Vec::new();
    for unique in uniq {
        let mut uses = 0usize;
        for face in solid.face_iter() {
            for wire in face.absolute_boundaries() {
                for edge in wire.edge_iter() {
                    if edge.is_same(&unique) {
                        uses += 1;
                    }
                }
            }
        }
        out.push((unique, uses));
    }
    out
}

/// The number of faces whose boundary passes through `point` (endpoint-vertex
/// incidence).
fn faces_through(solid: &Solid, point: Point3) -> usize {
    let mut count = 0usize;
    for face in solid.face_iter() {
        let mut touches = false;
        'wires: for wire in face.absolute_boundaries() {
            for edge in wire.edge_iter() {
                let (a, b) = edge.absolute_ends();
                if (a.point() - point).magnitude() <= INSERTION_TOL
                    || (b.point() - point).magnitude() <= INSERTION_TOL
                {
                    touches = true;
                    break 'wires;
                }
            }
        }
        if touches {
            count += 1;
        }
    }
    count
}

/// The landed `rewrite::resolve` D2 acceptance predicate mirrored verbatim:
/// the name's endpoint pair matches exactly one unique edge (within the
/// insertion tolerance, either order), that edge is used by exactly two
/// faces, and each endpoint vertex is box-like (three incident faces). A name
/// passing this is one the landed `fillet` accepts without an `Empty` or
/// `NonCanonicalCarrier` refusal on that edge.
fn accepted_by_landed_fillet(solid: &Solid, row: &FilletRow) -> bool {
    let named_edges = unique_edges_with_uses(solid);
    let matches: Vec<&(Edge<Point3, Curve>, usize)> = named_edges
        .iter()
        .filter(|(edge, _)| {
            let (p0, p1) = edge.absolute_ends();
            let near = |x: Point3, y: Point3| (x - y).magnitude() <= INSERTION_TOL;
            (near(row.a, p0.point()) && near(row.b, p1.point()))
                || (near(row.a, p1.point()) && near(row.b, p0.point()))
        })
        .collect();
    if matches.len() != 1 {
        return false;
    }
    let edge = &matches[0].0;
    if matches[0].1 != 2 {
        return false;
    }
    let (va, vb) = edge.absolute_ends();
    faces_through(solid, va.point()) == 3 && faces_through(solid, vb.point()) == 3
}

#[test]
fn edge_resolution_names_blend_targets() {
    // The box [0,4] x [0,4] x [0,2]; its four vertical edges are the columns
    // over (x, y) in {(0,0), (4,0), (0,4), (4,4)}.
    let solid = box_solid([0.0, 0.0, 0.0], [4.0, 4.0, 2.0]);
    let root = EntityId::src(11);

    // Select the vertical edges through the selector layer and resolve each.
    let mut vertical: Vec<(Point3, Point3)> = Vec::new();
    for edge in edges(&solid, root.clone()).into_vec() {
        if let Some(EdgeName::Straight { a, b }) = edge.edge_name() {
            let vertical_edge = (a.x - b.x).abs() <= FACTS_EPS
                && (a.y - b.y).abs() <= FACTS_EPS
                && (a.z - b.z).abs() > FACTS_EPS;
            if vertical_edge {
                vertical.push((a, b));
            }
        }
    }
    assert_eq!(vertical.len(), 4);

    // The four names are exactly the four columns, endpoint order agnostic.
    let mut columns: Vec<(f64, f64)> = vertical
        .iter()
        .map(|(a, b)| {
            assert!((a.z - 0.0).abs() <= FACTS_EPS || (a.z - 2.0).abs() <= FACTS_EPS);
            assert!((b.z - 0.0).abs() <= FACTS_EPS || (b.z - 2.0).abs() <= FACTS_EPS);
            (a.x, a.y)
        })
        .collect();
    columns.sort_by(|x, y| {
        (x.0, x.1)
            .partial_cmp(&(y.0, y.1))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let expected = [
        (0.0f64, 0.0f64),
        (0.0f64, 4.0f64),
        (4.0f64, 0.0f64),
        (4.0f64, 4.0f64),
    ];
    for (got, want) in columns.iter().zip(expected.iter()) {
        assert!((got.0 - want.0).abs() <= FACTS_EPS && (got.1 - want.1).abs() <= FACTS_EPS);
    }

    // Runtime round trip: hand the resolved names (as `FilletSpec`-shaped
    // rows) back to the landed D2 acceptance predicate. Every
    // vertical name resolves to one unique edge used by exactly two faces with
    // box-like endpoints — the exact set of conditions the landed `fillet`'s
    // resolver enforces before it rebuilds the blended solid.
    let rows: Vec<FilletRow> = vertical
        .iter()
        .map(|(a, b)| FilletRow { a: *a, b: *b })
        .collect();
    for row in &rows {
        assert!(
            accepted_by_landed_fillet(&solid, row),
            "the landed fillet resolver must accept the resolved name {row:?}"
        );
    }
    // The four names are pairwise distinct edges (the landed resolver refuses
    // a duplicated spec edge, so a distinct-acceptance set is required).
    for (i, row_a) in rows.iter().enumerate() {
        for row_b in rows.iter().skip(i + 1) {
            let distinct = (row_a.a - row_b.a).magnitude() > FACTS_EPS
                && (row_a.a - row_b.b).magnitude() > FACTS_EPS;
            assert!(
                distinct,
                "resolved vertical names must be pairwise distinct"
            );
        }
    }
}
