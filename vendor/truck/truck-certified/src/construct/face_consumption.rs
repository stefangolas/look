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

//! CC-032-FACE-CONSUMPTION (CC program Phase D, spine S12 consumer; theory
//! §5.4): the trim arrangement decides what survives the blend.
//!
//! Do not decide in advance how much of a support face survives. The face's
//! surviving trim is an OUTCOME: construct the arrangement of the trimming
//! pcurves inside the support chart, mark the removed cells, and take
//! `F_i_new = F_i \ R_i`, where `R_i` is the removed region — the blend side
//! of the contact curves. The classic short intermediate face A-B-C vanishes
//! when its cell does: there is no cascading special-case solver here, and a
//! face whose retained set is empty is reported [`FaceOutcome::Vanished`].
//!
//! # The arrangement input (Section 1)
//!
//! [`FaceConsumption`] packages one support chart together with the contact
//! pcurves that trim it: each [`ContactPcurve`] is the parameter-space
//! projection of one certified contact curve `q_i(s)` of the [`BlendTrace`]
//! (`crate::construct::blend`, seam S12) onto the support's chart, and each
//! rides its landed provenance id through the parallel [`FaceConsumption::trim_provenance`]
//! vector ([`SourceRef`] carries those ids verbatim — it never re-derives
//! them). The arrangement's original trimming pcurves are the support chart's
//! own boundary (the rectangle [`SupportDescription::domain`]); the new
//! contact pcurves — including the contact curves of neighbouring fillets
//! that reach this face — are the curves this module arranges.
//!
//! # The certified v1 envelope and the arrangement seam
//!
//! The 2-D arrangement of the pcurves routes through the LANDED arrangement
//! machinery at the construction seam: the curve work — splitting the pcurves
//! at their mutual intersections into an analytic planar subdivision — is the
//! solver-family `arrange` of `truck-geometry::arrange` (A2), and the cell
//! classification this module carries is the stage that marks each resulting
//! cell on the blend side. The v1 fixtures of the blend spine are the
//! constant-radius planar class (CC-030/CC-031), whose contact pcurves on an
//! affine support chart are full straight chords of the chart rectangle (a
//! `u = const` chord or a `v = const` chord, endpoints on the chart boundary).
//! Over that envelope the arrangement is the exact product decomposition of
//! the chart by the chord coordinates — every cell is a parameter rectangle
//! whose edges lie either on the chart boundary or on one contact chord — and
//! that decomposition is realized here directly, deterministically, so the two
//! stages compose at the same seam. A contact pcurve outside that envelope (a
//! partial chord, a diagonal, or a non-analytic pcurve) is refused
//! [`ConstructRefusal::InvalidInput`] at chart build time; the general
//! trimmed-pcurve envelope is a later packet's booking.
//!
//! # Cell classification (Section 1, pre-made)
//!
//! Each cell carries the blend side of every contact chord as signed side
//! data: the signed function of a chord is `u − c` (a `u = const` chord) or
//! `v − c` (a `v = const` chord), and the chord's [`ContactPcurve::blend_side`]
//! names which side lies inside the removed region `R_i`. A cell is
//! classified by INTERVAL EVALUATION of that signed data over a certified box
//! in the cell's interior (a deterministic witness box, see
//! [`CELL_WITNESS_FRACTION`]); a cell whose evaluation cannot certify a sign
//! — an undecided cell — is refused with [`ConstructRefusal::ConditioningBelowThreshold`],
//! never labelled by a guess.
//!
//! # Determinism (the Σ-determines-topology obligation, theory §10.1)
//!
//! The arrangement stage's obligation is carried by this module's
//! determinism: identical traces produce identical chord-coordinate cuts,
//! hence identical cells in a fixed enumeration order, hence identical cell
//! labels and identical surviving combinatorics. Every reduction in this
//! module runs in a fixed order with directed rounding (C9); the only
//! randomness-free inputs are the exact endpoints of the certified pcurves.
//!
//! # Scope guards (stop conditions)
//!
//! (1) The concave-edge trim of the sharp offset variant reuses this module —
//! CC-024 owns the sharp-side entry; here only the blend-side classification
//! lands. (2) [`BlendTrace`] is CC-030's output; if the contact-pcurve
//! projection needs data that trace does not carry, that is a spine S12
//! refinement (a `QUESTION.md`), never a re-trace of branches here — this
//! module consumes the projected pcurves as its input record.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`. Every `Interval` below is the C3 universe
//! (`construct::Interval`), never a second interval type.
//!
//! [`BlendTrace`]: crate::construct::blend::BlendTrace

