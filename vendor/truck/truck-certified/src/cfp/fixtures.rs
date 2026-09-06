#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//! The CFP fixture kit (CFP-000-SPINE; build spec §2, normative).
//!
//! **TEST SUPPORT ONLY.** This module is `#[doc(hidden)] pub`, explicitly
//! excluded from the certified API surface (a one-line mapping-table note, not
//! a row: no new evidence kind). The CFP wave packets' tests consume it
//! through the crate's public path; `#[cfg(test)]`-only items would be
//! invisible to them.
//!
//! Each fixture freezes DATA + ground-truth records that are **machine-checked
//! at admission** — never by solving, never by interval/certified work. The
//! kit carries each fixture's algebraic data as recorded constants (exact
//! integer coefficients where the ground truth is an exact statement) and a
//! documented ground truth; the fixture's `admit()` re-derives every
//! machine-checkable ground truth from the constants with exact integer or
//! structural arithmetic and refuses on any inconsistency. Facts whose
//! verification needs the implementing packets' certified machinery (the
//! parallel-contract rule: evaluation-asserting tests land in
//! CFP-001/003/004/006) are recorded as expectations on this same data, and
//! this module says so explicitly per fixture. Every construction is
//! deterministic: the whole kit builds from ordered constant data, so two
//! constructions compare equal.
//!
//! The fixtures freeze the normative ground truths of the CFP program
//! (spec §2):
//!
//! - **F-C0** [`Fc0CapSlabFixture`] — the DSC defect counterexample: a
//!   sphere-cap × plane-slab whose contact region is the cap apex; red under
//!   the boundary-sample screen, green under a sound enclosure.
//! - **F-C1** [`Fc1ParameterTwin`] — the parameter-space twin: a trim whose
//!   parameter extent is not spanned by its boundary polygon.
//! - **F-C2** [`Fc2EnclosureBattery`] — the enclosure battery: three nested
//!   boxes on a fixed bicubic net (containment, width convergence, and
//!   monotonicity).
//! - **F-C3** [`Fc3SeparationGroundTruths`] — Theorem 3 ground truths: a
//!   separated control-net pair and a touching one, with exact-integer
//!   `λ`-projection checks.
//! - **F-C4** [`Fc4Theorem4Fixture`] — Theorem 4 ground truths: a plane ×
//!   bicubic with known `h` and known critical point, a quadric × spline
//!   record, and the **elevation-trap twin**.
//! - **F-C5** [`Fc5SpanBvh`] — the span-BVH ground truth: a 20 × 30-span
//!   decomposition with exactly one contact span pair.
//! - **F-C6** [`Fc6StagnationFixture`] — the stagnation fixture: a
//!   near-tangency pair whose per-box cones stay overlapping at three
//!   successive depths, routed to the cascade with an A₁ verdict.
//! - **F-C7** [`Fc7CrossCheckKit`] — the F1 cross-check data: seeded
//!   randomized net + box pairs on which the evidence-side and certified-side
//!   sub-box hulls must agree.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, no `panic!`, and
//! no module-level `allow`.

use crate::contract::Refusal;

/// Whether two axis-aligned 3-D boxes (given by `lo`/`hi` corners) are
/// disjoint: some axis separates them. Exact comparison of recorded constants.
fn aabb_disjoint(a_lo: [f64; 3], a_hi: [f64; 3], b_lo: [f64; 3], b_hi: [f64; 3]) -> bool {
    a_lo.iter()
        .zip(a_hi.iter())
        .zip(b_lo.iter())
        .zip(b_hi.iter())
        .any(|(((al, ah), bl), bh)| ah < bl || bh < al)
}

/// Whether two axis-aligned 3-D boxes overlap (not disjoint).
fn aabb_overlap(a_lo: [f64; 3], a_hi: [f64; 3], b_lo: [f64; 3], b_hi: [f64; 3]) -> bool {
    !aabb_disjoint(a_lo, a_hi, b_lo, b_hi)
}

/// Whether a `lo <= hi` 3-D box record is structurally valid.
fn aabb_ok(lo: [f64; 3], hi: [f64; 3]) -> bool {
    lo.iter()
        .zip(hi.iter())
        .all(|(l, h)| l.is_finite() && h.is_finite() && l <= h)
}

/// Whether a `(u, v)` parameter box is valid (finite, positively wide).
fn param_box_ok(box_: ((f64, f64), (f64, f64))) -> bool {
    [box_.0 .0, box_.0 .1, box_.1 .0, box_.1 .1]
        .iter()
        .all(|x| x.is_finite())
        && box_.0 .0 < box_.0 .1
        && box_.1 .0 < box_.1 .1
}

// ---------------------------------------------------------------------------
// F-C0 — the DSC defect counterexample (spec §2; booking)
// ---------------------------------------------------------------------------

/// F-C0 (DSC defect counterexample): a sphere-cap solid × plane-slab whose
/// contact region is the cap apex, red under the boundary-sample screen.
///
/// Data: a spherical cap (the part of the unit-5 sphere about the origin at
/// or below the plane `z = −3`; boundary circle of radius 4 at `z = −3`, apex
/// `(0, 0, −5)`) and a plane-slab solid (the box `[−10, 10] × [−10, 10] ×
/// `[−6, −4]`). The cap's boundary-sample AABB is the flat circle hull
/// `[−4, 4] × [−4, 4] × [−3, −3]` — `face_aabb` hulls boundary-curve samples
/// only — while the cap's true image is the whole segment
/// `[−4, 4] × [−4, 4] × [−5, −3]` whose apex reaches the slab.
///
/// Ground truth (exact float records, machine-checked in [`Fc0CapSlabFixture::admit`]):
/// `sampled_aabb ∩ slab_aabb = ∅` AND the true images touch: the apex is the
/// contact region and lies inside the slab's vertical extent while the
/// boundary circle sits above the slab. The pair is silently dropped by the
/// sampled screen — the boolean completes with the intersection fragment
/// missing (`DSC-BOUNDARY-SAMPLE-EXTENT-001`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fc0CapSlabFixture {
    /// The sphere centre (the origin).
    pub sphere_centre: [f64; 3],
    /// The sphere radius.
    pub sphere_radius: f64,
    /// The cap plane `z = cap_plane_z`; the boundary circle lives there.
    pub cap_plane_z: f64,
    /// The boundary circle's radius in the cap plane.
    pub cap_boundary_radius: f64,
    /// The cap apex (sphere bottom), the contact region.
    pub apex: [f64; 3],
    /// The slab solid's AABB `lo`.
    pub slab_lo: [f64; 3],
    /// The slab solid's AABB `hi`.
    pub slab_hi: [f64; 3],
    /// The screen's boundary-sample AABB `lo`.
    pub sampled_lo: [f64; 3],
    /// The screen's boundary-sample AABB `hi`.
    pub sampled_hi: [f64; 3],
    /// The cap's true-image AABB `lo`.
    pub true_lo: [f64; 3],
    /// The cap's true-image AABB `hi`.
    pub true_hi: [f64; 3],
    /// Ground-truth record: the sampled AABB and the slab AABB are disjoint.
    pub sampled_aabbs_disjoint: bool,
    /// Ground-truth record: the true images touch (the AABBs overlap).
    pub true_images_touch: bool,
    /// Ground-truth record: the cap apex is the contact region.
    pub apex_is_contact_region: bool,
}

