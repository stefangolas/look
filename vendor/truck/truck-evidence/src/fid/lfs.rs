//! BG-FID-001: primitive certified face-scale evidence.
//!
//! The scaffold contract prose is preserved: the certified quantity is a
//! LOWER bound on local feature size, typed `LfsLowerBound` and never a bare
//! `lfs` — a bare name invites a future call site to read the bound as an
//! equality. **Bound direction** (BG-FID-007): every downstream gate has the
//! form `q < c * lfs_lower`, so substituting a LOWER bound is conservative:
//! it can refuse an instance the true value would admit, and can never admit
//! one the true value would refuse. **Refusals are epistemic**: a refusal
//! asserts the bound could not be CERTIFIED, not that the feature is small.
//!
//! This packet ships **primitive evidence only** — three independently
//! certified component directions per face cell
//! ([`FaceScaleComponents`]) and a local, witnessed-scope wedge slope
//! ([`WedgeSlopeLowerBound`]). The certificate types that WOULD overclaim
//! (`TubeWidthLowerBound`, `ChiLowerBound`) are deliberately not created here:
//! Federer's closed-manifold reach decomposition does not transfer to trimmed
//! patches by citation (open obligation L-FEDERER-PATCH), so no computed
//! quantity may claim to bound a tube width, reach or lfs — even though each
//! COMPONENT direction is certifiable today. The wedge bound is local,
//! witnessed-scope only: what [CCSL09] Def 4.3 defines is `chi_K(t)`, an
//! infimum over an entire distance locus; what BG-INV-109 actually witnesses
//! is ONE point per edge (a midpoint normal-pair) with a certified lower
//! bound on `sin phi`. That supports a LOCAL normalized-slope lower bound and
//! nothing more.

#![deny(clippy::unwrap_used)]

use crate::enclosure::{Box3, EnclosureSurface, Interval};

/// A typed refusal for the primitive evidence functions.
///
/// Refusal provenance is part of this kernel's semantics; it is not thrown
/// away. Every public function returns `Result<T, FidRefusal>`; each cause is
/// mapped exactly as named so a caller can distinguish "geometry too curved
/// to certify here" from "you gave me garbage".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FidRefusal {
    /// The immersion margin collapsed on this cell: the normal direction is
    /// not certifiably well-defined there.
    ImmersionUnresolved,
    /// The first-form eigenvalue bracket could not certify a positive lower
    /// bound on this cell (cell too wide for the interval arithmetic).
    MetricLowerBoundUnresolved,
    /// An input margin was outside its mathematical domain.
    InvalidMargin,
    /// Fewer than two witness boxes were supplied where two are required.
    InsufficientWitnesses,
}

/// Three independently certified component directions for one face cell.
///
/// This type makes NO claim about tubes, reach or feature size: composing
/// these into a tube-width statement requires L-FEDERER-PATCH (open). What
/// each field certifies is exactly one direction: a lower bound on the
/// face-interior radius of curvature, a lower bound on the distance to
/// non-incident sheets, and a lower bound on the distance to the face's own
/// boundary.
///
/// Empty-set semantics, explicit: `d(A, ∅) = +∞`; both distance components
/// are `+∞` when their slice is empty, and `conservative_min()` of components
/// including `+∞` ignores them exactly as extended reals. Infinity is
/// intentional and permitted (a plane is flat, so its curvature radius is
/// `+∞`).
///
/// @feeds [CCS05, Thm 2.1:H2]            # would supply thickening containment
/// @via-open-lemma FID-L-TUBE
/// @establishes
///   component-wise certified directions (this struct)
/// @does-not-establish
///   topological thickening | local reach | isotopy
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceScaleComponents {
    /// From [`curvature_radius_lower`]; `+inf` permitted (flat cell).
    pub curvature_radius_lower: f64,
    /// `d(cell image, exclusion boxes)`; `+inf` when the slice is empty.
    pub nonincident_separation_lower: f64,
    /// `d(cell image, boundary boxes)`; `+inf` when the slice is empty.
    pub boundary_distance_lower: f64,
}

impl FaceScaleComponents {
    /// The conservative three-way minimum, treating `+∞` components as
    /// absent exactly as extended reals do.
    pub fn conservative_min(&self) -> f64 {
        self.curvature_radius_lower
            .min(self.nonincident_separation_lower)
            .min(self.boundary_distance_lower)
    }
}