use crate::certified_map::SurfaceRegion;
use crate::construct::refusal::ConstructRefusal;
use crate::construct::Interval;

/// The fraction of a cell's width used to build the certified witness box in
/// the cell's interior, on each axis.
///
/// The signed side data of a contact chord is evaluated over this interior box
/// (not over the cell's closed box, whose boundary lies ON the chord). The
/// box is the middle `1 − 2·CELL_WITNESS_FRACTION` of the cell on each axis;
/// the 0.25 fraction keeps the witness a strict positive distance from every
/// chord of the arrangement while remaining deterministic (fixed-order `f64`
/// reductions over the exact cell endpoints).
pub const CELL_WITNESS_FRACTION: f64 = 0.25;

/// A reference into the landed provenance id space.
///
/// Every [`ContactPcurve`] rides one of these through the parallel
/// [`FaceConsumption::trim_provenance`] vector, and every surviving
/// [`RetainedCell`] records the ones of its bounding contact curves. The id
/// is the caller's landed provenance id verbatim — the source representation's
/// `SourceFaceId` / `SourceEdgeId` / `SourceEntityId` vocabulary — never a
/// value re-derived here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRef {
    /// The landed provenance id the reference rides on.
    pub id: u64,
}

/// The side of a contact chord named by signed side data.
///
/// For a chord at coordinate `c` the signed function is `u − c` (a `u = const`
/// chord) or `v − c` (a `v = const` chord). [`TrimSide::Lower`] is the side of
/// smaller parameter coordinate (negative signed value), [`TrimSide::Upper`]
/// the side of larger parameter coordinate (positive signed value).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimSide {
    /// The side of smaller parameter coordinate along the chord's span axis.
    Lower,
    /// The side of larger parameter coordinate along the chord's span axis.
    Upper,
}

/// The side of a [`ContactPcurve`] that lies inside the removed region `R_i`.
///
/// This is the contact curve's side sign carried by the blend trace: cells
/// whose signed-side interval evaluation certifies them on [`ContactPcurve::blend_side`]
/// are the removed cells of the arrangement. Same spelling as [`TrimSide`];
/// the two enums are kept apart so a removed side can never be mistaken for a
/// surviving-cell record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendSide {
    /// The removed region lies on the [`TrimSide::Lower`] side of the chord.
    Lower,
    /// The removed region lies on the [`TrimSide::Upper`] side of the chord.
    Upper,
}

impl BlendSide {
    /// The [`TrimSide`] this removed side names.
    ///
    /// A cell is removed when its certified side equals the chord's blend side
    /// in the shared signed-coordinate sense; the conversion is the only place
    /// the two vocabularies meet.
    pub fn trim_side(self) -> TrimSide {
        match self {
            BlendSide::Lower => TrimSide::Lower,
            BlendSide::Upper => TrimSide::Upper,
        }
    }
}

/// One support chart consumed by a blend.
///
/// `domain` is the support's full parameter chart — the original trimming
/// pcurves of `F_i`, closed as the chart rectangle — and `source` rides the
/// support's own landed provenance id.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportDescription {
    /// The certified parameter domain `((u0, u1), (v0, v1))` of the support
    /// face.
    pub domain: SurfaceRegion,
    /// The support's landed provenance reference.
    pub source: SourceRef,
}

/// The arrangement input of one face consumption.
///
/// `support` is the support chart being trimmed, `contact_pcurves` the
/// parameter-space projections of the certified contact curves that cut it —
/// the new contact pcurves and the neighbouring fillets' contact curves
/// reaching this face — and `trim_provenance` the landed provenance ids, one
/// per contact pcurve in index order.
#[derive(Debug, Clone)]
pub struct FaceConsumption {
    /// The support face being consumed.
    pub support: SupportDescription,
    /// The contact pcurves arranged inside the support chart.
    pub contact_pcurves: Vec<ContactPcurve>,
    /// The landed provenance id of every contact pcurve, in index order.
    pub trim_provenance: Vec<SourceRef>,
}

