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

//! The closed-wire loft as strips (CC-012-LOFT-STRIPS, theory §2.2 L3, spine
//! S8/S9 consumer).
//!
//! A closed-wire loft is **r strips over matched edges**, not one periodic
//! surface: each section curve is split at `r - 1` matched split parameters
//! into `r` arcs (exact knot insertion + cut through the landed nurbs `Cut`
//! operation), and strip `j` is the ordinary CC-010 loft of the `j`-th arc of
//! every section. Adjacent strips share their split vertex data BY
//! CONSTRUCTION IDENTITY (P6): the split value `V_k` at a strip-pair boundary
//! for station `k` is computed exactly once into a deterministic registry and
//! both neighbouring strips consume the registry entry, so their common
//! boundary is the SAME computation and the seam agreement is BITWISE — the
//! only exactness available in the loft pipeline.
//!
//! # Section 1 — identity-bearing split data (P6)
//!
//! The registry is keyed by `EntityId`s built with the landed DAG algebra
//! ([`EntityId::sel`], [`Op::output`]): each strip-pair boundary carries the
//! seam identity
//!
//! ```text
//! Op { kind: OpKind::Loft, params: OpParams::List(section ids) }
//!     .output(&section inputs, slot = strip-pair index)
//! ```
//!
//! and the per-station split vertex is the structural `Pole(k)` sub-entity of
//! that seam. The registry is a `BTreeMap` over the entities' stable FNV-1a
//! content keys (deterministic iteration; never a `HashMap`), storing the
//! homogeneous split value once per identity. Two evaluations of the same
//! split point in different call orders never meet: the value is computed
//! ONCE. Recomputing instead of consuming the registry is the failure the
//! bitwise test exists to catch.
//!
//! # Section 2 — the builder
//!
//! [`loft_closed_wire`] makes the sections compatible (CC-010 exact degree
//! elevation + knot union), validates the split indices against the shared
//! clamped knot vector, builds the r strip lofts through
//! [`loft_sections`](crate::construct::loft::loft_sections) with ONE shared
//! [`factor_banded_tp`](crate::construct::banded::factor_banded_tp)
//! factorization, and certifies the weight field of every delivered strip
//! (CC-011), applying the returned refinements to the shipped strip net. The
//! L3 hypotheses — clamped u-knots per strip, and identical v-stations /
//! degree / v-knots across all strips — are structural here and asserted at
//! build time ([`ConstructRefusal::InvalidInput`] on a violation).
//!
//! # Section 3 — the L3 gate
//!
//! The u-endpoint control row of strip `i` and the u-start control row of
//! strip `i + 1` are BYTE-equal (f64 bit patterns) because they are the same
//! interpolation through the same factor of the same registry values — no
//! tolerance anywhere.
//!
//! # House rules
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use crate::construct::banded::factor_banded_tp;
use crate::construct::config::CC_DEPTH_MAX;
use crate::construct::loft::{loft_collocation_bands, loft_sections, make_compatible, LoftOutput};
use crate::construct::loft_weights::certify_weight_field;
use crate::construct::refusal::ConstructRefusal;
use truck_base::evidence::Budget;
use truck_geometry::prelude::{BSplineCurve, BSplineSurface, Cut, KnotVec, Vector4};
use truck_topology::{EntityId, Op, OpKind, OpParams, Selector};

/// The per-strip weight-certification subdivision budget.
///
/// CC-012 runs CC-011's certification once per delivered strip with a fixed
/// builder-owned budget. Certification terminates at the normative depth cap
/// [`CC_DEPTH_MAX`] whenever the field is admissible; this subdivision count
/// only needs to exceed the number of dyadic splits any admissible field can
/// require before every hull lies strictly above zero (the CC-011 test corpus
/// uses the same generous order for its "unlimited" budgets).
const STRIP_WEIGHT_SUBDIV_BUDGET: u32 = 1 << 20;

/// FNV-1a 64-bit offset basis (the landed identity algebra's constants).
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
/// FNV-1a 64-bit prime (the landed identity algebra's constants).
const FNV_PRIME: u64 = 0x00000100000001b3;

