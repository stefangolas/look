//! ADM-003-VOLUME conformance: the certified volume assembly over the admitted
//! patches.
//!
//! The module under test is
//! [`truck_certified::construct::volume_facts`]: the divergence-form face
//! 2-form `(1/3)âˆ¬ XÂ·(X_uÃ—X_v)` assembled over the L1-extracted patches, EXACT
//! for polynomial (unit-weight) faces and certified via the landed L5
//! reciprocal-power primitive for the rational (conic/revolution) faces, with
//! the closure discipline (volume scored only over a proven-closed, oriented
//! boundary) and the certified algebraic-trim cell bracket (scope decision 3,
//! route (a)).
//!
//! The four required tests:
//!
//! - **`frustum_special_case_bit_identical`** â€” the V5 frustum special case:
//!   the closed square-pyramid frustum (six unit-weight bilinear faces) is
//!   scored through the new machinery and answers BIT-IDENTICALLY to the
//!   telescoping closed form `V = h/3Â·(Aâ‚ + Aâ‚‚ + âˆš(Aâ‚Aâ‚‚))` on its own dyadic
//!   fixture â€” the exact polynomial route returns the exactly-rounded value;
//! - **`polynomial_face_volume_matches_closed_form`** â€” the same closed solid
//!   class on different (non-integral) data still matches the closed form
//!   within the certified bracket;
//! - **`rational_face_volume_within_certified_bound`** â€” the biquadratic NURBS
//!   octant of the unit sphere (the ADM-SHIM rational volume fixture) is a
//!   genuinely rational face; its certified bracket contains the exactly
//!   computable enclosing-solid volume `Ï€/6` within the L5 tail bound;
//! - **`unclosed_boundary_detected_not_scored`** â€” a boundary missing one
//!   lateral face (four free rim sides) refuses the volume fact typed and is
//!   never scored.
//!
//! Supporting tests:
//!
//! - `rational_face_route_matches_on_face_and_patch_forms` â€” the octant fact
//!   is identical through the face-extraction route and the direct patch
//!   route;
//! - `algebraic_trim_bracket_closes_on_quarter_disk` â€” the certified cell
//!   decomposition (route (a)) brackets the quarter-disk trimmed-domain
//!   integral of the unit density (closed form `Ï€/4`).

#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]

use truck_certified::construct::patches::PatchSide;
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::volume_facts::{
    certify_algebraic_trim_bracket, certify_face_form, certify_patch_form, certify_solid_volume,
    FaceGlue, OrientedFace, SolidBoundary, VolumeOptions,
};
use truck_geometry::prelude::{BSplineSurface, KnotVec, Vector4};

#[path = "patch_fixtures.rs"]
mod patch_fixtures;

/// The relative closed-form matching tolerance of the polynomial volume
/// tests (the exact route round-trips dyadic-rational volumes; the bound is
/// orders above the rounding noise at unit scale). H-3: named, dimensionless.
const CLOSED_FORM_REL_TOL: f64 = 1.0e-12;

/// The certified bound asserted on the rational-face route's bracket width.
/// H-3: named, dimensionless.
const RATIONAL_FACE_BOUND: f64 = 1.0e-2;

/// The certified bound asserted on the trim route's bracket width. H-3:
/// named, dimensionless.
const TRIM_BRACKET_BOUND: f64 = 5.0e-3;

/// The default certified options.
fn options() -> VolumeOptions {
    VolumeOptions::default()
}

/// The `Ok` value of a certified construction; the refusal arm is a fixture
/// bug, never a silent pass.
fn expect_fact<T>(result: Result<T, ConstructRefusal>, what: &str) -> T {
    match result {
        Ok(fact) => fact,
        Err(refusal) => panic!("{what} refused: {refusal:?}"),
    }
}

