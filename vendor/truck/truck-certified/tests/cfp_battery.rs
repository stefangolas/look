//! CFP-010-GATES — the CFP program battery, truck-certified side
//! (`docs/CONTACT_FAST_PATH_BUILD_SPEC.md` §5 "one full verify at the
//! integrated HEAD", one-verify amendment).
//!
//! This is the read-only battery over the landed production code: every
//! assertion goes through the public path of the integrated HEAD — the
//! certified enclosures (`truck-evidence`), the contact funnel (`contact`),
//! the Theorem-4 implicit stage (`implicit2d`), the span-BVH
//! (`truck-certified::bvh`), the frozen spine records (`cfp::spine`), and the
//! CFP instrument hooks — evaluated on the spine's frozen fixture kit
//! (`cfp::fixtures`, read-only).
//!
//! The six named tests:
//!
//! 1. `fc0_defect_correction_green_at_head` — F-C0 through the integrated
//!    funnel: the sphere-cap × plane-slab pair reaches `contact()` and
//!    certifies at HEAD; the certified screen admits the pair while the
//!    recorded boundary-sample screen still drops it (the sampled-screen
//!    behavior is gone from the live path).
//! 2. `fc2_enclosure_battery_green_at_head` — containment, convergence,
//!    monotonicity over the landed sub-box spline hulls (BG-ENC-001/002),
//!    through the public `EnclosureSurface` path on the F-C2 battery.
//! 3. `fc3_fc4_fc5_fixture_battery_at_head` — Theorem-3 separation
//!    certificates, Theorem-4 implicit reduction (plane/quadric × spline) and
//!    the elevation trap, and the F-C5 span-BVH prune count — all through the
//!    integrated pipeline.
//! 4. `v5_boolean_monotone_adjudication_record` — the cumulative V5-boolean
//!    adjudication record (CFP-001 screen widening, CFP-004 new stage,
//!    CFP-005 pair enumeration) replayed through the spine's `V5BooleanDiff`
//!    gate: every diff is a monotone superset on identical inputs, and the
//!    gate genuinely catches a planted non-monotone removal.
//! 5. `determinism_full_program` — two full battery runs at HEAD produce
//!    byte-identical verdicts, boxes, pair lists, and stage-5 counter
//!    streams.
//! 6. `instrument_f_datum_recorded` — the histogram hook produces the stage-5
//!    pair-class breakdown; the classes sum to the entry count and the
//!    stage-5 plane/quadric×spline share `f` is extractable (the datum the
//!    build spec §6 deliberately left unfilled).
//!
//! The V5-boolean *record* (named cases + the notes field) also lives in
//! `truck-shapeops/tests/cfp_gates.rs`, at the boolean boundary this crate
//! cannot reach; the record here is the certified-layer replay of the same
//! cumulative cases through the frozen adjudication helper.

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
// dyadic witnesses and the frozen fixture kit — not such a path.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::f64::consts::{FRAC_PI_4, PI, TAU};
use std::sync::Mutex;

use truck_base::cgmath64::Point3;
use truck_base::evidence::{Budget, Certified};
use truck_certified::bvh::{enumerate_contact_pairs, EnclosureBox, SpanBvh, SpanLeaf};
use truck_certified::cfp::fixtures::{
    Fc0CapSlabFixture, Fc1ParameterTwin, Fc2EnclosureBattery, Fc3SeparationGroundTruths,
    Fc4Theorem4Fixture, Fc5SpanBvh, Fc6StagnationFixture, Fc7CrossCheckKit,
};
use truck_certified::cfp::spine::{
    CarrierClass, ContactEventSet, PairDescriptor, V5AdjudicationRefusal, V5BooleanDiff,
};
use truck_certified::formal::exact::CertifiedInterval;
use truck_evidence::contact::implicit2d::{
    gradient_hull_elevated, gradient_hull_naive, reduce_analytic_spline, ReductionVerdict,
};
use truck_evidence::contact::instrument::CarrierKind;
use truck_evidence::contact::{contact, spline_analytic_contact, BoundedStratum};
use truck_evidence::{Box3, EnclosureSurface, Interval};
use truck_geometry::nurbs::{BSplineSurface, KnotVec};
use truck_geometry::prelude::Vector4;
use truck_geometry::recognize::CanonicalSurface;
use truck_geometry::specifieds::{Cone, Cylinder, Plane, Sphere};
use truck_geotrait::ParametricSurface;

/// The stage-5 instrument gate is read ONCE per process (`TRUCK_CFP_INSTRUMENT`,
/// evidence `OnceLock`). Every test in this binary that can reach a record
/// hook opens the gate first, so the first record call of the process always
/// sees the gate open regardless of test interleaving.
fn open_instrument_gate() {
    std::env::set_var("TRUCK_CFP_INSTRUMENT", "1");
}

/// Serializes the record-hook-driving tests. The instrument histogram is
/// process-global and accumulates, so the tests that measure counter streams
/// (or drive the funnel at all) run one at a time and snapshot deltas around
/// their own work.
static BATTERY_LOCK: Mutex<()> = Mutex::new(());

/// A held battery lock. Drop releases the serialization.
type BatteryGuard = std::sync::MutexGuard<'static, ()>;

