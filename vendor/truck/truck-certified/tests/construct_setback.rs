//! CC-033-SETBACK integration tests: the n-valent corner setback patch
//! certified on its four counts over real v1 data — boundary equality to the
//! prescribed profile/spring arcs, the G¹ ribbons with strictly positive λ,
//! the whole-patch regularity margin, and the global embeddedness count whose
//! projection search exhausts onto the pairwise fallback. The test names are
//! the contract.
//!
//! Routing note (STOP CONDITION 2): a GENUINE n = 3 triple corner routes
//! through CC-020's `solve_triple_node`, not a setback patch — setback serves
//! n ≥ 4 (and degenerate triples that no triple node serves). Every fixture
//! below is n = 4 (8 arcs), and `SetbackInput::try_new` refuses fewer arcs, so
//! the n = 3 routing is kept OUT of this module by construction.

#![deny(clippy::unwrap_used)]

use truck_certified::construct::config::CC_ETA_J;
use truck_certified::construct::graphdisk::GraphDiskCert;
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::setback::{
    build_setback_patch, certify_boundary, certify_embeddedness, certify_g1_ribbons,
    certify_regularity, pairwise_embeddedness, ArcKind, EmbeddedVerdict, SetbackArc, SetbackInput,
    G1_PLANE_TOL,
};

/// The chamfered-square octagon of the convex fixtures (CCW, dyadic): the rim
/// corners at `z = 0`.
fn convex_rim() -> [[f64; 3]; 8] {
    [
        [-1.0, -0.5, 0.0],
        [-0.5, -1.0, 0.0],
        [0.5, -1.0, 0.0],
        [1.0, -0.5, 0.0],
        [1.0, 0.5, 0.0],
        [0.5, 1.0, 0.0],
        [-0.5, 1.0, 0.0],
        [-1.0, 0.5, 0.0],
    ]
}

/// The octagram `{8/3}` rim of the refusal fixture: a self-crossing chord loop
/// (its non-adjacent vertical walls cross), still a structurally valid closed
/// chain.
fn star_rim() -> [[f64; 3]; 8] {
    let angle = |k: usize| -> [f64; 3] {
        let theta = (k as f64) * 135.0_f64.to_radians();
        [theta.cos(), theta.sin(), 0.0]
    };
    [
        angle(0),
        angle(1),
        angle(2),
        angle(3),
        angle(4),
        angle(5),
        angle(6),
        angle(7),
    ]
}

/// The straight chord arc control points between two rim corners (endpoints
/// exactly `p` and `q`).
fn chord(p: [f64; 3], q: [f64; 3]) -> [[f64; 3]; 4] {
    let mut points = [[0.0_f64; 3]; 4];
    points[0] = p;
    points[3] = q;
    for j in 1..3 {
        let t = (j as f64) / 3.0;
        points[j] = [
            p[0] + t * (q[0] - p[0]),
            p[1] + t * (q[1] - p[1]),
            p[2] + t * (q[2] - p[2]),
        ];
    }
    points
}

/// The degree-3 coefficient field that is the linear blend of two endpoint
/// values (endpoints exactly `a` and `b`).
fn linear_field(a: [f64; 3], b: [f64; 3]) -> [[f64; 3]; 4] {
    let mut field = [[0.0_f64; 3]; 4];
    field[0] = a;
    field[3] = b;
    for j in 1..3 {
        let t = (j as f64) / 3.0;
        field[j] = [
            a[0] + t * (b[0] - a[0]),
            a[1] + t * (b[1] - a[1]),
            a[2] + t * (b[2] - a[2]),
        ];
    }
    field
}

/// The unit vector of a direction.
fn unit(v: [f64; 3]) -> [f64; 3] {
    let norm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    assert!(
        norm.is_finite() && norm > 0.0, // H-3: fixture directions are nonzero unit-scale
        "the fixture direction is nonzero"
    );
    [v[0] / norm, v[1] / norm, v[2] / norm]
}

