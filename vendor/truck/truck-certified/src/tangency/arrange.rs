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

//! The T2 carrier arrangement into atoms (CTE-007-T2ARRANGE; theory §5.2,
//! §5.6, §5.8, §5.9).
//!
//! Given a certified common-carrier coincidence witness (theory §4.4,
//! [`CoincidenceWitness`]) whose two trim domains lie in the carrier chart,
//! and the tangential contact curves incident to the carrier (CTE-005's
//! [`A2BranchCurve`] output — the structural T1 → T2 link), this module
//! builds the certified carrier arrangement:
//!
//! - the trim-domain boundary curves of the two operands AND the tangential
//!   contact curves are the arrangement **strata**;
//! - the 2-D open cells of the stratum arrangement are the **atoms** on which
//!   the operands' two-bit side signatures are constant (H-atom, §5.2). A
//!   tangential curve must be a stratum precisely because the side bits can
//!   change across it; omitting it would break constancy on the atom.
//!
//! **Scope of this module (pre-made decision, not relitigated).** The
//! arrangement realises the certified *combinatorics* of the atoms over the
//! recorded carrier data: trim domains are the axis-aligned parameter boxes
//! the frozen shapes carry, and a tangential stratum is admitted only when its
//! certified samples trace a chart-straight (axis-aligned) line — the F5 A₂
//! branch `u1 = 0` class. Anything else refuses with a named refusal rather
//! than guessing a curved-cell decomposition. Full general-curve arrangement
//! is the §6.3 open tail.
//!
//! The decision on each atom is a function of the signature (T2.4, §5.5):
//! [`atom_decision`] maps the two operands' side signatures through the §5.4
//! coordinatewise algebra and the §5.5 three-valued extraction
//! `{Drop, Keep{canonical}, Keep{flipped}}` — never a policy. The side
//! signatures themselves are NOT re-derived here by a new membership
//! predicate: an atom's operand signature is the classifier-produced side of
//! the operand's trim when the atom lies inside that trim, and the exterior
//! state `00` otherwise (§5.3 implementation note). Emission (§5.5 corollary)
//! emits ONE canonical record plus its provenance set for every kept atom —
//! never two coincident faces.
//!
//! **H-atom admission (scope decision 1).** Every atom is checked at
//! admission: its representative chart point sits off every operand trim
//! boundary and every tangential stratum, and the certified `ε₀` range keeps
//! the probe points `p ± ε·n_C` off those boundaries for every
//! `0 < ε < ε₀`. Because the trim boundaries are dyadic cut lines, the
//! clearance is dyadic and `ε₀` is recorded exactly as half the minimum cell
//! clearance. A subdivision that cannot certify a cell (a degenerate zero-
//! width cell) refuses by name — it never guesses.
//!
//! **House rules.** No `unwrap`/`expect`/`panic!`/`todo!` and no
//! out-of-range indexing reachable from geometry (H-1); every fallible path
//! returns the named [`TangencyRefusal`] (H-2); no absolute constants in
//! predicates — the only slacks are width-relative (H-3); float-computed
//! values are never recorded as exact (H-6); fixed stratum order and fixed
//! enumeration make every output deterministic.

use crate::contract::Refusal;
use crate::tangency::shapes::{A2BranchCurve, CarrierStratum, CoincidenceWitness, SideState};
use crate::tangency::TangencyRefusal;

/// The horizontal or vertical cut coordinates the arrangement refines the
/// chart domain into, split per axis.
#[derive(Debug, Clone)]
struct Cuts {
    /// The ascending distinct x cut coordinates.
    xs: Vec<f64>,
    /// The ascending distinct y cut coordinates.
    ys: Vec<f64>,
}

/// One primitive axis-aligned chart cell `(x0, x1) x (y0, y1)` of the cut
/// grid, identified by its column/row indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CellRef {
    /// The column index into `Cuts::xs` (the interval `xs[i]..xs[i+1]`).
    col: usize,
    /// The row index into `Cuts::ys` (the interval `ys[j]..ys[j+1]`).
    row: usize,
}

/// The constant-on-a-cell signature of one primitive cell: which operand trim
/// domains the cell is inside, and which side of every tangential stratum.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CellSig {
    /// The cell's interior lies inside trim A.
    in_a: bool,
    /// The cell's interior lies inside trim B.
    in_b: bool,
    /// The index of the vertical tangential stratum the cell lies to the
    /// RIGHT of (`0` when the cell is left of the first, etc.).
    v_side: usize,
    /// The index of the horizontal tangential stratum the cell lies ABOVE.
    h_side: usize,
}

/// One axis-aligned cell of an atom's geometry (a dyadic rectangle in the
/// carrier chart).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// The `x` interval `(lo, hi)`.
    pub x: (f64, f64),
    /// The `y` interval `(lo, hi)`.
    pub y: (f64, f64),
}

