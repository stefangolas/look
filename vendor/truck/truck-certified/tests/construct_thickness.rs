//! CC-026-THICKNESS integration tests (spine S7 consumer; theory §7.1, with
//! §7.2–§7.3 deferred): the conservative certified shell thickness
//! `t_safe = min(t_focal, d_min/2)` over real certified surface maps and
//! CC-021 offset strata — a unit-sphere cap's `t_safe` covers the whole
//! radius, a curved plate is bounded by its own focal event, two parallel
//! plates are bounded by half the gap, the non-adjacent exclusion of the
//! bottleneck term matches the CC-022 glue plan, and an enclosure straddling
//! the minimum refuses `NonGenericThicknessEvent`. The test names are the
//! contract.

#![deny(clippy::unwrap_used)]

use truck_certified::certified_map::{admit_surface, CertifiedSurfaceMap};
use truck_certified::construct::offset_strata::{face_stratum, OffsetStratum};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::stars::{BoundaryRef, FaceSide, Glue, GluePlan, SharedBoundary};
use truck_certified::construct::thickness::{d_min_over_nonadjacent, t_focal, t_safe};
use truck_certified::formal::numeric::PositiveFinite;
use truck_geometry::prelude::{BSplineSurface, KnotVec, Point3};

/// A declared positive tau for the fixtures.
fn tau(value: f64) -> PositiveFinite {
    PositiveFinite::new(value).expect("a positive declared tau")
}

/// Admit a surface fixture with the given declared tau, panicking only on a
/// test-bug refusal.
fn admitted(surface: &BSplineSurface<Point3>, value: f64) -> CertifiedSurfaceMap {
    admit_surface(surface, tau(value)).expect("the surface fixture admits")
}

/// The flat horizontal plane `(u, v, h)` over `[0, 1]^2`: a flat face
/// (`J_t = 1` exactly, no focal event along either offset direction).
fn flat_plane(height: f64) -> BSplineSurface<Point3> {
    let uknot = KnotVec::bezier_knot(1);
    let vknot = KnotVec::bezier_knot(1);
    let ctrl = vec![
        vec![Point3::new(0.0, 0.0, height), Point3::new(0.0, 1.0, height)],
        vec![Point3::new(1.0, 0.0, height), Point3::new(1.0, 1.0, height)],
    ];
    BSplineSurface::new((uknot, vknot), ctrl)
}

/// The parabolic-cylinder plate `S(u, v) = (2u, 2v, u²)` over `[0, 1]^2` (the
/// CC-002 `curved_patch` data, the CC-021 parabolic face): `S_u × S_v =
/// (−4u, 0, 4)` so `σ* = 4`, the only nonzero second partial is
/// `S_uu = (0, 0, 2)`, and the parabola `z = x²/4` carries max principal
/// curvature `κ_max = 1/2` at the apex (`u = 0`). The first focal event along
/// the offset direction is `1/κ_max = 2` — the plate's own focal bound.
fn parabolic_plate() -> BSplineSurface<Point3> {
    let uknot = KnotVec::bezier_knot(2);
    let vknot = KnotVec::bezier_knot(1);
    let ctrl = vec![
        vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 2.0, 0.0)],
        vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 2.0, 0.0)],
        vec![Point3::new(2.0, 0.0, 1.0), Point3::new(2.0, 2.0, 1.0)],
    ];
    BSplineSurface::new((uknot, vknot), ctrl)
}

/// The unit-sphere cap local model `S(u, v) = (a·u, a·v, a²·(u² + v²)/2)` over
/// `[0, 1]^2` for a dyadic `a`: the quadratic cap osculating the unit sphere
/// at its apex, whose centre of curvature (the focal point of the offset
/// along the outward normal) sits at unit distance `1` from the apex. The cap
/// is exactly representable as a degree-2×2 tensor Bézier patch: `x` and `y`
/// are the linear maps `a·u`, `a·v` (control stations `(0, a/2, a)`), and
/// `z = a²·(u² + v²)/2` separates as `cap·(u_quad + v_quad)` with
/// `cap = a²/2` and `(0, 0, 1)` quadratic control patterns in `u` and `v`.
fn sphere_cap(a: f64) -> BSplineSurface<Point3> {
    let uknot = KnotVec::bezier_knot(2);
    let vknot = KnotVec::bezier_knot(2);
    let stations = [0.0, 0.5, 1.0];
    let cap = a * a / 2.0;
    let mut ctrl = Vec::with_capacity(3);
    for i in 0..3 {
        let mut row = Vec::with_capacity(3);
        for j in 0..3 {
            let x = a * stations[i];
            let y = a * stations[j];
            let u_bit = if i == 2 { 1.0 } else { 0.0 };
            let v_bit = if j == 2 { 1.0 } else { 0.0 };
            let z = cap * (u_bit + v_bit);
            row.push(Point3::new(x, y, z));
        }
        ctrl.push(row);
    }
    BSplineSurface::new((uknot, vknot), ctrl)
}

