//! BG-KV2-403-S6 integration tests: the §11 exact-overlap sheet classifier
//! (`kernel/sheet.rs`) — the real `PsiMap`, the four §11 conditions over the
//! recognized carriers and the certified leaf-pair affine map, the
//! `NearOverlap` disproof of ExactSheet, and the no-tolerance / no-libm
//! discipline.

#![deny(clippy::unwrap_used)]

use std::fmt::Debug;

use truck_certified::kernel::certs::{PsiMapKind, SheetCert};
use truck_certified::kernel::evidence::{ClaimVerdict, Refusal, RefusalKind, VerdictClass};
use truck_certified::kernel::fixtures;
use truck_certified::kernel::leaf::{
    BezierLeaf, CarrierData, RationalCarrier, RationalCarrierKind,
};
use truck_certified::kernel::patch::{CertifiedPatch, IBox2};
use truck_certified::kernel::sheet::{exact_sheet, normal_dot_sign, PsiMap};
use truck_certified::kernel::SignCert;

/// The near-overlap radius offset between two coaxial cylinders: a radius
/// difference of `NEAR_RADIAL` admits no exact psi (no affine/bilinear/identity
/// map sends radius `r` onto radius `r + NEAR_RADIAL`).
const NEAR_RADIAL: f64 = 1e-3; // H-3: radial near-overlap offset between coaxial unit cylinders

/// Extract the `Ok` of a fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => panic!("a construction that must succeed was refused: {err:?}"),
    }
}

/// A 2-axis parameter box.
fn box2(lo: [f64; 2], hi: [f64; 2]) -> IBox2 {
    construct(IBox2::try_new(lo, hi))
}

/// The certified identity map.
fn identity_psi() -> PsiMap {
    construct(PsiMap::identity())
}

/// The sheet-domain box exercised in the fixtures: inside the coaxial
/// carriers' chart `[0, 2*PI] x [0, 1]` (stored as data by the fixture kit)
/// and inside the leaf unit square, with the cylinder's half-angle parameter
/// away from the chart degenerations so the oriented normals keep a certified
/// sign margin.
fn sheet_domain() -> IBox2 {
    box2([0.3, 0.2], [0.7, 0.6])
}

/// The unit plane graph leaf `(u, v) -> (u, v, 0)` over `[0,1]^2`.
fn plane_leaf() -> BezierLeaf {
    construct(BezierLeaf::try_new(
        1,
        1,
        vec![
            [0.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 1.0, 0.0, 1.0],
        ],
    ))
}

/// A plane carrier `(u, v) -> origin + u*u_dir + v*v_dir`.
fn plane_carrier(v_dir: [f64; 3]) -> RationalCarrier {
    construct(RationalCarrier::try_new(
        RationalCarrierKind::Plane,
        CarrierData::Plane {
            origin: [0.0, 0.0, 0.0],
            u_dir: [1.0, 0.0, 0.0],
            v_dir,
        },
        box2([-1.0, -1.0], [1.0, 1.0]),
    ))
}

/// The coaxial-adjacent-but-offset twin of the first fixture cylinder: the
/// same z-axis chart, radius offset by [`NEAR_RADIAL`]. No exact psi maps the
/// unit cylinder onto it.
fn offset_cylinder_twin(first: &RationalCarrier) -> RationalCarrier {
    construct(RationalCarrier::try_new(
        RationalCarrierKind::Cylinder,
        CarrierData::Cylinder {
            origin: [0.0, 0.0, 0.0],
            axis: [0.0, 0.0, 1.0],
            radius: 1.0 + NEAR_RADIAL,
            height: (0.0, 1.0),
        },
        first.domain,
    ))
}

/// Condition (2)+(3)+(4) certify on the same recognized rational carrier under
/// the identity closed-form psi (the shim kit's coaxial fixture): `SheetCert`
/// for real.
#[test]
fn same_carrier_identity_psi_certifies() {
    let fx = construct(fixtures::coaxial_cylinders());
    let d = sheet_domain();
    match exact_sheet(&fx.first, &fx.second, d, identity_psi()) {
        ClaimVerdict::Proven(cert) => {
            assert_eq!(cert.domain, d);
            assert_eq!(cert.psi_kind, PsiMapKind::Identity);
            assert_eq!(cert.det_dpsi.value(), 1.0);
        }
        other => panic!("the coaxial identity sheet must certify: {other:?}"),
    }
}

/// Two identical unit plane leaves under the certified affine map certify the
/// leaf-pair sheet; the certified affine transport (Rule B outward rounding)
/// encloses the exact image of a genuinely nontrivial affine map.
#[test]
fn leaf_pair_affine_psi_certifies() {
    let leaf = plane_leaf();
    let d = sheet_domain();
    let psi = construct(PsiMap::try_new(
        PsiMapKind::Affine,
        [[1.0, 0.0], [0.0, 1.0]],
        [0.0, 0.0],
    ));
    match exact_sheet(&leaf, &leaf, d, psi) {
        ClaimVerdict::Proven(cert) => {
            assert_eq!(cert.psi_kind, PsiMapKind::Affine);
            assert_eq!(cert.domain, d);
            assert_eq!(cert.det_dpsi.value(), 1.0);
        }
        other => panic!("the identical-leaf affine sheet must certify: {other:?}"),
    }
    // The certified transport of a nontrivial affine map encloses the exact
    // image: (s,t) = (0.25 + 0.5u, 0.25 + 0.5v) sends
    // [0.5, 0.75] x [0.25, 0.5] onto [0.5, 0.625] x [0.375, 0.5].
    let map = construct(PsiMap::try_new(
        PsiMapKind::Affine,
        [[0.5, 0.0], [0.0, 0.5]],
        [0.25, 0.25],
    ));
    assert_eq!(map.det_value(), 0.25);
    let image = map.image_box(box2([0.5, 0.25], [0.75, 0.5]));
    assert!(
        image.lo[0] <= 0.5 && 0.625 <= image.hi[0],
        "certified image must enclose the exact x range [0.5, 0.625]"
    );
    assert!(
        image.lo[1] <= 0.375 && 0.5 <= image.hi[1],
        "certified image must enclose the exact y range [0.375, 0.5]"
    );
}