/// An atom of the carrier arrangement (theory §5.2): a maximal connected
/// 2-D open cell of the stratum arrangement, on which the operands' side
/// signatures are constant (H-atom).
///
/// An atom carries its cells (the primitive dyadic rectangles that make up
/// its region — the dyadic-exact geometry of the canonical emission record),
/// its membership of the two trim domains, and its certified probe clearance
/// `ε₀` (scope decision 1): for the atom's representative chart point `p`,
/// the probe points `p ± ε·n_C` are off every operand boundary for every
/// `0 < ε < ε₀`.
#[derive(Debug, Clone, PartialEq)]
pub struct Atom {
    /// The primitive cells of the atom, in deterministic row-major order.
    cells: Vec<Cell>,
    /// The representative chart point `p` of the atom (its first cell's
    /// centre), used by the probe admission.
    rep: (f64, f64),
    /// The certified probe range bound `ε₀ > 0` (half the minimum cell
    /// clearance to the arrangement's cut lines).
    epsilon0: f64,
    /// Whether the atom's interior lies inside operand A's trim.
    in_a: bool,
    /// Whether the atom's interior lies inside operand B's trim.
    in_b: bool,
    /// Operand A's constant side signature on the atom (§5.3): the trim's
    /// recorded side when `in_a`, the exterior state `00` otherwise.
    side_a: SideState,
    /// Operand B's constant side signature on the atom (§5.3).
    side_b: SideState,
}

impl Atom {
    /// The atom's cells, in deterministic order.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// The representative chart point `p`.
    pub fn rep(&self) -> (f64, f64) {
        self.rep
    }

    /// The certified probe range bound `ε₀ > 0` (H-atom admission).
    pub fn epsilon0(&self) -> f64 {
        self.epsilon0
    }

    /// Whether the atom lies inside operand A's trim.
    pub fn in_a(&self) -> bool {
        self.in_a
    }

    /// Whether the atom lies inside operand B's trim.
    pub fn in_b(&self) -> bool {
        self.in_b
    }

    /// Operand A's constant side signature on the atom.
    pub fn side_a(&self) -> SideState {
        self.side_a
    }

    /// Operand B's constant side signature on the atom.
    pub fn side_b(&self) -> SideState {
        self.side_b
    }
}

/// The certified carrier arrangement of a coincidence witness (theory §5.2):
/// the atoms (the 2-D open cells of the stratum arrangement) and the ordered
/// stratum inventory.
#[derive(Debug, Clone, PartialEq)]
pub struct CarrierArrangement {
    /// The atoms, in deterministic order (by the atom's first cell, row-major).
    pub atoms: Vec<Atom>,
    /// The ordered stratum inventory: the two operands' trim-boundary strata
    /// then the admitted tangential contact curves.
    pub strata: Vec<CarrierStratum>,
}

impl CarrierArrangement {
    /// The atoms, verbatim.
    pub fn atoms(&self) -> &[Atom] {
        &self.atoms
    }

    /// The ordered stratum inventory, verbatim.
    pub fn strata(&self) -> &[CarrierStratum] {
        &self.strata
    }
}

/// The coordinatewise Boolean operation over two operands' side signatures
/// (theory §5.4), in the certified T2 vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarrierOp {
    /// Regularised union `σ_A ∨ σ_B`.
    Union,
    /// Regularised intersection `σ_A ∧ σ_B`.
    Intersection,
    /// Regularised difference `σ_A ∧ ¬σ_B` (A minus B).
    Difference,
}

/// The §5.5 three-valued decision on an atom (corollary T2.4): the geometry is
/// the shared carrier itself, so there is no A-versus-B choice — only whether
/// the atom survives on the result's boundary and, if so, with which
/// orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomDecision {
    /// Both result sides carry the same material: the atom is not on the
    /// result's boundary (state `00` or `11`).
    Drop,
    /// The atom is on the result's boundary.
    Keep {
        /// Whether the atom's canonical orientation must be reversed so the
        /// normal points toward the empty (`m_R = 0`) side.
        flip: bool,
    },
}

/// The certified containment of one trim domain in the other (theory §5.6),
/// decided by interval separation of the dyadic trim-boundary parameter boxes.
///
/// Only when the separation is STRICT (the boxes neither touch nor overlap)
/// does the cheap predicate certify; a touching or overlapping pair is the
/// full-arrangement class and reports [`ContainmentVerdict::ArrangementNeeded`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainmentVerdict {
    /// The two trim domains are disjoint (strict interval separation on an
    /// axis).
    Disjoint,
    /// `D_A ⊆ D_B` (strictly, with a positive margin on every axis).
    AInsideB,
    /// `D_B ⊆ D_A` (strictly, with a positive margin on every axis).
    BInsideA,
    /// The trim boundaries touch or overlap: containment is not decided by
    /// interval separation alone and the certified arrangement decides it.
    ArrangementNeeded,
}

/// One emitted canonical geometry record of a kept atom (theory §5.5
/// corollary): the atom emits ONE canonical geometry record plus its
/// provenance set — never two coincident faces. Two coincident operands on an
/// atom collapse to a single record citing both provenance members.
#[derive(Debug, Clone, PartialEq)]
pub struct AtomEmission {
    /// The canonical geometry: the atom's cells on the shared carrier.
    pub cells: Vec<Cell>,
    /// The provenance set: whether operand A and/or operand B had geometry on
    /// this atom.
    pub in_a: bool,
    /// The provenance set: operand B membership.
    pub in_b: bool,
    /// Whether the canonical orientation must be reversed.
    pub flip: bool,
}

/// The width-relative slack deciding whether a set of tangential samples is
/// chart-straight (dimensionless, so no absolute length constant — H-3).
fn straight_slack(width: f64) -> f64 {
    width * 1.0e-9
}