/// A contact pcurve: the parameter-space projection of one certified contact
/// curve `q_i(s)` onto the support chart.
///
/// In the certified v1 envelope the projection is a full chord of the chart
/// rectangle: `from`/`to` are two boundary points of the chart sharing one
/// parameter coordinate (a `u = const` chord when the shared coordinate is
/// `u`, a `v = const` chord when it is `v`), and the chord coordinate is
/// strictly interior to the chart. `blend_side` is the contact curve's side
/// sign: the side of the chord on which the removed region `R_i` — the blend
/// side — lies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContactPcurve {
    /// One endpoint of the chord, `[u, v]` in the support chart.
    pub from: [f64; 2],
    /// The other endpoint of the chord, `[u, v]` in the support chart.
    pub to: [f64; 2],
    /// The side of the chord inside the removed region `R_i`.
    pub blend_side: BlendSide,
}

/// A 2-D parameter cell of the trim arrangement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamBox {
    /// The cell's `u` interval.
    pub u: (f64, f64),
    /// The cell's `v` interval.
    pub v: (f64, f64),
}

/// One cell of the trim arrangement with its classification label.
///
/// `removed` is true exactly when the cell lies inside the removed region
/// `R_i` (the blend side of at least one contact chord).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrimCell {
    /// The 2-D cell.
    pub cell: ParamBox,
    /// Whether the cell lies inside the removed region `R_i`.
    pub removed: bool,
}

/// The trim provenance a surviving cell records.
///
/// The edit-graph requirement of the theory's output section: a surviving
/// cell records WHICH contact curve bounds it and ON WHICH SIDE the cell lies,
/// together with the curve's landed provenance reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrimBound {
    /// The index of the contact pcurve (into [`FaceConsumption::contact_pcurves`]).
    pub curve: usize,
    /// The side of the contact pcurve the surviving cell lies on.
    pub side: TrimSide,
    /// The landed provenance reference riding the contact pcurve.
    pub source: SourceRef,
}

/// A surviving cell of `F_i_new = F_i \ R_i`.
#[derive(Debug, Clone, PartialEq)]
pub struct RetainedCell {
    /// The retained 2-D cell.
    pub cell: ParamBox,
    /// The contact pcurves that bound the cell, each with the side the cell
    /// lies on and its provenance.
    pub bounds: Vec<TrimBound>,
}

/// The outcome of one face consumption.
///
/// A face either survives with its retained cells — `F_i_new = F_i \ R_i` is
/// the union of the retained cells, each carrying its trim provenance — or it
/// vanishes when the arrangement leaves it no retained cell. An empty retained
/// set is [`FaceOutcome::Vanished`], never an empty `Survived`.
#[derive(Debug, Clone, PartialEq)]
pub enum FaceOutcome {
    /// The face survives: `F_i_new` is the union of the retained cells.
    Survived {
        /// The retained cells of the face, in the arrangement's fixed cell
        /// order.
        retained: Vec<RetainedCell>,
    },
    /// The face is fully consumed: no cell of the arrangement survives.
    Vanished,
}

/// The validated arrangement plan of one face: the contact chords and the
/// product cells they induce.
struct Plan {
    /// The validated contact chords, in input index order.
    chords: Vec<Chord>,
    /// The arrangement cells in the fixed enumeration order (row-major over
    /// the `v` bands then the `u` slabs, both increasing).
    cells: Vec<ParamBox>,
}

/// The span axis of a full-chord contact pcurve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    /// The chord has constant `u`; its signed function is `u − c`.
    U,
    /// The chord has constant `v`; its signed function is `v − c`.
    V,
}

/// One validated contact chord of the arrangement.
#[derive(Debug, Clone, Copy)]
struct Chord {
    /// The pcurve's index into `FaceConsumption::contact_pcurves`.
    index: usize,
    /// The chord's span axis.
    axis: Axis,
    /// The chord's parameter coordinate on that axis.
    coord: f64,
    /// The chord's blend side (the removed side).
    blend: BlendSide,
}

/// Classify every cell of the trim arrangement of a face consumption.
///
/// The arrangement of the contact pcurves inside the support chart is built
/// (the chord-coordinate product decomposition of the certified v1 envelope),
/// and each cell is marked removed exactly when the interval evaluation of the
/// signed side data certifies it on the blend side of a contact chord. An
/// undecided cell — an evaluation whose signed interval straddles the chord —
/// is refused [`ConstructRefusal::ConditioningBelowThreshold`], never guessed.
/// Malformed consumption inputs are refused [`ConstructRefusal::InvalidInput`].
pub fn classify_cells(fc: &FaceConsumption) -> Result<Vec<TrimCell>, ConstructRefusal> {
    let plan = build_plan(fc)?;
    let mut cells = Vec::with_capacity(plan.cells.len());
    for cell in &plan.cells {
        let removed = is_removed(cell, &plan.chords)?;
        cells.push(TrimCell {
            cell: *cell,
            removed,
        });
    }
    Ok(cells)
}