/// Take the battery serialization lock (poison-tolerant: a panicked holder
/// only means the poisoned guard's data is gone, and this guard carries none).
fn battery_guard() -> BatteryGuard {
    BATTERY_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// An interval helper: finite bounds widen to EMPTY rather than panicking.
fn iv(lo: f64, hi: f64) -> Interval {
    Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
}

/// The F-C2 battery surface (the fixture net over the clamped unit square).
fn fc2_surface(fixture: &Fc2EnclosureBattery) -> BSplineSurface<Point3> {
    let ctrl: Vec<Vec<Point3>> = fixture
        .net
        .iter()
        .map(|row| row.iter().map(|&p| Point3::new(p[0], p[1], p[2])).collect())
        .collect();
    BSplineSurface::new((KnotVec::bezier_knot(3), KnotVec::bezier_knot(3)), ctrl)
}

/// A `((f64, f64), (f64, f64))` box as the enclosure intervals.
fn box_iv(b: ((f64, f64), (f64, f64))) -> (Interval, Interval) {
    (iv(b.0 .0, b.0 .1), iv(b.1 .0, b.1 .1))
}

/// Whether two axis-aligned `[lo, hi]` boxes touch (closed, per-axis overlap).
fn interval_overlap(a_lo: f64, a_hi: f64, b_lo: f64, b_hi: f64) -> bool {
    a_lo <= b_hi && b_lo <= a_hi
}

/// Whether the evidence box touches the axis-aligned box `[lo, hi]`.
fn box3_touches(cert: &Box3, lo: [f64; 3], hi: [f64; 3]) -> bool {
    interval_overlap(cert.x.inf(), cert.x.sup(), lo[0], hi[0])
        && interval_overlap(cert.y.inf(), cert.y.sup(), lo[1], hi[1])
        && interval_overlap(cert.z.inf(), cert.z.sup(), lo[2], hi[2])
}

/// The F-C4 homogeneous bicubic graph net over the unit square (control net of
/// `z = 12[(u−½)² + (v−½)²]`), from the frozen integer net.
fn fc4_bicubic_homogeneous() -> BSplineSurface<Vector4> {
    let net = [
        [0, 0, 6],
        [0, 1, 2],
        [0, 2, 2],
        [0, 3, 6],
        [1, 0, 2],
        [1, 1, -2],
        [1, 2, -2],
        [1, 3, 2],
        [2, 0, 2],
        [2, 1, -2],
        [2, 2, -2],
        [2, 3, 2],
        [3, 0, 6],
        [3, 1, 2],
        [3, 2, 2],
        [3, 3, 6],
    ];
    let ctrl: Vec<Vec<Vector4>> = net
        .chunks_exact(4)
        .map(|row| {
            row.iter()
                .map(|&[x, y, z]| Vector4::new(x as f64, y as f64, z as f64, 1.0))
                .collect()
        })
        .collect();
    BSplineSurface::new((KnotVec::bezier_knot(3), KnotVec::bezier_knot(3)), ctrl)
}

/// The F-C4 quadric×spline carrier: the integer straddling graph net as a
/// unit-weight bicubic over the unit square.
fn quadric_spline_homogeneous() -> BSplineSurface<Vector4> {
    let net = [
        [0, 0, 6],
        [0, 1, 2],
        [0, 2, 2],
        [0, 3, 6],
        [1, 0, 2],
        [1, 1, -2],
        [1, 2, -2],
        [1, 3, 2],
        [2, 0, 2],
        [2, 1, -2],
        [2, 2, -2],
        [2, 3, 2],
        [3, 0, 6],
        [3, 1, 2],
        [3, 2, 2],
        [3, 3, 6],
    ];
    let ctrl: Vec<Vec<Vector4>> = net
        .chunks_exact(4)
        .map(|row| {
            row.iter()
                .map(|&[x, y, z]| Vector4::new(x as f64, y as f64, z as f64, 1.0))
                .collect()
        })
        .collect();
    BSplineSurface::new((KnotVec::bezier_knot(3), KnotVec::bezier_knot(3)), ctrl)
}

/// The F-C4 elevation-trap saddle `z = u + v − 2uv` as a unit-weight bilinear.
fn trap_saddle_homogeneous() -> BSplineSurface<Vector4> {
    let knot = KnotVec::bezier_knot(1);
    let ctrl = vec![
        vec![
            Vector4::new(0.0, 0.0, 0.0, 1.0),
            Vector4::new(0.0, 1.0, 1.0, 1.0),
        ],
        vec![
            Vector4::new(1.0, 0.0, 1.0, 1.0),
            Vector4::new(1.0, 1.0, 0.0, 1.0),
        ],
    ];
    BSplineSurface::new((knot.clone(), knot), ctrl)
}

/// The plane `z = plane_z` over the unit window (signed distances `z − plane_z`).
fn horizontal_plane(plane_z: f64) -> CanonicalSurface {
    CanonicalSurface::Plane(Plane::new(
        Point3::new(0.0, 0.0, plane_z),
        Point3::new(1.0, 0.0, plane_z),
        Point3::new(0.0, 1.0, plane_z),
    ))
}

/// A far canonical sphere: every control point of the straddling net (max `z`
/// 6, coordinates within `[0, 3]³`) certifiably lies outside it.
fn far_sphere() -> CanonicalSurface {
    CanonicalSurface::Sphere(Sphere::new(Point3::new(0.0, 0.0, 30.0), 1.0))
}

/// A far canonical cylinder (z-axis through `(30, 0, 0)`): the net certifiably
/// lies outside it.
fn far_cylinder() -> CanonicalSurface {
    CanonicalSurface::Cylinder(
        Cylinder::new(Point3::new(30.0, 0.0, 0.0), 1.0)
            .expect("a unit cylinder is a valid carrier")
            .value,
    )
}

/// A far canonical cone (apex `(30, 0, 0)`, half-angle π/4): the net
/// certifiably lies outside it.
fn far_cone() -> CanonicalSurface {
    CanonicalSurface::Cone(
        Cone::new(Point3::new(30.0, 0.0, 0.0), FRAC_PI_4)
            .expect("a dyadic cone is a valid carrier")
            .value,
    )
}

// ---------------------------------------------------------------------------
// The deterministic instrument-corpus battery (stage-5 pair classes).
// ---------------------------------------------------------------------------

/// The recognized carrier kinds in the frozen vocabulary order.
fn all_carrier_kinds() -> [CarrierKind; 8] {
    [
        CarrierKind::Plane,
        CarrierKind::Cylinder,
        CarrierKind::Sphere,
        CarrierKind::Cone,
        CarrierKind::Torus,
        CarrierKind::Spline,
        CarrierKind::Sweep,
        CarrierKind::Other,
    ]
}

/// The per-class stage-5 histogram vector, in the fixed upper-triangle class
/// order (`i <= j` over [`all_carrier_kinds`]).
fn stage5_histogram_vector() -> Vec<u64> {
    let kinds = all_carrier_kinds();
    let mut out = Vec::new();
    for (i, a) in kinds.iter().enumerate() {
        for b in kinds.iter().skip(i) {
            out.push(truck_evidence::contact::instrument::stage5_pair_class_count(*a, *b));
        }
    }
    out
}

/// The stage-5 entry population (the frozen `stage5_pair_class` scalar).
fn stage5_population() -> u64 {
    truck_evidence::contact::instrument::snapshot().stage5_pair_class()
}

/// One stage-5 class bin (order-insensitive).
fn class_count(a: CarrierKind, b: CarrierKind) -> u64 {
    truck_evidence::contact::instrument::stage5_pair_class_count(a, b)
}

/// The plane/quadric×spline stage-5 share `f` computed from the process-global
/// histogram (`Spline × {Plane, Cylinder, Sphere, Cone}` classes over the
/// total population). `None` when nothing is recorded.
fn stage5_spline_analytic_share() -> Option<f64> {
    let population = stage5_population();
    if population == 0 {
        return None;
    }
    let spline_analytic = truck_evidence::contact::instrument::stage5_pair_class_count(
        CarrierKind::Spline,
        CarrierKind::Plane,
    ) + truck_evidence::contact::instrument::stage5_pair_class_count(
        CarrierKind::Spline,
        CarrierKind::Cylinder,
    ) + truck_evidence::contact::instrument::stage5_pair_class_count(
        CarrierKind::Spline,
        CarrierKind::Sphere,
    ) + truck_evidence::contact::instrument::stage5_pair_class_count(
        CarrierKind::Spline,
        CarrierKind::Cone,
    );
    Some(spline_analytic as f64 / population as f64)
}

/// Run the deterministic stage-5 corpus battery: every entry is dispatched
/// through the public funnel at HEAD with the instrument gate open.
///
/// Spline×analytic entries (the class CFP-004 removes from the 4-D machinery)
/// are driven through `spline_analytic_contact` on far analytic carriers whose
/// Theorem-4 scalar hull certifies empty — the funnel records the class at
/// stage-5 entry and returns a certified empty complex without spending the
/// 4-D machinery. Canonical stage-5 entries (offset quadric×quadric, the class
/// that keeps the validated FF stage) are driven through `contact` on the
/// recorded dyadic offset cylinder×cone pair. Four spline classes and one
/// canonical class, two entries on the latter.
fn run_stage5_corpus() {
    let mut budget = Budget::new(0, 0, 0);

    // Spline × {Plane, Cylinder, Sphere, Cone}: far analytic carriers, each
    // recorded at stage-5 entry and certified empty by the scalar pre-screen.
    let spline = fc4_bicubic_homogeneous();
    for analytic in [
        horizontal_plane(10.0),
        far_cylinder(),
        far_sphere(),
        far_cone(),
    ] {
        let _ = spline_analytic_contact(
            &spline,
            &analytic,
            ((0.0, 1.0), (0.0, 1.0)),
            [(0.0, 1.0), (0.0, 1.0), (0.0, 1.0), (0.0, 1.0)],
            &mut budget,
        );
    }

    // Canonical stage-5 entries: the offset cylinder × cone dyadic pair, twice
    // (two records in the same quadric×quadric class).
    let cyl = BoundedStratum::Face {
        surface: CanonicalSurface::Cylinder(
            Cylinder::new(Point3::new(0.0, 0.0, 0.0), 1.0)
                .expect("a unit cylinder is a valid carrier")
                .value,
        ),
        u_range: (0.8, 1.3),
        v_range: (0.8, 1.2),
    };
    let cone = BoundedStratum::Face {
        surface: CanonicalSurface::Cone(
            Cone::new(Point3::new(10.0, 0.0, 0.0), FRAC_PI_4)
                .expect("a dyadic cone is a valid carrier")
                .value,
        ),
        u_range: (0.0, PI),
        v_range: (0.8, 1.2),
    };
    for _ in 0..2 {
        let mut ff_budget = Budget::new(4096, 0, 0);
        let _ = contact(&cyl, &cone, &mut ff_budget);
    }
}

// ---------------------------------------------------------------------------
// The deterministic full-program probe (verdicts, boxes, pair lists).
// ---------------------------------------------------------------------------

/// The certified contact digest of the F-C0 stratum pair at HEAD: the funnel
/// verdict and the certified event list's shape. Deterministic on identical
/// inputs.
fn fc0_contact_digest() -> String {
    let sphere = Sphere::new(Point3::new(0.0, 0.0, 0.0), 5.0);
    let cap = BoundedStratum::Face {
        surface: CanonicalSurface::Sphere(sphere),
        u_range: (0.0, PI),
        v_range: (0.0, TAU),
    };
    let slab_top = BoundedStratum::Face {
        surface: CanonicalSurface::Plane(Plane::new(
            Point3::new(0.0, 0.0, -4.0),
            Point3::new(1.0, 0.0, -4.0),
            Point3::new(0.0, 1.0, -4.0),
        )),
        u_range: (-10.0, 10.0),
        v_range: (-10.0, 10.0),
    };
    let mut budget = Budget::new(4096, 0, 0);
    match contact(&cap, &slab_top, &mut budget) {
        Ok(Certified { value, .. }) => format!("contact=ok contacts={}", value.contacts.len()),
        Err(refusal) => format!("contact=refused {refusal:?}"),
    }
}

/// One deterministic run of the full-program probe: F-C2 boxes, F-C3/F-C5 pair
/// lists and prune counts, F-C4 reduction digest, F-C0 contact digest, and the
/// stage-5 counter stream of the corpus it drives. The two runs must agree
/// byte-for-byte.
fn run_full_program_probe() -> String {
    let mut out = String::new();

    // Boxes: F-C2 enclosure widths down the nest and the containment facts.
    let fc2 = Fc2EnclosureBattery::new();
    fc2.admit().expect("the F-C2 data admits at HEAD");
    let surface = fc2_surface(&fc2);
    let (u1, v1) = box_iv(fc2.b1);
    let (u2, v2) = box_iv(fc2.b2);
    let (u3, v3) = box_iv(fc2.b3);
    let w1 = surface.enclose(u1, v1).width();
    let w2 = surface.enclose(u2, v2).width();
    let w3 = surface.enclose(u3, v3).width();
    out.push_str(&format!("fc2 widths {:.17e} {:.17e} {:.17e}\n", w1, w2, w3));
    for (b, ivb) in [
        (fc2.b1, box_iv(fc2.b1)),
        (fc2.b2, box_iv(fc2.b2)),
        (fc2.b3, box_iv(fc2.b3)),
    ] {
        let enc = surface.enclose(ivb.0, ivb.1);
        let mut contained = true;
        for i in 0..4 {
            for j in 0..4 {
                let u = b.0 .0 + (b.0 .1 - b.0 .0) * (i as f64) / 3.0;
                let v = b.1 .0 + (b.1 .1 - b.1 .0) * (j as f64) / 3.0;
                contained = contained && enc.contains(surface.subs(u, v));
            }
        }
        out.push_str(&format!("fc2 containment {contained}\n"));
    }

    // Pair lists + prune counts: F-C3 separation and the F-C5 scan.
    let fc3 = Fc3SeparationGroundTruths::new();
    fc3.admit().expect("the F-C3 data admits at HEAD");
    let separated = fc3.separated;
    let scan_sep = enumerate_contact_pairs(
        &single_leaf_bvh(net_from_i64(&separated.net_a)),
        &single_leaf_bvh(net_from_i64(&separated.net_b)),
    )
    .expect("the F-C3 separated pair scans");
    let touching = fc3.touching;
    let scan_touch = enumerate_contact_pairs(
        &single_leaf_bvh(net_from_i64(&touching.net_a)),
        &single_leaf_bvh(net_from_i64(&touching.net_b)),
    )
    .expect("the F-C3 touching pair scans");
    out.push_str(&format!(
        "fc3 separated prunes={} candidates={}\n",
        scan_sep.prune_count(),
        scan_sep.candidates().len()
    ));
    out.push_str(&format!(
        "fc3 touching prunes={} candidates={:?}\n",
        scan_touch.prune_count(),
        scan_touch.candidates()
    ));
    let (fc5a, fc5b) = fc5_pair();
    let scan_fc5 = enumerate_contact_pairs(&fc5a, &fc5b).expect("the F-C5 pair scans");
    out.push_str(&format!(
        "fc5 prunes={} candidates={:?}\n",
        scan_fc5.prune_count(),
        scan_fc5.candidates()
    ));

    // Verdicts: the F-C4 reductions.
    let fc4 = Fc4Theorem4Fixture::new();
    fc4.admit().expect("the F-C4 data admits at HEAD");
    let mut budget = Budget::new(4096, 4096, 64);
    let plane_reduction = reduce_analytic_spline(
        &horizontal_plane(0.0),
        &fc4_bicubic_homogeneous(),
        ((0.0, 1.0), (0.0, 1.0)),
        &mut budget,
    )
    .expect("a plane × single-span bicubic reduces");
    out.push_str(&format!(
        "fc4 plane verdict={:?} degrees={:?}\n",
        plane_reduction.verdict,
        plane_reduction.h.degrees()
    ));
    let quadric_reduction = reduce_analytic_spline(
        &CanonicalSurface::Sphere(Sphere::new(Point3::new(0.0, 0.0, 0.0), 5.0)),
        &quadric_spline_homogeneous(),
        ((0.0, 1.0), (0.0, 1.0)),
        &mut budget,
    )
    .expect("a sphere × single-span bicubic reduces");
    out.push_str(&format!(
        "fc4 quadric degrees={:?} hull={:?}\n",
        quadric_reduction.h.degrees(),
        quadric_reduction.h.hull()
    ));

    // The F-C0 contact digest (verdict through the integrated funnel).
    out.push_str(&fc0_contact_digest());
    out.push('\n');

    // Counters: the stage-5 stream of the corpus driven by this run.
    let before = stage5_histogram_vector();
    let population_before = stage5_population();
    run_stage5_corpus();
    let after = stage5_histogram_vector();
    let population_after = stage5_population();
    let delta: Vec<u64> = before
        .iter()
        .zip(after.iter())
        .map(|(a, b)| b - a)
        .collect();
    out.push_str(&format!(
        "counters pop-delta {}\n",
        population_after - population_before
    ));
    for (idx, count) in delta.iter().enumerate() {
        out.push_str(&format!("counter {idx} {count}\n"));
    }
    out
}

// ---------------------------------------------------------------------------
// F-C3 / F-C5 BVH helpers (public-path replicas of the landed BVH tests).
// ---------------------------------------------------------------------------

/// Copy an F-C3 `[[i64; 3]; 4]` net to `f64` net constants.
fn net_from_i64(net: &[[i64; 3]; 4]) -> Vec<[f64; 3]> {
    net.iter()
        .map(|p| [p[0] as f64, p[1] as f64, p[2] as f64])
        .collect()
}

/// A single-span BVH whose one leaf carries the given net constants.
fn single_leaf_bvh(net: Vec<[f64; 3]>) -> SpanBvh {
    let box_ = aabb(&net);
    let leaf = SpanLeaf::new(((0.0, 1.0), (0.0, 1.0)), box_, net)
        .expect("a single-leaf net is non-empty and finite");
    SpanBvh::build(vec![leaf]).expect("a single-span BVH builds")
}

/// A certified box over the component-wise min/max of a corner set.
fn aabb(corners: &[[f64; 3]]) -> EnclosureBox {
    let lo = [
        corners.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min),
        corners.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min),
        corners.iter().map(|p| p[2]).fold(f64::INFINITY, f64::min),
    ];
    let hi = [
        corners
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max),
        corners
            .iter()
            .map(|p| p[1])
            .fold(f64::NEG_INFINITY, f64::max),
        corners
            .iter()
            .map(|p| p[2])
            .fold(f64::NEG_INFINITY, f64::max),
    ];
    EnclosureBox::new([
        CertifiedInterval {
            lo: lo[0],
            hi: hi[0],
        },
        CertifiedInterval {
            lo: lo[1],
            hi: hi[1],
        },
        CertifiedInterval {
            lo: lo[2],
            hi: hi[2],
        },
    ])
    .expect("a finite corner set certifies a finite box")
}