/// The delivered strip decomposition of a closed-wire loft.
///
/// `strips[j]` is the CC-010 loft of the `j`-th arc of every section; the
/// strips tile the closed section parameter domain in order, and adjacent
/// strips share their split vertex data by construction identity (P6).
/// `seam_ids[p]` is the P6 identity of the boundary shared by strip `p` and
/// strip `p + 1` (empty when there is a single strip).
#[derive(Debug, Clone)]
pub struct LoftStrips {
    /// The `r` strip lofts, in section-parameter order. `r = 1` (no splits)
    /// degenerates to a single open-style strip over the whole domain.
    pub strips: Vec<LoftOutput>,
    /// The strip-pair boundary identities (one per interior split).
    pub seam_ids: Vec<EntityId>,
}

/// Build a closed-wire loft as `r` strips over matched edges.
///
/// `sections` are the wire sections (one per station); `splits` are strictly
/// increasing indices into the sections' shared clamped knot vector selecting
/// the matched split parameters (exact existing knot values — the split is a
/// pure multiplicity raise + cut, never a new parameter value); `stations` is
/// the strictly increasing `v` stationing shared by every strip; `degree` is
/// the shared `v` interpolation degree.
///
/// The sections are first made compatible through
/// [`make_compatible`](crate::construct::loft::make_compatible) (exact degree
/// elevation + exact knot union; an empty or unclamped input refuses
/// [`ConstructRefusal::InvalidInput`]). Every strip is built with the SAME
/// banded factorization of the shared collocation storage, so the number of
/// strips is `splits.len() + 1`. Every delivered strip's homogeneous weight
/// field is certified once through CC-011 and the certificate's refinements
/// are applied to the shipped strip net (the CC-012 storage obligation). Any
/// weight field that does not certify strictly positive refuses
/// [`ConstructRefusal::NonPositiveWeightField`].
///
/// The L3 structural hypotheses are asserted here and refuse
/// [`ConstructRefusal::InvalidInput`] on a violation: every split parameter is
/// strictly interior and strictly increasing (a degenerate zero-length arc
/// refuses), every strip arc family is clamped and shares one `u` basis, and
/// the strip count stays `splits.len() + 1`.
pub fn loft_closed_wire(
    sections: &[BSplineCurve<Vector4>],
    splits: &[usize],
    stations: &[f64],
    degree: usize,
) -> Result<LoftStrips, ConstructRefusal> {
    if sections.is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }

    // Exact compatibility: one shared clamped u basis and degree.
    let compatible = make_compatible(sections)?;
    let shared_knot = compatible[0].knot_vec().clone();
    let u_degree = compatible[0].degree();
    for section in &compatible {
        if section.degree() != u_degree || section.knot_vec() != &shared_knot {
            return Err(ConstructRefusal::InvalidInput);
        }
        if !section.is_clamped() {
            return Err(ConstructRefusal::InvalidInput);
        }
    }

    // The matched split parameters (validated against the shared knot vector).
    let params = split_parameters(&shared_knot, splits)?;
    let strip_count = params.len() + 1;
    let domain_lo = shared_knot[0];
    let domain_hi = shared_knot[shared_knot.len() - 1];

    // ONE shared factorization of the shared stationing (reused by every
    // strip; identical epsilon on every strip is the observable).
    let factor = factor_banded_tp(&loft_collocation_bands(stations, degree)?)?;

    // The P6 split-value registry: each identity's homogeneous value is
    // computed once and consumed by both adjacent strips.
    let seam_ids: Vec<EntityId> = (0..(strip_count - 1))
        .map(|pair| seam_identity(compatible.len(), pair))
        .collect();
    let mut registry: BTreeMap<u64, (EntityId, [f64; 4])> = BTreeMap::new();
    let mut strips = Vec::with_capacity(strip_count);

    for j in 0..strip_count {
        let start = if j == 0 { domain_lo } else { params[j - 1] };
        let end = if j + 1 == strip_count {
            domain_hi
        } else {
            params[j]
        };

        let mut arcs = Vec::with_capacity(compatible.len());
        for (k, section) in compatible.iter().enumerate() {
            let mut arc = cut_subcurve(section, start, end)?;

            // Canonicalize the interior boundaries through the registry: the
            // shared split value is computed once per identity and both
            // strips consume it (P6, Section 1).
            if j > 0 {
                let id = vertex_identity(&seam_ids[j - 1], k);
                let value = registered_value(&mut registry, id, section, start)?;
                set_arc_boundary(&mut arc, true, value);
            }
            if j + 1 < strip_count {
                let id = vertex_identity(&seam_ids[j], k);
                let value = registered_value(&mut registry, id, section, end)?;
                set_arc_boundary(&mut arc, false, value);
            }

            // L3 hypothesis (1): clamped u-knots on every strip arc.
            if !arc.is_clamped() {
                return Err(ConstructRefusal::InvalidInput);
            }
            arcs.push(arc);
        }

        // L3 hypothesis (1): the strip's arc family shares one u basis.
        let arc_knot = arcs[0].knot_vec().clone();
        let arc_degree = arcs[0].degree();
        for arc in &arcs {
            if arc.knot_vec() != &arc_knot || arc.degree() != arc_degree {
                return Err(ConstructRefusal::InvalidInput);
            }
        }

        let output = loft_sections(&arcs, stations, degree, &factor)?;
        let surface = apply_weight_certification(output.surface)?;
        strips.push(LoftOutput {
            surface,
            epsilon: output.epsilon,
        });
    }

    Ok(LoftStrips { strips, seam_ids })
}