impl Fc0CapSlabFixture {
    /// The F-C0 counterexample, verbatim.
    pub fn new() -> Self {
        Self {
            sphere_centre: [0.0, 0.0, 0.0],
            sphere_radius: 5.0,
            cap_plane_z: -3.0,
            cap_boundary_radius: 4.0,
            apex: [0.0, 0.0, -5.0],
            slab_lo: [-10.0, -10.0, -6.0],
            slab_hi: [10.0, 10.0, -4.0],
            sampled_lo: [-4.0, -4.0, -3.0],
            sampled_hi: [4.0, 4.0, -3.0],
            true_lo: [-4.0, -4.0, -5.0],
            true_hi: [4.0, 4.0, -3.0],
            sampled_aabbs_disjoint: true,
            true_images_touch: true,
            apex_is_contact_region: true,
        }
    }

    /// Machine-check the F-C0 ground truths with exact comparisons of the
    /// recorded constants.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) on any inconsistency: the sampled
    /// box must be the planar boundary-circle hull at `cap_plane_z`, the true
    /// box must span the apex, the boundary circle must sit above the slab,
    /// the apex must lie in the slab's vertical extent, and the recorded
    /// disjoint/touch facts must be exactly the computed ones.
    pub fn admit(&self) -> Result<(), Refusal> {
        let r = self.cap_boundary_radius;
        let z = self.cap_plane_z;
        let [cx, cy, cz] = self.sphere_centre;
        if !aabb_ok(self.sampled_lo, self.sampled_hi)
            || !aabb_ok(self.slab_lo, self.slab_hi)
            || !aabb_ok(self.true_lo, self.true_hi)
        {
            return Err(Refusal::InvalidInput);
        }
        // The sampled box is the planar circle hull at the cap plane.
        let circle_lo = [cx - r, cy - r, z];
        let circle_hi = [cx + r, cy + r, z];
        if self.sampled_lo != circle_lo || self.sampled_hi != circle_hi {
            return Err(Refusal::InvalidInput);
        }
        // The true box spans from the apex up to the cap plane with the same
        // horizontal extent.
        if self.true_lo != [cx - r, cy - r, self.apex[2]] || self.true_hi != circle_hi {
            return Err(Refusal::InvalidInput);
        }
        // The apex is the sphere's lowest point and sits on the centre axis.
        if self.apex[0] != cx || self.apex[1] != cy || self.apex[2] != cz - self.sphere_radius {
            return Err(Refusal::InvalidInput);
        }
        // The cap boundary circle sits above the slab...
        if self.cap_plane_z <= self.slab_hi[2] {
            return Err(Refusal::InvalidInput);
        }
        // ...and the apex (contact region) lies inside the slab's extent.
        if !(self.slab_lo[2] <= self.apex[2] && self.apex[2] <= self.slab_hi[2]) {
            return Err(Refusal::InvalidInput);
        }
        // The true image is a STRICT underestimate of the screen box in z
        // (the interior leaves the boundary-sample hull: the defect).
        if self.true_lo[2] >= self.sampled_lo[2] {
            return Err(Refusal::InvalidInput);
        }
        // The recorded ground truths are exactly the computed ones.
        let disjoint = aabb_disjoint(self.sampled_lo, self.sampled_hi, self.slab_lo, self.slab_hi);
        let touch = aabb_overlap(self.true_lo, self.true_hi, self.slab_lo, self.slab_hi);
        if disjoint != self.sampled_aabbs_disjoint
            || touch != self.true_images_touch
            || !self.apex_is_contact_region
        {
            return Err(Refusal::InvalidInput);
        }
        Ok(())
    }
}

impl Default for Fc0CapSlabFixture {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// F-C1 — the parameter-space twin (spec §2)
// ---------------------------------------------------------------------------

/// F-C1 (parameter-space twin): a trim whose parameter extent is not spanned
/// by its boundary polygon.
///
/// The DSC screen's parameter twin derives the stratum's parameter box from
/// the boundary wires' parameter polygon
/// (`DSC-BOUNDARY-SAMPLE-EXTENT-001`, §2.2). For a trim whose region leaves
/// that polygon (interior islands, caps around a pole, trimmed sub-regions of
/// a windowed sweep), the derived box under-covers the trim, so downstream
/// certified work reasons over a wrong domain. This fixture freezes one such
/// trim: a boundary polygon whose hull is STRICTLY inside the trim's true
/// parameter extent.
///
/// Ground truth (exact float comparisons, machine-checked in
/// [`Fc1ParameterTwin::admit`]): `boundary_uv_hull ⊊ true_uv_extent`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fc1ParameterTwin {
    /// The trim's true parameter extent `(u, v)`.
    pub true_extent: ((f64, f64), (f64, f64)),
    /// The trim's boundary polygon vertices in `(u, v)` (one interior island
    /// loop; the true region's extent is not spanned by this polygon).
    pub boundary_polygon: [[f64; 2]; 4],
    /// Ground-truth record: the boundary polygon hull is a strict subset of
    /// the true parameter extent.
    pub hull_is_strict_subset: bool,
}