/// Build the certified carrier arrangement of a coincidence witness (theory
/// §5.2; spine §2: "production lands in CTE-007").
///
/// The atoms are the connected 2-D open cells of the arrangement of the two
/// operands' trim-domain boundaries and the tangential contact curves; every
/// atom is H-atom admitted at construction. A refusal is always a named
/// [`TangencyRefusal`].
pub fn arrange_carrier<P>(
    witness: &CoincidenceWitness<P>,
    tangential: &[A2BranchCurve],
) -> Result<CarrierArrangement, TangencyRefusal> {
    let a_dom = witness.a().domain();
    let b_dom = witness.b().domain();
    let chart = witness.carrier().chart().domain();
    if !domain_ok(a_dom) || !domain_ok(b_dom) || !domain_ok(chart) {
        return Err(invalid());
    }
    if !domains_within(a_dom, chart) || !domains_within(b_dom, chart) {
        return Err(invalid());
    }

    let (width, height) = (chart[0].1 - chart[0].0, chart[1].1 - chart[1].0);
    if !width.is_finite() || width <= 0.0 || !height.is_finite() || height <= 0.0 {
        return Err(invalid());
    }

    // Collect the tangential strata, admitting only chart-straight
    // (axis-aligned) contact curves (the F5 `u1 = 0` class).
    let mut vertical_lines: Vec<f64> = Vec::new();
    let mut horizontal_lines: Vec<f64> = Vec::new();
    let mut strata: Vec<CarrierStratum> = Vec::new();
    strata.push(CarrierStratum::TrimBoundary {
        operand: witness.a().operand(),
    });
    strata.push(CarrierStratum::TrimBoundary {
        operand: witness.b().operand(),
    });
    for curve in tangential {
        let (v_x, v_y) = chart_straight_line(curve, width, height).ok_or_else(invalid)?;
        match (v_x, v_y) {
            (Some(x), None) => {
                if !vertical_lines.contains(&x) {
                    vertical_lines.push(x);
                }
            }
            (None, Some(y)) => {
                if !horizontal_lines.contains(&y) {
                    horizontal_lines.push(y);
                }
            }
            _ => return Err(invalid()),
        }
        strata.push(CarrierStratum::TangentialContact(curve.clone()));
    }

    // The sorted cut lines per axis: the chart bounds, the trim-domain
    // boundary coordinates, and the tangential stratum lines.
    let mut xs = vec![chart[0].0, chart[0].1];
    let mut ys = vec![chart[1].0, chart[1].1];
    for x in [a_dom[0].0, a_dom[0].1, b_dom[0].0, b_dom[0].1] {
        push_unique(&mut xs, x, straight_slack(width));
    }
    for y in [a_dom[1].0, a_dom[1].1, b_dom[1].0, b_dom[1].1] {
        push_unique(&mut ys, y, straight_slack(height));
    }
    for x in &vertical_lines {
        push_unique(&mut xs, *x, straight_slack(width));
    }
    for y in &horizontal_lines {
        push_unique(&mut ys, *y, straight_slack(height));
    }
    vertical_lines.sort_by(total_cmp);
    horizontal_lines.sort_by(total_cmp);
    if xs.len() < 2 || ys.len() < 2 {
        return Err(invalid());
    }
    let cuts = Cuts { xs, ys };

    // Label every primitive cell and merge adjacent same-signature cells into
    // the atoms (the maximal connected cells of the arrangement).
    let cells = primitive_cells(witness, &cuts, &vertical_lines, &horizontal_lines)?;
    let atoms = merge_atoms(&cells, &cuts, a_dom, b_dom, witness)?;

    Ok(CarrierArrangement { atoms, strata })
}

/// Whether a trim domain box lies inside the carrier chart box.
fn domains_within(domain: [(f64, f64); 2], chart: [(f64, f64); 2]) -> bool {
    chart[0].0 <= domain[0].0
        && domain[0].1 <= chart[0].1
        && chart[1].0 <= domain[1].0
        && domain[1].1 <= chart[1].1
}

/// Whether `(lo, hi)` is a valid finite ascending interval.
fn interval_ok((lo, hi): (f64, f64)) -> bool {
    lo.is_finite() && hi.is_finite() && lo <= hi
}

/// Whether a two-axis domain is two valid finite ascending intervals.
fn domain_ok(domain: [(f64, f64); 2]) -> bool {
    interval_ok(domain[0]) && interval_ok(domain[1])
}

/// Whether a trim box is non-degenerate (strictly positive width and height).
fn non_degenerate(domain: [(f64, f64); 2]) -> bool {
    domain[0].0 < domain[0].1 && domain[1].0 < domain[1].1
}

/// Pushes `v` into the ascending list unless an entry is equal within the
/// width-relative slack.
fn push_unique(list: &mut Vec<f64>, v: f64, slack: f64) {
    if list.iter().any(|u| (u - v).abs() <= slack) {
        return;
    }
    list.push(v);
    list.sort_by(total_cmp);
}

/// The deterministic total order over finite floats.
fn total_cmp(a: &f64, b: &f64) -> std::cmp::Ordering {
    a.total_cmp(b)
}

