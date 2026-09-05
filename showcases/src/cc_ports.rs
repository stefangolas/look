//! The anti-corruption layer for the in-flight CC (certified construction)
//! program. Every showcase feature the CC packets will land is named HERE and
//! ONLY here: the showcase tables and builders are written against this trait
//! today, compile against the `LandedPorts` stub, and switch to the real
//! `truck-certified::construct` signatures behind the `cc` feature when the
//! packets land. The same trait is what `truck123d` (the pyo3 facade) wraps
//! later, so the Python side inherits identical semantics.
//!
//! Refusal discipline: a port that is not yet landed reports
//! `UnsupportedEnvelope(ContactReductionDeferred)` — a deferred capability is
//! surfaced, never silently skipped and never approximated.

use truck_base::cgmath64::Point3;
use truck_base::evidence::{EnvelopeCase, Outcome, Refusal};
use truck_geometry::constructive::DirectTolerance;
use truck_modeling::{Curve, Solid, Wire};

/// The CC-031 admissible radius-law family (v1). Part of the portable table
/// format: the same enum is what the amphora/teapot tables serialize.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RadiusLaw {
    /// CC-031 constant law.
    Constant(f64),
    /// CC-031 linear law: `start` at s=0 to `end` at s=1.
    Linear(f64, f64),
    /// CC-031 cubic Hermite law: values and tangents at both ends,
    /// `(v0, m0, v1, m1)`.
    CubicHermite(f64, f64, f64, f64),
    /// CC-031 monotone-cubic law through `(s, r)` control radii.
    MonotoneCubic(Vec<(f64, f64)>),
}

impl RadiusLaw {
    /// The radius at normalized station `s ∈ [0, 1]`. Portable math: this is
    /// the exact evaluation truck123d must reproduce.
    pub fn at(&self, s: f64) -> f64 {
        match self {
            RadiusLaw::Constant(r) => *r,
            RadiusLaw::Linear(start, end) => start + (end - start) * s,
            RadiusLaw::CubicHermite(v0, m0, v1, m1) => {
                let t = s;
                let t2 = t * t;
                let t3 = t2 * t;
                let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
                let h10 = t3 - 2.0 * t2 + t;
                let h01 = -2.0 * t3 + 3.0 * t2;
                let h11 = t3 - t2;
                h00 * v0 + h10 * m0 + h01 * v1 + h11 * m1
            }
            RadiusLaw::MonotoneCubic(points) => monotone_pchip(points, s),
        }
    }
}

/// Fritsch–Carlson monotone piecewise-cubic interpolation (the portable
/// reference implementation of the `MonotoneCubic` law; the certified side
/// will re-derive it with interval bounds).
fn monotone_pchip(points: &[(f64, f64)], s: f64) -> f64 {
    if points.is_empty() {
        return f64::NAN;
    }
    if points.len() == 1 || s <= points[0].0 {
        return points[0].1;
    }
    if s >= points[points.len() - 1].0 {
        return points[points.len() - 1].1;
    }
    let mut seg = 0usize;
    for i in 0..points.len() - 1 {
        if s >= points[i].0 && s <= points[i + 1].0 {
            seg = i;
            break;
        }
    }
    let (x0, y0) = points[seg];
    let (x1, y1) = points[seg + 1];
    let h = x1 - x0;
    let t = (s - x0) / h;
    let delta = (y1 - y0) / h;
    let m0 = if seg == 0 {
        delta
    } else {
        let d_prev = (y0 - points[seg - 1].1) / (x0 - points[seg - 1].0);
        if d_prev * delta <= 0.0 {
            0.0
        } else {
            2.0 * d_prev * delta / (d_prev + delta)
        }
    };
    let m1 = if seg + 2 == points.len() {
        delta
    } else {
        let d_next = (points[seg + 2].1 - y1) / (points[seg + 2].0 - x1);
        if d_next * delta <= 0.0 {
            0.0
        } else {
            2.0 * delta * d_next / (delta + d_next)
        }
    };
    let t2 = t * t;
    let t3 = t2 * t;
    (2.0 * t3 - 3.0 * t2 + 1.0) * y0
        + (t3 - 2.0 * t2 + t) * h * m0
        + (-2.0 * t3 + 3.0 * t2) * y1
        + (t3 - t2) * h * m1
}