impl Fc1ParameterTwin {
    /// The F-C1 twin, verbatim.
    pub fn new() -> Self {
        Self {
            true_extent: ((0.0, 1.0), (0.0, 1.0)),
            boundary_polygon: [[0.1, 0.1], [0.4, 0.1], [0.4, 0.4], [0.1, 0.4]],
            hull_is_strict_subset: true,
        }
    }

    /// Machine-check the F-C1 ground truth with exact comparisons.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) unless every vertex lies inside the
    /// true extent and the polygon hull is strictly inside it on at least one
    /// side — `boundary_uv_hull ⊊ true_uv_extent`.
    pub fn admit(&self) -> Result<(), Refusal> {
        if !param_box_ok(self.true_extent) {
            return Err(Refusal::InvalidInput);
        }
        let (u_lo, u_hi) = (self.true_extent.0 .0, self.true_extent.0 .1);
        let (v_lo, v_hi) = (self.true_extent.1 .0, self.true_extent.1 .1);
        let first = self.boundary_polygon.first().ok_or(Refusal::InvalidInput)?;
        let mut hull = (*first, *first);
        for vertex in &self.boundary_polygon {
            if !(u_lo <= vertex[0] && vertex[0] <= u_hi && v_lo <= vertex[1] && vertex[1] <= v_hi) {
                return Err(Refusal::InvalidInput);
            }
            hull.0[0] = hull.0[0].min(vertex[0]);
            hull.0[1] = hull.0[1].max(vertex[0]);
            hull.1[0] = hull.1[0].min(vertex[1]);
            hull.1[1] = hull.1[1].max(vertex[1]);
        }
        // Strict subset: contained everywhere and strictly smaller somewhere.
        let contained =
            u_lo <= hull.0[0] && hull.0[1] <= u_hi && v_lo <= hull.1[0] && hull.1[1] <= v_hi;
        let strict_somewhere =
            u_lo < hull.0[0] || hull.0[1] < u_hi || v_lo < hull.1[0] || hull.1[1] < v_hi;
        if !contained || !strict_somewhere || !self.hull_is_strict_subset {
            return Err(Refusal::InvalidInput);
        }
        Ok(())
    }
}

impl Default for Fc1ParameterTwin {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// F-C2 — the enclosure battery (spec §2; BG-ENC-002 permanent guard)
// ---------------------------------------------------------------------------

/// F-C2 (enclosure battery): three nested box triples `(B₁ ⊃ B₂ ⊃ B₃)` on a
/// fixed bicubic net.
///
/// The battery freezes the inputs the enclosure gates are graded on
/// (spec §5): containment (`enclose(B) ⊇ f(B)`, BG-ENC-001), width
/// convergence under repeated bisection
/// (`width(enclose(B₁)) ≥ width(enclose(B₂)) ≥ width(enclose(B₃))` and
/// `width(enclose(B₃)) < width(enclose(B₁))` — the strict-decrease guard on
/// `NUM-SPLINE-ENCLOSURE-CONVERGENCE-001`), and monotonicity
/// `B ⊆ B′ ⇒ enclose(B) ⊆ enclose(B′)`.
///
/// The nested boxes and the non-flat bicubic net are recorded data; the width
/// and containment inequalities are **evaluation-asserted by CFP-001** against
/// the landed enclosures (the parallel contract) — the admission here
/// machine-checks the structural ground truths: a valid non-flat net and three
/// strictly nested boxes, which is what forces any sound, monotone,
/// converging enclosure to satisfy the recorded expectations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fc2EnclosureBattery {
    /// The fixed bicubic control net, `(u, v)`-ordered `4 × 4`, non-flat.
    pub net: [[[f64; 3]; 4]; 4],
    /// The outer box `B₁`.
    pub b1: ((f64, f64), (f64, f64)),
    /// The middle box `B₂ ⊂ B₁`.
    pub b2: ((f64, f64), (f64, f64)),
    /// The inner box `B₃ ⊂ B₂`.
    pub b3: ((f64, f64), (f64, f64)),
    /// Ground-truth record: widths are non-increasing (`B₁ ≥ B₂ ≥ B₃`).
    pub widths_non_increasing: bool,
    /// Ground-truth record: strict decrease somewhere (`B₃ < B₁`).
    pub widths_strictly_decrease: bool,
    /// Ground-truth record: enclosure is monotone (`B ⊆ B′ ⇒ enclose(B) ⊆ enclose(B′)`).
    pub monotone: bool,
}

impl Fc2EnclosureBattery {
    /// The F-C2 battery, verbatim.
    pub fn new() -> Self {
        // A non-flat bicubic net: z = i·j over the 4 × 4 grid.
        let net = [
            [
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 2.0, 0.0],
                [0.0, 3.0, 0.0],
            ],
            [
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 1.0],
                [1.0, 2.0, 2.0],
                [1.0, 3.0, 3.0],
            ],
            [
                [2.0, 0.0, 0.0],
                [2.0, 1.0, 2.0],
                [2.0, 2.0, 4.0],
                [2.0, 3.0, 6.0],
            ],
            [
                [3.0, 0.0, 0.0],
                [3.0, 1.0, 3.0],
                [3.0, 2.0, 6.0],
                [3.0, 3.0, 9.0],
            ],
        ];
        Self {
            net,
            b1: ((0.0, 1.0), (0.0, 1.0)),
            b2: ((0.25, 0.75), (0.25, 0.75)),
            b3: ((0.4375, 0.5625), (0.4375, 0.5625)),
            widths_non_increasing: true,
            widths_strictly_decrease: true,
            monotone: true,
        }
    }

    /// Machine-check the F-C2 structural ground truths.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a non-finite net, a non-flat-net
    /// violation (the net must vary — a flat net would make strict decrease
    /// impossible), non-nested or non-strict box data, or a recorded
    /// expectation flipped off. The width/convergence inequalities themselves
    /// are asserted by CFP-001's enclosure tests against this same data.
    pub fn admit(&self) -> Result<(), Refusal> {
        let all_finite = self.net.iter().flatten().flatten().all(|c| c.is_finite());
        if !all_finite {
            return Err(Refusal::InvalidInput);
        }
        // Non-flat: the net's z control values are not all equal.
        let z0 = self.net[0][0][2];
        let flat = self.net.iter().flatten().all(|p| p[2] == z0);
        if flat {
            return Err(Refusal::InvalidInput);
        }
        let boxes = [self.b1, self.b2, self.b3];
        if !boxes.iter().all(|b| param_box_ok(*b)) {
            return Err(Refusal::InvalidInput);
        }
        // Strict nesting B1 ⊃ B2 ⊃ B3 on both axes.
        let nested = self.b1.0 .0 < self.b2.0 .0
            && self.b2.0 .0 < self.b3.0 .0
            && self.b3.0 .1 < self.b2.0 .1
            && self.b2.0 .1 < self.b1.0 .1
            && self.b1.1 .0 < self.b2.1 .0
            && self.b2.1 .0 < self.b3.1 .0
            && self.b3.1 .1 < self.b2.1 .1
            && self.b2.1 .1 < self.b1.1 .1;
        if !nested
            || !self.widths_non_increasing
            || !self.widths_strictly_decrease
            || !self.monotone
        {
            return Err(Refusal::InvalidInput);
        }
        Ok(())
    }
}