/// The chart-straight line a tangential curve traces, when it is one: the
/// curve is admitted iff every sample's first two coordinates (the carrier
/// chart axes) are constant on one axis within the width-relative slack.
///
/// Returns `(Some(x), None)` for the vertical line `x = const` and
/// `(None, Some(y))` for the horizontal line `y = const`.
fn chart_straight_line(
    curve: &A2BranchCurve,
    width: f64,
    height: f64,
) -> Option<(Option<f64>, Option<f64>)> {
    let x = curve.samples().first()?.point()[0];
    let y = curve.samples().first()?.point()[1];
    let x_const = curve
        .samples()
        .iter()
        .all(|s| (s.point()[0] - x).abs() <= straight_slack(width));
    let y_const = curve
        .samples()
        .iter()
        .all(|s| (s.point()[1] - y).abs() <= straight_slack(height));
    if x_const && !y_const {
        Some((Some(x), None))
    } else if y_const && !x_const {
        Some((None, Some(y)))
    } else {
        // Both constant (a point curve) or neither (a genuinely curved or
        // oblique trace): not an axis-aligned stratum this module realises.
        None
    }
}

/// One primitive grid cell with its signature and its centre.
#[derive(Debug, Clone)]
struct Primitive {
    /// The grid reference.
    cell: CellRef,
    /// The constant-on-cell signature.
    sig: CellSig,
    /// The interval pair of the cell.
    x: (f64, f64),
    /// The vertical interval pair of the cell.
    y: (f64, f64),
}

/// Labels every primitive cell of the cut grid with its signature.
fn primitive_cells<P>(
    witness: &CoincidenceWitness<P>,
    cuts: &Cuts,
    vertical_lines: &[f64],
    horizontal_lines: &[f64],
) -> Result<Vec<Primitive>, TangencyRefusal> {
    let a_dom = witness.a().domain();
    let b_dom = witness.b().domain();
    let n_cols = cuts.xs.len() - 1;
    let n_rows = cuts.ys.len() - 1;
    let mut out = Vec::with_capacity(n_cols * n_rows);
    for row in 0..n_rows {
        let (y0, y1) = (cuts.ys[row], cuts.ys[row + 1]);
        let y_mid = (y0 + y1) * 0.5;
        if !y_mid.is_finite() {
            return Err(invalid());
        }
        for col in 0..n_cols {
            let (x0, x1) = (cuts.xs[col], cuts.xs[col + 1]);
            let x_mid = (x0 + x1) * 0.5;
            if !x_mid.is_finite() {
                return Err(invalid());
            }
            let in_a = inside_box(a_dom, x_mid, y_mid);
            let in_b = inside_box(b_dom, x_mid, y_mid);
            let v_side = vertical_lines.iter().filter(|t| **t < x_mid).count();
            let h_side = horizontal_lines.iter().filter(|t| **t < y_mid).count();
            out.push(Primitive {
                cell: CellRef { col, row },
                sig: CellSig {
                    in_a,
                    in_b,
                    v_side,
                    h_side,
                },
                x: (x0, x1),
                y: (y0, y1),
            });
        }
    }
    Ok(out)
}

/// Whether the midpoint lies strictly inside the axis-aligned box.
fn inside_box(domain: [(f64, f64); 2], x_mid: f64, y_mid: f64) -> bool {
    domain[0].0 < x_mid && x_mid < domain[0].1 && domain[1].0 < y_mid && y_mid < domain[1].1
}

/// Merges adjacent same-signature primitive cells into the atoms and admits
/// each atom (H-atom probe admission).
fn merge_atoms<P>(
    cells: &[Primitive],
    cuts: &Cuts,
    a_dom: [(f64, f64); 2],
    b_dom: [(f64, f64); 2],
    witness: &CoincidenceWitness<P>,
) -> Result<Vec<Atom>, TangencyRefusal> {
    if !non_degenerate(a_dom) || !non_degenerate(b_dom) {
        return Err(invalid());
    }
    let n_cols = cuts.xs.len() - 1;
    let mut parent: Vec<usize> = (0..cells.len()).collect();
    for i in 0..cells.len() {
        for j in (i + 1)..cells.len() {
            if cells[i].sig != cells[j].sig {
                continue;
            }
            if adjacent(&cells[i], &cells[j], n_cols) {
                union(&mut parent, i, j);
            }
        }
    }
    let mut roots: Vec<usize> = Vec::new();
    for i in 0..cells.len() {
        let r = find(&mut parent, i);
        if !roots.contains(&r) {
            roots.push(r);
        }
    }
    roots.sort_by(total_cmp_usize);

    let mut atoms = Vec::with_capacity(roots.len());
    for root in roots {
        let mut members: Vec<usize> = Vec::new();
        for i in 0..cells.len() {
            if find(&mut parent, i) == root {
                members.push(i);
            }
        }
        members.sort_by(total_cmp_usize);
        // The representative primitive is the first in row-major order.
        let rep = members.first().copied().ok_or_else(invalid)?;
        let rep_sig = &cells[rep].sig;
        let mut atom_cells: Vec<Cell> = Vec::with_capacity(members.len());
        let mut min_clearance = f64::INFINITY;
        for &m in &members {
            atom_cells.push(Cell {
                x: cells[m].x,
                y: cells[m].y,
            });
            let centre = (
                (cells[m].x.0 + cells[m].x.1) * 0.5,
                (cells[m].y.0 + cells[m].y.1) * 0.5,
            );
            min_clearance = min_clearance.min(line_clearance(centre, cuts));
        }
        if !min_clearance.is_finite() || min_clearance <= 0.0 {
            return Err(invalid());
        }
        let epsilon0 = min_clearance * 0.5;
        let rep_centre = (
            (cells[rep].x.0 + cells[rep].x.1) * 0.5,
            (cells[rep].y.0 + cells[rep].y.1) * 0.5,
        );
        let atom = Atom {
            cells: atom_cells,
            rep: rep_centre,
            epsilon0,
            in_a: rep_sig.in_a,
            in_b: rep_sig.in_b,
            side_a: operand_side(witness.a().side(), rep_sig.in_a)?,
            side_b: operand_side(witness.b().side(), rep_sig.in_b)?,
        };
        // H-atom admission (scope decision 1): the representative is strictly
        // interior — at least `2·ε₀` from every cut line — so the probes
        // `p ± ε·n_C` for every `0 < ε < ε₀` stay off every operand trim
        // boundary and every tangential stratum.
        let d = line_clearance(atom.rep, cuts);
        if d < 2.0 * epsilon0 {
            return Err(invalid());
        }
        atoms.push(atom);
    }
    Ok(atoms)
}

