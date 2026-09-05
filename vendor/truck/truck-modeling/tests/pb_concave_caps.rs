#![deny(clippy::unwrap_used)]

//! PB-003-CONCAVE-CAPS — the concave-cap layer of the facet backend: the
//! deterministic cap triangulation of non-convex facet rings (ear clipping
//! over the ring projected onto its carrier plane), the bit-identical convex
//! fast path, and the typed refusals that stay typed.

use truck_base::evidence::{EnvelopeCase, Refusal};
use truck_geometry::base::*;
use truck_geometry::constructive::*;
use truck_modeling::facet_sweep::{
    facet_sweep, facet_sweep_certified, FacetSweepResult, FacetVerdict,
};
use truck_polymesh::*;

/// The unit-square profile, CCW in the frame plane.
fn unit_square() -> Profile2D {
    Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ],
    }
}

/// The 2x-scaled square (the tapered pair's end profile).
fn larger_square() -> Profile2D {
    Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 2.0),
            Point2::new(0.0, 2.0),
        ],
    }
}

/// The concave L-shaped profile (6 vertices): the non-convex cap fixture.
fn l_shape() -> Profile2D {
    Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 1.0),
            Point2::new(1.0, 1.0),
            Point2::new(1.0, 2.0),
            Point2::new(0.0, 2.0),
        ],
    }
}

/// A self-intersecting figure-eight ring (a bowtie): edges 0 and 2 cross at
/// their center — deliberately non-simple.
fn figure_eight() -> Profile2D {
    Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
            Point2::new(1.0, 0.0),
        ],
    }
}

/// A regular `n`-gon of the given radius, CCW.
fn regular_polygon(n: usize, radius: f64) -> Profile2D {
    let vertices = (0..n)
        .map(|i| {
            let t = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
            Point2::new(radius * t.cos(), radius * t.sin())
        })
        .collect();
    Profile2D { vertices }
}

/// A straight spine of length 2 along +X.
fn straight_spine() -> LineSpine {
    LineSpine {
        start: Point3::new(0.0, 0.0, 0.0),
        end: Point3::new(2.0, 0.0, 0.0),
    }
}

/// The +Z FixedPlane frame law.
fn fixed_plane_z() -> FrameLaw {
    FrameLaw::FixedPlane {
        normal: Vector3::unit_z(),
    }
}

/// Resolves `n` uniform stations over `[0, 1]`.
fn uniform_stations(n: usize) -> Vec<f64> {
    match (SamplingPolicy::UniformCount { spine: n }).resolve(0.0, 1.0) {
        Ok(stations) => stations,
        Err(_) => vec![f64::NAN],
    }
}

/// Sweeps and extracts the result; any refusal fails the test loudly.
fn swept(
    recipe: &SpineFrameRecipe<LineSpine, ProfileLaw, FrameLaw>,
    stations: &[f64],
    ring: usize,
) -> FacetSweepResult {
    match facet_sweep(recipe, stations, ring) {
        Ok(result) => result,
        Err(error) => panic!("facet_sweep refused (ring = {ring}): {error:?}"),
    }
}

/// Independent winding recomputation (no HashMap): every undirected edge must
/// appear exactly twice with opposite effective directions.
fn independent_winding_violations(mesh: &PolygonMesh) -> usize {
    let mut edges: Vec<(usize, usize, i32)> = Vec::new();
    for face in mesh.faces().face_iter() {
        let n = face.len();
        for e in 0..n {
            let u = face[e].pos;
            let v = face[(e + 1) % n].pos;
            let (lo, hi) = if u < v { (u, v) } else { (v, u) };
            let direction = if u < v { 1 } else { -1 };
            edges.push((lo, hi, direction));
        }
    }
    edges.sort_unstable();
    let mut violations = 0usize;
    let mut at = 0usize;
    while at < edges.len() {
        let (lo, hi, _) = edges[at];
        let count = edges[at..]
            .iter()
            .take_while(|&&(l, h, _)| l == lo && h == hi)
            .count();
        let direction_sum: i32 = edges[at..at + count].iter().map(|e| e.2).sum();
        if count != 2 || direction_sum != 0 {
            violations += 1;
        }
        at += count;
    }
    violations
}

