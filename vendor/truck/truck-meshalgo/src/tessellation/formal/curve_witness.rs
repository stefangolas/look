//! Checkpoint 5: curve-on-cylinder witnesses in the developed universal cover.
//!
//! # Scope
//!
//! Exactly the two curve families the supported subset names: an axial line
//! (constant angular coordinate) and a circumferential circular arc (constant
//! axial coordinate, sweeping the angle). Both project *exactly* into the
//! developed plane once the endpoints are known to lie on the certified
//! cylinder — there is no numerical approximation to bound, unlike the
//! planar slice's polygonal projection.
//!
//! # The developed coordinate convention
//!
//! [`super::cylinder`]'s module docs fix the developed plane as `(axial =
//! First, angular = Second)`, and [`super::cylinder::CylinderSchema::angular_coordinate`]
//! *is* the surface's own periodic parameter `v` — not a value this module
//! re-derives or wraps. A [`Point2`] produced here has `x` the axial
//! coordinate and `y` the developed (possibly non-principal) angular
//! coordinate.
//!
//! # Why the arc witness takes a declared sweep rather than inferring one
//!
//! Two points at the same radius and axial coordinate admit more than one arc
//! between them — short way, long way, or wrapped around one or more extra
//! turns — and nothing about the two endpoints alone disambiguates which the
//! source curve actually traces. The authoritative fact is the source curve's
//! own trimmed parameter interval, which for a circle *is* the signed angular
//! sweep, unwrapped. [`circumferential_arc_witness`] takes that sweep as an
//! input and treats the endpoints as the sample that confirms it, never as
//! the thing that certifies it: this is the same discipline
//! `identify_cylinder`'s `verify_angular_convention` already applies to the
//! `2π` period itself.
//!
//! # The closed-edge rule: a closed circular edge is one full traversal
//!
//! The source-semantic rule this module relies on, stated once because
//! [`complete_circle_witness`] is built on it and nothing else in the subtree
//! can derive it:
//!
//! > An `edge_curve` whose two ends are the *same* source vertex, carried on a
//! > circle, represents exactly one complete traversal of that circle — one
//! > full period, once, in the curve's own parameter direction.
//!
//! It is not a fact about coordinates and it is not recoverable from the
//! trimmed parameter interval. An importer recovers an edge's trim by solving
//! each of its two vertex points onto the curve; when the edge is closed those
//! two solves are the *same* solve, so the interval degenerates to `(u, u)`
//! and the extent the source did declare is gone — not because the source
//! declared a zero sweep, but because coincident endpoints cannot carry an
//! extent. The rule above is what supplies the missing extent, and it is the
//! reason a full turn may be certified here without an endpoint confirming it.
//!
//! Two things bound it, and both are enforced rather than assumed:
//!
//! - *Once*, not `k` times. The rule licenses exactly one period. A multiply
//!   wound occurrence would need its own authoritative turn count, which no
//!   closed `edge_curve` carries, and this module never invents one.
//! - *Closed by identity*, not by coincidence. Two distinct source vertices at
//!   one point are a zero-length edge, not a closed one, and
//!   [`identify_source_curve_witness`] refuses the complete-circle reading for
//!   them ([`WitnessFailure::OccurrenceNotClosed`]) rather than welding them.
//!
//! # Why this does not touch `cylinder.rs`
//!
//! [`super::cylinder::CylinderSchema`] already exposes everything a witness
//! needs — `axial_coordinate`, `angular_coordinate`, `axis`, `radius` — through
//! its public accessors. The on-cylinder check the witnesses need is built
//! from those accessors rather than by exporting `cylinder.rs`'s private
//! `point_on_cylinder`, so the frozen module gains no new surface.

use super::cylinder::CylinderSchema;
use super::numeric::FiniteF64;
use super::planar_slice::SliceCategory;
use std::f64::consts::TAU;
use truck_geometry::prelude::{InnerSpace, Point2, Point3, Vector3};

/// Which curve family a witness represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessClass {
    /// A straight segment parallel to the cylinder axis: constant angular
    /// coordinate, varying axial coordinate.
    AxialLine,
    /// A circular arc at constant axial coordinate, sweeping the angular
    /// coordinate. May cross the seam.
    CircumferentialArc,
}

impl WitnessClass {
    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::AxialLine => "axial_line",
            Self::CircumferentialArc => "circumferential_arc",
        }
    }
}

/// Why a curve could not be certified as a witness in the developed cover.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WitnessFailure {
    /// An input coordinate was `NaN` or infinite.
    NonFiniteInput,
    /// The traversal start point is not on the certified cylinder.
    StartNotOnCylinder,
    /// The traversal end point is not on the certified cylinder.
    EndNotOnCylinder,
    /// An axial-line candidate's endpoints are not separated by a segment
    /// parallel to the axis.
    NotParallelToAxis,
    /// An axial-line candidate's endpoints coincide, so it has no direction.
    DegenerateAxialSegment,
    /// A circumferential-arc candidate's endpoints do not share an axial
    /// coordinate.
    NotConstantAxialCoordinate,
    /// A circumferential-arc candidate declared a zero angular sweep, so it
    /// has no direction.
    ZeroSweep,
    /// The declared sweep, developed from the start angle, does not land on
    /// the declared end point.
    SweepDoesNotReachDeclaredEndpoint,
    /// A complete-circle candidate's own certified placement is not the
    /// cylinder parallel through its endpoint: its plane is not perpendicular
    /// to the axis, its center is not on the axis, or its radius is not the
    /// cylinder's.
    CircleNotACylinderParallel,
    /// A complete-circle candidate was presented on an occurrence whose start
    /// and end are *different* source vertices, so no authoritative fact says
    /// the occurrence covers the circle's whole period.
    OccurrenceNotClosed,
}