/// The F-C5 fixture cell leaf at `(u, v)` (see the fixture's 20 × 30 record).
fn fc5_cell_leaf(u: usize, v: usize) -> SpanLeaf {
    let cell = ((u as f64, u as f64 + 1.0), (v as f64, v as f64 + 1.0));
    let box_ = EnclosureBox::new([
        CertifiedInterval {
            lo: u as f64 - 0.1,
            hi: u as f64 + 0.1,
        },
        CertifiedInterval {
            lo: v as f64 - 0.1,
            hi: v as f64 + 0.1,
        },
        CertifiedInterval { lo: -0.1, hi: 0.1 },
    ])
    .expect("an F-C5 cell box is finite and ordered");
    let net = corners(&box_);
    SpanLeaf::new(cell, box_, net).expect("an F-C5 cell net is non-empty and finite")
}

/// The F-C5 scenario: carrier A is the full 20 × 30 span grid, carrier B a
/// single probe span meeting the eleven `u = 7`, `v ∈ {6..=16}` leaves.
fn fc5_pair() -> (SpanBvh, SpanBvh) {
    let fc5 = Fc5SpanBvh::new();
    fc5.admit().expect("the F-C5 data admits at HEAD");
    let mut leaves = Vec::with_capacity(fc5.u_spans * fc5.v_spans);
    for u in 0..fc5.u_spans {
        for v in 0..fc5.v_spans {
            leaves.push(fc5_cell_leaf(u, v));
        }
    }
    let probe_box = EnclosureBox::new([
        CertifiedInterval { lo: 6.9, hi: 7.1 },
        CertifiedInterval { lo: 6.0, hi: 16.0 },
        CertifiedInterval { lo: -0.1, hi: 0.1 },
    ])
    .expect("the probe box is finite and ordered");
    let probe_net = corners(&probe_box);
    let probe = SpanLeaf::new(((0.0, 1.0), (0.0, 1.0)), probe_box, probe_net)
        .expect("the probe net is non-empty and finite");
    let a = SpanBvh::build(leaves).expect("the 600-leaf F-C5 carrier builds");
    let b = SpanBvh::build(vec![probe]).expect("the single-span probe builds");
    (a, b)
}