/// The doubled area of one triangle (the magnitude of its cross product).
fn triangle_double_area(positions: &[Point3], tri: [usize; 3]) -> f64 {
    let a = positions[tri[0]];
    let b = positions[tri[1]];
    let c = positions[tri[2]];
    (b - a).cross(c - a).magnitude()
}

/// Asserts that every ring edge `(j, j + 1 mod k)` of a cap group whose ring
/// ordinals are offset by `base` is covered by exactly ONE triangle of
/// `triangles` — the ring boundary is shared once with the side band, never
/// covered twice (overlap) and never left open (a gap).
fn assert_ring_edges_covered_once(triangles: &[[usize; 3]], base: usize, k: usize) {
    let mut counts = vec![0usize; k];
    for tri in triangles {
        for (u, w) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            let lo = u.min(w);
            let hi = u.max(w);
            if hi - lo == 1 {
                counts[lo - base] += 1;
            } else if lo == base && hi == base + k - 1 {
                counts[k - 1] += 1;
            }
        }
    }
    for (j, &count) in counts.iter().enumerate() {
        assert_eq!(
            count, 1,
            "ring edge {j} must be covered by exactly one cap triangle"
        );
    }
}

/// Binary STL bytes for the mesh: 80-byte header, u32 triangle count, then per
/// fan triangle the computed normal and three vertex positions as f32s. This
/// is the byte stream the V5 byte-identity guard hashes.
fn stl_binary_bytes(mesh: &PolygonMesh) -> Vec<u8> {
    let positions = mesh.positions();
    let mut triangles: Vec<(Point3, Point3, Point3)> = Vec::new();
    for face in mesh.face_iter() {
        let n = face.len();
        for e in 1..n.saturating_sub(1) {
            triangles.push((
                positions[face[0].pos],
                positions[face[e].pos],
                positions[face[e + 1].pos],
            ));
        }
    }
    let mut out: Vec<u8> = vec![0; 80];
    out.extend((triangles.len() as u32).to_le_bytes());
    for (a, b, c) in triangles {
        let normal = (b - a).cross(c - a);
        let normal = if normal.magnitude() > 0.0 {
            normal.normalize()
        } else {
            Vector3::zero()
        };
        for v in [normal.x, normal.y, normal.z] {
            out.extend((v as f32).to_le_bytes());
        }
        for p in [a, b, c] {
            for v in [p.x, p.y, p.z] {
                out.extend((v as f32).to_le_bytes());
            }
        }
        out.extend(0u16.to_le_bytes());
    }
    out
}