impl WitnessFailure {
    /// Which semantic category this failure belongs to, in the taxonomy
    /// `FORMAL_SYSTEM.md` Definition 3 already fixes for the planar slice.
    ///
    /// The line here is *authority*, not severity: a curve whose endpoints do
    /// not lie on the certified cylinder, or whose own declared parameter
    /// interval does not reach its own declared endpoint, contradicts a claim
    /// the source itself makes — `Inconsistent`. A curve that is simply not
    /// axial or not at constant axial coordinate is not contradicting
    /// anything; it is valid geometry the supported subset does not cover —
    /// `Unsupported`, the same as a degenerate or zero-length candidate. A
    /// non-finite input coordinate is a statement about the machine.
    pub fn category(self) -> SliceCategory {
        match self {
            Self::NonFiniteInput => SliceCategory::OperationalFailure,
            Self::StartNotOnCylinder
            | Self::EndNotOnCylinder
            | Self::SweepDoesNotReachDeclaredEndpoint => SliceCategory::Inconsistent,
            Self::NotParallelToAxis
            | Self::NotConstantAxialCoordinate
            | Self::DegenerateAxialSegment
            | Self::ZeroSweep
            | Self::CircleNotACylinderParallel => SliceCategory::Unsupported,
            // Not a contradiction and not a proved fact about the face: the
            // source presented a curve whose own trim carries no extent on an
            // occurrence whose source topology does not supply one either, so
            // nothing establishes what the occurrence covers.
            Self::OccurrenceNotClosed => SliceCategory::Unresolved,
        }
    }

    /// A short stable tag, for probe records.
    pub fn tag(self) -> &'static str {
        match self {
            Self::NonFiniteInput => "witness_non_finite_input",
            Self::StartNotOnCylinder => "witness_start_not_on_cylinder",
            Self::EndNotOnCylinder => "witness_end_not_on_cylinder",
            Self::NotParallelToAxis => "witness_not_parallel_to_axis",
            Self::DegenerateAxialSegment => "witness_degenerate_axial_segment",
            Self::NotConstantAxialCoordinate => "witness_not_constant_axial_coordinate",
            Self::ZeroSweep => "witness_zero_sweep",
            Self::SweepDoesNotReachDeclaredEndpoint => "witness_sweep_does_not_reach_endpoint",
            Self::CircleNotACylinderParallel => "witness_circle_not_a_cylinder_parallel",
            Self::OccurrenceNotClosed => "witness_occurrence_not_closed",
        }
    }
}

/// Dimensionless-in-radius tolerance floor shared by the on-cylinder and
/// constant-coordinate checks below. `1e-9` matches the parallelism and
/// convention floors [`super::cylinder`] already certifies at, so a witness is
/// refused only when the discrepancy is orders of magnitude past floating
/// point noise.
const RELATIVE_TOLERANCE: f64 = 1e-9;

fn finite_point(point: Point3) -> Result<Point3, WitnessFailure> {
    for coordinate in [point.x, point.y, point.z] {
        FiniteF64::new(coordinate).map_err(|_| WitnessFailure::NonFiniteInput)?;
    }
    Ok(point)
}

/// Whether `point` lies on the cylinder, within a tolerance scaled by radius.
fn radial_gap(schema: &CylinderSchema, point: Point3) -> f64 {
    let r = point - schema.origin();
    let axial = r.dot(schema.axis());
    let radial = r - axial * schema.axis();
    (radial.magnitude() - schema.radius().get()).abs()
}

fn scaled_tolerance(schema: &CylinderSchema) -> f64 {
    RELATIVE_TOLERANCE * schema.radius().get().max(1.0)
}

/// One curve-on-cylinder witness: a continuous developed segment, exact over
/// its complete interval.
///
/// `start` and `end` are in the developed `(axial, angular)` chart, with the
/// angular coordinate left unwrapped — a seam-crossing arc's `end.y` may lie
/// outside `[0, 2*PI)` or even be negative, and that is precisely how its
/// continuity is represented.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveOnCylinderWitness {
    /// Which curve family this witness represents.
    pub class: WitnessClass,
    /// The developed start point: `(axial_coordinate, angular_coordinate)`.
    pub start: Point2,
    /// The developed end point, continuous with `start` in the universal
    /// cover — not reduced modulo the period.
    pub end: Point2,
}

impl CurveOnCylinderWitness {
    /// The developed displacement `end - start`.
    pub fn displacement(&self) -> Point2 {
        Point2::new(self.end.x - self.start.x, self.end.y - self.start.y)
    }
}