/// Condition (3): `n1 · (n2 o psi)` certified of constant sign over the box —
/// positive for the co-oriented coaxial pair, negative for the mirrored plane.
#[test]
fn constant_sign_normal_dot_certified() {
    let fx = construct(fixtures::coaxial_cylinders());
    let d = sheet_domain();
    match normal_dot_sign(&fx.first, &fx.second, d, identity_psi()) {
        ClaimVerdict::Proven(sign) => assert_eq!(sign, SignCert::Positive),
        other => panic!("the coaxial normals must certify positive: {other:?}"),
    }
    // The mirrored plane `(u, v, 0)` vs `(u, -v, 0)`: oriented normals oppose.
    let up = plane_carrier([0.0, 1.0, 0.0]);
    let down = plane_carrier([0.0, -1.0, 0.0]);
    match normal_dot_sign(&up, &down, d, identity_psi()) {
        ClaimVerdict::Proven(sign) => assert_eq!(sign, SignCert::Negative),
        other => panic!("the mirrored-plane normals must certify negative: {other:?}"),
    }
}

/// Condition (4): `det Dpsi` certified nonzero. A degenerate (det-zero) affine
/// map refuses at construction, and the shim's `SheetCert` constructor refuses
/// a zero determinant.
#[test]
fn det_dpsi_nonzero_certified() {
    let psi = construct(PsiMap::try_new(
        PsiMapKind::Affine,
        [[2.0, 0.0], [0.0, 0.5]],
        [0.0, 0.0],
    ));
    assert_eq!(psi.det_value(), 1.0);
    match PsiMap::try_new(PsiMapKind::Affine, [[1.0, 1.0], [1.0, 1.0]], [0.0, 0.0]) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::WeightDegenerate);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        Ok(_) => panic!("a det-zero affine PsiMap must refuse"),
    }
    let domain = box2([0.0, 0.0], [1.0, 1.0]);
    assert!(
        SheetCert::try_new(domain, PsiMapKind::Affine, 0.0).is_err(),
        "SheetCert::try_new must refuse a degenerate (0) map determinant"
    );
}

/// A coaxial-adjacent-but-offset twin (radius offset, no exact psi) is
/// `Refuse(NearOverlap)`, backed `Disproven` of ExactSheet — never admitted as
/// a sheet, never left undecided, and never relaxed to a tolerance.
#[test]
fn near_overlap_refuses_disproven_of_exact_sheet() {
    let fx = construct(fixtures::coaxial_cylinders());
    let offset = offset_cylinder_twin(&fx.first);
    let d = sheet_domain();
    match exact_sheet(&fx.first, &offset, d, identity_psi()) {
        ClaimVerdict::Disproven(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::NearOverlap);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        other => panic!("an offset coaxial twin must refuse the exact sheet: {other:?}"),
    }
}

/// The exact-sheet entry points carry no tolerance and no tolerance-tagged
/// sheet constructor exists anywhere in the module (§21's deferred table,
/// enforced): the module source contains no `tolerance`/`TOL_` token outside
/// comments, and `exact_sheet` is pinned to its four-argument §11 signature.
#[test]
fn tolerance_sheet_is_not_admitted() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/kernel/sheet.rs");
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => panic!("sheet.rs must be readable: {err}"),
    };
    let code: Vec<&str> = source
        .lines()
        .map(|line| match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        })
        .collect();
    for line in &code {
        assert!(
            !line.contains("tolerance") && !line.contains("TOL_"),
            "no sheet code may carry a tolerance tag or compare at a tolerance: {line:?}"
        );
    }
    let _: fn(
        &dyn CertifiedPatch,
        &dyn CertifiedPatch,
        IBox2,
        PsiMap,
    ) -> ClaimVerdict<SheetCert, Refusal, &'static str> = exact_sheet;
}

/// N4: no transcendental call may appear in `sheet.rs` (bit reproducibility is
/// a hard consequence of the exact-sheet discipline).
#[test]
fn no_transcendental_call_in_sheet_module() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/kernel/sheet.rs");
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => panic!("sheet.rs must be readable: {err}"),
    };
    let code: Vec<&str> = source
        .lines()
        .map(|line| match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        })
        .collect();
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
            .iter()
            .any(|line| contains_word(line, needle) || line.contains("std::f64::consts"));
        assert!(
            !present,
            "no transcendental call may appear outside comments in sheet.rs (found {needle})"
        );
    }
    let sqrt_present = code.iter().any(|line| line.contains("sqrt"));
    assert!(
        !sqrt_present,
        "no sqrt call may appear outside comments in sheet.rs"
    );
}