/// CC-004 `Clear` outcome: the certified minimum distance and the margin by
/// which it clears the required separation.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClearCert {
    pub distance: f64,
    pub required: f64,
    pub margin: f64,
}

/// CC-025 canal regularity outcome for one spine.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CanalCert {
    pub regular: bool,
    pub min_curvature_radius: f64,
    pub tube_radius: f64,
}

/// CC-023/CC-026 thickness outcome: the thinnest certified wall.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ThicknessCert {
    pub t_safe: f64,
    pub t_focal: f64,
    pub d_min_half: f64,
}

/// One body rib: a closed ring at height `z` (the loft's station wire, in
/// client-side representation until CC-010's `Wire` ingestion lands).
#[derive(Debug, Clone, PartialEq)]
pub struct RibWire {
    pub z: f64,
    pub ring: truck_geometry::constructive::Profile2D,
}

/// The CC ports. Implementations: `LandedPorts` (today; defers) and, behind
/// the `cc` feature, the `truck-certified` realization.
pub trait CcPorts {
    /// CC-010..014: loft `station_wires` with declared positional
    /// correspondence into a closed solid.
    fn loft(&self, stations: &[Wire], _tol: &DirectTolerance) -> Outcome<Solid> {
        let _ = stations;
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred))
    }

    /// CC-010..014, amphora-facing: loft the rib set (height-ordered rings)
    /// into the closed vessel body.
    fn loft_ribs(&self, ribs: &[RibWire]) -> Outcome<Solid> {
        let _ = ribs;
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred))
    }

    /// CC-015: Gordon boolean-sum blend over the same rib set (A/B against
    /// [`CcPorts::loft`]).
    fn gordon(&self, stations: &[Wire]) -> Outcome<Solid> {
        let _ = stations;
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred))
    }

    /// CC-015, amphora-facing: Gordon blend over the rib set.
    fn gordon_ribs(&self, ribs: &[RibWire]) -> Outcome<Solid> {
        let _ = ribs;
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred))
    }

    /// CC-030/031: variable-radius blend along one named edge.
    fn blend_var_radius(
        &self,
        solid: &Solid,
        _edge: (Point3, Point3),
        _law: &RadiusLaw,
    ) -> Outcome<Solid> {
        let _ = solid;
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred))
    }

    /// CC-030/031, amphora-facing: the handle-root blend where each handle
    /// meets the body, radius growing with height per the law.
    fn blend_handle_root(&self, ribs: &[RibWire], _law: &RadiusLaw) -> Outcome<Solid> {
        let _ = ribs;
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred))
    }

    /// CC-004: certified minimum distance (Clear) between two solids.
    fn clear(&self, a: &Solid, b: &Solid, required: f64) -> Outcome<ClearCert> {
        let _ = (a, b, required);
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred))
    }

    /// CC-025: canal regularity certificate for one spine.
    fn canal_regularity(&self, spine: &Curve, tube_radius: f64) -> Outcome<CanalCert> {
        let _ = (spine, tube_radius);
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred))
    }

    /// CC-025, amphora-facing: canal regularity of the handle spine by table
    /// points (the spine is rebuilt kernel-side from the same table).
    fn canal_cert(
        &self,
        handle_points: &[(f64, f64, f64)],
        azimuth_deg: f64,
        tube_radius: f64,
    ) -> Outcome<CanalCert> {
        let _ = (handle_points, azimuth_deg, tube_radius);
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred))
    }

    /// CC-023/026: the thinnest certified shell wall of `solid`.
    fn certify_shell(&self, solid: &Solid) -> Outcome<ThicknessCert> {
        let _ = solid;
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred))
    }

    /// CC-023/026, amphora-facing: the thinnest certified wall of the lofted
    /// body over the rib set.
    fn shell_thickness(&self, ribs: &[RibWire]) -> Outcome<ThicknessCert> {
        let _ = ribs;
        Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred))
    }
}

/// The ports available today: every CC feature defers with the typed refusal.
#[derive(Debug, Clone, Copy, Default)]
pub struct LandedPorts;

impl CcPorts for LandedPorts {}