/// A unit-weight single-span bilinear spline face over the four corners in the
/// parameter order `p00 = X(0,0)`, `p10 = X(1,0)`, `p01 = X(0,1)`,
/// `p11 = X(1,1)`.
fn bilinear_surface(
    p00: [f64; 3],
    p10: [f64; 3],
    p01: [f64; 3],
    p11: [f64; 3],
) -> BSplineSurface<Vector4> {
    let knots = KnotVec::from(vec![0.0_f64, 0.0, 1.0, 1.0]);
    let to_v4 = |p: [f64; 3]| Vector4::new(p[0], p[1], p[2], 1.0);
    let ctrl = vec![vec![to_v4(p00), to_v4(p01)], vec![to_v4(p10), to_v4(p11)]];
    BSplineSurface::new((knots.clone(), knots), ctrl)
}

/// The corner `(u, v) = (0.5, 0.5)` tangent estimates of a unit-weight
/// bilinear face, from its control net (the exact Bernstein derivative at the
/// parameter midpoint).
fn bilinear_tangents(surface: &BSplineSurface<Vector4>) -> ([f64; 3], [f64; 3]) {
    let c = surface.control_points();
    let p = |i: usize, j: usize| [c[i][j].x, c[i][j].y, c[i][j].z];
    // dX/du at the center = Î£_j (c[1][j] âˆ’ c[0][j])Â·B_j(1/2), B_j(1/2) = 1/2.
    let mut du = [0.0; 3];
    let mut dv = [0.0; 3];
    for axis in 0..3 {
        du[axis] = 0.5 * ((p(1, 0)[axis] - p(0, 0)[axis]) + (p(1, 1)[axis] - p(0, 1)[axis]));
        dv[axis] = 0.5 * ((p(0, 1)[axis] - p(0, 0)[axis]) + (p(1, 1)[axis] - p(1, 0)[axis]));
    }
    (du, dv)
}

/// The cross product of two `RÂ³` vectors.
fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The dot product of two `RÂ³` vectors.
fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The midpoint of two points.
fn midpoint(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        0.5 * (a[0] + b[0]),
        0.5 * (a[1] + b[1]),
        0.5 * (a[2] + b[2]),
    ]
}

/// A `FaceGlue` between two faces' parameter sides.
fn glue(a: usize, sa: PatchSide, b: usize, sb: PatchSide) -> FaceGlue {
    FaceGlue {
        lhs: (a, sa),
        rhs: (b, sb),
    }
}

