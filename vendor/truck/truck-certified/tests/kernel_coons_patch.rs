//! The C5 `CertifiedPatch` implementation tests (BG-KV2-205-C5PATCH): the
//! six required machine-checked ground truths for the bilinear `CoonsSurface`
//! implementor.
//!
//! Every fixture is a `CoonsSurface` whose four boundary curves are straight
//! `Line<Point3>` segments between the cached corners, so the landed surface
//! is exactly the bilinear corner interpolation the certified patch encloses.
//! Ground truths are the corner points, the corner average at `u = v = 0.5`,
//! and the landed `jacobian()` cross product — never a solver. The source-scan
//! test reads `coons_patch.rs` itself to pin the N4 guarantee.

#![deny(clippy::unwrap_used)]

use truck_certified::kernel::config;
use truck_certified::kernel::coons_patch;
use truck_certified::kernel::evidence::ClaimVerdict;
use truck_certified::kernel::patch::{CertifiedPatch, IBox2, IBox3};
use truck_geometry::prelude::*;

/// The certified-enclosure containment slack (H-3): certified enclosures are
/// outward-rounded, so a sampled point never lands more than this far outside.
const ENCLOSURE_SLACK: f64 = 1e-9; // H-3: certified-enclosure containment slack
/// The certified `EG - F^2` containment slack (H-3): the certified enclosure
/// is outward-rounded and the landed `jacobian()` is a rounded `f64`, so the
/// squared magnitude never misses by more than this.
const EGF2_SLACK: f64 = 1e-9; // H-3: certified EG-F^2 containment slack

/// A `Result` from a corner-consistent fixture constructor: the fixture data
/// is valid by construction, so the refusal arm is a test-bug panic.
fn construct<T>(result: std::result::Result<T, truck_geometry::constructive::ConstructError>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("a construction that must succeed was refused: {error:?}"),
    }
}

fn box2(lo: [f64; 2], hi: [f64; 2]) -> IBox2 {
    match IBox2::try_new(lo, hi) {
        Ok(box_) => box_,
        Err(refusal) => panic!("a well-formed test box was refused: {refusal:?}"),
    }
}

/// Build the straight-segment `CoonsSurface` over the four corners.
fn coons(p00: Point3, p10: Point3, p01: Point3, p11: Point3) -> CoonsSurface<Line<Point3>> {
    construct(CoonsSurface::try_new(
        Line(p00, p10),
        Line(p01, p11),
        Line(p00, p01),
        Line(p10, p11),
    ))
}

/// The warped bilinear quad: the unit square mapped to a non-planar bilinear
/// patch with corners `(0,0,0)`, `(1,0,0)`, `(0,1,1)`, `(1,1,1)`. The center
/// `u = v = 0.5` is the average of the four corners.
fn warped() -> CoonsSurface<Line<Point3>> {
    coons(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 1.0),
        Point3::new(1.0, 1.0, 1.0),
    )
}

/// A regular (non-degenerate) bilinear patch: the twisted quad with corners
/// `(0,0,0)`, `(1,0,0)`, `(0,1,0)`, `(1,1,1)`. Its `EG - F^2 = 1 + u^2 + v^2`
/// is bounded below by 1 on the unit square.
fn twisted() -> CoonsSurface<Line<Point3>> {
    coons(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
        Point3::new(1.0, 1.0, 1.0),
    )
}

/// A folded bilinear patch: the planar bow-tie quad with one corner pulled
/// across the diagonal, corners `(0,0,0)`, `(1,1,0)`, `(1,0,0)`, `(0,1,0)`.
/// It is construction-valid (the corner equalities hold) but
/// geometry-invalid: the `v`-derivative collapses on the fold line
/// `u = 0.5`, where the exposed Jacobian `S_u x S_v` vanishes.
fn folded() -> CoonsSurface<Line<Point3>> {
    coons(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    )
}

/// Whether every coordinate of `p` lies in the certified enclosure `box_`, to
/// the outward-rounding containment slack.
fn enclosed(box_: IBox3, p: [f64; 3]) -> bool {
    for k in 0..3 {
        if p[k] < box_.lo[k] - ENCLOSURE_SLACK || p[k] > box_.hi[k] + ENCLOSURE_SLACK {
            return false;
        }
    }
    true
}

#[test]
fn coons_enclose_contains_sampled_points() {
    let patch = warped();
    let p00 = Point3::new(0.0, 0.0, 0.0);
    let p10 = Point3::new(1.0, 0.0, 0.0);
    let p01 = Point3::new(0.0, 1.0, 1.0);
    let p11 = Point3::new(1.0, 1.0, 1.0);
    // Ground truth: the surface passes exactly through the four corners, and
    // the center `u = v = 0.5` is exactly the average of the corners (a
    // bilinear patch property, asserted exactly).
    assert_eq!(patch.subs(0.0, 0.0), p00);
    assert_eq!(patch.subs(1.0, 0.0), p10);
    assert_eq!(patch.subs(0.0, 1.0), p01);
    assert_eq!(patch.subs(1.0, 1.0), p11);
    let average = Point3::new(
        (p00.x + p10.x + p01.x + p11.x) * 0.25,
        (p00.y + p10.y + p01.y + p11.y) * 0.25,
        (p00.z + p10.z + p01.z + p11.z) * 0.25,
    );
    assert_eq!(patch.subs(0.5, 0.5), average);

    // The sampled `(u, v)` grid points all lie in the certified enclosure of
    // the unit box.
    let unit = box2([0.0, 0.0], [1.0, 1.0]);
    let enclosure = CertifiedPatch::enclose(&patch, unit);
    for i in 0..=8u32 {
        for j in 0..=8u32 {
            let u = i as f64 / 8.0;
            let v = j as f64 / 8.0;
            let p = patch.subs(u, v);
            let (px, py, pz) = (p.x, p.y, p.z);
            assert!(
                enclosed(enclosure, [px, py, pz]),
                "enclose over the unit box misses ({u}, {v}) -> ({px}, {py}, {pz})"
            );
        }
    }
}

