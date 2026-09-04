//! BG-KV2-402-S7 integration tests: the §12 R7 ball-center residual, its
//! certification as a C2 tube at n = 7, the Canal representation and the Δ_off
//! diagnostic, and the §12.3 three-face corner (compositional via R8, refused
//! when unsolved).

#![deny(clippy::unwrap_used)]

use std::fmt::Debug;

use truck_certified::kernel::canal::{
    build_frame7, c2_certify_tube7, corner_compositional, delta_off, Canal, CornerPoint, DirField,
    R7System, SideSign,
};
use truck_certified::kernel::certs::PointCert3;
use truck_certified::kernel::config;
use truck_certified::kernel::engine::SquareResidualEval;
use truck_certified::kernel::evidence::{ClaimVerdict, RefusalKind, VerdictClass};
use truck_certified::kernel::graph::{ArcId, ChartId};
use truck_certified::kernel::leaf::BezierLeaf;
use truck_certified::kernel::patch::{CertifiedPositive, IBox2, IBox3};
use truck_certified::kernel::residual::ResidualId;
use truck_certified::kernel::residuals_r89::{BezierLeaf1, R8System};
use truck_certified::kernel::Interval;

/// Extract the `Ok` of any fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct_ok<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => panic!("a construction that must succeed was refused: {err:?}"),
    }
}

/// A 2-axis parameter box.
fn box2(u_lo: f64, u_hi: f64, v_lo: f64, v_hi: f64) -> IBox2 {
    construct_ok(IBox2::try_new([u_lo, v_lo], [u_hi, v_hi]))
}

/// A 3-axis parameter box.
fn box3(lo: [f64; 3], hi: [f64; 3]) -> IBox3 {
    construct_ok(IBox3::try_new(lo, hi))
}

/// The six-axis perpendicular box of the n = 7 tube.
fn box6(lo: [f64; 6], hi: [f64; 6]) -> truck_certified::kernel::patch::IBox<6> {
    construct_ok(truck_certified::kernel::patch::IBox::<6>::try_new(lo, hi))
}

/// A point interval.
fn iv(x: f64) -> Interval {
    Interval::point(x)
}

/// One certified positive unit weight (the §7.1 value argument).
fn positive_one() -> CertifiedPositive {
    construct_ok(CertifiedPositive::try_new(1.0))
}

// ---------------------------------------------------------------------------
// The two-plane fixture: a rolling ball in the dihedral of the plane `z = 0`
// and the plane `y = 0`.
// ---------------------------------------------------------------------------

/// The unit-weight plane `S1(u, v) = (u, v, 0)` at bidegree `(1, 1)` (plane
/// `z = 0`, normal `n1 = (0,0,1)`).
fn plane_z0() -> BezierLeaf {
    let control = vec![
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 0.0, 1.0],
    ];
    construct_ok(BezierLeaf::try_new(1, 1, control))
}

/// The unit-weight plane `S2(s, t) = (t, 0, s)` at bidegree `(1, 1)` (plane
/// `y = 0`, parameter normal `n2 = S2_s × S2_t = (0,1,0)`).
fn plane_y0() -> BezierLeaf {
    // Control points over rows (s) and columns (t): (t, 0, s) for (s,t) in
    // {0,1}². s=0: (0,0,0),(1,0,0); s=1: (0,0,1),(1,0,1).
    let control = vec![
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [1.0, 0.0, 1.0, 1.0],
    ];
    construct_ok(BezierLeaf::try_new(1, 1, control))
}

/// The two-plane ball-center fixture: radius `r = 0.25`, spine parameter
/// `x = 0.5`.
///
/// The center `c = (x, r, r)` is at distance `r` from both planes on the
/// `(+z, +y)` side; the contact parameters are `u = x`, `v = r` on `S1` and
/// `s = r`, `t = x` on `S2`. Both parameter normals point at the ball, so the
/// side pair is `σ = (+1, +1)`. The R7 zero set through it is the straight
/// line `(c, u, v, s, t) = (x, r, r, x, r, r, x)`.
struct TwoPlaneFixture {
    /// The `z = 0` plane leaf.
    s1: BezierLeaf,
    /// The `y = 0` plane leaf.
    s2: BezierLeaf,
    /// The rolling-ball radius.
    r: f64,
}

fn two_plane_fixture() -> TwoPlaneFixture {
    TwoPlaneFixture {
        s1: plane_z0(),
        s2: plane_y0(),
        r: 0.25,
    }
}