/// The closed, oriented square-pyramid-frustum boundary with bottom half-width
/// `a`, top half-width `b` and height `h`, centered on the z axis on the plane
/// `z = 0`.
///
/// Face order: 0 bottom cap, 1 top cap, 2 north, 3 south, 4 east, 5 west. The
/// orientations are derived from the convex-solid outward normals (the signed
/// `X_uÃ—X_v` against the face-centre-to-centroid direction), so the assembly's
/// own parametrizations are exercised exactly as a caller would declare them.
fn frustum_boundary(a: f64, b: f64, h: f64) -> SolidBoundary {
    let bottom = bilinear_surface([-a, -a, 0.0], [a, -a, 0.0], [-a, a, 0.0], [a, a, 0.0]);
    let top = bilinear_surface([-b, -b, h], [b, -b, h], [-b, b, h], [b, b, h]);
    let north = bilinear_surface([-a, a, 0.0], [a, a, 0.0], [-b, b, h], [b, b, h]);
    let south = bilinear_surface([-a, -a, 0.0], [a, -a, 0.0], [-b, -b, h], [b, -b, h]);
    let east = bilinear_surface([a, -a, 0.0], [a, a, 0.0], [b, -b, h], [b, b, h]);
    let west = bilinear_surface([-a, -a, 0.0], [-a, a, 0.0], [-b, -b, h], [-b, b, h]);

    let surfaces = vec![bottom, top, north, south, east, west];
    // The solid centroid: the mean of every face's corners (a strictly
    // interior point of the convex frustum).
    let mut centroid = [0.0; 3];
    let mut count = 0_usize;
    for surface in &surfaces {
        let c = surface.control_points();
        for row in c {
            for point in row {
                centroid[0] += point.x;
                centroid[1] += point.y;
                centroid[2] += point.z;
                count += 1;
            }
        }
    }
    centroid[0] /= count as f64;
    centroid[1] /= count as f64;
    centroid[2] /= count as f64;

    let faces = surfaces
        .iter()
        .map(|surface| {
            let c = surface.control_points();
            let center = midpoint(
                midpoint(
                    [c[0][0].x, c[0][0].y, c[0][0].z],
                    [c[1][1].x, c[1][1].y, c[1][1].z],
                ),
                midpoint(
                    [c[0][1].x, c[0][1].y, c[0][1].z],
                    [c[1][0].x, c[1][0].y, c[1][0].z],
                ),
            );
            let (du, dv) = bilinear_tangents(surface);
            let normal = cross3(du, dv);
            let outward = [
                center[0] - centroid[0],
                center[1] - centroid[1],
                center[2] - centroid[2],
            ];
            let orientation = if dot3(normal, outward) > 0.0 {
                1.0
            } else {
                -1.0
            };
            OrientedFace {
                surface: surface.clone(),
                orientation,
            }
        })
        .collect();

    let glue_edges = vec![
        // Bottom cap rims.
        glue(0, PatchSide::SideVMin, 3, PatchSide::SideVMin),
        glue(0, PatchSide::SideVMax, 2, PatchSide::SideVMin),
        glue(0, PatchSide::SideUMax, 4, PatchSide::SideVMin),
        glue(0, PatchSide::SideUMin, 5, PatchSide::SideVMin),
        // Top cap rims.
        glue(1, PatchSide::SideVMin, 3, PatchSide::SideVMax),
        glue(1, PatchSide::SideVMax, 2, PatchSide::SideVMax),
        glue(1, PatchSide::SideUMax, 4, PatchSide::SideVMax),
        glue(1, PatchSide::SideUMin, 5, PatchSide::SideVMax),
        // Lateral corners.
        glue(2, PatchSide::SideUMin, 5, PatchSide::SideUMax),
        glue(2, PatchSide::SideUMax, 4, PatchSide::SideUMax),
        glue(3, PatchSide::SideUMin, 5, PatchSide::SideUMin),
        glue(3, PatchSide::SideUMax, 4, PatchSide::SideUMin),
    ];
    SolidBoundary {
        faces,
        glue: glue_edges,
    }
}

/// The closed-form frustum volume `V = h/3Â·(Aâ‚ + Aâ‚‚ + âˆš(Aâ‚Aâ‚‚))` of the square
/// frustum with bottom half-width `a`, top half-width `b` and height `h`.
fn frustum_volume(a: f64, b: f64, h: f64) -> f64 {
    let a1 = (2.0 * a) * (2.0 * a);
    let a2 = (2.0 * b) * (2.0 * b);
    (h / 3.0) * (a1 + a2 + (a1 * a2).sqrt())
}

/// The octant face of the ADM-SHIM rational volume fixture as a single-span
/// biquadratic homogeneous `BSplineSurface<Vector4>`.
fn octant_face() -> BSplineSurface<Vector4> {
    let volume = patch_fixtures::rational_quadratic_volume_fixture();
    let patch = volume.patch;
    let knots = KnotVec::from(vec![0.0_f64, 0.0, 0.0, 1.0, 1.0, 1.0]);
    let mut controls: Vec<Vec<Vector4>> = Vec::new();
    for (i, row) in patch.numerator().iter().enumerate() {
        let mut control_row: Vec<Vector4> = Vec::new();
        for (j, a) in row.iter().enumerate() {
            control_row.push(Vector4::new(a[0], a[1], a[2], patch.weights()[i][j]));
        }
        controls.push(control_row);
    }
    BSplineSurface::new((knots.clone(), knots), controls)
}

