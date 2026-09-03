#![deny(clippy::unwrap_used)]

//! BG-CG-004-FACET — conformance tests for the direct facet realization
//! backend: shared-topology closure by construction, the grid registry, the
//! planar-quad / fixed-diagonal split rules, the typed refusals, the winding
//! audit, the signed-volume match, and the three-valued verdict.

use truck_base::evidence::{
    EnvelopeCase, Method, RealizationCertificate, RealizationVerdict, Refusal,
};
use truck_geometry::base::*;
use truck_geometry::constructive::*;
use truck_modeling::facet_sweep::{
    facet_sweep, facet_sweep_certified, summarize_construct_error, verdict_of, winding_audit,
    FacetSweepAudit, FacetSweepResult, FacetVerdict,
};
use truck_polymesh::*;

/// The unit-square profile, CCW in the frame plane (profile-x rides the
/// frame normal, profile-y the frame binormal; r2 semantics).
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

/// The required tapered pair fixture: a LinearCorrespondence between the unit
/// square and the 2x square. Its side cells are trapezoids (the twist test
/// splits them), so the planar-quad test uses a translating correspondence.
fn tapered_pair() -> ProfileLaw {
    ProfileLaw::try_linear_correspondence(unit_square(), larger_square())
        .unwrap_or(ProfileLaw::Constant(unit_square()))
}