/// The exact seven-var solution `(c_x, c_y, c_z, u, v, s, t)` of the fixture
/// at spine parameter `x`.
fn fixture_solution(fx: &TwoPlaneFixture, x: f64) -> [f64; 7] {
    [x, fx.r, fx.r, x, fx.r, fx.r, x]
}

// ---------------------------------------------------------------------------
// R7 residual construction
// ---------------------------------------------------------------------------

#[test]
fn r7_residual_builds_from_two_rational_carriers() {
    let fx = two_plane_fixture();
    let sys = construct_ok(R7System::try_new(&fx.s1, &fx.s2, fx.r));

    assert_eq!(sys.arity(), 7);
    assert_eq!(sys.nrows(), 6);
    assert_eq!(sys.radius(), fx.r);
    assert_eq!(sys.a(), &fx.s1);
    assert_eq!(sys.b(), &fx.s2);

    // The residual vanishes at the exact fixture solution.
    let z = fixture_solution(&fx, 0.5);
    let box_: Vec<Interval> = z.iter().map(|x| iv(*x)).collect();
    let r = sys.eval(&box_);
    assert_eq!(r.len(), 6);
    for (k, component) in r.iter().enumerate() {
        assert!(
            component.contains(0.0),
            "the R7 residual must vanish at the ball-center solution: component {k} = {component:?}"
        );
    }
}

#[test]
fn r7_refuses_invalid_radius() {
    let fx = two_plane_fixture();
    match R7System::try_new(&fx.s1, &fx.s2, -1.0) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        Ok(_) => panic!("a non-positive radius must refuse the R7 system"),
    }
    match R7System::try_new(&fx.s1, &fx.s2, f64::NAN) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::NonFinite);
        }
        Ok(_) => panic!("a non-finite radius must refuse the R7 system"),
    }
    // A raw zero-degree leaf refuses at the system gate too.
    let bad = BezierLeaf {
        degree_u: 0,
        degree_v: 1,
        control: vec![[0.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]],
    };
    match R7System::try_new(&bad, &fx.s2, fx.r) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
        }
        Ok(_) => panic!("a raw zero-degree leaf must refuse the R7 system"),
    }
}

// ---------------------------------------------------------------------------
// The n = 7 certification: the two-plane ball center certifies as a C2 tube
// ---------------------------------------------------------------------------

#[test]
fn ball_center_solution_certifies_on_two_plane_fixture() {
    let fx = two_plane_fixture();
    let sys = construct_ok(R7System::try_new(&fx.s1, &fx.s2, fx.r));
    let z = fixture_solution(&fx, 0.5);

    // The frame at the fixture solution: the tangent must be the spine
    // direction of the straight line.
    let built = match build_frame7(&sys, z) {
        Ok(b) => b,
        Err(refusal) => {
            panic!("the two-plane frame must build at the fixture solution: {refusal:?}")
        }
    };

    // The C2 tube over a short spine interval: the perpendicular box is a
    // small symmetric neighbourhood of the solution.
    let i_tau = Interval {
        lo: -0.001,
        hi: 0.001,
    };
    let b_perp = box6([-0.005; 6], [0.005; 6]);
    match c2_certify_tube7(&sys, &built.frame, i_tau, b_perp) {
        ClaimVerdict::Proven(cert) => {
            assert!(cert.rho <= config::RHO_MAX, "rho must satisfy the ceiling");
            assert_eq!(cert.residual, ResidualId::R7);
            assert_eq!(cert.i_tau, i_tau);
        }
        ClaimVerdict::Disproven(refusal) => {
            panic!("the two-plane ball-center solution must certify Proven, refused: {refusal:?}")
        }
        ClaimVerdict::Inconclusive(reason) => {
            panic!("the two-plane ball-center solution must certify Proven, inconclusive: {reason}")
        }
    }
}