/// One face stratum over an admitted surface at the given offset.
fn face(map: &CertifiedSurfaceMap, offset: f64) -> OffsetStratum {
    face_stratum(map, offset).expect("the face stratum certifies at this offset")
}

/// The seam glue identifying two strata along an arbitrary side pair with a
/// shared boundary identity.
fn glue_pair(a: usize, b: usize) -> Glue {
    Glue {
        a: BoundaryRef {
            stratum: a,
            side: FaceSide::UMin,
            boundary: SharedBoundary::new(7),
        },
        b: BoundaryRef {
            stratum: b,
            side: FaceSide::VMax,
            boundary: SharedBoundary::new(7),
        },
    }
}

#[test]
fn unit_sphere_t_safe_is_the_whole_radius() {
    // The unit-sphere cap focal model: a cap of a unit ball whose offset along
    // the outward normal focuses at the ball's centre, a certified distance of
    // `1` (the sphere's radius) from the apex. With no competing strata the
    // bottleneck term is empty and `t_safe` is exactly the focal term; its
    // certified lower bound must cover the full radius `1` (within the
    // `sqrt(CC_ETA_J)` margin-floor sliver and the certified enclosure width
    // of the quadratic cap).
    let a = 2.0_f64.powi(-14);
    let map = admitted(&sphere_cap(a), 1e-12); // H-3: declared tau below the tiny-cap margin a^2
    let safe = t_safe(&map, &[], &GluePlan::default()).expect("the cap certifies a shell");
    assert!(
        safe.lo <= 1.0,
        "the certified bound never exceeds the unit radius: {}",
        safe.lo
    );
    assert!(
        1.0 - safe.lo < 1e-3, // H-3: certified focal bound covers the unit radius within the cap enclosure width
        "the certified bound covers the whole radius: t_safe.lo = {}",
        safe.lo
    );
    assert!(
        safe.hi >= safe.lo,
        "the returned interval is ordered: {:?}",
        safe
    );

    // The focal term itself reports the same lower bound: the cap is the whole
    // shelling constraint here.
    let focal = t_focal(&map, ((0.0, 1.0), (0.0, 1.0))).expect("the cap focal term certifies");
    assert!(
        (safe.lo - focal.lo).abs() < 1e-9, // H-3: the sole shelling term is the focal term
        "the cap shell bound equals its focal bound: safe.lo = {}, focal.lo = {}",
        safe.lo,
        focal.lo
    );
}

#[test]
fn thin_plate_t_safe_is_bounded_by_focal_term() {
    // A curved plate: the parabolic-cylinder face with `κ_max = 1/2` at its
    // apex, whose first focal event sits at `t = 1/κ_max = 2` along the
    // offset direction. `t_safe` of the plate is bounded by that focal term —
    // the certified lower bound reaches `2` and never exceeds it.
    let map = admitted(&parabolic_plate(), 1.0);
    let safe = t_safe(&map, &[], &GluePlan::default()).expect("the plate certifies a shell");
    assert!(
        safe.lo > 0.0 && safe.lo.is_finite(),
        "the plate focal bound is finite and positive: {}",
        safe.lo
    );
    assert!(
        safe.lo <= 2.0,
        "the certified bound never exceeds the focal event 1/κ_max = 2: {}",
        safe.lo
    );
    assert!(
        2.0 - safe.lo < 1e-6, // H-3: the certified focal bound reaches the dyadic focal event 2
        "the plate is focal-bounded at 2: t_safe.lo = {}",
        safe.lo
    );
}

