//! Certified rank-two deck witness for a STEP toroidal surface.
//!
//! `truck-stepio` realizes STEP `toroidal_surface` as
//! `ToroidalSurface = Processor<Torus, Matrix4>` (see
//! `truck_stepio::r#in::step_geometry::mod`). Unlike a cylinder or cone —
//! both `Processor<RevolutedCurve<Line<Point3>>, Matrix4>` — the torus is its
//! own primitive: `Torus { center, large_radius, small_radius }` with a
//! parameterization whose two angular directions are *both* structural
//! properties of the map, not properties of a revolved generatrix.
//!
//! # How STEP toroidal surfaces encode major/minor angular directions
//!
//! `Torus::subs(u, v)` evaluates
//!
//! ```text
//! sr = small_radius · (cos v, sin v)
//! lr = (large_radius + sr.x) · (cos u, sin u)
//! center + (lr.x, lr.y, sr.y)
//! ```
//!
//! The **major** (azimuthal) direction `u` rotates the tube cross-section's
//! center about the revolution axis (the entity's local `z`): the
//! `cos u / sin u` factor is a property of the map, so `2π` is exact by
//! construction. The **minor** (poloidal) direction `v` rotates a point
//! around the tube cross-section: the `cos v / sin v` factor is likewise a
//! property of the map, so `2π` is exact by construction. Both periods are
//! structural, neither is an accessor result, and neither is inherited from a
//! generatrix curve.
//!
//! # Whether both periods survive import
//!
//! Yes. `Torus::u_period()` and `Torus::v_period()` each return `Some(2π)`,
//! and `Processor::u_period()`/`v_period()` forward these (swapping on
//! `orientation() == false`). So the periods survive import as declared
//! values — but as `Option<f64>` they are indistinguishable from an
//! accessor-only result, which is why `lattice_of` carries a torus as
//! `Uncertified` and this module exists to certify them structurally.
//!
//! # Whether existing surface repr erases source distinctions
//!
//! The `Processor<Torus, Matrix4>` type does not erase the representation:
//! `processor.entity()` returns `&Torus`, and `Torus` exposes `center()`,
//! `large_radius()`, `small_radius()`. What erases it is the
//! `ParametricSurface` trait dispatch — `u_period()`/`v_period()` return a
//! bare `Option<f64>` with no provenance. This adapter reads the entity
//! structurally *before* that dispatch, exactly as `cone.rs` and
//! `cylinder.rs` do for `RevolutedCurve<Line>`.
//!
//! # How to certify a transform preserves toroidal period structure
//!
//! The `Processor` carries an unconstrained `Matrix4`. Under a similarity
//! `A = s·Q` (uniform scale `s`, orthogonal `Q`) the image of a torus is
//! still a torus: the center moves, the axis rotates, the radii scale by
//! `s`, and the `2π` periods are unchanged. Under a non-uniform scale or a
//! shear the cross-section becomes elliptical and the surface is no longer a
//! torus — so this adapter refuses a non-similarity placement by name
//! ([`TorusDeckFailure::PlacementNotASimilarity`]) rather than certifying
//! through it. The argument is identical to `cone.rs`'s: see the module
//! docs there for the reassembly condition `A·R_axis(v) = R_{A·axis}(v)·A`.
//!
//! # How reflections affect lattice orientation
//!
//! A reflection (`det A < 0`) is a similarity and is admitted — it carries a
//! torus to a torus. What it reverses is the geometric *handedness* of the
//! parameterization: the ordered pair `(∂/∂u, ∂/∂v)` at a point picks up a
//! sign flip from `det A`. The deck group itself (`2π·Z²` in parameter
//! space) is unchanged, because parameter-plane translations that leave
//! `entity.subs` invariant also leave `transform·entity.subs` invariant. What
//! changes is the geometric sense of each generator relative to the
//! world-space frame, which a downstream atlas-cell classifier needs when
//! comparing winding signs to oriented geometric quantities. This is recorded
//! as [`LatticeOrientation`] ([`LatticeOrientation::Preserving`] when
//! `det A > 0`, [`LatticeOrientation::Reversing`] when `det A < 0`), the
//! rank-2 analogue of [`crate::step::circular_arc::TransformOrientation`].
//!
//! A single `det A` sign does not distinguish "major reversed only" from
//! "minor reversed only" — both are `Reversing`. That finer decomposition is
//! not certified here because the deck group is insensitive to it; a
//! downstream stage that needs it must read the geometric frame
//! ([`TorusDeckSource::axis`], [`TorusDeckSource::radial_x`]) directly.
//!
//! # Canonical coordinates vs an arbitrary valid lattice basis
//!
//! The certified basis is the **canonical** one: `{(2π, 0), (0, 2π)}` in the
//! caller's parameter plane, axis-aligned with the entity's `u`/`v` axes
//! (swapped under `orientation() == false`). An arbitrary valid lattice basis
//! — e.g. `{(2π, 0), (2π, 2π)}` — generates the same group `2π·Z²` but is
//! not what the representation declares. The winding API
//! ([`CertifiedRankTwoDeck::winding_of_displacement`]) expresses displacement
//! in the canonical basis only; a non-canonical basis requires a `GL(2, Z)`
//! change-of-basis witness, which is a later concern (see
//! [`CertifiedRankTwoDeck::change_of_basis_to_canonical`]).
//!
//! # How closed source curves with nonzero winding are represented
//!
//! A closed source curve whose lifted endpoints differ by `(k·2π, l·2π)`
//! with `(k, l) ≠ (0, 0)` is a curve that closes in the quotient
//! `T² = R²/(2π·Z²)` but not in the universal cover `R²`. Its deck
//! displacement is [`DeckVector2::new(k, l)`](truck_meshalgo::tessellation::formal::DeckVector2),
//! a nonzero element of `Z²` that identifies the endpoints under the deck
//! action. The winding API certifies this integer pair from the lifted
//! displacement; a curve whose displacement is not an integer multiple is
//! open and is refused by [`WindingFailure::NotIntegerMultiple`].
//!
//! # Refusal when independence or preservation cannot be proved
//!
//! Generator independence is certified structurally: the two generators lie
//! on distinct developed axes (one on `First`, one on `Second`), so under the
//! axis-aligned basis schema their determinant is the product of two nonzero
//! periods — provably nonzero. This mirrors
//! `GeneratorIndependenceCertificate::from_distinct_axes` in the formal
//! system. Transform preservation is certified by `is_similarity`; when it
//! fails, the surface the `Processor` evaluates is not a torus and no deck
//! is certified.