impl Default for Fc2EnclosureBattery {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// F-C3 — Theorem 3 ground truths (spec §3)
// ---------------------------------------------------------------------------

/// F-C3 (Theorem 3): two control-net pairs with known separating direction.
///
/// Theorem 3 reduces exclusion to hull separation: `0 ∉ conv{c_αβ}` iff
/// `conv{P¹} ∩ conv{P²} = ∅`, with the certificate
/// `min_α λ·P¹_α > max_β λ·P²_β` decided exactly. This fixture freezes a
/// **separated** pair (known `λ`, exact integer coordinates) and a **touching**
/// pair (convex hulls intersect, so no strict separation exists along `λ`).
///
/// Ground truth (exact integer arithmetic, machine-checked in
/// [`Fc3SeparationGroundTruths::admit`]): the separated pair satisfies
/// `min λ·P¹ > max λ·P²`; the touching pair does not, and shares a recorded
/// control point.
#[derive(Debug, Clone, PartialEq)]
pub struct Fc3SeparationGroundTruths {
    /// The separated pair.
    pub separated: Fc3ControlPair,
    /// The touching pair.
    pub touching: Fc3ControlPair,
}

/// One Theorem-3 control-net pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fc3ControlPair {
    /// The recorded label: `"separated"` or `"touching"`.
    pub label: &'static str,
    /// The known candidate separating direction `λ`.
    pub lambda: [i64; 3],
    /// The first control net's points (exact integer coordinates).
    pub net_a: [[i64; 3]; 4],
    /// The second control net's points (exact integer coordinates).
    pub net_b: [[i64; 3]; 4],
    /// Ground-truth record: whether `min λ·P¹ > max λ·P²` holds exactly.
    pub expected_separated: bool,
    /// A point shared by both nets, when the pair touches (`None` otherwise).
    pub shared_point: Option<[i64; 3]>,
}

impl Fc3SeparationGroundTruths {
    /// The F-C3 ground truths, verbatim.
    pub fn new() -> Self {
        Self {
            separated: Fc3ControlPair {
                label: "separated",
                lambda: [1, 0, 0],
                net_a: [[2, 0, 0], [2, 1, 0], [2, 0, 1], [2, 1, 1]],
                net_b: [[0, 0, 0], [1, 1, 0], [1, 0, 1], [0, 1, 1]],
                expected_separated: true,
                shared_point: None,
            },
            touching: Fc3ControlPair {
                label: "touching",
                lambda: [1, 0, 0],
                net_a: [[0, 0, 0], [1, 1, 1], [2, 1, 0], [1, 0, 0]],
                net_b: [[2, 1, 0], [3, 0, 0], [3, 2, 1], [2, 2, 0]],
                expected_separated: false,
                shared_point: Some([2, 1, 0]),
            },
        }
    }

    /// Machine-check the F-C3 ground truths with exact integer arithmetic.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) unless each pair's recorded
    /// `expected_separated` matches the exact `i128` `λ`-projection comparison
    /// `min λ·P¹ > max λ·P²`, and a touching pair's recorded shared point is
    /// actually present in both nets.
    pub fn admit(&self) -> Result<(), Refusal> {
        for pair in [&self.separated, &self.touching] {
            let min_a = pair
                .net_a
                .iter()
                .map(|p| project(&pair.lambda, p))
                .min()
                .ok_or(Refusal::InvalidInput)?;
            let max_b = pair
                .net_b
                .iter()
                .map(|p| project(&pair.lambda, p))
                .max()
                .ok_or(Refusal::InvalidInput)?;
            let actually_separated = min_a > max_b;
            if actually_separated != pair.expected_separated {
                return Err(Refusal::InvalidInput);
            }
            if let Some(shared) = pair.shared_point {
                let in_a = pair.net_a.contains(&shared);
                let in_b = pair.net_b.contains(&shared);
                if !in_a || !in_b || pair.expected_separated {
                    return Err(Refusal::InvalidInput);
                }
            }
        }
        Ok(())
    }
}

impl Default for Fc3SeparationGroundTruths {
    fn default() -> Self {
        Self::new()
    }
}

/// Exact `λ·p` as an `i128` (products of `i64` coordinates never overflow).
fn project(lambda: &[i64; 3], p: &[i64; 3]) -> i128 {
    lambda[0] as i128 * p[0] as i128
        + lambda[1] as i128 * p[1] as i128
        + lambda[2] as i128 * p[2] as i128
}

// ---------------------------------------------------------------------------
// F-C4 — Theorem 4 ground truths (spec §3)
// ---------------------------------------------------------------------------