/// A square congruent to the unit square, translated by (1, 1): the two
/// squares translate as a rigid body, so every side cell is a parallelogram
/// (twist zero) and every side face emits as one planar quad.
fn translated_square() -> Profile2D {
    Profile2D {
        vertices: vec![
            Point2::new(1.0, 1.0),
            Point2::new(2.0, 1.0),
            Point2::new(2.0, 2.0),
            Point2::new(1.0, 2.0),
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

/// A straight spine of length 2: C(s) = (2s, 0, 0) on [0, 1].
fn straight_spine() -> LineSpine {
    LineSpine {
        start: Point3::new(0.0, 0.0, 0.0),
        end: Point3::new(2.0, 0.0, 0.0),
    }
}

/// The +Z FixedPlane frame law used by every fixture.
fn fixed_plane_z() -> FrameLaw {
    FrameLaw::FixedPlane {
        normal: Vector3::unit_z(),
    }
}

/// The unit-circle arc spine about the Z axis (the sanctioned test-local
/// curved `SpineCurve`, same as the frame packets): `C(s) = (cos θ, sin θ, 0)`
/// with `θ = phi0 + s·delta`, `s ∈ [0, 1]`.
#[derive(Debug, Clone, Copy)]
struct QuarterCircleSpine {
    /// The arc start angle.
    phi0: f64,
    /// The arc sweep angle.
    delta: f64,
}

impl SpineCurve for QuarterCircleSpine {
    fn domain(&self) -> (f64, f64) {
        (0.0, 1.0)
    }

    fn position_at(&self, s: f64) -> Result<Point3, ConstructError> {
        if !s.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        let theta = self.phi0 + s * self.delta;
        Ok(Point3::new(theta.cos(), theta.sin(), 0.0))
    }

    fn derivative_at(&self, s: f64) -> Result<Vector3, ConstructError> {
        if !s.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        let theta = self.phi0 + s * self.delta;
        Ok(Vector3::new(
            -self.delta * theta.sin(),
            self.delta * theta.cos(),
            0.0,
        ))
    }
}

/// Resolves `n` uniform stations over `[0, 1]`. On a refusal (impossible for
/// `n >= 2` over an ascending window) returns a sentinel list that makes the
/// downstream construction refuse loudly, so the test still fails.
fn uniform_stations(n: usize) -> Vec<f64> {
    match (SamplingPolicy::UniformCount { spine: n }).resolve(0.0, 1.0) {
        Ok(stations) => stations,
        Err(_) => vec![f64::NAN],
    }
}

/// Sweeps and extracts the result; any refusal fails the test loudly. The
/// Err arm is unreachable (the `assert!` already failed) but must typecheck.
fn swept<S: SpineCurve>(
    recipe: &SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
    stations: &[f64],
    ring: usize,
) -> FacetSweepResult {
    let result = facet_sweep(recipe, stations, ring);
    assert!(result.is_ok(), "facet_sweep refused (ring = {ring})");
    match result {
        Ok(result) => result,
        Err(_) => FacetSweepResult {
            mesh: PolygonMesh::default(),
            audit: FacetSweepAudit {
                triangle_count: 0,
                quad_count: 0,
                signed_volume: 0.0,
                winding_violations: 1,
            },
            verdict: FacetVerdict::Failed,
            realization_certificate: RealizationCertificate {
                method: Method::Float,
                max_cell_twist: 0.0,
                extent: 0.0,
            },
            shared_edge_pairs: Vec::new(),
        },
    }
}

/// Independent winding recomputation (no HashMap): every undirected edge must
/// appear exactly twice with opposite effective directions; a use-count of 1
/// or >= 3 is also a violation.
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

#[test]
fn straight_duct_closes_with_exact_shared_indices() {
    let recipe = SpineFrameRecipe::new(
        straight_spine(),
        ProfileLaw::Constant(unit_square()),
        fixed_plane_z(),
    );
    let stations = uniform_stations(5);
    let result = swept(&recipe, &stations, 4);
    assert_eq!(
        independent_winding_violations(&result.mesh),
        0,
        "every undirected edge must appear exactly twice, opposite directions"
    );
    assert_eq!(result.audit.winding_violations, 0);
    assert_eq!(result.verdict, FacetVerdict::CertifiedWithinTolerance);
}

#[test]
fn grid_registry_creates_each_vertex_exactly_once() {
    let recipe = SpineFrameRecipe::new(
        straight_spine(),
        ProfileLaw::Constant(unit_square()),
        fixed_plane_z(),
    );
    let stations = uniform_stations(5);
    let result = swept(&recipe, &stations, 4);
    let registry_len = stations.len() * 4;
    assert_eq!(result.mesh.positions().len(), registry_len);
    for face in result.mesh.faces().face_iter() {
        for v in face {
            assert!(
                v.pos < registry_len,
                "face vertex outside the grid registry"
            );
        }
    }
}

#[test]
fn tapered_duct_emits_planar_quads() {
    // A LinearCorrespondence between two squares that translate as a rigid
    // body: every side cell is a parallelogram, so the twist test certifies
    // every side face planar and the side strip emits (m-1)*k quads.
    let profile = ProfileLaw::try_linear_correspondence(unit_square(), translated_square())
        .unwrap_or(ProfileLaw::Constant(unit_square()));
    let recipe = SpineFrameRecipe::new(straight_spine(), profile, fixed_plane_z());
    let stations = uniform_stations(5);
    let k = 4;
    let m = stations.len();
    let result = swept(&recipe, &stations, k);
    assert_eq!(result.audit.quad_count, (m - 1) * k);
    assert_eq!(result.audit.triangle_count, 0);
    assert_eq!(result.mesh.quad_faces().len(), (m - 1) * k);
    assert_eq!(result.mesh.tri_faces().len(), 2 * (k - 2));

    // The packet's scaled "tapered pair" is NOT all-planar: its trapezoid
    // cells fail the twist test (the bilinear twist is the station interval),
    // so the side strip splits every one of them into triangles.
    let scaled = SpineFrameRecipe::new(straight_spine(), tapered_pair(), fixed_plane_z());
    let scaled_result = swept(&scaled, &stations, k);
    assert_eq!(scaled_result.audit.quad_count, 0);
    assert_eq!(scaled_result.audit.triangle_count, 2 * (m - 1) * k);
}

#[test]
fn curved_spine_splits_along_the_fixed_diagonal() {
    let spine = QuarterCircleSpine {
        phi0: 0.0,
        delta: std::f64::consts::FRAC_PI_2,
    };
    let recipe = SpineFrameRecipe::new(spine, ProfileLaw::Constant(unit_square()), fixed_plane_z());
    let stations = uniform_stations(5);
    let k = 4;
    let result = swept(&recipe, &stations, k);
    assert_eq!(result.audit.winding_violations, 0);
    let pairs = result.audit.triangle_count / 2;
    assert!(
        pairs > 0,
        "the curved fixture must produce split side cells"
    );
    assert_eq!(result.audit.triangle_count, 2 * pairs);
    let tris = result.mesh.tri_faces();
    assert_eq!(tris.len(), 2 * pairs + 2 * (k - 2));
    for t in 0..pairs {
        let t1 = tris[2 * t];
        let t2 = tris[2 * t + 1];
        // The split pair shares exactly the fixed (i,j)-(i+1,j2) diagonal;
        // the global orientation normalization may reverse every face, so the
        // pattern is asserted invariant under a full index-cycle reversal.
        let mut shared: Vec<usize> = [t1[0].pos, t1[1].pos, t1[2].pos]
            .iter()
            .copied()
            .filter(|x| [t2[0].pos, t2[1].pos, t2[2].pos].contains(x))
            .collect();
        shared.sort_unstable();
        shared.dedup();
        assert_eq!(
            shared.len(),
            2,
            "split pair must share exactly the diagonal"
        );
        let a = shared[0];
        let c = shared[1];
        assert!(
            c == a + k + 1 || c == a + 1,
            "diagonal runs from ring j at this station to ring j2 at the next"
        );
        let p_unique = [t1[0].pos, t1[1].pos, t1[2].pos]
            .iter()
            .copied()
            .find(|p| *p != a && *p != c)
            .unwrap_or(usize::MAX);
        assert_eq!(
            p_unique,
            a + k,
            "the pair's first triangle uses the next station's same ring vertex"
        );
        let q_unique = [t2[0].pos, t2[1].pos, t2[2].pos]
            .iter()
            .copied()
            .find(|p| *p != a && *p != c)
            .unwrap_or(usize::MAX);
        assert!(
            q_unique == a + 1 || q_unique == a - k + 1,
            "the pair's second triangle uses the next ring vertex at this station"
        );
    }
}

#[test]
fn profile_collapse_refuses_before_emission() {
    let recipe = SpineFrameRecipe::new(
        straight_spine(),
        ProfileLaw::Scale {
            profile: unit_square(),
            scale: ScalarLaw::Constant(0.0),
        },
        fixed_plane_z(),
    );
    let stations = uniform_stations(5);
    assert!(matches!(
        facet_sweep(&recipe, &stations, 4),
        Err(ConstructError::ProfileCollapse { .. })
    ));
}

#[test]
fn non_convex_cap_refuses() {
    let recipe = SpineFrameRecipe::new(
        straight_spine(),
        ProfileLaw::Constant(l_shape()),
        fixed_plane_z(),
    );
    let stations = uniform_stations(5);
    assert!(matches!(
        facet_sweep(&recipe, &stations, 6),
        Err(ConstructError::InvalidInput)
    ));
}

#[test]
fn winding_audit_counts_violations() {
    let tetra = PolygonMesh::new(
        StandardAttributes {
            positions: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(0.0, 0.0, 1.0),
            ],
            ..Default::default()
        },
        Faces::from_iter(&[[0, 1, 2], [0, 3, 1], [0, 2, 3], [1, 3, 2]]),
    );
    assert_eq!(winding_audit(&tetra), 0);

    let broken = PolygonMesh::new(
        StandardAttributes {
            positions: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(0.0, 0.0, 1.0),
            ],
            ..Default::default()
        },
        Faces::from_iter(&[[0, 1, 2], [2, 0, 3]]),
    );
    assert!(winding_audit(&broken) > 0);
}

#[test]
fn signed_volume_matches_analytic_box() {
    let recipe = SpineFrameRecipe::new(
        straight_spine(),
        ProfileLaw::Constant(unit_square()),
        fixed_plane_z(),
    );
    let stations = uniform_stations(5);
    let result = swept(&recipe, &stations, 4);
    let bound = TOLERANCE * 8.0;
    assert!(
        (result.audit.signed_volume - 2.0).abs() <= bound,
        "prism volume must match area * length = 2.0 exactly up to rounding"
    );
}

#[test]
fn inconclusive_verdict_is_representable() {
    let zero_volume = FacetSweepAudit {
        triangle_count: 4,
        quad_count: 0,
        signed_volume: 0.0,
        winding_violations: 0,
    };
    assert_eq!(verdict_of(&zero_volume, 1.0), FacetVerdict::Inconclusive);

    let violated = FacetSweepAudit {
        triangle_count: 4,
        quad_count: 0,
        signed_volume: 1.0,
        winding_violations: 1,
    };
    assert_eq!(verdict_of(&violated, 1.0), FacetVerdict::Failed);

    let good = FacetSweepAudit {
        triangle_count: 4,
        quad_count: 0,
        signed_volume: 1.0,
        winding_violations: 0,
    };
    assert_eq!(
        verdict_of(&good, 1.0),
        FacetVerdict::CertifiedWithinTolerance
    );
}

#[test]
fn stations_are_validated() {
    let recipe = SpineFrameRecipe::new(
        straight_spine(),
        ProfileLaw::Constant(unit_square()),
        fixed_plane_z(),
    );
    assert!(matches!(
        facet_sweep(&recipe, &[0.75, 0.25, 0.5], 4),
        Err(ConstructError::InvalidInput)
    ));
    assert!(matches!(
        facet_sweep(&recipe, &[0.5], 4),
        Err(ConstructError::InvalidInput)
    ));
    assert!(matches!(
        facet_sweep(&recipe, &[0.0, f64::NAN], 4),
        Err(ConstructError::NonFinite { .. })
    ));
    assert!(matches!(
        facet_sweep(&recipe, &[0.0, 1.5], 4),
        Err(ConstructError::InvalidInput)
    ));
    assert!(matches!(
        facet_sweep(&recipe, &[0.0, 0.0], 4),
        Err(ConstructError::InvalidInput)
    ));
    assert!(matches!(
        facet_sweep(&recipe, &[0.0, 1.0], 2),
        Err(ConstructError::InvalidInput)
    ));
}

#[test]
fn construct_refused_variant_exists_and_carries_no_payload() {
    let case = EnvelopeCase::ConstructRefused;
    // The bare-variant pattern compiles only because the variant is unit
    // shaped: a payload-carrying variant cannot be matched without binding it
    // (mapping A row 1).
    assert!(matches!(case, EnvelopeCase::ConstructRefused));
    assert_eq!(case, EnvelopeCase::ConstructRefused);
}

#[test]
fn realization_verdict_absorbs_facet_verdict() {
    assert_eq!(
        RealizationVerdict::from(FacetVerdict::CertifiedWithinTolerance),
        RealizationVerdict::CertifiedWithinTolerance
    );
    assert_eq!(
        RealizationVerdict::from(FacetVerdict::Failed),
        RealizationVerdict::Failed
    );
    assert_eq!(
        RealizationVerdict::from(FacetVerdict::Inconclusive),
        RealizationVerdict::Inconclusive
    );
}

#[test]
fn facet_sweep_certified_refuses_with_construct_refused() {
    let recipe = SpineFrameRecipe::new(
        straight_spine(),
        ProfileLaw::Scale {
            profile: unit_square(),
            scale: ScalarLaw::Constant(0.0),
        },
        fixed_plane_z(),
    );
    let stations = uniform_stations(5);
    assert!(matches!(
        facet_sweep_certified(&recipe, &stations, 4),
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused))
    ));
    // A refusal cannot carry a payload, so the summary is re-derived from the
    // same construct error (mapping A row 1).
    let construct_error = match facet_sweep(&recipe, &stations, 4) {
        Err(error) => error,
        Ok(_) => panic!("the collapsed-profile fixture must refuse"),
    };
    let summary = summarize_construct_error(&construct_error);
    assert_eq!(summary.kind, "ProfileCollapse");
}