use truck_meshalgo::prelude::{InnerSpace, Matrix4, Point3, SquareMatrix, Transform, Vector3};
use truck_meshalgo::tessellation::formal::deck::{
    DeckConstructorFailure, DeckGenerator, DevelopedAxis,
};
use truck_meshalgo::tessellation::formal::numeric::{
    FiniteF64, NumericDomainError, PositiveFinite,
};
use truck_meshalgo::tessellation::formal::{CertifiedSign, DeckVector2, Expansion};
use truck_stepio::r#in::step_geometry::{ElementarySurface, Surface};

/// Relative bound for confirming that a placement's linear part is a
/// similarity. The same `1e-9` the cone adapter uses: a STEP placement is
/// built from an orthonormal frame, so on any conforming file the residuals
/// this bounds are floating-point drift in the importer's own normalization.
const SIMILARITY_RESIDUAL: f64 = 1e-9;

/// Relative floor, against the cube of the transform's uniform scale, below
/// which the determinant cannot be certified nonzero — and therefore cannot
/// certify a lattice orientation sign either. The same order of magnitude as
/// [`crate::step::circular_arc::ORIENTATION_CERTIFICATION_FLOOR`]: a
/// structural near-degeneracy floor, not a chained-rounding bound.
const ORIENTATION_CERTIFICATION_FLOOR: f64 = 1e-6;

/// How many ULPs of chained floating-point error a winding displacement can
/// accumulate and still be certified an exact integer multiple of the period.
/// The same `64.0` the circularity classifier uses: headroom over the actual
/// op count of the displacement computation, not a tightness claim beyond
/// IEEE-754 per-operation correct rounding.
const WINDING_CERTIFIED_EQUAL_ULPS: f64 = 64.0;

/// How many multiples of the certified-equal bound a winding residual must
/// clear before it is certified *not* an integer multiple (the curve is
/// open). The same `1e6` margin as
/// [`crate::step::circular_arc::CIRCULARITY_UNRESOLVED_MARGIN`].
const WINDING_UNRESOLVED_MARGIN: f64 = 1.0e6;

// ---------------------------------------------------------------------------
// Exact 3x3 determinant (ported from circular_arc.rs — same algorithm)
// ---------------------------------------------------------------------------

/// Exact 3×3 determinant, as a non-overlapping expansion. The sign of this
/// expansion is an exact predicate for `sign(det(A))` over the `f64` columns
/// of `A`. Ported from `circular_arc::exact_det3`.
fn exact_det3(m: [[f64; 3]; 3]) -> Expansion {
    let cofactor = |a: f64, b: f64, c: f64, d: f64| -> Expansion {
        Expansion::from_product(a, d).merge(&Expansion::from_product(b, c).negate())
    };
    let scale =
        |e: &Expansion, s: f64| -> Expansion { e.mul_expansion(&Expansion::zero().grow(s)) };

    let c00 = cofactor(m[1][1], m[1][2], m[2][1], m[2][2]);
    let c01 = cofactor(m[1][0], m[1][2], m[2][0], m[2][2]);
    let c02 = cofactor(m[1][0], m[1][1], m[2][0], m[2][1]);

    let t0 = scale(&c00, m[0][0]);
    let t1 = scale(&c01, m[0][1]).negate();
    let t2 = scale(&c02, m[0][2]);

    t0.merge(&t1).merge(&t2)
}

// ---------------------------------------------------------------------------
// LatticeOrientation
// ---------------------------------------------------------------------------