/// Certify an axial-line witness from its physical endpoints.
///
/// `start` and `end` are the traversal-order endpoints — the composed sense
/// already applied exactly once, upstream, the same discipline
/// [`super::planar_slice::certified_planar_curves`] follows. Reversing the
/// traversal is exactly swapping `start` and `end`; nothing else changes.
pub fn axial_line_witness(
    schema: &CylinderSchema,
    start: Point3,
    end: Point3,
) -> Result<CurveOnCylinderWitness, WitnessFailure> {
    let start = finite_point(start)?;
    let end = finite_point(end)?;
    let tolerance = scaled_tolerance(schema);

    if radial_gap(schema, start) > tolerance {
        return Err(WitnessFailure::StartNotOnCylinder);
    }
    if radial_gap(schema, end) > tolerance {
        return Err(WitnessFailure::EndNotOnCylinder);
    }

    let axial_start = schema.axial_coordinate(start);
    let axial_end = schema.axial_coordinate(end);
    if (axial_end - axial_start).abs() <= tolerance {
        return Err(WitnessFailure::DegenerateAxialSegment);
    }

    // Constant angular coordinate: the two points must share a radial
    // direction. `angular_coordinate` is `atan2` in the recorded frame, whose
    // range is `(-PI, PI]`, so two points at the *same* radial direction
    // return the identical value with no branch ambiguity — unlike the arc
    // case, an axial segment never approaches the seam from two sides.
    let theta_start = schema.angular_coordinate(start);
    let theta_end = schema.angular_coordinate(end);
    if (theta_end - theta_start).abs() > RELATIVE_TOLERANCE {
        return Err(WitnessFailure::NotParallelToAxis);
    }

    // Each endpoint reports *its own* directly-computed angular coordinate,
    // not one borrowed from the other end — verified equal within tolerance
    // above, but reported this way so that an adjacent occurrence sharing
    // this vertex, which computes the identical coordinate independently
    // from the identical canonical position, gets a bit-exact match rather
    // than one perturbed by this witness's internal rounding path.
    Ok(CurveOnCylinderWitness {
        class: WitnessClass::AxialLine,
        start: Point2::new(axial_start, theta_start),
        end: Point2::new(axial_end, theta_end),
    })
}

/// Certify a circumferential-arc witness from its physical endpoints and its
/// source curve's own declared signed angular sweep.
///
/// `declared_sweep` is the source curve's trimmed parameter interval, `u2 -
/// u1`, in the surface's own angular parameter — exact and unwrapped. It is
/// what the developed end angle is built *from*; the physical `end` point is
/// only the sample that confirms it lands there, never independently used to
/// pick a branch. A seam-crossing arc is exactly one whose declared sweep
/// carries `theta_start + declared_sweep` outside `[0, 2*PI)`, and it is
/// represented that way, not wrapped back in.
pub fn circumferential_arc_witness(
    schema: &CylinderSchema,
    start: Point3,
    end: Point3,
    declared_sweep: f64,
) -> Result<CurveOnCylinderWitness, WitnessFailure> {
    let start = finite_point(start)?;
    let end = finite_point(end)?;
    let sweep = FiniteF64::new(declared_sweep)
        .map_err(|_| WitnessFailure::NonFiniteInput)?
        .get();
    let tolerance = scaled_tolerance(schema);

    if radial_gap(schema, start) > tolerance {
        return Err(WitnessFailure::StartNotOnCylinder);
    }
    if radial_gap(schema, end) > tolerance {
        return Err(WitnessFailure::EndNotOnCylinder);
    }

    let axial_start = schema.axial_coordinate(start);
    let axial_end = schema.axial_coordinate(end);
    if (axial_end - axial_start).abs() > tolerance {
        return Err(WitnessFailure::NotConstantAxialCoordinate);
    }

    if sweep == 0.0 {
        return Err(WitnessFailure::ZeroSweep);
    }

    let theta_start = schema.angular_coordinate(start);
    let theta_end_developed = theta_start + sweep;

    // Confirm — never certify — that the declared sweep actually reaches the
    // declared endpoint: the developed angle reduced to the principal branch
    // must agree with the endpoint's own angular coordinate.
    let theta_end_actual = schema.angular_coordinate(end);
    let reduced = principal_branch(theta_end_developed);
    let mut angular_gap = (reduced - theta_end_actual).abs();
    if angular_gap > TAU - angular_gap {
        angular_gap = TAU - angular_gap;
    }
    if angular_gap > RELATIVE_TOLERANCE {
        return Err(WitnessFailure::SweepDoesNotReachDeclaredEndpoint);
    }

    // The axial coordinate is each endpoint's own directly-computed value —
    // verified equal to the other end's within tolerance above, but reported
    // this way so a join against an adjacent occurrence sharing this vertex
    // matches bit-exactly. The angular coordinate cannot receive the same
    // treatment: `theta_end_developed` is the unwrapped developed angle a
    // seam-crossing arc needs, and `schema.angular_coordinate(end)` is the
    // wrapped principal-branch value confirmed above, not a substitute for it.
    Ok(CurveOnCylinderWitness {
        class: WitnessClass::CircumferentialArc,
        start: Point2::new(axial_start, theta_start),
        end: Point2::new(axial_end, theta_end_developed),
    })
}

/// A source circle's own certified placement, as the occurrence's own curve
/// domain presents it.
///
/// `sweep_axis` is the unit vector about which *this occurrence's own curve
/// parameter* advances right-handedly — the circle's own
/// `basis_cos x basis_sin` normal with the converted curve's parameter sense
/// already folded in exactly once, the same fold
/// [`SourceCurveFamily::CircularArc`]'s `parameter_interval` already carries.
/// It is the fact a *complete* circle needs and its collapsed trim interval no
/// longer holds: which way around the cylinder the occurrence runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompleteCirclePlacement {
    /// The circle's center.
    pub center: Point3,
    /// See the type docs.
    pub sweep_axis: Vector3,
    /// The circle's certified radius.
    pub radius: f64,
    /// Whether the source `Processor`'s orientation was forward (`true`) or
    /// inverted (`false`). The `sweep_axis` already folds this; carrying the
    /// flag separately lets a downstream consumer recover the un-oriented
    /// normal.
    pub curve_orientation: bool,
}

