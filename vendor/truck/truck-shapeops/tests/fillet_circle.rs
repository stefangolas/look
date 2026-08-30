//! BG-CAD-P12-BLEND — the circular-rim fillet on the rewrite engine (D1/D2/D3):
//! the table 6.4 row "center locus Circle -> Torus" (the constant-frame case),
//! dyadic witnesses only (tests 1-7 of PACKET.md). The primary fixture is the
//! landed cylinder via the boolean_m2 recipe (disk profile R=2 ->
//! `extrude_profile` height 2); the top-rim fillet is the Finding 2 witness
//! THROUGH THE ENTRY.
//!
//! Every assertion is a statement about the `fillet_circle()` rewrite in
//! `truck_shapeops::rewrite`, whose `Solid::try_new` acceptance gate (D6) is
//! exercised directly by the `expect_ok` helper. Test 5 (the P11 ride) imports
//! `truck_evidence::contact` directly (the normal-dependency D-ride).

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
// dyadic witnesses - not such a path.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::HashMap;
use std::collections::HashSet;
use std::f64::consts::{FRAC_PI_2, TAU};
use truck_base::cgmath64::{Matrix4, Point2, Point3, Vector3, Vector4};
use truck_base::contact::{ContactDimension, ContactEventKind};
use truck_base::evidence::{Budget, EnvelopeCase, Method, Outcome, Refusal};
use truck_evidence::contact::{contact, BoundedStratum, ContactLocus};
use truck_geometry::arrange::{arrange, Arrangement};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::prelude::*;
use truck_geometry::recognize::CanonicalSurface;
use truck_modeling::extrude::extrude_profile;
use truck_shapeops::rewrite::{fillet_circle, CircleFilletSpec};
use truck_topology::{Edge, EdgeID, Solid};

/// The certified-point residual: the certification precision achieved on the
/// P11 ride (the probe's 1e-9; unit-scale certified-point residual, not a
/// length).
const RESIDUAL: f64 = 1.0e-9; // H-3: unit-scale certified-point residual, not a length

// ---------------------------------------------------------------------------
// construction helpers (the boolean_m2 fixture recipe)
// ---------------------------------------------------------------------------

/// A placed full-period circle at `center` with radius `r`.
fn placed_circle(center: Point3, r: f64) -> Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4> {
    Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
        Matrix4 {
            x: Vector4::new(r, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, r, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(center.x, center.y, center.z, 1.0),
        },
    )
}

/// A pure-disk profile: one full circle of radius `r` at `center`.
fn disk_profile(center: Point2, r: f64) -> (Vec<Curve>, Arrangement) {
    let circle = Curve::Circle(placed_circle(Point3::new(center.x, center.y, 0.0), r));
    let profile = vec![circle];
    let ok = arrange(&profile, None).unwrap();
    (profile, ok.value)
}

/// The solid `height`-extrude of a profile.
fn extrude_solid(
    profile: &[Curve],
    arr: &Arrangement,
    height: f64,
) -> Solid<Point3, Curve, Surface> {
    extrude_profile(profile, arr, height)
        .expect("the dyadic profile extrudes")
        .value
}

/// The primary fixture: the disk profile R=2 extruded to height 2 (the Finding
/// 1 census — 3 faces, 2 unique edges, 2 unique vertices).
fn cylinder_fixture() -> Solid<Point3, Curve, Surface> {
    let (profile, arr) = disk_profile(Point2::new(0.0, 0.0), 2.0);
    extrude_solid(&profile, &arr, 2.0)
}

/// The top-rim spec: the Finding 2 primary witness.
fn top_rim_spec() -> CircleFilletSpec {
    CircleFilletSpec {
        center: Point3::new(0.0, 0.0, 2.0),
        edge_radius: 2.0,
        radius: 0.5,
    }
}

/// The bottom-rim spec: the Finding 3 witness (the s-rule's other branch).
fn bottom_rim_spec() -> CircleFilletSpec {
    CircleFilletSpec {
        center: Point3::new(0.0, 0.0, 0.0),
        edge_radius: 2.0,
        radius: 0.5,
    }
}

/// The match-based `Ok` unwrapper of the packet (D1).
fn expect_ok<T>(r: Outcome<T>) -> T {
    match r {
        Ok(c) => c.value,
        Err(e) => panic!("unexpected refusal: {e:?}"),
    }
}

/// Runs one fillet_circle with a fresh budget.
fn run_fillet_circle(
    solid: &Solid<Point3, Curve, Surface>,
    specs: &[CircleFilletSpec],
) -> Outcome<Solid<Point3, Curve, Surface>> {
    let mut budget = Budget::new(1000, 1000, 1000);
    fillet_circle(solid, specs, &mut budget)
}