/// Certified component directions for one face cell.
///
/// The cell image is enclosed by `surface.enclose(cell)`; `box_distance` to
/// every exclusion box is then a certified lower bound on the point-set
/// distance, because the enclosure contains the whole image and the boxes are
/// certified boxes for the structures to exclude.
///
/// @feeds [CCS05, Thm 2.1:H2]            # would supply thickening containment
/// @via-open-lemma FID-L-TUBE
/// @establishes
///   component-wise certified directions (this struct)
/// @does-not-establish
///   topological thickening | local reach | isotopy
pub fn face_scale_components(
    surface: &impl EnclosureSurface,
    cell: (Interval, Interval),
    nonincident_boxes: &[Box3],
    boundary_boxes: &[Box3],
) -> Result<FaceScaleComponents, FidRefusal> {
    let curvature_radius_lower = curvature_radius_lower(surface, cell)?;
    let image = surface.enclose(cell.0, cell.1);
    let nonincident_separation_lower = separation_lower(&image, nonincident_boxes);
    let boundary_distance_lower = separation_lower(&image, boundary_boxes);
    Ok(FaceScaleComponents {
        curvature_radius_lower,
        nonincident_separation_lower,
        boundary_distance_lower,
    })
}

/// A certified lower bound on the face-interior radius of curvature over the
/// cell.
///
/// Implemented exactly as scratch-validated on the sphere carrier, and sound
/// everywhere it answers. The first-form eigenvalue bracket is
///
/// ```text
/// lambda_min([E F; F G]) >= ((E+G) - sqrt((E-G)^2 + 4F^2)) / 2
/// ```
///
/// at interval worst-cases, PROVIDED `sup((E-G)^2) <= delta_mag^2` (true by
/// construction: `|E-G| <= max(|E.sup - G.inf|, |G.sup - E.inf|)`) and
/// `sup(F^2) <= f_mag^2` (the F-magnitude term: `f_mag = max(|F.inf|,
/// |F.sup|)`). **Using `F.sup^2` there is a SOUNDNESS REVERSAL** — for
/// `F = [-10,-1]`, `sup^2` reads 1 while `sup|F^2|` is 100, which inflates
/// `lam_min_lo` and DEFLATES the curvature bound. Normalization uses the
/// carrier's own `immersion_lower_bound` (the iota route). The
/// sum-of-coefficients numerator is deliberately coarse; over-estimation
/// costs only eps budget downstream.
///
/// @feeds [CCS05, Thm 2.1:H2]            # supplies the face-interior curvature direction
/// @via-open-lemma FID-L-FEDERER-PATCH
/// @establishes a certified lower bound on the face-interior radius of
///   curvature over the cell
/// @does-not-establish any tube-width, reach or lfs claim
pub fn curvature_radius_lower(
    surface: &impl EnclosureSurface,
    cell: (Interval, Interval),
) -> Result<f64, FidRefusal> {
    let (uu, vv) = cell;
    let su = surface.enclose_der(1, 0, uu, vv);
    let sv = surface.enclose_der(0, 1, uu, vv);
    let s2u = surface.enclose_der(2, 0, uu, vv);
    let s12 = surface.enclose_der(1, 1, uu, vv);
    let s2v = surface.enclose_der(0, 2, uu, vv);
    let e = dot_box(&su, &su);
    let f = dot_box(&su, &sv);
    let g = dot_box(&sv, &sv);
    let n_raw = cross_box(&su, &sv);
    let iota = surface.immersion_lower_bound(uu, vv);
    if iota <= 0.0 {
        return Err(FidRefusal::ImmersionUnresolved);
    }
    let l_up = mag_up(dot_box(&s2u, &n_raw)) / iota;
    let m_up = mag_up(dot_box(&s12, &n_raw)) / iota;
    let n_up = mag_up(dot_box(&s2v, &n_raw)) / iota;
    let delta_mag = (e.sup() - g.inf()).abs().max((g.sup() - e.inf()).abs());
    let f_mag = f.inf().abs().max(f.sup().abs());
    let disc_up = (delta_mag * delta_mag + 4.0 * f_mag * f_mag).sqrt();
    let lam_min_lo = 0.5 * (e.inf() + g.inf() - disc_up);
    if lam_min_lo <= 0.0 {
        return Err(FidRefusal::MetricLowerBoundUnresolved);
    }
    let k_up = (l_up + m_up + n_up) / lam_min_lo;
    if k_up == 0.0 {
        // Flat within enclosure (a plane, say): every numerator is zero.
        return Ok(f64::INFINITY);
    }
    Ok(1.0 / k_up)
}