/// Certify a complete-circle witness: one occurrence that traverses an entire
/// cylinder parallel exactly once.
///
/// This is the case a trimmed parameter interval cannot express. A source
/// `edge_curve` on a full circle has one vertex used for both of its ends, so
/// an importer that recovers the trim by solving each end onto the curve gets
/// the *same* parameter twice and hands downstream a degenerate interval
/// `(u, u)` — not because the source declared a zero sweep, but because two
/// coincident endpoints carry no extent. The extent is instead the circle's
/// own complete period, and the direction is the occurrence's own parameter
/// sense (`placement.sweep_axis`), neither of which is guessed from the
/// coincident endpoints.
///
/// The endpoint therefore cannot confirm anything here the way
/// [`circumferential_arc_witness`]'s does — `theta_start ± period` reduces to
/// `theta_start` for *any* circle. So the obligation the endpoint carries in
/// the arc case is discharged structurally instead: the circle's own certified
/// placement must *be* the cylinder parallel through that endpoint — plane
/// perpendicular to the axis, center on the axis, radius the cylinder's — and
/// the sweep is the surface's own certified period, never a full turn assumed
/// from coincident endpoints.
pub fn complete_circle_witness(
    schema: &CylinderSchema,
    placement: CompleteCirclePlacement,
    start: Point3,
    forward: bool,
) -> Result<CurveOnCylinderWitness, WitnessFailure> {
    let start = finite_point(start)?;
    let center = finite_point(placement.center)?;
    for coordinate in [
        placement.sweep_axis.x,
        placement.sweep_axis.y,
        placement.sweep_axis.z,
        placement.radius,
    ] {
        FiniteF64::new(coordinate).map_err(|_| WitnessFailure::NonFiniteInput)?;
    }
    let tolerance = scaled_tolerance(schema);

    if radial_gap(schema, start) > tolerance {
        return Err(WitnessFailure::StartNotOnCylinder);
    }

    // The circle is the cylinder parallel through `start`, structurally. Every
    // obligation is read from the source circle's own certified placement;
    // none of them is recovered from a sample.
    //
    // Unit length and parallelism are separate obligations, checked
    // separately. `sweep_axis`'s magnitude is a claim about the *caller* —
    // that it handed over a direction and not an unnormalized cross product —
    // and its alignment with the axis is a claim about the *face*. Folding
    // both into one `|dot| == 1` test would let the two errors alias: a
    // non-unit vector at an oblique angle can land on `|dot| == 1`, and the
    // sign taken from it below would then be read off geometry that was never
    // certified perpendicular to the axis at all.
    let sweep_axis_length = placement.sweep_axis.magnitude();
    if (sweep_axis_length - 1.0).abs() > RELATIVE_TOLERANCE {
        return Err(WitnessFailure::CircleNotACylinderParallel);
    }
    let axis_alignment = placement.sweep_axis.dot(schema.axis());
    if (axis_alignment.abs() - 1.0).abs() > RELATIVE_TOLERANCE {
        return Err(WitnessFailure::CircleNotACylinderParallel);
    }
    let center_offset = center - schema.origin();
    let center_radial = center_offset - center_offset.dot(schema.axis()) * schema.axis();
    if center_radial.magnitude() > tolerance {
        return Err(WitnessFailure::CircleNotACylinderParallel);
    }
    if (placement.radius - schema.radius().get()).abs() > tolerance {
        return Err(WitnessFailure::CircleNotACylinderParallel);
    }

    // The sweep: the surface's *own* certified angular period, signed by the
    // occurrence's own parameter sense against the surface's own angular
    // sense, then composed with the selected edge-use direction exactly once.
    // `angular_coordinate` runs right-handedly about `axis` (its frame is
    // `radial_y == axis x radial_x`), so an occurrence whose parameter
    // advances right-handedly about `+axis` advances the angular coordinate,
    // and one about `-axis` retreats it.
    let period = schema.deck_generator().signed_period().get();
    let parameter_sense = if axis_alignment > 0.0 { 1.0 } else { -1.0 };
    let selected_sense = if forward { 1.0 } else { -1.0 };
    let sweep = period * parameter_sense * selected_sense;

    // Both developed ends report the *same* axial coordinate value, bit for
    // bit: the occurrence begins and ends at one source vertex, whose single
    // canonical position is the only thing either end is computed from.
    let axial = schema.axial_coordinate(start);
    let theta_start = schema.angular_coordinate(start);
    Ok(CurveOnCylinderWitness {
        class: WitnessClass::CircumferentialArc,
        start: Point2::new(axial, theta_start),
        end: Point2::new(axial, theta_start + sweep),
    })
}

// ---------------------------------------------------------------------------
// Source-authoritative classification
// ---------------------------------------------------------------------------

/// Which curve family a source edge's 3D representation structurally *is* —
/// the source-authoritative input to [`identify_source_curve_witness`] that
/// decides witness class and declared sweep.
///
/// Deliberately carries no endpoint positions of its own. The canonical
/// endpoints [`identify_source_curve_witness`] certifies against always come
/// from the caller's certified vertex positions (the same
/// `vertex_position(...)` accessor [`super::planar_slice::certified_planar_curves`]
/// uses), never from a curve's own possibly-independently-rounded copy of a
/// shared vertex — that discipline is what let two adjacent occurrences join
/// bit-exactly in Checkpoint 6, and folding a curve's own endpoint back in
/// here would quietly undo it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SourceCurveFamily {
    /// A straight segment.
    Line,
    /// A circular arc, with the source curve's own trimmed parameter
    /// interval `(t0, t1)` in the curve's own direction. For a circle
    /// parameterised as `center + radius (cos(t) radial_x + sin(t)
    /// radial_y)`, `t1 - t0` is exactly the signed angular sweep.
    CircularArc {
        /// The trimmed parameter interval, in the curve's own direction.
        parameter_interval: (f64, f64),
    },
    /// A *complete* circle: a circular source curve whose trimmed parameter
    /// interval carries no extent, because the source edge closes on one
    /// vertex and both of its ends solve to the same curve parameter. See
    /// [`complete_circle_witness`], which is the only thing that certifies
    /// one, and which requires the occurrence's source topology to close as
    /// well before it accepts the circle's period as the extent.
    CompleteCircle {
        /// The circle's own certified placement and parameter sense.
        placement: CompleteCirclePlacement,
    },
}