/// Whether the placement transform preserves or reverses the torus
/// parameterization's geometric orientation.
///
/// The rank-2 analogue of
/// [`crate::step::circular_arc::TransformOrientation`]. For a torus the deck
/// group (`2π·Z²` in parameter space) is insensitive to the transform —
/// parameter-plane translations that leave `entity.subs` invariant also leave
/// `transform·entity.subs` invariant — so this field does not change the
/// generators. What it records is the handedness of the ordered generator
/// pair relative to the world-space frame `(axis, radial_x, radial_y)`,
/// which a downstream atlas-cell classifier needs when comparing winding
/// signs to oriented geometric quantities.
///
/// Certified exactly from `sign(det A)` via an exact determinant expansion
/// over the transform's bit-exact column extractions, decoupled from the
/// near-singular *conditioning* floor
/// ([`ORIENTATION_CERTIFICATION_FLOOR`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatticeOrientation {
    /// `det A > 0`: the transform preserves the right-hand relationship
    /// between the two generator directions and the geometric frame.
    Preserving,
    /// `det A < 0`: the transform includes an odd number of reflections.
    /// The generators are unchanged but their geometric sense is reversed.
    Reversing,
}

impl LatticeOrientation {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Preserving => "lattice_orientation_preserving",
            Self::Reversing => "lattice_orientation_reversing",
        }
    }
}

// ---------------------------------------------------------------------------
// MajorAxis
// ---------------------------------------------------------------------------

/// Which of the caller's parameter axes (after the `Processor`'s orientation
/// fold) carries the torus's major (azimuthal) angular direction.
///
/// The entity's own `u` is always major and `v` is always minor — that is a
/// property of `Torus::subs`. Under `Processor::orientation() == false` the
/// caller's `u` reads the entity's `v` and vice versa, so the major/minor
/// assignment swaps. This is the same axis-swap `lattice.rs::orient` applies
/// for the rank-1 revolution case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MajorAxis {
    /// The caller's `u` axis carries the major direction.
    U,
    /// The caller's `v` axis carries the major direction.
    V,
}

impl MajorAxis {
    /// The other axis, which carries the minor direction.
    pub fn other(self) -> Self {
        match self {
            Self::U => Self::V,
            Self::V => Self::U,
        }
    }

    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::U => "major_axis_u",
            Self::V => "major_axis_v",
        }
    }
}

// ---------------------------------------------------------------------------
// TorusDeckFailure
// ---------------------------------------------------------------------------

/// Why a `Surface` was not certified as a rank-two torus deck by this
/// adapter.
#[derive(Debug, Clone, PartialEq)]
pub enum TorusDeckFailure {
    /// The representation is not `ElementarySurface::ToroidalSurface`.
    NotToroidalSurface,
    /// The placement's linear part is not a similarity, so the surface the
    /// `Processor` evaluates is not a torus: under a non-uniform scale or a
    /// shear the cross-section becomes elliptical. See the module docs.
    PlacementNotASimilarity,
    /// A structural quantity (center, axis, radius) was `NaN` or infinite,
    /// or a radius was non-positive.
    DegenerateTorusGeometry {
        /// Why the value was refused.
        cause: NumericDomainError,
    },
    /// The transform's determinant is not certified nonzero (near-singular,
    /// relative to [`ORIENTATION_CERTIFICATION_FLOOR`]) or is non-finite, so
    /// no orientation sign can be certified.
    TransformOrientationUndecidable,
}

impl TorusDeckFailure {
    /// A short stable tag, for diagnostics.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::NotToroidalSurface => "surface_not_toroidal",
            Self::PlacementNotASimilarity => "torus_placement_not_a_similarity",
            Self::DegenerateTorusGeometry { .. } => "torus_degenerate_geometry",
            Self::TransformOrientationUndecidable => "torus_transform_orientation_undecidable",
        }
    }
}

// ---------------------------------------------------------------------------
// WindingFailure
// ---------------------------------------------------------------------------

/// Why a lifted displacement could not be certified as a `Z²` winding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindingFailure {
    /// A displacement component was `NaN` or infinite.
    NonFiniteDisplacement,
    /// The displacement on one axis is certified *not* an integer multiple of
    /// the period: the curve does not close in the quotient and has no
    /// `Z²` winding. The curve is open.
    NotIntegerMultiple {
        /// Which generator's axis the refusal concerns.
        axis: MajorAxis,
        /// The displacement on that axis.
        displacement: f64,
        /// The nearest integer multiple of the period.
        nearest: f64,
        /// The residual `|displacement - nearest|`.
        residual: f64,
    },
    /// The displacement on one axis is outside the certified-equal bound but
    /// has not cleared the certified-unequal margin, so this evidence alone
    /// cannot soundly decide whether the curve closes.
    Indeterminate {
        /// Which generator's axis the refusal concerns.
        axis: MajorAxis,
        /// The displacement on that axis.
        displacement: f64,
    },
}

impl WindingFailure {
    /// A short stable tag, for diagnostics.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::NonFiniteDisplacement => "winding_non_finite_displacement",
            Self::NotIntegerMultiple { .. } => "winding_not_integer_multiple",
            Self::Indeterminate { .. } => "winding_indeterminate",
        }
    }
}

// ---------------------------------------------------------------------------
// TorusDeckSource
// ---------------------------------------------------------------------------