#[test]
fn offset_equivalence_theorem_12_1_holds_on_fixture() {
    // Theorem 12.1: an R7 solution with sign(N_i·(c − S_i)) = σ_i is exactly
    // the offset-intersection c = S1 + σ1 r n1 = S2 + σ2 r n2. On the fixture
    // the two offset constructions coincide with the center at every spine
    // sample, and the certified side signs agree with the chosen σ.
    let fx = two_plane_fixture();
    let sys = construct_ok(R7System::try_new(&fx.s1, &fx.s2, fx.r));

    for x in [0.25f64, 0.5, 0.75] {
        let z = fixture_solution(&fx, x);
        let c = [z[0], z[1], z[2]];
        // The R7 residual vanishes at the sample.
        let box_: Vec<Interval> = z.iter().map(|x| iv(*x)).collect();
        let r = sys.eval(&box_);
        for (k, component) in r.iter().enumerate() {
            assert!(component.contains(0.0), "residual row {k} vanishes");
        }
        // The offsets S_i + σ_i r n_i with the geometric unit normals of the
        // two planes coincide with c. n1 = (0,0,1) (z = 0 plane, parameter
        // normal of S1), n2 = (0,1,0) (parameter normal of the y = 0 plane
        // leaf), σ1 = σ2 = +1.
        let n1 = [0.0, 0.0, 1.0];
        let n2 = [0.0, 1.0, 0.0];
        // S1 foot at (u, v) = (x, r): S1 = (x, r, 0).
        let foot1 = [z[3], z[4], 0.0];
        // S2 foot at (s, t) = (r, x): S2(s,t) = (t, 0, s) = (x, 0, r).
        let foot2 = [z[6], 0.0, z[5]];
        let c1 = [
            foot1[0] + fx.r * n1[0],
            foot1[1] + fx.r * n1[1],
            foot1[2] + fx.r * n1[2],
        ];
        let c2 = [
            foot2[0] + fx.r * n2[0],
            foot2[1] + fx.r * n2[1],
            foot2[2] + fx.r * n2[2],
        ];
        for k in 0..3 {
            assert!(
                (c1[k] - c[k]).abs() <= 1e-12,
                "offset 1 reproduces the center at x = {x}: axis {k}"
            );
            assert!(
                (c2[k] - c[k]).abs() <= 1e-12,
                "offset 2 reproduces the center at x = {x}: axis {k}"
            );
        }
        // The certified side signs at the sample.
        let s1 = SideSign::try_new(fx.r, fx.r).unwrap_or_else(|_| panic!("signs are certified"));
        assert_eq!(s1.pair(), (1, 1));
    }
}

#[test]
fn rank_structure_theorem_12_2_offset_subsumed_on_fixture() {
    // Theorem 12.2: at a solution rank DR7 = 6 iff both offsets are immersed
    // at c and their tangent planes at c are distinct — offset regularity is
    // SUBSUMED, no separate Δ_off precondition. On the fixture the certified
    // Jacobian has full row rank 6 (the frame builds with a non-degenerate
    // kernel direction) and the Δ_off diagnostic of each planar carrier over
    // the contact box excludes zero (the offset is immersed), computed as a
    // check, never as a precondition.
    let fx = two_plane_fixture();
    let sys = construct_ok(R7System::try_new(&fx.s1, &fx.s2, fx.r));
    let z = fixture_solution(&fx, 0.5);

    // Full row rank 6: a non-degenerate maximal-minor direction exists, i.e.
    // the frame construction succeeds (it refuses a degenerate kernel
    // direction below the TOL_JACOBIAN floor).
    let built = match build_frame7(&sys, z) {
        Ok(b) => b,
        Err(refusal) => panic!("rank 6 at the fixture solution must build a frame: {refusal:?}"),
    };
    let m_norm_sq = built.m.iter().map(|c| c * c).sum::<f64>();
    assert!(
        m_norm_sq > config::EPS_REP * config::EPS_REP,
        "the kernel direction is non-degenerate"
    );

    // The Δ_off diagnostic of each planar carrier over a contact box excludes
    // zero (the offsets are immersed) — the plane's EG − F² is the only
    // surviving term (L = M = N = 0).
    let uv = box2(0.3, 0.7, 0.1, 0.4);
    for leaf in [&fx.s1, &fx.s2] {
        let d = construct_ok(delta_off(leaf, fx.r, uv));
        assert!(d.excludes_zero, "planar Δ_off must exclude zero");
        assert!(d.egf2.0 > 0.0, "a plane has EG - F^2 > 0: {:?}", d.egf2);
    }
}

// ---------------------------------------------------------------------------
// Canal representation and the no-orthogonality audit
// ---------------------------------------------------------------------------

#[test]
fn canal_builds_with_contact_fields_and_side_signs() {
    // The contact direction fields d1 = (c − S1)/r and d2 = (c − S2)/r of the
    // fixture are unit vectors in the contact directions.
    let d1 = construct_ok(DirField::try_new(1, 1, [0.0, 0.0, 1.0]));
    let d2 = construct_ok(DirField::try_new(2, 1, [0.0, 1.0, 0.0]));
    let canal = construct_ok(Canal::try_new(ArcId(3), 0.25, (1, 1), (d1, d2)));
    assert_eq!(canal.spine, ArcId(3));
    assert_eq!(canal.r, 0.25);
    assert_eq!(canal.sigma, (1, 1));
    assert_eq!(canal.contact.0.face, 1);
    assert_eq!(canal.contact.1.face, 2);
}