/// Consume one support face: `F_i_new = F_i \ R_i` as an outcome of the trim
/// arrangement.
///
/// The arrangement cells are classified by the signed side data (see
/// [`classify_cells`]); the cells inside the removed region `R_i` are
/// discarded, and the face survives with the remaining cells — each recording
/// the contact pcurve and side that bound it — or vanishes when no cell
/// remains. An undecided cell refuses [`ConstructRefusal::ConditioningBelowThreshold`];
/// malformed inputs refuse [`ConstructRefusal::InvalidInput`].
pub fn consume_face(fc: &FaceConsumption) -> Result<FaceOutcome, ConstructRefusal> {
    let plan = build_plan(fc)?;
    let mut retained = Vec::new();
    for cell in &plan.cells {
        if is_removed(cell, &plan.chords)? {
            continue;
        }
        let bounds = surviving_bounds(cell, &plan, fc)?;
        retained.push(RetainedCell {
            cell: *cell,
            bounds,
        });
    }
    if retained.is_empty() {
        Ok(FaceOutcome::Vanished)
    } else {
        Ok(FaceOutcome::Survived { retained })
    }
}

/// Build the validated arrangement plan of a face consumption.
fn build_plan(fc: &FaceConsumption) -> Result<Plan, ConstructRefusal> {
    let domain = fc.support.domain;
    let ((u0, u1), (v0, v1)) = domain;
    if !(u0.is_finite() && u1.is_finite() && v0.is_finite() && v1.is_finite()) {
        return Err(ConstructRefusal::InvalidInput);
    }
    if !(u0 < u1 && v0 < v1) {
        return Err(ConstructRefusal::InvalidInput);
    }
    if fc.contact_pcurves.len() != fc.trim_provenance.len() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let mut chords = Vec::with_capacity(fc.contact_pcurves.len());
    for (index, pcurve) in fc.contact_pcurves.iter().enumerate() {
        chords.push(chord_of(*pcurve, domain, index)?);
    }
    let u_cuts = coordinate_cuts(u0, u1, &chords, Axis::U);
    let v_cuts = coordinate_cuts(v0, v1, &chords, Axis::V);
    let cells = product_cells(&u_cuts, &v_cuts)?;
    Ok(Plan { chords, cells })
}

/// Validate one contact pcurve as a full chord of the chart, refusing
/// anything outside the certified v1 envelope.
fn chord_of(
    pcurve: ContactPcurve,
    domain: SurfaceRegion,
    index: usize,
) -> Result<Chord, ConstructRefusal> {
    let ((u0, u1), (v0, v1)) = domain;
    let (fu, fv) = (pcurve.from[0], pcurve.from[1]);
    let (tu, tv) = (pcurve.to[0], pcurve.to[1]);
    if !(fu.is_finite() && fv.is_finite() && tu.is_finite() && tv.is_finite()) {
        return Err(ConstructRefusal::InvalidInput);
    }
    if fu == tu {
        // A vertical chord `u = const`, spanning the chart's whole `v` span.
        let coord = fu;
        if !(coord > u0 && coord < u1) {
            return Err(ConstructRefusal::InvalidInput);
        }
        let lo = fv.min(tv);
        let hi = fv.max(tv);
        if lo == v0 && hi == v1 {
            Ok(Chord {
                index,
                axis: Axis::U,
                coord,
                blend: pcurve.blend_side,
            })
        } else {
            Err(ConstructRefusal::InvalidInput)
        }
    } else if fv == tv {
        // A horizontal chord `v = const`, spanning the chart's whole `u` span.
        let coord = fv;
        if !(coord > v0 && coord < v1) {
            return Err(ConstructRefusal::InvalidInput);
        }
        let lo = fu.min(tu);
        let hi = fu.max(tu);
        if lo == u0 && hi == u1 {
            Ok(Chord {
                index,
                axis: Axis::V,
                coord,
                blend: pcurve.blend_side,
            })
        } else {
            Err(ConstructRefusal::InvalidInput)
        }
    } else {
        Err(ConstructRefusal::InvalidInput)
    }
}