// ---------------------------------------------------------------------------
// measurement helpers
// ---------------------------------------------------------------------------

/// The unique-edge count (per-use iteration deduped by edge id).
fn unique_edges(solid: &Solid<Point3, Curve, Surface>) -> usize {
    solid
        .edge_iter()
        .map(|e| e.id())
        .collect::<HashSet<_>>()
        .len()
}

/// The unique-vertex count (per-use iteration deduped by vertex id).
fn unique_vertices(solid: &Solid<Point3, Curve, Surface>) -> usize {
    solid
        .vertex_iter()
        .map(|v| v.id())
        .collect::<HashSet<_>>()
        .len()
}

/// The torus carriers of the solid's faces.
fn torus_faces(solid: &Solid<Point3, Curve, Surface>) -> Vec<truck_geometry::specifieds::Torus> {
    solid
        .face_iter()
        .filter_map(|face| match face.surface() {
            Surface::Torus(t) => Some(t),
            _ => None,
        })
        .collect()
}

/// The center and radius of a circle-carried edge curve.
fn circle_geometry(edge: &Edge<Point3, Curve>) -> Option<(Point3, f64)> {
    let Curve::Circle(c) = edge.curve() else {
        return None;
    };
    let t = c.transform();
    let center = Point3::new(t.w.x, t.w.y, t.w.z);
    let radius = Vector3::new(t.x.x, t.x.y, t.x.z).magnitude();
    Some((center, radius))
}

/// One distinct circle edge of a solid: its geometry and the surface kinds of
/// the faces that use it (the "shared as instances" check).
struct CircleEdgeUse {
    center: Point3,
    radius: f64,
    face_kinds: Vec<String>,
}

/// The distinct circle edges of a solid, keyed by edge id, with the geometry
/// and the surface kinds of the faces that use them.
fn circle_edge_uses(solid: &Solid<Point3, Curve, Surface>) -> Vec<CircleEdgeUse> {
    let mut order: Vec<EdgeID<Curve>> = Vec::new();
    let mut uses: HashMap<EdgeID<Curve>, CircleEdgeUse> = HashMap::new();
    for face in solid.face_iter() {
        let kind = match face.surface() {
            Surface::Cylinder(_) => "Cylinder",
            Surface::Torus(_) => "Torus",
            Surface::Plane(_) => "Plane",
            _ => "Other",
        };
        for wire in face.absolute_boundaries() {
            for edge in wire.edge_iter() {
                let Some((center, radius)) = circle_geometry(edge) else {
                    continue;
                };
                match uses.get_mut(&edge.id()) {
                    Some(entry) => {
                        if !entry.face_kinds.iter().any(|k| k == kind) {
                            entry.face_kinds.push(kind.to_string());
                        }
                    }
                    None => {
                        order.push(edge.id());
                        uses.insert(
                            edge.id(),
                            CircleEdgeUse {
                                center,
                                radius,
                                face_kinds: vec![kind.to_string()],
                            },
                        );
                    }
                }
            }
        }
    }
    order
        .into_iter()
        .filter_map(|id| uses.remove(&id))
        .collect()
}

/// The first circle edge of the solid whose curve matches `center`/`radius`.
fn find_circle_edge(
    solid: &Solid<Point3, Curve, Surface>,
    center: Point3,
    radius: f64,
) -> Edge<Point3, Curve> {
    solid
        .face_iter()
        .flat_map(|face| face.absolute_boundaries().iter().flatten())
        .find(|edge| circle_geometry(edge).is_some_and(|(c, r)| c == center && r == radius))
        .cloned()
        .expect("the circle edge exists")
}