/// The orientation sign of the unit-sphere octant parameterization: the
/// surface's outward normal is `X` itself (unit sphere), so the parameter
/// orientation is `+1` exactly when `XÂ·(X_uÃ—X_v) > 0` at a regular interior
/// parameter.
fn octant_orientation(face: &BSplineSurface<Vector4>) -> f64 {
    // The fixture octant is a single (2,2) span; sample at (1/2, 1/2) with
    // the analytic Bernstein basis and its derivatives.
    let c = face.control_points();
    let rows = c.len();
    let cols = c[0].len();
    let (u, v) = (0.5, 0.5);
    let eval = |deriv_u: bool, deriv_v: bool| -> ([f64; 3], f64) {
        let mut num = [0.0; 3];
        let mut w = 0.0;
        for i in 0..rows {
            for j in 0..cols {
                let bu = if deriv_u {
                    bern_deriv(rows - 1, i, u)
                } else {
                    bern_basis(rows - 1, i, u)
                };
                let bv = if deriv_v {
                    bern_deriv(cols - 1, j, v)
                } else {
                    bern_basis(cols - 1, j, v)
                };
                let b = bu * bv;
                let point = c[i][j];
                num[0] += b * point.x;
                num[1] += b * point.y;
                num[2] += b * point.z;
                w += b * point.w;
            }
        }
        (num, w)
    };
    let (a, w) = eval(false, false);
    let (au, wu) = eval(true, false);
    let (av, wv) = eval(false, true);
    let xu = [
        (au[0] * w - a[0] * wu) / (w * w),
        (au[1] * w - a[1] * wu) / (w * w),
        (au[2] * w - a[2] * wu) / (w * w),
    ];
    let xv = [
        (av[0] * w - a[0] * wv) / (w * w),
        (av[1] * w - a[1] * wv) / (w * w),
        (av[2] * w - a[2] * wv) / (w * w),
    ];
    let x = [a[0] / w, a[1] / w, a[2] / w];
    if dot3(cross3(xu, xv), x) > 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// The degree-`n` Bernstein basis value `B_k^n(t)`.
fn bern_basis(n: usize, k: usize, t: f64) -> f64 {
    let c = binom(n, k) as f64;
    c * t.powi(k as i32) * (1.0 - t).powi((n - k) as i32)
}

/// The derivative `dB_k^n/dt` of the Bernstein basis.
fn bern_deriv(n: usize, k: usize, t: f64) -> f64 {
    // d/dt B_k^n = nÂ·(B_{k-1}^{n-1} âˆ’ B_k^{n-1}), out-of-range bases zero.
    let mut result = 0.0;
    if k >= 1 {
        result += bern_basis(n - 1, k - 1, t);
    }
    if k <= n - 1 {
        result -= bern_basis(n - 1, k, t);
    }
    (n as f64) * result
}

/// The integer binomial `C(n, k)`.
fn binom(n: usize, k: usize) -> usize {
    let k = k.min(n - k);
    let mut r = 1usize;
    for i in 1..=k {
        r = r * (n - k + i) / i;
    }
    r
}

#[test]
fn frustum_special_case_bit_identical() {
    // The V5 frustum special case: the closed square-pyramid frustum with
    // bottom half-width 2, top half-width 1 and height 3 has the telescoping
    // closed-form volume h/3Â·(Aâ‚ + Aâ‚‚ + âˆš(Aâ‚Aâ‚‚)) = (3/3)Â·(16 + 4 + 8) = 28.
    // The fixture is dyadic, so the exact polynomial route must answer
    // BIT-IDENTICALLY through the new machinery.
    let boundary = frustum_boundary(2.0, 1.0, 3.0);
    assert_eq!(boundary.faces.len(), 6);
    assert_eq!(boundary.glue.len(), 12);

    let fact = expect_fact(
        certify_solid_volume(&boundary, &options()),
        "frustum volume",
    );
    let closed = frustum_volume(2.0, 1.0, 3.0);
    assert_eq!(closed, 28.0);
    assert_eq!(
        fact.value.to_bits(),
        closed.to_bits(),
        "the certified frustum value must be bit-identical to the closed form \
         (exact polynomial route on the dyadic fixture)"
    );
    assert_eq!(fact.value, 28.0);
    assert!(
        fact.bracket.contains(closed),
        "the certified bracket [{}, {}] must contain the closed form {closed}",
        fact.bracket.lo,
        fact.bracket.hi
    );
    assert!(fact.closed, "the scored boundary is certified closed");
    assert_eq!(fact.faces, 6);
    assert_eq!(fact.patches, 6);
}

#[test]
fn polynomial_face_volume_matches_closed_form() {
    // The same closed solid class on non-integral data (bottom half-width 3,
    // top half-width 1, height 5): the exact volume 260/3 is not dyadic, so
    // the certified value matches the closed form within the certified
    // bracket instead of bit-for-bit.
    let boundary = frustum_boundary(3.0, 1.0, 5.0);
    let fact = expect_fact(
        certify_solid_volume(&boundary, &options()),
        "polynomial frustum volume",
    );
    let closed = frustum_volume(3.0, 1.0, 5.0);
    assert!(
        fact.bracket.contains(closed),
        "the certified bracket [{}, {}] must contain the closed form {closed}",
        fact.bracket.lo,
        fact.bracket.hi
    );
    let scale = 1.0 + closed.abs();
    assert!(
        (fact.value - closed).abs() <= CLOSED_FORM_REL_TOL * scale,
        "the certified value {} diverges from the closed form {closed} beyond the \
         certified tolerance",
        fact.value
    );
    assert!(
        fact.bracket.width() <= 1.0e-9 * scale,
        "the exact route's bracket is tight"
    );
    assert!(fact.closed);
}

#[test]
fn rational_face_volume_within_certified_bound() {
    // The rational-quadratic unit-sphere octant: a genuinely rational face
    // (bidegree (2,2), non-constant strictly positive weights) whose
    // enclosing-solid volume is exactly Ï€/6 (the ADM-SHIM fixture's closed
    // form). The certified L5 route must bracket Ï€/6 within its declared
    // bound â€” never a point pretending exactness.
    let volume = patch_fixtures::rational_quadratic_volume_fixture();
    let closed = volume.closed_form_volume;
    assert_eq!(closed, std::f64::consts::PI / 6.0);

    // The face-extraction route on the homogeneous spline face.
    let face = octant_face();
    let orientation = octant_orientation(&face);
    let oriented = OrientedFace {
        surface: face,
        orientation,
    };
    let fact_face = expect_fact(
        certify_face_form(&oriented, &options()),
        "octant rational face form",
    );
    assert!(
        fact_face.bracket.contains(closed),
        "the certified bracket [{}, {}] must contain the octant volume {closed}",
        fact_face.bracket.lo,
        fact_face.bracket.hi
    );
    assert!(
        fact_face.certified_error <= RATIONAL_FACE_BOUND,
        "the certified L5 error {} exceeds the declared bound {RATIONAL_FACE_BOUND}",
        fact_face.certified_error
    );
    let width = fact_face.bracket.hi - fact_face.bracket.lo;
    assert!(
        width <= RATIONAL_FACE_BOUND,
        "the certified bracket width {width} exceeds the declared bound"
    );

    // The direct patch route on the kit's own patch agrees with the face
    // route (the same extracted patch scored two ways).
    let fact_patch = expect_fact(
        certify_patch_form(&volume.patch, orientation, &options()),
        "octant rational patch form",
    );
    let scale = 1.0 + fact_face.value.abs();
    assert!(
        (fact_patch.value - fact_face.value).abs() <= CLOSED_FORM_REL_TOL * scale,
        "the patch route value {} diverges from the face route value {}",
        fact_patch.value,
        fact_face.value
    );
    assert!(
        fact_patch.bracket.contains(closed),
        "the patch-route bracket must also contain {closed}"
    );
}

#[test]
fn unclosed_boundary_detected_not_scored() {
    // Remove the north lateral face (index 2) from the closed frustum: its
    // four rims become free, the shared-boundary structure no longer closes,
    // and the volume fact must refuse typed â€” never a score over an
    // unproven-closed boundary.
    let full = frustum_boundary(2.0, 1.0, 3.0);
    let removed = 2_usize;
    let remap = |i: usize| -> usize {
        if i > removed {
            i - 1
        } else {
            i
        }
    };
    let faces: Vec<OrientedFace> = full
        .faces
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != removed)
        .map(|(_, face)| face.clone())
        .collect();
    let glue: Vec<FaceGlue> = full
        .glue
        .iter()
        .copied()
        .filter(|g| g.lhs.0 != removed && g.rhs.0 != removed)
        .map(|g| FaceGlue {
            lhs: (remap(g.lhs.0), g.lhs.1),
            rhs: (remap(g.rhs.0), g.rhs.1),
        })
        .collect();
    let open = SolidBoundary { faces, glue };

    match certify_solid_volume(&open, &options()) {
        Ok(fact) => panic!(
            "an unclosed boundary must refuse the volume fact, got a scored volume {}",
            fact.value
        ),
        Err(ConstructRefusal::InvalidInput) => {}
        Err(refusal) => panic!("expected a typed InvalidInput refusal, got {refusal:?}"),
    }
}