/// The sorted, deduplicated cut coordinates along one chart axis: the two
/// domain bounds plus every chord coordinate on that axis. The boundary and
/// chord coordinates are exact `f64` values, so deduplication is exact.
fn coordinate_cuts(lo: f64, hi: f64, chords: &[Chord], axis: Axis) -> Vec<f64> {
    let mut cuts = vec![lo, hi];
    for chord in chords {
        if chord.axis == axis {
            cuts.push(chord.coord);
        }
    }
    cuts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = Vec::with_capacity(cuts.len());
    for value in cuts {
        let duplicate = out.last().map_or(false, |last| *last == value);
        if !duplicate {
            out.push(value);
        }
    }
    out
}

/// The product cells of the chord-coordinate cuts, in the fixed enumeration
/// order: `v` bands from low to high, `u` slabs from low to high within each
/// band.
fn product_cells(u_cuts: &[f64], v_cuts: &[f64]) -> Result<Vec<ParamBox>, ConstructRefusal> {
    if u_cuts.len() < 2 || v_cuts.len() < 2 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let mut cells = Vec::new();
    for v in v_cuts.windows(2) {
        for u in u_cuts.windows(2) {
            cells.push(ParamBox {
                u: (u[0], u[1]),
                v: (v[0], v[1]),
            });
        }
    }
    Ok(cells)
}

/// Whether a cell lies inside the removed region `R_i`: its certified side is
/// the blend side of at least one contact chord.
fn is_removed(cell: &ParamBox, chords: &[Chord]) -> Result<bool, ConstructRefusal> {
    for chord in chords {
        if side_at(chord, cell)? == chord.blend.trim_side() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The certified side of a cell relative to one contact chord.
///
/// The signed function `u − c` (or `v − c`) is evaluated in `Interval`
/// arithmetic over a deterministic witness box in the cell's interior. A
/// strictly negative enclosure is the [`TrimSide::Lower`] side, a strictly
/// positive enclosure the [`TrimSide::Upper`] side; an enclosure straddling
/// the chord is an undecided cell and refuses, never a guessed label.
fn side_at(chord: &Chord, cell: &ParamBox) -> Result<TrimSide, ConstructRefusal> {
    let span = match chord.axis {
        Axis::U => witness_axis(cell.u.0, cell.u.1)?,
        Axis::V => witness_axis(cell.v.0, cell.v.1)?,
    };
    let signed = span.sub(&Interval::point(chord.coord));
    if signed.hi < 0.0 {
        Ok(TrimSide::Lower)
    } else if signed.lo > 0.0 {
        Ok(TrimSide::Upper)
    } else {
        Err(ConstructRefusal::ConditioningBelowThreshold)
    }
}

/// The certified witness box interval along one axis: the middle fraction
/// `1 − 2·CELL_WITNESS_FRACTION` of the axis span, strictly inside the cell.
///
/// Computed in a fixed order over the exact cell endpoints; a cell too thin to
/// admit a positive-width witness is undecidable and refuses
/// [`ConstructRefusal::ConditioningBelowThreshold`].
fn witness_axis(lo: f64, hi: f64) -> Result<Interval, ConstructRefusal> {
    if !(lo.is_finite() && hi.is_finite() && lo < hi) {
        return Err(ConstructRefusal::InvalidInput);
    }
    let margin = (hi - lo) * CELL_WITNESS_FRACTION;
    let inner_lo = lo + margin;
    let inner_hi = hi - margin;
    if inner_lo < inner_hi {
        Ok(Interval {
            lo: inner_lo,
            hi: inner_hi,
        })
    } else {
        Err(ConstructRefusal::ConditioningBelowThreshold)
    }
}

/// The trim bounds a surviving cell records: every contact chord sharing an
/// edge of the cell, each with the side the cell lies on and the chord's
/// landed provenance reference.
fn surviving_bounds(
    cell: &ParamBox,
    plan: &Plan,
    fc: &FaceConsumption,
) -> Result<Vec<TrimBound>, ConstructRefusal> {
    let mut bounds = Vec::new();
    for chord in &plan.chords {
        let adjacent = match chord.axis {
            Axis::U => cell.u.0 == chord.coord || cell.u.1 == chord.coord,
            Axis::V => cell.v.0 == chord.coord || cell.v.1 == chord.coord,
        };
        if !adjacent {
            continue;
        }
        let side = side_at(chord, cell)?;
        let source = match fc.trim_provenance.get(chord.index) {
            Some(source) => *source,
            None => return Err(ConstructRefusal::InvalidInput),
        };
        bounds.push(TrimBound {
            curve: chord.index,
            side,
            source,
        });
    }
    Ok(bounds)
}