/// Asserts that a circle edge's sampled points satisfy both carriers: the
/// torus equation `(rhat - R)^2 + (z - cz)^2 = r^2` and the `expected` signed
/// predicate (on the wall: `rhat = R`; on the cap: `z = cap_z`).
fn assert_circle_on_torus_and_carrier(
    edge: &Edge<Point3, Curve>,
    torus: &truck_geometry::specifieds::Torus,
    expected_rhat: Option<f64>,
    expected_z: Option<f64>,
    what: &str,
) {
    let curve = edge.curve();
    for t in [0.0, FRAC_PI_2, FRAC_PI_2 * 2.0, FRAC_PI_2 * 3.0] {
        let p = curve.subs(t);
        let rhat = (p.x * p.x + p.y * p.y).sqrt();
        let torus_res = (rhat - torus.large_radius()) * (rhat - torus.large_radius())
            + (p.z - torus.center().z) * (p.z - torus.center().z)
            - torus.small_radius() * torus.small_radius();
        assert!(
            torus_res.abs() <= RESIDUAL,
            "{what}: sampled point {p:?} must lie on the torus (residual {torus_res})"
        );
        if let Some(er) = expected_rhat {
            assert!(
                (rhat - er).abs() <= RESIDUAL,
                "{what}: sampled point {p:?} must sit at rhat = {er}"
            );
        }
        if let Some(ez) = expected_z {
            assert!(
                (p.z - ez).abs() <= RESIDUAL,
                "{what}: sampled point {p:?} must sit at z = {ez}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Test 1: the Finding 2 witness THROUGH THE ENTRY (the top rim).
// ---------------------------------------------------------------------------

#[test]
fn fillet_circle_top_rim() {
    let solid = cylinder_fixture();
    let result = expect_ok(run_fillet_circle(&solid, &[top_rim_spec()]));
    assert_eq!(result.boundaries().len(), 1);
    // Finding 2 census: 4 faces, 3 unique edges, 3 unique vertices.
    assert_eq!(result.face_iter().count(), 4);
    assert_eq!(unique_edges(&result), 3);
    assert_eq!(unique_vertices(&result), 3);
    // Exactly one torus face: center (0,0,1.5), major 1.5, minor 0.5.
    let tori = torus_faces(&result);
    assert_eq!(tori.len(), 1, "exactly one torus face");
    assert_eq!(tori[0].center(), Point3::new(0.0, 0.0, 1.5));
    assert_eq!(tori[0].large_radius(), 1.5);
    assert_eq!(tori[0].small_radius(), 0.5);
    // The junction circles exact and shared as instances: r=2@z=1.5 shared by
    // the wall (Cylinder) and the torus; r=1.5@z=2 shared by the torus and the
    // top cap (Plane).
    let circles = circle_edge_uses(&result);
    assert_eq!(circles.len(), 3, "three distinct circle edges");
    let wall_junction = circles
        .iter()
        .find(|c| c.center == Point3::new(0.0, 0.0, 1.5) && c.radius == 2.0)
        .expect("the wall junction circle");
    assert_eq!(wall_junction.face_kinds.len(), 2);
    assert!(wall_junction.face_kinds.iter().any(|k| k == "Cylinder"));
    assert!(wall_junction.face_kinds.iter().any(|k| k == "Torus"));
    let cap_junction = circles
        .iter()
        .find(|c| c.center == Point3::new(0.0, 0.0, 2.0) && c.radius == 1.5)
        .expect("the cap junction circle");
    assert_eq!(cap_junction.face_kinds.len(), 2);
    assert!(cap_junction.face_kinds.iter().any(|k| k == "Torus"));
    assert!(cap_junction.face_kinds.iter().any(|k| k == "Plane"));
    // The bottom cap untouched: its rim circle r=2@z=0 survives, shared by the
    // bottom cap (Plane) and the wall (Cylinder).
    let bottom = circles
        .iter()
        .find(|c| c.center == Point3::new(0.0, 0.0, 0.0) && c.radius == 2.0)
        .expect("the bottom rim circle survives");
    assert_eq!(bottom.face_kinds.len(), 2);
    assert!(bottom.face_kinds.iter().any(|k| k == "Plane"));
    assert!(bottom.face_kinds.iter().any(|k| k == "Cylinder"));
}

// ---------------------------------------------------------------------------
// Test 2: the Finding 3 witness (the bottom rim).
// ---------------------------------------------------------------------------

#[test]
fn fillet_circle_bottom_rim() {
    let solid = cylinder_fixture();
    let result = expect_ok(run_fillet_circle(&solid, &[bottom_rim_spec()]));
    assert_eq!(result.face_iter().count(), 4);
    assert_eq!(unique_edges(&result), 3);
    assert_eq!(unique_vertices(&result), 3);
    let tori = torus_faces(&result);
    assert_eq!(tori.len(), 1, "exactly one torus face");
    assert_eq!(tori[0].center(), Point3::new(0.0, 0.0, 0.5));
    assert_eq!(tori[0].large_radius(), 1.5);
    assert_eq!(tori[0].small_radius(), 0.5);
}

// ---------------------------------------------------------------------------
// Test 3: the Finding 4 witness (both rims in ONE entry call).
// ---------------------------------------------------------------------------

#[test]
fn fillet_circle_both_rims() {
    let solid = cylinder_fixture();
    let result = expect_ok(run_fillet_circle(
        &solid,
        &[top_rim_spec(), bottom_rim_spec()],
    ));
    // Finding 4 census: 5 faces, 4 unique edges, 4 unique vertices.
    assert_eq!(result.face_iter().count(), 5);
    assert_eq!(unique_edges(&result), 4);
    assert_eq!(unique_vertices(&result), 4);
    let tori = torus_faces(&result);
    assert_eq!(tori.len(), 2, "two tori");
    let centers: Vec<Point3> = tori.iter().map(|t| t.center()).collect();
    assert!(centers.contains(&Point3::new(0.0, 0.0, 1.5)));
    assert!(centers.contains(&Point3::new(0.0, 0.0, 0.5)));
}

// ---------------------------------------------------------------------------
// Test 4: the D6 certificate on the test-1 result (carrier-derived, never
// vertex-bbox).
// ---------------------------------------------------------------------------

#[test]
fn fillet_circle_junction_certificate() {
    let solid = cylinder_fixture();
    let result = expect_ok(run_fillet_circle(&solid, &[top_rim_spec()]));
    let tori = torus_faces(&result);
    assert_eq!(tori.len(), 1);
    let torus = tori[0];
    assert_eq!(torus.center(), Point3::new(0.0, 0.0, 1.5));
    assert_eq!(torus.large_radius(), 1.5);
    assert_eq!(torus.small_radius(), 0.5);
    // The wall junction circle: sampled points on the torus AND on the wall
    // carrier (rhat = R = 2).
    let wall_junction = find_circle_edge(&result, Point3::new(0.0, 0.0, 1.5), 2.0);
    assert_circle_on_torus_and_carrier(&wall_junction, &torus, Some(2.0), None, "wall junction");
    // The cap junction circle: sampled points on the torus AND on the cap
    // (z = cap_z = 2, rhat = R - r = 1.5).
    let cap_junction = find_circle_edge(&result, Point3::new(0.0, 0.0, 2.0), 1.5);
    assert_circle_on_torus_and_carrier(&cap_junction, &torus, Some(1.5), Some(2.0), "cap junction");
    // The cap junction circle's curve center/radius exact.
    let (cc, cr) = circle_geometry(&cap_junction).expect("a circle curve");
    assert_eq!(cc, Point3::new(0.0, 0.0, 2.0));
    assert_eq!(cr, 1.5);
}

// ---------------------------------------------------------------------------
// Test 5: the P11 ride — the realized torus face through the landed
// dispatcher (Finding 6).
// ---------------------------------------------------------------------------

#[test]
fn fillet_circle_torus_face_rides_p11_pairs() {
    let solid = cylinder_fixture();
    let result = expect_ok(run_fillet_circle(&solid, &[top_rim_spec()]));
    // Lift the RESULT's torus face as the torus-aware validated-FF witness.
    let torus_face = result
        .face_iter()
        .find(|face| matches!(face.surface(), Surface::Torus(_)))
        .expect("the fillet torus face");
    let Surface::Torus(torus) = torus_face.surface() else {
        unreachable!("the found face is a torus");
    };
    let torus_patch = BoundedStratum::Face {
        surface: CanonicalSurface::Torus(torus),
        u_range: (0.0, TAU),
        v_range: (0.0, FRAC_PI_2),
    };
    // The band-clear plane at z = 1.75 (torus-local z = +0.25 = r/2).
    let plane_patch = BoundedStratum::Face {
        surface: CanonicalSurface::Plane(Plane::new(
            Point3::new(-3.0, -3.0, 1.75),
            Point3::new(3.0, -3.0, 1.75),
            Point3::new(-3.0, 3.0, 1.75),
        )),
        u_range: (0.0, 1.0),
        v_range: (0.0, 1.0),
    };
    let mut budget = Budget::new(20000, 0, 0);
    let out = contact(&torus_patch, &plane_patch, &mut budget)
        .expect("the torus face certifies the band plane under healthy budget");
    assert_eq!(out.cert.method, Method::Interval);
    assert_eq!(out.value.contacts.len(), 1);
    let record = out.value.contacts.first().expect("one contact record");
    assert_eq!(record.dimension, ContactDimension::Arc1);
    assert_eq!(record.kind, ContactEventKind::Transverse);
    let ContactLocus::ValidatedBranchCover(cover) = &record.locus else {
        panic!("the ride emits a validated branch cover");
    };
    assert!(!cover.points.is_empty(), "the cover certifies crossings");
    // Every certified point sits at z = 1.75 exactly on the torus, at one of
    // the two closed-form radii rhat = 1.5 +- sqrt(0.1875), and both branches
    // are present (the Finding 6 witness).
    let (outer, inner) = (1.5 + 0.1875f64.sqrt(), 1.5 - 0.1875f64.sqrt());
    let (mut saw_outer, mut saw_inner) = (false, false);
    for p in &cover.points {
        let rhat = (p.x * p.x + p.y * p.y).sqrt();
        let torus_res = (rhat - 1.5) * (rhat - 1.5) + (p.z - 1.5) * (p.z - 1.5) - 0.25;
        assert!(
            (p.z - 1.75).abs() <= RESIDUAL,
            "point {p:?} must sit at z = 1.75 exactly"
        );
        assert!(
            torus_res.abs() <= RESIDUAL,
            "point {p:?} must lie on the torus exactly (residual {torus_res})"
        );
        assert!(
            (rhat - outer).abs() <= RESIDUAL || (rhat - inner).abs() <= RESIDUAL,
            "point {p:?} must sit at a closed-form radius, rhat = {rhat}"
        );
        saw_outer |= (rhat - outer).abs() <= RESIDUAL;
        saw_inner |= (rhat - inner).abs() <= RESIDUAL;
    }
    assert!(saw_outer, "the outer branch is present");
    assert!(saw_inner, "the inner branch is present");
}

// ---------------------------------------------------------------------------
// Test 6: the Finding 5 multi-wire cap refusal (budget untouched).
// ---------------------------------------------------------------------------

#[test]
fn fillet_circle_multiwire_cap_refuses() {
    // The washer: the 4x4 square plus a hole circle r=1 at the origin, extruded
    // height 1 (the plate-with-hole recipe shape; the hole rides at the origin
    // so the rim's circle center matches the packet's spec center (0,0,1) — see
    // RESULT.json notes). The top face is a TWO-wire plane (outer square + hole
    // self-loop circle); filleting the hole's rim has that 2-wire face as the
    // cap, so the D2 neighborhood check refuses.
    let mut profile = vec![
        Curve::Line(Line(
            Point3::new(-4.0, -4.0, 0.0),
            Point3::new(4.0, -4.0, 0.0),
        )),
        Curve::Line(Line(
            Point3::new(4.0, -4.0, 0.0),
            Point3::new(4.0, 4.0, 0.0),
        )),
        Curve::Line(Line(
            Point3::new(4.0, 4.0, 0.0),
            Point3::new(-4.0, 4.0, 0.0),
        )),
        Curve::Line(Line(
            Point3::new(-4.0, 4.0, 0.0),
            Point3::new(-4.0, -4.0, 0.0),
        )),
    ];
    profile.push(Curve::Circle(placed_circle(
        Point3::new(0.0, 0.0, 0.0),
        1.0,
    )));
    let ok = arrange(&profile, None).unwrap();
    let washer = extrude_solid(&profile, &ok.value, 1.0);
    assert_eq!(washer.face_iter().count(), 7, "the Finding 5 washer census");
    let spec = CircleFilletSpec {
        center: Point3::new(0.0, 0.0, 1.0),
        edge_radius: 1.0,
        radius: 0.25,
    };
    let mut budget = Budget::new(1000, 1000, 1000);
    let before = budget;
    let result = fillet_circle(&washer, &[spec], &mut budget);
    assert!(
        matches!(
            result,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier
            ))
        ),
        "the multi-wire cap must refuse the envelope at the D2 lift"
    );
    assert_eq!(
        budget, before,
        "the D2 lift refuses before any certified work"
    );
}

// ---------------------------------------------------------------------------
// Test 7: the D3 overflow refusal (radius 2 consumes the whole wall AND
// collapses the cap).
// ---------------------------------------------------------------------------

#[test]
fn fillet_circle_overflow_refuses() {
    let solid = cylinder_fixture();
    let spec = CircleFilletSpec {
        center: Point3::new(0.0, 0.0, 2.0),
        edge_radius: 2.0,
        radius: 2.0,
    };
    let mut budget = Budget::new(1000, 1000, 1000);
    let result = fillet_circle(&solid, &[spec], &mut budget);
    // r = 2 >= |z_other - cap_z| = 2 (the wall would vanish) AND r >= R = 2
    // (the cap would collapse): the D3 overflow refuses Empty before minting.
    assert!(
        matches!(result, Err(Refusal::Empty)),
        "overflow refuses Empty"
    );
}