#[test]
fn coons_regularity_proven_on_a_regular_patch() {
    let patch = twisted();
    // A regular box around the patch center: the certified `EG - F^2` lower
    // bound is ~0.51 there, far above the 1e-12 singular-map floor (interval
    // evaluation on the full unit box is dependency-widened, so the certified
    // callers subdivide until the arm holds). // H-3: regularity-floor prose comparison
    let d = box2([0.4, 0.4], [0.6, 0.6]);
    match CertifiedPatch::regularity(&patch, d) {
        ClaimVerdict::Proven(bound) => {
            assert!(
                bound.value() > 0.1,
                "the certified lower bound {} must clear TOL_JACOBIAN by a wide margin",
                bound.value()
            );
        }
        other => panic!("the twisted bilinear patch must certify Proven on {d:?}: {other:?}"),
    }
}

#[test]
fn coons_folded_patch_is_inconclusive_not_proven() {
    let patch = folded();
    // The folded patch is construction-valid (try_new accepts the corner
    // equalities) and geometry-invalid: on the fold line the exposed Jacobian
    // vanishes, so regularity is never Proven.
    let cases = [
        box2([0.5, 0.5], [0.5, 0.5]),
        box2([0.49, 0.49], [0.51, 0.51]),
        box2([0.25, 0.25], [0.75, 0.75]),
    ];
    let mut saw_disproven = false;
    for d in cases {
        match CertifiedPatch::regularity(&patch, d) {
            ClaimVerdict::Proven(_) => {
                panic!("a folded patch must never certify Proven over {d:?}")
            }
            ClaimVerdict::Disproven(witness) => {
                saw_disproven = true;
                assert_eq!(
                    witness.box_, d,
                    "the degeneracy witness carries the queried box"
                );
                assert!(
                    witness.egf2.0 <= witness.egf2.1,
                    "the degeneracy enclosure must be ordered (lo <= hi)"
                );
                assert!(
                    witness.egf2.1 < config::TOL_JACOBIAN,
                    "the degeneracy enclosure {} must sit below the singular-map floor",
                    witness.egf2.1
                );
            }
            ClaimVerdict::Inconclusive(_) => {}
        }
    }
    assert!(
        saw_disproven,
        "at least one folded box must Disprove, carrying the Degeneracy witness"
    );
}

#[test]
fn coons_weight_bound_is_the_constant_one_plumbing() {
    let patch = warped();
    let d = box2([0.0, 0.0], [1.0, 1.0]);
    match CertifiedPatch::weight_bound(&patch, d) {
        Some(ClaimVerdict::Proven(bound)) => {
            assert_eq!(
                bound.value(),
                1.0,
                "the bilinear patch's weight is exactly 1"
            );
        }
        other => panic!("the polynomial patch must report the constant-1 bound: {other:?}"),
    }
}

#[test]
fn coons_jacobian_and_certifiedpatch_regularity_agree() {
    // Spec §5.9's one-call rule: the landed `jacobian()` cross product and
    // `CertifiedPatch::regularity`'s `EG - F^2` describe the same surface.
    // At every sampled `(u, v)`, the certified `EG - F^2` enclosure over a
    // small surrounding box contains the squared magnitude of the landed
    // `f64` Jacobian, and regularity is Proven there (the patch is regular).
    let patch = twisted();
    let half_width = 0.001;
    for i in 0..=7u32 {
        for j in 0..=7u32 {
            let u = 0.0625 + i as f64 * 0.125;
            let v = 0.0625 + j as f64 * 0.125;
            let d = box2(
                [u - half_width, v - half_width],
                [u + half_width, v + half_width],
            );
            let jacobian = patch.jacobian(u, v);
            let jacobian2 =
                jacobian.x * jacobian.x + jacobian.y * jacobian.y + jacobian.z * jacobian.z;
            let enclosure = coons_patch::egf2(&patch, d);
            let slack = EGF2_SLACK * (1.0 + jacobian2);
            assert!(
                enclosure.lo <= jacobian2 + slack && jacobian2 - slack <= enclosure.hi,
                "landed |J|^2 = {jacobian2} at ({u}, {v}) outside the certified \
                 EG-F^2 enclosure [{}, {}]",
                enclosure.lo,
                enclosure.hi
            );
            match CertifiedPatch::regularity(&patch, d) {
                ClaimVerdict::Proven(bound) => {
                    assert!(
                        bound.value() > 0.0,
                        "the certified regularity lower bound must be positive"
                    );
                }
                other => panic!("a regular sample box must certify Proven: {other:?}"),
            }
        }
    }
}

#[test]
fn no_transcendental_call_in_coons_patch_module() {
    let source = include_str!("../src/kernel/coons_patch.rs");
    let banned = ["sin", "cos", "atan2", "exp", "ln", "log", "powf", "sqrt"];
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    for (line_no, raw) in source.lines().enumerate() {
        let code = match raw.find("//") {
            Some(index) => &raw[..index],
            None => raw,
        };
        for token in banned {
            for (at, _) in code.match_indices(token) {
                let after = at + token.len();
                let left_clear = code[..at].chars().next_back().is_none_or(|c| !is_word(c));
                let right_clear = code[after..].chars().next().is_none_or(|c| !is_word(c));
                assert!(
                    !(left_clear && right_clear),
                    "line {} carries the transcendental call token {token}: {code}",
                    line_no + 1
                );
            }
        }
    }
}