/// Step 3's rank-1 route. Classify a source curve, in the selected traversal
/// direction, into a certified witness — deriving the witness's *class* from
/// which [`SourceCurveFamily`] variant the source curve structurally is, and
/// (for an arc) its declared sweep from the curve's own parameter interval —
/// never from a caller's bare assertion of either.
///
/// `start`/`end` are the traversal-order canonical endpoints (see the module
/// docs); `forward` is Step 2's retained composed sense
/// ([`super::planar_slice::TraversalOccurrence::forward`]): `true` means the
/// traversal runs with the curve's own parameter direction (`t0 -> t1`),
/// `false` means against it (`t1 -> t0`, sweep negated). `closed_occurrence`
/// is the other source-identity fact Step 2 retained: whether this
/// occurrence's start and end are the *same* source vertex. Only
/// [`SourceCurveFamily::CompleteCircle`] consults it, and it is passed in
/// rather than inferred here for the same reason `forward` is — it is a fact
/// about source topology, and nothing in a pair of coordinates can stand in
/// for it. This is the single
/// production introduction rule for a rank-1 witness; [`WitnessSpec`] remains
/// available only for constructing synthetic test fixtures that do not carry
/// a full source curve representation.
///
/// [`WitnessSpec`]: super::cylinder_lift::WitnessSpec
pub fn identify_source_curve_witness(
    schema: &CylinderSchema,
    family: SourceCurveFamily,
    start: Point3,
    end: Point3,
    forward: bool,
    closed_occurrence: bool,
) -> Result<CurveOnCylinderWitness, WitnessFailure> {
    match family {
        SourceCurveFamily::Line => axial_line_witness(schema, start, end),
        // A complete circle's extent is its own period, and only an
        // occurrence that Step 2 already joined *to itself* — same source
        // vertex at both ends, by identity — presents one. Coincident
        // coordinates are never accepted in place of that fact: two distinct
        // source vertices that happen to sit at one point would be a
        // zero-length edge, not a full turn.
        SourceCurveFamily::CompleteCircle { placement } => {
            if !closed_occurrence {
                return Err(WitnessFailure::OccurrenceNotClosed);
            }
            complete_circle_witness(schema, placement, start, forward)
        }
        SourceCurveFamily::CircularArc {
            parameter_interval: (t0, t1),
        } => {
            FiniteF64::new(t0).map_err(|_| WitnessFailure::NonFiniteInput)?;
            FiniteF64::new(t1).map_err(|_| WitnessFailure::NonFiniteInput)?;
            if t0 == t1 {
                return Err(WitnessFailure::ZeroSweep);
            }
            let signed_sweep = if forward { t1 - t0 } else { t0 - t1 };
            circumferential_arc_witness(schema, start, end, signed_sweep)
        }
    }
}

/// Reduce an angular coordinate into the surface's own principal branch
/// `(-PI, PI]`, matching `atan2`'s range exactly so a developed value can be
/// compared against `angular_coordinate`'s output.
fn principal_branch(theta: f64) -> f64 {
    let wrapped = theta.rem_euclid(TAU);
    if wrapped > std::f64::consts::PI {
        wrapped - TAU
    } else {
        wrapped
    }
}

#[cfg(test)]
mod tests {
    use super::super::cylinder::{identify_cylinder, CylinderIdentification};
    use super::*;
    use truck_geometry::prelude::{Line, RevolutedCurve, Vector3};

    fn z_cylinder(radius: f64, h: f64) -> CylinderSchema {
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(radius, 0.0, 0.0), Point3::new(radius, 0.0, h)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        match identify_cylinder(&revo) {
            CylinderIdentification::Cylinder(c) => c.schema().clone(),
            other => panic!("expected a certified cylinder, got {other:?}"),
        }
    }

    /// A point on the cylinder at angle `theta` and axial coordinate `z`.
    fn on_cylinder(schema: &CylinderSchema, z: f64, theta: f64) -> Point3 {
        schema.origin()
            + z * schema.axis()
            + schema.radius().get() * theta.cos() * schema.radial_x()
            + schema.radius().get() * theta.sin() * schema.radial_y()
    }

    // -- axial line -------------------------------------------------------

    #[test]
    fn positive_axial_direction_is_a_witness() {
        let schema = z_cylinder(2.0, 5.0);
        let start = on_cylinder(&schema, 0.0, 0.3);
        let end = on_cylinder(&schema, 3.0, 0.3);
        let w = axial_line_witness(&schema, start, end).expect("axial witness");
        assert_eq!(w.class, WitnessClass::AxialLine);
        assert!((w.start.x - 0.0).abs() < 1e-9);
        assert!((w.end.x - 3.0).abs() < 1e-9);
        assert!((w.start.y - w.end.y).abs() < 1e-9, "constant angle");
        assert!(w.displacement().x > 0.0);
    }