/// Whether two primitive cells share a full edge segment.
fn adjacent(a: &Primitive, b: &Primitive, n_cols: usize) -> bool {
    let (ca, cb) = (a.cell, b.cell);
    let same_row = ca.row == cb.row;
    let same_col = ca.col == cb.col;
    if same_row {
        return ca.col.abs_diff(cb.col) == 1;
    }
    if same_col {
        return ca.row.abs_diff(cb.row) == 1;
    }
    let _ = n_cols;
    false
}

/// The union-find root of `i` (path halving).
fn find(parent: &mut [usize], i: usize) -> usize {
    let mut r = i;
    while parent[r] != r {
        r = parent[r];
    }
    let mut cur = i;
    while parent[cur] != cur {
        let next = parent[cur];
        parent[cur] = r;
        cur = next;
    }
    r
}

/// Union-find union of `i` and `j`.
fn union(parent: &mut [usize], i: usize, j: usize) {
    let (ri, rj) = (find(parent, i), find(parent, j));
    if ri != rj {
        parent[ri] = rj;
    }
}

/// Deterministic order for the union-find roots.
fn total_cmp_usize(a: &usize, b: &usize) -> std::cmp::Ordering {
    a.cmp(b)
}

/// The distance from a chart point to the nearest cut line (any row or column
/// boundary) of the arrangement.
fn line_clearance(p: (f64, f64), cuts: &Cuts) -> f64 {
    let mut d = f64::INFINITY;
    for x in &cuts.xs {
        d = d.min((p.0 - x).abs());
    }
    for y in &cuts.ys {
        d = d.min((p.1 - y).abs());
    }
    d
}

/// The operand's constant side signature on an atom: the recorded trim side
/// inside the trim, the exterior state `00` outside it (the classifier's own
/// data — no new membership predicate).
fn operand_side(recorded: SideState, inside: bool) -> Result<SideState, TangencyRefusal> {
    if inside {
        return Ok(recorded);
    }
    // `SideState::new(0)` is the exterior state `00`; it is in range by
    // construction, so the refusal branch is defensive only.
    match SideState::new(0) {
        Ok(state) => Ok(state),
        Err(_) => Err(invalid()),
    }
}

/// The two operands' side signatures on an atom (theory §5.3). This is the
/// `(σ_A, σ_B)` pair the T2 decision is a function of (T2.4); it is derived
/// from the classifier's recorded trim sides, never by a new predicate.
pub fn side_signature(atom: &Atom) -> (SideState, SideState) {
    (atom.side_a, atom.side_b)
}

/// The §5.5 boundary extraction of a result side signature `σ_R`:
/// `00`/`11` drop, `10` keeps canonical, `01` keeps flipped.
fn extract(sigma_minus: bool, sigma_plus: bool) -> AtomDecision {
    if sigma_minus == sigma_plus {
        AtomDecision::Drop
    } else {
        AtomDecision::Keep { flip: !sigma_minus }
    }
}

/// The §5.4 coordinatewise combine of the two operands' side signatures under
/// an operation, evaluated per bit (no new membership predicate).
fn combine_minus_plus(op: CarrierOp, sa: SideState, sb: SideState) -> (bool, bool) {
    let (a_minus, a_plus) = (sa.minus(), sa.plus());
    let (b_minus, b_plus) = (sb.minus(), sb.plus());
    match op {
        CarrierOp::Union => (a_minus || b_minus, a_plus || b_plus),
        CarrierOp::Intersection => (a_minus && b_minus, a_plus && b_plus),
        CarrierOp::Difference => (a_minus && !b_minus, a_plus && !b_plus),
    }
}

/// The T2 decision on an atom (theory §5.4–§5.5, T2.4): a function of the
/// signature, never a policy. `Drop`, `Keep{canonical}` and `Keep{flipped}`
/// are the only answers.
pub fn atom_decision(sa: SideState, sb: SideState, op: CarrierOp) -> AtomDecision {
    let (r_minus, r_plus) = combine_minus_plus(op, sa, sb);
    extract(r_minus, r_plus)
}