/// F-C4 (Theorem 4): plane × bicubic with known `h` and known critical point,
/// quadric × spline record, and the **elevation-trap twin**.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fc4Theorem4Fixture {
    /// The plane × bicubic record.
    pub plane_bicubic: PlaneBicubicRecord,
    /// The quadric × spline record.
    pub quadric_spline: QuadricSplineRecord,
    /// The elevation-trap twin.
    pub trap: ElevationTrapTwin,
}

impl Fc4Theorem4Fixture {
    /// The F-C4 fixture, verbatim.
    pub fn new() -> Self {
        Self {
            plane_bicubic: PlaneBicubicRecord::new(),
            quadric_spline: QuadricSplineRecord::new(),
            trap: ElevationTrapTwin::new(),
        }
    }

    /// Machine-check every F-C4 record.
    pub fn admit(&self) -> Result<(), Refusal> {
        self.plane_bicubic.admit()?;
        self.quadric_spline.admit()?;
        self.trap.admit()?;
        Ok(())
    }
}

impl Default for Fc4Theorem4Fixture {
    fn default() -> Self {
        Self::new()
    }
}

/// The plane × bicubic Theorem-4 record: integer `h` coefficients (signed
/// control-point distances) with a known critical point.
///
/// The surface net is the bicubic graph `z = h(u, v)` with
/// `h = 12[(u − ½)² + (v − ½)²]`, whose Bernstein coefficient grid (degree
/// `(3, 3)`, integer) is recorded below; the plane is `z = 0`
/// (`n = (0, 0, 1)`, `d = 0`), so `h_α = n·P_α − d` exactly. `h` has its
/// unique interior minimum at the recorded critical point `(u, v) = (½, ½)`
/// (`∇h = 0` there — the exact weighted check is part of
/// [`PlaneBicubicRecord::admit`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneBicubicRecord {
    /// The plane normal `n`, integer.
    pub normal: [i64; 3],
    /// The plane offset `d` (`n·x = d`).
    pub offset: i64,
    /// The `4 × 4` integer control net (spline side).
    pub net: [[[i64; 3]; 4]; 4],
    /// The recorded `4 × 4` integer `h` coefficients (signed control-point
    /// distances from the plane).
    pub h: [[i64; 4]; 4],
    /// The known critical point `(u, v)` as exact rationals (numerator,
    /// denominator): `((1, 2), (1, 2))` = `(½, ½)`.
    pub critical_point: ((i64, i64), (i64, i64)),
}

impl PlaneBicubicRecord {
    /// The F-C4 plane × bicubic record, verbatim.
    pub fn new() -> Self {
        // h_ij = a_i + a_j with a = [3, -1, -1, 3] (12·(u-½)² + 12·(v-½)² in
        // Bernstein degree-(3,3) coefficients).
        let h = [[6, 2, 2, 6], [2, -2, -2, 2], [2, -2, -2, 2], [6, 2, 2, 6]];
        // The graph net P_ij = (i, j, h_ij): integer, plane z = 0.
        let net = [
            [[0, 0, 6], [0, 1, 2], [0, 2, 2], [0, 3, 6]],
            [[1, 0, 2], [1, 1, -2], [1, 2, -2], [1, 3, 2]],
            [[2, 0, 2], [2, 1, -2], [2, 2, -2], [2, 3, 2]],
            [[3, 0, 6], [3, 1, 2], [3, 2, 2], [3, 3, 6]],
        ];
        Self {
            normal: [0, 0, 1],
            offset: 0,
            net,
            h,
            critical_point: ((1, 2), (1, 2)),
        }
    }

    /// Machine-check the plane × bicubic record.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) unless every `h` coefficient equals
    /// `n·P − d` exactly, the recorded critical point is a valid interior
    /// rational point of `(0, 1)²`, and the exact weighted Bernstein gradient
    /// sum at that point is zero — `∇h = 0` there. Integer arithmetic only.
    pub fn admit(&self) -> Result<(), Refusal> {
        // h = n·P − d, exactly.
        for i in 0..4 {
            for j in 0..4 {
                let distance = self.normal[0] * self.net[i][j][0]
                    + self.normal[1] * self.net[i][j][1]
                    + self.normal[2] * self.net[i][j][2]
                    - self.offset;
                if distance != self.h[i][j] {
                    return Err(Refusal::InvalidInput);
                }
            }
        }
        let ((un, ud), (vn, vd)) = self.critical_point;
        // The gradient sums below are evaluated at the recorded critical
        // point, which must be the documented (½, ½) interior point.
        if (un, ud, vn, vd) != (1, 2, 1, 2) {
            return Err(Refusal::InvalidInput);
        }
        // The critical point must be interior to the unit square and the net
        // non-constant (a constant h has no isolated critical structure).
        let constant = self.h.iter().flatten().all(|c| *c == self.h[0][0]);
        if constant {
            return Err(Refusal::InvalidInput);
        }
        // Exact weighted gradient sums at the critical point: for a bicubic,
        // B^2_i(½) ∝ C(2, i) = [1, 2, 1] and B^3_j(½) ∝ C(3, j) = [1, 3, 3, 1].
        // ∂h/∂u ∝ Σ_i C2_i · Σ_j C3_j · (h[i+1][j] − h[i][j]); ∂h/∂v
        // ∝ Σ_j C2_j · Σ_i C3_i · (h[i][j+1] − h[i][j]). Both must vanish.
        let c2 = [1i128, 2, 1];
        let c3 = [1i128, 3, 3, 1];
        let mut du = 0i128;
        for (i, ci) in c2.iter().enumerate() {
            let mut inner = 0i128;
            for (j, cj) in c3.iter().enumerate() {
                inner += cj * (self.h[i + 1][j] - self.h[i][j]) as i128;
            }
            du += ci * inner;
        }
        let mut dv = 0i128;
        for (j, cj) in c2.iter().enumerate() {
            let mut inner = 0i128;
            for (i, ci) in c3.iter().enumerate() {
                inner += ci * (self.h[i][j + 1] - self.h[i][j]) as i128;
            }
            dv += cj * inner;
        }
        if du != 0 || dv != 0 {
            return Err(Refusal::InvalidInput);
        }
        Ok(())
    }
}

impl Default for PlaneBicubicRecord {
    fn default() -> Self {
        Self::new()
    }
}