#[test]
fn canal_has_no_orthogonality_certificate_field() {
    // Prop 12.3 makes the normal-plane invariant a theorem: Canal carries NO
    // orthogonality certificate field. The audit is a source scan (the struct
    // body of Canal declares exactly spine, r, sigma, contact — no further
    // field) plus a structural assertion that no certified-orthogonality
    // certificate type exists anywhere in the module.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/kernel/canal.rs");
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => panic!("canal.rs must be readable: {err}"),
    };
    let code = strip_comments(&source);
    assert!(
        !code.contains("orthogonality"),
        "no code token may name an orthogonality certificate field"
    );
    assert!(
        !code.contains("CertifiedOrthogonality"),
        "CertifiedOrthogonality must not exist anywhere in the module"
    );
    // The struct body declares exactly the four §16 fields.
    let start = code
        .find("pub struct Canal")
        .expect("Canal struct declaration present");
    let open = code[start..].find('{').expect("Canal struct opens");
    let body = &code[start + open..];
    let close = body.find('}').expect("Canal struct closes");
    let body = &body[..close];
    for field in [
        "pub spine: ArcId",
        "pub r: f64",
        "pub sigma: (i8, i8)",
        "pub contact: (DirField, DirField)",
    ] {
        assert!(body.contains(field), "Canal must declare {field}");
    }
}

#[test]
fn dir_field_refuses_non_unit_direction() {
    match DirField::try_new(1, 1, [1.0, 1.0, 0.0]) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        Ok(_) => panic!("a non-unit contact direction must refuse"),
    }
    match DirField::try_new(3, 1, [1.0, 0.0, 0.0]) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
        }
        Ok(_) => panic!("a parent face outside {{1,2}} must refuse"),
    }
}

// ---------------------------------------------------------------------------
// The §12.3 three-face corner
// ---------------------------------------------------------------------------

/// The trihedral corner fixture: the two-face fillet spine of the planes
/// `z = 0` and `y = 0` (spine line `(x, r, r)`), cut by the third plane
/// `x = 0` whose offset surface `O3` is the plane `x = r`. The corner center is
/// `c = (r, r, r)`.
struct CornerFixture {
    /// The spine line leaf `c(t) = (t, r, r)` over `t ∈ [0, 1]`.
    spine: BezierLeaf1,
    /// The offset plane leaf `O3(u, v) = (r, u, v)` over `(u, v) ∈ [0, 1]²`.
    o3: BezierLeaf,
    /// The rolling-ball radius.
    r: f64,
}

fn corner_fixture() -> CornerFixture {
    let r = 0.25;
    let spine = construct_ok(BezierLeaf1::try_new(
        1,
        vec![[0.0, r, r, 1.0], [1.0, r, r, 1.0]],
        ChartId(0),
    ));
    let o3 = construct_ok(BezierLeaf::try_new(
        1,
        1,
        vec![
            [r, 0.0, 0.0, 1.0],
            [r, 0.0, 1.0, 1.0],
            [r, 1.0, 0.0, 1.0],
            [r, 1.0, 1.0, 1.0],
        ],
    ));
    CornerFixture { spine, o3, r }
}

#[test]
fn three_face_corner_compositional_via_r8() {
    // The R8 system `c12(t) − O3(u,v)` over the spine leaf and the third
    // plane's offset surface has the dyadic root `(t, u, v) = (r, r, r)`. The
    // compositional solve certifies it as a square C1 (R8), giving the corner
    // center (r, r, r).
    let fx = corner_fixture();
    let root = [fx.r, fx.r, fx.r];
    let b = box3([0.2, 0.2, 0.2], [0.3, 0.3, 0.3]);
    let w = vec![positive_one()];
    match corner_compositional(&fx.spine, &fx.o3, b, &w) {
        Ok(CornerPoint { center, cert }) => {
            assert_eq!(cert.residual, ResidualId::R8);
            for k in 0..3 {
                assert!(
                    (center[k] - root[k]).abs() <= 1e-9,
                    "the corner center must be ({}, {}, {}): axis {k} gives {}",
                    fx.r,
                    fx.r,
                    fx.r,
                    center[k]
                );
            }
        }
        Err(refusal) => panic!("the trihedral corner must solve compositionally: {refusal:?}"),
    }
}