#[test]
fn two_parallel_plates_t_safe_is_bounded_by_half_gap() {
    // Two parallel flat plates a certified distance `gap = 2` apart: two flat
    // faces have no focal event, so the shelling bound is the bottleneck term
    // alone — the growing offsets of the two non-adjacent source plates meet
    // at half the gap, and `t_safe` must be bounded by `gap/2 = 1`.
    let gap = 2.0_f64;
    let map = admitted(&flat_plane(0.0), 0.5);
    let strata = vec![face(&map, 0.0), face(&admitted(&flat_plane(gap), 0.5), 0.0)];
    let safe = t_safe(&map, &strata, &GluePlan::default()).expect("the plates certify a shell");
    assert!(
        safe.lo <= gap / 2.0,
        "the certified bound never exceeds half the gap: {}",
        safe.lo
    );
    assert!(
        gap / 2.0 - safe.lo < 1e-9, // H-3: the bottleneck bound reaches half the dyadic gap
        "the parallel plates are bottleneck-bounded at half the gap: t_safe.lo = {}",
        safe.lo
    );
}

#[test]
fn non_adjacent_exclusion_matches_star_glue_plan() {
    // Three parallel plates at heights 0, 0.25 and 1, with the closest pair
    // (0, 1) identified by a glue seam. The bottleneck term must EXCLUDE the
    // glued pair (its contact is handled by the local star certificates) and
    // report the certified minimum over the remaining non-adjacent pairs —
    // the (1, 2) gap of 0.75 — never the glued (0, 1) gap of 0.25.
    let plate0 = face(&admitted(&flat_plane(0.0), 0.5), 0.0);
    let plate1 = face(&admitted(&flat_plane(0.25), 0.5), 0.0);
    let plate2 = face(&admitted(&flat_plane(1.0), 0.5), 0.0);
    let strata = vec![plate0, plate1, plate2];

    let plan = GluePlan {
        seams: vec![glue_pair(0, 1)],
    };
    let d_min = d_min_over_nonadjacent(&strata, &plan).expect("the non-adjacent minimum certifies");
    assert!(
        d_min >= 0.5,
        "the glued (0, 1) pair is excluded: d_min = {d_min}"
    );
    assert!(
        0.75 - d_min < 1e-9, // H-3: the exclusion leaves the (1, 2) gap of 0.75 as the minimum
        "the non-adjacent minimum is the (1, 2) gap 0.75, got {d_min}"
    );

    // Without the glue plan the same three plates have no excluded pair, and
    // the minimum is the closest (0, 1) gap of 0.25 — the exclusion is pinned
    // by the plan, never by geometry.
    let empty = GluePlan::default();
    let d_all = d_min_over_nonadjacent(&strata, &empty).expect("the full minimum certifies");
    assert!(
        0.25 - d_all < 1e-9, // H-3: with no glue the (0, 1) gap of 0.25 is the minimum
        "the unglued minimum is the (0, 1) gap 0.25, got {d_all}"
    );
}

#[test]
fn enclosure_straddle_refuses_no_generic_event() {
    // The curved plate's focal event is certified inside `[~2, 1/H.lo]` (the
    // mean curvature lower bound `H.lo ≈ 0.088` gives the certified upper
    // bound `1/H.lo ≈ 11.3`), while two parallel plates ten units apart place
    // the bottleneck at `d_min/2 = 5` — strictly INSIDE the certified focal
    // enclosure. The minimum therefore straddles the evidence: the ordering of
    // the focal and bottleneck events is not generically decidable on the v1
    // evidence (the exact `valid_shell_interval` of theory §7.2–§7.3 is
    // deferred) and the construction refuses `NonGenericThicknessEvent`.
    let map = admitted(&parabolic_plate(), 1.0);
    let focal = t_focal(&map, ((0.0, 1.0), (0.0, 1.0))).expect("the plate focal term certifies");
    assert!(
        focal.lo.is_finite() && focal.hi.is_finite(),
        "the straddle fixture needs a finite focal enclosure: {focal:?}"
    );

    let gap = 10.0_f64;
    let strata = vec![
        face(&admitted(&flat_plane(0.0), 0.5), 0.0),
        face(&admitted(&flat_plane(gap), 0.5), 0.0),
    ];
    let d_min = d_min_over_nonadjacent(&strata, &GluePlan::default()).expect("the plates certify");
    let half_gap = d_min / 2.0;
    assert!(
        half_gap > focal.lo && half_gap < focal.hi,
        "the bottleneck half-gap {half_gap} sits inside the focal enclosure [{}, {}]",
        focal.lo,
        focal.hi
    );

    match t_safe(&map, &strata, &GluePlan::default()) {
        Err(ConstructRefusal::NonGenericThicknessEvent) => {}
        Ok(interval) => panic!(
            "the straddling enclosure must refuse NonGenericThicknessEvent, certified: {interval:?}"
        ),
        Err(refusal) => panic!("expected NonGenericThicknessEvent, got: {refusal:?}"),
    }
}