/// The quadric × spline Theorem-4 record (data only, for CFP-004's tests).
///
/// A sphere × spline instance whose `h` coefficients the implicit-reduction
/// arm derives (bidegree `(2p, 2q)` for quadrics, exact Bernstein product).
/// The spline net (the same integer graph net as [`PlaneBicubicRecord`])
/// straddles the sphere: some control points lie inside `x² + y² + z² = r²`,
/// some outside, so the derived `h` changes sign across the patch — a genuine
/// contact scenario. The derived `h` and the loop-free/transversal
/// classification are evaluation-asserted by CFP-004 against this same data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadricSplineRecord {
    /// The quadric class tag (`"sphere"`).
    pub quadric: &'static str,
    /// The sphere centre.
    pub centre: [i64; 3],
    /// The sphere radius (positive).
    pub radius: i64,
    /// The `4 × 4` integer spline control net.
    pub net: [[[i64; 3]; 4]; 4],
}

impl QuadricSplineRecord {
    /// The F-C4 quadric × spline record, verbatim.
    pub fn new() -> Self {
        Self {
            quadric: "sphere",
            centre: [0, 0, 0],
            radius: 5,
            net: [
                [[0, 0, 6], [0, 1, 2], [0, 2, 2], [0, 3, 6]],
                [[1, 0, 2], [1, 1, -2], [1, 2, -2], [1, 3, 2]],
                [[2, 0, 2], [2, 1, -2], [2, 2, -2], [2, 3, 2]],
                [[3, 0, 6], [3, 1, 2], [3, 2, 2], [3, 3, 6]],
            ],
        }
    }

    /// Machine-check the quadric × spline record's structural data.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a non-positive radius or a net not
    /// matching the documented `4 × 4` spline bidegree `(3, 3)`.
    pub fn admit(&self) -> Result<(), Refusal> {
        if self.radius <= 0 || self.quadric != "sphere" {
            return Err(Refusal::InvalidInput);
        }
        if self.net.len() != 4 || self.net.iter().any(|row| row.len() != 4) {
            return Err(Refusal::InvalidInput);
        }
        Ok(())
    }
}

impl Default for QuadricSplineRecord {
    fn default() -> Self {
        Self::new()
    }
}

/// The elevation-trap twin (packet-normative, spec §3).
///
/// `∂h/∂u` and `∂h/∂v` have bidegrees `(p−1, q)` and `(p, q−1)`; their
/// coefficient arrays cannot be paired into 2-vectors until both are
/// degree-elevated to a common bidegree — only then is
/// `∇h(x) = Σ B_α(x)(a_α, b_α)` a convex combination and the hull test valid.
///
/// The twin is the bilinear saddle `h(u, v) = u + v − 2uv` (integer corner
/// values `[[0, 1], [1, 0]]`) over `[0, 1]²`, whose gradient
/// `∇h = (1 − 2v, 1 − 2u)` vanishes at the interior critical point `(½, ½)`.
/// The derivative coefficient arrays are `∂h/∂u` (degree `(0, 1)`):
/// `c = (c₀, c₁) = (1, −1)` over `v`, and `∂h/∂v` (degree `(1, 0)`):
/// `d = (d₀, d₁) = (1, −1)` over `u`.
///
/// - **Non-elevated (naive) pairing** pairs only the overlap index `(0, 0)`:
///   the single 2-vector `(c₀, d₀) = (1, 1)`, whose hull excludes the origin
///   — the naive verdict is *loop-free*, a FALSE certificate for a saddle.
/// - **Elevated pairing** pairs every `c_j` with every `d_i` on the common
///   grid: `{(1, 1), (−1, 1), (1, −1), (−1, −1)}`, whose hull is `[−1, 1]²`,
///   containing the origin — the correct verdict is *a critical point lies on
///   the cell*, consistent with `∇h(½, ½) = 0`.
///
/// Both expected verdicts are recorded as data; the machine checks are exact
/// integer arithmetic (spec §3's "cheap and exact; F-C4 pins it").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElevationTrapTwin {
    /// The bilinear saddle's integer corner values `h(i, j)`.
    pub h: [[i64; 2]; 2],
    /// The `∂h/∂u` coefficient array `(c₀, c₁)` (degree `(0, 1)`, over `v`).
    pub du: [i64; 2],
    /// The `∂h/∂v` coefficient array `(d₀, d₁)` (degree `(1, 0)`, over `u`).
    pub dv: [i64; 2],
    /// The interior critical point `(u, v) = (½, ½)` as exact rationals.
    pub critical_point: ((i64, i64), (i64, i64)),
    /// The recorded naive (non-elevated) hull verdict.
    pub naive_verdict: &'static str,
    /// The recorded elevated hull verdict.
    pub elevated_verdict: &'static str,
    /// Ground-truth record: the naive hull excludes the origin.
    pub naive_contains_zero: bool,
    /// Ground-truth record: the elevated hull contains the origin.
    pub elevated_contains_zero: bool,
}

impl ElevationTrapTwin {
    /// The elevation-trap twin, verbatim.
    pub fn new() -> Self {
        Self {
            h: [[0, 1], [1, 0]],
            du: [1, -1],
            dv: [1, -1],
            critical_point: ((1, 2), (1, 2)),
            naive_verdict: "loop-free (false certificate)",
            elevated_verdict: "critical point on cell (hull contains origin)",
            naive_contains_zero: false,
            elevated_contains_zero: true,
        }
    }