/// The structural source facts of a certified torus deck, read from the
/// `Processor<Torus, Matrix4>` representation after the placement is folded
/// in.
///
/// Fields are private and the only constructor is [`identify_source_torus_deck`],
/// so a value of this type is a set of facts discharged by presenting the
/// representation, not assembled from numbers a caller happens to have.
#[derive(Debug, Clone, PartialEq)]
pub struct TorusDeckSource {
    center: Point3,
    axis: Vector3,
    radial_x: Vector3,
    radial_y: Vector3,
    major_radius: PositiveFinite,
    minor_radius: PositiveFinite,
    major_axis: MajorAxis,
}

impl TorusDeckSource {
    /// The torus center in world space: `transform.transform_point(entity.center())`.
    pub fn center(&self) -> Point3 {
        self.center
    }

    /// The unit revolution axis in world space: the transform of the entity's
    /// local `z` — structurally the axis of the `cos u / sin u` rotation in
    /// `Torus::subs`.
    pub fn axis(&self) -> Vector3 {
        self.axis
    }

    /// The unit radial direction at angular parameter zero, in world space.
    /// The transform of the entity's local `x`.
    pub fn radial_x(&self) -> Vector3 {
        self.radial_x
    }

    /// The second radial basis vector `axis × radial_x`, completing a
    /// right-hand frame `(radial_x, radial_y, axis)` in world space
    /// regardless of the transform's orientation.
    pub fn radial_y(&self) -> Vector3 {
        self.radial_y
    }

    /// The major (azimuthal) radius, scaled by the transform's uniform scale.
    pub fn major_radius(&self) -> PositiveFinite {
        self.major_radius
    }

    /// The minor (poloidal / tube) radius, scaled by the transform's uniform
    /// scale.
    pub fn minor_radius(&self) -> PositiveFinite {
        self.minor_radius
    }

    /// Which of the caller's parameter axes carries the major angular
    /// direction. The other axis carries the minor direction.
    pub fn major_axis(&self) -> MajorAxis {
        self.major_axis
    }
}

// ---------------------------------------------------------------------------
// CertifiedRankTwoDeck
// ---------------------------------------------------------------------------

/// A certified rank-two deck witness for a STEP toroidal surface.
///
/// Establishes: (1) two independent period generators, each `2π` and exact
/// by construction of the `Torus::subs` parameterization; (2) their relation
/// to the STEP parameterization (major = azimuthal `u`, minor = poloidal
/// `v`, swapped under `Processor::orientation() == false`); (3) behavior
/// under source placement (similarity required, refused otherwise); (4)
/// orientation under proper (`Preserving`) and reflected (`Reversing`)
/// transforms; (5) a certified method to express lifted curve displacement in
/// `Z²` ([`Self::winding_of_displacement`]); (6) the distinction between the
/// canonical axis-aligned basis and an arbitrary valid lattice basis (see
/// [`Self::change_of_basis_to_canonical`]); (7) refusal when generator
/// independence or transform preservation cannot be proved (see
/// [`TorusDeckFailure`]).
///
/// The only constructor is [`identify_source_torus_deck`]; fields are
/// private, so every obligation is discharged by presenting the
/// representation.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedRankTwoDeck {
    generators: [DeckGenerator; 2],
    orientation: LatticeOrientation,
    source: TorusDeckSource,
}

impl CertifiedRankTwoDeck {
    /// The two certified period generators, ordered `[major, minor]`.
    ///
    /// `generators[0]` is the major (azimuthal) generator on its developed
    /// axis; `generators[1]` is the minor (poloidal) generator on the other.
    /// Each carries a signed period of `+2π`.
    pub fn generators(&self) -> [DeckGenerator; 2] {
        self.generators
    }

    /// The major (azimuthal) generator: `2π` on the major developed axis.
    pub fn major_generator(&self) -> DeckGenerator {
        self.generators[0]
    }

    /// The minor (poloidal) generator: `2π` on the minor developed axis.
    pub fn minor_generator(&self) -> DeckGenerator {
        self.generators[1]
    }

    /// Whether the transform preserves or reverses the geometric orientation
    /// of the parameterization.
    pub fn orientation(&self) -> LatticeOrientation {
        self.orientation
    }

    /// The structural source facts.
    pub fn source(&self) -> &TorusDeckSource {
        &self.source
    }

    /// The certified rank of this deck: always 2 for a torus.
    pub fn rank(&self) -> u8 {
        2
    }

