//! Checkpoint 9: polygonal realization and triangulation for a certified
//! rank-1 cylinder disk.
//!
//! # Reuse, not a new triangulator
//!
//! [`super::planar_slice::certified_polygonal_region`],
//! [`super::planar_slice::triangulate`] and
//! [`super::planar_slice::final_validity`] are reused completely unchanged.
//! They are generic over [`Rank0DevelopedBoundary`] and
//! [`CertifiedPolygonalRegion`] — types that carry a
//! [`BoundedMaterialRegion`] and a list of occurrences with their
//! represented approximation error — and never actually depend on the
//! *rank* of the ambient lattice; "rank 0" in their names is a fact about
//! the callers `planar_slice` was written for, not a constraint the
//! functions themselves enforce. A cylinder occurrence's represented
//! approximation error is exactly `0.0` (checkpoint 8's
//! [`super::cylinder_arrangement::placed_occurrences`] already builds it
//! that way, since a developed witness is analytically exact), so the
//! "exact polygon" certificate these functions grant to a rank-0 line-
//! bounded face is equally available here — for the same reason, not by
//! coincidence.
//!
//! What this module adds is only the one genuinely rank-1-specific step: the
//! reused functions stop at a 2D `TriangulatedRegion` in the developed
//! `(axial, angular)` chart, and [`lift_to_cylinder`] is the map back onto
//! the certified cylinder's physical embedding — the direct analogue of
//! [`super::planar_slice::lift_to_3d`], through the periodic trigonometric
//! parameterization rather than an affine one.

use super::cylinder::CylinderSchema;
use super::cylinder_arrangement::CertifiedCylinderDisk;
use super::planar_slice::{
    certified_polygonal_region, final_validity, triangulate, CertifiedPolygonalRegion,
    FinalValidityReport, Rank0DevelopedBoundary, Rank0Displacement, SliceExit, TriangulatedRegion,
};
use truck_geometry::prelude::{Point2, Point3, Vector3};

/// Step 9's complete rank-1 product: a validated triangle mesh, still in the
/// developed chart, plus its physical lift onto the certified cylinder.
#[derive(Debug, Clone)]
pub struct CertifiedCylinderMesh {
    /// The developed (2D) triangulated region.
    pub developed: TriangulatedRegion,
    /// The eleven-item final validity report.
    pub validity: FinalValidityReport,
    /// The developed vertices, lifted onto the cylinder's physical
    /// embedding, in the same order as `developed.vertices`.
    pub physical_vertices: Vec<Point3>,
}

/// Step 8A. Certify the disk's boundary as an exact polygon.
///
/// `occurrences` must be the same placed occurrences
/// [`super::cylinder_arrangement::certify_cylinder_disk`] built for this
/// disk (its `points` feed [`Rank0DevelopedBoundary::approximation_bound`]);
/// `disk.material` is that same call's certified material region.
pub fn certify_cylinder_polygon(
    disk: &CertifiedCylinderDisk,
    occurrences: &[super::planar_slice::CertifiedPlanarCurveOccurrence],
    tolerance: f64,
) -> Result<CertifiedPolygonalRegion, SliceExit> {
    let developed = Rank0DevelopedBoundary {
        occurrences: occurrences.to_vec(),
        displacements: vec![Rank0Displacement; occurrences.len()],
    };
    certified_polygonal_region(disk.material.clone(), &developed, tolerance)
}

/// Steps 8A, 8B and the physical lift, composed: certify the polygon,
/// triangulate it, run the final validity battery, and map every vertex back
/// onto the certified cylinder.
///
/// Reuses [`triangulate`] and [`final_validity`] exactly as the planar slice
/// does; the only new arithmetic is [`lift_to_cylinder`].
pub fn certify_cylinder_mesh(
    disk: &CertifiedCylinderDisk,
    occurrences: &[super::planar_slice::CertifiedPlanarCurveOccurrence],
    schema: &CylinderSchema,
    tolerance: f64,
) -> Result<CertifiedCylinderMesh, SliceExit> {
    let polygon = certify_cylinder_polygon(disk, occurrences, tolerance)?;
    let developed = triangulate(&polygon)?;
    let validity = final_validity(&developed, &polygon)?;
    let physical_vertices = lift_to_cylinder(&developed, schema);
    Ok(CertifiedCylinderMesh {
        developed,
        validity,
        physical_vertices,
    })
}