/// The certified containment of the witness's two trim domains (theory §5.6),
/// decided cheaply by interval separation of the dyadic trim-boundary
/// parameter boxes. Strict separation certifies; a touching or overlapping
/// pair reports [`ContainmentVerdict::ArrangementNeeded`] — containment
/// decides WHERE the bit operations evaluate, never WHAT they return.
pub fn certify_containment<P>(witness: &CoincidenceWitness<P>) -> ContainmentVerdict {
    let a = witness.a().domain();
    let b = witness.b().domain();
    // Strict disjointness: an axis separates the two closed boxes.
    let separated_x = a[0].1 <= b[0].0 || b[0].1 <= a[0].0;
    let separated_y = a[1].1 <= b[1].0 || b[1].1 <= a[1].0;
    if separated_x || separated_y {
        return ContainmentVerdict::Disjoint;
    }
    // Strict interior containment on every axis.
    let a_inside_b = b[0].0 < a[0].0 && a[0].1 < b[0].1 && b[1].0 < a[1].0 && a[1].1 < b[1].1;
    let b_inside_a = a[0].0 < b[0].0 && b[0].1 < a[0].1 && a[1].0 < b[1].0 && b[1].1 < a[1].1;
    if a_inside_b {
        ContainmentVerdict::AInsideB
    } else if b_inside_a {
        ContainmentVerdict::BInsideA
    } else {
        ContainmentVerdict::ArrangementNeeded
    }
}

/// The §5.5-corollary emission of the kept atoms of an operation: every kept
/// atom emits ONE canonical geometry record (its cells on the shared carrier)
/// plus its provenance set — never two coincident faces. Two coincident
/// operands on one atom collapse to a single record citing both.
pub fn emit_kept(atoms: &[Atom], op: CarrierOp) -> Vec<AtomEmission> {
    let mut out = Vec::new();
    for atom in atoms {
        let (sa, sb) = side_signature(atom);
        match atom_decision(sa, sb, op) {
            AtomDecision::Drop => {}
            AtomDecision::Keep { flip } => out.push(AtomEmission {
                cells: atom.cells.clone(),
                in_a: atom.in_a,
                in_b: atom.in_b,
                flip,
            }),
        }
    }
    out
}