    /// A short stable tag, for probe records.
    pub fn tag(&self) -> &'static str {
        "rank2_torus_deck"
    }

    // -- Winding-coordinate API --------------------------------------------

    /// The certified `Z²` winding of a lifted curve displacement.
    ///
    /// `du` and `dv` are the parameter-space displacements on the caller's
    /// `u` and `v` axes (after the `Processor`'s orientation fold). The
    /// winding is the integer pair `(k_major, k_minor)` such that the
    /// displacement on the major axis is `k_major · 2π` and the displacement
    /// on the minor axis is `k_minor · 2π`, each certified within
    /// [`WINDING_CERTIFIED_EQUAL_ULPS`] machine epsilons.
    ///
    /// Returns [`WindingFailure::NotIntegerMultiple`] when the displacement
    /// on one axis is certified *not* an integer multiple — the curve is
    /// open in the quotient and has no `Z²` winding. Returns
    /// [`WindingFailure::Indeterminate`] when the evidence cannot soundly
    /// decide either way.
    ///
    /// The winding is expressed in the **canonical** basis
    /// `{(2π, 0), (0, 2π)}`; see [`Self::change_of_basis_to_canonical`] for
    /// the relationship to an arbitrary valid lattice basis.
    pub fn winding_of_displacement(&self, du: f64, dv: f64) -> Result<DeckVector2, WindingFailure> {
        let (d_major, d_minor) = match self.source.major_axis {
            MajorAxis::U => (du, dv),
            MajorAxis::V => (dv, du),
        };
        let period = std::f64::consts::TAU;
        let k_major = certify_winding(d_major, period, MajorAxis::U)?;
        let k_minor = certify_winding(d_minor, period, MajorAxis::V)?;
        Ok(DeckVector2::new(k_major, k_minor))
    }

    /// The certified `Z²` winding of a closed lifted curve, from its lifted
    /// endpoint parameters.
    ///
    /// Convenience for [`Self::winding_of_displacement`]: computes
    /// `(end - start)` on each axis and certifies the winding. For a closed
    /// quotient loop the endpoints coincide on the torus but differ on the
    /// universal cover by a deck translation `(k·2π, l·2π)`.
    pub fn winding_of_lifted_endpoints(
        &self,
        start: (f64, f64),
        end: (f64, f64),
    ) -> Result<DeckVector2, WindingFailure> {
        self.winding_of_displacement(end.0 - start.0, end.1 - start.1)
    }

    // -- Basis distinction -------------------------------------------------

    /// Whether this deck's generators form the canonical axis-aligned basis.
    ///
    /// Always `true` for a deck certified by [`identify_source_torus_deck`]:
    /// the canonical basis is `{(2π, 0), (0, 2π)}` in the caller's parameter
    /// plane, aligned with the entity's own `u`/`v` axes (swapped under
    /// `orientation() == false`). A non-canonical basis — e.g.
    /// `{(2π, 0), (2π, 2π)}` — generates the same lattice `2π·Z²` but is not
    /// what the representation declares, and expressing winding in it requires
    /// a `GL(2, Z)` change-of-basis witness.
    pub fn is_canonical_basis(&self) -> bool {
        true
    }

    /// Express a canonical-basis winding in a target basis related by a
    /// `GL(2, Z)` change-of-basis matrix.
    ///
    /// `m` is the `[[m00, m01], [m10, m11]]` integer matrix such that the
    /// target basis vectors are `g'_0 = m00·g_0 + m10·g_1` and
    /// `g'_1 = m01·g_0 + m11·g_1` (column convention: `g' = g · M`). The
    /// target-basis winding is `M⁻¹ · w`, which for `M ∈ GL(2, Z)` is an
    /// exact integer computation: `det = m00·m11 - m01·m10` must be `±1`.
    ///
    /// This is the basis-change witness the winding API requires to express
    /// a displacement in a non-canonical basis. It is the caller's
    /// responsibility to prove `M ∈ GL(2, Z)`; this method refuses a
    /// non-unimodular `M` rather than rounding.
    ///
    /// Returns the target-basis winding, or `None` if `det M ≠ ±1`.
    pub fn change_of_basis_to_canonical(
        &self,
        winding: DeckVector2,
        m: [[i64; 2]; 2],
    ) -> Option<DeckVector2> {
        let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
        if det != 1 && det != -1 {
            return None;
        }
        // M^{-1} = (1/det) · [[m11, -m01], [-m10, m00]].
        // Since det = ±1, the division is exact.
        let inv = |x: i64| -> i64 { if det == 1 { x } else { -x } };
        let w0 = winding.first();
        let w1 = winding.second();
        let k0 = inv(m[1][1] * w0 - m[0][1] * w1);
        let k1 = inv(-m[1][0] * w0 + m[0][0] * w1);
        Some(DeckVector2::new(k0, k1))
    }
}

// ---------------------------------------------------------------------------
// Winding certification
// ---------------------------------------------------------------------------