/// Map every developed vertex back onto the certified cylinder's physical
/// embedding: `x = axial * axis + radius * (cos(angular) radial_x + sin(angular)
/// radial_y)`, relative to `origin`.
///
/// The angular coordinate is used exactly as developed — never reduced to a
/// principal branch first. `cos`/`sin` are `2*PI`-periodic, so an unwrapped
/// developed angle (e.g. one carrying a full extra turn) lifts to the
/// identical physical point a wrapped one would, with no special case and no
/// loss of the developed-chart information the earlier stages relied on.
pub fn lift_to_cylinder(mesh: &TriangulatedRegion, schema: &CylinderSchema) -> Vec<Point3> {
    mesh.vertices
        .iter()
        .map(|p: &Point2| point_on_cylinder(schema, p.x, p.y))
        .collect()
}

fn point_on_cylinder(schema: &CylinderSchema, axial: f64, angular: f64) -> Point3 {
    let radius = schema.radius().get();
    let radial: Vector3 =
        radius * angular.cos() * schema.radial_x() + radius * angular.sin() * schema.radial_y();
    schema.origin() + axial * schema.axis() + radial
}

#[cfg(test)]
mod tests {
    use super::super::super::source_evidence::{BoundId, EdgeUseId};
    use super::super::curve_witness::{axial_line_witness, circumferential_arc_witness};
    use super::super::cylinder::{identify_cylinder, CylinderIdentification};
    use super::super::cylinder_arrangement::{certify_cylinder_disk, placed_occurrences};
    use super::*;
    use truck_geometry::prelude::{InnerSpace, Line, RevolutedCurve};
    use truck_topology::compress::OuterBoundStanding;

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

    fn on_cylinder(schema: &CylinderSchema, z: f64, theta: f64) -> Point3 {
        schema.origin()
            + z * schema.axis()
            + schema.radius().get() * theta.cos() * schema.radial_x()
            + schema.radius().get() * theta.sin() * schema.radial_y()
    }

    fn declared_outer() -> OuterBoundStanding {
        OuterBoundStanding::Declared {
            bound_index: 0,
            declared_count: 1,
        }
    }

    /// The full synthetic vertical slice's terminal milestone: a supported
    /// cylinder disk reaches a validated physical mesh, with every lifted
    /// vertex actually on the certified cylinder.
    #[test]
    fn a_narrow_quad_produces_a_valid_physical_mesh() {
        let schema = z_cylinder(2.0, 5.0);
        let p0 = on_cylinder(&schema, 0.0, 0.2);
        let p1 = on_cylinder(&schema, 0.0, 1.4);
        let p2 = on_cylinder(&schema, 3.0, 1.4);
        let p3 = on_cylinder(&schema, 3.0, 0.2);
        let witnesses = vec![
            circumferential_arc_witness(&schema, p0, p1, 1.2).unwrap(),
            axial_line_witness(&schema, p1, p2).unwrap(),
            circumferential_arc_witness(&schema, p2, p3, -1.2).unwrap(),
            axial_line_witness(&schema, p3, p0).unwrap(),
        ];
        let edge_uses: Vec<_> = (0..4).map(|i| EdgeUseId::new(BoundId(0), i)).collect();
        let placements = vec![0i64, 0, 0, 0];
        let generator = schema.deck_generator();

        let disk = certify_cylinder_disk(
            &edge_uses,
            &witnesses,
            &placements,
            generator,
            declared_outer(),
            &[0],
        )
        .expect("a narrow quad certifies a valid disk");

        let occurrences = placed_occurrences(&edge_uses, &witnesses, &placements, &generator);
        let mesh = certify_cylinder_mesh(&disk, &occurrences, &schema, 1e-9)
            .expect("a narrow quad's disk triangulates and validates");

        assert_eq!(mesh.validity.triangles, mesh.developed.triangles.len());
        assert_eq!(
            mesh.developed.triangles.len() + 2,
            mesh.developed.vertices.len()
        );
        assert_eq!(mesh.physical_vertices.len(), mesh.developed.vertices.len());

        let radius = schema.radius().get();
        for vertex in &mesh.physical_vertices {
            let r = *vertex - schema.origin();
            let axial = r.dot(schema.axis());
            let radial = r - axial * schema.axis();
            assert!(
                (radial.magnitude() - radius).abs() < 1e-9,
                "lifted vertex {vertex:?} is not on the cylinder"
            );
        }
    }
}