/// The named invalid-input refusal.
fn invalid() -> TangencyRefusal {
    TangencyRefusal::Input(Refusal::InvalidInput)
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. Unit-test assertions on hand-built dyadic fixtures are
// not such a path.
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::tangency::qpoly::QPoly;
    use crate::tangency::shapes::{
        A2BranchSample, CarrierChart, CarrierNormal, CoincidenceCarrier, CoincidenceIdentity,
        CoincidenceWitness,
    };

    /// A carrier normal.
    fn normal(direction: [f64; 3]) -> CarrierNormal {
        CarrierNormal::new(direction).expect("a valid carrier normal")
    }

    /// A carrier chart.
    fn chart(domain: [(f64, f64); 2]) -> CarrierChart {
        CarrierChart::new(domain).expect("a valid carrier chart")
    }

    /// A coincidence witness over the two operand trims.
    fn witness(
        carrier: CoincidenceCarrier,
        a_dom: [(f64, f64); 2],
        a_state: u8,
        b_dom: [(f64, f64); 2],
        b_state: u8,
    ) -> CoincidenceWitness<QPoly> {
        let side_a = SideState::new(a_state).expect("a valid side state");
        let side_b = SideState::new(b_state).expect("a valid side state");
        CoincidenceWitness::new(
            carrier,
            crate::tangency::shapes::CarrierTrimDomain::new(0, a_dom, side_a)
                .expect("a valid A trim"),
            crate::tangency::shapes::CarrierTrimDomain::new(1, b_dom, side_b)
                .expect("a valid B trim"),
            CoincidenceIdentity::ProvenanceIdentical,
        )
        .expect("a valid coincidence witness")
    }

    /// The F7-style coplanar carrier: the plane `z = 0` with the downward
    /// normal, chart `[0, 1]²` in the tests unless overridden.
    fn plane_carrier(domain: [(f64, f64); 2]) -> CoincidenceCarrier {
        CoincidenceCarrier::new(normal([0.0, 0.0, -1.0]), chart(domain))
            .expect("a valid coplanar carrier")
    }

    /// The identical coplanar cap pair (F7-style): both caps on the same
    /// carrier with the same outward orientation.
    fn identical_caps() -> CoincidenceWitness<QPoly> {
        witness(
            plane_carrier([(0.0, 1.0), (0.0, 1.0)]),
            [(0.0, 1.0), (0.0, 1.0)],
            0b10,
            [(0.0, 1.0), (0.0, 1.0)],
            0b10,
        )
    }

    /// The anti-oriented coplanar caps (the butt-join seam, F7-style).
    fn anti_caps() -> CoincidenceWitness<QPoly> {
        witness(
            plane_carrier([(0.0, 1.0), (0.0, 1.0)]),
            [(0.0, 1.0), (0.0, 1.0)],
            0b10,
            [(0.0, 1.0), (0.0, 1.0)],
            0b01,
        )
    }

    /// An F5 A₂ branch curve along the vertical chart line `x = 0`, in the
    /// carrier chart `[-1, 1] x [0, 1]` (the F5 reduced contact is
    /// `u1² = 0`, whose branch is the `u1 = 0` line).
    fn f5_branch_curve() -> A2BranchCurve {
        let sample = |x: f64, y: f64| {
            A2BranchSample::new([x, y, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0])
                .expect("a valid branch sample")
        };
        A2BranchCurve::new(
            [(-1.0, 1.0), (0.0, 1.0)],
            vec![sample(0.0, 0.25), sample(0.0, 0.5), sample(0.0, 0.75)],
        )
        .expect("a valid F5 branch curve")
    }

    #[test]
    fn tangential_curve_is_stratum_on_f5_carrier() {
        // The F5 A₂ branch curve is a stratum of the carrier arrangement: the
        // carrier chart `[-1, 1] x [0, 1]` with two identical full trim
        // domains is split by the `u1 = 0` branch line into two atoms, and
        // the side signatures are constant across the stratum (H-atom) —
        // omitting the curve would merge the two into one atom and break the
        // H-atom hypothesis exactly where T1's A₂ output feeds T2's
        // arrangement.
        let carrier = plane_carrier([(-1.0, 1.0), (0.0, 1.0)]);
        let w = witness(
            carrier,
            [(-1.0, 1.0), (0.0, 1.0)],
            0b10,
            [(-1.0, 1.0), (0.0, 1.0)],
            0b10,
        );
        let arrangement = arrange_carrier(&w, &[f5_branch_curve()]).expect("F5 arranges");

        // The curve is recorded as a tangential-contact stratum.
        let has_tangent = arrangement
            .strata
            .iter()
            .any(|s| matches!(s, CarrierStratum::TangentialContact(_)));
        assert!(has_tangent, "the F5 branch curve is an arrangement stratum");

        // The branch line splits the domain into two atoms (left and right of
        // the `u1 = 0` line).
        assert_eq!(
            arrangement.atoms.len(),
            2,
            "the F5 branch line splits the carrier into two atoms"
        );
        let (sa_left, sb_left) = side_signature(&arrangement.atoms[0]);
        let (sa_right, sb_right) = side_signature(&arrangement.atoms[1]);
        assert_eq!((sa_left, sb_left), (sa_right, sb_right));
        assert_eq!(sa_left.state(), 0b10);
        assert_eq!(sb_left.state(), 0b10);

        // Every atom's cells lie strictly on one side of the stratum line.
        for atom in &arrangement.atoms {
            for cell in atom.cells() {
                let side = cell.x.0 >= 0.0 || cell.x.1 <= 0.0;
                assert!(side, "no cell straddles the branch line");
            }
        }
    }

    #[test]
    fn h_atom_probe_points_off_boundary() {
        // The H-atom probe admission (scope decision 1) holds on every atom of
        // the F7-style coplanar fixtures at the certified ε₀: the identical
        // cap pair, the anti (butt) cap pair, and a partial-overlap pair. For
        // each atom the representative chart point `p` sits at least `2·ε₀`
        // from every trim boundary and stratum, so `p ± ε·n_C` is off every
        // operand boundary for every `0 < ε < ε₀`.
        let fixtures: Vec<CoincidenceWitness<QPoly>> = vec![
            identical_caps(),
            anti_caps(),
            witness(
                plane_carrier([(0.0, 1.5), (0.0, 1.5)]),
                [(0.0, 1.0), (0.0, 1.0)],
                0b10,
                [(0.5, 1.5), (0.5, 1.5)],
                0b10,
            ),
        ];
        for w in &fixtures {
            let arrangement = arrange_carrier(w, &[]).expect("the coplanar fixture arranges");
            assert!(!arrangement.atoms.is_empty());
            for atom in arrangement.atoms() {
                let epsilon0 = atom.epsilon0();
                assert!(
                    epsilon0.is_finite() && epsilon0 > 0.0,
                    "every atom carries a certified ε₀ > 0"
                );
                // The representative is interior by at least `2·ε₀`: the
                // dyadic cut lines are the only operand/stratum boundaries.
                let cuts = chart_cuts_for(&w);
                let d = line_clearance(atom.rep(), &cuts);
                assert!(
                    d >= 2.0 * epsilon0,
                    "p is off every boundary by more than ε₀ (d = {d}, ε₀ = {epsilon0})"
                );
            }
        }
    }

    /// Rebuilds the arrangement's cut lines for a witness (the probe-distance
    /// audit recomputes the dyadic boundaries the test asserts against).
    fn chart_cuts_for<P>(w: &CoincidenceWitness<P>) -> Cuts {
        let chart = w.carrier().chart().domain();
        let a = w.a().domain();
        let b = w.b().domain();
        let width = chart[0].1 - chart[0].0;
        let height = chart[1].1 - chart[1].0;
        let mut xs = vec![chart[0].0, chart[0].1];
        let mut ys = vec![chart[1].0, chart[1].1];
        for x in [a[0].0, a[0].1, b[0].0, b[0].1] {
            push_unique(&mut xs, x, straight_slack(width));
        }
        for y in [a[1].0, a[1].1, b[1].0, b[1].1] {
            push_unique(&mut ys, y, straight_slack(height));
        }
        Cuts { xs, ys }
    }

    #[test]
    fn containment_certified_without_case_explosion() {
        // §5.6: `D_A ⊆ D_B` reduces to two atoms, and the strict interval
        // separation certifies containment without invoking any full curve
        // arrangement.
        let w = witness(
            plane_carrier([(0.0, 1.0), (0.0, 1.0)]),
            [(0.25, 0.75), (0.25, 0.75)],
            0b10,
            [(0.0, 1.0), (0.0, 1.0)],
            0b10,
        );
        assert_eq!(
            certify_containment(&w),
            ContainmentVerdict::AInsideB,
            "D_A ⊆ D_B certifies by interval separation"
        );

        let arrangement = arrange_carrier(&w, &[]).expect("the contained pair arranges");
        assert_eq!(
            arrangement.atoms.len(),
            2,
            "D_A ⊆ D_B reduces to exactly two atoms (D_A and D_B \\ D_A)"
        );

        // The disjoint pair certifies the other way, and a partial overlap is
        // the arrangement class (not silently guessed).
        let disjoint = witness(
            plane_carrier([(0.0, 2.0), (0.0, 1.0)]),
            [(0.0, 0.5), (0.0, 1.0)],
            0b10,
            [(1.0, 1.5), (0.0, 1.0)],
            0b10,
        );
        assert_eq!(
            certify_containment(&disjoint),
            ContainmentVerdict::Disjoint,
            "separated boxes certify disjoint without the arrangement"
        );

        let overlap = witness(
            plane_carrier([(0.0, 1.5), (0.0, 1.5)]),
            [(0.0, 1.0), (0.0, 1.0)],
            0b10,
            [(0.5, 1.5), (0.5, 1.5)],
            0b10,
        );
        assert_eq!(
            certify_containment(&overlap),
            ContainmentVerdict::ArrangementNeeded,
            "a partial overlap is not decided by interval separation"
        );
    }

    #[test]
    fn atom_emission_is_single_canonical_record() {
        // §5.5 corollary: a kept coincident atom emits ONE canonical geometry
        // record plus its provenance set, never two coincident faces. The
        // identical cap pair's union keeps the shared atom once with both
        // provenance members; the anti (butt) pair's union drops the shared
        // wall (state `11`, certified internal).
        let identical = arrange_carrier(&identical_caps(), &[]).expect("identical pair arranges");
        let emitted = emit_kept(&identical.atoms, CarrierOp::Union);
        assert_eq!(
            emitted.len(),
            1,
            "the identical union emits exactly one canonical record"
        );
        let record = &emitted[0];
        assert!(record.in_a && record.in_b, "the record cites both operands");
        assert!(
            !record.flip,
            "the identical union keeps the canonical orientation"
        );
        assert!(
            !record.cells.is_empty(),
            "the record carries the atom geometry"
        );

        let anti = arrange_carrier(&anti_caps(), &[]).expect("the anti pair arranges");
        let dropped = emit_kept(&anti.atoms, CarrierOp::Union);
        assert!(
            dropped.is_empty(),
            "the anti (butt) union drops the shared wall (10 ∨ 01 = 11)"
        );

        // Difference on the anti pair keeps the wall as A's boundary,
        // unflipped — the §5.9 row `10 − 01 = 10`.
        let diff = emit_kept(&anti.atoms, CarrierOp::Difference);
        assert_eq!(diff.len(), 1);
        assert!(!diff[0].flip);
    }

    #[test]
    fn decision_is_a_function_of_the_signature() {
        // §5.4/§5.5 by exhaustion: `atom_decision` is `Drop` iff the two
        // result sides carry equal material, and a `Keep` flips toward the
        // empty side. Covers the §5.9 derived rows: 10∪10 = 10 keep,
        // 10∪01 = 11 drop, 10∩10 = 10 keep, 10−10 = 00 drop,
        // 10−01 = 10 keep unflipped, 11−10 = 01 keep flipped.
        let s = |bits: u8| SideState::new(bits).expect("a valid state");
        assert_eq!(
            atom_decision(s(0b10), s(0b10), CarrierOp::Union),
            AtomDecision::Keep { flip: false }
        );
        assert_eq!(
            atom_decision(s(0b10), s(0b01), CarrierOp::Union),
            AtomDecision::Drop
        );
        assert_eq!(
            atom_decision(s(0b10), s(0b10), CarrierOp::Intersection),
            AtomDecision::Keep { flip: false }
        );
        assert_eq!(
            atom_decision(s(0b10), s(0b10), CarrierOp::Difference),
            AtomDecision::Drop
        );
        assert_eq!(
            atom_decision(s(0b10), s(0b01), CarrierOp::Difference),
            AtomDecision::Keep { flip: false }
        );
        assert_eq!(
            atom_decision(s(0b11), s(0b10), CarrierOp::Difference),
            AtomDecision::Keep { flip: true }
        );
        assert_eq!(
            atom_decision(s(0b00), s(0b10), CarrierOp::Union),
            AtomDecision::Keep { flip: false }
        );

        // Definitional completeness over the two-bit states.
        for ra in 0u8..4 {
            for rb in 0u8..4 {
                for op in [
                    CarrierOp::Union,
                    CarrierOp::Intersection,
                    CarrierOp::Difference,
                ] {
                    let (sa, sb) = (s(ra), s(rb));
                    let (r_minus, r_plus) = combine_minus_plus(op, sa, sb);
                    match atom_decision(sa, sb, op) {
                        AtomDecision::Drop => assert_eq!(r_minus, r_plus),
                        AtomDecision::Keep { flip } => {
                            assert_ne!(r_minus, r_plus);
                            assert_eq!(flip, !r_minus);
                        }
                    }
                }
            }
        }
    }
}