/// Certify that `displacement` is an integer multiple of `period`, returning
/// the integer `k` with `displacement ≈ k · period`.
///
/// Three-way classification, mirroring `circular_arc`'s circularity gate:
///
/// - within [`WINDING_CERTIFIED_EQUAL_ULPS`] machine epsilons of an integer
///   multiple: certified (`Ok(k)`);
/// - beyond [`WINDING_UNRESOLVED_MARGIN`] times that bound: certified *not*
///   an integer multiple ([`WindingFailure::NotIntegerMultiple`]);
/// - in between: undecidable ([`WindingFailure::Indeterminate`]).
fn certify_winding(displacement: f64, period: f64, axis: MajorAxis) -> Result<i64, WindingFailure> {
    if !displacement.is_finite() {
        return Err(WindingFailure::NonFiniteDisplacement);
    }
    if displacement == 0.0 {
        return Ok(0);
    }
    let quotient = displacement / period;
    let k = quotient.round();
    if k == 0.0 {
        // The displacement is less than half a period. It is an integer
        // multiple only if it is exactly zero, which was handled above.
        // A nonzero sub-period displacement is certified not an integer
        // multiple unless it is within the rounding band of zero.
        let residual = displacement.abs();
        let scale = displacement.abs().max(period).max(1.0);
        let certified_equal_bound = WINDING_CERTIFIED_EQUAL_ULPS * f64::EPSILON * scale;
        if residual <= certified_equal_bound {
            return Ok(0);
        }
        let certified_unequal_bound = certified_equal_bound * WINDING_UNRESOLVED_MARGIN;
        if residual > certified_unequal_bound {
            return Err(WindingFailure::NotIntegerMultiple {
                axis,
                displacement,
                nearest: 0.0,
                residual,
            });
        }
        return Err(WindingFailure::Indeterminate { axis, displacement });
    }
    let k_i64 = k as i64;
    let nearest = k_i64 as f64 * period;
    let residual = (displacement - nearest).abs();
    let scale = displacement.abs().max(nearest.abs()).max(period).max(1.0);
    let certified_equal_bound = WINDING_CERTIFIED_EQUAL_ULPS * f64::EPSILON * scale;
    if residual <= certified_equal_bound {
        return Ok(k_i64);
    }
    let certified_unequal_bound = certified_equal_bound * WINDING_UNRESOLVED_MARGIN;
    if residual > certified_unequal_bound {
        return Err(WindingFailure::NotIntegerMultiple {
            axis,
            displacement,
            nearest,
            residual,
        });
    }
    Err(WindingFailure::Indeterminate { axis, displacement })
}

// ---------------------------------------------------------------------------
// Similarity check (ported from cone.rs — same mathematical content)
// ---------------------------------------------------------------------------