/// The witnessed scope of the wedge-slope bound.
///
/// @definition [CCSL09, Def 4.3]          # chi_K - a definition, not an instance
/// @uses-lemma FID-L-WEDGE-SLOPE
/// @establishes local normalized-slope lower bound at the witnessed point
/// @does-not-establish global chi_K
/// @feeds-open-lemma FID-L-COVERAGE
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WedgeScope {
    /// BG-INV-109 v1 samples a POINT: the edge's parameter midpoint.
    EdgeMidpointWitness,
}

/// A local, witnessed-scope lower bound on the normalized slope at a wedge.
///
/// This is NOT `ChiLowerBound`: `chi_K(t)` infers over an entire distance
/// locus; promoting local wedge evidence to it requires L-COVERAGE — future
/// type-level promotion, not prose.
///
/// KNOWN LIMITATION, documented not hidden: at `sin_margin = 1` the bound
/// still reports `1/sqrt(2)` because a sine certificate cannot see branch
/// identity. Distinguishing healthy near-flat (`dot(n_A,n_B) >= c`) from
/// near-knife (`dot <= -c`) needs SIGNED alignment evidence BG-INV-109 lacks.
/// To improve THIS lower bound one wants an upper bound on the normal angle,
/// i.e. a lower bound on the dot product — extending INV-109 is future work.
///
/// @definition [CCSL09, Def 4.3]          # chi_K - a definition, not an instance
/// @uses-lemma FID-L-WEDGE-SLOPE
/// @establishes local normalized-slope lower bound at the witnessed point
/// @does-not-establish global chi_K
/// @feeds-open-lemma FID-L-COVERAGE
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WedgeSlopeLowerBound {
    /// The certified lower bound on the local normalized slope.
    pub value: f64,
    /// The scope the certificate is valid over.
    pub scope: WedgeScope,
}

/// The certified normalized-slope lower bound from a sine margin.
///
/// Derivation (this IS the deliverable):
///
/// - For a wedge whose adjacent face normals make angle `phi in [0, pi]`:
///   `d(0, conv{n_A, n_B}) = cos(phi/2)` — the local normalized-slope value
///   on the bisector region. It dies correctly at BOTH knife degeneracies
///   (folded `psi -> 0` and crack `psi -> 2pi` force antiparallel normals,
///   `phi -> pi`) and equals 1 when flat (`phi -> 0`).
/// - BG-INV-109 certifies `sin phi >= sin_margin`, i.e.
///   `phi in [arcsin s, pi - arcsin s]`; `cos(phi/2)` is decreasing there, so
///   the sound WORST case sits at the right endpoint:
///   `cos((pi - arcsin s)/2) = sin(arcsin s / 2)` — the formula below.
///   Monotone increasing in `s`; goes to 0 when no non-degeneracy is
///   certified.
///
/// `InvalidMargin` unless `0 < sin_margin <= 1`.
///
/// @definition [CCSL09, Def 4.3]          # chi_K - a definition, not an instance
/// @uses-lemma FID-L-WEDGE-SLOPE
/// @establishes local normalized-slope lower bound at the witnessed point
/// @does-not-establish global chi_K
/// @feeds-open-lemma FID-L-COVERAGE
pub fn wedge_slope_lower_from_sin_margin(
    sin_margin: f64,
) -> Result<WedgeSlopeLowerBound, FidRefusal> {
    if !(sin_margin > 0.0 && sin_margin <= 1.0) {
        return Err(FidRefusal::InvalidMargin);
    }
    let tt = Interval::try_from((sin_margin, sin_margin)).unwrap_or(Interval::EMPTY);
    let one = Interval::try_from((1.0, 1.0)).unwrap_or(Interval::EMPTY);
    let two = Interval::try_from((2.0, 2.0)).unwrap_or(Interval::EMPTY);
    let sixteen = Interval::try_from((16.0, 16.0)).unwrap_or(Interval::EMPTY);
    let seven = Interval::try_from((7.0, 7.0)).unwrap_or(Interval::EMPTY);
    let two56 = Interval::try_from((256.0, 256.0)).unwrap_or(Interval::EMPTY);
    let series_cutoff = 1.0e-6; // H-3: dimensionless sine-margin cancellation threshold, not a length
    let value = if sin_margin < series_cutoff {
        // s/2 + s³/16 + 7s⁵/256, all terms positive (certified downward).
        let s2 = tt * tt;
        let s3 = tt * s2;
        let s5 = s3 * s2;
        (tt / two + s3 / sixteen + s5 * seven / two56).inf()
    } else {
        let inner = (one - (one - tt * tt).sqrt()) / two;
        inner.sqrt().inf()
    };
    Ok(WedgeSlopeLowerBound {
        value,
        scope: WedgeScope::EdgeMidpointWitness,
    })
}

