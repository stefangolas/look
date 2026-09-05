//! CC-023-SHELL-BRIDGE integration tests (spine S7 consumer; theory §4.2–4.4):
//! the shell certificate — the S1 embedding certificate on the quotient (seam
//! stars through `certify_star`, the certified reach-bound broad phase, and
//! the retained pairs through the evidence contact funnel onto the
//! three-valued CC-014 verdict) and the S1′ solid corollary (closed,
//! connected, orientation-consistent pre-made checks). A convex plane-faced
//! prism shells `Solid` with every pair `Certified`; an unintended self
//! contact reports `Contact`; an undecidable pair is `Inconclusive`, never
//! `Certified`; an open complex refuses the corollary with the typed `Open`
//! outcome; and S1′ requires closed, connected, orientable complexes before it
//! ever says `Solid`. The test names are the contract.

#![deny(clippy::unwrap_used)]

use truck_base::evidence::Budget;
use truck_certified::certified_map::{admit_surface, CertifiedSurfaceMap};
use truck_certified::construct::offset_strata::{face_stratum, OffsetStratum};
use truck_certified::construct::shell::{
    certify_shell, ShellCert, ShellPairVerdict, SolidOutcome, SurfaceOnlyReason,
};
use truck_certified::construct::stars::{BoundaryRef, FaceSide, Glue, GluePlan, SharedBoundary};
use truck_certified::formal::numeric::PositiveFinite;
use truck_geometry::prelude::{BSplineSurface, KnotVec, Point3};

/// A declared positive tau for the fixtures.
fn tau(value: f64) -> PositiveFinite {
    PositiveFinite::new(value).expect("a positive declared tau")
}