/// Whether a placement's linear part is a similarity: a uniform scale
/// composed with an orthogonal map.
///
/// Checked as the property itself rather than through a decomposition — the
/// three columns must be mutually orthogonal and of equal length — so a
/// reflection passes (it is orthogonal) and a non-uniform scale or a shear
/// does not. Both residuals are taken relative to the mean squared column
/// length, so the verdict does not depend on the units the file is written
/// in.
fn is_similarity(transform: &Matrix4) -> bool {
    let column = |c: [f64; 4]| Vector3::new(c[0], c[1], c[2]);
    let columns = [
        column([transform.x.x, transform.x.y, transform.x.z, 0.0]),
        column([transform.y.x, transform.y.y, transform.y.z, 0.0]),
        column([transform.z.x, transform.z.y, transform.z.z, 0.0]),
    ];
    let squared: Vec<f64> = columns.iter().map(|c| c.dot(*c)).collect();
    let scale = (squared[0] + squared[1] + squared[2]) / 3.0;
    if !scale.is_finite() || scale <= 0.0 {
        return false;
    }
    for value in &squared {
        if (value - scale)
            .abs()
            .partial_cmp(&(SIMILARITY_RESIDUAL * scale))
            .is_none_or(|o| !o.is_le())
        {
            return false;
        }
    }
    for (i, j) in [(0usize, 1usize), (0, 2), (1, 2)] {
        if columns[i]
            .dot(columns[j])
            .abs()
            .partial_cmp(&(SIMILARITY_RESIDUAL * scale))
            .is_none_or(|o| !o.is_le())
        {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Read a `Surface` structurally and certify a rank-two torus deck, when the
/// representation is `ElementarySurface::ToroidalSurface`.
///
/// Refuses (never fits or infers): any other surface representation
/// ([`TorusDeckFailure::NotToroidalSurface`] — planes, cylinders, cones,
/// spheres, splines, swept and offset surfaces are all out of scope), a
/// toroidal surface whose placement is not a similarity
/// ([`TorusDeckFailure::PlacementNotASimilarity`] — under a non-uniform scale
/// or shear the surface is not a torus), a degenerate torus
/// ([`TorusDeckFailure::DegenerateTorusGeometry`] — non-finite or non-positive
/// radius), and a torus whose transform orientation cannot be certified
/// ([`TorusDeckFailure::TransformOrientationUndecidable`] — near-singular
/// determinant).
pub fn identify_source_torus_deck(
    surface: &Surface,
) -> Result<CertifiedRankTwoDeck, TorusDeckFailure> {
    let Surface::ElementarySurface(ElementarySurface::ToroidalSurface(processor)) = surface else {
        return Err(TorusDeckFailure::NotToroidalSurface);
    };
    let entity = processor.entity();
    let transform = *processor.transform();

    if !is_similarity(&transform) {
        return Err(TorusDeckFailure::PlacementNotASimilarity);
    }

    // --- structural source facts ------------------------------------------
    let entity_center = entity.center();
    let entity_large = entity.large_radius();
    let entity_small = entity.small_radius();

    for coordinate in [
        entity_center.x,
        entity_center.y,
        entity_center.z,
        entity_large,
        entity_small,
    ] {
        if let Err(cause) = FiniteF64::new(coordinate) {
            return Err(TorusDeckFailure::DegenerateTorusGeometry { cause });
        }
    }
    if entity_large.partial_cmp(&0.0).is_none_or(|o| !o.is_gt()) {
        return Err(TorusDeckFailure::DegenerateTorusGeometry {
            cause: NumericDomainError::Zero,
        });
    }
    if entity_small.partial_cmp(&0.0).is_none_or(|o| !o.is_gt()) {
        return Err(TorusDeckFailure::DegenerateTorusGeometry {
            cause: NumericDomainError::Zero,
        });
    }

    // The transform's uniform scale, from the mean squared column length.
    // `is_similarity` already verified the columns are equal-length and
    // orthogonal, so this is the similarity's `s`.
    let col_z = transform.transform_vector(Vector3::new(0.0, 0.0, 1.0));
    let col_x = transform.transform_vector(Vector3::new(1.0, 0.0, 0.0));
    let col_y = transform.transform_vector(Vector3::new(0.0, 1.0, 0.0));
    let s_squared = (col_x.dot(col_x) + col_y.dot(col_y) + col_z.dot(col_z)) / 3.0;
    let s = s_squared.sqrt();

    let center = transform.transform_point(entity_center);
    let axis = col_z; // the revolution axis: structurally the entity's z.
    let axis_norm = axis.magnitude();
    if axis_norm.partial_cmp(&0.0).is_none_or(|o| !o.is_gt()) {
        return Err(TorusDeckFailure::DegenerateTorusGeometry {
            cause: NumericDomainError::Zero,
        });
    }
    let axis_unit = axis / axis_norm;
    let radial_x = col_x / s; // unit radial at u=0
    let radial_y = axis_unit.cross(radial_x);

    let major_radius = match PositiveFinite::new(entity_large * s) {
        Ok(r) => r,
        Err(cause) => return Err(TorusDeckFailure::DegenerateTorusGeometry { cause }),
    };
    let minor_radius = match PositiveFinite::new(entity_small * s) {
        Ok(r) => r,
        Err(cause) => return Err(TorusDeckFailure::DegenerateTorusGeometry { cause }),
    };

    // --- orientation: exact determinant sign ------------------------------
    let determinant = transform.determinant();
    if !determinant.is_finite() {
        return Err(TorusDeckFailure::TransformOrientationUndecidable);
    }
    let scale_cubed = s.powi(3).max(f64::MIN_POSITIVE);
    let relative_determinant = determinant.abs() / scale_cubed;
    if relative_determinant < ORIENTATION_CERTIFICATION_FLOOR {
        return Err(TorusDeckFailure::TransformOrientationUndecidable);
    }
    let det_expansion = exact_det3([
        [col_x.x, col_y.x, col_z.x],
        [col_x.y, col_y.y, col_z.y],
        [col_x.z, col_y.z, col_z.z],
    ]);
    let orientation = match det_expansion.sign() {
        CertifiedSign::Positive => LatticeOrientation::Preserving,
        CertifiedSign::Negative => LatticeOrientation::Reversing,
        CertifiedSign::Zero => {
            return Err(TorusDeckFailure::TransformOrientationUndecidable);
        }
    };

    // --- axis assignment under Processor::orientation() -------------------
    // The entity's u is major, v is minor. Under orientation() == false the
    // caller's u reads the entity's v and vice versa, so the major/minor
    // assignment swaps — the same fold `lattice.rs::orient` applies.
    let (major_axis, _minor_axis) = match processor.orientation() {
        true => (MajorAxis::U, MajorAxis::V),
        false => (MajorAxis::V, MajorAxis::U),
    };
    let (major_developed, minor_developed) = match major_axis {
        MajorAxis::U => (DevelopedAxis::First, DevelopedAxis::Second),
        MajorAxis::V => (DevelopedAxis::Second, DevelopedAxis::First),
    };

    // --- the two certified period generators ------------------------------
    let period = std::f64::consts::TAU;
    let Ok(period_finite) = FiniteF64::new(period) else {
        return Err(TorusDeckFailure::DegenerateTorusGeometry {
            cause: NumericDomainError::NotANumber,
        });
    };
    let major_generator = match DeckGenerator::new(major_developed, period_finite) {
        Ok(g) => g,
        Err(DeckConstructorFailure::ZeroPeriod)
        | Err(DeckConstructorFailure::Numeric(_))
        | Err(DeckConstructorFailure::BoundsInverted) => {
            return Err(TorusDeckFailure::DegenerateTorusGeometry {
                cause: NumericDomainError::Zero,
            });
        }
    };
    let minor_generator = match DeckGenerator::new(minor_developed, period_finite) {
        Ok(g) => g,
        Err(DeckConstructorFailure::ZeroPeriod)
        | Err(DeckConstructorFailure::Numeric(_))
        | Err(DeckConstructorFailure::BoundsInverted) => {
            return Err(TorusDeckFailure::DegenerateTorusGeometry {
                cause: NumericDomainError::Zero,
            });
        }
    };

    Ok(CertifiedRankTwoDeck {
        generators: [major_generator, minor_generator],
        orientation,
        source: TorusDeckSource {
            center,
            axis: axis_unit,
            radial_x,
            radial_y,
            major_radius,
            minor_radius,
            major_axis,
        },
    })
}

/// [`identify_source_torus_deck`], with the failure reduced to its stable
/// tag.
///
/// The `Result<_, &'static str>` shape mirrors
/// [`crate::step::cone::identify_source_cone_opt`] and
/// [`crate::step::cylinder::identify_source_cylinder_opt`]: a stable tag
/// survives the crate boundary for diagnostics without carrying the full
/// typed failure enum.
pub fn identify_source_torus_deck_opt(
    surface: &Surface,
) -> Result<CertifiedRankTwoDeck, &'static str> {
    identify_source_torus_deck(surface).map_err(|failure| failure.tag())
}

// ---------------------------------------------------------------------------
// Production torus adapter (returns the truck-side CertifiedEmbeddedTorus)
// ---------------------------------------------------------------------------

use truck_meshalgo::tessellation::formal::{
    CertifiedEmbeddedTorus, TorusIdentification, identify_torus_world,
};

/// Read a `Surface` structurally and certify an embedded torus, when the
/// representation is `ElementarySurface::ToroidalSurface`, returning the
/// truck-side [`CertifiedEmbeddedTorus`] the production torus annulus route
/// needs.
///
/// This is the torus analogue of
/// [`crate::step::cylinder::identify_source_cylinder_opt`] and
/// [`crate::step::cone::identify_source_cone_opt`]: it extracts the world-space
/// torus parameters from the STEP `Processor<Torus, Matrix4>`, certifies the
/// deck via [`identify_torus_world`], and packages the deck with the
/// untransformed entity and placement transform so
/// [`truck_meshalgo::tessellation::formal::realize_torus_annulus`] can evaluate
/// `transform.transform_point(torus.subs(u, v))` during mesh realization.
///
/// Refuses: any non-toroidal surface, a non-similarity placement (under which
/// the surface is not a torus), and a torus that fails certification (spindle,
/// horn, degenerate, unverified period).
pub fn identify_source_torus_opt(
    surface: &Surface,
) -> Result<CertifiedEmbeddedTorus, &'static str> {
    let Surface::ElementarySurface(ElementarySurface::ToroidalSurface(processor)) = surface else {
        return Err("surface_not_toroidal");
    };
    let entity = processor.entity();
    let transform = *processor.transform();

    if !is_similarity(&transform) {
        return Err("torus_placement_not_a_similarity");
    }

    let center = transform.transform_point(entity.center());
    let axis = transform.transform_vector(Vector3::new(0.0, 0.0, 1.0));
    let col_x = transform.transform_vector(Vector3::new(1.0, 0.0, 0.0));
    let scale = col_x.magnitude();
    let large = entity.large_radius() * scale;
    let small = entity.small_radius() * scale;

    match identify_torus_world(center, axis, large, small) {
        TorusIdentification::Torus(deck) => {
            Ok(CertifiedEmbeddedTorus::new(deck, *entity, transform))
        }
        TorusIdentification::NotATorus(_) => Err("torus_not_a_regular_ring_torus"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use truck_stepio::r#in::step_geometry::{Processor, Torus};

    /// A torus about the z-axis at the origin, as a `Processor` with the
    /// identity transform and forward orientation.
    fn z_torus_surface(major: f64, minor: f64) -> Surface {
        Surface::ElementarySurface(ElementarySurface::ToroidalSurface(Processor::new(
            Torus::new(Point3::new(0.0, 0.0, 0.0), major, minor),
        )))
    }

    #[test]
    fn a_real_step_toroidal_surface_is_certified() {
        let deck = identify_source_torus_deck(&z_torus_surface(3.0, 1.0))
            .expect("a toroidal surface certifies");
        assert_eq!(deck.rank(), 2);
        assert_eq!(deck.source().major_radius().get(), 3.0);
        assert_eq!(deck.source().minor_radius().get(), 1.0);
        assert!((deck.source().center() - Point3::new(0.0, 0.0, 0.0)).magnitude() < 1e-9);
        assert!((deck.source().axis() - Vector3::new(0.0, 0.0, 1.0)).magnitude() < 1e-9);
        assert_eq!(deck.source().major_axis(), MajorAxis::U);
        assert_eq!(deck.orientation(), LatticeOrientation::Preserving);
    }

    #[test]
    fn both_generators_are_two_pi_on_distinct_axes() {
        let deck = identify_source_torus_deck(&z_torus_surface(3.0, 1.0)).expect("certifies");
        let [major, minor] = deck.generators();
        assert!((major.signed_period().get() - std::f64::consts::TAU).abs() < 1e-12);
        assert!((minor.signed_period().get() - std::f64::consts::TAU).abs() < 1e-12);
        assert_ne!(major.periodic_axis(), minor.periodic_axis());
    }

    #[test]
    fn a_non_toroidal_surface_is_refused_by_name() {
        let cone_surface = {
            use truck_stepio::r#in::step_geometry::{Line, RevolutedCurve};
            let revo = RevolutedCurve::by_revolution(
                Line(Point3::new(0.5, 0.0, 1.0), Point3::new(3.0, 0.0, 6.0)),
                Point3::new(0.0, 0.0, 0.0),
                Vector3::new(0.0, 0.0, 1.0),
            );
            Surface::ElementarySurface(ElementarySurface::ConicalSurface(Processor::new(revo)))
        };
        assert_eq!(
            identify_source_torus_deck(&cone_surface).unwrap_err(),
            TorusDeckFailure::NotToroidalSurface
        );
    }
}