/// The matched split parameters of the shared clamped knot vector `knot`,
/// selected by strictly increasing interior indices.
///
/// Refuses [`ConstructRefusal::InvalidInput`] when an index is out of range,
/// when its knot value lies on the domain boundary (a seam split would create
/// a zero-length arc), when the indices are not strictly increasing, or when
/// two indices select the same knot value (a degenerate zero-length strip).
fn split_parameters(knot: &KnotVec, splits: &[usize]) -> Result<Vec<f64>, ConstructRefusal> {
    let last = knot.len() - 1;
    let a = knot[0];
    let b = knot[last];
    let mut params = Vec::with_capacity(splits.len());
    let mut previous: Option<usize> = None;
    for &index in splits {
        if index > last {
            return Err(ConstructRefusal::InvalidInput);
        }
        let value = knot[index];
        match (value > a, value < b) {
            (true, true) => {}
            _ => return Err(ConstructRefusal::InvalidInput),
        }
        if let Some(prev) = previous {
            if index <= prev {
                return Err(ConstructRefusal::InvalidInput);
            }
        }
        if let Some(&last_value) = params.last() {
            if value == last_value {
                return Err(ConstructRefusal::InvalidInput);
            }
        }
        params.push(value);
        previous = Some(index);
    }
    Ok(params)
}

/// The sub-curve of `section` over `[start, end]`, by exact knot insertion and
/// cut.
///
/// `start`/`end` must be knot values of `section` (they are validated against
/// the shared knot vector by the caller); the cuts raise the boundary knots to
/// their full multiplicity, so the returned arc is clamped on `[start, end]`.
fn cut_subcurve(
    section: &BSplineCurve<Vector4>,
    start: f64,
    end: f64,
) -> Result<BSplineCurve<Vector4>, ConstructRefusal> {
    let knot = section.knot_vec();
    let a = knot[0];
    let b = knot[knot.len() - 1];
    let mut work = section.clone();
    if start > a {
        work = work.cut(start);
    }
    if end < b {
        let _ = work.cut(end);
    }
    Ok(work)
}

/// Overwrite an arc's boundary control point with the canonical split value.
fn set_arc_boundary(arc: &mut BSplineCurve<Vector4>, start: bool, value: [f64; 4]) {
    let index = if start {
        0
    } else {
        arc.control_points().len() - 1
    };
    *arc.control_point_mut(index) = Vector4::new(value[0], value[1], value[2], value[3]);
}

/// The P6 seam identity of strip-pair `pair`: the `Loft` construction over the
/// `section_count` section imports, output slot = the strip-pair index.
fn seam_identity(section_count: usize, pair: usize) -> EntityId {
    let params = OpParams::List(
        (0..section_count as u32)
            .map(OpParams::Index)
            .collect::<Vec<OpParams>>(),
    );
    let op = Op {
        kind: OpKind::Loft,
        params,
    };
    let inputs: Vec<EntityId> = (0..section_count as u64).map(EntityId::src).collect();
    op.output(&inputs, pair as u32)
}

/// The per-station split-vertex identity: the `Pole(k)` structural sub-entity
/// of the strip-pair boundary.
fn vertex_identity(seam: &EntityId, station: usize) -> EntityId {
    EntityId::sel(seam.clone(), Selector::Pole(station as u32))
}

/// The stable FNV-1a content key of an identity (the landed algebra's
/// hashing; `BTreeMap` key so iteration is deterministic, never a `HashMap`).
fn registry_key(id: &EntityId) -> u64 {
    let mut hasher = SplitHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}