/// The cross product.
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Build the 2n-sided setback input over a rim whose hub vertices are
/// `hub[k]`: the rim corner `k` advanced to its hub. The arcs alternate
/// `Profile`/`Spring`, the cross field is the linear blend of the two corner
/// advances, and the adjacent plane normal is the unit normal of the chord
/// with the first advance.
fn setback_input(rim: &[[f64; 3]], hub: &[[f64; 3]]) -> SetbackInput {
    let count = rim.len();
    assert!(count >= 8 && count % 2 == 0);
    let mut advances = Vec::with_capacity(count);
    for k in 0..count {
        advances.push([
            hub[k][0] - rim[k][0],
            hub[k][1] - rim[k][1],
            hub[k][2] - rim[k][2],
        ]);
    }
    let mut arcs = Vec::with_capacity(count);
    for i in 0..count {
        let j = (i + 1) % count;
        let p = rim[i];
        let q = rim[j];
        let curve = chord(p, q);
        let cross_field = linear_field(advances[i], advances[j]);
        let chord_dir = unit([q[0] - p[0], q[1] - p[1], q[2] - p[2]]);
        let normal = unit(cross(chord_dir, advances[i]));
        arcs.push(SetbackArc {
            kind: if i % 2 == 0 {
                ArcKind::Profile
            } else {
                ArcKind::Spring
            },
            curve,
            normal,
            cross: cross_field,
        });
    }
    SetbackInput::try_new(arcs).expect("the fixture input is closed v1 data")
}

/// The plateau fixture: the convex octagon rim with a homothetic raised hub
/// (`s` contraction about the centre, height `z0`) — a graphable cap over the
/// rim, the primary graph-disk path of COUNT 4.
fn plateau_input(scale: f64, height: f64) -> SetbackInput {
    let rim = convex_rim();
    let mut hub = [[0.0_f64; 3]; 8];
    for k in 0..8 {
        hub[k] = [scale * rim[k][0], scale * rim[k][1], height];
    }
    setback_input(&rim, &hub)
}

/// The bucket fixture: the convex octagon rim with the hub straight above the
/// rim (`scale = 1`, vertical seam spokes) — the walls stand vertical, so no
/// single projection certifies and COUNT 4 exhausts onto the PAIRWISE
/// fallback.
fn bucket_input(height: f64) -> SetbackInput {
    plateau_input(1.0, height)
}

/// Build and certify a valid fixture, panicking on an unexpected refusal.
fn built(input: &SetbackInput) -> truck_certified::construct::setback::SetbackPatch {
    build_setback_patch(input).expect("the certified construction succeeds on closed v1 data")
}

#[test]
fn boundary_matches_prescribed_profiles_and_springs() {
    // COUNT 1. The plateau corner (n = 4): eight arcs alternate Profile (the
    // setback cuts across the incoming fillet flanks) and Spring (the
    // surviving primary faces). The built patch's outer ribbon boundaries must
    // equal the prescribed arcs up to the delivered enclosure ε.
    let input = plateau_input(0.35, 0.1);
    let patch = built(&input);

    // Routing: 8 arcs is n = 4; a genuine n = 3 corner routes through CC-020's
    // triple node and is refused here (fewer than 2 * SETBACK_MIN_VALENCE
    // arcs). A 6-arc loop cannot even be expressed as a valid setback input.
    assert_eq!(patch.arc_count(), 8, "n = 4 carries eight boundary arcs");
    assert!(patch.epsilon >= 0.0); // H-3: epsilon is a certified width, non-negative by construction

    let records = certify_boundary(&patch).expect("the boundary count certifies");
    assert_eq!(records.len(), 8, "one boundary record per boundary arc");
    for record in &records {
        assert!(record.arc < 8, "the record names its prescribed arc");
        assert!(
            record.max_deviation <= patch.epsilon,
            "the outer boundary equals the prescribed arc up to ε: {:?}",
            record
        );
        assert_eq!(
            record.max_deviation, 0.0,
            "the v=0 control row is the arc control points bitwise"
        ); // H-3: bitwise control copy
    }
    // The arcs alternate Profile and Spring around the closed loop.
    for i in 0..patch.input.arcs.len() {
        let next = (i + 1) % patch.input.arcs.len();
        assert_ne!(
            patch.input.arcs[i].kind, patch.input.arcs[next].kind,
            "the boundary arcs alternate Profile and Spring"
        );
    }
}