/// The affine surface `S(u, v) = base + u*du + v*dv` over `[0, 1]^2`.
fn affine_surface(base: [f64; 3], du: [f64; 3], dv: [f64; 3]) -> BSplineSurface<Point3> {
    let uknot = KnotVec::bezier_knot(1);
    let vknot = KnotVec::bezier_knot(1);
    let add = |a: [f64; 3], b: [f64; 3]| [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
    let p = |v: [f64; 3]| Point3::new(v[0], v[1], v[2]);
    let ctrl = vec![
        vec![p(base), p(add(base, dv))],
        vec![p(add(base, du)), p(add(add(base, du), dv))],
    ];
    BSplineSurface::new((uknot, vknot), ctrl)
}

/// Admit an affine face fixture with a declared tau of `1e-3` (the fixtures
/// include thin prism faces whose certified area margin is as small as `0.1`,
/// so the tau must sit well below the smallest area).
fn admitted(base: [f64; 3], du: [f64; 3], dv: [f64; 3]) -> CertifiedSurfaceMap {
    let surface = affine_surface(base, du, dv);
    admit_surface(&surface, tau(1e-3)).expect("the affine face fixture admits")
}

/// Admit a surface fixture with the declared tau, panicking only on a
/// test-bug refusal.
fn admit_any(surface: &BSplineSurface<Point3>) -> CertifiedSurfaceMap {
    admit_surface(surface, tau(1e-3)).expect("the surface fixture admits")
}

/// The four source corners `corners[u][v]` of an affine face.
fn corners(base: [f64; 3], du: [f64; 3], dv: [f64; 3]) -> [[[f64; 3]; 2]; 2] {
    let add = |a: [f64; 3], b: [f64; 3]| [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
    let mut out = [[[0.0_f64; 3]; 2]; 2];
    for u in 0..2 {
        for v in 0..2 {
            let mut p = base;
            if u == 1 {
                p = add(p, du);
            }
            if v == 1 {
                p = add(p, dv);
            }
            out[u][v] = p;
        }
    }
    out
}

/// The tilted plane `(u, -(1 - v)*c, (1 - v)*s)` over `[0, 1]^2` with
/// `(c, s) = (cos φ, sin φ)`, φ = 45°: the second leaf of the open-book wedge.
fn tilted_leaf() -> BSplineSurface<Point3> {
    let c = 0.7071067811865476;
    let s = 0.7071067811865476;
    let base = [0.0, -c, s];
    let du = [1.0, 0.0, 0.0];
    let dv = [0.0, c, -s];
    affine_surface(base, du, dv)
}

/// The two canonical side endpoints of an edge (the corner-pair the side runs
/// along in the `(u, v)`-counter-clockwise boundary walk).
fn side_endpoints(c: &[[[f64; 3]; 2]; 2], side: usize) -> ([f64; 3], [f64; 3]) {
    let arcs = [
        ((0, 1), (0, 0)), // UMin
        ((1, 0), (1, 1)), // UMax
        ((0, 0), (1, 0)), // VMin
        ((1, 1), (0, 1)), // VMax
    ];
    let (start, end) = arcs[side];
    (c[start.0][start.1], c[end.0][end.1])
}

/// The `FaceSide` of a canonical side index (`UMin = 0, UMax = 1, VMin = 2,
/// VMax = 3`).
fn face_side(side: usize) -> FaceSide {
    match side {
        0 => FaceSide::UMin,
        1 => FaceSide::UMax,
        2 => FaceSide::VMin,
        3 => FaceSide::VMax,
        _ => panic!("a face has exactly four sides"),
    }
}

/// Whether two edge endpoint pairs are the SAME edge (same point set).
fn same_edge(a: ([f64; 3], [f64; 3]), b: ([f64; 3], [f64; 3])) -> bool {
    (a.0 == b.0 && a.1 == b.1) || (a.0 == b.1 && a.1 == b.0) // H-3: exact identity equality of the shared corner coordinates
}

/// The cross product of two 3-vectors.
fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// All four outward-normal bilinear parametrizations of one affine square
/// patch (one per choice of the `(u, v) = (0, 0)` corner).
fn outward_variants(
    base: [f64; 3],
    du: [f64; 3],
    dv: [f64; 3],
) -> Vec<([f64; 3], [f64; 3], [f64; 3])> {
    let c = corners(base, du, dv);
    let n_ref = cross3(du, dv);
    let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let mut out = Vec::new();
    for u0 in 0..2 {
        for v0 in 0..2 {
            let o = c[u0][v0];
            let e1 = sub(c[1 - u0][v0], o);
            let e2 = sub(c[u0][1 - v0], o);
            let dot = cross3(e1, e2);
            let aligned = dot[0] * n_ref[0] + dot[1] * n_ref[1] + dot[2] * n_ref[2] > 0.0; // H-3: sign of the cross product of exact axis-aligned edge vectors
            if aligned {
                out.push((o, e1, e2));
            } else {
                out.push((o, e2, e1));
            }
        }
    }
    out
}

/// The built strata of an axis-aligned box shell: `base` is the box's low
/// corner and `dims` its extents. The six faces are parametrized with their
/// outward normals in the fixed outward single-handed order `+z, -z, +x, -x,
/// +y, -y` — every seam's boundary walks run in opposite directions, so
/// CC-022's rim discharge certifies all twelve closed stars. `flip_top`
/// replaces the `+z` face with a parametrization whose normal is REVERSED (an
/// inward nesting), a provenance inconsistency the S1′ orientation check must
/// catch.
fn box_strata_and_glue(
    base: [f64; 3],
    dims: [f64; 3],
    t: f64,
    flip_top: bool,
) -> (Vec<OffsetStratum>, GluePlan) {
    let [lx, ly, lz] = dims;
    let ox = base[0];
    let oy = base[1];
    let oz = base[2];
    let b = |x: f64, y: f64, z: f64| [ox + x, oy + y, oz + z];
    let refs: Vec<([f64; 3], [f64; 3], [f64; 3])> = vec![
        (b(0.0, 0.0, lz), [lx, 0.0, 0.0], [0.0, ly, 0.0]),
        (b(0.0, ly, 0.0), [lx, 0.0, 0.0], [0.0, -ly, 0.0]),
        (b(lx, 0.0, 0.0), [0.0, ly, 0.0], [0.0, 0.0, lz]),
        (b(0.0, 0.0, lz), [0.0, ly, 0.0], [0.0, 0.0, -lz]),
        (b(0.0, ly, 0.0), [0.0, 0.0, lz], [lx, 0.0, 0.0]),
        (b(0.0, 0.0, lz), [0.0, 0.0, -lz], [lx, 0.0, 0.0]),
    ];
    // The single-handed parametrization choice (per-face variant index) under
    // which CC-022's rim discharge certifies all twelve closed stars.
    let win = [0usize, 1, 1, 0, 2, 0];
    let mut defs: Vec<([f64; 3], [f64; 3], [f64; 3])> = Vec::new();
    for (face, (r_base, du, dv)) in refs.iter().enumerate() {
        defs.push(outward_variants(*r_base, *du, *dv)[win[face]]);
    }
    if flip_top {
        // The +z face with its normal REVERSED: the same top square, mapped
        // with `S_u × S_v = -z`.
        defs[0] = (b(0.0, ly, lz), [lx, 0.0, 0.0], [0.0, -ly, 0.0]);
    }
    let strata: Vec<OffsetStratum> = defs
        .iter()
        .map(|(base, du, dv)| {
            face_stratum(&admitted(*base, *du, *dv), t)
                .expect("the flat box face stratum certifies at this offset")
        })
        .collect();
    let lattices: Vec<[[[f64; 3]; 2]; 2]> = defs
        .iter()
        .map(|(base, du, dv)| corners(*base, *du, *dv))
        .collect();
    let glue = shell_glue(&lattices);
    (strata, glue)
}

/// The exact glue plan of a closed face complex: two strata are seamed along
/// the side pair whose source corner sets coincide (every edge of the complex
/// is identified pairwise by identity — the shared-feature identity tokens are
/// assigned per matched edge).
fn shell_glue(lattices: &[[[[f64; 3]; 2]; 2]]) -> GluePlan {
    let n = lattices.len();
    let mut seams: Vec<Glue> = Vec::new();
    let mut next_id = 1u64;
    for i in 0..n {
        for j in (i + 1)..n {
            for si in 0..4 {
                let ei = side_endpoints(&lattices[i], si);
                for sj in 0..4 {
                    let ej = side_endpoints(&lattices[j], sj);
                    if same_edge(ei, ej) {
                        let boundary = SharedBoundary::new(next_id);
                        next_id += 1;
                        seams.push(Glue {
                            a: BoundaryRef {
                                stratum: i,
                                side: face_side(si),
                                boundary,
                            },
                            b: BoundaryRef {
                                stratum: j,
                                side: face_side(sj),
                                boundary,
                            },
                        });
                    }
                }
            }
        }
    }
    GluePlan { seams }
}

/// A fresh budget generous enough never to bind on the completed fixtures.
fn budget() -> Budget {
    Budget::new(1 << 20, 1 << 20, 64)
}

/// Extract the `Ok` shell certificate; a refusal here is a test-bug panic.
fn cert_of(strata: Vec<OffsetStratum>, glue: &GluePlan, budget: &mut Budget) -> ShellCert {
    match certify_shell(strata, glue, budget) {
        Ok(cert) => cert,
        Err(refusal) => panic!("the shell must certify, refused: {refusal:?}"),
    }
}

/// One face stratum of a unit square sheet at height `base_z`, offset `t`
/// along its `+z` normal (open, non-seamed).
fn sheet_at(base_z: f64, t: f64) -> OffsetStratum {
    face_stratum(
        &admitted([0.0, 0.0, base_z], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        t,
    )
    .expect("the flat sheet face stratum certifies at this offset")
}

#[test]
fn pn_convex_prism_shells_certified_at_small_t() {
    // A thin convex plane-faced prism (`plane` faces only — the landed flat
    // carriers) at a small outward offset `t`. The three opposite-face pairs
    // are the only non-seam pairs: the in-plane opposite pairs are pruned by
    // the certified reach bound and the thin top/bottom pair is retained and
    // funnelled through the evidence contact funnel, which certifies the two
    // realized offset planes parallel and distinct. Every pair is Certified,
    // all twelve seam stars certify embedded, and the closed connected
    // orientation-consistent complex shells Solid.
    let (strata, glue) = box_strata_and_glue([0.0, 0.0, 0.0], [1.0, 1.0, 0.1], 0.06, false);
    let mut budget = budget();
    let cert = cert_of(strata, &glue, &mut budget);
    assert_eq!(
        cert.stars_certified, 12,
        "every seam star certifies embedded"
    );
    assert_eq!(cert.pairs.len(), 3, "the three opposite-face pairs");
    for (index, verdict) in cert.pairs.iter().enumerate() {
        assert_eq!(
            *verdict,
            ShellPairVerdict::Certified,
            "opposite pair {index} is certified contact-free"
        );
    }
    assert_eq!(cert.solid, Some(SolidOutcome::Solid));
}

#[test]
fn self_contact_pair_reports_unintended_contact() {
    // Two coincident sheets (identical source faces offset along the same
    // outward normal) realize onto the SAME offset plane over the SAME patch:
    // the retained pair's funnel certifies a Region-2 coincident contact. The
    // pair verdict is Contact — the caller refuses `UnintendedContact` — never
    // Certified, and the shell is not certified a solid (solid is None).
    let a = sheet_at(0.0, 0.3);
    let b = sheet_at(0.0, 0.3);
    let glue = GluePlan { seams: Vec::new() };
    let mut budget = budget();
    let cert = cert_of(vec![a, b], &glue, &mut budget);
    assert_eq!(cert.pairs, vec![ShellPairVerdict::Contact]);
    assert!(
        !cert.pairs.iter().any(|v| *v == ShellPairVerdict::Certified),
        "an unintended contact is never certified"
    );
    assert_eq!(cert.solid, None, "the shell did not certify a solid");
}

#[test]
fn undecided_pair_surfaces_inconclusive_never_certified() {
    // Two parallel sheets whose realized planes are certifiably disjoint — but
    // the caller's budget is exhausted, so the retained pair's funnel cannot
    // run. Budget exhaustion is Inconclusive, never Certified.
    let a = sheet_at(0.0, 0.2);
    let b = sheet_at(0.3, 0.2);
    let glue = GluePlan { seams: Vec::new() };
    let mut exhausted = Budget::new(0, 0, 0);
    let cert = cert_of(vec![a, b], &glue, &mut exhausted);
    assert_eq!(cert.pairs, vec![ShellPairVerdict::Inconclusive]);
    assert!(
        !cert.pairs.iter().any(|v| *v == ShellPairVerdict::Certified),
        "an undecided pair is never Certified"
    );
}

#[test]
fn open_complex_refuses_solid_corollary_with_typed_outcome() {
    // The two-leaf open-book wedge (CC-022's certified star): two flat leaves
    // glued along their shared crease with the remaining six sides unglued.
    // The shell certifies embedded (there are no non-seam pairs) but the
    // complex is OPEN — the S1′ corollary is refused with the typed
    // SurfaceOnly { Open } outcome, never Solid.
    let a = face_stratum(
        &admitted([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        0.0,
    )
    .expect("the flat leaf certifies at offset 0");
    let b = face_stratum(&admit_any(&tilted_leaf()), 0.0)
        .expect("the flat tilted leaf certifies at offset 0");
    let crease = SharedBoundary::new(7);
    let glue = GluePlan {
        seams: vec![Glue {
            a: BoundaryRef {
                stratum: 0,
                side: FaceSide::VMin,
                boundary: crease,
            },
            b: BoundaryRef {
                stratum: 1,
                side: FaceSide::VMax,
                boundary: crease,
            },
        }],
    };
    let mut budget = budget();
    let cert = cert_of(vec![a, b], &glue, &mut budget);
    assert_eq!(cert.stars_certified, 1, "the seam star certifies embedded");
    assert!(cert.pairs.is_empty(), "the two leaves are a seam pair");
    assert_eq!(
        cert.solid,
        Some(SolidOutcome::SurfaceOnly {
            reason: SurfaceOnlyReason::Open
        })
    );
}

#[test]
fn s1_prime_requires_closed_connected_orientable() {
    // S1′ says Solid only for a closed connected orientable complex. Each
    // pre-made check is exercised: a closed connected complex with a REVERSED
    // nesting (the top face's parametrization flipped inward) is
    // OrientationUnresolved; an open sheet is Open; a two-component complex of
    // two disjoint closed prisms is Disconnected; only the closed connected
    // orientable prism is Solid.
    let mut prism_budget = budget();
    let (prism, prism_glue) = box_strata_and_glue([0.0, 0.0, 0.0], [0.6, 0.6, 0.6], 0.08, false);
    let prism_cert = cert_of(prism, &prism_glue, &mut prism_budget);
    assert_eq!(prism_cert.solid, Some(SolidOutcome::Solid));
    assert!(
        prism_cert
            .pairs
            .iter()
            .all(|v| *v == ShellPairVerdict::Certified),
        "the convex prism's pairs are all certified"
    );

    let mut flip_budget = budget();
    let (flipped, flipped_glue) = box_strata_and_glue([0.0, 0.0, 0.0], [0.6, 0.6, 0.6], 0.08, true);
    let flipped_cert = cert_of(flipped, &flipped_glue, &mut flip_budget);
    assert_eq!(
        flipped_cert.solid,
        Some(SolidOutcome::SurfaceOnly {
            reason: SurfaceOnlyReason::OrientationUnresolved
        }),
        "a reversed nesting never certifies a solid"
    );

    let mut open_budget = budget();
    // A single open sheet (the +z face alone, no seams): its four sides are
    // unidentified, so the closure check refuses the corollary with Open.
    let (open_box, _open_glue) = box_strata_and_glue([2.0, 2.0, 2.0], [0.5, 0.5, 0.5], 0.05, false);
    let open_strata = vec![open_box[0].clone()];
    let open_plan = GluePlan { seams: Vec::new() };
    let open_cert = cert_of(open_strata, &open_plan, &mut open_budget);
    assert_eq!(
        open_cert.solid,
        Some(SolidOutcome::SurfaceOnly {
            reason: SurfaceOnlyReason::Open
        })
    );

    let mut two_budget = budget();
    let (comp_a, glue_a) = box_strata_and_glue([0.0, 0.0, 0.0], [0.5, 0.5, 0.5], 0.05, false);
    let (comp_b, glue_b) = box_strata_and_glue([4.0, 0.0, 0.0], [0.5, 0.5, 0.5], 0.05, false);
    let mut two = comp_a;
    two.extend(comp_b);
    let offset = |glue: &mut GluePlan, shift: usize| {
        for seam in &mut glue.seams {
            seam.a.stratum += shift;
            seam.b.stratum += shift;
        }
    };
    let mut plan_b = glue_b;
    offset(&mut plan_b, 6);
    let mut seams = glue_a.seams;
    seams.extend(plan_b.seams);
    let two_plan = GluePlan { seams };
    let two_cert = cert_of(two, &two_plan, &mut two_budget);
    assert_eq!(
        two_cert.solid,
        Some(SolidOutcome::SurfaceOnly {
            reason: SurfaceOnlyReason::Disconnected
        })
    );
}