#[test]
fn corner_unsolved_refuses() {
    // A corner whose spine never reaches the third face's offset (the offset
    // plane is translated out of reach) has no certified R8 root in the box:
    // the solve refuses CornerUnsolved (Inconclusive), and the typed refusal
    // carries the name.
    let fx = corner_fixture();
    let far = construct_ok(BezierLeaf::try_new(
        1,
        1,
        vec![
            [2.0, 0.0, 0.0, 1.0],
            [2.0, 0.0, 1.0, 1.0],
            [2.0, 1.0, 0.0, 1.0],
            [2.0, 1.0, 1.0, 1.0],
        ],
    ));
    let b = box3([0.2, 0.2, 0.2], [0.8, 0.8, 0.8]);
    let w = vec![positive_one()];
    match corner_compositional(&fx.spine, &far, b, &w) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::CornerUnsolved);
            assert_eq!(refusal.backing, VerdictClass::Inconclusive);
        }
        Ok(CornerPoint { center, cert }) => {
            panic!(
                "an out-of-reach corner must refuse CornerUnsolved, certified center {center:?} {cert:?}"
            )
        }
    }

    let refusal = truck_certified::kernel::canal::corner_unsolved_refusal(b);
    assert_eq!(refusal.kind, RefusalKind::CornerUnsolved);
    assert_eq!(refusal.backing, VerdictClass::Inconclusive);
}

#[test]
fn r8_system_matches_s1a_seam() {
    // The corner's R8 system is the S1A curve–surface seam, square arity 3:
    // the compositional solve routes through R8System.
    let fx = corner_fixture();
    let sys = construct_ok(R8System::try_new(&fx.spine, &fx.o3));
    assert_eq!(sys.arity(), 3);
    // The homogeneous residual vanishes at the root (t, u, v) = (r, r, r).
    let at_root = sys.eval(&[iv(fx.r), iv(fx.r), iv(fx.r)]);
    for (k, component) in at_root.iter().enumerate() {
        assert!(
            component.contains(0.0),
            "the R8 residual must vanish at the corner root: component {k} = {component:?}"
        );
    }
    let b = box3([0.2, 0.2, 0.2], [0.3, 0.3, 0.3]);
    let w = vec![positive_one()];
    match truck_certified::kernel::engine::krawczyk_c1_n3(&sys, b, &w) {
        ClaimVerdict::Proven(cert) => {
            let cert = construct_ok(PointCert3::try_new(ResidualId::R8, cert.box_, cert.rho));
            assert_eq!(cert.residual, ResidualId::R8);
            for axis in 0..3 {
                let root = [fx.r, fx.r, fx.r][axis];
                assert!(
                    cert.box_.lo[axis] <= root && root <= cert.box_.hi[axis],
                    "the certified box contains the corner root on axis {axis}"
                );
            }
        }
        other => panic!("the R8 corner root must certify Proven: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// N4/N5 discipline scans
// ---------------------------------------------------------------------------

/// Strip `//` line comments, `///`/`//!` doc comments, and `/* ... */` blocks.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            let mut depth = 1usize;
            i += 2;
            while i < chars.len() && depth > 0 {
                if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                    depth -= 1;
                    i += 2;
                } else if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                    depth += 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if chars[i] == '/' && (i + 1 >= chars.len() || chars[i + 1] == '/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

#[test]
fn no_transcendental_call_in_canal_module() {
    // N4: the canal module performs no transcendental call — no sin, cos,
    // atan2, exp, ln, log, powf anywhere, and no sqrt outside frame
    // normalization (the engine carve-out: sqrt only on norm lines).
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/kernel/canal.rs");
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => panic!("canal.rs must be readable: {err}"),
    };
    let code = strip_comments(&source);
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let contains_word = |hay: &str, word: &str| {
        hay.match_indices(word).any(|(i, _)| {
            let before = i
                .checked_sub(1)
                .map(|j| hay.as_bytes()[j] as char)
                .map(is_word)
                .unwrap_or(false);
            let after = hay
                .as_bytes()
                .get(i + word.len())
                .map(|b| *b as char)
                .map(is_word)
                .unwrap_or(false);
            !before && !after
        })
    };
    for needle in ["sin", "cos", "atan2", "exp", "ln", "log", "powf"] {
        let present = code
            .lines()
            .any(|line| contains_word(line, needle) || line.contains("std::f64::consts"));
        assert!(
            !present,
            "no transcendental call may appear outside comments in canal.rs (found {needle})"
        );
    }
    // sqrt appears only in frame normalization contexts.
    let sqrt_lines: Vec<&str> = code.lines().filter(|line| line.contains("sqrt")).collect();
    assert!(
        sqrt_lines
            .iter()
            .all(|l| l.contains("norm") || l.contains("norm_sq")),
        "sqrt must appear only in frame normalization: {sqrt_lines:?}"
    );
    assert!(!sqrt_lines.is_empty(), "frame normalization uses sqrt");
}