#[test]
fn g1_ribbon_conditions_hold_with_positive_lambda() {
    // COUNT 2. On every boundary the ribbon's cross tangent equals
    // λ(u)·d(u) with the certified λ enclosure STRICTLY positive and d in the
    // adjacent tangent plane: the plane gap of P_v(u,0) against the delivered
    // normal stays inside G1_PLANE_TOL, and the certified λ lower bound is
    // strictly positive (fold-back prevention is part of this certificate).
    let input = plateau_input(0.35, 0.1);
    let patch = built(&input);
    let records = certify_g1_ribbons(&patch).expect("the G¹ ribbons count certifies");
    assert_eq!(records.len(), 8, "one G¹ record per boundary arc");
    for record in &records {
        assert!(
            record.lambda.lo > 0.0, // H-3: certified λ is strictly positive
            "the certified λ enclosure is strictly positive: {:?}",
            record.lambda
        );
        assert!(record.lambda.is_finite());
        assert!(
            record.plane_gap.lo >= -G1_PLANE_TOL && record.plane_gap.hi <= G1_PLANE_TOL,
            "the cross tangent stays in the adjacent tangent plane: {:?}",
            record.plane_gap
        );
    }
}

#[test]
fn regularity_margin_holds_on_the_whole_patch() {
    // COUNT 3. The certified lower bound of inf ‖P_u × P_v‖ over the WHOLE
    // patch (every ribbon and every hub quad) via the CC-002 hull path on the
    // pieces' Bézier form stays above CC_ETA_J.
    let input = plateau_input(0.35, 0.1);
    let patch = built(&input);
    assert_eq!(
        patch.pieces.len(),
        12,
        "eight ribbons plus four hub quads tile the 2n-sided region"
    );
    let cert = certify_regularity(&patch).expect("the regularity count certifies");
    assert_eq!(cert.per_piece.len(), 12);
    assert!(
        cert.margin_lower > CC_ETA_J,
        "the whole-patch margin floor holds: {}",
        cert.margin_lower
    );
    for margin in &cert.per_piece {
        assert!(*margin > CC_ETA_J, "every piece is regular: {}", margin);
    }
}

#[test]
fn projection_exhaustion_falls_back_and_still_certifies_or_refuses() {
    // COUNT 4. The plateau is a genuine graph over its rim plane: the
    // normative projection search finds a projection and the graph-disk
    // decider certifies. The BUCKET (walls vertical, cap horizontal) admits no
    // single projection — the search EXHAUSTS with NoAdmissibleProjection and
    // the PAIRWISE fallback still certifies the union. A self-crossing star
    // bucket has non-adjacent walls that cross: the fallback refuses
    // NoAdmissibleProjection instead of guessing.

    // Primary path: the graph-disk certificate wins.
    let plateau = built(&plateau_input(0.35, 0.1));
    let embedded = certify_embeddedness(&plateau).expect("the plateau is embedded");
    match embedded.verdict {
        EmbeddedVerdict::GraphDisk { w, cert } => {
            assert!(w.iter().all(|c| c.is_finite()));
            let _: &GraphDiskCert = &cert;
            assert_eq!(cert.pieces.len(), 12, "one disk piece per region piece");
        }
        EmbeddedVerdict::Pairwise => panic!("the plateau is graphable; the search must win"),
    }
    assert!(embedded.margins.iter().all(|m| *m > CC_ETA_J));

    // Fallback-certify: the projection search exhausts, the pairwise discharge
    // certifies.
    let bucket = built(&bucket_input(0.5));
    let embedded = certify_embeddedness(&bucket).expect("the bucket is embedded");
    match embedded.verdict {
        EmbeddedVerdict::Pairwise => {}
        EmbeddedVerdict::GraphDisk { .. } => {
            panic!("the vertical bucket admits no single projection; the search must exhaust")
        }
    }

    // The pairwise discharge itself certifies the bucket (exercised directly).
    pairwise_embeddedness(&bucket).expect("the pairwise fallback certifies the bucket");

    // Fallback-refuse: a self-crossing star bucket has crossing non-adjacent
    // walls; the fallback refuses NoAdmissibleProjection rather than guessing.
    let star = star_rim();
    let mut star_hub = [[0.0_f64; 3]; 8];
    for k in 0..8 {
        star_hub[k] = [star[k][0], star[k][1], 0.5];
    }
    let star_bucket = built(&setback_input(&star, &star_hub));
    match certify_embeddedness(&star_bucket) {
        Err(ConstructRefusal::NoAdmissibleProjection) => {}
        Err(other) => {
            panic!("a crossing setback must refuse NoAdmissibleProjection, got {other:?}")
        }
        Ok(_) => panic!("a crossing setback must refuse, never certify"),
    }
}