/// The interval dot product of two boxes, an enclosure of
/// `{ a · b : a in A, b in B }`.
fn dot_box(a: &Box3, b: &Box3) -> Interval {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// The interval cross product of two boxes (duplicated locally; enclosure.rs
/// visibility stays untouched). Sound but loose, exactly as in the crate's
/// shared helper.
fn cross_box(a: &Box3, b: &Box3) -> Box3 {
    Box3 {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

/// `max(|i.inf|, |i.sup|)` — an upper bound on `|v|` for every `v` in `i`.
fn mag_up(i: Interval) -> f64 {
    i.inf().abs().max(i.sup().abs())
}

/// A lower bound on the point-set distance between two boxes: per-axis
/// `max(lo_b - hi_a, lo_a - hi_b)` clamped at 0, Euclidean-combined.
fn box_distance(a: &Box3, b: &Box3) -> f64 {
    let gap = |lo_a: f64, hi_a: f64, lo_b: f64, hi_b: f64| (lo_b - hi_a).max(lo_a - hi_b).max(0.0);
    let dx = gap(a.x.inf(), a.x.sup(), b.x.inf(), b.x.sup());
    let dy = gap(a.y.inf(), a.y.sup(), b.y.inf(), b.y.sup());
    let dz = gap(a.z.inf(), a.z.sup(), b.z.inf(), b.z.sup());
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// The distance lower bound from the enclosed cell image to the nearest box
/// in `boxes`, or `+∞` when `boxes` is empty (`d(A, ∅) = +∞`, explicitly).
fn separation_lower(image: &Box3, boxes: &[Box3]) -> f64 {
    if boxes.is_empty() {
        return f64::INFINITY;
    }
    boxes
        .iter()
        .map(|b| box_distance(image, b))
        .fold(f64::INFINITY, f64::min)
}

#[cfg(test)]
mod tests {
    // GATE-1: the fid module (including its test module) stays under the
    // crate's unwrap denial; unit tests assert on hand-built witnesses, and
    // `must` below is the deny-clean spelling of an unwrap.
    #![deny(clippy::unwrap_used)]

    use super::*;
    use truck_base::cgmath64::{EuclideanSpace, InnerSpace, Matrix3, Point3, Rad, Vector3};
    use truck_geometry::specifieds::{Plane, Sphere};

    /// A test interval, degrading to EMPTY (and failing the test that uses
    /// it) rather than panicking on a malformed bound.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// Test-only unwrap that stays under the crate's deny list: unit tests
    /// assert on hand-built witnesses, so a refusal here is a test bug.
    fn must<T>(r: Result<T, FidRefusal>) -> T {
        match r {
            Ok(value) => value,
            Err(_) => unreachable!("unit-test witness must certify"),
        }
    }

    /// Compare two finite floats within `slack`; equal infinities pass.
    fn assert_close(a: f64, b: f64, slack: f64, what: &str) {
        if a.is_infinite() {
            assert!(b.is_infinite(), "{what}: {a} vs {b}");
        } else {
            assert!(
                (a - b).abs() <= slack,
                "{what}: bound moved from {a} to {b}"
            );
        }
    }

    /// The half-thickness of the cube's edge witness boxes, in cube-length
    /// units.
    const EDGE_HALF: f64 = 0.05; // H-3: dimensionless half-thickness of the edge witness boxes, not a bare length
    /// The interior face-parameter cell used across the cube tests.
    const CELL_LO: f64 = 0.4; // H-3: dimensionless lower face parameter of the interior cell
    const CELL_HI: f64 = 0.6; // H-3: dimensionless upper face parameter of the interior cell
    /// The true nearest-edge distance of the interior cell to an edge box:
    /// `CELL_LO - EDGE_HALF`.
    const NEAREST_EDGE: f64 = 0.35; // H-3: true nearest-edge distance in cube-length units, an upper bound the bound may not exceed
    /// The true distance from the cell to the opposite sheet of the unit cube.
    const NEIGHBOURING_SHEET: f64 = 1.0; // H-3: true opposite-sheet distance in cube-length units, an upper bound
    /// Float slack for AABB-distance comparisons (the Euclidean combine
    /// rounds in f64, not outward).
    const AABB_SLACK: f64 = 1.0e-9; // H-3: float slack between two AABB distances, dimensionless, not a length

    /// One face's witness configuration: the carrier plane, the parameter
    /// cell, its own boundary boxes, and the non-incident exclusion boxes.
    struct CubeFaceConfig {
        plane: Plane,
        cell: (Interval, Interval),
        boundary_boxes: Vec<Box3>,
        nonincident_boxes: Vec<Box3>,
    }

    /// The three faces of the unit cube `[0,1]^3` used by the tests: `z = 0`
    /// (bottom), `x = 1` (right), `y = 1` (back). Every witness point passes
    /// through `motion` so one builder serves the base, translated and
    /// rotated configurations. Boxes are re-AABB'd after the motion; a rigid
    /// motion preserves the true distances, and re-AABBing only ever loses
    /// tightness, never soundness.
    fn cube_faces(motion: impl Fn(Point3) -> Point3) -> [CubeFaceConfig; 3] {
        let cell = (iv(CELL_LO, CELL_HI), iv(CELL_LO, CELL_HI));
        let h = EDGE_HALF;
        [
            // Bottom face z = 0: parameter (u, v) maps to (u, v, 0).
            CubeFaceConfig {
                plane: Plane::new(
                    motion(Point3::new(0.0, 0.0, 0.0)),
                    motion(Point3::new(1.0, 0.0, 0.0)),
                    motion(Point3::new(0.0, 1.0, 0.0)),
                ),
                cell,
                boundary_boxes: vec![
                    // The four edges of the z = 0 square.
                    box_aabb(&motion, (-h, 0.0, -h), (h, 1.0, h)),
                    box_aabb(&motion, (1.0 - h, 0.0, -h), (1.0 + h, 1.0, h)),
                    box_aabb(&motion, (0.0, -h, -h), (1.0, h, h)),
                    box_aabb(&motion, (0.0, 1.0 - h, -h), (1.0, 1.0 + h, h)),
                ],
                // The top sheet z = 1 is the only non-incident face.
                nonincident_boxes: vec![box_aabb(&motion, (0.0, 0.0, 1.0), (1.0, 1.0, 1.0))],
            },
            // Right face x = 1: parameter (u, v) maps to (1, u, v).
            CubeFaceConfig {
                plane: Plane::new(
                    motion(Point3::new(1.0, 0.0, 0.0)),
                    motion(Point3::new(1.0, 1.0, 0.0)),
                    motion(Point3::new(1.0, 0.0, 1.0)),
                ),
                cell,
                boundary_boxes: vec![
                    // The four edges of the x = 1 square.
                    box_aabb(&motion, (1.0 - h, -h, 0.0), (1.0 + h, h, 1.0)),
                    box_aabb(&motion, (1.0 - h, 1.0 - h, 0.0), (1.0 + h, 1.0 + h, 1.0)),
                    box_aabb(&motion, (1.0 - h, 0.0, -h), (1.0 + h, 1.0, h)),
                    box_aabb(&motion, (1.0 - h, 0.0, 1.0 - h), (1.0 + h, 1.0, 1.0 + h)),
                ],
                // The opposite sheet x = 0 is the only non-incident face.
                nonincident_boxes: vec![box_aabb(&motion, (0.0, 0.0, 0.0), (0.0, 1.0, 1.0))],
            },
            // Back face y = 1: parameter (u, v) maps to (u, 1, v).
            CubeFaceConfig {
                plane: Plane::new(
                    motion(Point3::new(0.0, 1.0, 0.0)),
                    motion(Point3::new(1.0, 1.0, 0.0)),
                    motion(Point3::new(0.0, 1.0, 1.0)),
                ),
                cell,
                boundary_boxes: vec![
                    // The four edges of the y = 1 square.
                    box_aabb(&motion, (-h, 1.0 - h, 0.0), (h, 1.0 + h, 1.0)),
                    box_aabb(&motion, (1.0 - h, 1.0 - h, 0.0), (1.0 + h, 1.0 + h, 1.0)),
                    box_aabb(&motion, (0.0, 1.0 - h, -h), (1.0, 1.0 + h, h)),
                    box_aabb(&motion, (0.0, 1.0 - h, 1.0 - h), (1.0, 1.0 + h, 1.0 + h)),
                ],
                // The opposite sheet y = 0 is the only non-incident face.
                nonincident_boxes: vec![box_aabb(&motion, (0.0, 0.0, 0.0), (1.0, 0.0, 1.0))],
            },
        ]
    }

    /// The axis-aligned bounding box of the eight transformed corners of the
    /// box with corners `lo` and `hi` (base coordinates).
    fn box_aabb(
        motion: &impl Fn(Point3) -> Point3,
        lo: (f64, f64, f64),
        hi: (f64, f64, f64),
    ) -> Box3 {
        let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for &x in &[lo.0, hi.0] {
            for &y in &[lo.1, hi.1] {
                for &z in &[lo.2, hi.2] {
                    let p = motion(Point3::new(x, y, z));
                    min.x = min.x.min(p.x);
                    min.y = min.y.min(p.y);
                    min.z = min.z.min(p.z);
                    max.x = max.x.max(p.x);
                    max.y = max.y.max(p.y);
                    max.z = max.z.max(p.z);
                }
            }
        }
        Box3 {
            x: iv(min.x, max.x),
            y: iv(min.y, max.y),
            z: iv(min.z, max.z),
        }
    }

    #[test]
    fn cube_face_components_upper_bound() {
        let faces = cube_faces(|p| p);
        for face in faces.iter() {
            let c = must(face_scale_components(
                &face.plane,
                face.cell,
                &face.nonincident_boxes,
                &face.boundary_boxes,
            ));
            assert!(
                c.conservative_min() <= NEAREST_EDGE + AABB_SLACK,
                "conservative_min {} exceeds the nearest-edge bound {}",
                c.conservative_min(),
                NEAREST_EDGE
            );
            assert!(
                c.conservative_min() <= NEIGHBOURING_SHEET + AABB_SLACK,
                "conservative_min {} exceeds the neighbouring-sheet bound {}",
                c.conservative_min(),
                NEIGHBOURING_SHEET
            );
        }
    }

    #[test]
    fn global_scale_zero_stratified_positive() {
        // The cube's GLOBAL feature size is 0 (it collapses at every sharp
        // edge), yet the stratified, per-cell directions are all positive on
        // interior cells. Anti-regression against a future global-reach
        // shortcut.
        let faces = cube_faces(|p| p);
        for face in faces.iter() {
            let c = must(face_scale_components(
                &face.plane,
                face.cell,
                &face.nonincident_boxes,
                &face.boundary_boxes,
            ));
            assert!(
                c.curvature_radius_lower > 0.0,
                "curvature radius must be positive on a plane cell"
            );
            assert!(
                c.nonincident_separation_lower > 0.0,
                "non-incident separation must be positive on an interior cell"
            );
            assert!(
                c.boundary_distance_lower > 0.0,
                "boundary distance must be positive on an interior cell"
            );
        }
    }

    #[test]
    fn translation_invariance() {
        // Translation by a nonzero vector: every component equal within a
        // tiny slack (AABB box-distance IS translation invariant).
        const TX: f64 = 0.5; // H-3: translation in x, in cube-length units
        const TY: f64 = -1.3; // H-3: translation in y, in cube-length units
        const TZ: f64 = 2.7; // H-3: translation in z, in cube-length units
        let t = Vector3::new(TX, TY, TZ);
        let base = cube_faces(|p| p);
        let moved = cube_faces(|p| p + t);
        for (a, b) in base.iter().zip(moved.iter()) {
            let ca = must(face_scale_components(
                &a.plane,
                a.cell,
                &a.nonincident_boxes,
                &a.boundary_boxes,
            ));
            let cb = must(face_scale_components(
                &b.plane,
                b.cell,
                &b.nonincident_boxes,
                &b.boundary_boxes,
            ));
            assert_close(
                ca.curvature_radius_lower,
                cb.curvature_radius_lower,
                AABB_SLACK,
                "curvature radius under translation",
            );
            assert_close(
                ca.nonincident_separation_lower,
                cb.nonincident_separation_lower,
                AABB_SLACK,
                "non-incident separation under translation",
            );
            assert_close(
                ca.boundary_distance_lower,
                cb.boundary_distance_lower,
                AABB_SLACK,
                "boundary distance under translation",
            );
        }
    }

    #[test]
    fn rotated_configuration_stays_sound() {
        // Rotate the whole configuration: do NOT assert equality (AABB
        // separation bounds are not rotation-tight); assert each rotated
        // conservative_min() is positive AND still a lower bound on the true
        // hand-computed distance.
        const ROT_DEG: f64 = 12.0; // H-3: rotation angle in degrees, dimensionless
        const ROT_ANGLE_RAD: f64 = ROT_DEG * core::f64::consts::PI / 180.0; // H-3: the same rotation angle in radians, dimensionless
        let rot = Matrix3::from_axis_angle(Vector3::unit_z(), Rad(ROT_ANGLE_RAD));
        let rotated = cube_faces(|p| Point3::from_vec(rot * p.to_vec()));
        for face in rotated.iter() {
            let c = must(face_scale_components(
                &face.plane,
                face.cell,
                &face.nonincident_boxes,
                &face.boundary_boxes,
            ));
            assert!(
                c.conservative_min() > 0.0,
                "rotated conservative_min must stay positive, got {}",
                c.conservative_min()
            );
            assert!(
                c.conservative_min() <= NEAREST_EDGE + AABB_SLACK,
                "rotated conservative_min {} exceeds the nearest-edge bound {}",
                c.conservative_min(),
                NEAREST_EDGE
            );
            assert!(
                c.conservative_min() <= NEIGHBOURING_SHEET + AABB_SLACK,
                "rotated conservative_min {} exceeds the neighbouring-sheet bound {}",
                c.conservative_min(),
                NEIGHBOURING_SHEET
            );
        }
    }

    #[test]
    fn wedge_slope_monotone_and_knife_limit() {
        // Monotone increasing in sin_margin over (0, 1]; goes to 0 as s -> 0;
        // refuses at s = 0 and s > 1 (InvalidMargin). An exactly antiparallel
        // wedge measures sin phi = 0, which the underlying INV-109 check would
        // fail; the refusal propagates.
        const SAMPLES: [f64; 6] = [0.1, 0.3, 0.5, 0.7, 0.9, 1.0]; // H-3: dimensionless sine margins, monotone-increasing witness points
        let mut prev = 0.0;
        for &s in SAMPLES.iter() {
            let bound = must(wedge_slope_lower_from_sin_margin(s));
            assert!(
                bound.value > prev,
                "bound must be strictly increasing in sin_margin: f({s}) = {} <= {prev}",
                bound.value
            );
            prev = bound.value;
        }
        const TINY_MARGIN: f64 = 1.0e-4; // H-3: a tiny dimensionless sine margin
        const TINY_BOUND: f64 = 1.0e-3; // H-3: the slope bound must vanish below this dimensionless threshold
        let tiny = must(wedge_slope_lower_from_sin_margin(TINY_MARGIN));
        assert!(
            tiny.value < TINY_BOUND,
            "bound {} not small for sin_margin = {}",
            tiny.value,
            TINY_MARGIN
        );
        assert!(
            matches!(
                wedge_slope_lower_from_sin_margin(0.0),
                Err(FidRefusal::InvalidMargin)
            ),
            "s = 0 must refuse as InvalidMargin"
        );
        assert!(
            matches!(
                wedge_slope_lower_from_sin_margin(1.0 + TINY_MARGIN),
                Err(FidRefusal::InvalidMargin)
            ),
            "s > 1 must refuse as InvalidMargin"
        );
        // An antiparallel normal pair: sin phi = 0 exactly, and feeding the
        // measured value propagates the refusal that INV-109 would raise.
        let plane_a = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let plane_b = Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, -1.0, 0.0),
        );
        let sin_phi = plane_a.normal().cross(plane_b.normal()).magnitude();
        assert!(
            sin_phi <= TINY_BOUND,
            "antiparallel normals must measure sin phi ~ 0, got {sin_phi}"
        );
        assert!(
            matches!(
                wedge_slope_lower_from_sin_margin(sin_phi),
                Err(FidRefusal::InvalidMargin)
            ),
            "an antiparallel wedge's measured sin phi must refuse as InvalidMargin"
        );
    }

    #[test]
    fn wedge_slope_lower_bound_is_conservative_at_small_margins() {
        // The 3-term series branch (s below the cancellation threshold) must
        // return a certified lower bound on sin(asin(s)/2): never above an
        // independent upward-slack f64 reference, never collapsed to zero. The
        // buggy closed form over-reports at the larger witness and collapses
        // at smaller margins.
        const SMALL_MARGINS: [f64; 2] = [1.0e-6, 1.0e-8]; // H-3: dimensionless sine margins in the cancellation regime
        for &s in SMALL_MARGINS.iter() {
            let bound = must(wedge_slope_lower_from_sin_margin(s));
            let ref_hi = (0.5 * s + s * s * s / 16.0).next_up();
            assert!(
                bound.value <= ref_hi,
                "s={s}: bound {} exceeds the true sup {ref_hi}",
                bound.value
            );
            assert!(bound.value > 0.0, "s={s}: bound collapsed to zero");
        }
    }

    #[test]
    fn sphere_curvature_term_soundness() {
        // Sphere r = 2: the true radius of curvature is 2.0, so the certified
        // lower bound must be <= 2.0 (soundness direction!) on cells around
        // (u, v) = (1.1, 0.7). The scratch reference (a looser pre-iota
        // normalization) reported radii ~0.0977/~0.3575/~0.5768 at widths
        // {0.125, 0.0625, 0.03125}; the Decision-2 iota route (as implemented
        // here) measures radii ~0.407/~0.615/~0.741 on the same cells —
        // tighter, still strictly increasing, and still well under 2.0.
        let sphere = Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0);
        const CENTER_U: f64 = 1.1; // H-3: polar cell-center parameter, dimensionless (a parameter, not a length)
        const CENTER_V: f64 = 0.7; // H-3: azimuth cell-center parameter, dimensionless
        const WIDE: f64 = 0.125; // H-3: cell width in parameters, dimensionless
        const MID: f64 = 0.0625; // H-3: cell width in parameters, dimensionless
        const FINE: f64 = 0.03125; // H-3: cell width in parameters, dimensionless
        const TRUE_RADIUS: f64 = 2.0; // H-3: the sphere's true radius of curvature in model units
        const CURV_SLACK: f64 = 1.0e-9; // H-3: float slack between a curvature radius and the true radius, dimensionless
        let cell = |w: f64| {
            (
                iv(CENTER_U - w / 2.0, CENTER_U + w / 2.0),
                iv(CENTER_V - w / 2.0, CENTER_V + w / 2.0),
            )
        };
        let r_wide = must(curvature_radius_lower(&sphere, cell(WIDE)));
        let r_mid = must(curvature_radius_lower(&sphere, cell(MID)));
        let r_fine = must(curvature_radius_lower(&sphere, cell(FINE)));
        assert!(
            r_wide <= TRUE_RADIUS + CURV_SLACK,
            "radius {r_wide} exceeds the true curvature radius"
        );
        assert!(
            r_mid <= TRUE_RADIUS + CURV_SLACK,
            "radius {r_mid} exceeds the true curvature radius"
        );
        assert!(
            r_fine <= TRUE_RADIUS + CURV_SLACK,
            "radius {r_fine} exceeds the true curvature radius"
        );
        assert!(
            r_wide < r_mid && r_mid < r_fine,
            "radius must strictly increase under refinement, got {r_wide} < {r_mid} < {r_fine}"
        );
        // A pole-straddling cell (u in [-0.1, 0.1]) drives the immersion
        // margin to zero: it must refuse, never certify.
        let pole = curvature_radius_lower(&sphere, (iv(-0.1, 0.1), iv(0.5, 0.9)));
        assert!(
            matches!(pole, Err(FidRefusal::ImmersionUnresolved))
                || matches!(pole, Err(FidRefusal::MetricLowerBoundUnresolved)),
            "a pole-straddling cell must refuse, got {pole:?}"
        );
    }

    #[test]
    fn wedge_formula_matches_geometry() {
        // Plane pairs with known normal angles phi in {30, 90, 150} degrees:
        // compute sin phi from the planes' ACTUAL normals, feed the measured
        // value, assert (a) the result equals the closed form to final bits
        // and (b) the geometric claim numerically: dist(0, segment[n_A, n_B])
        // = cos(phi/2), computed directly, satisfies result <= cos(phi/2) +
        // slack.
        const PHIS_DEG: [f64; 3] = [30.0, 90.0, 150.0]; // H-3: wedge normal angles in degrees, dimensionless
        const FINAL_BITS: f64 = 1.0e-15; // H-3: float slack at the final-bit scale, dimensionless
        const GEOM_SLACK: f64 = 1.0e-9; // H-3: float slack for the geometric distance claim, dimensionless
        for &deg in PHIS_DEG.iter() {
            let phi = deg.to_radians();
            let plane_a = Plane::new(
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            );
            // Plane B's normal is (sin phi, 0, cos phi), an angle phi from A's
            // normal (0, 0, 1).
            let plane_b = Plane::new(
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(phi.cos(), 0.0, -phi.sin()),
                Point3::new(0.0, 1.0, 0.0),
            );
            let n_a = plane_a.normal();
            let n_b = plane_b.normal();
            let sin_phi = n_a.cross(n_b).magnitude();
            let bound = must(wedge_slope_lower_from_sin_margin(sin_phi));
            let expected = ((1.0 - (1.0 - sin_phi * sin_phi).sqrt()) / 2.0).sqrt();
            assert!(
                (bound.value - expected).abs() <= FINAL_BITS,
                "closed-form mismatch at {deg} deg: {} vs {expected}",
                bound.value
            );
            // dist(0, segment[n_A, n_B]) = |n_A + n_B| / 2 = cos(phi/2).
            let seg_distance = (n_a + n_b).magnitude() / 2.0;
            assert!(
                bound.value <= seg_distance + GEOM_SLACK,
                "geometric claim broken at {deg} deg: bound {} > cos(phi/2) = {seg_distance}",
                bound.value
            );
        }
    }
}