#[test]
fn rational_face_route_matches_on_face_and_patch_forms() {
    // Convenience alias of the octant check in the required test above: kept
    // as its own named conformance so the face/patch route identity is pinned
    // independently of the certified-bound assertions.
    let volume = patch_fixtures::rational_quadratic_volume_fixture();
    let face = octant_face();
    let orientation = octant_orientation(&face);
    let face_fact = expect_fact(
        certify_face_form(
            &OrientedFace {
                surface: face,
                orientation,
            },
            &options(),
        ),
        "octant face form",
    );
    let patch_fact = expect_fact(
        certify_patch_form(&volume.patch, orientation, &options()),
        "octant patch form",
    );
    let scale = 1.0 + patch_fact.value.abs();
    assert!(
        (face_fact.value - patch_fact.value).abs() <= CLOSED_FORM_REL_TOL * scale,
        "face route {} and patch route {} must agree on the octant form",
        face_fact.value,
        patch_fact.value
    );
    assert!(
        face_fact.value > 0.0,
        "the outward-oriented octant form must be positive"
    );
}

#[test]
fn algebraic_trim_bracket_closes_on_quarter_disk() {
    // The certified cell decomposition (scope decision 3, route (a)) over the
    // algebraic trim of a polynomial density: the unit square trimmed by the
    // pullback circle P(s, t) = 1 âˆ’ sÂ² âˆ’ tÂ² (kept region the quarter disk)
    // with the unit density has the closed-form integral Ï€/4. The two-sided
    // certified bracket must contain it and close to the requested tolerance.
    let unit: Vec<Vec<f64>> = vec![
        vec![1.0, 1.0, 1.0],
        vec![1.0, 1.0, 1.0],
        vec![1.0, 1.0, 1.0],
    ];
    // The (2,2) Bernstein net of 1 âˆ’ sÂ² âˆ’ tÂ²: 1 minus sÂ² (rows: subtract at the
    // s-degree-2 corner) minus tÂ² (columns: subtract at the t-degree-2 corner).
    let quarter_circle: Vec<Vec<f64>> = vec![
        vec![1.0, 1.0, 0.0],
        vec![1.0, 1.0, 0.0],
        vec![0.0, 0.0, -1.0],
    ];
    let bracket = expect_fact(
        certify_algebraic_trim_bracket(&unit, &quarter_circle, 1.0e-3),
        "quarter-disk trim bracket",
    );
    let closed = std::f64::consts::PI / 4.0;
    assert!(
        bracket.contains(closed),
        "the certified trim bracket [{}, {}] must contain the quarter-disk \
         integral {closed}",
        bracket.lo,
        bracket.hi
    );
    let width = bracket.hi - bracket.lo;
    assert!(
        width <= TRIM_BRACKET_BOUND,
        "the certified trim bracket width {width} exceeds the declared bound"
    );
}