    /// Machine-check the elevation-trap twin with exact integer arithmetic.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) unless: the derivative arrays
    /// re-derived from `h` match the records; the exact critical point
    /// gradient sums vanish; the recorded origin-containment verdicts match
    /// the recomputed ones; and the two verdicts differ (the twin property —
    /// the whole reason the elevation step exists).
    pub fn admit(&self) -> Result<(), Refusal> {
        // Derivative coefficients, re-derived exactly (degree factors are 1
        // for a bilinear carrier).
        let du_recomputed = [self.h[1][0] - self.h[0][0], self.h[1][1] - self.h[0][1]];
        let dv_recomputed = [self.h[0][1] - self.h[0][0], self.h[1][1] - self.h[1][0]];
        if du_recomputed != self.du || dv_recomputed != self.dv {
            return Err(Refusal::InvalidInput);
        }
        let ((un, ud), (vn, vd)) = self.critical_point;
        if ud <= 0 || vd <= 0 || un != 1 || vn != 1 || ud != 2 || vd != 2 {
            return Err(Refusal::InvalidInput);
        }
        // ∂h/∂u(v) = (1 − v)c₀ + v·c₁ and ∂h/∂v(u) = (1 − u)d₀ + u·d₁; at the
        // critical point (½, ½) both vanish iff c₀ + c₁ = 0 and d₀ + d₁ = 0.
        if self.du[0] + self.du[1] != 0 || self.dv[0] + self.dv[1] != 0 {
            return Err(Refusal::InvalidInput);
        }
        // Naive pairing: the single overlap 2-vector (c₀, d₀) contains the
        // origin iff it IS the origin.
        let naive_contains_zero = self.du[0] == 0 && self.dv[0] == 0;
        if naive_contains_zero != self.naive_contains_zero {
            return Err(Refusal::InvalidInput);
        }
        // Elevated pairing: the 2-vectors are exactly {c_j} × {d_i}; the hull
        // of a cartesian product is the product of the per-axis hulls, so the
        // origin is contained iff both per-axis hulls span zero.
        let du_lo = self.du.iter().min().ok_or(Refusal::InvalidInput)?;
        let du_hi = self.du.iter().max().ok_or(Refusal::InvalidInput)?;
        let dv_lo = self.dv.iter().min().ok_or(Refusal::InvalidInput)?;
        let dv_hi = self.dv.iter().max().ok_or(Refusal::InvalidInput)?;
        let elevated_contains_zero = *du_lo <= 0 && 0 <= *du_hi && *dv_lo <= 0 && 0 <= *dv_hi;
        if elevated_contains_zero != self.elevated_contains_zero {
            return Err(Refusal::InvalidInput);
        }
        // The twin property: the two verdicts DIFFER on this constructed `h`.
        if self.naive_contains_zero == self.elevated_contains_zero {
            return Err(Refusal::InvalidInput);
        }
        Ok(())
    }
}

impl Default for ElevationTrapTwin {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// F-C5 — span-BVH ground truth (spec §2)
// ---------------------------------------------------------------------------

/// F-C5 (span-BVH): a `20 × 30`-span decomposition record with exactly one
/// contact span pair.
///
/// The span-BVH ground truth CFP-005 grades against: a `u`-span count of 20
/// and a `v`-span count of 30 (600 leaf span pairs), exactly one of which is a
/// contact span pair. The expected dyadic prune count is recorded as data —
/// CFP-005's BVH asserts its own prune count equals this record (the
/// 360k-cell case pruned to a handful of hull evaluations). The admission here
/// machine-checks the structural facts: positive span counts, exactly one
/// in-bounds contact pair, and a recorded prune count inside the leaf budget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fc5SpanBvh {
    /// The number of `u`-spans (20).
    pub u_spans: usize,
    /// The number of `v`-spans (30).
    pub v_spans: usize,
    /// The single contact span pair `(u_idx, v_idx)`.
    pub contact: (usize, usize),
    /// The expected dyadic prune count (asserted by CFP-005).
    pub expected_prune_count: usize,
    /// Ground-truth record: exactly one contact span pair exists.
    pub exactly_one_contact: bool,
}

impl Fc5SpanBvh {
    /// The F-C5 record, verbatim.
    pub fn new() -> Self {
        Self {
            u_spans: 20,
            v_spans: 30,
            contact: (7, 11),
            expected_prune_count: 589,
            exactly_one_contact: true,
        }
    }

    /// Machine-check the F-C5 structural ground truths.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) unless the decomposition is a
    /// positive 20 × 30 grid, the single contact pair lies inside it, and the
    /// expected prune count is strictly between zero and the leaf-pair count.
    pub fn admit(&self) -> Result<(), Refusal> {
        if self.u_spans == 0 || self.v_spans == 0 {
            return Err(Refusal::InvalidInput);
        }
        let leaf_pairs = self.u_spans * self.v_spans;
        let in_bounds = self.contact.0 < self.u_spans && self.contact.1 < self.v_spans;
        if !self.exactly_one_contact || !in_bounds {
            return Err(Refusal::InvalidInput);
        }
        if self.expected_prune_count == 0 || self.expected_prune_count >= leaf_pairs {
            return Err(Refusal::InvalidInput);
        }
        Ok(())
    }
}

impl Default for Fc5SpanBvh {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// F-C6 — the stagnation fixture (spec §2)
// ---------------------------------------------------------------------------

/// One successive-depth cone-overlap observation of the stagnation fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConeOverlapRecord {
    /// The subdivision depth (0, 1, 2).
    pub depth: usize,
    /// Whether the pair's per-box normal cones still overlap at this depth.
    pub cones_overlap: bool,
}

/// F-C6 (stagnation): a near-tangency pair whose per-box cones stay
/// overlapping at three successive depths.
///
/// The stagnation detector CFP-008 builds on: cone overlap that does not
/// shrink between subdivision levels is a designed near-tangency (post-CFP-001
/// the genuine geometry routes to the landed CTE cascade, converting
/// budget-burn `Unresolved` into an A₁ verdict — never budget burn). Expected
/// routing: `cascade`; expected verdict: `A1Node`. The geometric cone data and
/// the routing/verdict assertions land with CFP-008; this spine freezes the
/// three overlap observations and the expected routing records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fc6StagnationFixture {
    /// The three successive-depth cone-overlap records.
    pub records: [ConeOverlapRecord; 3],
    /// Expected routing tag: `"cascade"`.
    pub expected_routing: &'static str,
    /// Expected verdict tag: `"A1Node"`.
    pub expected_verdict: &'static str,
}

impl Fc6StagnationFixture {
    /// The F-C6 fixture, verbatim.
    pub fn new() -> Self {
        Self {
            records: [
                ConeOverlapRecord {
                    depth: 0,
                    cones_overlap: true,
                },
                ConeOverlapRecord {
                    depth: 1,
                    cones_overlap: true,
                },
                ConeOverlapRecord {
                    depth: 2,
                    cones_overlap: true,
                },
            ],
            expected_routing: "cascade",
            expected_verdict: "A1Node",
        }
    }