/// FNV-1a 64 over the STL bytes.
fn stl_hash(mesh: &PolygonMesh) -> u64 {
    let bytes = stl_binary_bytes(mesh);
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

#[test]
fn concave_ring_caps_through_cdt() {
    // The L-shaped (non-convex) profile sweeps and caps correctly: the cap
    // triangulation tiles the ring without overlap, the solid is closed with
    // positive volume. Ground truth: the L has area 3 (a 2x2 square minus a
    // 1x1 notch) and the sweep is 2 long, so the prism volume is 6.
    let recipe = SpineFrameRecipe::new(
        straight_spine(),
        ProfileLaw::Constant(l_shape()),
        fixed_plane_z(),
    );
    let stations = uniform_stations(5);
    let result = swept(&recipe, &stations, 6);
    let mesh = &result.mesh;
    let k = 6;
    let m = stations.len();
    assert_eq!(mesh.positions().len(), m * k);

    // The constant straight sweep emits every side cell as one planar quad,
    // so every mesh triangle is a cap triangle: 2 * (k - 2) of them.
    assert_eq!(result.audit.quad_count, (m - 1) * k);
    assert_eq!(result.audit.triangle_count, 0);
    assert_eq!(mesh.tri_faces().len(), 2 * (k - 2));
    assert_eq!(mesh.quad_faces().len(), (m - 1) * k);

    // The solid is closed with a clean winding audit (independent recompute
    // plus the backend audit) and the mesh-level verdict certifies it.
    assert_eq!(independent_winding_violations(mesh), 0);
    assert_eq!(result.audit.winding_violations, 0);
    assert_eq!(result.verdict, FacetVerdict::CertifiedWithinTolerance);

    // Positive volume with the analytic ground truth (H-3): 3 * 2 = 6.
    assert!(
        (result.audit.signed_volume - 6.0).abs() <= TOLERANCE * 64.0,
        "the L prism must have volume 6, got {}",
        result.audit.signed_volume
    );

    // The cap triangles cover the ring without overlap: each cap's triangle
    // double-area sums to the ring's double area (2 * 3 = 6), and every ring
    // boundary edge is covered by exactly one cap triangle.
    let tris = mesh.tri_faces();
    let positions = mesh.positions();
    let ring_double_area = 6.0;
    let start_cap_double_area: f64 = tris[..k - 2]
        .iter()
        .map(|t| triangle_double_area(positions, [t[0].pos, t[1].pos, t[2].pos]))
        .sum();
    assert!(
        (start_cap_double_area - ring_double_area).abs() <= TOLERANCE * 64.0,
        "the start cap must tile the ring exactly (double area 6), got {start_cap_double_area}"
    );
    let r0 = (m - 1) * k;
    let end_cap_double_area: f64 = tris[k - 2..]
        .iter()
        .map(|t| triangle_double_area(positions, [t[0].pos, t[1].pos, t[2].pos]))
        .sum();
    assert!(
        (end_cap_double_area - ring_double_area).abs() <= TOLERANCE * 64.0,
        "the end cap must tile the ring exactly (double area 6), got {end_cap_double_area}"
    );
    let start_cap_indices: Vec<[usize; 3]> = tris[..k - 2]
        .iter()
        .map(|t| [t[0].pos, t[1].pos, t[2].pos])
        .collect();
    let end_cap_indices: Vec<[usize; 3]> = tris[k - 2..]
        .iter()
        .map(|t| [t[0].pos, t[1].pos, t[2].pos])
        .collect();
    assert_ring_edges_covered_once(&start_cap_indices, 0, k);
    assert_ring_edges_covered_once(&end_cap_indices, r0, k);
}

#[test]
fn convex_fast_path_bit_identical() {
    // The V5 guard: every convex fixture's cap triangulation is byte-identical
    // before/after PB-003. The hashes below were measured from the pre-change
    // build (2026-09-05) — the apex fan is the fast path and a convex ring
    // never reaches the ear-clip path, so its STL bytes must not move.
    let cases: [(&str, ProfileLaw, usize, usize, u64); 4] = [
        (
            "unit square, 5 stations, ring 4",
            ProfileLaw::Constant(unit_square()),
            5,
            4,
            0xb4f3_89cd_04b5_b231,
        ),
        (
            "unit square to 2x square, 5 stations, ring 4",
            ProfileLaw::try_linear_correspondence(unit_square(), larger_square())
                .unwrap_or(ProfileLaw::Constant(unit_square())),
            5,
            4,
            0xd3d3_ca66_c4ff_b509,
        ),
        (
            "regular pentagon, 4 stations, ring 5",
            ProfileLaw::Constant(regular_polygon(5, 1.0)),
            4,
            5,
            0x9591_8c41_e247_3575,
        ),
        (
            "regular hexagon, 3 stations, ring 6",
            ProfileLaw::Constant(regular_polygon(6, 1.0)),
            3,
            6,
            0x79ca_783b_65f2_80d9,
        ),
    ];
    for (name, profile, station_count, ring, expected) in cases {
        let recipe = SpineFrameRecipe::new(straight_spine(), profile, fixed_plane_z());
        let stations = uniform_stations(station_count);
        let result = swept(&recipe, &stations, ring);
        let actual = stl_hash(&result.mesh);
        assert_eq!(
            actual, expected,
            "the convex fixture '{name}' must keep its pre-PB-003 STL bytes (V5)"
        );
    }
}

#[test]
fn concave_ring_refusals_stay_typed() {
    // A self-intersecting (figure-eight) ring still refuses typed: the ring's
    // simplicity check precedes any triangulation. No repair, no tolerance
    // games — the same `ConstructError::InvalidInput` the convex gate raised
    // before the concave-cap layer landed.
    let recipe = SpineFrameRecipe::new(
        straight_spine(),
        ProfileLaw::Constant(figure_eight()),
        fixed_plane_z(),
    );
    let stations = uniform_stations(3);
    assert!(matches!(
        facet_sweep(&recipe, &stations, 4),
        Err(ConstructError::InvalidInput)
    ));
    assert!(matches!(
        facet_sweep_certified(&recipe, &stations, 4),
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused))
    ));
}
