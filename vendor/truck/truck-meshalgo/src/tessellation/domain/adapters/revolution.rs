//! Conical and Revoluted Surface Parametric Quotient Adapter.
//!
//! Provides the quotient schema for conical and revoluted surface primitives,
//! integrating the exact closed-form analytical vector apex solver
//!   u_apex = - (W0 · ΔW) / ||ΔW||²
//! and collapse proof ||W(u_apex)|| <= 10⁻³.

use crate::cgmath::{InnerSpace, Point2, Point3, Vector2, Vector3};
use truck_geometry::prelude::*;

use crate::tessellation::domain::schema::{
    DeckLattice, ParametricQuotient, ParametricQuotientSurface, SchemaFailure, SingularStratum,
    StratumCertificate, StratumId,
};

/// Adapter wrapping a revoluted or conical surface to expose its quotient schema.
#[derive(Debug)]
pub struct RevolutionAdapter<'a, S> {
    /// Underlying parametric surface reference.
    pub surface: &'a S,
}

impl<'a, S> RevolutionAdapter<'a, S> {
    /// Creates a new `RevolutionAdapter` for the given surface.
    pub fn new(surface: &'a S) -> Self {
        Self { surface }
    }
}

impl<'a, S> ParametricQuotientSurface for RevolutionAdapter<'a, S>
where
    S: ParametricSurface3D<Point = Point3<f64>, Vector = Vector3<f64>>,
{
    fn quotient_schema(&self, _tol: f64) -> std::result::Result<ParametricQuotient, SchemaFailure> {
        let vp = self
            .surface
            .v_period()
            .ok_or(SchemaFailure::UnsupportedSurfaceType)?;
        let lattice = DeckLattice::Rank1 {
            generator: Vector2::new(0.0, vp),
        };

        let mut singular_strata = Vec::new();

        // Exact closed-form vector analytical cone apex solver:
        // W(u) = P(u, 0) - P(u, π) = W0 + u * ΔW
        let w = |u: f64| -> Vector3<f64> {
            let p0 = self.surface.subs(u, 0.0);
            let p_half = self.surface.subs(u, 0.5 * vp);
            p0 - p_half
        };

        let w0 = w(0.0);
        let w1 = w(1.0);
        let dw = w1 - w0;
        let dw2 = dw.magnitude2();

        if dw2 >= 1e-12 {
            let u_apex = -w0.dot(dw) / dw2;
            let res = w(u_apex).magnitude();
            if res <= 1e-3 {
                let apex_p3 = self.surface.subs(u_apex, 0.0);
                singular_strata.push(SingularStratum {
                    id: StratumId(0),
                    image: apex_p3,
                    certificate: StratumCertificate::AnalyticLinearRevolution {
                        apex_u: u_apex,
                        residual: res,
                    },
                });
            }
        }

        Ok(ParametricQuotient {
            lattice,
            singular_strata,
            u_period: self.surface.u_period(),
            v_period: Some(vp),
        })
    }

    fn evaluate_cover(&self, uv: Point2<f64>) -> Point3<f64> {
        self.surface.subs(uv.x, uv.y)
    }
}