    #[test]
    fn negative_axial_direction_is_a_witness() {
        let schema = z_cylinder(2.0, 5.0);
        let start = on_cylinder(&schema, 3.0, 0.3);
        let end = on_cylinder(&schema, 0.0, 0.3);
        let w = axial_line_witness(&schema, start, end).expect("axial witness");
        assert!(w.displacement().x < 0.0);
        assert!((w.start.y - w.end.y).abs() < 1e-9);
    }

    #[test]
    fn a_segment_off_axis_is_refused() {
        let schema = z_cylinder(2.0, 5.0);
        let start = on_cylinder(&schema, 0.0, 0.3);
        let end = on_cylinder(&schema, 3.0, 0.9); // angle also changes
        assert_eq!(
            axial_line_witness(&schema, start, end).unwrap_err(),
            WitnessFailure::NotParallelToAxis
        );
    }

    #[test]
    fn a_degenerate_axial_segment_is_refused() {
        let schema = z_cylinder(2.0, 5.0);
        let point = on_cylinder(&schema, 1.0, 0.3);
        assert_eq!(
            axial_line_witness(&schema, point, point).unwrap_err(),
            WitnessFailure::DegenerateAxialSegment
        );
    }

    // -- circumferential arc ----------------------------------------------

    #[test]
    fn positive_angular_direction_is_a_witness() {
        let schema = z_cylinder(2.0, 5.0);
        let start = on_cylinder(&schema, 1.0, 0.2);
        let end = on_cylinder(&schema, 1.0, 1.4);
        let w = circumferential_arc_witness(&schema, start, end, 1.2).expect("arc witness");
        assert_eq!(w.class, WitnessClass::CircumferentialArc);
        assert!(
            (w.start.x - w.end.x).abs() < 1e-9,
            "constant axial coordinate"
        );
        assert!((w.displacement().y - 1.2).abs() < 1e-9);
    }

    #[test]
    fn negative_angular_direction_is_a_witness() {
        let schema = z_cylinder(2.0, 5.0);
        let start = on_cylinder(&schema, 1.0, 1.4);
        let end = on_cylinder(&schema, 1.0, 0.2);
        let w = circumferential_arc_witness(&schema, start, end, -1.2).expect("arc witness");
        assert!((w.displacement().y - (-1.2)).abs() < 1e-9);
    }

    #[test]
    fn seam_crossing_in_the_positive_direction_stays_continuous() {
        let schema = z_cylinder(2.0, 5.0);
        // From just before the seam to just after it, the short way: sweep of
        // +0.4 carries theta from 3.0 past PI (the branch cut) to -2.983...
        // in the principal branch, but the developed value must keep climbing.
        let theta_start = 3.0;
        let sweep = 0.4;
        let start = on_cylinder(&schema, 2.0, theta_start);
        let end = on_cylinder(&schema, 2.0, theta_start + sweep);
        let w = circumferential_arc_witness(&schema, start, end, sweep).expect("seam witness");
        assert!((w.start.y - theta_start).abs() < 1e-9);
        assert!(
            (w.end.y - (theta_start + sweep)).abs() < 1e-9,
            "developed angle is not wrapped back into the principal branch: {}",
            w.end.y
        );
        assert!(w.end.y > TAU / 2.0 - 1e-6 || w.end.y > std::f64::consts::PI);
    }

    #[test]
    fn seam_crossing_in_the_negative_direction_stays_continuous() {
        let schema = z_cylinder(2.0, 5.0);
        let theta_start = -3.0;
        let sweep = -0.4;
        let start = on_cylinder(&schema, 2.0, theta_start);
        let end = on_cylinder(&schema, 2.0, theta_start + sweep);
        let w = circumferential_arc_witness(&schema, start, end, sweep).expect("seam witness");
        assert!((w.end.y - (theta_start + sweep)).abs() < 1e-9);
        assert!(w.end.y < -std::f64::consts::PI);
    }

    #[test]
    fn a_wrong_declared_sweep_is_refused() {
        let schema = z_cylinder(2.0, 5.0);
        let start = on_cylinder(&schema, 1.0, 0.2);
        let end = on_cylinder(&schema, 1.0, 1.4);
        // The physical endpoint sits at theta=1.4; declaring a sweep of 1.5
        // develops to theta=1.7, a different point than the one sampled, so
        // the confirmation must reject it rather than trust the declaration.
        let wrong = circumferential_arc_witness(&schema, start, end, 1.5);
        assert_eq!(
            wrong.unwrap_err(),
            WitnessFailure::SweepDoesNotReachDeclaredEndpoint
        );
    }

    #[test]
    fn selected_source_edge_reversal_swaps_start_and_end_and_negates_sweep() {
        let schema = z_cylinder(2.0, 5.0);
        let a = on_cylinder(&schema, 1.0, 0.2);
        let b = on_cylinder(&schema, 1.0, 1.4);
        let forward = circumferential_arc_witness(&schema, a, b, 1.2).expect("forward");
        let reversed = circumferential_arc_witness(&schema, b, a, -1.2).expect("reversed");
        assert!((forward.start.y - reversed.end.y).abs() < 1e-9);
        assert!((forward.end.y - reversed.start.y).abs() < 1e-9);
        assert!((forward.displacement().y + reversed.displacement().y).abs() < 1e-9);
    }