/// Compute-or-consume the homogeneous split value of `section` at `t` for the
/// given identity.
///
/// The value is computed ONCE per identity (Section 1): a later evaluation of
/// the same split point in another strip consumes the registry entry instead
/// of recomputing. The canonical value is the back control of the section cut
/// at `t` (the full-multiplicity breakpoint value).
fn registered_value(
    registry: &mut BTreeMap<u64, (EntityId, [f64; 4])>,
    id: EntityId,
    section: &BSplineCurve<Vector4>,
    t: f64,
) -> Result<[f64; 4], ConstructRefusal> {
    let key = registry_key(&id);
    if let Some((stored, value)) = registry.get(&key) {
        if *stored == id {
            return Ok(*value);
        }
    }
    let point = curve_value_at(section, t)?;
    let value = [point.x, point.y, point.z, point.w];
    registry.insert(key, (id, value));
    Ok(value)
}

/// The homogeneous value of `section` at the interior parameter `t`, computed
/// by exact knot insertion + cut (the single P6 computation of a split value).
fn curve_value_at(section: &BSplineCurve<Vector4>, t: f64) -> Result<Vector4, ConstructRefusal> {
    let mut work = section.clone();
    let _ = work.cut(t);
    match work.control_points().last() {
        Some(point) => Ok(*point),
        None => Err(ConstructRefusal::InvalidInput),
    }
}

/// CC-011 weight certification of one delivered strip, applying the returned
/// refinements to the shipped net (the CC-012 storage obligation).
///
/// Refuses [`ConstructRefusal::NonPositiveWeightField`] when the strip's
/// weight field does not certify strictly positive within the normative depth
/// cap.
fn apply_weight_certification(
    surface: BSplineSurface<Vector4>,
) -> Result<BSplineSurface<Vector4>, ConstructRefusal> {
    let mut budget = Budget::new(STRIP_WEIGHT_SUBDIV_BUDGET, 0, CC_DEPTH_MAX);
    let cert = certify_weight_field(&surface, &mut budget)?;
    if cert.refinements.is_empty() {
        return Ok(surface);
    }
    let mut refined = surface;
    for &(axis, _, knot) in &cert.refinements {
        if axis {
            refined.add_vknot(knot);
        } else {
            refined.add_uknot(knot);
        }
    }
    Ok(refined)
}

/// MurmurHash3's 64-bit finalizer (the landed algebra's finalizer).
fn fmix64(mut k: u64) -> u64 {
    k ^= k >> 33;
    k = k.wrapping_mul(0xff51afd7ed558ccd);
    k ^= k >> 33;
    k = k.wrapping_mul(0xc4ceb9fe1a85ec53);
    k ^= k >> 33;
    k
}

/// A process-, platform- and toolchain-stable FNV-1a hasher over the `Hash`
/// byte stream (the landed `StableHasher` pattern, little-endian integer
/// writes). Used only as the deterministic registry key, never as an identity.
#[derive(Default)]
struct SplitHasher(u64);

impl SplitHasher {
    /// A fresh hasher at the offset basis.
    fn new() -> Self {
        SplitHasher(FNV_OFFSET_BASIS)
    }
}

impl Hasher for SplitHasher {
    fn finish(&self) -> u64 {
        fmix64(self.0)
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(FNV_PRIME);
        }
    }

    fn write_u8(&mut self, i: u8) {
        self.write(&[i]);
    }

    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_le_bytes());
    }

    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_le_bytes());
    }

    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_le_bytes());
    }

    fn write_u128(&mut self, i: u128) {
        self.write(&i.to_le_bytes());
    }

    fn write_usize(&mut self, i: usize) {
        self.write(&(i as u64).to_le_bytes());
    }

    fn write_i8(&mut self, i: i8) {
        self.byte(i as u8);
    }

    fn write_i16(&mut self, i: i16) {
        self.write(&i.to_le_bytes());
    }

    fn write_i32(&mut self, i: i32) {
        self.write(&i.to_le_bytes());
    }

    fn write_i64(&mut self, i: i64) {
        self.write(&(i as u64).to_le_bytes());
    }

    fn write_i128(&mut self, i: i128) {
        self.write(&i.to_le_bytes());
    }

    fn write_isize(&mut self, i: isize) {
        self.write(&(i as u64).to_le_bytes());
    }
}

impl SplitHasher {
    fn byte(&mut self, b: u8) {
        self.0 ^= u64::from(b);
        self.0 = self.0.wrapping_mul(FNV_PRIME);
    }
}