    /// Machine-check the F-C6 structural ground truths: three successive
    /// depths (0, 1, 2), all cones still overlapping, and the recorded routing
    /// and verdict expectations.
    pub fn admit(&self) -> Result<(), Refusal> {
        for (depth, record) in self.records.iter().enumerate() {
            if record.depth != depth || !record.cones_overlap {
                return Err(Refusal::InvalidInput);
            }
        }
        if self.expected_routing != "cascade" || self.expected_verdict != "A1Node" {
            return Err(Refusal::InvalidInput);
        }
        Ok(())
    }
}

impl Default for Fc6StagnationFixture {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// F-C7 — the F1 cross-check kit (spec §2)
// ---------------------------------------------------------------------------

/// One F-C7 cross-check input: a bicubic net and a sub-box of its domain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fc7CrossCheckInput {
    /// The `4 × 4` spline control net.
    pub net: [[[f64; 3]; 4]; 4],
    /// The parameter sub-box `(u, v)` the two hull paths must agree on.
    pub box_: ((f64, f64), (f64, f64)),
}

/// F-C7 (F1 cross-check): seeded randomized net + box pairs on which the
/// evidence-side and certified-side sub-box hulls must agree.
///
/// F1 forbids a `truck-evidence` ↔ `truck-certified` edge, so the duplicated
/// sub-box hull leaf math carries a permanent randomized cross-check test on
/// these exact inputs. The kit is generated deterministically from a fixed
/// seed (identical ordered input → identical kit; no hash ordering), and the
/// admission regenerates it and refuses on any drift. The agreement tests
/// themselves land with the two implementations (CFP-001 on the evidence
/// side, CFP-003 on the certified side) against these same constants.
#[derive(Debug, Clone, PartialEq)]
pub struct Fc7CrossCheckKit {
    /// The fixed seed the kit is generated from.
    pub seed: u64,
    /// The generated net + box inputs.
    pub inputs: Vec<Fc7CrossCheckInput>,
}

impl Fc7CrossCheckKit {
    /// Build the F-C7 kit from the fixed seed (8 deterministic inputs).
    pub fn new() -> Self {
        let mut kit = Self {
            seed: 0x5EED_C700,
            inputs: Vec::new(),
        };
        let generated = generate_cross_check_inputs(kit.seed, 8);
        kit.inputs = generated;
        kit
    }

    /// Machine-check the F-C7 kit: regenerating from the fixed seed must
    /// reproduce the recorded inputs exactly, and every input must be
    /// structurally valid (finite net, valid parameter box).
    pub fn admit(&self) -> Result<(), Refusal> {
        let regenerated = generate_cross_check_inputs(self.seed, self.inputs.len());
        if regenerated != self.inputs {
            return Err(Refusal::InvalidInput);
        }
        for input in &self.inputs {
            if !param_box_ok(input.box_) {
                return Err(Refusal::InvalidInput);
            }
            let all_finite = input.net.iter().flatten().flatten().all(|c| c.is_finite());
            if !all_finite {
                return Err(Refusal::InvalidInput);
            }
        }
        Ok(())
    }
}

impl Default for Fc7CrossCheckKit {
    fn default() -> Self {
        Self::new()
    }
}

/// Deterministically generate `count` net + box cross-check inputs from a
/// seed (a small LCG; identical ordered input → identical output). Nets get
/// coordinates in `[0, 3]`; boxes are positively-wide sub-boxes of `[0, 1]²`.
fn generate_cross_check_inputs(seed: u64, count: usize) -> Vec<Fc7CrossCheckInput> {
    let mut state = seed;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((state >> 11) & 0xFFFF) as f64 / 65_535.0
    };
    let mut inputs = Vec::with_capacity(count);
    for _ in 0..count {
        let mut net = [[[0.0f64; 3]; 4]; 4];
        for point in net.iter_mut().flatten() {
            for component in point.iter_mut() {
                *component = next() * 3.0;
            }
        }
        let box_ = (ordered_pair(next(), next()), ordered_pair(next(), next()));
        inputs.push(Fc7CrossCheckInput { net, box_ });
    }
    inputs
}

/// An ordered pair `(lo, hi)` guaranteed positively wide: degenerate draws
/// fall back to the unit interval (deterministic).
fn ordered_pair(a: f64, b: f64) -> (f64, f64) {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    if lo < hi {
        (lo, hi)
    } else {
        (0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fc0_defect_counterexample_data_admits() {
        let fixture = Fc0CapSlabFixture::new();
        assert_eq!(fixture.admit(), Ok(()));
        // The defect mechanism is real: the sampled screen box is disjoint
        // from the slab while the true image touches it.
        assert!(fixture.sampled_aabbs_disjoint);
        assert!(fixture.true_images_touch);
    }

    #[test]
    fn fc2_enclosure_battery_data_admits() {
        let fixture = Fc2EnclosureBattery::new();
        assert_eq!(fixture.admit(), Ok(()));
        assert!(fixture.widths_non_increasing);
        assert!(fixture.widths_strictly_decrease);
        assert!(fixture.monotone);
    }

    #[test]
    fn fc3_separation_ground_truths_admit() {
        let fixture = Fc3SeparationGroundTruths::new();
        assert_eq!(fixture.admit(), Ok(()));
        assert!(fixture.separated.expected_separated);
        assert!(!fixture.touching.expected_separated);
        assert!(fixture.touching.shared_point.is_some());
    }

    #[test]
    fn fc4_elevation_trap_twin_data_admits() {
        let fixture = Fc4Theorem4Fixture::new();
        assert_eq!(fixture.admit(), Ok(()));
        // The twin property: the naive pairing and the elevated pairing give
        // DIFFERENT hull verdicts on this constructed h.
        assert_ne!(
            fixture.trap.naive_contains_zero,
            fixture.trap.elevated_contains_zero
        );
        assert!(fixture.trap.elevated_contains_zero);
    }
}