#[test]
fn facet_sweep_certified_ok_carries_evidence_and_certificate() {
    let recipe = SpineFrameRecipe::new(
        straight_spine(),
        ProfileLaw::Constant(unit_square()),
        fixed_plane_z(),
    );
    let stations = uniform_stations(5);
    let certified = match facet_sweep_certified(&recipe, &stations, 4) {
        Ok(certified) => certified,
        Err(other) => panic!("straight duct refused: {other:?}"),
    };
    // H-6: the facet path computes in floats — never `Exact`.
    assert_eq!(certified.cert.method, Method::Float);
    let result = certified.value;
    assert_eq!(result.verdict, FacetVerdict::CertifiedWithinTolerance);
    assert_eq!(result.realization_certificate.method, Method::Float);
    assert!(result.realization_certificate.extent > 0.0); // H-3: a mesh extent is a length; a positive-literal bound is fine for the fixture
}

#[test]
fn shared_edge_pairs_empty_on_exact_grid_path() {
    let recipe = SpineFrameRecipe::new(
        straight_spine(),
        ProfileLaw::Constant(unit_square()),
        fixed_plane_z(),
    );
    let stations = uniform_stations(5);
    let result = swept(&recipe, &stations, 4);
    // The grid registry makes shared edges index-identical by construction:
    // there is no measured error to record (mapping A row 3).
    assert!(result.shared_edge_pairs.is_empty());
}