    #[test]
    fn a_non_constant_axial_coordinate_is_refused_for_an_arc() {
        let schema = z_cylinder(2.0, 5.0);
        let start = on_cylinder(&schema, 0.0, 0.2);
        let end = on_cylinder(&schema, 1.0, 1.4);
        assert_eq!(
            circumferential_arc_witness(&schema, start, end, 1.2).unwrap_err(),
            WitnessFailure::NotConstantAxialCoordinate
        );
    }

    // -- category taxonomy --------------------------------------------------

    #[test]
    fn non_admitted_but_valid_geometry_is_unsupported_not_inconsistent() {
        assert_eq!(
            WitnessFailure::NotParallelToAxis.category(),
            SliceCategory::Unsupported
        );
        assert_eq!(
            WitnessFailure::NotConstantAxialCoordinate.category(),
            SliceCategory::Unsupported
        );
    }

    #[test]
    fn contradictions_with_authoritative_source_claims_stay_inconsistent() {
        assert_eq!(
            WitnessFailure::StartNotOnCylinder.category(),
            SliceCategory::Inconsistent
        );
        assert_eq!(
            WitnessFailure::EndNotOnCylinder.category(),
            SliceCategory::Inconsistent
        );
        assert_eq!(
            WitnessFailure::SweepDoesNotReachDeclaredEndpoint.category(),
            SliceCategory::Inconsistent
        );
    }

    // -- source-authoritative classification --------------------------------

    #[test]
    fn a_source_line_classifies_as_an_axial_witness_in_its_own_direction() {
        let schema = z_cylinder(2.0, 5.0);
        let a = on_cylinder(&schema, 0.0, 0.3);
        let b = on_cylinder(&schema, 3.0, 0.3);
        let w = identify_source_curve_witness(&schema, SourceCurveFamily::Line, a, b, true, false)
            .expect("a source line classifies");
        assert_eq!(w.class, WitnessClass::AxialLine);
        assert!(w.displacement().x > 0.0);
    }

    #[test]
    fn a_reversed_source_line_swaps_endpoints() {
        let schema = z_cylinder(2.0, 5.0);
        let a = on_cylinder(&schema, 0.0, 0.3);
        let b = on_cylinder(&schema, 3.0, 0.3);
        // Traversal order is `b -> a`; the family is still `Line`, and only
        // the caller-supplied endpoint order changes, exactly as
        // `axial_line_witness` already expects.
        let w = identify_source_curve_witness(&schema, SourceCurveFamily::Line, b, a, false, false)
            .expect("a reversed source line classifies");
        assert!(w.displacement().x < 0.0);
    }

    /// The declared sweep is derived from the circle's own parameter
    /// interval `t1 - t0`, not asserted by the caller.
    #[test]
    fn a_source_circular_arc_classifies_with_its_own_declared_sweep() {
        let schema = z_cylinder(2.0, 5.0);
        let start = on_cylinder(&schema, 1.0, 0.2);
        let end = on_cylinder(&schema, 1.0, 1.4);
        let family = SourceCurveFamily::CircularArc {
            parameter_interval: (0.2, 1.4),
        };
        let w = identify_source_curve_witness(&schema, family, start, end, true, false)
            .expect("a source circular arc classifies");
        assert_eq!(w.class, WitnessClass::CircumferentialArc);
        assert!((w.displacement().y - 1.2).abs() < 1e-9);
    }

    #[test]
    fn a_reversed_source_circular_arc_negates_the_declared_sweep() {
        let schema = z_cylinder(2.0, 5.0);
        let start = on_cylinder(&schema, 1.0, 0.2);
        let end = on_cylinder(&schema, 1.0, 1.4);
        let family = SourceCurveFamily::CircularArc {
            parameter_interval: (0.2, 1.4),
        };
        let forward = identify_source_curve_witness(&schema, family, start, end, true, false)
            .expect("forward");
        // Traversal order reverses: endpoints swap and the sweep negates.
        let reversed = identify_source_curve_witness(&schema, family, end, start, false, false)
            .expect("reversed");
        assert!((forward.displacement().y + reversed.displacement().y).abs() < 1e-9);
    }

    // -- complete circles ---------------------------------------------------

    /// The corpus shape, in the small: a closed circular `edge_curve` whose
    /// trim collapsed to a point still develops to a full turn of the
    /// surface's own certified period — taken from the circle's placement and
    /// the closed-edge rule, never from the coincident endpoints.
    #[test]
    fn a_complete_circle_sweeps_the_surfaces_own_certified_period() {
        let schema = z_cylinder(2.0, 5.0);
        let point = on_cylinder(&schema, 1.0, 0.4);
        let placement = CompleteCirclePlacement {
            center: schema.origin() + 1.0 * schema.axis(),
            sweep_axis: schema.axis(),
            radius: schema.radius().get(),
            curve_orientation: true,
        };
        let w = complete_circle_witness(&schema, placement, point, true)
            .expect("a closed circular edge on the parallel is a complete-circle witness");
        assert_eq!(w.class, WitnessClass::CircumferentialArc);
        let period = schema.deck_generator().signed_period().get();
        assert!((w.displacement().y - period).abs() < 1e-12);
        assert_eq!(
            w.start.x, w.end.x,
            "both ends report one source vertex's single axial coordinate, bit for bit"
        );
    }