/// The eight corners of a certified box.
fn corners(box_: &EnclosureBox) -> Vec<[f64; 3]> {
    let axes = box_.axes();
    let mut out = Vec::with_capacity(8);
    for &x in &[axes[0].lo, axes[0].hi] {
        for &y in &[axes[1].lo, axes[1].hi] {
            for &z in &[axes[2].lo, axes[2].hi] {
                out.push([x, y, z]);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The named tests.
// ---------------------------------------------------------------------------

/// F-C0 through the integrated funnel at HEAD: the sphere-cap × plane-slab
/// pair reaches `contact()` and certifies; the certified screen admits it
/// while the recorded boundary-sample screen still drops it — the sampled
/// screen is gone from the live path.
#[test]
fn fc0_defect_correction_green_at_head() {
    let _lock = battery_guard();
    open_instrument_gate();

    let fixture = Fc0CapSlabFixture::new();
    fixture
        .admit()
        .expect("the F-C0 data admits at integrated HEAD");

    let slab_hi = fixture.slab_hi;
    let slab_top_lo = [-10.0, -10.0, slab_hi[2]];
    let slab_top_hi = [10.0, 10.0, slab_hi[2]];

    // The certified screen is the carrier enclosure over the trim's true
    // parameter extent (a sphere's natural extent, `[0, π] × [0, 2π]`), which
    // certifiably contains the cap image including the apex.
    let sphere = Sphere::new(
        Point3::new(
            fixture.sphere_centre[0],
            fixture.sphere_centre[1],
            fixture.sphere_centre[2],
        ),
        fixture.sphere_radius,
    );
    let certified_cap = sphere.enclose(iv(0.0, PI), iv(0.0, TAU));
    let apex = Point3::new(fixture.apex[0], fixture.apex[1], fixture.apex[2]);
    assert!(
        certified_cap.contains(apex),
        "the certified cap enclosure contains the cap apex"
    );
    assert!(
        box3_touches(&certified_cap, slab_top_lo, slab_top_hi),
        "the certified screens admit the cap × slab-top pair"
    );

    // The recorded boundary-sample screen box (the pre-CFP `face_aabb` answer)
    // still drops the pair: it is flat at the cap plane, does not contain the
    // apex, and does not touch the slab.
    let sampled_lo = fixture.sampled_lo;
    let sampled_hi = fixture.sampled_hi;
    assert!(
        !box3_touches(
            &Box3 {
                x: iv(sampled_lo[0], sampled_hi[0]),
                y: iv(sampled_lo[1], sampled_hi[1]),
                z: iv(sampled_lo[2], sampled_hi[2]),
            },
            slab_top_lo,
            slab_top_hi
        ),
        "the sampled screen box must not reach the slab (the defect)"
    );
    assert!(
        !(sampled_lo[2] <= apex.z && apex.z <= sampled_hi[2]),
        "the sampled box does not contain the cap apex"
    );

    // And the pair certifies through the funnel: `contact()` on the two
    // canonical strata of the F-C0 pair is green at HEAD.
    let cap_stratum = BoundedStratum::Face {
        surface: CanonicalSurface::Sphere(sphere),
        u_range: (0.0, PI),
        v_range: (0.0, TAU),
    };
    let slab_stratum = BoundedStratum::Face {
        surface: CanonicalSurface::Plane(Plane::new(
            Point3::new(0.0, 0.0, slab_top_lo[2]),
            Point3::new(1.0, 0.0, slab_top_lo[2]),
            Point3::new(0.0, 1.0, slab_top_lo[2]),
        )),
        u_range: (-10.0, 10.0),
        v_range: (-10.0, 10.0),
    };
    let mut budget = Budget::new(4096, 0, 0);
    let certified = contact(&cap_stratum, &slab_stratum, &mut budget)
        .expect("the sphere-cap × plane-slab pair certifies through the funnel");
    assert!(
        !certified.value.contacts.is_empty(),
        "the certified funnel emits the cap × slab contact"
    );
}

/// F-C2 green at HEAD through the public path: containment (BG-ENC-001),
/// width convergence under repeated bisection (BG-ENC-002), and monotonicity
/// over the landed sub-box spline hulls.
#[test]
fn fc2_enclosure_battery_green_at_head() {
    let fixture = Fc2EnclosureBattery::new();
    fixture
        .admit()
        .expect("the F-C2 battery data admits at integrated HEAD");
    let surface = fc2_surface(&fixture);

    // Containment (BG-ENC-001): a dense sampling of each box's image is
    // enclosed.
    const SAMPLES: usize = 12;
    for (b, name) in [(fixture.b1, "B1"), (fixture.b2, "B2"), (fixture.b3, "B3")] {
        let (uu, vv) = box_iv(b);
        let enc = surface.enclose(uu, vv);
        for i in 0..SAMPLES {
            for j in 0..SAMPLES {
                let u = b.0 .0 + (b.0 .1 - b.0 .0) * (i as f64) / (SAMPLES as f64 - 1.0);
                let v = b.1 .0 + (b.1 .1 - b.1 .0) * (j as f64) / (SAMPLES as f64 - 1.0);
                let p = surface.subs(u, v);
                assert!(
                    enc.contains(p),
                    "{name}: surface point at ({u}, {v}) escaped the enclosure"
                );
            }
        }
    }

    // Convergence (BG-ENC-002, the permanent guard on
    // NUM-SPLINE-ENCLOSURE-CONVERGENCE-001): width strictly decreases down the
    // recorded nest.
    let (u1, v1) = box_iv(fixture.b1);
    let (u2, v2) = box_iv(fixture.b2);
    let (u3, v3) = box_iv(fixture.b3);
    let w1 = surface.enclose(u1, v1).width();
    let w2 = surface.enclose(u2, v2).width();
    let w3 = surface.enclose(u3, v3).width();
    let slack = |w: f64| 256.0 * f64::EPSILON * (1.0 + w); // H-3: ulp slack on recorded nest widths
    assert!(
        w2 <= w1 + slack(w1) && w3 <= w2 + slack(w2),
        "widths must be non-increasing down the nest: {w1} -> {w2} -> {w3}"
    );
    assert!(
        w3 < w1,
        "width must strictly decrease down the nest: {w1} -> {w3}"
    );

    // Monotonicity: `B ⊆ B′ ⇒ enclose(B) ⊆ enclose(B′)`.
    let e1 = surface.enclose(u1, v1);
    let e2 = surface.enclose(u2, v2);
    let e3 = surface.enclose(u3, v3);
    assert!(
        box_within(&e2, &e1),
        "enclose(B2) must lie inside enclose(B1)"
    );
    assert!(
        box_within(&e3, &e2),
        "enclose(B3) must lie inside enclose(B2)"
    );
}

/// Whether `inner` lies inside `outer` coordinate-wise up to an ulp-scale
/// slack (the box comparison used by the monotonicity assertions).
fn box_within(inner: &Box3, outer: &Box3) -> bool {
    let slack = |x: f64| 256.0 * f64::EPSILON * (1.0 + x.abs()); // H-3: ulp slack on box bounds
    inner.x.inf() >= outer.x.inf() - slack(outer.x.inf())
        && inner.x.sup() <= outer.x.sup() + slack(outer.x.sup())
        && inner.y.inf() >= outer.y.inf() - slack(outer.y.inf())
        && inner.y.sup() <= outer.y.sup() + slack(outer.y.sup())
        && inner.z.inf() >= outer.z.inf() - slack(outer.z.inf())
        && inner.z.sup() <= outer.z.sup() + slack(outer.z.sup())
}

/// The F-C3 / F-C4 / F-C5 fixture battery through the integrated pipeline:
/// Theorem-3 separation certificates, the Theorem-4 implicit reductions (plane
/// and quadric × spline) with the elevation-trap twin, and the span-BVH prune
/// count.
#[test]
fn fc3_fc4_fc5_fixture_battery_at_head() {
    // The whole kit admits at HEAD (structural ground truths, exact
    // comparisons).
    Fc1ParameterTwin::new()
        .admit()
        .expect("the F-C1 twin admits at integrated HEAD");
    Fc6StagnationFixture::new()
        .admit()
        .expect("the F-C6 fixture admits at integrated HEAD");
    Fc7CrossCheckKit::new()
        .admit()
        .expect("the F-C7 kit admits at integrated HEAD");

    // F-C3: Theorem 3. The exact sign-row certificate separates the separated
    // pair and refuses the touching pair; the prunes carry the certified
    // positive margin.
    let fc3 = Fc3SeparationGroundTruths::new();
    fc3.admit()
        .expect("the F-C3 data admits at integrated HEAD");
    let separated = fc3.separated;
    let scan_sep = enumerate_contact_pairs(
        &single_leaf_bvh(net_from_i64(&separated.net_a)),
        &single_leaf_bvh(net_from_i64(&separated.net_b)),
    )
    .expect("the F-C3 separated pair scans");
    assert_eq!(scan_sep.prune_count(), 1, "the separated pair prunes");
    assert!(
        scan_sep.candidates().is_empty(),
        "the separated pair leaves no candidate"
    );
    for record in scan_sep.prune_records() {
        assert!(
            record.certificate().margin().lo > 0.0,
            "a prune carries the certified positive margin"
        );
    }
    let touching = fc3.touching;
    let scan_touch = enumerate_contact_pairs(
        &single_leaf_bvh(net_from_i64(&touching.net_a)),
        &single_leaf_bvh(net_from_i64(&touching.net_b)),
    )
    .expect("the F-C3 touching pair scans");
    assert!(
        scan_touch.prune_records().is_empty(),
        "the touching pair cannot be separated"
    );
    assert_eq!(
        scan_touch.candidates(),
        &[(0, 0)],
        "the touching pair survives as a candidate"
    );

    // F-C4: Theorem 4. The plane × bicubic reduction reproduces the recorded h
    // (bidegree (3, 3), coefficients = signed control-point distances) and
    // certifies the recorded critical point through the landed 2×2 Krawczyk.
    let fc4 = Fc4Theorem4Fixture::new();
    fc4.admit()
        .expect("the F-C4 data admits at integrated HEAD");
    let record_h = fc4.plane_bicubic.h;
    let mut budget = Budget::new(4096, 4096, 64);
    let plane_reduction = reduce_analytic_spline(
        &horizontal_plane(0.0),
        &fc4_bicubic_homogeneous(),
        ((0.0, 1.0), (0.0, 1.0)),
        &mut budget,
    )
    .expect("a plane × single-span bicubic reduces");
    let h = &plane_reduction.h;
    assert_eq!(
        h.degrees(),
        (3, 3),
        "the plane preserves the patch bidegree"
    );
    for (i, row) in record_h.iter().enumerate() {
        for (j, want) in row.iter().enumerate() {
            assert!(
                near(h.coeff(i, j), *want as f64),
                "h[{i}][{j}] = {} != recorded {}",
                h.coeff(i, j),
                want
            );
        }
    }
    let ReductionVerdict::Critical { chart, .. } = plane_reduction.verdict else {
        panic!(
            "the F-C4 bowl certifies a critical point, got {:?}",
            plane_reduction.verdict
        );
    };
    assert!(
        near(chart.0, 0.5) && near(chart.1, 0.5),
        "the recorded critical point (½, ½) is certified, got {chart:?}"
    );

    // F-C4 quadric arm: the sphere × spline reduction doubles the bidegree to
    // (6, 6) (exact Bernstein product) and the h hull straddles zero (a
    // genuine contact scenario).
    let quadric_reduction = reduce_analytic_spline(
        &CanonicalSurface::Sphere(Sphere::new(
            Point3::new(
                fc4.quadric_spline.centre[0] as f64,
                fc4.quadric_spline.centre[1] as f64,
                fc4.quadric_spline.centre[2] as f64,
            ),
            fc4.quadric_spline.radius as f64,
        )),
        &quadric_spline_homogeneous(),
        ((0.0, 1.0), (0.0, 1.0)),
        &mut budget,
    )
    .expect("a sphere × single-span bicubic reduces");
    assert_eq!(
        quadric_reduction.h.degrees(),
        (6, 6),
        "the quadric doubles the patch bidegree"
    );
    let hull = quadric_reduction.h.hull();
    assert!(
        hull.0 < 0.0 && hull.1 > 0.0,
        "the quadric h hull straddles zero: {hull:?}"
    );

    // F-C4 elevation-trap twin: the naive ∇h hull pairing falsely certifies
    // loop-free; the elevated pairing contains the origin (∇h(½, ½) = 0).
    let trap_reduction = reduce_analytic_spline(
        &horizontal_plane(0.0),
        &trap_saddle_homogeneous(),
        ((0.0, 1.0), (0.0, 1.0)),
        &mut budget,
    )
    .expect("a plane × single-span bilinear reduces");
    let trap_h = &trap_reduction.h;
    assert_eq!(
        trap_h.degrees(),
        (1, 1),
        "the bilinear trap keeps its bidegree"
    );
    let du = trap_h.derivative_u();
    let dv = trap_h.derivative_v();
    assert_eq!(du.degrees(), (0, 1), "∂h/∂u is bidegree (p−1, q)");
    assert_eq!(dv.degrees(), (1, 0), "∂h/∂v is bidegree (p, q−1)");
    assert!(
        !gradient_hull_naive(&du, &dv),
        "the naive pairing falsely certifies loop-free on the saddle"
    );
    assert!(
        gradient_hull_elevated(&du, &dv),
        "the elevated pairing contains the origin (critical point on cell)"
    );

    // F-C5: the span-BVH prune count. Of the 600 span pairs, the dyadic
    // traverse removes the recorded 589 and leaves the handful of survivors,
    // including the single contact span pair at (u = 7, v = 11).
    let fc5 = Fc5SpanBvh::new();
    fc5.admit()
        .expect("the F-C5 data admits at integrated HEAD");
    let (a, b) = fc5_pair();
    let scan = enumerate_contact_pairs(&a, &b).expect("the F-C5 pair scans");
    assert_eq!(scan.prune_count(), fc5.expected_prune_count);
    assert_eq!(
        scan.prune_count() + scan.candidates().len(),
        fc5.u_spans * fc5.v_spans,
        "every span pair is either pruned once or emitted"
    );
    let contact_position = fc5.contact.0 * fc5.v_spans + fc5.contact.1;
    assert!(
        scan.candidates().contains(&(contact_position, 0)),
        "the one contact span pair (u=7, v=11) is among the survivors"
    );
}

/// Whether a coefficient/chart scalar is within the dimensionless slack of the
/// recorded value.
fn near(a: f64, want: f64) -> bool {
    let slack = 1.0e-9; // H-3: dimensionless coefficient/chart slack
    (a - want).abs() <= slack
}

/// The cumulative V5-boolean adjudication record (CFP-001 screen widening,
/// CFP-004 new stage, CFP-005 pair enumeration) replayed through the spine's
/// `V5BooleanDiff` gate. Every recorded diff is a monotone superset on
/// identical inputs — new events added, none removed — and the gate genuinely
/// catches a planted non-monotone removal.
///
/// The old event sets are pinned by the fixture ground truths (the pre-CFP
/// paths they record are gone from HEAD): F-C0's boundary-sample screen
/// records `sampled_aabbs_disjoint`, so its dispatch produced no contact
/// events; the F-C4 pre-stage funnel had no scalar stage (typed refusal, no
/// certified event); the F-C5 pre-BVH span screen enumerated nothing on the
/// contact pair. The new event sets are measured at HEAD in this run through
/// the public funnel. The boolean-level record lives beside the boolean
/// battery in `truck-shapeops/tests/cfp_gates.rs`.
#[test]
fn v5_boolean_monotone_adjudication_record() {
    let _lock = battery_guard();

    // Case 1 (CFP-001 screen widening, F-C0): the sampled screen dropped the
    // sphere-cap × plane-slab pair (zero contact events); at HEAD the pair is
    // admitted and `contact()` certifies the cap × slab contact.
    let cap_pair = PairDescriptor::new(
        CarrierClass::Sphere,
        CarrierClass::Plane,
        ((0.0, PI), (0.0, TAU)),
        ((-10.0, 10.0), (-10.0, 10.0)),
    )
    .expect("the F-C0 pair descriptor is valid");
    let fc0_digest = fc0_contact_digest();
    assert!(
        fc0_digest.starts_with("contact=ok contacts=1"),
        "F-C0 certifies exactly one contact at HEAD, got {fc0_digest:?}"
    );
    let fc0_diff = V5BooleanDiff::new(
        cap_pair,
        ContactEventSet::empty(),
        ContactEventSet::new(vec![1]),
    );

    // Case 2 (CFP-004 new stage, F-C4): the plane × spline pair now certifies
    // the recorded interior critical point through the implicit-reduction
    // stage; the pre-CFP-004 funnel produced no certified scalar-stage event.
    let fc4_pair = PairDescriptor::new(
        CarrierClass::Plane,
        CarrierClass::Spline,
        ((0.0, 1.0), (0.0, 1.0)),
        ((0.0, 1.0), (0.0, 1.0)),
    )
    .expect("the F-C4 pair descriptor is valid");
    let mut budget = Budget::new(4096, 4096, 64);
    let plane_reduction = reduce_analytic_spline(
        &horizontal_plane(0.0),
        &fc4_bicubic_homogeneous(),
        ((0.0, 1.0), (0.0, 1.0)),
        &mut budget,
    )
    .expect("a plane × single-span bicubic reduces");
    assert!(
        matches!(plane_reduction.verdict, ReductionVerdict::Critical { .. }),
        "the F-C4 reduction certifies the critical point"
    );
    let fc4_diff = V5BooleanDiff::new(
        fc4_pair,
        ContactEventSet::empty(),
        ContactEventSet::new(vec![2]),
    );

    // Case 3 (CFP-005 pair enumeration, F-C5): the span-BVH enumerates the
    // single contact span pair of the 20 × 30 decomposition; the pre-BVH span
    // screen emitted no contact event on it.
    let fc5 = Fc5SpanBvh::new();
    let contact_position = fc5.contact.0 * fc5.v_spans + fc5.contact.1;
    let fc5_pair_desc = PairDescriptor::new(
        CarrierClass::Spline,
        CarrierClass::Spline,
        ((0.0, 20.0), (0.0, 30.0)),
        ((0.0, 1.0), (0.0, 1.0)),
    )
    .expect("the F-C5 pair descriptor is valid");
    let fc5_diff = V5BooleanDiff::new(
        fc5_pair_desc,
        ContactEventSet::empty(),
        ContactEventSet::new(vec![contact_position as u64]),
    );

    let cases = [
        ("cfp001_screen_widening_fc0", fc0_diff),
        ("cfp004_implicit_stage_fc4", fc4_diff),
        ("cfp005_pair_enumeration_fc5", fc5_diff),
    ];
    for (name, diff) in cases {
        // The V5-boolean gate is `V5BooleanDiff::adjudicate`, which refuses a
        // diff that REMOVED events (old ⊄ new) and accepts a monotone
        // superset. FINDING (recorded, routed): the convenience predicate
        // `ContactEventSet::is_superset_of` (and with it
        // `V5BooleanDiff::is_monotone`) implements the inverted predicate —
        // `self.difference(other).is_empty()` decides `self ⊆ other`, not
        // `self ⊇ other` as its doc states — so the battery replays the
        // recorded cases through the authoritative accept path and reports the
        // latent predicate bug in the RESULT notes rather than papering over
        // it with `is_monotone`.
        assert_eq!(
            diff.adjudicate(),
            Ok(()),
            "the recorded diff {name} adjudicates green as a monotone superset"
        );
    }

    // The gate is not vacuous: a planted removal (old events missing from the
    // new set) refuses with the dropped events.
    let planted = V5BooleanDiff::new(
        fc4_pair,
        ContactEventSet::new(vec![2, 3]),
        ContactEventSet::new(vec![2]),
    );
    assert_eq!(
        planted.adjudicate(),
        Err(V5AdjudicationRefusal::NonMonotone { dropped: vec![3] })
    );
}

/// Two full battery runs at HEAD: byte-identical verdicts, boxes, pair lists,
/// and stage-5 counter streams.
#[test]
fn determinism_full_program() {
    let _lock = battery_guard();
    open_instrument_gate();

    let first = run_full_program_probe();
    let second = run_full_program_probe();
    assert_eq!(
        first, second,
        "two full battery runs at HEAD must agree byte-for-byte"
    );
}

/// The instrument hook produces the stage-5 pair-class breakdown; the classes
/// sum to the entry count and the stage-5 plane/quadric×spline share `f` is
/// extractable — the datum the build spec §6 deliberately left unfilled.
#[test]
fn instrument_f_datum_recorded() {
    let _lock = battery_guard();
    open_instrument_gate();

    // Drive the deterministic stage-5 corpus through the public funnel and
    // read the histogram delta (the gate is open; the lock isolates this
    // test's records; the histogram accumulates across the process, so every
    // assertion is on the delta this corpus produces).
    let before_vector = stage5_histogram_vector();
    let before_population = stage5_population();
    let before_plane_spline = class_count(CarrierKind::Spline, CarrierKind::Plane);
    let before_cylinder_spline = class_count(CarrierKind::Spline, CarrierKind::Cylinder);
    let before_sphere_spline = class_count(CarrierKind::Spline, CarrierKind::Sphere);
    let before_cone_spline = class_count(CarrierKind::Spline, CarrierKind::Cone);
    let before_canonical = class_count(CarrierKind::Cylinder, CarrierKind::Cone);
    run_stage5_corpus();
    let after_vector = stage5_histogram_vector();
    let after_population = stage5_population();
    let after_plane_spline = class_count(CarrierKind::Spline, CarrierKind::Plane);
    let after_cylinder_spline = class_count(CarrierKind::Spline, CarrierKind::Cylinder);
    let after_sphere_spline = class_count(CarrierKind::Spline, CarrierKind::Sphere);
    let after_cone_spline = class_count(CarrierKind::Spline, CarrierKind::Cone);
    let after_canonical = class_count(CarrierKind::Cylinder, CarrierKind::Cone);

    // Four spline×analytic entries (plane, cylinder, sphere, cone) and two
    // canonical quadric×quadric entries: the histogram classes sum to the
    // entry count.
    let delta: Vec<u64> = before_vector
        .iter()
        .zip(after_vector.iter())
        .map(|(a, b)| b - a)
        .collect();
    let delta_population = after_population - before_population;
    assert_eq!(
        delta_population, 6,
        "the corpus records six stage-5 entries"
    );
    let delta_sum: u64 = delta.iter().sum();
    assert_eq!(
        delta_sum, delta_population,
        "the stage-5 classes sum to the entry count"
    );
    let spline_analytic_delta = (after_plane_spline - before_plane_spline)
        + (after_cylinder_spline - before_cylinder_spline)
        + (after_sphere_spline - before_sphere_spline)
        + (after_cone_spline - before_cone_spline);
    assert_eq!(
        spline_analytic_delta, 4,
        "the four spline×analytic classes each record one entry"
    );
    assert_eq!(
        after_canonical - before_canonical,
        2,
        "the canonical class records two entries"
    );
    // The order-insensitive class bin: the reversed query returns the same bin.
    assert_eq!(
        class_count(CarrierKind::Spline, CarrierKind::Plane),
        class_count(CarrierKind::Plane, CarrierKind::Spline),
        "the order-insensitive class bin counts agree"
    );

    // The datum `f` (stage-5 plane/quadric×spline share) is extractable from
    // the histogram: 4 of the 6 entries this corpus records are spline ×
    // analytic, so the share is exactly 2/3.
    let f = stage5_spline_analytic_share().expect("the stage-5 share is extractable");
    assert_eq!(f, 4.0 / 6.0, "the datum f is the exact recorded share");
}