    /// Orientation sign, from the circle's own parameter sense. A circle whose
    /// parameter advances right-handedly about `-axis` runs the *other* way
    /// round the cylinder, and nothing about its coincident endpoints says so.
    #[test]
    fn a_reversed_sweep_axis_reverses_the_complete_circles_direction() {
        let schema = z_cylinder(2.0, 5.0);
        let point = on_cylinder(&schema, 1.0, 0.4);
        let center = schema.origin() + 1.0 * schema.axis();
        let radius = schema.radius().get();
        let forward_axis = CompleteCirclePlacement {
            center,
            sweep_axis: schema.axis(),
            radius,
            curve_orientation: true,
        };
        let reversed_axis = CompleteCirclePlacement {
            center,
            sweep_axis: -schema.axis(),
            radius,
            curve_orientation: true,
        };
        let a = complete_circle_witness(&schema, forward_axis, point, true).expect("witness");
        let b = complete_circle_witness(&schema, reversed_axis, point, true).expect("witness");
        assert!(
            (a.displacement().y + b.displacement().y).abs() < 1e-12,
            "the two parameter senses develop to opposite full turns"
        );
        assert!(a.displacement().y > 0.0 && b.displacement().y < 0.0);
    }

    /// The selected edge-use direction composes on top of the parameter sense,
    /// exactly once — the same discipline the arc witness follows.
    #[test]
    fn a_reversed_edge_use_reverses_the_complete_circles_direction_again() {
        let schema = z_cylinder(2.0, 5.0);
        let point = on_cylinder(&schema, 1.0, 0.4);
        let placement = CompleteCirclePlacement {
            center: schema.origin() + 1.0 * schema.axis(),
            sweep_axis: -schema.axis(),
            radius: schema.radius().get(),
            curve_orientation: true,
        };
        let forward = complete_circle_witness(&schema, placement, point, true).expect("witness");
        let reversed = complete_circle_witness(&schema, placement, point, false).expect("witness");
        assert!((forward.displacement().y + reversed.displacement().y).abs() < 1e-12);
        // Reversed edge use over a `-axis` circle is the same traversal as a
        // forward edge use over a `+axis` one: the two folds compose, and
        // neither is applied twice.
        let plus = CompleteCirclePlacement {
            sweep_axis: schema.axis(),
            ..placement
        };
        let plus_forward = complete_circle_witness(&schema, plus, point, true).expect("witness");
        assert!((reversed.displacement().y - plus_forward.displacement().y).abs() < 1e-12);
    }

    /// The closed-edge rule is about vertex *identity*. An occurrence whose
    /// two ends are different source vertices does not get the full-turn
    /// reading even when those vertices sit at the same point — that is a
    /// zero-length edge, and the refusal says the extent is unestablished
    /// rather than claiming a fact about the face.
    #[test]
    fn a_complete_circle_on_an_occurrence_with_distinct_vertices_is_refused() {
        let schema = z_cylinder(2.0, 5.0);
        let point = on_cylinder(&schema, 1.0, 0.4);
        let family = SourceCurveFamily::CompleteCircle {
            placement: CompleteCirclePlacement {
                center: schema.origin() + 1.0 * schema.axis(),
                sweep_axis: schema.axis(),
                radius: schema.radius().get(),
                curve_orientation: true,
            },
        };
        let closed = identify_source_curve_witness(&schema, family, point, point, true, true)
            .expect("a closed occurrence certifies");
        assert!(closed.displacement().y.abs() > 0.0);

        let open = identify_source_curve_witness(&schema, family, point, point, true, false)
            .expect_err("two distinct vertices at one point are not a closed edge");
        assert_eq!(open, WitnessFailure::OccurrenceNotClosed);
        assert_eq!(open.category(), SliceCategory::Unresolved);
    }

    /// The obligation the coincident endpoints cannot discharge is discharged
    /// on the circle's own placement instead: a circle that is not *the*
    /// cylinder parallel through its endpoint is refused, whichever of the
    /// three ways it misses.
    #[test]
    fn a_circle_that_is_not_the_cylinder_parallel_is_refused() {
        let schema = z_cylinder(2.0, 5.0);
        let point = on_cylinder(&schema, 1.0, 0.4);
        let center = schema.origin() + 1.0 * schema.axis();
        let radius = schema.radius().get();
        let refuse = |placement| {
            assert_eq!(
                complete_circle_witness(&schema, placement, point, true).unwrap_err(),
                WitnessFailure::CircleNotACylinderParallel
            );
        };
        // Plane not perpendicular to the axis.
        refuse(CompleteCirclePlacement {
            center,
            sweep_axis: schema.radial_x(),
            radius,
            curve_orientation: true,
        });
        // Center off the axis.
        refuse(CompleteCirclePlacement {
            center: center + 0.5 * schema.radial_x(),
            sweep_axis: schema.axis(),
            radius,
            curve_orientation: true,
        });
        // Not the cylinder's radius.
        refuse(CompleteCirclePlacement {
            center,
            sweep_axis: schema.axis(),
            radius: radius * 0.5,
            curve_orientation: true,
        });
        // An unnormalized sweep axis is a caller defect, and it is refused on
        // its own length rather than aliasing into the parallelism test.
        refuse(CompleteCirclePlacement {
            center,
            sweep_axis: 2.0 * schema.axis(),
            radius,
            curve_orientation: true,
        });
    }

    #[test]
    fn a_zero_length_source_parameter_interval_is_refused() {
        let schema = z_cylinder(2.0, 5.0);
        let point = on_cylinder(&schema, 1.0, 0.2);
        let family = SourceCurveFamily::CircularArc {
            parameter_interval: (0.2, 0.2),
        };
        let exit =
            identify_source_curve_witness(&schema, family, point, point, true, true).unwrap_err();
        assert_eq!(exit, WitnessFailure::ZeroSweep);
        assert_eq!(exit.category(), SliceCategory::Unsupported);
    }
}
